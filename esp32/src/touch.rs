//! CST816D capacitive touch driver via ESP-IDF I2C.
//!
//! Pin mapping for Waveshare ESP32-S3-Touch-LCD-2:
//!   SDA  = GPIO 48
//!   SCL  = GPIO 47
//!   INT  = GPIO 46
//!
//! Touch zone mapping (240x320 portrait):
//!   Body area (y < 288): tap = Confirm
//!   Bottom bar (y >= 288):
//!     x < 80  = Back
//!     80..160 = Secondary
//!     x >= 160 = Confirm
//!   Gestures:
//!     Swipe up    = Up
//!     Swipe down  = Down
//!     Swipe left  = Left
//!     Swipe right = Right
//!   Long press (>1.5s) = PowerOffShortcut

use esp_idf_hal::gpio::{Input, PinDriver};
use esp_idf_hal::i2c::I2cDriver;
use faraday_core::gui::app::InputEvent;

const CST816D_ADDR: u8 = 0x15;

const GESTURE_NONE: u8 = 0x00;
const GESTURE_SWIPE_UP: u8 = 0x01;
const GESTURE_SWIPE_DOWN: u8 = 0x02;
const GESTURE_SWIPE_LEFT: u8 = 0x03;
const GESTURE_SWIPE_RIGHT: u8 = 0x04;
const GESTURE_SINGLE_TAP: u8 = 0x05;
const GESTURE_LONG_PRESS: u8 = 0x0C;

const FOOTER_Y: u16 = 288;
const THIRD_WIDTH: u16 = 80;

pub struct Touch<'d> {
    i2c: I2cDriver<'d>,
    _int: PinDriver<'d, Input>,
}

impl<'d> Touch<'d> {
    pub fn new(
        i2c: I2cDriver<'d>,
        int: PinDriver<'d, Input>,
    ) -> Self {
        Self {
            i2c,
            _int: int,
        }
    }

    pub fn poll(&mut self) -> Option<InputEvent> {
        let mut buf = [0u8; 6];
        if self.i2c.write_read(CST816D_ADDR, &[0x01], &mut buf, 100).is_err() {
            return None;
        }

        let gesture = buf[0];
        let finger_num = buf[1];
        let x = (((buf[2] & 0x0F) as u16) << 8) | buf[3] as u16;
        let y = (((buf[4] & 0x0F) as u16) << 8) | buf[5] as u16;

        if finger_num == 0 && gesture == GESTURE_NONE {
            return None;
        }

        match gesture {
            GESTURE_SWIPE_UP => Some(InputEvent::Up),
            GESTURE_SWIPE_DOWN => Some(InputEvent::Down),
            GESTURE_SWIPE_LEFT => Some(InputEvent::Left),
            GESTURE_SWIPE_RIGHT => Some(InputEvent::Right),
            GESTURE_LONG_PRESS => Some(InputEvent::PowerOffShortcut),
            GESTURE_SINGLE_TAP => Some(Self::map_tap(x, y)),
            _ => {
                if finger_num > 0 {
                    Some(Self::map_tap(x, y))
                } else {
                    None
                }
            }
        }
    }

    fn map_tap(x: u16, y: u16) -> InputEvent {
        if y >= FOOTER_Y {
            if x < THIRD_WIDTH {
                InputEvent::Back
            } else if x < THIRD_WIDTH * 2 {
                InputEvent::Secondary
            } else {
                InputEvent::Confirm
            }
        } else {
            InputEvent::Confirm
        }
    }
}
