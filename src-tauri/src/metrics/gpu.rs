//! GPU load (residency vs. max frequency) on Apple Silicon via
//! [macmon](https://crates.io/crates/macmon) IOReport. `macmon::Sampler` is not `Send`, so it
//! lives on a dedicated thread and answers sample requests over a channel.

use super::HISTORY_LEN;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

/// `get_metrics` needs at least 100 ms; four subsample steps use this total window.
const SAMPLE_WINDOW_MS: u32 = 200;

/// Shared flag so `supported()` does not need to touch the worker after startup.
struct GpuControl {
    supported: Arc<AtomicBool>,
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    req: mpsc::Sender<()>,
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    resp: mpsc::Receiver<f32>,
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    _join: std::thread::JoinHandle<()>,
}

impl GpuControl {
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    fn new_stub() -> Self {
        Self {
            supported: Arc::new(AtomicBool::new(false)),
        }
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn new_worker() -> Self {
        let supported = Arc::new(AtomicBool::new(false));
        let sup = Arc::clone(&supported);
        let (req_tx, req_rx) = mpsc::channel::<()>();
        let (resp_tx, resp_rx) = mpsc::channel();

        let join = std::thread::Builder::new()
            .name("netmon-gpu".to_string())
            .spawn(move || {
                let mut sampler = match macmon::Sampler::new() {
                    Ok(s) => {
                        sup.store(true, Ordering::Relaxed);
                        Some(s)
                    }
                    Err(_) => None,
                };
                let mut last = 0.0f32;
                for () in req_rx {
                    let v = if let Some(s) = sampler.as_mut() {
                        match s.get_metrics(SAMPLE_WINDOW_MS) {
                            Ok(m) => {
                                // Second tuple value is ~0.0..=1.0 utilization vs. max frequency.
                                let p = (m.gpu_usage.1 * 100.0).clamp(0.0, 100.0);
                                last = p;
                                p
                            }
                            Err(_) => last,
                        }
                    } else {
                        0.0
                    };
                    if resp_tx.send(v).is_err() {
                        break;
                    }
                }
            })
            .expect("spawn netmon-gpu");

        Self {
            supported,
            req: req_tx,
            resp: resp_rx,
            _join: join,
        }
    }

    fn supported(&self) -> bool {
        self.supported.load(Ordering::Relaxed)
    }

    fn sample(&mut self) -> f32 {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            if self.req.send(()).is_err() {
                return 0.0;
            }
            return self.resp.recv().unwrap_or(0.0);
        }
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            0.0
        }
    }
}

pub struct GpuMetrics {
    history: Vec<f32>,
    control: GpuControl,
}

impl GpuMetrics {
    pub fn new() -> Self {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            Self {
                history: Vec::with_capacity(HISTORY_LEN),
                control: GpuControl::new_worker(),
            }
        }
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            Self {
                history: Vec::with_capacity(HISTORY_LEN),
                control: GpuControl::new_stub(),
            }
        }
    }

    pub fn supported(&self) -> bool {
        self.control.supported()
    }

    pub fn measure(&mut self) -> f32 {
        let pct = self.control.sample();
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
