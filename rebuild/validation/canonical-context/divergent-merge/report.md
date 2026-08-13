# V04 — Divergent canonical context merge and recovery

## Status

Passed for the maintained deterministic fixture and production Rust evidence at
`f5c7b1dd`. Two consecutive maintained executions each reported 156 mapped assertions,
13 integration-test targets, 93 passing integration tests, fixture
SHA-256 `f55de3e0df56d2383a3cc39872fdc16d6fec5d7cbe3774b797bd2cee327866eb`,
and `status: passed`. The independently authored checkpoint audit also passed.
The Phase 5 implementation gate is `ready` within the known limits below. The
post-commit exact final aggregate has not run and is not part of this report.

## Goal

Verify that divergent portable Canonical Context can be compared and merged
without silently changing user-owned meaning, accepting semantically invalid
complete state, resurrecting forgotten content, or leaving partial canonical
mutation. Verify that current import, generated merge, `ExplicitMerged`, state
replacement, deterministic export/read, reopen, operation replay, lineage,
binding, and managed-bundle behavior all cross their maintained production
boundaries.

The focused checkpoint goal is stricter: a forgotten Source can support a
Checkpoint fact only through a content-free witness naming the exact active
Checkpoint, Source tombstone, semantic use, and ordered position. An unrelated
Project Source tombstone must provide no fallback authority.

## Accepted decisions being validated

- `open-decisions.md` Q6: stable Project identity, explicit clone binding,
  source-independent read, bounded automatic merge, and user-owned semantic
  conflict resolution.
- `open-decisions.md` Q7: distinct correction, supersession, contradiction,
  review-due, and forgetting semantics with minimal content-free tombstones.
- `open-decisions.md` Q11: source-grounded Checkpoints with independent work,
  verification, user-review, and user-acceptance facts.
- `portable-context.md` sections 6–11: deterministic export, validated import,
  trustworthy common base, conflict vocabulary, resolution authority, merge
  provenance, and deletion propagation.
- `failure-and-recovery.md`: atomic canonical mutation, explicit failure and
  repair-required outcomes, deterministic retry, and no partial success.
- `versioning-policy.md`: one current schema and portable format, with
  unsupported versions rejected before mutation.

No accepted product Decision is changed by this evidence.

## Input repositories and revisions

The sole repository input is the current reconstruction workspace at
`f5c7b1dd` (`test: harden portable semantic mutation coverage`). The production
state under audit includes `304b6cc2` (`fix: enforce portable canonical semantic
parity`) and `72ac1a8d` (`fix: scope Checkpoint forgotten Source witnesses`).
Their actual production diffs and current interfaces were inspected.

The sole maintained V04 fixture is
`rebuild/validation/canonical-context/divergent-merge/fixtures/v04-scenarios/`.
Its directory hash is the fixture SHA-256 reported in Status; its
`scenario.json` SHA-256 is
`9e98fafae37544dde74292f4788b9ea438697f0851cade418ce4295a24784e5b`.
The fixture is self-authored, CC0-1.0, and uses no external repository.

## Environment and tool versions

- Linux x86-64, WSL2 kernel `6.18.33.2-microsoft-standard-WSL2`.
- `rustc 1.97.1 (8bab26f4f 2026-07-14)`.
- `cargo 1.97.1 (c980f4866 2026-06-30)`.
- Python `3.12.3` for fixture validation and process orchestration only.
- Canonical schema version 12 and portable bundle format version 5.
- Locked Rust dependencies and bundled SQLite; no network, model, provider, or
  remote-service input.

## Candidate approaches

The selected executable approach is the production Rust `Store` and its single
complete-state semantic boundary,
`canonical_state::validate_payload`. Direct transactions project their complete
state through `validate_project_state` before commit. Portable bundle import,
generated merge targets, `ExplicitMerged`, state replacement, and export reach
the same owner through their production paths.

Checkpoint Source forgetting uses the maintained
`checkpoint_forgotten_source_witnesses` table. Each row is owned by one active
Checkpoint and records one forgotten Source identity, one typed semantic use,
and one position. Direct Source forgetting captures existing supporting basis,
changed basis, verification, user-review, and user-acceptance slots before
clearing active references. Generated merge sanitation performs the same typed
conversion. Checkpoint forgetting removes every owned witness.

The Python V04 harness validates fixture identity, maps declared assertions to
named Rust tests, catalogs the integration tests, and launches those tests. It
does not implement portable semantics, witness matching, merge selection,
forgetting sanitation, state replacement, or a second semantic oracle.

## Commands and configuration

The maintained V04 command is:

```text
rebuild/validation/canonical-context/divergent-merge/assertions.py
```

It catalogs and serially executes these 13 integration targets through Cargo:
`canonical_read`, `context_checkpoint`, `divergent_merge`, `forgetting_matrix`,
`inquiry`, `invariant_catalog`, `kernel`, `lifecycle`, `portable_bundle`,
`portable_invariants`, `portable_process`, `process_reopen`, and
`transaction_process`. The serial setting remains scoped to this maintained
V04 command.

