<div align="center">

<img src="src-tauri/icons/128x128.png" alt="netmon icon" width="96">

# netmon

**A lightweight tray app for real-time network, CPU, RAM & GPU monitoring.**

[![Release](https://img.shields.io/github/v/release/j3ffwin5tonone/netmon?label=release)](https://github.com/j3ffwin5tonone/netmon/releases)
[![macOS](https://img.shields.io/badge/macOS-Apple%20Silicon%20%7C%20Intel-black?logo=apple)](https://github.com/j3ffwin5tonone/netmon/releases)
[![Windows](https://img.shields.io/badge/Windows-x64-0078D6?logo=windows&logoColor=white)](https://github.com/j3ffwin5tonone/netmon/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](package.json)

*Live metrics in the tray — click for a full dashboard with 60-second history.*

<!-- Screenshot: replace with a real capture of the dashboard window -->
<img src="docs/screenshot-dashboard.png" alt="netmon dashboard" width="720">

</div>

---

## Highlights

- **Tray first** — download/upload, CPU, RAM and GPU at a glance:
  `↓ 12.3 MB/s ↑ 1.2 MB/s · CPU 23% · RAM 67% · GPU 15%`
  - **macOS:** live text in the menu bar
  - **Windows:** same metrics in the tray icon tooltip (Windows does not support tray title text)
- **One-click dashboard** — four clean cards with 60-second history charts, updated every second
- **GPU metrics** — Apple Silicon via IOReport (**no `sudo`**); Windows via PDH (same counters as Task Manager, no admin)
- **Tiny footprint** — native Rust backend (Tauri 2), no Electron; system WebView (WKWebView / WebView2)
- **Free & open source** — MIT licensed

## Install

### macOS

Grab the latest `.dmg` from the **[Releases page](https://github.com/j3ffwin5tonone/netmon/releases)**, drag `netmon.app` to Applications, launch — the icon appears in your menu bar.

### Windows

Grab the latest NSIS installer (`netmon_*_x64-setup.exe`) or MSI from the **[Releases page](https://github.com/j3ffwin5tonone/netmon/releases)** (or from the **Windows Build** GitHub Action artifacts). Install and launch — netmon appears in the system tray.

The installer uses the **system WebView2** runtime (downloaded at install time if missing). That keeps the app small and avoids bundling a private browser engine — no extra runtime cost versus a typical Tauri/Windows app.

> **Note:** On Intel Macs the GPU card shows "n/a". On Apple Silicon and Windows, GPU utilization is shown when the platform sampler initializes successfully.

## Usage

| Action | Result |
|--------|--------|
| **Left-click** tray icon | Open the dashboard window |
| **Right-click** tray icon | Menu → Quit |
| **Hover** tray icon (Windows) | Show live metrics in the tooltip |

## Metrics

| Metric | Source | Notes |
|--------|--------|-------|
| Network | [`sysinfo`](https://crates.io/crates/sysinfo) | Per-interface byte delta per second (loopback excluded) |
| CPU | `sysinfo` | Global usage across all cores |
| RAM | `sysinfo` | Used vs. total memory |
| GPU | [`macmon`](https://github.com/vladkens/macmon) / PDH | Apple Silicon: IOReport; Windows: `\GPU Engine(*)\Utilization Percentage` (Task Manager) |

## Building from source

Requirements: [Node.js](https://nodejs.org/) 18+, [Rust](https://rustup.rs) (stable).

- **macOS:** Xcode Command Line Tools
- **Windows:** [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (usually preinstalled on Windows 10/11), plus Visual Studio C++ build tools for Rust

```bash
git clone https://github.com/j3ffwin5tonone/netmon.git
cd netmon
npm install
npm run tauri dev      # development
npm run tauri build    # release build
```

Artifacts land under `src-tauri/target/release/bundle/`:

- macOS → `macos/` / `dmg/`
- Windows → `msi/` and `nsis/`

CI builds Windows installers on every push via [`.github/workflows/windows.yml`](.github/workflows/windows.yml).

## Architecture

```
netmon/
├── src/                 # SvelteKit frontend (dashboard, charts)
│   └── lib/components/  # NetworkChart, CpuChart, GpuChart, MemoryChart
└── src-tauri/           # Tauri 2 / Rust backend
    └── src/metrics/     # cpu, memory, network, gpu collectors
```

The backend collects a `MetricsSnapshot` every second and emits it to the frontend via the `metrics-update` event. History is a 60-entry ring buffer; GPU sampling runs on a dedicated thread (Apple Silicon and Windows).

## Acknowledgments

[Tauri](https://tauri.app) · [SvelteKit](https://kit.svelte.dev) · [sysinfo](https://crates.io/crates/sysinfo) · [macmon](https://github.com/vladkens/macmon) · [Windows PDH](https://learn.microsoft.com/en-us/windows/win32/perfctrs/about-performance-counters)

## License

MIT
