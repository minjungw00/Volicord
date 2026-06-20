# Agent integration reference

This document owns how agent-facing surfaces are registered, selected for current surface context, and described by capability declarations. It also defines the boundary for carrying owner-result Harness context into an agent surface.

It does not define API schemas, method behavior, storage effects, security guarantee meanings, projection/display authority boundaries, or rendered template wording.

## Owns / Does not own

This document owns:

- surface registration inputs and selector meaning for agent integration
- current surface and actor context boundaries, including `surface_id`, `surface_instance_id`, request-level `VerifiedSurfaceContext`, and authority-resolution `VerifiedActorContext`
- capability declaration boundaries for `capability_profile`
- agent context transfer rules between owner results and a surface
- fallback display when the selected surface or current surface context is unavailable, mismatched, stale, or capability-limited
- one-language-per-`doc_id` retrieval guidance for agent context

This document does not own:

- surface-specific workflows; see [Surface Recipes](../guides/surface-recipes.md)
- API request envelopes, response branches, schema shapes, method access requirements, or access-class value names; see [API Schema Core](api/schema-core.md), [API Methods](api/methods.md), method owners, and [API Value Sets](api/schema-value-sets.md)
- `harness-mcp` executable startup, process environment, stdio framing, startup validation, response wrapping, or shutdown; see [MCP Transport](mcp-transport.md)
- storage layout, artifact lifecycle, or staged-handle validation; see storage and artifact owners through [Reference Index](README.md)
- security guarantee meanings or access-boundary wording; see [Security](security.md)
- authority versus projected display rules; see [Projection and template display boundaries](projection-and-templates.md)
- rendered body wording, public display labels, or template phrasing; see [Template Bodies](template-bodies.md)

## Integration boundary

Agent-facing surfaces carry context between Harness owner results and an agent. They do not create Harness authority.

Condition:
- An agent may rely on a surface only through owner-returned state or a compatible current surface context.
- Display text, chat messages, generated files, surface descriptions, `Product Repository` files, projections, and agent memory are support context only.

Agent may:
- include a registered surface selector when the method owner requires it
- show owner-result state and display labels
- pass compact owner-result context to the agent

Agent must not:
- treat surface prose, copied identifiers, rendered displays, or agent memory as authority
- create Core state, `Write Authorization`, evidence sufficiency, user-owned judgment, close readiness, acceptance, residual-risk acceptance, artifact authority, or security guarantees from display text

Owner links:
- [Core Model](core-model.md) owns Core authority, user-owned judgment, close readiness, acceptance, and residual-risk boundaries.
- [Runtime Boundaries](runtime-boundaries.md) owns `Product Repository`, Harness Server source/installation, executable-process, `Harness Runtime Home`, and external MCP host configuration separation.
- [Projection and template display boundaries](projection-and-templates.md) owns authority versus projected display rules.

## Surface registration

Surface registration names the user-selected surface and the facts method owners need when their contracts decide whether that surface can support a request.

Condition:
- `surface_id` is a selector for a registered local surface.
- `surface_instance_id` distinguishes a registered instance when a method owner returns or requires it.
- `surfaces.local_access_json` is the baseline source of registered local access grants for that surface instance.
- The preferred grant field is `authorized_access_classes: string[]`; it may contain multiple documented access classes for the same surface instance. `access_class: string` is a backward-compatible single-value fallback.
- A baseline-workflow registration profile may expand to the explicit access-class set `read_status`, `core_mutation`, `write_authorization`, `artifact_registration`, and `run_recording`.
- A full-workflow profile must be explicitly selected and must not be the implicit default.
- `verification_basis: string` is controlled registration or adapter-binding diagnostic metadata that explains how the grant was established. It does not grant access.
- `interaction_role: string` identifies whether the surface instance acts as `agent` or `user_interaction` for authority-resolution purposes. Baseline registration has no mixed-role surface instance.
- Registration facts are usable only through owner-returned verification for the current request.

Agent may:
- pass `surface_id` and `surface_instance_id` when a method owner requires them
- display owner-returned unavailable, mismatched, stale, or insufficient surface states

Agent must not:
- infer local reachability, access class, `verified=true`, or artifact provenance from caller prose, copied identifiers, generated Markdown, chat text, projection text, or agent memory
- treat `surface_id`, `surface_instance_id`, or a surface name as permission evidence
- treat `capability_profile`, requested invocation access, or `verification_basis` as an access grant
- treat environment variables, public request fields, or caller-supplied labels as trusted verification-basis text or audit facts

Owner links:
- [API Methods](api/methods.md) and method owners define method request conditions.
- [API Value Sets](api/schema-value-sets.md) owns access-class value names.
- [Security](security.md) owns access-boundary and guarantee wording.

