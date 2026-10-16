//! Stats the menu bar and desktop widget can show, and which of them suit this Mac right now.

use serde::{Deserialize, Serialize};

use crate::devices;
use crate::icons;
use crate::sample::History;
use crate::sampler::{DiskStat, NetStat, State};
use crate::theme::color;
use crate::widgets::{Series, fmt_bytes, fmt_rate, fmt_used};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stat {
    Cpu,
    Memory,
    Swap,
    Gpu,
    Network,
    Disk,
    Battery,
}

impl Stat {
    pub const ALL: [Self; 7] = [
        Self::Cpu,
        Self::Memory,
        Self::Swap,
        Self::Gpu,
        Self::Network,
        Self::Disk,
        Self::Battery,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Cpu => "Processor",
            Self::Memory => "Memory",
            Self::Swap => "Swap",
            Self::Gpu => "GPU",
            Self::Network => "Network",
            Self::Disk => "Drive",
            Self::Battery => "Battery",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Cpu => icons::CPU,
            Self::Memory | Self::Swap => icons::MEMORY,
            Self::Gpu => icons::GRAPHICS_CARD,
            Self::Network => icons::NETWORK,
            Self::Disk => icons::HARD_DRIVE,
            Self::Battery => icons::BATTERY_HIGH,
        }
    }
}

fn pct(used: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        used as f64 / total as f64 * 100.0
    }
}

fn mean(h: &History) -> f64 {
    if h.is_empty() {
        0.0
    } else {
        h.values().sum::<f64>() / h.len() as f64
    }
}

/// The connection that carried the most traffic since boot: what people mean by "network".
pub fn primary_net(s: &State) -> Option<&NetStat> {
    s.nets
        .iter()
        .filter(|n| devices::visible(n) && !devices::is_tunnel(&n.name, devices::port(n)))
        .max_by_key(|n| n.rx_total + n.tx_total)
}

/// The startup disk, else the first drive.
pub fn primary_disk(s: &State) -> Option<&DiskStat> {
    s.disks.iter().find(|d| d.mount == "/").or(s.disks.first())
}

/// Bytes per second in at most four characters (`"125B"`, `"1.2K"`, `"34M"`): menu bar space
/// beside the notch is scarce, and the stat's icon already says it is a rate.
fn compact(bps: f64) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    let mut v = bps.max(0.0);
    let mut unit = 0;
    while v >= 999.5 && unit < UNITS.len() - 1 {
        v /= 1000.0;
        unit += 1;
    }
    if unit > 0 && v < 9.95 {
        format!("{v:.1}{}", UNITS[unit])
    } else {
        format!("{v:.0}{}", UNITS[unit])
    }
}

/// The bare current value (`"23%"`, `"↓1.2 MB/s ↑20 kB/s"`); `None` when this Mac doesn't
/// report the stat.
pub fn value(stat: Stat, s: &State) -> Option<String> {
    Some(match stat {
        Stat::Cpu => format!("{:.0}%", s.cpu_total.last().unwrap_or(0.0)),
        Stat::Memory => format!("{:.0}%", pct(s.mem_used, s.mem_total)),
        Stat::Swap => fmt_bytes(s.swap_used),
        Stat::Gpu => format!("{:.0}%", s.gpu.as_ref()?.util_hist.last().unwrap_or(0.0)),
        Stat::Network => {
            let n = primary_net(s)?;
            format!("↓{} ↑{}", compact(n.rx_bps), compact(n.tx_bps))
        }
        Stat::Disk => {
            let d = primary_disk(s)?;
            format!("R{} W{}", compact(d.read_bps), compact(d.write_bps))
        }
        Stat::Battery => format!("{:.0}%", s.battery.as_ref()?.percent),
    })
}

