# Agent Connection Reference

This document owns Agent Connection and current connection context boundaries
for local MCP host integrations. It defines how an Agent Connection, its
connection intent, connected projects, connection mode, `actor_source`, and
`operation_category` are interpreted before a request enters Core.

It does not define public API schemas, method behavior, storage effects,
security guarantee meanings, `volicord-mcp` wire behavior, or Core authority
semantics.

## Owns / Does Not Own

This document owns:

- Agent Connection meaning and Connection Projects membership rules
- connection intent meaning: `personal`, `shared`, and `global`
- current connection context boundaries for MCP-host calls
- `actor_source` and `operation_category` provenance boundaries
- User Channel versus Agent Connection boundaries for authority-bearing
  judgment resolution
- repository-root project selection and project availability boundaries at the
  Agent Connection layer
- agent context transfer rules between owner results and an Agent Connection
- fallback display when the selected Agent Connection or current connection
  context is unavailable, mismatched, stale, or insufficient

This document does not own:

- API request envelopes, response branches, schema shapes, or operation-category
  value names; see [API Schema Core](api/schema-core.md),
  [API Methods](api/methods.md), method owners, and
  [API Value Sets](api/schema-value-sets.md)
- `volicord-mcp` executable startup, process environment, stdio framing,
  startup validation, response wrapping, or shutdown; see
  [MCP Transport](mcp-transport.md)
- administrative setup, connection, status, verification, mode, remove, project,
  and export commands; see [Administrative CLI](admin-cli.md)
- storage layout, artifact lifecycle, or staged-handle validation; see storage
  and artifact owners through [Reference Index](README.md)
- security guarantee meanings or access-boundary wording; see
  [Security](security.md)
- authority versus projected display rules; see
  [Projection and template display boundaries](projection-and-templates.md)
- rendered body wording, public display labels, or template phrasing; see
  [Template Bodies](template-bodies.md)

## Agent Connection

An Agent Connection is a local MCP host connection unit stored under the
`Volicord Runtime Home`. It has an internal `connection_id`, but ordinary
text-mode user flows select it by host, connection intent, and repository root
through the commands owned by [Administrative CLI](admin-cli.md).

One `volicord-mcp` process is bound to one Agent Connection. Generated host
configuration may contain the internal connection id so the host can start that
process, but the id is not a user authority token and is not required as a
normal command input.

Stored Agent Connection fields include:

- `connection_id`
- `host_kind`
- `connection_intent`
- `server_name`
- `config_target`
- `mode`
- `enabled`
- `managed_fingerprint`
- `last_verified_status`
- creation and update timestamps

The internal host configuration key `server_name` defaults to `volicord`.

<a id="lifecycle-and-state-boundaries"></a>
## Lifecycle And State Boundaries

An Agent Connection lifecycle spans several state surfaces. A command can change
one surface without changing the others.

