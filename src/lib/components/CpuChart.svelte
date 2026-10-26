<script lang="ts">
  import type { MetricsSnapshot } from "$lib/metrics";
  import { average, peak, formatPercent } from "$lib/chart";
  import MetricPlot from "./MetricPlot.svelte";
  let { samples, current, end }: { samples: MetricsSnapshot[]; current: MetricsSnapshot | null; end: number } = $props();
  let points = $derived(samples.map((s) => ({ timestamp: s.timestamp_ms, value: s.cpu_percent })));
</script>
<section class="metric-card">
  <div class="metric-header"><span class="metric-label">CPU</span><span class="metric-value">{formatPercent(current?.cpu_percent)}</span></div>
  <MetricPlot {points} {end} color="#df7900" label="CPU usage, zero to 100 percent, over the last 60 seconds" />
  <div class="metric-footer"><span>Avg samples {formatPercent(average(points))}</span><span>Peak {formatPercent(peak(points))}</span></div>
</section>
<style>.metric-value { color: #c56b00; }</style>
