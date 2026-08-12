# `volicord-context` implementation basis

This crate is the synchronous Canonical Context Kernel. Its current production
scope is deliberately narrow: an explicit-path SQLite store can create and
reopen stable Projects, maintain local clone bindings, record typed Sources and
provenance, persist explicit Source relations, and durably record Questions and
atomic explicit user Decisions, typed Context Items, and source-grounded
Checkpoints, lifecycle revisions, supersession, contradiction, review-due state,
and user-authorized forgetting. This file records concrete
Phase 4 implementation choices; the product and architecture meaning remains
owned by `rebuild/docs/design/`.

## Runtime and persistence choices

- Rust 1.85, edition 2021, with synchronous APIs only.
- One caller-supplied database path. The crate does not inspect the current
  directory, repository root, `VOLICORD_HOME`, or any legacy runtime location.
- One transactional SQLite database can contain multiple Projects. Project,
  Source, binding, relation, revision, and operation rows carry or are rooted
  in a typed `ProjectId` scope.
- Every canonical mutation takes a caller-supplied `OperationId`, acquires an
  immediate transaction, and commits one domain operation. Writers are
  serialized by the mutable store handle and SQLite's writer transaction.
- `canonical_state` is the single complete-Project semantic validation owner.
  Direct commands validate their resulting Project view after recording the
  operation and before transaction commit. Production export, decoded bundle
  admission, generated merge targets, Project-state replacement, and explicit
  merged input all use the same boundary; transition preconditions and local
  operation/binding invariants remain separately named responsibilities. The
  maintained mapping is in `INVARIANTS.md`.
- The open path applies and verifies foreign keys, WAL journal mode,
  `synchronous=FULL`, and `secure_delete=ON`. Linux crash and filesystem residue
  behavior still belongs to later destructive fault validation.
- Schema metadata is `{ kind = "volicord-context", version = 11 }`. An existing
  malformed store or any non-current version is rejected before durability
  configuration or canonical mutation; no older-schema or legacy decoder is
  present.
- Project and local-binding revisions are append-only history. Sources are
  immutable revision-one records. Relations are explicit, directed, and
  Project-scoped.
- Local absolute clone paths and current availability remain in local binding
  tables. Typed Source payloads hold portable locators or snapshot bases
  separately, so rebinding does not rewrite historical Source provenance.
- IDs are distinct 128-bit Rust newtypes. Production entropy comes directly
  from the operating system; tests can inject a finite deterministic sequence.
- Persisted UTC time uses signed Unix microseconds from an injected clock.
  Timestamps do not generate identity, order conflicting writes, or choose a
  winner.
- Operation rows preserve the exact bounded input basis, committed outcome,
  result kind, result identity, result revision, Project, and commit time. A
  schema-backed dependency relation records every canonical identity whose
  content is embedded in each input basis, including import and merge bundle
  bases. The same available operation and input replays its immutable prior
  result; different input conflicts. Forgetting any dependency replaces the
  whole input basis with a content-free state, removes its dependency rows,
  retains operation/result identity for duplicate prevention, and makes replay
  deterministically return `NotFound` rather than reconstructing content or
  reporting success. Preconditions produce typed stale-basis errors. The kernel
  does not retry a canonical write under another identity.
- Question identity is independent of its prompt basis. Revision-one Question
  content is immutable and includes exact Source/dependency basis, displayed
  alternatives, agent recommendation, trade-offs, uncertainty, material scope,
  and one of the accepted terminal outcomes.
- A Question response operation accepts only an already explicit alternative,
  delegation, or non-Decision terminal outcome. It atomically creates or
  validates a current-host user-turn Source, records the exact revision link,
  creates a Decision when required, transitions the Question, and records the
  operation outcome. Recommendation and user choice remain separate fields.
  Every Decision and correction snapshot also owns the narrow
  `current_host_user_turn` authority fact established by that direct command.
  This fact survives Source forgetting without retaining the forgotten turn,
  host, session, actor identity, or Source payload; an active Source must still
  independently prove the same Project, Source kind, and user actor.
  Choice and delegation also create exactly one immutable
  `question_decision_history_witnesses` row in the same transaction. It records
  only Project, exact Question revision, original root Decision, answered or
  delegated outcome, response Source identity, content-free authority/creation
  provenance, and creation time. It contains no prompt, alternatives,
  recommendation, choice/delegate value, rationale, Source payload, or content
  hash. Non-Decision terminal outcomes create no such witness.
