//! CST816D capacitive touch driver via ESP-IDF I2C.
//!
//! Pin mapping for Waveshare ESP32-S3-Touch-LCD-2:
//!   SDA  = GPIO 48
//!   SCL  = GPIO 47
//!   INT  = GPIO 46  (active-low, interrupt-driven)
//!
//! Register 0x03 event-flag encoding (bits [7:6]):
//!   0x00 = PRESS DOWN, 0x01 = LIFT UP, 0x02 = CONTACT
//!
//! We emit on PRESS DOWN by checking finger_num > 0 (reliable) rather than
//! comparing the event-flag byte (chip-revision-sensitive).  The 50 ms
//! debounce absorbs the paired LIFT-UP interrupt and any stale
//! GESTURE_SINGLE_TAP that arrives with the finger-up report.
//!
//! All non-gesture taps produce `BodyTap { x, y }` regardless of position.
//! The main loop decides how to interpret the coordinates; this keeps the
//! driver free of any knowledge about screen layout (char grids, list rows,
//! footer zones) that belongs in the application layer.
//!
//! Gestures:
//!     Swipe up    = Up
//!     Swipe down  = Down
//!     Swipe left  = Left
//!     Swipe right = Right
//!   Long press (>1.5 s) = PowerOffShortcut

use esp_idf_hal::gpio::{Input, InterruptType, PinDriver};
use esp_idf_hal::i2c::I2cDriver;
use faraday_core::gui::app::InputEvent;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const CST816D_ADDR: u8 = 0x15;

const GESTURE_SWIPE_UP: u8 = 0x01;
const GESTURE_SWIPE_DOWN: u8 = 0x02;
const GESTURE_SWIPE_LEFT: u8 = 0x03;
const GESTURE_SWIPE_RIGHT: u8 = 0x04;
const GESTURE_SINGLE_TAP: u8 = 0x05;
const GESTURE_LONG_PRESS: u8 = 0x0C;


static INT_FIRED: AtomicBool = AtomicBool::new(false);

// 50 ms absorbs the LIFT-UP interrupt (~50 ms after contact) and any
// GESTURE_SINGLE_TAP that arrives with it, without blocking rapid tapping
// or typing (minimum inter-tap gap is 50 ms ≈ 20 taps/second).
const DEBOUNCE: Duration = Duration::from_millis(50);

/// What the touch driver delivers to the main loop.
///
/// Gestures and footer-zone taps are mapped to platform-independent
/// `InputEvent`s. Body taps carry raw coordinates so the main loop can
/// implement tap-to-select without any touch logic leaking into core.
pub enum TouchEvent {
    Input(InputEvent),
    BodyTap { x: u16, y: u16 },
}

pub struct Touch<'d> {
    i2c: I2cDriver<'d>,
    int: PinDriver<'d, Input>,
    last_event: Instant,
}

impl<'d> Touch<'d> {
    pub fn new(i2c: I2cDriver<'d>, mut int: PinDriver<'d, Input>) -> Self {
        int.set_interrupt_type(InterruptType::NegEdge)
            .expect("touch: set interrupt type");
        // SAFETY: closure only writes to a global AtomicBool — no captures,
        // safe from ISR context.
        unsafe {
            int.subscribe(|| {
                INT_FIRED.store(true, Ordering::Relaxed);
            })
            .expect("touch: subscribe interrupt");
        }
        int.enable_interrupt().expect("touch: enable interrupt");

        Self {
            i2c,
            int,
            last_event: Instant::now() - DEBOUNCE,
        }
    }

    pub fn poll(&mut self) -> Option<TouchEvent> {
        if !INT_FIRED.swap(false, Ordering::Relaxed) {
            return None;
        }
        self.int.enable_interrupt().ok();

        let mut buf = [0u8; 6];
        if self.i2c.write_read(CST816D_ADDR, &[0x01], &mut buf, 30).is_err() {
            return None;
        }

        let gesture   = buf[0];
        let finger_num = buf[1];
        let x = (((buf[2] & 0x0F) as u16) << 8) | buf[3] as u16;
        let y = (((buf[4] & 0x0F) as u16) << 8) | buf[5] as u16;

        // Named gesture: fire immediately. GESTURE_SINGLE_TAP is kept as a
        // fallback for cases where the chip classifies the tap itself; the
        // debounce prevents it from doubling an event already emitted on
        // PRESS DOWN.
        let named: Option<TouchEvent> = match gesture {
            GESTURE_SWIPE_UP    => Some(TouchEvent::Input(InputEvent::Up)),
            GESTURE_SWIPE_DOWN  => Some(TouchEvent::Input(InputEvent::Down)),
            GESTURE_SWIPE_LEFT  => Some(TouchEvent::Input(InputEvent::Left)),
            GESTURE_SWIPE_RIGHT => Some(TouchEvent::Input(InputEvent::Right)),
            GESTURE_LONG_PRESS  => Some(TouchEvent::Input(InputEvent::PowerOffShortcut)),
            GESTURE_SINGLE_TAP  => Some(Self::map_tap(x, y)),
            _                   => None,
        };
        if let Some(ev) = named {
            return self.emit(ev);
        }

        // No named gesture: emit when a finger is present (PRESS DOWN).
        // LIFT UP reports finger_num = 0 and falls through to None, so it
        // never produces a duplicate — the debounce exists only to absorb
        // the rare LIFT-UP with stale finger_num = 1 that some chip
        // revisions send.
        if finger_num > 0 {
            return self.emit(Self::map_tap(x, y));
        }

        None
    }

    fn emit(&mut self, event: TouchEvent) -> Option<TouchEvent> {
        if self.last_event.elapsed() < DEBOUNCE {
            return None;
        }
        self.last_event = Instant::now();
        Some(event)
    }

    fn map_tap(x: u16, y: u16) -> TouchEvent {
        TouchEvent::BodyTap { x, y }
    }
}
