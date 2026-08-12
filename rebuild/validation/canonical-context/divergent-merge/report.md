# V04 — Divergent canonical context merge and recovery

## Status

Passed for the maintained deterministic fixture and production Rust test
matrix. All 72 mapped assertions and 64 Rust integration tests passed in two
consecutive executions. This evidence covers the current local single-writer
merge, direct forgetting, portable semantic validation, managed deletion, and
scoped recovery boundaries; it does not claim unmanaged-copy deletion, network
synchronization, or concurrent multi-writer safety.

## Goal

Verify that divergent portable canonical histories can be merged without
silently changing user-owned meaning or retaining forgotten content in
canonical copies, operation replay state, managed bundles, SQLite database/WAL
state, or managed temporary candidates. Verify that import and
`ExplicitMerged` reject checksummed, lineage-consistent forgotten-Question
payloads which retain Question-owned Decision presentation or omit required
review state before canonical mutation. Verify deterministic export/import/read
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

The reviewed starting HEAD was `7e19da15` (`docs: record phase 4 implementation
conclusions`). Its history includes `da38390e` (`fix: close canonical forgetting
dependencies`), `5ff5efb5` (`fix: sanitize forgetting after bundle merge`), and
`f147e446` (`test: expand V04 forgetting coverage`). The portable invariant
correction under test is `1c9b74fd` (`fix: enforce forgotten Question portable
invariants`).

The sole maintained fixture is the self-authored `v04-divergent-merge` fixture
under `fixtures/v04-scenarios/`. Its shared-manifest directory SHA-256 is
`3ce89d4e7ba8682ddf0b2256861555af3b1483bd15d2a47a234f67356571f3a8`;
the `scenario.json` SHA-256 is
`7042939d2c9c2d793545add31799ca4c3da2cac7c509b0769ec6afb24ff60ebc`.
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
owns three-way bundle comparison, complete record selection, one portable table
semantic validator, schema-backed operation dependencies, direct and
merge-selected forgetting sanitation, SQLite deletion hygiene,
registered-bundle refresh, durable sanitation state, and replay behavior. The
central validator is reached by import, explicit merged input, generated merge
targets, state replacement, and generated export validation. Every executable
V04 assertion invokes named production Rust tests.

The Python harness checks fixture/manifest identity, verifies that every
declared scenario maps to an existing Rust test, catalogs the production test
set, and launches it serially. It does not implement dependency discovery,
forgetting, portable semantic validation, merge selection, sanitation, or
replay semantics.

## Commands and configuration

Maintained focused commands:

```text
rebuild/scripts/validate focused v04-fixture-manifest-portable-invariant -- rebuild/scripts/check-fixture-manifest rebuild/validation/shared/fixture-manifest.json
rebuild/scripts/validate focused v04-portable-invariant-assertions -- rebuild/validation/canonical-context/divergent-merge/assertions.py
rebuild/scripts/validate focused v04-portable-invariant-assertions-repeat -- rebuild/validation/canonical-context/divergent-merge/assertions.py
rebuild/scripts/validate focused v04-report-shape-portable-invariant -- rebuild/scripts/check-validation-report rebuild/validation/canonical-context/divergent-merge/report.md
rebuild/scripts/validate focused v04-context-tests-portable-invariant -- cargo test --locked --manifest-path rebuild/Cargo.toml -p volicord-context --all-targets --all-features
```

The assertion harness catalogs and executes:

