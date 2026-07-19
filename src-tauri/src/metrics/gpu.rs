//! GPU load sampling.
//!
//! - **Apple Silicon:** [macmon](https://github.com/vladkens/macmon) IOReport. `macmon::Sampler`
//!   is not `Send`, so it lives on a dedicated thread and publishes via atomics.
//! - **Windows:** PDH `\GPU Engine(*)\Utilization Percentage` (same source as Task Manager).
//! - **Elsewhere:** unsupported stub (`gpu_supported = false`).

use super::HISTORY_LEN;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
#[cfg(any(
    all(target_os = "macos", target_arch = "aarch64"),
    windows
))]
use std::time::Duration;

/// Discard `get_metrics_now` baselines older than this (slightly above the 1s sample cadence).
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const STALE_AFTER_MS: u32 = 2_000;

#[cfg(any(
    all(target_os = "macos", target_arch = "aarch64"),
    windows
))]
const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

/// Shared flag + last GPU % (f32 bits) so `measure()` never blocks on the sampler.
struct GpuControl {
    supported: Arc<AtomicBool>,
    last_bits: Arc<AtomicU32>,
    #[cfg(any(
        all(target_os = "macos", target_arch = "aarch64"),
        windows
    ))]
    _join: std::thread::JoinHandle<()>,
}

impl GpuControl {
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        windows
    )))]
    fn new_stub() -> Self {
        Self {
            supported: Arc::new(AtomicBool::new(false)),
            last_bits: Arc::new(AtomicU32::new(0.0f32.to_bits())),
        }
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn new_worker() -> Self {
        let supported = Arc::new(AtomicBool::new(false));
        let last_bits = Arc::new(AtomicU32::new(0.0f32.to_bits()));
        let sup = Arc::clone(&supported);
        let last = Arc::clone(&last_bits);

        let join = std::thread::Builder::new()
            .name("netmon-gpu".to_string())
            .spawn(move || {
                let mut sampler = match macmon::Sampler::new() {
                    Ok(s) => {
                        sup.store(true, Ordering::Relaxed);
                        s
                    }
                    Err(_) => return,
                };

                loop {
                    match sampler.get_metrics_now(STALE_AFTER_MS) {
                        Ok(Some(m)) => {
                            let p = (m.gpu_usage.1 * 100.0).clamp(0.0, 100.0);
                            last.store(p.to_bits(), Ordering::Relaxed);
                        }
                        Ok(None) => {
                            // Baseline established, or stale baseline discarded — keep last %.
                        }
                        Err(_) => {}
                    }
                    std::thread::sleep(SAMPLE_INTERVAL);
                }
            })
            .expect("spawn netmon-gpu");

        Self {
            supported,
            last_bits,
            _join: join,
        }
    }

    #[cfg(windows)]
    fn new_worker() -> Self {
        let supported = Arc::new(AtomicBool::new(false));
        let last_bits = Arc::new(AtomicU32::new(0.0f32.to_bits()));
        let sup = Arc::clone(&supported);
        let last = Arc::clone(&last_bits);

        let join = std::thread::Builder::new()
            .name("netmon-gpu".to_string())
            .spawn(move || {
                let Some(mut sampler) = windows_gpu::Sampler::open() else {
                    return;
                };
                sup.store(true, Ordering::Relaxed);

                // PDH rate counters need a baseline sample before the first valid reading.
                let _ = sampler.collect();
                std::thread::sleep(Duration::from_millis(200));

                loop {
                    if let Some(p) = sampler.sample() {
                        last.store(p.to_bits(), Ordering::Relaxed);
                    }
                    std::thread::sleep(SAMPLE_INTERVAL);
                }
            })
            .expect("spawn netmon-gpu");

        Self {
            supported,
            last_bits,
            _join: join,
        }
    }

    fn supported(&self) -> bool {
        self.supported.load(Ordering::Relaxed)
    }

    fn last(&self) -> f32 {
        f32::from_bits(self.last_bits.load(Ordering::Relaxed))
    }
}

pub struct GpuMetrics {
    history: VecDeque<f32>,
    control: GpuControl,
}

