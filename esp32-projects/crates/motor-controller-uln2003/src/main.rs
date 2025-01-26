use anyhow::{Ok, Result};
use esp_idf_svc::hal::{
    delay::Delay,
    gpio::{OutputPin, PinDriver},
    prelude::Peripherals,
};
use log::info;

use motor_controller_uln2003::{Direction, StepperMotor, ULN2003};

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    let mut motor = ULN2003::new(
        PinDriver::output(peripherals.pins.gpio23.downgrade_output())?,
        PinDriver::output(peripherals.pins.gpio22.downgrade_output())?,
        PinDriver::output(peripherals.pins.gpio21.downgrade_output())?,
        PinDriver::output(peripherals.pins.gpio20.downgrade_output())?,
        Some(Delay::new(10000)), /* 10 ms */
    );
    info!("Motor controller initialized");

    let cycles_per_rev = 2048;
    let revs = 1000;
    info!("Stepping for {revs} steps at 1 ms delay");
    motor.step_for(cycles_per_rev * revs, 2).unwrap();

    info!("sleeping for 1 second");
    std::thread::sleep(std::time::Duration::from_secs(1));

    info!("Reversing for {revs} steps at 1 ms delay");
    motor.set_direction(Direction::Reverse);
    motor.step_for(cycles_per_rev * revs, 2).unwrap();

    Ok(())
}
