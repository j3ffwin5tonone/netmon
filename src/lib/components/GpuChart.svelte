<script lang="ts">
  import type { MetricsSnapshot } from "$lib/metrics";
  import { peak, formatPercent } from "$lib/chart";
  import MetricPlot from "./MetricPlot.svelte";
  let { samples, current, end }: { samples: MetricsSnapshot[]; current: MetricsSnapshot | null; end: number } = $props();
  let points = $derived(samples.map((s) => ({ timestamp: s.timestamp_ms, value: s.gpu_percent })));
</script>
<section class="metric-card">
  <div class="metric-header"><span class="metric-label">GPU</span><span class="metric-value">{current ? current.gpu_percent === null ? "n/a" : formatPercent(current.gpu_percent) : "—"}</span></div>
  <MetricPlot {points} {end} color="#af52de" label="GPU active time, zero to 100 percent, over the last 60 seconds; unavailable samples are gaps" />
  <div class="metric-footer"><span>{current?.gpu_percent === null ? "Unavailable or stale" : "GPU active time"}</span><span>Peak {formatPercent(peak(points))}</span></div>
</section>
<style>.metric-value { color: #9850b8; }</style>
