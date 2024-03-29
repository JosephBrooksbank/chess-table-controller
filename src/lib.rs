pub mod gpio;
pub mod config {
    use esp_idf_hal::peripherals::Peripherals;
    use esp_idf_sys::EspError;

    pub fn set_up_esp() -> Result<Peripherals, EspError> {
        esp_idf_hal::sys::link_patches();
        esp_idf_svc::log::EspLogger::initialize_default();
        // take control of peripherals
        Peripherals::take()
    }
}

pub mod web;

pub mod stepper_web_control;
