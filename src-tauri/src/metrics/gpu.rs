//! Time-based GPU activity, sampled independently from the UI.
//!
//! Apple Silicon reads only GPU IOReport residency counters. Windows sums the
//! per-process PDH counters for each physical GPU/engine, then uses the busiest
//! engine. Missing, failed or stale readings are `None`, never a frozen percentage.

#[cfg(any(all(target_os = "macos", target_arch = "aarch64"), windows))]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(any(all(target_os = "macos", target_arch = "aarch64"), windows))]
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

const MAX_SAMPLE_AGE: Duration = Duration::from_secs(3);
#[cfg(any(all(target_os = "macos", target_arch = "aarch64"), windows))]
const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
#[cfg(any(all(target_os = "macos", target_arch = "aarch64"), windows))]
const RETRY_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Clone, Copy)]
struct Reading {
    percent: f32,
    sampled_at: SystemTime,
}

impl Reading {
    fn fresh_value(self, now: SystemTime) -> Option<f32> {
        let age = now.duration_since(self.sampled_at).ok()?;
        (age <= MAX_SAMPLE_AGE && self.percent.is_finite())
            .then_some(self.percent.clamp(0.0, 100.0))
    }
}

pub struct GpuMetrics {
    #[cfg(any(all(target_os = "macos", target_arch = "aarch64"), windows))]
    latest: Arc<Mutex<Option<Reading>>>,
    #[cfg(not(any(all(target_os = "macos", target_arch = "aarch64"), windows)))]
    latest: Mutex<Option<Reading>>,
    #[cfg(any(all(target_os = "macos", target_arch = "aarch64"), windows))]
    stop: Arc<AtomicBool>,
    #[cfg(any(all(target_os = "macos", target_arch = "aarch64"), windows))]
    worker: Option<std::thread::JoinHandle<()>>,
}

impl GpuMetrics {
    pub fn new() -> Self {
        #[cfg(any(all(target_os = "macos", target_arch = "aarch64"), windows))]
        {
            let latest = Arc::new(Mutex::new(None));
            let stop = Arc::new(AtomicBool::new(false));
            let output = Arc::clone(&latest);
            let stopping = Arc::clone(&stop);
            let worker = std::thread::Builder::new()
                .name("netmon-gpu".into())
                .spawn(move || run_worker(output, stopping))
                .map_err(|error| eprintln!("GPU sampling unavailable: {error}"))
                .ok();
            Self {
                latest,
                stop,
                worker,
            }
        }
        #[cfg(not(any(all(target_os = "macos", target_arch = "aarch64"), windows)))]
        {
            Self {
                latest: Mutex::new(None),
            }
        }
    }

    /// The lock protects only a small published value, never a platform API call.
    pub fn measure(&mut self) -> Option<f32> {
        self.latest
            .try_lock()
            .ok()
            .and_then(|sample| sample.and_then(|sample| sample.fresh_value(SystemTime::now())))
    }
}

#[cfg(any(all(target_os = "macos", target_arch = "aarch64"), windows))]
impl Drop for GpuMetrics {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            // Never make window shutdown wait on a driver. A pending IOReport
            // sample finishes within its 1 s interval, then observes `stop`.
            // A parked retry/Windows worker is woken immediately.
            if worker.is_finished() {
                let _ = worker.join();
            }
        }
    }
}

#[cfg(any(all(target_os = "macos", target_arch = "aarch64"), windows))]
fn publish(output: &Mutex<Option<Reading>>, percent: Option<f32>) {
    if let Ok(mut sample) = output.lock() {
        *sample = percent
            .filter(|value| value.is_finite())
            .map(|percent| Reading {
                percent: percent.clamp(0.0, 100.0),
                sampled_at: SystemTime::now(),
            });
    }
}

#[cfg(any(all(target_os = "macos", target_arch = "aarch64"), windows))]
fn run_worker(output: Arc<Mutex<Option<Reading>>>, stop: Arc<AtomicBool>) {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    use apple_gpu::Sampler;
    #[cfg(windows)]
    use windows_gpu::Sampler;

    while !stop.load(Ordering::Acquire) {
        // Upstream parsers can panic on an unfamiliar channel layout. Treat that
        // like a failed sensor rather than silently freezing the last reading.
        let opened = catch_unwind(AssertUnwindSafe(Sampler::open));
        if let Ok(Some(mut sampler)) = opened {
            while !stop.load(Ordering::Acquire) {
                let next = catch_unwind(AssertUnwindSafe(|| sampler.sample(&stop)))
                    .ok()
                    .flatten();
                if stop.load(Ordering::Acquire) {
                    break;
                }
                publish(&output, next);
                if next.is_none() {
                    break;
                }
            }
        }
        publish(&output, None);
        if !stop.load(Ordering::Acquire) {
            std::thread::park_timeout(RETRY_INTERVAL);
        }
    }
}

