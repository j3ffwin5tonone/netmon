import type { MetricsHistory, MetricsSnapshot } from "./metrics";

export const HISTORY_MS = 60_000;
export const HISTORY_LIMIT = 60;
export const STALE_MS = 2_500;

/** Sequence numbers establish order even when async requests finish out of order. */
export function mergeSamples(
  existing: readonly MetricsSnapshot[],
  incoming: readonly MetricsSnapshot[],
  now: number,
): MetricsSnapshot[] {
  const bySequence = new Map<number, MetricsSnapshot>();
  for (const sample of [...existing, ...incoming]) {
    if (Number.isFinite(sample.timestamp_ms) && Number.isSafeInteger(sample.sequence)) {
      bySequence.set(sample.sequence, sample);
    }
  }
  const sorted = [...bySequence.values()].sort((a, b) => a.sequence - b.sequence);
  // A backwards wall-clock adjustment starts a new chart segment rather than
  // leaving old, future-dated points on screen.
  let start = 0;
  for (let i = 1; i < sorted.length; i++) {
    if (sorted[i].timestamp_ms < sorted[i - 1].timestamp_ms) start = i;
  }
  return sorted.slice(start)
    .filter((s) => s.timestamp_ms > now - HISTORY_MS && s.timestamp_ms <= now + STALE_MS)
    .slice(-HISTORY_LIMIT);
}

export function latestFresh(samples: readonly MetricsSnapshot[], now: number): MetricsSnapshot | null {
  const latest = samples.at(-1);
  return latest && now - latest.timestamp_ms <= STALE_MS ? latest : null;
}

/** Keep a local clock only while visible; it never performs IPC. */
export function createVisibilityClock(
  tick: () => void,
  schedule: (tick: () => void) => () => void = (tick) => {
    const timer = setInterval(tick, 1000);
    return () => clearInterval(timer);
  },
) {
  let stop: (() => void) | null = null;
  let disposed = false;
  return {
    setVisible(visible: boolean) {
      if (disposed) return;
      if (visible && !stop) {
        tick();
        stop = schedule(tick);
      } else if (!visible && stop) {
        stop();
        stop = null;
      }
    },
    dispose() {
      disposed = true;
      stop?.();
      stop = null;
    },
  };
}

/** Backend owns the history; keep only events newer than its returned snapshot. */
export function reconcileHistory(
  current: readonly MetricsSnapshot[],
  remote: readonly MetricsSnapshot[],
  now: number,
): MetricsSnapshot[] {
  const newest = remote.reduce((last, sample) => Math.max(last, sample.sequence), -1);
  return mergeSamples(remote, current.filter((s) => s.sequence > newest), now);
}

export interface DashboardState {
  samples: MetricsSnapshot[];
  networkSelection: string | null;
  now: number;
  error: string | null;
}

/** Contains the request/event race handling independently of the view lifecycle. */
export function createHistoryController(
  fetchHistory: () => Promise<MetricsHistory>,
  publish: (state: DashboardState) => void,
  clock: () => number = Date.now,
) {
  let state: DashboardState = { samples: [], networkSelection: null, now: clock(), error: null };
  let request = 0;
  let disposed = false;
  function send() {
    if (!disposed) publish(state);
  }
  return {
    advanceClock() {
      if (disposed) return;
      const now = clock();
      state = { ...state, now, samples: mergeSamples(state.samples, [], now) };
      send();
    },
    receive(sample: MetricsSnapshot) {
      if (disposed) return;
      const now = clock();
      state = { ...state, now, samples: mergeSamples(state.samples, [sample], now) };
      send();
    },
    async synchronize() {
      const ownRequest = ++request;
      const now = clock();
      state = { ...state, now, samples: mergeSamples(state.samples, [], now) };
      send();
      try {
        const remote = await fetchHistory();
        if (disposed || ownRequest !== request) return;
        const now = clock();
        state = {
          samples: reconcileHistory(state.samples, remote.samples, now),
          networkSelection: remote.network_selection,
          now,
          error: null,
        };
      } catch (error) {
        if (disposed || ownRequest !== request) return;
        state = { ...state, error: `Could not refresh metrics: ${String(error)}` };
      }
      send();
    },
    dispose() { disposed = true; request++; },
  };
}
