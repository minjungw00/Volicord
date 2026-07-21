# System Requirements

This document owns the operating-environment prerequisites for the first
Volicord release. It defines eligible platform environments, the WSL2
topology, executable and filesystem prerequisites, Runtime Home and Product
Repository placement, and the conditions that require setup or validation to
stop.

Ordinary build, package, checksum, platform, and publication validation belongs
to [Validation](../maintain/validation.md), while managed operational-session
authorization belongs to [Agent Connection](agent-connection.md). Executable
path and version observations are diagnostic inputs to current operational
verification.

<a id="surface-stability"></a>
## Surface Stability

The four `PlatformEnvironment` values, five published target triples, the exact
first-release WSL2 distribution identity, WSL2 topology and ext4 boundary,
managed stdio MCP prerequisite, and stop criteria are stable contracts. Other
runner images, package-manager commands, executable locations, and diagnostic
prose are release or implementation details unless another owner marks them
stable.

<a id="first-release-environment-matrix"></a>
## First-Release Environment Matrix

Volicord publishes five binary targets. The supported execution environments
for those binaries are:

| `target_triple` | `platform_environment` | Required boundary |
|---|---|---|
| `x86_64-unknown-linux-gnu` | `linux` | Every component executes on native x86-64 Linux. |
| `aarch64-unknown-linux-gnu` | `linux` | Every component executes on native AArch64 Linux. |
| `aarch64-apple-darwin` | `macos` | Every component executes on native Apple Silicon macOS. |
| `x86_64-apple-darwin` | `macos` | Every component executes on native Intel x86-64 macOS. |
| `x86_64-pc-windows-msvc` | `native_windows` | Every component executes as native x86-64 Windows components. WSL coordinates are ineligible. |
| `x86_64-unknown-linux-gnu` | `wsl2` | Every component executes inside the same supported WSL2 distribution and uses its Linux filesystem as specified below. |

Target compatibility is a Volicord platform constraint. Native Linux and WSL2
remain distinct environments even when they execute the same x86-64 Linux
binary. One architecture or environment
does not establish the runtime prerequisites of another. Release packaging
still builds and checks every published target independently.

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

The first-release WSL2 boundary and distribution identity are:

| Observation | Requirement |
|---|---|
| `/proc/sys/kernel/osrelease` | A supported Microsoft WSL2 kernel |
| `/etc/os-release` `ID` | `ubuntu` |
| `/etc/os-release` `VERSION_ID` | `24.04` |

Platform checks use the kernel release to distinguish native Linux, WSL1, and
WSL2. A supported WSL2 kernel uses `/etc/os-release` to establish the Ubuntu ID
and version. No WSL environment variable is a prerequisite. Filesystem
observations enforce the supported topology. Operational authorization
separately validates current Connection and session ownership.

The WSL2 runtime boundary must establish WSL2 explicitly and requires
`target_triple=x86_64-unknown-linux-gnu`. An ordinary Linux `target_os` result
is insufficient. Operational Connection and session records remain local to
their Runtime Home and platform environment; they are not converted to
`linux` or `native_windows` authority.

The Product Repository, Runtime Home, Codex executable, Volicord executable,
managed Codex configuration, and every generated managed artifact must resolve
inside that distribution's Linux ext4 filesystem. A different distribution,
image, or filesystem is not inferred to be equivalent.

The following WSL topologies are unsupported and must fail with a
machine-readable unsupported-environment reason before installation or managed
launch:

- WSL1
- Codex on Windows with Volicord, the repository, or Runtime Home in WSL2
- Codex in WSL2 with a native Windows Volicord process, repository, or Runtime
  Home
- a Product Repository or Runtime Home on `/mnt/c`, `/mnt/d`, another
  `/mnt/*` path, or another DrvFS mount
- conversion or inference between Windows and Linux paths, PIDs, environment
  values, Connections, runtime sessions, or project sessions
- reuse of native Windows Runtime Home session records in WSL2 or WSL2 session
  records on native Windows
- a distribution whose `/etc/os-release` identity is outside the current
  first-release WSL2 identity

A WSL shutdown or restart ends the live managed runtime session. Its project
sessions cannot authorize later calls; a new managed MCP lifecycle records a
new runtime session and project sessions.

Platform observation and unsupported-topology outcomes are machine-readable.
`platform_environment_unavailable` is an `Unavailable` outcome when the kernel
release needed to classify the host cannot be read. Native Linux requires no
distribution-identity observation after that kernel classification. For a
supported WSL2 kernel, an unreadable `/etc/os-release` or a missing or malformed
required `ID` or `VERSION_ID` produces the `Unavailable` reason
`wsl2_distribution_unavailable`. A valid identity with an unsupported `ID` or
`VERSION_ID` produces the `Rejected` reason
`unsupported_wsl2_distribution`. `unsupported_wsl1` identifies WSL1, and
`unsupported_wsl2_filesystem` identifies a non-ext4 component. An unavailable
observation is not converted into a rejected or native environment.

<a id="toolchain-requirements"></a>
## Toolchain Requirements

Building and testing the workspace requires the Rust toolchain declared by the
repository. The maintained workspace currently targets Rust 1.85 or newer
compatible stable Rust. Use Cargo from the same toolchain for formatting,
checking, linting, and tests.

Runtime prerequisites are:

