# V07 — Project-scoped privacy and local-only mode

## Status

Passed against Production Rust commit
`4ef58cb1488e69030f4e31c2ce2f41096d053274`. The maintained V07 matrix maps
43 requirements to 12 Production integration targets and discovered 62 tests;
every mapped target passed. This status closes the Project-scoped
local/interactive/background authority boundary, Candidate privacy integration,
managed Derived deletion, and local-only degradation behavior. It does not
claim a commercial provider, network transport, perfect secret detection,
provider-side deletion completeness, Guarded dispatch, host UI, V08, or V11.

## Goal

Verify that Volicord remains useful without an external semantic provider and
that any optional background provider processing requires current explicit
Project opt-in, cannot exceed its scope, records what was actually transmitted,
and cannot overwrite or delete user-owned canonical meaning. Verify the
existing Candidate collection, retention, promotion, dismissal, expiry, and
inspection lifecycle without replacing it with provider state.

## Accepted decisions being validated

Accepted Q3 requires local Canonical Context and structural processing,
first-class provider-independent operation, separate current-host interactive
access, default-off Project-scoped background transmission, inspectable
provider/model/scope/exclusion/filtering state, raw-source exclusion from
portable context, and deletable semantic annotations and caches. Accepted Q10
keeps Candidate identity, collection opt-out, retention, inspection, and
promotion disposition separate from canonical records. Accepted Q7 preserves
correction, supersession, forgetting, and related managed-deletion meaning.
The active Privacy and Provider Boundary additionally requires request-time
authorization, actual Source manifests, revoke, provider degradation, and
separate local/provider deletion truth.

## Input repositories and revisions

Production behavior is fixed at
`4ef58cb1488e69030f4e31c2ce2f41096d053274`. The self-authored
`v07-privacy-boundary` fixture has directory hash
`f5b234f6776f89f1afda91fa1f0d6c36cd5a870f32b6c5dcfa618b683759f299`.
It contains an included `src/lib.rs` with a deliberately fake secret-like line,
an excluded `src/vendor/generated.rs`, and an inventory-visible
`docs/readme.md` outside the requested provider scope. The V07 matrix file has
SHA-256 `52357d5f87e7642aa23f6b4fa83e5b01f455984f95fb7e6df20064ada1230416`.

The matrix also reuses `v01-rust` for local structural/ecosystem capability,
`v09-phase-6-matrix` for Candidate, Inquiry, Decision, Checkpoint, Recall, and
inspection behavior, and `v06-source-grounded-documents` for provider-independent
Project projections and documents.

## Environment and tool versions

The evaluation ran on Linux
`6.18.33.2-microsoft-standard-WSL2` x86_64 with `rustc 1.97.1` and
`cargo 1.97.1`. The provider oracle was a deterministic in-process adapter; no
credential store, remote endpoint, network protocol, provider process, browser,
viewer, or external service was required.

## Candidate approaches

1. A vendor SDK or mock HTTP service would make transport and vendor semantics
   part of the product contract before either is selected. The chosen boundary
   uses a small provider trait and deterministic adapter result vocabulary.
2. Combining authorization and invocation in one call would leave no exact
   insertion point for later Guarded confirmation and could race revoke. The
   chosen API prepares an authorization/manifest token and consumes it in a
   separate dispatch call that rechecks the current policy revision.
3. Moving Candidate content into the privacy store would create a second
   Candidate lifecycle. The chosen integration asks the existing Candidate
   store to clean only records related to a canonical invalidation while
   preserving disposition and promotion targets.
4. Treating local deletion as provider deletion would make retention claims
   untruthful. The chosen records keep local content removal and provider-side
   deletion outcomes independent.

## Commands and configuration

```text
rebuild/scripts/validate focused v07-production-commit -- cargo test --manifest-path rebuild/Cargo.toml --workspace --all-targets --all-features
rebuild/scripts/validate focused v07-fixture-manifest -- rebuild/scripts/check-fixture-manifest rebuild/validation/shared/fixture-manifest.json
rebuild/scripts/validate focused v07-assertions -- rebuild/validation/privacy/local-only-boundary/assertions.py
rebuild/scripts/validate focused v07-assertions-repeat -- rebuild/validation/privacy/local-only-boundary/assertions.py
rebuild/scripts/validate focused v07-report-shape -- rebuild/scripts/check-validation-report rebuild/validation/privacy/local-only-boundary/report.md
```

