# Focused Viewer reading findings

Synthetic diagnostic support only; no human observation or qualification.

Baseline HEAD: `8c81800f80e7afe292ccc7faa944caee57ff23ef`. Maintained workload: Viewer integration test
`representative_large_repository_page_is_deterministically_bounded`, 192 generated
Python modules plus its maintained setup, deep English rendering, one cold and
eight warm renders. Test source SHA-256: `263785226703b6e9831f5a7750b120da9c8e88846242dea2d427cd9deaef613d`.
Environment: Linux 6.18.33.2-microsoft-standard-WSL2 x86_64;
rustc: rustc 1.97.1 (8bab26f4f 2026-07-14); cargo: cargo 1.97.1 (c980f4866 2026-06-30); python: Python 3.12.3; git: git version 2.43.0. Debug test profile.

Command: `rebuild/scripts/validate focused viewer-reading-baseline -- cargo test
--manifest-path rebuild/Cargo.toml -p volicord-viewer --test viewer
representative_large_repository_page_is_deterministically_bounded -- --exact
--nocapture`. Numeric exit 0, no termination/spawn error. Cold render
92.012 ms; warm range
90.380–93.156 ms.
Each render: one projection pass, one analysis decode, zero health re-decodes.
Raw complete streams/result remain ignored under `rebuild/.local/validation/`.
This does not measure long-history exact selection, RSS or human reading cost;
no universal latency claim follows. Existing stage profiles were available; no
additional profiler was required. Re-run the named workload to compare changes.

Prototype input: `fixtures/viewer-reading/scenario.json` and independent
`expected.json`. Run `python3 rebuild/validation/end-to-end/multi-repository/
viewer_reading_prototype.py` (one unbroken script path) to recreate a disposable
self-contained HTML under ignored `rebuild/.local/viewer-reading/`.

Reading tasks: choose a Work by identity despite duplicate titles; follow its
Goal, changes, next step, scoped Decisions/code, then exact record evidence.
Prototype annotations are explicitly labeled and never canonical facts. No
listener, provider, new production Viewer, runtime home or database is created.

Observed by deterministic artifact inspection: identity anchors expose distinct
Works; tools sit separately; raw audit-heavy multilingual text requires explicit
disclosure. Missing rationale/result remains unavailable. Navigation alone cannot
explain long free prose; excerpts still require reading the original to determine
intent/results. Semantic Work realization, production navigation/selection controls,
read-cost optimization and human comprehension evidence remain unimplemented.

Baseline dependency identity: `rebuild/Cargo.lock` SHA-256 `02ed72ca48eafa2ec8162716d85fb557f0d9da5f5166486e259f33f604ce277a`.
Fixture setup and generator are in the hashed test source; no private input was used.

Maintained regression consumer: `volicord-operations --test viewer_reading` loads
the shared scenario and independent expectations, constructs real canonical
records through Local Operations/Store, and adds a declared synthetic semantic
binding between the C and TypeScript file entities. This binding is not a claim
that the structural analyzer resolves cross-language calls. The Python fixture
also contains real local-call and unresolved-call evidence; existing projection
flow tests check that unresolved endpoints never become fabricated entities.

Implemented reading limits: display quotations keep 384 Unicode scalar values,
report exact omitted bytes/characters, and retain full original text. Selected
Work/history/Decision details survive bounded parent lists; graph endpoint bounds
and omissions remain explicit. Failed, rejected and unverified observations keep
their own Checkpoint and Source identities; earlier passes expose later changes.
Missing explanatory text still cannot explain intent or results. Fixed bilingual
labels preserve original quote language rather than attesting translation.

Remaining read-cost work: canonical full-history reads and revision catalogs,
full latest-analysis decoding, canonical-scope clones, repeated per-Work/source
scans, and full selected-Work original/state arrays are not paginated or cached.
The earlier baseline measures the maintained default rendering workload only.
The foundation makes no measured speedup, long-history memory ceiling or human
comprehension claim. Later Viewer controls must use the maintained selector APIs
and owner contracts, not depend on this disposable HTML.