/// SF Symbol shown before the value in the menu bar. Battery follows its charge level.
pub fn symbol(stat: Stat, s: &State) -> &'static str {
    match stat {
        Stat::Cpu => "cpu",
        Stat::Memory => "memorychip",
        Stat::Swap => "arrow.left.arrow.right",
        Stat::Gpu => "cube.transparent",
        Stat::Network => "network",
        Stat::Disk => "internaldrive",
        Stat::Battery => match s.battery.as_ref() {
            Some(b) if b.state == "Charging" => "battery.100.bolt",
            Some(b) => match (b.percent / 25.0).round() as u32 {
                0 => "battery.0",
                1 => "battery.25",
                2 => "battery.50",
                3 => "battery.75",
                _ => "battery.100",
            },
            None => "battery.100",
        },
    }
}

/// Text-only menu bar label (`"CPU 23%"`), for Macs without the stat's SF Symbol.
pub fn short(stat: Stat, s: &State) -> Option<String> {
    let tag = match stat {
        Stat::Cpu => "CPU",
        Stat::Memory => "MEM",
        Stat::Swap => "SWAP",
        Stat::Gpu => "GPU",
        Stat::Network => "NET",
        Stat::Disk => "DISK",
        Stat::Battery => "BAT",
    };
    Some(format!("{tag} {}", value(stat, s)?))
}

/// In-app preview of the menu bar: each stat's icon and value, skipping ones this Mac
/// doesn't report.
pub fn title(stats: &[Stat], s: &State) -> String {
    let parts: Vec<String> = stats
        .iter()
        .filter_map(|&st| Some(format!("{} {}", st.icon(), value(st, s)?)))
        .collect();
    if parts.is_empty() {
        "Resources".into()
    } else {
        parts.join("   ")
    }
}

/// What a desktop widget card shows for one stat.
pub struct Card<'a> {
    pub value: String,
    pub series: Vec<Series<'a>>,
    pub y_max: Option<f64>,
}

pub fn card(stat: Stat, s: &State) -> Option<Card<'_>> {
    Some(match stat {
        Stat::Cpu => Card {
            value: format!("{:.0} %", s.cpu_total.last().unwrap_or(0.0)),
            series: vec![(&s.cpu_total, color::CPU)],
            y_max: Some(100.0),
        },
        Stat::Memory => Card {
            value: fmt_used(s.mem_used, s.mem_total),
            series: vec![(&s.mem_hist, color::MEMORY)],
            y_max: Some(100.0),
        },
        Stat::Swap => Card {
            value: fmt_used(s.swap_used, s.swap_total),
            series: vec![(&s.swap_hist, color::SWAP)],
            y_max: Some(100.0),
        },
        Stat::Gpu => {
            let g = s.gpu.as_ref()?;
            Card {
                value: format!("{:.0} %", g.util_hist.last().unwrap_or(0.0)),
                series: vec![(&g.util_hist, color::GPU)],
                y_max: Some(100.0),
            }
        }
        Stat::Network => {
            let n = primary_net(s)?;
            Card {
                value: format!("↓ {} · ↑ {}", fmt_rate(n.rx_bps), fmt_rate(n.tx_bps)),
                series: vec![(&n.rx_hist, color::NETWORK), (&n.tx_hist, color::TX)],
                y_max: None,
            }
        }
        Stat::Disk => {
            let d = primary_disk(s)?;
            Card {
                value: format!("R {} · W {}", fmt_rate(d.read_bps), fmt_rate(d.write_bps)),
                series: vec![(&d.read_hist, color::DRIVE), (&d.write_hist, color::WRITE)],
                y_max: None,
            }
        }
        Stat::Battery => {
            let b = s.battery.as_ref()?;
            Card {
                value: format!("{:.0} % · {}", b.percent, b.state),
                series: vec![(&b.charge_hist, color::BATTERY)],
                y_max: Some(100.0),
            }
        }
    })
}

