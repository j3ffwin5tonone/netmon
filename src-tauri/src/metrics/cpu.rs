use super::HISTORY_LEN;
use sysinfo::{CpuRefreshKind, RefreshKind, System, MINIMUM_CPU_UPDATE_INTERVAL};

pub struct CpuMetrics {
    system: System,
    history: Vec<f32>,
}

impl CpuMetrics {
    pub fn new() -> Self {
        let mut system = System::new_with_specifics(
            RefreshKind::new().with_cpu(CpuRefreshKind::new().with_cpu_usage()),
        );
        // CPU usage is computed from deltas; prime with a short interval so the first UI tick is meaningful.
        system.refresh_cpu_usage();
        std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
        system.refresh_cpu_usage();

        Self {
            system,
            history: Vec::with_capacity(HISTORY_LEN),
        }
    }

    pub fn measure(&mut self) -> f32 {
        self.system.refresh_cpu_usage();
        let pct = self.system.global_cpu_usage();

        self.history.push(pct);
        if self.history.len() > HISTORY_LEN {
            self.history.remove(0);
        }

        pct
    }

    pub fn history(&self) -> &[f32] {
        &self.history
    }
}
