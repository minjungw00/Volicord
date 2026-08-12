# Phase 4 Canonical Context Kernel conclusion

- Phase 5 implementation gate: `ready`
- Scope: the synchronous Rust Canonical Context Kernel, its SQLite store, portable bundle,
  deterministic canonical read basis, divergent merge, and managed forgetting behavior
- Excluded claim: this conclusion does not certify Repository Intelligence, user-facing Recall,
  documents, Candidate Inspection, Guarded effects, installation, or later validation work

## Canonical records

- **Project and Source:** Project is the stable scope root and is not forgettable. Project identity,
  rename history, explicit local clone binding, typed Source identity, provenance, availability,
  and portable locator/snapshot basis survive restart and another-path import. Source forgetting
  removes its payload and managed relations while retaining only a minimal tombstoned identity.
- **Question and Decision:** Questions preserve exact revision, Source/dependency basis,
  alternatives, recommendation, material scope, and terminal state. An explicit current-host
  response atomically creates or validates its user-turn Source, records a Decision when required,
  and transitions the Question. Decision choice/delegation, user rationale, applicability,
  assumptions, revisit triggers, provenance, correction history, and supersession remain distinct
  from the agent recommendation.
- **Context Item and Checkpoint:** All eight Context Item roles preserve typed provenance and Source
  basis. Checkpoints preserve meaningful completion, pause, or handoff state with independent work,
  verification, user-review, and user-acceptance facts. Both kinds support user-authorized
  forgetting; Project deletion was not added.

## Lifecycle and dependency closure

Correction appends an immutable non-semantic revision under the same identity. A changed Decision
uses a new identity and directed supersession. Contradiction retains both records and their Source
bases, and `review_due` does not silently invalidate a Decision. Forgetting is a separate
user-authorized transition for Source, Question, Decision, Context Item, and Checkpoint.

Every persisted operation input is registered through the schema-backed
`operation_dependencies` relation. The common operation recorder rejects an empty dependency set,
and import and merge derive dependencies from the complete canonical payload. Forgetting looks up
dependent operations by canonical identity, not by operation-kind or result-identity lists. It
erases the complete input basis and all dependency rows while retaining operation/result identity
for duplicate prevention. Same-input replay then returns `NotFound`; changed input cannot recover
or replace forgotten content.

Source-only forgetting after `supersede_decision` removes an inline created user-turn Source from
the Source table, the supersession operation input, SQLite free/WAL state, and registered bundle
bytes. Correction, response, supersession, Checkpoint, import, merge, and forgetting operation
inputs are covered by direct Rust tests and the schema-level dependency guard.

Question-only forgetting deletes prompt and presentation storage, dependencies, response links,
and Question revisions. It also clears copied alternatives, recommendation key, rationale, and
recommendation Sources from every surviving Decision and Decision revision. The independently
owned choice/delegation, user rationale, applicability, assumptions, revisit triggers, user-turn
provenance, and minimal tombstoned Question identity remain. Each affected Decision is marked
`review_due`. Reopen, deterministic read, export/import, re-export, raw managed-byte scans, and an
independent merge probe confirm that Question-owned content is not reconstructed or reintroduced.

## Supported forgetting matrix

The maintained Rust matrix covers Source, Question, Decision, Context Item, and Checkpoint across
current-host user authorization, wrong-Project rejection, replay, restart, relation cleanup,
portable export/import, deterministic read, merge propagation, managed residue scanning, and
minimal tombstones. Tombstones contain only Project identity, record kind, record identity, and
forgetting time; they contain no raw payload or content hash. Project remains outside the
forgetting API.

## Managed storage and portability

The store uses one caller-supplied SQLite path with foreign keys, WAL, `synchronous=FULL`, and
`secure_delete=ON`. Successful forgetting checkpoints and truncates WAL, vacuums the database,
checkpoints again, removes affected managed temporary state, and refreshes every registered
portable bundle. Tests scan database, WAL/SHM when present, temporary candidate, managed bundle,
clean imported store, and re-exported bytes for sensitive fixture values.

