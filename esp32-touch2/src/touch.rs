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
//!     revisions). We ignore it and derive taps from finger travel instead.
//!
//! Tap vs. swipe: a body tap is emitted on *lift*, not on press-down, and only
//! when the finger never travelled beyond TAP_SLOP from its press-down point
//! and no swipe gesture fired during the contact. A swipe (or any drag past the
//! slop) marks the touch as moved, so swiping a scrollable list never selects a
//! row. Emitting on lift is what makes the distinction possible — at press-down
//! we cannot yet tell a tap from the start of a swipe.
//!
//! Gestures (vertical inverted — content tracks the finger, like touch-native
//! scrolling: dragging up moves the selection down; horizontal unchanged):
//!     Swipe up    = Down
//!     Swipe down  = Up
//!     Swipe left  = Left
//!     Swipe right = Right
//!   Long press (>1.5 s) = PowerOffShortcut

use esp32_common::{BoardTouch, TouchEvent};
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
const GESTURE_LONG_PRESS: u8 = 0x0C;

static INT_FIRED: AtomicBool = AtomicBool::new(false);

// Debounce for named gestures (swipes, long press): guards against the chip
// sending the same gesture code twice in quick succession.
const DEBOUNCE: Duration = Duration::from_millis(120);

// How long finger_num must stay at 0 before we accept it as a real lift.
// The CST816D pulses finger_num=0 briefly mid-hold; 150 ms filters those out
// while still being far below any deliberate lift-and-retap timing.
const LIFT_CONFIRM: Duration = Duration::from_millis(150);

// Finger travel (px) from the press-down point beyond which a touch counts as a
// swipe/drag rather than a tap. Below it we emit a tap on lift; at or above it
// the tap is suppressed so dragging a scrollable list never selects a row.
const TAP_SLOP: u16 = 24;

pub struct Touch<'d> {
    i2c: I2cDriver<'d>,
    int: PinDriver<'d, Input>,
    last_event: Instant,
    finger_down: bool,
    lifting_since: Option<Instant>,
    // Press-down point of the current touch (recorded on first contact, cleared
    // on confirmed lift). Some(..) while a finger is down.
    press_origin: Option<(u16, u16)>,
    // Set once the finger travels past TAP_SLOP or a swipe gesture fires; gates
    // the tap that would otherwise be emitted on lift.
    moved: bool,
    // Throttles the repeat gesture reports the CST816D emits while a swipe is
    // held. `swipe_repeat` is the minimum spacing between successive steps
    // within one contact (`None` = discrete: a single step per physical swipe).
    // `last_gesture` is when the last step fired. Both reset on press-down so a
    // fresh swipe always steps immediately. Set per-screen by the main loop via
    // `set_swipe_repeat`.
    swipe_repeat: Option<Duration>,
    last_gesture: Option<Instant>,
}

impl<'d> Touch<'d> {
    pub fn new(mut i2c: I2cDriver<'d>, mut int: PinDriver<'d, Input>) -> Self {
        // Best-effort wake from a prior deep sleep (power-mode register 0xA5 back
        // to 0x00 = normal). We have no touch-RST line, so if the chip was put in
        // deep sleep before the device slept and its I2C is still alive, this
        // brings it back; harmless on a freshly powered chip.
        let _ = i2c.write(CST816D_ADDR, &[0xA5, 0x00], 30);

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
            press_origin: None,
            moved: false,
            swipe_repeat: None,
            last_gesture: None,
        }
    }

    fn emit(&mut self, event: TouchEvent) -> Option<TouchEvent> {
        if self.last_event.elapsed() < DEBOUNCE {
            return None;
        }
        self.last_event = Instant::now();
        Some(event)
    }
}

