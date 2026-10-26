//! Timestamped, bounded system metrics shared by the tray and dashboard.
mod cpu;
mod gpu;
mod memory;
mod network;

use serde::Serialize;
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const HISTORY_LEN: usize = 60;
const HISTORY_MS: u64 = 60_000;

#[derive(Serialize, Clone, Debug)]
pub struct SpeedEntry {
    pub down: f64, // MiB/s
    pub up: f64,   // MiB/s
}

#[derive(Serialize, Clone, Debug)]
pub struct MemoryEntry {
    pub used_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct MetricsSnapshot {
    pub sequence: u64,
    pub timestamp_ms: u64,
    pub network: SpeedEntry,
    pub network_interface: Option<String>,
    pub network_interfaces: Vec<String>,
    pub cpu_percent: f32,
    pub memory: MemoryEntry,
    pub memory_percent: f32,
    /// Active time / observation time; None when unavailable or stale.
    pub gpu_percent: Option<f32>,
}

#[derive(Serialize, Clone, Debug)]
pub struct MetricsHistory {
    pub samples: Vec<MetricsSnapshot>,
    pub network_selection: Option<String>,
}

fn timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn retain_recent(history: &mut VecDeque<MetricsSnapshot>, now_ms: u64) {
    while history
        .front()
        .is_some_and(|s| s.timestamp_ms <= now_ms.saturating_sub(HISTORY_MS))
    {
        history.pop_front();
    }
    while history.len() > HISTORY_LEN {
        history.pop_front();
    }
}

pub struct AppState {
    network: network::NetworkMetrics,
    cpu: cpu::CpuMetrics,
    memory: memory::MemoryMetrics,
    gpu: gpu::GpuMetrics,
    history: VecDeque<MetricsSnapshot>,
    sequence: u64,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            network: network::NetworkMetrics::new(),
            cpu: cpu::CpuMetrics::new(),
            memory: memory::MemoryMetrics::new(),
            gpu: gpu::GpuMetrics::new(),
            history: VecDeque::with_capacity(HISTORY_LEN),
            sequence: 0,
        }
    }

    pub fn tick(&mut self) -> MetricsSnapshot {
        let network = self.network.measure();
        let cpu_percent = self.cpu.measure();
        let (used_bytes, total_bytes, memory_percent) = self.memory.measure();
        self.sequence = self.sequence.saturating_add(1);
        let timestamp_ms = timestamp_ms();
        // Wall-clock changes cannot cause a chart to connect samples in reverse time.
        if self
            .history
            .back()
            .is_some_and(|s| timestamp_ms <= s.timestamp_ms)
        {
            self.history.clear();
        }
        let snapshot = MetricsSnapshot {
            sequence: self.sequence,
            timestamp_ms,
            network,
            network_interface: self.network.selected_interface(),
            network_interfaces: self.network.interfaces(),
            cpu_percent,
            memory: MemoryEntry {
                used_bytes,
                total_bytes,
            },
            memory_percent,
            gpu_percent: self.gpu.measure(),
        };
        self.history.push_back(snapshot.clone());
        retain_recent(&mut self.history, timestamp_ms);
        snapshot
    }

    pub fn history_snapshot(&mut self) -> MetricsHistory {
        retain_recent(&mut self.history, timestamp_ms());
        MetricsHistory {
            samples: self.history.iter().cloned().collect(),
            network_selection: self.network.selection(),
        }
    }

    pub fn set_network_interface(&mut self, name: Option<String>) -> Result<(), String> {
        self.network.set_interface(name)
    }
}

pub fn format_speed(mb: f64) -> String {
    if !mb.is_finite() || mb <= 0.0 {
        "0 B/s".to_string()
    } else if mb >= 1.0 {
        format!("{:.1} MiB/s", mb)
    } else if mb >= 1.0 / 1024.0 {
        format!("{:.0} KiB/s", mb * 1024.0)
    } else {
        format!("{:.0} B/s", mb * 1_048_576.0)
    }
}

/// Four-character speed for the menu bar so `NSStatusItem` width stays stable.
/// Variable-width titles (`0 B/s` vs `999.9 MiB/s`) get culled on crowded macOS menu bars.
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
    let down = if snapshot.network_interface.is_some() {
        format_speed_compact(snapshot.network.down)
    } else {
        " n/a".into()
    };
    let up = if snapshot.network_interface.is_some() {
        format_speed_compact(snapshot.network.up)
    } else {
        " n/a".into()
    };
    if let Some(gpu_percent) = snapshot.gpu_percent {
        format!(
            "↓{down} ↑{up} {cpu:>3.0}% {ram:>3.0}% {gpu:>3.0}%",
            cpu = snapshot.cpu_percent.clamp(0.0, 100.0),
            ram = snapshot.memory_percent.clamp(0.0, 100.0),
            gpu = gpu_percent.clamp(0.0, 100.0),
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
    let network = match &snapshot.network_interface {
        Some(name) => format!(
            "{name}: ↓ {} ↑ {}",
            format_speed(snapshot.network.down),
            format_speed(snapshot.network.up)
        ),
        None => "Network unavailable".into(),
    };
    let mut result = format!(
        "{network}\nCPU {:.0}%  RAM {:.0}%",
        snapshot.cpu_percent, snapshot.memory_percent
    );
    if let Some(gpu) = snapshot.gpu_percent {
        result.push_str(&format!("  GPU {gpu:.0}%"));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(t: u64) -> MetricsSnapshot {
        MetricsSnapshot {
            sequence: t,
            timestamp_ms: t,
            network: SpeedEntry { down: 0.0, up: 0.0 },
            network_interface: Some("en0".into()),
            network_interfaces: vec!["en0".into()],
            cpu_percent: 10.0,
            memory: MemoryEntry {
                used_bytes: 1,
                total_bytes: 2,
            },
            memory_percent: 50.0,
            gpu_percent: Some(30.0),
        }
    }
    #[test]
    fn history_expires_by_time_and_count() {
        let mut history: VecDeque<_> = (1..=80).map(|i| snapshot(i * 1000)).collect();
        retain_recent(&mut history, 80_000);
        assert_eq!(history.len(), 60);
        assert_eq!(history.front().unwrap().timestamp_ms, 21_000);
        retain_recent(&mut history, 180_000);
        assert!(history.is_empty());
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
            assert_eq!(format_speed_compact(mb).chars().count(), 4, "{mb}");
        }
    }
    #[test]
    fn tooltip_uses_binary_units_and_omits_stale_gpu() {
        assert_eq!(format_speed(1.0), "1.0 MiB/s");
        assert_eq!(format_speed(0.5), "512 KiB/s");
        assert_eq!(format_speed(512.0 / 1_048_576.0), "512 B/s");
        let mut s = snapshot(1);
        s.gpu_percent = None;
        assert!(!format_tray_tooltip(&s).contains("GPU"));
    }
}
