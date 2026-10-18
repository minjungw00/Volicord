Current reader contract: [D-01–D-18 / AC-01–AC-16](../../docs/design/reader-experience-design.md).
This supersedes historical two-menu, neutral-normal-card and single observation-date
guidance below. New output is unassessed until reviewed; previous delivered UI was rejected.

# Volicord local viewer

This crate is the loopback-only local Project viewer. It renders existing
projection, privacy, health, document, and Guarded request data and delegates
all writes to `volicord-operations`. It owns no domain state or database.

Run it with an explicit Project and optional separate runtime:

```text
volicord-viewer --runtime /absolute/runtime --project PROJECT_ID
```

Export the current Viewer projection as one self-contained, read-only local
HTML snapshot and exit:

```text
volicord-viewer --runtime /absolute/runtime --project PROJECT_ID --snapshot /absolute/path/viewer.html
```

Snapshot publication is atomic, refuses a relative or existing destination,
opens no listener, and performs no upload or other network transmission. The
result keeps current degradation, privacy state, document previews, and a
closed Project/canonical/repository-analysis basis disclosure, but contains no
forms, authenticity token, mutation endpoint, script, or live-Viewer link. It
remains readable after the Runtime is no longer available. Sharing the file is
a separate user-controlled action outside this command.

Fixed product text is bundled in English and Korean. `--language` records an
arbitrary requested generated-content language without an allowlist. The Viewer
shows current recorded Work interpretations only for that exact language. Explicit
active-host preparation and recording happen through Operations; navigation never
invokes a host or provider. Missing generation, stale evidence and unavailable
dependencies have separate notices. Fixed English text is never presented as
requested-language generation success.

Live reads use purpose-oriented views (default `overview`):

```text
/?view=overview
/?view=repository
/?view=work&work=CANONICAL_GOAL_ID
/?view=code&scope=work&work=CANONICAL_GOAL_ID&entity=ENTITY_ID
/?view=code&scope=repository
/?view=decisions&decision=DECISION_ID
/?view=tools&tool=documents|memory|status|evidence
```

The brand sits left of one right-aligned navigation containing Overview,
Repository explanation, Work and Tools. Tools includes Decision inspection,
Analysis/privacy, Documents, Memory and Evidence, preserving exact locale/language
and protected actions. Read-only status sits outside navigation and is scoped to
the actual reading or offline surface.

Overview shows PROJECT OVERVIEW, the actual Project name, canonical Purpose or
short absence, Recent Work and All Work. It contains no duplicate repository
summary, RECENT WORK eyebrow or Project Decision list. Repository reading works
with or without Work. Cards require an explicitly host-authored name and meaningful
problem/actual-approach/outcome synopsis for normal-content completion. Missing
interpretation is incomplete and links to permitted original instructions; the
renderer never copies a Goal into a summary. Cards and detail share exact Work,
realization and event bases: original Goal creation and latest same-Work Checkpoint
recording, in UTC with inspectable record/revision. Missing events stay absent.

A selected Work is a blue-gray editorial document with breadcrumb, authored
heading/lead, recorded state, supported narrative and exact limits. Related
Decisions retain their explicit/applied Work relation. Original Goal, independent
verification/review/acceptance, directions and complete history remain inspectable
through native record disclosures. Ordinary reads never generate.

Each citation opens its exact bound claim and observed Source in a side panel on
wide screens and a full viewport panel on narrow screens. Original Source bytes
and observed line coordinates remain unchanged; no diff or execution is inferred.
The return link restores the exact cited paragraph. Native URL fragments power
both live and script-free offline journeys. Live selection checks current retained
bytes through `/citation`, bound to exact subject/realization/unit/citation identity.
It displays at most 4,096 UTF-8 bytes per page, within a 32,768-byte JSON response,
and offers the next portion of that same cited range. Unavailability clears the
point's code and preserves its return link. Offline documents include no such
endpoint or controls and explicitly disclose excluded evidence. The live presentation script adds
Escape, scroll/focus restoration, linked disclosure opening and narrow-panel
keyboard containment with inert background. HTTP grants only its exact SHA-256
script digest. Only exact same-origin selected-citation reads are requested; generation, persistence
and Runtime mutation do not occur.
Snapshots select one screen at a time through local fragments, keeping the same
Overview, index and document components. Original/evidence records are open in
snapshot documents so they remain reachable without scripts; all retained support
information remains in the file. Without CSS `:has`, all content is inline.

