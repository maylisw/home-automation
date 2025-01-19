use anyhow::Result;
use core::time::Duration;
use esp_idf_svc::hal::{
    gpio::OutputPin,
    peripheral::Peripheral,
    rmt::{config::TransmitConfig, FixedLengthSignal, PinState, Pulse, RmtChannel, TxRmtDriver},
};

pub use rgb::RGB8;

pub struct WS2812RMT<'a> {
    tx_rtm_driver: TxRmtDriver<'a>,
}

#[derive(Debug, Clone, Copy)]
pub enum Color {
    Red,
    Green,
    Blue,
    Yellow,
    Cyan,
    Magenta,
    White,
    Off,
    Orange,
    Purple,
    Pink,
    Custom(u8, u8, u8),
}

impl From<Color> for RGB8 {
    fn from(color: Color) -> Self {
        match color {
            Color::Red => Self::new(0xff, 0x0, 0x0),
            Color::Green => Self::new(0x0, 0xff, 0x0),
            Color::Blue => Self::new(0x0, 0x0, 0xff),
            Color::Yellow => Self::new(0xff, 0xff, 0x0),
            Color::Cyan => Self::new(0x0, 0xff, 0xff),
            Color::Magenta => Self::new(0xff, 0x0, 0xff),
            Color::White => Self::new(0xff, 0xff, 0xff),
            Color::Off => Self::new(0x0, 0x0, 0x0),
            Color::Orange => Self::new(0xff, 0x32, 0x0),
            Color::Purple => Self::new(0xc8, 0x0, 0xff),
            Color::Pink => Self::new(0xc7, 0x15, 0x85),
            Color::Custom(r, g, b) => Self::new(r, g, b),
        }
    }
}

impl<'d> WS2812RMT<'d> {
    // Rust ESP Board gpio2,  ESP32-C3-DevKitC-02 gpio8
    pub fn new(
        led: impl Peripheral<P = impl OutputPin> + 'd,
        channel: impl Peripheral<P = impl RmtChannel> + 'd,
    ) -> Result<Self> {
        let config = TransmitConfig::new().clock_divider(2);
        let tx = TxRmtDriver::new(channel, led, &config)?;
        Ok(Self { tx_rtm_driver: tx })
    }

    pub fn set_pixel(&mut self, color: Color) -> Result<()> {
        let rgb = RGB8::from(color);
        let color: u32 = (u32::from(rgb.g) << 16) | (u32::from(rgb.r) << 8) | u32::from(rgb.b);
        let ticks_hz = self.tx_rtm_driver.counter_clock()?;

        let tick_high_0 = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(350))?;
        let tick_low_0 = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(800))?;

        let tick_high_1 = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(700))?;
        let tick_low_1 = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(600))?;

        let mut signal = FixedLengthSignal::<24>::new();
        for i in (0..24).rev() {
            let bit_on = ((1 << i) & color) != 0;

            let (high_pulse, low_pulse) = if bit_on {
                (tick_high_1, tick_low_1)
            } else {
                (tick_high_0, tick_low_0)
            };
            #[allow(clippy::cast_sign_loss)]
            signal.set(23 - i as usize, &(high_pulse, low_pulse))?;
        }
        self.tx_rtm_driver.start_blocking(&signal)?;

        Ok(())
    }
}

const fn ns(nanos: u64) -> Duration {
    Duration::from_nanos(nanos)
}
