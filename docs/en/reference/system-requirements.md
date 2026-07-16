# System Requirements

This document owns environment applicability and prerequisites that a reader should check before installing Volicord executables or connecting an MCP host. It classifies operating environment, shell, toolchain, executable layout, filesystem access, Runtime Home, Product Repository, and MCP host prerequisites using evidence available in this repository.

This document does not define administrative command behavior, MCP stdio behavior, storage effects, host trust, public API behavior, schemas, or security guarantees. Exact behavior remains with [Administrative CLI](admin-cli.md), [MCP Transport](mcp-transport.md), [Runtime Boundaries](runtime-boundaries.md), and [Agent Connection](agent-connection.md).

## Status Vocabulary

| Status | Meaning in this document |
|---|---|
| Supported | A baseline path is documented by the relevant owner documents and can be checked before installation. Support is limited to the stated requirement; it is not an operating-system support promise unless this page says so. |
| Verified | The repository contains direct evidence for the statement, such as workspace metadata, maintained examples, source checks, tests, or checked-in validation tooling. |
| Unverified | The environment may work, but this repository does not contain enough evidence to document it as supported or verified. |
| Out of scope | The environment or procedure is not covered by the maintained baseline, is explicitly rejected by owner documents, or would require instructions not present in this repository. |

Do not infer support from Rust portability alone. A Rust crate being portable in principle is not evidence that this repository verifies a named operating system, shell, package manager, container image, remote host, or agent-host version.

## Applicability Matrix