Code identifies the selected Work, repository or exact-entity read basis and keeps
a concise reliability summary beside the ordinary explanation. The summary counts
selected issues and capability gaps, distinguishes capability state from freshness,
and keeps Source/analysis blockers visible with an Analysis link. Supported Work and
source-bound behavior meaning precedes the closed **Full Code diagnostics** native
disclosure. Expanding it retains every selected issue and gap in producer order,
with identities, original scope, freshness, reason, consequence and usable remainder.
Root `.` reports have a readable repository-coverage label while their exact source
scope stays inspectable. A report's affected-area metadata can describe its wider
repository coverage; its inclusion is determined by the selected read basis.
Unavailable code meaning remains explicit. These same components serve live and
self-contained snapshot reads in English and Korean. Overview, Work and Decision
contextual warnings retain their existing ordinary presentation.

Code then presents supported structure and directed relationships.
Containment/dependency/reference/type relations are labeled separately
from static syntactic calls; neither proves runtime or data flow. A scope with no
relations shows components and an analysis action without an invented diagram.
Analysis shows the shared stored-analysis availability and independent freshness,
observed scope coverage, failed attempts and retained prior results. Snapshot identities,
exact counts, analyzers and Runtime diagnostics are closed disclosures. Refresh remains
explicit CLI guidance (`volicord analyze`, or the supplied repair command) in this
Project's bound repository and Runtime. The Viewer has no analyzer POST action and
never analyzes on navigation; refreshed analysis does not regenerate interpretations.

All links carry `locale=en|ko` and unrestricted requested `language`. Work and
Decision navigation supplies validated identities; no opaque-ID entry is needed.
The Work and Decisions root views list recorded choices in pages of 64;
`page=N` is a nonnegative decimal page index, accepted only on those lists.
Previous/next links preserve locale and language. Exact detail reads resolve
before display bounds. Malformed or mismatched selectors return 400; an identity
absent from this Project returns 404. Valid Work with no code remains readable
with a scoped gap. When stored analysis is unavailable, entity detail is explicitly
unverifiable; the page preserves canonical remainder without claiming that entity
exists or is absent. Removed `level` forms are usage errors/400.

`volicord viewer open --view overview` forwards to the installed sibling
`volicord-viewer`; direct launch also accepts `--view`. Named `code` launch opens
repository scope and `tools` opens health/privacy. Whole-snapshot export accepts
no `--view` or `--level`; it includes bounded Overview, Work, repository Code,
Decisions, health/privacy, and four document previews on one read basis. Internal
fragment links refer only to included sections; omitted Work-specific code detail
is explicit. Snapshot disclosure retains original audit text, not redacted text.

Memory correction, supersession and forgetting, explicit document export, and
exact Guarded confirmation use the existing Local Operations authority. Forms
return to parsed allowlisted views. Host, Origin, Fetch Metadata, authenticity,
CSP, exact target/revision and retry boundaries apply to every mutation. Ordinary
navigation performs no analysis, provider request or canonical mutation. No
arbitrary filesystem-path source endpoint is provided.

Work interpretations answer purpose, reported change, expected effect, verification
limits and next-step uncertainty in the document narrative. Original result
quotations and recorded directions remain in full evidence. Goal/rationale quotations do not claim translation. Missing Purpose/result/user
rationale remains unavailable. Completed work, verification, user review and
acceptance remain independent facts in full evidence, including failed/rejected history.
Code diagrams show static grounded evidence, not confirmed runtime ordering.
Grounded components without resolved relationships remain inspectable as
disconnected architecture nodes with an explicit no-edge gap. They do not populate
the static call figure; only exact syntactic-call relations support that figure.
The same entities/relations have a keyboard/touch-readable list and disclosure
path. Only diagrams scroll horizontally; ordinary prose/evidence wraps.

Client disconnects and response-write failures end only the affected connection;
the listener continues serving subsequent requests without retrying a mutation.