The portable bundle has its own kind and version, deterministic JSON bytes, stable Project and
canonical identities, lineage, canonical relations, revisions, supersession, and minimal
tombstones. Local absolute bindings, operation rows, Candidates, Derived State, indexes, caches,
raw tool traffic, full transcripts, and raw Source bodies are excluded. Source-independent read
and explicit another-path binding work after clean import without rewriting historical Source
basis. Unsupported schema or bundle versions fail before mutation; there is no legacy decoder,
migration, importer, or dual-runtime path.

## Merge sanitation and deletion propagation

Three-way comparison requires an explicit trustworthy base and preserves all six conflict
classes. Independent additions and verified one-sided non-semantic corrections can merge
automatically; Question/Decision meaning, delete/modify, competing binding, and unavailable-base
cases remain user-owned. Resolution is bound to the exact conflict set/revision and a current-host
user-turn Source; timestamps, table order, paths, and import order do not choose a winner.

Both meaningful delete/modify orientations are covered for Source, Question, Decision, and Context
Item. Checkpoint has both one-sided forgetting paths because the current surface has no content
revision operation. Selecting a forgotten target replaces its complete record closure, invokes the
same operation and Question-copy sanitation as direct forgetting, erases merge input/provenance
bases that could retain selected-away content, and persists managed sanitation as `pending` until
database hygiene and every registered-bundle refresh complete. A pre-commit fault rolls back the
entire mutation. A post-commit obstruction returns `RepairRequired`; replay completes sanitation
without duplicate mutation, after which sanitized and changed replay inputs return `NotFound`.
Importing the result into a clean store preserves the same current/history/tombstone state and
deterministic bytes.

## V04, read, and recovery status

V04 is `passed`. Its self-authored fixture hash matches the shared manifest. The Python harness
owns only fixture-to-test orchestration and invokes Rust production behavior; it does not contain a
second merge or forgetting implementation. Two consecutive audit runs each reported 60 mapped
assertions, 11 production integration-test targets, 56 passing integration tests, the same fixture
hash, and no failure. The maintained report shape and every referenced raw artifact were verified.
Its claims remain bounded to the observed local single-writer, managed-storage scenarios.

`read_canonical_basis` is deterministic, read-only, and independent of Repository Intelligence,
providers, MCP, CLI, viewer, renderer, Product Repository availability, local paths, locale,
timezone, access frequency, ranking, and model output. It remains usable after restart, Source
loss, export/import, another-path binding, merge, branch resolution, and forgetting. It is an input
basis for later Recall work, not a claim that user-facing Recall is complete.

Rust unit, integration, lifecycle, inquiry, portability, merge, canonical-read, process-reopen, and
transaction/process tests cover the production-relevant V03 and V05 persistence, provenance,
atomic response, deterministic ordering, restart, rollback, hard-termination, and replay
semantics. Only committed canonical state survives; failed writes leave no partial canonical
success, and repeated operation identity prevents duplicate mutation.

## Dependency and legacy boundary

The nested workspace contains only `volicord-context`, all packages are under `rebuild/`, and
`rebuild/Cargo.lock` is tracked. The production dependency graph is synchronous and local:
`getrandom`, bundled `rusqlite`, `serde`, and `serde_json`; `tempfile` is test-only. Manifest,
lockfile, metadata, dependency-tree, license, and source scans show no legacy Volicord crate,
async runtime, network synchronization, second storage backend, speculative service hierarchy,
legacy Runtime Home/schema access, compatibility alias, or parallel production path. Durable
production transitions contain no `panic!`, `unwrap`, or `expect` state enforcement. No runtime
database, journal, bundle, Source copy, generated index, or log is tracked.

## Remaining known limits and Decision triggers

- Managed forgetting cannot erase user copies, backups, filesystem snapshots, cloud/provider
  retention, bundles moved outside registered paths, or state in another clone until that clone
  receives and selects the tombstone.
