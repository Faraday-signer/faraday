//! Faraday ESP32-S3-Touch-LCD-2 ("touch2") — air-gapped Solana signer.
//!
//! Board binary for the Waveshare ESP32-S3-Touch-LCD-2: a 240×320 ST7789T3
//! panel, CST816D touch, and an OV2640/OV5640 camera. This board has no battery
//! hardware. It owns the board-specific peripheral init, then hands the concrete
//! display / touch drivers to the shared `esp32_common::run` loop.

use esp_idf_hal::gpio::{PinDriver, Pull};
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::ledc::{config::TimerConfig, LedcDriver, LedcTimerDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::spi::{
    config::{Config as SpiConfig, DriverConfig},
    Dma, SpiDeviceDriver, SpiDriver,
};
use esp_idf_hal::units::FromValueType;
use esp_idf_svc::log::EspLogger;
use faraday_core::ui::Theme;

mod display;
mod touch;

fn main() {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();
    log::info!("Faraday ESP32-S3-Touch-LCD-2 v0.1.0");

    let peripherals = Peripherals::take().expect("failed to take peripherals");

    // SPI bus: SCLK=39, MOSI=38, MISO=40 (shared with SD; unused for the
    // write-only display but reserved to match the board wiring).
    let spi_driver = SpiDriver::new(
        peripherals.spi2,
        peripherals.pins.gpio39,
        peripherals.pins.gpio38,
        Some(peripherals.pins.gpio40),
        // DMA lets the CPU yield to FreeRTOS (and the watchdog idle task)
        // during each SPI transfer instead of busy-polling.  Size matches
        // the chunk size used in flush() — just under the 32 767-byte
        // ESP32-S3 hardware limit per transaction.
        &DriverConfig::new().dma(Dma::Auto(32764)),
    )
    .expect("SPI driver init failed");

    // No hardware CS — the driver manages CS manually (see display.rs).
    let spi_config = SpiConfig::new().baudrate(40.MHz().into());
    let spi_device = SpiDeviceDriver::new(spi_driver, None::<esp_idf_hal::gpio::Gpio45>, &spi_config)
        .expect("SPI device init failed");

    // GPIO 42 (DC) is JTAG MTMS — PinDriver alone leaves it attached to the
    // JTAG peripheral with its output driver disabled, so it can never go high
    // and every command parameter / pixel byte is silently dropped. Reset it to
    // plain GPIO function first.
    unsafe {
        esp_idf_svc::sys::gpio_reset_pin(42);
    }

    let cs = PinDriver::output(peripherals.pins.gpio45).expect("CS pin init failed");
    let dc = PinDriver::output(peripherals.pins.gpio42).expect("DC pin init failed");
    let ledc_timer = LedcTimerDriver::new(
        peripherals.ledc.timer0,
        &TimerConfig::default().frequency(1.kHz().into()),
    )
    .expect("LEDC timer init failed");
    let bl = LedcDriver::new(peripherals.ledc.channel0, ledc_timer, peripherals.pins.gpio1)
        .expect("LEDC BL init failed");

    let display = display::Display::new(spi_device, cs, dc, bl);

    // I2C touch: SDA=48, SCL=47, INT=46.
    let i2c_config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio48,
        peripherals.pins.gpio47,
        &i2c_config,
    )
    .expect("I2C init failed");

    let touch_int =
        PinDriver::input(peripherals.pins.gpio46, Pull::Up).expect("INT pin init failed");
    let touch = touch::Touch::new(i2c, touch_int);

    // BOOT button (GPIO0) as a soft power button — long-press → deep sleep.
    let power_btn = esp32_common::power::PowerButton::new(peripherals.pins.gpio0);

    // This board has no battery hardware, so there is no gauge and the GUI draws
    // no battery icon. Other ESP32-S3 boards pass a real `BoardBattery` here.
    esp32_common::run(
        display,
        touch,
        power_btn,
        None::<esp32_common::NoBattery>,
        Theme::faraday_320(),
    );
}
