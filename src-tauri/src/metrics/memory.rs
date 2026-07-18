use super::HISTORY_LEN;
use std::collections::VecDeque;
use sysinfo::{MemoryRefreshKind, RefreshKind, System};

pub struct MemoryMetrics {
    system: System,
    history: VecDeque<f32>,
}

impl MemoryMetrics {
    pub fn new() -> Self {
        let mut system = System::new_with_specifics(
            RefreshKind::new().with_memory(MemoryRefreshKind::new().with_ram()),
        );
        system.refresh_memory();

        Self {
            system,
            history: VecDeque::with_capacity(HISTORY_LEN),
        }
    }

    pub fn measure(&mut self) -> (u64, u64, f32) {
        self.system.refresh_memory();
        let total = self.system.total_memory();
        let used = self.system.used_memory();
        let pct = if total > 0 {
            (100.0 * used as f64 / total as f64) as f32
        } else {
            0.0
        };

        if self.history.len() == HISTORY_LEN {
            self.history.pop_front();
        }
        self.history.push_back(pct);

        (used, total, pct)
    }

    pub fn history(&self) -> &VecDeque<f32> {
        &self.history
    }
}