/// Frequency-independent active residency. Negative or empty counter windows
/// are invalid; a valid all-idle sample is exactly 0%.
#[cfg(any(all(target_os = "macos", target_arch = "aarch64"), test))]
fn active_residency_percent<'a>(states: impl Iterator<Item = (&'a str, i64)>) -> Option<f32> {
    let (mut active, mut total) = (0u128, 0u128);
    let mut known_idle_state = false;
    for (name, ticks) in states {
        if ticks < 0 {
            return None;
        }
        let ticks = ticks as u128;
        total += ticks;
        if matches!(name, "IDLE" | "DOWN" | "OFF") {
            known_idle_state = true;
        } else {
            active += ticks;
        }
    }
    // Without any known idle state an unfamiliar schema could look 100% busy.
    (total > 0 && known_idle_state).then(|| (100.0 * active as f64 / total as f64) as f32)
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
mod apple_gpu {
    use super::{active_residency_percent, AtomicBool, Ordering, SAMPLE_INTERVAL};
    use macmon::sources::{cfio_get_residencies, IOReport};
    use std::time::SystemTime;

    pub(super) struct Sampler(IOReport);

    impl Sampler {
        pub(super) fn open() -> Option<Self> {
            IOReport::new(vec![("GPU Stats", Some("GPU Performance States"))])
                .ok()
                .map(Self)
        }

        pub(super) fn sample(&mut self, stop: &AtomicBool) -> Option<f32> {
            let mut percent: Option<f32> = None;
            let started = SystemTime::now();
            for (sample, elapsed_ms) in self.0.get_samples(SAMPLE_INTERVAL.as_millis() as u64, 1) {
                // After suspend or a very delayed call, reinitialize the baseline.
                if stop.load(Ordering::Acquire)
                    || elapsed_ms > 2_000
                    || started.elapsed().ok()?.as_millis() > 2_000
                {
                    return None;
                }
                for channel in sample {
                    if channel.channel != "GPUPH" && !channel.channel.ends_with("_GPUPH") {
                        continue;
                    }
                    let states = cfio_get_residencies(channel.item);
                    let activity = active_residency_percent(
                        states.iter().map(|(name, ticks)| (name.as_str(), *ticks)),
                    )?;
                    percent = Some(percent.unwrap_or(0.0).max(activity));
                }
            }
            percent
        }
    }
}

/// The key retains adapter LUID, physical GPU index and engine index, but not
/// PID or engine display type. Processes sharing one engine must be summed.
#[cfg(any(windows, test))]
fn engine_key(name: &str) -> Option<&str> {
    let rest = name.strip_prefix("pid_")?;
    let (pid, _) = rest.split_once("_luid_")?;
    if pid.is_empty() || !pid.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let start = name.find("luid_")?;
    let rest = &name[start..];
    let (key, _) = rest.split_once("_engtype_")?;
    let (adapter, engine) = key.rsplit_once("_eng_")?;
    let (luid, physical) = adapter.rsplit_once("_phys_")?;
    if luid == "luid_"
        || physical.is_empty()
        || engine.is_empty()
        || !physical.chars().all(|ch| ch.is_ascii_digit())
        || !engine.chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }
    Some(key)
}

#[cfg(any(windows, test))]
fn aggregate_gpu_percent<'a>(items: impl Iterator<Item = (&'a str, f64)>) -> Option<f32> {
    let mut per_engine = std::collections::HashMap::<&str, f64>::new();
    for (name, value) in items {
        if !value.is_finite() || value < 0.0 {
            continue;
        }
        let Some(key) = engine_key(name) else {
            continue;
        };
        *per_engine.entry(key).or_default() += value;
    }
    per_engine
        .values()
        .copied()
        .reduce(f64::max)
        .map(|value| value.clamp(0.0, 100.0) as f32)
}

