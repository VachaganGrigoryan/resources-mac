//! One function per sidebar page. Each draws from a borrowed [`State`](crate::sampler::State).

pub mod cpu;
pub mod drives;
pub mod memory;
pub mod network;