The assertion verifies the exact Production commit subject, rejects Production
changes after that commit, checks that all workspace packages stay under
`rebuild/`, rejects legacy Volicord dependencies, validates the shared fixture
manifest and four evidence roles, discovers every mapped Rust test with
`cargo test -- --list`, and runs each mapped Production target.

## Observed results

The first assertion run completed in 3.805 seconds and reported four fixture
sources, five evidence groups, 43 mapped requirements, 12 Production targets,
62 discovered tests, and three provider-scope fixture files. Every executed
target passed.

The privacy oracle observed no dispatch token and no provider invocation before
opt-in even after recording current-host interactive access. A Project opt-in
persisted provider, model, purpose, capability, allowed `src` scope, vendor and
binary exclusions, fake secret-like marker filtering, known filtering limits,
and local/provider retention expectations. An attempted `docs` scope expansion
was rejected. Dispatch sent one filtered `src/lib.rs`, recorded it as
transmitted, recorded the generated vendor file as excluded and not transmitted,
and recorded the document as outside requested scope and not transmitted.
Revoke between preparation and dispatch produced `not_authorized` and did not
increase the adapter invocation count.

Unavailable provider state produced no invocation and no transmission. Failed,
partial, and stale adapter results remained distinct after actual transmission.
Canonical read state remained byte-for-byte equal through degradation, and a
real canonical correction revision remained unchanged after a conflicting
provider-derived cache was recorded. Annotation identity, retention expiry,
snapshot staleness, local deletion, provider deletion failure, and restart
inspection remained explicit.

Canonical Source forgetting removed only Candidate and managed Derived content
linked to that Source. Unrelated Candidate and Derived records remained current.
Existing opt-out, dismissal, expiry, explicit deletion, promotion, and Candidate
Inspection tests passed unchanged; promoted canonical targets survived Candidate
cleanup. Portable export and the privacy SQLite store contained no raw request
body marker after dispatch and deletion.

## Coverage and failures

Covered authority states are never enabled, enabled, disabled, revoked,
provider unavailable, provider failed, partial, and stale. Covered Source
outcomes are included, excluded, outside requested scope, outside opt-in scope,
filter not applied, filter no match, filtered, not transmitted, and transmitted.
Covered privacy lifecycle behavior includes opt-in persistence, dispatch-time
revision checks, Candidate opt-out/retention/deletion preservation, annotation
and cache identity, retention expiry, invalidation, local deletion,
provider-deletion failure, correction protection, canonical forgetting
propagation, and raw-body portable exclusion.

No final V07 command failed. Real network failure, real provider retention and
deletion behavior, hostile secret corpora, background scheduling, Guarded
confirmation/consumption, CLI/viewer privacy settings, MCP/Codex transport,
Linux installation, and combined V11 journeys are excluded rather than reported
as successful.

## Performance and resource observations

The first maintained assertion completed in 3.805 seconds after build reuse.
The six focused privacy tests completed in approximately 0.08 seconds within
that run, and the post-Production-commit workspace test completed in 6.667
seconds. Fixtures are deliberately small and deterministic. These observations
do not establish large-repository filtering throughput, provider latency,
network bandwidth, peak memory, database growth, or long-retention cleanup cost.

## Privacy and external transmission

No external transmission occurred during validation. The deterministic adapter
received only the in-memory filtered body at the explicit dispatch boundary.
Raw bodies are not serializable request records and were absent from portable
Canonical Context and persisted privacy state. The fake marker filter is
evidence of explicit policy and truthful outcomes, not a claim that all secrets
can be detected. Provider retention expectations and provider deletion results
remain inspectable statements distinct from successful local deletion.

## Acceptance results

- Pass: zero background provider invocation occurs before valid explicit
  Project opt-in; interactive current-host access creates no background consent.
- Pass: Project policy persists provider, model, purpose, requested capability,
  source scope, exclusions, filtering policy and limits, and retention meaning.
