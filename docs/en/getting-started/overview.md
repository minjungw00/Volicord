# Getting Started Overview

This is the first-read overview for Volicord. It explains the product thesis in ordinary language and routes exact contract questions to Reference owners.

<a id="what-volicord-is"></a>
## What Volicord Is

Volicord is the local work-authority product/system for AI-assisted product work: a local authority control plane for a user, an AI host, and an agent. Its thesis is simple: AI-assisted work should keep the user's authority basis visible while the work moves.

Core is the local authority record for Volicord state. Volicord is the broader product/system around that record, including its local runtime components, Agent Connections, supported host configuration, and documentation routes.

Volicord is not an OS security product. It does not provide OS sandboxing, filesystem ACLs, network policy, or secret isolation.

## The Ordinary Problem

A user might ask an agent to change product behavior, investigate a failure, or prepare a release note. The agent may inspect files, propose a plan, write code, run tests, and summarize the outcome. That speed is useful, but it can also hide substitutions:

- A small request becomes a broader product change.
- A product decision gets buried inside implementation.
- Evidence for one claim starts sounding like evidence for everything.
- A passing test is treated as final acceptance.
- A user's casual approval is treated as every unresolved judgment being settled.

Volicord exists to make those substitutions visible. It gives the agent and user a local place to keep scope, user-owned judgment, evidence, verification criteria, acceptance, residual risk, and close readiness distinct.

## Local Pieces

These names are related, but they are not interchangeable.

| Name | First-read meaning | Exact owner |
|---|---|---|
| Volicord | The local work-authority product/system and authority control plane for AI-assisted product work. | [What Volicord Is](#what-volicord-is) |
| Core | The local authority record for Volicord state. | [Core Model](../reference/core-model.md) |
| Volicord implementation | The implementation set maintained by this repository, including Core, storage, types, the `volicord` CLI, `volicord-mcp`, tests, documentation, and validation tooling. | [Runtime Boundaries](../reference/runtime-boundaries.md) |
| `volicord` | The local administrative CLI that initializes Runtime Homes, registers projects, manages Agent Connections, and provides the local User Channel. | [Administrative CLI](../reference/admin-cli.md) |
| `volicord-mcp` | The stdio MCP adapter process that generated host configuration starts as a child process for the selected Agent Connection. | [MCP Transport](../reference/mcp-transport.md) |
| `Volicord Runtime Home` | The local runtime data space for Volicord operational data as storage/runtime owners define it. | [Runtime Boundaries](../reference/runtime-boundaries.md) |
| `Product Repository` | The user's project workspace and product files. It may contain explicitly selected project-scoped host configuration, but it is not Core authority and is not a runtime home. | [Runtime Boundaries](../reference/runtime-boundaries.md) |
| Agent Connection | A local MCP host connection unit. It binds one host configuration target to one managed connection identity, a mode, and explicitly connected Projects. | [Agent Connection Reference](../reference/agent-connection.md) |
| User Channel | The local user path for authority-bearing user judgments. Agent Connections do not record `user_only` judgments. | [Administrative CLI](../reference/admin-cli.md#user-channel-commands) |

The current baseline agent host model is connection-based. One `volicord-mcp`
process binds to one Agent Connection through an internal connection identity,
and the connection can access only Projects explicitly connected to it. Exact
project-selection and MCP tool-argument behavior belongs to
[Agent Connection Reference](../reference/agent-connection.md) and
[MCP Transport](../reference/mcp-transport.md).

## What Setup Does

Agent setup can:

- create or reuse Runtime Home records
- register or reuse a `Product Repository`
- create or reuse an Agent Connection
- connect one selected repository project during each `volicord connect`
  invocation
- install Codex or Claude Code host configuration, or export generic MCP configuration
- run setup verification and report `complete`, `action_required`, or `failed`

Agent setup must not:

- grant access to every Project in the Runtime Home
- store Volicord runtime databases or runtime records in a `Product Repository`
- bypass Codex project trust, Claude Code project MCP approval, OAuth, reloads, restarts, or other host-owned actions
- promise that a model will choose Volicord tools automatically

## First-Read Authority Concepts

At first-read level, Volicord documentation keeps these authority concepts separate and routes their exact meaning to [Core Model](../reference/core-model.md):

- User-owned judgment remains user-owned; an agent may explain options, but it must not invent the judgment.
- The User Channel records user judgments with `actor_source=local_user` and `operation_category=user_only`.
- Agent Connection calls use agent-connection provenance and an operation category allowed by the connection mode.
- Evidence supports a specific recorded claim. It is not final acceptance or residual-risk acceptance.
- Verification criteria guide what should be checked. They are not themselves evidence or acceptance.
- A `Write Check` is Core-state compatibility for one product-file write attempt. It is distinct from ordinary write approval, sensitive-action approval, final acceptance, and residual-risk acceptance, and it is not OS permission.
- Close readiness is a Core authority concept, not a proof of product correctness.

## Connection Modes

Agent Connections can be read-oriented or workflow-capable. Use read-oriented
mode when a host should inspect state, discover projects, or check close
readiness without workflow mutation tools. Use workflow mode for normal agent
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
