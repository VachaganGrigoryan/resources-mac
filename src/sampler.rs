//! Background sampling: refreshes sysinfo once per second into a shared [`State`].

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use sysinfo::{Disks, Networks, ProcessesToUpdate, System, Users};

use crate::collect::apps::running_apps;
use crate::collect::battery::{BatteryStat, read_battery};
use crate::collect::gpu::{GpuStat, read_gpu};
use crate::sample::{AppUsage, History, ProcInfo, aggregate_apps, rate, rate_elapsed};

pub const INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, Default)]
pub struct DiskStat {
    pub name: String,
    pub mount: String,
    pub total: u64,
    pub avail: u64,
    pub read_bps: f64,
    pub write_bps: f64,
    pub read_hist: History,
    pub write_hist: History,
}

#[derive(Clone, Debug, Default)]
pub struct NetStat {
    pub name: String,
    pub rx_bps: f64,
    pub tx_bps: f64,
    pub rx_total: u64,
    pub tx_total: u64,
    pub rx_hist: History,
    pub tx_hist: History,
}

/// Everything the UI draws. Written only by the sampler thread.
#[derive(Clone, Debug, Default)]
pub struct State {
    pub cpu_model: String,
    pub cpu_total: History,
    pub cpu_cores: Vec<History>,
    pub mem_total: u64,
    pub mem_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub mem_hist: History,
    pub disks: Vec<DiskStat>,
    pub nets: Vec<NetStat>,
    pub procs: Vec<ProcInfo>,
    pub apps: Vec<AppUsage>,
    /// `None` when this Mac exposes no GPU statistics; the page is then hidden.
    pub gpu: Option<GpuStat>,
    /// `None` on Macs without a battery; the page is then hidden.
    pub battery: Option<BatteryStat>,
}

pub struct Sampler {
    sys: System,
    disks: Disks,
    nets: Networks,
    users: Users,
    /// Previous tick on both clocks, to detect sleep (see [`rate_elapsed`]).
    last: Option<(Instant, SystemTime)>,
    disk_prev: HashMap<String, (u64, u64)>,
    net_prev: HashMap<String, (u64, u64)>,
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}

impl Sampler {
    pub fn new() -> Self {
        Self {
            sys: System::new(),
            disks: Disks::new_with_refreshed_list(),
            nets: Networks::new_with_refreshed_list(),
            users: Users::new_with_refreshed_list(),
            last: None,
            disk_prev: HashMap::new(),
            net_prev: HashMap::new(),
        }
    }

    pub fn tick(&mut self, s: &mut State) {
        let now = (Instant::now(), SystemTime::now());
        let elapsed = self.last.map_or(Duration::ZERO, |(mono, wall)| {
            rate_elapsed(now.0 - mono, now.1.duration_since(wall).unwrap_or_default())
        });
        self.last = Some(now);

        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        self.sys.refresh_processes(ProcessesToUpdate::All, true);
        self.disks.refresh(true);
        self.nets.refresh(true);

        let cpus = self.sys.cpus();
        if s.cpu_model.is_empty() {
            s.cpu_model = cpus
                .first()
                .map(|c| c.brand().trim().to_owned())
                .unwrap_or_default();
        }
        s.cpu_cores.resize_with(cpus.len(), History::default);
        for (hist, cpu) in s.cpu_cores.iter_mut().zip(cpus) {
            hist.push(cpu.cpu_usage().clamp(0.0, 100.0) as f64);
        }
        s.cpu_total
            .push(self.sys.global_cpu_usage().clamp(0.0, 100.0) as f64);

        s.mem_total = self.sys.total_memory();
        s.mem_used = self.sys.used_memory();
        s.swap_total = self.sys.total_swap();
        s.swap_used = self.sys.used_swap();
        s.mem_hist.push(pct(s.mem_used, s.mem_total));

        s.disks = self
            .disks
            .list()
            .iter()
            // APFS exposes system sub-volumes that duplicate the boot disk.
            .filter(|d| !d.mount_point().starts_with("/System/Volumes"))
            .map(|d| {
                let mount = d.mount_point().to_string_lossy().into_owned();
                let u = d.usage();
                let cur = (u.total_read_bytes, u.total_written_bytes);
                let prev = self.disk_prev.insert(mount.clone(), cur).unwrap_or(cur);
                let mut stat = s
                    .disks
                    .iter()
                    .find(|o| o.mount == mount)
                    .cloned()
                    .unwrap_or_default();
                stat.name = d.name().to_string_lossy().into_owned();
                stat.mount = mount;
                stat.total = d.total_space();
                stat.avail = d.available_space();
                stat.read_bps = rate(prev.0, cur.0, elapsed);
                stat.write_bps = rate(prev.1, cur.1, elapsed);
                stat.read_hist.push(stat.read_bps);
                stat.write_hist.push(stat.write_bps);
                stat
            })
            .collect();

        let mut nets: Vec<NetStat> = self
            .nets
            .list()
            .iter()
            .map(|(name, data)| {
                let cur = (data.total_received(), data.total_transmitted());
                let prev = self.net_prev.insert(name.clone(), cur).unwrap_or(cur);
                let mut stat = s
                    .nets
                    .iter()
                    .find(|o| &o.name == name)
                    .cloned()
                    .unwrap_or_default();
                stat.name = name.clone();
                stat.rx_total = cur.0;
                stat.tx_total = cur.1;
                stat.rx_bps = rate(prev.0, cur.0, elapsed);
                stat.tx_bps = rate(prev.1, cur.1, elapsed);
                stat.rx_hist.push(stat.rx_bps);
                stat.tx_hist.push(stat.tx_bps);
                stat
            })
            .collect();
        nets.sort_by(|a, b| a.name.cmp(&b.name));
        s.nets = nets;

        s.procs = self
            .sys
            .processes()
            .values()
            .map(|p| ProcInfo {
                pid: p.pid().as_u32(),
                ppid: p.parent().map(|pp| pp.as_u32()),
                name: p.name().to_string_lossy().into_owned(),
                user: p
                    .user_id()
                    .and_then(|uid| self.users.get_user_by_id(uid))
                    .map(|u| u.name().to_owned())
                    .unwrap_or_default(),
                cpu: p.cpu_usage(),
                mem: p.memory(),
                threads: p.tasks().map(|t| t.len()),
            })
            .collect();
        s.apps = aggregate_apps(&running_apps(), &s.procs);

        match read_gpu() {
            Some((name, util, mem)) => {
                let g = s.gpu.get_or_insert_with(GpuStat::default);
                g.name = name;
                g.util_hist.push(util);
                g.mem_used = mem;
            }
            None => s.gpu = None,
        }

        match read_battery() {
            Some((percent, health, cycles, state)) => {
                let b = s.battery.get_or_insert_with(BatteryStat::default);
                b.percent = percent;
                b.health = health;
                b.cycles = cycles;
                b.state = state;
                b.charge_hist.push(percent as f64);
            }
            None => s.battery = None,
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

/// Samples forever on a background thread, publishing a fresh [`State`] each tick.
pub fn spawn(state: Arc<Mutex<State>>, ctx: egui::Context) {
    std::thread::Builder::new()
        .name("sampler".into())
        .spawn(move || {
            let mut sampler = Sampler::new();
            let mut local = State::default();
            loop {
                sampler.tick(&mut local);
                // Swap a finished snapshot in so the UI never waits on a refresh.
                *state.lock().unwrap_or_else(|e| e.into_inner()) = local.clone();
                ctx.request_repaint();
                std::thread::sleep(INTERVAL);
            }
        })
        .expect("spawn sampler thread");
}
