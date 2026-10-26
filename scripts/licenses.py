#!/usr/bin/env python3
"""Render/check offline notices without network access (Python 3.11+)."""
import argparse
import hashlib
import html
import json
from pathlib import Path
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validated_inventory():
    inventory = json.loads((ROOT / "licenses/inventory.json").read_text(encoding="utf-8"))
    for name, checksum in inventory["lockfiles"].items():
        if digest(ROOT / name) != checksum:
            raise ValueError(f"{name} changed: refresh and review the license inventory first")
    cargo = tomllib.loads((ROOT / "src-tauri/Cargo.lock").read_text())
    expected_cargo = {(p["name"], p["version"], p["source"]) for p in cargo["package"] if "source" in p}
    actual_cargo = {(p["name"], p["version"], p["source"]) for p in inventory["packages"] if p["ecosystem"] == "cargo"}
    npm = json.loads((ROOT / "package-lock.json").read_text())
    expected_npm = {(k.split("node_modules/")[-1], p["version"], p["resolved"], p["integrity"]) for k, p in npm["packages"].items() if k}
    actual_npm = {(p["name"], p["version"], p["source"], p["integrity"]) for p in inventory["packages"] if p["ecosystem"] == "npm"}
    if actual_cargo != expected_cargo or actual_npm != expected_npm:
        raise ValueError("License inventory does not cover the exact locked dependency set")
    checksums = {(p["name"], p["version"]): p.get("checksum") for p in cargo["package"]}
    documents = list(inventory["standard_terms"].values())
    inventory["native"] = json.loads((ROOT / "licenses/native.json").read_text(encoding="utf-8"))
    for component in inventory["native"]:
        if not any(p["name"] == component["via"]["name"] and p["version"] == component["via"]["version"] for p in inventory["packages"]):
            raise ValueError(f"Native component needs re-verification: {component['name']}")
        documents.extend(component["documents"])
    for package in inventory["packages"]:
        if not package["license"] or not package["documents"]:
            raise ValueError(f"Missing license evidence: {package['name']}")
        documents.extend(package["documents"])
        if "MPL-2.0" in package["license"]:
            archive = package["source_archive"]
            if digest(ROOT / archive["file"]) != archive["sha256"] or archive["sha256"] != checksums[(package["name"], package["version"])]:
                raise ValueError(f"MPL source archive does not match Cargo.lock: {package['name']}")
    for document in documents:
        if digest(ROOT / "licenses" / document["file"]) != document["sha256"]:
            raise ValueError(f"License text changed: {document['file']}")
    return inventory


