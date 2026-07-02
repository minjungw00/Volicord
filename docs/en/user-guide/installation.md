# Installation

This tutorial prepares the local `volicord` executable. The ordinary first-run
path records the installation profile while running
`volicord init --host HOST --repo PATH --profile record` in the
[Quickstart](quickstart.md). Use `volicord doctor` when you need to inspect the
saved installation profile.

Exact command behavior belongs to
[Administrative CLI Reference](../reference/admin-cli.md). Runtime location and
repository separation belong to [Runtime Boundaries](../reference/runtime-boundaries.md).

## Prerequisites

- A supported release-binary environment from
  [System Requirements](../reference/system-requirements.md), or Docker when
  using the Docker path below.
- A POSIX-style shell with `curl` or `wget`, `tar`, and a writable install
  directory for Linux, WSL2, or macOS; or PowerShell for native Windows.
- A Git repository to use as the Product Repository when you are ready to
  connect a host.

## Install A Release Binary

The primary user path is a release binary. The POSIX install script detects
Linux, WSL2, or macOS, selects the matching release tarball, verifies the
matching `.sha256` file when it can download one, and installs only the
`volicord` executable. The native Windows PowerShell install script selects the
`x86_64-pc-windows-msvc` zip archive, verifies the matching `.sha256` file when
it can download one, and installs only `volicord.exe`. Neither script edits
shell startup files implicitly.

For Linux, WSL2, or macOS, download or copy `scripts/install.sh` from the same
repository that publishes the Volicord release assets, then run it with the
release repository named explicitly:

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

For native Windows x86_64, download or copy `scripts/install.ps1` from the same
repository that publishes the Volicord release assets, then run it in
PowerShell:

```powershell
.\scripts\install.ps1 -Repo OWNER/REPO
```

To install a specific tag:

```powershell
.\scripts\install.ps1 -Repo OWNER/REPO -Version v0.1.0
```

For a non-GitHub release mirror:

```powershell
.\scripts\install.ps1 -ReleaseBaseUrl https://example.invalid/releases/v0.1.0
```

The default native Windows install directory is
`%LOCALAPPDATA%\Volicord\bin`. Use `-InstallDir` to choose a different
user-local directory:

```powershell
.\scripts\install.ps1 -Repo OWNER/REPO -InstallDir "$env:LOCALAPPDATA\Volicord\bin"
```

The Windows installer prints a current-session `PATH` command when the install
directory is not already on `PATH`. To append the install directory to the
user-level `PATH`, rerun with `-UpdateUserPath`:

```powershell
.\scripts\install.ps1 -Repo OWNER/REPO -UpdateUserPath
```

