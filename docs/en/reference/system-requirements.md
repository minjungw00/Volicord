# System Requirements

This document owns the operating-environment prerequisites for the first
Volicord release. It defines eligible platform environments, the WSL2
topology, executable and filesystem prerequisites, Runtime Home and Product
Repository placement, and the conditions that require setup or validation to
stop.

It does not claim that a release cell ran or passed. Exact finalized-artifact
results belong to [Host Release Evidence](host-release-evidence.md), while
managed binding and receipt semantics belong to
[Agent Connection](agent-connection.md).

<a id="surface-stability"></a>
## Surface Stability

The four `PlatformEnvironment` values, five published target triples, six
required target/environment cells, the exact first-release WSL2
distribution/image coordinate, WSL2 topology and ext4 boundary, managed stdio
MCP prerequisite, and stop criteria are stable contracts. Other runner images,
package-manager commands, executable locations, and diagnostic prose are
release or implementation details unless another owner marks them stable.

<a id="first-release-environment-matrix"></a>
## First-Release Environment Matrix

Volicord publishes five binary targets and requires six independent release
environment cells:

| `target_triple` | `platform_environment` | Required boundary |
|---|---|---|
| `x86_64-unknown-linux-gnu` | `linux` | Every component executes on native x86-64 Linux. |
| `aarch64-unknown-linux-gnu` | `linux` | Every component executes on native AArch64 Linux. |
| `aarch64-apple-darwin` | `macos` | Every component executes on native Apple Silicon macOS. |
| `x86_64-apple-darwin` | `macos` | Every component executes on native Intel x86-64 macOS. |
| `x86_64-pc-windows-msvc` | `native_windows` | Every component executes as native x86-64 Windows components. WSL coordinates are ineligible. |
| `x86_64-unknown-linux-gnu` | `wsl2` | Every component executes inside the same supported WSL2 distribution and uses its Linux filesystem as specified below. |

An environment is eligible for a release claim only when its exact embedded
`CodexSupportEntry` has a matching external `CodexReleaseEvidenceEntry` with
`validation_evidence.validation_result=passed` and the exact exercised Volicord
digest. A pass in one row does not establish another row. Native Linux and WSL2
remain distinct even when they validate the same x86-64 Linux binary. Likewise,
one macOS or Linux architecture cannot establish the other. Repository tests,
cross-compilation, packaging, or a compatible-looking target triple do not
substitute for an executed cell.

The first-release product surface in every row is:

- `host_kind=codex`
- `integration_profile=record`
- `connection_scope=personal` or `connection_scope=shared`
- managed stdio MCP
- CLI inbox for user actions

Other hosts, profiles, transports, and user channels are outside this release.

<a id="wsl2-topology"></a>
## WSL2 Topology

The supported WSL2 topology is a single complete environment:

```text
one pinned Ubuntu LTS WSL2 distribution
  ├─ Codex process
  ├─ Volicord process
  ├─ Product Repository on the distribution ext4 filesystem
  ├─ Volicord Runtime Home on the distribution ext4 filesystem
  └─ Codex/Volicord executables, managed configuration, and generated managed
     artifacts on the distribution ext4 filesystem
```

The exact first-release WSL2 coordinate is:

| Coordinate | Exact value |
|---|---|
| `WSL_DISTRO_NAME` | `Ubuntu-24.04` |
| `/etc/os-release` `ID` | `ubuntu` |
| `/etc/os-release` `VERSION_ID` | `24.04` |
| `platform_release_coordinate.environment_image` | `Ubuntu-24.04-LTS-WSL2` |

The product observes the distribution name, operating-system identity, WSL2
kernel boundary, and filesystem type. The support-catalog image value is the
exact coordinate registered for those observed distribution facts; an entry
for another image cannot authorize this coordinate.

The WSL2 runtime boundary must establish WSL2 explicitly and requires
`target_triple=x86_64-unknown-linux-gnu`. An ordinary Linux `target_os` result
is insufficient. Its `ManagedHostBinding` and
`HostVerificationReceipt` bind `platform_environment=wsl2`; neither can be
reused under `linux` or `native_windows`.

