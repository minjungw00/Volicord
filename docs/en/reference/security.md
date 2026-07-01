# Security reference

This document owns Volicord security guarantee wording, local connection assumptions, sensitive-action approval boundaries, `operation_category` security meaning, and explicit security non-guarantees.

## Owns / does not own

| This document owns | This document does not own |
|---|---|
| Supported guarantee semantics for `cooperative` and connection-observation `detective` wording. | API method request/response schemas or method-specific behavior. |
| The boundary that no baseline preventive guarantee is supported. | Storage record layout, artifact lifecycle detail, locks, hashes, or migrations. |
| Local connection assumptions, `operation_category` non-claims, and access-boundary non-claims. | Connector implementation or host-specific operating recipes. |
| Sensitive-action approval as a security-adjacent user-owned judgment boundary. | OS permissions, deployment controls, arbitrary-tool sandboxing, or host policy. |
| Non-authority rules for local files, generated displays, copied identifiers, chat text, and agent memory. | Runtime location definitions; see [Runtime Boundaries](runtime-boundaries.md). |
| Host trust, host approval, and guidance non-guarantees for Agent Connections. | Codex or Claude Code host configuration syntax; see [Administrative CLI](admin-cli.md). |

## Boundary summary

Volicord security wording describes record and policy boundaries inside documented Volicord paths. It does not describe an operating-system sandbox, malware scanner, network isolation layer, full host-trust enforcement system, or general host policy engine.

| Surface | Supported security meaning | Non-guarantee |
|---|---|---|
| `Volicord Runtime Home` | Storage/runtime owners define which Volicord operational records live there and how they are validated. | Runtime Home placement is not OS sandboxing, tamper-proof isolation, host trust, network isolation, malware scanning, or secret scanning. |
| `Product Repository` | Product files can be inspected as inputs, and compatible product-file writes can be governed by owner-defined Core, user-judgment, and `Write Check` paths. | Product files are not Volicord state, and Volicord does not provide arbitrary product-file edit permission, malware scanning, secret scanning, or global filesystem interception. |
| Agent Connections and host configuration | Agent Connections provide documented connection context, `actor_source` provenance, connection intent, mode, and Connection Projects allowlists when the current invocation matches the registered connection. | Connection configuration is not OS permission, host trust, user identity, or proof that an external host loaded or exposed `volicord mcp --stdio`. |
| `volicord mcp --stdio` | The adapter routes MCP calls through Agent Connection checks, Runtime Home state, Core, and Store. | The process does not itself grant arbitrary product-file edit authority, record authority-bearing user judgments, enforce host trust, block commands, block networks, or isolate tools. |
| `volicord` CLI | Administrative commands manage setup, registry state, and supported host-integration state. | The CLI is not a public API security boundary, host trust controller, OS permission mechanism, or blanket write approval. |

## Supported security guarantees

<a id="honest-guarantee-display"></a>
Volicord may describe a guarantee only when [Scope](scope.md) and this security owner both support the guarantee level. Guarantee display is derived from the current `operation_category`, current Agent Connection or `User Channel` provenance where relevant, recorded observation facts, and supported baseline scope. If the claim depends on an observed connection result, the relevant observation must be recorded for the named connection or evidence source and observed scope.

Guarantee display must stay scoped to the connection, operation, or evidence observation that justifies it. A cooperative Run report or cooperative `agent_report` observation is not a `detective` or externally observed fact unless a separate supported observation or external result is recorded and cited.

The supported guarantee display labels are `cooperative` and `detective`; the value names are owned by [API Value Sets](api/schema-value-sets.md).

### `cooperative`

`cooperative` is the default baseline security guarantee.

Conditions:
- The caller, Agent Connection, User Channel, local admin path, or connector follows the documented Volicord paths.
- The claim stays inside documented Core, API, storage, runtime, and user-judgment boundaries.

May claim:
- Volicord records, write compatibility, evidence summaries, user-owned judgments, and close-readiness results are governed by their owner contracts.
- Volicord may reject, block, or require a focused user-owned judgment when the relevant owner contract says the current state is not compatible.

Must not claim:
- `cooperative` blocks arbitrary tool behavior, host commands, network access, secret access, or product-file edits outside Volicord-owned paths.
- `cooperative` provides OS permission enforcement, sandboxing, tamper-proof isolation, or full security isolation.

### Connection-observation `detective`

`detective` is supported only as a limited, observation-backed claim.

Conditions:
- The claim names the Agent Connection, User Channel, external evidence source, or other owner-supported observation source.
- The relevant `operation_category` and owner-supported observation path support the claim.
- The relevant observation or enforcement check has passed and produced supported facts for the observed operation.
- The observed scope is documented.
- Changed-path wording is used only when the recorded observation reports changed paths for the relevant operation.

May claim:
- The checked observation source supports limited observation or mismatch reporting inside the documented observed scope.
- Observed changed paths support a limited changed-path detection claim when the reporting condition is met.
- Missing or insufficient observation support routes to documented error behavior when the relevant owner defines that behavior.

