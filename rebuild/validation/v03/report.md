# V03 — Canonical Context and portable bundle

## Status

Passed for the maintained disposable fixture. The experiment demonstrates the
required persistence, restart, portability, crash, derived-state deletion, and
managed sensitive-deletion behaviors. It recommends an experimental boundary;
it does not finalize a production schema or promote the prototype.

## Goal

Determine whether Project, Source, Question, Decision, Context Item, and
Checkpoint records can remain durable and portable without Repository
Intelligence, an LLM, a host integration, or any legacy runtime dependency, and
choose evidence-backed storage, revision, bundle, and clone-binding boundaries
for later validation work.

## Accepted decisions being validated

- `open-decisions.md` Q6: generated Project identity, explicit clone binding,
  source-independent bundle reading, and portable canonical relations.
- `open-decisions.md` Q7: non-semantic revision, semantic Decision
  supersession, contradicted/review-due state, deletion, and a non-recoverable
  minimal tombstone.
- Product charter sections 6, 7, 16, and 17 and acceptance scenarios J, K, O,
  and P: canonical/derived separation, restart, portability, correction, and a
  physically separate fresh-service boundary.
- `validation-plan.md` V03: transaction atomicity, deterministic
  serialization, source rebinding, deletion-residue checks, and schema-version
  handling.

No accepted product decision is changed by this report.

## Input repositories and revisions

The repository baseline was
`beda6a5e5faf56935c3106b296ad5f59ff596d2d`. The maintained, self-authored
`v03-canonical-scenario` fixture is the content authority, with SHA-256
`5c4e236e66c20bcd142588497ac62a2aac1d59e53613498bf687bcdfe91651b7`
under the fixture-manifest hashing convention.

The fixture fixes one generated Project ID, primary and another-clone paths,
all six canonical concepts, distinct user and agent provenance, and a unique
sensitive byte sequence. It is declared CC0-1.0.

## Environment and tool versions

- Linux x86-64, WSL2 kernel `6.18.33.2-microsoft-standard-WSL2`.
- Python `3.12.3`.
- Python standard-library SQLite `3.45.1`.
- No dependency was downloaded and no external database, semantic provider,
  network service, or LLM was used.

## Candidate approaches

1. A transactional SQLite candidate stores canonical rows and revision rows,
   keeps clone bindings local, uses foreign keys, `WAL`, `synchronous=FULL`,
   and `secure_delete=ON`, and emits a separately versioned canonical JSON
   bundle. It was exercised for all required V03 behaviors.
2. An atomic canonical-JSON snapshot candidate stores the same versioned bundle
   directly, writes and fsyncs a sibling temporary file, atomically renames it,
   and fsyncs the containing directory. It was exercised for initialization,
   creation/read, deterministic representation, restart, and a SIGKILL before
   publication. The last published snapshot survived byte-identically, while
   an orphan non-canonical temporary file remained for cleanup.

Both candidates were executable. Results were not manufactured for an
unavailable third-party candidate.

## Commands and configuration

The maintained focused commands are:

```text
rebuild/scripts/validate focused v03-fixture-manifest -- rebuild/scripts/check-fixture-manifest rebuild/validation/fixture-manifest.json
rebuild/scripts/validate focused v03-assertions -- rebuild/validation/v03/assertions.py
rebuild/scripts/validate focused v03-assertions-repeat -- rebuild/validation/v03/assertions.py
rebuild/scripts/validate focused v03-report-shape -- rebuild/scripts/check-validation-report rebuild/validation/v03/report.md
```

The assertion program creates all runtime state beneath ignored
`rebuild/.local/v03/`, launches separate hard-terminated helpers, closes and
reopens stores, exports twice, imports into a fresh database, binds another
clone path, deletes and rebuilds derived state, forgets a sensitive record, and
scans every remaining managed runtime file for its exact bytes.

## Observed results

- The fixed Project identity survived close/reopen and export/import.
- Source, Question, Decision, Context Item, and Checkpoint creation/read passed;
  Project is represented separately. A user Decision required an exact user
  turn source, while an agent Checkpoint retained agent/source provenance.
