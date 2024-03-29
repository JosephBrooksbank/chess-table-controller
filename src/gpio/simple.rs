pub use esp_idf_hal::gpio::Level;
use esp_idf_hal::gpio::{
    AnyIOPin, Gpio16, Gpio2, Gpio26, Gpio27, Input, Output, OutputPin, PinDriver, Pins, Pull,
};
use esp_idf_hal::peripheral::Peripheral;
use std::time::Duration;

pub struct Button<'a> {
    pin_driver: PinDriver<'a, AnyIOPin, Input>,
}

impl<'a> Button<'a> {
    pub fn new(pin: AnyIOPin) -> Self {
        let mut pin_driver = PinDriver::input(pin).unwrap();
        pin_driver.set_pull(Pull::Down).unwrap();
        Button { pin_driver }
    }
}

pub struct Led<'d, Pin>
where
    Pin: OutputPin,
{
    pin: PinDriver<'d, Pin, Output>,
}

impl<'d, Pin> Led<'d, Pin>
where
    Pin: OutputPin,
{
    pub fn new(pin: impl Peripheral<P = Pin> + 'd) -> Self {
        Led {
            pin: PinDriver::output(pin).unwrap(),
        }
    }

    pub fn turn_on(&mut self) {
        self.pin.set_high().unwrap();
    }
    pub fn turn_off(&mut self) {
        self.pin.set_low().unwrap();
    }
    pub fn flash(&mut self, on: Duration) {
        self.turn_on();
        std::thread::sleep(on);
        self.turn_off();
    }

    pub fn set_level(&mut self, level: Level) {
        self.pin.set_level(level).unwrap()
    }
}