## Purpose-oriented navigation implementation

The Viewer README now owns the operative route, CLI and whole-snapshot procedures.
`ViewerView` and `CodeScope` map to the shared Work selector; `ProjectionDetail`
resolves Decision/entity detail before parent display bounds. Work and Decision
indexes have native previous/next links in pages of 64. Static export uses one
canonical/analysis projection, shared section functions, native details and only
existing unique internal fragments. Selected-Work code is explicitly omitted from
this bounded whole snapshot; repository code remains separately labeled.

Real Viewer render, in-memory HTTP and executable listener tests exercise the
maintained adversarial canonical fixture through Local Operations. Semantic assertions
cover exact old Work identity, all its observations, missing rationale, Goal-only
work, independent failed/rejected/unverified states, multilingual quotation labels,
strict invalid/not-found behavior, pagination and static fragment closure. Mutation,
forgetting/repair, document export, Guarded target/revision/Source and disconnect
checks remain actual entry-point checks. No markup check attests browser interaction
or human comprehension.

Focused navigation validation: Viewer (all targets/features), Operations CLI/reading,
projection/document/current-flow, V06, current CLI parity, V11 self-check, archive,
Dogfood capture and campaign self-tests passed. Listener and campaign special-socket
fixtures required sandbox escalation for local socket access. V08 assertions stopped
at the pre-existing historical Production-drift guard against `c17279bb`; it did not
run the integration journey. The guard was not weakened. Current executable Viewer
and Operations CLI checks supply scoped entry-point evidence only.

The reading task "explain a long free-prose Goal/result without reading the original"
remains unsupported where canonical input offers no semantic summary. Quotations,
labeled excerpts and explicit unavailable explanations are shown; fixed labels do
not attest translation. Browser keyboard, touch, zoom and narrow-screen interaction,
plus actual human comprehension, require independent next-session verification.


## Grounded code navigation

Focused Code tests cover long/duplicate symbol labels, distinguishing path suffixes,
variable node heights, SCC cycles, distinct parallel ports and self-loop geometry.
The real-entry-point 90-module fixture selects an entity omitted from the first map,
checks incoming/outgoing lists and retained Source/ranges, rejects unrelated Work
scope, and checks HTML escaping with the actual `unsafe<&>.py` locator. The original
polyglot fixture and unresolved structural calls remain in shared Operations tests.
No constituent relations are grouped; graph omissions and upstream omissions have
separate counts, and full names/endpoints remain in the native details/list path.
Snapshot links remain closed to actual unique targets.

No browser automation tool, Chromium binary or Python Playwright installation was
available in this session. Prototype and production artifacts were inspected through
maintained markup/semantic assertions, not browser interaction or visual/human review.
Independent browser checks must exercise SVG focus/fragment behavior, details keyboard
and touch operation, zoom, long-name node fit, dense/cyclic edge legibility and narrow
ordinary-text layout. This session does not implement that dedicated runner.

## Requested-section implementation and reproducible budgets

This section supersedes the earlier remaining read-cost list for unnecessary
Viewer graph/document work. Canonical complete-history reads, revision catalogs,
scoped clones and per-Work/source scans remain; no persistent cache was added.
`ProjectionReadRequirements`, `ProjectReadSections` and the profiled Operations
entry point are maintained in the projection owner and Viewer README. Ordinary
Overview/Work/Decision requests read AnalysisMetadata, with code/inspection
`NotRequested`; Code requests one graph, and snapshots share one projection and
graph among all Works and four previews. Thin projections cannot generate full
documents or narrative plans. Failed decode attempts are counted. Store/recovery
health retains material warnings without silently validating graph bodies;
full integrity diagnostics remain in Tools Status. Corrupt graph and unavailable
analysis keep canonical remainder; metadata alone does not attest graph integrity.

