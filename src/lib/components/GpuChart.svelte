<script lang="ts">
  /** Platform GPU sampler — false on Intel Macs / when PDH or IOReport is unavailable */
  let {
    history,
    current,
    supported,
  }: { history: number[]; current: number; supported: boolean } = $props();

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

  let gpuPath = $derived(toPath(history));
  let gpuArea = $derived(
    gpuPath ? `${gpuPath} L ${WIDTH} ${HEIGHT} L 0 ${HEIGHT} Z` : "",
  );
  let peak = $derived(history.length ? Math.max(...history) : 0);
</script>

<section class="card">
  <div class="card-header">
    <span class="card-label">GPU</span>
    {#if supported}
      <span class="card-value">{current.toFixed(0)}%</span>
    {:else}
      <span class="card-value muted">n/a</span>
    {/if}
  </div>

  {#if supported}
    <svg viewBox="0 0 {WIDTH} {HEIGHT}" preserveAspectRatio="none">
      <line class="grid" x1="0" y1={HEIGHT * 0.25} x2={WIDTH} y2={HEIGHT * 0.25} />
      <line class="grid" x1="0" y1={HEIGHT * 0.5} x2={WIDTH} y2={HEIGHT * 0.5} />
      <line class="grid" x1="0" y1={HEIGHT * 0.75} x2={WIDTH} y2={HEIGHT * 0.75} />
      {#if history.length >= 2}
        <path class="area" d={gpuArea} />
        <path class="line" d={gpuPath} />
      {/if}
    </svg>
  {:else}
    <div class="unsupported">GPU metrics unavailable on this device</div>
  {/if}

  <div class="card-footer">
    <span>GPU utilization</span>
    {#if supported}
      <span>Peak {peak.toFixed(0)}%</span>
    {/if}
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
    font-weight: 600;
    color: #af52de;
    font-variant-numeric: tabular-nums;
  }
  .card-value.muted {
    color: #aeaeb2;
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
    fill: rgba(175, 82, 222, 0.1);
  }
  .line {
    fill: none;
    stroke: #af52de;
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .unsupported {
    flex: 1;
    min-height: 80px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    color: #aeaeb2;
    background: repeating-linear-gradient(-45deg, #fafafa 0 8px, #f4f4f6 8px 16px);
    border-radius: 6px;
  }
  .card-footer {
    display: flex;
    justify-content: space-between;
    margin-top: 10px;
    font-size: 11px;
    color: #86868b;
  }
</style>
