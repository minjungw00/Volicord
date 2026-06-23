# System requirements reference

This document owns environment applicability and prerequisites that a reader should check before installing Harness Server executables or connecting an MCP host. It classifies operating environment, shell, toolchain, executable layout, filesystem access, Runtime Home, Product Repository, and MCP host prerequisites using evidence available in this repository.

This document does not define administrative command behavior, MCP stdio behavior, storage effects, host trust, public API behavior, schemas, or security guarantees. Exact behavior remains with [Administrative CLI](admin-cli.md), [MCP Transport](mcp-transport.md), [Runtime Boundaries](runtime-boundaries.md), and [Agent Integration](agent-integration.md).

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
| Source build toolchain | Supported and verified for Rust 1.85 or newer with Cargo. | The workspace root `Cargo.toml` sets `rust-version = "1.85"` and all workspace packages inherit that value. The Installation page uses Cargo build commands for `harness-cli` and `harness-mcp`. | Install or select Rust 1.85+ with Cargo before using the source build path. |
| Operating-system family | No named OS family is declared as generally supported by this checkout. POSIX-style command examples and Unix-gated tests are verified as repository evidence, not as a promise for every POSIX system. | Maintained examples use `sh` fences with `export`, command substitution, inline environment assignment, colon-separated `PATH`, and `test -x`. CLI integration tests create `#!/bin/sh` fake executables behind `#[cfg(unix)]` and set executable bits with `std::os::unix::fs::PermissionsExt`. No checked-in CI workflow matrix is present in this checkout. | Use a POSIX-style shell for maintained command examples. Treat named OSes, containers, WSL, remote shells, Windows `cmd.exe`, and PowerShell as unverified unless a future owner document adds evidence. |
| Shell syntax | Supported for the maintained POSIX-style examples. Other shells are unverified for these examples. | Installation examples use `export HARNESS_BIN="$(pwd)/target/debug"`, quoted variable expansion, `PATH="$HARNESS_BIN:$PATH"`, and `test -x`. | If your shell cannot run that syntax, translate the commands yourself and verify each resulting command before continuing. |
| Cargo package and executable names | Supported and verified. | `crates/harness-cli/Cargo.toml` defines binary `harness`; `crates/harness-mcp/Cargo.toml` defines the `harness-mcp` package and executable role. | Build or install both `harness` and `harness-mcp`; do not treat one executable as a substitute for the other. |
| Package-manager installation | Out of scope. | The Installation page documents source build and separately installed executable discovery, but no package-manager procedure or release layout is defined in repository owners. | Use the source build path or an already installed executable directory that contains both executables. |
| Host version minimums for Codex and Claude Code | No stable minimum host version is defined. Host compatibility is checked operationally, not by a documented version floor. | Codex verification looks for `codex` on `PATH` and runs `codex --version`. Claude Code verification inspects host state through `claude mcp get <server_name>`. Administrative verification owns the final result states. | Use `harness agent verify --integration-id <id>` after installation. Do not rely on an undocumented Codex or Claude Code minimum version. |

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

- `export NAME=value`
- `$(pwd)` command substitution
- quoted variable expansion such as `"$HARNESS_BIN/harness"`
- inline environment assignment such as `PATH="$HARNESS_BIN:$PATH" command ...`
- `test -x`
- forward-slash paths in examples
- colon-separated `PATH`

`HARNESS_BIN` is only a tutorial shell variable used by the documentation examples. Harness does not read `HARNESS_BIN` as configuration. The host configuration receives either an absolute executable path or the portable command name `harness-mcp`, depending on scope.

`HARNESS_HOME` is different. It is a real Runtime Home selection input for `harness` administrative commands and `harness-mcp` process startup, as defined by their owner documents.

## Executable Layout And Discovery

Before installation, one selected executable location must make both roles available:

- `harness`
- `harness-mcp`

For source builds, the debug executables are expected under `target/debug` and release executables under `target/release`. For separately installed executables, select one absolute directory that contains both executable roles.

Before continuing, verify from the same shell:

```sh
"$HARNESS_BIN/harness" --version
"$HARNESS_BIN/harness" agent --help
"$HARNESS_BIN/harness-mcp" --version
"$HARNESS_BIN/harness-mcp" --help
```

For POSIX-style shells, `test -x "$HARNESS_BIN/harness"` and `test -x "$HARNESS_BIN/harness-mcp"` are the maintained example checks for executable files.

Host configuration has scope-specific command requirements:

- Codex `user` scope and Claude Code `local` or `user` scope require an absolute `harness-mcp` command path when `--mcp-command` is supplied. If omitted, the administrative CLI tries the `harness` executable's sibling directory and then `PATH`.
- Codex and Claude Code `project` scope must use the portable command name `harness-mcp`; the future host process must be able to find `harness-mcp` on its own `PATH`.
- Generic export can render explicit configuration, but it remains user-managed until a host-specific owner defines an observable loadability gate.

## Runtime Home Requirements

A usable `Harness Runtime Home` must be a local filesystem location the selected process can create, read, and write when the requested administrative or MCP operation needs runtime records.

Before installation:

- Select a Runtime Home that is not the `Product Repository` and is not inside or above the `Product Repository`.
- Ensure the selected user can create the directory or write into it when running `harness init`, project registration, agent installation, or verification.
- Ensure future `harness-mcp` host processes receive the same Runtime Home selection when the default `$HOME/.harness` is not the intended location.

Runtime Home selection and exact creation behavior are owned by [Administrative CLI](admin-cli.md) and [MCP Transport](mcp-transport.md). Runtime location and separation rules are owned by [Runtime Boundaries](runtime-boundaries.md).

## Product Repository Requirements

A `Product Repository` must be an existing local directory for project registration and project-scoped host setup. It must remain separate from `Harness Runtime Home`.

Read access is required when Harness validates or uses the registered project. Write access to the `Product Repository` is required only for owner-defined product-file writes or explicitly requested integration files, including:

- project-scoped Codex `.codex/config.toml`
- project-scoped Claude Code `.mcp.json`
- optional Harness-managed guidance blocks or files

Noninteractive project-scoped host configuration or guidance writes require the explicit repository-write approval flag defined by [Administrative CLI](admin-cli.md). Runtime records, SQLite databases, generated records, logs, projections, QA results, acceptance records, close-readiness state, and residual-risk records do not belong in the `Product Repository`.

## Host Configuration Requirements

For direct host installation, the administrative process must be able to inspect the target host configuration and write managed configuration when the selected host and scope require it.

Baseline host and scope requirements:

| Host | Scope | Environment prerequisite |
|---|---|---|
| Codex | `user` | `CODEX_HOME` or `HOME` must identify the user Codex configuration location; `codex` must be available on `PATH` for the availability check. |
| Codex | `project` | The `Product Repository` must be writable when applying `.codex/config.toml`; the future Codex host must find `harness-mcp` on `PATH`; Codex project trust may still be required. |
| Claude Code | `local`, `user` | The `claude` executable must be launchable by the administrative process so Harness can use `claude mcp` commands. |
| Claude Code | `project` | The `Product Repository` must be writable when applying `.mcp.json`; the future Claude Code host must find `harness-mcp` on `PATH`; project MCP approval may still be required. |
| Generic | `export` | A writable export target is needed only when writing an export file. The external host remains user-managed and unverified until loaded and checked by a host-specific mechanism. |

Installing host configuration does not prove that the host trusted, approved, loaded, initialized, or exposed `harness-mcp`. Host Installation meaning and host trust boundaries are owned by [Agent Integration](agent-integration.md).

## MCP Host Environment Requirements

The baseline MCP host environment must be able to start `harness-mcp --integration <integration_id>` as a local child process and communicate over stdin/stdout. It is not a network listener requirement.

The host process environment must provide:

- an executable `harness-mcp` command according to the configured command path or `PATH`
- `HARNESS_HOME` when the intended Runtime Home is not the default home-derived location
- local filesystem access to the Runtime Home and each explicitly allowed `Product Repository`

`harness-mcp --check --integration <integration_id>` is a startup validation check. It is not complete host integration verification. Complete host verification requires the administrative result gates defined by [Administrative CLI](admin-cli.md).

## Stop Criteria

Stop before installation when any of these conditions apply:

- Rust 1.85+ with Cargo is unavailable and you are using the source build path.
- You cannot run or reliably translate the maintained POSIX-style shell examples.
- `harness` or `harness-mcp` is missing, is not executable by the selected user, or cannot print help and version output.
- The selected Runtime Home cannot be created, read, or written by the processes that need it.
- The Runtime Home and Product Repository are the same path or one contains the other.
- The Product Repository is missing, is not a directory, or is not writable for a requested project-scoped configuration or guidance write.
- Project-scoped host configuration cannot rely on `harness-mcp` from the future host `PATH`.
- Codex or Claude Code is required for the selected host path but the administrative compatibility check cannot launch or interpret the host.
- A required host trust, project trust, project MCP approval, OAuth, reload, restart, or comparable host-owned action remains and the operator cannot complete it.
- The planned environment depends on Windows, PowerShell, a package manager, a container image, a remote host, a network listener, or a host-version promise that this repository does not document.

When repository evidence is insufficient, classify the environment as unverified and use the owner-defined verification commands before relying on it.
