# Installation

This tutorial prepares the local `volicord` executable and records the
installation profile used by later project, connection, export, MCP, and
`User Channel` commands. It is the setup step before the
[Quickstart](quickstart.md).

Exact command behavior belongs to
[Administrative CLI Reference](../reference/admin-cli.md). Runtime location and
repository separation belong to [Runtime Boundaries](../reference/runtime-boundaries.md).

## Prerequisites

- A supported release-binary environment from
  [System Requirements](../reference/system-requirements.md), or Docker when
  using the Docker path below.
- A POSIX-style shell with `curl` or `wget`, `tar`, and a writable install
  directory.
- A Git repository to use as the Product Repository when you are ready to
  connect a host.

## Install A Release Binary

The primary user path is a release binary. The install script detects Linux,
WSL2, or macOS, selects the matching release tarball, verifies the matching
`.sha256` file when it can download one, and installs only the `volicord`
executable. It does not edit shell startup files.

Download or copy `scripts/install.sh` from the same repository that publishes
the Volicord release assets, then run it with the release repository named
explicitly:

```sh
VOLICORD_REPO=OWNER/REPO sh ./scripts/install.sh
```

`OWNER/REPO` is the GitHub repository that hosts the release assets for this
checkout. By default the script downloads from that repository's latest
release. To install a specific tag, set `VOLICORD_VERSION`:

```sh
VOLICORD_REPO=OWNER/REPO VOLICORD_VERSION=v0.1.0 sh ./scripts/install.sh
```

For a non-GitHub release mirror, provide the directory that contains the
target-named tarball and checksum:

```sh
VOLICORD_RELEASE_BASE_URL=https://example.invalid/releases/v0.1.0 sh ./scripts/install.sh
```

The default install directory is `~/.local/bin`. Use `VOLICORD_INSTALL_DIR` to
choose a different directory:

```sh
VOLICORD_REPO=OWNER/REPO VOLICORD_INSTALL_DIR=/usr/local/bin sh ./scripts/install.sh
```

The script fails before downloading on unsupported operating systems or CPU
architectures. If a checksum file is present but cannot be verified, the script
fails. If the checksum file is unavailable, the script warns; set
`VOLICORD_REQUIRE_CHECKSUM=1` when installation must fail instead.

No Homebrew tap, package-manager package, or external package registry is
claimed by this repository unless a matching repository artifact is added.

After installation, verify the installed command:

```sh
volicord --version
volicord --help
volicord mcp --help
volicord init --help
```

For the ordinary first repository connection, continue with
`volicord init --host HOST --repo PATH` in the
[Quickstart](quickstart.md). `volicord init` can initialize the Runtime Home and
installation profile while it connects the selected Product Repository and
writes guarded host integration files.

Use `volicord setup` when you want to prepare or repair only the installation
profile before connecting a repository:

```sh
volicord setup
```