- A Context correction incremented its revision to 2, and both revisions
  survived bundle import. A new user Decision
  semantically superseded the old one with reciprocal `supersedes` and
  `superseded_by` links. `contradicted` and `review_due` remained explicit.
- Two consecutive SQLite exports were byte-identical. The post-forget bundle
  was 4,547 bytes with SHA-256
  `d00d172a6430a50005c361cf83a8c304c72c541f7d47479a0d58767f1e94d49c`.
- Import preserved record identity and relations. The imported store began with
  no local path and accepted only the fixture's explicit another-clone binding;
  the portable `src/lib.rs` Source locator remained unchanged.
- Three helper processes exited by SIGKILL (`-9`). The SQLite transaction
  killed before commit left no record or revision; the committed transaction
  survived. The JSON candidate killed before publication retained the prior
  snapshot byte-for-byte.
- Complete removal of the derived directory did not change the canonical
  record count or readability.
- Forgetting removed the sensitive canonical row and its revisions, erased the
  derived index and recoverable bounded log, rewrote both managed bundles,
  checkpointed/truncated journals, vacuumed the databases, and left only a
  tombstone containing record ID, Project ID, record kind, and deletion time.
  A byte scan of database files, bundles, indexes, logs, journals, tombstones,
  and JSON-candidate artifacts found zero sensitive matches.
- The observed SQLite file was 53,248 bytes. The published JSON snapshot was
  563 bytes; its SIGKILL left an 895-byte orphan temporary file containing no
  sensitive fixture value.

## Coverage and failures

Every V03-required canonical concept and state was exercised. Coverage includes
stable identity, explicit path binding, provenance rejection, revision,
supersession, contradiction, review due, deterministic export/import, schema
and format fields, another-path rebinding, restart, pre/post-commit SIGKILL,
derived removal, managed forgetting, and token-based absence of legacy runtime
dependencies in the executable prototype and fixture.

No maintained assertion failed in the evidence run. Earlier disposable draft
runs exposed that imported managed state also needed explicit forgetting; the
final assertion performs that deletion rather than concealing the residue.
V04 divergence and conflict merge, production migrations, and schema upgrade
paths are intentionally not covered.

## Performance and resource observations

The two final focused runs completed their internal assertions in `155.037 ms`
and `155.583 ms` (`182.404 ms` and `182.176 ms` including the validation
runner). They created two 53,248-byte
SQLite stores, two 4,547-byte managed bundles after forgetting, and one
563-byte published JSON snapshot. These are small-fixture observations, not
benchmarks. Peak memory and concurrent-writer throughput were not measured.

## Privacy and external transmission

All source and state were self-authored and local. No network request, external
provider, source transmission, or raw source copy occurred. Complete command
stdout/stderr and exit outcomes are in ignored repository-local validation
artifacts. The unique sensitive value was not placed in an argument or report,
and the final runtime-wide byte scan returned an empty match list.

## Acceptance results

- Pass: stable generated Project identity and explicit clone/path binding.
- Pass: creation/read and user-versus-agent provenance separation.
- Pass: correction revision, semantic Decision supersession, contradiction,
  and review-due representation.
- Pass: deterministic versioned bundle export/import with identity and relation
  preservation.
- Pass: explicit another-path Source rebinding after import.
- Pass: process restart and hard-termination recovery of only committed state.
- Pass: deleting all rebuildable derived state preserved canonical records.
- Pass: managed sensitive deletion produced no byte match in bundles, indexes,
  logs, journals, databases, JSON artifacts, or the minimal tombstone.
- Pass: the experiment has no legacy runtime, schema, identifier, importer,
  detector, command alias, dual-read, or legacy crate dependency.

## Known limits

- The fixture is small, single-process during ordinary writes, and uses fixed
  identifiers and timestamps; concurrency, lock contention, large bundles,
  corruption repair, and schema upgrades are not measured.
- `secure_delete`, WAL checkpointing, and vacuum were observed on SQLite 3.45.1
  and the tested filesystem; other filesystems and backup/snapshot layers can
  retain bytes outside the managed experiment root.
- Forgetting rewrites registered managed bundles, but cannot revoke arbitrary
  bundle copies a user already moved elsewhere. Cross-clone deletion
  propagation belongs to V04 and remains a Q7 revisit concern if it proves
  insufficient.
