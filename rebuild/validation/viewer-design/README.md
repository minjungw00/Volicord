# Task-centered Viewer design trial

Disposable local reading design; production migration, framework choice and human
acceptance are pending. This support creates no canonical records, generates no
explanation and invokes no provider. The provisional review contract and observed
limits are in [report.md](report.md).

The delivered private package starts at
`rebuild/.local/viewer-task-design/review/index.html`. Open it directly, or run
`python3 serve.py` **from that package directory** and open the printed loopback
URL. Stop with Ctrl-C. Python 3 is the only preview runtime. All review links,
Source files, scripts, baseline snapshots and observations are package-local.
The script-free documents need neither Python nor JavaScript. Do not publish the
private retained material.

## Reproduce with the available retained inputs

From the repository root, select new ignored output paths. Existing outputs are
never overwritten. `build.py` rejects missing originals, altered identities,
unavailable/incomplete readback, wrong Work or claim-to-Source associations,
changed exact byte/line ranges and an unsupported before-state. The prototype is
read only to identify the visual reference; its text is never a rendering input.
Historical prototype/feedback/fixture identities are verified but their executable
HTML and editorial text are excluded from the delivered content inputs.

```sh
rebuild/scripts/validate focused viewer-design-build -- python3 -B \
  rebuild/validation/viewer-design/build.py \
  --retained rebuild/.local/reader-documents \
  --prototype rebuild/.local/prototype/prototype-v2-as-reviewed.html \
  --output rebuild/.local/viewer-task-design/new-candidate

rebuild/scripts/validate focused viewer-design-inputs -- python3 -B \
  rebuild/validation/viewer-design/self_test.py \
  --retained rebuild/.local/reader-documents \
  --build rebuild/.local/viewer-task-design/new-candidate \
  --stress rebuild/.local/viewer-task-design/new-stress.html
```

The builder verifies Product source equivalence to the baseline capture commit.
If it fails, recapture a current accurate baseline rather than silently using an
older layout. Captured Current means historical readback, not renewed retention
or current Source freshness. B/Click are independently Source-derived authored
controls previously recorded/read through Product, not autonomous generation.

Browser prerequisites are the already available Playwright Core, Chromium and
Korean font configuration. A private JSON configuration supplies absolute paths:

```json
{
  "build": "/absolute/ignored/new-candidate",
  "output": "/absolute/ignored/new-browser-output",
  "stress": "/absolute/ignored/new-stress.html",
  "playwright": "/existing/playwright-core/module",
  "chromium": "/existing/chromium/executable",
  "fontconfig": "/existing/korean-fonts.conf"
}
```

```sh
rebuild/scripts/validate focused viewer-design-browser -- node \
  rebuild/validation/viewer-design/browser.cjs /absolute/private/config.json

rebuild/scripts/validate focused viewer-listener -- python3 -B \
  rebuild/validation/viewer-architecture/listener_test.py
```

The browser driver checks the two Works at 1440/768/390/320 CSS px, every
citation/return, original Source coordinates and content parity, history, keyboard
code scrolling, missing Source, wrong Work, hostile long multilingual controls,
actual requests and script-free offline documents. It records screenshots,
identities and explicit child cleanup. The listener control compiles the actual
Rust listener with independent neutral assets; `--binary EXECUTABLE` can instead
exercise a launcher for an already built preview. Local socket/Chromium operations
may need the execution environment's sandbox exception.

`package.py --candidate DIR --browser DIR --output NEW_DIR --validation RUN_DIR
[RUN_DIR ...]` verifies the tested candidate manifest, copies its unchanged reader
and evidence into a new package, adds an observations page and entry link, and
hashes every delivered file. Browser evidence remains bound to the preserved
`candidate-manifest.json`; the final manifest identifies the packaging changes.
Run `self_test.py --retained DIR --build FINAL_DIR` to check final hashes and all
active links. Inspect wrapper `result.json`, stdout/stderr and the inner result;
exit 0 is technical support evidence, never human understanding or approval.

The review package carries exact support copies. Its `support_commit` records the
commit whose source bytes were independently compared with those copies. Absolute
paths in retained JSON describe historical provenance; the runnable package does
not resolve them. No repository-wide aggregate, new evaluation platform, frontend
framework dependency or Node runtime service is introduced.