Must not claim:
- A copied `connection_id`, `operation_category`, connector description, `Projection`, generated display, chat message, or agent memory proves capability or observation.
- Connection declarations alone raise a guarantee above `cooperative`.
- A cooperative Run report, cooperative `agent_report`, or unverified claim raises a display above `cooperative` without a supporting observed fact.
- `detective` wording becomes prevention, sandboxing, OS permission enforcement, full monitoring, or tamper-proof storage.

### Preventive guarantees

The baseline contract does not define a supported preventive guarantee.

Must not claim:
- Volicord prevents arbitrary tool execution.
- Volicord provides universal pre-tool blocking.
- Volicord observes or blocks command, network, or secret access by default.
- Volicord provides OS sandboxing, host permission enforcement, or stronger isolation.

## Sensitive-action approval boundary

Sensitive-action approval is a user-owned judgment for a named sensitive step inside a bounded `SensitiveActionScope`.

May claim:
- Sensitive-action approval can be required before write compatibility, Run recording, or close when the relevant owner defines that requirement.
- The approved sensitive step remains scoped to the prompt, `SensitiveActionScope`, affected object, and visible consequence that the user was asked to judge.

Must not claim:
- Sensitive-action approval is `Write Check`, `WriteCheckAttemptScope`, OS permission, shell permission, command approval, deployment approval, final acceptance, residual-risk acceptance, or product correctness.
- Sensitive-action approval authorizes product-file writes, commands, hosts, networks, secrets, destructive operations, or unbounded activity.
- Broad approval substitutes for a required sensitive-action approval, final acceptance, residual-risk acceptance, scope decision, or `Write Check`.

Owner links:
- [Core Model](core-model.md) owns user-owned judgment and non-substitution rules.
- [API Judgment Schemas](api/schema-judgment.md) owns `SensitiveActionScope` shape.
- [Prepare-write method](api/method-prepare-write.md) owns `volicord.prepare_write` behavior.

## Local connection assumptions

Volicord security claims assume local actors use the documented Volicord contracts for Volicord state, records, artifacts, write compatibility, and user-owned judgments.

May claim:
- Local product files can be inputs to Volicord checks or user-owned judgments.
- Local runtime data location can be defined by storage/runtime owners.
- Agent Connections can provide `actor_source=agent_connection:<connection_id>` provenance when [Agent Connection Reference](agent-connection.md), method owners, and this security owner allow the claim. The `connection_id` segment is the process-binding/provenance spelling in that string, not a user-facing authority token or storage-field name.
- The `User Channel` can provide `actor_source=local_user` provenance for authority-bearing user judgments when Core and method owners require it.
- Connection Projects define the explicit `project_internal_id` allowlist for an Agent Connection. User-facing commands select projects by repository root, project name, alias, or a `project_selector` returned by Volicord.
- `operation_category` classifies an operation as `read`, `agent_workflow`, `user_only`, or `admin_local`.
- Baseline actor provenance is cooperative local provenance, not cryptographic human identity.

Must not claim:
- Local filesystem access proves Volicord authority.
- A local path, directory name, copied identifier, displayed identifier, or rendered text is a security token.
- Direct local modification outside those documented Volicord contracts creates valid Volicord records, evidence, acceptance, residual-risk acceptance, `Write Check`, or artifact authority.
- `Volicord Runtime Home` is automatically an OS security boundary, sandbox, or isolation layer.
- A caller-supplied `verified` flag, requested `operation_category`, copied `actor_source`, public request field, or environment variable supplies trusted provenance.
- `actor_source=agent_connection:<connection_id>` proves human identity or supplies user authority.
- Host configuration installation proves that a host has trusted, approved, loaded, initialized, or exposed the MCP server.
- Repository guidance, MCP server instructions, or host rule files enforce model behavior or guarantee that an agent will choose Volicord tools.

## Authority boundaries

### Volicord records

Volicord records carry authority only through the owner contracts that create, validate, or update them.

Must not claim:
- Local file contents are tamper-proof because they describe or store Volicord data.
- Product text, generated text, or copied record-looking text directly mutates Volicord records.

### `Product Repository` files

[Runtime Boundaries](runtime-boundaries.md) defines `Product Repository` as the product-file boundary. This section owns only security claims and non-claims about that boundary.

May claim:
- Product files can be inspected as inputs.
- Compatible product-file writes can be governed by current scope, current Change Unit compatibility, user-owned judgments, and `Write Check` when the write owner requires them.

Must not claim:
- Product files are Volicord state.
- Product files prove Volicord authority.
- Product files become Volicord records because Volicord metadata is nearby.

### `Volicord Runtime Home`

For security wording, treat `Volicord Runtime Home` as the runtime/storage-owned operational data location.

Runtime location is defined by [Runtime Boundaries](runtime-boundaries.md). This section owns only the security non-claims for that location.

May claim:
- Storage/runtime owners define which Volicord operational data belongs there and how it is validated.

