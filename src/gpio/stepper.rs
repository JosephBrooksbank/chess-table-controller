use std::num::NonZeroU32;

use esp_idf_hal::{delay, timer};
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::{Output, OutputPin, Pin, PinDriver};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::task::notification::Notification;
use esp_idf_hal::timer::{Timer, TimerConfig, TimerDriver};
use esp_idf_sys::EspError;
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
}

pub trait Stepper {
    fn step(&mut self, pulse_width: u32) -> anyhow::Result<(), EspError>;
    fn set_direction(&mut self, dir: StepperDirection) -> anyhow::Result<(), EspError>;
    fn drive(&mut self, steps: u32, pulse_width: u32, accel: u64, max_sps: u64) -> anyhow::Result<(), EspError>;
}

pub struct StepperMotor<PinA, PinB, TIMER00, TIMER01>
    where
        PinA: Pin,
        PinB: Pin,
{
    step: PinDriver<'static, PinA, Output>,
    dir: PinDriver<'static, PinB, Output>,
    sps_timer: TIMER00,
    accel_timer: TIMER01,

}

impl<PinA, PinB, TIMER00, TIMER01> StepperMotor<PinA, PinB, TIMER00, TIMER01>
    where
        PinA: OutputPin,
        PinB: OutputPin,
        TIMER00: Peripheral<P=timer::TIMER00>,
        TIMER01: Peripheral<P=timer::TIMER01>,
{
    pub fn new(
        stepper_pin: impl Peripheral<P=PinA> + 'static,
        dir_pin: impl Peripheral<P=PinB> + 'static,
        sps_timer: TIMER00,
        accel_timer: TIMER01,
    ) -> Self {
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

    fn setup_timers<'d, Timer00, Timer1>(timer00: impl Peripheral<P=Timer00> + 'd, timer01: impl Peripheral<P=Timer1> + 'd) -> (TimerDriver<'d>, TimerDriver<'d>)
        where
            Timer00: Timer,
            Timer1: Timer
    {
        let mut timer_config = TimerConfig::new();
        timer_config.auto_reload = true;
        timer_config.divider = 10;

        let sps_timer = TimerDriver::new(timer00, &timer_config).unwrap();
        let accel_timer = TimerDriver::new(timer01, &timer_config).unwrap();
        (sps_timer, accel_timer)
    }

    pub unsafe fn drive(&mut self, steps: u32, pulse_width: u32, accel: u64, max_sps: u64) -> anyhow::Result<(), EspError> {
        let sps_timer = self.sps_timer.clone_unchecked();
        let accel_timer = self.accel_timer.clone_unchecked();
        let (mut sps_timer, mut accel_timer) = Self::setup_timers(sps_timer, accel_timer);
        let mut current_sps = 1;
        let mut current_step = 0;
        
        let stop_accel = (steps as f64 * Self::ACCEL_PERCENT).floor() as u32;
        let mut start_decel = steps - stop_accel;
        

        let instruction_notifications = Notification::new();
        let sps_instruction_notifier = instruction_notifications.notifier();
        let accel_instruction_notifier = instruction_notifications.notifier();
        let timer_hz = sps_timer.tick_hz();

        let sps_timer_register = timer_hz / current_sps;
        sps_timer.subscribe(move || {
            sps_instruction_notifier.notify_and_yield(NonZeroU32::new(Instruction::Step as u32).unwrap());
        }).unwrap();
        sps_timer.set_alarm(sps_timer_register).unwrap();
        sps_timer.enable_alarm(true).unwrap();
        sps_timer.enable_interrupt().unwrap();
        sps_timer.enable(true).unwrap();

        let accel_timer_register = accel_timer.tick_hz() / accel;
        accel_timer.set_alarm(accel_timer_register).unwrap();
        accel_timer.enable_alarm(true).unwrap();
        accel_timer.subscribe(move || {
            accel_instruction_notifier.notify_and_yield(NonZeroU32::new(Instruction::Accelerate as u32).unwrap());
        }).unwrap();
        accel_timer.enable_interrupt().unwrap();
        accel_timer.enable(true).unwrap();

        
        let mut stepper_state = StepperState::Accelerating;

        loop {
            match instruction_notifications.wait(delay::BLOCK) {
                None => {}
                Some(instruction) => {
                    match instruction.get() {
                        1 => {
                            self.step(pulse_width)?;
                            current_step += 1;
                            info!("current step: {}", current_step);
                            if current_step >= steps {
                                break;
                            }
                            
                            if current_step == stop_accel {
                                stepper_state = StepperState::Constant;
                            }
                            if current_step >= start_decel {
                                stepper_state = StepperState::Decelerating;
                            }
                        }
                        2 => {
                            match stepper_state {
                                StepperState::Accelerating => {
                                    current_sps += 1;
                                    if current_sps >= max_sps {
                                        stepper_state = StepperState::Constant;
                                        current_sps = max_sps;
                                        start_decel = steps - current_step;
                                    }
                                }
                                StepperState::Constant => {continue;}
                                StepperState::Decelerating => {
                                    current_sps -= 1;
                                    if current_sps == 0 {
                                        current_sps = 1;
                                    }
                                    stepper_state = StepperState::Constant;
                                }
                            }
                            info!("current sps: {}", current_sps);
                            let sps_timer_register = timer_hz / current_sps;
                            sps_timer.set_alarm(sps_timer_register).unwrap();
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }
}
enum Instruction {
    Step = 1,
    Accelerate = 2,
}