# V04 — Divergent canonical context merge and recovery

## Status

Passed for the maintained deterministic fixture and production Rust test
matrix. All 60 mapped assertions and 56 Rust integration tests passed in two
consecutive executions. This evidence covers the current local single-writer
merge, direct forgetting, managed deletion, and scoped recovery boundaries; it
does not claim unmanaged-copy deletion, network synchronization, or concurrent
multi-writer safety.

## Goal

Verify that divergent portable canonical histories can be merged without
silently changing user-owned meaning or retaining forgotten content in
canonical copies, operation replay state, managed bundles, SQLite database/WAL
state, or managed temporary candidates. Verify deterministic export/import/read
behavior and explicit recovery after transaction, process, publication,
import, forgetting, and post-merge sanitation faults.

## Accepted decisions being validated

- `open-decisions.md` Q6: stable Project identity, explicit clone binding,
  source-independent read, safe independent-addition merge, and user ownership
  of semantic and delete/modify conflicts.
- `open-decisions.md` Q7: correction versus supersession, user forgetting
  authority, privacy over immutable audit, and minimal non-content tombstones.
- `portable-context.md` sections 7–11: trustworthy common base, conflict
  vocabulary, bounded automatic resolution, exact user resolution Source,
  merge provenance, deterministic portability, and deletion propagation.
- `failure-and-recovery.md`: atomic canonical mutation, explicit failure or
  repair-required outcomes, idempotent scoped recovery, and no success claim
  for incomplete managed sanitation.
- `versioning-policy.md`: one current schema and bundle contract with rejection
  of unsupported versions before mutation.

No accepted product Decision is changed by this report.

## Input repositories and revisions

The reviewed starting HEAD was `0fad04fa` (`test: validate canonical context
recovery and merge`). The production implementation under test includes
`da38390e` (`fix: close canonical forgetting dependencies`) and `5ff5efb5`
(`fix: sanitize forgetting after bundle merge`).

The sole maintained fixture is the self-authored `v04-divergent-merge` fixture
under `fixtures/v04-scenarios/`. Its shared-manifest directory SHA-256 is
`d69107ca3cd06c81f8a9d8b68495a571eda5ac2c82089ee8493f488f00f655b7`;
the `scenario.json` SHA-256 is
`27151dceacfdac9e5e3480578970b7ff4555dc6e38f32ac767903e43f52bc7d5`.
It retains `validation_id: V04`, is declared CC0-1.0, and uses no external
repository.

## Environment and tool versions

- Linux x86-64, WSL2 kernel `6.18.33.2-microsoft-standard-WSL2`.
- `rustc 1.97.1 (8bab26f4f 2026-07-14)`.
- `cargo 1.97.1 (c980f4866 2026-06-30)`.
- Python `3.12.3` for fixture validation and process orchestration only.
- Locked Rust dependencies and bundled SQLite; no model, provider, remote
  service, package download, or network input was used.

## Candidate approaches

The executable candidate is the production Rust `Store` implementation. It
owns three-way bundle comparison, complete record selection, schema-backed
operation dependencies, direct and merge-selected forgetting sanitation,
SQLite deletion hygiene, registered-bundle refresh, durable sanitation state,
and replay behavior. Every executable V04 assertion invokes named production
Rust tests.

The Python harness checks fixture/manifest identity, verifies that every
declared scenario maps to an existing Rust test, catalogs the production test
set, and launches it serially. It does not implement dependency discovery,
forgetting, merge selection, sanitation, or replay semantics.

## Commands and configuration

Maintained focused commands:

```text
rebuild/scripts/validate focused v04-fixture-manifest-expanded -- rebuild/scripts/check-fixture-manifest rebuild/validation/shared/fixture-manifest.json
rebuild/scripts/validate focused v04-assertions-expanded-final -- rebuild/validation/canonical-context/divergent-merge/assertions.py
rebuild/scripts/validate focused v04-assertions-expanded-final-repeat -- rebuild/validation/canonical-context/divergent-merge/assertions.py
rebuild/scripts/validate focused v04-report-shape-expanded -- rebuild/scripts/check-validation-report rebuild/validation/canonical-context/divergent-merge/report.md
rebuild/scripts/validate focused v04-context-tests-expanded -- cargo test --manifest-path rebuild/Cargo.toml -p volicord-context --all-targets --all-features
rebuild/scripts/validate focused v04-architecture-expanded -- rebuild/scripts/check-architecture-contracts
rebuild/scripts/validate self-test
```

The assertion harness catalogs and executes:

```text
cargo test --manifest-path rebuild/Cargo.toml -p volicord-context --test canonical_read --test context_checkpoint --test divergent_merge --test forgetting_matrix --test inquiry --test kernel --test lifecycle --test portable_bundle --test portable_process --test process_reopen --test transaction_process -- --list
cargo test --manifest-path rebuild/Cargo.toml -p volicord-context --test canonical_read --test context_checkpoint --test divergent_merge --test forgetting_matrix --test inquiry --test kernel --test lifecycle --test portable_bundle --test portable_process --test process_reopen --test transaction_process -- --test-threads=1
```

