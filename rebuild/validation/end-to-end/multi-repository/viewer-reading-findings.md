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

The browser timing support also records native PaintTiming first-paint and
first-contentful-paint marks for actual document loads, plus keyboard-navigation
input-to-FCP using browser time origins. These are distinct from HTTP completion
and automation input/two-frame intervals. Disclosure intervals retain their
explicit incremental-paint limitation. No new timing threshold or human verdict
is introduced. The authoritative gate exposed a separate contract-checker
fixture defect: maintained relative links to the Viewer guide and budget JSON
were not copied into its positive fixture. Copying tracked link targets and
adding both missing-target controls preserves the existing link-check authority.

The ordered Final suite also exposed an older HTTP regression fixture's invalid
assumption that every entity omitted from the repository map is outside a Work.
Identity ordering can omit a legitimate Work seed. The rejection counterexample
now selects a disconnected generated-file entity independently of the display
bound, retaining its 404 expectation. More real C Work-path seeds than the map
can display provide a separate positive control: an omitted seed remains 200
and the selected diagram node. Product scope/membership behavior is unchanged;
the named cost workload's generator/count/stage ceilings are unchanged. The
browser result now hashes the maintained reading-test source explicitly too.


## Whole-snapshot V11 consumer correction

A real authorized gate at candidate `5e230e66fae53a584d727ce92585ac224267bd07`
passed ordered Final, live provider qualification, all three authenticated probes,
resource budgets and credential audit. Its three `source_grounded_understanding`
checks failed because the consumer expected adjacent SVG attributes and compared
whole-snapshot repository Code against empty current-Work MCP components. It also
required removed empty-state wording and a resolved edge even when displayed stored
entities truthfully have no resolved relationship. The gate/result/archive remain
retained and independently verified; this failed aggregate is not a pass.

The corrected bounded consumer reads attributes independently of order, requires
repository scope labeling, matches node Analysis Snapshot identities with MCP
snapshot evidence, and follows exact native entity/list fragments and readable
labels. Every rendered edge must match the native exact relation identity, class
and endpoints; resolved relationships between displayed nodes cannot silently
lose their edges. Disconnected components require explicit no-edge/no-flow gaps;
empty diagrams require inspectable capability-gap explanations. Controls reject
missing edges, foreign snapshots, missing fragments/relations/narratives, changed
list labels, invented endpoints and uninspectable absence. This aligns a supporting
conformance consumer with the current snapshot scope; independent canonical/source
semantics remain the separate browser runner's responsibility. It adds no human
verdict, provider authority, threshold or alternative gate.

## Question-scoped answers and actual Work interpretation

The earlier record-reading/excerpt observations are superseded for ordinary Work
results by the current `WorkAnswers`, independent `WorkOverview`, shared locale
labels and explicit Work explanation lifecycle. Original evidence remains available;
Recall and document quotations retain their existing responsibilities.

The baseline focused product-entry-point reproductions failed with exit 101 for
all three reported defects: null result suppression, current Work outside an
ID-paged catalog, and Korean Debug labels. Connected regression tests now pass.
Ninety Works with identity order opposed to chronology demonstrate classification
before independent category bounds (eight) and exact total/displayed/omitted counts.
Blank state-change authoring is rejected by the canonical writer, so its attempted
prefix preserves prior answers; a supported null verification observation preserves
the earlier result. Later changes disclose earlier verification as historical.

The audit-heavy search case places semantic change after a large audit prefix;
a truthful 384-character excerpt cannot answer purpose/change/effect. The canonical
fields do not separately structure those facts. Current production uses explicit
active-host interpretation of a full evidence plan, rather than fixture recognizers
or a renamed excerpt. A reordered/paraphrased search variant and independent CSV
failure case exercise different source wording. The preserved generic result supplies
no identifiable feature or expected effect.

