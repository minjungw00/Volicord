# Installation

This tutorial prepares the local `volicord` executable. The ordinary first-run
path records the installation profile while running
`volicord init --shared --host HOST --repo PATH --profile record` in the
[Quickstart](quickstart.md). Use `volicord doctor` when you need to inspect the
saved installation profile.

Exact command behavior belongs to
[Administrative CLI Reference](../reference/admin-cli.md). Runtime location and
repository separation belong to [Runtime Boundaries](../reference/runtime-boundaries.md).

## Prerequisites

- Rust 1.85 or newer with Cargo for the source-build path, a complete published
  release-asset set for the release-installer path, or Docker for the local
  container path. See [System Requirements](../reference/system-requirements.md).
- For published release installation, a POSIX-style shell with `curl` or
  `wget`, `tar`, and a writable install directory on Linux, WSL2, or macOS; or
  PowerShell on native Windows.
- A Git repository to use as the Product Repository when you are ready to
  connect a host.

## Build From Source

The source build is the directly reproducible native path from this checkout:

```sh cli-example
cargo build --locked --release -p volicord-cli --bin volicord
./target/release/volicord --version
```

To install the built executable in a user command directory, replace
`$HOME/.local/bin` with another directory already on `PATH` if needed:

```sh cli-example
mkdir -p "$HOME/.local/bin"
install -m 0755 target/release/volicord "$HOME/.local/bin/volicord"
volicord --version
```

