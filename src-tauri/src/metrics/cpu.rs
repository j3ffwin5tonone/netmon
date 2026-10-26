use sysinfo::{CpuRefreshKind, RefreshKind, System};

pub struct CpuMetrics {
    system: System,
}

impl CpuMetrics {
    pub fn new() -> Self {
        // Seed CPU counters without blocking startup. The scheduled first measurement
        // happens one second later, beyond sysinfo's minimum sampling interval.
        Self {
            system: System::new_with_specifics(
                RefreshKind::new().with_cpu(CpuRefreshKind::new().with_cpu_usage()),
            ),
        }
    }

    pub fn measure(&mut self) -> f32 {
        self.system.refresh_cpu_usage();
        self.system.global_cpu_usage().clamp(0.0, 100.0)
    }
}