A fresh canonical Runtime was produced by maintained `seed_work_explanation_runtime`,
with no retained interpretations. The active Codex host read the actual public CLI
preparations, authored eight responses (four cases × English/Korean) and recorded
all eight through `work explain record`, each exit 0. Search answers describe
sequence-based older-response exclusion, its expected visible-result effect, the
recorded unit pass and missing browser checks. Export answers distinguish temporary-file
publication from a failed disk-full check and pending cleanup. Generic answers
state that the feature/effect is unspecified and separate explicit NotRun, review
and rejection. Responses/prose remain ignored local model output, not maintained
fixtures or production templates. Generator/model identity is self-reported/unknown,
not independently certified.

The maintained `work_explanation_browser.py` and shared browser driver observed all
five question answers in ordinary reading for eight actual loopback pages at 390×900;
all eight passed independent claim requirements. Result evidence stayed closed and
audit hashes were absent from ordinary answers. Creator uncertainty was inspectable.
GET/navigation changed neither canonical export bytes nor provider/managed-record
counts. Complete streams, numeric exit 0, screenshots and browser/font identities
were preserved in focused local artifacts. An initial DOM-success run revealed missing
Korean glyphs on screenshot inspection. A local Korean-capable font configuration
corrected that prerequisite; the runner now rejects absent Korean fonts. The rerun
passed and the Korean search screenshot was inspected for readable glyphs.

Lifecycle tests separately cover exact language, stale response rejection, wrong
format/evidence role, supported correction, conflict invalidation and fresh conflict
warning, corrupt/non-current cache, no verification record, explicit local deletion
and canonical forgetting with retained bytes removed. Fake test responses prove
mechanics only. Preparation/reference hashes do not prove prose truth or authorship.
The maintained reproduction inputs, public interfaces and browser commands are in
`rebuild/crates/volicord-viewer/README.md`; fresh host output is required on reproduction.
These observations establish a narrow implemented explanation path, not a human
comprehension assessment, naturalistic campaign, official V11 or cutover gate.


## Question-scoped answer integration review (2026-10-03)

Current implementation supersedes the earlier excerpt/prototype limitations above.
Entry baseline was `b87dcaf2`; before broad integration, public prepare/record/read
was independently exercised from an empty runtime with five maintained Work
inputs, including a new checksum task. The audit-heavy change was the final
same-Work and Project Checkpoint. English/Korean ordinary answers were compared
with original source, independent required/forbidden claims and exact basis.
The source-poor Work remained nonspecific. A real SHA-256 is legitimate checksum
subject matter; unrelated audit tokens remain in closed original evidence.

The integrated proof adds two Decisions: an offline-queue user reason distinct
from the remote-sync recommendation, and missing user rationale. Fourteen live
browser checks passed, with Work Overview/detail/snapshot paragraph equality and
Decision detail/snapshot equality. Current-host authored responses were recorded
through public CLI; the seed includes canonical inputs only. This is synthetic
content review, not human acceptance, a naturalistic campaign or official V11.
GET/export changed neither canonical records nor managed/privacy counts.
Korean screenshots were inspected with Noto CJK fonts. Reproduce via the seed,
prepare/record and browser instructions in the Viewer README; do not substitute
preloaded/generated expected prose for an actual generation step.

One `ExplanationPlan`/`RetainedExplanation` decoder and privacy-managed store
serve Work and Decision subjects. `prepare_explanation`, `record_explanation`
and `delete_explanation` bind project, subject, revision, language, full evidence
fingerprint, sources and self-reported host/model provenance. CLI entry points
are `work explain` and `decision explain` prepare/record/delete. `work_answers`
and `decision_answers` return shared `QuestionAnswers`: generated prose,
independent facts, availability/diagnostic and exact provenance. Consumers are
Viewer, all four documents, snapshot, status/decisions/Recall CLI and Recall/
repository-understanding MCP. Language and fixed UI locale are separate inputs.
No read invokes a provider or silently generates an answer.

