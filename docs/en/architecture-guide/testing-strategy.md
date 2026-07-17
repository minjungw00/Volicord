# Testing Strategy

Tests protect owner-defined current behavior. They do not create product
contracts, preserve removed surfaces, or justify broader support claims.

## Choose The Narrowest Layer

| Test layer | Use it for |
|---|---|
| Unit test | Pure parsing, canonical encoding, closed values, and policy decisions. |
| Crate integration test | Adapter boundaries, Store reads/writes, process behavior, and strict persisted-record rejection. |
| Conformance test | Public cross-method outcomes, error categories, replay, effects, and projections. |
| Release-validation cell | Exact finalized Codex artifact behavior on one platform environment. |
| Documentation check | Owner routing, links, terminology, parity, examples, and generated-source drift. |

Use disposable Runtime Homes and Product Repositories. Keep fixtures minimal and
typed. A fixture proves parser or implementation behavior only; it does not
prove a real Codex artifact or platform support.

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
- Guard observation and unrecorded-change suppression outcomes; and
- unsupported Codex artifact and configuration drift behavior.

## Codex Release Validation

Release support is four independent platform cells:

```text
linux
macos
native_windows
wsl2
```

Each cell executes the closed scenario catalog against exact finalized Codex
and Volicord executable digests in its own exact environment. No platform result
substitutes for another. Runtime lookup tests the embedded
`CodexSupportCatalog` without release evidence. Release validation tests the
external `CodexReleaseEvidenceManifest`, including deterministic parsing and
cross-checking against the catalog. The evidence manifest may contain zero
through four entries and must report only actual attempts. A `passed` result is
release evidence only for its exact catalog coordinates and Volicord digest.

Mock, fixture, rebuilt, selected, or neighboring artifacts cannot replace the
final bytes. Failed, unavailable, and not-run scenarios remain explicit in the
evidence rules owned by
[Host Release Evidence](../reference/host-release-evidence.md).

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
