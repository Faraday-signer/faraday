//! Faraday ESP32-S3 — air-gapped Solana signer.

use esp_idf_hal::adc::attenuation::DB_12;
use esp_idf_hal::adc::oneshot::config::{AdcChannelConfig, Calibration};
use esp_idf_hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{PinDriver, Pull};
use esp_idf_hal::ledc::{config::TimerConfig, LedcDriver, LedcTimerDriver};
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
use faraday_core::ui::widgets::FOOTER_H;
use touch::TouchEvent;

mod battery;
mod camera;
mod display;
mod power;
mod qr_decode;
mod touch;


fn main() {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();
    log::info!("Faraday ESP32-S3 v0.1.0");

    // Validate the hardware (mbedtls) BIP39 seed path against a known-answer
    // vector before any wallet can be created or loaded. The host test suite
    // can't exercise this path, so a divergence from the BIP39 standard would
    // otherwise surface only as silently wrong addresses. Fail closed.
    assert!(
        faraday_core::crypto::bip39::seed_derivation_self_test(),
        "BIP39 seed self-test failed — hardware SHA512 path diverges from spec"
    );

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
    ).expect("LEDC timer init failed");
    let bl = LedcDriver::new(peripherals.ledc.channel0, ledc_timer, peripherals.pins.gpio1)
        .expect("LEDC BL init failed");

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

    // BOOT button (GPIO0) as a soft power button — long-press → deep sleep.
    let mut power_btn = power::PowerButton::new(peripherals.pins.gpio0);

    // External battery monitor. Pack voltage is sampled on GPIO5 (net BAT_ADC),
    // tapped off the board's R19/R20 = 200k/100k divider (V_pack = 3 × V_adc),
    // and turned into a charge-level icon (no charging/presence detection is
    // possible on this board — see battery::BatteryMonitor). If the ADC can't
    // init we run without a battery icon.
    let bat_cfg = AdcChannelConfig {
        attenuation: DB_12,
        calibration: Calibration::Curve,
        ..Default::default()
    };
    let mut bat_chan = AdcDriver::new(peripherals.adc1)
        .ok()
        .and_then(|adc| AdcChannelDriver::new(adc, peripherals.pins.gpio5, &bat_cfg).ok());
    let mut bat_monitor = battery::BatteryMonitor::new();
    // Back-date so the first loop iteration samples immediately. `checked_sub`
    // guards against underflow early in boot (the monotonic clock is still near
    // zero), in which case the first sample just lands one interval later.
    let mut last_bat_sample = std::time::Instant::now()
        .checked_sub(std::time::Duration::from_secs(BATTERY_SAMPLE_SECS))
        .unwrap_or_else(std::time::Instant::now);

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
    const TAP_CONFIRM_DELAY_MS: u64 = 40;

    // How often to re-sample the battery. The pack voltage moves slowly, so a
    // couple of seconds keeps the gauge fresh without spinning the ADC.
    const BATTERY_SAMPLE_SECS: u64 = 2;

    // Footer action bar: bottom FOOTER_H strip, divided into three equal
    // thirds — Back (left) / Secondary (middle) / Confirm (right), matching
    // the glyphs drawn by EdgeHints. Applied only when the tap doesn't fall
    // on a grid or list area, which now sit above the bar.
    let footer_h = FOOTER_H as u16;
    let footer_y = app.theme.height as u16 - footer_h;
    let footer_third = app.theme.width as u16 / 3;

    let mut camera: Option<camera::EspCamera> = None;
    let mut last_draw = std::time::Instant::now();
    let mut pending_tap_confirm: Option<std::time::Instant> = None;
    loop {
        // BOOT long-press → power off. Wipe the wallet (zeroizing the seed/keys),
        // show a brief "powering off" frame, then deep-sleep until BOOT is pressed
        // again. Deep-sleep wake is a full power-cycle reset, so `main()` reruns
        // from scratch (first screen, RAM cleared) and the USB-Serial/JTAG comes
        // back flashable. wait_release() keeps GPIO0 high before arming so the
        // held press doesn't immediately re-wake.
        if power_btn.long_pressed() {
            app.wipe_wallet();
            let _ = screens::draw_powering_off(&mut display, &app.theme);
            display.flush();
            FreeRtos::delay_ms(800);
            power_btn.wait_release();
            // Drop the board's standby current before sleeping: backlight off,
            // panel into sleep-in, and the OV2640 powered down + held (it's the
            // ~30 mA hog — left powered, it would keep draining the battery while
            // the chip sleeps).
            display.set_backlight(0);
            display.sleep();
            touch.sleep(); // CST816D into deep sleep (see Touch::sleep)
            drop(camera.take()); // drop the camera handle (sensor soft-standby)
            power::camera_power_down_hold();
            power::enter_deep_sleep();
        }

        // Touch checked at 5 ms resolution to catch short INT pulses reliably.
        match touch.poll() {
            Some(TouchEvent::Input(event)) => {
                // The word-entry grid is tap-only: swipes (which arrive as
                // directional inputs) must not move a cursor there. Drop them
                // on that screen; every other screen keeps swipe navigation.
                let is_swipe = matches!(
                    event,
                    InputEvent::Up | InputEvent::Down | InputEvent::Left | InputEvent::Right
                );
                if !(is_swipe && app.on_word_picker()) {
                    // Any directional gesture or footer tap cancels a pending
                    // tap-confirm so the two don't stack.
                    pending_tap_confirm = None;
                    app.handle_input(event);
                }
            }
            Some(TouchEvent::BodyTap { x, y }) => {
                if app.tap_keyboard(x, y) {
                    // On-screen keyboard (passphrase / message entry): a tap
                    // on a key mutates the buffer directly. Accept and Back
                    // live in the footer action bar, so no Confirm is fired
                    // here; taps in the keyboard body above the footer (text
                    // box / slider) are absorbed.
                    pending_tap_confirm = None;
                } else if app.tap_word_grid(x, y) {
                    // Word-entry alphabet grid: same reasoning as char grid.
                    pending_tap_confirm = None;
                    app.handle_input(InputEvent::Confirm);
                } else if y >= footer_y {
                    // Footer action bar (Back / Secondary / Confirm thirds).
                    // Only reached when neither grid type claimed the tap.
                    pending_tap_confirm = None;
                    let event = if x < footer_third {
                        InputEvent::Back
                    } else if x < footer_third * 2 {
                        InputEvent::Secondary
                    } else {
                        InputEvent::Confirm
                    };
                    // The word picker has no Check cell (letters are tap-only),
                    // so a tap on the right third must not commit a letter.
                    if !(event == InputEvent::Confirm && app.on_word_picker()) {
                        if event == InputEvent::Confirm && app.confirm_will_derive() {
                            let _ = screens::draw_computing(&mut display, &app.theme);
                            display.flush();
                        }
                        app.handle_input(event);
                    }
                } else if let Some(layout) = app.tap_layout() {
                    // List screen: move selection then fire Confirm after a
                    // short delay so the highlight is visible for one frame.
                    if y >= layout.list_top {
                        let body_y = (y - layout.list_top) as usize;
                        let list_h =
                            (app.theme.height as u16 - footer_h - layout.list_top) as usize;
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
                } else if app.tap_pages_review() {
                    // TX review: a body tap pages forward through the
                    // structured review (same as a down/right swipe). The SIGN
                    // footer cell (Confirm) signs. Routing taps to Secondary
                    // keeps Confirm reserved for signing.
                    pending_tap_confirm = None;
                    app.handle_input(InputEvent::Secondary);
                } else if !app.on_word_picker() {
                    // Read-only / advance-only screen (word display, card
                    // confirm, QR view, about, errors…): tap anywhere fires
                    // Confirm so the user can page forward. Excludes the word
                    // picker, where only a tap on a letter cell selects.
                    pending_tap_confirm = None;
                    app.handle_input(InputEvent::Confirm);
                }
            }
            None => {}
        }

        if let Some(t) = pending_tap_confirm {
            if t.elapsed().as_millis() >= TAP_CONFIRM_DELAY_MS as u128 {
                pending_tap_confirm = None;
                if app.confirm_will_derive() {
                    let _ = screens::draw_computing(&mut display, &app.theme);
                    display.flush();
                }
                app.handle_input(InputEvent::Confirm);
            }
        }

        app.tick();

        // Sample the battery every couple of seconds and surface it to the GUI,
        // which draws the footer icon. A failed read leaves the last value.
        if let Some(chan) = bat_chan.as_mut() {
            if last_bat_sample.elapsed().as_secs() >= BATTERY_SAMPLE_SECS {
                last_bat_sample = std::time::Instant::now();
                // Average a handful of quick reads to knock down ADC noise
                // before the slope detector sees the sample.
                let mut acc = 0u32;
                let mut n = 0u32;
                for _ in 0..8 {
                    if let Ok(mv) = chan.read() {
                        acc += mv as u32;
                        n += 1;
                    }
                }
                if n > 0 {
                    app.battery = bat_monitor.update((acc / n) as u16);
                }
            }
        }

        // Only refresh the display (and pull the heavy preview frame) at ~30 Hz,
        // independently of the much faster touch-poll loop.
        let will_draw = last_draw.elapsed().as_millis() >= 33;

        // Camera lifecycle: open/close based on whether the current screen
        // needs a camera feed; pull frames and QR results into App fields.
        let wants_camera = app.wants_camera();
        if wants_camera && camera.is_none() && app.camera_error.is_none() {
            match camera::EspCamera::open() {
                Ok(cam) => {
                    cam.set_decode_enabled(true);
                    camera = Some(cam);
                }
                Err(e) => {
                    log::warn!("camera open failed: {e}");
                    app.camera_error = Some(e);
                }
            }
        } else if !wants_camera {
            // Leaving any camera screen tears the handle down *and* clears a
            // latched open error. Clearing must not be gated on `camera.is_some()`:
            // a failed open() leaves `camera` None but `camera_error` Some, so
            // gating here would strand the error forever (line 303's guard then
            // blocks every retry) — the user would have to power-cycle. Always
            // clearing lets re-entering the screen retry the open.
            if camera.is_some() {
                camera = None;
                app.latest_frame = None;
                app.scan_diag = faraday_core::camera::ScanDiagnostics::default();
            }
            app.camera_error = None;
        }
        let mut camera_died = false;
        if let Some(cam) = &camera {
            if let Some(err) = cam.take_fatal_err() {
                log::warn!("camera fatal: {err}");
                app.camera_error = Some(err);
                camera_died = true;
            } else {
                app.scan_diag = cam.diagnostics();
                // Clone the ~480 KB SVGA preview frame only on the iterations we
                // actually draw. Pulling it every loop iteration saturates PSRAM
                // bandwidth, which tears the camera DMA (black blocks) and starves
                // the display flush (flicker). The decoder thread keeps its own
                // copy, so QR detection is unaffected.
                if will_draw {
                    if let Some(frame) = cam.latest() {
                        app.latest_frame = Some(frame);
                    }
                }
                if let Some(qr) = cam.take_qr() {
                    app.scanned_qr = Some(qr);
                }
                cam.set_small_qr_mode(app.wants_small_qr_scan());
            }
        }
        if camera_died {
            camera = None;
            app.scan_diag = faraday_core::camera::ScanDiagnostics::default();
        }

        // Display refreshed at ~30 Hz independently of the touch poll rate.
        if will_draw {
            if app.is_blanked() {
                let elapsed_ms = app.splash_anim_start.elapsed().as_millis() as u64;
                let _ = screens::draw_splash(&mut display, &app.theme, elapsed_ms);
            } else {
                // Blit the camera frame first so the GUI overlay renders on top.
                if app.wants_camera() {
                    if let Some(frame) = &app.latest_frame {
                        display.blit_camera_frame(frame, app.theme.bg);
                    }
                }
                let _ = app.draw(&mut display);
            }
            display.flush();
            last_draw = std::time::Instant::now();
        }

        FreeRtos::delay_ms(5);
    }
}
