<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import NetworkChart from "$lib/components/NetworkChart.svelte";
  import CpuChart from "$lib/components/CpuChart.svelte";
  import GpuChart from "$lib/components/GpuChart.svelte";
  import MemoryChart from "$lib/components/MemoryChart.svelte";
  import type { MemoryEntry, MetricsHistory, MetricsSnapshot, SpeedEntry } from "$lib/metrics";

  const HISTORY_LEN = 60;

  let networkHistory = $state<SpeedEntry[]>([]);
  let cpuHistory = $state<number[]>([]);
  let memoryHistory = $state<number[]>([]);
  let gpuHistory = $state<number[]>([]);
  let gpuSupported = $state(false);
  let currentNetwork = $state<SpeedEntry>({ down: 0, up: 0 });
  let currentCpu = $state(0);
  let currentMemoryPercent = $state(0);
  let currentGpu = $state(0);
  let currentMemory = $state<MemoryEntry>({ used_bytes: 0, total_bytes: 0 });
  let unlisten: (() => void) | null = null;

  function pushRing<T>(history: T[], value: T) {
    if (history.length >= HISTORY_LEN) {
      history.shift();
    }
    history.push(value);
  }

  onMount(async () => {
    const h = await invoke<MetricsHistory>("get_metrics_history");
    networkHistory = h.network;
    cpuHistory = h.cpu;
    memoryHistory = h.memory;
    gpuHistory = h.gpu;
    gpuSupported = h.gpu_supported;
    if (networkHistory.length > 0) {
      currentNetwork = networkHistory[networkHistory.length - 1];
    }
    if (cpuHistory.length > 0) {
      currentCpu = cpuHistory[cpuHistory.length - 1];
    }
    if (memoryHistory.length > 0) {
      currentMemoryPercent = memoryHistory[memoryHistory.length - 1];
    }
    if (gpuHistory.length > 0) {
      currentGpu = gpuHistory[gpuHistory.length - 1];
    }

    unlisten = await listen<MetricsSnapshot>("metrics-update", (event) => {
      if (document.visibilityState === "hidden") {
        return;
      }
      const p = event.payload;
      currentNetwork = p.network;
      currentCpu = p.cpu_percent;
      currentMemoryPercent = p.memory_percent;
      currentMemory = p.memory;
      currentGpu = p.gpu_percent;
      gpuSupported = p.gpu_supported;
      pushRing(networkHistory, p.network);
      pushRing(cpuHistory, p.cpu_percent);
      pushRing(memoryHistory, p.memory_percent);
      pushRing(gpuHistory, p.gpu_percent);
    });
  });

  onDestroy(() => {
    unlisten?.();
  });
</script>

<main>
  <header>
    <span class="app-name">netmon</span>
    <span class="app-sub">Last 60 s · updated every second</span>
  </header>
  <div class="grid">
    <NetworkChart history={networkHistory} current={currentNetwork} />
    <CpuChart history={cpuHistory} current={currentCpu} />
    <GpuChart
      history={gpuHistory}
      current={currentGpu}
      supported={gpuSupported}
    />
    <MemoryChart
      history={memoryHistory}
      current={currentMemoryPercent}
      memory={currentMemory}
    />
  </div>
</main>

<style>
  :global(body) {
    margin: 0;
    background: #f5f5f7;
    color: #1d1d1f;
    font-family:
      -apple-system,
      BlinkMacSystemFont,
      "SF Pro Text",
      system-ui,
      sans-serif;
  }
  main {
    box-sizing: border-box;
    min-height: 100vh;
    padding: 16px;
    user-select: none;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    padding: 0 2px;
  }
  .app-name {
    font-size: 15px;
    font-weight: 600;
  }
  .app-sub {
    font-size: 12px;
    color: #86868b;
  }
  .grid {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
    gap: 12px;
  }
</style>