Restart, malformed/foreign/language/version rejection, deletion, regeneration,
forget and rejection of saved stale document/snapshot publication passed against
Local Operations. Document metadata version is 8. Publication revalidates under
the mutation lock and never overwrites an existing file. Already exported offline
copies cannot be retracted. Generated text is self-reported interpretation;
structural validation checks bindings/citations, not semantic entailment or model
identity authenticity. Exact quote DTOs remain solely for Goal labels, original
result/next-step/rationale and closed inspection. Checkpoint observation facts,
code entity identity/navigation and graph evidence remain active. Removed paths
include primary result/status excerpts, duplicate next-step aggregation, flattened
verification lists, duplicate Decision explanation fields and the Work-only
explanation module/decoder. Code structure/flow explanations continue to derive
from actual graph evidence and disclose unresolved/runtime-flow gaps.

Relevant focused checks passed: affected projections/Operations/host/Viewer Rust
all-target/all-feature suites (D1/N1/N2 included), V06 document assertions, fixture
hashes, current CLI parity, multi-repository self-check (including restart
negative controls), Dogfood capture/resume/document-realization support and archive
support. External live provider qualification remains ignored without current
transmission authorization. Initial runner invocation/selector and displaced
wire-field expectations failed, were corrected, and rerun; full numeric outcomes,
stdout/stderr and cleanup remain in ignored focused artifacts. No gate/final was
invoked.

Pre-performance integration workload: `requested_sections_on_large_repository`,
192 added Python modules and maintained scenario history, debug profile, nine
samples per Overview/Work/Decision/code/snapshot; fresh adapter first, then warm
without OS cache flushing. Median milliseconds (total / projection / documents):
Overview 42.198 / 8.830 / 0; Work 41.221 / 7.608 / 0; Decision 39.908 / 8.434 / 0;
code 105.469 / 13.696 / 0; snapshot 217.526 / 14.371 / 94.900.
This still reads complete canonical history and eagerly copies detailed Work
histories. Bounded visible sections do not establish bounded read cost.
The pre-integration snapshot median was 122.476 ms; added live-basis validation
caused repeated canonical/store reads and requires measured improvement.

Generation-path costs are separate: fourteen public CLI preparations took
7.18–8.59 ms each (process startup included); full plan envelopes were
5,080–11,993 bytes. Ten Work records took 267 ms as a batch and four Decision
records 97 ms after rebinding. Host interpretation was performed in this session;
no isolated model inference latency or token-cost measurement was available, so
these timings measure preparation/recording, not model generation speed. Reads
consume the retained response without generating. No external provider was used.


## Required-evidence materialization review

The performance change uses a request-local borrowed Work history index to select
page/Overview/exact/Recall identities before copying detailed prose and historical
state evidence. One materializer supplies each required subject; all Overview
counts are computed independently of the page. Current Recall reuses that Work
selection. Timeline payload copying follows its declared bound. Exact Work and
unassociated topology seeds no longer start by cloning complete unrelated histories.
The canonical equality digest streams the same Debug bytes instead of allocating
another complete prose buffer; equality with the former digest is tested.
Explanation freshness is prepared once per retained subject/language per read,
using the same decoder and storage. Document-set validation gathers provenance
and checks it once after generation; inspection coordination preserves Candidate
store degradation. The initial use of mutation-path preparation broke the MCP
Candidate-unavailable preview invariant, was corrected, and the negative test and
workspace suite passed. Lifecycle/publication safeguards remain in force.

Named large-history workload: Operations integration test
`large_history_materializes_only_required_subjects_before_evidence_copying`.
A valid maintained fixture basis is expanded in memory to 512 Work identities,
8,192 Checkpoints (16 per Work, final verification-only observation), 56,033,280
reported-change bytes, mixed current/completed/paused states and page size four.
This isolates projection/history cost; canonical SQLite loading, OS cache flushing,
model generation and RSS are excluded. Pages 0, 64, 127 and 128 retain identical
Overview counts `[171,171,170,512]`, complete source/state/result meaning and reject
full document generation from explicitly unrequested sections.

