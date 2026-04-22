<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import NetworkChart from "$lib/components/NetworkChart.svelte";
  import CpuChart from "$lib/components/CpuChart.svelte";
  import GpuChart from "$lib/components/GpuChart.svelte";
  import MemoryChart from "$lib/components/MemoryChart.svelte";
  import type { MemoryEntry, MetricsHistory, MetricsSnapshot, SpeedEntry } from "$lib/metrics";

  let networkHistory: SpeedEntry[] = [];
  let cpuHistory: number[] = [];
  let memoryHistory: number[] = [];
  let gpuHistory: number[] = [];
  let gpuSupported = false;
  let currentNetwork: SpeedEntry = { down: 0, up: 0 };
  let currentCpu = 0;
  let currentMemoryPercent = 0;
  let currentGpu = 0;
  let currentMemory: MemoryEntry = { used_bytes: 0, total_bytes: 0 };
  let unlisten: (() => void) | null = null;

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
      const p = event.payload;
      currentNetwork = p.network;
      currentCpu = p.cpu_percent;
      currentMemoryPercent = p.memory_percent;
      currentMemory = p.memory;
      currentGpu = p.gpu_percent;
      gpuSupported = p.gpu_supported;
      networkHistory = [...networkHistory.slice(-59), p.network];
      cpuHistory = [...cpuHistory.slice(-59), p.cpu_percent];
      memoryHistory = [...memoryHistory.slice(-59), p.memory_percent];
      gpuHistory = [...gpuHistory.slice(-59), p.gpu_percent];
    });
  });

  onDestroy(() => {
    unlisten?.();
  });
</script>

<main>
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
