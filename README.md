# netmon

**netmon** is a lightweight macOS menu bar app that shows network, CPU, RAM, and (on Apple Silicon) GPU utilization in real time. Click the tray icon to open a dashboard with 60-second history charts.

## Features

- **Menu bar** — live values in the tray title (download/upload, CPU, RAM, optional GPU)
- **Network** — download and upload speed in MB/s or KB/s
- **CPU** — overall utilization across all cores
- **Memory** — used percentage and absolute bytes
- **GPU** — graphics core utilization on Apple Silicon (IOReport, no `sudo`)
- **Dashboard** — SVG charts with a 60-second history, updated every second

## Requirements

| Tool | Version |
|------|---------|
| [Node.js](https://nodejs.org/) | 18+ recommended |
| [Rust](https://www.rust-lang.org/tools/install) | stable (via `rustup`) |
| macOS | primary target; GPU metrics only on **Apple Silicon** (`aarch64`) |

For development:

- Xcode Command Line Tools (`xcode-select --install`)
- Optional: [VS Code](https://code.visualstudio.com/) with [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode), [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode), and [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Installation & development

```bash
git clone https://github.com/j3ffwin5tonone/netmon.git
cd netmon
npm install
npm run tauri dev
```

The first run compiles all Rust backend dependencies; this may take a few minutes.

## Release build

```bash
npm run tauri build
```

The `.app` bundle is written to `src-tauri/target/release/bundle/macos/`.

## Usage

1. After launch, a tray icon appears in the menu bar.
2. The title shows current metrics, e.g.  
   `↓ 12.3 MB/s ↑ 1.2 MB/s · CPU 23% · RAM 67% · GPU 15%`
3. **Left-click** the icon to open the dashboard window.
4. **Right-click** (or use the tray menu) → **Quit** to exit.

The main window starts hidden (`visible: false` in the Tauri config) and is opened only from the tray.

## Metrics

| Metric | Source | Notes |
|--------|--------|-------|
| Network | [`sysinfo`](https://crates.io/crates/sysinfo) | Per-interface byte delta per second |
| CPU | `sysinfo` | Global CPU usage |
| RAM | `sysinfo` | Used vs. total memory |
| GPU | [`macmon`](https://crates.io/crates/macmon) | `aarch64-apple-darwin` only; usage relative to max GPU frequency (IOReport) |

On Intel Macs and non-macOS builds, `gpu_supported` is false; the dashboard shows “GPU — n/v”.

## Architecture

```
netmon/
├── src/                    # SvelteKit frontend (dashboard, charts)
│   └── lib/
│       ├── metrics.ts      # TypeScript types (mirror of Rust backend)
│       └── components/     # NetworkChart, CpuChart, GpuChart, MemoryChart
└── src-tauri/              # Tauri 2 / Rust backend
    └── src/
        ├── lib.rs          # tray, event loop, Tauri commands
        └── metrics/        # cpu, memory, network, gpu
```

- The backend collects a [`MetricsSnapshot`](src-tauri/src/metrics/mod.rs) every second and emits it to the frontend via the Tauri `metrics-update` event.
- GPU sampling runs on a dedicated thread (`macmon::Sampler` is not `Send`); the UI communicates with it over a channel.
- History: ring buffer of 60 entries (`HISTORY_LEN`).

## Scripts

| Command | Description |
|---------|-------------|
| `npm run dev` | Vite frontend only (port 1420) |
| `npm run tauri dev` | Run the app in development mode |
| `npm run tauri build` | Production build |
| `npm run check` | Svelte / TypeScript check |

## License

MIT — see [`package.json`](package.json).

## Acknowledgments

- [Tauri](https://tauri.app/) — desktop shell
- [SvelteKit](https://kit.svelte.dev/) — UI
- [sysinfo](https://crates.io/crates/sysinfo) — system metrics
- [macmon](https://github.com/vladkens/macmon) — Apple Silicon GPU / IOReport (vladkens)