An isolated `git archive 97ae54b6 rebuild` under `/tmp` ran the identical fixture
and invariant checks with only a measurement counter at the old Work materializer
and the new-cost assertions removed. Actual old materializations: 1,024 per page
(two complete passes); elapsed microseconds 1,284,388 / 1,204,130 / 1,210,367 /
1,210,297. Current materializations: 28 / 28 / 28 / 24, with 448 / 448 / 448 / 384
complete Checkpoint histories and 3,074,846 / 3,074,848 / 3,074,846 / 2,635,584
input bytes; elapsed microseconds 852,207 / 857,559 / 854,267 / 847,112.
The instrumentation is ignored experiment output; no alternate production reader
or performance branch remains. Reproduce the maintained current workload with:

```text
rebuild/scripts/validate focused answer-history-cost -- cargo test --manifest-path rebuild/Cargo.toml -p volicord-operations --test viewer_reading large_history_materializes_only_required_subjects_before_evidence_copying -- --exact --nocapture
```

Final nine-sample Viewer workload medians, same debug fixture/adapter conditions as
the integration baseline, milliseconds (total / projection / documents):

| Workload | Integration baseline | Current |
| --- | --- | --- |
| Overview | 42.198 / 8.830 / 0 | 41.569 / 9.367 / 0 |
| Exact Work | 41.221 / 7.608 / 0 | 40.261 / 8.796 / 0 |
| Decision | 39.908 / 8.434 / 0 | 41.434 / 9.681 / 0 |
| Code | 105.469 / 13.696 / 0 | 106.321 / 14.634 / 0 |
| Snapshot | 217.526 / 14.371 / 94.900 | 151.182 / 15.188 / 28.476 |

No consistent thin-route latency improvement is established at this fixture size.
Snapshot/document repeated-read reduction and large-history evidence construction
are directly measured; no universal bounded-total-read claim follows. The Viewer
workload has four Work groups/164 associated Checkpoints and no stored explanations,
so freshness-plan count is zero. A retained Work plus Decision lifecycle test
asserts exactly two preparations despite their duplicate appearances. The actual
fourteen retained answers also passed the browser proof after optimization without
regeneration. Code/snapshot still decode one analysis body; ordinary routes decode
one metadata envelope, with no health re-decode. Raw logs/results remain ignored.

Final required nested-workspace Rust validation: 500 passed, zero failed, one
external live-provider test ignored; no transmission authorization was inferred.
The new preparation-count assertion also passed separately. Metadata confirms all
nine packages under `rebuild/` and zero legacy path dependencies. Formatting,
workspace Clippy, fixture manifest and architecture ownership checks are rerun on
the completed tree. Remaining costs: complete canonical SQLite/revision reads,
complete-history metadata/index/digest traversal, full evidence for required Work
histories and managed-store reads; historical coverage lists within a required Work
can still grow quadratically. These are visible residual costs, not a display-bound
claim. No retained content failure remains in the exercised five Work/two Decision
cases. Host interpretation cost remains unisolated as described above.


A separate fresh checksum response was prepared, interpreted by the current host,
recorded and compared through live/offline reads: preparation 9.835 ms, full plan
envelope 10,640 bytes, host evidence-reading/interpretation/authoring wall time
32,335.625 ms, response 1,751 bytes and public recording 21.661 ms (exit 0).
This is one host-phase observation including tool handoff, not isolated inference,
model-token billing or a generation SLA. Read profiles include no generation.

During isolated baseline measurement, sharing `CARGO_TARGET_DIR` overwrote a test
executable: the later current-workload command emitted baseline counter rows.
That apparent current result was rejected. Reconstruction-package build outputs
were cleaned; current tests, binaries, workload profiles and browser proof were
rebuilt/rerun before the performance commit. Future archived baseline runs must use
a separate target directory. The earlier measured current rows above preceded the
baseline build and remain independent, with full source-path compiler output.


