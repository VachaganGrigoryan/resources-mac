use resources_mac::sampler::{Sampler, State};
use std::time::Duration;

#[test]
fn two_ticks_fill_history() {
    let mut sampler = Sampler::new();
    let mut state = State::default();
    sampler.tick(&mut state);
    std::thread::sleep(Duration::from_millis(300));
    sampler.tick(&mut state);

    assert_eq!(state.cpu_total.len(), 2);
    assert!(state.cpu_total.values().all(|v| (0.0..=100.0).contains(&v)));
    assert!(state.mem_total > 0);
    assert!(!state.procs.is_empty());
    let cores = std::thread::available_parallelism().unwrap().get();
    assert_eq!(state.cpu_cores.len(), cores);
    assert!(!state.cpu_model.is_empty());
}

#[test]
fn sum_nets_adds_rates_and_totals() {
    use resources_mac::sampler::{NetStat, sum_nets};
    let a = NetStat {
        rx_bps: 10.0,
        tx_bps: 1.0,
        rx_total: 100,
        tx_total: 7,
        ..Default::default()
    };
    let b = NetStat {
        rx_bps: 5.0,
        tx_bps: 2.0,
        rx_total: 50,
        tx_total: 3,
        ..Default::default()
    };
    assert_eq!(sum_nets(&[&a, &b]), (15.0, 3.0, 150, 10));
    assert_eq!(sum_nets(&[]), (0.0, 0.0, 0, 0));
}
