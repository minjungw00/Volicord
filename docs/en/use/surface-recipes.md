# Surface recipes

This document owns practical surface-specific usage recipes for the current documentation set. It is Use documentation, not a connector contract, API schema, storage contract, security proof, or implementation guide.

## What this guide is

Use this guide when an agent or maintainer needs to describe how Harness-connected work should feel in a named surface: CLI, IDE/editor, chat, or local MCP.

Recipes translate the connector contract into user-visible behavior. They say what to show, when to refresh, how to report failures, and how to summarize status without turning copied text, rendered displays, or agent memory into authority.

## What this guide is not

This guide does not define connector behavior, `capability_profile`, `VerifiedSurfaceContext`, fallback semantics, API methods, schema fields, storage effects, artifact authority, template bodies, or guarantee meanings. Those contracts stay in the Reference owners linked below.

This guide also does not prove that this documentation-only repository contains a Harness Server, runtime state, generated artifacts, or executable conformance output.

## CLI surface

A CLI surface should keep status compact and action-oriented. Before a meaningful operation, show the current task or work shape, active scope and non-goals, relevant paths or operation class, pending user-owned judgments, known blockers, current guarantee level, and the next safe command or action.

For write-capable work, the CLI recipe is:

- Refresh Harness status before relying on old terminal output.
- Name the intended product-file paths or operation before checking write compatibility.
- Use the owner path for write compatibility when product-file writes are near.
- After meaningful execution, record what ran, what changed, which checks passed or failed, and which evidence or artifact refs support the claim.
- If artifact bytes matter, treat staging as temporary input until the owner path promotes a persistent `ArtifactRef`.

Terminal logs, shell history, local paths, and copied summaries are useful context, but they are not authority by themselves. If command observation is not supported for the current surface, say the guarantee is cooperative or capability-limited.

## IDE/editor surface

An IDE/editor surface should make the active editing boundary visible without claiming that the editor prevents every unsafe action. Show the active task, intended files, scope match or mismatch, stale state, and the next safe action close to the edit workflow.

For editing work, the recipe is:

- Refresh the active task and scope before broad or public-interface-facing edits.
- Compare intended file paths with the current scope before writing.
- Keep user-owned product, UX, accessibility, security, dependency, migration, final acceptance, and residual-risk decisions separate from editor convenience prompts.
- After saving meaningful edits, summarize changed files, checks run, evidence gaps, and close blockers.
- Display changed-path detection as `detective` only after the relevant capability check passed for the observed scope.

An IDE/editor integration may help keep the agent honest, but it must not claim stronger security behavior than the security owner supports. See [Security](../reference/security.md).

## Chat surface

A chat surface is best at focused shaping, judgment requests, status summaries, and human-readable failure reporting. It should not treat conversational text as a durable record unless an owner path records the judgment, evidence, acceptance, or residual-risk decision.

For chat work, the recipe is:

- Start from the user's ordinary request; do not require Harness labels.
- Inspect available state before asking questions.
- Ask one blocking user-owned judgment at a time unless the user explicitly asks for grouped options.
- Summarize what is verified, what is stale or unavailable, what remains user-owned, and the next safe action.
- Do not treat "yes", "go ahead", or "looks good" as final acceptance, residual-risk acceptance, sensitive-action approval, and write compatibility all at once.

If the chat surface cannot verify local access, state, artifact availability, or capability support, say that directly and route to reconnect, refresh, a capable surface, a narrower operation, or explicit non-Harness continuation.

## Local MCP surface

A local MCP surface is the practical route for asking the active Harness owner paths for status, scope updates, write checks, run/evidence recording, user-judgment capture, artifact staging, and close checks when those methods are active.

For local MCP work, the recipe is:

- Confirm Core/MCP reachability before relying on protected state.
- Treat `surface_id` as a selector, not authority.
- Expect the owner path to derive `VerifiedSurfaceContext` and return a compatible result or a failure condition.
- Refresh when `state_version`, task identity, active scope, baseline, surface capability, or artifact context changes.
- Keep artifact body access separate from artifact staging and run/evidence recording.
- When local MCP is unavailable, report `MCP_UNAVAILABLE` or the owner-provided equivalent and name the next safe recovery action.

Do not use local MCP availability to imply stronger observation, prevention, or isolation than the relevant owners support. For canonical guarantee wording, see [Security](../reference/security.md).

## Failure reporting

Failure reporting should lead with the primary blocker and the next action that can unblock it. Name whether the blocker is user-owned, agent-resolvable, surface/system-owned, or capability-limited.

Include:

- what the surface tried to do
- what was verified
- what was unavailable, stale, mismatched, or insufficient
- the owner-provided code when available, such as `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, or `CAPABILITY_INSUFFICIENT`
- what was not changed or not recorded
- the next safe action

### Status summaries

When summarizing status for the user, keep it short and decision-ready. A good status summary says: current task or work shape, active scope, primary blocker, guarantee level, evidence or artifact gap when relevant, close readiness when relevant, and one next safe action.

Do not bury the user in schema fields, logs, generated projections, or long history. Pull exact Reference detail only when the next action depends on the contract.

## Links to reference contracts

- [Agent Integration Reference](../reference/agent-integration.md): connector behavior, `capability_profile`, `VerifiedSurfaceContext`, context push/pull, fallback semantics, and connector conformance.
- [MVP API](../reference/api/mvp-api.md): active method behavior and request conditions.
- [API Value Sets](../reference/api/schema-value-sets.md): access classes and enum-like API values.
- [API Errors](../reference/api/errors.md): public error codes and recovery routing.
- [Security](../reference/security.md): guarantee wording and non-claims.
- [Runtime Boundaries](../reference/runtime-boundaries.md): Product Repository, Harness Server, and Runtime Home separation.
- [Active MVP Scope](../reference/active-mvp-scope.md): active, profile-gated, and later-candidate boundaries.