The rebuilt current run passed with numeric exit 0, no termination and one ignored
external provider test. Its output contains 501 passing test results including the
nested private-runtime probe. Current workload rows are `WORK_HISTORY_SAMPLE`, not
the rejected baseline rows: pages 0/64/127/128 took 886,737 / 894,092 / 881,231 /
873,454 microseconds with the same 28/28/28/24 materializations. The rebuilt Viewer
medians (total / projection / documents ms) were Overview 40.017 / 9.355 / 0,
Work 38.337 / 8.195 / 0, Decision 40.747 / 9.247 / 0, code 104.125 / 14.255 / 0,
snapshot 154.328 / 15.327 / 28.651. These reruns confirm the same limited conclusion;
thin-route timings remain variable. The rebuilt current browser passed all fourteen
checks, including the timed regenerated checksum answer, without read mutation.
Current workspace Clippy also passed with no compiler warnings.


## CLI ordinary-reading disclosure completion

A final consumer audit found that the generic human Recall renderer still expanded
root Source/snapshot/audit fields beside shared answers. Dedicated ordinary-field
routing for status/Recall/Decisions now uses the same answer payload, separates host
interpretation from independently selected facts/states, quotes only active purpose/
Question/context consumers and directs exact inspection to unchanged `--json`.
Grouping gaps and omission counts remain visible; no hash/content heuristic is used.
Administration and mutation receipts preserve their supported identity output.
A six-command en/ko regression uses retained Work/Decision responses, checks that
answer/state labels are visible and mixed original/root audit identities stay out
of ordinary output; exact JSON provenance is still covered by existing assertions.
Directly affected CLI/local-operations/reading tests, current CLI parity and an actual
retained checksum status/Recall/Decision read are rerun for this correction.

The disclosure review also found supporting-Source availability/freshness visible
only in evidence on a no-realizer Work. Shared Work/Decision answer facts now show
unavailable/unknown availability and stale/unknown freshness independently; a
Current freshness value never makes an unavailable Source usable. Work grounding
includes verification/review/acceptance Sources as well as Goal/Checkpoint Sources.
The existing unavailable-but-current-source regression retains original text and
Work identity, and now checks ordinary bilingual gap facts. This correction is
upstream of all reading consumers, with exact statuses retained in evidence.


Final disclosure validation: affected four-crate all-target/all-feature suite passed
288 tests (exit 0, no termination; external live-provider test remained ignored),
actual six human/JSON CLI reads passed, CLI parity retained ten maintained shapes
and rejected twelve removed shapes, V06 passed, all fourteen actual browser checks
passed, architecture ownership/formatting/workspace Clippy passed. Full preserved
streams and corrected failed probes remain under ignored focused validation paths.
No gate/final, official V11 or naturalistic campaign was invoked.


## Comparable projection/document regression resolution (2026-10-03)

Entry source was `4f40d372f57779b2c89eadc93ebe1afceb5d5be3`, with a clean tree.
The unchanged named WSL2/i7-13700K (16 exposed CPUs), Rust/Cargo 1.97.1 debug
workload reproduced the repeated regression, with no worker-limit override or
concurrent expensive check. Enforced run
`20261002T203007.671732Z-projection-cost-before-enforced-q0zkmygx` failed at
Overview projection 9.825 ms (exit 101). Complete diagnostic run
`20261002T203031.616744Z-projection-cost-before-diagnostic-sjr70upf` retained
all 45 rows (exit 0, timing not enforced): Overview, Work and Decision projection
failed 9/9 each, Snapshot documents failed 9/9, and Code projection failed 2/9.
All route totals and read/decode/document counts passed. This confirms the
100301 repeated finding rather than replacing it with one sample or a gate pass.
All earlier failure evidence remains retained.

Temporary stage instrumentation (removed from production) isolated canonical
Debug equality hashing at approximately 3.6–4.2 ms, and document generation at
11–12 ms versus 29–35 ms including repeated canonical/live-basis inspection.
The first identity-format/empty-invalidation change alone still failed Overview
projection at 6.564 ms. Typed equality alone then passed projection ceilings but
failed Snapshot documents at 26.359 ms. Those enforced failures and the full
profiling streams remain in ignored focused artifacts.