Each script fails before downloading on unsupported operating systems or CPU
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
`volicord init --host HOST --repo PATH --profile record` in the
[Quickstart](quickstart.md). `volicord init` can initialize the Runtime Home and
installation profile while it connects the selected Product Repository, writes
project-scoped MCP configuration, and records integration status.
Detective setup has the verified host-hook and session watcher requirements
described in the
[Administrative CLI Reference](../reference/admin-cli.md#agent-host-setup-and-init).
On native Windows, use `--profile record`; `--profile detective` fails with an
unsupported-platform diagnostic until Windows host hooks and watcher behavior
are implemented and tested.

`volicord init` creates or reuses the selected `Volicord Runtime Home` and saves
the installation profile while connecting a repository. It discovers the running
`volicord` executable, stores the MCP launch command, and checks whether the
selected command is available on `PATH` for future terminals and agent hosts.
Exact Runtime Home selection, MCP launch command behavior, and output behavior
belong to [Administrative CLI Reference](../reference/admin-cli.md#runtime-home-selection).
Its status answers whether setup still needs a named user or host action, so
`action_required` can appear even after durable local state has been saved.

Ensure the installed `volicord` binary is available on `PATH` before running
host setup. Shell startup file changes are never implicit. If you update `PATH`
through your shell startup files, open a new shell or restart or reload existing
agent host processes before expecting them to see the command.

For automation or deterministic local layouts, use explicit init options:

| Option | When to use it |
|---|---|
| `--mcp-command PATH` | Store a specific `volicord` command for generated MCP launch entries when init should not use the running executable. |
| `--home PATH` | Select a non-default `Volicord Runtime Home`. |

After completing any prompt or action-required command-availability step, check
installation-profile health:

```sh
volicord doctor
```

`doctor` reports installation-profile health, not primary `init` progress. It
reports `complete` when the saved profile is usable, even if it also reports
command-availability warnings or recommended `PATH` and command-link actions
for future shells or agent hosts. `action_required` names a blocking local
repair action, such as fixing an executable path.

## Use An Existing Installed Executable

If `volicord` already exists on `PATH`, you can go straight to the
[Quickstart](quickstart.md). Run doctor when you want to inspect the
installation profile:

```sh
volicord doctor
```

Init uses the same installation-profile contract whether the executable came
from a release install, a development source build, or another installed command
directory. Use `volicord init --mcp-command PATH ...` only when generated host
configuration should start MCP through a different `volicord` command path. If
init reports `action_required`, complete the named local or host action before
starting new terminals or agent hosts. Ordinary `volicord init` and `volicord
connection add` commands use the saved installation profile.

## Development Source Build

Source builds are for implementers and local development, not the primary user
install path. From the Volicord source repository:

```sh
cargo build --workspace --bins
./target/debug/volicord --version
./target/debug/volicord init --host codex --repo /path/to/your-product-repo --profile record
```

This builds and runs the local development executable at
`./target/debug/volicord`. For a host to use the development executable, make
the selected `volicord` command available to the host process or use an
installed release binary for normal host setup. Rust toolchain requirements for
this path are listed in
[System Requirements](../reference/system-requirements.md#toolchain-requirements).

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

For example, inspect or repair the Docker installation profile with the same
mounts:

```sh
docker run --rm -it \
  -v volicord-home:/var/lib/volicord \
  -v "$PWD:/workspace" \
  volicord:local setup
```

For record-profile setup in Docker, run
`volicord init --host HOST --repo /workspace --profile record` with the same
mounts. Detective Docker setup has the same verified host-hook and session
watcher requirements as non-container setup. After the Runtime Home contains
the project registration and Agent Connection you want to serve, for example
from that matching `volicord init` run or a lower-level `volicord connection add` run,
start the local HTTP MCP endpoint with an operator-provided token:

```sh
VOLICORD_HTTP_TOKEN="$(openssl rand -hex 32)"
docker run --rm \
  -p 127.0.0.1:8765:8765 \
  -v volicord-home:/var/lib/volicord \
  -v "$PWD:/workspace" \
  volicord:local serve --transport local-http \
    --container-listen 0.0.0.0:8765 \
    --token "$VOLICORD_HTTP_TOKEN" \
    --project /workspace
```

The `-p 127.0.0.1:8765:8765` mapping publishes the container port only on the
host loopback interface. `--container-listen 0.0.0.0:8765` is for this Docker
publishing shape; native local runs should use the default loopback `--listen`
behavior instead. Do not publish the container port on `0.0.0.0`, a public host
interface, or a remote host, and do not store `VOLICORD_HTTP_TOKEN` in
repository files. Treat this as local/Docker transport only, not a public
network API, SaaS endpoint, multi-user server, or security boundary.

## What Installation Does Not Do

Installing the binary alone does not register a Product Repository and does not
install host configuration. Project registration happens when you run
`volicord project use` or a command such as
`volicord init --host HOST --repo PATH --profile record` or
`volicord connection add` from inside a Git repository.

Project naming and internal identity behavior are owned by the
[Administrative CLI Reference](../reference/admin-cli.md#project-commands).
Internal identities are stored by Volicord and are not first-time setup inputs.

## Next Step

Connect a host to the Product Repository:

```sh
volicord init --host codex --repo /path/to/your-product-repo --profile record
```

`/path/to/your-product-repo` is an example path for the Product Repository where
you want the agent to work. Use `--profile detective` only when the selected host,
platform, and repository configuration satisfy the verified detective
prerequisites; native Windows uses `--profile record` because detective is not
supported there.

For the full first-run path, continue with the [Quickstart](quickstart.md). For
host-specific details, see [Agent Host Setup](../user-guide/agent-host-setup.md).
