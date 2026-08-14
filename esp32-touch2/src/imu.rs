//! QMI8658 6-axis IMU driver — shake detection for the light/dark theme
//! Easter egg. Shares the CST816D's I2C bus (SDA=48, SCL=47); the chip
//! straps to address 0x6A or 0x6B, so `probe` tries both and a board
//! without the chip simply never toggles.
//!
//! A shake is several large accelerometer spikes in a short window: at
//! rest total acceleration sits at 1g, so `| |a| - 1g |` past a threshold
//! marks a spike, three spikes inside the window fire the gesture, and a
//! cooldown keeps one physical shake from toggling twice.

use esp32_common::BoardImu;
use esp_idf_hal::i2c::I2cDriver;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

const WHO_AM_I: u8 = 0x00; // reads 0x05 on a QMI8658
const CHIP_ID: u8 = 0x05;
const CTRL1: u8 = 0x02;
const CTRL2: u8 = 0x03;
const CTRL7: u8 = 0x08;
const AX_L: u8 = 0x35; // AX/AY/AZ, six bytes little-endian

/// LSB per g at the ±4g full scale configured in `probe` (32768 / 4).
const LSB_PER_G: f32 = 8192.0;

/// Deviation from 1g that counts as one spike.
const SPIKE_G: f32 = 0.6;
/// Spikes needed inside the window to call it a shake.
const SPIKES_NEEDED: u8 = 3;
/// Window the spikes must fall into.
const WINDOW_MS: u128 = 700;
/// Minimum spacing between counted spikes (one waggle = one spike).
const SPIKE_GAP_MS: u128 = 60;
/// Dead time after a fired shake.
const COOLDOWN_MS: u128 = 1200;

pub struct Imu<'d> {
    i2c: Rc<RefCell<I2cDriver<'d>>>,
    addr: u8,
    spikes: u8,
    window_start: Option<Instant>,
    last_spike: Option<Instant>,
    fired_at: Option<Instant>,
}

impl<'d> Imu<'d> {
    /// Find and configure the QMI8658: accelerometer at ±4g / 58.75 Hz,
    /// gyroscope off. Returns `None` when the chip isn't present.
    pub fn probe(i2c: Rc<RefCell<I2cDriver<'d>>>) -> Option<Self> {
        let addr = [0x6Au8, 0x6B].into_iter().find(|&a| {
            let mut id = [0u8; 1];
            i2c.borrow_mut()
                .write_read(a, &[WHO_AM_I], &mut id, 30)
                .is_ok()
                && id[0] == CHIP_ID
        })?;
        {
            let mut bus = i2c.borrow_mut();
            // CTRL1: register address auto-increment, for the 6-byte burst read.
            let _ = bus.write(addr, &[CTRL1, 0x40], 30);
            // CTRL2: accel full scale ±4g (aFS=001), ODR 58.75 Hz (aODR=0111).
            let _ = bus.write(addr, &[CTRL2, 0x17], 30);
            // CTRL7: accelerometer on, gyroscope off.
            let _ = bus.write(addr, &[CTRL7, 0x01], 30);
        }
        log::info!("imu: QMI8658 at 0x{addr:02X}");
        Some(Self {
            i2c,
            addr,
            spikes: 0,
            window_start: None,
            last_spike: None,
            fired_at: None,
        })
    }
}

impl BoardImu for Imu<'_> {
    fn shake(&mut self) -> bool {
        if let Some(t) = self.fired_at {
            if t.elapsed().as_millis() < COOLDOWN_MS {
                return false;
            }
            self.fired_at = None;
        }

        let mut raw = [0u8; 6];
        if self
            .i2c
            .borrow_mut()
            .write_read(self.addr, &[AX_L], &mut raw, 30)
            .is_err()
        {
            return false;
        }
        let g = |i: usize| i16::from_le_bytes([raw[i], raw[i + 1]]) as f32 / LSB_PER_G;
        let (ax, ay, az) = (g(0), g(2), g(4));
        let dev = ((ax * ax + ay * ay + az * az).sqrt() - 1.0).abs();

        // Stale window: start over.
        if let Some(w) = self.window_start {
            if w.elapsed().as_millis() > WINDOW_MS {
                self.spikes = 0;
                self.window_start = None;
            }
        }

        if dev > SPIKE_G
            && self
                .last_spike
                .is_none_or(|t| t.elapsed().as_millis() >= SPIKE_GAP_MS)
        {
            self.last_spike = Some(Instant::now());
            self.window_start.get_or_insert_with(Instant::now);
            self.spikes += 1;
            if self.spikes >= SPIKES_NEEDED {
                self.spikes = 0;
                self.window_start = None;
                self.fired_at = Some(Instant::now());
                return true;
            }
        }
        false
    }
}