Focused validation also ran the fixture-manifest checker, the complete
`volicord-context` all-target/all-feature suite, the invariant catalog, the
portable-invariant target, and formatting checks through
`rebuild/scripts/validate focused`.

## Observed results

- The central complete-state boundary semantically decodes active Sources,
  Context Items, Checkpoints, Questions, Decisions, revision histories,
  relations, tombstones, and surviving content-free witnesses.
- A checksummed, lineage-consistent attack removed a Checkpoint's actual Source
  relations, set verification/review/acceptance Source fields to the tagged
  portable null, added an unrelated Source tombstone, and remained valid JSON
  with correct envelope checksum and history basis. Production import rejected
  it for the Checkpoint supporting-Source semantic invariant before mutation.
- Direct Source forgetting produced only relation-specific checkpoint
  witnesses. A Source used in both supporting and changed basis produced two
  distinct use/position witnesses; one user Source used for review and
  acceptance also produced two distinct witnesses. A Checkpoint with multiple
  Sources retained the active slots and exact forgotten slots in deterministic
  order.
- Missing, wrong-owner, non-tombstoned/wrong-Source, wrong semantic-use, wrong
  verification-position, duplicate, ambiguous, and active-plus-forgotten slot
  inputs were rejected. One correct forgotten relation plus an unrelated Source
  tombstone remained valid.
- Valid direct forgetting exported deterministically, imported into a clean
  store, produced the same canonical read basis, re-exported deterministically,
  and reopened with the same state. Forgetting the Checkpoint removed all its
  owned Source witnesses.
- Valid `ExplicitMerged` input preserved exact witnesses. Forged or unrelated
  explicit merged input was validated before resolution, replacement, lineage
  mutation, or operation success and was rejected without partial mutation.
- A generated merge target involving incoming Source forgetting created the
  exact supporting-basis witness required by a locally added Checkpoint and
  passed the central validator before state replacement.
- Failed import and `ExplicitMerged` cases preserved canonical read, stable
  export, operation rows and replay state, lineage, local clone binding,
  registered managed-bundle bytes, and reopened state.
- All semantic JSON-null mutations now use the actual tagged
  `PortableValue::Null` representation. Test preconditions verify every table
  cell's portable tag, current format version, recomputed semantic history
  basis, recomputed common-base basis for the crafted state, and recomputed
  checksum. Exact import and `ExplicitMerged` diagnostics demonstrate that the
  repaired question, Source, and Checkpoint cases reach their intended semantic
  invariants rather than JSON decoding, integrity, or version rejection.
- Trustworthy-base independent additions and bounded one-sided non-semantic
  correction merged automatically. Same-record, Question-state, semantic
  Decision, delete/modify, Source-binding, and unavailable-base conflicts
  remained user-owned.
- `ChooseLocal`, `ChooseIncoming`, `ExplicitMerged`, and context-branch results
  preserved exact conflict identity/revision, current-host user resolution
  Source, input bases, result basis, and merge provenance.
- Direct and merge-selected forgetting maintained minimal tombstones,
  operation-dependency cleanup, managed-bundle refresh, database/WAL/temp
  sanitation, deterministic replay, and explicit recovery from pre-commit and
  post-commit faults.
- Post-merge export/import and canonical read remained deterministic; Source
  absence did not prevent source-independent Project, Decision, Context Item,
  Checkpoint, lifecycle, and tombstone inspection.
- Question-specific Decision-history, user Decision authority, exact Question
  revision/presentation, correction, supersession, forgotten-Question
  sanitation, and review-due invariants remained covered by the same central
  boundary.

## Coverage and failures

The maintained harness covers 156 declared assertions through 93 Rust tests in
13 integration targets. Coverage includes all six V04 conflict classes, four
resolution modes, exact resolution requirements, deterministic import/export
and read, direct and merge-selected forgetting, generated targets,
`ExplicitMerged`, replacement, replay, reopen, managed sanitation, and
no-partial-mutation behavior.

Checkpoint forgotten-Source coverage includes the unrelated-tombstone attack,
exact direct witnesses, wrong Checkpoint/Source/use/position, correct plus
unrelated tombstone, multiple dimensions per Source, multiple Sources per
Checkpoint, valid and forged explicit merge, generated merge, deterministic
export/read/reopen, and rejection-state preservation.

Both maintained executions passed with identical counts and fixture identity.
The focused all-target/all-feature crate suite also passed. No current
production defect was demonstrated, no production Rust was changed by the test
commit, and no fixture condition was relaxed.

## Performance and resource observations

The two maintained V04 executions completed in 11,383.551 ms and 11,368.615 ms.
They include test cataloging, Cargo process startup, 93 integration tests, and
validation-runner capture. These deterministic fixtures are correctness
evidence, not performance benchmarks. Peak memory, very large histories,
concurrent-writer throughput, and filesystem amplification were not measured.

## Privacy and external transmission

All fixtures, databases, bundles, logs, and mutation inputs were local.
Forgotten-Source witnesses contain identities, use, and position only; they do
not contain Source body, locator, command output, user text, or recoverable
content hash. No source code or fixture content was sent to a provider or
remote service. Raw validation output remains under ignored
`rebuild/.local/validation/`.