| Area | Status | Repository evidence | Before continuing |
|---|---|---|---|
| Release binary packaging and installation | **Supported and verified.** This status covers the target triples in this table. A concrete installation also requires a published matching asset set. | `.github/workflows/release.yml` builds target-named release archives, runs smoke tests against each built binary, and generates `.sha256` files. POSIX targets are `.tar.gz` archives containing only `volicord`; native Windows is a `.zip` archive containing only `volicord.exe`. The downloaded `install.sh` and `install.ps1` assets select those target names. | Confirm that the selected release repository, tag, or mirror provides the installer, target archive, and checksum before using this path. Otherwise use a source build, local Docker build, or an existing installed executable. |
| Release version and build-descriptor alignment | **Supported and verified** for the checked-in tag-release workflow. | Before any release build job, `.github/workflows/release.yml` runs `cargo run --locked -p xtask -- release-version-check`. For a tag it also passes `--tag` and requires the tag to equal `v` plus the root `[workspace.package].version`; the checker requires every workspace member package to use `version.workspace=true`. Each release matrix job verifies that the checkout is clean and `HEAD` equals `GITHUB_SHA`, supplies `VOLICORD_BUILD_GIT_COMMIT`, `VOLICORD_BUILD_GIT_DIRTY=false`, and `VOLICORD_BUILD_PROFILE=release`, then checks the built binary's Git, tree, metadata-source, target, and exact-profile fields. CLI and MCP transport tests verify that `volicord --version` and MCP initialize `serverInfo.version` preserve the inherited package SemVer. The separate `build_id` records source and compilation dimensions without a build timestamp; it is an operational descriptor, not a binary digest or an identifier for the exact contents of a dirty tree. Automatic Git discovery is used only when the workspace root is the actual worktree top level. A source archive without its own Git worktree uses `git=unknown`, `tree=unknown`, and `metadata_source=unknown`. Builders may provide the Git commit and dirty values only as a valid pair; malformed or partial values fail the build. `VOLICORD_BUILD_PROFILE` supplies an exact profile name; otherwise only Cargo's approximate `debug`/`release` `profile_class` is recorded and `profile_exact=false`. | Before publishing a tag, complete the batch-level SemVer assessment, update the root workspace package version and lockfile as needed, and use the exact matching `vX.Y.Z` tag. Build from the intended clean commit and retain the displayed `build_id` with operational test results. Treat environment-supplied metadata as a builder assertion rather than a cryptographic attestation. |
| Host release evidence gate | The external contract and its `tests/release-validation` implementation are **present and repository-tested**. Those tests verify gate mechanics only, not live host feature support; production local-web manifest acquisition remains **unavailable**. | [Host Release Evidence](host-release-evidence.md) owns the four versioned schemas, exact external candidate binding, fixed twelve cells, canonical evaluator, half-open freshness rule, create-new manifest, and separate-process audit. The `volicord-release-validation-tests` package implements and tests those schema, path, evaluation, publication, and audit mechanics. These artifacts are produced after the final executable and remain outside the binary and Runtime Home. The current production adapter has no trusted acquisition path and external release artifacts are not runtime trust inputs. | Run the owner-defined package commands when evaluating the gate or externally supplied exact live cells. Passing repository tests does not make any host feature `verified`. Do not claim credential-bearing local-web availability from a manifest, `build_id`, persisted row, copied digest, or fixture. Use CLI inbox for production fallback. |
| Linux x86_64 | **Supported and verified** as release target `x86_64-unknown-linux-gnu`. | The release workflow builds on `ubuntu-24.04` and packages `volicord-x86_64-unknown-linux-gnu.tar.gz`. | Use a Linux x86_64 environment with a POSIX-style shell and the install-script tools listed below. |
| Linux aarch64 | **Supported and verified** as release target `aarch64-unknown-linux-gnu`. | The release workflow builds on the native `ubuntu-24.04-arm` runner and packages `volicord-aarch64-unknown-linux-gnu.tar.gz`. | Use a Linux aarch64 environment with a POSIX-style shell and the install-script tools listed below. |
| WSL2 | **Supported** as Linux when `uname` reports `Linux` and the architecture is `x86_64` or `aarch64`. | The POSIX installer treats WSL2 as Linux because the observable platform is the Linux userspace. Native Windows uses the separate PowerShell installer and Windows target. | Use WSL2 with the matching Linux architecture. Do not pass WSL paths to a native Windows Volicord process. |
| macOS arm64 | **Supported and verified** as release target `aarch64-apple-darwin`. | The release workflow builds on a macOS arm64 runner and packages `volicord-aarch64-apple-darwin.tar.gz`. | Use a macOS arm64 environment with a POSIX-style shell and the install-script tools listed below. |
| macOS x86_64 | **Supported and verified** as release target `x86_64-apple-darwin`. | The release workflow builds on a macOS Intel runner and packages `volicord-x86_64-apple-darwin.tar.gz`. | Use a macOS x86_64 environment with a POSIX-style shell and the install-script tools listed below. |
| Docker | **Supported and verified** as a local runtime option with the checked-in `Dockerfile`. No external image registry is claimed. | The checked-in `Dockerfile` builds the release CLI into a Debian runtime image. The release workflow builds the image and smoke-tests `volicord --help` and `volicord serve --help`. The Installation page documents local `docker build` and host-loopback `docker run` usage. | Build the image from this repository or from a trusted source copy. The maintained baseline does not include a registry image. |
| Native Windows x86_64 binary, installation, and Record setup | **Supported and verified** only for release target packaging, PowerShell installation, and the `record` setup path on `x86_64-pc-windows-msvc`. This row establishes no `HostFeatureSupportStatus` value. `volicord evidence capture-command` and the managed per-final-output fixed-UI authority disclosure are explicit exceptions and are unavailable on native Windows. The canonical `volicord status` fallback remains available. | The release workflow builds on `windows-2022`, smoke-tests `target/x86_64-pc-windows-msvc/release/volicord.exe`, packages `volicord-x86_64-pc-windows-msvc.zip`, generates `.sha256`, and runs a native Windows `cargo test --workspace --all-targets --all-features` job. The downloaded `install.ps1` asset installs the matching binary under a user-local directory by default. Bounded command process-group termination and generated final-output wrapper implementation are available only on Linux and macOS; implementation availability is not a typed host-support claim. | Use PowerShell on native Windows x86_64. Use `volicord init --host HOST --repo PATH --profile record` for setup. Run command evidence capture or managed fixed-UI final-output disclosure in WSL2, Linux, or macOS; on native Windows inspect current authority with `volicord status --task TASK_ID --json` or `volicord status --json` when there is no active Task. |
| Native Windows Detective profile | **Out of scope.** | Detective setup writes POSIX `sh` hook wrappers for the built-in adapters. The CLI rejects `volicord init --profile detective` on native Windows with `DETECTIVE_WINDOWS_UNSUPPORTED`. | Use `--profile record` on native Windows, or run Volicord in WSL2, Linux, or macOS where the owner-defined host-hook and watcher setup prerequisites are met. |
| Source build toolchain | **Supported and verified** for Rust 1.85 or newer with Cargo. | The workspace root `Cargo.toml` sets `rust-version = "1.85"` and all workspace packages inherit that value. The Installation page documents the source-build path. | Install or select Rust 1.85+ with Cargo when using the source-build path. |
| Shell syntax | **Supported** for maintained POSIX-style examples on Linux, WSL2, and macOS, and for maintained PowerShell examples on native Windows. Other shells are **unverified** for these examples. | POSIX installation examples use `sh`-compatible environment assignments, temporary installer paths, and `~/.local/bin`. Native Windows installation examples use a downloaded `install.ps1` release asset, PowerShell parameters or environment variables, and `%LOCALAPPDATA%\Volicord\bin`. CLI integration tests create `#!/bin/sh` fake executables behind `#[cfg(unix)]`; the release workflow runs the PowerShell smoke test on Windows. | Use the shell syntax for the selected operating environment and verify the installed command before continuing. |
| Executable role names | **Supported and verified.** | Reference owners define `volicord` as the installed executable for administrative CLI commands and the `mcp` subcommand used by the local MCP stdio adapter. | Build or install `volicord`; host configuration should start MCP with `volicord mcp --stdio ...`. |
| Package-manager installation | **Out of scope.** | No Homebrew tap, Homebrew formula, Linux package-manager package, or external package registry is claimed by this repository. | Use a source build, local Docker build, an existing `volicord` executable, or a release installer backed by a verified published asset set. |
| Host version compatibility for Codex and Claude Code | No stable minimum host version is defined. Exact reviewed compatibility is version-specific: the current Codex release matrix recognizes probe output `codex-cli 0.144.4` as canonical `host_version=0.144.4`. | Codex verification finds `codex` on `PATH`, runs `codex --version`, and retains the parsed canonical coordinate separately from the raw probe envelope in that fresh verification result. A stored verification report is diagnostic history and is not reused as the current installed-host coordinate by connection status or Doctor. Claude Code verification inspects host state through `claude mcp get <server_name>`. Administrative verification owns final result states. | Use `volicord connection verify HOST [--repo PATH] [--shared|--global]` after installation. Do not generalize the `0.144.4` review into a minimum-version promise or another host version. |
| Codex managed final-output root resolution | **Implemented configuration prerequisite** for best-effort display in local Git work trees on Linux and macOS. It does not by itself establish `record_final_output` or `detective_final_output` support. Without current matching probes an implemented aggregate is `implemented_unverified`; explicit capability absence yields `unsupported_by_host`, and a current failed or down prerequisite yields `temporarily_unavailable`. The managed fixed-UI display is unavailable for a non-Git Codex `record` installation. | Generated Codex final-output commands resolve the Git work-tree root with `git rev-parse --show-toplevel` before dispatching to the Volicord-managed wrapper. A non-Git Codex `record` initialization succeeds without generating that handler and reports the display configuration as unavailable. Root resolution proves neither authenticated exact replay nor safe block-only finalization. Current fixed-UI and Stop/replay probe observations, not host-version equality, determine the aggregate. | Use a local Git work tree when best-effort Codex managed final-output display is desired. In a non-Git Codex `record` repository, inspect current authority with `volicord status --task TASK_ID --json`, or `volicord status --json` when there is no active Task. Claude Code does not require a Git root for this handler. |
| Codex Detective profile host-hook root resolution | **Implemented and repository-verified as a setup prerequisite** for local Git work trees. This prerequisite status does not establish host feature support or promote `registered_connection_observation`. | Generated Codex detective host-hook commands resolve the Git work-tree root with `git rev-parse --show-toplevel` before dispatching to Volicord-managed wrappers, and initialization rejects Detective profile setup when that root strategy cannot be satisfied. These checks cover root resolution only. | For the Codex Detective profile, use a Product Repository with a `.git` work-tree root and ensure the Codex hook environment can run `git` from repository subdirectories. Use `--profile record` when this prerequisite is not available. |
| Git workspace-coordinate reference storage | The Git `files` reference backend is **supported and verified** for loose refs, `packed-refs`, normal worktrees, and linked worktrees. Git `reftable` reference storage is **out of scope**. | Workspace-coordinate capture reads bounded Git control files without invoking Git. It detects an explicit non-`files` `extensions.refStorage` value and fails closed instead of treating an existing branch as unborn. | Use the `files` reference backend for a Git-backed Product Repository whose Change Unit/write-ticket path relies on workspace coordinates. Convert an unsupported repository before using that path. |

