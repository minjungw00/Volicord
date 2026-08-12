# V04 — Divergent canonical context merge and recovery

## Status

Passed for the maintained deterministic fixture and production Rust test
matrix. All 44 mapped semantic/recovery assertions and 46 Rust tests passed in
two consecutive executions. This validates the current local, single-writer
merge and managed-recovery boundary; it does not claim network synchronization
or multi-writer coordination.

## Goal

Verify that divergent portable canonical histories can be compared and merged
without silently changing user-owned meaning or resurrecting forgotten
content. The same evidence must show deterministic export/import/read behavior
and recovery of only committed production state after transaction, process,
publication, import, merge, and forgetting failures.

## Accepted decisions being validated

- `open-decisions.md` Q6: stable Project identity, explicit clone binding,
  source-independent read, safe independent-addition merge, and user ownership
  of semantic conflicts, delete/modify conflicts, and unverified histories.
- `open-decisions.md` Q7: correction versus supersession, user forgetting
  authority, privacy over immutable audit, and minimal non-content tombstones.
- `portable-context.md` sections 7–11: trustworthy common base, the complete
  conflict vocabulary, bounded automatic resolution, exact user resolution
  Source, merge provenance, deterministic post-merge portability, and deletion
  propagation.
- `failure-and-recovery.md`: atomic canonical mutation, explicit degraded or
  failed outcomes, committed-state recovery, retry/replay, and non-publication
  of partial portable state.

No accepted product Decision is changed by this report.

## Input repositories and revisions

The original production baseline was
`fd0ba482` (`feat: expose deterministic canonical read basis`). The production
implementation under test includes `0e39eb8a` (`fix: merge complete canonical
record closures`) and `12d75d46` (`feat: complete canonical record forgetting`).

The only input fixture is the self-authored `v04-divergent-merge` fixture under
`fixtures/v04-scenarios/`. Its shared-manifest directory SHA-256 is
`94c8f809d506e2ae8676d33ce3babb890cb3f21bbc5028fee6cc5851838b860f`;
the `scenario.json` file SHA-256 is
`7f0316709e4f1520568d502cc34ed3ef2bde4d26b2699723c0f70acbad94f253`.
It fixes `validation_id: V04`, Project identity, common-base authority,
conflict/result classes, resolution requirements, post-merge assertions, and
recovery cases. It is declared CC0-1.0 and uses no external repository.

## Environment and tool versions

- Linux x86-64, WSL2 kernel `6.18.33.2-microsoft-standard-WSL2`.
- `rustc 1.97.1 (8bab26f4f 2026-07-14)` and
  `cargo 1.97.1 (c980f4866 2026-06-30)`.
- Python `3.12.3` for metadata and process orchestration only.
- The crate uses its locked Rust dependencies and bundled SQLite; no package,
  model, provider, remote service, or network input was used.

## Candidate approaches

1. The executable candidate is the production Rust `Store` three-way bundle
   comparison and merge path. It uses verified lineage, explicit conflict
   classes, whole canonical-record closures, user-owned resolution, minimal
   tombstones, transactional replacement, and deterministic portable bundles.
   It was exercised directly by all maintained assertions.
2. Row-wise union, last-writer-wins, timestamp/import-order selection,
   repository-path identity inference, and model-selected semantic resolution
   were evaluated as unsafe policies. The conflict fixtures demonstrate why
   they cannot satisfy Q6/Q7, so they were rejected rather than implemented as
   a second candidate.

The Python harness validates fixture metadata, maps every scenario to named
Rust tests, and launches those tests. It contains no merge, persistence, or
domain implementation.

## Commands and configuration

The maintained focused commands are:

```text
rebuild/scripts/validate focused v04-fixture-manifest -- rebuild/scripts/check-fixture-manifest rebuild/validation/shared/fixture-manifest.json
rebuild/scripts/validate focused v04-assertions -- rebuild/validation/canonical-context/divergent-merge/assertions.py
rebuild/scripts/validate focused v04-assertions-repeat -- rebuild/validation/canonical-context/divergent-merge/assertions.py
rebuild/scripts/validate focused v04-report-shape -- rebuild/scripts/check-validation-report rebuild/validation/canonical-context/divergent-merge/report.md
rebuild/scripts/validate focused v04-architecture -- rebuild/scripts/check-architecture-contracts
rebuild/scripts/validate self-test
```

The assertion harness catalogs and then executes the production suite with:

```text
cargo test --manifest-path rebuild/Cargo.toml -p volicord-context --test canonical_read --test divergent_merge --test forgetting_matrix --test inquiry --test kernel --test lifecycle --test portable_bundle --test portable_process --test process_reopen --test transaction_process -- --list
cargo test --manifest-path rebuild/Cargo.toml -p volicord-context --test canonical_read --test divergent_merge --test forgetting_matrix --test inquiry --test kernel --test lifecycle --test portable_bundle --test portable_process --test process_reopen --test transaction_process -- --test-threads=1
```

