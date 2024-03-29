use std::time::Duration;

use esp_idf_hal::gpio::{Output, OutputPin, Pin, PinDriver};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_sys::EspError;

enum StepperDirection {
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

    pub fn step(&mut self) -> anyhow::Result<(), EspError> {
        self.step.set_high()?;
        std::thread::sleep(Duration::from_millis(5));
        self.step.set_low()?;
        std::thread::sleep(Duration::from_millis(5));
        Ok(())
    }

    // todo! ensure these directions are correct.
    pub fn set_direction(&mut self, dir: StepperDirection) -> anyhow::Result<(), EspError> {
        match dir {
            StepperDirection::Clockwise => self.dir.set_high(),
            StepperDirection::Counterclockwise => self.dir.set_low(),
        }
    }
}
