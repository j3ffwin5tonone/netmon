<script lang="ts">
  import type { MetricsSnapshot } from "$lib/metrics";
  import { average, formatPercent, formatGiB } from "$lib/chart";
  import MetricPlot from "./MetricPlot.svelte";
  let { samples, current, end }: { samples: MetricsSnapshot[]; current: MetricsSnapshot | null; end: number } = $props();
  let points = $derived(samples.map((s) => ({ timestamp: s.timestamp_ms, value: s.memory_percent })));
</script>
<section class="metric-card">
  <div class="metric-header">
    <span class="metric-label">Memory</span>
    <span class="metric-value">{formatPercent(current?.memory_percent)}</span>
  </div>
  <div class="memory-size">{#if current && current.memory.total_bytes > 0}{formatGiB(current.memory.used_bytes)} / {formatGiB(current.memory.total_bytes)}{:else}—{/if}</div>
  <MetricPlot {points} {end} color="#007aff" label="Used RAM, zero to 100 percent, over the last 60 seconds" />
  <div class="metric-footer"><span>Avg samples {formatPercent(average(points))}</span><span>Used physical RAM</span></div>
</section>
<style>.metric-value { color: #007aff; } .memory-size { font-size: 12px; color: #6e6e73; margin: -4px 0 6px; font-variant-numeric: tabular-nums; }</style>
