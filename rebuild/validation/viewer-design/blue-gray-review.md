# Viewer review: project overview and blue-gray reading

Status: revised disposable candidate, responding to explicit user feedback;
production migration and overall human acceptance remain pending. Entry HEAD:
`3465d632eb08d1c3672b529aac6e5edb985856b3`. The worktree was clean. The original
[trial report](report.md), prototype, inputs and generated review package remain
unchanged. Only the validation support in this directory changed.

The current private entry is
`rebuild/.local/viewer-task-design/blue-gray/review/overview.html`; the complete
comparison starts at the sibling `index.html`. Run `python3 serve.py` from that
package directory. All live links, source files, baseline snapshots, scripts and
screens remain package-local. No Node runtime, remote asset or provider is needed.

## Revised composition

- Palette roles match the independently identified reference exactly: page
  `#f3f5f9`, panel `#ffffff`, soft surface `#f3f6fa`, ink `#1c2b40`, muted
  `#53637a`, border `#dce3ec`, accent `#2357a0`, selection `#eaf1fc`.
  Dark panel/soft/ink/muted/border/accent/selection likewise match the reference.
- Project overview presents an attributed Project goal and the latest three
  Works. The Work page lists all Works. Both contain the same A/B/C cards in
  descending retained Checkpoint time for this three-Work catalog.
- Cards contain a title, recorded status and visible content summary, with a
  distinct reading button. Original A/B/C Checkpoints are completed; this does
  not establish fresh execution, verification or acceptance.
- Work explanation precedes its limits. Source-unavailable notices remain
  immediate. Original instructions and complete evidence are separately bordered
  native disclosures with visible expand/collapse controls.
- Navigation has a selected state. Reading, return, evidence, disclosure and
  source-access controls use borders, fill, hover and keyboard-focus states.
- Rust/Python syntax colors are generated with locally installed Pygments 2.17.2.
  Span text is checked against exact Source bytes and line coordinates. Multiline
  comments/strings retain their text. Coloring implies no semantic verification,
  diff or narrower claim-to-line association. Other languages remain plain text;
  Product polyglot capability is unchanged. Offline exports retain the colors.

This applies the user's layout direction to a disposable design. It does not
rewrite the production projection owner's opening-limit contract or select a
frontend framework.

## Independently admitted project material

`inputs/catalog.json` carries the explicit authored card wording and the exact
bundle/document identities. `catalog.py` reads the hash-bound historical canonical
export without importing it or changing records. It rejects foreign Project/Work,
wrong/non-latest Checkpoint, unsupported state and ungrounded Project goal text.
The three historical Goals are A `85c42f32b1b55870e9242f14ebe00a92`, B
`8d2bd548b3c992616f798045ec72d5da`, C `9b366bdbc34592f725332249b7ffb2ad`.

The project goal is an exact quotation from the maintained Product charter,
separately attributed as a document. No canonical Project Purpose was present;
none was invented. Card titles and summaries were authored from the original
Goals/Checkpoint reports, never from prototype prose. A/C detail preserves those
original records and limits; no new detailed code explanation was generated for
those Works. B retains the prior full admitted reading document and Source ranges.
Click remains available as an independent no-patch comparison control.

## Focused evidence

All successful wrappers below have numeric exit 0, no forced termination or spawn
failure, and preserved stdout/stderr. Inner results and actual screenshots were
inspected. The original listener and preview-server code were not changed again.

| Evidence under `rebuild/.local/` | Result |
| --- | --- |
| `validation/20261016T092116.431412Z-viewer-blue-gray-controls-final-rht7uqot` | 15 controls passed: original input checks plus catalog association, wrong Work/goal, exact text after lexical coloring, hostile/multilingual text, package hashes and active links |
| `validation/20261016T092118.467984Z-viewer-blue-gray-browser-final-nh5mslnz` | 1440/768/390/320 px, A/B/C card parity and original states, goal quote, navigation, original-record boxes, limits after explanation, syntax spans, original Source coordinates, keyboard code scrolling/history/return, unavailable/wrong Work, offline documents and 132 citation round trips |
| `validation/20261016T092237.049407Z-viewer-blue-gray-dark-palette-an13mpw7` | Dark palette exact-reference values and colored mobile code inspected at 390 px |
| `validation/20261016T092344.456995Z-viewer-blue-gray-manual-package-d9v9o7ff` | Exact documented launch, identical overview/all-Work cards, keyboard-opened bordered instructions, colored code/return and final screenshot links passed |

`review/manifest.json` identifies every delivered file; its preserved
`candidate-manifest.json` binds the exact browser candidate. Screens and the full
browser inner result are under `review/observations/`. The observed changes have
clearer controls and consistent blue-gray surfaces in desktop and mobile captures;
that is agent visual inspection, not a new human acceptance claim.

Skipped: workspace aggregate, Rust tests/Clippy, Final/V11, legacy and provider
validation. No Product/Rust/Cargo/dependency contract changed. No required tool was
unavailable. Remaining limits: long code lines require horizontal scrolling,
A/C have canonical report detail rather than newly generated source interpretation,
and screen-reader use, native zoom and human comprehension remain unobserved.
The complete original campaign ZIP was not independently reverified.
