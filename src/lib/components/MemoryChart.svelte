<script lang="ts">
  import type { MemoryEntry } from "$lib/metrics";

  export let history: number[];
  export let current: number;
  export let memory: MemoryEntry;

  const WIDTH = 360;
  const HEIGHT = 160;
  const PAD = 30;
  const SCALE_MAX = 100;

  function toPath(data: number[]): string {
    if (data.length < 2) return "";
    return data
      .map((d, i) => {
        const x = PAD + (i / (data.length - 1)) * (WIDTH - PAD * 2);
        const clamped = Math.min(SCALE_MAX, Math.max(0, d));
        const y = HEIGHT - PAD - (clamped / SCALE_MAX) * (HEIGHT - PAD * 2);
        return `${i === 0 ? "M" : "L"} ${x.toFixed(1)} ${y.toFixed(1)}`;
      })
      .join(" ");
  }

  function formatGb(bytes: number): string {
    return (bytes / 1024 ** 3).toLocaleString("de-DE", {
      minimumFractionDigits: 1,
      maximumFractionDigits: 1,
    });
  }

  $: ramPath = toPath(history);
  $: hasSize = memory.total_bytes > 0;
</script>

<section class="chart-block">
  <div class="headline">
    <span class="ram">RAM {current.toFixed(0)}%</span>
    {#if hasSize}
      <span class="sub">({formatGb(memory.used_bytes)} / {formatGb(memory.total_bytes)} GB)</span>
    {:else}
      <span class="sub dim">(…)</span>
    {/if}
  </div>

  <svg viewBox="0 0 {WIDTH} {HEIGHT}">
    {#each [0.25, 0.5, 0.75] as frac}
      <line
        class="grid"
        x1={PAD}
        y1={HEIGHT - PAD - frac * (HEIGHT - PAD * 2)}
        x2={WIDTH - PAD}
        y2={HEIGHT - PAD - frac * (HEIGHT - PAD * 2)}
      />
    {/each}

    <line class="axis" x1={PAD} y1={PAD} x2={PAD} y2={HEIGHT - PAD} />
    <line class="axis" x1={PAD} y1={HEIGHT - PAD} x2={WIDTH - PAD} y2={HEIGHT - PAD} />

    <text class="label" x={PAD - 4} y={PAD + 4} text-anchor="end">100%</text>
    <text class="label" x={PAD - 4} y={HEIGHT - PAD} text-anchor="end">0%</text>

    {#if history.length >= 2}
      <path class="line ram" d={ramPath} />
    {/if}
  </svg>

  <div class="legend">
    <span class="legend-ram">■ RAM-Auslastung</span>
    <span class="legend-time">Letzte 60 s</span>
  </div>
</section>

<style>
  .chart-block {
    margin-bottom: 8px;
  }
  .headline {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    align-items: baseline;
    gap: 8px;
    font-size: 22px;
    font-weight: 600;
    margin-bottom: 12px;
  }
  .ram {
    color: #4fc3f7;
  }
  .sub {
    font-size: 15px;
    font-weight: 500;
    color: #b0bec5;
  }
  .sub.dim {
    color: #666;
  }
  svg {
    width: 100%;
    height: auto;
  }
  .grid {
    stroke: #2a2a4a;
    stroke-width: 0.5;
  }
  .axis {
    stroke: #3a3a5a;
    stroke-width: 1;
  }
  .label {
    fill: #888;
    font-size: 9px;
  }
  .line {
    fill: none;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .line.ram {
    stroke: #4fc3f7;
  }
  .legend {
    display: flex;
    justify-content: center;
    gap: 16px;
    font-size: 11px;
    color: #888;
    margin-top: 8px;
  }
  .legend-ram {
    color: #4fc3f7;
  }
</style>
