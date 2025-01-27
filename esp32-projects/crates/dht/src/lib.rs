use core::fmt;
use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin, PinState},
};

// === Reading ===

/// A sensor reading
#[derive(Debug, Clone, Copy)]
pub struct Reading {
    humidity: f32,
    temperature: f32,
}

impl Reading {
    /// Returns the ambient humidity, as a percentage value from 0.0 to 100.0
    pub const fn humidity(&self) -> f32 {
        self.humidity
    }

    /// Returns the ambient temperature, in degrees Celsius
    pub const fn temperature(&self) -> f32 {
        self.temperature
    }

    pub fn fahrenheit(&self) -> f32 {
        self.temperature.mul_add(1.8, 32.0)
    }
}

impl fmt::Display for Reading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Temperature: {} C ({} F), Humidity {}%",
            self.temperature,
            self.fahrenheit(),
            self.humidity
        )
    }
}

// === Pulses ===

#[derive(Copy, Clone, Debug)]
struct Pulse {
    lo: u8,
    hi: u8,
}

// === DhtError ===

/// A type detailing various errors the DHT sensor can return
#[derive(Debug, Clone)]
pub enum DhtError<HE> {
    /// The DHT sensor was not found on the specified GPIO
    NotPresent,
    /// The checksum provided in the DHT sensor data did not match the checksum of the data itself (expected, calculated)
    ChecksumMismatch(u8, u8),
    /// The seemingly-valid data has impossible values (e.g. a humidity value less than 0 or greater than 100)
    InvalidData,
    /// The read timed out
    Timeout,
    /// Received a low-level error from the HAL while reading or writing to pins
    PinError(HE),
}

impl<HE> From<HE> for DhtError<HE> {
    fn from(error: HE) -> Self {
        Self::PinError(error)
    }
}

impl<HE: fmt::Debug> fmt::Display for DhtError<HE> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use DhtError::{ChecksumMismatch, InvalidData, NotPresent, PinError, Timeout};
        match self {
            NotPresent => write!(f, "DHT device not found"),
            ChecksumMismatch(expected, calculated) => write!(
                f,
                "Data read was corrupt (expected checksum {expected}, calculated {calculated})",
            ),
            InvalidData => f.write_str("Received data is out of range"),
            Timeout => f.write_str("Timed out waiting for a read"),
            PinError(err) => write!(f, "HAL pin error: {:?}", err),
        }
    }
}

impl<HE: fmt::Debug> std::error::Error for DhtError<HE> {}

// === InterruptControl ===

/// Trait that allows us to disable interrupts when reading from the sensor
pub trait InterruptControl {
    fn enable_interrupts(&mut self);
    fn disable_interrupts(&mut self);
}

/// A dummy implementation of `InterruptControl` that does nothing
pub struct NoopInterruptControl;

impl InterruptControl for NoopInterruptControl {
    fn enable_interrupts(&mut self) {}
    fn disable_interrupts(&mut self) {}
}

// === DhtSensor ===

/// A trait for reading data from the sensor
///
/// This level of indirection is useful so you can write generic code that
/// does not assume whether a DHT11 or DHT22 sensor is being used.
pub trait DhtSensor<HE> {
    /// Reads data from the sensor and returns a `Reading`
    fn read(&mut self) -> Result<Reading, DhtError<HE>>;
}

pub struct Dht<
    HE,
    ID: InterruptControl,
    D: DelayNs,
    P: InputPin<Error = HE> + OutputPin<Error = HE>,
> {
    interrupt_disabler: ID,
    delay: D,
    pin: P,
}

