use esp_idf_svc::hal::{
    delay::Delay,
    gpio::{IOPin, PinDriver},
    prelude::Peripherals,
};
use log::info;

use dht::{Dht11, DhtSensor, NoopInterruptControl};

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let pin = match PinDriver::input_output_od(peripherals.pins.gpio7.downgrade()) {
        Ok(pin) => pin,
        Err(err) => panic!("error setting gpio7: {:?}", err),
    };

    let mut dht = Dht11::new(NoopInterruptControl, Delay::new_default(), pin);
    info!("DHT11 setup on pin 7");

    loop {
        match dht.read() {
            Ok(res) => {
                info!("DHT11 read: {res}")
            }
            Err(err) => {
                info!("error during read: {}", err);
            }
        };
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}
