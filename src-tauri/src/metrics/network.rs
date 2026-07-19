use super::{SpeedEntry, HISTORY_LEN};
use std::collections::VecDeque;
use sysinfo::Networks;

/// Unix `lo` / `lo0`, Windows `Loopback Pseudo-Interface *`, etc.
/// Avoids a bare `starts_with("lo")`, which would drop "Local Area Connection" on Windows.
fn is_loopback_interface(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n == "lo"
        || (n.starts_with("lo") && n.chars().nth(2).is_some_and(|c| c.is_ascii_digit()))
        || n.contains("loopback")
}

pub struct NetworkMetrics {
    networks: Networks,
    history: VecDeque<SpeedEntry>,
}

impl NetworkMetrics {
    pub fn new() -> Self {
        Self {
            networks: Networks::new_with_refreshed_list(),
            history: VecDeque::with_capacity(HISTORY_LEN),
        }
    }

    /// Delta since last refresh; excludes loopback interfaces.
    pub fn measure(&mut self) -> SpeedEntry {
        // refresh() updates internal counters; received()/transmitted()
        // return bytes since the *previous* refresh — exactly the delta we need.
        self.networks.refresh();

        let (mut down, mut up) = (0u64, 0u64);
        for (name, data) in self.networks.iter() {
            if !is_loopback_interface(name) {
                down += data.received();
                up += data.transmitted();
            }
        }

        let entry = SpeedEntry {
            down: down as f64 / 1_048_576.0,
            up: up as f64 / 1_048_576.0,
        };

        if self.history.len() == HISTORY_LEN {
            self.history.pop_front();
        }
        self.history.push_back(entry.clone());

        entry
    }

    pub fn history(&self) -> &VecDeque<SpeedEntry> {
        &self.history
    }
}