```text
cargo test --manifest-path rebuild/Cargo.toml -p volicord-context --test canonical_read --test context_checkpoint --test divergent_merge --test forgetting_matrix --test inquiry --test kernel --test lifecycle --test portable_bundle --test portable_invariants --test portable_process --test process_reopen --test transaction_process -- --list
cargo test --manifest-path rebuild/Cargo.toml -p volicord-context --test canonical_read --test context_checkpoint --test divergent_merge --test forgetting_matrix --test inquiry --test kernel --test lifecycle --test portable_bundle --test portable_invariants --test portable_process --test process_reopen --test transaction_process -- --test-threads=1
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
- A direct forgotten-Question export with sanitized active Decision,
  sanitized Decision revisions, and `review_due` imported successfully. A
  forgotten Question with no surviving Decision also imported without a
  spurious review requirement, and an active Question with an empty Decision
  presentation remained valid.
- Crafted payloads restored active-Decision presentation, restored only an
  immutable Decision revision, or removed `review_due`. Their semantic history
  basis and envelope checksum were recomputed. Import rejected each as
  `CorruptState`, distinct from the separately asserted `IntegrityFailure` for
  a checksum mismatch.
- `ExplicitMerged` rejected the same lineage-consistent invalid state before
  replacement, both with a locally active Question and with the local Question
  already tombstoned. Neither path reported conflict-resolution success or
  invoked post-merge sanitation as a substitute for validation.
- Every portable-invariant rejection preserved the canonical read, deterministic
  export, exact operation replay rows, lineage rows, registered managed bundle,
  local binding, and successful reopened read. An inconsistent internal store
  was also prevented from overwriting an existing authoritative export.
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
- Both final runs reported the same fixture hash, 72 mapped assertions, 12
  production targets, 64 passing integration tests, and `status: passed`.

## Coverage and failures

Coverage includes all six conflict classes, four resolution modes, exact
resolution Source/conflict identity/revision, direct Source/Question dependency
closure, all five forgettable kinds, both meaningful delete/modify
orientations, operation replay cleanup, changed-input rejection, managed bundle
refresh, SQLite/WAL/temp residue, pre-commit rollback, post-commit sanitation
failure/recovery, restart, clean import, deterministic read/export, format
rejection, and central forgotten-Question dependent validation for import,
`ExplicitMerged`, generated targets, state replacement, and export.

Three earlier pre-fix probes failed with exit 101: the
`supersede_decision` Source payload remained in SQLite; Question alternatives
remained in a surviving Decision; and incoming forgetting left a local modified
Decision rationale in SQLite. The expanded production tests pass after the two
earlier fix commits. The reviewed `7e19da15` state still accepted a tombstoned
Question with retained Decision presentation or missing review state when a
payload recomputed checksum and lineage. `1c9b74fd` corrected that production
boundary before this V04-only update. The expanded V04 run exposed no further
production defect, so no production behavior was changed in this validation
commit and no fixture was weakened.

## Performance and resource observations

The two final focused V04 runs completed in `3750.010 ms` and `3772.554 ms`,
including test cataloging, Cargo process startup, 64 integration
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
- Pass: the central portable table validator rejects a forgotten Question with
  active or revision-only Decision presentation, or an active Decision without
  `review_due`, after checksum and lineage verification and before mutation.
- Pass: import and `ExplicitMerged`, including an already-forgotten local
  Question, return typed `CorruptState` without changing canonical,
  operational, lineage, managed-bundle, or binding state.
- Pass: valid sanitized import, no-surviving-Decision forgetting, active
  Question with empty presentation, direct forgetting, export/import, merge,
  canonical read, and deterministic re-export remain accepted.
- Pass: generated export is validated before publication and does not overwrite
  an existing bundle when internal canonical tables violate the invariant.
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
- Crafted portable inputs use the current JSON format and test-only envelope
  recomputation support. This is not a public unchecked bundle editor, a future
  format upgrader, or evidence for signatures or adversarial parser hardening.

## Recommended implementation choice

Retain the production three-way merge boundary with verified lineage,
canonical-record closure selection, schema-backed operation dependencies, one
shared forgetting sanitation path, and one central portable semantic validator.
Validate submitted and generated portable state before canonical replacement or
publication. Keep canonical commit and managed sanitation as explicit durable
states: initial success requires sanitation completion, while post-commit
failure returns `RepairRequired` and permits only scoped replay recovery under
the original operation identity.

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
- Reject checksum-only trust: integrity proves payload bytes were not changed
  after checksum calculation, not that their canonical meaning is consistent.
- Reject a normal-export round trip as the only portable test: production
  export already emits sanitized state and cannot represent crafted valid-hash
  input that crosses the trust boundary.
- Reject merge postprocessing as a substitute for input validation: an
  `ExplicitMerged` payload can retain forgotten content without creating a new
  local forgetting event.
- Reject silent sanitation of submitted portable state: it would rewrite user
  input, checksum, lineage, and review meaning while reporting false success.
- Reject import-only validation duplicated separately from merge: it would
  leave explicit merge, generated targets, state replacement, or export with a
  different invariant boundary.

## Reusable primitive decision

`test_support_only`. Fixture-to-test mapping, crafted-envelope checksum/history
recomputation, deterministic process/fault orchestration, and production-byte
scanning remain maintained validation support. Portable semantic validation,
dependency discovery, forgetting, merge, replay, and sanitation are production
Rust responsibilities; no experiment implementation is proposed for promotion.

## Decision revisit trigger status

Not triggered. Q6 and Q7 remain accepted: all maintained local divergence and
forgetting paths were representable without last-writer authority, implicit
cascade, silent input rewrite, or retained managed payload. The invariant adds
the review state already established by direct forgetting and does not change a
product Decision. The documented triggers remain live for unrepresentable
conflicts, loss of stable identity, impractical explicit binding,
unmanaged-copy guarantees, or later concurrent/remote evidence.

## Follow-up work

- Independently audit the central validator, V04 evidence, commit separation,
  and focused results before relying on the existing Phase 5 gate conclusion.
- Exercise this provenance and repair-required state in later user-facing
  conflict/health surfaces without broadening V04.
- Measure large histories, concurrent writers, and filesystem variance before
  expanding the supported runtime boundary.

## Artifacts

Maintained inputs are `scenario.json`, `assertions.py`, this report, the shared
fixture-manifest entry, and named Rust integration tests. Ignored raw evidence:

- runner self-test:
  `rebuild/.local/validation/20260812T194627.703087Z-runner-self-test-i5yr79he`
  (exit 0);
- pre-fix Source, Question, and merge-residue probes:
  `rebuild/.local/validation/20260812T183706.302106Z-baseline-supersede-source-leak-5r_2dvpq`,
  `rebuild/.local/validation/20260812T183715.666150Z-baseline-question-copy-leak-725hfw80`, and
  `rebuild/.local/validation/20260812T183718.937944Z-baseline-merge-forgetting-residue-9t6o2t_k`
  (each exit 101);
- portable-invariant fixture-manifest check:
  `rebuild/.local/validation/20260812T195059.133862Z-v04-fixture-manifest-portable-invariant-final-gf0bw1ap`
  (exit 0);
- portable-invariant production assertions and deterministic repeat:
  `rebuild/.local/validation/20260812T194904.462203Z-v04-portable-invariant-assertions-16qj_6tv`
  and
  `rebuild/.local/validation/20260812T194908.255529Z-v04-portable-invariant-assertions-repeat-6d8cgjne`
  (both exit 0);
- report-shape and complete `volicord-context` checks:
  `rebuild/.local/validation/20260812T195059.054080Z-v04-report-shape-portable-invariant-qpxb7_qc`
  and
  `rebuild/.local/validation/20260812T195059.220181Z-v04-context-tests-portable-invariant-hn94zu_7`
  (both exit 0).
