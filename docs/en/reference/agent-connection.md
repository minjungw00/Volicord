# Agent Connection Reference

Workflow-mode connections may dispatch the supported Agent-owned methods. The
returned tagged workflow action catalog admits Task-state-bound mutations, and
each admitted method requires its exact current method-specific form before
Core. `required_action` is selected by progression state, while close blockers
remain a separate close-readiness projection.

This document defines the current `host_kind=codex` Agent Connection contract. It owns the
exact `host_kind=codex` Record connection surface, the canonical connection
verification report, managed configuration ownership, integration revisions,
and the validated operational-session boundary between the Codex adapter and
Core.

<a id="owns-and-does-not-own"></a>

## Owns / Does Not Own

This document owns:

- the accepted `host_kind`, integration profile, connection intents, transport,
  user-action delivery path, modes, and platform-environment values;
- the canonical `ConnectionVerificationReport`, its closed status values,
  deterministic aggregation, strict encoding, and missing-report projection;
- Connection and project integration revisions;
- authoritative managed-host runtime, negotiated MCP profile, and project
  session ownership;
- `ValidatedAgentSession` and the checks required before Core consumes it; and
- Codex adapter discovery, installation, verification, repair, and uninstall
  responsibilities.

This document does not own:

- stdio framing, MCP initialization, tool routing, or shutdown; see
  [MCP Transport](mcp-transport.md);
- administrative command syntax, output, or exit codes; see
  [Administrative CLI](admin-cli.md);
- exact database tables or storage effects; see
  [Storage Records](storage-records.md) and
  [Storage Effects](storage-effects.md);
- ordinary build, package, platform, and release validation; see
  [Validation](../maintain/validation.md);
- operating-system topology and filesystem prerequisites; see
  [System Requirements](system-requirements.md);
- Core `UserActionRequest` and `UserActionResolution` schemas; see
  [API User Action Schemas](api/schema-user-action.md); or
- product-wide failure-category and security meanings; see
  [Failure Model](failure-model.md) and [Security](security.md).

<a id="surface-stability"></a>

## Surface Stability