The Product Repository, Runtime Home, Codex executable, Volicord executable,
managed Codex configuration, and every generated managed artifact must resolve
inside that distribution's Linux ext4 filesystem. A different distribution,
image, or filesystem is not inferred to be equivalent.

The following WSL topologies are unsupported and must fail with a
machine-readable unsupported-environment reason before installation or receipt
use:

- WSL1
- Codex on Windows with Volicord, the repository, or Runtime Home in WSL2
- Codex in WSL2 with a native Windows Volicord process, repository, or Runtime
  Home
- a Product Repository or Runtime Home on `/mnt/c`, `/mnt/d`, another
  `/mnt/*` path, or another DrvFS mount
- conversion or inference between Windows and Linux paths, PIDs, environment
  values, process bindings, or receipts
- reuse of a native Windows receipt in WSL2 or a WSL2 receipt on native Windows
- a distribution not named by the current WSL2 support entry

A WSL shutdown or restart invalidates live process identity. A binding or
receipt whose process or freshness coordinates no longer match is stale and
must be rejected before a fresh verify flow produces a new receipt.

Unsupported topology is machine-readable. `unsupported_wsl1` identifies WSL1;
`unsupported_wsl_cross_topology` identifies inconsistent WSL kernel/environment
facts; `unsupported_wsl2_distribution` identifies a distribution-coordinate
mismatch; and `unsupported_wsl2_filesystem` identifies a non-ext4 component.
These are `UnsupportedContract` outcomes. An unavailable kernel, distribution,
or filesystem observation remains `Unavailable`; it is not converted into an
unsupported or native environment.

<a id="toolchain-requirements"></a>
## Toolchain Requirements

Building and testing the workspace requires the Rust toolchain declared by the
repository. The maintained workspace currently targets Rust 1.85 or newer
compatible stable Rust. Use Cargo from the same toolchain for formatting,
checking, linting, tests, and release-validation contract tests.

Runtime prerequisites are:

- a finalized Volicord executable for the selected exact target and platform environment;
- an exact Codex executable whose artifact digest, target triple, platform coordinate, profile,
  and required capabilities match an embedded support-catalog entry;
- SQLite support supplied by the Volicord build;
- filesystem operations required by the selected native platform or WSL2
  adapter; and
- stdio pipes that preserve the managed MCP process boundary.