The production changes are field-wise canonical equality hashing, one formatting
write per opaque ID, no canonical Store open when the forgetting journal has no
incomplete operations, and coordinated complete document reads. Derived `Hash`
traverses all `Eq` fields and sequence/enum boundaries into SHA-256 without Debug
escaping; it binds full Source observations, lifecycle, revision, relation,
forgetting and merge history. Its value intentionally changes: this current-build
equality token is neither canonical identity nor a portable fingerprint. The
Work/Decision preparation fingerprint, exact actions and evidence keys do not
change. No persisted cache, additional reader, schema version or provider call
was introduced.

The existing inspection lock spans the initial canonical/projection read through
all four validated document generations. This makes one freshly matched evidence
basis coherent for the entire request, eliminating the second identical SQLite
history read and digest without weakening correction/deletion exclusion. Lock
acquisition remains charged to canonical reads; generation, grounding, language
and output-bound validation remain charged to documents. Health/privacy/HTML
stages occur after releasing the lock. Supplied old projections still use the
independent live-basis validator; publication still revalidates under the mutation
lock. The tradeoff is that complete document read/compute holds the existing
exclusive coordination lock; it runs no analyzer, host generation or provider.

Focused enforced verification
`20261002T204828.039991Z-optimized-cost-final-focused-hrnebqht` passed (exit 0,
no termination/spawn error), with all nine samples per route and every unchanged
stage ceiling/count. Ranges below are all nine samples, not means:

| Stage | Entry range ms | Optimized range ms | Unchanged ceiling ms |
| --- | ---: | ---: | ---: |
| Overview projection | 9.242–10.854 | 3.660–4.616 | 5 |
| Work projection | 8.265–9.492 | 2.802–3.811 | 5 |
| Decision projection | 9.294–10.123 | 3.691–4.435 | 5 |
| Code projection | 14.130–15.624 | 8.654–10.313 | 15 |
| Snapshot projection | 14.950–15.704 | 9.378–11.831 | 18 |
| Snapshot documents | 29.014–31.277 | 10.198–12.319 | 18 |

A separate `git archive` of actual entry HEAD under ignored local state uses a
separate Cargo target directory. Only its cost test's output-identity reporting
and sha2 dev dependency were instrumented; no workload or timing boundary changed.
`archived-cost-output-identities` retains 45 old-source rows and output hashes,
but overlapped the functional Viewer suite and is supplementary identity/count
evidence, not the uncontended timing comparator above. A disposable frozen-read
probe then read the exact same fresh Runtime and 192-module repository through
entry and optimized production paths. All 14 en/ko artifacts (full projection,
all four document bodies/grounding/rendered outputs, and five Viewer surfaces)
were byte-equal after substituting only the intentional current-build equality
token. Raw original/normalized identities and both probe streams remain ignored;
no second production answer reader or maintained snapshot oracle was added.

Focused functional coverage includes required-history selection, all four
documents for en/ko/unavailable fr-CA, latest/exact/unrelated/Repository selectors,
Current/absent/stale/corrupt explanation behavior, correction/forget/deletion,
publication rejection, and full restart negative controls. The document lifecycle
consumer compares coordinated output with independent supplied-projection
validation, while retaining its canonical and generated-binding assertions.
The browser consumer additionally rejects missing/duplicate cold/warm rows,
missing output/basis hashes, every individual count failure and each stage overrun
independently of total latency. Output SHA-256 recording occurs after timing ends.
Initial compile/test instrumentation mistakes and sandbox-blocked listener results
are preserved; corrected Viewer socket/lifecycle support passed with socket access.
Workspace Clippy and architecture-owner routing were clean. These focused results
precede final-candidate install/content/browser/gate/archive evidence, which is
reported separately in the final session handoff. No human qualification or Phase 9
approval follows from these scoped technical observations.
