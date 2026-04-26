//! GPIO button and joystick input for Waveshare 1.3" LCD HAT.
//!
//! All buttons are active LOW with internal pull-ups.
//! Supports debouncing and long-press detection.

use rppal::gpio::{Gpio, InputPin, Level};
use std::thread;
use std::time::{Duration, Instant};

const KEY1_PIN: u8 = 21;
const KEY2_PIN: u8 = 20;
const KEY3_PIN: u8 = 16;
const JOY_UP_PIN: u8 = 6;
const JOY_DOWN_PIN: u8 = 19;
const JOY_LEFT_PIN: u8 = 5;
const JOY_RIGHT_PIN: u8 = 26;
const JOY_PRESS_PIN: u8 = 13;

const DEBOUNCE: Duration = Duration::from_millis(50);
const LONG_PRESS: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Key1,
    Key2,
    Key3,
    JoyUp,
    JoyDown,
    JoyLeft,
    JoyRight,
    JoyPress,
}

#[derive(Debug, Clone, Copy)]
pub struct ButtonEvent {
    pub button: Button,
    pub long_press: bool,
}

/// Maps each Button variant to its GPIO pin number.
const BUTTON_PINS: [(Button, u8); 8] = [
    (Button::Key1, KEY1_PIN),
    (Button::Key2, KEY2_PIN),
    (Button::Key3, KEY3_PIN),
    (Button::JoyUp, JOY_UP_PIN),
    (Button::JoyDown, JOY_DOWN_PIN),
    (Button::JoyLeft, JOY_LEFT_PIN),
    (Button::JoyRight, JOY_RIGHT_PIN),
    (Button::JoyPress, JOY_PRESS_PIN),
];

pub struct Buttons {
    pins: Vec<(Button, InputPin)>,
    last_event: Instant,
}

impl Buttons {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let gpio = Gpio::new()?;
        let mut pins = Vec::new();

        for &(button, pin_num) in &BUTTON_PINS {
            let pin = gpio.get(pin_num)?.into_input_pullup();
            pins.push((button, pin));
        }

        Ok(Buttons {
            pins,
            last_event: Instant::now() - DEBOUNCE,
        })
    }

    /// Poll for a currently pressed button (non-blocking).
    pub fn read(&self) -> Option<Button> {
        for (button, pin) in &self.pins {
            if pin.read() == Level::Low {
                return Some(*button);
            }
        }
        None
    }

    /// Wait for a button press with debouncing and long-press detection.
    /// Returns `None` if timeout expires (0 = wait forever).
    pub fn wait_for_press(&mut self, timeout: Duration) -> Option<ButtonEvent> {
        let start = Instant::now();

        loop {
            if let Some(button) = self.read() {
                // Debounce
                if self.last_event.elapsed() < DEBOUNCE {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }

                // Detect long press
                let press_start = Instant::now();
                while self.read() == Some(button) {
                    if press_start.elapsed() > LONG_PRESS {
                        self.last_event = Instant::now();
                        // Wait for release
                        while self.read() == Some(button) {
                            thread::sleep(Duration::from_millis(10));
                        }
                        return Some(ButtonEvent { button, long_press: true });
                    }
                    thread::sleep(Duration::from_millis(10));
                }

                self.last_event = Instant::now();
                return Some(ButtonEvent { button, long_press: false });
            }

            if !timeout.is_zero() && start.elapsed() > timeout {
                return None;
            }

            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Wait indefinitely for any button press.
    pub fn wait_for_any(&mut self) -> ButtonEvent {
        loop {
            if let Some(event) = self.wait_for_press(Duration::from_secs(3600)) {
                return event;
            }
        }
    }
}
