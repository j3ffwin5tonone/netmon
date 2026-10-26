# Third-party notices and distribution

The MIT license in `LICENSE` applies to netmon's original code and artwork.
Dependency code, data, and native redistributables retain their own licenses.
The project owner confirmed the copyright name and rights to the original code
and app icon on 2026-10-26. Unused starter logos were removed; the favicon uses
netmon's own icon.

## Included materials

- `licenses/inventory.json`: exact versions, source references, declared license
  expressions, provenance and SHA-256 hashes of each notice.
- `licenses/texts/`: original upstream notices and supplemental standard texts.
- `static/licenses.html`: offline, script-free About and license page, embedded
  in the app and also installed as a standalone HTML resource.
- `static/legal-sources/`: exact, unmodified source archives for the packages
  declaring MPL-2.0, including optional alternatives for completeness. Their
  SHA-256 hashes are checked against Cargo.lock. MPL terms continue to apply to
  covered code, including any future modifications.

The inventory intentionally covers all locked npm and Cargo packages, including
build tools and optional platforms. It is a conservative notice inventory, not
an assertion that every listed package is linked into every installer. Build
tools' licenses do not automatically apply to their output.

Where a dependency offers alternatives, netmon uses MIT where offered, otherwise
Apache-2.0 where offered, otherwise CC0-1.0 or Unlicense where offered. Obligations
joined by AND and additional notices must all be retained.
For `tao-macros`, the Apache-2.0 alternative is explicitly selected and its own
upstream notices are retained; the selected alternative is recorded in the inventory.
Original license texts are included even when an alternative is not selected. Authorship metadata is
preserved as metadata; copyright holders or years are never invented.

Some published crates omit their workspace license files. These are recovered
from the commit recorded in `.cargo_vcs_info.json`. A small number of companion
packages use the corresponding parent package's notice, with that provenance
recorded explicitly. Some upstream packages only declare MIT/MPL rather than
providing a full text; the declaration, any upstream notices and the corresponding
SPDX standard terms are retained. Template placeholders in standard terms are
not actual copyright attributions.

## Platform scope

The current release workflows build macOS and Windows installers. Each bundle
must contain `LICENSE`, `licenses.html`, and the complete `legal-sources/`
directory. The same notices are embedded in the frontend, reachable without
network access through **About & Licenses**.

macOS uses Apple's installed frameworks and WKWebView rather than redistributing
them. Preserve the objc2 upstream licensing note about SDK-derived bindings;
building and distributing macOS binaries also remains subject to Apple's SDK
terms. This project does not purport to relicense Apple components.

Windows uses the system Evergreen WebView2 Runtime (downloaded by the installer
if absent), not a bundled fixed-version Chromium runtime. The Microsoft loader
distributed with the Rust bindings has separate Microsoft terms, in addition to
the bindings' MIT license.
`licenses/native.json` records the original SDK 1.0.3800.47 package, its BSD-3-Clause
license and NOTICE, and matching SHA-256 hashes for all three loader architectures.
The collector identifies the SDK from the binding generator at the crate's exact
published source commit and verifies the loader bytes against Microsoft's
versioned NuGet SDK.

There is no Linux installer release workflow. Before adding one, inventory the
native libraries actually copied into the artifact, such as GTK, GLib,
WebKitGTK, libappindicator and their dependencies. Their LGPL/GPL/other native
terms are not replaced by the Rust wrapper licenses. Include required notices,
corresponding sources and relinking/installation materials for the chosen bundle.
Do not describe a future Linux AppImage as license-verified based only on the
Cargo/npm check.

## Maintenance

1. Install the exact dependencies with `npm ci --ignore-scripts` and
   `cargo fetch --locked --manifest-path src-tauri/Cargo.toml`.
2. After dependency updates, run `python3 scripts/collect-licenses.py`. It reads
   Cargo metadata and installed npm packages, retrieves versioned upstream
   notices when necessary and records provenance. Network access is required
   only for collection, never for normal builds or viewing the notices.
3. Review changes to license expressions, upstream notices, companion-package
   substitutions, native redistributables and source availability. A successful
   collection is evidence of completeness of the inventory, not a substitute
   for this review. Keep the upstream license text verbatim.
4. Run `npm run licenses:generate` and `npm run licenses:check`, then the normal
   frontend/Rust checks and a packaged-app smoke test. Commit the inventory,
   texts, source archives and generated page together with the lockfiles.

The ordinary build and CI perform the check without network access. A changed
lockfile, missing package, altered notice, stale HTML or mismatched MPL archive
fails the build instead of silently shipping outdated notices.
