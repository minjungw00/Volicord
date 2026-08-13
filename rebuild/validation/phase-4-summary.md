# Phase 4 Canonical Context Kernel conclusion

- Phase 5 implementation gate: `ready`
- Evidence basis: current `f5c7b1dd`, the independently passed
  relation-specific Checkpoint witness audit, and two identical maintained V04
  executions of 156 mapped assertions, 13 integration targets, and 93 passing
  integration tests
- Scope: the synchronous Rust Canonical Context Kernel, SQLite store, portable
  bundle format 5, canonical schema 12, deterministic canonical read basis,
  divergent merge, and managed forgetting behavior
- Excluded claim: this conclusion does not certify Repository Intelligence,
  user-facing Recall, documents, Candidate Inspection, Guarded effects,
  installation, network synchronization, or the later multi-repository gate
- Final aggregate status: not yet run; this document does not claim it passed

## Canonical records and complete-state authority

Project is the stable scope root. Source, Question, Decision, Context Item, and
Checkpoint retain typed identity, Project ownership, provenance, relations, and
their distinct lifecycle meaning. User judgment remains separate from agent
recommendation, observed fact, and generated interpretation.

`canonical_state::validate_payload` is the central complete-state semantic
boundary. Direct writes validate their complete transactional projection before
commit. Bundle import, generated merge targets, `ExplicitMerged`, state
replacement, and production export reach the same owner through their current
production paths. The boundary semantically decodes Source and Context Item
payloads, Checkpoint dimensions, Question/Decision history, corrections,
supersession, relations, tombstones, and allowed content-free witnesses.

The current portable representation uses tagged values, including
`{"type":"null"}` for `PortableValue::Null`. Maintained semantic mutation tests
construct that exact representation, verify current format version, validate
all portable value tags, recompute the canonical history/common-base basis and
checksum, and then require exact semantic diagnostics from production import
and `ExplicitMerged`. The repaired cases therefore do not pass merely because
raw JSON null fails decoding or because integrity, lineage, or version checks
fail first.

## Question and Decision integrity

An answered or delegated Question carries one Question-specific content-free
history witness for the exact Question revision, original root Decision,
terminal outcome, response Source, authority, and creation kind. Active and
forgotten identities remain rooted without copying Question text, alternatives,
recommendation, Decision rationale, Source payload, or a recoverable content
hash into the witness.

Active Decision authority requires the exact same-Project current-host user-turn
Source authored by the user, or its explicitly owned forgotten-Source authority
witness. Exact displayed Question revision, alternatives, recommendation,
choice/delegation, terminal outcome, response link, current snapshot, immutable
revision sequence, meaning-preserving correction, and linear acyclic
same-Question supersession remain mutually consistent.

Question forgetting removes Question-owned presentation from every surviving
Decision and Decision revision and creates the required `review_due` state.
Independently owned choice/delegation, user rationale, applicability,
assumptions, revisit triggers, user-turn provenance, and minimal tombstoned
Question identity remain readable and portable.

## Relation-specific Checkpoint Source forgetting

Checkpoint work state, automated verification, user review, and user acceptance
remain independent facts. When an active Source is forgotten, production
captures each actual Checkpoint relation before clearing it in
`checkpoint_forgotten_source_witnesses`. One row names:

- the exact active Checkpoint;
- the exact forgotten Source tombstone;
- `supporting_basis`, `changed_basis`, `verification`, `user_review`, or
  `user_acceptance` semantic use; and
- the exact ordered position, with position zero for singleton review and
  acceptance uses.

No Project-global Source tombstone condition participates in Checkpoint
admission. A checksummed, lineage-consistent state with all actual Checkpoint
Source relations removed, tagged-null verification/review/acceptance Sources,
and one unrelated Source tombstone is rejected for the Checkpoint semantic
invariant before mutation.

Maintained coverage includes direct exact witness creation, wrong Checkpoint,
wrong/non-tombstoned Source, wrong use, wrong verification position, duplicate
or ambiguous slot, correct relation plus unrelated tombstone, one Source used
by multiple Checkpoint dimensions, one Checkpoint using multiple Sources,
valid and forged explicit merge, generated merge with Source forgetting,
deterministic export/read/reopen, and no-partial-mutation behavior. Forgetting
the Checkpoint deletes every owned witness before leaving its own minimal
tombstone.

## Lifecycle, operation dependency, and managed forgetting

Correction, semantic supersession, contradiction, `review_due`, and forgetting
remain different transitions. Source, Question, Decision, Context Item, and
Checkpoint are forgettable under exact current-host user authority; Project is
not.

Every content-bearing operation input is registered in
`operation_dependencies`. Forgetting erases the input basis and dependency rows
while retaining only content-free duplicate-prevention result identity.
Same-input replay then reports the unavailable forgotten dependency, and
changed input cannot recover or replace forgotten content.

Managed forgetting removes affected active content and relations, keeps only
the required minimal tombstone or explicit content-free witness, sanitizes the
SQLite database and WAL boundary, removes managed temporary state, and refreshes
registered bundles. A pre-commit fault rolls back. A post-commit managed-output
obstruction returns `RepairRequired`, persists pending sanitation, and resumes
idempotently without repeating canonical mutation.

## Portable context, merge, read, and recovery

