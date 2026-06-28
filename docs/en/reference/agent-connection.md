# Agent Connection Reference

This document owns Agent Connection and current connection context boundaries for local MCP host integrations. It defines how an Agent Connection, its connected projects, connection mode, `actor_source`, and `operation_category` are interpreted before a request enters Core.

It does not define public API schemas, method behavior, storage effects, security guarantee meanings, `volicord-mcp` wire behavior, or Core authority semantics.

## Owns / Does Not Own

This document owns:

- Agent Connection meaning and Connection Projects membership rules
- current connection context boundaries for MCP-host calls
- `actor_source` and `operation_category` provenance boundaries
- User Channel versus Agent Connection boundaries for authority-bearing judgment resolution
- per-call MCP project selection and project availability boundaries
- agent context transfer rules between owner results and an Agent Connection
- fallback display when the selected Agent Connection or current connection context is unavailable, mismatched, stale, or insufficient

This document does not own:

- API request envelopes, response branches, schema shapes, or operation-category value names; see [API Schema Core](api/schema-core.md), [API Methods](api/methods.md), method owners, and [API Value Sets](api/schema-value-sets.md)
- `volicord-mcp` executable startup, process environment, stdio framing, startup validation, response wrapping, or shutdown; see [MCP Transport](mcp-transport.md)
- administrative Agent Connection commands, host setup, status, verification, and uninstall behavior; see [Administrative CLI](admin-cli.md)
- storage layout, artifact lifecycle, or staged-handle validation; see storage and artifact owners through [Reference Index](README.md)
- security guarantee meanings or access-boundary wording; see [Security](security.md)
- authority versus projected display rules; see [Projection and template display boundaries](projection-and-templates.md)
- rendered body wording, public display labels, or template phrasing; see [Template Bodies](template-bodies.md)

## Agent Connection

An Agent Connection is the local MCP host connection unit identified by `connection_id`. One `volicord-mcp --connection <connection_id>` process is bound to one Agent Connection, not to one fixed `Product Repository`.

Stored Agent Connection fields include:

- `connection_id`
- `host_kind`
- `host_scope`
- `server_name`
- `config_target`
- `mode`
- `enabled`
- `managed_fingerprint`
- `last_verified_status`
- creation and update timestamps

Rules:

- An Agent Connection is agent-facing and cannot act as the local `User Channel`.
- A connection can be enabled or disabled without editing host configuration.
- Registering a connection does not automatically grant every project in the `Volicord Runtime Home`.
- A connection can address only projects explicitly present in its Connection Projects records.
- `connection.mode=read_only` exposes read/project discovery operations. It is not a workflow-write capability.
- `connection.mode=workflow` exposes agent workflow operations as well as read/project discovery operations. It does not expose user-only judgment recording.
- `connection_id`, connection mode, host configuration, or MCP server instructions are not OS permissions, host trust, secret isolation, filesystem ACLs, network policy, or user authority.

Storage record families and DDL belong to [Storage Records](storage-records.md) and [Storage DDL](storage-ddl.md). Administrative creation, update, verification, and removal commands belong to [Administrative CLI](admin-cli.md).

## Connection Projects

Connection Projects are the explicit registry relationship between an Agent Connection and registered projects.

Membership fields:

- `connection_id`
- `project_id`
- creation timestamp
- a composite primary key over `connection_id` and `project_id`

Rules:

- Project membership does not bypass project status, path separation, storage executability, Agent Connection mode, or method-owned invocation requirements.
- Invalid current project registrations must be rejected by Connection Projects listing and access resolution instead of returned as connected project records.
- Inactive or otherwise execution-ineligible valid projects remain unavailable at execution time even if membership exists.
- Removing a Connection Project or disabling the Agent Connection must take effect without requiring host configuration to be rewritten.
- An Agent Connection with no connected projects may remain stored, and host configuration may also remain on disk. That stored state does not mean a new `volicord-mcp` process can start successfully.
- New MCP stdio startup and `volicord-mcp --check --connection <connection_id>` fail startup validation when the Agent Connection has zero connected projects.
- A `volicord-mcp` process that already started while at least one project was connected can observe later membership changes without host configuration being rewritten. After the last membership is removed, `volicord.list_projects` may return an empty project list, but project-routed public tools cannot proceed normally because no connected project remains.
- The Agent Connection is executable again only after a project is connected and the startup or per-call project checks can validate the required project state.

## Host Configuration Inventory

A stored Agent Connection is management inventory for Volicord-managed host configuration and verification state. The host configuration file remains the operational source of truth for the external host. The registry record is management inventory and last-known verification state, not a substitute for the host configuration.

