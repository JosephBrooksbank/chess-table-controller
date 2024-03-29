use std::sync::mpsc::Sender;

use esp_idf_hal::modem::Modem;
use esp_idf_hal::peripheral::PeripheralRef;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::http::{self, server::EspHttpServer};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use log::info;

use embedded_svc::{
    http::{Headers, Method},
    io::{Read, Write},
};
use serde::Deserialize;

pub fn create_server(
    modem: PeripheralRef<Modem>,
    ssid: &str,
    password: &str,
    stack_size: usize,
) -> anyhow::Result<EspHttpServer<'static>> {
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(EspWifi::new(modem, sys_loop.clone(), Some(nvs))?, sys_loop)?;

    let wifi_configuration = wifi::Configuration::Client(wifi::ClientConfiguration {
        ssid: ssid.try_into().unwrap(),
        bssid: None,
        auth_method: wifi::AuthMethod::WPA2Personal,
        password: password.try_into().unwrap(),
        channel: None,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;
    wifi.start()?;
    wifi.connect()?;
    info!("Wifi Connected!");
    wifi.wait_netif_up()?;

    info!(
        "Connected to Wi-Fi with info `{:?}`",
        wifi.wifi().sta_netif().get_ip_info()?
    );

    let server_configuration = esp_idf_svc::http::server::Configuration {
        stack_size,
        ..Default::default()
    };

    // keep wifi running FOREVER.
    // if we ever want to access or stop wifi later, don't call this.
    core::mem::forget(wifi);

    Ok(EspHttpServer::new(&server_configuration)?)
}