The workload `requested_sections_on_large_repository` uses the same maintained
canonical scenario and polyglot repository as the independent reading assertions,
plus 192 Python files generated by the original large-fixture function pattern.
It selects the exact older Work, explicit Decision and actual `python/worker.py`
entity. Setup/analyze/compilation are excluded. Each route gets a fresh adapter,
one first request and eight warm requests. Cold means adapter cold, not flushed
OS caches. Debug profile, Linux 6.18.33.2-microsoft-standard-WSL2 x86_64, Intel
Core i7-13700K (16 exposed logical CPUs), rustc 1.97.1, cargo 1.97.1. No provider,
network analysis, thread-limit override or expensive concurrent verification was
used for the budget verification.

| Route | Initial cold ms | Initial eight warm range ms | Verification cold ms | Verification eight warm range ms | Total ceiling ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Overview | 43.334 | 33.578–47.729 | 34.412 | 33.511–34.321 | 65 |
| exact Work | 35.126 | 33.088–37.858 | 33.156 | 32.084–34.665 | 65 |
| exact Decision | 34.200 | 33.822–37.846 | 35.383 | 32.922–34.889 | 65 |
| exact Code entity | 99.127 | 96.136–103.258 | 98.455 | 96.741–99.146 | 135 |
| whole snapshot | 122.652 | 122.219–129.358 | 119.843 | 119.372–125.802 | 180 |

[Read budgets](viewer-read-budgets.json) were fixed after the initial measurement
and before the separate verification. They retain rounded stage headroom above
observed maxima: ordinary canonical/analysis/projection ≤30/18/5 ms, no Candidate
basis read or document work; Code analysis/projection ≤95/15 ms, no documents;
snapshot analysis/projection/documents/HTML ≤95/18/18/30 ms. Other measured stage
ceilings are explicit in the JSON. Counts are unconditional semantic checks;
timing enforcement is opt-in for this named environment. These are regression
ceilings, not estimated population percentiles or universal product latency.
Existing V11/resource acceptance budgets and the original large-page 180,000-byte
ceiling were not relaxed. Snapshot output on this fixture was 876,117 bytes;
retained audit/original text is included rather than claimed redacted.

Initial raw samples/result: ignored validation artifact
`20261002T045922.657230Z-viewer-cost-measurement-ready-w_yc7p2q` (exit 0).
Independent budget verification:
`20261002T050129.186719Z-viewer-cost-budget-verification-malyv9yj` (exit 0).
Each preserves complete separate streams, timestamps, argv and numeric result;
no raw measurement files or Runtime state are committed. Verification stage
maxima, in milliseconds: ordinary canonical ≤14.886, analysis ≤10.864,
projection ≤2.061; Code analysis 67.602/projection 7.684; snapshot analysis
69.831/projection 7.739/documents 8.708/HTML 12.679. Samples do not establish RSS,
arbitrary long-history cost, OS-cache-cold behavior or human reading speed.

Reproduce counts and raw samples:

```text
rebuild/scripts/validate focused viewer-read-cost -- cargo test --manifest-path rebuild/Cargo.toml -p volicord-viewer --test reading requested_sections_on_large_repository -- --exact --nocapture
```

On the stated environment, reproduce the fixed timing check:

```text
rebuild/scripts/validate focused viewer-read-budget -- env VOLICORD_VIEWER_BUDGETS=1 cargo test --manifest-path rebuild/Cargo.toml -p volicord-viewer --test reading requested_sections_on_large_repository -- --exact --nocapture
```

Cost-commit source/fixture identities (before the final entity-focus fix):
`tests/reading.rs` SHA-256 `703eb3f6723716ca65c412565a35f53c8cb32cae76f6c4f1b00abe281c637068`;
scenario `e63cae059edd39ea3162ef354684442e3763459a5c2f9008a55f35fce1b44673`;
independent expected fixture `e898e390988b5e9bd5af6c62db77b695d94fe6053afb3f328e73254576bea456`;
Cargo.lock `665fc1c6f039c88df5031484d85effbe31f29c52c70ea0f78dfcd325c70f68d1`.
The older 192-module Evidence fixture also passes its unchanged byte ceiling:
cold 90.090 ms, warm 84.852–90.652 ms, one graph decode, no document work
(`20261002T050034.586465Z-viewer-cost-comparable-keafgipl`, exit 0). Its canonical
setup and surface differ from the richer requested-section workload; comparisons
to the historical deep page are diagnostic, not controlled speedup claims.

