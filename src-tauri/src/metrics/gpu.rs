//! GPU load (residency vs. max frequency) on Apple Silicon via
//! [macmon](https://github.com/vladkens/macmon) IOReport. `macmon::Sampler` is not `Send`, so it
//! lives on a dedicated thread and publishes the last sample via atomics.

use super::HISTORY_LEN;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Discard `get_metrics_now` baselines older than this (slightly above the 1s sample cadence).
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const STALE_AFTER_MS: u32 = 2_000;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

/// Shared flag + last GPU % (f32 bits) so `measure()` never blocks on IOReport.
struct GpuControl {
    supported: Arc<AtomicBool>,
    last_bits: Arc<AtomicU32>,
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    _join: std::thread::JoinHandle<()>,
}

impl GpuControl {
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
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
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            Self {
                history: VecDeque::with_capacity(HISTORY_LEN),
                control: GpuControl::new_worker(),
            }
        }
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
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

    /// Returns the latest GPU % without waiting on IOReport (worker updates asynchronously).
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
