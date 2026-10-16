//! One function per sidebar page. Each draws from a borrowed [`State`](crate::sampler::State).

pub mod apps;
pub mod battery;
pub mod cpu;
pub mod drives;
pub mod gpu;
pub mod memory;
pub mod network;
pub mod processes;
