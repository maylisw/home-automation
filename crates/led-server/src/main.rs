use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
};

use anyhow::{bail, Result};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::prelude::Peripherals;
use log::info;

use rgb_led::{RGB8, WS2812RMT};
use wifi::wifi;

#[toml_cfg::toml_config]
pub struct Config {
    #[default("Wokwi-GUEST")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
}

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;

    info!("Hello, world!");

    // Start the LED off yellow
    let mut led = WS2812RMT::new(peripherals.pins.gpio8, peripherals.rmt.channel0)?;
    led.set_pixel(RGB8::new(50, 50, 0))?;

    let app_config = CONFIG;

    // Connect to the Wi-Fi network
    let _wifi = match wifi(
        app_config.wifi_ssid,
        app_config.wifi_psk,
        peripherals.modem,
        sysloop,
    ) {
        Ok(inner) => {
            //set green when connected to the wifi
            led.set_pixel(RGB8::new(0x0, 0xff, 0x0))?;
            inner
        }
        Err(err) => {
            // Red!
            led.set_pixel(RGB8::new(0xff, 0, 0))?;
            bail!("Could not connect to Wi-Fi network: {:?}", err)
        }
    };

    // TCP server
    let ip = _wifi.sta_netif().get_ip_info()?.ip;
    let port = 1080;
    let listener = TcpListener::bind(format!("{ip}:{port}"))?;
    info!("Listening at {ip}:{port}");

    loop {
        let (mut stream, addr) = listener.accept()?;
        info!("Connected to {:#?}", addr);
        stream.write(b"Welcome!\n")?;

        let mut reader = BufReader::new(stream);
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
                "red" => RGB8::new(0xff, 0x0, 0x0),
                "orange" => RGB8::new(0xff, 0x50, 0x0),
                "yellow" => RGB8::new(0xff, 0xff, 0x0),
                "green" => RGB8::new(0x0, 0xff, 0x0),
                "blue" => RGB8::new(0x0, 0x0, 0xff),
                "purple" => RGB8::new(0xff, 0x0, 0xff),
                "violet" => RGB8::new(200, 0, 255),
                "pink" => RGB8::new(199, 21, 133),
                "white" => RGB8::new(0xff, 0xff, 0xff),
                _ => match parse_rgb(&buf.trim()) {
                    Ok(res) => res,
                    Err(_) => {
                        info!("{} is not a valid color. Try sending three comma seperated base10 values.", buf.trim());
                        continue;
                    }
                },
            };
            led.set_pixel(color)?;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn parse_rgb(buf: &str) -> Result<RGB8> {
    let mut val = vec![];
    for i in buf.split(',') {
        info!("i is {i}");
        val.push(i.parse::<u8>()?);
    }
    return Ok(RGB8::new(val[0], val[1], val[2]));
}
