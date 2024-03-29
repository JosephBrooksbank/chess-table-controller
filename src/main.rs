use std::process;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Sender, TryRecvError};
use std::thread;
use std::time::Duration;
use log::info;

use ctc::config;

use ctc::gpio::{Peripheral, IOPin};
use ctc::gpio::simple::{PinDefinitions, Led, Button, Level};
use ctc::gpio::stepper;


const SSID: &str = "generic network name";
const PASSWORD: &str = "MirrorWindowWall";
// need lots of stack to parse json
const STACK_SIZE: usize = 10240;


fn main() -> anyhow::Result<()> {
    let peripherals = config::set_up_esp()?;
    let mut server = config::create_server(
        peripherals.modem.into_ref(),
        SSID,
        PASSWORD,
        STACK_SIZE,
    )?;

    let (tx, rx) = mpsc::channel();
    config::add_web_handlers(&mut server, tx);

    let pins = PinDefinitions::build(peripherals.pins);

    let mut stepper = stepper::StepperMotor::new(pins.stepper, pins.stepper_dir);
    let mut led = Led::new(pins.onboard_led);
    let mut button = Button::new(pins.button.downgrade());

    let m = Arc::new(Mutex::new(true));

    let m2 = m.clone();
    thread::spawn(move || loop {
        let on;
        {
            let mutex_ref = m2.lock().unwrap();
            on = *mutex_ref;
        }
        led.set_level(match on {
            true => Level::High,
            false => Level::Low,
        });

        thread::sleep(Duration::from_millis(10));
    });

    loop {
        match rx.try_recv() {
            // Ok(val) => flash_light(&mut led),
            Ok(val) => {
                {
                    println!("setting on in mutex");
                    let mut m = m.lock().unwrap();
                    *m = val;
                }
                thread::sleep(Duration::from_millis(500));
                let mut m = m.lock().unwrap();
                println!("setting off in mutex");
                *m = false;
            }
            Err(e) => {
                if let TryRecvError::Disconnected = e {
                    info!("tx Disconnected!");
                    process::exit(1);
                }
            }
        };

        thread::sleep(Duration::from_millis(10));
    }
}
