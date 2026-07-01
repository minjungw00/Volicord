# System requirements reference

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
| Release binary installation | Supported and verified for the target triples named in this table. | `.github/workflows/release.yml` builds target-named release tarballs that contain only `volicord`, runs smoke tests against each built binary, and generates `.sha256` files. `scripts/install.sh` selects the same target names and installs only `volicord`. | Use the release binary path for first-run installation when your operating system and CPU architecture match a supported target. |
| Linux x86_64 | Supported and release-packaged as `x86_64-unknown-linux-gnu`. | The release workflow builds on `ubuntu-24.04` and packages `volicord-x86_64-unknown-linux-gnu.tar.gz`. | Use a Linux x86_64 environment with a POSIX-style shell and the install-script tools listed below. |
| Linux aarch64 | Supported and release-packaged as `aarch64-unknown-linux-gnu`. | The release workflow builds on the native `ubuntu-24.04-arm` runner and packages `volicord-aarch64-unknown-linux-gnu.tar.gz`. | Use a Linux aarch64 environment with a POSIX-style shell and the install-script tools listed below. |
| WSL2 | Supported through the matching Linux release binary when WSL2 reports `Linux` through `uname` and uses `x86_64` or `aarch64`. | The install script treats WSL2 as Linux because the observable platform is the Linux userspace. The repository does not add a native Windows binary path. | Use WSL2 with the matching Linux architecture. Do not use native Windows shells for the release binary path. |
| macOS arm64 | Supported and release-packaged as `aarch64-apple-darwin`. | The release workflow builds on a macOS arm64 runner and packages `volicord-aarch64-apple-darwin.tar.gz`. | Use a macOS arm64 environment with a POSIX-style shell and the install-script tools listed below. |
| macOS x86_64 | Supported and release-packaged as `x86_64-apple-darwin`. | The release workflow builds on a macOS Intel runner and packages `volicord-x86_64-apple-darwin.tar.gz`. | Use a macOS x86_64 environment with a POSIX-style shell and the install-script tools listed below. |
| Docker | Supported as a local runtime option when using the checked-in `Dockerfile`. No external image registry is claimed. | The checked-in `Dockerfile` builds the release CLI into a Debian runtime image. The release workflow builds the image and smoke-tests `volicord --help`. The Installation page documents local `docker build` and `docker run` usage. | Build the image from this repository or from a trusted source copy. Do not assume a published registry image exists unless a repository artifact adds it. |
| Native Windows | Out of scope. | No Windows binary target is built by the release workflow. `scripts/install.sh` rejects MINGW, MSYS, CYGWIN, and `Windows_NT` environments and points users to WSL2. | Use WSL2, Linux, macOS, or Docker instead of native Windows. |
| Development source build toolchain | Supported and verified for Rust 1.85 or newer with Cargo, as a development path. | The workspace root `Cargo.toml` sets `rust-version = "1.85"` and all workspace packages inherit that value. The Installation page keeps Cargo commands under the development source-build path. | Install or select Rust 1.85+ with Cargo only when using the development source-build path. |
| Shell syntax | Supported for maintained POSIX-style examples. Other shells are unverified for these examples. | Installation examples use `sh`-compatible environment assignments, `volicord` commands, setup-reported `PATH` actions when needed, home-relative paths such as `~/.local/bin`, slash-separated paths, and `PATH` command lookup. CLI integration tests create `#!/bin/sh` fake executables behind `#[cfg(unix)]` and set executable bits with `std::os::unix::fs::PermissionsExt`. | If your shell cannot run that syntax or expand those paths, translate the commands yourself and verify each resulting command before continuing. |
| Executable role names | Supported and verified. | Reference owners define `volicord` as the installed executable for administrative CLI commands and the `mcp` subcommand used by the local MCP stdio adapter. | Build or install `volicord`; host configuration should start MCP with `volicord mcp --stdio ...`. |
| Package-manager installation | Out of scope unless a matching repository artifact is added. | No Homebrew tap, Homebrew formula, Linux package-manager package, or external package registry is represented in this checkout. The supported first-run path is the release tarball plus install script. | Use the release binary install script, Docker, an existing `volicord` executable, or the development source-build path. |
| Host version minimums for Codex and Claude Code | No stable minimum host version is defined. Host compatibility is checked operationally, not by a documented version floor. | Codex verification looks for `codex` on `PATH` and runs `codex --version`. Claude Code verification inspects host state through `claude mcp get <server_name>`. Administrative verification owns the final result states. | Use `volicord connection verify HOST [--repo PATH] [--shared|--global]` after installation. Do not rely on an undocumented Codex or Claude Code minimum version. |

## Toolchain Requirements

Release binary installation does not require Rust or Cargo.

