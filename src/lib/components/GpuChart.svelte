<script lang="ts">
  export let history: number[];
  export let current: number;
  /** IOReport / Apple Silicon — false on Intel Macs and non-macOS builds */
  export let supported: boolean;

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

  $: gpuPath = toPath(history);
</script>

<section class="chart-block">
  <div class="headline">
    {#if supported}
      <span class="gpu">GPU {current.toFixed(0)}%</span>
    {:else}
      <span class="gpu-muted">GPU — n/v</span>
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

    {#if supported && history.length >= 2}
      <path class="line gpu" d={gpuPath} />
    {/if}
  </svg>

  <div class="legend">
    {#if supported}
      <span class="legend-gpu">■ GPU (Apple Silicon, IOReport)</span>
    {:else}
      <span class="legend-unavail">Grafik-Auslastung: Apple Silicon, macOS</span>
    {/if}
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
    font-size: 22px;
    font-weight: 600;
    margin-bottom: 12px;
  }
  .gpu {
    color: #4fc3f7;
  }
  .gpu-muted {
    color: #666e80;
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
  .line.gpu {
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
  .legend-gpu {
    color: #4fc3f7;
  }
  .legend-unavail {
    color: #6a7088;
  }
</style>
