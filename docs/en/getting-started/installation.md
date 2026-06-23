# Installation

This page owns the first setup stage: preparing the Harness Server executables. It covers source prerequisites, build commands, executable paths, and build verification for the current repository executables. It does not define package-manager distribution, operating-system support, public API behavior, storage effects, Product Repository registration, host trust, or MCP wire behavior.

## Prerequisites

For the authoritative pre-installation environment classification, read [System Requirements](../reference/system-requirements.md).

For the source build path, you need:

- a local checkout of this repository
- Rust 1.85 or newer with Cargo; Rust 1.85 is the minimum compiler version verified for the current workspace
- a shell that can run Cargo and local executables

For the next setup stage, you also need:

- a local `Product Repository` directory
- a separate `Harness Runtime Home`
- Codex, Claude Code, or another MCP host when you are ready to connect an agent host

## Build From The Repository Root

Working directory: Harness Server source repository root.

For a debug source build:

```sh
cargo build -p harness-cli -p harness-mcp
export HARNESS_BIN="$(pwd)/target/debug"

test -x "$HARNESS_BIN/harness"
test -x "$HARNESS_BIN/harness-mcp"
```

For a release source build:

```sh
cargo build --release -p harness-cli -p harness-mcp
export HARNESS_BIN="$(pwd)/target/release"
```

For separately installed executables:

```sh
export HARNESS_BIN="/absolute/path/to/installed/bin"
```

Choose one absolute directory that contains both `harness` and `harness-mcp`. `HARNESS_BIN` is a shell convenience variable for these examples; Harness does not read it as configuration. The Cargo package names are `harness-cli` and `harness-mcp`. The executable names are `harness` and `harness-mcp`.

## Verify The Build

After selecting `HARNESS_BIN`, verify the executables from the same shell:

```sh
"$HARNESS_BIN/harness" --version
"$HARNESS_BIN/harness" agent --help
"$HARNESS_BIN/harness-mcp" --version
"$HARNESS_BIN/harness-mcp" --help
```

The version commands print `harness <version>` and `harness-mcp <version>`. The help commands should print the `harness agent` command family and the integration-bound `harness-mcp --integration <integration_id>` process usage.

## Executable Discovery During Setup

`harness agent install` installs or exports host configuration that starts `harness-mcp --integration <integration_id>`.

For user-scope Codex or user/local-scope Claude Code setup, pass the selected absolute executable path with `--mcp-command "$HARNESS_BIN/harness-mcp"`, or put `harness-mcp` beside `harness` or on `PATH` so the CLI can discover it. The persisted host configuration receives the resolved absolute command path, not the shell variable.

For project-scoped Codex or Claude Code setup, the generated project file must remain shareable. Run setup with `PATH="$HARNESS_BIN:$PATH"` and use `--mcp-command harness-mcp` or omit `--mcp-command`. The project file keeps the portable command name, and the host environment must be able to find `harness-mcp` on `PATH`.

Installation location is not runtime state. Harness Server source or installation files contain executables, `Harness Runtime Home` contains Harness runtime records, `Product Repository` contains product files and selected project-scoped integration files, and the agent host owns its actual configuration and trust state.

## Next Step

Continue to [Quickstart](quickstart.md). It starts from a real supported host path for Codex or Claude Code.

Exact command behavior belongs to [Administrative CLI](../reference/admin-cli.md). Exact `harness-mcp` startup, environment, stdio transport, preflight, and shutdown behavior belongs to [MCP Transport](../reference/mcp-transport.md).
