# `volicord-context` implementation basis

This crate is the synchronous Canonical Context Kernel. Its current production
scope is deliberately narrow: an explicit-path SQLite store can create and
reopen stable Projects, maintain local clone bindings, record typed Sources and
provenance, persist explicit Source relations, and durably record Questions and
atomic explicit user Decisions. This file records concrete
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
- The open path applies and verifies foreign keys, WAL journal mode,
  `synchronous=FULL`, and `secure_delete=ON`. Linux crash and filesystem residue
  behavior still belongs to later destructive fault validation.
- Schema metadata is `{ kind = "volicord-context", version = 2 }`. An existing
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
  result kind, result identity, result revision, Project, and commit time. The
  same operation and input replays its immutable prior result; different input
  conflicts. Preconditions produce typed stale-basis errors. The kernel does
  not retry a canonical write under another identity.
- Question identity is independent of its prompt basis. Revision-one Question
  content is immutable and includes exact Source/dependency basis, displayed
  alternatives, agent recommendation, trade-offs, uncertainty, material scope,
  and one of the accepted terminal outcomes.
- A Question response operation accepts only an already explicit alternative,
  delegation, or non-Decision terminal outcome. It atomically creates or
  validates a current-host user-turn Source, records the exact revision link,
  creates a Decision when required, transitions the Question, and records the
  operation outcome. Recommendation and user choice remain separate fields.
- Source payloads cover repository snapshots and commits, files, symbols,
  bounded command outcomes, current-host user turns, URLs, and adopted
  artifacts. They preserve typed actor and optional observer provenance, but
  provide no field for full prompts, full tool arguments, entire Source bodies,
  unlimited command streams, or secret-bearing environments.

Portable bundle encoding is not implemented in this slice. The reserved later
boundary is kind `volicord-context-bundle`, format version `1`, using
deterministic UTF-8 JSON plus a corruption-detection checksum as accepted by the
workstream.

## Dependency review

The production dependency set has two direct crates:

| Dependency | Selection and purpose | Rust/Linux evidence | License and behavior |
|---|---|---|---|
| `rusqlite 0.32.1` with `bundled` | Maintained synchronous SQLite API and a pinned bundled SQLite build, avoiding a runtime service or system-library version dependency. | Edition 2021 source and its locked dependency graph have declared minimum Rust versions no newer than 1.65 where specified; it builds on Linux under the workspace's Rust 1.85 contract. | MIT. No network client, telemetry, async runtime, or external-service behavior. Bundled SQLite is public domain. |
| `getrandom 0.3.4` with `std` | Fills exactly 16 bytes from the OS for each production identity; no general RNG hierarchy is introduced. | Declares Rust 1.63 and has a native Linux backend. | MIT OR Apache-2.0. No telemetry or external-service behavior. |

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
```

`cc` uses the build-only `find-msvc-tools` and `shlex` crates, and `ahash` uses
build-only `version_check`. On the supported Linux target the bundled feature
compiles SQLite locally and does not invoke `pkg-config` or `vcpkg` to discover
a runtime database. The lockfile also contains the test-only `tempfile` graph;
it is not linked into the production crate.

The direct licenses were checked from the published crate manifests and license
files. Cargo metadata/tree review confirms there is no dependency on a legacy
Volicord crate and no production package outside `rebuild/`.