`volicord setup` creates or verifies the selected `Volicord Runtime Home` and
saves the installation profile. It discovers the running `volicord` executable,
stores the MCP launch command, and checks whether the selected command is
available on `PATH` for future terminals and agent hosts. Exact setup options,
MCP launch command behavior, and output behavior belong to
[Administrative CLI Reference](../reference/admin-cli.md#runtime-home-selection).
Its status answers whether the guided first-run setup experience still needs a
named user action, so `action_required` can appear even after the installation
profile has been saved.

In an interactive terminal, setup may offer command-availability choices when
the selected executable is not ready on `PATH`:

- create command links in a suggested directory that setup can verify is
  writable
- create a conventional user command directory such as `~/.local/bin` when it
  is missing and safe to create, then verify writability before linking
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
| `--link-bin PATH` | Create the directory if needed, verify it is writable, then create or update command links there. This does not by itself edit shell startup files. |
| `--mcp-command PATH` | Store a specific `volicord` command for generated MCP launch entries when setup should not use the running executable. |
| `--home PATH` | Select a non-default `Volicord Runtime Home`. |

For example, a noninteractive setup step can still choose a deterministic
command-link directory:

```sh
volicord setup --link-bin ~/.local/bin
```

After completing any prompt or action-required command-availability step, check
setup readiness:

```sh
volicord doctor
```

`doctor` reports installation-profile health, not first-run setup progress. It
reports `complete` when the saved profile is usable, even if it also reports
command-availability warnings or recommended `PATH` and command-link actions
for future shells or agent hosts. `action_required` names a blocking local
repair action, such as rerunning setup or fixing an executable path.

## Use An Existing Installed Executable

If `volicord` already exists on `PATH` and you only want to prepare or inspect
the installation profile before connecting a repository, run:

```sh
volicord setup
volicord doctor
```

Setup uses the same installation-profile contract whether the executable came
from a release install, a development source build, or another installed
command directory. Use `volicord setup --mcp-command PATH` only when generated
host configuration should start MCP through a different `volicord` command
path.
If setup reports `action_required`, complete the named local action before
starting new terminals or agent hosts. Ordinary `volicord init` and
`volicord connect` commands use the saved installation profile.

## Development Source Build

Source builds are for implementers and local development, not the primary user
install path. From the Volicord source repository:

```sh
cargo build --workspace --bins
./target/debug/volicord --version
./target/debug/volicord setup
```

This builds and runs the local development executable at
`./target/debug/volicord`. Rust toolchain requirements for this path are listed
in [System Requirements](../reference/system-requirements.md#toolchain-requirements).

## Docker Image

Docker support is for local container layouts and localhost MCP access. Build
the image from the Volicord source repository:

```sh
docker build -t volicord:local .
```

Use a Runtime Home volume and mount the Product Repository at the same container
path whenever you run setup, init, project, connection, and serve commands.
Project registrations store repository roots, so a Runtime Home prepared for
one path layout should not be reused with a different container workspace path.

For example, prepare or inspect the Docker Runtime Home with the same mounts:

```sh
docker run --rm -it \
  -v volicord-home:/var/lib/volicord \
  -v "$PWD:/workspace" \
  volicord:local setup
```

After the Runtime Home contains the project registration and Agent Connection
you want to serve, for example from a matching `volicord init` or
`volicord connect` run with the same mounts, start the local HTTP MCP endpoint
with an operator-provided token:

```sh
VOLICORD_HTTP_TOKEN="$(openssl rand -hex 32)"
docker run --rm \
  -p 127.0.0.1:8765:8765 \
  -v volicord-home:/var/lib/volicord \
  -v "$PWD:/workspace" \
  volicord:local serve --transport streamable-http \
    --listen 0.0.0.0:8765 \
    --allow-nonlocal-listen \
    --token "$VOLICORD_HTTP_TOKEN" \
    --project /workspace
```

The container listens on `0.0.0.0` only inside Docker so Docker can publish the
port. The host publish address remains `127.0.0.1`, and Volicord still requires
`--allow-nonlocal-listen` plus bearer authentication. Do not store
`VOLICORD_HTTP_TOKEN` in repository files.

## What Setup Does Not Do

Setup does not register a Product Repository and does not install host
configuration. Project registration happens when you run `volicord project use`
or a command such as `volicord init --host HOST --repo PATH` or
`volicord connect` from inside a Git repository.

Project naming and internal identity behavior are owned by the
[Administrative CLI Reference](../reference/admin-cli.md#project-commands).
Internal identities are stored by Volicord and are not first-time setup inputs.

## Next Step

Connect a host to the Product Repository:

```sh
volicord init --host codex --repo /path/to/your-product-repo
```

`/path/to/your-product-repo` is an example path for the Product Repository where
you want the agent to work.

For the full first-run path, continue with the [Quickstart](quickstart.md). For
host-specific details, see [Agent Host Setup](../guides/agent-host-setup.md).