Git is required when a workflow supplies or validates Git object IDs or when
the selected Product Repository operation explicitly requires Git. Git object
ID spelling follows [External Contracts](external-contracts.md#shared-git-object-id-contract).

## Executable And Process Requirements

The administrative process must resolve and execute the exact Codex artifact
that setup and verification bind. Command-name discovery alone never
establishes support. Verification hashes the resolved executable, matches the
exact embedded support-catalog entry for the current target and platform, records the process and
capability observations required by the binding, and emits a receipt only
after all adapter checks succeed.

The managed Codex configuration must launch the intended Volicord executable
with managed stdio MCP. The adapter validates the exact command, arguments,
forwarded environment, configuration target, process binding, required
capabilities, and platform environment through the canonical
`ManagedHostBinding`. Empty and absent environment values are distinct.

Executable identity, configuration identity, process identity, and receipt
freshness are independent checks. Matching one does not supply another.

## Runtime Home Requirements

`VOLICORD_HOME` selects the Volicord Runtime Home where the applicable runtime
owner permits that environment variable. The resulting path must be non-empty,
absolute under the selected platform's path rules, writable for initialization
and administrative repair, and accessible to the managed Volicord process.

The Runtime Home must remain within one platform environment. Native Windows
and WSL2 paths are never converted or shared. Inside WSL2 its path and nearest
existing ancestor must be on the distribution ext4 filesystem and not under
`/mnt/*`; a path that merely has Linux spelling is insufficient.

Fresh development data is created from the current canonical SQLite contract.
An existing database with another manifest is not upgraded, imported, or
reinterpreted; use a new Runtime Home or an explicitly new empty destination.

## Product Repository Requirements

A personal connection binds one explicit Product Repository. A shared
connection installs repository-portable managed Codex configuration and
resolves the current clone without embedding a developer-local project ID or
Runtime Home path in the shared file.

The repository path must:

- be non-empty and resolve under the current platform's canonical path rules;
- identify the same repository used by the managed binding and receipt;
- allow only the writes required by the requested install, repair, or uninstall
  operation;
- remain within the same platform environment as Codex, Volicord, and the
  Runtime Home; and
- for WSL2, resolve on the distribution ext4 filesystem outside `/mnt/*`.

Moving the repository, changing its canonical identity, or changing the
connection scope makes a prior binding or receipt mismatched. Verification is
required again; callers must not rewrite the old coordinates implicitly.

## Codex Configuration Requirements

The Codex adapter owns discovery, strict parsing, canonical projection, atomic
apply, verification, drift detection, repair, and safe uninstall for the
managed entry. Setup must preserve unrelated user configuration and reject an
unowned collision rather than overwrite it.

Personal and shared configuration locations are adapter-owned details. Core
and Store receive only canonical binding data and typed verification receipts;
they do not read Codex configuration files, tokenize shell commands, inspect
wrapper markers, or infer a platform path.

Repair may recreate only adapter-owned configuration and typed recoverable
values after reporting the detected reason. Uninstall removes only the exact
currently owned entry and refuses a changed or unowned entry.

## Managed MCP Environment Requirements

The managed process uses stdio exclusively for the public MCP transport. It
must receive the exact project, connection, Runtime Home, host, profile,
binding, and platform coordinates required by the current managed launch
contract. Missing, empty, duplicated, conflicting, or unrecognized required
coordinates are rejected; they are not guessed from the current directory,
neighboring configuration, or another connection.

Secrets and unrelated ambient environment values are not copied into managed
configuration. Diagnostics must not print tokens, complete sensitive payloads,
or unredacted sensitive absolute paths.

## Stop Criteria

Installation, verification, repair, managed launch, or receipt use must stop
when any applicable condition is present:

- the host or profile is not exact `codex` and `record`;
- the platform environment is absent, ambiguous, or outside the four-value set;
- the target triple is absent, unknown, or incompatible with the platform environment;
- the exact Codex artifact, target triple, platform environment, profile, and
  required capabilities have no exact embedded support-catalog entry;
- release publication is attempted without a current passing evidence entry for
  every required target/environment cell;
- the executable, process, binding, configuration, project, connection,
  policy, capability, or freshness coordinates disagree;
- managed configuration is malformed, unowned, or has drifted outside the
  repairable owner boundary;
- a persisted typed setup action or other required owner value is corrupt;
- a native Windows/WSL2 crossing, WSL1, `/mnt/*`, DrvFS, or unsupported WSL
  distribution is observed;
- the Runtime Home or Product Repository cannot be safely resolved or accessed;
- managed stdio cannot be established; or
- a required read or platform primitive is unavailable.

The result must preserve the applicable `Rejected`, `Unavailable`, `Corrupt`,
or `UnsupportedContract` category and its domain reason. It must not create a
default binding, synthetic receipt, fallback host, inferred platform, or
partial success.

## Adjacent Owners

- First-release included and excluded surfaces: [Scope](scope.md).
- Binding, receipt, persisted setup action, and adapter/Core boundaries:
  [Agent Connection](agent-connection.md).
- Exact artifact and platform evidence: [Host Release Evidence](host-release-evidence.md).
- Runtime path and repository boundaries: [Runtime Boundaries](runtime-boundaries.md).
- SQLite format acceptance: [Storage Versioning](storage-versioning.md).
- Product-wide failure meanings: [Failure Model](failure-model.md).
- Threat model and non-guarantees: [Security](security.md).
