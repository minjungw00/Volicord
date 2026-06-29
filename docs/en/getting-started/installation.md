# Installation

This tutorial prepares the local `volicord` and `volicord-mcp` executables and
records the setup profile used by later project, connection, export, and
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

Then create the setup profile:

```sh
./target/debug/volicord setup --link-bin ~/.local/bin
```

`volicord setup` creates or verifies the selected `Volicord Runtime Home`, finds
`volicord-mcp`, and saves the setup profile. MCP command discovery checks an
explicit `--mcp-command PATH` first when supplied, then a sibling
`volicord-mcp` next to the running `volicord`, then `PATH`.

When `--link-bin` is supplied, setup prepares both `volicord` and
`volicord-mcp` commands in that directory when feasible. The CLI can report the
needed `PATH` action, but it cannot permanently edit the parent shell
environment. Add `~/.local/bin` to your shell configuration if it is not already
there, then start new shells or MCP hosts from that environment.

Check setup readiness:

```sh
volicord doctor
```

`doctor` reports `complete` when the Runtime Home, setup profile, stored command
paths, and applicable command links are usable. `action_required` means the
command found a specific local repair action, such as rerunning setup or fixing
an executable path.

## Use Installed Executables

If `volicord` and `volicord-mcp` already exist on `PATH`, run:

```sh
volicord setup
volicord doctor
```

Setup discovers the MCP command from the running installation by sibling lookup
or `PATH` lookup. Use `volicord setup --mcp-command PATH` only when discovery
cannot find the `volicord-mcp` executable you intend to use. Ordinary
`volicord connect` commands use the saved setup profile in the resolved Runtime
Home; they do not ask for an MCP command path, Runtime Home path, project id,
internal host value, or registry value.

## What Setup Does Not Do

Setup does not register a product repository and does not install host
configuration. Project registration happens when you run `volicord project use`
or a command such as `volicord connect` from inside a Git repository.

The repository project name is derived from the repository directory and made
unique inside the selected Runtime Home when needed. Internal ids are stored by
Volicord and are not first-time setup inputs.

## Next Step

Move into the product repository and connect a host:

```sh
cd /work/acme-api
volicord connect codex
```

For the full first-run path, continue with the [Quickstart](quickstart.md). For
host-specific details, see [Agent Host Setup](../guides/agent-host-setup.md).
