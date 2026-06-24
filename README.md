# Volicord

**[English](README.md)** | [한국어](README.ko.md)

**AI moves. Judgment stays yours.**

A local work-authority system for AI-assisted product work.

Volicord helps a user and an agent keep the important parts of the work visible:
scope, user-owned judgment, evidence, verification criteria, final acceptance,
residual-risk acceptance, and close readiness.

This README is the first-user route for the current repository. It explains
what Volicord is, how the local executables and host setup fit together, how to
build and verify the executables, how to choose a supported host path, and how
to try a first Volicord-assisted interaction after setup.

Exact contracts remain in the maintained Reference documents linked throughout
this page.

## Contents

- [Overview](#overview)
- [Why Volicord Exists](#why-volicord-exists)
- [Concrete Scenario](#scenario)
- [How The Pieces Fit](#how-the-pieces-fit)
- [Terms For First Setup](#terms)
- [Current Capabilities And Boundaries](#support)
- [System And Shell Requirements](#system-requirements)
- [Example Values And Paths](#example-values)
- [Build And Verify Executables](#executable-installation)
- [Choose A Host Path](#host-selection)
- [Codex User-Scope Setup](#codex)
- [Claude Code Project-Scope Setup](#claude-code)
- [Generic Export](#generic-export)
- [Status And Verification](#verification)
- [First Use In Your Host](#first-use)
- [Data Ownership And Write Boundaries](#data-boundaries)
- [First-Install Troubleshooting](#troubleshooting)
- [Documentation Routes](#documentation-routes)

<a id="overview"></a>
## Overview

Volicord is for local product work where an AI agent helps you inspect, plan,
write, verify, and summarize changes. Its job is not to make every agent action
automatic. Its job is to keep the authority basis visible while the work moves.

In practical terms, Volicord helps an agent keep asking and answering questions
like these:

- What is in scope, and what is explicitly not in scope?
- Which decision belongs to the user?
- Which evidence supports this claim?
- Which checks were actually run?
- Is the user accepting the final result, or only approving the next write?
- Is a remaining risk named clearly enough for the user to accept or reject?
- What still blocks an honest close?

Volicord itself is not the local authority record. Core is the local authority
record for Volicord state. Volicord is the broader product/system around that
record, including local runtime components, host integration, documentation, and
workflows.

<a id="why-volicord-exists"></a>
## Why Volicord Exists

AI-assisted product work can move quickly enough that important distinctions get
blurred. A passing test can start sounding like final acceptance. A plausible
implementation detail can become an unstated product decision. A broad "looks
good" can be mistaken for approval of scope expansion, sensitive action,
residual risk, and close.

Volicord exists to slow down only the part that should not be blurred. The agent
can still move quickly, but scope, user judgment, evidence, verification
criteria, acceptance, residual risk, and close readiness stay separate.

<a id="scenario"></a>
## Concrete Scenario

Suppose you ask:

```text
Add email login, but keep password reset and account creation out of scope.
Make a plan first and do not write until I approve the first change.
```

A Volicord-assisted workflow should keep these facts distinct:

| Work item | What should stay visible |
|---|---|
| Scope | Email login is in scope; password reset and account creation are not. |
| User-owned judgment | You still decide product behavior, risk tradeoffs, and final acceptance. |
| Evidence | A diff, test output, logs, or source citation supports a specific claim only. |
| Verification criteria | The visible checks for the requested work are not the same thing as evidence or acceptance. |
| Write approval | Permission for a named write attempt is not approval for every later write or sensitive action. |
| Final acceptance | Passing checks can inform acceptance, but they do not replace your acceptance. |
| Residual risk | A named remaining risk must stay visible if you are being asked to accept it. |
| Close readiness | The task should close only when the close basis is honest for the current state. |

The point is ordinary: the agent should not turn speed into substitution.

<a id="how-the-pieces-fit"></a>
## How The Pieces Fit

The current local setup has four separate locations or actors:

```text
AI host
  Codex, Claude Code, or a user-managed MCP host
        |
        | starts a local child process
        v
volicord-mcp --integration <integration_id>
        |
        | uses the selected Volicord Runtime Home and allowed Product Repository
        v
Volicord Runtime Home                    Product Repository
  /Users/alex/.volicord                    /work/acme-api

volicord
  administrative CLI used for install, status, verification, and guidance
```

`volicord-mcp` is a local stdio child process started by the host. It is not a
TCP, HTTP, socket, or other network listener. One `volicord-mcp` process is bound
to one Agent Integration Profile by `--integration <integration_id>`. Project
selection happens per public Volicord tool call.

`volicord` is the administrative CLI. It is used to build setup state, install or
export host configuration, inspect status, and refresh verification. It is not a
long-running server and is not a public Volicord API method surface.

<a id="terms"></a>
## Terms For First Setup

| Term | Beginner meaning | More detail |
|---|---|---|
| Volicord | The local work-authority product/system for AI-assisted product work. | [Getting Started Overview](docs/en/getting-started/overview.md) |
| Core | The local authority record for Volicord state. | [Core Model](docs/en/reference/core-model.md) |
| Volicord implementation | The implementation set maintained by this repository, including Core, storage, types, the `volicord` CLI, `volicord-mcp`, tests, documentation, and validation tooling. | [Runtime Boundaries](docs/en/reference/runtime-boundaries.md) |
| `volicord` | The administrative CLI executable from package `volicord-cli`. | [Administrative CLI](docs/en/reference/admin-cli.md) |
| `volicord-mcp` | The local MCP stdio executable started by an AI host. | [MCP Transport](docs/en/reference/mcp-transport.md) |
| `Volicord Runtime Home` | Local runtime storage for Volicord records and operational data. | [Runtime Boundaries](docs/en/reference/runtime-boundaries.md) |
| `Product Repository` | Your project workspace and product-file boundary. | [Runtime Boundaries](docs/en/reference/runtime-boundaries.md) |
| Agent host | Codex, Claude Code, or a user-managed MCP host that can start `volicord-mcp`. | [Agent Integration](docs/en/reference/agent-integration.md) |
| Agent Integration Profile | The durable Volicord integration record selected by `integration_id`. | [Agent Integration](docs/en/reference/agent-integration.md) |
| Host Installation | Volicord-managed inventory for host configuration and last verification state. | [Agent Integration](docs/en/reference/agent-integration.md) |

<a id="support"></a>
## Current Capabilities And Boundaries

This repository currently contains:

- a Cargo Rust workspace
- the `volicord` administrative executable from `volicord-cli`
- the local stdio `volicord-mcp` executable from `volicord-mcp`
- maintained English and Korean documentation under `docs/`
- direct setup support for Codex and Claude Code
- generic MCP configuration export for user-managed hosts
- implementation, integration, and conformance test paths
- documentation metadata in `docs/doc-index.yaml`

Current first-setup support is intentionally local:

| Area | Current baseline |
|---|---|
| Executable source | Build from this source checkout, or use an already available Volicord installation directory containing both `volicord` and `volicord-mcp`. |
| Direct host setup | Codex and Claude Code have supported direct `volicord agent install` paths. |
| Generic MCP hosts | Generic export renders configuration for a user-managed host. Volicord does not directly install into that host or prove that the host loaded it. |
| MCP transport | The baseline process is local stdio: the host starts `volicord-mcp` as a child process. |
| Package managers | No package-manager installation path is documented by current owner docs. |
| Named operating systems | No named OS family is declared as generally supported by this checkout. Maintained examples use POSIX-style shell syntax. |
| Remote hosts and containers | Not documented as supported baseline setup paths by current owner docs. |

Codex and Claude Code setup can be successful as administrative work while
still requiring a host-owned action such as project trust, project MCP approval,
reload, restart, OAuth, or executable availability. Generic export normally
remains `action_required` because Volicord cannot observe whether the external
host loaded the exported configuration.

<a id="system-requirements"></a>
## System And Shell Requirements

Check these before running installation commands:

| Requirement | Current rule |
|---|---|
| Rust toolchain for source builds | Rust 1.85 or newer with Cargo. The workspace root `Cargo.toml` declares `rust-version = "1.85"`. |
| Shell examples | Maintained commands use POSIX-style syntax such as `export`, `$(pwd)`, quoted variables, inline environment assignment, colon-separated `PATH`, and `test -x`. |
| Executable layout | One selected directory must contain both runnable executables: `volicord` and `volicord-mcp`. |
| Runtime Home | Choose a local `Volicord Runtime Home` the selected user and future host process can read and write. |
| Product Repository | Choose an existing local `Product Repository` directory. It must be separate from the Runtime Home. |
| Host availability | Use Codex or Claude Code for direct setup, or a user-managed MCP host for generic export. No fixed minimum host versions are documented. |

If your shell cannot run the POSIX-style examples, translate them carefully and
verify each translated command before continuing. Do not treat Rust portability
as a claim that this repository supports a named OS, PowerShell, `cmd.exe`, a
container image, or a remote host.

For the complete requirement contract, use
[System Requirements](docs/en/reference/system-requirements.md).

<a id="example-values"></a>
## Example Values And Paths

The commands below use one consistent example set. Replace every example path
and ID with your real values.

| Example value | Meaning |
|---|---|
| `VOLICORD_BIN="/absolute/path/to/selected/bin"` | Shell convenience variable for one directory containing both executables. |
| `"$VOLICORD_BIN/volicord"` | The administrative CLI invocation. |
| `"$VOLICORD_BIN/volicord-mcp"` | Absolute `volicord-mcp` path for user/local-scope host configuration and generic export. |
| `/Users/alex/.volicord` | Example `Volicord Runtime Home`. |
| `/work/acme-api` | Example `Product Repository`. |
| `acme-api` | Example `project_id`. |
| `int-codex-team` | Example Codex `integration_id`. |
| `int-claude-acme` | Example Claude Code `integration_id`. |
| `int-generic-acme` | Example generic export `integration_id`. |

`VOLICORD_BIN` is only a shell variable used by these examples. Volicord does not
read it as configuration. Set it again in each new shell, or use absolute paths
directly.

`VOLICORD_HOME` is different. It is a real Runtime Home selection input for
administrative commands and for later `volicord-mcp` process startup when the
default home-derived Runtime Home is not the one you intend.

The `Volicord Runtime Home` and `Product Repository` must be separate resolved
paths. Do not place one inside the other.

<a id="executable-installation"></a>
## Build And Verify Executables

Use Path A when you are building from this repository. Use Path B when you
already have a Volicord installation directory containing both
executables.

### Path A: Build From Source

Working directory: Volicord source repository root.

Run toolchain checks first:

```sh
cargo --version
rustc --version
```

If either command is unavailable, or if the selected Rust compiler is older than
1.85, fix the toolchain before building.

For a debug build:

```sh
cargo build -p volicord-cli -p volicord-mcp
export VOLICORD_BIN="$(pwd)/target/debug"
```

For a release build:

```sh
cargo build --release -p volicord-cli -p volicord-mcp
export VOLICORD_BIN="$(pwd)/target/release"
```

### Path B: Select Installed Executables

Use this path when the executables are already available outside the source
checkout:

```sh
export VOLICORD_BIN="/absolute/path/to/installed/bin"
```

Replace the example path with the absolute directory that contains both
`volicord` and `volicord-mcp`.

### Verify The Selected Directory

From the same shell where `VOLICORD_BIN` is set:

```sh
test -x "$VOLICORD_BIN/volicord"
test -x "$VOLICORD_BIN/volicord-mcp"

"$VOLICORD_BIN/volicord" --version
"$VOLICORD_BIN/volicord" agent --help
"$VOLICORD_BIN/volicord-mcp" --version
"$VOLICORD_BIN/volicord-mcp" --help
```

The version commands should print `volicord <version>` and
`volicord-mcp <version>`. The help commands should show the `volicord agent`
command family and the integration-bound `volicord-mcp --integration
<integration_id>` process usage.

Continue only after both executables run from the same selected directory. This
proves the executables are ready for host setup. It does not create a Runtime
Home, register a Product Repository, or install host configuration.

For the focused tutorial, see
[Installation](docs/en/getting-started/installation.md).

<a id="host-selection"></a>
## Choose A Host Path

Choose one path for the first setup. You can add other paths later.

| Path | Use when | What Volicord can verify |
|---|---|---|
| Codex `user` scope | One personal Codex MCP entry should serve this repository now and may later serve more explicitly allowed repositories. | Direct setup can install Codex user configuration and run administrative verification. |
| Claude Code `project` scope | The Product Repository should carry a team-shared Claude Code `.mcp.json` entry. | Direct setup can write the project file when authorized, then report host-owned approval or completion state. |
| Generic `export` scope | You use another MCP host and will manage its configuration yourself. | Volicord can render configuration. It cannot prove the external host loaded it. |

The examples below intentionally show one Codex path, one Claude Code path, and
one generic export path. More host and scope combinations are documented in
[Agent Host Setup](docs/en/guides/agent-host-setup.md).

<a id="codex"></a>
## Codex User-Scope Setup

Use this path when one personal Codex configuration should load the same
Volicord integration across Codex projects.

Before running it:

- `VOLICORD_BIN` names the verified executable directory.
- Codex can read its user `config.toml` through `CODEX_HOME` or `HOME`.
- The `codex` executable is available on the administrative command `PATH` for
  the compatibility check.
- `/Users/alex/.volicord` and `/work/acme-api` are separate paths.
- This first install introduces `/work/acme-api` as a new project registration,
  so it provides both `--project-id acme-api` and `--repo-root /work/acme-api`.
  The project ID is a stable logical identifier you choose.
- `--integration-id`, `--runtime-home`, and the absolute `--mcp-command` are
  optional in general, but this example pins them so follow-up commands and
  generated host configuration use predictable values. For full argument rules,
  see [Administrative CLI](docs/en/reference/admin-cli.md#volicord-agent-install).

Install:

```sh
"$VOLICORD_BIN/volicord" agent install \
  --host codex \
  --scope user \
  --integration-id int-codex-team \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --runtime-home /Users/alex/.volicord \
  --mcp-command "$VOLICORD_BIN/volicord-mcp"
```

Because `--default-project-id` and `--server-name` are omitted, the new
integration uses the selected project as its default and the CLI derives a
stable host MCP server name from `integration_id`, such as
`volicord-int-codex-team`.

Expected first result includes:

```text
status: complete
integration_id: int-codex-team
host_kind: codex
host_scope: user
server_name: volicord-int-codex-team
verification: complete
```

The setup may write Runtime Home records under `/Users/alex/.volicord` and a
Codex user MCP entry. It does not write `/work/acme-api` unless optional
repository guidance is selected separately and explicitly authorized.

Independent completion check:

```sh
"$VOLICORD_BIN/volicord" agent verify \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.volicord
```

This path is complete when verification reports `status: complete`. If it
reports `action_required`, complete the named host-owned action and rerun
verification.

<a id="claude-code"></a>
## Claude Code Project-Scope Setup

Use this path when `/work/acme-api` should carry a team-shared Claude Code
`.mcp.json` entry.

Before running it:

- `VOLICORD_BIN` names the verified executable directory.
- `/Users/alex/.volicord` and `/work/acme-api` are separate paths.
- `volicord-mcp` will be available on the `PATH` used by the future Claude Code
  process.
- If Claude Code would not otherwise use `/Users/alex/.volicord`, the future
  Claude Code launch environment must provide `VOLICORD_HOME=/Users/alex/.volicord`.
- You intentionally allow the administrative command to write
  `/work/acme-api/.mcp.json`.
- `--integration-id` is optional but pinned so the verify command and generated
  server name are predictable. Project scope omits `--mcp-command` because the
  default is the portable `volicord-mcp` command.
- `--dry-run` is an optional zero-write preview and `--output json` only changes
  preview formatting. The real apply command keeps `--allow-repository-write`
  because it authorizes the intended project-file write.

Optional dry-run:

```sh
VOLICORD_HOME=/Users/alex/.volicord \
PATH="$VOLICORD_BIN:$PATH" \
"$VOLICORD_BIN/volicord" agent install \
  --host claude-code \
  --scope project \
  --integration-id int-claude-acme \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --dry-run \
  --output json
```

Apply setup:

```sh
VOLICORD_HOME=/Users/alex/.volicord \
PATH="$VOLICORD_BIN:$PATH" \
"$VOLICORD_BIN/volicord" agent install \
  --host claude-code \
  --scope project \
  --integration-id int-claude-acme \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --allow-repository-write
```

Expected first result before host approval may include:

```text
status: action_required
integration_id: int-claude-acme
host_kind: claude_code
host_scope: project
server_name: volicord-int-claude-acme
verification: action_required
```

`action_required` is a successful administrative result. It means a named
host-owned action remains, such as Claude Code project MCP approval, reload, or
restart.

The generated `.mcp.json` entry intentionally stores the portable command
`volicord-mcp` and does not embed a personal `VOLICORD_HOME`. The inline
`VOLICORD_HOME` and `PATH` values above apply only to the administrative command.
When Claude Code later starts the server, Claude Code's own environment must be
able to find `volicord-mcp` and select the intended Runtime Home if the default
would differ.

After completing the host-owned approval or reload step, verify:

```sh
VOLICORD_HOME=/Users/alex/.volicord \
PATH="$VOLICORD_BIN:$PATH" \
"$VOLICORD_BIN/volicord" agent verify \
  --integration-id int-claude-acme
```

This path is complete when verification reports `status: complete` and the
selected Host Installation reports `final_status: complete`.

<a id="generic-export"></a>
## Generic Export

Use generic export only for a host that Volicord does not install directly. This
path renders configuration for you to apply in the external host's own setup
flow.

The required choices are the host and scope. This example also supplies both
`--project-id acme-api` and `--repo-root /work/acme-api` to make project
selection explicit; the full omission rules stay in
[Administrative CLI](docs/en/reference/admin-cli.md#volicord-agent-install).
The optional `--integration-id`, `--runtime-home`, explicit `--mcp-command`, and
`--export-dir` are kept so the exported server name, Runtime Home environment,
command path, and destination are reproducible.

```sh
"$VOLICORD_BIN/volicord" agent install \
  --host generic \
  --scope export \
  --integration-id int-generic-acme \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --runtime-home /Users/alex/.volicord \
  --mcp-command "$VOLICORD_BIN/volicord-mcp" \
  --export-dir /tmp/volicord-mcp-export
```

The export contains a host-neutral MCP server object shaped like:

```json
{
  "mcpServers": {
    "volicord-int-generic-acme": {
      "command": "/absolute/path/to/selected/bin/volicord-mcp",
      "args": ["--integration", "int-generic-acme"],
      "env": {
        "VOLICORD_HOME": "/Users/alex/.volicord"
      }
    }
  }
}
```

Apply that configuration through the external host's own instructions. Volicord
does not directly install it, reload the host, or confirm that the host loaded
it. A generic export may remain `action_required` for that reason.

<a id="verification"></a>
## Status And Verification

These checks have different meanings:

| Command | What it tells you | What it does not prove |
|---|---|---|
| `volicord agent status` | Registry state, allowed projects, Host Installation inventory, last verification status, and guidance status. | It does not prove the host loaded or exposed the MCP server. |
| `volicord agent verify` | Administrative verification for selected Host Installations, including startup checks and host-specific gates where observable. | It does not make host-owned trust or approval decisions for you. |
| `volicord-mcp --check --integration <integration_id>` | Startup validation for the local `volicord-mcp` process and selected integration. | It is not complete host integration and does not prove Codex, Claude Code, or a generic host loaded it. |

Useful checks:

```sh
"$VOLICORD_BIN/volicord" agent status \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.volicord

"$VOLICORD_BIN/volicord" agent verify \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.volicord

VOLICORD_HOME=/Users/alex/.volicord \
"$VOLICORD_BIN/volicord-mcp" --check --integration int-codex-team
```

Setup result states at onboarding level:

| State | Meaning |
|---|---|
| `complete` | Administrative setup, relevant host-owned gates, MCP initialization, and tool discovery all succeeded for the selected installation. |
| `action_required` | The command succeeded, but a named host-owned action remains. Complete that action, then run `volicord agent verify`. |
| `partial_failure` | Some durable administrative action may have succeeded before a later step failed. Read `effects` and `residual_effects` before retrying. |
| `failed` | The requested setup or verification did not establish usable durable integration state or host configuration. Fix the reported error before retrying. |

Successful MCP startup does not prove that a host loaded or will consistently
use Volicord. Tool discovery also does not guarantee that an AI model will choose
Volicord for every request.

Exact result-state behavior belongs to
[Administrative CLI](docs/en/reference/admin-cli.md).

<a id="first-use"></a>
## First Use In Your Host

After the chosen host path is installed and verified, use the host normally. You
do not need to invoke MCP methods directly.

Good first requests are ordinary-language requests that ask the agent to keep
boundaries visible:

```text
Help me make this plan concrete before implementation. Show the current scope,
non-goals, unknowns, and the first safe action.
```

```text
What is known, what is still blocked, and what can safely happen next?
```

```text
Show what changed, what was checked, what residual risk is visible, and what
still blocks close.
```

If more than one project is allowed for the integration and the agent is unsure
which one to use, it should list allowed projects and retry with an explicit
project selection rather than guessing from folder names, host labels, or chat
memory.

For the user workflow, see [User Guide](docs/en/guides/user-workflow.md).

<a id="data-boundaries"></a>
## Data Ownership And Write Boundaries

Keep these locations separate:

| Location | Owner | What belongs there | What setup may write |
|---|---|---|---|
| Volicord source repository or installation | Volicord implementation maintainer or installer | Source checkout, installed executables, build output, documentation, tests, or required executable resources. | Source builds write Cargo output under `target/`. |
| `Volicord Runtime Home` | Local Volicord operator | Volicord registry, integration state, project state, runtime records, and runtime data as storage owners define them. | Agent setup creates or reuses Volicord records there. |
| `Product Repository` | Product project owner | Product files and explicitly selected project-scoped integration files. | Only explicitly selected and authorized integration files or guidance, such as `.codex/config.toml`, `.mcp.json`, `AGENTS.md` guidance, or `.claude/rules/` guidance. |
| Codex or Claude Code configuration | Host operator | Host-owned settings that start `volicord-mcp --integration <integration_id>`. | Direct setup may write managed host configuration where the selected host and scope require it. |
| Generic export target | User-managed host operator | Exported MCP configuration for another host. | The export file or directory you select, such as `/tmp/volicord-mcp-export`. |

Volicord runtime databases, runtime records, generated records, logs,
projections, QA results, acceptance records, close-readiness state, and
residual-risk records are not stored in the `Product Repository`.

Repository writes during setup are limited to explicitly selected integration
configuration or guidance, and noninteractive project-scoped writes require
`--allow-repository-write`. Those files are host configuration or advisory
context. They are not Core authority, evidence, final acceptance, close
readiness, residual-risk acceptance, or a security guarantee.

For exact location rules, use
[Runtime Boundaries](docs/en/reference/runtime-boundaries.md).

<a id="troubleshooting"></a>
## First-Install Troubleshooting

| Symptom | First safe response | Route |
|---|---|---|
| `cargo` or `rustc` is unavailable, or Rust is older than 1.85. | Select Rust 1.85+ with Cargo, then rerun the toolchain checks. | [System Requirements](docs/en/reference/system-requirements.md) |
| `target/debug` or `target/release` does not contain both executables. | Confirm which build command succeeded, select the matching output directory, and rerun every executable check. | [Installation](docs/en/getting-started/installation.md) |
| A help or version command fails. | Select the directory that actually contains runnable `volicord` and `volicord-mcp`. | [Agent Host Troubleshooting](docs/en/guides/agent-host-troubleshooting.md) |
| Runtime Home and Product Repository overlap. | Choose separate paths with no ancestor-descendant relationship. Do not repair by editing SQLite. | [Runtime Boundaries](docs/en/reference/runtime-boundaries.md) |
| Project-scoped setup refuses to write `.mcp.json` or `.codex/config.toml`. | Rerun only after deciding the repository write is intended and include `--allow-repository-write`. | [Administrative CLI](docs/en/reference/admin-cli.md) |
| Result is `action_required`. | Complete the named host-owned trust, approval, reload, restart, OAuth, or executable-availability action, then run `volicord agent verify`. | [Agent Host Troubleshooting](docs/en/guides/agent-host-troubleshooting.md) |
| Result is `partial_failure`. | Read `effects` and `residual_effects`; fix only the named issue before retrying. | [Agent Host Troubleshooting](docs/en/guides/agent-host-troubleshooting.md) |
| Result is `failed`. | Fix the reported error, run a dry-run before another write when available, then retry install or verify. | [Agent Host Troubleshooting](docs/en/guides/agent-host-troubleshooting.md) |
| A project-scoped host cannot find `volicord-mcp`. | Keep the project file portable and fix the future host process `PATH`. | [Agent Host Troubleshooting](docs/en/guides/agent-host-troubleshooting.md) |
| Generic export stays `action_required`. | Apply the exported configuration in the external host yourself; Volicord cannot observe that host's load state. | [Agent Host Setup](docs/en/guides/agent-host-setup.md) |

Do not delete the Runtime Home, Product Repository, artifact storage, Core
records, unrelated host entries, or user-edited guidance as a first response to
a setup error. Prefer status, dry-run, and verification commands that name the
specific problem.

<a id="documentation-routes"></a>
## Documentation Routes

| Need | Route |
|---|---|
| English documentation home | [docs/en/README.md](docs/en/README.md) |
| Korean documentation home | [docs/ko/README.md](docs/ko/README.md) |
| Documentation directory guide | [docs/README.md](docs/README.md) |
| First product orientation | [Getting Started Overview](docs/en/getting-started/overview.md) |
| Build and executable verification | [Installation](docs/en/getting-started/installation.md) |
| Shortest first host path | [Quickstart](docs/en/getting-started/quickstart.md) |
| Full host setup and generic export | [Agent Host Setup](docs/en/guides/agent-host-setup.md) |
| First-install recovery | [Agent Host Troubleshooting](docs/en/guides/agent-host-troubleshooting.md) |
| User workflow | [User Guide](docs/en/guides/user-workflow.md) |
| Multiple repositories | [Multi-Repository Agent Setup](docs/en/guides/multi-repository-agent-setup.md) |
| Agent workflow | [Agent Guide](docs/en/guides/agent-workflow.md) |
| Source-code learning | [Developer Documentation](docs/en/development/README.md) |
| Reference contracts | [Reference Index](docs/en/reference/README.md) |
| Administrative CLI contract | [Administrative CLI](docs/en/reference/admin-cli.md) |
| MCP process contract | [MCP Transport](docs/en/reference/mcp-transport.md) |
| Runtime location boundaries | [Runtime Boundaries](docs/en/reference/runtime-boundaries.md) |
| Public API method list | [API Methods](docs/en/reference/api/methods.md) |

`docs/doc-index.yaml` is maintenance metadata for owner routing, maintained
paths, applicability, dependencies, and bilingual maintenance. It is not
ordinary runtime configuration and is not the first document a new user needs
to read.
