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
/?view=work&work=CANONICAL_GOAL_ID
/?view=code&scope=work&work=CANONICAL_GOAL_ID&entity=ENTITY_ID
/?view=code&scope=repository
/?view=decisions&decision=DECISION_ID
/?view=tools&tool=documents|memory|status|evidence
```

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
limits and next step in ordinary reading. Original result quotations are closed
evidence disclosures. Goal/rationale quotations do not claim translation. Missing Purpose/result/user
rationale remains unavailable. Completed work, verification, user review and
acceptance are independently shown; failed/rejected history remains visible.
Code diagrams show static grounded evidence, not confirmed runtime ordering.
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
feeds `LocalOperations::project_projection_read_profiled`. Overview, Work and
Decisions request neither section, using the existing AnalysisMetadata reader for
snapshot identity, coverage and freshness. Code requests stored graph bodies but
not Candidate inspection; Memory requests inspection but not code. Documents and
whole snapshots request both. Evidence and Status request code; only Status runs
full stored-graph integrity diagnostics. Other pages expose that these diagnostics
were not requested, while store/recovery health and material metadata gaps remain
visible. Missing requested analysis is `Unavailable`; omitted materialization is
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
The four document previews share one final live-basis check; publication still
checks current provenance under Local Operations coordination.

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

## Explicit Work and Decision explanations

From a bound repository, or with explicit `--runtime` and `--project`:

```text
volicord --json work explain prepare --work GOAL_ID --language ko
volicord --json work explain record --work GOAL_ID --language ko --input /absolute/host-response.json
volicord --json work explain delete --work GOAL_ID
```

Preparation returns `{operation, plan}`. The current active agent reads the entire
plan under its existing source-access authority and creates a response JSON using
`format_kind: "volicord_explanation"`, `format_version: 1`, the exact
`plan_fingerprint`, `language`, `generator: {host, session, agent, model}` and
`paragraphs: [{question, text, evidence_keys}]`. Each of `purpose`, `reported_change`,
`expected_effect`, `verification`, `next_step` requires one paragraph. `limits` is
optional. Use prepared evidence keys; unknown agent/model is null, and unavailable
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
of `relay`, `relay_variant`, `export`, `older`, `checksum`, and each language `en`, `ko`, run
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
the browser checks these four recordings alongside the ten Work recordings.

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
