//! Faraday — air-gapped Solana transaction signer.

#![forbid(unsafe_code)]

mod crypto;

#[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
mod camera;
#[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
mod ui;
#[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
mod gui;
#[cfg(target_os = "linux")]
mod hardware;
#[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
mod qr;
#[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
mod signer;
#[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
mod parser;

#[cfg(any(feature = "_desktop_sim", target_os = "linux"))]
use gui::app::{App, InputEvent};

fn main() {
    println!("Faraday v0.1.0");

    #[cfg(feature = "_desktop_sim")]
    run_simulator();

    #[cfg(all(target_os = "linux", not(feature = "_desktop_sim")))]
    run_pi();

    #[cfg(all(not(target_os = "linux"), not(feature = "_desktop_sim")))]
    run_headless();
}

/// Desktop simulator: renders to a minifb window.
#[cfg(feature = "_desktop_sim")]
fn run_simulator() {
    use gui::framebuffer::Framebuffer;
    use minifb::{Key, Window, WindowOptions, Scale};
    use std::time::Duration;

    let mut fb = Framebuffer::new();
    let mut app = App::new();

    let mut window = Window::new(
        "Faraday Simulator",
        240, 240,
        WindowOptions {
            scale: Scale::X2,
            ..WindowOptions::default()
        },
    ).expect("Failed to create window");

    window.set_target_fps(30);
    window.set_key_repeat_delay(0.3);
    window.set_key_repeat_rate(0.1);

    // Splash. Keep pumping the window for 2 seconds rather than sleeping —
    // a bare sleep lets key presses queue up, and the first one gets consumed
    // without action on the first iteration of the main loop.
    if let Err(e) = app.draw(&mut fb) {
        eprintln!("draw: {e:?}");
    }
    let buf = fb.to_rgb888();
    let splash_start = std::time::Instant::now();
    while window.is_open() && splash_start.elapsed() < Duration::from_secs(2) {
        if let Err(e) = window.update_with_buffer(&buf, 240, 240) {
            eprintln!("update_with_buffer: {e:?}");
        }
    }
    app.enter_main_menu();
    // Reset idle timer so the splash duration doesn't count toward blanking.
    app.last_activity = std::time::Instant::now();

    // Long-press Back (Escape) detection. Tap = normal Back; hold ≥ threshold
    // fires PowerOffShortcut instead and suppresses the trailing Back.
    let hold_threshold = Duration::from_millis(1500);
    let mut esc_down_at: Option<std::time::Instant> = None;
    let mut esc_long_press_fired = false;

    while window.is_open() {
        // Long-press handling for Back first. Runs every frame so we can
        // emit the shortcut mid-hold (the user shouldn't have to release).
        let esc_held = window.is_key_down(Key::Escape);
        let long_press_event = match (esc_down_at, esc_held) {
            (None, true) => {
                // Rising edge — start the timer, don't emit anything yet.
                esc_down_at = Some(std::time::Instant::now());
                esc_long_press_fired = false;
                None
            }
            (Some(t), true) if !esc_long_press_fired && t.elapsed() >= hold_threshold => {
                // Held past the threshold — fire the shortcut once.
                esc_long_press_fired = true;
                Some(InputEvent::PowerOffShortcut)
            }
            (Some(t), false) => {
                // Released. Emit normal Back only if the shortcut didn't fire.
                let fire_back = !esc_long_press_fired && t.elapsed() < hold_threshold;
                esc_down_at = None;
                esc_long_press_fired = false;
                if fire_back { Some(InputEvent::Back) } else { None }
            }
            _ => None,
        };

        // Other key-event detection (unchanged).
        let event = if let Some(ev) = long_press_event {
            Some(ev)
        } else if window.is_key_pressed(Key::Up, minifb::KeyRepeat::Yes) {
            Some(InputEvent::Up)
        } else if window.is_key_pressed(Key::Down, minifb::KeyRepeat::Yes) {
            Some(InputEvent::Down)
        } else if window.is_key_pressed(Key::Left, minifb::KeyRepeat::Yes) {
            Some(InputEvent::Left)
        } else if window.is_key_pressed(Key::Right, minifb::KeyRepeat::Yes) {
            Some(InputEvent::Right)
        } else if window.is_key_pressed(Key::Enter, minifb::KeyRepeat::No)
            || window.is_key_pressed(Key::Z, minifb::KeyRepeat::No)
        {
            Some(InputEvent::Confirm)
        } else if window.is_key_pressed(Key::X, minifb::KeyRepeat::No) {
            Some(InputEvent::Secondary)
        } else {
            None
        };

        if let Some(ev) = event {
            app.handle_input(ev);
        }

        app.tick();

        use embedded_graphics_core::draw_target::DrawTarget;
        if app.is_blanked() {
            // Idle timeout reached — show the Faraday logo (splash) instead
            // of a black screen, so the device is visibly "on + idle" rather
            // than indistinguishable from powered-off.
            let _ = gui::screens::draw_splash(&mut fb);
        } else {
            // On camera screens, blit the latest webcam frame as background so
            // overlay drawing in app.draw() paints on top of live preview. When
            // no frame is available yet (camera warming up or unavailable), fill
            // with a dark background so stale pixels from the previous screen
            // don't leak through.
            if app.wants_camera() {
                match app.latest_frame.clone() {
                    Some(frame) => fb.blit_camera_frame(&frame),
                    None => {
                        let _ = fb.clear(gui::colors::BG_DARK);
                    }
                }
            }
            if let Err(e) = app.draw(&mut fb) {
                eprintln!("draw: {e:?}");
            }
        }
        let buf = fb.to_rgb888();
        if let Err(e) = window.update_with_buffer(&buf, 240, 240) {
            eprintln!("update_with_buffer: {e:?}");
        }
    }
}