## Current surface context

`VerifiedSurfaceContext` is the internal, derived context for one invocation. A Harness Server executable role such as the `harness-mcp` local adapter process derives it from registered surface records, adapter-derived invocation context, and the requested invocation access, then method owners decide whether the derived context is compatible with the request. It is not a public request payload.

An MCP session is bound at adapter startup to exactly one `project_id`, one `surface_id`, and one `surface_instance_id`. Those binding values remain fixed for the lifetime of the session. The public `ToolEnvelope.project_id` and `ToolEnvelope.surface_id` fields remain present for protocol consistency, but each request value must exactly match the session binding. A request cannot switch project, surface, or surface instance.

When `surface_instance_id` is not configured explicitly, adapter startup may select a registered default for the bound project and surface, or a single unambiguous registered candidate. Absence or ambiguity is a startup failure. Identical `surface_instance_id` values in different projects do not weaken project binding; the bound `project_id` remains part of the session identity and every request must match it.

An MCP session does not bind one fixed access class for the whole process. The MCP adapter derives the requested invocation access from the public method name and typed params for the current call. Public request params never contain an invocation access class, invocation `surface_instance_id`, capability profile, verification basis, or `VerifiedSurfaceContext`. Core independently verifies both the session binding and that the method-derived requested access is included in the registered grant in `surfaces.local_access_json` before it derives `VerifiedSurfaceContext`.

Method-derived requested access:

| Public method and typed params | Requested access |
|---|---|
| `harness.status` | `read_status` |
| `harness.intake` | `core_mutation` |
| `harness.update_scope` | `core_mutation` |
| `harness.prepare_write` | `write_authorization` |
| `harness.stage_artifact` | `artifact_registration` |
| `harness.record_run` | `run_recording` |
| `harness.request_user_judgment` | `core_mutation` |
| `harness.record_user_judgment` | `core_mutation` |
| `harness.close_task` with `intent=check` | `read_status` |
| Other `harness.close_task` intents | `core_mutation` |

`InvocationContext.access_class`, or an equivalent implementation concept, is the requested invocation access for the current call. It is not authority and cannot grant an access class. `VerifiedSurfaceContext` can be derived only when the requested invocation access is included in the registered grant in `surfaces.local_access_json`.

Verification basis for newly derived contexts is composed only from controlled registration and adapter-binding values. Environment variables and public request fields cannot supply arbitrary verification-basis text. Controlled examples include `local_admin_registration`, `mcp_stdio_surface_binding`, `cli_direct_surface_binding`, and `test_fixture_binding`. Existing stored arbitrary basis strings may remain historical data, but newly written values use the controlled vocabulary. Verification basis is diagnostic metadata and never grants access.

Internal surface shape, not a public API schema:

```yaml
VerifiedSurfaceContext:
  project_id: string
  surface_id: string
  surface_instance_id: string
  access_class: string
  capability_profile: object
  verification_basis: string
```

`VerifiedActorContext` is the internal, derived actor-provenance context used when a method resolves authority-bearing user judgments. It is derived from the bound surface instance, registration role, adapter invocation context, and the public `ToolEnvelope.actor_kind` attribution. It is not a public request payload.

Internal actor shape, not a public API schema:

```yaml
VerifiedActorContext:
  role: agent | user_interaction
  surface_id: string
  surface_instance_id: string
  verification_basis: string
  assurance_level: string
```

Baseline `assurance_level` means cooperative registered-surface provenance, not cryptographic human identity. Authority-bearing resolution requires a `VerifiedActorContext.role=user_interaction`, a matching bound `surface_id` and `surface_instance_id`, and public `actor_kind=user`. `ToolEnvelope.actor_kind` is attribution only; an agent-role surface cannot gain user authority by submitting `actor_kind=user`.

