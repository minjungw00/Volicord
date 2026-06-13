# Surface Recipes

Use this guide when an agent needs to make Harness-connected work feel clear in a named surface: CLI, IDE/editor, chat, or local MCP.

These recipes describe user-visible flow: what to show, when to refresh, how to report failure, and how to summarize status without turning copied text, rendered displays, or agent memory into authority. Exact connector behavior and security guarantees stay in the Reference owners.

## CLI surface

A CLI surface should keep status compact and action-oriented.

Before meaningful work, show:

- active task or work boundary
- active scope and non-goals
- relevant paths, command, or operation class
- pending user-owned judgment
- known blocker and next safe action

For write-capable work:

- Refresh status before relying on old terminal output.
- Name intended paths or operations before checking write compatibility.
- After execution, summarize what ran, what changed, which checks passed or failed, and what evidence supports the claim.
- Treat terminal logs, shell history, local paths, and copied summaries as context, not authority by themselves.

If command observation or artifact access is unavailable for the current surface, say so and name a safe recovery action.

## IDE/editor surface

An IDE/editor surface should keep the active editing boundary close to the edit workflow without claiming that the editor prevents every unsafe action.

Show:

- active task
- intended files
- scope match or mismatch
- stale state when relevant
- next safe action

For editing work:

- Refresh task and scope before broad or public-interface-facing edits.
- Compare intended file paths with the active scope before writing.
- Keep product, UX, accessibility, security, dependency, migration, final-acceptance, and residual-risk decisions separate from editor convenience prompts.
- After meaningful saves, summarize changed files, checks run, evidence gaps, and close blockers.

An IDE/editor integration may help keep the agent honest, but security wording and guarantee levels belong to [Security](../reference/security.md).

## Chat surface

A chat surface is best at shaping, judgment requests, status summaries, and human-readable failure reporting.

For chat work:

- Start from the user's ordinary request.
- Inspect available state before asking questions.
- Ask one blocking user-owned judgment at a time unless the user asks for grouped options.
- Summarize what is verified, what is stale or unavailable, what remains user-owned, and the next safe action.
- Do not treat "yes", "go ahead", or "looks good" as final acceptance, residual-risk acceptance, sensitive-action approval, and write compatibility all at once.

If the chat surface cannot verify local access, state, artifact availability, or capability support, say that directly and route to reconnect, refresh, a capable surface, a narrower operation, or explicit non-Harness continuation.

## Local MCP surface

A local MCP surface is the practical route for asking supported Harness owner paths for status, scope updates, write checks, run/evidence recording, user-judgment capture, artifact staging, and close checks when those methods are supported.

For local MCP work:

- Confirm reachability before relying on protected state.
- Treat `surface_id` as a selector, not authority.
- Refresh when `state_version`, task identity, active scope, surface capability, or artifact context changes.
- Keep artifact body access separate from artifact staging and run/evidence recording.
- When local MCP is unavailable, report the owner-provided condition and name the next safe recovery action.

Do not use local MCP availability to imply stronger observation, prevention, or isolation than the relevant owners support.

## Failure reporting

Failure reporting should lead with the primary blocker and the next action that can unblock it.

Name whether the blocker is:

- user-owned
- agent-resolvable
- surface/system-owned
- capability-limited

Include what the surface tried to do, what was verified, what was unavailable or stale, what was not changed or recorded, any owner-provided code, and the next safe action.

## Status summaries

Keep status short and decision-ready.

A good status summary says:

- active task or work boundary
- active scope
- primary blocker
- active guarantee level or capability limit
- evidence or artifact gap when relevant
- close readiness when relevant
- one next safe action

Do not bury the user in schema fields, logs, generated readable views, or long history. Pull exact Reference detail only when the next action depends on the contract.

## Reference owners

Use these owner routes for exact contracts:

- [Agent Integration Reference](../reference/agent-integration.md): connector behavior, `capability_profile`, `VerifiedSurfaceContext`, context exchange, fallback boundaries.
- [API Methods](../reference/api/methods.md): supported method list and method-owner routing.
- [API error codes](../reference/api/error-codes.md), [API error routing](../reference/api/error-routing.md): public error codes and recovery routing.
- [Security](../reference/security.md): guarantee wording and non-claims.
- [Runtime Boundaries](../reference/runtime-boundaries.md): `Product Repository`, Harness Server, and `Harness Runtime Home` separation.
- [Scope](../reference/scope.md): baseline, profile-gated, and out-of-scope boundaries.

## Where to go next

Use [Agent Guide](agent-guide.md) for general agent behavior, then [Agent Integration Reference](../reference/agent-integration.md) for exact connector ownership. Implementers should continue through the [Reference Index](../reference/README.md) instead of treating these recipes as API or schema contracts.
