# V06 — Source-grounded project projections and documents

## Status

Passed against Production Rust commit `5c1b534f`. The maintained V06 matrix maps
34 requirements to the `volicord-projections` Production integration test, and
the focused assertion run passed with four authoritative fixture sources. This
status covers the read-only project surface and document-generation boundary;
it does not claim viewer, publication, adoption, provider, or combined-journey
behavior.

## Goal

Verify that Canonical Context and Repository Intelligence can be projected into
the Phase 7 project read surface and four required documents while keeping
claims grounded, degraded analysis visible, output deterministic, and canonical
and Candidate state unchanged.

## Accepted decisions being validated

Accepted Q5 defines the Project & Architecture Guide, Decision Report,
Implementation Plan, and Handoff / Resume Document; portable Markdown and
self-contained HTML; explicit destination-only repository publication; and the
required Project, snapshot, Decision, capability, gap, generator, and time
metadata. Accepted Q3 and Q9 preserve bounded Recall, independent work,
verification, review, and acceptance meaning, and Candidate Inspection without
granting lifecycle authority. The active Projection and Document owner further
requires structural facts, semantic results, and explicit agent inference to
remain distinguishable.

## Input repositories and revisions

The evaluation runs against Production commit
`5c1b534f5f71db705ae44a62efe6fd81805207eb`. The V06 matrix reuses these
manifested authorities:

- `v02-rust-cargo` at
  `8161bab1034f30b88012a4dcc4ffba5ccfb43c6023561c4d30f51afa6f2fb899`
  for single-language structural and semantic input.
- `v01-polyglot` at
  `21d9b78a1ff12838932aeabeea2b1c12c1a901cf9db238e4dd25584bcff0baff`
  for polyglot coverage.
- `v01-javascript` at
  `8dd3d47acf0f5ddef3d21d33cc7c869d36db8d59983a3a3f6df036b1b62996a4`
  for partial or failed analyzer scope.
- `v09-phase-6-matrix` at
  `f56eb31f1805b1e0cda38f113a6b7adbf964a6808fbb1ca465d576aebc7ec7c6`
  for active and superseded Decisions, the latest Checkpoint, an open Question,
  stale or unavailable basis, bounded Recall, and Candidate Inspection.

The self-authored `v06-source-grounded-documents` requirement matrix has hash
`edcc686c28e9e682d9c335206811ca589989cb49a1b48ddf09839d4bf9981578`.

## Environment and tool versions

The focused evaluation ran on Linux
`6.18.33.2-microsoft-standard-WSL2` x86-64 with `rustc 1.97.1` and
`cargo 1.97.1`. The Production path is in-process and required no browser,
viewer server, external renderer, analyzer executable, or network service.

## Candidate approaches

1. Independent Markdown and HTML template models would make semantic drift and
   missing grounding metadata likely. The selected design renders both formats
   from one typed semantic body.
2. Writing generated documents directly from the projection crate would mix
   read projection with filesystem authority. The selected design returns a
   bounded publication artifact and optional requested destination for a later
   Local Operations caller.
3. A validation-only document engine would duplicate Production meaning. The
   selected V06 orchestration discovers and executes the maintained Production
   Rust integration test and only maps requirements and fixture roles to it.

## Commands and configuration

```text
rebuild/scripts/validate focused v06-fixture-manifest -- rebuild/scripts/check-fixture-manifest rebuild/validation/shared/fixture-manifest.json
rebuild/scripts/validate focused v06-assertions -- rebuild/validation/projections/source-grounded-documents/assertions.py
rebuild/scripts/validate focused v06-assertions-repeat -- rebuild/validation/projections/source-grounded-documents/assertions.py
rebuild/scripts/validate focused v06-report-shape -- rebuild/scripts/check-validation-report rebuild/validation/projections/source-grounded-documents/report.md
```

The assertion requires exactly four evidence groups and four fixture roles,
checks at least 30 unique requirement mappings, discovers the mapped Production
test with `cargo test -- --list`, rejects filesystem-write APIs in the two
projection owners, and then runs the Production integration test.

## Observed results

The assertion reported four fixture sources, four groups, 34 mapped
requirements, and one discovered Production integration test. The test passed
and generated the same project projection and document set on repeated calls
for fixed inputs and time. All four documents exposed grounding metadata and a
shared typed body rendered as portable Markdown and self-contained inline-CSS
HTML. No requested destination was written, and the Canonical Context and
Candidate read bases remained equal before and after projection and generation.

