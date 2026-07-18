<script lang="ts">
  import type { SpeedEntry } from "$lib/metrics";

  let { history, current }: { history: SpeedEntry[]; current: SpeedEntry } =
    $props();

  const WIDTH = 300;
  const HEIGHT = 96;

  let maxVal = $derived(
    Math.max(0.1, ...history.map((e) => Math.max(e.down, e.up))) * 1.15,
  );

  function toPath(data: SpeedEntry[], key: "down" | "up", scale: number): string {
    if (data.length < 2) return "";
    return data
      .map((d, i) => {
        const x = (i / (data.length - 1)) * WIDTH;
        const y = HEIGHT - 2 - (d[key] / scale) * (HEIGHT - 6);
        return `${i === 0 ? "M" : "L"} ${x.toFixed(1)} ${y.toFixed(1)}`;
      })
      .join(" ");
  }

  let downPath = $derived(toPath(history, "down", maxVal));
  let upPath = $derived(toPath(history, "up", maxVal));
  let downArea = $derived(
    downPath ? `${downPath} L ${WIDTH} ${HEIGHT} L 0 ${HEIGHT} Z` : "",
  );
  let peakDown = $derived(Math.max(0, ...history.map((e) => e.down)));

  function formatSpeed(v: number): string {
    if (v < 1) return `${(v * 1024).toFixed(0)} KB/s`;
    return `${v.toFixed(1)} MB/s`;
  }
</script>

<section class="card">
  <div class="card-header">
    <span class="card-label">Network</span>
    <span class="card-value">
      <span class="down">↓ {formatSpeed(current.down)}</span>
      <span class="up">↑ {formatSpeed(current.up)}</span>
    </span>
  </div>

  <svg viewBox="0 0 {WIDTH} {HEIGHT}" preserveAspectRatio="none">
    <line class="grid" x1="0" y1={HEIGHT * 0.25} x2={WIDTH} y2={HEIGHT * 0.25} />
    <line class="grid" x1="0" y1={HEIGHT * 0.5} x2={WIDTH} y2={HEIGHT * 0.5} />
    <line class="grid" x1="0" y1={HEIGHT * 0.75} x2={WIDTH} y2={HEIGHT * 0.75} />
    {#if history.length >= 2}
      <path class="area" d={downArea} />
      <path class="line down-line" d={downPath} />
      <path class="line up-line" d={upPath} />
    {/if}
  </svg>

  <div class="card-footer">
    <span class="legend">
      <span class="down">● Download</span>
      <span class="up">● Upload</span>
    </span>
    <span>Peak ↓ {formatSpeed(peakDown)}</span>
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
    display: flex;
    gap: 10px;
  }
  .down {
    color: #007aff;
    font-weight: 600;
  }
  .up {
    color: #34c759;
    font-weight: 600;
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
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .down-line {
    stroke: #007aff;
  }
  .up-line {
    stroke: #34c759;
  }
  .card-footer {
    display: flex;
    justify-content: space-between;
    margin-top: 10px;
    font-size: 11px;
    color: #86868b;
  }
  .card-footer .down,
  .card-footer .up {
    font-weight: 400;
  }
  .legend {
    display: flex;
    gap: 12px;
  }
</style>