This path requires the Rust toolchain named in
[System Requirements](../reference/system-requirements.md#toolchain-requirements).
It does not depend on a published release host.

## Install Published Release Assets

Use this path only when a release distributor provides a matching installer,
target archive, and checksum set at a known base URL. The checked-in scripts
and packaging workflow define how those assets work; source-tree availability
does not establish that a particular repository, tag, or mirror has published
them. If you do not have a verified asset source, use the source-build path
above.

The POSIX install script detects
Linux, WSL2, or macOS, selects the matching release tarball, verifies the
matching `.sha256` file when it can download one, and installs only the
`volicord` executable. The native Windows PowerShell install script selects the
`x86_64-pc-windows-msvc` zip archive, verifies the matching `.sha256` file when
it can download one, and installs only `volicord.exe`. Neither script edits
shell startup files implicitly.

For Linux, WSL2, or macOS, download the `install.sh` release asset to a
temporary file, then run it with the release asset base URL named explicitly:

```sh
repo=OWNER/REPO
base="https://github.com/$repo/releases/latest/download"
tmp="$(mktemp "${TMPDIR:-/tmp}/install-volicord.XXXXXX")"
curl -fsSL "$base/install.sh" -o "$tmp"
VOLICORD_RELEASE_BASE_URL="$base" VOLICORD_REQUIRE_CHECKSUM=1 sh "$tmp"
```

`OWNER/REPO` is the GitHub repository that hosts the Volicord release assets.
By default the example downloads from that repository's latest release. To
install a specific tag, use the tag-specific release asset base URL:

```sh
repo=OWNER/REPO
version=v0.8.0
base="https://github.com/$repo/releases/download/$version"
tmp="$(mktemp "${TMPDIR:-/tmp}/install-volicord.XXXXXX")"
curl -fsSL "$base/install.sh" -o "$tmp"
VOLICORD_RELEASE_BASE_URL="$base" VOLICORD_REQUIRE_CHECKSUM=1 sh "$tmp"
```

For a non-GitHub release mirror, provide the directory that contains the
installer asset, target-named tarball, and checksum:

```sh
base="https://example.invalid/releases/v0.8.0"
tmp="$(mktemp "${TMPDIR:-/tmp}/install-volicord.XXXXXX")"
curl -fsSL "$base/install.sh" -o "$tmp"
VOLICORD_RELEASE_BASE_URL="$base" VOLICORD_REQUIRE_CHECKSUM=1 sh "$tmp"
```

The default install directory is `~/.local/bin`. Use `--install-dir PATH` for a
single command, or `VOLICORD_INSTALL_DIR` for environment-driven automation.
These examples reuse `$base` and `$tmp` from the selected release above:

```sh
VOLICORD_RELEASE_BASE_URL="$base" VOLICORD_REQUIRE_CHECKSUM=1 sh "$tmp" --install-dir /usr/local/bin
VOLICORD_RELEASE_BASE_URL="$base" VOLICORD_REQUIRE_CHECKSUM=1 VOLICORD_INSTALL_DIR=/usr/local/bin sh "$tmp"
```

To preview the detected target, release asset, checksum plan, install
directory, and binary name without downloading release archives or writing
installation files, add `--dry-run`. Use `--print-target` when automation only
needs the target identifier:

```sh
VOLICORD_RELEASE_BASE_URL="$base" VOLICORD_REQUIRE_CHECKSUM=1 sh "$tmp" --dry-run
sh "$tmp" --print-target
```

For native Windows x86_64, download the `install.ps1` release asset and run it
in PowerShell:

```powershell
$repo = "OWNER/REPO"
$base = "https://github.com/$repo/releases/latest/download"
$tmp = Join-Path $env:TEMP "install-volicord.ps1"
Invoke-WebRequest "$base/install.ps1" -OutFile $tmp
& $tmp -ReleaseBaseUrl $base -RequireChecksum
```

To install a specific tag:

```powershell
$repo = "OWNER/REPO"
$version = "v0.8.0"
$base = "https://github.com/$repo/releases/download/$version"
$tmp = Join-Path $env:TEMP "install-volicord.ps1"
Invoke-WebRequest "$base/install.ps1" -OutFile $tmp
& $tmp -ReleaseBaseUrl $base -RequireChecksum
```

For a non-GitHub release mirror:

```powershell
$base = "https://example.invalid/releases/v0.8.0"
$tmp = Join-Path $env:TEMP "install-volicord.ps1"
Invoke-WebRequest "$base/install.ps1" -OutFile $tmp
& $tmp -ReleaseBaseUrl $base -RequireChecksum
```

The default native Windows install directory is
`%LOCALAPPDATA%\Volicord\bin`. Use `-InstallDir` to choose a different
user-local directory. This example reuses `$base` and `$tmp` from the selected
release above:

```powershell
& $tmp -ReleaseBaseUrl $base -RequireChecksum -InstallDir "$env:LOCALAPPDATA\Volicord\bin"
```

To preview the detected target, release asset, checksum plan, install
directory, binary name, and requested `PATH` behavior without downloading
release archives, installing, or changing the user `PATH`, add `-DryRun`. Use
`-PrintTarget` when automation only needs the target identifier:

```powershell
& $tmp -ReleaseBaseUrl $base -RequireChecksum -DryRun
& $tmp -PrintTarget
```

The Windows installer prints a current-session `PATH` command when the install
directory is not already on `PATH`. To append the install directory to the
user-level `PATH`, rerun with `-UpdateUserPath`:

```powershell
& $tmp -ReleaseBaseUrl $base -RequireChecksum -UpdateUserPath
```

Each script fails before downloading on unsupported operating systems or CPU
architectures. If a checksum file is present but cannot be verified, the script
fails. If the checksum file is unavailable, the script warns; set
`VOLICORD_REQUIRE_CHECKSUM=1` when installation must fail instead.

No Homebrew tap, package-manager package, or external package registry is
claimed by this repository unless a matching repository artifact is added.

After installation, verify the installed command:

```sh cli-example
volicord --version
volicord --help
volicord mcp --help
volicord init --help
```

For the ordinary first repository connection, continue with
`volicord init --shared --host codex --repo PATH --profile record` in the
[Quickstart](quickstart.md). `volicord init` creates or reuses the selected
Runtime Home, connects the Product Repository, writes the managed Codex stdio
configuration, and records integration status. `action_required` can remain
until the named Codex trust, reload, or verification step is complete.

For a new Runtime Home, `init` builds and validates the Registry and
installation profile in same-parent staging, then publishes the whole
directory by an atomic no-replace rename. The staged singleton contains one
opaque publication ID. The successful publisher retains an invocation-specific
guard through synchronization and read-back. If the selected path already
exists, inspection is read-only. A manifest or schema mismatch preserves that
home; keep it for owner-approved recovery and rerun with a fresh explicit
`--home`. Use an owner-defined importer only when the current owners provide
one.

`init` acquires the selected canonical Runtime Home's OS-backed setup lease
before inspection and constructs its complete setup plan without writing any
target. A dry run holds the same lease while generating and reporting its
coherent plan. If another setup owns the lease, `init` reports a typed busy
result and asks you to wait for it to finish; do not delete coordination files.
Prepare then stages the exact Codex configuration and every repository hook,
wrapper, rule, policy, exclude, and managed guidance file beside its target,
and prepares the Store recovery boundary. Commit publishes or validates the
Runtime Home, applies Store mutations, atomically replaces repository files in
deterministic order, replaces Codex configuration last, and records the
integration revision. The lease remains held until success reporting and
cleanup or complete rollback. A failure restores already replaced files and
checkpointed Store bytes when they remain unchanged, removes owned staging
when safe, and reports `preserved`, `rolled_back`, or
`partially_rolled_back` precisely. Runtime Home removal additionally requires
the owned publication guard to revalidate the exact ID, manifest, paths,
schema, installation, and absence of managed-host consumption; concurrent
setup cannot enter Store mutation while that rollback authority is live, and
ownership mismatches stop removal.
Recursive removal effect and parent-directory durability are reported
separately. If the Runtime Home is absent but parent synchronization failed,
the report says it was removed with unconfirmed durability; it does not say
the publication remains. An incomplete or uncertain removal is reported
separately and is not retried against a recreated path. A recovery entry is retained
and named in the diagnostic if deleting it would discard a pre-existing file
after a later writer made restoration unsafe.
Runtime Home, Codex home, and Product Repository may be on different
filesystems: preparation is complete before commit and each file replacement
is atomic, but the whole multi-filesystem operation is not globally atomic.

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

```sh cli-example
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

```sh cli-example
volicord doctor
```

Init uses the same installation-profile contract whether the executable came
from a release install, a source build, or another installed command
directory. Use `volicord init --mcp-command PATH ...` only when generated host
configuration should start MCP through a different `volicord` command path. If
init reports `action_required`, complete the named local or host action before
starting new terminals or agent hosts. Ordinary `volicord init` and `volicord
connection add` commands use the saved installation profile.

## Docker Images

The root `Dockerfile` is the general-purpose source-building definition for
development and CI. `Dockerfile.release` has a separate production packaging
responsibility: the release workflow supplies the already validated
`x86_64-unknown-linux-gnu` executable as `volicord`, and the release Dockerfile
copies those exact bytes into the image without rebuilding Volicord. The
workflow requires the SHA-256 digest of `/usr/local/bin/volicord` inside the
image to equal the validated raw artifact digest.

For a local source build, use the general-purpose root Dockerfile. Mount a
disposable Runtime Home and the intended Product Repository, then run
administrative checks or the bound stdio process. The container image does not
add a separate public transport or change platform applicability.

```sh
docker build -t volicord:local .
docker run --rm -it \
  -v volicord-home:/var/lib/volicord \
  -v "$PWD:/workspace" \
  volicord:local doctor
```

Use the same mounts for `init`, connection verification, and managed stdio so
the process sees the same Runtime Home and Product Repository.

## What Installation Does Not Do

Installing the binary alone does not register a Product Repository and does not
install host configuration. Project registration happens when you run
`volicord project use` or a command such as
`volicord init --shared --host HOST --repo PATH --profile record` or
`volicord connection add` from inside a Git repository.

Project naming and internal identity behavior are owned by the
[Administrative CLI Reference](../reference/admin-cli.md#project-commands).
Internal identities are stored by Volicord and are not first-time setup inputs.

## Connect Codex

Connect a host to the Product Repository:

```sh cli-example
volicord init --shared --host codex --repo /path/to/your-product-repo --profile record
```

`/path/to/your-product-repo` is an example path for the Product Repository where
you want Codex to work. The first release uses the `record` profile on every
supported platform.

This command applies the canonical managed launcher, Connection, and current
Guard setup before it reports one hierarchical `IntegrationActivationPlan`. A
successful apply does not by itself prove that a reloaded managed host or the
current hooks have run. Default init output presents the plan in one
`Required next steps` section and keeps `Optional active diagnostics`
separate.

This shared setup requires the host launch environment to provide the same
nonempty, absolute `VOLICORD_HOME` selected by init. The repository-visible
configuration forwards that value and does not embed a machine-local Runtime
Home path.

If init reports `review_required_by_setup`, finish activation in the host:

1. restart or reload Codex in this repository;
2. review the current project hook definition in the Codex hook UI or with
   `/hooks`;
3. start a new conversation and ask
   `Run the Volicord integration verification.`;
4. after the agent finishes, read the current connection status.

The in-chat agent must use `volicord.list_projects`,
`volicord.begin_integration_verification`, the returned
`volicord.guard_probe`, and `volicord.get_integration_verification` in that
order. If the tools are not exposed, report managed MCP unavailable. Do not
substitute raw stdio, hand-authored Codex `_meta`, resources, resource
templates, or CLI preflight as proof. Obey terminal workflow states; do not use
shell sleep or poll loops or restart verification automatically in the same
turn. `volicord connection verify` remains optional active diagnostics and
does not replace host-owned hook review or managed in-chat evidence.

For the full first-run path, continue with the [Quickstart](quickstart.md). For
host-specific details, see [Agent Host Setup](../user-guide/agent-host-setup.md).