def render(inventory):
    escape = html.escape
    version = json.loads((ROOT / "package.json").read_text())["version"]
    cargo = tomllib.loads((ROOT / "src-tauri/Cargo.toml").read_text())
    config = json.loads((ROOT / "src-tauri/tauri.conf.json").read_text())
    if version != cargo["package"]["version"] or version != config["version"]:
        raise ValueError("Application versions disagree")
    if cargo["package"].get("license") != "MIT" or config["bundle"].get("license") != "MIT":
        raise ValueError("Application license metadata must match LICENSE")
    parts = [f'''<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'">
<title>About netmon &amp; Licenses</title><style>
:root {{ color-scheme: light dark; font: 15px/1.55 system-ui, sans-serif; }}
body {{ max-width: 900px; margin: auto; padding: 24px; }}
h1 {{ font-size: 26px; margin-bottom: 0; }} h2 {{ font-size: 20px; margin-top: 32px; }}
h3 {{ font-size: 16px; }} nav {{ display: flex; flex-wrap: wrap; gap: 16px; margin: 24px 0; }}
a {{ color: LinkText; }} pre, code {{ white-space: pre-wrap; overflow-wrap: anywhere; font-size: 12px; }}
details {{ border-top: 1px solid GrayText; padding: 10px 0; }} summary {{ cursor: pointer; font-weight: 600; }}
.support {{ display: inline-block; padding: 10px 16px; border: 1px solid currentColor; border-radius: 8px; font-weight: 600; text-decoration: none; }}
.support:hover {{ text-decoration: underline; }} .support:focus-visible {{ outline: 2px solid currentColor; outline-offset: 4px; }}
.meta {{ font-size: 13px; }} .source {{ display: block; user-select: text; }}
@media (max-width: 420px) {{ body {{ padding: 14px; }} }}
@media print {{ details > * {{ display: block !important; }} }}
</style></head><body>
<h1>netmon</h1><p>Version {escape(version)} · Network, CPU, RAM &amp; GPU monitor</p>
<p>Copyright (c) 2026 Andreas Distel</p>
<p>netmon's original code and artwork are licensed under the MIT License.
Third-party components retain their own licenses and notices below.</p>
<p class="meta">Project: <code class="source">https://github.com/j3ffwin5tonone/netmon</code></p>
<p><a class="support" href="https://paypal.me/adSoftware1" rel="noreferrer">🍺 Buy the developer a beer</a></p>
<p class="meta">Optional tip via PayPal. Opens in your default browser.<br><code class="source">https://paypal.me/adSoftware1</code></p>
<nav aria-label="License sections"><a href="#netmon">netmon license</a><a href="#dependencies">Third-party notices</a><a href="#sources">MPL sources</a><a href="#terms">Standard license texts</a></nav>
<h2 id="netmon">netmon — MIT License</h2><pre>{escape((ROOT / 'LICENSE').read_text())}</pre>
<h2 id="dependencies">Third-party notices</h2>
<p>This inventory includes all locked Rust and npm packages, including build tools and optional platforms.
Inclusion does not mean that every package is shipped or linked on your platform.
Original notices are reproduced verbatim. Published author names are attribution metadata, not an assertion of copyright ownership.
Where upstream only declares a license, its declaration and the standard terms are provided separately.</p>
<p>For an OR expression, the permissive MIT option is used when available; otherwise Apache-2.0 when available, otherwise CC0-1.0 or Unlicense as offered, except where a selected option is explicitly stated below.
AND terms and additional notices apply together. Legacy slash-separated alternatives retain their upstream meaning.
No third-party code is relicensed under netmon's MIT license.</p>
''']
    for package in inventory["packages"]:
        parts.append(f'<details><summary>{escape(package["ecosystem"])} · {escape(package["name"])} {escape(package["version"])}</summary>')
        parts.append(f'<p class="meta">Declared license: {escape(package["license"])}<br>Exact source:<code class="source">{escape(package.get("download", package["source"]))}</code></p>')
        if "selected_license" in package:
            parts.append(f'<p>Selected license option: {escape(package["selected_license"])}</p>')
        for document in package["documents"]:
            text = (ROOT / "licenses" / document["file"]).read_text(encoding="utf-8")
            parts.append(f'<h3>{escape(document["name"])}</h3><pre>{escape(text)}</pre><p class="meta">Notice provenance:<code class="source">{escape(document["source"])}</code></p>')
        parts.append('</details>')
    parts.append('<h2>Native redistributables</h2>')
    for component in inventory["native"]:
        parts.append(f'<h3>{escape(component["name"])} {escape(component["version"])}</h3><p>License: {escape(component["license"])}. The loader binaries match the original Microsoft SDK package byte-for-byte for x64, x86 and ARM64.</p>')
        for document in component["documents"]:
            text = (ROOT / "licenses" / document["file"]).read_text(encoding="utf-8")
            parts.append(f'<details><summary>{escape(document["name"])}</summary><pre>{escape(text)}</pre><p class="meta"><code class="source">{escape(document["source"])}</code></p></details>')
    parts.append('''<h2 id="sources">MPL source availability</h2>
<p>The exact, unmodified upstream source archives listed below are included with this application in the <code>legal-sources</code> directory and in the netmon repository under <code>static/legal-sources</code>.
They retain their MPL-2.0 terms and notices. Checksums match Cargo.lock.
Sources are also available without charge at these version-specific URLs (select and copy to your browser).</p>''')
    for package in inventory["packages"]:
        if "source_archive" not in package:
            continue
        archive = package["source_archive"]
        parts.append(f'<p><strong>{escape(package["name"])} {escape(package["version"])}</strong><br>Included file: <code>{escape(archive["file"].removeprefix("static/"))}</code><br><code class="source">{escape(package["download"])}</code><br>SHA-256: <code>{archive["sha256"]}</code></p>')
    parts.append('<h2 id="terms">Standard license texts</h2><p>These standard texts supplement the package-specific notices above. Template placeholders in standard texts are not copyright attributions; the actual upstream notices remain authoritative.</p>')
    for name, document in inventory["standard_terms"].items():
        text = (ROOT / "licenses" / document["file"]).read_text(encoding="utf-8")
        parts.append(f'<details><summary>{escape(name)}</summary><pre>{escape(text)}</pre><p class="meta">Source:<code class="source">{escape(document["source"])}</code></p></details>')
    parts.append('''<h2>System components</h2><p>WKWebView and Apple frameworks, WebView2, and Linux system libraries are governed by their respective suppliers' terms.
This inventory covers the locked application dependencies; it does not replace the notices of separately installed operating-system components.
For Linux distributions that bundle additional native libraries, include the matching native-library notices and any required source/relinking materials before distributing that bundle.</p>
<p><a href="#netmon">Back to netmon license</a></p></body></html>''')
    return '\n'.join(parts) + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='Fail if notices or inventory are stale')
    args = parser.parse_args()
    try:
        inventory = validated_inventory()
        output = render(inventory)
        path = ROOT / 'static/licenses.html'
        if args.check:
            if not path.exists() or path.read_text(encoding='utf-8') != output:
                raise ValueError('static/licenses.html is stale; run python3 scripts/licenses.py')
        else:
            path.write_text(output, encoding='utf-8')
        print(f'Licenses verified: {len(inventory["packages"])} packages; notices and MPL sources complete for the locked inventory.')
    except (ValueError, KeyError, OSError) as error:
        print(f'License check failed: {error}', file=sys.stderr)
        raise SystemExit(1)


if __name__ == '__main__':
    main()
