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

Then run guided setup from the freshly built CLI:

```sh
./target/debug/volicord setup
```

`volicord setup` creates or verifies the selected `Volicord Runtime Home` and
saves the installation profile. It discovers the running `volicord` executable,
looks for `volicord-mcp`, and checks whether the selected commands are available
on `PATH` for future terminals and agent hosts. Exact setup options, MCP command
discovery order, and output behavior belong to
[Administrative CLI Reference](../reference/admin-cli.md#runtime-home-selection).

In an interactive terminal, setup may offer command-availability choices when
the selected executables are not ready on `PATH`:

- create command links in a suggested directory
- create command links and, after explicit approval, add a managed `PATH` block
  to a supported shell startup file
- create command links and print the shell command to run yourself
- print a shell command for manual `PATH` repair
- skip command linking for now

Shell startup file changes are never implicit. If setup can identify a
supported shell startup file, it shows the target file and managed block and
asks for approval before writing. The managed block is Volicord-owned and does
not rewrite unrelated shell configuration. Unsupported shells or platforms
require manual action.

Setup cannot change the parent shell's current `PATH`. A printed
`export PATH=...` command affects only the terminal where you run it. If setup
writes or asks you to update a shell startup file, open a new shell or restart
or reload existing agent host processes before expecting them to see the
commands.

For automation or deterministic local layouts, use explicit setup options:

| Option | When to use it |
|---|---|
| `--link-bin PATH` | Create or update command links in a specific directory. This does not by itself edit shell startup files. |
| `--mcp-command PATH` | Store a specific `volicord-mcp` executable when sibling discovery or `PATH` lookup would choose the wrong command or cannot find one. |
| `--home PATH` | Select a non-default `Volicord Runtime Home`. |

For example, a noninteractive link step can choose the link directory:

```sh
./target/debug/volicord setup --link-bin ~/.local/bin
```

After completing any prompt or action-required command-availability step, check
setup readiness:

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
If setup reports `action_required`, complete the named local action before
starting new terminals or agent hosts. Ordinary `volicord connect` commands use
the saved installation profile.

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