#[cfg(any(windows, test))]
fn pdh_value_is_valid(status: u32) -> bool {
    matches!(status, 0 | 1) // PDH_CSTATUS_VALID_DATA / PDH_CSTATUS_NEW_DATA
}

#[cfg(windows)]
mod windows_gpu {
    use super::{aggregate_gpu_percent, pdh_value_is_valid, AtomicBool, Ordering, SAMPLE_INTERVAL};
    use std::mem::{size_of, MaybeUninit};
    use std::time::SystemTime;
    use windows::{
        core::{w, PCWSTR},
        Win32::System::Performance::{
            PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData,
            PdhGetFormattedCounterArrayW, PdhOpenQueryW, PDH_FMT_COUNTERVALUE_ITEM_W,
            PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
        },
    };

    pub(super) struct Sampler {
        query: PDH_HQUERY,
        counter: PDH_HCOUNTER,
        last_collection: SystemTime,
    }

    impl Sampler {
        pub(super) fn open() -> Option<Self> {
            unsafe {
                let mut query = PDH_HQUERY::default();
                if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
                    return None;
                }
                let mut counter = PDH_HCOUNTER::default();
                if PdhAddEnglishCounterW(
                    query,
                    w!("\\GPU Engine(*)\\Utilization Percentage"),
                    0,
                    &mut counter,
                ) != 0
                {
                    let _ = PdhCloseQuery(query);
                    return None;
                }
                let sampler = Self {
                    query,
                    counter,
                    last_collection: SystemTime::now(),
                };
                // Rate counters require a baseline; retry rather than publishing zero on failure.
                if PdhCollectQueryData(query) != 0 {
                    return None;
                }
                Some(sampler)
            }
        }

