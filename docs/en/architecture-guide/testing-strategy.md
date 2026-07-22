# Testing Strategy

Tests protect owner-defined current behavior. They do not create product
contracts, preserve removed surfaces, or justify broader support claims.

## Choose The Narrowest Layer

| Test layer | Use it for |
|---|---|
| Unit test | Pure parsing, canonical encoding, closed values, and policy decisions. |
| Crate integration test | Adapter boundaries, Store reads/writes, process behavior, and strict persisted-record rejection. |
| Conformance test | Public cross-method outcomes, error categories, replay, effects, and projections. |
| Release-integrity test | Volicord target, version, package, checksum, and workflow invariants. |
| Documentation check | Owner routing, links, terminology, parity, examples, and generated-source drift. |

Use disposable Runtime Homes and Product Repositories. Keep fixtures minimal and
typed. A fixture proves parser or implementation behavior only; it does not
prove behavior of a real Codex installation or platform support.

## Pinned MCP Specification Inputs

`tests/conformance/mcp-spec/` owns the minimal versioned upstream schemas and
license attribution needed for deterministic MCP conformance work. Its manifest
keeps finalized initialization-based revisions separate from pre-release-only
inputs, pins full upstream commits, records the handshake family and release
classification, checksums every local artifact, and records
`volicord_conformance_covered`. That field means only that the revision is in
Volicord's repository-owned offline runtime conformance matrix; it is not
external MCP certification. Production support requires a released,
non-pre-release entry with a pinned schema, a production protocol profile, and
`volicord_conformance_covered=true`. A tracked pre-release entry remains outside
production support and has the coverage field set to `false`.

`cargo run -p xtask -- mcp-spec-check` is the offline integrity gate. It parses
the manifest, validates classifications and immutable references, and verifies
schema presence, schema family, attribution, checksums, and exact set parity
among manifest production support, compiled production protocol profiles, and
the adapter-owned conformance revision declaration without network access. Its
report gives deterministic counts for all pinned revisions,
production-supported revisions, Volicord-conformance-covered revisions, and
tracked pre-release revisions. `cargo run -p xtask -- mcp-spec-sync` is an
explicit maintenance action: it resolves the recorded releases to their pinned
commits, downloads into a temporary directory, preserves the reviewed support
and coverage metadata, validates the complete candidate, and only then replaces
the fixture. Ordinary builds and tests never invoke the networked sync path.

## Required Boundary Coverage

Durable tests should cover, as applicable:

- unknown members, duplicate keys, malformed closed values, and corrupt stored
  owner records;
- structural rejection before policy, replay, ticket invalidation, or mutation;
- read-only branches with no authority event or state-version advance;
- one atomic successful mutation and exact replay behavior;
- current-contract mismatch routed through the owned corrupt-data failure;
- missing or ineligible operation-result rows remaining
  `OPERATION_RESULT_UNAVAILABLE`;
- MCP rejection of hidden context and CLI-only UserAction resolution;
- authoritative MCP runtime-session source separation, milestone ordering,
  current revisions, project binding, and diagnostics non-authority;
- exact revision-set parity among `production_supported=true` manifest entries,
  production protocol profiles, `volicord_conformance_covered=true` entries,
  and adapter-owned conformance cases, with tracked pre-release generations
  excluded from production support;
- for every conformance-covered revision, standalone `initialize`, the
  initialized notification, `tools/list`, pinned-schema validation, required
  tools, the designated round-trip tool, revision-specific tool projection and
  operation batching, invalid lifecycle behavior, initialization-batch
  rejection, and EOF/shutdown;
- exact-match and counter-offer negotiation plus profile-specific initialize
  capabilities, batching, `tools/list`, and `tools/call` wire projection;
- independently pinned Codex host fixtures that are not derived from the
  production protocol registry and do not substitute for revision conformance,
  with CLI conformance evidence kept separate from actual `managed_host`
  observations;
- typed diagnostic codes and bounded/redacted facts, transactional finding and
  cause persistence, deterministic roots, dependency-driven `Blocked` checks,
  and equivalent concise, verbose, and lossless JSON projections of one report;
- Guard manifest exact-shape and owner binding, hash-free policy commands versus
  hash-bound runtime commands, wrapper/file drift, platform-independent script
  executable expectations, current-owned hook observations, and older-event
  exclusion;
- repeated Guard initialization with stable identities and preservation of
  unrelated repository content;
- Guard observation and unrecorded-change suppression outcomes; and
- Codex configuration drift and behavior-probe failure reporting.

Operational interoperability coverage accepts arbitrary bounded version
strings, exercises initialize and tool-list milestones, checks required tools
and safe read-only calls, audits Guard artifacts and required-phase
observations, and isolates session ownership and integration revisions.

## Release Integrity And Optional Host Smoke

The durable release test package is `tests/release-integrity`. It verifies all
five published Volicord targets, version consistency, canonical text bytes,
package and archive shape, packaged-binary identity, checksum output, and the
ordinary build and package structure in the release workflow.

Generic release-integrity tests cover Volicord platform build and package
artifacts. Operational Codex interoperability tests separately cover managed
configuration, MCP initialization, required tools, safe tool round trips,
Guard observations, session ownership, and revision isolation as defined by
[Agent Connection](../reference/agent-connection.md).

A real-Codex run is optional operational smoke. It may report the bounded host
version as a diagnostic and repeat the observation when that version changes.
Its result applies only to the behavior observed in that configuration and
environment; it does not establish future host behavior, human identity, or
runtime authority. Missing smoke infrastructure does not block the ordinary
Volicord release checks.

## Documentation Validation

Meaning-changing paired documents require English/Korean semantic parity.
Generated contract projections must match their sources. Run:

```sh
cargo run -p xtask -- docs-check
git diff --check
```

Then run the targeted stale-surface scan appropriate to the change and inspect
the diff for owner routing, exact identifiers, paths, anchors, and repository
hygiene.

## Rust Validation

The normal workspace gate is:

```sh
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all-targets --all-features
```

If a narrower command is necessary, record why and identify the unexecuted
workspace checks in the handoff.