/// Pi hardware: ST7789 display + GPIO buttons.
#[cfg(all(target_os = "linux", not(feature = "_desktop_sim")))]
fn run_pi() {
    use crate::hardware::buttons::{Button, Buttons};
    use crate::hardware::st7789::ST7789;
    use std::time::Duration;

    let mut display = match ST7789::new() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Display init failed: {}", e);
            return;
        }
    };

    let mut buttons = match Buttons::new() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Button init failed: {}", e);
            return;
        }
    };

    let mut app = App::new();

    // Splash — draw errors intentionally ignored to avoid crashing the device.
    let _ = app.draw(&mut display);
    display.flush();
    std::thread::sleep(Duration::from_secs(2));
    app.enter_main_menu();
    // Reset idle timer so splash doesn't count toward blanking.
    app.last_activity = std::time::Instant::now();

    loop {
        use embedded_graphics_core::draw_target::DrawTarget;

        if let Some(event) = buttons.wait_for_press(Duration::from_millis(33)) {
            let input = if event.long_press && event.button == Button::Key3 {
                // Long-press Back → Power Off shortcut. Driver's 500ms long-
                // press threshold is enough to distinguish from a regular tap.
                InputEvent::PowerOffShortcut
            } else {
                match event.button {
                    Button::JoyUp => InputEvent::Up,
                    Button::JoyDown => InputEvent::Down,
                    Button::JoyLeft => InputEvent::Left,
                    Button::JoyRight => InputEvent::Right,
                    Button::Key1 | Button::JoyPress => InputEvent::Confirm,
                    Button::Key3 => InputEvent::Back,
                    Button::Key2 => InputEvent::Secondary,
                }
            };
            app.handle_input(input);
        }

        // Drive camera lifecycle + pull latest frame + auto-advance on QR.
        app.tick();

        if app.is_blanked() {
            let _ = crate::gui::screens::draw_splash(&mut display);
        } else {
            // On camera screens, blit preview first, then let the screen
            // overlay draw on top. Fill dark when no frame is ready yet.
            if app.wants_camera() {
                match app.latest_frame.clone() {
                    Some(frame) => display.blit_camera_frame(&frame),
                    None => {
                        let _ = display.clear(crate::gui::colors::BG_DARK);
                    }
                }
            }
            let _ = app.draw(&mut display);
        }
        display.flush();
    }
}

/// Headless mode: just run crypto sanity check.
#[cfg(all(not(target_os = "linux"), not(feature = "_desktop_sim")))]
fn run_headless() {
    println!("No display available. Run with --features simulator for desktop UI.");
    println!("Running crypto sanity check...");

    let mnemonic = match crypto::bip39::mnemonic_from_entropy(b"faraday rust test", 12) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Mnemonic generation failed: {e}");
            return;
        }
    };
    println!("Mnemonic: {}", mnemonic);
    if !crypto::bip39::validate_mnemonic(&mnemonic) {
        eprintln!("Mnemonic validation failed");
        return;
    }

    let seed = crypto::bip39::mnemonic_to_seed(&mnemonic, "");
    let keypair = crypto::slip0010::derive_solana_keypair(&seed, 0);
    println!("Address: {}", crypto::derivation::address(&keypair));
    println!("All OK.");
}