Rust tests use explicit temporary stores, deterministic IDs/clocks, SQLite
triggers, locks, managed-bundle obstructions, child processes, and hard
termination. Runtime evidence remains in temporary directories or ignored
`rebuild/.local/validation/` artifacts.

## Observed results

- Verified common-base merge still combined independent additions and bounded
  non-semantic correction. Same-record, Question-state, semantic Decision,
  source-binding, unavailable-base, supersede/supersede, and delete/modify
  conflicts remained user-owned.
- Both delete/modify orientations passed for Source, Question, Decision, and
  Context Item. Checkpoint covered both meaningful one-sided forgetting paths;
  it has no content-revision operation in the current production surface.
- Incoming forgetting selected over local content exercised all five
  forgettable kinds. Local operation payloads and merge input/provenance
  verifiers were erased, target tombstones remained, registered bundles were
  refreshed, and SQLite database/WAL/temp byte scans found no selected-away
  content.
- Source-only forgetting after `supersede_decision` erased the embedded created
  user-turn Source payload although the operation result identity was a
  Decision. The replay row retained only duplicate-prevention identity and a
  content-free forgotten-dependency state; replay returned `NotFound`.
- Question-only forgetting left the Decision alive with its explicit user
  choice, rationale, applicability, user-turn provenance, and tombstoned
  Question identity. Current and immutable revision rows lost alternatives,
  recommendation key/rationale/Source list, and the Decision became review due.
  Restart, export, clean-store import, deterministic read, and re-export passed.
- Every available operation builder registered at least one canonical content
  dependency. A production-unit negative test proved a content-bearing
  operation cannot omit registration silently. No forgetting query uses an
  operation-kind allowlist or result-ID ownership predicate.
- A trigger fault before merge commit preserved the complete local modified
  state and no tombstone. A managed temporary-path obstruction after canonical
  commit returned `RepairRequired`, persisted sanitation `pending`, and did not
  return ordinary merge success. Removing the obstruction and replaying the
  same operation completed database/bundle sanitation without repeating the
  canonical mutation; the sanitized replay and changed input were then safely
  rejected because equality could no longer be verified.
- Generic child-process tests separately proved termination before commit and
  after commit/before response behavior, complete committed-state reopen,
  operation duplicate prevention, publication/import interruption, and process
  result preservation.
- Post-merge bundle bytes imported into a clean store, produced the same
  tombstone/read state, and re-exported deterministically. Unsupported schema
  and bundle versions failed before mutation.
- Both final runs reported the same fixture hash, 60 mapped assertions, 11
  production targets, 56 passing integration tests, and `status: passed`.

## Coverage and failures

Coverage includes all six conflict classes, four resolution modes, exact
resolution Source/conflict identity/revision, direct Source/Question dependency
closure, all five forgettable kinds, both meaningful delete/modify
orientations, operation replay cleanup, changed-input rejection, managed bundle
refresh, SQLite/WAL/temp residue, pre-commit rollback, post-commit sanitation
failure/recovery, restart, clean import, deterministic read/export, and format
rejection.

Three pre-fix probes failed at the reviewed HEAD with exit 101: the
`supersede_decision` Source payload remained in SQLite; Question alternatives
remained in a surviving Decision; and incoming forgetting left a local modified
Decision rationale in SQLite. The expanded production tests pass after the two
fix commits. One intermediate commit-2 Clippy run failed only on
`unnecessary_lazy_evaluations`; the test expression was corrected without
changing its assertion, and the warnings-denied rerun passed. The expanded V04
suite exposed no additional production defect, so no production change was
made in this validation commit and no fixture was weakened.

## Performance and resource observations

The two final focused V04 runs completed in `2924.925 ms` and `2996.183 ms`,
including fixture checks, test cataloging, Cargo process startup, 56 integration
tests, and validation-runner capture. These small deterministic fixtures are
not benchmarks. Peak memory, very large histories, concurrent-writer
throughput, and filesystem space amplification were not measured.

## Privacy and external transmission

All fixtures and runtime state were self-authored and local. No provider,
network call, telemetry endpoint, LLM, source transmission, or external
repository was used. Sensitive tokens existed only inside Rust tests and were
scanned across the managed database, WAL/SHM when present, managed temporary
candidate, registered bundle, imported store, and re-exported bundle. Complete
runner stdout/stderr and result metadata remain in ignored local artifacts.

## Acceptance results

- Pass: schema-backed dependency discovery replaces operation-kind and
  result-ID ownership inference.
- Pass: Source-only forgetting removes indirect `supersede_decision` payloads.
- Pass: Question-only forgetting removes copied presentation from surviving
  Decisions and revisions while preserving independent Decision meaning.
