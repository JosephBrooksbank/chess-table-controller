pub use esp_idf_hal::gpio::IOPin;
use esp_idf_hal::gpio::{Gpio14, Gpio2, Gpio26, Gpio27, Pins};
pub use esp_idf_hal::peripheral::Peripheral;
pub mod simple;
pub mod stepper;

pub struct PinDefinitions {
    pub stepper: Gpio27,
    pub stepper_dir: Gpio14,
    pub onboard_led: Gpio2,
    pub button: Gpio26,
}

impl PinDefinitions {
    pub fn build(pins: Pins) -> Self {
        Self {
            stepper: pins.gpio27,
            stepper_dir: pins.gpio14,
            onboard_led: pins.gpio2,
            button: pins.gpio26,
        }
    }
}
