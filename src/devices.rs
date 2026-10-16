//! Sidebar names for drives and network interfaces, following upstream
//! `src/utils/{drive,network}.rs` ("512 GB Drive", "Wi-Fi Connection").

use std::collections::HashMap;

use crate::icons;
use crate::sampler::NetStat;

/// Parses `networksetup -listallhardwareports` into device → port name (`"en0" → "Wi-Fi"`).
pub fn parse_hardware_ports(out: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut port = None;
    for line in out.lines() {
        if let Some(p) = line.strip_prefix("Hardware Port: ") {
            port = Some(p.trim().to_owned());
        } else if let Some(dev) = line.strip_prefix("Device: ")
            && let Some(p) = port.take()
        {
            map.insert(dev.trim().to_owned(), p);
        }
    }
    map
}

/// macOS system tunnels (iCloud Private Relay, VPN apps): `utun*`, `ipsec*`, `ppp*` that are
/// not listed as a hardware port. They are merged into one "VPN Connection" entry.
pub fn is_tunnel(iface: &str, port: Option<&str>) -> bool {
    port.is_none()
        && ["utun", "ipsec", "ppp"]
            .iter()
            .any(|t| iface.starts_with(t))
}

/// Loopback, Apple's peer-to-peer Wi-Fi links and interfaces that never carried traffic
/// (idle tunnels, bridges) are noise.
pub fn visible(n: &NetStat) -> bool {
    n.name != "lo0"
        && !n.name.starts_with("awdl")
        && !n.name.starts_with("llw")
        && n.rx_total + n.tx_total > 0
}

/// The interface's hardware port, if macOS knows one.
pub fn port(n: &NetStat) -> Option<&str> {
    Some(n.port.as_str()).filter(|p| !p.is_empty())
}

/// Display name and icon for an interface, given its hardware port name if macOS knows one.
pub fn net_display_name(iface: &str, port: Option<&str>) -> (String, &'static str) {
    match port {
        Some(p) if p.contains("Wi-Fi") || p.contains("AirPort") => {
            ("Wi-Fi Connection".into(), icons::WIFI_HIGH)
        }
        Some(p) if p.contains("Ethernet") || p.contains("LAN") => {
            ("Ethernet Connection".into(), icons::NETWORK)
        }
        Some(p) => (p.to_owned(), icons::NETWORK),
        None if ["utun", "ipsec", "ppp"]
            .iter()
            .any(|v| iface.starts_with(v)) =>
        {
            ("VPN Connection".into(), icons::SHIELD_CHECK)
        }
        None => (iface.to_owned(), icons::GLOBE),
    }
}

/// Display name and icon for a drive: capacity in whole decimal units, like upstream.
pub fn drive_display_name(total: u64, removable: bool) -> (String, &'static str) {
    const UNITS: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];
    let mut v = total as f64;
    let mut unit = 0;
    while v >= 1000.0 && unit < UNITS.len() - 1 {
        v /= 1000.0;
        unit += 1;
    }
    let icon = if removable {
        icons::USB
    } else {
        icons::HARD_DRIVE
    };
    (format!("{v:.0} {} Drive", UNITS[unit]), icon)
}

/// "59 Wh Battery", like upstream; plain "Battery" when the design capacity is unknown.
pub fn battery_display_name(design_wh: Option<f32>) -> String {
    design_wh.map_or_else(|| "Battery".to_owned(), |wh| format!("{wh:.0} Wh Battery"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PORTS: &str = "
Hardware Port: Ethernet
Device: en0
Ethernet Address: aa:bb:cc:dd:ee:ff

Hardware Port: Wi-Fi
Device: en1
Ethernet Address: 11:22:33:44:55:66

Hardware Port: Thunderbolt Bridge
Device: bridge0
Ethernet Address: N/A

VLAN Configurations
===================
";

    #[test]
    fn parse_hardware_ports_maps_device_to_port() {
        let m = parse_hardware_ports(PORTS);
        assert_eq!(m.get("en0").map(String::as_str), Some("Ethernet"));
        assert_eq!(m.get("en1").map(String::as_str), Some("Wi-Fi"));
        assert_eq!(
            m.get("bridge0").map(String::as_str),
            Some("Thunderbolt Bridge")
        );
        assert_eq!(m.len(), 3);
    }

    #[test]
    fn net_display_name_cases() {
        assert_eq!(
            net_display_name("en1", Some("Wi-Fi")),
            ("Wi-Fi Connection".into(), icons::WIFI_HIGH)
        );
        assert_eq!(
            net_display_name("en5", Some("USB 10/100/1000 LAN")),
            ("Ethernet Connection".into(), icons::NETWORK)
        );
        assert_eq!(
            net_display_name("en0", Some("Ethernet")),
            ("Ethernet Connection".into(), icons::NETWORK)
        );
        assert_eq!(
            net_display_name("utun4", None),
            ("VPN Connection".into(), icons::SHIELD_CHECK)
        );
        assert_eq!(
            net_display_name("bridge0", Some("Thunderbolt Bridge")),
            ("Thunderbolt Bridge".into(), icons::NETWORK)
        );
        assert_eq!(
            net_display_name("gif0", None),
            ("gif0".into(), icons::GLOBE)
        );
    }

    #[test]
    fn battery_display_name_uses_design_capacity() {
        assert_eq!(battery_display_name(Some(59.4)), "59 Wh Battery");
        assert_eq!(battery_display_name(None), "Battery");
    }

    #[test]
    fn is_tunnel_cases() {
        assert!(is_tunnel("utun3", None));
        assert!(is_tunnel("ipsec0", None));
        assert!(is_tunnel("ppp0", None));
        assert!(!is_tunnel("en0", Some("Wi-Fi")));
        assert!(!is_tunnel("gif0", None));
        // A tunnel macOS lists as a hardware port is a real connection, not a system tunnel.
        assert!(!is_tunnel("utun9", Some("Corporate VPN")));
    }

    #[test]
    fn drive_display_name_uses_decimal_capacity() {
        assert_eq!(
            drive_display_name(512_110_190_592, false),
            ("512 GB Drive".into(), icons::HARD_DRIVE)
        );
        assert_eq!(
            drive_display_name(2_000_000_000_000, true),
            ("2 TB Drive".into(), icons::USB)
        );
    }
}