- Evidence uses small deterministic fixtures and the supported Linux SQLite boundary. Concurrent
  writers, large histories, lock contention, adversarial filesystem corruption, filesystem
  variance, and other operating systems are not measured.
- Checkpoint is immutable in the current API, so delete/modify coverage uses its two meaningful
  one-sided forgetting orientations rather than a synthetic content-revision conflict.
- The post-commit sanitation fault is a deterministic managed-bundle obstruction; instruction-level
  process termination at that exact boundary is represented by durable `pending` recovery plus the
  separate hard-termination suite.
- Encryption, signatures, compression, remote synchronization, team authority, conflict UI, and
  unmanaged-copy revocation remain outside the Phase 4 implementation.

All recorded Q1–Q13 revisit triggers remain inactive on this evidence. The Phase 4-relevant Q6 and
Q7 triggers remain live for future evidence of unrepresentable conflict, loss of stable identity,
impractical explicit binding, or deletion guarantees outside the documented managed boundary;
none was met here.

## Phase 5 gate

`ready`. The complete Phase 4 implementation and maintained evidence satisfy the Canonical Context
Kernel gate without an unresolved production defect, unregistered current operation dependency,
surviving Question-owned copy, merge sanitation gap, V04 overclaim, dependency concern, or active
Decision revisit trigger. This conclusion does not complete Phase 5 or any later validation.

## Maintained source, test, report, and validation references

- Design owners: `rebuild/docs/design/architecture.md`,
  `rebuild/docs/design/domain-model.md`, `rebuild/docs/design/inquiry-and-decision.md`,
  `rebuild/docs/design/portable-context.md`, `rebuild/docs/design/versioning-policy.md`, and
  `rebuild/docs/design/failure-and-recovery.md`
- Implementation basis and manifests: `rebuild/crates/volicord-context/README.md`,
  `rebuild/Cargo.toml`, `rebuild/Cargo.lock`, and
  `rebuild/crates/volicord-context/Cargo.toml`
- Production source: `rebuild/crates/volicord-context/src/lib.rs`,
  `rebuild/crates/volicord-context/src/error.rs`,
  `rebuild/crates/volicord-context/src/identity.rs`,
  `rebuild/crates/volicord-context/src/merge.rs`,
  `rebuild/crates/volicord-context/src/model.rs`,
  `rebuild/crates/volicord-context/src/portable.rs`,
  `rebuild/crates/volicord-context/src/read.rs`,
  `rebuild/crates/volicord-context/src/store.rs`, and
  `rebuild/crates/volicord-context/src/time.rs`
- Rust tests: `rebuild/crates/volicord-context/tests/canonical_read.rs`,
  `rebuild/crates/volicord-context/tests/context_checkpoint.rs`,
  `rebuild/crates/volicord-context/tests/divergent_merge.rs`,
  `rebuild/crates/volicord-context/tests/forgetting_matrix.rs`,
  `rebuild/crates/volicord-context/tests/inquiry.rs`,
  `rebuild/crates/volicord-context/tests/kernel.rs`,
  `rebuild/crates/volicord-context/tests/lifecycle.rs`,
  `rebuild/crates/volicord-context/tests/portable_bundle.rs`,
  `rebuild/crates/volicord-context/tests/portable_process.rs`,
  `rebuild/crates/volicord-context/tests/process_reopen.rs`, and
  `rebuild/crates/volicord-context/tests/transaction_process.rs`
- Maintained reports: `rebuild/validation/canonical-context/portability/report.md`,
  `rebuild/validation/canonical-context/divergent-merge/report.md`, and
  `rebuild/validation/inquiry/frontier-resume/report.md`
- V04 assets: `rebuild/validation/canonical-context/divergent-merge/assertions.py`,
  `rebuild/validation/canonical-context/divergent-merge/fixtures/v04-scenarios/scenario.json`, and
  `rebuild/validation/shared/fixture-manifest.json`
- Validation interfaces: `rebuild/scripts/validate`, `rebuild/scripts/check-fixture-manifest`,
  `rebuild/scripts/check-validation-report`, and `rebuild/scripts/check-architecture-contracts`