Supported host and scope matrix:

| Host kind | Baseline scopes | Scope meaning |
|---|---|---|
| `codex` | `user`, `project` | User scope may load across the user's Codex projects. Project scope writes project-scoped Codex MCP configuration and depends on Codex project trust before the host loads it. |
| `claude_code` | `local`, `project`, `user` | Local and project scopes load only for the associated project. User scope may load across the user's Claude Code projects. |
| `generic` | `export` | Volicord exports explicit configuration for a user-managed host and does not claim direct installation. |

Rules:

- Project and local scopes permit exactly the associated `Product Repository`.
- User scope may permit multiple explicitly added `Product Repository` registrations.
- Host trust, project trust, project MCP approval, OAuth, or any comparable host-controlled approval cannot be bypassed by Volicord.
- A host configuration write can be successful as a file operation while the result state remains `action_required` because the host has not yet trusted, approved, loaded, initialized, or exposed the server.
- `last_verified_status=complete` may be stored only for an administrative verification result that satisfied the operational gates owned by [Administrative CLI](admin-cli.md#agent-connection-result-states). A direct Volicord-spawned MCP handshake is not enough by itself.
- `last_verified_status=action_required` is the expected state when Volicord can manage or export configuration but a host-owned trust, approval, OAuth, reload, or restart action remains.
- `generic` export remains user-managed configuration inventory. It does not prove external host loading and must not become `complete` unless a host-specific owner later defines an observable loadability gate.
- Rejected, missing, changed, unavailable, and unknown host states are not `complete` Agent Connection states.
- Product Repository guidance, generated host instructions, and MCP server instructions can improve tool selection, but they are not enforcement mechanisms and cannot guarantee that a model will choose Volicord tools.

## Current Connection Context

Current connection context is the local invocation context derived for one MCP tool call. It is derived by the local adapter from the selected Agent Connection, the selected project, the method being called, and the request envelope. It is not a public request payload.

An MCP session is bound at adapter startup to exactly one `connection_id`. The selected project is determined per public MCP tool call, not fixed for the process lifetime.

Project selection for public MCP method calls is deterministic:

1. Use `ToolEnvelope.project_id` when supplied.
2. If it is absent and the Agent Connection has exactly one connected available project, use that project.
3. Otherwise reject the call as ambiguous and instruct the agent to call `volicord.list_projects`.

The adapter must not guess a project from folder names, process current working directory, host roots, host labels, or the first row returned by storage. MCP roots may be used only as optional future or host-provided hints. Roots do not change the deterministic selection order above.

`volicord.list_projects` is a read-only MCP adapter utility tool. It lists only projects explicitly connected to the bound Agent Connection, reports availability, and provides enough project identity information for an agent to choose a valid `project_id`. It is outside the public Volicord Core API method list and must not be added to that list.

Before a public tool call enters Core, the MCP adapter must verify:

- the Agent Connection exists and is enabled
- the selected project is explicitly connected to that Agent Connection
- the selected project is active and executable
- the connection mode allows the method's `operation_category`

Connection modes and operation categories:

| Agent Connection mode | Allowed operation categories through MCP | MCP-visible public method tools | MCP adapter utility tools |
|---|---|---|---|
| `read_only` | `read` | 2: `volicord.status`, `volicord.close_task` | `volicord.list_projects` |
| `workflow` | `read`, `agent_workflow` | 8: `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.prepare_write`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_judgment`, `volicord.close_task` | `volicord.list_projects` |

`volicord.record_user_judgment` has `operation_category=user_only`. It is a public Core API method for the User Channel path, but it is not exposed by Agent Connections. The supported local user path for recording an authority-bearing answer is the `volicord user` command group owned by [Administrative CLI](admin-cli.md#user-channel-commands).

Internal actor shape, not a public API schema:

```yaml
InvocationContext:
  actor_source: local_user | system | agent_connection:<connection_id>
  operation_category: read | agent_workflow | user_only | admin_local
  verification_basis: string
  assurance_level: string
```

Baseline `assurance_level` means cooperative local provenance, not cryptographic human identity. Authority-bearing user-judgment resolution requires `actor_source=local_user`, `operation_category=user_only`, compatible User Channel provenance, and method-owned compatibility. An Agent Connection cannot gain user authority by submitting copied user text or generated guidance.

Conditions:

- A public API request has exactly one derived `InvocationContext`.
- Public `ToolEnvelope.project_id`, when present, is a deterministic project selector constrained by the Agent Connection's connected projects. It is not caller authority and cannot grant access to an unlisted, inactive, or invalid project.
- `ToolEnvelope` does not expose `actor_source` or `operation_category`. If raw MCP arguments include those fields, the adapter rejects the call before Core execution.
- Nested payloads such as `ArtifactInput` or `StagedArtifactHandle` do not add a second invocation context.
- Authority-provenance fields for resolved authority-bearing judgments come from the derived `InvocationContext`, not caller text, labels, answer payloads, copied refs, generated Markdown, or Product Repository guidance.
- Protected reads, mutations, and artifact operations can rely on an invocation only when the method owner accepts the derived context.

Agent may:

- preserve derived invocation context when displaying or passing owner-result context
- expose absent or incompatible context as unavailable, mismatched, stale, or insufficient Agent Connection state

Agent must not:

- submit `InvocationContext` as a request payload
- assert `verified=true`
- submit `actor_source=local_user` or `operation_category=user_only` from an Agent Connection to satisfy user authority
- submit arbitrary verification-basis text as public request authority
- fabricate staged artifact provenance
- use copied identifiers, generated Markdown, chat text, projection text, or agent memory as substitutes for current connection context

Owner links:

- Exact request envelopes and response shapes belong to [API Schema Core](api/schema-core.md), [API Methods](api/methods.md), and method owners.
- `operation_category` value names belong to [API Value Sets](api/schema-value-sets.md).
- `volicord-mcp` startup, connection binding, environment variables, stdio framing, startup validation, response wrapping, and shutdown belong to [MCP Transport](mcp-transport.md).

## User Channel And Agent Connections

Agent Connections are agent-facing connections. They are not the `User Channel`, even when the model is relaying a user's words.

Conditions:

- The supported local CLI path for a human user to inspect pending judgments and record a selected Core-generated option is the `volicord user` command group owned by [Administrative CLI](admin-cli.md#user-channel-commands).
- Authority-bearing user-judgment resolution requires `actor_source=local_user`, `operation_category=user_only`, and compatible User Channel provenance.
- `actor_source=agent_connection:<connection_id>` cannot become `local_user` provenance by relaying text from a user.

Agent may:

- request a missing user-owned judgment when a method owner supports that path
- display pending judgment state and Core-generated options returned by owners
- route the human user to the supported `User Channel`

Agent must not:

- record an authority-bearing user decision from an Agent Connection
- treat a natural-language approval, chat reply, generated Markdown status, or rendered projection as User Channel provenance
- broaden one selected option into final acceptance, residual-risk acceptance, sensitive-action approval, scope acceptance, or another judgment kind
- create evidence sufficiency, acceptance, residual-risk acceptance, close readiness, or security authority from displayed judgment text

Owner links:

- [Core Model](core-model.md) owns the authority meaning of user-owned judgments, final acceptance, residual-risk acceptance, evidence, and close readiness.
- [Record-user-judgment method](api/method-record-user-judgment.md) owns public method behavior for resolving one pending judgment.
- [Projection and template display boundaries](projection-and-templates.md) owns generated display and projection authority boundaries.

## Agent Behavior Guidance

Agent behavior guidance has two layers:

- MCP server instructions are always supplied by the server during MCP initialization.
- Optional `Product Repository` guidance is installed only with explicit user authorization when an administrative command supports it.

Rules:

- MCP server instructions may describe cross-tool workflows, project selection rules, and limitations that apply across Volicord tools.
- Optional repository guidance may add a Volicord-managed block or host-specific rule file inside a `Product Repository` only under the boundary owned by [Runtime Boundaries](runtime-boundaries.md#explicit-integration-files-in-product-repositories).
- Guidance can improve tool selection, but it is not authority, access control, user judgment, security enforcement, or proof that a model will choose Volicord tools.

## Agent Context Transfer

Agent context transfer gives the agent enough owner context for the next action without turning the packet into an authority record.

Conditions:

- Agent context should contain only owner results needed for the next action and current connection-context limits that affect that action.
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

## Fallback Boundary

Fallback display applies when current connection context or a required connection mode is unavailable, mismatched, stale, or insufficient for the requested operation.

Agent may:

- move to a suitable connection mode or a different connected project
- narrow the operation
- request the missing user-owned judgment
- continue outside Volicord only when the user explicitly chooses that mode

Agent must:

- expose the limitation in support or display text
- route machine-readable failure meanings to [API error codes](api/error-codes.md) and [API error details](api/error-details.md)
- route user-facing wording to [Template Bodies](template-bodies.md)

Agent must not:

- fabricate authority
- hide unavailable, mismatched, stale, or insufficient context states inside ordinary success text
- continue outside Volicord without the user's explicit choice