impl<'d> BoardTouch for Touch<'d> {
    /// Set the held-swipe repeat spacing for the current screen. `Some(interval)`
    /// lets a held swipe keep stepping at most once per `interval` (continuous
    /// scrolling, e.g. menus); `None` latches it to one step per physical swipe
    /// (the paper-backup walkthrough). Applied on the next `poll`.
    fn set_swipe_repeat(&mut self, repeat: Option<Duration>) {
        self.swipe_repeat = repeat;
    }

    fn poll(&mut self) -> Option<TouchEvent> {
        // Confirm a lift if finger_num=0 has persisted long enough.
        // Checked on every main-loop tick (every 5 ms) so the timer resolves
        // even when no new interrupt fires after the finger is lifted.
        if let Some(t) = self.lifting_since {
            if t.elapsed() >= LIFT_CONFIRM {
                self.finger_down = false;
                self.lifting_since = None;
                // Lift confirmed: a finger that pressed and lifted without
                // travelling past TAP_SLOP and without a swipe gesture is a tap.
                // Emit it at the press-down point (lift coords are often stale).
                let origin = self.press_origin.take();
                if !self.moved {
                    if let Some((x, y)) = origin {
                        return self.emit(TouchEvent::BodyTap { x, y });
                    }
                }
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
            // Fall through: swipe gestures are reported on lift and processed below.
        } else {
            // Finger is still present — cancel any in-progress lift timer.
            self.lifting_since = None;
            match self.press_origin {
                Some((ox, oy)) => {
                    // Track drag distance from the press-down point so a swipe
                    // the chip never classifies still suppresses the tap.
                    if x.abs_diff(ox) > TAP_SLOP || y.abs_diff(oy) > TAP_SLOP {
                        self.moved = true;
                    }
                }
                None => {
                    // First contact of a new touch: record the origin.
                    self.press_origin = Some((x, y));
                    self.moved = false;
                    self.last_gesture = None;
                    self.finger_down = true;
                }
            }
        }

        // Named gestures. Swipes and long press are directional/timed gestures;
        // each marks the touch as moved so the trailing lift emits no tap. Taps
        // are derived from finger travel on lift, not from GESTURE_SINGLE_TAP.
        let named: Option<TouchEvent> = match gesture {
            GESTURE_SWIPE_UP    => { self.moved = true; Some(TouchEvent::Input(InputEvent::Down)) }
            GESTURE_SWIPE_DOWN  => { self.moved = true; Some(TouchEvent::Input(InputEvent::Up)) }
            GESTURE_SWIPE_LEFT  => { self.moved = true; Some(TouchEvent::Input(InputEvent::Left)) }
            GESTURE_SWIPE_RIGHT => { self.moved = true; Some(TouchEvent::Input(InputEvent::Right)) }
            GESTURE_LONG_PRESS  => { self.moved = true; Some(TouchEvent::Input(InputEvent::PowerOffShortcut)) }
            _                   => None,
        };
        if let Some(ev) = named {
            // Throttle held-swipe repeats: the first gesture of a contact always
            // fires; later reports fire only once `swipe_repeat` has elapsed
            // (never, when discrete) so a held swipe scrolls at a steady pace
            // instead of racing at the poll rate.
            let allow = match (self.swipe_repeat, self.last_gesture) {
                (_, None) => true,
                (None, Some(_)) => false,
                (Some(interval), Some(t)) => t.elapsed() >= interval,
            };
            if !allow {
                return None;
            }
            self.last_gesture = Some(Instant::now());
            return self.emit(ev);
        }

        None
    }

    /// Put the CST816D into deep sleep (power-mode register 0xA5 = 0x03) so it
    /// stops scanning while the device is powered off (~1 mA saved). NOTE: many
    /// CST816 variants only leave deep sleep on a hardware RST pulse, and this
    /// board has no touch-RST line we drive — on wake we rely on the best-effort
    /// I2C wake in `new()`. If the panel is dead after a sleep cycle, that's why.
    fn sleep(&mut self) {
        let _ = self.i2c.write(CST816D_ADDR, &[0xA5, 0x03], 30);
    }
}
