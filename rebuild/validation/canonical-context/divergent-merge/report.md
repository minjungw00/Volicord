# V04 — Divergent canonical context merge and recovery

## Status

Passed for the maintained deterministic fixture and production Rust test
matrix. All 138 mapped assertions and 87 Rust integration tests passed in two
consecutive executions. This evidence covers the current local single-writer
merge, centralized full-state boundary, semantic invariant catalog,
Question-specific Decision-history witness, deterministic portable mutation
matrix, lifecycle metamorphisms, direct forgetting, managed deletion, and
scoped recovery boundaries. The Phase 5 gate remains blocked pending the
requested independent final audit; this report does not claim unmanaged-copy
deletion, network synchronization, cryptographic authenticity, or concurrent
multi-writer safety.

## Goal

Verify that divergent portable canonical histories can be merged without
silently changing user-owned meaning or retaining forgotten content in
canonical copies, operation replay state, managed bundles, SQLite database/WAL
state, or managed temporary candidates. Verify that import and
`ExplicitMerged` reject checksummed, lineage-consistent forgotten-Question
payloads which retain Question-owned Decision presentation or omit required
review state before canonical mutation. Verify that the same central production
boundary rejects Decisions authorized by non-user Sources, nonexistent exact
Question revisions, undisplayed choices, inconsistent terminal outcomes or
response links, orphan/invalid supersession roles, and forged revision history.
Verify valid direct answered, delegated, corrected, superseded,
Source-forgotten, and Question-forgotten states, deterministic
export/import/read behavior, and explicit recovery after transaction, process,
publication, import, forgetting, and post-merge sanitation faults.
Verify that an unrelated Decision tombstone cannot justify one terminal
Question, that the content-free witness remains rooted through valid
Decision/Source/Question forgetting, and that recomputed single-invariant
mutations cross import and `ExplicitMerged` without partial mutation.

## Accepted decisions being validated

- `open-decisions.md` Q6: stable Project identity, explicit clone binding,
  source-independent read, safe independent-addition merge, and user ownership
  of semantic and delete/modify conflicts.
- `open-decisions.md` Q7: correction versus supersession, user forgetting
  authority, privacy over immutable audit, and minimal non-content tombstones.
- `portable-context.md` sections 7–11: trustworthy common base, conflict
  vocabulary, bounded automatic resolution, exact user resolution Source,
  merge provenance, deterministic portability, and deletion propagation.
- `inquiry-and-decision.md` sections 6–9: exact current-host user-turn Source,
  exact displayed Question revision, explicit choice/delegation, atomic
  response, and matching `answered`/`delegated` terminal outcome.
- `failure-and-recovery.md`: atomic canonical mutation, explicit failure or
  repair-required outcomes, idempotent scoped recovery, and no success claim
  for incomplete managed sanitation.
- `versioning-policy.md`: one current schema and bundle contract with rejection
  of unsupported versions before mutation.

No accepted product Decision is changed by this report.

## Input repositories and revisions

The reviewed starting HEAD was `be317b4e` (`docs: conclude phase 4 canonical
context gate`). Its verified history includes `ef07e095` (`fix: enforce portable
Decision provenance`) and `60a1e72f` (`test: cover portable Decision semantic
integrity`). The systemic production corrections under test are `0f30cb69`
(`refactor: centralize canonical state validation`) and `feaad4f5` (`fix:
require per-question Decision history`).

The sole maintained fixture is the self-authored `v04-divergent-merge` fixture
under `fixtures/v04-scenarios/`. Its shared-manifest directory SHA-256 is
`e395a3a8128bd1f0393c2228d4453426a223ca1a035f58b7dc11de0878c87e90`;
the `scenario.json` SHA-256 is
`3d0e00bfa8749ded6db49fd599f6ef1aca4b71167473a48851cb2357f7080133`.
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
owns three-way bundle comparison, complete record selection, one
`canonical_state` full-state boundary, schema-backed operation dependencies,
direct and merge-selected forgetting sanitation, SQLite deletion hygiene,
registered-bundle refresh, durable sanitation state, and replay behavior. The
central boundary covers active/tombstoned Sources and Questions, exact Question
revisions and response links, content-free Question Decision-history witnesses,
active Decisions and revision histories, supersession, terminal outcomes, and
review state. Direct command postconditions, import, explicit merged input,
generated merge targets, state replacement, and generated export validation
all reach it. `INVARIANTS.md` assigns semantic-slug ownership and executable
anchors without treating local operation or clone-binding rows as portable.

