//! Aggregated system metrics for the UI and tray. Add new domains (memory, GPU, …) here
//! as new fields on [`MetricsSnapshot`] / [`MetricsHistory`] and dedicated submodules.

mod cpu;
mod memory;
mod network;

use serde::Serialize;

pub(crate) const HISTORY_LEN: usize = 60;

#[derive(Serialize, Clone, Debug)]
pub struct SpeedEntry {
    pub down: f64, // MB/s
    pub up: f64,   // MB/s
}

#[derive(Serialize, Clone, Debug)]
pub struct MemoryEntry {
    pub used_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct MetricsSnapshot {
    pub network: SpeedEntry,
    pub cpu_percent: f32,
    pub memory: MemoryEntry,
    pub memory_percent: f32,
}

#[derive(Serialize, Clone, Debug)]
pub struct MetricsHistory {
    pub network: Vec<SpeedEntry>,
    pub cpu: Vec<f32>,
    pub memory: Vec<f32>,
}

pub struct AppState {
    network: network::NetworkMetrics,
    cpu: cpu::CpuMetrics,
    memory: memory::MemoryMetrics,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            network: network::NetworkMetrics::new(),
            cpu: cpu::CpuMetrics::new(),
            memory: memory::MemoryMetrics::new(),
        }
    }

    pub fn tick(&mut self) -> MetricsSnapshot {
        let network = self.network.measure();
        let cpu_percent = self.cpu.measure();
        let (used_bytes, total_bytes, memory_percent) = self.memory.measure();
        MetricsSnapshot {
            network,
            cpu_percent,
            memory: MemoryEntry {
                used_bytes,
                total_bytes,
            },
            memory_percent,
        }
    }

    pub fn history_snapshot(&self) -> MetricsHistory {
        MetricsHistory {
            network: self.network.history().to_vec(),
            cpu: self.cpu.history().to_vec(),
            memory: self.memory.history().to_vec(),
        }
    }
}

pub fn format_speed(mb: f64) -> String {
    if mb >= 1.0 {
        format!("{:.1} MB/s", mb)
    } else if mb >= 0.001 {
        format!("{:.0} KB/s", mb * 1024.0)
    } else {
        "0 B/s".to_string()
    }
}
