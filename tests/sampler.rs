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
