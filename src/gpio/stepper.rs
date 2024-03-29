use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::{Output, OutputPin, Pin, PinDriver};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_sys::EspError;
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
}

impl<'d, PinA, PinB> StepperMotor<'d, PinA, PinB>
where
    PinA: OutputPin,
    PinB: OutputPin,
{
    pub fn new(
        stepper_pin: impl Peripheral<P = PinA> + 'd,
        dir_pin: impl Peripheral<P = PinB> + 'd,
    ) -> Self {
        let mut stepper = StepperMotor {
            step: PinDriver::output(stepper_pin).unwrap(),
            dir: PinDriver::output(dir_pin).unwrap(),
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

    pub fn drive(&mut self, steps: u32, pulse_width: u32) -> anyhow::Result<(), EspError> {
        for _ in 0..steps {
            self.step(pulse_width)?;
        }
        Ok(())
    }
}