The Python harness checks fixture/manifest identity, verifies that every
declared scenario maps to an existing Rust test, catalogs the production test
set, and launches it serially. Rust test support creates valid command states,
recomputes checksum and history basis for crafted inputs, and checks preserved
state. Neither layer implements a second dependency, forgetting, portable
semantic-validation, merge-selection, sanitation, or replay oracle.

## Commands and configuration

Maintained focused commands:

```text
rebuild/scripts/validate focused commit3-fixture-manifest -- rebuild/scripts/check-fixture-manifest rebuild/validation/shared/fixture-manifest.json
rebuild/scripts/validate focused commit3-v04-assertions -- rebuild/validation/canonical-context/divergent-merge/assertions.py
rebuild/scripts/validate focused commit3-v04-assertions-repeat -- rebuild/validation/canonical-context/divergent-merge/assertions.py
rebuild/scripts/validate focused commit3-report-shape -- rebuild/scripts/check-validation-report rebuild/validation/canonical-context/divergent-merge/report.md
rebuild/scripts/validate focused commit3-context-tests -- cargo test --locked --manifest-path rebuild/Cargo.toml -p volicord-context --all-targets --all-features
rebuild/scripts/validate focused commit3-architecture -- rebuild/scripts/check-architecture-contracts
```

The assertion harness catalogs and executes:

```text
cargo test --manifest-path rebuild/Cargo.toml -p volicord-context --test canonical_read --test context_checkpoint --test divergent_merge --test forgetting_matrix --test inquiry --test invariant_catalog --test kernel --test lifecycle --test portable_bundle --test portable_invariants --test portable_process --test process_reopen --test transaction_process -- --list
cargo test --manifest-path rebuild/Cargo.toml -p volicord-context --test canonical_read --test context_checkpoint --test divergent_merge --test forgetting_matrix --test inquiry --test invariant_catalog --test kernel --test lifecycle --test portable_bundle --test portable_invariants --test portable_process --test process_reopen --test transaction_process -- --test-threads=1
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
- Direct production states for an original answered Decision, delegation,
  correction, supersession, Source forgetting, and Question forgetting all
  passed portable export/import. The Source-forgotten state retained only the
  Decision-owned `current_host_user_turn` authority fact plus the minimal
  Source identity; the forgotten Source payload, actor identity, host, session,
  and turn were absent.
- Original answered and delegated responses created exactly one immutable
  `question_decision_history_witnesses` row. The row retained Project,
  Question/revision, root Decision, terminal outcome, Source identity, authority
  literal, creation kind, and timestamp, while carrying no Question text,
  alternative, recommendation, choice/delegate, rationale, Source payload, or
  content hash. It survived valid root-Decision, Source, and Question forgetting
  and remained unchanged by correction and supersession.
- A deterministic table-driven mutation matrix recomputed checksum and lineage
  for witness removal; unrelated tombstone; wrong Question, revision, outcome,
  root, Source, or authority; duplicate witness; missing/tombstoned root
  mismatch; terminal/role mismatch; active-plus-tombstone; Decision authority,
  Question revision, alternative; and portable local-binding injection.
  Companion deterministic cases cover supersession branch/cycle and correction
  semantics. Import and `ExplicitMerged` rejected every represented mutation as
  `CorruptState` while preserving canonical read, deterministic export,
  lineage, binding, replay rows, managed bytes, and reopened state.
- Command-generated state passed full-state validation, export, clean import,
  equal canonical read, deterministic re-export, local-binding exclusion, and
  reopen. Maintained metamorphic mappings cover Decision/Source/Question
  forgetting, both merge-forgetting orientations, merge/export/import/read,
  replay after forgetting, and restart across direct, portable, forgetting,
  merge, and recovery operations.
- Active Decision authority was rejected when its Source was a file or when a
  current-host-like Source was agent-authored. The same representative inputs
  were rejected by the direct command path, while the final valid direct
  answered response exported successfully.
- Crafted Decisions referencing revision 99, choosing an undisplayed
  alternative, disagreeing with the root Question outcome, omitting or
  mismatching the exact response Source, or adding an unrelated root Decision
  were rejected after checksum and history-basis recomputation.
- Branching and cyclic supersession were rejected. A direct linear
  supersession retained one exact Question identity/revision, allowed the
  superseding choice to differ from the original terminal outcome, and remained
  valid after its Source or Question was forgotten.
- Missing/gapped Decision revision history, current-row/current-revision
  mismatch, revision-only provenance substitution, and semantic applicability
  mutation disguised as correction were rejected. Direct presentation-only
  correction remained valid, including after its authorization Source was
  forgotten under the same narrow authority witness.
- A direct forgotten-Question export with sanitized active Decision,
  sanitized Decision revisions, and `review_due` imported successfully. A
  forgotten Question with no surviving Decision also imported without a
  spurious review requirement. An active-Question Decision with empty
  presentation was rejected because it no longer matched the exact Question
  revision; only direct Question forgetting permits that sanitation.
- Crafted payloads restored active-Decision presentation, restored only an
  immutable Decision revision, or removed `review_due`. Their semantic history
  basis and envelope checksum were recomputed. Import rejected each as
  `CorruptState`, distinct from the separately asserted `IntegrityFailure` for
  a checksum mismatch.
- `ExplicitMerged` rejected the same lineage-consistent invalid state before
  replacement, both with a locally active Question and with the local Question
  already tombstoned. Neither path reported conflict-resolution success or
  invoked post-merge sanitation as a substitute for validation.
- `ExplicitMerged` also rejected non-user Decision authority with an active
  local Source and after the local Decision Source was already forgotten. The
  submitted invalid bundle was validated before resolution, state replacement,
  lineage mutation, or operation success.
- Every portable-invariant rejection preserved the canonical read, deterministic
  export, exact operation replay rows, lineage rows, registered managed bundle,
  local binding, and successful reopened read. An inconsistent internal store
  was also prevented from overwriting an existing authoritative export.
- Generated merge targets and state replacement continue to call the same
  portable table boundary. There is no separately supplied generated-target
  corruption hook: submitted invalid local/incoming/explicit bundles are
  rejected before generation, while production-generated semantic-conflict
  targets are validated before replacement.
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
- The one writable/readable production contract is canonical schema version 11
  and portable bundle format version 4. No older-format decoder, migration,
  compatibility branch, or dual writer was added.
- Both final runs reported the same fixture hash, 138 mapped assertions, 13
  production targets, 87 passing integration tests, and `status: passed`.

## Coverage and failures

Coverage includes all six conflict classes, four resolution modes, exact
resolution Source/conflict identity/revision, direct Source/Question dependency
closure, all five forgettable kinds, both meaningful delete/modify
orientations, operation replay cleanup, changed-input rejection, managed bundle
refresh, SQLite/WAL/temp residue, pre-commit rollback, post-commit sanitation
failure/recovery, restart, clean import, deterministic read/export, format
rejection, and central forgotten-Question dependent validation for import,
`ExplicitMerged`, generated targets, state replacement, and export. Decision
coverage adds direct-write parity, active and forgotten Source authority,
exact Question revision/presentation/choice/outcome/response linkage,
supersession role/shape, current snapshot and correction-history integrity,
export refusal, and no-partial-mutation/reopen behavior.
The current expansion adds the 16-row semantic invariant catalog, direct
postcondition call-path proof, exact Question-response witness lifecycle,
Question-specific unrelated-tombstone rejection, 19 table-driven portable
mutations across import and `ExplicitMerged`, valid semantic/deterministic
round-trip equivalence, and explicit lifecycle/merge/replay/restart metamorphic
mappings. Generated targets and replacement remain production-generated rather
than arbitrarily injectable; representative merge targets and central call-path
checks cover those boundaries. Isolated inconsistent export coverage uses
representative missing-witness, non-user-authority, and forgotten-Question
states rather than attempting to bypass SQLite for every JSON-only mutation.

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

Six Decision pre-fix probes against `3639575f` failed with exit 101 because the
recomputed crafted bundles were accepted: file Source authority, agent-authored
host-turn authority, nonexistent Question revision, invalid alternative,
outcome/response-link mismatch, and Decision-revision provenance substitution.
`ef07e095` corrected the shared production boundary. The expanded test-only V04
run exposed no additional production defect.

The pre-fix probe at `be317b4e` removed the active Decision and response link
from answered and delegated Questions, inserted an unrelated Decision
tombstone, recomputed checksum and lineage, and observed both imports succeed.
`0f30cb69` first centralized existing behavior without changing that result;
`feaad4f5` then introduced the exact content-free witness and removed the
bundle-wide fallback. The subsequent test-only mutation/metamorphic expansion
exposed only one fixture ID-sequence typo before passing; it required no
production behavior change and no weakened mutation.

## Performance and resource observations

The two focused V04 evidence runs completed in `7684.234 ms` and `7792.191 ms`,
including test cataloging, Cargo process startup, 87 integration
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

- Pass: one central Canonical full-state validator governs direct command
  postconditions, import, explicit merged input, generated targets, state
  replacement, and export; the Python harness contains no Canonical rule
  implementation.
- Pass: the 16-row invariant catalog assigns semantic owners, direct
  construction/transition ownership, full-state enforcement, portable and
  forgetting applicability, and executable direct/portable anchors.
- Pass: every answered/delegated Question has one exact, immutable,
  content-free Decision-history witness; open and non-Decision terminal
  Questions reject one, and unrelated Decision tombstones cannot substitute.
- Pass: recomputed single-invariant mutations cross import and
  `ExplicitMerged` as semantic `CorruptState` failures with canonical,
  operational, lineage, binding, managed-output, and reopen preservation.
- Pass: valid command state, Decision/Source/Question forgetting, both
  merge-forgetting orientations, merge round trip, replay after forgetting,
  and restart mappings preserve their maintained semantics.
- Pass: active Decision and correction authority requires a same-Project,
  user-authored `current_host_user_turn` Source. File and agent-authored Sources
  fail both representative direct admission and crafted portable admission.
- Pass: Source forgetting removes the Source body and sensitive metadata while
  retaining the independently Decision-owned authority fact. A Source tombstone
  without that witness is insufficient.
- Pass: active-Question Decisions match an existing exact Question revision,
  alternatives, recommendation key/rationale/Source basis, explicit choice or
  delegation, original response link, and root terminal outcome.
- Pass: linear non-branching acyclic supersession preserves Question identity
  and revision while allowing a superseding choice to differ from the original
  terminal outcome. Orphan, branch, and cycle inputs fail.
- Pass: every active Decision has a complete revision sequence and matching
  current snapshot. Corrections preserve all semantic fields, use valid user
  authorization, and change only permitted rationale presentation.
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
- Pass: valid sanitized import, no-surviving-Decision forgetting, direct
  forgetting, export/import, merge, canonical read, and deterministic re-export
  remain accepted. Active-Question empty presentation is correctly rejected.
- Pass: generated export is validated before publication and does not overwrite
  an existing bundle when internal canonical tables violate forgotten-Question
  or non-user Decision authority invariants.
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

- The Decision-owned authority witness is a narrow canonical provenance fact,
  not a cryptographic signature or proof against a malicious bundle author.
  V04 establishes internal semantic consistency and direct-command parity after
  integrity recomputation, not remote authenticity.
- A minimal forgotten Decision tombstone intentionally carries no Question or
  choice content. The separate Question-specific witness proves only the exact
  root identity, Question/revision, answered/delegated role, response Source
  identity/authority, creation kind, and time; it cannot reconstruct the
  forgotten choice, delegate, rationale, or presentation.
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
- Arbitrary corruption cannot be injected between production merge-target
  generation and replacement. That boundary is covered by central call-path
  checks and representative generated targets; JSON-representable mutations are
  exercised at submitted import and `ExplicitMerged` boundaries, while
  representative inconsistent SQLite states exercise export refusal.
- Passing this maintained evidence does not complete Phase 4 or unblock Phase
  5 by itself. The requested independent audit must craft states outside these
  fixtures, replay focused suites, and reassess the complete gate.

## Recommended implementation choice

Retain the production three-way merge boundary with verified lineage,
canonical-record closure selection, schema-backed operation dependencies, one
shared forgetting sanitation path, the content-free Question-specific
Decision-history witness, Decision-owned forgotten-Source authority, and one
central Canonical full-state validator.
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
- Reject Source-identity-exists-only validation: an active file, repository,
  command, URL, artifact, provider, or agent-authored Source can exist without
  carrying user Decision authority.
- Reject trusting an arbitrary Source tombstone: forgetting removes the actor,
  Source kind, host/session/turn, and payload, so the tombstone alone cannot
  establish prior user authority. The Decision-owned witness is required.
- Reject Question-exists-only validation: identity existence does not prove the
  exact revision, displayed alternatives/recommendation, response Source,
  terminal outcome, or Decision role.
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
scanning remain maintained validation support. Portable Decision semantic
validation, dependency discovery, forgetting, merge, replay, and sanitation are
production Rust responsibilities; no experiment implementation is proposed for
promotion.

## Decision revisit trigger status

Not triggered. Q6 and Q7 remain accepted: all maintained local divergence and
forgetting paths were representable without last-writer authority, implicit
cascade, silent input rewrite, retained managed payload, or unverifiable
Source-tombstone authority. The witness preserves the already accepted exact
current-host user provenance after privacy deletion and does not change a
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

- unrelated-tombstone answered/delegated pre-fix reproduction:
  `rebuild/.local/validation/20260812T221927.691030Z-reproduce-question-history-bypass-ciqpu2cv`
  (exit 0 because both invalid imports were accepted at the reviewed baseline);
- invariant-catalog and mutation prechecks:
  `rebuild/.local/validation/20260812T224451.941093Z-commit3-catalog-precheck-cv06_y3h`
  and
  `rebuild/.local/validation/20260812T224452.058700Z-commit3-mutation-precheck-i25d_hb1`
  (both exit 0);
- test-only fixture failures preserved without production changes:
  `rebuild/.local/validation/20260812T224435.243305Z-commit3-mutation-catalog-precheck-r_ubdezv`
  (exit 101, wrong `LocalBinding` test field) and
  `rebuild/.local/validation/20260812T224532.287111Z-commit3-v04-assertions-evidence-tnpxgrfa`
  (exit 1 after the new equivalence fixture exhausted its injected ID sequence);
- passing V04 invariant-parity evidence and deterministic repeat:
  `rebuild/.local/validation/20260812T224600.482827Z-commit3-v04-assertions-evidence-rerun-at5fm3g9`
  and
  `rebuild/.local/validation/20260812T224619.033410Z-commit3-v04-assertions-repeat-evidence-2vtwz844`
  (both exit 0 with the same 138/13/87 summary);
- final focused catalog, parity/mutation, metamorphic, and complete package
  checks:
  `rebuild/.local/validation/20260812T224914.750424Z-commit3-invariant-catalog-37344wi4`,
  `rebuild/.local/validation/20260812T224914.889755Z-commit3-parity-mutation-ujfhfike`,
  `rebuild/.local/validation/20260812T224919.282032Z-commit3-metamorphic-d5_t5bsn`,
  and
  `rebuild/.local/validation/20260812T224922.293954Z-commit3-context-all-lh8n2374`
  (all exit 0);
- final focused formatting, clippy, architecture-contract, and runner checks:
  `rebuild/.local/validation/20260812T224914.205207Z-commit3-format-8uoup3a0`,
  `rebuild/.local/validation/20260812T224914.411137Z-commit3-clippy-6qlubxcu`,
  `rebuild/.local/validation/20260812T224926.351353Z-commit3-architecture-1wkx6lod`,
  and
  `rebuild/.local/validation/20260812T224926.480809Z-commit3-runner-self-test-45q_q5e_`
  (all exit 0);

- runner self-test:
  `rebuild/.local/validation/20260812T204101.227363Z-runner-self-test-6uafvj61`
  (exit 0);
- Decision pre-fix authority, Question linkage, choice/outcome, and revision
  provenance probes:
  `rebuild/.local/validation/20260812T204333.073104Z-baseline-file-source-decision-accepted-pgcoocw2`,
  `rebuild/.local/validation/20260812T204343.130470Z-baseline-agent-host-turn-decision-accepted-_uqe9aoy`,
  `rebuild/.local/validation/20260812T204343.272082Z-baseline-missing-question-revision-accepted-97qo54v8`,
  `rebuild/.local/validation/20260812T204343.412980Z-baseline-invalid-alternative-accepted-m0ubd23c`,
  `rebuild/.local/validation/20260812T204343.554689Z-baseline-outcome-response-link-accepted-ej3pnmj8`, and
  `rebuild/.local/validation/20260812T204343.699114Z-baseline-decision-revision-provenance-accepted-83n22s0y`
  (each exit 101 because the expected rejection was absent);
- Decision-integrity production assertions and deterministic repeat:
  `rebuild/.local/validation/20260812T205742.025279Z-v04-decision-integrity-assertions-aauki5br`
  and
  `rebuild/.local/validation/20260812T205746.721332Z-v04-decision-integrity-assertions-repeat-bkbgao2v`
  (both exit 0);
- fixture-manifest, report-shape, architecture-contract, and complete
  `volicord-context` checks:
  `rebuild/.local/validation/20260812T210031.396606Z-v04-fixture-manifest-decision-integrity-fvchyjun`,
  `rebuild/.local/validation/20260812T210031.487811Z-v04-report-shape-decision-integrity-2tmmp0o_`,
  `rebuild/.local/validation/20260812T210031.569043Z-v04-architecture-decision-integrity-fz693rxh`, and
  `rebuild/.local/validation/20260812T210031.684976Z-v04-context-tests-decision-integrity-5qxxc3_5`
  (all exit 0);
- post-report runner self-test:
  `rebuild/.local/validation/20260812T210039.661917Z-runner-self-test-46qcxjn3`
  (exit 0).
