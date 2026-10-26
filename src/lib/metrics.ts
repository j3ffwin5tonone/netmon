/** Mirrors the Rust snapshot. Network rates use binary MiB/s. */
export interface SpeedEntry {
  down: number;
  up: number;
}

export interface MemoryEntry {
  used_bytes: number;
  total_bytes: number;
}

export interface MetricsSnapshot {
  sequence: number;
  timestamp_ms: number;
  network: SpeedEntry;
  network_interface: string | null;
  network_interfaces: string[];
  cpu_percent: number;
  memory: MemoryEntry;
  memory_percent: number;
  /** Active GPU time, 0–100; null when unsupported or stale. */
  gpu_percent: number | null;
}

export interface MetricsHistory {
  samples: MetricsSnapshot[];
  network_selection: string | null;
}