The development source-build path requires:

- Rust 1.85 or newer.
- Cargo from the selected Rust toolchain.
- A local checkout of this repository.
- Network or local dependency availability sufficient for Cargo to resolve the workspace dependencies.

Rust 1.85 is a compiler requirement for this workspace. It is not required for
release binary installation and is not an operating-system support claim.

Rust implementation validation is not required just to read or use these requirements. Maintainers who edit Rust source, Cargo manifests, tests, fixtures, or build configuration should follow the Rust validation policy in the repository working rules.

## Shell And Path Requirements

Release install examples assume a POSIX-style shell with:

- environment assignment before a command, such as `VOLICORD_REPO=OWNER/REPO sh ./scripts/install.sh`
- `curl` or `wget` for downloading release assets
- `tar` for extracting the target-named release archive
- `awk`, `wc`, `tr`, and `sed` for checksum and archive-shape checks
- `sha256sum` or `shasum` when checksum verification is available
- current-session `PATH` updates when setup prints a shell command
- home-relative paths such as `~/.local/bin`
- command lookup through `PATH`
- forward-slash paths in examples

The install script verifies the downloaded `.sha256` file when that checksum
asset is available. If the checksum file is present but cannot be verified, the
script fails. If the checksum file is unavailable, the script warns and
continues unless `VOLICORD_REQUIRE_CHECKSUM=1` is set.

Current-session `PATH` examples affect only the shell where they are run. They
do not install commands persistently for future shells or MCP hosts.

The CLI cannot permanently edit the parent shell `PATH`. During setup, Volicord
can help make its commands available on `PATH` by offering safe choices such as
command links, creating a missing conventional user command directory such as
`~/.local/bin` when that is safe, a printed shell command, or an explicitly
approved managed shell startup block when the shell is supported. Setup verifies
writability before placing command links. Existing shells and MCP hosts may need
restart or reload before they see a changed startup file or command link
directory.

`VOLICORD_HOME` is different. It is a real Runtime Home selection input for `volicord` administrative commands and `volicord mcp --stdio` process startup, as defined by their owner documents.

## Executable Layout And Discovery

Before installation, one selected executable location must make the installed executable available:

- `volicord`

Release tarballs are expected to contain only:

- `volicord`

The install script installs only that executable. For development source
builds, the debug executable is expected under `target/debug` and the release
executable under `target/release`. For separately installed executables, select
an installation layout where setup can find `volicord` through an explicit
setup option or `PATH`.

Before setup from a release binary or another installed command directory,
verify the installed executable from the same shell:

```sh
volicord --version
volicord --help
volicord mcp --help
volicord init --help
volicord setup --help
volicord guard --help
volicord serve --help
```

Before setup from a development source build, verify the built executable from
the same shell:

```sh
./target/debug/volicord --version
./target/debug/volicord --help
./target/debug/volicord mcp --help
```

After setup guidance or linking has made the command visible, verify ordinary
command lookup:

```sh
volicord --version
volicord init --help
volicord setup --help
volicord connect --help
volicord mcp --version
volicord mcp --help
```