- Context Items preserve one of eight statement roles independently from user,
  observation, agent, or generated-interpretation provenance. Facts require
  repository/command observation; explicit preferences require a current-host
  user-turn Source; generated interpretations cannot be relabeled as facts.
- Checkpoints accept only caller-supplied supporting, changed, Decision,
  Question, and command-verification bases. Completion, pause, and handoff are
  meaningful boundaries, while work, automated verification, user review, and
  user acceptance remain separately represented and source-linked as needed.
  The kernel performs no repository or dirty-worktree observation.
- Context Item and Decision corrections append immutable revision snapshots
  under the same identity. Their typed inputs expose only non-semantic text,
  and bounded formatting/typography/token checks reject semantic replacement.
  Decision choice, delegation, Question linkage, applicability, assumptions,
  and revisit triggers have no in-place correction path.
- A changed Decision is a new identity with a directed `supersedes` relation.
  History order and the single unsuperseded current selection use stable
  database ordering rather than timestamps as conflict authority.
- Contradiction retains both records and their Source basis without selecting a
  winner. Review-due state records the changed basis while leaving the Decision
  readable and valid pending explicit review.
- Record-level forgetting supports every content-bearing canonical record kind:
  Source, Question, Decision, Context Item, and Checkpoint. Project remains the
  Project scope and identity root; whole-Project deletion is not supported.
  Every typed operation requires a current-host user-turn Source, preserves
  Project scoping, deletes active and revision content plus content-bearing
  replay input, rewrites owned links, refreshes managed bundles, and leaves
  only Project, record kind, record identity, and forgetting time in the
  tombstone. The content-free Question response-history witness survives
  Decision or Source forgetting so the exact original response role remains
  verifiable; it is not changed by Decision correction or supersession. Source
  and Question identities may remain only as tombstoned IDs
  in surviving historical Decision or relation references; Source payload,
  Question prompt/state support, dependencies, Checkpoint links, and other raw
  content do not. Forgetting a Question also clears Question-owned displayed
  alternatives and recommendation fields from surviving Decisions and every
  immutable Decision revision while preserving the user's choice, rationale,
  applicability, provenance, and tombstoned Question identity; the Decision is
  marked review due because its presentation basis is no longer interpretable.
  Each operation returns the same narrow invalidation result
  for later Candidate/Derived owners.
- On supported Linux SQLite, forgetting commits with `secure_delete=ON`, then
  checkpoints/truncates WAL, vacuums the database, and checkpoints again. Tests
  scan the managed database, WAL/SHM, and sibling files for the forgotten bytes
  after reopen. This managed guarantee does not cover user copies, filesystem
  snapshots, cloud/provider retention, backups, or other clones.
- Source payloads cover repository snapshots and commits, files, symbols,
  bounded command outcomes, current-host user turns, URLs, and adopted
  artifacts. They preserve typed actor and optional observer provenance, but
  provide no field for full prompts, full tool arguments, entire Source bodies,
  unlimited command streams, or secret-bearing environments.

## Portable bundle choices

- The portable representation is kind `volicord-context-bundle`, format
  version `4`. This is the only readable and writable version. Older or newer
  versions are rejected before mutation; there is no legacy decoder or
  migration path.
- The payload is compact UTF-8 JSON with one LF terminator, fixed lexicographic
  object-key order, stable table/row/column ordering, and hexadecimal typed
  identities/bytes. It contains no per-export current timestamp. SHA-256 covers
  the canonical payload only for corruption detection; the format has no
  signature, encryption, compression, or remote synchronization behavior.
