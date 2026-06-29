# Installation

This tutorial prepares the local `volicord` and `volicord-mcp` executables and
records the installation profile used by later project, connection, export, and
`User Channel` commands. It is the setup step before the
[Quickstart](quickstart.md).

Exact command behavior belongs to
[Administrative CLI Reference](../reference/admin-cli.md). Runtime location and
repository separation belong to [Runtime Boundaries](../reference/runtime-boundaries.md).

## Prerequisites

- Rust 1.85 or newer, as listed in
  [System Requirements](../reference/system-requirements.md).
- A shell that can run Cargo and local binaries.
- A product repository that is a Git repository when you are ready to connect a
  host.

## Build From Source

From the Volicord source repository:

```sh
cargo build --workspace --bins
```

This builds both local executables:

- `./target/debug/volicord`
- `./target/debug/volicord-mcp`

Then create the installation profile:

```sh
export PATH="$PWD/target/debug:$PATH"
volicord setup
```

The `export PATH=...` line affects only the current terminal session. It lets
that shell find the freshly built `volicord` and `volicord-mcp` commands.
`volicord setup` creates or verifies the selected `Volicord Runtime Home` and
saves the installation profile. Exact setup options, MCP command discovery
order, and output behavior belong to
[Administrative CLI Reference](../reference/admin-cli.md#runtime-home-selection).

If you want persistent command links from this source build, run setup with
`--link-bin`:

```sh
volicord setup --link-bin ~/.local/bin
```

When `--link-bin` is supplied, setup prepares both `volicord` and
`volicord-mcp` commands in that directory when feasible. The CLI can report the
needed `PATH` action, but it cannot permanently edit the parent shell
environment. Add `~/.local/bin` to your shell configuration if it is not already
there, then start new shells or MCP hosts from that environment.

Check setup readiness:

```sh
volicord doctor
```

`doctor` reports `complete` when setup is usable. `action_required` names a
specific local repair action, such as rerunning setup or fixing an executable
path.

## Use Installed Executables

If `volicord` and `volicord-mcp` already exist on `PATH`, run:

```sh
volicord setup
volicord doctor
```

Setup uses the same installation-profile contract whether the executables came
from a source build or an installed command directory. Use
`volicord setup --mcp-command PATH` only when the default discovery described by
the CLI reference cannot find the `volicord-mcp` executable you intend to use.
Ordinary `volicord connect` commands use the saved installation profile.

## What Setup Does Not Do

Setup does not register a product repository and does not install host
configuration. Project registration happens when you run `volicord project use`
or a command such as `volicord connect` from inside a Git repository.

Project naming and internal identity behavior are owned by the
[Administrative CLI Reference](../reference/admin-cli.md#project-commands).
Internal identities are stored by Volicord and are not first-time setup inputs.

## Next Step

Move into the product repository and connect a host:

```sh
cd /path/to/your-product-repo
volicord connect codex
```

`/path/to/your-product-repo` is a placeholder for the Git product repository
you want the host to work on.

For the full first-run path, continue with the [Quickstart](quickstart.md). For
host-specific details, see [Agent Host Setup](../guides/agent-host-setup.md).