        pub(super) fn sample(&mut self, stop: &AtomicBool) -> Option<f32> {
            std::thread::park_timeout(SAMPLE_INTERVAL);
            if stop.load(Ordering::Acquire)
                || self.last_collection.elapsed().ok()?.as_millis() > 2_000
            {
                return None;
            }
            self.last_collection = SystemTime::now();
            if unsafe { PdhCollectQueryData(self.query) } != 0 {
                return None;
            }
            read_aggregated(self.counter)
        }
    }

    impl Drop for Sampler {
        fn drop(&mut self) {
            unsafe {
                let _ = PdhCloseQuery(self.query);
            }
        }
    }

    fn read_aggregated(counter: PDH_HCOUNTER) -> Option<f32> {
        // Each retry probes the required size again: PDH warns that a failed
        // fill does not necessarily return a usable new buffer size.
        for _ in 0..3 {
            let mut bytes = 0u32;
            let mut count = 0u32;
            let status = unsafe {
                PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut bytes, &mut count, None)
            };
            if status != PDH_MORE_DATA && status != 0 {
                return None;
            }
            if bytes == 0 || count == 0 {
                return Some(0.0);
            }
            // Keep an unexpected provider response from allocating unbounded memory.
            if bytes > 16 * 1024 * 1024 {
                return None;
            }
            let units = (bytes as usize).div_ceil(size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>());
            // This allocation has the alignment of the PDH structs, unlike Vec<u8>.
            let mut buffer = vec![MaybeUninit::<PDH_FMT_COUNTERVALUE_ITEM_W>::zeroed(); units];
            let capacity = units * size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
            let mut used = capacity as u32;
            let status = unsafe {
                PdhGetFormattedCounterArrayW(
                    counter,
                    PDH_FMT_DOUBLE,
                    &mut used,
                    &mut count,
                    Some(buffer.as_mut_ptr().cast()),
                )
            };
            if status == PDH_MORE_DATA {
                continue;
            }
            if status != 0 || used as usize > capacity || count as usize > units {
                return None;
            }
            if count == 0 {
                return Some(0.0);
            }
            if count as usize * size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>() > used as usize {
                return None;
            }
            let start = buffer.as_ptr() as usize;
            let end = start + used as usize;
            let items = unsafe {
                std::slice::from_raw_parts(
                    buffer.as_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>(),
                    count as usize,
                )
            };
            let mut owned = Vec::with_capacity(items.len());
            for item in items {
                if !pdh_value_is_valid(item.FmtValue.CStatus) {
                    continue;
                }
                let address = item.szName.0 as usize;
                if address < start || address >= end || address % 2 != 0 {
                    continue;
                }
                // Names are packed into the same caller-owned buffer. Bound the
                // UTF-16 scan by that buffer instead of trusting a terminator.
                let name_units =
                    unsafe { std::slice::from_raw_parts(item.szName.0, (end - address) / 2) };
                let Some(length) = name_units.iter().position(|unit| *unit == 0) else {
                    continue;
                };
                let Ok(name) = String::from_utf16(&name_units[..length]) else {
                    continue;
                };
                let value = unsafe { item.FmtValue.Anonymous.doubleValue };
                owned.push((name, value));
            }
            return aggregate_gpu_percent(
                owned.iter().map(|(name, value)| (name.as_str(), *value)),
            );
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn residency_is_activity_not_frequency_weighted() {
        let states = [("OFF", 700), ("P1", 200), ("P2", 100)];
        assert_eq!(active_residency_percent(states.into_iter()), Some(30.0));
        assert_eq!(
            active_residency_percent([("DOWN", 100), ("P1", 0)].into_iter()),
            Some(0.0)
        );
        assert_eq!(
            active_residency_percent([("IDLE", 0), ("P1", 100)].into_iter()),
            Some(100.0)
        );
    }

    #[test]
    fn invalid_residency_windows_are_unavailable() {
        assert_eq!(active_residency_percent(std::iter::empty()), None);
        assert_eq!(
            active_residency_percent([("OFF", 0), ("P1", 0)].into_iter()),
            None
        );
        assert_eq!(
            active_residency_percent([("OFF", -1), ("P1", 1)].into_iter()),
            None
        );
        assert_eq!(
            active_residency_percent([("unknown", 100)].into_iter()),
            None
        );
    }

    #[test]
    fn sample_freshness_rejects_old_future_and_invalid_values() {
        let now = SystemTime::now();
        let value = Reading {
            percent: 35.0,
            sampled_at: now,
        };
        assert_eq!(value.fresh_value(now + Duration::from_secs(2)), Some(35.0));
        assert_eq!(value.fresh_value(now + Duration::from_secs(4)), None);
        assert_eq!(
            Reading {
                sampled_at: now + Duration::from_secs(1),
                ..value
            }
            .fresh_value(now),
            None
        );
        assert_eq!(
            Reading {
                percent: f32::NAN,
                ..value
            }
            .fresh_value(now),
            None
        );
    }

    #[test]
    fn processes_on_same_engine_are_summed() {
        let items = [
            ("pid_1_luid_0x0_0xA_phys_0_eng_0_engtype_3D", 40.0),
            ("pid_2_luid_0x0_0xA_phys_0_eng_0_engtype_3D", 30.0),
            ("pid_3_luid_0x0_0xA_phys_0_eng_1_engtype_Copy", 10.0),
        ];
        assert_eq!(aggregate_gpu_percent(items.into_iter()), Some(70.0));
    }

    #[test]
    fn adapters_physical_gpus_and_engines_are_not_added_together() {
        let items = [
            ("pid_1_luid_0x0_0xA_phys_0_eng_0_engtype_3D", 40.0),
            ("pid_2_luid_0x0_0xA_phys_1_eng_0_engtype_3D", 30.0),
            ("pid_3_luid_0x0_0xB_phys_0_eng_0_engtype_3D", 60.0),
        ];
        assert_eq!(aggregate_gpu_percent(items.into_iter()), Some(60.0));
    }

    #[test]
    fn invalid_windows_values_are_skipped_and_sum_is_clamped() {
        let name = "pid_1_luid_0x0_0xA_phys_0_eng_0_engtype_3D";
        assert_eq!(
            aggregate_gpu_percent([(name, 80.0), (name, 40.0)].into_iter()),
            Some(100.0)
        );
        assert_eq!(
            aggregate_gpu_percent([(name, f64::NAN), (name, -1.0), ("unknown", 40.0)].into_iter()),
            None
        );
        assert!(pdh_value_is_valid(0));
        assert!(pdh_value_is_valid(1));
        assert!(!pdh_value_is_valid(0x800007D5));
    }

    #[cfg(any(all(target_os = "macos", target_arch = "aarch64"), windows))]
    #[test]
    fn failed_sample_clears_previously_valid_reading() {
        let output = Mutex::new(None);
        publish(&output, Some(42.0));
        assert!(output.lock().unwrap().is_some());
        publish(&output, None);
        assert!(output.lock().unwrap().is_none());
    }
}
