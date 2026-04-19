/** Mirrors Rust `SpeedEntry` / snapshot fields — extend when adding RAM, GPU, etc. */

export interface SpeedEntry {
  down: number;
  up: number;
}

export interface MetricsSnapshot {
  network: SpeedEntry;
  cpu_percent: number;
}

export interface MetricsHistory {
  network: SpeedEntry[];
  cpu: number[];
}
