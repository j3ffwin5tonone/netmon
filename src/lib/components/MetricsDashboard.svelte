<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import NetworkChart from "$lib/components/NetworkChart.svelte";
  import CpuChart from "$lib/components/CpuChart.svelte";
  import GpuChart from "$lib/components/GpuChart.svelte";
  import MemoryChart from "$lib/components/MemoryChart.svelte";
  import { createHistoryController, createVisibilityClock, latestFresh, type DashboardState } from "$lib/history";
  import { createListenerConnection } from "$lib/connection";
  import type { MetricsHistory, MetricsSnapshot } from "$lib/metrics";

  let view = $state<DashboardState>({ samples: [], networkSelection: null, now: Date.now(), error: null });
  let selecting = $state(false);
  let selectionError = $state<string | null>(null);
  let connectionError = $state<string | null>(null);
  let retry = $state<() => void>(() => {});
  const controller = createHistoryController(
    () => invoke<MetricsHistory>("get_metrics_history"),
    (state) => { view = state; },
  );
  let latest = $derived(view.samples.at(-1));
  let current = $derived(latestFresh(view.samples, view.now));
  let interfaces = $derived(latest?.network_interfaces ?? []);
  let missingSelection = $derived(view.networkSelection !== null && !interfaces.includes(view.networkSelection));

  async function selectInterface(event: Event) {
    const name = (event.currentTarget as HTMLSelectElement).value || null;
    selecting = true;
    selectionError = null;
    try {
      await invoke("set_network_interface", { name });
      await controller.synchronize();
    } catch (error) {
      selectionError = `Could not select interface: ${String(error)}`;
    } finally {
      selecting = false;
    }
  }

  onMount(() => {
    const visibleClock = createVisibilityClock(() => controller.advanceClock());
    const connection = createListenerConnection([
      // Subscribe before fetching, preserving events arriving while invoke is pending.
      () => listen<MetricsSnapshot>("metrics-update", (event) => {
        if (document.visibilityState !== "hidden") controller.receive(event.payload);
      }),
      () => listen("dashboard-shown", () => refresh()),
    ], (error) => { connectionError = error; });
    const synchronize = async () => {
      if (await connection.connect()) await controller.synchronize();
    };
    const refresh = () => {
      const visible = document.visibilityState !== "hidden";
      visibleClock.setVisible(visible);
      if (visible) void synchronize();
    };
    retry = () => { selectionError = null; void synchronize(); };
    const visibilityChanged = () => refresh();
    visibleClock.setVisible(document.visibilityState !== "hidden");
    window.addEventListener("focus", refresh);
    document.addEventListener("visibilitychange", visibilityChanged);
    void synchronize();
    return () => {
      visibleClock.dispose();
      controller.dispose();
      connection.dispose();
      window.removeEventListener("focus", refresh);
      document.removeEventListener("visibilitychange", visibilityChanged);
    };
  });
</script>

<main>
  <header>
    <span class="app-name">netmon</span>
    <span class="app-sub">Last 60 s · updated every second</span>
  </header>
  <div class="network-selection">
    <label for="network-interface">Network interface</label>
    <select id="network-interface" value={view.networkSelection ?? ""} onchange={selectInterface} disabled={selecting}>
      <option value="">Auto · busiest interface</option>
      {#if missingSelection}<option value={view.networkSelection ?? ""}>{view.networkSelection} (unavailable)</option>{/if}
      {#each interfaces as name}<option value={name}>{name}</option>{/each}
    </select>
    <span class="selection-hint">One interface at a time; VPN traffic is not counted twice.</span>
  </div>
  {#if view.error || selectionError || connectionError}
    <div class="error" role="status">
      <span>{selectionError ?? connectionError ?? view.error}</span>
      <button onclick={retry}>Retry</button>
    </div>
  {/if}
  <div class="grid">
    <NetworkChart samples={view.samples} {current} end={view.now} />
    <CpuChart samples={view.samples} {current} end={view.now} />
    <GpuChart samples={view.samples} {current} end={view.now} />
    <MemoryChart samples={view.samples} {current} end={view.now} />
  </div>
</main>

<style>
  :global(body) { margin: 0; background: #f5f5f7; color: #1d1d1f; font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif; }
  main { box-sizing: border-box; min-height: 100vh; padding: 16px; user-select: none; display: flex; flex-direction: column; gap: 12px; }
  header { display: flex; align-items: baseline; justify-content: space-between; gap: 8px; flex-wrap: wrap; padding: 0 2px; }
  .app-name { font-size: 15px; font-weight: 600; }
  .app-sub { font-size: 12px; color: #6e6e73; }
  .network-selection { display: flex; align-items: center; flex-wrap: wrap; gap: 6px 10px; font-size: 12px; }
  .network-selection label { font-weight: 600; }
  select { max-width: 100%; min-height: 28px; color: #1d1d1f; background: #fff; border: 1px solid #d2d2d7; border-radius: 5px; padding: 3px 8px; }
  .selection-hint { color: #6e6e73; }
  .grid { flex: 1; display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
  .error { display: flex; align-items: center; gap: 8px; font-size: 12px; padding: 10px; background: #fff2de; border-radius: 6px; }
  .error button { flex: none; }
  :global(.metric-card) { min-width: 0; background: #fff; border-radius: 10px; border: 0.5px solid #e2e2e6; padding: 14px 16px 12px; display: flex; flex-direction: column; min-height: 185px; }
  :global(.metric-header) { display: flex; justify-content: space-between; align-items: baseline; flex-wrap: wrap; gap: 6px; margin-bottom: 10px; }
  :global(.metric-label) { font-size: 12px; font-weight: 600; color: #6e6e73; text-transform: uppercase; letter-spacing: 0.05em; }
  :global(.metric-value) { font-family: "SF Mono", ui-monospace, Menlo, monospace; font-size: 15px; font-weight: 600; font-variant-numeric: tabular-nums; }
  :global(.metric-footer) { display: flex; justify-content: space-between; gap: 8px; flex-wrap: wrap; margin-top: 10px; font-size: 11px; color: #6e6e73; }
  @media (max-width: 660px) { .grid { grid-template-columns: minmax(0, 1fr); } }
  @media (max-width: 420px) { main { padding: 10px; } :global(.metric-card) { padding: 12px; } }
</style>
