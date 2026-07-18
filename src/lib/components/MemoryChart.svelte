<script lang="ts">
  import type { MemoryEntry } from "$lib/metrics";

  let {
    history,
    current,
    memory,
  }: { history: number[]; current: number; memory: MemoryEntry } = $props();

  const WIDTH = 300;
  const HEIGHT = 96;
  const SCALE_MAX = 100;

  function toPath(data: number[]): string {
    if (data.length < 2) return "";
    return data
      .map((d, i) => {
        const x = (i / (data.length - 1)) * WIDTH;
        const clamped = Math.min(SCALE_MAX, Math.max(0, d));
        const y = HEIGHT - 2 - (clamped / SCALE_MAX) * (HEIGHT - 6);
        return `${i === 0 ? "M" : "L"} ${x.toFixed(1)} ${y.toFixed(1)}`;
      })
      .join(" ");
  }

  function formatGb(bytes: number): string {
    return (bytes / 1024 ** 3).toFixed(1);
  }

  let ramPath = $derived(toPath(history));
  let ramArea = $derived(
    ramPath ? `${ramPath} L ${WIDTH} ${HEIGHT} L 0 ${HEIGHT} Z` : "",
  );
  let avg = $derived(
    history.length ? history.reduce((a, b) => a + b, 0) / history.length : 0,
  );
  let hasSize = $derived(memory.total_bytes > 0);
</script>

<section class="card">
  <div class="card-header">
    <span class="card-label">Memory</span>
    <span class="card-value">
      <span class="pct">{current.toFixed(0)}%</span>
      {#if hasSize}
        <span class="sub">{formatGb(memory.used_bytes)} / {formatGb(memory.total_bytes)} GB</span>
      {/if}
    </span>
  </div>

  <svg viewBox="0 0 {WIDTH} {HEIGHT}" preserveAspectRatio="none">
    <line class="grid" x1="0" y1={HEIGHT * 0.25} x2={WIDTH} y2={HEIGHT * 0.25} />
    <line class="grid" x1="0" y1={HEIGHT * 0.5} x2={WIDTH} y2={HEIGHT * 0.5} />
    <line class="grid" x1="0" y1={HEIGHT * 0.75} x2={WIDTH} y2={HEIGHT * 0.75} />
    {#if history.length >= 2}
      <path class="area" d={ramArea} />
      <path class="line" d={ramPath} />
    {/if}
  </svg>

  <div class="card-footer">
    <span>Avg {avg.toFixed(0)}%</span>
    <span>Last 60 s</span>
  </div>
</section>

<style>
  .card {
    background: #fff;
    border-radius: 10px;
    border: 0.5px solid #e2e2e6;
    padding: 14px 16px 12px;
    display: flex;
    flex-direction: column;
  }
  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 10px;
  }
  .card-label {
    font-size: 12px;
    font-weight: 600;
    color: #86868b;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .card-value {
    font-family: "SF Mono", ui-monospace, Menlo, monospace;
    font-size: 15px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .pct {
    font-weight: 600;
    color: #007aff;
  }
  .sub {
    color: #86868b;
    font-size: 12px;
  }
  svg {
    width: 100%;
    flex: 1;
    min-height: 80px;
    display: block;
  }
  .grid {
    stroke: #f0f0f2;
    stroke-width: 1;
  }
  .area {
    fill: rgba(0, 122, 255, 0.09);
  }
  .line {
    fill: none;
    stroke: #007aff;
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .card-footer {
    display: flex;
    justify-content: space-between;
    margin-top: 10px;
    font-size: 11px;
    color: #86868b;
  }
</style>
