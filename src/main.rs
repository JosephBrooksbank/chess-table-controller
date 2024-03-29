use log::info;
use std::process;
use std::sync::mpsc::{Sender, TryRecvError};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use ctc::{config, stepper_web_control, stepper_web_control::StepperControl, web};

use ctc::gpio::simple::{Button, Led, Level};
use ctc::gpio::stepper;
use ctc::gpio::PinDefinitions;
use ctc::gpio::{IOPin, Peripheral};

const SSID: &str = "generic network name";
const PASSWORD: &str = "MirrorWindowWall";
// need lots of stack to parse json
const STACK_SIZE: usize = 10240;

fn main() -> anyhow::Result<()> {
    let peripherals = config::set_up_esp()?;
    let mut server = web::create_server(peripherals.modem.into_ref(), SSID, PASSWORD, STACK_SIZE)?;

    let (tx, rx) = mpsc::channel();
    stepper_web_control::add_stepper_web_control(&mut server, tx, "/");

    let pins = PinDefinitions::build(peripherals.pins);

    let mut stepper = stepper::StepperMotor::new(pins.stepper, pins.stepper_dir);
    let mut led = Led::new(pins.onboard_led);
    let mut button = Button::new(pins.button.downgrade());

    loop {
        match rx.try_recv() {
            Ok(val) => {
                led.turn_on();
                stepper.drive(val.steps, val.pulse_width)?;
                led.turn_off();
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
