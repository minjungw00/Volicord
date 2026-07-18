# Exact Codex Release Evidence Gate

## Context

A support claim must name the finalized Codex executable bytes and the platform
where the required first-release behavior was observed. A fixture, version
label, neighboring build, or result from another platform is insufficient.
The Volicord executable cannot embed evidence whose identity includes a digest
of those same final executable bytes without creating a self-reference.

## Decision

Embed only a strict `CodexSupportCatalog` containing exact Codex policy
coordinates. Keep `CodexReleaseEvidenceManifest`, including the exact exercised
Volicord digest and cell results, external to every Volicord executable. Release
validation loads the external evidence and cross-checks every entry against the
embedded catalog.

Build and finalize the Codex and Volicord candidates before validation, compute
their exact SHA-256 digests, and execute the closed release scenario catalog
independently on:

```text
x86_64-unknown-linux-gnu / linux
aarch64-unknown-linux-gnu / linux
aarch64-apple-darwin / macos
x86_64-apple-darwin / macos
x86_64-pc-windows-msvc / native_windows
x86_64-unknown-linux-gnu / wsl2
```

Build each published Volicord target once. The build job records the target,
source revision, executable name, and raw executable SHA-256, then uploads the
raw bytes as an immutable workflow artifact. Every matching cell downloads that
artifact; WSL2 transfers the same Linux x86-64 bytes to ext4 and verifies the
digest there. Each cell validates once and emits fresh external evidence for
that digest. Publication requires all six passing cells, downloads the same
five build artifacts, packages without rebuilding, and verifies each extracted
archive member against the validated raw digest.

Each external `CodexReleaseEvidenceEntry` binds both artifact digests, one
`PlatformEnvironment`, the complete first-release `CodexCapability` set,
`integration_profile=record`, exact target and runner coordinates, scenario
results, and evidence digest. The matching `CodexSupportEntry` contains only
the Codex digest, target triple, platform and release coordinate, profile, and verified
capabilities.

The external evidence manifest is an honest report, not a target-shaped
placeholder. It may contain zero through six entries. A required cell with no
qualifying attempt has no entry. A passing result establishes release evidence
only for its exact catalog coordinates and Volicord digest; results never
propagate between cells or artifacts. The production runtime authorization
path does not read either release contract.

Review recomputes digests and validates both closed shapes. Release publication
fails closed for `unsupported_host_artifact`; operational MCP, CLI, Core, and
Store authorization never consumes the catalog or release evidence.

## Consequences

- Signing, stripping, or any executable-byte change requires a new build
  artifact and validation of the new digest. Packaging may change only the
  surrounding archive and metadata.
- A Volicord digest can appear in external evidence without changing the
  embedded support-catalog identity.
- External release evidence is never an embedded resource, generated Rust
  constant, or build-script input.
- WSL2 is independent from native Linux and native Windows.
- Native Linux and WSL2 produce distinct evidence for the same Linux x86-64
  build artifact.
- A missing build, runner, Codex artifact, evidence entry, or WSL2 execution
  blocks publication.
- Linux and macOS architectures are independent target identities.
- Mock and parser fixtures remain non-release evidence.
- Failed, unavailable, and not-run outcomes remain explicit and cannot be
  relabeled as passing.
- Release results are evidence, not runtime identity or user authentication.

The exact support-catalog, evidence-manifest, cell, and digest contracts belong to
[Host Release Evidence](../../reference/host-release-evidence.md).
