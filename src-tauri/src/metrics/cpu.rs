use super::HISTORY_LEN;
use std::collections::VecDeque;
use sysinfo::{CpuRefreshKind, RefreshKind, System, MINIMUM_CPU_UPDATE_INTERVAL};

pub struct CpuMetrics {
    system: System,
    history: VecDeque<f32>,
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
            history: VecDeque::with_capacity(HISTORY_LEN),
        }
    }

    pub fn measure(&mut self) -> f32 {
        self.system.refresh_cpu_usage();
        let pct = self.system.global_cpu_usage();

        if self.history.len() == HISTORY_LEN {
            self.history.pop_front();
        }
        self.history.push_back(pct);

        pct
    }

    pub fn history(&self) -> &VecDeque<f32> {
        &self.history
    }
}
