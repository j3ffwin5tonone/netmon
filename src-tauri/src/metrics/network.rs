use super::{SpeedEntry, HISTORY_LEN};
use sysinfo::Networks;

pub struct NetworkMetrics {
    networks: Networks,
    history: Vec<SpeedEntry>,
}

impl NetworkMetrics {
    pub fn new() -> Self {
        Self {
            networks: Networks::new_with_refreshed_list(),
            history: Vec::with_capacity(HISTORY_LEN),
        }
    }

    /// Delta since last refresh; excludes loopback interfaces.
    pub fn measure(&mut self) -> SpeedEntry {
        // refresh() updates internal counters; received()/transmitted()
        // return bytes since the *previous* refresh — exactly the delta we need.
        self.networks.refresh();

        let (mut down, mut up) = (0u64, 0u64);
        for (name, data) in self.networks.iter() {
            if !name.starts_with("lo") {
                down += data.received();
                up += data.transmitted();
            }
        }

        let entry = SpeedEntry {
            down: down as f64 / 1_048_576.0,
            up: up as f64 / 1_048_576.0,
        };

        self.history.push(entry.clone());
        if self.history.len() > HISTORY_LEN {
            self.history.remove(0);
        }

        entry
    }

    pub fn history(&self) -> &[SpeedEntry] {
        &self.history
    }
}
