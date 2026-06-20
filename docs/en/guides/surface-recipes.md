# Surface Recipes

Use this guide when an agent needs to make Harness-connected work feel clear in a named surface: CLI, IDE/editor, chat, or local MCP.

These recipes describe user-visible flow. They cover:

- what to show
- when to refresh
- how to report failure
- how to summarize status
- how to avoid turning copied text, rendered displays, or agent memory into authority

They do not define:

- API behavior
- storage effects
- security guarantees
- surface registration
- context-exchange contracts

Exact contracts stay in the Reference owners for:

- connector behavior and current surface context
- registration details
- storage effects
- security guarantees

## CLI surface

A CLI surface should keep status compact and action-oriented.

Before meaningful work, show:

- current `Task` or work boundary
- current scope and non-goals
- relevant paths, command, or operation class
- pending user-owned judgment
- known blocker and next safe action

For write-capable work:

- Refresh status before relying on old terminal output.
- Name intended paths or operations before checking write compatibility.
- After execution, summarize what ran and what changed.
- Name which checks passed or failed.
- Name the evidence that supports each claim.
- Treat terminal logs, shell history, local paths, and copied summaries as context, not authority by themselves.

If command observation or artifact access is unavailable for the current surface, say so and name a safe recovery action.

## IDE/editor surface

An IDE/editor surface should keep the editing boundary close to the edit workflow.

It should not claim that the editor prevents every unsafe action.

Show:

- current `Task`
- intended files
- current scope match or mismatch
- stale state when relevant
- next safe action

For editing work:

- Refresh the task and current scope before broad or public-interface-facing edits.
- Compare intended file paths with the current scope before writing.
- Keep product, UX, accessibility, security, dependency, and data-shape decisions separate from editor convenience prompts.
- Keep final-acceptance and residual-risk decisions separate from editor convenience prompts.
- After meaningful saves, summarize changed files, checks run, evidence gaps, and close-readiness blockers.

An IDE/editor integration may help keep the agent honest, but security wording and guarantee levels belong to [Security](../reference/security.md).

## Chat surface

A chat surface is best at shaping, judgment requests, status summaries, and human-readable failure reporting.

For chat work:

- Start from the user's ordinary request.
- Inspect available state before asking questions.
- Ask one blocking user-owned judgment at a time unless the user asks for grouped options.
- Summarize what is verified, what is stale or unavailable, what remains user-owned, and the next safe action.
- Do not treat "yes", "go ahead", or "looks good" as a bundle of every approval or judgment.
- Keep final acceptance, residual-risk acceptance, sensitive-action approval, write approval, and `Write Authorization` distinct.

If the chat surface cannot verify local access, state, artifact availability, or capability support, say that directly.

Route to the applicable next action:

- reconnect
- refresh
- use a capable surface
- narrow the operation
- continue explicitly outside Harness

## Local MCP surface

A local MCP surface is the practical route for supported Harness methods.

Use it for status, scope updates, write checks, run/evidence recording, user-judgment capture, artifact staging, and close checks.

For executable local setup, start with [Local MCP Setup](local-mcp-setup.md); its common path is `harness setup local-mcp`.

This recipe does not define the supported method list, surface registration, context exchange, storage effects, or security guarantees. Use the Reference owners for those details.

For local MCP work:

- Confirm reachability before relying on protected state.
- Use [Agent Integration Reference](../reference/agent-integration.md) for `surface_id`, `VerifiedSurfaceContext`, and `capability_profile`.
- Use the same owner for surface registration and current surface context.
- Treat identifiers as routing context, not authority by themselves.
- Refresh user-visible status when `state_version`, task identity, current scope, surface capability, or artifact context changes.
- Keep artifact body access separate from artifact staging and run/evidence recording.
- Use [Artifact Storage](../reference/storage-artifacts.md) for storage-effect detail.
- When local MCP is unavailable, report the owner-provided condition and name the next safe recovery action.

Do not use local MCP availability to imply stronger observation, prevention, or isolation than the relevant owners support.

## Failure reporting

Failure reporting should lead with the primary blocker and the next action that can unblock it.

Name whether the blocker is:

- user-owned
- agent-resolvable
- surface/system-owned
- capability-limited

Include:

- what the surface tried to do
- what was verified
- what was unavailable or stale
- what was not changed or recorded
- any owner-provided code
- the next safe action

## Status summaries

Keep status short and decision-ready.

A good status summary says:

- current `Task` or work boundary
- current scope
- primary blocker
- applicable guarantee level or capability limit
- evidence or artifact gap when relevant
- close readiness when relevant
- one next safe action

Do not bury the user in schema fields, logs, generated readable views, or long history.

Pull exact Reference detail only when the next action depends on the contract.

## Reference owners

Use these owner routes for exact contracts:

- [Agent Integration Reference](../reference/agent-integration.md): connector behavior, `capability_profile`, `VerifiedSurfaceContext`, surface registration, context exchange, fallback boundaries.
- [API Methods](../reference/api/methods.md): supported method list and method-owner routing.
- [API error codes](../reference/api/error-codes.md), [API error routing](../reference/api/error-routing.md): public error codes and recovery routing.
- [Artifact Storage](../reference/storage-artifacts.md): artifact access, staging, and storage-effect detail.
- [Security](../reference/security.md): guarantee wording and non-claims.
- [Runtime Boundaries](../reference/runtime-boundaries.md): Harness Server source/installation files, executable processes, `Product Repository`, `Harness Runtime Home`, and external MCP host configuration separation.
- [Scope](../reference/scope.md): baseline, profile-gated, and out-of-scope boundaries.

## Where to go next

Use [Agent Guide](agent-workflow.md) for general agent behavior.

Use [Agent Integration Reference](../reference/agent-integration.md) for exact connector ownership.

Implementers should continue through the [Reference Index](../reference/README.md). Do not treat these recipes as API or schema contracts.
