<script lang="ts">
  import type { SpeedEntry } from "$lib/metrics";

  export let history: SpeedEntry[];
  export let current: SpeedEntry;

  const WIDTH = 360;
  const HEIGHT = 160;
  const PAD = 30;

  $: maxVal = Math.max(0.1, ...history.map((e) => Math.max(e.down, e.up))) * 1.2;

  function toPath(data: SpeedEntry[], key: "down" | "up"): string {
    if (data.length < 2) return "";
    return data
      .map((d, i) => {
        const x = PAD + (i / (data.length - 1)) * (WIDTH - PAD * 2);
        const y = HEIGHT - PAD - (d[key] / maxVal) * (HEIGHT - PAD * 2);
        return `${i === 0 ? "M" : "L"} ${x.toFixed(1)} ${y.toFixed(1)}`;
      })
      .join(" ");
  }

  $: downPath = toPath(history, "down");
  $: upPath = toPath(history, "up");

  function formatSpeed(v: number): string {
    if (v < 1) return `${(v * 1024).toFixed(0)} KB/s`;
    return `${v.toFixed(1)} MB/s`;
  }
</script>

<section class="chart-block">
  <div class="headline">
    <span class="down">↓ {formatSpeed(current.down)}</span>
    <span class="up">↑ {formatSpeed(current.up)}</span>
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

    <text class="label" x={PAD - 4} y={PAD + 4} text-anchor="end">
      {formatSpeed(maxVal)}
    </text>
    <text class="label" x={PAD - 4} y={HEIGHT - PAD} text-anchor="end">0</text>

    {#if history.length >= 2}
      <path class="line down" d={downPath} />
      <path class="line up" d={upPath} />
    {/if}
  </svg>

  <div class="legend">
    <span class="legend-down">■ Download</span>
    <span class="legend-up">■ Upload</span>
    <span class="legend-time">Letzte 60 s</span>
  </div>
</section>

<style>
  .chart-block {
    margin-bottom: 8px;
  }
  .headline {
    display: flex;
    justify-content: center;
    gap: 24px;
    font-size: 22px;
    font-weight: 600;
    margin-bottom: 12px;
  }
  .down {
    color: #4fc3f7;
  }
  .up {
    color: #81c784;
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
  .line.down {
    stroke: #4fc3f7;
  }
  .line.up {
    stroke: #81c784;
  }
  .legend {
    display: flex;
    justify-content: center;
    gap: 16px;
    font-size: 11px;
    color: #888;
    margin-top: 8px;
  }
  .legend-down {
    color: #4fc3f7;
  }
  .legend-up {
    color: #81c784;
  }
</style>
