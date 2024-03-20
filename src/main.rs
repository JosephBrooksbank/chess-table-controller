use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{self, PinDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::task::block_on;

fn main() -> anyhow::Result<()> {
    esp_idf_hal::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    // take control of peripherals
    let peripherals = Peripherals::take()?;

    let mut step = PinDriver::output(peripherals.pins.gpio27)?;
    let mut led = PinDriver::output(peripherals.pins.gpio2)?;
    let mut button = PinDriver::input(peripherals.pins.gpio26)?;
    button.set_pull(gpio::Pull::Up)?;

    block_on(async {
        loop {
            button.wait_for_low().await.unwrap();
            led.set_high().unwrap();

            button.wait_for_high().await.unwrap();
            led.set_low().unwrap();
        }
    })
}
