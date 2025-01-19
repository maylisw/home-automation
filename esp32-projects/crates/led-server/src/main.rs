use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
};

use anyhow::{bail, Result};
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use log::info;

use rgb_led::{Color, WS2812RMT};
use wifi::wifi;

#[toml_cfg::toml_config]
pub struct Config {
    #[default("Wokwi-GUEST")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
}

// TODO: refactor
const HOSTNAME: Option<&'static str> = Some("espy");

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // Start the LED off yellow
    let mut led = WS2812RMT::new(peripherals.pins.gpio8, peripherals.rmt.channel0)?;
    led.set_pixel(Color::Yellow)?;

    let app_config = CONFIG;

    // Connect to the Wi-Fi network
    let esp_wifi = match wifi(
        app_config.wifi_ssid,
        app_config.wifi_psk,
        HOSTNAME,
        peripherals.modem,
        sysloop,
        nvs,
    ) {
        Ok(inner) => {
            // Green for success
            led.set_pixel(Color::Green)?;
            inner
        }
        Err(err) => {
            // Red for failure
            led.set_pixel(Color::Red)?;
            bail!("Could not connect to Wi-Fi network: {:?}", err)
        }
    };

    // TCP server
    let port = 1080;
    let address = format!("{}:{port}", esp_wifi.sta_netif().get_ip_info()?.ip);

    let listener = TcpListener::bind(&address)?;
    info!("Listening at {address}");

    loop {
        let (mut stream, addr) = listener.accept()?;
        info!("Connected to {:#?}", addr);
        stream.write_all(b"Welcome!\n Please type a color to change the LED. Supported colors are:\n - red\n - orange\n - yellow\n - green\n - cyan\n - blue\n - magenta\n - purple\n - pink\n - white\n - off\n- custom (format is r,g,b)\n")?;

        let mut reader = BufReader::new(&stream);
        let mut buf = String::new();

        while {
            buf.clear();
            match reader.read_line(&mut buf) {
                Ok(bytes) => bytes != 0,
                Err(e) => {
                    info!("error: {e}");
                    false
                }
            }
        } {
            info!("recieved {:#?}", buf);
            let color = match buf.trim() {
                "red" => Color::Red,
                "orange" => Color::Orange,
                "yellow" => Color::Yellow,
                "green" => Color::Green,
                "cyan" => Color::Cyan,
                "blue" => Color::Blue,
                "magenta" => Color::Magenta,
                "purple" => Color::Purple,
                "pink" => Color::Pink,
                "white" => Color::White,
                "off" => Color::Off,
                _ => {
                    if let Ok(res) = parse_rgb(buf.trim()) {
                        res
                    } else {
                        info!("{} is not a valid color. Try sending three comma seperated base10 values.", buf.trim());
                        continue;
                    }
                }
            };
            led.set_pixel(color)?;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn parse_rgb(buf: &str) -> Result<Color> {
    let mut val = vec![];
    for i in buf.split(',') {
        info!("i is {i}");
        val.push(i.parse::<u8>()?);
    }
    Ok(Color::Custom(val[0], val[1], val[2]))
}
