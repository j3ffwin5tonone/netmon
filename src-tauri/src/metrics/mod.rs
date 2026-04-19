//! Aggregated system metrics for the UI and tray. Add new domains (memory, GPU, …) here
//! as new fields on [`MetricsSnapshot`] / [`MetricsHistory`] and dedicated submodules.

mod cpu;
mod network;

use serde::Serialize;

pub(crate) const HISTORY_LEN: usize = 60;

#[derive(Serialize, Clone, Debug)]
pub struct SpeedEntry {
    pub down: f64, // MB/s
    pub up: f64,   // MB/s
}

#[derive(Serialize, Clone, Debug)]
pub struct MetricsSnapshot {
    pub network: SpeedEntry,
    pub cpu_percent: f32,
}

#[derive(Serialize, Clone, Debug)]
pub struct MetricsHistory {
    pub network: Vec<SpeedEntry>,
    pub cpu: Vec<f32>,
}

pub struct AppState {
    network: network::NetworkMetrics,
    cpu: cpu::CpuMetrics,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            network: network::NetworkMetrics::new(),
            cpu: cpu::CpuMetrics::new(),
        }
    }

    pub fn tick(&mut self) -> MetricsSnapshot {
        let network = self.network.measure();
        let cpu_percent = self.cpu.measure();
        MetricsSnapshot {
            network,
            cpu_percent,
        }
    }

    pub fn history_snapshot(&self) -> MetricsHistory {
        MetricsHistory {
            network: self.network.history().to_vec(),
            cpu: self.cpu.history().to_vec(),
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
