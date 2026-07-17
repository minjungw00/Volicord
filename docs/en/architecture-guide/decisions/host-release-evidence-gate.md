# Exact Codex Release Evidence Gate

## Context

A support claim must name the finalized Codex executable bytes and the platform
where the required first-release behavior was observed. A fixture, version
label, neighboring build, or result from another platform is insufficient.

## Decision

Build and finalize the Codex candidate before validation, compute its exact
SHA-256 digest, and execute the closed release scenario catalog independently
on:

```text
linux
macos
native_windows
wsl2
```

Each `CodexReleaseCell` binds the artifact digest, one
`PlatformEnvironment`, the complete first-release `CodexCapability` set,
`integration_profile=record`, exact runner coordinates, scenario results, and
evidence digest.

The checked-in manifest is an honest report, not a target-shaped placeholder.
It may contain zero through four cells. A platform with no qualifying attempt
has no cell. Only a cell whose evidence status is `passed` supports its exact
coordinates; results never propagate between cells or artifacts.

Review recomputes digests and validates the closed shape. Production consumes
only owner-defined exact passing evidence and fails closed for
`unsupported_host_artifact`.

## Consequences

- Signing, stripping, packaging, or any byte change requires validation of the
  new digest.
- WSL2 is independent from native Linux and native Windows.
- Mock and parser fixtures remain non-release evidence.
- Failed, unavailable, and not-run outcomes remain explicit and cannot be
  relabeled as passing.
- Release results are evidence, not runtime identity or user authentication.

The exact cell schema, catalog, and digest algorithm belong to
[Host Release Evidence](../../reference/host-release-evidence.md).
