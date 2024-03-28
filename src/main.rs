use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use esp_idf_hal::gpio::{self, Output, OutputPin, Pin, PinDriver};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_sys::EspError;

enum StepperDirection {
    Clockwise,
    Counterclockwise,
}
struct StepperMotor<'d, PinA, PinB>
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
    fn new(
        stepper_pin: impl Peripheral<P = PinA> + 'd,
        dir_pin: impl Peripheral<P = PinB> + 'd,
    ) -> Self {
        StepperMotor {
            step: PinDriver::output(stepper_pin).unwrap(),
            dir: PinDriver::output(dir_pin).unwrap(),
        }
    }

    fn step(&mut self) -> anyhow::Result<(), EspError> {
        self.step.set_high()?;
        std::thread::sleep(Duration::from_millis(5));
        self.step.set_low()?;
        std::thread::sleep(Duration::from_millis(5));
        Ok(())
    }

    // todo! ensure these directions are correct.
    fn set_direction(&mut self, dir: StepperDirection) -> anyhow::Result<(), EspError> {
        match dir {
            StepperDirection::Clockwise => self.dir.set_high(),
            StepperDirection::Counterclockwise => self.dir.set_low(),
        }
    }
}

fn main() -> anyhow::Result<()> {
    esp_idf_hal::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    // take control of peripherals
    let peripherals = Peripherals::take()?;

    let mut stepper = StepperMotor::new(peripherals.pins.gpio27, peripherals.pins.gpio14);
    stepper.set_direction(StepperDirection::Clockwise)?;
    let mut led = PinDriver::output(peripherals.pins.gpio2)?;
    let mut button = PinDriver::input(peripherals.pins.gpio26)?;
    button.set_pull(gpio::Pull::Down)?;

    let m = Arc::new(Mutex::new(true));

    let m2 = m.clone();
    thread::spawn(move || loop {
        let on;
        {
            let mutex_ref = m2.lock().unwrap();
            on = *mutex_ref;
        }
        led.set_level(match on {
            true => gpio::Level::High,
            false => gpio::Level::Low,
        })
        .unwrap();

        thread::sleep(Duration::from_millis(10));
    });

    loop {
        if button.is_high() {
            let mut mutex_ref = m.lock().unwrap();
            *mutex_ref = true;
            println!("Sending Pulse to Stepper!");
        } else {
            let mut mutex_ref = m.lock().unwrap();
            *mutex_ref = false;
        }

        thread::sleep(Duration::from_millis(10));
    }
}