## Toolchain Requirements

A release installation from published assets does not require Rust or Cargo.

The source-build path requires:

- Rust 1.85 or newer.
- Cargo from the selected Rust toolchain.
- A local checkout of this repository.
- Network or local dependency availability sufficient for Cargo to resolve the workspace dependencies.

Rust 1.85 is a compiler requirement for this workspace. It is not required for
installation from published release assets and is not an operating-system
support claim.

Rust implementation validation is not required just to read or use these requirements. Maintainers who edit Rust source, Cargo manifests, tests, fixtures, or build configuration should follow the Rust validation policy in the repository working rules.

## Shell And Path Requirements

Linux, WSL2, and macOS release install examples assume a POSIX-style shell with:

- environment assignment before a command, such as `VOLICORD_RELEASE_BASE_URL="$base" VOLICORD_REQUIRE_CHECKSUM=1 sh "$tmp"`
- `curl` for downloading the installer asset to a temporary path
- `curl` or `wget` for installer-managed release asset downloads
- `mktemp` for creating temporary installer paths
- `tar` for extracting the target-named release archive
- `awk`, `wc`, `tr`, and `sed` for checksum and archive-shape checks
- `sha256sum` or `shasum` when checksum verification is available
- current-session `PATH` updates when setup prints a shell command
- home-relative paths such as `~/.local/bin`
- command lookup through `PATH`
- forward-slash paths in examples

