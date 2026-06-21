# Agent integration reference

This document owns how agent-facing surfaces are registered, selected for current surface context, and described by capability declarations. It also defines the boundary for carrying owner-result Harness context into an agent surface.

It does not define API schemas, method behavior, storage effects, security guarantee meanings, projection/display authority boundaries, or rendered template wording.

## Owns / Does not own

This document owns:

- Agent Integration Profile meaning and integration project membership rules
- Host Installation inventory meaning and host trust boundary
- surface registration inputs and selector meaning for agent integration
- current surface and actor context boundaries, including `surface_id`, `surface_instance_id`, request-level `VerifiedSurfaceContext`, and authority-resolution `VerifiedActorContext`
- capability declaration boundaries for `capability_profile`
- MCP project selection and per-project execution validation boundaries
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

## Agent Integration Profile

An Agent Integration Profile is the durable registry record for one coding-agent integration. One `harness-mcp` process is bound to one integration, not to one fixed `Product Repository`.

Stored profile fields:

- `integration_id`
- `interaction_role`
- `surface_id`
- `surface_instance_id`
- optional `default_project_id`
- `enabled`
- creation and update timestamps

Rules:

- The coding-agent integration role is `agent`.
- The profile supplies the surface and surface-instance binding for MCP calls.
- A profile can be enabled or disabled without editing host configuration.
- Registering a profile does not automatically grant access to every project in the `Harness Runtime Home`.
- An integration has access only to projects that are explicitly present in its project membership records.

Storage record families and DDL belong to [Storage Records](storage-records.md) and [Storage DDL](storage-ddl.md). Administrative creation, update, verification, and removal commands belong to [Administrative CLI](admin-cli.md).

## Integration project membership

Integration project membership is an explicit many-to-many registry relationship between an Agent Integration Profile and registered projects.

Membership fields:

- `integration_id`
- `project_id`
- creation timestamp
- a composite primary key over `integration_id` and `project_id`

Rules:

- A default project must also be an allowed project.
- Removing a project that is still the integration default must fail until the default is cleared or changed.
- Project membership does not bypass project status, path separation, storage executability, surface registration, or local access grants.
- Inactive, invalid, or execution-ineligible projects remain unavailable at execution time even if a stale membership row exists.
- Revoking membership or disabling the integration must take effect without requiring host configuration to be rewritten.

## Host Installation

A Host Installation is a registry inventory record for Harness-managed host configuration and verification state. The host configuration file remains the operational source of truth for the host. The registry record is management inventory and last-known verification state, not a substitute for the host configuration.

Stored installation fields:

- `installation_id`
- `integration_id`
- `host_kind`
- `host_scope`
- `server_name`
- `config_target`
- `managed_fingerprint`
- `last_verified_status`
- creation and update timestamps

Supported host and scope matrix:

| Host kind | Baseline scopes | Scope meaning |
|---|---|---|
| `codex` | `user`, `project` | User scope may load across the user's Codex projects. Project scope writes project-scoped Codex MCP configuration and depends on Codex project trust before the host loads it. |
| `claude_code` | `local`, `project`, `user` | Local and project scopes load only for the associated project. User scope may load across the user's Claude Code projects. |
| `generic` | `export` | Harness exports explicit configuration for a user-managed host and does not claim direct installation. |

Rules:

- Project and local scopes permit exactly the associated `Product Repository`.
- User scope may permit multiple explicitly added `Product Repository` registrations.
- Host trust, project trust, project MCP approval, OAuth, or any comparable host-controlled approval cannot be bypassed by Harness.
- A host installation can be successful as a file operation while the result state remains `action_required` because the host has not yet trusted, approved, loaded, initialized, or exposed the server.
- Agent guidance can improve tool selection, but it is not an enforcement mechanism and cannot guarantee that a model will choose Harness tools.

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

`VerifiedSurfaceContext` is the internal, derived context for one invocation. A Harness Server executable role such as the `harness-mcp` local adapter process derives it from the selected Agent Integration Profile, selected project, registered surface records, adapter-derived invocation context, and the requested invocation access. Method owners then decide whether the derived context is compatible with the request. It is not a public request payload.

An MCP session is bound at adapter startup to exactly one `integration_id`. The integration supplies `surface_id` and `surface_instance_id`. The selected project is determined per public MCP tool call, not fixed for the process lifetime.

Project selection for public MCP method calls is deterministic:

1. Use `ToolEnvelope.project_id` when supplied.
2. If it is absent and the integration permits exactly one available project, use that project.
3. If it is absent and a valid explicit `default_project_id` exists, use that default.
4. Otherwise reject the call as ambiguous and instruct the agent to call `harness.list_projects`.

The adapter must not guess a project from folder names, process current working directory, host roots, host labels, or the first row returned by storage. MCP roots may be used only as optional future or host-provided hints. Roots do not change the deterministic selection order above.

`harness.list_projects` is a read-only MCP adapter utility tool. It lists only projects explicitly allowed for the integration, shows project availability and default status, and provides enough project identity information for an agent to choose a valid `project_id`. It is outside the nine public Harness Core API methods and must not be added to the public method list.

Before a public tool call enters Core, the MCP adapter must verify:

- the integration exists and is enabled
- the selected project is explicitly allowed for that integration
- the selected project is active and executable
- the integration's `surface_id` and `surface_instance_id` are registered for that project
- the requested access class is authorized for that surface instance

The MCP session does not bind one fixed access class for the whole process. The MCP adapter derives the requested invocation access from the public method name and typed params for the current call. Public request params never contain an invocation access class, invocation `surface_instance_id`, capability profile, verification basis, or `VerifiedSurfaceContext`. Core independently verifies both the selected integration/project binding and that the method-derived requested access is included in the registered grant in `surfaces.local_access_json` before it derives `VerifiedSurfaceContext`.

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

Verification basis for newly derived contexts is composed only from controlled registration and adapter-binding values. Environment variables and public request fields cannot supply arbitrary verification-basis text. Controlled examples include `local_admin_registration`, `agent_integration_binding`, `mcp_stdio_surface_binding`, `cli_direct_surface_binding`, and `test_fixture_binding`. Existing stored arbitrary basis strings may remain historical data, but newly written values use the controlled vocabulary. Verification basis is diagnostic metadata and never grants access.

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
- Public `ToolEnvelope.project_id`, when present, is a deterministic project selector constrained by the integration project membership. It is not caller authority and cannot grant access to an unlisted, inactive, or invalid project.
- Public `ToolEnvelope.surface_id` remains an API envelope field where schema owners define it. The MCP adapter derives invocation surface identity from the selected integration and must not let caller text override the integration's `surface_id` or `surface_instance_id`.
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
- `harness-mcp` startup, integration binding, environment variables, stdio framing, startup validation, response wrapping, and shutdown belong to [MCP Transport](mcp-transport.md).

## Agent behavior guidance

Agent behavior guidance has two layers:

- MCP server instructions are always supplied by the server during MCP initialization.
- Optional `Product Repository` guidance is installed only with explicit user authorization.

Rules:

- MCP server instructions may describe cross-tool workflows, project selection rules, and limitations that apply across Harness tools.
- Optional repository guidance may add a Harness-managed block or host-specific rule file inside a `Product Repository` only under the boundary owned by [Runtime Boundaries](runtime-boundaries.md#explicit-integration-files-in-product-repositories).
- Guidance can improve tool selection, but it is not authority, access control, user judgment, security enforcement, or proof that a model will choose Harness tools.

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
