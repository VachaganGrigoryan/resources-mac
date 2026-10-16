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

/// Ring buffer of the most recent `cap` samples.
#[derive(Clone, Debug)]
pub struct History {
    cap: usize,
    buf: VecDeque<f64>,
}

impl History {
    pub fn new(cap: usize) -> Self {
        Self {
            cap,
            buf: VecDeque::with_capacity(cap),
        }
    }

    pub fn push(&mut self, v: f64) {
        if self.buf.len() == self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(v);
    }

    pub fn values(&self) -> impl Iterator<Item = f64> + '_ {
        self.buf.iter().copied()
    }

    pub fn last(&self) -> Option<f64> {
        self.buf.back().copied()
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new(crate::HISTORY_LEN)
    }
}

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
    fn history_caps_at_60() {
        let mut h = History::new(60);
        for i in 0..61 {
            h.push(i as f64);
        }
        assert_eq!(h.len(), 60);
        assert_eq!(h.values().next(), Some(1.0));
        assert_eq!(h.last(), Some(60.0));
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
