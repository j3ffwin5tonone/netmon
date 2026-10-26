import { HISTORY_MS, STALE_MS } from "./history";

export interface ChartPoint {
  timestamp: number;
  value: number | null;
  /** Changing interface means changing measurement source: do not join lines. */
  source?: string | null;
}

export const CHART_WIDTH = 300;
export const CHART_HEIGHT = 96;

export function chartPath(points: readonly ChartPoint[], end: number, max: number): string {
  const parts: string[] = [];
  let previous: ChartPoint | undefined;
  for (const point of points) {
    if (point.value === null || !Number.isFinite(point.value) || point.timestamp <= end - HISTORY_MS || point.timestamp > end) {
      previous = undefined;
      continue;
    }
    const contiguous = previous && point.timestamp > previous.timestamp
      && point.timestamp - previous.timestamp <= STALE_MS && point.source === previous.source;
    const x = ((point.timestamp - (end - HISTORY_MS)) / HISTORY_MS) * CHART_WIDTH;
    const value = Math.max(0, Math.min(max, point.value));
    const y = CHART_HEIGHT - 2 - (value / Math.max(max, 0.000001)) * (CHART_HEIGHT - 6);
    parts.push(`${contiguous ? "L" : "M"} ${x.toFixed(1)} ${y.toFixed(1)}`);
    previous = point;
  }
  return parts.join(" ");
}

export function validValues(points: readonly ChartPoint[]): number[] {
  return points.flatMap((p) => p.value !== null && Number.isFinite(p.value) ? [p.value] : []);
}

export function average(points: readonly ChartPoint[]): number | null {
  const values = validValues(points);
  return values.length ? values.reduce((a, b) => a + b, 0) / values.length : null;
}

export function peak(points: readonly ChartPoint[]): number | null {
  const values = validValues(points);
  return values.length ? Math.max(...values) : null;
}

export function formatPercent(value: number | null | undefined): string {
  return value === null || value === undefined || !Number.isFinite(value) ? "—" : `${value.toFixed(0)}%`;
}

export function formatSpeed(mib: number | null | undefined): string {
  if (mib === null || mib === undefined || !Number.isFinite(mib)) return "—";
  const value = Math.max(0, mib);
  if (value < 1 / 1024) return `${(value * 1024 ** 2).toFixed(0)} B/s`;
  if (value < 1) return `${(value * 1024).toFixed(0)} KiB/s`;
  if (value >= 1024) return `${(value / 1024).toFixed(1)} GiB/s`;
  return `${value.toFixed(1)} MiB/s`;
}

export function formatGiB(bytes: number): string {
  return `${(bytes / 1024 ** 3).toFixed(1)} GiB`;
}
