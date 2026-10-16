//! Battery charge, health, cycle count and power via `starship-battery` (IOKit AppleSmartBattery).

use starship_battery::units::energy::watt_hour;
use starship_battery::units::power::watt;
use starship_battery::units::ratio::percent;

use crate::sample::History;

#[derive(Clone, Debug, Default)]
pub struct BatteryStat {
    pub percent: f32,
    pub health: f32,
    pub cycles: Option<u32>,
    pub state: String,
    /// Capacity when new, in watt-hours; names the battery ("59 Wh Battery").
    pub design_wh: Option<f32>,
    /// Current charge or discharge power in watts.
    pub power_w: f32,
    pub model: Option<String>,
    pub charge_hist: History,
    pub power_hist: History,
}

/// One reading of the first battery.
pub struct BatteryReading {
    pub percent: f32,
    pub health: f32,
    pub cycles: Option<u32>,
    pub state: String,
    pub design_wh: Option<f32>,
    pub power_w: f32,
    pub model: Option<String>,
}

/// The first battery, or `None` on desktops.
pub fn read_battery() -> Option<BatteryReading> {
    let manager = starship_battery::Manager::new().ok()?;
    let b = manager.batteries().ok()?.next()?.ok()?;
    let state = match b.state() {
        starship_battery::State::Charging => "Charging",
        starship_battery::State::Discharging => "Discharging",
        starship_battery::State::Full => "Full",
        starship_battery::State::Empty => "Empty",
        _ => "Unknown",
    };
    let design = b.energy_full_design().get::<watt_hour>();
    Some(BatteryReading {
        percent: b.state_of_charge().get::<percent>().clamp(0.0, 100.0),
        health: b.state_of_health().get::<percent>().max(0.0),
        cycles: b.cycle_count(),
        state: state.to_owned(),
        design_wh: (design > 0.0).then_some(design),
        power_w: b.energy_rate().get::<watt>().abs(),
        model: b.model().map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_battery_does_not_panic() {
        if let Some(r) = read_battery() {
            assert!((0.0..=100.0).contains(&r.percent));
            assert!(r.health >= 0.0);
            assert!(r.power_w >= 0.0);
        }
    }
}
