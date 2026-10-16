//! Battery charge, health and cycle count via `starship-battery` (IOKit AppleSmartBattery).

use starship_battery::units::ratio::percent;

use crate::sample::History;

#[derive(Clone, Debug, Default)]
pub struct BatteryStat {
    pub percent: f32,
    pub health: f32,
    pub cycles: Option<u32>,
    pub state: String,
    pub charge_hist: History,
}

/// `(charge %, health %, cycle count, state)` of the first battery, or `None` on desktops.
pub fn read_battery() -> Option<(f32, f32, Option<u32>, String)> {
    let manager = starship_battery::Manager::new().ok()?;
    let b = manager.batteries().ok()?.next()?.ok()?;
    let state = match b.state() {
        starship_battery::State::Charging => "Charging",
        starship_battery::State::Discharging => "Discharging",
        starship_battery::State::Full => "Full",
        starship_battery::State::Empty => "Empty",
        _ => "Unknown",
    };
    Some((
        b.state_of_charge().get::<percent>().clamp(0.0, 100.0),
        b.state_of_health().get::<percent>().max(0.0),
        b.cycle_count(),
        state.to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_battery_does_not_panic() {
        if let Some((pct, health, _, _)) = read_battery() {
            assert!((0.0..=100.0).contains(&pct));
            assert!(health >= 0.0);
        }
    }
}
