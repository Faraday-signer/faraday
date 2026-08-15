//! Shared ESP32-S3 event loop.
//!
//! Board binary crates construct their concrete display / touch / battery types
//! (implementing the [`crate::board`] traits), then hand them to [`run`], which
//! owns the main loop: touch handling, the camera/QR lifecycle, battery polling,
//! the draw cadence, and BOOT-button power-off. Everything board-specific lives
//! behind the traits, so this loop is identical across boards.

use esp_idf_hal::delay::FreeRtos;
use faraday_core::gui::app::{App, InputEvent, ListTap};
use faraday_core::gui::screens;
use faraday_core::ui::widgets::FOOTER_H;
use faraday_core::ui::Theme;

use crate::board::{BoardBattery, BoardDisplay, BoardImu, BoardTouch, TouchEvent};
use crate::{camera, power};

/// Run the Faraday firmware. Takes ownership of the board peripherals and never
/// returns (it loops until BOOT long-press deep-sleeps the chip).
pub fn run<'d, D, T, B, M>(
    mut display: D,
    mut touch: T,
    mut power_btn: power::PowerButton<'d>,
    mut battery: Option<B>,
    mut imu: Option<M>,
    theme: Theme,
) where
    D: BoardDisplay,
    T: BoardTouch,
    B: BoardBattery,
    M: BoardImu,
{
    // Validate the hardware (mbedtls) BIP39 seed path against a known-answer
    // vector before any wallet can be created or loaded. The host test suite
    // can't exercise this path, so a divergence from the BIP39 standard would
    // otherwise surface only as silently wrong addresses. Fail closed.
    assert!(
        faraday_core::crypto::bip39::seed_derivation_self_test(),
        "BIP39 seed self-test failed — hardware SHA512 path diverges from spec"
    );

    // Turn on the SAR-ADC entropy source so `esp_random` is hardware-conditioned
    // rather than pseudo-random. It draws on ADC voltage noise, needs no radio,
    // and must stay enabled for the app's lifetime — never call the matching
    // bootloader_random_disable() while running.
    //
    // The SAR-ADC entropy source and the ADC driver are mutually exclusive: a
    // future board wiring a real ADC-based `BoardBattery` here must not drive
    // ADC1 concurrently, or it will both corrupt battery reads and silently
    // degrade the RNG mid-session. The current touch2 board passes `None`, so
    // this is safe today.
    extern "C" {
        fn bootloader_random_enable();
    }
    unsafe { bootloader_random_enable() };

    rng_health_check();

    // Back-date so the first loop iteration samples immediately. `checked_sub`
    // guards against underflow early in boot (the monotonic clock is still near
    // zero), in which case the first sample just lands one interval later.
    let mut last_bat_sample = std::time::Instant::now()
        .checked_sub(std::time::Duration::from_secs(BATTERY_SAMPLE_SECS))
        .unwrap_or_else(std::time::Instant::now);

    let mut app = App::new(theme);

    // No splash screen — boot straight into the main menu.
    app.enter_main_menu();
    app.last_activity = std::time::Instant::now();

    // How long a tapped option on the seed-verification quiz is held
    // highlighted (green if correct, red if wrong) before it commits — long
    // enough for one display frame to render the flash. Every other tapped row
    // commits immediately, no flash.
    const TAP_CONFIRM_DELAY_MS: u64 = 40;

    // Held-swipe scroll pacing on continuous screens (menus, coin/dice, word
    // screens): one step per this interval, i.e. ~3 positions/second.
    const SWIPE_REPEAT_MS: u64 = 300;

    // Footer action bar: bottom FOOTER_H strip, divided into three equal
    // thirds — Back (left) / Secondary (middle) / Confirm (right), matching
    // the glyphs drawn by EdgeHints. Applied only when the tap doesn't fall
    // on a grid or list area, which now sit above the bar.
    let footer_h = FOOTER_H as u16;
    let footer_y = app.theme.height as u16 - footer_h;
    let footer_third = app.theme.width as u16 / 3;

    let mut camera: Option<camera::EspCamera> = None;
    let mut last_draw = std::time::Instant::now();
    // Shake-detection poll cadence (the light/dark theme Easter egg).
    let mut last_shake_poll = std::time::Instant::now();
    let mut pending_tap_confirm: Option<std::time::Instant> = None;
    // Safety timer for the press-down highlight (FA-21): a press that never
    // resolves into a tap or gesture (finger dragged a little, then lifted)
    // would otherwise leave a row highlighted forever.
    let mut press_started: Option<std::time::Instant> = None;
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

        // Held swipes scroll continuously (paced) on most screens, but step one
        // piece at a time through the paper-backup walkthrough.
        touch.set_swipe_repeat(if app.swipe_discrete() {
            None
        } else {
            Some(std::time::Duration::from_millis(SWIPE_REPEAT_MS))
        });

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
                    press_started = None;
                    app.handle_input(event);
                }
            }
            Some(TouchEvent::Press { x, y: py }) => {
                let _ = x;
                // Press-down feedback (FA-21): highlight the touched list row
                // the moment contact begins; the action still fires on lift
                // (BodyTap). Keyboards, grids, and the footer draw their own
                // affordances — list rows are the dead-feeling case.
                if py < footer_y {
                    app.press_list_row(py, footer_h);
                    if app.pressed_row.is_some() {
                        press_started = Some(std::time::Instant::now());
                    }
                }
            }
            Some(TouchEvent::BodyTap { x, y }) => {
                app.clear_press();
                press_started = None;
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
                    } else if x < footer_third * 2 && !app.tap_pages_review() {
                        InputEvent::Secondary
                    } else {
                        // On the TX review the sign band spans the middle +
                        // right cells (FA-23): a tap anywhere on it is the
                        // sign action. Body taps already page, so the middle
                        // cell's Secondary role is redundant there.
                        InputEvent::Confirm
                    };
                    // The word picker has no Check cell (letters are tap-only),
                    // so a tap on the right third must not commit a letter.
                    // Same for tap-to-go list screens (FA-21): their Confirm
                    // cell is empty chrome and must stay inert, or an
                    // invisible commit lives in the corner.
                    if !(event == InputEvent::Confirm
                        && (app.on_word_picker() || app.footer_confirm_inert()))
                    {
                        if event == InputEvent::Confirm && app.confirm_will_derive() {
                            let _ = screens::draw_computing(&mut display, &app.theme);
                            display.flush();
                        }
                        app.handle_input(event);
                    }
                } else {
                    match app.tap_list_row(y, footer_h) {
                        ListTap::Committed => {
                            // List screen: the tapped row is now selected.
                            // Commit immediately — no highlight flash. The one
                            // exception is the seed-verification quiz: flash the
                            // tapped option for one frame (pending_tap_confirm)
                            // — green if correct, red if wrong — before the
                            // Confirm advances or resets the quiz.
                            pending_tap_confirm = None;
                            if app.on_verify_quiz() {
                                app.verify_flash = true;
                                pending_tap_confirm = Some(std::time::Instant::now());
                            } else {
                                if app.confirm_will_derive() {
                                    let _ = screens::draw_computing(&mut display, &app.theme);
                                    display.flush();
                                }
                                app.handle_input(InputEvent::Confirm);
                            }
                        }
                        ListTap::Absorbed => {
                            // Dead-zone tap on a list screen (empty slot below
                            // the last drawn row). Consume it and do nothing:
                            // must NOT fall through to the catch-all Confirm
                            // below, which would commit the currently
                            // highlighted row (e.g. SHOW / YES after a swipe).
                        }
                        ListTap::NotAList => {
                            if app.is_picker_screen() {
                                // Coin-flip / dice-roll value grid: tapping a
                                // cell selects that value and commits it
                                // immediately. Taps off the grid are absorbed so
                                // a stray tap can't commit the current value.
                                if app.tap_picker(x, y) {
                                    app.handle_input(InputEvent::Confirm);
                                }
                            } else if app.tap_pages_review() {
                                // TX review: a body tap pages forward through the
                                // structured review (same as a down/right swipe).
                                // The SIGN footer cell (Confirm) signs. Routing
                                // taps to Secondary keeps Confirm reserved for
                                // signing.
                                pending_tap_confirm = None;
                                app.handle_input(InputEvent::Secondary);
                            } else if app.requires_explicit_confirm() {
                                // Security gate (checksum-less CompactSeedQR
                                // address confirm, #89): a body tap must NOT
                                // commit. Absorb it — only the footer Check cell
                                // confirms and Cross/Back cancels, so the user
                                // can't double-tap past the address without
                                // reading it.
                                pending_tap_confirm = None;
                            } else if !app.on_word_picker() {
                                // Read-only / advance-only screen (word display,
                                // card confirm, QR view, about, errors…): tap
                                // anywhere fires Confirm so the user can page
                                // forward. Excludes the word picker, where only a
                                // tap on a letter cell selects.
                                pending_tap_confirm = None;
                                app.handle_input(InputEvent::Confirm);
                            }
                        }
                    }
                }
            }
            None => {}
        }

        // Only the seed-verification flash defers a Confirm now: hold the
        // green/red highlight for one frame, then commit (advance or reset).
        // `handle_input` clears `verify_flash`, ending the flash.
        if let Some(t) = pending_tap_confirm {
            if t.elapsed().as_millis() >= TAP_CONFIRM_DELAY_MS as u128 {
                pending_tap_confirm = None;
                app.handle_input(InputEvent::Confirm);
            }
        }

        // Expire an unresolved press highlight (see `press_started`).
        if let Some(t) = press_started {
            if t.elapsed().as_millis() >= 600 {
                press_started = None;
                app.clear_press();
            }
        }

        app.tick();

        // Sample the battery every couple of seconds and surface it to the GUI,
        // which draws the footer icon. A board with no gauge (`None`) or a sample
        // that returns nothing leaves the last value via the gauge's own caching.
        if let Some(bat) = battery.as_mut() {
            if last_bat_sample.elapsed().as_secs() >= BATTERY_SAMPLE_SECS {
                last_bat_sample = std::time::Instant::now();
                app.battery = bat.sample();
            }
        }

        // Shake Easter egg: a firm shake flips the theme between the dark
        // and light palettes. Polled at ~25 Hz; detection and cooldown
        // live in the board's IMU driver.
        if let Some(m) = imu.as_mut() {
            if last_shake_poll.elapsed().as_millis() >= 40 {
                last_shake_poll = std::time::Instant::now();
                if m.shake() {
                    app.toggle_theme();
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
                // Grab the latest preview frame (a shared Arc clone, not a
                // ~480 KB copy) only on the iterations we actually draw — the
                // reference is only consumed by the blit below, so refreshing it
                // off-draw is pointless work. The decoder shares the same Arc, so
                // QR detection is unaffected.
                if will_draw {
                    if let Some(frame) = cam.latest() {
                        app.latest_frame = Some(frame);
                    }
                }
                if let Some(qr) = cam.take_qr() {
                    app.scanned_qr = Some(qr);
                }
                // Only run the (CPU-bound, PSRAM-bandwidth-hungry) QR decoder on
                // screens that actually scan; the camera-entropy screen just hashes
                // a preview frame and needs no decode. Set per-frame so a screen
                // change while the camera stays open re-gates it correctly.
                cam.set_decode_enabled(app.wants_qr_decode());
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

/// Boot RNG liveness check: 256 samples through the statistical tests in
/// `faraday_core::crypto::rng_health` (repetition count at the SP 800-90B
/// §4.4.1 cutoff, monobit, distinct-values). Fail-closed: a rejected RNG
/// panics before any wallet code can draw entropy.
///
/// Honest scope (see the core module docs): this catches a dead, stuck,
/// cycling, or grossly biased generator. It cannot detect a conditioned
/// source whose true entropy input silently failed — `esp_random` output
/// stays statistically plausible in that state. That gap is covered by
/// wallet creation XORing camera entropy with the RNG, so a seed is never
/// weaker than the stronger source.
fn rng_health_check() {
    use faraday_core::crypto::rng_health::{check_samples, RNG_HEALTH_SAMPLES};

    let mut samples = [0u32; RNG_HEALTH_SAMPLES];
    for v in samples.iter_mut() {
        *v = unsafe { esp_idf_sys::esp_random() };
    }
    if let Err(reason) = check_samples(&samples) {
        panic!("RNG health check failed — {reason}");
    }
    // Success is otherwise silent; one line makes the check observable over
    // serial instead of invisible.
    log::info!("rng health: {} samples ok", RNG_HEALTH_SAMPLES);
}

// How often to re-sample the battery. The pack voltage moves slowly, so a
// couple of seconds keeps the gauge fresh without spinning the ADC.
const BATTERY_SAMPLE_SECS: u64 = 2;
