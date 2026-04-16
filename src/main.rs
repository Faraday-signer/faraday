//! Faraday — air-gapped Solana transaction signer.

mod crypto;

#[cfg(any(feature = "simulator", target_os = "linux"))]
mod gui;
#[cfg(target_os = "linux")]
mod hardware;
#[cfg(any(feature = "simulator", target_os = "linux"))]
mod qr;
#[cfg(any(feature = "simulator", target_os = "linux"))]
mod signer;
#[cfg(any(feature = "simulator", target_os = "linux"))]
mod parser;

#[cfg(any(feature = "simulator", target_os = "linux"))]
use gui::app::{App, InputEvent};

fn main() {
    println!("Faraday v0.1.0");

    #[cfg(feature = "simulator")]
    run_simulator();

    #[cfg(all(target_os = "linux", not(feature = "simulator")))]
    run_pi();

    #[cfg(all(not(target_os = "linux"), not(feature = "simulator")))]
    run_headless();
}

/// Desktop simulator: renders to a minifb window.
#[cfg(feature = "simulator")]
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

    // Splash
    app.draw(&mut fb).unwrap();
    let buf = fb.to_rgb888();
    window.update_with_buffer(&buf, 240, 240).unwrap();
    std::thread::sleep(Duration::from_secs(2));
    app.enter_main_menu();

    while window.is_open() {
        // Map keyboard to InputEvent
        let event = if window.is_key_pressed(Key::Up, minifb::KeyRepeat::Yes) {
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
        } else if window.is_key_pressed(Key::Escape, minifb::KeyRepeat::No) {
            Some(InputEvent::Back)
        } else if window.is_key_pressed(Key::X, minifb::KeyRepeat::No) {
            Some(InputEvent::Secondary)
        } else {
            None
        };

        if let Some(ev) = event {
            app.handle_input(ev);
        }

        app.draw(&mut fb).unwrap();
        let buf = fb.to_rgb888();
        window.update_with_buffer(&buf, 240, 240).unwrap();
    }
}

/// Pi hardware: ST7789 display + GPIO buttons.
#[cfg(all(target_os = "linux", not(feature = "simulator")))]
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

    // Splash
    app.draw(&mut display).unwrap();
    display.flush();
    std::thread::sleep(Duration::from_secs(2));
    app.enter_main_menu();

    loop {
        app.draw(&mut display).unwrap();
        display.flush();

        if let Some(event) = buttons.wait_for_press(Duration::from_millis(100)) {
            let input = match event.button {
                Button::JoyUp => InputEvent::Up,
                Button::JoyDown => InputEvent::Down,
                Button::JoyLeft => InputEvent::Left,
                Button::JoyRight => InputEvent::Right,
                Button::Key1 | Button::JoyPress => InputEvent::Confirm,
                Button::Key3 => InputEvent::Back,
                Button::Key2 => InputEvent::Secondary,
            };
            app.handle_input(input);
        }
    }
}

/// Headless mode: just run crypto sanity check.
#[cfg(all(not(target_os = "linux"), not(feature = "simulator")))]
fn run_headless() {
    println!("No display available. Run with --features simulator for desktop UI.");
    println!("Running crypto sanity check...");

    let mnemonic = crypto::bip39::mnemonic_from_entropy(b"faraday rust test", 12).unwrap();
    println!("Mnemonic: {}", mnemonic);
    assert!(crypto::bip39::validate_mnemonic(&mnemonic));

    let seed = crypto::bip39::mnemonic_to_seed(&mnemonic, "");
    let keypair = crypto::slip0010::derive_solana_keypair(&seed, 0);
    println!("Address: {}", crypto::derivation::address(&keypair));
    println!("All OK.");
}