| Surface | Stored or owned by | Changed by | Boundary |
|---|---|---|---|
| Setup profile | Runtime Home registry setup records, including the selected Runtime Home identity and MCP command location. | `volicord setup`. | Setup is required local configuration. It is not a host trust decision, user judgment, or public API method. |
| Agent Connection registry state | `agent_connections` records under the `Volicord Runtime Home`, including internal identity, host kind, connection intent, internal server name, `config_target`, `connection.mode`, enabled state, managed fingerprint, and `last_verified_status`. | `volicord connect` creates or updates the record, `volicord connection mode` changes mode, `volicord connection verify` updates verification status, and `volicord connection remove` may remove the record after membership removal. | Registry state is management state. It is not the host configuration file and is not proof that the external host loaded, trusted, approved, or exposed the MCP server. |
| Connection Projects membership | `connection_projects` records under the same Runtime Home. | `volicord connect`, `volicord project use`, and connection removal flows can add, validate, or remove membership according to the selected repository root. | Membership controls the Agent Connection project allowlist. It does not register every Runtime Home project and does not delete project registration, project state, or Core records. |
| Host configuration | The MCP host configuration location named by `config_target`, or a user-managed generic export. | `volicord connect` installs or updates managed host configuration; `volicord connection remove` removes only matching managed content when safety checks permit it; `volicord export mcp-config` renders host-neutral configuration. | Host configuration starts `volicord-mcp`, but remains an external host integration surface. It is not identical to registry state. |
| Verification state | `last_verified_status` in the Agent Connection registry record, plus command output owned by [Administrative CLI](admin-cli.md#agent-connection-result-states). | `volicord connect` and `volicord connection verify` run observable setup, host, MCP startup, MCP initialization, and `tools/list` checks where available. | Verification can inspect both Volicord-side state and host/MCP readiness. MCP startup validation alone is not a `complete` Agent Connection. |
| Invocation eligibility | Current connection context derived by the MCP adapter at startup and per public tool call. | Affected by `enabled`, connected project availability, `connection.mode`, and the method's `operation_category`. | Eligibility can become unavailable after registry or project-state changes without any host configuration rewrite. |
| Removal | Managed host content, `connection_projects`, and sometimes `agent_connections`. | `volicord connection remove`. | Removal must not delete a `Product Repository`, project registration, project state, Core records, the Runtime Home itself, artifact storage, or unrelated host configuration. |

Rules:

- An Agent Connection is agent-facing and cannot act as the local
  `User Channel`.
- A connection can be enabled, disabled, removed, or changed in mode without
  treating host configuration text as authority.
- Registering a connection does not automatically grant every project in the
  `Volicord Runtime Home`.
- A connection can address only projects explicitly present in its Connection
  Projects records or selected through an owner-defined repository-root
  registration path.
- `connection.mode=workflow` is the default Agent Connection mode. It exposes
  agent workflow operations as well as read/project discovery operations. It
  does not expose user-only judgment recording.
- `connection.mode=read_only` exposes read/project discovery operations. It is
  not a workflow-write capability.
- `connection_id`, connection mode, connection intent, host configuration, or
  MCP server instructions are not OS permissions, host trust, secret isolation,
  filesystem ACLs, network policy, or user authority.

Storage record families and DDL belong to [Storage Records](storage-records.md)
and [Storage DDL](storage-ddl.md). Administrative creation, update,
verification, mode, export, and removal commands belong to
[Administrative CLI](admin-cli.md).

## Connection Intents

Connection intent describes where the host configuration is meant to be used. It
is not a security level and not an authority grant.

| Intent | Meaning | Must not infer |
|---|---|---|
| `personal` | User-owned host configuration for the current user's ordinary local flow. | It does not prove host trust, user identity, or access to every local project. |
| `shared` | Project-owned or project-shared host configuration stored only as an explicit integration file in a selected `Product Repository`. | It is not Volicord runtime state, and it does not authorize arbitrary product-file edits. |
| `global` | User-wide host configuration for a supported host, with project access still constrained by repository-root registration and Connection Projects. | It does not connect every repository and does not bypass project or host trust. |

The baseline directly managed host kinds are `codex` and `claude_code`.
Host-neutral MCP configuration export is a separate export flow. Exported
configuration can use internal registry state needed to start `volicord-mcp`,
but export is not a normal connection intent for direct host installation.

## Connection Projects

Connection Projects are the explicit registry relationship between an Agent
Connection and registered projects. User-facing commands select projects by
repository root or project name; storage still keeps internal `project_id`
values for referential integrity and provenance.

Membership fields:

- `connection_id`
- `project_id`
- creation timestamp
- a composite primary key over `connection_id` and `project_id`

Rules:

- Project membership does not bypass project status, path separation, storage
  executability, Agent Connection mode, or method-owned invocation requirements.
- Invalid current project registrations must be rejected by Connection Projects
  listing and access resolution instead of returned as connected project
  records.
- Inactive or otherwise execution-ineligible valid projects remain unavailable
  at execution time even if membership exists.
- Removing a Connection Project or disabling the Agent Connection must take
  effect without requiring host configuration to be rewritten.
- An Agent Connection with no connected projects may remain stored, and host
  configuration may also remain on disk. That stored state does not mean a new
  `volicord-mcp` process can start successfully.
- New MCP stdio startup and startup checks fail when the Agent Connection has
  zero connected projects.
- A `volicord-mcp` process that already started while at least one project was
  connected can observe later membership changes without host configuration
  being rewritten. After the last membership is removed, project discovery may
  report no available projects, and project-routed public tools cannot proceed
  normally.
- The Agent Connection is executable again only after a project is connected and
  the startup or per-call project checks can validate the required project
  state.

## Host Configuration Inventory

A stored Agent Connection is management inventory for Volicord-managed host
configuration and verification state. The host configuration file remains the
operational source of truth for the external host. The registry record is
management inventory and last-known verification state, not a substitute for
the host configuration.

Rules:

- The registry stores `host_kind`, `connection_intent`, internal server name,
  configuration target, mode, enabled state, managed fingerprint, and last
  verification status.
- Host trust, project trust, project MCP approval, OAuth, or any comparable
  host-controlled approval cannot be bypassed by Volicord.
- A host configuration write can be successful as a file operation while the
  result state remains `action_required` because the host has not yet trusted,
  approved, loaded, initialized, or exposed the server.
- `last_verified_status=complete` may be stored only for an administrative
  verification result that satisfied the operational gates owned by
  [Administrative CLI](admin-cli.md#agent-connection-result-states). A direct
  Volicord-spawned MCP handshake is not enough by itself.
- `last_verified_status=action_required` is the expected state when Volicord can
  manage or export configuration but a host-owned trust, approval, OAuth,
  reload, restart, command-link repair, or setup repair remains.
- Rejected, missing, changed, unavailable, and unknown host states are not
  `complete` Agent Connection states.
- Product Repository guidance, generated host instructions, and MCP server
  instructions can improve tool selection, but they are not enforcement
  mechanisms and cannot guarantee that a model will choose Volicord tools.

<a id="current-connection-context"></a>
## Current Connection Context

Current connection context is the local invocation context derived for one MCP
tool call. It is derived by the local adapter from the bound Agent Connection,
the selected project, the method being called, and adapter-owned invocation
facts. It is not a public request payload.

An MCP session is bound at adapter startup to exactly one internal
`connection_id`. Project selection is resolved from the Agent Connection's
registered repository roots and host-provided project context where available.
Public MCP tool input schemas must not expose internal request envelopes,
protocol metadata, `connection_id`, `project_id`, `actor_source`,
`operation_category`, or verification-basis fields as caller-owned inputs.

Project selection for public MCP method calls is deterministic:

1. Use the project already bound by the selected Agent Connection when exactly
   one available project is eligible.
2. When the connection can see a host-provided repository root, match that root
   to one connected registered project.
3. Otherwise reject the call as ambiguous or unavailable with actionable text
   that names the repository-root setup or connection command needed to repair
   the state.

When explicit selection is needed, the MCP-visible selector is the
`project_selector` value returned by `volicord.list_projects`, not a caller-owned
Core envelope field.

The adapter must not guess a project from folder names, arbitrary process
current working directory values, host labels, or the first row returned by
storage. Host roots may be used only as host-provided repository-root evidence;
they do not bypass registration, Connection Projects, or path-separation
checks.

Before a public tool call enters Core, the MCP adapter must verify:

- the Agent Connection exists and is enabled
- the selected project is explicitly connected to that Agent Connection
- the selected project is active and executable
- the connection mode allows the method's `operation_category`

Connection modes and operation categories:

| Agent Connection mode | Allowed operation categories through MCP | MCP-visible public method tools |
|---|---|---|
| `workflow` | `read`, `agent_workflow` | `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.prepare_write`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_judgment`, `volicord.check_close`, `volicord.close_task` |
| `read_only` | `read` | `volicord.status`, `volicord.check_close` |

The adapter-owned `volicord.list_projects` utility is visible in both
`workflow` and `read_only` modes. `volicord.check_close` is the read-only MCP
close-readiness tool. `volicord.close_task` is the workflow-only MCP mutation
tool and must not appear in `read_only` tool discovery.

`volicord.record_user_judgment` has `operation_category=user_only`. It is a
public Core API method for the User Channel path, but it is not exposed by Agent
Connections. The supported local user path for recording an authority-bearing
answer is the `volicord user` command group owned by
[Administrative CLI](admin-cli.md#user-channel-commands).

Internal actor shape, not a public API schema:

```yaml
InvocationContext:
  actor_source: local_user | system | agent_connection:<connection_id>
  operation_category: read | agent_workflow | user_only | admin_local
  verification_basis: string
  assurance_level: string
```

Baseline `assurance_level` means cooperative local provenance, not
cryptographic human identity. Authority-bearing user-judgment resolution
requires `actor_source=local_user`, `operation_category=user_only`, compatible
User Channel provenance, and method-owned compatibility. An Agent Connection
cannot gain user authority by submitting copied user text or generated guidance.

Conditions:

- A public API request has exactly one derived `InvocationContext`.
- Internal project selection is constrained by the Agent Connection's connected
  projects. It is not caller authority and cannot grant access to an unlisted,
  inactive, or invalid project.
- MCP-visible public tool schemas do not expose `actor_source`,
  `operation_category`, `connection_id`, `project_id`, request metadata, or
  protocol envelope fields. If raw MCP arguments include those fields, the
  adapter rejects the call before Core execution.
- Nested payloads such as `ArtifactInput` or `StagedArtifactHandle` do not add
  a second invocation context.
- Authority-provenance fields for resolved authority-bearing judgments come
  from the derived `InvocationContext`, not caller text, labels, answer
  payloads, copied refs, generated Markdown, or Product Repository guidance.
- Protected reads, mutations, and artifact operations can rely on an invocation
  only when the method owner accepts the derived context.

Agent may:

- preserve derived invocation context when displaying or passing owner-result
  context
- expose absent or incompatible context as unavailable, mismatched, stale, or
  insufficient Agent Connection state

Agent must not:

- submit `InvocationContext` as a request payload
- assert `verified=true`
- submit `actor_source=local_user` or `operation_category=user_only` from an
  Agent Connection to satisfy user authority
- submit arbitrary verification-basis text as public request authority
- fabricate staged artifact provenance
- use copied identifiers, generated Markdown, chat text, projection text, or
  agent memory as substitutes for current connection context

Owner links:

- Exact request envelopes and response shapes belong to
  [API Schema Core](api/schema-core.md), [API Methods](api/methods.md), and
  method owners.
- `operation_category` value names belong to
  [API Value Sets](api/schema-value-sets.md).
- `volicord-mcp` startup, connection binding, environment variables, stdio
  framing, startup validation, response wrapping, and shutdown belong to
  [MCP Transport](mcp-transport.md).

## User Channel And Agent Connections

Agent Connections are agent-facing connections. They are not the
`User Channel`, even when the model is relaying a user's words.

Conditions:

- The supported local CLI path for a human user to inspect pending judgments and
  record a selected Core-generated option is the `volicord user` command group
  owned by [Administrative CLI](admin-cli.md#user-channel-commands).
- Authority-bearing user-judgment resolution requires `actor_source=local_user`,
  `operation_category=user_only`, and compatible User Channel provenance.
- `actor_source=agent_connection:<connection_id>` cannot become `local_user`
  provenance by relaying text from a user.

Agent may:

- request a missing user-owned judgment when a method owner supports that path
- display pending judgment state and Core-generated options returned by owners
- route the human user to the supported `User Channel`

Agent must not:

- record an authority-bearing user decision from an Agent Connection
- treat a natural-language approval, chat reply, generated Markdown status, or
  rendered projection as User Channel provenance
- broaden one selected option into final acceptance, residual-risk acceptance,
  sensitive-action approval, scope acceptance, or another judgment kind
- create evidence sufficiency, acceptance, residual-risk acceptance, close
  readiness, or security authority from displayed judgment text

Owner links:

- [Core Model](core-model.md) owns the authority meaning of user-owned
  judgments, final acceptance, residual-risk acceptance, evidence, and close
  readiness.
- [Record-user-judgment method](api/method-record-user-judgment.md) owns public
  method behavior for resolving one pending judgment.
- [Projection and template display boundaries](projection-and-templates.md)
  owns generated display and projection authority boundaries.

## Agent Behavior Guidance

Agent behavior guidance has two layers:

- MCP server instructions are always supplied by the server during MCP
  initialization.
- Optional `Product Repository` guidance is installed only with explicit user
  authorization when an administrative command supports it.

Rules:

- MCP server instructions may describe cross-tool workflows, project selection
  rules, and limitations that apply across Volicord tools.
- Optional repository guidance may add a Volicord-managed block or
  host-specific rule file inside a `Product Repository` only under the boundary
  owned by
  [Runtime Boundaries](runtime-boundaries.md#explicit-integration-files-in-product-repositories).
- Guidance can improve tool selection, but it is not authority, access control,
  user judgment, security enforcement, or proof that a model will choose
  Volicord tools.

## Agent Context Transfer

Agent context transfer gives the agent enough owner context for the next action
without turning the packet into an authority record.

Conditions:

- Agent context should contain only owner results needed for the next action and
  current connection-context limits that affect that action.
- A context packet is support context, not Core state, storage state, evidence,
  acceptance, residual-risk acceptance, or close output.

Agent may:

- pass compact context containing the current Task summary, current scope,
  `state_version`, pending user-owned judgments, blockers, next safe action,
  evidence and artifact summaries, close-readiness and residual-risk summaries,
  owner-supported guarantee display, and source or limitation notes
- retrieve exact owner sections only when the next action needs them
- include both language versions for the same `doc_id` when bilingual
  maintenance requires semantic-parity review

Agent must not:

- inject full schemas, DDL, historical logs, artifact bodies, unrelated contract
  material, out-of-scope catalogs, exact template bodies, or both language
  versions for the same `doc_id` by default
- treat a stale or copied context packet as newer authority than the owner
  result or underlying record

Owner links:

- [Template Bodies](template-bodies.md) owns agent context packet wording.
- [Reference Index](README.md) routes exact owner sections.
- [Translation Policy](../maintain/translation-policy.md) owns bilingual
  semantic-parity review guidance.

## Fallback Boundary

Fallback display applies when current connection context or a required
connection mode is unavailable, mismatched, stale, or insufficient for the
requested operation.

Agent may:

- move to a suitable connection mode or a different connected project
- narrow the operation
- request the missing user-owned judgment
- continue outside Volicord only when the user explicitly chooses that mode

Agent must:

- expose the limitation in support or display text
- route machine-readable failure meanings to
  [API error codes](api/error-codes.md) and
  [API error details](api/error-details.md)
- route user-facing wording to [Template Bodies](template-bodies.md)

Agent must not:

- fabricate authority
- hide unavailable, mismatched, stale, or insufficient context states inside
  ordinary success text
- continue outside Volicord without the user's explicit choice
