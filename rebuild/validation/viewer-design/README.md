# Viewer journey comparison

Disposable local reading design; production migration, framework choice and human
acceptance are pending. This support generates no explanation and invokes no
provider. The exact artifact, source-first outcomes, measured benefits/regressions
and feedback audit are in [journey-review.md](journey-review.md). The provisional
[experience requirements](experience-requirements.md) do not replace Product owners.
Earlier [card](card-details-review.md), [palette](blue-gray-review.md) and
[initial](report.md) reviews remain unchanged.

The delivered private package starts at
`rebuild/.local/viewer-journeys/review/index.html`; answer-free comparison tasks are
in `journeys.html`, measurements in `journey-observations.html`. The movable
`rebuild/.local/viewer-journeys/viewer-review.zip` contains the same package.
Extract it anywhere. Open the entry directly, or run
`python3 serve.py` **from that package directory** and open the printed loopback
URL. Stop with Ctrl-C. Python 3 is the only preview runtime. All review links,
Source files, scripts, baseline snapshots and observations are package-local.
The script-free documents need neither Python nor JavaScript. Do not publish the
private retained material.

## Matched journey inputs

The ordinary A/B/C catalog contains historical Works, while B/Click reading controls
have separate identities. `journeys.html` uses matched single-control discovery
for each comparison; do not treat the historical catalog as the same candidate set.
All relevant Work prose, facts, Goals, source ranges and audits remain; discovery
and non-Work destination differences are explicit in `inputs/journeys.json`.

Prepare the independent missing-explanation case from the existing authored fixture
using the retained, baseline-bound CLI, then pass its output to the builder below:

```sh
rebuild/scripts/validate focused viewer-journeys-missing -- python3 -B \
  rebuild/validation/viewer-design/journeys.py \
  --fixture rebuild/.local/reader-documents/support-overview/goal-only \
  --binary rebuild/.local/reader-documents/final-bin/volicord \
  --output rebuild/.local/viewer-journeys/new-missing
```

This reads canonical Goal/Checkpoint absence independently and runs status/export
against a disposable Runtime copy. It creates no canonical records or explanations.
Originals stay read-only. Missing Source remains unavailable; an unrelated repository
function cannot supply the answer. `journeys.py` fixes bounded source-first assertions
from original code/stdout/reports; answer-free presentation tasks are separate.
These checks are not a blind model evaluation or human acceptance.

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
  --catalog rebuild/.local/viewer-task-design/blue-gray/catalog.json \
  --retained rebuild/.local/reader-documents \
  --prototype rebuild/.local/prototype/prototype-v2-as-reviewed.html \
  --missing-case rebuild/.local/viewer-journeys/new-missing \
  --output rebuild/.local/viewer-journeys/new-candidate

rebuild/scripts/validate focused viewer-design-inputs -- python3 -B \
  rebuild/validation/viewer-design/self_test.py \
  --retained rebuild/.local/reader-documents \
  --build rebuild/.local/viewer-journeys/new-candidate \
  --missing-case rebuild/.local/viewer-journeys/new-missing \
  --stress rebuild/.local/viewer-journeys/new-stress.html
```

The builder verifies Product source equivalence to the baseline capture commit.
If it fails, recapture a current accurate baseline rather than silently using an
older layout. Captured Current means historical readback, not renewed retention
or current Source freshness. B/Click are independently Source-derived authored
controls previously recorded/read through Product, not autonomous generation.

Build-time syntax highlighting uses the installed Python Pygments package; the
preview and script-free outputs do not depend on it. Unknown code languages remain
plain text. No analyzer capability or Product dependency changes.

Browser prerequisites are the already available Playwright Core, Chromium and
Korean font configuration. A private JSON configuration supplies absolute paths:

```json
{
  "build": "/absolute/ignored/new-candidate",
  "output": "/absolute/ignored/new-browser-output",
  "stress": "/absolute/ignored/new-stress.html",
  "playwright": "/existing/playwright-core/module",
  "chromium": "/existing/chromium/executable",
  "fontconfig": "/existing/korean-fonts.conf",
  "refined": true
}
```

```sh
rebuild/scripts/validate focused viewer-design-browser -- node \
  rebuild/validation/viewer-design/browser.cjs /absolute/private/config.json