The process tests use explicit temporary store paths, deterministic IDs and
clocks, SQLite locks/triggers, marker files, child test processes, and hard
termination. Runtime files remain in temporary directories or ignored
`rebuild/.local/` evidence roots.

## Observed results

- A verified common base automatically combined independent additions and a
  one-sided formatting correction. Competing same-record corrections,
  different Question terminal states, competing Decision supersessions,
  delete/modify, source-binding conflict, and unavailable common base remained
  user-owned.
- Conflict output retained base/local/incoming history bases, affected
  identities, Source bases, consequence, uncertainty, and conflict-set
  identity/revision. A mismatched identity or revision was rejected.
- Choose-local, choose-incoming, a distinct explicitly curated result bundle,
  and context branch all used the exact current-host user-turn Source. Canonical
  read exposed merge provenance and branch basis after completion.
- Context Item and Decision delete/modify passed in both resolution directions.
  Source and Question forget/modify remained conflicts; a one-sided Checkpoint
  deletion propagated safely beside an independent Context correction.
- Every merged/imported forgotten identity had exactly one minimal tombstone
  and no active content-bearing closure. Raw portable-byte scans found no
  forgotten payload, and correction, supersession, dependency, link, replay
  input, or child rows did not reconstruct it.
- Post-merge exports/imports and canonical reads retained stable ordering and
  identity. Format corruption/newer-version inputs failed before mutation.
  Imported repository Sources were explicitly unavailable while canonical
  conflict work remained possible.
- Hard termination while a production operation was blocked before commit
  left no Project or operation replay entry. Hard termination after commit but
  before a process response retained the Project; the identical operation
  replayed and changed input was rejected.
- Bundle publication failure preserved the prior published bytes; import and
  merge interruption preserved prior database state. Forgetting faults rolled
  back complete closures and retried safely. Managed database, WAL, sibling,
  temporary, and bundle scans found no sensitive fixture bytes.
- Portable bytes excluded local absolute clone paths, operations, Candidate
  markers, Derived State markers, indexes/caches, raw tool traffic, transcripts,
  source copies, and legacy runtime identifiers.
- Both maintained executions reported the same fixture hash, 44 mapped
  assertions, ten production test targets, 46 passing tests, and `status:
  passed`.

## Coverage and failures

All scenarios declared by the V04 fixture were mapped to named production Rust
tests before execution. Coverage includes every required conflict class,
compatible correction, same-record and Question-state conflict,
supersede/supersede, all four resolution modes, exact resolution Source and
revision, replay/change rejection, provenance, deletion propagation, all five
forgettable record kinds, deterministic bundle/read behavior, format handling,
and the complete requested process/fault matrix.

The reviewed pre-fix regression failed because choosing the forgotten Context
side retained the incoming correction bytes. It passed after canonical-record
closure selection. During V04 test-support development, one smoke run used a
purported correction equal to the original presentation; production correctly
rejected it as invalid input. The fixture value was corrected, and both final
maintained assertion runs passed. No later run exposed another production
defect, and no assertion was weakened to obtain a pass.

## Performance and resource observations

The two final focused V04 assertion runs completed in `2656.497 ms` and
`2262.306 ms`, including test cataloging, process startup, Cargo execution, and
validation-runner capture. The 46 production tests themselves completed in
approximately two seconds per run on the maintained small fixture. Peak memory,
large-history behavior, concurrent-writer throughput, and filesystem space
amplification were not measured; these observations are not benchmarks.

## Privacy and external transmission

All fixtures and runtime state were self-authored and local. No provider,
network call, telemetry endpoint, LLM, source transmission, or external
repository was used. Sensitive test tokens were generated only inside Rust
tests and scanned across managed database, WAL, temporary, sibling, and bundle
artifacts. Raw command output and exit metadata remain in ignored local
validation artifacts.

## Acceptance results

- Pass: deterministic trustworthy-base comparison and explicit unavailable-base
  behavior.
- Pass: independent additions and bounded one-sided non-semantic correction are
  automatic; semantic and ambiguous same-record changes are not.
- Pass: Decision, Question, delete/modify, source-binding, and competing
  supersession consequences remain user-owned and inspectable.
- Pass: choose-local, choose-incoming, distinct explicit-result, and branch
  modes require exact conflict identity/revision and user-turn Source.
- Pass: complete canonical-record closures preserve the selected forgetting or
  modified side without tombstone/content coexistence.
- Pass: Source, Question, Decision, Context Item, and Checkpoint forgetting
  propagates through meaningful merge cases without content resurrection.
- Pass: merge provenance, replay, changed-input rejection, restart,
  export/import, deterministic canonical read, and bundle-version behavior.
- Pass: pre-commit and post-commit/pre-response hard termination, publication,
  import, merge, and forgetting recovery preserve only committed state.
