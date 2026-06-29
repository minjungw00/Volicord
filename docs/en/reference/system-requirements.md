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
| Source build toolchain | Supported and verified for Rust 1.85 or newer with Cargo. | The workspace root `Cargo.toml` sets `rust-version = "1.85"` and all workspace packages inherit that value. The Installation page uses Cargo build commands for the administrative CLI and MCP adapter source packages. | Install or select Rust 1.85+ with Cargo before using the source build path. |
| Operating-system family | No named OS family is declared as generally supported by this checkout. POSIX-style command examples and Unix-gated tests are verified as repository evidence, not as a promise for every POSIX system. | Maintained examples use `sh` fences with Cargo commands, relative executable paths such as `./target/debug/volicord`, home-relative paths such as `~/.local/bin`, slash-separated paths, and `PATH` command lookup. CLI integration tests create `#!/bin/sh` fake executables behind `#[cfg(unix)]` and set executable bits with `std::os::unix::fs::PermissionsExt`. No checked-in CI workflow matrix is present in this checkout. | Use a POSIX-style shell for maintained command examples. Treat named OSes, containers, WSL, remote shells, Windows `cmd.exe`, and PowerShell as unverified unless a future owner document adds evidence. |
| Shell syntax | Supported for the maintained POSIX-style examples. Other shells are unverified for these examples. | Installation examples use `cargo build --workspace --bins`, `./target/debug/volicord setup --link-bin ~/.local/bin`, and plain `volicord connect ...` commands after setup or linking. | If your shell cannot run that syntax or expand those paths, translate the commands yourself and verify each resulting command before continuing. |
| Executable role names | Supported and verified. | Reference owners define `volicord` as the administrative CLI role and `volicord-mcp` as the local MCP adapter role. | Build or install both `volicord` and `volicord-mcp`; do not treat one executable as a substitute for the other. |
| Package-manager installation | Out of scope. | The Installation page documents source build and separately installed executable discovery, but no package-manager procedure or release layout is defined in repository owners. | Use the source build path or an already installed executable directory that contains both executables. |
| Host version minimums for Codex and Claude Code | No stable minimum host version is defined. Host compatibility is checked operationally, not by a documented version floor. | Codex verification looks for `codex` on `PATH` and runs `codex --version`. Claude Code verification inspects host state through `claude mcp get <server_name>`. Administrative verification owns the final result states. | Use `volicord connection verify HOST [--repo PATH] [--shared|--global]` after installation. Do not rely on an undocumented Codex or Claude Code minimum version. |

## Toolchain Requirements

The source build path requires:

- Rust 1.85 or newer.
- Cargo from the selected Rust toolchain.
- A local checkout of this repository.
- Network or local dependency availability sufficient for Cargo to resolve the workspace dependencies.

Rust 1.85 is a compiler requirement for this workspace. It is not an operating-system support claim.

Rust implementation validation is not required just to read or use these requirements. Maintainers who edit Rust source, Cargo manifests, tests, fixtures, or build configuration should follow the Rust validation policy in the repository working rules.

## Shell And Path Requirements

Maintained command examples assume a POSIX-style shell with:

- Cargo command invocation such as `cargo build --workspace --bins`
- relative executable paths such as `./target/debug/volicord`
- home-relative paths such as `~/.local/bin`
- command lookup through `PATH`
- forward-slash paths in examples

The CLI cannot permanently edit the parent shell `PATH`. When `volicord setup
--link-bin PATH` prepares command links, add that directory to your shell
configuration before starting new shells or MCP hosts if the command is not
already visible.

`VOLICORD_HOME` is different. It is a real Runtime Home selection input for `volicord` administrative commands and `volicord-mcp` process startup, as defined by their owner documents.

## Executable Layout And Discovery

Before installation, one selected executable location must make both roles available:

- `volicord`
- `volicord-mcp`

For source builds, the debug executables are expected under `target/debug` and release executables under `target/release`. For separately installed executables, select an installation layout where setup can find both executable roles through an explicit setup option, sibling discovery, or `PATH`.

Before setup from a source build, verify the built executables from the same
shell:

```sh
./target/debug/volicord --version
./target/debug/volicord setup --help
./target/debug/volicord-mcp --version
./target/debug/volicord-mcp --help
```

After setup, linking, or separate installation has made the commands visible,
verify ordinary command lookup:

```sh
volicord --version
volicord setup --help
volicord connect --help
volicord-mcp --version
volicord-mcp --help
```

