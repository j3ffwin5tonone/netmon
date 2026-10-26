use super::SpeedEntry;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use sysinfo::Networks;

const MIB: f64 = 1_048_576.0;
// A delayed/resumed sample must not represent several seconds of traffic as live load.
const MAX_SAMPLE_INTERVAL: Duration = Duration::from_secs(5);

/// Unix `lo` / `lo0`, Windows `Loopback Pseudo-Interface *`, etc.
/// Avoids a bare `starts_with("lo")`, which would drop "Local Area Connection" on Windows.
fn is_loopback_interface(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n == "lo"
        || (n.starts_with("lo") && n.chars().nth(2).is_some_and(|c| c.is_ascii_digit()))
        || n.contains("loopback")
}

#[derive(Clone, Copy)]
struct Counters {
    received: u64,
    transmitted: u64,
}

fn zero_speed() -> SpeedEntry {
    SpeedEntry { down: 0.0, up: 0.0 }
}

#[derive(Default)]
struct RateTracker {
    previous: BTreeMap<String, Counters>,
    previous_at: Option<Instant>,
    requested: Option<String>,
    selected: Option<String>,
}

impl RateTracker {
    fn set_interface(&mut self, selection: Option<String>) -> Result<(), String> {
        if let Some(name) = selection.as_deref() {
            if name.trim().is_empty()
                || name.len() > 256
                || name.chars().any(char::is_control)
                || is_loopback_interface(name)
            {
                return Err("Choose a non-loopback network interface.".into());
            }
        }
        // Retain an explicit choice while disconnected; do not silently switch adapters.
        self.requested = selection;
        if let Some(name) = self.requested.as_ref() {
            self.selected = self.previous.contains_key(name).then(|| name.clone());
        }
        Ok(())
    }

    fn sample(&mut self, counters: BTreeMap<String, Counters>, now: Instant) -> SpeedEntry {
        let elapsed = self
            .previous_at
            .and_then(|at| now.checked_duration_since(at));
        let interval = elapsed.filter(|dt| !dt.is_zero() && *dt <= MAX_SAMPLE_INTERVAL);
        let rates: BTreeMap<_, _> = counters
            .iter()
            .map(|(name, current)| {
                let rate = self
                    .previous
                    .get(name)
                    .zip(interval)
                    .and_then(|(previous, dt)| {
                        // A reset in either direction starts a fresh baseline for this adapter.
                        let down = current.received.checked_sub(previous.received)?;
                        let up = current.transmitted.checked_sub(previous.transmitted)?;
                        Some(SpeedEntry {
                            down: down as f64 / MIB / dt.as_secs_f64(),
                            up: up as f64 / MIB / dt.as_secs_f64(),
                        })
                    })
                    .unwrap_or_else(zero_speed);
                (name.clone(), rate)
            })
            .collect();

        self.selected = if let Some(name) = self.requested.as_ref() {
            counters.contains_key(name).then(|| name.clone())
        } else {
            self.choose_automatic(&counters, &rates)
        };
        let speed = self
            .selected
            .as_ref()
            .and_then(|name| rates.get(name))
            .cloned()
            .unwrap_or_else(zero_speed);
        self.previous = counters;
        self.previous_at = Some(now);
        speed
    }

    fn choose_automatic(
        &self,
        counters: &BTreeMap<String, Counters>,
        rates: &BTreeMap<String, SpeedEntry>,
    ) -> Option<String> {
        // Select exactly one adapter: adding a VPN and its physical transport would
        // double-count the same traffic. The chosen adapter is displayed in the UI.
        let mut selected = self
            .selected
            .as_ref()
            .filter(|name| counters.contains_key(*name))
            .cloned();
        if selected.is_none() {
            // Prefer an adapter with observed lifetime traffic for the initial idle sample.
            selected = counters
                .iter()
                .max_by_key(|(_, counter)| {
                    u128::from(counter.received) + u128::from(counter.transmitted)
                })
                .map(|(name, _)| name.clone());
        }
        let mut busiest = selected
            .as_ref()
            .and_then(|name| rates.get(name))
            .map_or(0.0, |rate| rate.down + rate.up);
        for (name, rate) in rates {
            let traffic = rate.down + rate.up;
            if traffic > busiest {
                selected = Some(name.clone());
                busiest = traffic;
            }
        }
        selected
    }
}

pub struct NetworkMetrics {
    networks: Networks,
    tracker: RateTracker,
}

impl NetworkMetrics {
    pub fn new() -> Self {
        let networks = Networks::new_with_refreshed_list();
        let mut tracker = RateTracker::default();
        tracker.sample(read_counters(&networks), Instant::now());
        Self { networks, tracker }
    }

    pub fn measure(&mut self) -> SpeedEntry {
        // refresh_list already refreshes counters. One enumeration per sample keeps
        // unplugged adapters out and primes newly attached adapters without stale deltas.
        self.networks.refresh_list();
        self.tracker
            .sample(read_counters(&self.networks), Instant::now())
    }

    pub fn interfaces(&self) -> Vec<String> {
        self.tracker.previous.keys().cloned().collect()
    }

    pub fn selected_interface(&self) -> Option<String> {
        self.tracker.selected.clone()
    }

    pub fn selection(&self) -> Option<String> {
        self.tracker.requested.clone()
    }

    pub fn set_interface(&mut self, selection: Option<String>) -> Result<(), String> {
        self.tracker.set_interface(selection)
    }
}

