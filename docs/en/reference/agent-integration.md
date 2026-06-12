# Agent integration reference

This document owns agent connector behavior and capability-context boundaries for the current documentation set. It does not own surface-specific usage recipes; those live in [Surface Recipes](../use/surface-recipes.md).

This is documentation reference material only. It does not implement a connector, MCP server, runtime state, generated manifest, or conformance runner.

## Owns / Does not own

This document owns:

- `capability_profile` meaning at the connector boundary
- `VerifiedSurfaceContext` meaning at the connector boundary
- guarantee display gating from verified capability context
- context push/pull guidance
- connector fallback semantics
- connector conformance boundary
- one-language-per-`doc_id` retrieval guidance for agent context

This document does not own:

- CLI, IDE/editor, chat, or local MCP surface recipes; see [Surface Recipes](../use/surface-recipes.md)
- API method behavior, request envelopes, or schema shapes; see API owners through [Reference Index](README.md)
- storage layout, artifact lifecycle, or staged-handle validation; see storage and artifact owners through [Reference Index](README.md)
- security guarantee meanings; see [Security](security.md)
- Product Repository, Harness Server, and Runtime Home separation; see [Runtime Boundaries](runtime-boundaries.md)
- exact template bodies; see [Template Bodies](template-bodies.md)

## Connector boundary

Connectors carry context between Harness and an agent surface.

Condition:
- The connector is only a carrier between owner-defined Harness results and the selected agent surface.
- Local surface authority depends on the registered and verified surface context defined by the API and security owners.

Agent may:
- request owner-defined Harness state through a connector
- display owner results
- pass compact context to the agent

Agent must not:
- treat a connector description, generated file, chat text, Product Repository file, projection, or agent memory as authority by itself
- create Core state, user-owned judgment, `Write Authorization`, evidence sufficiency, artifact authority, close readiness, residual-risk acceptance, or security guarantees from prose or cached display text

Owner links:
- [Surface Recipes](../use/surface-recipes.md) owns surface-specific recipes.
- [Runtime Boundaries](runtime-boundaries.md) owns Product Repository, Harness Server, and Runtime Home separation.
- [Security](security.md) owns security guarantee meanings.

## `capability_profile`

`capability_profile` is the connector-owned description of what a registered surface can support.

Condition:
- The relevant owner documents must make a concept active before `capability_profile` can describe it as supported.
- Before a protected read, mutation, artifact operation, detective display, or guarantee claim relies on `capability_profile`, compare it with the registered local surface and the current request.
- Profile-gated behavior remains inactive until active-scope and owner documents promote it with scope, fallback behavior, and proof expectations.

Agent may:
- describe supported access classes
- describe local reachability
- describe changed-path detection
- describe artifact staging or body-read support
- describe display capabilities
- show missing support as unavailable or capability-limited

Agent must not:
- treat `capability_profile` as authority by itself
- use a stale, copied, generated, or user-provided capability description to make a out-of-scope capability active
- use the same description to justify a stronger guarantee level

Fallback:
- When required support is absent, stale, mismatched, or insufficient, display unavailable or capability-limited state and route the next decision to the relevant owner result.

Owner links:
- [Scope](scope.md) owns active and profile-gated scope boundaries.
- [Security](security.md) owns guarantee vocabulary and guarantee-strength non-claims.
- [API Value Sets](api/schema-value-sets.md) owns access-class value names.

## Local surface registration

Local surface registration provides the facts a server uses to derive verified surface context. A connector may carry selectors and display owner results, but it does not create local authority.

Condition:
- A request may select a registered local surface with `surface_id`.
- A server, not the connector, matches the selected surface to registered local facts, transport/session/binding evidence, access class, and capability posture.
- Protected reads, mutations, and artifact operations can rely on a surface only when the method owner says the verified context is compatible.

Agent may:
- pass `surface_id` as a selector
- display owner-returned unavailable, mismatched, stale, or insufficient surface states

Agent must not:
- assert local reachability, access class, `verified=true`, or staged artifact provenance from caller prose, copied identifiers, generated Markdown, chat text, projection text, or agent memory
- treat `surface_id`, `surface_instance_id`, or a surface name as permission evidence

Fallback:
- If registered facts cannot be matched to the current request, display the local surface as unavailable, mismatched, stale, or insufficient until the owner method returns a compatible verified context.

Owner links:
- [API Methods](api/methods.md) and method owner documents own method request conditions.
- [API Value Sets](api/schema-value-sets.md) owns access-class values.
- [Security](security.md) owns access-boundary and guarantee wording.

## `VerifiedSurfaceContext`

`VerifiedSurfaceContext` is the result a server derives by matching a request's selected `surface_id` to registered local surface facts, transport/session/binding evidence, access class, and capability posture.

Condition:
- A public API request has exactly one request-level `VerifiedSurfaceContext.access_class`.
- Nested payloads such as artifact inputs do not add a second request access class.
- In a server, staged artifact provenance such as `created_by_surface_id` and `created_by_surface_instance_id` comes from `VerifiedSurfaceContext`.
- Protected reads and mutations can rely on a surface only when the API owner says the verified context is compatible with the method.

Agent may:
- use an owner response that includes compatible request-level `VerifiedSurfaceContext`
- preserve request-level `VerifiedSurfaceContext.access_class` when displaying or passing context

