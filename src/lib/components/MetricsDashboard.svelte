<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import NetworkChart from "$lib/components/NetworkChart.svelte";
  import CpuChart from "$lib/components/CpuChart.svelte";
  import type { MetricsHistory, MetricsSnapshot, SpeedEntry } from "$lib/metrics";

  let networkHistory: SpeedEntry[] = [];
  let cpuHistory: number[] = [];
  let currentNetwork: SpeedEntry = { down: 0, up: 0 };
  let currentCpu = 0;
  let unlisten: (() => void) | null = null;

  onMount(async () => {
    const h = await invoke<MetricsHistory>("get_metrics_history");
    networkHistory = h.network;
    cpuHistory = h.cpu;
    if (networkHistory.length > 0) {
      currentNetwork = networkHistory[networkHistory.length - 1];
    }
    if (cpuHistory.length > 0) {
      currentCpu = cpuHistory[cpuHistory.length - 1];
    }

    unlisten = await listen<MetricsSnapshot>("metrics-update", (event) => {
      const p = event.payload;
      currentNetwork = p.network;
      currentCpu = p.cpu_percent;
      networkHistory = [...networkHistory.slice(-59), p.network];
      cpuHistory = [...cpuHistory.slice(-59), p.cpu_percent];
    });
  });

  onDestroy(() => {
    unlisten?.();
  });
</script>

<main>
  <NetworkChart history={networkHistory} current={currentNetwork} />
  <CpuChart history={cpuHistory} current={currentCpu} />
</main>

<style>
  :global(body) {
    margin: 0;
    background: #1a1a2e;
    color: #e0e0e0;
    font-family:
      -apple-system,
      BlinkMacSystemFont,
      "SF Pro Text",
      system-ui,
      sans-serif;
  }
  main {
    padding: 16px;
    user-select: none;
  }
</style>
