#![no_std]
#![feature(stmt_expr_attributes)]

pub mod dprint;

use display_interface_spi::SPIInterfaceNoCS;
use embedded_hal::digital::v2::OutputPin;
use esp32;
use esp32_hal::{
    clock_control::{ClockControl, XTAL_FREQUENCY_AUTO},
    dport::Split,
    gpio::{Gpio18, Gpio19, Gpio27, Gpio5, Output, PushPull},
    i2c,
    prelude::*,
    serial::{config::Config, Pins, Serial},
    spi::{self, SPI},
    target,
    timer::Timer,
};

use axp20x::AXP20X;
use st7789::{Orientation, ST7789};

pub struct NoPin {}

impl Default for NoPin {
    fn default() -> Self {
        Self {}
    }
}

#[derive(Debug)]
pub enum Infallible {}

impl OutputPin for NoPin {
    /// Error type
    type Error = Infallible;

    /// Drives the pin low
    ///
    /// *NOTE* the actual electrical state of the pin may not actually be low, e.g. due to external
    /// electrical sources
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Drives the pin high
    ///
    /// *NOTE* the actual electrical state of the pin may not actually be high, e.g. due to external
    /// electrical sources
    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub enum TWatchError {
    DisplayError,
    PMUError
}

pub fn sleep(delay: MicroSeconds) {
    esp32_hal::clock_control::sleep(delay);
}

pub struct TWatch {
    pub pmu: AXP20X<esp32_hal::i2c::I2C<esp32::I2C0>>,
    pub display: ST7789<
        SPIInterfaceNoCS<
            esp32_hal::spi::SPI<
                esp32::SPI2,
                Gpio18<Output<PushPull>>,
                Gpio19<Output<PushPull>>,
                Gpio19<Output<PushPull>>,
                Gpio5<Output<PushPull>>,
            >,
            Gpio27<Output<PushPull>>,
        >,
        NoPin,
    >,
}

impl TWatch {
    pub fn new(dp: target::Peripherals) -> Self {
        dprintln!("Creating new Twatch");

        let (mut dport, dport_clock_control) = dp.DPORT.split();

        let clkcntrl = ClockControl::new(
            dp.RTCCNTL,
            dp.APB_CTRL,
            dport_clock_control,
            XTAL_FREQUENCY_AUTO,
        )
        .unwrap();

        let (clkcntrl_config, mut watchdog) = clkcntrl.freeze().unwrap();
    
        watchdog.disable();

        let (_, _, _, mut watchdog0) = Timer::new(dp.TIMG0, clkcntrl_config);
        let (_, _, _, mut watchdog1) = Timer::new(dp.TIMG1, clkcntrl_config);
        watchdog0.disable();
        watchdog1.disable();

        let pins = dp.GPIO.split();

        // Use UART1 as example: will cause dprintln statements not to be printed
        let _serial: Serial<_, _, _> = Serial::new(
            dp.UART1,
            Pins {
                tx: pins.gpio1,
                rx: pins.gpio3,
                cts: None,
                rts: None,
            },
            Config {
                // default configuration is 19200 baud, 8 data bits, 1 stop bit & no parity (8N1)
                baudrate: 115200.Hz(),
                ..Config::default()
            },
            clkcntrl_config,
        )
        .unwrap();

        let mut gpio_backlight = pins.gpio12.into_push_pull_output();
        let sclk = pins.gpio18.into_push_pull_output();
        let sdo = pins.gpio19.into_push_pull_output();
        let cs = pins.gpio5.into_push_pull_output();

        let spi: SPI<
            esp32::SPI2,
            Gpio18<Output<PushPull>>,
            Gpio19<Output<PushPull>>,
            Gpio19<Output<PushPull>>,
            Gpio5<Output<PushPull>>,
        > = SPI::<esp32::SPI2, _, _, _, _>::new(
            dp.SPI2,
            spi::Pins {
                sclk,
                sdo,
                sdi: None,
                cs: Some(cs),
            },
            spi::config::Config {
                baudrate: 80.MHz().into(),
                bit_order: spi::config::BitOrder::MSBFirst,
                data_mode: spi::config::MODE_0,
            },
            clkcntrl_config,
        )
        .unwrap();

        let i2c0 = i2c::I2C::new(
            dp.I2C0,
            i2c::Pins {
                sda: pins.gpio21,
                scl: pins.gpio22,
            },
            400_000,
            &mut dport,
        );
        let mut pmu = axp20x::AXP20X::new(i2c0);
        pmu.init(&mut esp32_hal::delay::Delay::new()).unwrap();

        gpio_backlight.set_low().unwrap();

        let gpio_dc = pins.gpio27.into_push_pull_output();

        let spi_if = SPIInterfaceNoCS::new(spi, gpio_dc);

        // create driver
        let mut display = ST7789::new(spi_if, NoPin::default(), 240, 320);

        // set default orientation
        display.set_orientation(Orientation::Portrait).unwrap();

        gpio_backlight.set_high().unwrap();
        Self { display, pmu }
    }

    pub fn get_battery_percentage(&mut self) -> Result<u8, axp20x::AXPError> {
        dprintln!("Get battery percentage\r");
        match self.pmu.get_battery_percentage() {
            Ok(127) => {
                let voltage: f32 = self.pmu.get_battery_voltage()?;
                let level = ((voltage - 3200.0_f32) * 100_f32) / 1000_f32;
                Ok(level.clamp(0.0, 100.0) as u8)
            }
            Ok(v) => Ok(v),
            Err(e) => Err(e),
        }
    }
}