Labels follow the canonical vocabulary in
[Documentation Policy](../maintain/documentation-policy.md#surface-stability-labels).

| Surface | Stability | Contract |
|---|---|---|
| Supported value sets, `ConnectionVerificationReport`, integration revisions, authoritative operational sessions, and `ValidatedAgentSession` | `stable` | These are exact boundary contracts. |
| Codex discovery, managed installation, verification, repair, uninstall, and drift result semantics | `stable` | Implementations may change without changing the observable contract. |
| Adapter modules, filesystem helpers, the hidden launcher, and Store lease/query helpers | `internal` | They preserve the stable boundary but are not public surfaces. |
| Human-readable verification guidance and client/host version observations | `diagnostic` | Machine-readable categories, reasons, and typed fields remain authoritative. |

<a id="supported-surface"></a>

## Supported Surface

Volicord accepts exactly this Agent Connection surface:

| Dimension | Exact value |
|---|---|
| Host | `host_kind=codex` |
| Integration profile | `integration_profile=record` |
| Connection intent | `personal` or `shared` |
| Connection mode | `read_only` or `workflow` |
| Transport | Volicord-managed stdio MCP entered through the hidden host launcher; public manual stdio remains `volicord mcp serve` |
| Production MCP revisions | `2024-10-07`, `2024-11-05`, `2025-03-26`, `2025-06-18`, or `2025-11-25` |
| User-owned action delivery | CLI inbox |
| Platform environment | `linux`, `macos`, `native_windows`, or `wsl2` |

A `personal` connection installs user-owned local Codex configuration. A
`shared` connection installs project-owned Codex configuration inside the
selected `Product Repository`. A personal managed launch identifies one
registered Connection; the Connection's allowed projects remain its
authoritative Store-owned memberships. A shared managed launch resolves its
Connection and project through repository discovery.

The Connection owns its host kind, scope, mode, managed-configuration
fingerprint, project membership, immutable Store-owned integration-instance ID,
and Store-owned integration generation. Connection and project integration
revisions derived from those current owner facts are local lifecycle and
correlation coordinates.

A genuinely new Agent Connection defaults to `workflow`; a new Connection
created by `volicord connection add --read-only` instead starts in
`read_only`. Setup replay and repair use the persisted mode of a matching
current Agent Connection exactly. In particular, omitting `--read-only` from
add is not a `workflow` transition request, and supplying it for an established
`workflow` Connection does not authorize an implicit transition. Setup never
infers mode from the Record profile. `volicord connection mode` is the only
command that changes an established mode and increments the integration
generation.

An Agent Connection is a stored local integration record in the
`Volicord Runtime Home`. It does not grant operating-system permission,
establish user identity, or prove that Codex loaded the managed entry. One
managed stdio MCP process is bound to one current Agent Connection.

User-owned actions are delivered through the CLI inbox. An MCP agent may
request an owner-defined action, but it cannot act as the local user channel or
resolve the action on the user's behalf.

For the production revision set, an exact requested `protocolVersion` selects
the same session profile. Every other identifier is rejected as unsupported
without substituting another profile. Support is never inferred from lexical
or date ordering, numeric or date parsing, ranges, prefixes, or package
versions, and the user cannot configure the supported set. Exact parameter,
capability, and response behavior is owned by
[MCP Transport](mcp-transport.md#protocol-revision-negotiation).

<a id="managed-mcp-launch-contract"></a>

## Managed MCP Launch Contract

One typed managed MCP launch contract is the canonical source for the hidden
launcher command and arguments, static and forwarded Runtime Home bindings, the
personal/shared distinction, strict launch-shape validation, canonical
projection, and deterministic managed-fingerprint inputs. Managed provenance
begins only with successful one-time launch-lease consumption.

A personal connection requires the selected canonical absolute Runtime Home
and selected absolute `volicord` executable. It invokes the hidden host-owned
launcher, stores only the Runtime Home as process configuration, and forwards
no parent-environment name:

```toml
[mcp_servers.volicord]
command = "/absolute/path/to/volicord"
args = ["_host-launch", "codex", "--connection", "<connection_id>"]

[mcp_servers.volicord.env]
VOLICORD_HOME = "/absolute/runtime/home"
```

A personal entry carries no project selector. Its arguments contain no
`--project`, and its static environment contains no project marker. The Agent
Connection's authoritative Product Repository associations remain Store-owned
Connection Project memberships rather than process-launch state.

A shared connection contains only the clone-portable repository-discovery
launch. It uses `volicord` from `PATH`, forwards only `VOLICORD_HOME`, and has
no static environment table:

```toml
[mcp_servers.volicord]
command = "volicord"
args = ["_host-launch", "codex", "--discover-repository"]
env_vars = ["VOLICORD_HOME"]
```

The shared launch contains no absolute executable, Runtime Home, Connection ID,
project ID, or other machine-local lifecycle coordinate. A personal entry with
a project argument or project environment marker, a static/forwarded
environment collision, a blank or duplicate forwarded name, an incomplete
personal binding, or a mixed personal/shared argument or environment shape is
invalid.

Generated Codex configuration is an adapter projection of this contract. The
configuration contains no launch lease, nonce, reusable secret, or raw operating-
system handle. CLI verification derives its
public preflight and manual stdio probe commands from the same binding facts;
neither probe is a managed-host launch.

The Codex adapter parses the current TOML shape and validates a managed entry by
reconstructing this same typed contract. Unknown launch keys, malformed values,
and noncanonical shapes are drift rather than a second accepted form. The
adapter preserves only a valid `tools.<known-tool>.approval_mode` overlay and
excludes it from launch identity. The managed fingerprint covers the canonical
launch projection together with host kind, scope, and server name. Formatting
differences do not change it; a launch-semantic difference does.

The hidden launcher strictly reloads the current canonical entry, verifies the
enabled Connection and exact integration revision, and verifies that the
entry's fingerprint equals the current stored managed fingerprint. It then
creates one short-lived Registry launch lease and transitions in memory to the
stdio adapter. The lease ID is not written to Codex configuration, process
arguments, logs, or a public environment variable. MCP bootstrap atomically
consumes the lease and creates the `managed_host` runtime session. Consumption
requires the exact Connection, `codex` host kind, integration revision, and
managed fingerprint captured by the launcher. A lease is single-use; replay,
expiry, mismatch, and cancellation fail closed, and a normal launcher failure
terminalizes any still-unused lease.

<a id="connection-verification-report"></a>

## `ConnectionVerificationReport`

One report is the canonical serialized connection and activation state:

```schema
ConnectionVerificationReport:
  status: complete | action_required | failed
  activation_state: configured | host_reload_required |
    hook_review_required_or_unknown | mcp_observation_required |
    guard_verification_required | complete | failed
  hook_activation_state: unknown | review_required_by_setup |
    effective_by_observation | managed_by_policy |
    bypassed_for_invocation | disabled
  checked_at: UtcTimestamp
  checks: ConnectionCheck[]
  root_cause_ids: DiagnosticFindingId[]
  activation_plan: IntegrationActivationPlan

ConnectionCheck:
  id: ConnectionCheckKind
  status: passed | pending | failed | blocked | not_applicable
  depends_on: ConnectionCheckKind[]
  cause_finding_ids: DiagnosticFindingId[]
  code?: string
  summary: string
  details?: object
  observed_at?: UtcTimestamp

McpPreflightEvidence:
  configuration: passed | failed
  registry_read: passed | failed
  project_reads: McpProjectReadEvidence[]
  schema_validation: passed | failed
  protocol_profiles: passed | failed
  host_contracts: passed | failed
  writeability:
    status: not_checked
    requires: connection_verify
  side_effects: []

McpActiveVerificationEvidence:
  registry_write: passed | failed
  project_writes: McpProjectWriteEvidence[]
  protocol_conformance: McpRevisionConformance[]
  host_compatibility: McpHostCompatibilityEvidence[]
  observed_at: UtcTimestamp
  source: connection_verify
  side_effects: McpSideEffectKind[]

IntegrationActivationPlan:
  state: IntegrationActivationState
  required_steps: ActivationStep[]
  optional_diagnostics: ActivationStep[]

ActivationStep:
  id: ActivationStepId
  initiator: user | host | volicord | agent
  executor: user | host | volicord | agent
  execution_channel: cli | codex_ui | codex_chat | mcp_tool
  prerequisites: ActivationStepId[]
  completes_checks: ConnectionCheckKind[]
  root_finding_ids: DiagnosticFindingId[]
  instruction: string
  diagnostic_only: boolean
  agent_sequence: AgentSequenceStep[]

AgentSequenceStep:
  tool: AgentToolId
  condition: always | workflow_awaiting_probe |
    workflow_awaiting_observation
```

`status`, `activation_state`, `hook_activation_state`, `checked_at`, `checks`,
`root_cause_ids`, `activation_plan`, and every plan or step member shown above
are required. Optional check members `code`, `details`, and `observed_at` are
omitted when absent rather than serialized as null. Unknown members, duplicate
JSON keys, duplicate check kinds, duplicate activation-step IDs, explicit null
for an optional member, and unknown enum values are invalid. Non-null check
codes are 1 through 128 ASCII bytes and match `[a-z][a-z0-9_]*`. `summary` and
`instruction` values are 1 through 4,096 UTF-8 bytes and contain no NUL. A
non-null `details` value is a JSON object whose serialized form is at most 16
KiB. A check or activation step contains at most 16 dependency edges and 32
root-finding references. A report contains at most 64 checks and an activation
plan contains at most 32 total required and optional steps. The serialized
report is at most 64 KiB.

`checked_at` records the one logical time at which the report was evaluated.
The CLI projects that same evaluation instant as `generated_at` in its current
diagnostic report. A check's optional `observed_at` instead records when the
decisive persisted evidence for that check was observed or reached its current
state. It is not filled from report evaluation time. A check with no current
decisive evidence omits `observed_at`.

Checks are sorted by the stable snake-case spelling of `ConnectionCheckKind` in
ascending UTF-8 byte order. Activation steps use deterministic topological
ordering over `prerequisites`, with the current workflow order resolving
independent steps; serialized ID spelling is not the ordering rule. Strict
decoding rejects a noncanonical topological order rather than silently
normalizing it. Plan construction rejects cycles, unknown prerequisites,
duplicate step IDs, a nested agent tool exposed at top level, and a
diagnostic-only step in `required_steps`.

`ConnectionCheckKind` is the closed current-product vocabulary:
`connection_removal`, `diagnostic_lookup`, `guard_files`, `ambient_hook_coverage`,
`guard_observation`, `correlated_guard_verification`, `hook_source_activation`,
`host_executable`, `host_reload`, `host_session`, `managed_capability_proof`,
`managed_config`, `managed_session_health`, `mcp_server`, `mode_transition`,
`process_startup`, `project_trust`, `required_tools`,
`runtime_session_lookup`, `setup_plan`, `tool_round_trip`, and
`verification_not_run`.
Operational verification uses the applicable checks in
the table below. Missing-report and administrative command planning use the
remaining named kinds. `diagnostic_lookup` and `runtime_session_lookup` are
used only by their bounded administrative diagnostic operations; arbitrary
adapter-defined check IDs are not accepted.

The `mcp_server` check details contain sibling `preflight` and
`last_active_verification` members. `preflight.evidence` is one immutable
`McpPreflightEvidence` created only from the read-only preflight report.
`last_active_verification` is either null when no active run exists or one
`McpActiveVerificationEvidence`; it is never stored inside, merged into, or
projected as a mutation of preflight evidence. Preflight writeability is always
`not_checked`, requires `connection_verify`, and has exactly
`side_effects=[]`.

Active verification records Registry and per-project write results separately,
records each disposable protocol-revision and host-compatibility result, and
identifies its own `observed_at` and `source=connection_verify`. Its closed MCP
side-effect values are `rollback_only_registry_write_probe`,
`rollback_only_project_write_probe`, `disposable_protocol_conformance`, and
`disposable_host_compatibility`. The operation selects the active source and
the separate evidence shape directly; schema-version integers, numeric host
versions, and combined-shape decoding do not select them. Human, verbose, and
JSON projections preserve this distinction. Concise Connection output strictly
decodes a present `McpActiveVerificationEvidence` into one latest-snapshot
active-verification conclusion and one Store-writeability conclusion. With no
active evidence it says `Last active verification: not run` and
`Last verified storage writeability: not checked`, with no evidence-time or
source line. With evidence it shows `Last active verification: passed|failed`,
the exact persisted `observed_at`, the humanized source `connection verify`,
and `Last verified storage writeability: passed|failed`. Registry
writeability and every included project writeability result must pass for Store
writeability to pass. Those results plus every included protocol-conformance
and host-compatibility probe must pass for active verification to pass.
Malformed present evidence is an error. Concise output omits IDs, individual
Store results, protocol matrices, and host fixtures. Verbose output derives
Connection-owned typed human projections for active verification with Store
writeability, protocol conformance, host compatibility, and Hook path safety
from their strict typed inputs. It always retains aggregate results.
Homogeneous success is compact: Store reports the Registry-and-project count
without successful IDs, protocol revisions use canonical production order without per-stage
success fields, host profiles use one row each, and a verified Hook assessment
reports dimension and source counts without paths. A failed, incomplete,
unavailable, mixed, corrupt, or contradictory component expands its exact IDs,
lifecycle stages, diagnostic facts, and bounded nonverified Hook evidence.
JSON remains exhaustive and never labels an active write result as a preflight
effect. The evidence time and source expose provenance without a fresh/stale
classification, universal validity duration, or automatic expiration.

Selected status and Connection list project the same typed human vocabulary:
aggregate status is `ready`, `action required`, or `failed`; integration and
hook activation use intentional space-separated phrases; and check counts use
`passed`, `blocked`, `pending`, `failed`, then `not applicable` when the detail
level includes it. Stable underscore spellings remain the machine-readable
JSON and stored values.

`ActivationStepId` is the closed current-product vocabulary:
`reload_codex`, `review_project_hooks`, `request_integration_verification`,
`read_connection_status`, `run_optional_active_diagnostics`,
`repair_hook_contract`, and `repair_managed_configuration`.
`IntegrationActivationPlan` in the verification report is the one activation
plan owner; host plans and host effects do not carry a second step list.

An activation step expresses semantic work through its stable ID, distinct
initiator and executor, execution channel, step prerequisites, intended checks,
root findings, bounded instruction, diagnostic class, and optional nested agent
sequence. It contains no executable shell text. JSON consumers use these
members as report facts rather than executing instruction content.
When complete current selector coordinates are available, the human renderer
constructs executable follow-up guidance from its typed current CLI invocation
context. That renderer-owned guidance is not copied into step JSON or the
persisted report.

### Hook activation evidence

`HookActivationState` reports only evidence Volicord can name:

| Variant | Wire value | Required evidence |
|---|---|---|
| `Unknown` | `unknown` | No authoritative host state and no compatible event for the current hook definition. Absence is not a trust judgment. |
| `ReviewRequiredBySetup` | `review_required_by_setup` | Setup created or changed the project-local hook definition. Host review must happen again even if an older definition ran. |
| `EffectiveByObservation` | `effective_by_observation` | A compatible Guard event exists for the current installed definition, policy hash, integration revision, and installation boundary. |
| `ManagedByPolicy` | `managed_by_policy` | The host explicitly reports that current hook activation is policy-managed. |
| `BypassedForInvocation` | `bypassed_for_invocation` | The host explicitly reports a one-invocation bypass. This is not durable activation. |
| `Disabled` | `disabled` | The host explicitly reports the hook source disabled. |

The precedence is explicit disabled evidence, a setup definition change,
policy management, invocation bypass, current-definition observation, then
`unknown`. There is deliberately no `trusted` hook state. Project or
configuration trust remains a separate host/user-owned concern represented by
`project_trust`; it neither proves nor is inferred from project-local hook
activation.

Guard observations are current only when they occurred at or after the current
installation definition boundary and match its policy hash, integration
revision, and installation. Reapplying byte-identical definition content
preserves that boundary. Changing any managed definition content advances the
boundary, so an older event cannot prove the new definition effective. Reports
expose current hook-definition content hashes separately from ambient
prompt/pre/post phase details.

### Connection activation progression

`IntegrationActivationState` has these exact variants and stable wire values:

| Variant | Wire value | Meaning |
|---|---|---|
| `Configured` | `configured` | Managed configuration exists, but no later activation stage is decisive yet. |
| `HostReloadRequired` | `host_reload_required` | The managed host must reload the current configuration. |
| `HookReviewRequiredOrUnknown` | `hook_review_required_or_unknown` | Current hook review is required or hook-source activation remains unknown. |
| `McpObservationRequired` | `mcp_observation_required` | Current managed-host session and capability evidence is incomplete. |
| `GuardVerificationRequired` | `guard_verification_required` | The correlated first-party Guard verification is incomplete. |
| `Complete` | `complete` | Every current activation check is complete. |
| `Failed` | `failed` | A required activation or diagnostic check failed. |

The state is derived in this order:

1. any failed or blocked required check produces `failed`;
2. incomplete `managed_config` produces `configured`;
3. incomplete `host_reload` produces `host_reload_required`;
4. a hook state other than `effective_by_observation` or `managed_by_policy`
   produces `hook_review_required_or_unknown`;
5. incomplete `managed_session_health` or `managed_capability_proof` produces
   `mcp_observation_required`;
6. incomplete `correlated_guard_verification` produces
   `guard_verification_required`;
7. otherwise the state is `complete`.

Every check in the report is required for that report. The top-level status is
derived and cannot disagree with the checks:

1. any `blocked` check or non-recoverable `failed` check produces
   `status=failed`;
2. otherwise any `pending` check or recoverable failed
   `correlated_guard_verification` produces `status=action_required`;
3. otherwise the report contains only `passed` and `not_applicable` checks and
   produces `status=complete`.

The five check statuses have exactly these meanings:

- `passed`: the check completed successfully;
- `pending`: its required external observation or user-triggered event has not
  occurred and no failed prerequisite currently prevents it;
- `failed`: the check itself observed a failure;
- `blocked`: the check could not run or be observed because a prerequisite
  check failed; and
- `not_applicable`: the check does not apply to this Connection or profile.

`depends_on` is the canonical explicit dependency edge set for the check kind.
Operational verification uses these chains:

```text
managed_config -> host_reload -> hook_source_activation
host_reload -> managed_session_health -> managed_capability_proof
hook_source_activation -> ambient_hook_coverage -> correlated_guard_verification
```

`project_trust` is evaluated independently. It can explain a host-owned
prerequisite but is never folded into `hook_source_activation`.
`managed_session_health` uses the fixed `latest_managed_attempt` role: the newest
current-revision `managed_host` runtime. `managed_capability_proof` uses the
distinct `latest_managed_capability_proof` role: the newest single runtime that completed
initialize, the initialized notification, `tools/list`, required-tool
validation, and the designated safe tool call. No check combines milestones
from different sessions. A terminal latest attempt fails current session
health; an older complete proof remains visible under its own role but cannot
hide that failure. Actual MCP peer `clientInfo` and the separately probed PATH
executable/version also remain distinct report facts.

Only `failed` and `blocked` checks may carry `cause_finding_ids`. A `blocked`
check carries the canonical sorted union of the independent root-finding IDs on
its failed or blocked prerequisites. `root_cause_ids` is the sorted,
deduplicated union for the complete check graph. A blocked check whose causes
do not match its failed prerequisites, a dependency cycle, or noncanonical
dependency edges makes the report invalid.

The current Codex connection report contains these operational checks:

| Check ID | Current role |
|---|---|
| `managed_config` | The selected target contains the canonical managed entry. |
| `host_reload` | A current-revision managed-host attempt shows that the host loaded current configuration. |
| `hook_source_activation` | Carries the typed hook activation state and current definition hashes; ambient phase observations stay in details. |
| `managed_session_health` | Reports `latest_managed_attempt`, including a terminal protocol failure. |
| `managed_capability_proof` | Reports the distinct `latest_managed_capability_proof` and its same-session capability milestones. |
| `ambient_hook_coverage` | Reports only current-definition execution, managed-file integrity, and general configured prompt/pre/post phase coverage. |
| `correlated_guard_verification` | Reports the latest current verification attempt and the latest completed current proof as distinct evidence. |
| `project_trust` | Reports separately available project/configuration trust applicability without changing hook state. |
| `host_executable` | Reports the separately probed PATH executable and version as a diagnostic. |
| `mcp_server` | Reports CLI-owned MCP preflight/self-test facts as diagnostics only. |

CLI MCP preflight is read-only and creates no runtime session. The manual
stdio self-test creates `session_source=manual_cli` only in a disposable
per-command Runtime Home; neither preflight nor that disposable evidence
satisfies `managed_session_health`, `managed_capability_proof`, or
`correlated_guard_verification`. MCP resources and resource templates likewise do not
prove tool exposure. Guard uses `ambient_hook_coverage` and
`correlated_guard_verification` as focused top-level activation checks.
The strict Guard manifest owns the current policy hash, integration revision,
typed runtime commands, complete Volicord-managed artifact expectations, and
required hook phases. It also names the exact `host_contract_profile` and
deterministic `host_contract_digest`; the current values select
`codex-command-hooks` and its reviewed contract identity. Policy and runtime
commands are distinct projections of one typed invocation. Audit rejects a
manifest whose profile or digest differs from that exact selection, compares
every managed artifact with its canonical current expectation, and requires
compatible current events for every required phase.

The managed-file audit also carries one bounded Hook path-safety assessment for
overall safety, CWD independence, and subdirectory invocation. Exact current
artifacts under the current owner binding establish `verified`; an affirmative
current contract violation establishes `failed`; insufficient applicable
evidence remains `not_recorded`; an audit or owning boundary that was not
available remains `not_checked`; and an integration with no applicable Hook
property is `not_applicable`. Aggregation is independent of input order: a
failed applicable phase or installation wins, then `not_recorded`, then
`not_checked`; verified applicable inputs remain verified, while
`not_applicable` is neutral unless no applicable input exists. Ambient coverage
cannot pass unless the managed files and applicable path-safety assessment pass.

Verbose Connection output summarizes a uniformly verified assessment without
artifact paths. Any nonverified or internally contradictory dimension expands
the bounded evidence in failure-first order with its source, reason,
installation, phase, and path; reaching the evidence bound is labelled as a
possible omission. JSON retains the complete typed assessment in either case.

A recorded verification-tool name mismatch never passes
`managed_capability_proof`. Its bounded facts expose the exact expected and
observed tool names. A prior revision, non-managed runtime row, missing
milestone, or milestones split across sessions cannot substitute.

Any bounded Codex version proceeds through these behavioral checks. The PATH
executable version does not select or disqualify a managed runtime session.

`dry_run` is an operation mode, never a connection or check status.
Configuration matching, executable availability, protocol and host versions,
capability observations, and observation timestamps belong in check facts;
they do not introduce another public or persisted status enum.

Each step ID has fixed actor, channel, diagnostic class, agent sequence, and
intended-check metadata:

| ID | Initiator / executor / channel | Intended completed checks |
|---|---|---|
| `reload_codex` | `user` / `host` / `codex_ui` | `host_reload` |
| `review_project_hooks` | `user` / `user` / `codex_ui` | `hook_source_activation` |
| `request_integration_verification` | `user` / `agent` / `codex_chat` | `managed_session_health`, `managed_capability_proof`, `ambient_hook_coverage`, `correlated_guard_verification` |
| `read_connection_status` | `user` / `volicord` / `cli` | none |
| `run_optional_active_diagnostics` | `user` / `volicord` / `cli` | active diagnostic checks; optional only |
| `repair_hook_contract` | `user` / `user` / `codex_ui` | `hook_source_activation`, `ambient_hook_coverage` |
| `repair_managed_configuration` | `user` / `volicord` / `cli` | `managed_config` |

`request_integration_verification` contains this nested sequence:
`volicord.list_projects`, `volicord.begin_integration_verification`,
workflow-directed `volicord.guard_probe`, and workflow-directed
`volicord.get_integration_verification`. The user initiates the Codex chat
request and the agent executes the tools. The Guard probe is never a sibling
top-level step. `awaiting_probe` permits the probe once,
`awaiting_observation` permits the status tool once, and `repair_required` or
`complete` stops tool execution and same-turn restart.

Strict construction and decoding reject step metadata that does not match its
ID. `root_finding_ids` connects a step to its current independent causes.
Required steps are derived from root findings and current check state. A
blocked downstream check does not emit an observation step; its blocker's
repair step comes first. Equivalent steps from several symptoms collapse to
one stable step. A repair-required correlated attempt projects
`repair_hook_contract` or `repair_managed_configuration` as applicable, not a
blind Guard probe. `run_optional_active_diagnostics` stays separate from
required activation. Registry storage does not keep an independent
verification status or activation array. A
connection with no completed persisted report is projected as a synthesized
`status=action_required` report containing one `verification_not_run` pending
check, one required `request_integration_verification` step, and one optional
`run_optional_active_diagnostics` step. Reading that projection does not
persist it.

The administrative CLI projects init, add, status, verify, mode, and remove
through the current schema-2 `DiagnosticReport`. It carries the canonical
checks, bounded findings and cause edges resolved from the current evaluation
overlay and Store APIs, derived
root IDs, the same root-scoped `IntegrationActivationPlan`, Connection context,
operation-specific result details, and report limits. Concise, verbose, and
JSON output are projections of this same report and identify the same roots.
No renderer derives a cause or remediation category from summary prose, and no
projection re-exposes a fact redacted by `DiagnosticFinding`.

A current Connection report selects findings from the exact IDs referenced by
its `failed` and `blocked` checks and then resolves only their bounded cause
chains. Resolution uses an inline finding from the current evaluation before
an explicitly persisted Store seed, while retaining explicit provenance for
each reference. The combined graph may contain inline current findings,
persisted immutable occurrences, and persisted active current-state findings.
An independent current finding appears only when the operation
deliberately selects it; the report never treats every stored finding on the
same integration revision as current. CLI-owned current-state operational
findings bind each closed diagnostic value to one immutable definition and use
typed subjects for the exact managed-config target, Product Repository trust,
Guard managed artifact, Guard phase, Guard Installation, Guard event,
integration revision, or verification tool. Each subject owns its scope,
typed versioned canonical identity encoding and opaque subject identity, and a
separate safe display projection. Path-bearing subjects canonicalize filesystem
aliases before deriving the opaque identity and do not persist the canonical
path bytes. Each `CurrentDiagnosticKey` includes the complete Connection scope,
full code, domain, stage, source, and opaque subject identity. Its stable ID is
the full fixed digest of that complete key, so the same diagnostic code on two
artifacts or phases remains two findings while re-observing one subject
refreshes only its snapshot, including its safe display projection.

Active verification reconciles each complete CLI owner observation set. It
activates or refreshes the conditions still observed and explicitly resolves
previously active conditions omitted after a successful repair, compatible
revision, or fresh observation. Resolved current findings remain available by
exact ID but are not reportable current findings and do not reappear through a
failed or blocked check's current projection.

The JSON projection includes `generated_at` as the report time and the exact
current integration revision in Connection context when one exists. The
persisted verification `checked_at` remains the observation time for that
verification and is not repeated as a competing top-level time. Status builds
a complete in-memory current evaluation from stored active-probe facts and
current observations, but that read does not persist the projection or modify
any timestamp. It needs no verification run to make an inline cause
reportable. Only a reference explicitly classified as persisted whose Store
row is absent is represented as the typed
`diagnostics.finding_record_missing` observation; rendering does not fabricate
the missing domain facts. An inline finding is returned as the actual cause
and never receives missing-record substitution or
`action.diagnostics.rebuild_current_observations` guidance.

`volicord connection list` applies that same current evaluator independently
to every selected Connection Project membership. One invocation supplies one
evaluation timestamp and request-scoped Connection evidence; current
configuration, runtime-session evidence, project Store and Guard state remain
subject to their exact owner reads. A membership-local unavailable result is a
closed typed value and cannot become a persisted or synthesized aggregate
status. It does not suppress successful sibling memberships. Repository
filtering selects memberships before these current reads.

Optional active verification captures the exact typed Connection integration
revision before it plans or probes. Store persists the resulting report only
when that same revision is still current, using one immediate Registry
transaction for the comparison and report replacement. The write changes only
`verification_report_json` and the ordinary row update timestamp. A revision
conflict leaves the existing report and every owner field unchanged and
requires verification to be rerun. Verification observes managed
configuration; it never applies, adopts, or records a newly planned managed
fingerprint.

The persisted report retains the immutable preflight evidence and the latest
active verification evidence as the separate `mcp_server` detail members
defined above. Replacing the complete report may replace the complete
`last_active_verification` value, but no path rewrites a field inside
`preflight.evidence`. A preflight-only or failed-preflight evaluation has no
active value and cannot report writeability as passed.

A persisted action missing any required typed member, containing an unknown
member, using noncanonical reference order, or carrying metadata inconsistent
with its ID is a malformed current report and strict reads reject it. Active
verification may replace such a report through the same revision-guarded
replacement boundary; it does not modify unrelated Connection owner state. No
in-place JSON rewrite or alternate decoder applies.

Operational compatibility is determined from the current managed configuration
and the protocol, tool-list, required-tool, safe-call, and Guard behavior the
adapter actually observed. `complete` means every applicable required check
passed and every other check is `not_applicable`. It does not establish
operating-system enforcement, actor or human
identity, correctness, future behavior, or tamper-proof recording. Core
invocation authorization is evaluated separately for each managed MCP call.

## Integration Revisions And Operational Sessions

Agent Connection mutations, managed-launch lease issue/consumption/cleanup,
runtime and project-session observations, lifecycle milestones, verification
writes, and terminal finding links acquire per-operation `SharedWriter`
admission for their exact Runtime Home. They retain the permit-derived Store
context through every Registry and project-database effect. Setup instead uses
its existing `ExclusiveSetup` context. While setup is exclusive, these
operations return `runtime_home.mutation.setup_in_progress` with no partial
Connection, session, finding, or verification record.

The current Connection integration revision is a typed, domain-separated
canonical SHA-256 digest. Its basis is the Agent Connection identity, immutable
Store-owned integration-instance ID, host kind, intent, scope, mode, server
name, configuration target, and current exact managed-configuration
fingerprint, plus the nonnegative Store-owned integration generation. That
fingerprint covers the managed server command and entry and identifies the
Volicord-managed host configuration that a setup owner last successfully
applied or adopted. Only setup, repair, staged activation, or another explicit
managed-configuration mutation may replace it. Replacing it changes the
integration revision and clears the prior verification report atomically;
compatible replay with the same fingerprint may retain that report.

For `volicord init`, the fingerprint and integration revision belong to one
typed setup plan built after acquiring the canonical Runtime Home setup lease.
They are recorded only after the plan's Runtime Home and Store mutations are
checkpointed and its repository files and Codex configuration have been
atomically replaced. The lease remains held through result construction,
cleanup, or rollback. A planned, preserved, rolled-back, or partially
rolled-back setup does not emit the committed activation plan. A competing
setup is busy before planning, and other concurrent target changes fail before
stale bytes are written and remain external owner state.

Store generates a new opaque integration-instance ID only when it inserts a
new physical `agent_connections` row. Compatible registration replay,
enabled-state and verification updates, staged activation and cleanup recovery,
and mode transitions preserve it. Physical deletion removes the instance with
the row. Recreating the same deterministic Connection identity therefore gets
a new integration-instance ID even when every caller-visible target and
configuration input is unchanged.

The integration generation begins at zero and increments exactly once for each
successful real mode transition within that physical instance; a same-mode
no-op leaves it unchanged. Therefore the generation distinguishes revisions
within one physical instance, while the integration-instance ID distinguishes
deletion and recreation. Returning to a previously used mode still creates a
new revision and cannot make evidence from that earlier mode generation current
again.

The observed executable path, host version, and MCP client name/version remain
diagnostic facts. Authorization uses the current Connection, revisions, and
authoritative runtime/project session bindings.

The actual MCP peer is the bounded `clientInfo.name` and `clientInfo.version`
observed on that runtime session. The PATH probe is the separately observed
Codex executable path and version. Reports and findings never substitute one
for the other. When both versions exist and differ,
`host.codex.peer_version_differs_from_path_probe` records warning evidence with
both fact objects; the mismatch alone is not a fatal Connection failure.
The explicitly selected `codex-mcp-turn-metadata` profile owns MCP
session/thread/turn metadata. Malformed MCP metadata, inconsistent nested and
top-level thread coordinates, and registered-session mismatch use
`host.codex.metadata_malformed`,
`host.codex.session_thread_turn_inconsistent`, and
`host.codex.registered_session_correlation_mismatch` respectively.

Each MCP process start creates an opaque Registry runtime-session ID before
host thread metadata exists. `session_source` is exactly `managed_host`,
`manual_cli`, `cli_preflight`, or `integration_probe`. Only atomic successful
launch-lease consumption can create `managed_host`, and only `managed_host` can
authorize an Agent Connection call. Public `volicord mcp serve` always
records `manual_cli`; preflight creates no session, and integration probes
never count as managed host activity. The runtime session retains its owning Connection and Connection
integration revision.

After a valid initialize request, that runtime owns one session-scoped typed
MCP selection. Its `McpSessionMilestones` retain the runtime, source,
Connection, integration revision, process start, actual peer `clientInfo`,
requested/selected/negotiated protocol revisions, initialize and initialized-
notification completion, `tools/list` time and exact deterministic returned
tool identities, required-tool validation time, canonical verification-tool
identity/time, terminal finding, and last observation. Invalid combinations
are rejected: negotiation requires completed initialization, required-tool
success requires an actual list observation, verification-tool success
requires same-session required-tool validation, and a managed capability proof
requires `session_source=managed_host`. Reconnection creates a new runtime and
new milestones; profiles and milestones are not shared across processes.

Connection report context gathers session IDs from check evidence as well as
finding correlation. Each entry preserves the closed roles
`latest_managed_attempt`, `latest_managed_capability_proof`,
`guard_verification_attempt`, and `guard_verification_proof`. One session with
several roles appears once with a canonical role list. Context also carries
the relevant Guard verification IDs. Human and JSON projections expose the
same role assignment.

The project integration revision extends the Connection revision with the
current project workflow-policy fingerprint, current Guard installation
identity/policy hash or explicit absence of Guard ownership, the repository
observer semantic contract digest, and the canonical Product Repository
effect-catalog digest. It also binds three current semantic digests: managed
agent guidance, the workflow action contract, and the MCP semantic schema. The
guidance digest covers the closed current workflow facts rendered into the
managed `AGENTS.md` block and MCP server instructions; it is not a hash of
project-specific prose. The action-contract digest covers catalog admission,
method-specific form identity, exact fixed-argument binding, authoritative
context, retry, and typed basis-mismatch semantics. The semantic-schema digest
covers type-owned constraints, generic required-nullable semantics, explicit
discriminators, branch-local validation, and bounded runtime semantic
projection. A change to any of these semantic digests changes the managed
integration revision even when the hook command text is unchanged. Historical
Guard events remain evidence for their recorded revision and cannot satisfy
current-definition coverage. `host_sessions`
retains the revision-scoped local session ID, Connection, exact native session,
and observation times. `host_turns` retains turns shared by both contract
sources. `host_tool_invocations` retains hook tool-use IDs and canonical tool
names. Store derives the local session ID with a domain-separated digest over
the Connection internal ID, exact project integration revision, and exact
native session only after resolving the project and validating current Guard
ownership. Callers cannot supply the complete local ID, and the stored project
revision is immutable.

The `CodexCommandHooks` marker selects the `codex-command-hooks` parser, which yields
`CodexHookPromptCorrelation` for `UserPromptSubmit` and
`CodexHookToolCorrelation` for `PreToolUse` and `PostToolUse`. Prompt
correlation requires only session and turn. Tool correlation additionally
requires tool-use ID and canonical tool name. No hook phase has a thread
coordinate. The distinct `CodexMcpTurnMetadata` marker selects the
`codex-mcp-turn-metadata` parser, which yields `CodexMcpCorrelation` with
required session, thread, and turn. `HostNativeCorrelation` preserves these
source variants instead of providing a generic interchangeable coordinate.
Store phase checks and SQL discriminators reject cross-source or incomplete
combinations.

Repository Observation uses only the exact `CodexHookToolCorrelation` shared
by the matching `PreToolUse` and `PostToolUse` events. A generic optional
`host_invocation_id`, MCP thread coordinate, path report, or tool name alone
cannot correlate that observation. The complete observation coordinate and
lifecycle belong to [Repository Observation](repository-observation.md).

An accepted `UserPromptSubmit` establishes its exact typed turn and atomically
closes only open observations from different established turns in the same
derived project session. It never orders turn identifiers or closes a parallel
observation in that current turn. Authoritative graceful or failed termination
of the exact bound `managed_host` runtime closes the session's remaining open
observations through its bounded Registry project bindings. Recovery applies
that effect only to runtime rows already carrying a terminal fact.

The Connection's registered `server_name` is decoded as `McpServerKey` and
remains separate from each complete `McpRawToolName`. The
`CodexMcpCallableNames` contract projects those coordinates to one
`HostCallableIdentity` under `codex-mcp-callable-names`. It never treats a
period-delimited segment of the raw tool name as the server key. Generated
hooks, MCP preflight diagnostics, and Guard verification resolve the same
collision-checked `McpToolCatalog`; reverse resolution is an exact catalog
lookup, not punctuation parsing. The adapter selects this semantic contract
directly and does not derive host behavior from an observed Codex package
version.

The same semantic command-hook contract owns the typed tool-routing strategy.
For tool phases it routes the reviewed native host-tool set plus the
Connection's server-qualified MCP callables. It uses the registered namespace
when the callable projection preserves that namespace, or exact tokens derived
from `McpToolCatalog` otherwise. The generated matcher is only an acquisition
boundary: after delivery, the wrapper resolves the observed callable through
`McpToolCatalog`. The canonical catalog classifies every resolved tool as
`ProbeTarget`, `WorkflowControl`, or `UnrelatedKnownTool`, and catalog
construction rejects contradictory role metadata. Callable and role are
resolved before any probe-specific session, turn, verification-ID, or tool-use
check. Only the exact `AgentToolId::GuardProbe` probe target may continue
through those checks and complete verification. Begin, status, and all other
known tools record nonterminal `UnrelatedRoutedTool` trace even when they carry
the current verification ID or different coordinates; they cannot satisfy the
probe or produce coordinate or callable mismatch. An unknown same-server
callable is likewise nonterminal unless it claims the exact current
verification ID, in which case it records terminal `CallableIdentityUnknown`.
An incompatible payload and actual probe-target coordinate mismatches retain
their distinct acquisition stages.

The durable stages are `ProbeAcknowledged`, `UnrelatedRoutedTool`,
`HookEventNotObserved`,
`HookPayloadIncompatible`, `CallableIdentityUnknown`,
`CallableIdentityMismatch`, `VerificationIdMismatch`, `SessionMismatch`,
`TurnMismatch`, `ToolUseMismatch`, `PreToolMatched`, and `PostToolMatched`.
They retain bounded callable and categorical correlation facts only. In
particular, `HookEventNotObserved` says that Volicord received no event; it
does not diagnose a matcher fault or a host-emission fault.
`UnrelatedRoutedTool` is trace only: it is not a repair reason, retry input,
probe proof, acknowledgement, or status-read-budget consumer.

Guard correlation and Guard policy are separate steps. A compatible hook
correlation may reach policy and produce `Continue`, `ContinueWithContext`,
`ContinueWithWarning`, or `Deny`. An incompatible hook contract instead
records an observation failure when possible, produces no policy decision, and
does not satisfy the phase. In the Codex `record` profile, the adapter continues
that host action with bounded context and exit `0`; it does not convert the
observation failure into denial. Event persistence unavailability follows the
same non-synthetic-denial rule. Only an explicit compatible `PreToolUse`
policy `Deny` becomes a Codex permission denial. `PostToolUse` output can warn
about or require reconciliation of an already-completed action, but cannot
claim it prevented the action.

The host-neutral boundary is `GuardHookOutcome`: observation outcome, optional
policy decision, bounded diagnostics, and safe feedback. The Codex adapter,
not Core or Store, owns stdout hook JSON, stderr, process exit, context,
warning, and denial projection.

A Guard observation may create normalized host, turn, and tool rows but never
creates the MCP-only `managed_mcp_sessions` row. The first actual managed MCP
tool call validates the current managed runtime without mutation, establishes
or validates that exact MCP anchor, revalidates current owner inputs while
reserving the cross-database binding, and only then attaches its runtime in a
final project transaction. Connection, project, Guard Installation, revision,
native-session, thread, or existing-runtime conflicts detected against the MCP
anchor leave no new Registry binding. The anchor may remain unbound if a later
Registry reservation fails, but it is not authorization. A Registry
reservation left by interruption before final attachment is also not
authorization; exact replay under unchanged owner state reuses it and finishes
the attachment. An attached MCP session cannot be rebound across a runtime
session, Connection, project, host session, or host thread.

Runtime rows are historical process observations, not launch leases or
liveness claims. Launch leases exist only to authorize one bootstrap
transition and do not turn runtime rows into liveness records. A crashed
process that did not record an authoritative terminal fact may leave an
apparently open row, and multiple
cooperative Codex processes may be current concurrently. Neither condition
blocks Guard correlation: no runtime is guessed from open rows, and different
host sessions bind independently.

A real Connection mode transition is one Registry transaction over the
Connection and the strict manifest of every owned Guard Installation. The
transaction changes only the mode, integration generation, stored verification
report, manifest integration revisions, and affected Registry timestamps. It
does not change Guard commands, managed-file inventories, policy hashes, host
configuration, or Product Repository files. All candidates must be complete,
current, and owner-matched before any write; otherwise no transition is
committed.

These records establish locally observed cooperative protocol/session ownership
under current configuration. They do not establish client, host, actor,
operating-system-user, or human identity. MCP client name/version and observed
host executable version accept arbitrary bounded future values and remain
diagnostics only.

The integration-instance ID, integration generation, and derived integration
revisions are local lifecycle and correlation coordinates. Store owns their
lifecycle inputs; callers cannot select them.

### Managed in-chat integration verification

`GuardIntegrationVerificationRun` is the durable attempt for the first-party
in-chat workflow. Its immutable semantic coordinate is Connection, project,
managed MCP runtime session, native host session and turn, integration
revision, Guard Installation, host-contract profile, hook-definition digest,
and policy digest. Exactly one verification ID exists for that coordinate,
including after it becomes terminal. The row also records the semantic
observation policy, bounded status-read count, cleanup boundary, first probe
acknowledgement, correlated prompt/pre/post event IDs, completion, and terminal
finding. Cleanup time does not change attempt identity or workflow state.

Only an actual current `managed_host` call can begin, probe, or read a run.
Manual stdio, CLI preflight, and integration probes cannot create success.
`IntegrationVerificationWorkflowState` is the one public projection shared by
begin, probe, and status. Its tagged alternatives are `awaiting_probe` with the
canonical Guard probe tool, `awaiting_observation` with the canonical status
tool, authoritative acknowledgement time, and remaining bounded reads,
`complete` with completion time, and `repair_required` with a typed repair
reason, separate retry policy, and bounded finding. Tool fields are typed
canonical `AgentToolId` projections, not arbitrary strings. `complete` and
`repair_required` are immutable terminal states.

The host contract selects `HookObservationPolicy` semantically as
`Synchronous { allowed_status_reads }` or
`Deferred { deadline, allowed_status_reads }`. The reviewed current Codex
command-hook contract selects synchronous observation with one status read;
neither package version numbers nor numeric profile generations select this
behavior. The deterministic sequence is begin or resume, call GuardProbe once
when requested, call status according to that policy, then stop at a terminal
state. There is no sleep loop or automatic same-turn retry.

Probe acknowledgement is first-write-wins for the exact verification ID,
Connection, managed runtime session, native host session, and native host turn.
The first eligible call records the timestamp. Replay retains
`awaiting_observation` and that original timestamp. Exact replay after
completion or repair returns the same terminal state. No replay changes
completion or matched events. A different caller coordinate is rejected
without exposing the acknowledgement, and a terminal attempt without one
cannot acquire a late acknowledgement.

A pass requires the prompt, pre-tool, and post-tool records to belong to the same
run session and turn; pre/post must share the tool-use ID, generated exact probe
name, and verification-ID input. The current Guard Installation, policy hash,
integration revision, hook-contract digest, and managed runtime must still
match. Prompt is no later than pre-tool, and pre-tool is earlier than post-tool.
No historical event search or cross-attempt phase assembly is allowed.
Exhausting synchronous reads produces the most precise typed repair reason:
missing event, incompatible payload, callable, verification ID, session, turn,
or tool-use mismatch. Owner drift likewise distinguishes integration revision,
hook definition, and policy changes. Retry eligibility is represented only by
`no_automatic_retry`, `new_turn_required`, `host_reload_required`,
`hook_review_required`, or `repair_required`; a new attempt still requires a
genuinely different eligible coordinate.

`ambient_hook_coverage` and `correlated_guard_verification` deliberately do
not share one boolean. `AmbientGuardCoverageEvidence` proves only current hook
definition execution and general coverage of every configured prompt/pre/post
phase. `CorrelatedGuardAttemptEvidence` retains the latest current attempt,
including verification, runtime-session, host-session, host-turn, event,
expected/observed callable, acquisition-stage, repair, retry, and timestamp
facts. `CorrelatedGuardProof` retains the latest completed current proof.

The verbose Connection projection strictly derives one typed human view from
those records. Matching attempt/proof correlation coordinates are grouped once
under `Correlation`; attempt state, repair and recovery facts, and lifecycle
times remain under `Attempt`; and proof lifecycle remains under `Completed
proof`. If a newer pending or repair-required attempt coexists with an older
completed proof, matching coordinates remain shared while divergent values are
shown separately as the latest attempt identity and earlier completed proof
identity. The proof is labelled as earlier historical evidence and does not
imply that it satisfies the newer attempt. A same-verification identity or
completion divergence is corrupt data and fails the strict projection boundary
instead of being merged for display. JSON retains both complete evidence
objects and their exact underscore values.

The correlated check selects its `observed_at` from those typed records:

- `awaiting_probe` uses the latest attempt's creation time;
- `awaiting_observation` uses the latest actual applicable acquisition
  observation, probe acknowledgement, or attempt creation time;
- `complete` requires the completed current proof and uses that proof's
  completion time;
- `repair_required` uses the latest attempt's terminal repair-transition time;
  and
- no current run omits `observed_at`.

The selection never uses report evaluation time. A passed check requires the
attempt and proof to identify the same verification, Connection, Connection
Project membership, integration revision, managed runtime session, native
host session and turn, Guard Installation, policy, and hook definition.
Creation cannot follow acknowledgement or terminal completion;
acknowledgement cannot follow terminal completion; applicable acquisition
observations cannot precede creation or follow the terminal transition; and a
completed attempt and its proof must carry the same completion time. Missing,
malformed, chronologically invalid, or identity-inconsistent persisted facts
are corrupt data and never fall back to `checked_at` or `generated_at`.

No attempt, or an active attempt under a deferred host policy, is `pending`.
`complete` is `passed`. `repair_required` is always `failed` with typed
recoverability and action; a recoverable failure may make the aggregate
`action_required`, but neither its check nor attempt state becomes pending. An
older completed proof remains historical capability evidence when a newer
attempt fails and cannot make the current check pass. Diagnostic codes are
selected directly from the typed repair reason and acquisition stage, never
from summary wording. The run is cooperative local
evidence only. It does not automate or bypass Codex project trust, modify MCP
trust configuration, establish user or host identity, or create Core workflow
authority.

<a id="validated-agent-session"></a>

## `ValidatedAgentSession`

Core accepts Agent Connection invocation authority only through this
non-serializable typed boundary:

```rust
struct ValidatedAgentSession {
    connection_id: AgentConnectionId,
    project_id: ProjectId,
    runtime_session_id: AgentRuntimeSessionId,
    project_session_id: AgentSessionId,
    integration_revision: IntegrationRevision,
}
```

It is created only after validating all of the following current facts:

1. the Agent Connection exists and is enabled;
2. the project exists and is currently a Connection Project;
3. the runtime session belongs to that Connection;
4. the project session has a non-null runtime binding;
5. the Registry binding exactly matches the runtime, Connection, project,
   project session, and host session;
6. the project session belongs to that runtime session, Connection, and
   project;
7. the runtime and project session revisions match current Connection and
   project integration revisions;
8. the Connection mode allows the requested operation category;
9. `ActorSource::AgentConnection` exactly names the validated Connection;
10. a project-scoped operation exactly names the validated project;
11. the runtime session has `session_source=managed_host`, never `manual_cli`,
   `cli_preflight`, or `integration_probe`.

The adapter validates the authoritative runtime and project rows on every
project tool call before constructing Core invocation context. Executable path,
host version, and client version remain diagnostics outside this authorization
decision.

Core derives the audit basis deterministically:

```text
connection:<connection_id>/session:<project_session_id>/revision:<project_integration_revision>
```

This basis is the deterministic local lifecycle and correlation coordinate for
the validated operational ownership recorded in the audit event.

## Codex Adapter Responsibilities

The Codex adapter owns host-specific configuration inspection and mutation:

- discover the Codex configuration target;
- install only the managed entry selected by current Connection inputs;
- project the canonical managed MCP launch contract into Codex TOML and parse
  it strictly back into the same contract;
- validate that exact current entry again before issuing a one-time launch
  lease and entering managed stdio;
- detect missing, modified, or extra managed configuration as drift;
- report executable availability and bounded host version diagnostics;
- repair owner-defined managed state from current canonical inputs; and
- uninstall only matching Volicord-managed state.

The Codex adapter does not classify native Linux or WSL2 and does not validate
the process target or filesystem restrictions. Those observations and checks
belong to the platform filesystem boundary under
[System Requirements](system-requirements.md).

Setup and repair plan and validate first, apply or adopt host configuration,
then commit the resulting managed fingerprint and owner-coherent Registry and
Guard state. Verification begins only from that final Connection record and
persists its report against that record's exact integration revision. Report
persistence never performs a second fingerprint update.

Runtime authorization validates the current enabled Connection, project
membership, allowed mode, managed runtime session, revision-scoped project
session, and exact Registry/project binding. Command names, executable paths,
version strings, Runtime Home configuration, and local session metadata are
diagnostic or routing facts and do not establish actor or human identity. The
launch lease is an evidence-integrity transition coordinate, not an operating-
system actor credential.

Repair does not overwrite unrelated Codex configuration or silently change the
selected project, Connection, intent, profile, or platform environment.
For both `workflow` and `read_only`, repair may restore missing owned Codex
configuration, Guard files, and the current Guard Installation while preserving
the Connection mode and integration generation. If repair applies a different
canonical managed configuration, it records the new managed fingerprint,
changes the integration revision, and invalidates the prior verification
report before verification under the new revision. If the fingerprint is
unchanged, the current revision is preserved.
Uninstall removes only content whose current managed identity still matches
Volicord ownership.

An explicit connection-removal command also retires connection-owned Registry
integration state. Removing one membership deletes its Registry project-session
bindings and Guard Installation. A multi-project personal Connection remains,
with its connection-wide runtime sessions and other projects' owned rows, until
its last membership is removed. Last-membership removal deletes all remaining
Registry project-session bindings, Guard Installations, runtime sessions, and
the Agent Connection. Project-local Agent Sessions, Guard observations,
workflow history, evidence, and other authority records remain historical
Product Repository state. They cannot authorize a later call without a current
Registry membership and a current validated runtime/project session.

Connection replacement cleanup uses the same project-scoped Registry
retirement order. For a superseded multi-project Connection, it removes only
the selected project's runtime/project bindings, Guard Installation, and
membership in the atomic replacement activation transaction. For a superseded
last-project Connection, it keeps the old Connection disabled with that complete project
inventory and a pending-host-cleanup marker until external cleanup succeeds.
The final Registry transaction revalidates the replacement and exact retained
inventory before deleting the bindings, Guard Installation, and membership and
clearing the marker. Cleanup or revalidation failure leaves the inventory
intact for retry; successful cleanup retains the disabled zero-membership
historical Connection and its connection-wide runtime sessions.

## Threat Model

Trusted:

- the same operating-system user account;
- the `Volicord Runtime Home` owned by that account; and
- that account's Store write access.

Untrusted:

- external host and client input;
- a manual, integration-probe, stale, closed, or wrong-revision session;
- a session for another project, runtime, or Connection;
- manually modified configuration; and
- client/host version and process metadata as identity claims.

Tampering with Runtime Home by a malicious process running with the same user
permissions is outside the current threat model. The local records are
therefore cooperative and are not tamper-proof against another process with the
same account access.

## Adjacent Owners

- Managed stdio MCP behavior: [MCP Transport](mcp-transport.md).
- Install, verify, repair, and uninstall commands:
  [Administrative CLI](admin-cli.md).
- Platform cells and WSL2 topology:
  [System Requirements](system-requirements.md).
- Ordinary build, package, platform, and release validation:
  [Validation](../maintain/validation.md).
- Runtime and repository path boundaries:
  [Runtime Boundaries](runtime-boundaries.md).
- Security guarantees and non-guarantees: [Security](security.md).
