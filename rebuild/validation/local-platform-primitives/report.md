# V10 — Local platform primitive qualification

## Status

Passed on Linux at the recorded HEAD. The maintained assertions reproduce every accepted behavior below and validate a final classification for each candidate.

## Goal

Qualify only domain-independent process and filesystem responsibilities needed by later Local Operations, and classify legacy storage patterns without creating another canonical storage authority.

## Accepted decisions being validated

- D3 and Q6: Project identity stays independent of local repository path, worktree, clone, and remote similarity.
- D5 and D11: derived observations and fingerprints stay separate from canonical meaning.
- Q8-A and Q8-B: Linux is the first supported platform and the new workspace has no legacy dependency or Runtime Home path.
- Q13: no accepted Decision is reopened unless its recorded basis or revisit trigger changes.
- `failure-and-recovery.md`: complete stdout, complete stderr, exit/termination, duration, timeout, cancellation, and child-tree cleanup are separate observable facts.
- `versioning-policy.md`: atomic publication and schema checks do not become legacy migration or a second storage engine.

## Input repositories and revisions

- Repository HEAD: `4131a965678f9fcbff77b85f04a0a47d6b643896`.
- Fixture `v10-local-platform-primitives`, whose content hash is catalogued in `rebuild/validation/shared/fixture-manifest.json`.
- Legacy sources are inspected and executed only as reference candidates: `volicord-platform-process`, `volicord-test-process`, the independent portions of `volicord-platform-fs`, and storage patterns in `volicord-store`.

## Environment and tool versions

- Required environment: Linux with `/proc`, process groups, Python 3, SQLite, Git, and Rust 1.85 or newer.
- The probe records the exact Git version and validation runner records the command, timestamps, duration, exit/termination, and complete separate streams.
- No external provider or network access is used.

## Candidate approaches

1. Move an existing legacy crate unchanged: rejected because filesystem and test-process crates retain dependencies or output semantics outside the new responsibility.
2. Extract a mechanically independent function: accepted only for atomic no-replace publication, whose source/destination/effect/durability contract is already responsibility-bounded.
3. Reimplement observed Linux behavior behind reconstruction-owned types: selected for process supervision, path containment, local Git coordinates, and source fingerprints.
4. Keep complex observer/storage code as design and failure reference: selected where production owners already exist or legacy meaning is entangled.

## Commands and configuration

Run each command through the focused validation runner from the repository root:

```text
rebuild/scripts/validate focused v10-assertions -- python3 rebuild/validation/local-platform-primitives/assertions.py
rebuild/scripts/validate focused v10-report -- rebuild/scripts/check-validation-report rebuild/validation/local-platform-primitives/report.md
rebuild/scripts/validate focused v10-fixtures -- rebuild/scripts/check-fixture-manifest rebuild/validation/shared/fixture-manifest.json
rebuild/scripts/validate focused v10-legacy-platform-process -- cargo test -p volicord-platform-process
rebuild/scripts/validate focused v10-legacy-test-process -- cargo test -p volicord-test-process
rebuild/scripts/validate focused v10-legacy-filesystem -- cargo test -p volicord-platform-fs --lib no_replace_file_publication
```

The Python probe uses an isolated temporary directory, a self-authored Git repository, linked worktree and clone, and a disposable SQLite database. It does not use a Runtime Home.

## Observed results

- Process: complete stdout and complete stderr remained byte-distinct, numeric exit 23 remained failure, and monotonic duration was positive.
- Timeout/cancellation: the timeout trigger, termination request, signal result, and descendant observation remained separate. Linux process-group termination stopped the pipe-holding child-tree; timeout alone was never recorded as cleanup confirmation.
- Paths and symlinks: normalization was deterministic and a repository-relative path traversing an outward symlink resolved outside the root and was rejected.
- Repository/worktree/clone: a primary and linked worktree shared one local clone coordinate but retained distinct worktree coordinates. A separate clone with the same commit retained a different local clone coordinate. None was treated as Project identity.
- Dirty observation: porcelain-v2 bytes changed after a worktree edit and their opaque digest changed without attributing the edit to the observer.
- Fingerprint: regular-file bytes/mode and symlink target used distinct typed hash domains.
- Atomic publication: one complete staged file became visible without replacing an existing destination; a losing staged source remained available for caller-owned cleanup.
- Storage: a process exit inside an uncommitted SQLite transaction left no row after reopen, committed schema metadata remained intact, and integrity check returned `ok`.

## Coverage and failures

Covered: Linux process groups, binary stdout/stderr, nonzero exit, timeout-triggered kill, descendant termination observation, normalization, symlink escape, Git primary/linked/separate-clone coordinates, dirty status, typed fingerprints, no-replace publication, transaction crash, schema metadata, and repair/rebuild classification.

Unsupported or excluded: Windows/macOS behavior, cgroup/subreaper guarantees, PID-namespace escape, network filesystems, hostile concurrent path replacement beyond primitive result reporting, full Git observer parity, and legacy schema recovery. No probe failure was hidden as success.

## Performance and resource observations

The maintained probe is bounded by a 150 ms lifecycle timeout and two-second descendant observation window. The focused runner owns exact elapsed duration and output artifact sizes. No production performance or large-repository scaling claim is made.