The Production test retained distinct structural, semantic, and agent
interpretation classes; marked agent interpretation as explicit inference;
retained current and superseded Decision states; separated Checkpoint work,
verification, user review, and acceptance; and degraded project health for
partial semantic coverage. English fixed text, Korean fixed text, and a
non-allowlisted `fr-CA` requested-language label were accepted.

## Coverage and failures

Covered input roles are single-language, polyglot, partial or failed analysis,
and canonical/resume evidence with active and superseded Decisions, latest
Checkpoint, open Question, stale or unavailable basis, Candidate Inspection,
and bounded Recall. The mapped Production behavior covers overview, Resume,
Repository Map, Decision–Context–Code links, Checkpoint timeline, canonical and
Candidate inspection, four documents, grounding, metadata, formats, fixed text,
determinism, no mutation, and the publication-artifact boundary.

No final focused V06 command failed. Viewer and HTTP behavior, CLI/MCP/host
wiring, filesystem publication, adoption, provider transport, Guarded effects,
large-repository narrative quality, accessibility, and V11 are excluded rather
than reported as successful.

## Performance and resource observations

The Production integration test completed in approximately 0.12 seconds after
build reuse on small deterministic fixtures. The evidence does not establish
large-repository latency, peak memory, output-size ceilings, or renderer
performance.

## Privacy and external transmission

All fixtures, Canonical Context data, analysis results, and rendering remained
local. The assertion and Production test used no provider, network service,
background process, or external source transmission and created no background
authority.

## Acceptance results

- Pass: the project overview, Repository Map, Decision–Context–Code data,
  Checkpoint timeline, canonical inspection, bounded Recall, and Candidate
  Inspection preserve current canonical and source identities.
- Pass: all four required documents exist with source/Decision/analysis or
  explicit-inference grounding and the required metadata.
- Pass: partial coverage degrades health and remains in capability gaps and
  omissions instead of being described as complete; the V06 matrix retains
  failed, unavailable, unsupported, and stale evidence roles.
- Pass: active and superseded Decisions and independent work, verification,
  review, and acceptance states remain distinguishable.
- Pass: Markdown and self-contained HTML render one semantic body, and English
  and Korean fixed text are available without allowlisting requested languages.
- Pass: repeated projection and generation are deterministic for fixed basis
  and time and do not mutate Canonical Context or Candidate state.
- Pass: no repository write occurs; an explicit destination is returned only as
  a bounded artifact for a later authorized publisher.

## Known limits

The maintained fixtures are small and deterministic. V06 verifies requested
language acceptance and fixed English/Korean product text, not translation or
narrative quality for arbitrary requested languages. It does not measure
large-repository selection quality, accessibility, browser compatibility,
atomic filesystem publication, adoption, or end-to-end handoff comprehension.

## Recommended implementation choice

Retain `volicord-projections` as the single read-side owner: build one bounded,
identity-preserving project projection, derive four typed document bodies from
it, and render Markdown and HTML from the same body. Keep publication and
adoption in later authorized boundaries and keep generated output non-canonical
until explicit review/import.

## Rejected alternatives and reasons

Do not add a second validation renderer because it would test a duplicate rather
than Production behavior. Do not make Markdown and HTML independent semantic
pipelines because equivalence would become incidental. Do not give the
projection crate filesystem, provider, or canonical mutation authority because
those responsibilities belong to Local Operations, provider policy, and
explicit adoption respectively.

## Reusable primitive decision

`production_evidence`. The Rust projection and document implementation is the
Production authority. The Python assertion remains maintained orchestration and
requirement mapping; it is not a candidate domain engine or renderer.

## Decision revisit trigger status

Not triggered. The evidence does not show that the four documents are
structurally unable to express design and handoff meaning or that the
Markdown/HTML boundary is inadequate. Accessibility and real-user handoff
quality remain later evidence, so their absence is not interpreted as success.

## Follow-up work

- Exercise privacy and provider state separately in V07.
- Validate local host and clean-journey wiring separately in V08.
- Evaluate real-repository scale, accessibility, user comprehension, and
  cross-agent handoff quality in V11.
- Add atomic filesystem publication and explicit document adoption only in
  their authorized subsystem boundaries.

## Artifacts

Maintained artifacts are this report, `assertions.py`,
`fixtures/v06-matrix.json`, the V06 shared fixture-manifest entry, the reused
V01/V02/V09 fixtures, and the Production `project_documents` integration test.
Focused runner results remain under ignored `rebuild/.local/validation/`.