- a finalized Volicord executable for the selected exact target and platform environment;
- an available Codex executable able to launch the managed configuration;
- SQLite support supplied by the Volicord build;
- filesystem operations required by the selected native platform or WSL2
  adapter; and
- stdio pipes that preserve the managed MCP process boundary.

Git is required when a workflow supplies or validates Git object IDs or when
the selected Product Repository operation explicitly requires Git. Git object
ID spelling follows [External Contracts](external-contracts.md#shared-git-object-id-contract).

## Executable And Process Requirements

The administrative process must resolve and execute the configured Codex
executable. When active verification runs, the executable must be discoverable
and its version command must succeed. Verification reports the resolved path and
observed host version as diagnostics. A different observed version makes the
current operational observation pending until managed Codex behavior is
observed again. Executable availability alone does not establish agent,
operating-system-user, or human identity.

The managed Codex configuration must launch the intended Volicord executable
with managed stdio MCP. The adapter validates the managed entry, command,
arguments, personal static or shared forwarded Runtime Home binding,
configuration target, and platform prerequisites through the canonical managed
launch contract. Managed launch markers are cooperative routing context, not
credentials. Empty and absent environment values are distinct.

Executable, configuration, process, client, and version observations are
diagnostic or setup facts. Runtime authorization validates the current
Connection, project membership, allowed mode, and Store-owned managed
runtime/project sessions and exact binding.

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
- identify the repository currently registered for the selected Connection;
- allow only the writes required by the requested install, repair, or uninstall
  operation;
- remain within the same platform environment as Codex, Volicord, and the
  Runtime Home; and
- for WSL2, resolve on the distribution ext4 filesystem outside `/mnt/*`.

Moving the repository, changing its canonical identity, changing Connection
scope, or advancing its integration revision makes prior project sessions
stale. A new managed MCP project session is required; callers must not rewrite
old coordinates implicitly.

## Codex Configuration Requirements

The Codex adapter owns discovery, strict parsing, canonical projection, atomic
apply, verification, drift detection, repair, and safe uninstall for the
managed entry. Setup must preserve unrelated user configuration and reject an
unowned collision rather than overwrite it.

Personal and shared configuration locations are adapter-owned details. Core
receives only `ValidatedAgentSession`; it does not read Codex configuration
files, tokenize shell commands, inspect wrapper markers, or infer a platform
path. Store owns the Connection, membership, integration revision, managed
runtime session, and project session records from which MCP validates that
boundary.

Repair may recreate only adapter-owned configuration and typed recoverable
values after reporting the detected reason. Uninstall removes only the exact
currently owned entry and refuses a changed or unowned entry.

## Managed MCP Environment Requirements

The managed process uses stdio exclusively for the public MCP transport. It
must receive its binding from the canonical managed launch context. A personal
entry carries its Connection and selected canonical absolute Runtime Home as
static values, carries no project selector, and forwards no environment name.
Its authoritative repository associations remain the Connection's Store-owned
project memberships. Repository-portable shared discovery forwards only
`VOLICORD_HOME` and resolves a registered current clone without embedding
machine-local IDs or paths. Missing, empty, conflicting, or unrecognized
required launch context is rejected; it is not guessed from another
Connection. The host and profile markers select this cooperative path but do
not authorize a tool call.

On initialize, MCP records one managed-host runtime session with bounded client
name/version and optional host version diagnostics. On each project tool call,
it records or selects the project session and validates current Connection
enablement, membership, mode, runtime/project session ownership, and both
integration revisions before constructing Core context. Compatibility for a
newly observed bounded host version is established by renewed operational
observation.

Secrets and unrelated ambient environment values are not copied into managed
configuration. Diagnostics must not print tokens, complete sensitive payloads,
or unredacted sensitive absolute paths.

## Stop Criteria

Installation, verification, repair, managed launch, or project call must stop
when any applicable condition is present:

- the host or profile is not exact `codex` and `record`;
- the platform environment is absent, ambiguous, or outside the four-value set;
- the target triple is absent, unknown, or incompatible with the platform environment;
- the managed configuration, project, Connection, membership, mode, runtime
  session, project session, or current integration revisions disagree;
- managed configuration is malformed, unowned, or has drifted outside the
  repairable owner boundary;
- a persisted typed setup action or other required owner value is corrupt;
- a native Windows/WSL2 crossing, WSL1, `/mnt/*`, DrvFS, or unsupported WSL
  distribution is observed;
- the Runtime Home or Product Repository cannot be safely resolved or accessed;
- managed stdio cannot be established; or
- a required read or platform primitive is unavailable.

The result must preserve the applicable `Rejected`, `Unavailable`, or `Corrupt`
category and its domain reason. It must not create a default session, synthetic
authorization, fallback host, inferred platform, or partial success.

## Adjacent Owners

- First-release included and excluded surfaces: [Scope](scope.md).
- Operational session, persisted setup action, and adapter/Core boundaries:
  [Agent Connection](agent-connection.md).
- Build, package, platform, and release validation: [Validation](../maintain/validation.md).
- Runtime path and repository boundaries: [Runtime Boundaries](runtime-boundaries.md).
- SQLite format acceptance: [Storage Versioning](storage-versioning.md).
- Product-wide failure meanings: [Failure Model](failure-model.md).
- Threat model and non-guarantees: [Security](security.md).