Host configuration uses MCP command information established by `volicord setup`.
For exact `--mcp-command`, discovery-order, `--link-bin`, connection, and
generic export behavior, use
[Administrative CLI](admin-cli.md#runtime-home-selection) and
[Generic MCP config export](admin-cli.md#generic-mcp-config-export).

Requirement summary:

- Setup must be able to find both `volicord` and `volicord-mcp`.
- Future host processes must be able to start the configured `volicord-mcp`
  command.
- Shared project host configuration must not embed a personal Runtime Home
  path. It uses `volicord-mcp` as a command name that the future host
  environment must resolve through `PATH`.
- Generic export can render explicit configuration, but it remains user-managed
  until a host-specific owner defines an observable loadability gate.

## Runtime Home Requirements

A usable `Volicord Runtime Home` must be a local filesystem location the selected process can create, read, and write when the requested administrative or MCP operation needs runtime records.

Before installation:

- Select a Runtime Home that is not the `Product Repository` and is not inside or above the `Product Repository`.
- Ensure the selected user can create the directory or write into it when running `volicord setup`, `volicord project use`, `volicord connect`, or `volicord connection verify`.
- Ensure future `volicord-mcp` host processes receive the same Runtime Home selection when the default `$HOME/.volicord` is not the intended location. Shared project host configuration must not carry a personal Runtime Home path, so each user must provide a non-default Runtime Home through their own local setup or environment.

Runtime Home selection and exact creation behavior are owned by [Administrative CLI](admin-cli.md) and [MCP Transport](mcp-transport.md). Runtime location and separation rules are owned by [Runtime Boundaries](runtime-boundaries.md).

## Product Repository Requirements

A `Product Repository` must be an existing local directory for project registration, project selection, and shared-intent host setup. It must remain separate from `Volicord Runtime Home`.

Read access is required when Volicord validates or uses the registered project. Write access to the `Product Repository` is required only for owner-defined product-file writes or explicitly requested integration files, including:

- project-scoped Codex `.codex/config.toml`
- project-scoped Claude Code `.mcp.json`
- optional Volicord-managed guidance blocks or files

Noninteractive shared-intent host configuration or guidance writes require the explicit `--shared` command path defined by [Administrative CLI](admin-cli.md#noninteractive-approval-behavior). Runtime records, SQLite databases, generated records, logs, projections, QA results, acceptance records, close-readiness state, and residual-risk records do not belong in the `Product Repository`.

<a id="host-configuration-requirements"></a>
## Host Configuration Requirements

For direct host configuration setup, the administrative process must be able to inspect the target host configuration and write managed configuration when the selected host and connection intent require it.

Baseline host and connection-intent requirements:

| Host | Connection intent | Environment prerequisite |
|---|---|---|
| Codex | `personal` | `CODEX_HOME` or `HOME` must identify the user Codex configuration location; `codex` must be available on `PATH` for the availability check. |
| Codex | `shared` | The selected `Product Repository` must be writable when applying `.codex/config.toml`; the future Codex host must be able to start `volicord-mcp` through `PATH`; the shared file must not embed a personal Runtime Home path; Codex project trust may still be required. |
| Claude Code | `personal`, `global` | The `claude` executable must be launchable by the administrative process so Volicord can use `claude mcp` commands. |
| Claude Code | `shared` | The selected `Product Repository` must be writable when applying `.mcp.json`; the future Claude Code host must be able to start `volicord-mcp` through `PATH`; the shared file must not embed a personal Runtime Home path; project MCP approval may still be required. |
| Generic | `export` | A writable export target is needed only when writing an export file. The external host remains user-managed and unverified until loaded and checked by a host-specific mechanism. |

Writing host configuration does not prove that the host trusted, approved, loaded, initialized, or exposed `volicord-mcp`. `managed host configuration state` meaning and host trust boundaries are owned by [Agent Connection](agent-connection.md).

## MCP Host Environment Requirements

The baseline MCP host environment must be able to start `volicord-mcp --connection <connection_id>` as a local child process and communicate over stdin/stdout. The `connection_id` process argument names the stored `connection_internal_id` written by generated host configuration or generic export output; it is not a public MCP tool argument. This is not a network listener requirement.

The host process environment must provide:

- an executable `volicord-mcp` command according to the configured command path or `PATH`
- `VOLICORD_HOME` when the intended Runtime Home is not the default home-derived location and the host configuration is allowed to carry a personal environment value
- local filesystem access to the Runtime Home and each explicitly allowed `Product Repository`

`volicord-mcp --check --connection <connection_id>` is a startup validation check for that process binding. It is not complete host integration verification. Complete host verification requires the administrative result gates defined by [Administrative CLI](admin-cli.md).

## Stop Criteria

Stop before installation when any of these conditions apply:

- Rust 1.85+ with Cargo is unavailable and you are using the source build path.
- You cannot run or reliably translate the maintained POSIX-style shell examples.
- `volicord` or `volicord-mcp` is missing, is not executable by the selected user, or cannot print help and version output.
- The selected Runtime Home cannot be created, read, or written by the processes that need it.
- The Runtime Home and Product Repository are the same path or one contains the other.
- The Product Repository is missing, is not a directory, or is not writable for a requested project-scoped configuration or guidance write.
- Shared-intent host configuration cannot start `volicord-mcp` from the future host environment's `PATH`.
- Codex or Claude Code is required for the selected host path but the administrative compatibility check cannot launch or interpret the host.
- A required host trust, project trust, project MCP approval, OAuth, reload, restart, or comparable host-owned action remains and the operator cannot complete it.
- The planned environment depends on Windows, PowerShell, a package manager, a container image, a remote host, a network listener, or a host-version promise that this repository does not document.

When repository evidence is insufficient, classify the environment as unverified and use the owner-defined verification commands before relying on it.