- A Project-scoped semantic allowlist carries Project and Source manifest data,
  Questions, Decisions, Context Items, Checkpoints, revision snapshots,
  relations, review state, supersession, and minimal tombstones. This mapping is
  independent from the runtime database file and excludes schema metadata,
  operation replay rows, local bindings, managed output paths, journals,
  Candidates, Derived State, indexes, caches, layouts, previews, raw tool
  traffic, full transcripts, and raw Source bodies.
- The lineage object records a SHA-256 history basis for the exact Project
  semantic state and the first exported or imported basis from which the local
  history diverged. Merge comparison accepts only an explicitly supplied base
  whose identity matches both histories; file names, paths, timestamps, and
  import order never establish ancestry.
- Repository-bound Source manifest entries import with `unavailable` local
  availability. Canonical records and historical Source identity remain
  readable without a repository, and an explicit later bind can establish a
  different local absolute path without changing portable bytes.
- Import verifies kind/version, checksum, Project scope, exact table contract,
  duplicate keys, lineage, relation endpoints, local equality, and conflict
  possibility before commit. It inserts exact identities in one immediate
  transaction, reports already-present state, records the checksum as a bounded
  replay basis with dependencies on every imported canonical content identity,
  and rejects divergent merge rather than choosing a winner. Forgetting any of
  those identities removes the checksum basis before managed deletion hygiene.
- One decoded portable Decision semantic index validates active/tombstoned
  Sources and Questions, exact Question revisions and response links, Decision
  revisions, correction authorization, directed non-branching supersession,
  terminal outcomes, and review-due state. It requires active authority Sources
  to be user-authored current-host turns, accepts a tombstoned authority Source
  only with a content-free authority witness, requires exactly one
  Question-specific response-history witness for every active answered or
  delegated Question, and verifies that its exact root Decision, outcome,
  response authority, and linear supersession lineage agree. An unrelated
  Decision tombstone cannot satisfy another Question. The validator also
  verifies exact Question presentation and rejects revision gaps,
  current-snapshot mismatch, or semantic mutation disguised as correction. The
  same boundary is reached by import, explicit merged input, generated merge
  targets, state replacement, and export before mutation or publication.
- Publication writes a fixed sibling temporary candidate, syncs it, atomically
  renames it over the final path, and syncs the containing directory. Import
  rejects the temporary name as non-authoritative. A regular orphan candidate
  is cleaned on the next export; an unexpected directory or other obstruction
  fails without replacing the prior final bundle.
- Successfully published paths are local managed state and never enter bundle
  bytes. User-authorized forgetting republishes those current managed bundles
  after SQLite deletion hygiene, removing forgotten raw content from both
  managed representations while retaining only the minimal tombstone. If a
  local obstruction prevents that post-commit refresh, forgetting reports
  `RepairRequired` explicitly rather than claiming complete managed cleanup.

## Divergent merge choices

- Comparison is an explicit three-way operation over local state, an incoming
  bundle, and a caller-selected base bundle. The base is trusted only when its
  history identity matches the lineage declared by both sides. A missing or
  mismatched base is reported as `common_base_unavailable`; paths, file names,
  timestamps, and import order are never ancestry evidence.
- Results use the six portable conflict classes and expose affected identities,
  all three history bases, Source availability, consequence, uncertainty, and
  the automatic/user-owned boundary. Independent rows and a one-sided verified
  correction can advance automatically. Question meaning/state, Decision
  meaning/applicability/supersession, delete/modify, competing bindings, and
  unverified histories cannot.
- An explicit resolution is bound to conflict-set identity and revision and to
  a current-host user-turn Source in the same Project. The supported outcomes
  choose local, choose incoming, accept an explicitly constructed merged
  bundle, or retain the local context with an inspectable incoming branch
  basis. No timestamp, last-writer, access-frequency, path-similarity, or model
  recommendation fallback exists.
