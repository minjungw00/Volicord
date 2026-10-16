# Viewer architecture comparison support

Disposable, read-only Rust-rendered and Vue component readers over the **same
retained authored controls**. These scripts neither generate Volicord explanations
nor change Product routing, storage, installation, authority or Source retention.
There is no gate/Final/V11 invocation. The reviewed conclusion is in
[report.md](report.md).

No campaign prose, copied repository, generated assets, screenshots, measurements,
runtime or executable belongs here. Keep them under ignored `rebuild/.local/`.
The existing `reader_document_review.cjs`, seed support and retained review package
remain the source of the independent expectations; this is a bounded comparison,
not a replacement evaluation system.

## Inputs and failure behavior

`prepare.py --retained DIR --output NEW_IGNORED_DIR` consumes the retained
`review-input-final.json`, `maintained-review/fixtures.json`,
`review/identities.json` and `browser-final/{b,click}-status.stdout` layout.
It verifies every original asset named by that review identity, full file hashes,
the authored units against public readback, output fingerprints, citation claim
hashes, exact admitted Source ranges and omission counts. Missing originals or
mismatched bytes fail; there is no synthetic substitute or fresh provider call.
The output directory must not exist. Output `lineage.json` binds input identities
and the shared `data.json` digest.

Historical Work identities remain distinct from fresh control identities. A
captured `current` explanation is a **historical readback**, not a new freshness
claim. Preserve the actual missing-before-state and no-patch investigation.
The complete selected Work readback remains inspectable in each candidate's audit.
The older declared campaign ZIP hash is not independently reverified by these
scripts; verification of retained members is a narrower claim.

## Build and inspect

Prerequisites: Linux, Python 3, Rust compiler/linker, Node/npm **for developer Vue
asset compilation only**, the existing Playwright module, Chromium and a Korean
font configuration. `package-lock.json` pins npm tarball integrity; lifecycle
scripts are disabled. Build inputs and outputs are private local material.

From the repository root, select a new output directory:

```sh
viewer_build="$PWD/rebuild/.local/viewer-architecture/reproduced"
rebuild/scripts/validate focused viewer-architecture-prepare -- python3 -B \
  rebuild/validation/viewer-architecture/prepare.py \
  --retained rebuild/.local/reader-documents --output "$viewer_build"
cp rebuild/validation/viewer-architecture/package.json "$viewer_build/"
cp rebuild/validation/viewer-architecture/package-lock.json "$viewer_build/"
cp rebuild/validation/viewer-architecture/build.mjs "$viewer_build/"
```

In that build directory, run `npm ci --ignore-scripts`, then `node build.mjs`.
For the recorded environment, `--offline --cache
/tmp/volicord-architecture-npm-cache` also works after the initial download.
The production bundle is minified, tree-shaken, runtime-only Vue with development
features disabled; no runtime template compiler, CDN or Node server is used.

```sh
rustc --edition=2021 --cfg vue -O reader.rs -o vue-reader
rustc --edition=2021 --test reader.rs -o renderer-tests
VIEWER_SECURITY_HTML=security.html ./renderer-tests
cp app.js app.first.js
node build.mjs
cmp app.js app.first.js
```

`rust-reader` was compiled by `prepare.py` before replacing `app.js` with the Vue
bundle. Both executables embed their own assets. Run either executable without
arguments and open the printed loopback URL; stop it with Ctrl-C. No process is
installed or started automatically. Open `b-offline.html` or `click-offline.html`
directly for the shared, script-free Rust export. The Vue architecture deliberately
keeps this separate Rust export responsibility.

Read the same tasks in both live candidates:

1. From the Work list, locate Work B. Explain the absent/unavailable/failed/stale
   next-action match at `archive/cli.rs:1423–1439`; distinguish the three detailed
   limitations from the separate 32-area projection bound. Read the retained
   aggregate exit 101 and later recheck limitation. Follow the mechanism citation
   and return to its exact paragraph and scroll position.
2. Locate Click. Explain when relative lazy opening resolves the CWD and when an
   existing stream is reused. Compare the preserved relative lazy/eager and
   absolute lazy observations. Preserve “no patch”, historical test counts and
   Linux/Windows/symlink limitations; return to the originating explanation.

The green line backgrounds identify the **entire admitted citation range**. They
do not invent a finer line-to-claim mapping or a change diff.

## Browser and resource observations

Copy a private browser configuration to a new ignored path. It contains absolute
`build`, `retained`, `output`, `playwright`, `chromium`, `fontconfig`,
`security_html` paths, `repeats` (recorded: 7), and optional `baseline_bin`.
The output directory must be new. The existing recorded configuration is
`rebuild/.local/viewer-architecture/browser-full-3.json`.

```sh
rebuild/scripts/validate focused viewer-architecture-browser -- node \
  rebuild/validation/viewer-architecture/browser.cjs NEW_CONFIG
```

Loopback listeners, Chromium and `/proc` observations may require the local
sandbox exception. The driver records support HEAD, driver/config/build/binary
hashes, environment, full sample rows, requests, screenshots and process cleanup.
It independently compares the retained authored units and public readback, every
rendered citation's locator/code, complete diagnostics, keyboard focus and scroll
return. It checks 1440/768/390/320 CSS px, original/hostile long text, unknown Work,
Source failure, Vue data-fetch failure, actual network requests and script-free
offline citation return. `repeats: 0` runs the same controls without timings.

Seven new browser processes per Work/candidate provide cold and warm pairs. Cold
means first navigation in a fresh Chromium process/context, **not cold disk or OS
cache**. Warm means another navigation in that process after the intervening task.
Candidates are interleaved; production baselines run separately on the same
machine. No CPU/network throttling is applied. Timing includes Playwright dispatch,
readiness checks and two animation frames; interaction timings have a substantial
frame/automation floor. CDP script/task/layout and heap counters are observations,
not isolated algorithmic timings. Browser process RSS/PSS is sampled after the task;
it is not peak memory. Historical full snapshots are counted separately from live
requests. Missing instrumentation must remain explicit.

`resources.py` accepts `--build`, `--product-bin`, `--runtime`, `--output`,
`--click-license`, optional `--retained` and `--strace` paths. Run it through the
same focused wrapper. It records seven server startups, Linux child CPU lifetime,
RSS, exact raw/gzip bytes/hashes, private executable packages, current Product
sizes and optional CLI/MCP process/network traces. Gzip numbers are offline
compression potential; these comparison servers actually send uncompressed HTTP.
Prototype server CPU excludes the Product projection/SQLite pipeline.

## Node-free installation controls

`node_free.sh source` is for a disposable Debian container with `/inputs` pointing
to generated build inputs, `/toolchain` to a read-only Rust toolchain, and a new
`/output` mount. Supply `ASSETS.sha256` for `reader.rs`, `reader.css`, `data.rs`,
`data.json`, `app.js`, `enhance.js`; it verifies them before compiling. Node/npm
must be absent. `node_free.sh runtime` instead tests current prebuilt Product
executables mounted at `/product`. Use `--network none --read-only --tmpfs /tmp`.
The scripts do not acquire tooling or alter the host installation.

The report cites exact focused Docker commands for the real current installer
from a Git archive, with a fresh build target, preinstalled Rust/C toolchain and
read-only Cargo download cache. That run includes no Node, network or old build
outputs. It does **not** measure internet downloads or toolchain installation.

Before reporting any result, inspect the focused wrapper's `result.json`, numeric
exit/termination, complete stdout/stderr and the inner result. Do not convert
technical checks or authored prose into human comprehension, autonomous generation
quality, semantic approval, production adoption or cutover acceptance.
