//! Aggregated system metrics for the UI and tray. Add new domains (memory, GPU, …) here
//! as new fields on [`MetricsSnapshot`] / [`MetricsHistory`] and dedicated submodules.

mod cpu;
mod gpu;
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
    /// GPU load 0–100 when `gpu_supported` is true (Apple Silicon + IOReport).
    pub gpu_percent: f32,
    pub gpu_supported: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct MetricsHistory {
    pub network: Vec<SpeedEntry>,
    pub cpu: Vec<f32>,
    pub memory: Vec<f32>,
    pub gpu: Vec<f32>,
    pub gpu_supported: bool,
}

pub struct AppState {
    network: network::NetworkMetrics,
    cpu: cpu::CpuMetrics,
    memory: memory::MemoryMetrics,
    gpu: gpu::GpuMetrics,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            network: network::NetworkMetrics::new(),
            cpu: cpu::CpuMetrics::new(),
            memory: memory::MemoryMetrics::new(),
            gpu: gpu::GpuMetrics::new(),
        }
    }

    pub fn tick(&mut self) -> MetricsSnapshot {
        let network = self.network.measure();
        let cpu_percent = self.cpu.measure();
        let (used_bytes, total_bytes, memory_percent) = self.memory.measure();
        let gpu_supported = self.gpu.supported();
        let gpu_percent = self.gpu.measure();
        MetricsSnapshot {
            network,
            cpu_percent,
            memory: MemoryEntry {
                used_bytes,
                total_bytes,
            },
            memory_percent,
            gpu_percent,
            gpu_supported,
        }
    }

    pub fn history_snapshot(&self) -> MetricsHistory {
        MetricsHistory {
            network: self.network.history().iter().cloned().collect(),
            cpu: self.cpu.history().iter().copied().collect(),
            memory: self.memory.history().iter().copied().collect(),
            gpu: self.gpu.history().iter().copied().collect(),
            gpu_supported: self.gpu.supported(),
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

/// Four-character speed for the menu bar so `NSStatusItem` width stays stable.
/// Variable-width titles (`0 B/s` vs `999.9 MB/s`) get culled on crowded macOS menu bars.
pub fn format_speed_compact(mb: f64) -> String {
    if !mb.is_finite() || mb < 0.0005 {
        return "  0B".to_string();
    }
    let (value, unit) = if mb >= 1024.0 {
        (mb / 1024.0, 'G')
    } else if mb >= 1.0 {
        (mb, 'M')
    } else {
        (mb * 1024.0, 'K')
    };
    // Keep the numeric part to 3 columns so rounding (9.96 → 10.0, 999.5 → 1000) cannot grow the item.
    if value >= 99.5 {
        format!("{:>3.0}{unit}", value.min(999.0))
    } else if value >= 9.95 {
        format!("{:>3.0}{unit}", value)
    } else {
        format!("{:>3.1}{unit}", value)
    }
}

pub fn format_tray_title(snapshot: &MetricsSnapshot) -> String {
    let down = format_speed_compact(snapshot.network.down);
    let up = format_speed_compact(snapshot.network.up);
    if snapshot.gpu_supported {
        format!(
            "↓{down} ↑{up} {cpu:>3.0}% {ram:>3.0}% {gpu:>3.0}%",
            cpu = snapshot.cpu_percent.clamp(0.0, 100.0),
            ram = snapshot.memory_percent.clamp(0.0, 100.0),
            gpu = snapshot.gpu_percent.clamp(0.0, 100.0),
        )
    } else {
        format!(
            "↓{down} ↑{up} {cpu:>3.0}% {ram:>3.0}%",
            cpu = snapshot.cpu_percent.clamp(0.0, 100.0),
            ram = snapshot.memory_percent.clamp(0.0, 100.0),
        )
    }
}

pub fn format_tray_tooltip(snapshot: &MetricsSnapshot) -> String {
    if snapshot.gpu_supported {
        format!(
            "↓ {} ↑ {}\nCPU {:.0}%  RAM {:.0}%  GPU {:.0}%",
            format_speed(snapshot.network.down),
            format_speed(snapshot.network.up),
            snapshot.cpu_percent,
            snapshot.memory_percent,
            snapshot.gpu_percent
        )
    } else {
        format!(
            "↓ {} ↑ {}\nCPU {:.0}%  RAM {:.0}%",
            format_speed(snapshot.network.down),
            format_speed(snapshot.network.up),
            snapshot.cpu_percent,
            snapshot.memory_percent
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(
        down: f64,
        up: f64,
        cpu: f32,
        ram: f32,
        gpu: f32,
        gpu_supported: bool,
    ) -> MetricsSnapshot {
        MetricsSnapshot {
            network: SpeedEntry { down, up },
            cpu_percent: cpu,
            memory: MemoryEntry {
                used_bytes: 0,
                total_bytes: 1,
            },
            memory_percent: ram,
            gpu_percent: gpu,
            gpu_supported,
        }
    }

    #[test]
    fn compact_speed_is_four_chars() {
        for mb in [
            0.0,
            0.0004,
            0.001,
            0.01,
            0.5,
            1.0,
            9.9,
            9.96,
            10.0,
            99.4,
            99.6,
            100.0,
            999.0,
            1024.0,
            2048.0,
            999.5 / 1024.0,
        ] {
            let s = format_speed_compact(mb);
            assert_eq!(s.chars().count(), 4, "{mb} -> {s:?}");
        }
    }

    #[test]
    fn tray_title_width_is_stable() {
        let idle = format_tray_title(&snapshot(0.0, 0.0, 0.0, 0.0, 0.0, true));
        let busy = format_tray_title(&snapshot(125.4, 12.2, 100.0, 99.0, 87.0, true));
        assert_eq!(idle.chars().count(), busy.chars().count());
        assert!(idle.chars().count() <= 28, "{idle}");
    }
}
