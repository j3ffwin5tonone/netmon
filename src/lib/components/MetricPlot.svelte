<script lang="ts">
  import { chartPath, CHART_HEIGHT, CHART_WIDTH, type ChartPoint } from "$lib/chart";
  let { points, secondary = [], end, scale = 100, color, secondaryColor = "#34c759", label }: {
    points: ChartPoint[];
    secondary?: ChartPoint[];
    end: number;
    scale?: number;
    color: string;
    secondaryColor?: string;
    label: string;
  } = $props();
  let path = $derived(chartPath(points, end, scale));
  let secondPath = $derived(chartPath(secondary, end, scale));
</script>

<svg viewBox="0 0 {CHART_WIDTH} {CHART_HEIGHT}" preserveAspectRatio="none" role="img" aria-label={label}>
  <line x1="0" y1={CHART_HEIGHT * 0.25} x2={CHART_WIDTH} y2={CHART_HEIGHT * 0.25} />
  <line x1="0" y1={CHART_HEIGHT * 0.5} x2={CHART_WIDTH} y2={CHART_HEIGHT * 0.5} />
  <line x1="0" y1={CHART_HEIGHT * 0.75} x2={CHART_WIDTH} y2={CHART_HEIGHT * 0.75} />
  {#if path}<path d={path} stroke={color} />{/if}
  {#if secondPath}<path d={secondPath} stroke={secondaryColor} />{/if}
</svg>
<div class="axis"><span>−60 s</span><span>now</span></div>

<style>
  svg { width: 100%; flex: 1; min-height: 80px; display: block; }
  line { stroke: #ededf0; stroke-width: 0.5; }
  path { fill: none; stroke-width: 1.5; stroke-linecap: round; stroke-linejoin: round; }
  .axis { display: flex; justify-content: space-between; font-size: 10px; color: #86868b; margin-top: 3px; }
</style>