fn read_counters(networks: &Networks) -> BTreeMap<String, Counters> {
    networks
        .iter()
        .filter(|(name, _)| !is_loopback_interface(name))
        .map(|(name, data)| {
            (
                name.clone(),
                Counters {
                    received: data.total_received(),
                    transmitted: data.total_transmitted(),
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counters(items: &[(&str, u64, u64)]) -> BTreeMap<String, Counters> {
        items
            .iter()
            .map(|(name, received, transmitted)| {
                (
                    (*name).into(),
                    Counters {
                        received: *received,
                        transmitted: *transmitted,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn actual_elapsed_time_controls_the_rate() {
        let start = Instant::now();
        let mut tracker = RateTracker::default();
        tracker.sample(counters(&[("en0", 100, 100)]), start);
        let reading = tracker.sample(
            counters(&[("en0", 100 + 2 * 1_048_576, 100 + 1_048_576)]),
            start + Duration::from_secs(2),
        );
        assert_eq!(reading.down, 1.0);
        assert_eq!(reading.up, 0.5);
    }

    #[test]
    fn new_disconnected_and_reconnected_adapters_do_not_replay_traffic() {
        let start = Instant::now();
        let mut tracker = RateTracker::default();
        tracker.sample(counters(&[("en0", 0, 0)]), start);
        let second = start + Duration::from_secs(1);
        let first = tracker.sample(counters(&[("en0", 1_048_576, 0)]), second);
        assert_eq!(first.down, 1.0);
        assert_eq!(
            tracker
                .sample(BTreeMap::new(), second + Duration::from_secs(1))
                .down,
            0.0
        );
        assert!(tracker.selected.is_none());
        let reconnected = tracker.sample(
            counters(&[("en0", 30 * 1_048_576, 0), ("usb0", 40 * 1_048_576, 0)]),
            second + Duration::from_secs(2),
        );
        assert_eq!(reconnected.down, 0.0);
        assert_eq!(tracker.previous.len(), 2);
    }

    #[test]
    fn reset_and_resume_reprime_counters() {
        let start = Instant::now();
        let mut tracker = RateTracker::default();
        tracker.sample(counters(&[("en0", 10_000, 10_000)]), start);
        let reset = tracker.sample(
            counters(&[("en0", 0, 20_000)]),
            start + Duration::from_secs(1),
        );
        assert_eq!(reset.up, 0.0);
        let resume = tracker.sample(
            counters(&[("en0", 60_000_000, 20_000)]),
            start + Duration::from_secs(61),
        );
        assert_eq!(resume.down, 0.0);
        let next = tracker.sample(
            counters(&[("en0", 60_000_000 + 1_048_576, 20_000)]),
            start + Duration::from_secs(62),
        );
        assert_eq!(next.down, 1.0);
    }

    #[test]
    fn automatic_selection_does_not_add_vpn_and_physical_traffic() {
        let start = Instant::now();
        let mut tracker = RateTracker::default();
        tracker.sample(counters(&[("en0", 100, 0), ("utun0", 0, 0)]), start);
        let reading = tracker.sample(
            counters(&[("en0", 100 + 1_048_576, 0), ("utun0", 1_048_576, 0)]),
            start + Duration::from_secs(1),
        );
        assert_eq!(reading.down, 1.0);
        assert_eq!(tracker.selected.as_deref(), Some("en0"));
        tracker.sample(
            counters(&[("en0", 100 + 1_048_576, 0), ("utun0", 1_048_576, 0)]),
            start + Duration::from_secs(2),
        );
        assert_eq!(tracker.selected.as_deref(), Some("en0"));
    }

    #[test]
    fn explicit_selection_waits_for_its_interface() {
        let start = Instant::now();
        let mut tracker = RateTracker::default();
        tracker.set_interface(Some("usb0".into())).unwrap();
        tracker.sample(counters(&[("en0", 10_000, 0)]), start);
        assert_eq!(tracker.requested.as_deref(), Some("usb0"));
        assert_eq!(tracker.selected, None);
        tracker.sample(
            counters(&[("en0", 20_000, 0), ("usb0", 40_000, 0)]),
            start + Duration::from_secs(1),
        );
        assert_eq!(tracker.selected.as_deref(), Some("usb0"));
        let result = tracker.sample(
            counters(&[("en0", 100_000, 0)]),
            start + Duration::from_secs(2),
        );
        assert_eq!(result.down, 0.0);
        assert_eq!(tracker.selected, None);
        assert_eq!(tracker.requested.as_deref(), Some("usb0"));
    }

    #[test]
    fn loopback_filter_preserves_windows_local_area_connection() {
        assert!(is_loopback_interface("lo"));
        assert!(is_loopback_interface("lo0"));
        assert!(is_loopback_interface("Loopback Pseudo-Interface 1"));
        assert!(!is_loopback_interface("Local Area Connection"));
        assert!(!is_loopback_interface("en0"));
    }

    #[test]
    fn zero_interval_and_invalid_selection_are_safe() {
        let start = Instant::now();
        let mut tracker = RateTracker::default();
        tracker.sample(counters(&[("en0", 0, 0)]), start);
        let rate = tracker.sample(counters(&[("en0", 100, 0)]), start);
        assert_eq!(rate.down, 0.0);
        for name in ["", " ", "lo0", "foo\nbar"] {
            assert!(tracker.set_interface(Some(name.into())).is_err());
        }
    }
}