Code exploration keeps SCC cycles together and derives edge direction only from
stored relations. Symbols lead diagram labels; shortest distinguishing path suffixes
and real range/kind distinguish duplicate names. Sized nodes and separate edge ports
retain self-loops/parallel edges. SVG links jump to full entity names and locators in
the equivalent native-details list, so full labels are available on keyboard/touch
without hover. Relationship lists preserve every constituent identity and count;
there is no relation grouping. Resolved endpoints have explicit repository-detail
links; unresolved endpoints never become nodes. Source inspection discloses existing
snapshot-bound Source/range data and performs no filesystem-path HTTP read.

Read requirements are explicit: `ProjectionReadRequirements { code, inspection }`
feeds `LocalOperations::project_projection_read_profiled`. Overview and Work request Candidate Inspection for permitted Learning associations;
Decisions request neither section. These views use the existing AnalysisMetadata
reader for snapshot identity, coverage and freshness. Code requests stored graph bodies but
not Candidate inspection; Memory requests inspection but not code. Documents and
whole snapshots request both. Evidence and Status request code; only Status runs
full stored-graph integrity diagnostics. Unrequested graph materialization is an
internal read intention and supplies no user warning. Canonical/Runtime blockers
remain visible above ordinary reading; selected Source/analysis limitations sit next
to the affected answer. Repository-wide diagnostics, analyzer identities and exact
omissions are available in closed Analysis/Evidence and offline disclosures. Missing requested analysis is `Unavailable`; omitted materialization is
`NotRequested`. Document generation and narrative planning reject incomplete
requested-section input. Default Operations Recall/documents keep their full
contracts. Work interpretations use the existing Privacy managed CachedSummary
store with explicit local recording, read-time evidence freshness and deletion/forget
integration, described below. They add no Viewer database or canonical records.

Work classification uses request-local references over complete canonical history.
Detailed evidence is created only for the union of page, Overview, selected and
Recall Work identities. Overview totals are independent of paging; selected Works
retain their full history. Profiles separate classified/indexed history from
materialized Work/Checkpoint input bytes and unique retained explanation freshness
preparations. Store reads and metadata indexes still scale with complete history;
these counters do not measure allocation, RSS or model inference. Explanation
freshness results are reused within one read, with no additional persisted cache.
The four document previews use `LocalOperations::project_documents_read_profiled`:
the existing inspection lock spans the initial complete projection and validated
generation, keeping canonical and freshly matched explanation inputs coherent.
Independent `documents_from_projection` calls still revalidate supplied inputs;
publication checks current provenance under Local Operations coordination. The
lock is released before Viewer health/privacy/render stages. No request receipt
or content is reused by another request. Profiles charge lock acquisition to
canonical reads and all document generation/validation to the document stage.
Canonical equality binds every Eq field through derived Hash and SHA-256 without
Debug formatting; its value is internal to the current build, not a portable ID.

`ViewerRenderProfile` exposes one projection pass, standalone metadata reads,
graph decode attempts (including failed attempts), Candidate basis reads, document
generation count and monotonic stage times. `render_snapshot_profiled` reports the
same one-basis snapshot path. Named fixture-specific timing budgets and reproduction
commands are in [Viewer findings](../../validation/end-to-end/multi-repository/viewer-reading-findings.md)
and [read budgets](../../validation/end-to-end/multi-repository/viewer-read-budgets.json).
They do not establish a universal latency guarantee or browser qualification.

The maintained nested-workspace installer places all three executables together:

```text
rebuild/install.sh --prefix /absolute/install-prefix --runtime-dir /absolute/runtime
```

From the bound repository, with `/absolute/install-prefix/bin` on PATH:

```text
VOLICORD_RUNTIME_DIR=/absolute/runtime volicord viewer open --view overview --language en
VOLICORD_RUNTIME_DIR=/absolute/runtime volicord viewer export --output /absolute/path/viewer.html --language en
```

Use a fresh absolute output destination. An explicit `--project PROJECT_ID` can
replace repository resolution. For development, build with `cargo build
--manifest-path rebuild/Cargo.toml -p volicord-operations -p volicord-viewer` and use
the sibling executables under `rebuild/target/debug`. Launch selects a view;
navigation supplies exact Work/Decision/entity identities. Export always renders
all bounded snapshot sections, regardless of the live entry view.

