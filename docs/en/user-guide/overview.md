# User Guide Overview

This page introduces Volicord before installation or operation. Use the linked
Reference pages when exact behavior matters.

<a id="what-volicord-is"></a>
## What Volicord Is

Volicord is a local work authority record for AI-assisted product work. It
keeps the user's decision basis visible while an agent inspects, changes, and
checks product files.

The central idea is simple: fast agent work should not hide changes in scope,
unsupported claims, decisions the user has not made, or blockers that remain at
the end.

## The Problem It Addresses

An agent can inspect files, propose a plan, write code, run tests, and summarize
the result. That speed is useful, but a concise summary can blur important
boundaries:

- a small request can become a broader product change
- an implementation choice can quietly become a product decision
- evidence for one claim can sound like evidence for everything
- a passing test can be mistaken for final acceptance
- a casual approval can be treated as every unresolved decision

Volicord records these facts separately so the user and agent can see what is
known, what was checked, what still needs a decision, and what blocks close.

## Local Pieces

| Name | First-read meaning |
|---|---|
| Volicord | The local work authority record for AI-assisted product work. |
| `Product Repository` | The user's project workspace and product files. It is not Volicord runtime state. |
| `Volicord Runtime Home` | The local location for Volicord operational data. It is separate from the Product Repository. |
| Agent Connection | The local connection between an MCP host and explicitly connected projects. |
| User Channel | The local path used to record a user-owned decision. |
| `volicord` | The installed administrative command used for setup, status, and the CLI User Channel. |

The host starts the local MCP adapter through `volicord mcp --stdio`. New users
do not need its process-binding details. Exact component and location boundaries
are in [Runtime Boundaries](../reference/runtime-boundaries.md),
[Agent Connection](../reference/agent-connection.md), and
[MCP Transport](../reference/mcp-transport.md).

## What Setup Changes

Ordinary setup registers one Product Repository, creates an Agent Connection,
and writes project-scoped host configuration and guidance. Runtime records stay
in the Volicord Runtime Home. The external host still controls project trust,
MCP approval, reloads, restarts, and active tool exposure.

Follow [Installation](installation.md) and [Quickstart](quickstart.md) for the
actual setup steps. Exact command effects belong to
[Administrative CLI](../reference/admin-cli.md).

## Authority Concepts

Volicord keeps these concepts distinct:

- **Scope** says what the current work includes and excludes.
- **User Judgment** records a choice that belongs to the user. The agent may
  explain options but must not invent the decision.
- **Evidence** supports a specific claim. It is not final acceptance or proof
  of correctness.
- **Verification criteria** describe what should be checked. They are not
  themselves Evidence or acceptance.
- A **Write Ticket** records that one proposed product-file change was checked
  against the current work boundary. It is not filesystem permission, code
  review approval, or proof that the write occurred.
- **Close Status** shows whether current records still contain blockers. It is
  not proof of correctness, test sufficiency, QA completion, deployment
  success, human review completion, or risk-free completion.

Exact authority meanings are defined in [Core Model](../reference/core-model.md).

## What Volicord Is Not

Volicord is not an OS permission system, sandbox, security boundary, coding
agent, test runner, correctness oracle, or human reviewer. Guidance and status
views can help an agent and user read the current state, but they do not replace
the underlying Volicord record.

Use [Scope](../reference/scope.md) for supported and unsupported product scope.
Use [Security](../reference/security.md) for exact guarantees and
non-guarantees.

## Next Reader Paths

| Goal | Next path |
|---|---|
| Install the executable | [Installation](installation.md) |
| Make the first connection | [Quickstart](quickstart.md) |
| Work with an agent | [User Workflow](user-workflow.md) |
| Configure or repair a host | [Agent Host Setup](agent-host-setup.md) and [Agent Host Troubleshooting](agent-host-troubleshooting.md) |
| Operate an agent | [Agent Guide](agent-workflow.md) |
| Learn exact contracts | [Reference Index](../reference/README.md) |
| Learn the implementation | [Architecture Guide](../architecture-guide/README.md) |