Condition:
- A public API request has exactly one request-level `VerifiedSurfaceContext.access_class`.
- A public API request has at most one authority-relevant `VerifiedActorContext`, and only authority-resolution method owners consume it.
- Public `ToolEnvelope.project_id` and `ToolEnvelope.surface_id` are request echoes of the fixed session binding. They are not caller-selected authority and cannot change the session.
- `surface_instance_id` remains adapter-derived invocation context. `ToolEnvelope` does not gain `surface_instance_id`; the shared request envelope stays with [API Schema Core](api/schema-core.md#tool-envelope).
- Nested payloads such as `ArtifactInput` or `StagedArtifactHandle` do not add a second request-level access class.
- Staged artifact provenance fields such as `created_by_surface_id` and `created_by_surface_instance_id` come from the derived `VerifiedSurfaceContext` at staging time, not caller text or nested artifact input.
- Authority-provenance fields for resolved authority-bearing judgments come from `VerifiedActorContext.surface_id` and `VerifiedActorContext.surface_instance_id`, not caller text, labels, answer payloads, or copied refs.
- Protected reads, mutations, and artifact operations can rely on a surface only when the method owner accepts the derived verified context.
- `capability_profile` can describe support, but it cannot grant or elevate `VerifiedSurfaceContext.access_class`.

Agent may:
- preserve request-level `VerifiedSurfaceContext.access_class` when displaying or passing context
- expose absent or incompatible context as unavailable, mismatched, stale, or insufficient surface state

Agent must not:
- submit `VerifiedSurfaceContext` as a request payload
- submit `VerifiedActorContext` as a request payload
- assert `verified=true`
- submit `surface_instance_id` as verification authority
- submit `actor_kind=user` from an `agent` role surface to satisfy user authority
- submit access class, capability profile, or verification basis as public request authority
- fabricate staged artifact provenance
- use copied identifiers, generated Markdown, chat text, projection text, or agent memory as substitutes for verified context
- use `capability_profile` or requested invocation access as a substitute for the registered grant

Owner links:
- Exact request envelopes and response shapes belong to [API Schema Core](api/schema-core.md), [API Methods](api/methods.md), and method owners.
- Access-class values belong to [API Value Sets](api/schema-value-sets.md).
- `harness-mcp` startup, environment variables, stdio framing, startup validation, response wrapping, and shutdown belong to [MCP Transport](mcp-transport.md).

## Capability declaration

`capability_profile` is an integration declaration describing what a registered surface can support. It is not authority by itself.

Condition:
- A capability may be declared supported only when [Scope](scope.md) and the affected owners define it as baseline or profile-gated supported behavior.
- Protected reads, mutations, artifact operations, and guarantee displays may use a capability declaration only with compatible current surface context and owner-method support.
- Capability declarations remain non-authoritative and cannot add a grant to `surfaces.local_access_json`.

Agent may:
- describe supported access classes
- describe local reachability
- describe artifact staging or body-read support
- describe display limits
- show missing support as unavailable or capability-limited

Agent must not:
- use `capability_profile` to activate an out-of-scope capability
- use `capability_profile` to grant or elevate an access class
- use stale, copied, generated, or user-provided capability text to justify a stronger security guarantee
- replace method-owner access conditions or security-owner guarantee wording with a capability declaration

Owner links:
- [Scope](scope.md) owns baseline and profile-gated scope boundaries.
- [Security](security.md) owns guarantee vocabulary and guarantee-strength non-claims.
- [API Value Sets](api/schema-value-sets.md) owns access-class value names.

## Agent context transfer

Agent context transfer gives the agent enough owner context for the next action without turning the packet into an authority record.

Condition:
- Agent context should contain only owner results needed for the next action and current surface-context limits that affect that action.
- A context packet is support context, not Core state, storage state, evidence, acceptance, residual-risk acceptance, or close output.

Agent may:
- pass compact context containing the current Task summary, current scope, `state_version`, pending user-owned judgments, blockers, next safe action, evidence and artifact summaries, close-readiness and residual-risk summaries, owner-supported guarantee display, and source or limitation notes
- retrieve exact owner sections only when the next action needs them
- include both language versions for the same `doc_id` when bilingual maintenance requires semantic-parity review

Agent must not:
- inject full schemas, DDL, historical logs, artifact bodies, unrelated contract material, out-of-scope catalogs, exact template bodies, or both language versions for the same `doc_id` by default
- treat a stale or copied context packet as newer authority than the owner result or underlying record

Owner links:
- [Template Bodies](template-bodies.md) owns agent context packet wording.
- [Reference Index](README.md) routes exact owner sections.
- [Translation Policy](../maintain/translation-policy.md) owns bilingual semantic-parity review guidance.

## Fallback boundary

Fallback display applies when the current surface context or a required integration capability is unavailable, mismatched, stale, or insufficient for the requested operation.

Agent may:
- move to a capable surface
- narrow the operation
- request the missing user-owned judgment
- continue outside Harness only when the user explicitly chooses that mode

Agent must:
- expose the limitation in support or display text
- route machine-readable failure meanings to [API error codes](api/error-codes.md) and [API error details](api/error-details.md)
- route user-facing wording to [Template Bodies](template-bodies.md) or [Surface Recipes](../guides/surface-recipes.md)

Agent must not:
- fabricate authority
- hide unavailable, mismatched, stale, or insufficient capability states inside ordinary success text
- continue outside Harness without the user's explicit choice