- The JSON candidate needs orphan-temp cleanup after termination and rewrites
  the full canonical snapshot for each publication.
- The prototype does not define production APIs, authorization, encryption,
  multi-project layout, merge semantics, or a final tombstone policy.

## Recommended implementation choice

Use transactional SQLite as the next experimental shared persistence basis and
a separate deterministic, human-inspectable canonical JSON bundle as the
portability boundary. Keep explicit schema and format names plus integer
versions in both. Keep clone/path bindings local and out of portable bundle
bytes; import preserves Project and Source identities, then an explicit bind
associates the current clone.

Treat non-semantic correction as a new revision of the same record. Treat a
semantic user-choice change as a new Decision with reciprocal supersession
links. Preserve contradiction/review state without rewriting facts. Delete a
sensitive record and recoverable revisions transactionally, retain only the
minimal non-content tombstone when needed, invalidate derived data, and compact
managed storage before claiming forget completion.

This recommendation selects experiment boundaries only. Production
architecture remains gated on review and later validations.

## Rejected alternatives and reasons

- Reject direct canonical JSON snapshot files as the shared persistence basis
  for V05: the experiment preserved the last published snapshot, but every
  update rewrites the entire state, clone bindings need another local store,
  multi-record transaction/query behavior is weak, and SIGKILL leaves an
  orphan temporary file requiring recovery cleanup.
- Reject database files as portable bundles: SQLite recovery and local binding
  details are useful runtime concerns but would expose journals, page layout,
  compaction behavior, and implementation schema as the portability contract.
- Reject in-place semantic Decision edits: they hide the prior user judgment
  that the accepted decisions require supersession to preserve.
- Reject path- or remote-derived Project identity and portable clone paths:
  another-path import demonstrated that stable identity plus explicit local
  binding is sufficient without conflating paths with Projects.

## Reusable primitive decision

`experimental_test_support`. V05 may import the V03 prototype explicitly to
avoid duplicate transaction and restart machinery. No source in
`rebuild/validation/v03/` is approved for production promotion, and
`rebuild/crates/volicord-context` remains an unchanged bootstrap shell.

## Decision revisit trigger status

Not triggered. The experiment represented the accepted Project, bundle,
revision, supersession, and deletion boundaries without narrowing them. The
known limitation for deletion propagation is assigned to V04; it does not yet
show that Q6 or Q7 is infeasible. No product question is reopened.

## Follow-up work

- Use only the SQLite experiment boundary as V05 persistence support and keep
  the dependency visibly under `rebuild/validation/`.
- Exercise divergent bundle additions, delete/modify conflicts, and deletion
  propagation in V04.
- Before production promotion, define concurrency, corruption, upgrade,
  encryption/privacy, cleanup, repair, and multi-project responsibilities with
  new tests and dependency review.
- Measure larger bundles and filesystem-specific residue behavior.

## Artifacts

Maintained inputs are the fixture-manifest entry,
`rebuild/validation/fixtures/v03/canonical-scenario.json`, `prototype.py`,
`assertions.py`, and this report. Raw evidence remains ignored:

- `rebuild/.local/v03/assertions-qkgrrexg/summary.json`, SHA-256
  `73239945940d6f5357aa7d733981f4460fa4a552a63bc91dc06f99f9571213d7`;
- repeat summary `rebuild/.local/v03/assertions-35ou63hw/summary.json`,
  SHA-256
  `843622233163830bf557c6b81c3982874cbe35ab0e7bc2d88ce5385a8ee322b9`;
- managed post-forget bundle, SHA-256
  `d00d172a6430a50005c361cf83a8c304c72c541f7d47479a0d58767f1e94d49c`;
- final focused assertions:
  `rebuild/.local/validation/20260808T203004.048691Z-v03-assertions-final-oeum4ytr`;
- deterministic repeat:
  `rebuild/.local/validation/20260808T203004.049015Z-v03-assertions-repeat-final-glawy2hu`;
- fixture and report checks:
  `rebuild/.local/validation/20260808T203004.048630Z-v03-fixture-manifest-final-5e0nhc2f`
  and
  `rebuild/.local/validation/20260808T202829.267348Z-v03-report-shape-t1pw91jj`.
