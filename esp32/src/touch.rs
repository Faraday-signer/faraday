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
//! We emit exactly one event per physical touch using a two-stage finger
//! state machine:
//!
//!   1. On the first finger_num > 0 with finger_down = false → emit & set
//!      finger_down = true.
//!   2. On finger_num = 0 → start a lift-confirmation timer.
//!   3. If finger_num > 0 returns before LIFT_CONFIRM expires → cancel timer
//!      (spurious chip 0→1 cycle during hold, stay down).
//!   4. If LIFT_CONFIRM expires with no finger_num > 0 → set finger_down =
//!      false; the next press can fire.
//!
//! This prevents the two repeat-event paths seen in practice:
//!   • The chip briefly reports finger_num=0 mid-hold before returning to 1.
//!     The confirmation timer absorbs these short 0-pulses.
//!   • The chip reports GESTURE_SINGLE_TAP on lift (or mid-hold on some
//!     revisions). Single-tap is gated by finger_down like a plain tap.
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

// Debounce for named gestures (swipes, long press): guards against the chip
// sending the same gesture code twice in quick succession.
const DEBOUNCE: Duration = Duration::from_millis(120);

// How long finger_num must stay at 0 before we accept it as a real lift.
// The CST816D pulses finger_num=0 briefly mid-hold; 150 ms filters those out
// while still being far below any deliberate lift-and-retap timing.
const LIFT_CONFIRM: Duration = Duration::from_millis(150);

/// What the touch driver delivers to the main loop.
pub enum TouchEvent {
    Input(InputEvent),
    BodyTap { x: u16, y: u16 },
}

pub struct Touch<'d> {
    i2c: I2cDriver<'d>,
    int: PinDriver<'d, Input>,
    last_event: Instant,
    finger_down: bool,
    lifting_since: Option<Instant>,
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
            finger_down: false,
            lifting_since: None,
        }
    }

    pub fn poll(&mut self) -> Option<TouchEvent> {
        // Confirm a lift if finger_num=0 has persisted long enough.
        // Checked on every main-loop tick (every 5 ms) so the timer resolves
        // even when no new interrupt fires after the finger is lifted.
        if let Some(t) = self.lifting_since {
            if t.elapsed() >= LIFT_CONFIRM {
                self.finger_down = false;
                self.lifting_since = None;
            }
        }

        if !INT_FIRED.swap(false, Ordering::Relaxed) {
            return None;
        }
        self.int.enable_interrupt().ok();

        let mut buf = [0u8; 6];
        if self.i2c.write_read(CST816D_ADDR, &[0x01], &mut buf, 30).is_err() {
            return None;
        }

        let gesture    = buf[0];
        let finger_num = buf[1];
        let x = (((buf[2] & 0x0F) as u16) << 8) | buf[3] as u16;
        let y = (((buf[4] & 0x0F) as u16) << 8) | buf[5] as u16;

        if finger_num == 0 {
            // Potential lift: start confirmation timer (only while finger was down).
            if self.finger_down && self.lifting_since.is_none() {
                self.lifting_since = Some(Instant::now());
            }
            // Fall through: gestures reported on lift (swipes, single tap)
            // are processed below.
        } else {
            // Finger is still present — cancel any in-progress lift timer.
            self.lifting_since = None;
        }

        // Named gestures. Swipes and long press are directional/timed gestures
        // and bypass finger_down. GESTURE_SINGLE_TAP is a plain tap reported
        // by the chip on lift; gate it the same way as a body tap so it cannot
        // repeat while a finger is held.
        let named: Option<TouchEvent> = match gesture {
            GESTURE_SWIPE_UP    => Some(TouchEvent::Input(InputEvent::Up)),
            GESTURE_SWIPE_DOWN  => Some(TouchEvent::Input(InputEvent::Down)),
            GESTURE_SWIPE_LEFT  => Some(TouchEvent::Input(InputEvent::Left)),
            GESTURE_SWIPE_RIGHT => Some(TouchEvent::Input(InputEvent::Right)),
            GESTURE_LONG_PRESS  => Some(TouchEvent::Input(InputEvent::PowerOffShortcut)),
            GESTURE_SINGLE_TAP if !self.finger_down => Some(Self::map_tap(x, y)),
            GESTURE_SINGLE_TAP  => return None,
            _                   => None,
        };
        if let Some(ev) = named {
            return self.emit(ev);
        }

        // Plain tap: emit only on the first contact while finger_down is clear.
        if finger_num > 0 && !self.finger_down {
            self.finger_down = true;
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
