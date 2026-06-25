//! BOOT-button (GPIO0) soft power control.
//!
//! GPIO0 is the ESP32-S3 BOOT push-button (active-low, external pull-up). We use
//! it as a power button:
//!   * Long-press (>= LONG_PRESS) while awake → deep sleep ("power off").
//!   * Press while asleep → ext0 wakes the chip with a full reset ("power on").
//!
//! Deep sleep powers the digital domain down, so waking is a complete power-cycle
//! reset: `main()` runs from scratch (first screen, RAM cleared) and the
//! USB-Serial/JTAG comes back up like a clean boot, so the board stays flashable
//! — a plain `esp_restart()` (CPU/system reset) does NOT re-init the USB-JTAG,
//! only a chip-level reset like this (or the physical EN button) does.
//!
//! Two things make deep sleep reliable here:
//!   * We keep an RTC pull-up on GPIO0 across sleep (the digital `Pull::Up` is
//!     dropped when the IO domain powers down); otherwise the pad floats low and
//!     instantly satisfies the ext0 wake-on-low, producing a sleep/wake loop.
//!   * We only sleep after the button is released (`wait_release`), so the still-
//!     held press doesn't wake us immediately.
//!
//! The `armed` flag stops a press that is *already held* at boot/wake from being
//! counted as a fresh long-press: the button must be seen released once before a
//! new long-press can arm — so the wake-press can't trigger an immediate
//! power-off.

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{Gpio0, Input, PinDriver, Pull};
use std::time::{Duration, Instant};

/// How long BOOT must be held to trigger power-off.
const LONG_PRESS: Duration = Duration::from_millis(1500);

pub struct PowerButton<'d> {
    pin: PinDriver<'d, Input>,
    pressed_since: Option<Instant>,
    /// True once the button has been seen released; gates long-press detection
    /// so a held press carried over from boot/wake can't fire immediately.
    armed: bool,
}

impl<'d> PowerButton<'d> {
    pub fn new(pin: Gpio0<'d>) -> Self {
        let pin = PinDriver::input(pin, Pull::Up).expect("BOOT pin init failed");
        Self { pin, pressed_since: None, armed: false }
    }

    /// True for one poll once BOOT has been held (active-low) for >= LONG_PRESS,
    /// but only after the button has been released at least once (see `armed`).
    pub fn long_pressed(&mut self) -> bool {
        let low = self.pin.is_low();
        if !self.armed {
            // Wait for a clean release before we start detecting presses.
            if !low {
                self.armed = true;
            }
            self.pressed_since = None;
            return false;
        }
        if low {
            match self.pressed_since {
                Some(t) if t.elapsed() >= LONG_PRESS => {
                    self.pressed_since = None;
                    return true;
                }
                Some(_) => {}
                None => self.pressed_since = Some(Instant::now()),
            }
        } else {
            self.pressed_since = None;
        }
        false
    }

    /// Block until BOOT is released, so sleep isn't armed (or re-entered) while
    /// the button is still down.
    pub fn wait_release(&self) {
        while self.pin.is_low() {
            FreeRtos::delay_ms(20);
        }
        FreeRtos::delay_ms(50); // debounce the release edge
    }
}

/// Drive the OV2640 PWDN line (GPIO17, active-high) high → sensor in power-down.
/// Safe to call when the camera is not initialized; a later `esp_camera_init`
/// reconfigures the pad and powers the sensor back on.
pub fn camera_power_down() {
    unsafe {
        esp_idf_sys::gpio_set_direction(
            esp_idf_sys::gpio_num_t_GPIO_NUM_17,
            esp_idf_sys::gpio_mode_t_GPIO_MODE_OUTPUT,
        );
        esp_idf_sys::gpio_set_level(esp_idf_sys::gpio_num_t_GPIO_NUM_17, 1);
    }
}

/// Power down the OV2640 and latch the pin so it stays down across deep sleep.
///
/// In deep sleep GPIO17 would otherwise float and the sensor keeps drawing
/// ~30 mA. Call after the camera driver has been dropped. The next boot re-inits
/// the camera, which releases the hold and powers the sensor back on as needed.
pub fn camera_power_down_hold() {
    camera_power_down();
    unsafe {
        esp_idf_sys::gpio_hold_en(esp_idf_sys::gpio_num_t_GPIO_NUM_17);
        esp_idf_sys::gpio_deep_sleep_hold_en();
    }
}

/// Arm ext0 wake on GPIO0 low and enter deep sleep. Does not return: the next
/// BOOT press wakes the chip via a full reset back into `main()`. The caller must
/// blank the display and wait for the button to be released first.
pub fn enter_deep_sleep() -> ! {
    unsafe {
        // Wake when BOOT pulls GPIO0 low.
        esp_idf_sys::esp_sleep_enable_ext0_wakeup(esp_idf_sys::gpio_num_t_GPIO_NUM_0, 0);
        // Hold an RTC pull-up on GPIO0 through deep sleep so the pad doesn't float
        // low (the digital Pull::Up is gone once the IO domain powers down) and
        // spuriously satisfy the wake-on-low condition. ext0 has already switched
        // the pad to RTC IO mode, so the rtc_gpio pull config applies.
        esp_idf_sys::rtc_gpio_pullup_en(esp_idf_sys::gpio_num_t_GPIO_NUM_0);
        esp_idf_sys::rtc_gpio_pulldown_dis(esp_idf_sys::gpio_num_t_GPIO_NUM_0);
        esp_idf_sys::esp_deep_sleep_start()
    }
}