- Pass: managed residue scans and portable exclusion checks found no forbidden
  content, local absolute path, Candidate, or Derived State.
- Pass: the executable assertion boundary invokes production Rust and does not
  duplicate merge semantics.

## Known limits

- Fixtures are intentionally small and deterministic. Concurrent writers,
  very large histories, lock contention, performance limits, and adversarial
  filesystem corruption are not measured.
- Hard-termination evidence uses Linux child-process kill behavior and the
  tested WSL2 filesystem. Other operating systems and storage layers remain
  unvalidated.
- The managed forgetting guarantee cannot erase bundle copies, backups, or
  snapshots moved outside the registered runtime boundary.
- Network synchronization, remote permissions, signatures, encryption,
  compression, conflict UI, and live collaboration are outside V04.

## Recommended implementation choice

Retain the production three-way merge boundary with explicit verified lineage,
canonical-record closure selection, deterministic bundle validation, and
transactional target replacement. Permit automatic combination only when
identity and relations prove the change non-semantic. Require the exact user
resolution Source for semantic, Question-state, delete/modify, source-binding,
and unavailable-base choices, and preserve merge/branch provenance for later
canonical reads.

## Rejected alternatives and reasons

- Reject row-wise table union: the pre-fix regression showed it can combine a
  tombstone with an opposing correction revision and resurrect forgotten data.
- Reject timestamps, table order, import order, or last-writer-wins: none proves
  semantic authority or privacy intent.
- Reject automatic two-way merge when the common base is missing: it cannot
  distinguish independent work from incompatible meaning.
- Reject path/name/remote similarity for Source binding: imported canonical
  context remains usable while the repository is unavailable, so local binding
  must stay explicit.
- Reject model recommendation as conflict authority: Decision, Question, and
  deletion outcomes require user judgment and exact provenance.
- Reject a Python merge oracle: duplicate domain semantics could agree with
  itself while production Rust remains wrong.

## Reusable primitive decision

`test_support_only`. The fixture-to-test mapping and child-process orchestration
are useful maintained validation support but are not production primitives.
The merge and forgetting implementations under test are already production
crate code; no experiment code is proposed for promotion.

## Decision revisit trigger status

Not triggered. Record-level closure selection represented all maintained V04
divergence classes, and stable Project identity plus explicit clone binding did
not obstruct the tested workflows. Deletion propagated through deterministic
bundles without retained managed payload. Q6 and Q7 therefore remain accepted;
larger, concurrent, remote, or unmanaged-copy evidence could still trigger
their documented review conditions later.

## Follow-up work

- Independently replay the production forgetting matrix, V04 assertions, and
  process/fault suite in the Phase 4 final session.
- Exercise the same merge provenance in the later combined multi-repository
  journey and user-facing conflict presentation.
- Measure large histories, concurrent writer behavior, filesystem variance,
  and repair paths before expanding the supported runtime boundary.
- Keep remote synchronization, permissions, encryption, and external-copy
  deletion outside this local validation until separately designed and owned.

## Artifacts

Maintained inputs are `scenario.json`, `assertions.py`, this report, the shared
fixture-manifest entry, and the named Rust integration tests. Raw evidence is
ignored and remains at:

- runner self-test:
  `rebuild/.local/validation/20260812T174519.742089Z-runner-self-test-vdy3lmy2`
  (exit 0);
- reviewed pre-fix regression:
  `rebuild/.local/validation/20260812T171251.209289Z-v04-delete-modify-red-zqtq_yr4`
  (exit 101), followed by fixed regression
  `rebuild/.local/validation/20260812T171738.417211Z-v04-delete-modify-green-x36r1683`
  (exit 0);
- complete closure regression suite:
  `rebuild/.local/validation/20260812T172022.912989Z-v04-merge-closure-regressions-h3uwxv3o`
  (exit 0);
- fixture-manifest check:
  `rebuild/.local/validation/20260812T174507.870647Z-v04-fixture-manifest-final-kagazijn`
  (exit 0);
- production assertions:
  `rebuild/.local/validation/20260812T174436.804072Z-v04-assertions-final-2kgr4hwt`
  (exit 0);
- deterministic repeat:
  `rebuild/.local/validation/20260812T174439.485129Z-v04-assertions-repeat-final-ilkwykf7`
  (exit 0);
- report-shape and architecture checks:
  `rebuild/.local/validation/20260812T174507.963257Z-v04-report-shape-final-7mryly00`
  and
  `rebuild/.local/validation/20260812T174508.043901Z-v04-architecture-final-sqf4kl66`
  (both exit 0);
- workspace Clippy with warnings denied and all-target/all-feature tests:
  `rebuild/.local/validation/20260812T174508.155527Z-v04-clippy-final-s1uskbka`
  and
  `rebuild/.local/validation/20260812T174508.422741Z-v04-context-tests-final-183r8s5a`
  (both exit 0; 56 total Rust tests passed).