Must not claim:
- `Volicord Runtime Home` is the `Product Repository`.
- `Volicord Runtime Home` is automatically a security boundary.
- Placing data under `Volicord Runtime Home` proves security authority or isolation.

### Agent Connections, User Channel, and operation categories

Connection identity, user-channel provenance, and operation categories limit what may be claimed.

May claim:
- Internal connection identity, connection intent, `connection.mode`, Connection Projects, `operation_category`, and `actor_source` can be used according to the runtime, Core, method, and security owners after the current invocation matches the documented connection context.
- `actor_source` can supply durable provenance only when the Core and method owners accept the value for the current authority-resolution operation.
- `actor_source=local_user` through the `User Channel` is required for authority-bearing user judgments.

Must not claim:
- `connection_id` alone is an authority token.
- A copied internal connection identifier proves capability or user authority.
- `connection.mode=workflow` is OS permission or broad authority.
- A `personal`, `shared`, or `global` connection intent is OS permission, host trust, or broad authority.
- `operation_category` is OS permission, host trust, or broad authority.
- `actor_source` copied from text is a caller authority token.
- Environment-controlled labels, public request fields, or arbitrary caller text are trusted authority, audit facts, or verification-basis inputs.

### Host trust and guidance

Host trust and approval decisions belong to the external host and the user. Volicord can install supported configuration and report whether further user action appears required, but it does not control the host trust decision.

May claim:
- managed host configuration state verification can distinguish `complete` from `action_required` and `failed` when the administrative CLI can observe the required checks.
- `action_required` can name installation-profile repair, command-link repair, host trust, approval, restart, reload, or comparable user-controlled actions when those actions are the remaining observable blocker.
- MCP server instructions and optional repository guidance can describe how an agent should select projects and tools.

Must not claim:
- Installing Codex or Claude Code configuration bypasses project trust, project MCP approval, OAuth, restart, reload, or comparable host-controlled actions.
- `action_required` is a failed installation when configuration was installed but the host still requires user-controlled trust or approval.
- Agent instructions, `AGENTS.md` blocks, `CLAUDE.md`, `.claude/rules/` files, or MCP server instructions are access control, security enforcement, user judgment, `Write Check`, or proof that a model will follow them.

### Generated displays and text

Generated displays, rendered templates, chat text, connector prose, and agent memory can help readers understand source records.

Must not claim:
- A rendered display, `Projection`, status card, template output, chat message, connector description, or agent memory is a new authority source.
- Displayed `ArtifactRef`, `UserJudgment`, `Write Check`, or `connection_id` text creates the authority named by those identifiers.

## Explicit non-guarantees

### Operating system and isolation

Volicord does not guarantee:

- OS-level sandboxing.
- OS permission enforcement.
- Network isolation.
- Tamper-proof isolation.
- Full security isolation.
- Isolation between local users, processes, tools, or hosts.

### Monitoring and prevention

Volicord does not guarantee:

- Full filesystem monitoring.
- Command monitoring by default.
- Network monitoring by default.
- Network blocking by default.
- Secret-access monitoring by default.
- Malware scanning.
- Secret scanning.
- Universal pre-tool blocking.
- Prevention of malicious agent behavior outside Volicord-owned paths.

### Host trust and integration

Volicord does not guarantee:

- Full host trust enforcement.
- That an external host trusted, approved, loaded, initialized, or exposed `volicord mcp --stdio`.
- That host instructions, repository guidance, or MCP server instructions force model or tool behavior.

### Storage and artifact authority

Volicord does not guarantee:

- Tamper-proof storage.
- Native artifact capture from Agent Connections as a baseline guarantee.
- Artifact authority from displayed identifiers alone.
- Validation or acceptance from copied artifact, run, evidence, or judgment text.

### Broad authority inference

Volicord does not allow readers or agents to infer authority from:

- Broad approval.
- Local path names.
- Copied `connection_id` process-binding values.
- Displayed `ArtifactRef` values.
- Rendered `Projection` output.
- `Product Repository` text.
- Connector prose.
- Chat text or agent memory.

## Related owners

- [Scope](scope.md): baseline inclusion, exclusions, and supported guarantee boundary.
- [Agent Connection Reference](agent-connection.md): Agent Connection, Connection Projects, current connection context, and Agent Connection/User Channel authority boundaries.
- [Runtime Boundaries](runtime-boundaries.md): User Channel location, Volicord source repository/installation files, executable processes, `Product Repository`, `Volicord Runtime Home`, and external MCP host configuration boundaries.
- [API Value Sets](api/schema-value-sets.md): `GuaranteeDisplay.level`, `operation_category`, and other value names.
- [API error routing](api/error-routing.md): public error routing.
- [Core Model](core-model.md): user-owned judgment, `Write Check`, acceptance, residual risk, and non-substitution rules.
- [API Judgment Schemas](api/schema-judgment.md): `SensitiveActionScope` and user-owned judgment schema shapes.
- [Storage Effects](storage-effects.md), [Storage Records](storage-records.md), and [Artifact Storage](storage-artifacts.md): storage effects, record layout, and artifact authority details.