- One merge replaces the selected Project-scoped portable state and records its
  bounded operational provenance in the same immediate transaction. When that
  target newly forgets a locally active identity, the transaction invokes the
  same dependency and copied-content sanitation used by direct forgetting,
  erases the merge input and any pre-merge history verifier, and durably marks
  managed sanitation `pending`. Only after WAL checkpoint/truncation, database
  compaction, temporary-candidate cleanup, and refresh of every registered
  bundle does it mark sanitation `complete` and return initial success. A
  post-commit failure returns `RepairRequired`; replay completes the pending
  sanitation without repeating canonical mutation. Once the forgotten input is
  erased, replay and changed-input reuse are both rejected with `NotFound`
  because equality can no longer be verified safely. Merges that select no new
  forgetting retain normal same-input replay and changed-input conflict.

## Deterministic canonical read basis

- `read_canonical_basis` is a read-only, provider- and analyzer-independent
  input contract for later Recall. It returns Project identity, active and
  terminal Questions, active and superseded Decisions with applicability,
  assumptions, contradictions and review-due state, Context Items, the latest
  Checkpoint and optional history, Sources with snapshot/availability/freshness,
  revision and canonical relations, tombstones, and merge/branch provenance.
- Canonical entity collections use typed identity bytes as their total order.
  Checkpoint chronology uses durable UTC microseconds with identity as the
  tie-breaker. Relations, tombstones, revisions, and merge bases use explicit
  tuple ordering. No local path, locale, timezone rendering, filesystem or map
  enumeration, process observation, access frequency, retrieval score, or model
  output participates.
- The basis contains no natural-language brief, ranking, budget, automatic
  session trigger, Candidate view, or user/agent projection. It remains usable
  after restart, source loss, portable import, another-path binding, merge, and
  branch resolution without Repository Intelligence, a provider, or Derived
  State.

## Dependency review

The production dependency set has four direct crates:

| Dependency | Selection and purpose | Rust/Linux evidence | License and behavior |
|---|---|---|---|
| `rusqlite 0.32.1` with `bundled` | Maintained synchronous SQLite API and a pinned bundled SQLite build, avoiding a runtime service or system-library version dependency. | Edition 2021 source and its locked dependency graph have declared minimum Rust versions no newer than 1.65 where specified; it builds on Linux under the workspace's Rust 1.85 contract. | MIT. No network client, telemetry, async runtime, or external-service behavior. Bundled SQLite is public domain. |
| `getrandom 0.3.4` with `std` | Fills exactly 16 bytes from the OS for each production identity; no general RNG hierarchy is introduced. | Declares Rust 1.63 and has a native Linux backend. | MIT OR Apache-2.0. No telemetry or external-service behavior. |
| `serde 1.0.228` with `derive` | Defines the private, typed portable envelope and payload mapping. | Supports the workspace Rust version and is exercised on Linux by deterministic round-trip tests. | MIT OR Apache-2.0. No I/O, network, telemetry, or runtime service behavior. |
| `serde_json 1.0.150` | Emits and validates the deterministic UTF-8 JSON representation; ordering comes from fixed structs and ordered arrays rather than hash maps. | Supports the workspace Rust version and is exercised by byte-identity and corruption tests. | MIT OR Apache-2.0. No network, telemetry, signature, encryption, or compression behavior. |

The locked normal/build transitive shape is small and local:

```text
getrandom
├── cfg-if
└── libc

rusqlite
├── bitflags
├── fallible-iterator
├── fallible-streaming-iterator
├── hashlink → hashbrown → ahash → (cfg-if, once_cell, zerocopy)
├── libsqlite3-sys → build-only (cc, pkg-config, vcpkg)
└── smallvec

serde → serde_core, optional derive macros
serde_json → serde_core, itoa, memchr, zmij
```

`cc` uses the build-only `find-msvc-tools` and `shlex` crates, and `ahash` uses
build-only `version_check`. On the supported Linux target the bundled feature
compiles SQLite locally and does not invoke `pkg-config` or `vcpkg` to discover
a runtime database. The lockfile also contains the test-only `tempfile` graph;
it is not linked into the production crate.

The direct licenses were checked from the published crate manifests and license
files. Cargo metadata/tree review confirms there is no dependency on a legacy
Volicord crate and no production package outside `rebuild/`.
