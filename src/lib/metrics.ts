/** Mirrors Rust `SpeedEntry` / snapshot fields — extend when adding GPU, etc. */

export interface SpeedEntry {
  down: number;
  up: number;
}

export interface MemoryEntry {
  used_bytes: number;
  total_bytes: number;
}

export interface MetricsSnapshot {
  network: SpeedEntry;
  cpu_percent: number;
  memory: MemoryEntry;
  memory_percent: number;
  /** 0–100 when `gpu_supported` */
  gpu_percent: number;
  gpu_supported: boolean;
}

export interface MetricsHistory {
  network: SpeedEntry[];
  cpu: number[];
  memory: number[];
  gpu: number[];
  gpu_supported: boolean;
}
