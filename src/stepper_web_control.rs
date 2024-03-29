use crate::gpio::stepper::StepperDirection;
use embedded_svc::http::Headers;
use embedded_svc::io::{Read, Write};
use esp_idf_svc::http::server::EspHttpServer;
use serde::Deserialize;
use std::sync::mpsc::Sender;

#[derive(Deserialize)]
pub struct StepperControl {
    pub direction: StepperDirection,
    pub steps: u32,
    pub pulse_width: u32,
}

pub fn add_stepper_web_control(
    server: &mut EspHttpServer,
    tx: Sender<StepperControl>,
    endpoint: &str,
) {
    server
        .fn_handler::<anyhow::Error, _>(
            endpoint,
            esp_idf_svc::http::Method::Post,
            move |mut req| {
                println!("handling post request");

                let len = req.content_len().unwrap_or(0) as usize;

                let mut buf = vec![0; len];
                req.read_exact(&mut buf)?;
                let mut response = req.into_ok_response()?;

                if let Ok(data) = serde_json::from_slice::<StepperControl>(&buf) {
                    response.write_all("Moving motor".as_bytes())?;
                    tx.send(data)?;
                } else {
                    response.write_all("JSON error".as_bytes())?;
                }
                Ok(())
            },
        )
        .unwrap();
}