rebuild/scripts/validate focused viewer-listener -- python3 -B \
  rebuild/validation/viewer-architecture/listener_test.py
```

The browser driver checks B/Click at 1440/768/390/320 CSS px, every
citation/return, original Source coordinates and content parity, history, keyboard
code scrolling, missing Source, wrong Work, hostile long multilingual controls,
actual requests and script-free offline documents. It records screenshots,
identities and explicit child cleanup. With journey inputs it additionally compares
30 paths (three cases, both layouts, 1440/1024/768/390/320), native Tab discovery,
first-screen geometry, scripted scroll/selection/return, missing explanation,
condition navigation and focus containment. Timings are automation, not human
search or comprehension. The listener control compiles the actual
Rust listener with independent neutral assets; `--binary EXECUTABLE` can instead
exercise a launcher for an already built preview. Local socket/Chromium operations
may need the execution environment's sandbox exception.

`package.py --candidate DIR --browser DIR --output NEW_DIR --validation RUN_DIR
[RUN_DIR ...] [--initial INITIAL_DIR --initial-browser INITIAL_OBSERVATIONS]`
verifies tested candidate manifests, copies the unchanged reader
and evidence into a new package, adds an observations page and entry link, and
hashes every delivered file. Browser evidence remains bound to the preserved
`candidate-manifest.json`; the final manifest identifies the packaging changes.
Run `self_test.py --retained DIR --build FINAL_DIR --missing-case MISSING_DIR` to
check final hashes and all active local files/fragments recursively. Use a fresh
browser configuration with `build` set to FINAL_DIR and a new `output`, then run:

```sh
rebuild/scripts/validate focused viewer-review-delivery -- node \
  rebuild/validation/viewer-design/review.cjs /absolute/private/review-config.json
```

This copies the package outside the checkout, runs exactly `python3 serve.py` from
that directory, checks comparison/initial/measurement/reading links at desktop and
mobile sizes, stops the server and tests network-disabled script-free reading.
Inspect wrapper `result.json`, stdout/stderr and the inner result;
exit 0 is technical support evidence, never human understanding or approval.

The review package carries exact support copies. Its `support_commit` records the
commit whose source bytes were independently compared with those copies. Absolute
paths in retained JSON describe historical provenance; the runnable package does
not resolve them. No repository-wide aggregate, new evaluation platform, frontend
framework dependency or Node runtime service is introduced.

## Project overview and catalog

`--catalog` is a private, explicitly authored review JSON input. It identifies an
immutable historical canonical export by full SHA-256, a Project and exact
Goal/Checkpoint identities, and separate authored titles/summaries. `catalog.py`
verifies Project/Work association and the latest retained Checkpoint before deriving
the status badge. It is disposable fixture reading, not a Product import or write.
The current catalog selects Volicord A/B/C; the overview shows the latest three
records and the Work page shows all records. With three records, cards match.

Project goal text is an exact paragraph from the independently hashed maintained
charter, attributed as a document quote. The historical Project has no canonical
Project Purpose; a Work Goal is never promoted to Purpose. A/C detail pages expose
their original Goal, canonical report and limits, without claiming new detailed
code interpretations. B keeps its independently admitted reading document. Click
remains a separate no-change review control, outside this Volicord catalog.

The revised disposable trial uses the reference light/dark blue-gray palette,
visible summary text on title/status/summary cards, explanation before limits,
boxed original instructions/full evidence, explicit button and disclosure states,
and build-time Rust/Python lexical coloring. The source-owned production Viewer
and its existing opening-limit contract are unchanged. Earlier user layout
instructions explain the palette/cards; approval of the new exact target is pending.
The revised scoped Code uses a wide focused dialog with the original claim,
contained keyboard focus and exact return. Conditional results have a source-quoted
table, and the full limit section has an opening shortcut. These are provisional
design choices; Product adoption remains a separate decision.

Card navigation is limited to the reading button; titles are plain text. Creation
time comes only from the first Goal revision, completion from a completed
completion Checkpoint. Timestamps display in KST with exact machine-readable
values and scoped tooltips; missing evidence remains explicitly missing. Limits
use the same divider and spacing as other narrative sections. Work-page guidance
does not hardcode the fixture count.