Selecting an entity focuses both diagrams and the equivalent entity/relation list
on that exact entity's bounded stored incoming/outgoing neighborhood, even when
it was omitted from the initial map. The selected node is retained and marked.
The Work/repository scope is still explicit; selecting a real one-hop neighbor
in Work scope does not assert canonical Work ownership of that neighbor. Resolved
relations retain both actual endpoints, unresolved targets remain evidence only,
and omitted relationships keep exact counts. A generic repository explanation
uses stored topology without claiming a canonical Work-seed link.


Repeatable browser supporting checks and tool prerequisites are maintained in the
[validation README](../../validation/README.md#browser-support-for-viewer-reading-v11-owner).
They exercise the actual CLI/server/export path, both locales, narrow viewports,
keyboard navigation, closed offline snapshots and real 200% tab zoom. Browser
tools are validation dependencies only. Automated observations do not establish
human comprehension or replace the authoritative technical gate.
The shared authored Code diagnostic fixture supplies thirty unavailable Sources
and fifteen polyglot capability gaps. Direct producer-to-Viewer tests bind selected
behavior, exact diagnostic counts/order and snapshot parity. Browser checks exercise
repository, Work and exact-callable reads at all maintained widths and native zoom,
assert initial DOM order and geometry, operate the disclosure with Tab/Enter/Space,
and reject expanded-by-default diagnostics, removed detail and hidden material warnings.
Long bilingual diagnostic stress is restored before candidate capture. No budget is
increased; optional named timing enforcement remains a separate diagnostic.

## Presentation and accessibility

`render.rs` coordinates projection reads and authority. `reading.rs` composes
purpose views from those projections; `presentation.rs` owns escaped HTML
primitives and embeds `presentation.css` into both live and offline output.
There is one server-rendered frontend with a bounded presentation script and no
remote assets or client framework.
Stable `data-work-id`, `data-decision-id`, `data-entity-id`, `data-question`,
statement roles and exact fragment targets remain the inspection selectors.

The selected reading design governs the masthead, Overview, Work index, documents,
record boxes and claim-specific Source points described above. There is no side rail,
secondary masthead menu, category-jump navigation or expanded audit content inside
Work cards. The main landmark has a keyboard skip link. Full evidence preserves
localized state observations before exact Source/revision detail, and distinguishes
Decision and Learning authority. Code inspection keeps its directed topology and
an equivalent native list/disclosure path without arbitrary filesystem access.

Requested explanation language remains unrestricted and independent of the fixed
English/Korean UI locale. Explicit CLI/query parameters preserve that selection;
ordinary navigation never generates or translates content. Generated paragraphs
carry their declared language; original quotations keep an unknown language hint.
Offline documents contain no forms or scripts. All included navigation resolves to
local fragments, and exact original/evidence records remain accessible.

`grounded_reader_browser.py` checks actual Product Overview/index/document screens
at 390/768/1440px in en/ko, including authored heading/synopsis/comparison controls,
exact Source bytes and claims, keyboard return/Escape/code scrolling, distinct Source
failures, unchanged retained state and offline reading with Runtime unavailable.
Its process group owns and stops both the CLI and Viewer child. These observations
are technical support; native zoom, actual screen-reader use, personal comprehension
and final visual acceptance remain separate. The visual reference is the selected
local `viewer-evidence-border/review/viewer` design. Prototype explanatory prose,
recorded statuses, observations and Source coordinates are not Product inputs.

## Explicit Work and Decision explanations

For an explicit user request in the active host, MCP `work_explanation` action
`generate` prepares exact Work facts and a separate scoped Source inventory.
The host investigates permitted relevant code, retrieves selected retained Sources
with `read_source`, authors natural prose with exact source/claim bindings and
uses `record`. Its initial awaiting-host status is not completed generation, and
the user does not construct JSON. Source links navigate to the actual cited code
or command output. Each citation opens its exact bound claim as host interpretation beside the code.
The Source point retains that claim's complete citation, Source revisions and hashes;
other paragraphs using the same range get their own points and exact return links.
Recorded verification, direction and chronology stay inside document evidence.
Expired/denied/changed dependencies withhold generated prose and code, with the
actual diagnostic visible on the selected document. Code regions accept keyboard
focus. Explicitly paired observations link to a retained cited before range without
inventing a diff; purpose labels and current/historical/command-use notices retain
their separate authority. Comparison units use the reference observation cards;
no structured result table is inferred from unstructured explanation wording.
The same bounded excerpt basis serves CLI/MCP, live Viewer and offline snapshots.
Viewer headings and the optional API `reading_title` select only the authored Heading
unit; synopsis has its own role. They are grounded derived wording, not canonical
short-title authority. The complete Goal stays accessible at the opening. Every
citation selects its own visible claim beside the exact Source range, with a native
return link to that unit. Shared code ranges retain separate claims/return points;
realization-bound fragments and browser history preserve context. Current Work and
Code use the same prose renderer, including observations/comparisons and guidance.
Self-contained snapshots share that document and retain exact internal destinations.
Before-state needs an explicit observed pair; current code
alone does not prove a patch, execution success or user approval. Missing host
capability/authority stays incomplete with recorded facts readable. See the
[Host procedure](../volicord-host/README.md) and `tools/list` schema.

The shared excerpt bound is 16,384 UTF-8 bytes, at most 4,096 per cited range,
with exact displayed bounds and omission counts. Changed observations show actual
selected before/after ranges rather than a synthesized diff. Identical paired
bytes and unavailable before-state have distinct notices. An earlier dependency
whose bytes were not cited is identified without manufacturing before code.

Missing, deleted and canonical-forgetting withheld explanations have distinct
read notices. Generation/record failures preserve the prior permitted output.
Readers and publication select the latest local insertion even when recording
timestamps tie; original observation times remain inspectable and unchanged.

From a bound repository, or with explicit `--runtime` and `--project`:

```text
volicord --json work explain prepare --work GOAL_ID --language ko
volicord --json work explain record --work GOAL_ID --language ko --input /absolute/host-response.json
volicord --json work explain delete --work GOAL_ID
```

Preparation returns `{operation, plan}`. The current active agent reads the entire
plan under its existing source-access authority and creates a response JSON using
`format_kind: "volicord_explanation"`, `format_version: 4`, the exact
`plan_fingerprint`, `language`, `generator: {host, session, agent, model}` and
`paragraphs: [{question, text, evidence_keys, citations}]`. The small reader vocabulary adds authored `heading` and `synopsis`, `observation`,
`comparison`, `limits` and claim-linked `reading_point` to the supported narrative
roles. Omit unavailable answers instead of filler. All units share grounding and
lifecycle validation; observations/comparisons/reading points require citations.
Heading and synopsis are optional singletons; consumers never synthesize them from
the first paragraph. Use prepared evidence keys; unknown agent/model is null, and unavailable
session correlation can be explicitly self-reported as unknown. Do not claim
independently verified identity. The plan supplies evidence and instructions, never
a prewritten answer. Record validates references and current basis, not prose truth.

The selected mechanism is active-host interpretation of full canonical material,
with deterministic selection and state facts. First-384-character quotations failed
the audit-heavy fixtures; feature purpose/effect are not separately structured facts.
There are no production fixture recognizers, hardcoded explanations or background
generation. The generic “The implementation changed” regression supports only that
limited report. A useful interpretation must say the feature/effect is unspecified.

Stored content identifies Project/Work, question, exact record revisions/fields,
Sources/snapshots/status, language, recording time and uncertain generator provenance.
Changes, corrections, conflicts or Source-status changes invalidate the fingerprint.
Stale/corrupt/non-current content is withheld. Explicit delete removes all retained
languages/history for the Work through managed cleanup; canonical forgetting removes
linked content and applies the existing read barrier. Cache deletion leaves canonical
context intact. GET, navigation and snapshot export never generate or transmit.

The shared `ExplanationSubject::{Work, Decision}` API replaces the Work-only Rust
DTOs and reader. Decision preparation/recording/deletion use the same format, validator
and managed store:

```text
volicord --json decision explain prepare --decision DECISION_ID --language ko
volicord --json decision explain record --decision DECISION_ID --language ko --input /absolute/response.json
volicord --json decision explain delete --decision DECISION_ID
```

Decision paragraphs are `user_rationale`, `recommendation`, `consequences`,
`applicability`; optional `limits` qualifies both subjects. CLI status/Recall/decisions
accept `--language`; fixed labels follow `--locale`. MCP Recall and repository_understanding
accept `requested_language` and `fixed_locale`. JSON exposes shared `answers` with
explicit `evidence`. Four document kinds use the same semantics and disclosure roles.
Publication rechecks current canonical and explanation basis; render again after
correction/forget/deletion. Already exported offline copies cannot be retracted.

Explanation preparation is limited to 131,072 compact JSON bytes, including the
fingerprint and mandatory `retention_budget`. It reserves required retained metadata
(including the longest recording timestamp) plus a guaranteed 16,384-byte response.
Compact realizations may use up to 32,768 bytes when the preparation's exact
`response_byte_capacity` permits it; individual paragraphs remain limited to
16,384 characters. The complete retained body and its shared decoder admit at most
147,456 bytes. JSON escaping and generator metadata count; UTF-8 characters are not
single bytes. A CLI/Dogfood response file may contain 65,536 raw bytes so formatting
need not reduce semantic capacity. Purpose and retention basis remain short fields
bounded at 16,384 bytes. See the [owning byte contract](../../docs/design/projections-and-documents.md#explanation-byte-contract).
Oversized responses exit 1 with the failing dimension, sizes and retry action.
Unrepresentable metadata is rejected at preparation before host generation; preserve
grounding and seek Product support. Current plans must be prepared again after this
contract change. Existing bounded MCP/Recall omissions remain explicit; full local
status/decisions and Viewer disclosures provide the complete retained evidence.
`work_explanation::large_retained_envelopes_reach_viewer_documents_and_offline_snapshot`
checks authored metadata-heavy inputs, near-limit readback, all documents and actual
snapshot publication, without claiming host generation or browser/human comprehension.

## Reproduce the narrow actual-host reading proof

Use a fresh ignored output directory; the seed creates canonical inputs only and
asserts that no Work interpretation is retained. All commands are run from repository
root. Wrap validation in `rebuild/scripts/validate focused LABEL -- COMMAND`.

```bash
cargo build --manifest-path rebuild/Cargo.toml -p volicord-operations -p volicord-viewer
VOLICORD_EXPLANATION_FIXTURE_ROOT="$PWD/rebuild/.local/work-proof-fresh" \
  cargo test --manifest-path rebuild/Cargo.toml -p volicord-viewer \
  --test work_explanation seed_work_explanation_runtime -- --exact
```

Read `work-proof-fresh/fixture.json` for `runtime`, `project` and `goals`. For each
of `relay`, `relay_variant`, `export`, `older`, `checksum`, `queue`, `unspecified`, and each language `en`, `ko`, run
the actual sibling executable, substituting the manifest values:

```text
rebuild/target/debug/volicord --runtime RUNTIME --project PROJECT --json work explain prepare --work GOAL --language LANGUAGE
rebuild/target/debug/volicord --runtime RUNTIME --project PROJECT --json work explain record --work GOAL --language LANGUAGE --input RESPONSE_FILE
```

Have the authorized current active agent interpret each preparation and write its
response in ignored local output before recording. Unit-test fake responses or
copying expected claims into a fixture do not establish explanation capability.
The independent cases and browser claim requirements live in
`rebuild/validation/end-to-end/multi-repository/fixtures/viewer-reading/answer-cases.json`.
New source-rich input is synthetic and distinct from the preserved generic case.
The manifest also contains `decisions`: generate `project` (audit-heavy offline reason)
and `explicit` (missing user reason) in en/ko through `decision explain prepare/record`.
Independent requirements are in `decision_cases` and `decision_browser_claim_terms`;
the browser checks these four recordings alongside the fourteen Work recordings.

Use the browser dependencies documented in the validation README and a Korean-capable
font (`fc-list :lang=ko` must be nonempty). Then run:

```bash
rebuild/scripts/validate focused work-explanation-browser -- \
  python3 rebuild/validation/end-to-end/multi-repository/work_explanation_browser.py \
  --fixture rebuild/.local/work-proof-fresh/fixture.json \
  --chromium /absolute/chromium/chrome \
  --playwright-module /absolute/node_modules/playwright-core \
  --library-path /absolute/browser-libraries
```

Omit `--library-path` when system libraries suffice. `FONTCONFIG_FILE` can point to
a local font configuration without system changes. The runner checks actual loopback
GET pages at 390×900, required ordinary five-question answers in both languages,
closed audit evidence, inspectable self-reported provenance, unchanged canonical
export and unchanged provider/managed-record counts. It retains screenshots, font
hashes, full streams and numeric exits under ignored validation output. Inspect the
Korean screenshot as well as DOM assertions. This proves the narrow implemented
path; it establishes no human comprehension, official V11 or cutover qualification.


Human CLI `status`, `recall` and `decisions` show the same shared question answers,
with separate host-interpretation and recorded-fact/state labels. Root exact
binding, source/snapshot and generator audit is available through `--json`;
ordinary output preserves grouping gaps and omission counts. Checksum subject
matter is unchanged. Administration/mutation command receipts retain their existing
identity output.

Full Work evidence shows independent current verification, review and acceptance
facts, prior-adversity summaries, and complete localized chronological observations
with exact times, revisions and Sources. Compact cards retain only their reading
heading/synopsis, lifecycle badge, separate creation/latest-record event bases and action. Current and historical
facts are not conflated. `work_history` compares the complete read model to canonical
input and challenges missing/duplicated current facts and deleted audit observations.
CLI/MCP retain their complete evidence arrays; no canonical state machine changes.

For final candidate evidence, first commit the candidate and install it with the
maintained installer into an ignored absolute prefix. Seed a fresh canonical fixture,
prepare each subject/language with that installed CLI, and have the active host read
the source plans and record its responses. Then add these options to the narrow
browser command above:

```text
--bin-dir /absolute/install-prefix/bin --require-clean
--lifecycle-response /absolute/fresh-fixture/response-lifecycle-relay-en.json
```

For the lifecycle response, export canonical context, copy that bundle to an ignored
path, import the copy into a fresh Runtime, bind the same repository, and prepare
`relay/en` with that Runtime. The active host reads this imported plan and authors
its response, including any changed Source-availability limits. Import can change
the fingerprint; the verifier rejects an original-runtime response with a different
basis. It reproduces the import in another disposable Runtime, records that supplied
response, checks partial/revision-mismatched rejection, correction-induced staleness,
unchanged interpretation after punctuation-only correction, forgetting and new snapshot
export through the public CLI. It never authors feature prose. Raw Runtime copies are
unsuitable for forgetting controls because they retain managed bundle destinations and
can scrub original exports. Canonical-before/after and provider/managed counts on the
original Runtime must remain identical. The supported expression-only replay is lifecycle
support; it does not establish a second independent semantic generation.

Both browser runners accept `--bin-dir`; final runs use `--require-clean` and an
immutable installation so a concurrent Cargo relink cannot substitute executable bytes.
`content-summary.json` and `browser-summary.json` remain separate bounded review
artifacts beside detailed local streams/screenshots. They are not gate capsules,
Naturalistic MCP memory observations or human qualification. The general runner also
exports and browser-reads Goal-only/change/null-verification/later-change prefixes in
both locales; the grounded Operations test independently exercises supported authoring
and whitespace rejection. Source conflict, deletion and format failures retain the
separate Rust lifecycle coverage.

## Capturing the displayed live state

Live responses include a non-executable observation meta entry identifying the
actual executable/process, Runtime/Project/view/selected subject, locale/language,
canonical/Source/Analysis and explanation realization basis. Offline snapshots
keep their deterministic contract without this live context. Executable hashing
is primed once at adapter construction, outside request-local render profiles;
constructor startup and full snapshot export cost remain separate measurements.

The [maintained capture and human-context procedure](../../validation/dogfood/viewer-observation.md)
uses the existing browser driver to attach to one already displayed local tab.
It retains screenshot/DOM/context hashes and leaves the browser usable after
attachment. Capture both locales and preserve separate directories before and
after generation/mutation. Browser artifacts establish displayed state only;
human inspection, 200% zoom declarations and experience remain separate. Actual
native zoom support uses the existing tab-zoom extension, never viewport emulation.


Work detail places related Learning in an explicit Candidate Inspection section
beside canonical Decision history. Overview links to that exact Work inspection.
The retained learning problem, alternatives, response and feedback keep Session
Candidate meaning; withheld, expired or deleted content is not reconstructed.
Decision readings show their retained exact originating Question context separately
from user choice and agent recommendation. Ordinary reading invokes no explanation
generation or provider.

## Repository explanation without Work

An explicitly selected repository explanation needs no Goal or Checkpoint. It is
host-authored Derived interpretation, separate from a user-recorded Project Purpose.
Use a fresh local Runtime Home or an already bound Project. The active host performs
these commands under its current repository access authority:

```sh
volicord --runtime /absolute/local-runtime --repository /absolute/repository --json init "Repository reading"
volicord --runtime /absolute/local-runtime --repository /absolute/repository --json analyze --exclude path/to/reference
volicord --runtime /absolute/local-runtime --repository /absolute/repository --json repository explain prepare --analysis ANALYSIS_ID --language en
volicord --runtime /absolute/local-runtime --repository /absolute/repository --json repository source observe --analysis ANALYSIS_ID --path src/main.py --retain-seconds 3600
volicord --runtime /absolute/local-runtime --repository /absolute/repository --json repository source read --analysis ANALYSIS_ID --source SOURCE_ID
```

Preparation binds the exact Project, local binding, Repository/Analysis Snapshot,
observed revision, complete inventory scope and capability/coverage/freshness.
`repository explain evidence --analysis ANALYSIS_ID` reads the full stored
metadata and diagnostics separately. Preparation carries exact diagnostic counts
and hashes and complete coverage path sets without repeating megabytes of IDs.
Partial structural or semantic analysis still permits scoped included-file
investigation. It is not complete semantic understanding. Excluded, ignored,
symlink, foreign-snapshot or changed files cannot be observed. Large files need
explicit absolute UTF-8 `--start-byte N --end-byte N` selection; the copy is at
most 16,384 bytes and the full-file verification scope is at most 16 MiB.

The host reads the returned Source body and authors a draft, rather than asking the
user to build JSON. The ephemeral draft has `plan_fingerprint`, `language`,
self-reported `generator: {host, session, agent, model}`, and `paragraphs` with
`question`, `text`, `evidence_keys`, and `sources`. A source selection contains
`source_id`, absolute `start_byte`, `end_byte` and nullable `claim` (an optional
`{start_byte, end_byte}` range in the authored unit; null binds the entire unit).
Use the plan's exact keys and reader roles. Unknown agent/model stays null.
Describe purpose, responsibilities, a complete representative mechanism and
material uncertainty; distinguish source-recorded reasons from inferred reasons.

```sh
volicord --json repository explain compose --analysis ANALYSIS_ID --input /absolute/host-draft.json
# Save the returned realization object as /absolute/host-realization.json.
volicord --json repository explain record --analysis ANALYSIS_ID --language en --input /absolute/host-realization.json
volicord repository explain read --language en
volicord viewer open --view repository --language en
volicord viewer export --output /absolute/repository-reading.html --language en
```

Use the same explicit runtime/repository options on every command, or run from its
bound repository with `VOLICORD_RUNTIME_DIR` set. Composition derives exact Source
identities, coordinates and hashes without retaining a document; recording is a
separate explicit operation. Neither operation authenticates host authorship or
certifies the prose's meaning. The same current content reaches ordinary CLI
`status`, MCP `repository_understanding`, the independent repository document
(linked from Work-free Overview) and script-free offline snapshots. All Source links retain exact
claim navigation, scope metadata and generator provenance. Snapshot export performs
no host generation and renews no retention.

`repository source delete --analysis ANALYSIS_ID --source SOURCE_ID` removes one
copy; dependent generated fields become unavailable. `repository explain delete
--analysis ANALYSIS_ID` deletes every language/history for that subject. Explicit
fresh observation/preparation/composition/recording is recovery after changed,
expired, deleted, withheld, corrupt or unsupported evidence. Reads never refill a
copy from current files. Current explanation format is 4; non-current content is
Unsupported, with no historical conversion or compatibility decoder.

Focused public contract control (authored support, no model/user assessment):

```sh
rebuild/scripts/validate focused repository-reading -- python3 rebuild/validation/end-to-end/multi-repository/repository_reading.py --bin-dir rebuild/target/debug --output rebuild/.local/repository-reading/control
```

For a repository document through MCP, call `repository_understanding` with
`reading_focus: "repository_explanation"`, exact `project_id` and
`requested_language`. This reads the same retained document and complete grounding
without also serializing the entire topology/Work overview. Default `overview`
remains the full understanding request. The focused response uses the existing
256 KiB complete-result admission; oversize rejects rather than truncating
grounding. It performs no generation, Source observation or retention renewal.