Host configuration normally uses MCP command information established by
`volicord init`; `volicord setup` can prepare or repair that installation
profile directly.
For exact `--mcp-command`, discovery-order, `--link-bin`, connection, and
generic export behavior, use
[Administrative CLI](admin-cli.md#runtime-home-selection) and
[Generic MCP config export](admin-cli.md#generic-mcp-config-export).

Requirement summary:

- The installation profile must identify a `volicord` command that can be
  found.
- Future host processes must be able to start the configured `volicord`
  command with `mcp --stdio --connection <connection_id>` arguments.
- Shared project host configuration must not embed a personal Runtime Home
  path. It uses `volicord` as a command name that the future host environment
  must resolve through `PATH`.
- Generic export can render explicit configuration, but it remains user-managed
  until a host-specific owner defines an observable loadability gate.

## Runtime Home Requirements

A usable `Volicord Runtime Home` must be a local filesystem location the selected process can create, read, and write when the requested administrative or MCP operation needs runtime records.

Before installation:

- Select a Runtime Home that is not the `Product Repository` and is not inside or above the `Product Repository`.
- Ensure the selected user can create the directory or write into it when running `volicord init`, `volicord setup`, `volicord project use`, `volicord connect`, or `volicord connection verify`.
- Ensure future `volicord mcp --stdio` host processes receive the same Runtime Home selection when the default `$HOME/.volicord` is not the intended location. Shared project host configuration must not carry a personal Runtime Home path, so each user must provide a non-default Runtime Home through their own local init, profile setup, or environment.

Runtime Home selection and exact creation behavior are owned by [Administrative CLI](admin-cli.md) and [MCP Transport](mcp-transport.md). Runtime location and separation rules are owned by [Runtime Boundaries](runtime-boundaries.md).

## Product Repository Requirements

A `Product Repository` must be an existing local directory for project registration, project selection, and shared-intent host setup. It must remain separate from `Volicord Runtime Home`.

Read access is required when Volicord validates or uses the registered project. Write access to the `Product Repository` is required only for owner-defined product-file writes or explicitly requested integration files, including:

- project-scoped Codex `.codex/config.toml`
- project-scoped Claude Code `.mcp.json`
- Volicord-managed `AGENTS.md` guidance blocks
- `.volicord/policy.json` guard policy files
- Volicord-managed Claude Code rule files under `.claude/rules/`

Noninteractive shared-intent host configuration or guidance writes require the explicit `--shared` command path defined by [Administrative CLI](admin-cli.md#noninteractive-approval-behavior). Runtime records, SQLite databases, generated records, logs, projections, QA results, acceptance records, close-readiness state, and residual-risk records do not belong in the `Product Repository`.

<a id="host-configuration-requirements"></a>
## Host Configuration Requirements

For direct host configuration setup, the administrative process must be able to inspect the target host configuration and write managed configuration when the selected host and connection intent require it.

Baseline host and connection-intent requirements:

| Host | Connection intent | Environment prerequisite |
|---|---|---|
| Codex | `personal` | `CODEX_HOME` or `HOME` must identify the user Codex configuration location; `codex` must be available on `PATH` for the availability check. |
| Codex | `shared` | The selected `Product Repository` must be writable when applying `.codex/config.toml`; the future Codex host must be able to start `volicord mcp --stdio` through `PATH`; the shared file must not embed a personal Runtime Home path; Codex project trust may still be required. |
| Claude Code | `personal`, `global` | The `claude` executable must be launchable by the administrative process so Volicord can use `claude mcp` commands. |
| Claude Code | `shared` | The selected `Product Repository` must be writable when applying `.mcp.json`; the future Claude Code host must be able to start `volicord mcp --stdio` through `PATH`; the shared file must not embed a personal Runtime Home path; project MCP approval may still be required. |
| Generic | `export` | A writable export target is needed only when writing an export file. The external host remains user-managed and unverified until loaded and checked by a host-specific mechanism. |

Writing host configuration does not prove that the host trusted, approved, loaded, initialized, or exposed `volicord mcp --stdio`. `managed host configuration state` meaning and host trust boundaries are owned by [Agent Connection](agent-connection.md).

## MCP Host Environment Requirements

The baseline MCP host environment must be able to start `volicord mcp --stdio --connection <connection_id>` as a local child process and communicate over stdin/stdout. The `connection_id` process argument names the stored `connection_internal_id` written by generated host configuration or generic export output; it is not a public MCP tool argument. This is not a network listener requirement.

The host process environment must provide:

- an executable `volicord` command according to the configured command path or `PATH`
- `VOLICORD_HOME` when the intended Runtime Home is not the default home-derived location and the host configuration is allowed to carry a personal environment value
- local filesystem access to the Runtime Home and each explicitly allowed `Product Repository`

`volicord mcp --check --connection <connection_id>` is a startup validation check for that process binding. It is not complete host integration verification. Complete host verification requires the administrative result gates defined by [Administrative CLI](admin-cli.md).

## Stop Criteria

Stop before installation when any of these conditions apply:

- Rust 1.85+ with Cargo is unavailable and you are using the source build path.
- No supported release binary target matches the operating system and CPU architecture.
- The install script reports an unsupported platform or unsupported CPU architecture.
- Checksum verification is required locally but the checksum file cannot be downloaded or verified.
- You cannot run or reliably translate the maintained POSIX-style shell examples.
- `volicord` is missing, is not executable by the selected user, or cannot print help and version output.
- The selected Runtime Home cannot be created, read, or written by the processes that need it.
- The Runtime Home and Product Repository are the same path or one contains the other.
- The Product Repository is missing, is not a directory, or is not writable for a requested project-scoped configuration or guidance write.
- Shared-intent host configuration cannot start `volicord mcp --stdio` from the future host environment's `PATH`.
- Codex or Claude Code is required for the selected host path but the administrative compatibility check cannot launch or interpret the host.
- A required host trust, project trust, project MCP approval, OAuth, reload, restart, or comparable host-owned action remains and the operator cannot complete it.
- The planned environment depends on native Windows, PowerShell, a package manager, a Homebrew tap, a published Docker registry image, a remote host, a network listener, or a host-version promise that this repository does not document.

When repository evidence is insufficient, classify the environment as unverified and use the owner-defined verification commands before relying on it.