Focused post-change checks cover all Viewer targets/features, selective-read
profiles, 70 additional Works without per-Work re-decode, exact Work-scoped
snapshot Decisions, internal fragment closure, Operations analysis/recovery and
forgetting barriers, full projection/documents and default consumer semantics.
Scoped clippy is clean. Tools Status can validate other retained snapshots as
explicit diagnostic work; the no-duplicate invariant applies to ordinary Code
and whole-snapshot paths. No gate, Final aggregate, official V11, naturalistic
campaign or human qualification ran. Browser gaps listed above remain for the
independent browser session; the README supplies installed/development launch and
export procedures and actual routes instead of requiring a temporary prototype.

## Final selected-entity focus correction

Final review found that an exact entity beyond the parent map was reachable in
its native detail, while the diagram could still show the initial neighborhood.
The shared `ProjectUnderstanding` presentation now focuses on the selected entity
and its actual bounded stored neighbors/incident relations. The 90-module HTTP
fixture asserts the omitted entity is a marked SVG node and that every focused
component/relation belongs to that stored neighborhood. Work-scope validity still
uses the existing canonical seed/real one-hop rule. No extra graph read, inferred
relation, Work ownership or runtime-flow claim is introduced. Generic repository
explanations and empty-flow wording no longer imply canonical Work-seed linkage.

The unchanged fixed budgets pass after this correction:
`20261002T051604.642859Z-viewer-selected-focus-budgets-0i24cy38`, exit 0,
one first/eight warm samples per route. Overview 45.678 ms cold, 32.703–44.025 ms
warm; Work 33.274 ms / 30.757–39.215 ms; Decision 32.166 ms / 31.569–33.831 ms;
focused Code 96.399 ms / 93.194–98.106 ms; snapshot 116.838 ms /
116.634–120.259 ms. Focused Code output is 41,883 bytes; snapshot is 876,604 bytes
on this workload. Count/stage ceilings remain unchanged. Latest test source
`tests/reading.rs` SHA-256 is
`d3682c02b111ab8c0d7088fb56618549ede4e31d563f7dc1e579b1c400c9b005`;
fixture and Cargo.lock identities above remain unchanged. These are focused
synthetic checks and retain the same browser/human/long-history limitations.


## Independent browser verification

The maintained `viewer_browser.py` and coupled driver replace the earlier
browser-tooling gap with bounded V11 supporting observations. They use freshly
built sibling CLI/Viewer binaries, actual `viewer open` and `viewer export`, and
real disposable Local Operations/Store records. The unchanged scenario and
independent expectations drive exact Work/history/Decision assertions. Raw
analysis identities, endpoints, Source and ranges supply the code basis; no
production projection helper supplies the entire expected result. A canonical
identity ordering fixture places the older Work past 64 other choices. A second
Project tests missing Purpose without treating its Goal as Purpose.

Browser inspection exposed two production defects: disclosed Work audit `dt`
labels overflowed a 390 CSS-pixel page (510 px scroll width, also failing at
native 200% zoom), and proportional-font SVG labels extended outside their node
rectangles. The fixes add ordinary audit-label wrapping and use bounded
monospace cell wrapping for diagram text, retaining long distinguishing endings
and wide Korean labels. Coupled Rust/browser regressions cover these surfaces;
the five copy mutations independently detect removed wrapping, prefix-only
labels, missing fragment targets, cross-Work facts under a retained identity,
and live links in snapshots. Originals and canonical facts are preserved.

