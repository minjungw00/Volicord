# Getting Started Overview

This is the first-read overview for Volicord. It explains the product thesis in ordinary language and routes exact contract questions to Reference owners.

<a id="what-volicord-is"></a>
## What Volicord Is

Volicord is a local work authority record for AI-assisted product work. Its thesis is simple: AI-assisted work should keep the user's authority basis visible while the work moves.

Volicord includes local runtime components, Agent Connections, supported host configuration, and documentation routes. Exact authority-record structure belongs to [Core Model](../reference/core-model.md), but first-read user paths do not require that internal term.

Volicord is not a permission system, OS security product, sandbox, or proof
system. Exact guarantee wording and non-guarantees live in
[Security](../reference/security.md).

## The Ordinary Problem

A user might ask an agent to change product behavior, investigate a failure, or prepare a release note. The agent may inspect files, propose a plan, write code, run tests, and summarize the outcome. That speed is useful, but it can also hide substitutions:

- A small request becomes a broader product change.
- A product decision gets buried inside implementation.
- Evidence for one claim starts sounding like evidence for everything.
- A passing test is treated as final acceptance.
- A user's casual approval is treated as every unresolved judgment being settled.

Volicord exists to make those substitutions visible. It gives the agent and user a local place to keep scope, User Judgment, Evidence, verification criteria, acceptance, residual risk, and Close Status distinct.

## Local Pieces

These names are related, but they are not interchangeable.

| Name | First-read meaning | Exact owner |
|---|---|---|
| Volicord | The local work authority record for AI-assisted product work. | [What Volicord Is](#what-volicord-is) |
| `volicord` | The installed executable that provides local administrative CLI commands, the local User Channel, and the `mcp` subcommand used by generated MCP host configuration. | [Administrative CLI](../reference/admin-cli.md) |
| `volicord mcp --stdio` | The stdio MCP process mode that generated host configuration starts as a child process for the selected Agent Connection. | [MCP Transport](../reference/mcp-transport.md) |
| `Volicord Runtime Home` | The local runtime data space for Volicord operational data as storage/runtime owners define it. | [Runtime Boundaries](../reference/runtime-boundaries.md) |
| `Product Repository` | The user's project workspace and product files. It may contain explicitly selected project-scoped host configuration, but it is not Volicord runtime state. | [Runtime Boundaries](../reference/runtime-boundaries.md) |
| Agent Connection | A local MCP host connection unit. It binds one host configuration target to one managed connection identity, a mode, and explicitly connected Projects. | [Agent Connection Reference](../reference/agent-connection.md) |
| User Channel | The local user path for authority-bearing user judgments. Agent Connections do not record `user_only` judgments. | [Administrative CLI](../reference/admin-cli.md#user-channel-commands) |

The current baseline agent host model is connection-based. One
`volicord mcp --stdio` process binds to one Agent Connection through an internal
connection identity, and the connection can access only Projects explicitly
connected to it. Exact project-selection and MCP tool-argument behavior belongs to
[Agent Connection Reference](../reference/agent-connection.md) and
[MCP Transport](../reference/mcp-transport.md).

## What Setup Does

Agent setup through the ordinary
`volicord init --host HOST --repo PATH --profile record` path can:

- create or reuse Runtime Home records
- create or reuse the installation profile
- register or reuse a `Product Repository`
- create or reuse an Agent Connection and Connection Projects membership
- install project-scoped Codex or Claude Code MCP configuration that starts
  `volicord mcp --stdio`
- install Volicord-managed guidance and policy metadata
- record integration state
- run setup verification and report `complete`, `action_required`, or `failed`

The Record profile (`--profile record`) records authority state and exposes MCP
tools without requiring host lifecycle hooks or a session watcher. The Detective
profile (`--profile detective`) adds supported host hooks and session watcher
observation. Host hooks can return cooperative host warning or denial decision
signals, and the watcher can report Unrecorded Changes after coverage starts;
neither surface prevents all writes, proves who changed a file, provides a
sandbox, or adds OS-level enforcement. Exact profile
behavior is defined by [Administrative CLI](../reference/admin-cli.md).

`volicord init` is the public first-run setup path. `volicord connection add`
remains the lower-level connection-management command for personal, shared,
global, and read-only flows.

Agent setup must not:

- grant access to every Project in the Runtime Home
- store Volicord runtime databases or runtime records in a `Product Repository`
- bypass Codex project trust, Claude Code project MCP approval, OAuth, reloads, restarts, or other host-owned actions
- promise that a model will choose Volicord tools automatically

## First-Read Authority Concepts

At first-read level, Volicord documentation keeps these authority concepts separate and routes their exact meaning to [Core Model](../reference/core-model.md):

- User-owned judgment remains user-owned; an agent may explain options, but it must not invent the judgment.
- Evidence supports a specific recorded claim. It is not final acceptance or residual-risk acceptance.
- Verification criteria guide what should be checked. They are not themselves evidence or acceptance.
- A Write Ticket records a Volicord work-authority decision for one product-file write attempt. It is distinct from ordinary write approval, sensitive-action approval, final acceptance, and residual-risk acceptance, and it is not OS permission, code review approval, or proof that a write occurred.
- Close Status is not proof of product correctness, test sufficiency, QA completion, deployment success, human review completion, or risk-free completion.

## Connection Modes

Agent Connections can be read-oriented or workflow-capable. Use read-oriented
mode when a host should inspect state, discover projects, or check Close Status
without workflow mutation tools. Use workflow mode for normal agent
workflow operations. Exact CLI selection behavior belongs to
[Administrative CLI](../reference/admin-cli.md#connection-intents-and-hosts),
and exact MCP-visible tool exposure belongs to
[MCP Transport](../reference/mcp-transport.md#tool-discovery-and-toolscall-response-wrapping).

## What Volicord Is Not

Use this overview for first-read product identity. For the exact supported baseline and out-of-scope boundaries, use [Scope](../reference/scope.md#product-role-exclusions).

Volicord does not turn a polished chat answer, generated summary, readable status card, copied identifier, optional repository guidance, or `Projection` into the authority record. Exact display boundaries belong to [Projection and Templates](../reference/projection-and-templates.md), runtime and location boundaries belong to [Runtime Boundaries](../reference/runtime-boundaries.md), and security wording belongs to [Security](../reference/security.md).

## Next Reader Journeys

| Reader | Next path |
|---|---|
| New product reader | [User Guide](../guides/user-workflow.md) |
| Environment check | [System Requirements](../reference/system-requirements.md) |
| First setup | [Installation](installation.md) -> [Quickstart](quickstart.md) |
| Agent host operator | [Quickstart](quickstart.md) -> [Agent Host Setup](../guides/agent-host-setup.md) -> [Agent Host Troubleshooting](../guides/agent-host-troubleshooting.md) |
| Multi-repository operator | [Multi-Repository Agent Setup](../guides/multi-repository-agent-setup.md) |
| Agent author | [Agent Guide](../guides/agent-workflow.md) -> [Agent Connection Reference](../reference/agent-connection.md) |
| Source-code learner | [Implementation Guide](../development/change-guide.md) -> [Architecture](../development/architecture.md) |
| Reference reader | [Reference Index](../reference/README.md), [Administrative CLI](../reference/admin-cli.md), [API Methods](../reference/api/methods.md) |

New readers should not need API schemas or owner metadata to understand what Volicord is. Use the [Reference Index](../reference/README.md) when you need exact contract owners.