Portable bundles include canonical Project identity, Source manifest, Questions,
Decisions, Context Items, Checkpoints, revisions, relations, merge provenance,
and minimum tombstones. They exclude local clone paths, operation rows,
Candidates, Derived State, indexes, caches, raw tool traffic, full transcripts,
and raw Source bodies. Source-independent canonical read remains available when
the repository is absent.

Three-way comparison preserves all six current conflict classes. Verified
independent additions and narrowly proven non-semantic correction can merge
automatically. Question/Decision meaning, same-record semantic change,
delete/modify, Source binding, and unavailable common base remain user-owned.
Resolution is bound to the exact conflict identity/revision and a current-host
user Source; timestamp, path, import order, and model recommendation do not
choose a winner.

Valid import, generated merge, `ExplicitMerged`, and state replacement preserve
deterministic canonical state. Post-merge export imports into a clean store,
produces the same canonical read basis, re-exports deterministically, and
reopens consistently. Invalid import and explicit merge preserve canonical
read, export bytes, operation replay rows, lineage, clone binding, managed
bundle bytes, and reopened state.

## Maintained V04 evidence

V04 is `passed`. The self-authored fixture directory hash is
`f55de3e0df56d2383a3cc39872fdc16d6fec5d7cbe3774b797bd2cee327866eb`.
The Python harness performs fixture-to-test mapping and Cargo orchestration
only; all merge, witness, semantic, forgetting, replacement, and recovery
assertions execute against production Rust behavior.

Two consecutive focused runs each reported:

- 156 mapped assertions;
- 13 production integration-test targets;
- 93 passing production integration tests;
- the same fixture hash; and
- `status: passed`.

The full `volicord-context` all-target/all-feature suite, portable-invariant
target, fixture manifest, invariant catalog, and Rust formatting check also
passed before the test commit. Current V04 evidence covers import, generated
merge, `ExplicitMerged`, state replacement, deterministic export/read, reopen,
no-partial mutation, managed sanitation, replay, and bounded process recovery.

## Dependency and repository boundary

The nested workspace contains only `volicord-context`, and every package is
under `rebuild/`. Its production dependencies remain synchronous and local:
`getrandom`, bundled `rusqlite`, `serde`, and `serde_json`; `sha2` and `tempfile`
are test-only. No reconstruction package depends on a legacy Volicord crate,
and no alternate runtime or storage implementation is part of this gate.

## Remaining known limits and Decision triggers

- Managed forgetting cannot erase user copies, backups, filesystem snapshots,
  provider retention, moved bundles, or another clone until it selects the
  tombstone.
- Evidence uses small deterministic fixtures on the supported Linux SQLite
  boundary. Concurrent writers, large histories, lock contention, adversarial
  filesystem corruption, other operating systems, and resource scaling are not
  measured.
- Checkpoint is immutable in the current API, so delete/modify coverage uses its
  meaningful one-sided forgetting cases rather than a synthetic revision path.
- Generated merge targets are production-generated, not arbitrary invalid
  injection points. Submitted base, local, incoming, and explicit bundles are
  validated before generated state is admitted.
- Encryption, signatures, compression, remote authenticity, team authority,
  conflict UI, user-facing Recall, Candidate behavior, Derived State, and
  unmanaged-copy revocation remain outside Phase 4.

No recorded Q1–Q13 revisit trigger is active on this evidence. Q6 and Q7 remain
open to future evidence that record-level three-way state cannot represent real
divergence, stable identity or explicit binding obstructs normal work, or the
documented managed deletion boundary is insufficient. Q11 remains open to
future evidence of inaccurate canonical Checkpoints. None was observed here.

## Phase 5 gate

`ready`. The current central semantic boundary, relation-specific Checkpoint
forgotten-Source evidence, deterministic V04 repeat, and maintained focused
suite support proceeding to Phase 5 without an unresolved production defect or
active Decision revisit trigger. This conclusion remains bounded by the known
limits above and does not complete Phase 5 or later validation.

The required exact post-commit final aggregate has not run and is not claimed
as evidence in this pre-aggregate summary.

## Maintained references

- Design owners: `rebuild/docs/design/architecture.md`,
  `rebuild/docs/design/domain-model.md`,
  `rebuild/docs/design/inquiry-and-decision.md`,
  `rebuild/docs/design/portable-context.md`,
  `rebuild/docs/design/versioning-policy.md`, and
  `rebuild/docs/design/failure-and-recovery.md`.
- Invariant catalog:
  `rebuild/crates/volicord-context/INVARIANTS.md`.
- Production boundary:
  `rebuild/crates/volicord-context/src/canonical_state.rs`,
  `portable.rs`, `merge.rs`, `store.rs`, and `read.rs`.
- Rust evidence: the 13 integration targets under
  `rebuild/crates/volicord-context/tests/` named by the V04 harness.
- V04 evidence:
  `rebuild/validation/canonical-context/divergent-merge/report.md`,
  `assertions.py`, `fixtures/v04-scenarios/scenario.json`, and
  `rebuild/validation/shared/fixture-manifest.json`.
- Validation interfaces: `rebuild/scripts/validate`,
  `rebuild/scripts/check-fixture-manifest`,
  `rebuild/scripts/check-validation-report`, and
  `rebuild/scripts/check-architecture-contracts`.
