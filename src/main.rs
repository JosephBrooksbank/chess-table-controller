use std::borrow::Borrow;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use esp_idf_hal::gpio::{self, Output, OutputPin, Pin, PinDriver};
use esp_idf_hal::io::Write;
use esp_idf_hal::modem::{Modem, WifiModemPeripheral};
use esp_idf_hal::peripheral::{Peripheral, PeripheralRef};
use esp_idf_hal::peripherals::{self, Peripherals};
use esp_idf_svc::eventloop::{EspEventLoop, EspSystemEventLoop};
use esp_idf_svc::http::server::{Configuration, EspHttpServer};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{self, AccessPointConfiguration, BlockingWifi, EspWifi};
use esp_idf_sys::EspError;
use log::info;

const SSID: &str = "generic network name";
const PASSWORD: &str = "MirrorWindowWall";
const CHANNEL: u8 = 11;
// need lots of stack to parse json
const STACK_SIZE: usize = 10240;

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

fn create_server(peripherals: Peripherals) -> anyhow::Result<EspHttpServer<'static>> {
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    let wifi_configuration = wifi::Configuration::Client(wifi::ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        bssid: None,
        auth_method: wifi::AuthMethod::WPA2Personal,
        password: PASSWORD.try_into().unwrap(),
        channel: None,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;
    wifi.start()?;
    info!("Wifi Started!");
    wifi.connect()?;
    info!("Wifi Connected!");
    wifi.wait_netif_up()?;

    info!(
        "Connected to Wi-Fi with info `{:?}`",
        wifi.wifi().sta_netif().get_ip_info()?
    );

    let server_configuration = esp_idf_svc::http::server::Configuration {
        stack_size: STACK_SIZE,
        ..Default::default()
    };

    // keep wifi running FOREVER.
    // if we ever want to access or stop wifi later, don't call this.
    core::mem::forget(wifi);

    Ok(EspHttpServer::new(&server_configuration)?)
}

fn main() -> anyhow::Result<()> {
    esp_idf_hal::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    // take control of peripherals
    let peripherals = Peripherals::take()?;
    let mut server = create_server(peripherals)?;

    server.fn_handler("/", esp_idf_svc::http::Method::Get, |req| {
        req.into_ok_response()?.write_all("TEST".as_bytes())
    })?;

    // let mut stepper = StepperMotor::new(peripherals.pins.gpio27, peripherals.pins.gpio14);
    // stepper.set_direction(StepperDirection::Clockwise)?;
    // let mut led = PinDriver::output(peripherals.pins.gpio2)?;
    // let mut button = PinDriver::input(peripherals.pins.gpio26)?;
    // button.set_pull(gpio::Pull::Down)?;

    let m = Arc::new(Mutex::new(true));

    let m2 = m.clone();
    thread::spawn(move || loop {
        let on;
        {
            let mutex_ref = m2.lock().unwrap();
            on = *mutex_ref;
        }
        // led.set_level(match on {
        //     true => gpio::Level::High,
        //     false => gpio::Level::Low,
        // })
        // .unwrap();

        thread::sleep(Duration::from_millis(10));
    });

    loop {
        // if button.is_high() {
        //     let mut mutex_ref = m.lock().unwrap();
        //     *mutex_ref = true;
        //     println!("Sending Pulse to Stepper!");
        // } else {
        //     let mut mutex_ref = m.lock().unwrap();
        //     *mutex_ref = false;
        // }

        thread::sleep(Duration::from_millis(10));
    }
}
