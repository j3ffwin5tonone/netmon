<script lang="ts">
  import type { MetricsSnapshot } from "$lib/metrics";
  import { formatSpeed, peak, validValues } from "$lib/chart";
  import MetricPlot from "./MetricPlot.svelte";
  let { samples, current, end }: { samples: MetricsSnapshot[]; current: MetricsSnapshot | null; end: number } = $props();
  let down = $derived(samples.map((s) => ({ timestamp: s.timestamp_ms, value: s.network_interface ? s.network.down : null, source: s.network_interface })));
  let up = $derived(samples.map((s) => ({ timestamp: s.timestamp_ms, value: s.network_interface ? s.network.up : null, source: s.network_interface })));
  let scale = $derived(Math.max(0.1, ...validValues(down), ...validValues(up)) * 1.15);
</script>
<section class="metric-card">
  <div class="metric-header"><span class="metric-label">Network</span><span class="interface">{current?.network_interface ?? "No active interface"}</span></div>
  <div class="rates metric-value"><span class="down">↓ {formatSpeed(current?.network_interface ? current.network.down : null)}</span><span class="up">↑ {formatSpeed(current?.network_interface ? current.network.up : null)}</span></div>
  <MetricPlot points={down} secondary={up} {end} {scale} color="#007aff" secondaryColor="#248a3d" label="Download and upload in MiB per second over the last 60 seconds; source changes are gaps" />
  <div class="metric-footer"><span><span class="down">Download</span> · <span class="up">Upload</span></span><span>Peak ↓ {formatSpeed(peak(down))}</span></div>
</section>
<style>
 .interface { color: #6e6e73; font-size: 12px; overflow-wrap: anywhere; }
 .rates { display: flex; flex-wrap: wrap; gap: 4px 14px; margin: -4px 0 8px; font-size: 14px; }
 .down { color: #007aff; } .up { color: #248a3d; }
</style>