impl GpuMetrics {
    pub fn new() -> Self {
        #[cfg(any(
            all(target_os = "macos", target_arch = "aarch64"),
            windows
        ))]
        {
            Self {
                history: VecDeque::with_capacity(HISTORY_LEN),
                control: GpuControl::new_worker(),
            }
        }
        #[cfg(not(any(
            all(target_os = "macos", target_arch = "aarch64"),
            windows
        )))]
        {
            Self {
                history: VecDeque::with_capacity(HISTORY_LEN),
                control: GpuControl::new_stub(),
            }
        }
    }

    pub fn supported(&self) -> bool {
        self.control.supported()
    }

    /// Returns the latest GPU % without waiting on the platform sampler.
    pub fn measure(&mut self) -> f32 {
        let pct = self.control.last();
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

#[cfg(windows)]
mod windows_gpu {
    use std::collections::HashMap;
    use windows::{
        core::{w, PCWSTR},
        Win32::System::Performance::{
            PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData,
            PdhGetFormattedCounterArrayW, PdhOpenQueryW, PDH_FMT_COUNTERVALUE_ITEM_W,
            PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
        },
    };

    const COUNTER_PATH: PCWSTR = w!("\\GPU Engine(*)\\Utilization Percentage");

    pub(super) struct Sampler {
        query: PDH_HQUERY,
        counter: PDH_HCOUNTER,
    }

    impl Sampler {
        pub(super) fn open() -> Option<Self> {
            unsafe {
                let mut query = PDH_HQUERY::default();
                if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
                    return None;
                }
                let mut counter = PDH_HCOUNTER::default();
                if PdhAddEnglishCounterW(query, COUNTER_PATH, 0, &mut counter) != 0 {
                    let _ = PdhCloseQuery(query);
                    return None;
                }
                Some(Self { query, counter })
            }
        }

        pub(super) fn collect(&mut self) -> bool {
            unsafe { PdhCollectQueryData(self.query) == 0 }
        }

        pub(super) fn sample(&mut self) -> Option<f32> {
            if !self.collect() {
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

    /// Adapter key from instance names like
    /// `pid_1234_luid_0x00000000_0x00012AB3_phys_0_eng_0_engtype_3D`.
    fn luid_key(name: &str) -> &str {
        if let Some(start) = name.find("luid_") {
            let rest = &name[start..];
            if let Some(end) = rest.find("_phys") {
                &rest[..end]
            } else {
                rest
            }
        } else {
            "unknown"
        }
    }

    /// Per-adapter max over engines, then max across adapters (engines run in parallel).
    pub(super) fn aggregate_gpu_percent<'a>(
        items: impl Iterator<Item = (&'a str, f64)>,
    ) -> f32 {
        let mut per_adapter: HashMap<&str, f64> = HashMap::new();
        for (name, value) in items {
            if !value.is_finite() || value < 0.0 {
                continue;
            }
            let key = luid_key(name);
            let entry = per_adapter.entry(key).or_insert(0.0);
            *entry = (*entry).max(value);
        }
        per_adapter
            .values()
            .copied()
            .fold(0.0_f64, f64::max)
            .clamp(0.0, 100.0) as f32
    }

    fn read_aggregated(counter: PDH_HCOUNTER) -> Option<f32> {
        unsafe {
            let mut buf_size = 0u32;
            let mut item_count = 0u32;
            let status = PdhGetFormattedCounterArrayW(
                counter,
                PDH_FMT_DOUBLE,
                &mut buf_size,
                &mut item_count,
                None,
            );
            if status != PDH_MORE_DATA && status != 0 {
                return None;
            }
            if buf_size == 0 || item_count == 0 {
                return Some(0.0);
            }

            let mut buffer = vec![0u8; buf_size as usize];
            let status = PdhGetFormattedCounterArrayW(
                counter,
                PDH_FMT_DOUBLE,
                &mut buf_size,
                &mut item_count,
                Some(buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W),
            );
            if status != 0 {
                return None;
            }

            let items = std::slice::from_raw_parts(
                buffer.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W,
                item_count as usize,
            );

            let pairs = items.iter().filter_map(|item| {
                if item.FmtValue.CStatus != 0 {
                    return None;
                }
                let name = item.szName.to_string().ok()?;
                let value = item.FmtValue.Anonymous.doubleValue;
                Some((name, value))
            });

            // Names must outlive the aggregate call; collect owned strings first.
            let owned: Vec<(String, f64)> = pairs.collect();
            Some(aggregate_gpu_percent(
                owned.iter().map(|(n, v)| (n.as_str(), *v)),
            ))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{aggregate_gpu_percent, luid_key};

        #[test]
        fn luid_key_extracts_adapter() {
            let name =
                "pid_1234_luid_0x00000000_0x00012AB3_phys_0_eng_0_engtype_3D";
            assert_eq!(luid_key(name), "luid_0x00000000_0x00012AB3");
        }

        #[test]
        fn aggregate_takes_max_per_adapter_then_max_adapters() {
            let items = [
                (
                    "pid_1_luid_0x0_0xA_phys_0_eng_0_engtype_3D",
                    40.0,
                ),
                (
                    "pid_2_luid_0x0_0xA_phys_0_eng_1_engtype_Copy",
                    10.0,
                ),
                (
                    "pid_3_luid_0x0_0xB_phys_0_eng_0_engtype_3D",
                    70.0,
                ),
            ];
            let pct = aggregate_gpu_percent(items.iter().map(|(n, v)| (*n, *v)));
            assert!((pct - 70.0).abs() < f32::EPSILON);
        }
    }
}
