# Harness

Harness is the local work-authority product/system for AI-assisted product work. It helps users and agents keep scope, user-owned judgment, evidence, verification criteria, final acceptance, residual-risk acceptance, and close readiness visible while product work moves through an AI-assisted workflow.

Harness exists for local product work where chat momentum can otherwise blur important boundaries: what is in scope, what the user actually decided, what evidence supports a claim, what was checked, and what still carries risk.

## Current Repository Surface

This repository currently contains:

- maintained English and Korean documentation
- a Cargo Rust workspace
- the `harness` administrative/bootstrap executable from package `harness-cli`
- the `harness-mcp` local MCP stdio executable from package `harness-mcp`
- direct host integration support for Codex, Claude Code, and generic export
- implementation, integration, and conformance test paths

The baseline local MCP process is stdio-based. An MCP host starts `harness-mcp` as a local child process with `--integration <integration_id>`; it is not a TCP, HTTP, socket, or other network listener.

## Prerequisites

For the source build and local setup path, you need:

- Rust 1.85 or newer with Cargo; Rust 1.85 is the minimum compiler version verified for the current workspace
- a local checkout of this repository, or another Harness Server installation that provides `harness` and `harness-mcp`
- a local `Product Repository` directory to allow for the integration
- a `Harness Runtime Home` that is separate from the `Product Repository`
- Codex, Claude Code, or another MCP host when you are ready to connect the MCP server

## Setup Shape

Agent integration setup has three separate stages:

1. Prepare Harness Server by building or locating `harness` and `harness-mcp`.
2. Run `harness agent install` for a real host: Codex, Claude Code, or generic export.
3. Complete any host-owned trust, approval, reload, or startup action reported as `action_required`.

The locations stay distinct:

| Location | Owner | Typical contents | Setup may write there? |
|---|---|---|---|
| Harness Server source or installation | Harness Server maintainer or installer | `harness`, `harness-mcp`, source files or installed executable resources. | A source build writes Cargo output under `target/`. |
| `Harness Runtime Home` | Local Harness operator | Harness registry, integration state, project state, and runtime data. | Yes. Agent setup creates or reuses records there. |
| `Product Repository` | Product project owner | Product files and explicitly selected project-scoped integration files. | Only when project-scoped host config or repository guidance is selected and authorized. |
| Codex or Claude Code configuration | Host operator | Host-owned settings that start `harness-mcp --integration <integration_id>`. | Yes for direct supported host setup, in the host's own location. |

Harness runtime databases, generated runtime records, logs, projections, QA results, acceptance records, close-readiness state, and residual-risk records are never stored in the `Product Repository`.

## Build The Executables

Working directory: Harness Server source repository root.

```sh
cargo build -p harness-cli -p harness-mcp
```

That builds:

- `target/debug/harness`
- `target/debug/harness-mcp`

For release executable paths and build verification, see [Installation](docs/en/getting-started/installation.md).

## First Host Setup

Use [Quickstart](docs/en/getting-started/quickstart.md) for the shortest supported host path. It shows both Codex and Claude Code.

Codex user-scope example for Product Repository A:

```sh
/opt/harness/bin/harness agent install \
  --host codex \
  --scope user \
  --integration-id int-codex-team \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --default-project-id acme-api \
  --runtime-home /Users/alex/.harness \
  --mcp-command /opt/harness/bin/harness-mcp
```

Expected success includes:

```text
status: complete
integration_id: int-codex-team
host_kind: codex
host_scope: user
server_name: harness-int-codex-team
verification: complete
```

`--server-name` is optional. When it is omitted, the CLI derives a stable
host MCP server name from `integration_id` and reports it in the result.

Claude Code project-scope example for Product Repository A:

```sh
HARNESS_HOME=/Users/alex/.harness \
PATH="/opt/harness/bin:$PATH" \
/opt/harness/bin/harness agent install \
  --host claude-code \
  --scope project \
  --integration-id int-claude-acme \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --mcp-command harness-mcp \
  --allow-repository-write
```

Expected setup may report:

```text
status: action_required
verification_detail: Claude Code requires user approval before project-scoped .mcp.json servers load
```

`action_required` is a successful administrative result. Complete the named host action in Codex or Claude Code, then run `harness agent verify`.

## Documentation Routes

- English documentation: [docs/en/README.md](docs/en/README.md)
- Korean documentation: [docs/ko/README.md](docs/ko/README.md)
- Documentation directory guide: [docs/README.md](docs/README.md)

Reader paths:

- Product users: [Getting Started Overview](docs/en/getting-started/overview.md), then [User Guide](docs/en/guides/user-workflow.md)
- First setup: [Installation](docs/en/getting-started/installation.md), [Quickstart](docs/en/getting-started/quickstart.md), then [Agent Host Setup](docs/en/guides/agent-host-setup.md)
- Multiple repositories: [Multi-Repository Agent Setup](docs/en/guides/multi-repository-agent-setup.md)
- Agents: [Agent Guide](docs/en/guides/agent-workflow.md)
- Source-code learners: [Developer Documentation](docs/en/development/README.md), then [Codebase Tour](docs/en/development/codebase-tour.md), [Request Lifecycle](docs/en/development/request-lifecycle.md), and [Architecture](docs/en/development/architecture.md)
- Reference readers: [Reference Index](docs/en/reference/README.md)

Reader documentation explains and sequences the product. Exact contracts live in Reference documents, including [Administrative CLI](docs/en/reference/admin-cli.md), [Agent Integration](docs/en/reference/agent-integration.md), [MCP Transport](docs/en/reference/mcp-transport.md), [Runtime Boundaries](docs/en/reference/runtime-boundaries.md), and [API Methods](docs/en/reference/api/methods.md). `docs/doc-index.yaml` is maintenance metadata for exact owner routing, not an ordinary reader's first step.
