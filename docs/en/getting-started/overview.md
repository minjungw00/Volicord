# Getting started overview

This is the first-read overview for Harness. It explains the product thesis in ordinary language and routes exact contract questions to the Reference owners.

## What Harness Is

Harness is the local work-authority product/system for AI-assisted product work. Its thesis is simple: AI-assisted work should keep the user's authority basis visible while the work moves.

In ordinary product work, an agent can move quickly from a request to code, tests, and a polished answer. Harness helps keep the important boundaries from disappearing along the way:

- what is in scope
- what the user has actually decided
- what evidence supports a claim
- what verification criteria were checked
- what final acceptance or residual-risk acceptance still belongs to the user
- whether the work is ready to close

Harness itself is not the local authority record. Core is the local authority record for Harness state. Harness is the broader product/system around that record, including its local runtime components, surfaces, documentation, and workflows.

## The Ordinary Problem

A user might ask an agent to change a product behavior, investigate a failure, or prepare a release note. The agent may inspect files, propose a plan, write code, run tests, and summarize the outcome. That speed is useful, but it can also hide substitutions:

- A small request becomes a broader product change.
- A product decision gets buried inside implementation.
- Evidence for one claim starts sounding like evidence for everything.
- A passing test is treated as final acceptance.
- A user's casual approval is treated as every unresolved judgment being settled.

Harness exists to make those substitutions visible. It gives the agent and user a local place to keep scope, judgment, evidence, verification criteria, acceptance, residual risk, and close readiness distinct.

## Local Pieces

These names are related, but they are not interchangeable.

| Name | First-read meaning | Exact owner |
|---|---|---|
| Harness | The local work-authority product/system for AI-assisted product work. | [Scope](../reference/scope.md) |
| Core | The local authority record for Harness state. | [Core Model](../reference/core-model.md) |
| `Harness Server` | The server implementation set maintained by this repository, not a synonym for Harness as a whole. | [Runtime Boundaries](../reference/runtime-boundaries.md) |
| `Harness Runtime Home` | The local runtime data space for Harness operational data as storage/runtime owners define it. | [Runtime Boundaries](../reference/runtime-boundaries.md) |
| `Product Repository` | The user's project workspace and product files. | [Runtime Boundaries](../reference/runtime-boundaries.md) |

The Harness Server source repository is the checkout that contains implementation crates, the `harness` administrative CLI, the `harness-mcp` local MCP adapter, tests, documentation, validation tooling, and repository configuration. A Harness Server installation is the deployed subset of executables and required runtime resources; it does not imply that every source-repository file is installed.

In the current local Rust implementation, `harness` and `harness-mcp` are distinct executable roles within Harness Server. `harness` performs local administrative setup. `harness-mcp` is the stdio MCP adapter process that an MCP host starts as a local child process and communicates with through stdin/stdout. The baseline process is not a network listener.

## First-Read Authority Concepts

Harness documentation keeps these concepts separate:

- User-owned judgment is a decision or assessment the user owns. An agent may explain options, but it must not invent the judgment.
- Evidence is material support for a specific claim, such as a diff, test output, screenshot, log, source citation, review note, or artifact reference.
- Verification criteria are user-visible criteria for checking work. They guide what should be checked; they are not themselves evidence or acceptance.
- `Write Authorization` is the exact product label for Core authority around one compatible product-file write attempt. It is distinct from ordinary write approval.
- Final acceptance and residual-risk acceptance are user-owned judgments.
- Close readiness is the reference concept for whether a task can honestly close from its current state.

For exact authority rules and non-substitution boundaries, use [Core Model](../reference/core-model.md).

## What Harness Is Not

Harness is not a prompt pack, chat script, API wrapper, workflow engine, report generator, dashboard, hosted agent platform, `Product Repository`, or `Harness Runtime Home`.

Harness also does not turn a polished chat answer, generated summary, readable status card, copied identifier, or `Projection` into the authority record. Exact display boundaries belong to [Projection and Templates](../reference/projection-and-templates.md), runtime and location boundaries belong to [Runtime Boundaries](../reference/runtime-boundaries.md), and security wording belongs to [Security](../reference/security.md).

## Next Reader Journeys

| Reader | Next path |
|---|---|
| New product reader | [User Guide](../guides/user-workflow.md) |
| First local setup | [Installation](installation.md) -> [Quickstart](quickstart.md) |
| Local MCP operator | [Quickstart](quickstart.md) -> [Local MCP Setup](../guides/local-mcp-setup.md) |
| Agent author | [Agent Guide](../guides/agent-workflow.md) -> [Agent Integration](../reference/agent-integration.md) |
| Source-code learner | [Implementation Guide](../development/change-guide.md) -> [Architecture](../development/architecture.md) |
| Reference reader | [Reference Index](../reference/README.md) |

New readers should not need API schemas or owner metadata to understand what Harness is. Use the [Reference Index](../reference/README.md) when you need exact contract owners.
