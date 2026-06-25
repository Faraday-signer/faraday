//! Board abstraction for the ESP32-S3 family.
//!
//! Everything that differs between boards (panel controller + resolution + pins,
//! battery sensing, touch controller) is hidden behind these traits. The shared
//! event loop in [`crate::run`] is generic over them, so adding a new board is a
//! matter of supplying a binary crate that constructs concrete types implementing
//! `BoardDisplay` / `BoardTouch` / `BoardBattery` and calls `crate::run`.

use std::time::Duration;

use embedded_graphics_core::{
    draw_target::DrawTarget, geometry::OriginDimensions, pixelcolor::Rgb565,
};
use faraday_core::camera::Frame;
use faraday_core::gui::app::{BatteryStatus, InputEvent};

/// What the touch driver delivers to the main loop. `Input` is a directional /
/// action gesture; `BodyTap` is a tap at a screen coordinate.
pub enum TouchEvent {
    Input(InputEvent),
    BodyTap { x: u16, y: u16 },
}

/// The board's framebuffer/panel. The core GUI draws through the standard
/// `DrawTarget` trait; these extra methods cover the platform-specific blit,
/// backlight, and power paths the loop drives directly.
pub trait BoardDisplay: DrawTarget<Color = Rgb565> + OriginDimensions {
    /// Push the framebuffer to the panel.
    fn flush(&mut self);
    /// Blit a camera preview frame into the buffer (GUI overlay draws on top).
    fn blit_camera_frame(&mut self, frame: &Frame, bg: Rgb565);
    /// Set backlight brightness, 0–100 (`0` disables it).
    fn set_backlight(&mut self, pct: u8);
    /// Put the panel into its lowest-power state before deep sleep.
    fn sleep(&mut self);
}

/// The board's touch input source.
pub trait BoardTouch {
    /// Poll for the next touch event, if any. Called at the loop's touch rate.
    fn poll(&mut self) -> Option<TouchEvent>;
    /// Set the held-swipe repeat spacing for the current screen (`None` =
    /// one step per physical swipe).
    fn set_swipe_repeat(&mut self, repeat: Option<Duration>);
    /// Put the touch controller into deep sleep before the device powers off.
    fn sleep(&mut self);
}

/// The board's battery gauge. Encapsulates the sampling strategy (ADC divider +
/// smoothing today, a fuel-gauge IC on future boards) behind a single read.
pub trait BoardBattery {
    /// Take one sample and return the current status, or `None` when no pack is
    /// present / the reading isn't ready.
    fn sample(&mut self) -> Option<BatteryStatus>;
}