The verified tooling is full headless Chrome for Testing 151.0.7922.34,
Playwright Core 1.62.1 and Node 24.19.0. Actual tab zoom uses a test-owned
extension's `chrome.tabs.setZoom` and `getZoom`, with per-tab automatic settings,
CSS zoom 1, visual viewport scale 1 and recorded width/pixel-ratio changes.
Native Tab/Enter reaches views, paged Works, SVG fragments, relationships and
Source disclosures. Scope-local native details, ordinary/root overflow and
independent graph wheel scrolling are checked at 390/768/1440 base widths in
both locales at 100% and 200%. Offline snapshots use real `file:` documents
with networking disabled after listener shutdown and physical Runtime removal.

Direct screenshot inspection found two environment/evidence limitations:
Playwright surface captures could be blank at deep scroll offsets with native
tab zoom, and the available fontconfig initially had no Korean font. The driver
now captures the actual browser viewport through `Page.captureScreenshot` with
`fromSurface=false`, scrolls to the relevant reading/disclosure surface and
records geometry. Korean glyph coverage is a checked prerequisite; a separately
prepared Noto Sans CJK KR font/configuration produced readable Korean captures.
Fonts are local validation tooling, never embedded or remotely fetched by the
Product or check. The runner records their paths/hashes and blocks missing
coverage instead of treating tofu glyphs as locale verification.

The final development positive control has 115 live and 17 offline checks,
including all five restored negative controls, Purpose absence and actual
stale/unavailable analysis. Server/child exits, termination and process cleanup
are separately preserved. HTTP response completion (36 samples), automation
input/two-animation-frame scheduling and Rust read-stage profiles (45 samples)
remain distinct; no actual human-paint, accessibility-completeness, RSS or
percentile claim follows. These development artifacts use an uncommitted runner
and are not final candidate evidence; a clean committed run is required.

The unchanged fixed debug workload budgets passed in the Korean-font control:
Overview 32.968 ms cold / 31.788–34.983 ms warm; Work 33.205 / 30.710–32.780;
Decision 32.164 / 32.787–36.025; Code 93.240 / 92.312–106.587; whole snapshot
118.351 / 115.123–122.692. Ordinary reads decoded metadata only and did no
Candidate/document work; Code decoded one graph and generated no documents;
snapshot decoded one graph, read Candidate basis once and generated four
previews. A preceding capture control failed Overview sample 2's understanding
stage at 4.633 ms against 2 ms while total time was 37.669 ms. Its failed raw
sample/log remains retained; the cause of that wall-time outlier is not
established, ceilings were not relaxed, and a later pass does not erase it.

The identified historical baseline was independently extracted using `git
archive 8c81800f80e7afe292ccc7faa944caee57ff23ef rebuild` into an ignored copy.
Its test and Cargo.lock hashes match the earlier recorded inputs exactly. The
original named 192-module workload was compiled/run offline with a separate
target directory: numeric exit 0, cold 94.327 ms and eight warm 90.032–92.831 ms,
one projection/decode and zero health re-decodes. Python V11/Dogfood controls
ran concurrently, so this is a reproducible diagnostic, not a controlled
speedup comparison. The baseline and richer requested-section workloads also
have different canonical input and reading surfaces.

Existing V08 assertions stop at their historical `c17279bb` Production-drift
guard before the integration journey. The unchanged guard belongs to V08;
current Viewer executable/CLI tests are separate scoped evidence. A Dogfood
campaign self-test initially failed restricted Unix-socket creation and cascaded
fixture cleanup; permitting local sockets made the maintained suite pass. No
historical evidence, gate policy, external authorization, human verdict or
qualification criterion was rewritten. Complete local logs/results/screenshots
remain ignored, while final browser and gate/archive evidence are reported
separately in the conversation.

Semantic summary-unavailable is still an observed comprehension limitation.
Quotations and truthful missing rationale do not explain intent on their own.
Fresh naturalistic Work A/B usage, Decision/Source comprehension, Learning and
semantic value, Naturalistic-memory evidence and direct en/ko human observations
remain separately required. Automated support and later gate execution cannot
supply those judgments or open Phase 9.
