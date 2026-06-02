//! Faraday ESP32-S3 — air-gapped Solana signer.

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{PinDriver, Pull};
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::spi::{
    config::{Config as SpiConfig, DriverConfig},
    Dma, SpiDeviceDriver, SpiDriver,
};
use esp_idf_hal::units::FromValueType;
use esp_idf_svc::log::EspLogger;
use faraday_core::gui::app::{App, InputEvent};
use faraday_core::gui::screens;
use faraday_core::ui::Theme;
use faraday_core::ui::widgets::list::visible_start;
use touch::TouchEvent;

mod display;
mod touch;


fn main() {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();
    log::info!("Faraday ESP32-S3 v0.1.0");

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
    let bl = PinDriver::output(peripherals.pins.gpio1).expect("BL pin init failed");

    let mut display = display::Display::new(spi_device, cs, dc, bl);

    // I2C touch: SDA=48, SCL=47, INT=46.
    let i2c_config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio48,
        peripherals.pins.gpio47,
        &i2c_config,
    )
    .expect("I2C init failed");

    let touch_int = PinDriver::input(peripherals.pins.gpio46, Pull::Up).expect("INT pin init failed");
    let mut touch = touch::Touch::new(i2c, touch_int);

    let mut app = App::new(Theme::faraday_320());

    // Splash screen
    let _ = app.draw(&mut display);
    display.flush();
    let splash_start = std::time::Instant::now();
    while splash_start.elapsed() < std::time::Duration::from_secs(2) {
        if touch.poll().is_some() {
            break;
        }
        FreeRtos::delay_ms(33);
    }
    app.enter_main_menu();
    app.last_activity = std::time::Instant::now();

    // Delay (ms) between tapping a list row and firing Confirm — long enough
    // for one display frame to render the highlight before the transition.
    const TAP_CONFIRM_DELAY_MS: u64 = 100;

    // Footer zone: bottom strip of the 320px display, divided into three
    // equal thirds.  Applied only when the tap doesn't fall on a grid or
    // list area, so char-grid action rows are never shadowed.
    const FOOTER_Y: u16 = 288;
    const FOOTER_THIRD: u16 = 80;

    let mut last_draw = std::time::Instant::now();
    let mut pending_tap_confirm: Option<std::time::Instant> = None;
    loop {
        // Touch checked at 5 ms resolution to catch short INT pulses reliably.
        match touch.poll() {
            Some(TouchEvent::Input(event)) => {
                // Any directional gesture or footer tap cancels a pending
                // tap-confirm so the two don't stack.
                pending_tap_confirm = None;
                app.handle_input(event);
            }
            Some(TouchEvent::BodyTap { x, y }) => {
                if app.tap_char_grid(x, y) {
                    // Char grid (passphrase / message entry): the action row
                    // (SPC CAPS DEL DONE) lives at the bottom of the grid and
                    // physically overlaps the footer zone — check this first so
                    // action-row taps are never shadowed by the footer mapping.
                    pending_tap_confirm = None;
                    app.handle_input(InputEvent::Confirm);
                } else if app.tap_word_grid(x, y) {
                    // Word-entry alphabet grid: same reasoning as char grid.
                    pending_tap_confirm = None;
                    app.handle_input(InputEvent::Confirm);
                } else if y >= FOOTER_Y {
                    // Footer zone (Back / Secondary / Confirm thirds).
                    // Only reached when neither grid type claimed the tap.
                    pending_tap_confirm = None;
                    let event = if x < FOOTER_THIRD {
                        InputEvent::Back
                    } else if x < FOOTER_THIRD * 2 {
                        InputEvent::Secondary
                    } else {
                        InputEvent::Confirm
                    };
                    app.handle_input(event);
                } else if let Some(layout) = app.tap_layout() {
                    // List screen: move selection then fire Confirm after a
                    // short delay so the highlight is visible for one frame.
                    if y >= layout.list_top {
                        let body_y = (y - layout.list_top) as usize;
                        let list_h = (app.theme.height as u16 - layout.list_top) as usize;
                        let slot = if list_h > 0 {
                            (body_y * layout.max_visible / list_h).min(layout.max_visible - 1)
                        } else {
                            0
                        };
                        let start = visible_start(
                            layout.total_items,
                            layout.max_visible,
                            layout.current_selected,
                        );
                        let row = (start + slot).min(layout.total_items.saturating_sub(1));
                        app.set_selected(row);
                    }
                    pending_tap_confirm = Some(std::time::Instant::now());
                } else {
                    // Read-only / advance-only screen (word display, card
                    // confirm, QR view, about, errors…): tap anywhere fires
                    // Confirm so the user can page forward.
                    pending_tap_confirm = None;
                    app.handle_input(InputEvent::Confirm);
                }
            }
            None => {}
        }

        if let Some(t) = pending_tap_confirm {
            if t.elapsed().as_millis() >= TAP_CONFIRM_DELAY_MS as u128 {
                pending_tap_confirm = None;
                app.handle_input(InputEvent::Confirm);
            }
        }

        app.tick();

        // Display refreshed at ~30 Hz independently of the touch poll rate.
        if last_draw.elapsed().as_millis() >= 33 {
            if app.is_blanked() {
                let elapsed_ms = app.splash_anim_start.elapsed().as_millis() as u64;
                let _ = screens::draw_splash(&mut display, &app.theme, elapsed_ms);
            } else {
                let _ = app.draw(&mut display);
            }
            display.flush();
            last_draw = std::time::Instant::now();
        }

        FreeRtos::delay_ms(5);
    }
}