## Acceptance results

| Acceptance | Result | Maintained evidence |
|---|---|---|
| Unrelated Source tombstone provides no Checkpoint authority | Passed | Independent audit and `unrelated_source_tombstone_does_not_authorize_missing_checkpoint_sources` |
| Exact relation-specific witness creation and read | Passed | `direct_checkpoint_source_forgetting_is_exact_portable_and_deterministic` |
| Wrong owner, Source, use, position, duplicate, and ambiguity rejection | Passed | `malformed_checkpoint_forgotten_source_witnesses_reject_every_portable_write_boundary` |
| Valid and invalid explicit merge boundary | Passed | Checkpoint witness tests plus `ExplicitMerged` no-partial-mutation matrix |
| Generated merge target with Source forgetting | Passed | `generated_and_explicit_merges_preserve_valid_checkpoint_source_witnesses` |
| Tagged portable null reaches semantic validation | Passed | Repaired portable mutation tests with representation, lineage, and checksum preconditions |
| Deterministic export/import/read/reopen | Passed | Portable, canonical-read, merge, and process-reopen targets |
| Failed writes preserve canonical and operational state | Passed | Import and explicit-merge mutation helpers plus recovery targets |
| Maintained deterministic repeat | Passed | Two identical 156/13/93 harness results |

## Known limits

- Evidence is bounded to the local synchronous single-writer kernel and small
  deterministic fixtures.
- Managed forgetting cannot erase user copies, backups, filesystem snapshots,
  provider retention, moved bundles, or another clone that has not selected the
  tombstone.
- Checkpoint has no content-revision operation, so its delete/modify coverage is
  limited to the meaningful one-sided forgetting cases in the current API.
- Generated targets are production-generated rather than an arbitrary invalid
  injection surface; submitted local, incoming, and explicit bundles are
  validated before target generation.
- Remote authenticity, signatures, encryption, compression, network sync,
  concurrent multi-writer behavior, conflict UI, user-facing Recall, Candidate
  behavior, and Derived State remain outside V04.
- The exact final aggregate is intentionally outside this pre-aggregate report.

## Recommended implementation choice

Retain the central complete-state semantic boundary and the relation-specific
Checkpoint forgotten-Source witness model. The maintained evidence supports a
Phase 5 implementation gate of `ready` without broadening V04 into later
subsystems or untested guarantees.

## Rejected alternatives and reasons

- A Project-global Source-tombstone boolean is rejected because it cannot bind
  forgotten evidence to one Checkpoint relation, use, or position.
- Raw JSON `null` is rejected for portable semantic mutation construction
  because the format uses a tagged value enum and raw null can stop at decoding.
- A Python semantic validator is rejected because it would duplicate production
  meaning and could disagree with the Rust boundary.
- Last-writer-wins, timestamp choice, table order, path similarity, and model
  recommendation are rejected as semantic conflict authority.
- Sanitizing invalid submitted state during import or explicit merge is rejected
  because invalid input must fail before mutation.

## Reusable primitive decision

The reusable production primitives are `canonical_state::validate_payload`,
the deterministic portable table representation, typed
`checkpoint_forgotten_source_witnesses`, complete record-closure selection,
state replacement, operation dependency cleanup, managed sanitation, and the
deterministic canonical read basis. The tagged-null constructor and envelope
precondition checker remain test-only support and do not define product
semantics.

## Decision revisit trigger status

No accepted Decision revisit trigger is active on this evidence. Q6 and Q7
remain subject to future evidence that record-level three-way state cannot
represent real divergence, stable identity/explicit binding obstructs normal
work, or managed deletion cannot satisfy its documented privacy boundary. Q11
remains subject to future evidence of inaccurate canonical Checkpoints. None of
those conditions was observed here.

## Follow-up work

Proceed to Phase 5 implementation work from the maintained Phase 4 owners and
this `ready` gate. Later validation still owns user-facing Recall and
Checkpoint quality, privacy/provider deletion completeness, document
grounding, multi-repository rehearsal, concurrency/resource behavior, and
unmanaged-copy limits. Run the repository's exact final aggregate only after
the documentation commit is complete and the worktree is clean.

## Artifacts

- Invariant catalog:
  `rebuild/crates/volicord-context/INVARIANTS.md`.
- Production implementation:
  `rebuild/crates/volicord-context/src/canonical_state.rs`,
  `portable.rs`, `merge.rs`, `store.rs`, and `read.rs`.
- Rust evidence:
  `rebuild/crates/volicord-context/tests/portable_invariants.rs` and the 12
  other integration targets listed under Commands and configuration.
- Maintained V04 harness and fixture:
  `rebuild/validation/canonical-context/divergent-merge/assertions.py`,
  `fixtures/v04-scenarios/scenario.json`, and
  `rebuild/validation/shared/fixture-manifest.json`.
- Preserved local runs:
  `rebuild/.local/validation/20260813T005220.956552Z-commit1-v04-assertions-8tklckbi/`
  and
  `rebuild/.local/validation/20260813T005241.480122Z-commit1-v04-assertions-repeat-beq02tqy/`.
