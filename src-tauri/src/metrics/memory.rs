use sysinfo::{MemoryRefreshKind, RefreshKind, System};

pub struct MemoryMetrics {
    system: System,
}

impl MemoryMetrics {
    pub fn new() -> Self {
        Self {
            system: System::new_with_specifics(
                RefreshKind::new().with_memory(MemoryRefreshKind::new().with_ram()),
            ),
        }
    }

    pub fn measure(&mut self) -> (u64, u64, f32) {
        // RAM is displayed; polling swap as well would add an unused system query.
        self.system
            .refresh_memory_specifics(MemoryRefreshKind::new().with_ram());
        let total = self.system.total_memory();
        let used = self.system.used_memory();
        let pct = if total > 0 {
            (100.0 * used as f64 / total as f64).clamp(0.0, 100.0) as f32
        } else {
            0.0
        };
        (used, total, pct)
    }
}
