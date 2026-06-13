//! External Li-ion battery monitoring — charge level only.
//!
//! The pack voltage is sampled on an ADC pin through a resistor divider, and the
//! charge level comes straight from that voltage. There is no charging/bolt
//! state: it can't be detected on this board (see below).
//!
//! HARDWARE (Waveshare ESP32-S3-Touch-LCD-2, confirmed against the schematic):
//!   * Battery voltage is on GPIO5 (ADC1_CH4), net `BAT_ADC`, tapped off a
//!     R19=200k / R20=100k divider from `VBAT`, so `V_pack = 3 × V_adc`.
//!     See [`DIVIDER`].
//!   * The charger (ETA6098) drives only the red charge LED; its STAT line is
//!     not on any GPIO, and there's no fuel-gauge IC.
//!
//! Two things therefore can't be known from voltage alone, both confirmed by
//! measurement on real hardware — hence the level-only icon:
//!   * **Charging** can't be detected — no status line, and on USB the terminal
//!     just sits at the charger's regulation voltage. To drive a bolt truthfully
//!     you'd wire a VBUS-sense GPIO or add a fuel gauge.
//!   * **Presence** can't be detected — on USB a full pack (~4.23–4.245 V) and
//!     an empty connector (~4.224–4.230 V) read the same, so the gauge shows
//!     whenever the line is powered. A fuel gauge (e.g. MAX17048) fixes both.

use faraday_core::gui::app::BatteryStatus;

/// Resistor-divider ratio: `V_pack = V_adc × DIVIDER`. R19=200k + R20=100k
/// → (200k + 100k) / 100k = 3.
pub const DIVIDER: f32 = 3.0;

/// Pack voltage (mV) mapped to 0% — a conservative Li-ion empty cutoff.
const BAT_MIN_MV: f32 = 3300.0;
/// Pack voltage (mV) mapped to 100% — nominal Li-ion full charge.
const BAT_MAX_MV: f32 = 4200.0;
/// Below this measured pack voltage we assume no battery is connected (the
/// divider floats near 0 V with nothing attached, i.e. powered off USB too).
const BAT_PRESENT_MV: f32 = 2500.0;

/// EMA weight for each new sample — lower = smoother, more lag. Steadies the
/// percentage needle against ADC noise and brief load sag (e.g. camera on).
const EMA_ALPHA: f32 = 0.35;

/// Map a pack voltage (mV) to a 0..=100 charge percentage.
pub fn percent_from_pack_mv(pack_mv: f32) -> u8 {
    (((pack_mv - BAT_MIN_MV) / (BAT_MAX_MV - BAT_MIN_MV)) * 100.0)
        .clamp(0.0, 100.0)
        .round() as u8
}

/// Stateful battery tracker: smooths the ADC voltage and derives the charge
/// level. Charging is not inferred (see the module docs).
pub struct BatteryMonitor {
    /// Exponentially-smoothed pack voltage (mV); `None` until the first sample.
    ema_mv: Option<f32>,
}

impl BatteryMonitor {
    pub fn new() -> Self {
        Self { ema_mv: None }
    }

    /// Feed one ADC reading (mV at the divider tap) and return the current
    /// status, or `None` when no pack is present.
    pub fn update(&mut self, adc_mv: u16) -> Option<BatteryStatus> {
        let pack_mv = adc_mv as f32 * DIVIDER;
        if pack_mv < BAT_PRESENT_MV {
            self.ema_mv = None;
            return None;
        }

        let ema = match self.ema_mv {
            Some(prev) => EMA_ALPHA * pack_mv + (1.0 - EMA_ALPHA) * prev,
            None => pack_mv,
        };
        self.ema_mv = Some(ema);

        Some(BatteryStatus {
            percent: percent_from_pack_mv(ema),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive the monitor with a fixed pack voltage for `n` samples, returning
    /// the final status.
    fn settle(m: &mut BatteryMonitor, pack_mv: f32, n: usize) -> Option<BatteryStatus> {
        let adc = (pack_mv / DIVIDER).round() as u16;
        let mut last = None;
        for _ in 0..n {
            last = m.update(adc);
        }
        last
    }

    #[test]
    fn no_battery_below_present_floor() {
        let mut m = BatteryMonitor::new();
        // 100 mV tap × 3 = 300 mV pack → treated as "no battery".
        assert_eq!(m.update(100), None);
    }

    #[test]
    fn percent_clamps_and_scales() {
        assert_eq!(percent_from_pack_mv(4200.0), 100); // full
        assert_eq!(percent_from_pack_mv(3300.0), 0); // empty
        assert_eq!(percent_from_pack_mv(4500.0), 100); // over-full clamps
        assert_eq!(percent_from_pack_mv(3750.0), 50); // midpoint
        assert_eq!(percent_from_pack_mv(3480.0), 20); // 20% boundary
    }

    #[test]
    fn high_voltage_pack_is_shown() {
        // On USB the terminal sits ~4.23 V whether full or empty — we can't tell
        // them apart, so the gauge always shows rather than risk hiding a pack.
        let mut m = BatteryMonitor::new();
        let s = settle(&mut m, 4230.0, 5);
        assert!(s.is_some());
        assert_eq!(s.unwrap().percent, 100);
    }
}
