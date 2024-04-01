use std::num::NonZeroU32;
use std::sync::{Arc, mpsc, Mutex};
use std::thread;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::{Output, OutputPin, Pin, PinDriver};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::task::notification::Notification;
use esp_idf_hal::timer;
use esp_idf_hal::timer::{TimerConfig, TimerDriver};
use esp_idf_sys::{EspError, qsort};
use log::info;
use serde::Deserialize;

#[derive(Deserialize)]
pub enum StepperDirection {
    Clockwise,
    Counterclockwise,
}


#[derive(PartialEq)]
enum StepperState {
    Accelerating,
    Constant,
    Decelerating,
    CurrentStep(u32),
}

pub struct StepperMotor< PinA, PinB>
where
    PinA: Pin,
    PinB: Pin,
{
    step: PinDriver<'static, PinA, Output>,
    dir: PinDriver<'static, PinB, Output>,
    sps_timer: Arc<Mutex<TimerDriver<'static >>>,
    accel_timer: Arc<Mutex<TimerDriver<'static >>>,

}

impl<PinA, PinB> StepperMotor<PinA, PinB>
where
    PinA: OutputPin,
    PinB: OutputPin,
{
    pub fn new(
        stepper_pin: impl Peripheral<P = PinA> + 'static,
        dir_pin: impl Peripheral<P = PinB> + 'static,
        sps_timer: impl Peripheral<P = timer::TIMER00> + 'static,
        accel_timer: impl Peripheral<P = timer::TIMER01> + 'static
    ) -> Self {

        let mut timer_config = TimerConfig::new();
        timer_config.auto_reload = true;
        timer_config.divider = 10;
        let sps_timer = Arc::new(Mutex::new(TimerDriver::new(sps_timer, &timer_config).unwrap()));
        let accel_timer = Arc::new(Mutex::new(TimerDriver::new(accel_timer, &timer_config).unwrap()));

        let mut stepper = StepperMotor {
            step: PinDriver::output(stepper_pin).unwrap(),
            dir: PinDriver::output(dir_pin).unwrap(),
            sps_timer,
            accel_timer,
        };
        stepper.set_direction(StepperDirection::Clockwise).unwrap();
        stepper
    }

    pub fn step(&mut self, pulse_width: u32) -> anyhow::Result<(), EspError> {
        self.step.set_high()?;
        Ets::delay_us(pulse_width);
        self.step.set_low()?;
        Ets::delay_us(pulse_width);
        Ok(())
    }

    // todo! ensure these directions are correct.
    pub fn set_direction(&mut self, dir: StepperDirection) -> anyhow::Result<(), EspError> {
        match dir {
            StepperDirection::Clockwise => self.dir.set_high(),
            StepperDirection::Counterclockwise => self.dir.set_low(),
        }
    }
    
    // const MAX_SPS: u64 = 1000;
    // const ACCEL: u64 = 100;
    const GRANULARITY: u64 = 1;
    
    const ACCEL_PERCENT: f64 = 0.2;


    pub unsafe fn drive(&mut self, steps: u32, pulse_width: u32, accel: u64, max_sps: u64) -> anyhow::Result<(), EspError> {
        let mut counter: u32 = 0;
        let mut sps = 1;
        let accels_per_sec =  accel / Self::GRANULARITY;
        let notification = Notification::new();
        let notifier = notification.notifier();
        let stop_accel = (steps as f64 *Self::ACCEL_PERCENT).floor() as u32;
        let mut start_decel = steps - stop_accel;
        
        // initial set up of timer in lower scope so the lock is released
        {
            let mut sps_timer = self.sps_timer.lock().unwrap();
            let timer_hz = sps_timer.tick_hz();
            let sps_timer_register = timer_hz / sps;


            sps_timer.subscribe(move || {
                let bitset = 0b10001010101;
                notifier.notify_and_yield(NonZeroU32::new(1).unwrap());
            })?;
            sps_timer.set_alarm(sps_timer_register)?;
            sps_timer.enable_interrupt()?;
            sps_timer.enable_alarm(true)?;
            sps_timer.enable(true)?;
        }
        
        let sps_timer_ref = self.sps_timer.clone();
        let accel_timer_ref = self.accel_timer.clone();
        let (stepper_state_tx, stepper_state_rx) = mpsc::channel();
        let (decel_tx, decel_rx) = mpsc::channel();
        
        
        
        thread::spawn(move || {
            info!("starting acceleration thread");
            let mut sps_timer = sps_timer_ref.lock().unwrap();
            let mut accel_timer = accel_timer_ref.lock().unwrap();
            let timer_hz = sps_timer.tick_hz();
            let mut accel_timer_register = accel_timer.tick_hz() / accels_per_sec as u64;
            info!("grabbed locks");
            let notification = Notification::new();
            let notifier = notification.notifier();
            accel_timer.subscribe(move || {
                let bitset = 0b10001010101;
                notifier.notify_and_yield(NonZeroU32::new(1).unwrap());
            }).unwrap();
            accel_timer.set_alarm(accel_timer_register).unwrap();
            accel_timer.enable_alarm(true).unwrap();
            accel_timer.enable_interrupt().unwrap();
            accel_timer.enable(true).unwrap();
            let mut stepper_state = StepperState::Accelerating;
            let mut current_step = 0;
            loop {
                if stepper_state == StepperState::Constant {
                    match stepper_state_rx.recv() {
                        Ok(val) => {
                            match val {
                                StepperState::Accelerating => {
                                    accel_timer.enable_alarm(true).unwrap();
                                    stepper_state = StepperState::Accelerating;
                                }
                                StepperState::Decelerating => {
                                    accel_timer.enable_alarm(true).unwrap();
                                    stepper_state = StepperState::Decelerating;
                                }
                                StepperState::Constant => {
                                    continue;
                                },
                                StepperState::CurrentStep(step) => {
                                    current_step = step;
                                }
                            }
                        },
                        Err(_) => {
                            break;
                        }
                    }
                }
                match stepper_state_rx.try_recv() {
                    Ok(val) => {
                        match val {
                            StepperState::CurrentStep(step) => {
                                current_step = step;
                            }
                            _ => {
                                stepper_state = val;
                            }
                        }
                    }
                    Err(e) => {
                        if let mpsc::TryRecvError::Disconnected = e {
                            break;
                        }
                    }
                }
                let bitset = notification.wait(esp_idf_hal::delay::BLOCK);
                if let Some(_bitset) = bitset {
                    match stepper_state {
                        StepperState::Accelerating => sps += 1,
                        StepperState::Constant => {
                            accel_timer.enable_alarm(false).unwrap();
                            continue;
                        },
                        StepperState::Decelerating => sps -= 1,
                        StepperState::CurrentStep(step) => {
                            current_step = step;
                        }
                    }
                    if sps > max_sps {
                        info!("setting the point to decelerate to {}", steps - current_step);
                        decel_tx.send(current_step).unwrap();
                        sps = max_sps; 
                        stepper_state = StepperState::Constant;
                    }
                    if sps == 0 {
                        sps = 1;
                    }
                    info!("sps: {}", sps);
                    sps_timer.set_alarm(timer_hz / sps).unwrap();
                    sps_timer.enable_alarm(true).unwrap()
                }
            }
        });


        loop {
            if counter == steps {
                break;
            }
            match decel_rx.try_recv() {
                Ok(step) => {
                    info!("setting the point to decelerate to {}", steps - step);
                    // start_decel = steps - step;
                }
                Err(e) => {
                    if let mpsc::TryRecvError::Disconnected = e {
                        break;
                    }
                }
            }
                
            let bitset = notification.wait(esp_idf_hal::delay::BLOCK);
            if let Some(bitset) = bitset {
                self.step(pulse_width)?;
                counter += 1;
                println!("on step: {}", counter);
                if counter == stop_accel {
                    println!("stop accel");
                    stepper_state_tx.send(StepperState::Constant).unwrap();
                }
                else if counter == start_decel {
                    println!("start decel");
                    stepper_state_tx.send(StepperState::Decelerating).unwrap();
                } else {
                    stepper_state_tx.send(StepperState::CurrentStep(counter)).unwrap();
                }
            }
        }
            
        Ok(())
    }
}