impl<HE, ID: InterruptControl, D: DelayNs, P: InputPin<Error = HE> + OutputPin<Error = HE>>
    Dht<HE, ID, D, P>
{
    const fn new(interrupt_disabler: ID, delay: D, pin: P) -> Self {
        Self {
            interrupt_disabler,
            delay,
            pin,
        }
    }

    fn read(&mut self, parse_data: fn(&[u8]) -> (f32, f32)) -> Result<Reading, DhtError<HE>> {
        self.interrupt_disabler.disable_interrupts();
        let res = self.read_uninterruptible(parse_data);
        self.interrupt_disabler.enable_interrupts();
        res
    }

    fn read_uninterruptible(
        &mut self,
        parse_data: fn(&[u8]) -> (f32, f32),
    ) -> Result<Reading, DhtError<HE>> {
        // START: start sequence
        self.pin.set_low()?;
        self.delay.delay_ms(18);

        self.pin.set_high()?;
        self.delay.delay_us(40);

        // Wait for DHT to signal data is ready (~80us low followed by ~80us high)
        self.wait_for_level(PinState::High, DhtError::NotPresent)?;
        self.wait_for_level(PinState::Low, DhtError::NotPresent)?;
        // END: start sequence

        // START: reading
        let mut pulses = [Pulse { lo: 0, hi: 0 }; 40];
        for pulse in &mut pulses[..] {
            // waiting to go high tells us how long we were low
            pulse.lo = self.wait_for_level(PinState::High, DhtError::Timeout)?;
            pulse.hi = self.wait_for_level(PinState::Low, DhtError::Timeout)?;
        }
        // END: reading

        let mut bytes: [u8; 5] = [0; 5];

        for (i, pulse) in pulses.chunks(8).enumerate() {
            let byte = &mut bytes[i];

            // if the high pulse is longer than the leading low pulse it's a 1
            for Pulse { lo, hi } in pulse {
                *byte <<= 1;
                if hi > lo {
                    *byte |= 1;
                }
            }
        }

        // START: checksum
        let expected = bytes[4];
        let actual = (bytes[0..=3]
            .iter()
            .fold(0u16, |acc, next| acc + u16::from(*next))
            & 0xff) as u8;
        if expected != actual {
            return Err(DhtError::ChecksumMismatch(actual, expected));
        }
        // END: checksum

        let (humidity, temperature) = parse_data(&bytes);
        if (0.0..=100.0).contains(&humidity) {
            Ok(Reading {
                humidity,
                temperature,
            })
        } else {
            Err(DhtError::InvalidData)
        }
    }

    #[inline(always)]
    fn wait_for_level(
        &mut self,
        level: PinState,
        on_timeout: DhtError<HE>,
    ) -> Result<u8, DhtError<HE>> {
        for elapsed in 0..=u8::MAX {
            let is_ready = match level {
                PinState::High => self.pin.is_high()?,
                PinState::Low => self.pin.is_low()?,
            };

            if is_ready {
                return Ok(elapsed);
            }
            self.delay.delay_us(1);
        }
        Err(on_timeout)
    }
}

// === Dht11 ===

/// A DHT11 sensor
pub struct Dht11<
    HE,
    ID: InterruptControl,
    D: DelayNs,
    P: InputPin<Error = HE> + OutputPin<Error = HE>,
> {
    dht: Dht<HE, ID, D, P>,
}

impl<HE, ID: InterruptControl, D: DelayNs, P: InputPin<Error = HE> + OutputPin<Error = HE>>
    Dht11<HE, ID, D, P>
{
    pub const fn new(interrupt_disabler: ID, delay: D, pin: P) -> Self {
        Self {
            dht: Dht::new(interrupt_disabler, delay, pin),
        }
    }

    fn parse_data(buf: &[u8]) -> (f32, f32) {
        (f32::from(buf[0]), f32::from(buf[2]))
    }
}

impl<HE, ID: InterruptControl, D: DelayNs, P: InputPin<Error = HE> + OutputPin<Error = HE>>
    DhtSensor<HE> for Dht11<HE, ID, D, P>
{
    fn read(&mut self) -> Result<Reading, DhtError<HE>> {
        self.dht.read(Self::parse_data)
    }
}

// === Dht22 ===

/// A DHT22 sensor
pub struct Dht22<
    HE,
    ID: InterruptControl,
    D: DelayNs,
    P: InputPin<Error = HE> + OutputPin<Error = HE>,
> {
    dht: Dht<HE, ID, D, P>,
}

impl<HE, ID: InterruptControl, D: DelayNs, P: InputPin<Error = HE> + OutputPin<Error = HE>>
    Dht22<HE, ID, D, P>
{
    pub const fn new(interrupt_disabler: ID, delay: D, pin: P) -> Self {
        Self {
            dht: Dht::new(interrupt_disabler, delay, pin),
        }
    }

    fn parse_data(buf: &[u8]) -> (f32, f32) {
        let humidity = f32::from((u16::from(buf[0]) << 8) | u16::from(buf[1])) / 10.0;
        let mut temperature = f32::from((u16::from(buf[2] & 0x7f) << 8) | u16::from(buf[3])) / 10.0;
        if buf[2] & 0x80 != 0 {
            temperature = -temperature;
        }
        (humidity, temperature)
    }
}

impl<HE, ID: InterruptControl, D: DelayNs, P: InputPin<Error = HE> + OutputPin<Error = HE>>
    DhtSensor<HE> for Dht22<HE, ID, D, P>
{
    fn read(&mut self) -> Result<Reading, DhtError<HE>> {
        self.dht.read(Self::parse_data)
    }
}