Native Windows release install examples assume PowerShell with:

- a downloaded `install.ps1` release asset invoked from a temporary path
- `Invoke-WebRequest` for downloading installer and release assets
- `Expand-Archive` for extracting the target-named `.zip`
- `Get-FileHash -Algorithm SHA256` when checksum verification is available
- user-level `PATH` updates only when explicitly requested with `-UpdateUserPath`
- local drive-letter paths for Runtime Home and Product Repository locations

The install scripts verify the downloaded `.sha256` file when that checksum
asset is available. If the checksum file is present but cannot be verified, the
script fails. If the checksum file is unavailable, the script warns and
continues unless `VOLICORD_REQUIRE_CHECKSUM=1` is set.

Current-session `PATH` examples affect only the shell where they are run. They
do not install commands persistently for future shells or MCP hosts.

On native Windows, the downloaded PowerShell installer with `-UpdateUserPath`
appends only the user-level `PATH` value when the install directory is not
already present. The script does not change machine-level `PATH`. Without
`-UpdateUserPath`, it prints a current-session `PATH` command and the installed
executable path.

The CLI cannot permanently edit the parent shell `PATH`. During setup, Volicord
can help make its commands available on `PATH` by offering safe choices such as
command links, creating a missing conventional user command directory such as
`~/.local/bin` when that is safe, a printed shell command, or an explicitly
user-confirmed managed shell startup block when the shell is supported. Setup verifies
writability before placing command links. Existing shells and MCP hosts may need
restart or reload before they see a changed startup file or command link
directory.

`VOLICORD_HOME` is different. It is a real Runtime Home selection input for `volicord` administrative commands and `volicord mcp --stdio` process startup, as defined by their owner documents.

## Executable Layout And Discovery

To use an installed executable, one selected executable location must provide:

- `volicord`

POSIX release tarballs are expected to contain only:

- `volicord`

Native Windows release zip archives are expected to contain only:

- `volicord.exe`

The install scripts install only that executable. For source
builds, the debug executable is expected under `target/debug` and the release
executable under `target/release`. For separately installed executables, select
an installation layout where setup can find `volicord` through an explicit
setup option or `PATH`.

Before first connection from a release binary or another installed command
directory, verify the installed executable from the same shell:

```sh
volicord --version
volicord --help
volicord mcp --help
volicord init --help
volicord status --help
volicord connection --help
volicord inbox --help
volicord serve --help
```

Before first connection from the release-mode source build documented by the
Installation guide, verify the built executable from the same shell:

```sh
./target/release/volicord --version
./target/release/volicord --help
./target/release/volicord mcp --help
```

After `init` or profile-repair guidance has made the command visible, verify
ordinary command lookup:

```sh
volicord --version
volicord init --help
volicord status --help
volicord connection add --help
volicord mcp --version
volicord mcp --help
```

Host configuration normally uses MCP command information established by
`volicord init`.
For exact `--mcp-command`, discovery-order, connection, and generic host
configuration behavior, use
[Administrative CLI](admin-cli.md#runtime-home-selection).

Requirement summary:

- The installation profile must identify a `volicord` command that can be
  found.
- Host processes that load personal/local bindings must be able to start the
  configured `volicord` command with `mcp --stdio --connection
  <connection_id>` arguments.
- Shared Codex and Claude Code project configuration must use the exact
  PATH-resolved `volicord mcp --stdio --discover-repository --host <host>`
  descriptor and must not embed local IDs, an absolute command, a literal
  Runtime Home path, or unrelated local-only environment entries. It must
  contain exactly one host-native forwarding form: Codex
  `env_vars = ["VOLICORD_HOME"]` or
  Claude Code `"env": {"VOLICORD_HOME": "${VOLICORD_HOME}"}`.
- User-managed generic host configuration remains user-managed and has no
  host-specific observable loadability gate.

## Runtime Home Requirements

A usable `Volicord Runtime Home` must be a local filesystem location the selected process can create, read, and write when the requested administrative or MCP operation needs runtime records.

Before installation:

- Select a Runtime Home that is not the `Product Repository` and is not inside or above the `Product Repository`.
- On native Windows, select a local drive-letter Runtime Home path. UNC paths,
  WSL UNC paths such as `\\wsl$\...`, and WSL mount-style paths such as
  `/mnt/c/...` are not supported native Windows Runtime Home paths.
- Ensure the selected user can create the directory or write into it when running `volicord init`, `volicord project use`, `volicord connection add`, or `volicord connection verify`.
- Ensure host processes that start shared repository discovery always receive
  the intended Runtime Home as an explicit, nonempty, absolute
  `VOLICORD_HOME`, including when that location is `$HOME/.volicord`. Shared
  project host configuration must not carry a personal Runtime Home path, so
  each user supplies the value through their own local init and host
  environment. Other explicit-binding modes retain their owner-defined normal
  Runtime Home selection rules.

Runtime Home selection and exact creation behavior are owned by [Administrative CLI](admin-cli.md) and [MCP Transport](mcp-transport.md). Runtime location and separation rules are owned by [Runtime Boundaries](runtime-boundaries.md).

## Product Repository Requirements

A `Product Repository` must be an existing local directory for project registration, project selection, and shared-intent host setup. It must remain separate from `Volicord Runtime Home`. On native Windows, use a local drive-letter path for the Product Repository; UNC paths and WSL paths are not supported for native Windows project registration.

Read access is required when Volicord validates or uses the registered project. Write access to the `Product Repository` is required only for owner-defined product-file writes or explicitly requested integration files, including:

- project-scoped Codex `.codex/config.toml`
- project-scoped Claude Code `.mcp.json`
- Volicord-managed `AGENTS.md` guidance blocks
- `.volicord/policy.json` local managed `volicord-policy-v2` mirrors
- Codex `.codex/hooks.json` hook configuration and Volicord-managed wrapper
  scripts under `.codex/hooks/`
- Volicord-managed Claude Code hook entries in `.claude/settings.json`
- Volicord-managed Claude Code hook wrapper scripts under `.claude/hooks/`
- Volicord-managed Claude Code rule files under `.claude/rules/`

Applying a generated guard-integration file from this list also requires the
selected filesystem and process to support a conditional same-directory commit.
This requirement applies to managed guidance, policy, hook, wrapper, and rule
files; project-scoped MCP configuration is applied through its host adapter and
does not inherit this guard-integration commit guarantee.

- The resolved Product Repository path and target-parent chain must remain
  directories that can be opened without following symbolic links. An existing
  target must be a regular file, and the target directory must allow creation
  and removal of private sibling staging entries.
- On Linux and macOS, an existing-file update requires native same-directory
  no-replace and exchange operations. The process must be able to read, reapply,
  and verify the predecessor's POSIX mode, user ID, group ID, and all extended
  attributes exposed by the platform interface.
- On native Windows, creation requires the access needed for a `MoveFileExW`
  no-replace move. Updating an existing file additionally requires a local NTFS
  volume that supports same-volume hard links, the ability to deny new write
  sharing on the predecessor, and access for `ReplaceFileW` replacement with a
  pre-reserved backup entry. Windows supplies the native attribute and ACL merge
  behavior. ReFS and network filesystems are not supported for this existing-file
  update path; failure to create the preservation hard link fails the update.
- A supported operating-system target does not imply that every network,
  virtual, userspace, or mounted filesystem supplies these namespace and
  metadata semantics. Such filesystems are unverified for managed-file update.
  If the required operation or metadata reproduction is unavailable, the CLI
  fails the write rather than reporting a successful managed update.

Codex detective setup requires the selected Product Repository to be a Git
work tree so generated hooks can resolve the project root without depending on
the host session cwd. This Git-root requirement is for Codex detective host hook path
safety only; it does not make integration files Volicord runtime state or add
OS-level sandboxing. `record` setup does not require Codex detective host hook
installation. The native Windows `record` path covers binary installation and
connection setup only; it establishes no host feature support status. Native
Windows Detective setup is rejected because Windows host hooks and watcher
behavior are unavailable.

Noninteractive shared-intent host configuration or guidance writes require the explicit `--shared` command path defined by [Administrative CLI](admin-cli.md#noninteractive-approval-behavior). Runtime records, SQLite databases, generated records, logs, projections, QA results, acceptance records, close-readiness state, and residual-risk records do not belong in the `Product Repository`.

<a id="host-configuration-requirements"></a>
## Host Configuration Requirements

For direct host configuration setup, the administrative process must be able to inspect the target host configuration and write managed configuration when the selected host and connection intent require it.

Baseline host and connection-intent requirements:

| Host | Connection intent | Environment prerequisite |
|---|---|---|
| Codex | `personal` | `CODEX_HOME` or `HOME` must identify the user Codex configuration location; `codex` must be available on `PATH` for the availability check. |
| Codex | `shared` | The selected `Product Repository` must be writable when applying `.codex/config.toml`; the Codex host must resolve `volicord` through `PATH`, start `mcp --stdio --discover-repository --host codex` from inside the clone, and provide the init-selected nonempty, absolute `VOLICORD_HOME`; the shared entry has no local IDs or literal Runtime Home path and forwards the host value through `env_vars = ["VOLICORD_HOME"]`; Codex project trust may still be required. |
| Claude Code | `personal`, `global` | The `claude` executable must be launchable by the administrative process so Volicord can use `claude mcp` commands. |
| Claude Code | `shared` | The selected `Product Repository` must be writable when applying `.mcp.json`; the Claude Code host must resolve `volicord` through `PATH`, start `mcp --stdio --discover-repository --host claude-code` from inside the clone, and provide the init-selected nonempty, absolute `VOLICORD_HOME`; the shared entry has no local IDs or literal Runtime Home path and forwards the host value through `"env": {"VOLICORD_HOME": "${VOLICORD_HOME}"}`; project MCP approval may still be required. |
| Generic | user-managed | Volicord does not write generic MCP host configuration. An enabled Agent Connection must already exist before an external host can be configured manually, and the resulting process must still pass MCP startup validation. The external host remains user-managed and unverified until loaded and checked by a host-specific mechanism. |

Writing host configuration does not prove that the host trusted, approved, loaded, initialized, or exposed `volicord mcp --stdio`. `managed host configuration state` meaning and host trust boundaries are owned by [Agent Connection](agent-connection.md).

Codex project trust and non-managed command-hook trust are separate
host-controlled prerequisites. Project trust only makes the project `.codex/`
layer eligible to load. Before relying on Detective hooks, the operator must
review and trust each exact current Volicord hook definition in Codex; a changed
definition requires another review. Project trust, an administrative
connection result of `complete`, `hook_path_safety=ok`, exact-owned generated
files, or a successful configuration audit does not prove command-hook trust,
event delivery, or execution.

Host feature applicability is separate from installation applicability and is
capability-probe-first. Managed verification probes actual hook calls,
structured target paths, structured changed paths, model-separated user-action
UI, Stop delivery/replay behavior, fixed-UI authority display, and advertised
MCP capability as applicable. A built-in implemented surface remains
`implemented_unverified` without fresh matching evidence, including on an
unknown newer host version. A current failed probe or down current prerequisite
is `temporarily_unavailable`; `unsupported_by_host` requires explicit capability
absence. `degraded` is diagnostic only. Exact meanings and precedence are owned
by [Host feature support state](agent-connection.md#host-feature-support-state).

The reviewed Codex `0.144.4` coordinate and any exact Claude Code version remain
validation, regression, and release-evidence coordinates. They are preserved as
observed and must not be used as primary runtime activation gates or generalized
into minimum-version promises. A different valid version is evaluated by its
actual probes and evidence.

## MCP Host Environment Requirements

The baseline MCP host environment must be able to start one of the following
accepted local child-process shapes and communicate over stdin/stdout:

- a personal/local binding: `volicord mcp --stdio --connection
  <connection_id> [--project <project_id>]`
- a shared repository descriptor: `volicord mcp --stdio
  --discover-repository --host codex|claude-code`

In the local-binding shape, the IDs name stored internal records and are not
public MCP tool arguments. In discovery shape, the process current directory
must be inside the intended Git clone, that canonical repository must be
registered in the selected Runtime Home, and exactly one enabled shared
connection for the selected host must include it. This is not a network
listener requirement.

The host process environment must provide:

- an executable `volicord` command according to the configured command path or `PATH`
- `VOLICORD_HOME` when the intended Runtime Home is not the default home-derived
  location; personal/local configuration may carry it
- for every shared repository-discovery launch, a present, nonempty, absolute
  `VOLICORD_HOME` matching the Runtime Home selected by init; the portable host
  entry forwards this value but does not embed its path, and startup does not
  substitute the platform default
- local filesystem access to the Runtime Home and each explicitly allowed `Product Repository`

For the reviewed Codex `0.144.4` managed path, the launch descriptor establishes
provenance only. Exact `clientInfo.name=codex-mcp-client`,
`clientInfo.version=0.144.4`, and strict request-side
`_meta.threadId` plus `_meta["x-codex-turn-metadata"]` session/thread/turn
metadata are required before the first known tool call binds the managed root
session and process-local thread digest. Pending launch creates no managed
effects. `CODEX_THREAD_ID`, PID, cwd, process ancestry, timing, and hook-event
proximity are not substitutes; exact behavior belongs to
[MCP Transport](mcp-transport.md#managed-host-session-input).

`volicord mcp --check --connection <connection_id>` is a startup validation check for that process binding. It is not complete host integration verification. Complete host verification requires the administrative result gates defined by [Administrative CLI](admin-cli.md).

## Stop Criteria

Stop before installation when any of these conditions apply:

- Rust 1.85+ with Cargo is unavailable and you are using the source build path.
- The selected release source does not provide the installer, matching target archive, and checksum required by the documented release path.
- When using published release assets, no supported target matches the operating system and CPU architecture.
- The install script reports an unsupported platform or unsupported CPU architecture.
- Checksum verification is required locally but the checksum file cannot be downloaded or verified.
- You cannot run or reliably adapt the maintained shell examples for the selected environment.
- `volicord` is missing, is not executable by the selected user, or cannot print help and version output.
- The selected Runtime Home cannot be created, read, or written by the processes that need it.
- The Runtime Home and Product Repository are the same path or one contains the other.
- Native Windows setup uses a UNC path, a WSL UNC path, or WSL mount-style path for the Runtime Home or Product Repository.
- The Product Repository is missing, is not a directory, or is not writable for a requested project-scoped configuration or guidance write.
- A requested guard-integration managed-file write cannot safely traverse the
  target without following symbolic links, cannot use the required
  same-directory namespace operation, or cannot reproduce the required
  existing-file metadata.
- Shared-intent host configuration cannot start `volicord mcp --stdio` from the host environment's `PATH`.
- A shared repository-discovery host environment cannot provide the
  init-selected `VOLICORD_HOME` as a present, nonempty, absolute value.
- Codex or Claude Code is required for the selected host path but the administrative compatibility check cannot launch or interpret the host.
- Native Windows setup requests `--profile detective`.
- A required host trust, project trust, project MCP approval, OAuth, reload, restart, or comparable host-owned action remains and the operator cannot complete it.
- The selected environment depends on a package manager, a Homebrew tap, a published Docker registry image, a remote host, a network listener, or a host-version promise that this repository does not document.

When repository evidence is insufficient, classify the environment as unverified and use the owner-defined verification commands before relying on it.
