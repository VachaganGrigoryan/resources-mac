//! Pure sampling math: counter rates, fixed-size histories and app aggregation.

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Duration;

/// Bytes per second between two cumulative counter readings.
/// A counter that went backwards (interface reset, wake from sleep) reads as 0, not a spike.
pub fn rate(prev: u64, cur: u64, elapsed: Duration) -> f64 {
    let secs = elapsed.as_secs_f64();
    if cur < prev || secs == 0.0 {
        return 0.0;
    }
    (cur - prev) as f64 / secs
}

/// Time to divide counter deltas by. `Instant` stops while the Mac sleeps, so a wall-clock gap
/// well beyond the monotonic one means we slept: return zero so rates read 0 instead of a spike.
pub fn rate_elapsed(mono: Duration, wall: Duration) -> Duration {
    if wall.saturating_sub(mono) > Duration::from_secs(2) {
        Duration::ZERO
    } else {
        mono
    }
}

pub use resources_ui::History;

#[derive(Clone, Debug)]
pub struct ProcInfo {
    pub pid: u32,
    pub ppid: Option<u32>,
    pub name: String,
    pub user: String,
    pub cpu: f32,
    pub mem: u64,
    pub threads: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct AppInfo {
    pub pid: u32,
    pub name: String,
    pub bundle_id: Option<String>,
    /// Dock icon; `Arc` keeps cloning `State` every tick cheap.
    pub icon: Option<std::sync::Arc<egui::ColorImage>>,
}

#[derive(Clone, Debug)]
pub struct AppUsage {
    pub app: AppInfo,
    pub cpu: f32,
    pub mem: u64,
    pub pids: Vec<u32>,
}

/// Sums each app's own process plus all its descendants.
/// Each pid is visited once, so a ppid cycle cannot loop forever.
/// Apps whose process no longer exists (macOS keeps auto-terminated apps listed) are dropped.
pub fn aggregate_apps(apps: &[AppInfo], procs: &[ProcInfo]) -> Vec<AppUsage> {
    let by_pid: HashMap<u32, &ProcInfo> = procs.iter().map(|p| (p.pid, p)).collect();
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for p in procs {
        if let Some(pp) = p.ppid {
            children.entry(pp).or_default().push(p.pid);
        }
    }

    apps.iter()
        .map(|app| {
            let mut usage = AppUsage {
                app: app.clone(),
                cpu: 0.0,
                mem: 0,
                pids: Vec::new(),
            };
            let mut seen = HashSet::new();
            let mut queue = VecDeque::from([app.pid]);
            while let Some(pid) = queue.pop_front() {
                if !seen.insert(pid) {
                    continue;
                }
                let Some(p) = by_pid.get(&pid) else { continue };
                usage.cpu += p.cpu;
                usage.mem += p.mem;
                usage.pids.push(pid);
                queue.extend(children.get(&pid).into_iter().flatten());
            }
            usage
        })
        .filter(|usage| !usage.pids.is_empty())
        .collect()
}

/// Every process no app owns, as one row (upstream's "System Processes"). Its pid is 0,
/// which no action accepts.
pub fn system_processes(apps: &[AppUsage], procs: &[ProcInfo]) -> AppUsage {
    let owned: HashSet<u32> = apps.iter().flat_map(|a| a.pids.iter().copied()).collect();
    let mut usage = AppUsage {
        app: AppInfo {
            pid: 0,
            name: "System Processes".into(),
            bundle_id: None,
            icon: None,
        },
        cpu: 0.0,
        mem: 0,
        pids: Vec::new(),
    };
    for p in procs.iter().filter(|p| !owned.contains(&p.pid)) {
        usage.cpu += p.cpu;
        usage.mem += p.mem;
        usage.pids.push(p.pid);
    }
    usage
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn proc(pid: u32, ppid: Option<u32>, cpu: f32, mem: u64) -> ProcInfo {
        ProcInfo {
            pid,
            ppid,
            name: format!("p{pid}"),
            user: "me".into(),
            cpu,
            mem,
            threads: None,
        }
    }

    fn app(pid: u32) -> AppInfo {
        AppInfo {
            pid,
            name: format!("app{pid}"),
            bundle_id: None,
            icon: None,
        }
    }

    #[test]
    fn rate_basic() {
        assert_eq!(rate(1000, 3000, Duration::from_secs(2)), 1000.0);
    }

    #[test]
    fn rate_counter_reset_is_zero() {
        assert_eq!(rate(5000, 10, Duration::from_secs(1)), 0.0);
    }

    #[test]
    fn rate_zero_elapsed_is_zero() {
        assert_eq!(rate(0, 100, Duration::ZERO), 0.0);
    }

    #[test]
    fn aggregate_sums_descendants() {
        let procs = [
            proc(10, Some(1), 1.0, 100),
            proc(11, Some(10), 2.0, 200),
            proc(12, Some(11), 3.0, 300),
            proc(20, Some(1), 9.0, 900),
        ];
        let out = aggregate_apps(&[app(10)], &procs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].cpu, 6.0);
        assert_eq!(out[0].mem, 600);
        let mut pids = out[0].pids.clone();
        pids.sort();
        assert_eq!(pids, [10, 11, 12]);
    }

    #[test]
    fn aggregate_handles_ppid_cycle() {
        let procs = [
            proc(10, Some(1), 1.0, 100),
            proc(11, Some(12), 2.0, 200),
            proc(12, Some(11), 3.0, 300),
        ];
        let out = aggregate_apps(&[app(10), app(11)], &procs);
        assert_eq!(out[0].pids, [10]);
        assert_eq!(out[1].mem, 500);
    }

    #[test]
    fn aggregate_drops_app_without_process() {
        // NSWorkspace still lists apps macOS already auto-terminated; they must not show as rows.
        let out = aggregate_apps(&[app(99), app(10)], &[proc(10, None, 1.0, 100)]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].app.pid, 10);
    }

    #[test]
    fn system_processes_collects_unowned_pids() {
        let procs = [
            proc(10, Some(1), 1.0, 100),
            proc(11, Some(10), 2.0, 200),
            proc(1, None, 0.5, 50),
            proc(30, Some(1), 4.0, 400),
        ];
        let apps = aggregate_apps(&[app(10)], &procs);
        let sys = system_processes(&apps, &procs);
        assert_eq!(sys.app.name, "System Processes");
        assert_eq!(sys.app.pid, 0);
        let mut pids = sys.pids.clone();
        pids.sort();
        assert_eq!(pids, [1, 30]);
        assert_eq!(sys.cpu, 4.5);
        assert_eq!(sys.mem, 450);
    }

    #[test]
    fn rate_elapsed_normal_tick() {
        let one = Duration::from_secs(1);
        assert_eq!(rate_elapsed(one, Duration::from_millis(1200)), one);
    }

    #[test]
    fn rate_elapsed_after_sleep_is_zero() {
        // Instant stops while the Mac sleeps; counters keep the traffic from dark wake.
        assert_eq!(
            rate_elapsed(Duration::from_secs(1), Duration::from_secs(3600)),
            Duration::ZERO
        );
    }
}