/// Stats worth watching on this Mac right now, most pressing first, each with the reason.
/// Processor and Memory are always offered as an everyday baseline.
pub fn suggest(s: &State) -> Vec<(Stat, &'static str)> {
    const MB: f64 = 1_000_000.0;
    let mut out = Vec::new();
    if mean(&s.cpu_total) > 50.0 {
        out.push((Stat::Cpu, "The processor was busy over the last minute"));
    }
    if pct(s.mem_used, s.mem_total) > 75.0 {
        out.push((Stat::Memory, "Memory is more than 75 % full"));
    }
    if pct(s.swap_used, s.swap_total) > 25.0 {
        out.push((Stat::Swap, "Your Mac is swapping memory to disk"));
    }
    if s.gpu.as_ref().is_some_and(|g| mean(&g.util_hist) > 20.0) {
        out.push((Stat::Gpu, "The GPU is in use"));
    }
    if s.battery.as_ref().is_some_and(|b| b.state == "Discharging") {
        out.push((Stat::Battery, "Running on battery"));
    }
    if primary_net(s).is_some_and(|n| n.rx_hist.max() + n.tx_hist.max() > MB) {
        out.push((Stat::Network, "Network traffic is high"));
    }
    if primary_disk(s).is_some_and(|d| d.read_hist.max() + d.write_hist.max() > 10.0 * MB) {
        out.push((Stat::Disk, "The drive is busy"));
    }
    for base in [Stat::Cpu, Stat::Memory] {
        if !out.iter().any(|(st, _)| *st == base) {
            out.push((base, "A good everyday overview"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collect::battery::BatteryStat;

    fn stats(out: &[(Stat, &str)]) -> Vec<Stat> {
        out.iter().map(|(s, _)| *s).collect()
    }

    #[test]
    fn idle_mac_suggests_cpu_and_memory() {
        assert_eq!(
            stats(&suggest(&State::default())),
            [Stat::Cpu, Stat::Memory]
        );
    }

    #[test]
    fn full_memory_and_battery_come_first() {
        let s = State {
            mem_total: 100,
            mem_used: 90,
            battery: Some(BatteryStat {
                state: "Discharging".into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            stats(&suggest(&s)),
            [Stat::Memory, Stat::Battery, Stat::Cpu]
        );
    }

    #[test]
    fn busy_cpu_is_suggested_once() {
        let mut s = State::default();
        for _ in 0..10 {
            s.cpu_total.push(80.0);
        }
        assert_eq!(stats(&suggest(&s)), [Stat::Cpu, Stat::Memory]);
    }

    #[test]
    fn missing_devices_have_no_text() {
        let s = State::default();
        assert_eq!(short(Stat::Gpu, &s), None);
        assert_eq!(short(Stat::Battery, &s), None);
        assert_eq!(short(Stat::Network, &s), None);
        assert!(card(Stat::Disk, &s).is_none());
    }

    #[test]
    fn title_joins_available_stats() {
        let mut s = State {
            mem_total: 200,
            mem_used: 50,
            ..Default::default()
        };
        s.cpu_total.push(23.4);
        assert_eq!(
            title(&[Stat::Cpu, Stat::Gpu, Stat::Memory], &s),
            format!("{} 23%   {} 25%", icons::CPU, icons::MEMORY)
        );
        assert_eq!(title(&[Stat::Gpu], &s), "Resources");
        assert_eq!(short(Stat::Cpu, &s).as_deref(), Some("CPU 23%"));
    }

    #[test]
    fn compact_rates_stay_short() {
        assert_eq!(compact(0.0), "0B");
        assert_eq!(compact(125.0), "125B");
        assert_eq!(compact(1_234.0), "1.2K");
        assert_eq!(compact(999_600.0), "1.0M");
        assert_eq!(compact(34_000_000.0), "34M");
    }

    #[test]
    fn battery_symbol_follows_charge() {
        let battery = |percent, state: &str| State {
            battery: Some(BatteryStat {
                percent,
                state: state.into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            symbol(Stat::Battery, &battery(81.0, "Discharging")),
            "battery.75"
        );
        assert_eq!(
            symbol(Stat::Battery, &battery(5.0, "Discharging")),
            "battery.0"
        );
        assert_eq!(
            symbol(Stat::Battery, &battery(40.0, "Charging")),
            "battery.100.bolt"
        );
        assert_eq!(
            value(Stat::Battery, &battery(81.4, "Full")).as_deref(),
            Some("81%")
        );
    }
}