- Pass: request scope cannot silently exceed opt-in and dispatch rechecks the
  current policy revision, so revoke blocks a prepared request.
- Pass: actual transmitted Source manifests distinguish excluded, filtered,
  not-transmitted, transmitted, unavailable, failed, partial, and stale facts.
- Pass: provider absence, disablement, revoke, unavailability, and failure do
  not damage canonical, inventory, structural, Inquiry, Decision, Checkpoint,
  Recall, or provider-independent projection/document behavior.
- Pass: Candidate opt-out stops only matching new automatic collection;
  existing Candidate visibility, disposition, promotion target, retention,
  expiry, dismissal, and explicit deletion meaning remain unchanged.
- Pass: canonical forgetting cleans only related managed Candidate/Derived
  content; Candidate or annotation cleanup does not infer canonical deletion.
- Pass: annotation/cache retention, invalidation, local deletion, and provider
  deletion outcome remain inspectable and cannot overwrite user corrections.
- Pass: raw repository bodies and provider request payloads are excluded from
  portable bundles and persisted privacy records by default.
- Pass: all reconstruction packages remain under `rebuild/` and no legacy
  Volicord dependency or dual privacy path was introduced.

## Known limits

The filtering fixture uses an explicit fake marker and does not measure recall
or precision against real secrets. Provider responses and deletion outcomes are
deterministic in-process observations, not claims about a commercial provider.
The boundary stores bounded manifest metadata and Derived text but this V07 run
does not measure long-term storage size, concurrent dispatch, cancellation,
timeouts, or crash recovery. Guarded confirmation remains deliberately absent;
provider opt-in alone is not represented as high-risk dispatch approval.

## Recommended implementation choice

Retain `volicord-privacy` as the Project-scoped optional provider boundary:
validate explicit current-host user provenance for policy changes, persist
revisioned policy state, prepare filtered manifests without dispatch, consume a
single-use authorization at a separately guarded-capable dispatch boundary,
and store annotations/caches as managed Derived State. Continue using the
existing Inquiry Candidate store and canonical invalidation identities for
scoped cleanup instead of creating provider-owned Candidate or canonical data.

## Rejected alternatives and reasons

Do not select a vendor, model, credential store, transport, or secret scanner
from this evidence. Do not reuse interactive access as background consent, put
raw bodies in policy/request persistence, describe marker filtering as a
guarantee, merge provider deletion with local deletion, or let provider output
write canonical records. Do not add a weaker confirmation mechanism around
dispatch; the exact prepare/dispatch boundary is reserved for the accepted
Guarded-effect owner.

## Reusable primitive decision

`production_evidence`. The revisioned privacy store, provider adapter contract,
prepared dispatch boundary, manifest vocabulary, managed Derived lifecycle, and
Candidate/canonical invalidation integration are Production responsibilities.
The Python assertion is maintained requirement mapping and orchestration only;
it is not a second privacy engine, filter, provider, or deletion implementation.

## Decision revisit trigger status

Not triggered. The evidence demonstrates technical separation of interactive
access and background transmission and preserves a useful local-only core
journey. The deterministic fake marker intentionally does not establish perfect
secret filtering or provider deletion completeness, so those known limits do
not broaden consent. A later real-repository evaluation that shows exclusions
or filtering cannot provide an understandable bounded policy must use the
accepted Q3 revisit process.

## Follow-up work

- Place background external dispatch behind exact Guarded confirmation only if
  the Local Operations owner classifies it as a Guarded high-risk effect.
- Add host/CLI/viewer privacy surfaces and clean Linux/Codex wiring in V08,
  without changing this Project-scoped authority contract.
- Exercise real repositories, real provider retention limitations, concurrency,
  cancellation, accessibility, and combined Candidate/Guarded journeys in V11.
- Keep provider/model/transport selection and credential storage as explicit
  later implementation choices.

## Artifacts

Maintained artifacts are this report, `assertions.py`, the
`v07-privacy-boundary` fixture directory and matrix, its shared fixture-manifest
entry, the reused V01/V06/V09 fixture authorities, and the mapped Production
Rust tests. Focused command results, complete stdout/stderr, exit status, and
durations remain under ignored `rebuild/.local/validation/`.