- Pass: all five supported forgettable kinds select complete closures in every
  meaningful merge orientation.
- Pass: merge-selected forgetting invokes shared dependency/copy sanitation,
  database hygiene, temporary cleanup, and registered-bundle refresh.
- Pass: incomplete post-commit sanitation returns `RepairRequired`, persists
  recoverable state, and cannot be reported as ordinary success.
- Pass: replay/recovery does not duplicate canonical mutation; unverifiable
  forgotten input and changed-input reuse are rejected safely.
- Pass: production-byte residue, clean import, deterministic read, and
  re-export assertions are executable rather than report-only claims.
- Pass: the harness invokes Rust production behavior and contains no second
  dependency, forgetting, merge, or sanitation implementation.

## Known limits

- Checkpoint is immutable in the current surface, so its incoming-forgetting
  coverage is one-sided rather than a synthetic content-revision conflict.
- The post-commit sanitation fault is a deterministic managed-bundle
  obstruction, not a process kill at an instruction-level boundary. Durable
  `pending` recovery and generic hard-termination behavior are tested
  separately.
- Fixtures are small. Concurrent writers, very large histories, lock
  contention, adversarial filesystem corruption, and other operating systems
  are not measured.
- Managed forgetting cannot erase copies, backups, snapshots, or bundles moved
  outside registered managed paths.
- Network synchronization, remote permissions, signatures, encryption,
  compression, conflict UI, and live collaboration remain outside V04.

## Recommended implementation choice

Retain the production three-way merge boundary with verified lineage,
canonical-record closure selection, schema-backed operation dependencies, and
one shared forgetting sanitation path. Keep canonical commit and managed
sanitation as explicit durable states: initial success requires sanitation
completion, while post-commit failure returns `RepairRequired` and permits only
scoped replay recovery under the original operation identity.

## Rejected alternatives and reasons

- Reject operation-kind allowlists: they missed indirect Source ownership in
  `supersede_decision` and would regress whenever a new builder embeds content.
- Reject result-ID-only ownership: an operation can return a Decision while
  embedding Source content, and one operation can depend on several records.
- Reject last-writer-wins, timestamps, table order, or import order: none proves
  semantic authority or privacy intent.
- Reject row-independent deletion sanitation: canonical tombstones alone do
  not clean copied Decision content, replay payloads, WAL/free pages, managed
  temporary candidates, or registered bundles.
- Reject report-only assertions without production-byte inspection: row reads
  can pass while raw managed artifacts still retain selected-away content.
- Reject a Python dependency/merge/sanitation oracle: duplicate domain
  semantics could agree with itself while production Rust remains wrong.

## Reusable primitive decision

`test_support_only`. Fixture-to-test mapping, deterministic process/fault
orchestration, and production-byte scanning remain maintained validation
support. Dependency discovery, forgetting, merge, replay, and sanitation are
already production Rust responsibilities; no experiment implementation is
proposed for promotion.

## Decision revisit trigger status

Not triggered. Q6 and Q7 remain accepted: all maintained local divergence and
forgetting paths were representable without last-writer authority, implicit
cascade, or retained managed payload. The documented triggers remain live for
unrepresentable conflicts, loss of stable identity, impractical explicit
binding, unmanaged-copy guarantees, or later concurrent/remote evidence.

## Follow-up work

- Independently replay Source/Question dependency cases, both merge
  orientations, managed sanitation, V04, and the process/fault suite in the
  Phase 4 final session.
- Exercise this provenance and repair-required state in later user-facing
  conflict/health surfaces without broadening V04.
- Measure large histories, concurrent writers, and filesystem variance before
  expanding the supported runtime boundary.

## Artifacts

Maintained inputs are `scenario.json`, `assertions.py`, this report, the shared
fixture-manifest entry, and named Rust integration tests. Ignored raw evidence:

- runner self-test:
  `rebuild/.local/validation/20260812T183417.823562Z-runner-self-test-84oirsij`
  (exit 0);
- pre-fix Source, Question, and merge-residue probes:
  `rebuild/.local/validation/20260812T183706.302106Z-baseline-supersede-source-leak-5r_2dvpq`,
  `rebuild/.local/validation/20260812T183715.666150Z-baseline-question-copy-leak-725hfw80`, and
  `rebuild/.local/validation/20260812T183718.937944Z-baseline-merge-forgetting-residue-9t6o2t_k`
  (each exit 101);
- expanded fixture-manifest check:
  `rebuild/.local/validation/20260812T185421.862519Z-v04-fixture-manifest-expanded-f2c7k1pg`
  (exit 0);
- final production assertions and deterministic repeat:
  `rebuild/.local/validation/20260812T185505.345385Z-v04-assertions-expanded-final-hgd_x8pm`
  and
  `rebuild/.local/validation/20260812T185512.490911Z-v04-assertions-expanded-final-repeat-f9ywc9or`
  (both exit 0).