Agent must not:
- assert `verified=true`
- supply staged artifact provenance from caller prose
- use copied identifiers, generated Markdown, chat text, projection text, or agent memory as substitutes for the verified context

Fallback:
- If verified context is absent or incompatible with the method, show the relevant unavailable, mismatched, stale, or insufficient state instead of relying on the surface.

Owner links:
- The exact request envelope and access-class values belong to the [API Methods](api/methods.md), method owner documents, and [API Value Sets](api/schema-value-sets.md).

## Guarantee display gating

Guarantee display starts at the current documented level: cooperative by default.

Condition:
- Limited `detective` display requires the relevant capability verification to pass.
- Limited `detective` display requires the security owner to allow that wording.
- The displayed scope must be limited to the named surface, capability, and observed scope.

Agent may:
- display limited `detective` wording only when every condition above is satisfied
- display limitation conditions when any of these are true:
  - Core, MCP, local access, changed-path detection, artifact access, or another required capability is unavailable
  - a required capability is stale, mismatched, or insufficient

Agent must not:
- infer `detective`, `preventive`, or `isolated` from a surface name
- infer a stronger guarantee level from a status card, chat summary, rendered projection, or user phrase

Fallback:
- When the relevant capability cannot support the displayed guarantee, show the limitation as unavailable or capability-limited instead of strengthening the claim.

Owner links:
- [Security](security.md) owns guarantee vocabulary and non-claims.
- [Scope](scope.md) owns current MVP scope and profile-gated boundaries.

## Context push and pull

Condition:
- A connector may push compact agent context only when it is fresh enough for the next action and compatible with the current surface.
- A connector should pull exact owner sections only when the next action needs them.

Agent may:
- push a compact packet containing:
  - current task summary
  - active scope and non-goals
  - relevant surface status
  - `state_version`
  - pending user-owned judgments
  - blockers
  - next safe action
  - evidence gaps
  - artifact availability summary
  - close readiness
  - residual-risk status
  - guarantee level
  - source refs and freshness
- pull exact owner sections for the next action

Agent must not:
- push full schemas
- push DDL
- push template bodies
- push historical logs
- push generated artifacts
- push full artifact contents
- push unrelated contract material
- push future catalog material
- push both languages for the same `doc_id`, unless bilingual maintenance requires semantic-parity review

Fallback:
- If a pushed context packet becomes stale, disconnected, or incompatible with the current surface, ask the owner path for a refreshed result or show the stale condition before the agent relies on it.

Owner links:
- [Reference Index](README.md) routes exact owner sections.
- [Surface Recipes](../use/surface-recipes.md) owns surface-specific usage recipes.
- [Translation Guide](../maintain/translation-guide.md) owns bilingual semantic-parity review expectations.

## Fallback semantics

Condition:
- Core, MCP, projection data, local access, changed-path detection, artifact access, or another required capability is unavailable
- a required capability is stale, mismatched, or insufficient

Agent may:
- reconnect or diagnose
- move to a capable surface
- narrow the operation
- refresh state
- request the missing user-owned judgment
- continue outside Harness only when the user explicitly chooses that mode

Agent must not:
- fabricate authority
- hide unavailable, mismatched, stale, or insufficient capability states inside ordinary success text
- continue outside Harness without the user's explicit choice

Fallback:
- Expose the limitation and route the next safe action to the relevant owner.
- Use owner-defined failure meanings. Typical routing is:
  - `MCP_UNAVAILABLE`: Core, MCP, or required surface reachability is unavailable.
  - `LOCAL_ACCESS_MISMATCH`: reachable local access does not match the registered surface expectation or has been revoked.
  - `CAPABILITY_INSUFFICIENT`: the surface is recognized but lacks a required access class, observation, artifact capability, or guarantee support.

Owner links:
- [API Errors](api/errors.md) owns public error-code routing.
- [Surface Recipes](../use/surface-recipes.md) owns practical surface failure summaries.
- [Security](security.md) owns guarantee limitation wording.

## Connector conformance boundary

Connector conformance means preserving owner-defined results and not strengthening them. A conforming connector:

- derives authority from owner paths rather than generated or conversational text
- preserves the request-level `VerifiedSurfaceContext.access_class` boundary
- reports unavailable, mismatched, stale, or insufficient capability states without inventing Core records
- displays guarantee levels only when the relevant owner and capability check support them
- keeps user-owned judgment, sensitive-action approval, final acceptance, residual-risk acceptance, evidence sufficiency, and close readiness distinct
- keeps surface recipes in [Surface Recipes](../use/surface-recipes.md) instead of turning this reference into an operating manual

This boundary is a documentation contract for future connector behavior. It is not an executable conformance runner and does not create generated conformance output.

## Related owners

- [Surface Recipes](../use/surface-recipes.md) for practical surface-specific usage.
- [API Schema Core](api/schema-core.md) and [API Value Sets](api/schema-value-sets.md) for common API context fields and values.
- [API Methods](api/methods.md) and method owner documents for method request conditions.
- [Security](security.md) for guarantee wording and non-claims.
- [Runtime Boundaries](runtime-boundaries.md) for Product Repository, Harness Server, and Runtime Home separation.
- [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), and [Artifact Storage](storage-artifacts.md) for storage and artifact authority boundaries.
