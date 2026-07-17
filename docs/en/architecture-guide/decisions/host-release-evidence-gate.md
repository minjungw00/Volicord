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
linux
macos
native_windows
wsl2
```

Each external `CodexReleaseEvidenceEntry` binds both artifact digests, one
`PlatformEnvironment`, the complete first-release `CodexCapability` set,
`integration_profile=record`, exact target and runner coordinates, scenario
results, and evidence digest. The matching `CodexSupportEntry` contains only
the Codex digest, platform and release coordinate, profile, and verified
capabilities.

The external evidence manifest is an honest report, not a target-shaped
placeholder. It may contain zero through four entries. A platform with no
qualifying attempt has no entry. A passing result establishes release evidence
only for its exact catalog coordinates and Volicord digest; results never
propagate between cells or artifacts. Runtime lookup reads only the embedded
catalog and fails closed when it is empty.

Review recomputes digests and validates both closed shapes. Production consumes
only owner-defined support policy and fails closed for
`unsupported_host_artifact`; it never consumes release evidence.

## Consequences

- Signing, stripping, packaging, or any byte change requires validation of the
  new digest.
- A Volicord digest can appear in external evidence without changing the
  embedded support-catalog identity.
- External release evidence is never an embedded resource, generated Rust
  constant, or build-script input.
- WSL2 is independent from native Linux and native Windows.
- Mock and parser fixtures remain non-release evidence.
- Failed, unavailable, and not-run outcomes remain explicit and cannot be
  relabeled as passing.
- Release results are evidence, not runtime identity or user authentication.

The exact support-catalog, evidence-manifest, cell, and digest contracts belong to
[Host Release Evidence](../../reference/host-release-evidence.md).
