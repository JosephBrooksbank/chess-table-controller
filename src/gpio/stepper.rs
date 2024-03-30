use std::sync::mpsc;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::{Output, OutputPin, Pin, PinDriver};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::timer;
use esp_idf_hal::timer::{TimerConfig, TimerDriver};
use esp_idf_sys::EspError;
use log::info;
use serde::Deserialize;

#[derive(Deserialize)]
pub enum StepperDirection {
    Clockwise,
    Counterclockwise,
}

pub struct StepperMotor<'d, PinA, PinB>
where
    PinA: Pin,
    PinB: Pin,
{
    step: PinDriver<'d, PinA, Output>,
    dir: PinDriver<'d, PinB, Output>,
    sps_timer: TimerDriver<'d >,
    accel_timer: TimerDriver<'d>,

}

impl<'d, PinA, PinB> StepperMotor<'d, PinA, PinB>
where
    PinA: OutputPin,
    PinB: OutputPin,
{
    pub fn new(
        stepper_pin: impl Peripheral<P = PinA> + 'd,
        dir_pin: impl Peripheral<P = PinB> + 'd,
        sps_timer: impl Peripheral<P = timer::TIMER00> + 'd,
        accel_timer: impl Peripheral<P = timer::TIMER01> + 'd
    ) -> Self {

        let mut timer_config = TimerConfig::new();
        timer_config.auto_reload = true;
        timer_config.divider = 10;
        let sps_timer = TimerDriver::new(sps_timer, &timer_config).unwrap();
        let accel_timer = TimerDriver::new(accel_timer, &timer_config).unwrap();

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

    const MAX_SPS: u32 = 1000;
    const SPSPS: u32 = 1;
    pub unsafe fn drive(&mut self, steps: u32, pulse_width: u32) -> anyhow::Result<(), EspError> {
        let mut counter: u32 = 0;
        let mut sps = 1;
        let timer_hz = self.sps_timer.tick_hz();
        let sps_timer_register = timer_hz / sps;
        let (tx,rx) = mpsc::channel::<u8>();
        self.sps_timer.subscribe(move || {
            tx.send(1).unwrap();
        })?;
        self.sps_timer.set_alarm(sps_timer_register)?;
        self.sps_timer.enable_interrupt()?;
        self.sps_timer.enable_alarm(true)?;
        self.sps_timer.enable(true)?;
        
        loop {
            if counter == steps {
                break;
            }
            if let Ok(_) = rx.recv() {
                self.step(pulse_width)?;
                counter += 1;
                info!("Counter: {}", counter);
            }
        }
            
        Ok(())
    }
}
