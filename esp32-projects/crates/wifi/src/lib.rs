use anyhow::{bail, Result};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::peripheral,
    ipv4::{
        ClientConfiguration as IpClientConfiguration, Configuration as IpConfiguration,
        DHCPClientSettings,
    },
    netif::{EspNetif, NetifConfiguration, NetifStack},
    nvs::EspDefaultNvsPartition,
    wifi::{
        AuthMethod, BlockingWifi, ClientConfiguration as WifiClientConfiguration,
        Configuration as WifiConfiguration, EspWifi, WifiDriver,
    },
};
use log::info;

pub fn wifi(
    ssid: &str,
    pass: &str,
    hostname: Option<&str>,
    modem: impl peripheral::Peripheral<P = esp_idf_svc::hal::modem::Modem> + 'static,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
) -> Result<Box<EspWifi<'static>>> {
    let mut auth_method = AuthMethod::WPA2Personal;
    if ssid.is_empty() {
        bail!("Missing WiFi name")
    }
    if pass.is_empty() {
        auth_method = AuthMethod::None;
        info!("Wifi password is empty");
    }

    let wifi_driver = WifiDriver::new(modem, sysloop.clone(), Some(nvs))?;

    let mut netif_config = NetifConfiguration::wifi_default_client();
    if let Some(hostname) = hostname {
        netif_config.ip_configuration = Some(IpConfiguration::Client(IpClientConfiguration::DHCP(
            DHCPClientSettings {
                hostname: Some(hostname.try_into().unwrap()),
            },
        )));
    }

    let mut esp_wifi = EspWifi::wrap_all(
        wifi_driver,
        EspNetif::new_with_conf(&netif_config)?,
        EspNetif::new(NetifStack::Ap)?,
    )?;

    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop)?;

    info!("Starting wifi...");

    wifi.start()?;

    info!("Scanning...");

    let wifi_channel = wifi.scan()?.into_iter().find(|a| a.ssid == ssid);

    let channel = if let Some(wifi_channel) = wifi_channel {
        info!(
            "Found configured access point {} on channel {}",
            ssid, wifi_channel.channel
        );
        Some(wifi_channel.channel)
    } else {
        info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            ssid
        );
        None
    };

    wifi.set_configuration(&WifiConfiguration::Client(WifiClientConfiguration {
        ssid: ssid
            .try_into()
            .expect("Could not parse SSID into WiFi config"),
        password: pass
            .try_into()
            .expect("Could not parse password into WiFi config"),
        auth_method,
        channel,
        ..Default::default()
    }))?;

    info!("Connecting wifi...");

    wifi.connect()?;

    info!("Waiting for DHCP lease...");

    wifi.wait_netif_up()?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!("Wifi DHCP info: {:?}", ip_info);

    Ok(Box::new(esp_wifi))
}