## Privacy and external transmission

All fixtures, processes, Git repositories, streams, and SQLite bytes remain local. Temporary runtime output is deleted by the probe or preserved only under ignored `rebuild/.local/validation/`; no provider is invoked and no source is transmitted.

## Acceptance results

- PASS: process containment can be defined without Task, UserAction, Write Ticket, Evidence, Guard admission, or Runtime Home types.
- PASS: complete streams, failure status, signal termination, duration, timeout trigger, cancellation intent, and cleanup observation can remain distinct.
- PASS: Linux child-tree cleanup is directly testable.
- PASS: path/symlink, local clone/worktree, dirty-state, fingerprint, and atomic-publication semantics can be responsibility-bounded.
- PASS: storage evidence supports the existing production canonical owner and does not justify another engine.
- PASS: reconstruction workspace independence can be preserved because no legacy crate needs promotion as a dependency.

## Known limits

- A process group is containment for cooperative local children, not a security sandbox and not a cgroup/subreaper guarantee.
- A timeout is only a trigger. Production must report cleanup as confirmed, incomplete, or unknown from subsequent observations.
- Local clone/worktree hashes are operational coordinates, never portable Project identity.
- Atomic namespace publication and parent-directory durability are separate outcomes.
- Full-stream artifacts remain subject to the privacy/retention owner; “complete” does not mean canonical or indefinitely retained.

## Recommended implementation choice

Create one small reconstruction-owned local platform crate. Its process boundary should use Linux process groups, drain complete streams to separate caller-selected operational artifacts, preserve numeric exit or signal termination and monotonic duration, and report timeout/cancellation/cleanup facts independently. Its filesystem boundary should reject symlink escapes, expose only local clone/worktree observations, use typed source fingerprints, and adopt atomic no-replace publication with a distinct durability result.

Do not move the legacy test harness, full repository observer, mutation lease, Store, schema, or repair API. The existing `volicord-context` crate remains the only canonical storage engine.

## Rejected alternatives and reasons

- Whole `volicord-platform-process` crate: its low-level polling API does not own the required complete operational result or cancellation truth.
- `volicord-test-process` as production: it intentionally bounds captures and reports omitted bytes, so it cannot preserve complete streams.
- Whole `volicord-platform-fs` crate: it depends on legacy `volicord-types` and process code and contains Runtime Home mutation admission.
- Whole repository observer: its product-relative types and high-complexity Git normalization belong to legacy behavior; current Repository Intelligence already owns analysis snapshots.
- Legacy Store: schema, IDs, Runtime Home, workflow authority, and repair assumptions are excluded, while the new canonical engine already owns transaction/version behavior.

## Reusable primitive decision

| Candidate | Final classification | Evidence-backed boundary |
|---|---|---|
| `legacy-platform-process` | `reimplement_from_behavior` | Retain Linux process-group and nonblocking-drain behavior, but define truthful new result/cancellation semantics. |
| `legacy-test-process` | `reference_only` | Retain test cases for streams, exit, timeout, and descendants; bounded capture is not the production contract. |
| `legacy-path-containment` | `reimplement_from_behavior` | Retain canonical-root and symlink-escape observations without Product Repository or workflow types. |
| `legacy-git-layout` | `reimplement_from_behavior` | Retain primary/linked worktree distinction as a local coordinate only. |
| `legacy-repository-observer` | `reference_only` | Retain dirty/failure cases; do not duplicate Repository Intelligence or legacy types. |
| `legacy-content-fingerprint` | `reimplement_from_behavior` | Retain typed, length-delimited SHA-256 behavior under a new source-observation responsibility. |
| `legacy-atomic-no-replace-publication` | `adopt_as_new_primitive` | Move the same-parent ordinary-file validation, Linux no-replace rename, effect, and parent-sync contract into a narrow module with new tests. |
| `legacy-runtime-home-mutation-lease` | `reject` | It is explicitly coupled to legacy Runtime Home admission. |
| `legacy-store-transaction-patterns` | `reference_only` | Transaction/crash/fault ideas inform tests only; `volicord-context` owns production canonical transactions. |
| `legacy-store-schema-and-repair` | `reject` | DDL, numeric legacy dispatch, Runtime Home recovery, and authority semantics are excluded. |

## Decision revisit trigger status

Decision revisit trigger: not triggered. Linux behavior supports the accepted Local Operations and recovery boundaries without narrowing Project identity, portability, polyglot Repository Intelligence, local-only operation, or legacy non-compatibility.

## Follow-up work

- Promote only the qualified process, path/local-Git/fingerprint, and atomic-publication responsibilities in the next commit.
- Add responsibility-level Rust tests for complete artifacts, exit/signal, timeout/cancellation truth, descendant cleanup, symlink escape, local clone/worktree identity, dirty changes, typed fingerprint, concurrent no-replace publication, and parent-sync failure effect.
- Leave CLI, orchestration, providers, viewer, packaging, V08, V11, and dogfood to their owning phases.

## Artifacts

- Maintained: this report, `fixtures/scenarios.json`, `probe.py`, `assertions.py`, and the shared fixture-manifest entry.
- Ignored: `rebuild/.local/validation/<focused-run>/command.json`, `result.json`, complete `stdout.log`, and complete `stderr.log`.
