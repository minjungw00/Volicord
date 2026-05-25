# MCP API And Schemas

## What this document helps you do

Use this reference to implement, test, or review the public MCP resource and tool contract for Harness. It owns read-only resources, public tools, common envelopes, request and response schemas, shared refs, error taxonomy, idempotency, state conflict behavior, `ValidatorResult`, and `ArtifactRef`.

It does not own SQLite DDL, storage layout, the full kernel transition table, projection template text, CLI command semantics, or connector cookbook details. Storage-owned JSON and DDL rules live in [Storage And DDL](storage-and-ddl.md).

## Read this when

- You are wiring an MCP client or server surface to Harness Core.
- You need the exact public request or response shape for a Harness tool.
- You are checking which errors, validator results, artifact refs, or projection refs can appear in API responses.
- You are writing conformance fixtures that assert public API behavior.

## API in plain language

MCP resources are read-only views. They can report current state, projection freshness, and user-facing summaries, but they must not create or repair state.

All state changes go through public tools and Core. A tool response may include projection paths and artifact refs, but those are references to canonical state records or durable evidence files, not replacements for canonical state.

The public request and response schemas in this document are the validation source for API payloads, including API-shaped payloads that Core later stores. Core must validate every storage JSON value before commit against either the API-owned shape here or the storage-owned shape in [Storage And DDL](storage-and-ddl.md). Malformed or schema-incompatible JSON is invalid state.

## Reference scope

This document owns:

- read-only MCP resources
- public MCP tools
- common tool envelope
- public request/response schemas
- shared refs including `StateRecordRef`, `ArtifactRef`, and projection refs
- public error taxonomy and primary error precedence
- idempotency behavior
- state conflict behavior as exposed through the API
- `ValidatorResult`
- artifact input and artifact ref schema as public API shapes

## Not covered here

This document does not own:

- SQLite DDL or storage layout; see [Storage And DDL](storage-and-ddl.md)
- storage-only JSON `TEXT` validation; see [Storage And DDL](storage-and-ddl.md)
- lock policy; see [Storage And DDL](storage-and-ddl.md)
- migrations; see [Storage And DDL](storage-and-ddl.md)
- artifact directory layout; see [Storage And DDL](storage-and-ddl.md)
- baseline capture storage format; see [Storage And DDL](storage-and-ddl.md)
- projection job table; see [Storage And DDL](storage-and-ddl.md)
- full kernel transition table; see [Kernel Reference](kernel.md)
- projection template bodies; see [Template Reference](templates/README.md); projection rules live in [Document Projection Reference](document-projection.md)
- operator command syntax; see [Operations And Conformance Reference](operations-and-conformance.md)
- connector capability profiles; see [Agent Integration Reference](agent-integration.md)
- connector cookbook recipes; see [Surface Cookbook](surface-cookbook.md)

## Minimal call flow

1. Read status with `harness.status`, `harness.next`, or read-only resources.
2. Intake or resume with `harness.intake` when a Task should be tracked.
3. Request decision if blocked with `harness.request_user_decision`.
4. Call `harness.prepare_write` before product write.
5. Call `harness.record_run` after run/change.
6. Record evidence/eval/QA/acceptance when applicable through the matching public tool or Decision Packet path.
7. Close when blockers clear with `harness.close_task`.

Capability is not a first-class kernel gate. Surface capability appears through:

- the `surface_capability_check` validator
- `harness.prepare_write.response.blocked_reasons`
- guarantee display in status and write decisions

Core preconditions and mechanical checks may run before or beside validators. Only stable IDs emitted as `ValidatorResult` and persisted in `validator_runs` are validator IDs; checks such as `scope_coverage`, `changed_paths`, `changed_paths_intent`, `approval_scope`, `baseline_freshness`, `qa_waiver_reason`, and `projection_freshness` remain Core checks unless an owning docs section explicitly promotes them.

## Read-only resources

Resources expose current state and projection-oriented summaries without mutating state:

```text
harness://project/current
harness://project/surfaces
harness://task/active
harness://task/{task_id}
harness://task/{task_id}/summary
harness://task/{task_id}/spine
harness://task/{task_id}/journey
harness://task/{task_id}/decision-packets
harness://task/{task_id}/change-unit-dag
harness://task/{task_id}/judgment-context
harness://task/{task_id}/reports/latest
harness://task/{task_id}/evidence-manifest
harness://task/{task_id}/bundle/current
harness://design/domain-language
harness://design/module-map
harness://design/interface-contracts
harness://policy/sensitive-categories
harness://status/card
```

Resource reads must not create Task records, decisions, projection jobs, or reconcile items. If a resource detects stale projection, it reports freshness; it does not repair it.

The Journey resources are projection-oriented reads over canonical state:

- `harness://task/{task_id}/journey` returns the current Journey Card and Journey Spine-oriented refs.
- `harness://task/{task_id}/decision-packets` returns active, resolved, deferred, and blocked Decision Packet summaries for the Task.
- `harness://task/{task_id}/change-unit-dag` returns Change Unit dependency refs and ordering summaries.
- `harness://task/{task_id}/judgment-context` returns the minimum current context needed for a user judgment, with optional pull refs separated from required context.

## Tool envelope

Every public tool request carries an envelope. State-changing tools require a non-null `idempotency_key` and `expected_state_version`. Read-only tools accept the same envelope for tracing; they may set `expected_state_version` to `null`.

State version scope is resolved by Core from the operation's primary addressed Task. The resolved primary Task may come from `ToolEnvelope.task_id`, a tool-specific `task_id`, or active Task resolution. Task-scoped mutations compare `expected_state_version` with that Task's `tasks.state_version`. If Core resolves no primary Task and the operation is project-scoped, it compares `expected_state_version` with `project_state.state_version`.

```yaml
ToolEnvelope:
  request_id: string
  idempotency_key: string | null
  expected_state_version: integer | null
  project_id: string
  task_id: string | null
  surface_id: string
  run_id: string | null
  actor_kind: user | lead_agent | evaluator | operator
  dry_run: boolean
```

### MCP boundary and caller trust

The public MCP contract assumes a local process or localhost connection for a registered project surface. Exposing the MCP server beyond that local boundary changes the threat model and requires a documented connector capability profile, access-control contract, and guarantee display. Without that stronger profile, a caller that can reach the MCP endpoint is still treated as a source of claims that Core must validate, not as automatically trusted authority.

The access-control contract can be implemented in different ways, such as localhost-only binding, a Unix-domain socket constrained by local file permissions, a per-project token, process-scoped configuration material, or an equivalent local control. These examples are not a schema enum and do not require one CLI syntax. What matters for the public API contract is that the caller's access mode matches the registered surface profile and that Core still validates every envelope claim before any mutation.

Unauthorized or off-profile callers must not be upgraded into authority because they can reach an endpoint. The API does not add an MVP `UNAUTHORIZED` error code for local-access profile mismatches. If the call cannot reach Core, no authoritative Core response exists. If Core or the operator can classify the problem, responses use existing `MCP_UNAVAILABLE` or `CAPABILITY_INSUFFICIENT` paths, with `details.mcp_unavailable_kind=unknown` when the access problem cannot be classified more specifically. Mismatched project, Task, surface, Run, or actor claims are resolved through the normal record-compatibility, state-conflict, scope, capability, and validator checks for the addressed tool.

Envelope fields are routing and audit claims:

- `project_id`, `task_id`, `surface_id`, and `run_id` must resolve to records compatible with the addressed operation. A caller cannot create authority by naming another project, Task, surface, or Run.
- `actor_kind` describes the claimed actor role for routing and policy checks. It must not by itself satisfy approval, user acceptance, Decision Packet resolution, Manual QA judgment, or detached verification independence.
- `idempotency_key` prevents duplicate committed mutations. It is not an authorization token, and replay is valid only for the same canonical request payload in the same `(project_id, tool_name, idempotency_key)` scope.
- `expected_state_version` is the caller's concurrency claim. A stale or wrong version returns `STATE_CONFLICT`; it must not be used to force an older Task or project view to win.
- `dry_run=true` returns diagnostics only. It does not reserve an idempotency key, create a Write Authorization, attach artifacts, or prove that a later write is safe.

## Common response

Common response fields:

```yaml
ToolResponseBase:
  request_id: string
  idempotency_key: string | null
  project_id: string
  task_id: string | null
  state_version: integer
  dry_run: boolean
  errors: ToolError[]
  validator_results: ValidatorResult[]
  events: EventRef[]
  projection_jobs: ProjectionJobRef[]
```

`dry_run=true` validates and returns the transition plan but does not update current records, append to `state.sqlite.task_events`, register artifacts, create consumable Write Authorization records, enqueue projection jobs, or create/update `tool_invocations` idempotency replay rows. Dry-run output is non-authoritative diagnostics; its `idempotency_key` is not consumed for replay.

`ToolResponseBase.state_version` returns the resulting version for the primary affected scope. For state-changing operations this is the Task State Version when Core resolves a primary Task, otherwise the Project State Version. Read-only responses return the current `state_version` for the primary read scope and do not increment it. When `dry_run=true` validates or plans without mutation, `state_version` reports the current primary affected or read scope version; it does not imply a virtual resulting version, idempotency-key consumption, replay row, appended event, or would-be clock increment.

## Shared schemas

```yaml
EventRef:
  event_id: string
  event_seq: integer
  event_type: string
  task_id: string | null
  state_version: integer

ProjectionJobRef:
  projection_job_id: string
  projection_kind: ProjectionKind
  target_ref: string
  projection_version: integer
```

`EventRef.state_version` is the resulting version for the event's affected scope. Task events use `tasks.state_version`; project-level events with `task_id=null` use `project_state.state_version`.

`EventRef.event_seq` mirrors `task_events.event_seq`. Responses list events in ascending `event_seq`; timestamps and `event_id` lexical order are never used for deterministic event ordering.

Event stability for fixture assertions is owned by the [Kernel Stable Event Catalog](kernel.md#stable-event-catalog). Tool sections below describe possible `EventRef.event_type` values that a response may return or an implementation may store; they do not define a second event taxonomy. Names labeled as stable are catalog names. Names outside the stable catalog may appear as implementation-local detail or audit events, but they are not fixture-stable and must not be required by MVP `expected_events` fixtures. ValidatorResult IDs, Core check names, projection status shorthands, and fixture seed shorthand are not event names unless the kernel catalog explicitly lists them.

`ProjectionKind` is an extensible enum with API-owned MVP tiering:

| Tier | Values | Requirement |
|---|---|---|
| MVP-required | `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT` | Reference implementations must support these kinds and enqueue/render them when their source records change. |
| MVP-optional | `MANUAL-QA`, `TDD-TRACE`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT` | Implementations support or enqueue these when policy applies, a source record exists, or the user/operator enables the projection. |
| Extension / optional | `DEC`, `DESIGN`, `EXPORT`, `JOURNEY-CARD` | Implementations may support these only when the corresponding optional projection is enabled. |

ProjectionKind extensibility does not make projections canonical state. Every projection job still renders a derived view from owner records and artifact refs. `DEC` is valid only for standalone Decision Packet Markdown when that feature is enabled, and it is not an MVP-required projection job. Absence of a standalone `DEC` job must not reduce MVP Decision Packet visibility, which is required through `TASK` projections, status/next responses, judgment-context resources, and decision-packet resources. Persisted `JOURNEY-CARD` Markdown is optional; current-position Journey Card output in `harness.status`, `harness.next`, and significant resume flows remains required for agency conformance.

`EXPORT` may include report profiles such as Release Handoff when the export feature is enabled. Such profiles are projection/report surfaces only; they do not create deployment authority, merge authority, production-monitoring authority, final acceptance, residual-risk acceptance, assurance upgrades, or Task close authority.

```yaml
ToolError:
  code: ErrorCode
  message: string
  retryable: boolean
  details: object

ToolErrorMcpUnavailableDetails:
  mcp_unavailable_kind: server_unavailable | surface_mcp_unavailable | stale_connection | unknown

StateSummary:
  mode: advisor | direct | work
  lifecycle_phase: intake | shaping | ready | executing | verifying | qa | waiting_user | blocked | completed | cancelled
  result: none | advice_only | passed | failed | cancelled
  close_reason: none | completed_verified | completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  assurance_level: none | self_checked | detached_verified
  gates:
    scope_gate: not_required | required | pending | passed | failed | blocked
    decision_gate: not_required | required | pending | resolved | deferred | blocked
    approval_gate: not_required | required | pending | granted | denied | expired
    design_gate: not_required | required | pending | passed | partial | waived | stale | blocked
    evidence_gate: not_required | none | partial | sufficient | stale | blocked
    verification_gate: not_required | required | pending | passed | failed | waived_by_user | blocked
    qa_gate: not_required | required | pending | passed | failed | waived
    acceptance_gate: not_required | required | pending | accepted | rejected
```

Sensitive categories:

```text
auth_change
permission_model_change
schema_change
dependency_change
public_api_change
destructive_write
network_write
external_service_write
secret_access
production_config_change
ci_cd_change
infra_or_deployment_change
privacy_or_pii_change
data_export
telemetry_or_logging_change
license_or_compliance_change
billing_or_cost_change
model_or_prompt_policy_change
policy_override
```

## ArtifactRef

An artifact ref points to a durable evidence file registered in the artifact store. Report projections and record projections use artifact refs when they need evidence-file references; the projection itself is not the evidence file.

Artifact registration is not a loose file dump. A staged file becomes a public `ArtifactRef` only after Core validates the staging or capture source, stored-byte integrity, `redaction_state`, and Task-scoped owner relation.

In the reference implementation, artifact registration is Task-scoped. `ArtifactRef.task_id` and `ArtifactInput.relation.task_id` are required and map to `artifacts.task_id` and `artifact_links.task_id`; `retention_class=project` affects retention policy, not artifact ownership scope.

Later Browser QA Capture uses this artifact boundary instead of a new MVP schema. Screen captures normally use `screenshot`; grouped QA outputs can use `qa_capture`; console logs and network traces can use `log` or `qa_capture`; accessibility snapshots and workflow recordings can use `qa_capture` or `other` with a clear description. All such artifacts remain subject to redaction, secret/PII handling, Task-scoped ownership, and Manual QA record or Feedback Loop attachment rules.

```yaml
ArtifactRef:
  artifact_id: string
  kind: diff | log | screenshot | checkpoint | bundle | manifest | qa_capture | export_component | design_probe | prototype | architecture_scan | decision_context | other
  uri: string
  sha256: string
  size_bytes: integer
  content_type: string
  redaction_state: none | redacted | secret_omitted | blocked
  task_id: string
  run_id: string | null
  created_at: string
  produced_by: lead_agent | evaluator | operator | harness
  retention_class: task | project | export | temporary
```

For the reference implementation, `uri` uses `harness-artifact://{project_id}/{artifact_id}`. The local file path is resolved through the per-project `artifacts` registry row in `state.sqlite`, not by trusting an absolute path in the API payload.

`redaction_state` is part of the public artifact contract:

| State | User/operator meaning |
|---|---|
| `none` | No redaction, omission, or blocking was applied because the registered bytes are allowed evidence under the current policy. |
| `redacted` | Sensitive content was removed before storage; the unredacted original is not available through Harness. |
| `secret_omitted` | Secret values or PII are intentionally omitted or replaced by handles. The artifact may support claims whose nonsecret evidence remains visible, but it cannot prove the omitted values themselves. |
| `blocked` | Capture or raw-payload storage was blocked for forbidden content. When Core records a blocked artifact ref, only a metadata notice may be exposed; evidence, QA, verification, projection, and export displays must show the block instead of implying the raw artifact is available. |

For `redacted`, `secret_omitted`, and `blocked`, `sha256`, `size_bytes`, and `content_type` describe the committed safe stored bytes, not a hidden original. For `blocked`, those bytes are the metadata-only notice that Core committed for audit and downstream display; they are never the forbidden raw payload. The notice artifact itself is not available raw evidence for the blocked capture.

Requests that create or attach evidence use `ArtifactInput`. A request may either reference an existing committed artifact or provide a staged file for Core to validate, register, and return as an `ArtifactRef`.

```yaml
ArtifactInput:
  input_id: string
  source_kind: staged_file | existing_artifact
  existing_artifact_ref: ArtifactRef | null
  staged: StagedArtifactSource | null
  kind: diff | log | screenshot | checkpoint | bundle | manifest | qa_capture | export_component | design_probe | prototype | architecture_scan | decision_context | other
  redaction_state: none | redacted | secret_omitted | blocked
  produced_by: lead_agent | evaluator | operator | harness
  retention_class: task | project | export | temporary
  relation:
    task_id: string
    run_id: string | null
    record_kind: task | change_unit | run | decision_packet | shared_design | residual_risk | evidence_manifest | eval | manual_qa_record | feedback_loop | tdd_trace | journey_spine_entry | projection
    record_id_hint: string | null
  description: string | null

StagedArtifactSource:
    staged_uri: string
    display_name: string | null
    content_type: string
    expected_sha256: string | null
    expected_size_bytes: integer | null
```

Rules:

- `source_kind=existing_artifact` requires `existing_artifact_ref` and must set `staged` to `null`.
- `source_kind=staged_file` requires `staged` and must set `existing_artifact_ref` to `null`.
- When an existing artifact is attached to a new record, Core verifies the artifact's task relation and rejects incompatible reuse.
- `staged_uri` is a locator for a Harness staging location or an approved capture adapter output, not permission to read an arbitrary file. Absolute paths, parent traversal, symlink escapes, repo-local paths, and caller-supplied URIs are untrusted unless the staging or capture adapter has already canonicalized them into an approved source.
- `staged_uri`, `display_name`, and supplied `content_type` are untrusted input until Core has validated the staging or capture source, stored bytes, redaction state, and owner relation.
- If `expected_sha256` or `expected_size_bytes` is present, Core verifies the stored bytes before commit. Whether or not those fields are supplied, Core records committed `sha256`, `size_bytes`, and `content_type` from the safe stored bytes after any redaction, omission, or blocking.
- Core applies redaction, omission, or blocking policy before final storage and records the committed artifact as an `ArtifactRef`.
- Logs, screenshots, network traces, export snapshots, and other captured evidence that may contain secrets or PII must be redacted, omitted, or blocked before registration when policy requires it.
- If policy requires omission or blocking, the committed ref records `redaction_state=secret_omitted` or `redaction_state=blocked`; callers must not treat omitted or blocked bytes as available evidence, QA material, verification input, projection body text, or export payload.
- A `blocked` metadata-only notice is still a committed registered artifact record when Core records it. The artifact ref, hash, size, content type, owner relation, and retention class apply to the metadata-only notice bytes and preserve audit/display continuity without making forbidden raw bytes available.
- Tool responses return committed `ArtifactRef` values in `registered_artifacts`, `bundle_ref`, or other response fields. Responses must not echo `staged_uri` as authority or as a durable evidence URI.
- `relation.record_kind` must name an existing canonical owner record or rendered projection ref that Core can validate. For non-projection owners in MVP, the concrete owner row must be Task-scoped to `relation.task_id`; project-scoped rows of the same owner kind are not artifact-link targets until a future extension adds project-scoped artifact storage/API. Verification bundles use `ArtifactRef.kind=bundle` or `manifest`; export outputs use `ArtifactRef.kind=export_component` or `retention_class=export`. Neither `verification_bundle` nor `export` is an MVP artifact relation record kind.
- `relation.record_kind=projection` is valid only for an already rendered or committed Task-scoped projection output that Core can resolve through `projection_jobs`. In MVP, `record_id_hint` names `projection_jobs.projection_job_id`, and the job's `task_id` must match `relation.task_id`; Core may use `target_ref` and `output_path` to validate the hint, but those values do not replace the job id as identity. Project-level projection jobs may still exist where owner docs allow them, but the current MVP artifact API does not register project-scoped artifact links for them.

Downstream consumers must carry the same meaning. Evidence Manifest, Manual QA, Eval, projection, export, Release Handoff, doctor, and artifact integrity displays may show refs, hashes, safe omission notes, handles, or blocked notices, but they must not inline, reconstruct, summarize, or export omitted or blocked raw values. `secret_omitted` may satisfy claims whose nonsecret evidence remains visible; it must leave claims that require omitted values unsupported or insufficient. `blocked` means the attempted input is unavailable for evidence, QA, verification, projection, export, or Release Handoff until a replacement artifact, compatible waiver, Decision Packet outcome, accepted risk, or other documented resolution closes that path.

Record or projection references use `StateRecordRef`, not `ArtifactRef`:

```yaml
StateRecordRef:
  record_kind: task | change_unit | change_unit_dependency | run | approval | write_authorization | decision_packet | journey_spine_entry | shared_design | domain_term | module_map_item | interface_contract | feedback_loop | residual_risk | evidence_manifest | eval | manual_qa_record | tdd_trace | reconcile_item | projection
  record_id: string
  projection_path: string | null
```

For `record_kind=projection`, `record_id` is the MVP projection identity: `projection_jobs.projection_job_id`. `projection_path` is optional display and recovery metadata; when present, it mirrors or narrows the job's `output_path` and must resolve under the same job. It is not an alternate key and does not imply a separate `projections` table.

MVP has no `accepted_risk` `StateRecordRef.record_kind`. Public fields named `accepted_risk_refs`, `accepted_refs`, or accepted-risk equivalents must use `StateRecordRef` entries with `record_kind=residual_risk`; accepted risk is metadata/state on those Residual Risk records.

Public refs to canonical design-support records use `record_kind=domain_term`, `record_kind=module_map_item`, or `record_kind=interface_contract` with the corresponding storage record id. Use `record_kind=projection` only when the ref targets a rendered Markdown projection such as `DOMAIN-LANGUAGE`, `MODULE-MAP`, or `INTERFACE-CONTRACT`, with `record_id=projection_jobs.projection_job_id`.

Public refs to canonical feedback-loop records use `record_kind=feedback_loop` with the `feedback_loops.feedback_loop_id`. Use `record_kind=tdd_trace` only for the red/green/refactor TDD evidence row; a Feedback Loop may cite a TDD Trace as execution evidence, but the TDD Trace does not replace the selected-loop definition.

Evidence references, approval scope, write authorization, Write Authority Summary display, and end-to-end paths use these shared shapes:

```yaml
EvidenceRefs:
  state_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]

ApprovalScope:
  sensitive_categories: string[]
  allowed_paths: string[]
  allowed_tools: string[]
  allowed_commands: string[]
  allowed_network_targets: string[]
  secret_scope: string[]
  baseline_ref: string | null

WriteAuthorizationSummary:
  write_authorization_id: string
  task_id: string
  change_unit_id: string
  basis_state_version: integer
  intended_operation: string
  intended_paths: string[]
  intended_tools: string[]
  intended_commands:
    - command: string
      command_class: string
      writes_product_files: boolean
  intended_network:
    - target: string
      direction: read | write
  intended_secrets:
    - secret_handle: string
      access_kind: read | write
  sensitive_categories: string[]
  baseline_ref: string | null
  approval_refs: StateRecordRef[]
  decision_packet_refs: StateRecordRef[]
  guarantee_level: cooperative | detective | preventive | isolated
  status: allowed | consumed | expired | stale | revoked
  consumed_by_run_id: string | null
  created_at: string
  consumed_at: string | null

WriteAuthoritySummary:
  active_change_unit_ref: StateRecordRef | null
  write_authorization_ref: StateRecordRef | null
  allowed_paths: string[]
  allowed_tools: string[]
  allowed_commands: string[]
  allowed_command_classes: string[]
  allowed_network_targets: string[]
  secret_scope: string[]
  sensitive_categories: string[]
  approval_status: not_required | required | pending | granted | denied | expired | unknown
  baseline_ref: string | null
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]
  note: "Autonomy Boundary is judgment latitude, not write authority."

EndToEndPath:
  trigger_or_input: string | null
  domain_logic: string | null
  persistence_or_state: string | null
  api_or_caller_boundary: string | null
  ui_or_observable_output: string | null
```

`WriteAuthorizationSummary` and `WriteAuthoritySummary` are API payload shapes only. This document does not define SQLite DDL for Write Authorization records. `WriteAuthoritySummary` is the display/read shape clients use to show the Write Authority Summary beside Autonomy Boundary judgment latitude.

When a client renders guard, freeze, or careful-mode controls, it uses these existing display shapes rather than adding authority fields. `guarantee_display.level` and `guarantee_display.notes` must describe the actual connected capability and current enforcement path. `blocked_reasons[].message` should name the concrete held or blocked condition, such as scope, MCP availability, approval, baseline, or capability, and must not rely on a command label like "guard" or "freeze" to imply a stronger guarantee.

`ProjectionKind` values in the Extension / optional tier, such as `DEC`, `DESIGN`, `EXPORT`, and `JOURNEY-CARD`, are valid projection job kinds only when their projection feature is enabled. MVP-required Decision Packet visibility is provided through `TASK` projections, status/next responses, judgment-context resources, and decision-packet resources. Persisted `JOURNEY-CARD` Markdown remains optional even though current-position Journey Card output is required in status, next, and significant resume flows. Full projection template text lives in [Template Reference](templates/README.md), not this API schema file.

Decision Packet, Write Authorization, Write Authority Summary, Journey Card, Judgment Context, Autonomy Boundary, Recommended Playbook, acceptance visibility, and residual-risk summaries are public MCP schemas. They describe API payloads only; owner docs define the canonical kernel records. `RecommendedPlaybook` is the display-only exception in this list: it has no canonical kernel record, DDL table, task event, or projection job of its own.

Role Lens behavior uses these existing display and routing schemas. A role lens may appear as a `RecommendedPlaybook`, may route to an existing Decision Packet, or may propose a `DecisionPacketCandidate`. It does not introduce a parallel public payload schema, authority record, or state transition.

```yaml
DecisionPacket:
  decision_packet_id: string
  task_id: string
  change_unit_id: string | null
  status: proposed | pending_user | resolved | deferred | rejected | blocked | superseded
  decision_kind: approval | scope_confirmation | design_choice | architecture_choice | product_tradeoff | autonomy_boundary | verification_waiver | qa_waiver | acceptance | residual_risk_acceptance | reconcile
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary
  what_user_is_deciding: string
  what_agent_may_decide_without_user: string[]
  affected_gates:
    - scope_gate | decision_gate | approval_gate | design_gate | evidence_gate | verification_gate | qa_gate | acceptance_gate
  affected_acceptance_criteria:
    - criteria_id: string
      statement: string
  options: DecisionPacketOption[]
  recommendation: DecisionPacketRecommendation | null
  deferral_consequence: string
  user_context: DecisionPacketUserContext
  approval_scope: ApprovalScope | null
  reconcile_item_id: string | null
  created_at: string
  resolved_at: string | null

DecisionPacketOption:
  option_id: string
  label: string
  benefits: string[]
  costs: string[]
  risks: string[]
  reversibility: reversible | partially_reversible | irreversible | unknown
  confidence: low | medium | high
  suitable_when: string[]
  evidence_refs: EvidenceRefs

DecisionPacketRecommendation:
  option_id: string | null
  reason: string
  uncertainty: string | null
  when_to_revisit: string | null

DecisionPacketUserContext:
  minimum_context: string[]
  optional_pull_refs: StateRecordRef[]

DecisionPacketCandidate:
  task_id: string
  change_unit_id: string | null
  decision_kind: approval | scope_confirmation | design_choice | architecture_choice | product_tradeoff | autonomy_boundary | verification_waiver | qa_waiver | acceptance | residual_risk_acceptance | reconcile
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary
  what_user_is_deciding: string
  what_agent_may_decide_without_user: string[]
  affected_gates:
    - scope_gate | decision_gate | approval_gate | design_gate | evidence_gate | verification_gate | qa_gate | acceptance_gate
  affected_acceptance_criteria:
    - criteria_id: string
      statement: string
  options: DecisionPacketOption[]
  recommendation: DecisionPacketRecommendation | null
  deferral_consequence: string
  user_context: DecisionPacketUserContext
  expires_at: string | null
  approval_scope: ApprovalScope | null
  reconcile_item_id: string | null

RecommendedPlaybook:
  playbook_id: string
  label: string
  reason: string
  applies_to:
    focus: status | shaping | decision | implementation | verification | qa | acceptance | reconcile
    state_refs: StateRecordRef[]
  route:
    display_route: continue_guidance | show_existing_decision_packet | propose_decision_packet_request | write_readiness_guidance | evidence_guidance | verification_guidance | manual_qa_guidance | close_readiness_guidance | reconcile_guidance
    decision_packet_ref: StateRecordRef | null
    decision_packet_route: none | existing_decision_packet | decision_packet_candidate_or_request_path
  guidance_refs: StateRecordRef[]
  authority_note: string

JourneyCardSummary:
  task_id: string
  state: StateSummary
  current_position: string
  next_action: string
  recommended_playbooks: RecommendedPlaybook[]
  active_change_unit_ref: StateRecordRef | null
  write_authority_summary: WriteAuthoritySummary | null
  active_decision_packet_refs: StateRecordRef[]
  blocker_refs: StateRecordRef[]
  residual_risk_summary: ResidualRiskSummary | null
  projection_freshness:
    status: current | stale | failed | unknown
    stale_refs: StateRecordRef[]

JudgmentContext:
  task_ref: StateRecordRef
  journey_card: JourneyCardSummary | null
  current_state_summary: StateSummary
  minimum_context: string[]
  relevant_refs: StateRecordRef[]
  evidence_refs: EvidenceRefs
  active_decision_packet_refs: StateRecordRef[]
  optional_pull_refs: StateRecordRef[]
  stale_or_missing_refs: StateRecordRef[]
  acceptance_visibility: AcceptanceVisibilityContext | null

AutonomyBoundarySummary:
  change_unit_id: string | null
  status: absent | proposed | active | exceeded | stale
  autonomy_profile: human_in_loop | afk_eligible | evaluator_only | read_only_advisor | null
  what_agent_may_do: string[]
  what_agent_may_decide_without_user: string[]
  what_requires_user_judgment: string[]
  stop_conditions: string[]
  triggered_stop_conditions: string[]
  related_decision_packet_refs: StateRecordRef[]

ResidualRiskSummary:
  status: none | visible | not_visible | accepted | blocked
  close_relevant_count: integer
  visible_refs: StateRecordRef[]
  not_visible_refs: StateRecordRef[]
  unaccepted_refs: StateRecordRef[]
  accepted_refs: StateRecordRef[]
  summary: string

AcceptanceVisibilityContext:
  residual_risk_summary: ResidualRiskSummary | null
  unaccepted_close_relevant_risk_refs: StateRecordRef[]
  evidence_summary_refs: StateRecordRef[]
  verification_status: not_required | required | pending | passed | failed | waived_by_user | blocked
  qa_status: not_required | required | pending | passed | failed | waived
  acceptance_status: not_required | required | pending | accepted | rejected
  what_acceptance_does_not_replace: string[]
```

`ResidualRiskSummary.status=none` means Core has no known close-relevant Residual Risk for the current Task and requested action. It satisfies residual-risk visibility for acceptance and ordinary successful close, with `close_relevant_count=0` and empty risk-ref arrays. It must not be returned when Core knows of hidden, blocked, or otherwise undisplayed close-relevant risk; those cases use `not_visible` or `blocked`.

`ResidualRiskSummary.visible_refs`, `not_visible_refs`, `unaccepted_refs`, `accepted_refs`, and related acceptance visibility risk-ref arrays contain `StateRecordRef` entries with `record_kind=residual_risk`. `visible_refs` lists close-relevant Residual Risk records visible in the current judgment context; `unaccepted_refs` may overlap with visible risk when risk acceptance is still needed. Accepted risk remains metadata/state on Residual Risk records.

Autonomy Boundary summaries describe judgment latitude, not scope authority. They do not authorize paths, tools, commands, network targets, secret access, or sensitive categories outside the active Change Unit scope and any required approval.

`decision_kind=approval` is retained as a stable public enum value. In both `DecisionPacket` and `DecisionPacketCandidate`, it means an approval-shaped judgment context for sensitive-change approval only. It cannot resolve user-owned judgment such as product trade-offs, design direction, architecture or material technical direction, unresolved security or product-security judgment, QA waiver, verification risk, final acceptance, or residual-risk acceptance unless those decisions are separately represented by compatible Decision Packets and gate updates.

## ValidatorResult

```yaml
ValidatorResult:
  validator_id: string
  validator_kind: state | scope | decision | approval | evidence | verification | qa | acceptance | design | autonomy_boundary | residual_risk | artifact | projection | connector | capability
  status: passed | warning | failed | blocked | skipped
  guarantee_level: cooperative | detective | preventive | isolated
  checked_at: string
  target:
    task_id: string | null
    change_unit_id: string | null
    run_id: string | null
    artifact_id: string | null
  summary: string
  findings:
    - code: string
      severity: info | warning | error | blocker
      message: string
      path: string | null
      artifact_ref: ArtifactRef | null
  blocked_reasons: string[]
  suggested_next_action: string | null
```

The `surface_capability_check` validator uses this schema with `validator_kind=capability`.

Stable MVP validator IDs emitted through `ValidatorResult` are:

- `decision_gate_check`
- `decision_quality_check`
- `autonomy_boundary_check`
- `feedback_loop_check`
- `tdd_trace_required`
- `codebase_stewardship_check`
- `residual_risk_visibility_check`
- `shared_design_alignment`
- `vertical_slice_shape`
- `domain_language_consistency`
- `module_interface_review`
- `manual_qa_required`
- `context_hygiene_check`
- `surface_capability_check`

The agency-critical subset commonly surfaced by status, next, write, and close flows is:

- `decision_quality_check`
- `autonomy_boundary_check`
- `feedback_loop_check`
- `tdd_trace_required`
- `codebase_stewardship_check`
- `residual_risk_visibility_check`
- `context_hygiene_check`

Design-quality validators omitted from this smaller subset, including `shared_design_alignment`, `vertical_slice_shape`, `domain_language_consistency`, and `module_interface_review`, remain part of the full stable MVP ValidatorResult-emitting set above.

Tool descriptions below separate `ValidatorResults emitted` from Core checks/preconditions. Core checks may still block transitions, update gates, populate blocked reasons, or appear in fixture assertions, but they are not validator IDs unless listed above.

## Error taxonomy

| Code | Meaning |
|---|---|
| `STATE_CONFLICT` | `expected_state_version` is stale for the relevant state version scope, lock ownership changed, or the same idempotency key was reused with a different payload |
| `NO_ACTIVE_TASK` | a Task is required but none is active or addressed |
| `NO_ACTIVE_CHANGE_UNIT` | a write-capable operation has no active scoped Change Unit |
| `SCOPE_REQUIRED` | scope confirmation is required before the requested write can proceed |
| `SCOPE_VIOLATION` | intended paths, tools, commands, network, secrets, or categories exceed scope |
| `WRITE_AUTHORIZATION_REQUIRED` | a write-capable run is missing a required Write Authorization from `prepare_write` |
| `WRITE_AUTHORIZATION_INVALID` | the supplied Write Authorization is absent, expired, stale, revoked, already consumed outside idempotent replay, or incompatible with the Task, Change Unit, baseline, intended operation, approval refs, or Decision Packet refs |
| `DECISION_REQUIRED` | blocking user-owned judgment requires a Decision Packet before the requested action can proceed |
| `DECISION_UNRESOLVED` | a relevant Decision Packet is pending, deferred without coverage, rejected, blocked, stale, or incompatible with the requested action |
| `AUTONOMY_BOUNDARY_EXCEEDED` | the intended operation exceeds the active Change Unit Autonomy Boundary |
| `APPROVAL_REQUIRED` | sensitive change requires approval before proceeding |
| `APPROVAL_DENIED` | the relevant approval was denied |
| `APPROVAL_EXPIRED` | approval expired or drifted from baseline/scope |
| `CAPABILITY_INSUFFICIENT` | the connected surface cannot satisfy a required validator or enforcement condition |
| `MCP_UNAVAILABLE` | required MCP access is unavailable, stale, or unreachable |
| `EVIDENCE_INSUFFICIENT` | required evidence coverage is absent, partial, stale, or blocked |
| `VERIFY_NOT_DETACHED` | verification cannot count as detached verification |
| `QA_REQUIRED` | required Manual QA is pending, failed, or missing |
| `ACCEPTANCE_REQUIRED` | required user acceptance is pending or rejected |
| `PROJECTION_STALE` | projection freshness is stale or failed for the requested action |
| `RECONCILE_REQUIRED` | human-editable or managed-block drift requires reconcile |
| `RESIDUAL_RISK_NOT_VISIBLE` | known close-relevant residual risk has not been made visible before acceptance or successful close |
| `ARTIFACT_MISSING` | a referenced artifact file is missing or integrity check failed |
| `BASELINE_STALE` | baseline no longer matches the repository state required by the operation |
| `VALIDATOR_FAILED` | generic fallback when one or more required validators failed and no more specific typed `ErrorCode` applies |

`WRITE_AUTHORIZATION_REQUIRED` and `WRITE_AUTHORIZATION_INVALID` are used only for missing or invalid Write Authorization. Scope violations still use `SCOPE_VIOLATION` when observed paths, tools, commands, network targets, secrets, or sensitive categories exceed authorized or active scope.

`MCP_UNAVAILABLE` remains the stable public `ErrorCode`. Diagnostic detail distinguishes `MCP_SERVER_UNAVAILABLE` from `SURFACE_MCP_UNAVAILABLE` without adding public error codes:

- `MCP_SERVER_UNAVAILABLE`: the tool call cannot reach Core, so no authoritative Core response is possible. The caller must diagnose or reconnect before claiming state changes.
- `SURFACE_MCP_UNAVAILABLE`: Core or an operator can observe that the connected surface lacks usable MCP, has stale MCP configuration, or cannot call required MCP tools. Product writes are held by instruction on cooperative surfaces or blocked before execution only when a stronger guard covers the operation. Core responses may use `MCP_UNAVAILABLE` or `CAPABILITY_INSUFFICIENT` with `details.mcp_unavailable_kind` depending on context.

When a `ToolError` object is available for an MCP availability problem, `details.mcp_unavailable_kind` may be `server_unavailable`, `surface_mcp_unavailable`, `stale_connection`, or `unknown`.

`DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID`, `AUTONOMY_BOUNDARY_EXCEEDED`, `RESIDUAL_RISK_NOT_VISIBLE`, and `MCP_UNAVAILABLE` are stable public `ErrorCode` values. Validator-specific detail still belongs in `ValidatorResult.findings`.

### Primary Error Code Precedence

Public tool responses carry one primary `ToolError.code` even when Core observes multiple blockers. When `ToolResponseBase.errors` is non-empty, `errors[0]` is the primary `ToolError` selected by this precedence table; any remaining entries may represent secondary blockers. Unless a tool subsection defines a narrower order, the primary code is the first applicable code in the precedence list below. Secondary blockers remain in tool-specific fields such as `blocked_reasons`, `CloseTaskResponse.blockers`, validator results, `ToolError.details`, and state summaries.

`Possible errors` lists enumerate admissible codes for a tool. They are not per-tool precedence tables.

If an MCP server or caller cannot reach Core at all, the surface or operator may report `MCP_UNAVAILABLE`, but no authoritative Core response or state mutation can be claimed. Once Core can evaluate the request, apply this order:

| Precedence | Primary `ErrorCode` | Selection note |
|---:|---|---|
| 1 | `STATE_CONFLICT` | stale `expected_state_version`, state lock conflict, or idempotency key reused with a different payload |
| 2 | `MCP_UNAVAILABLE` | required MCP access is unavailable, stale, or unreachable after Core or the operator can classify the availability problem |
| 3 | `NO_ACTIVE_TASK` | the operation requires a Task and none is active or addressed |
| 4 | `NO_ACTIVE_CHANGE_UNIT` | the operation is write-capable or close-relevant and no active scoped Change Unit applies |
| 5 | `BASELINE_STALE` | the requested operation depends on a stale baseline |
| 6 | `SCOPE_REQUIRED` | scope must be confirmed before the requested operation can proceed |
| 7 | `SCOPE_VIOLATION` | the intended or observed paths, tools, commands, network, secrets, or categories exceed active or authorized scope |
| 8 | `WRITE_AUTHORIZATION_REQUIRED` | a write-capable Run is missing a required Write Authorization |
| 9 | `WRITE_AUTHORIZATION_INVALID` | the supplied Write Authorization is stale, expired, revoked, consumed outside replay, or incompatible |
| 10 | `APPROVAL_DENIED` | a relevant sensitive-change approval was denied |
| 11 | `APPROVAL_EXPIRED` | a relevant sensitive-change approval expired or drifted from scope or baseline |
| 12 | `APPROVAL_REQUIRED` | a sensitive change needs approval and no compatible granted approval exists |
| 13 | `DECISION_UNRESOLVED` | an existing relevant Decision Packet is pending, deferred without coverage, rejected, blocked, stale, or incompatible |
| 14 | `AUTONOMY_BOUNDARY_EXCEEDED` | the intended operation exceeds the active Change Unit Autonomy Boundary, even when the next step is a Decision Packet |
| 15 | `DECISION_REQUIRED` | blocking user-owned judgment needs a Decision Packet before the action can proceed |
| 16 | `CAPABILITY_INSUFFICIENT` | the connected surface cannot satisfy a required capability or enforcement condition |
| 17 | `EVIDENCE_INSUFFICIENT` | required evidence coverage is absent, partial, stale, or blocked |
| 18 | `VERIFY_NOT_DETACHED` | verification cannot count as detached verification |
| 19 | `QA_REQUIRED` | required Manual QA is pending, failed, missing, or not validly waived |
| 20 | `RESIDUAL_RISK_NOT_VISIBLE` | known close-relevant residual risk has not been made visible before acceptance or close; not selected when `ResidualRiskSummary.status=none` confirms no known close-relevant risk |
| 21 | `ACCEPTANCE_REQUIRED` | required user acceptance is pending or rejected after residual-risk visibility is satisfied |
| 22 | `PROJECTION_STALE` | projection freshness is stale or failed for the requested action |
| 23 | `RECONCILE_REQUIRED` | human-editable or managed-block drift requires reconcile |
| 24 | `ARTIFACT_MISSING` | a referenced artifact file is missing or failed integrity checks |
| 25 | `VALIDATOR_FAILED` | generic validator fallback selected only when no more specific typed blocker above applies |

#### `harness.close_task` Close Blockers

`harness.close_task` may return multiple close blockers. The primary `ToolError` in `CloseTaskResponse.base.errors` uses the precedence above; when present, `CloseTaskResponse.base.errors[0].code` is the primary close error code. `CloseTaskResponse.blockers` should include the observed close blockers in the same relative order. Residual-risk visibility remains before `ACCEPTANCE_REQUIRED` for close and acceptance flows because required acceptance can be recorded or relied on only after close-relevant residual risk is visible.

## Idempotency

Idempotency keys are scoped to `(project_id, tool_name, idempotency_key)`. Repeating the same payload with the same key returns the original committed response. Reusing a key with a different payload returns `STATE_CONFLICT`.

`request_hash` is computed from canonical JSON encoded as UTF-8. The canonical input includes `tool_name`, the schema-normalized request body, and every `ToolEnvelope` field except `request_id` and `idempotency_key`; the included envelope fields are `expected_state_version`, `project_id`, `task_id`, `surface_id`, `run_id`, `actor_kind`, and `dry_run`. Before hashing, optional fields are normalized according to their request schema defaults and null/empty-field rules, object keys are sorted, arrays remain in schema-defined order unless a schema explicitly marks the array as order-insignificant, and Unicode strings are normalized consistently using NFC.

## State conflict behavior

For state-changing tools, Core compares `expected_state_version` with current project/task state. A mismatch returns `STATE_CONFLICT` and includes the current state version and a status summary in `details`. The caller must refresh state and either retry with a new idempotency key or replay the exact previous request.

State conflict comparison is scope-specific. Core first resolves the primary addressed Task from `ToolEnvelope.task_id`, any tool-specific `task_id`, or active Task resolution. Task-scoped tools compare against that Task's `tasks.state_version`; project-scoped tools with no resolved primary Task compare against `project_state.state_version`. `STATE_CONFLICT.details` should include `scope` (`task` or `project`), `current_state_version`, `expected_state_version`, and the relevant `project_id` plus `task_id` when `scope=task`; it may also include a compact status summary for refresh guidance.

## Public tools

### `harness.status`

Purpose: return project, surface, active Task, Journey Card, gate, guarantee, projection, active Decision Packet, Autonomy Boundary, Write Authority Summary, residual-risk, and pending-decision status.

Allowed actor: `user`, `lead_agent`, `evaluator`, `operator`.

Request schema:

```yaml
StatusRequest:
  envelope: ToolEnvelope
  include:
    task: boolean
    gates: boolean
    projections: boolean
    pending_decisions: boolean
    guarantees: boolean
    journey_card: boolean
    decision_packets: boolean
    autonomy_boundary: boolean
    write_authority: boolean
    residual_risk: boolean
    recommended_playbooks: boolean
```

Response schema:

```yaml
StatusResponse:
  base: ToolResponseBase
  active_task: StateSummary | null
  status_card: string
  journey_card: JourneyCardSummary | null
  pending_decisions: StateRecordRef[]
  active_decision_packet_refs: StateRecordRef[]
  recommended_playbooks: RecommendedPlaybook[]
  autonomy_boundary_summary: AutonomyBoundarySummary | null
  write_authority_summary: WriteAuthoritySummary | null
  residual_risk_summary: ResidualRiskSummary | null
  projection_freshness:
    status: current | stale | failed | unknown
    stale_refs: StateRecordRef[]
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]
```

State transition summary: no state transition.

EventRef values that may be returned: none.

Projection jobs enqueued: none.

`pending_decisions` contains unresolved user-action Decision Packets. `active_decision_packet_refs` contains all Decision Packets relevant to the current phase or requested action, including pending, deferred, blocked, or recently resolved packets. Both fields use `StateRecordRef` entries with `record_kind=decision_packet`.

`recommended_playbooks` is non-authoritative display guidance for the surface or agent stage router, computed from current state and policy/playbook context for status/next display. It may suggest procedures such as shared design, review, TDD, QA, guard checks, release handoff, or browser-QA candidacy. `RecommendedPlaybook.playbook_id` is a stable display/routing string identifier, not a Core-owned closed enum or DDL-backed value set. Known initial IDs include `shared-design`, `product-review`, `eng-review`, `design-review`, `security-review`, `tdd-loop`, `spec-review`, `code-quality-review`, `qa-review`, `guard-check`, `release-handoff`, and `browser-qa-candidate`; this list is not exhaustive for future display/playbook documentation. Recommended Playbook has no canonical kernel record, DDL table, `task_events` entry, or projection job by itself. It never mutates state, authorizes writes, satisfies gates, creates evidence, performs verification, records QA, accepts risk, accepts the result, or closes a Task. If following a recommendation would require user-owned judgment, the route must point to an existing Decision Packet or to the normal Decision Packet candidate/request path before any affected write or close proceeds. `route.display_route` values are display routes, not public tool names and not instructions to call a state-changing tool.

When both `StatusResponse.recommended_playbooks` and `StatusResponse.journey_card.recommended_playbooks` are present, they are the same computed guidance rendered at different display levels. The top-level field is for status surfaces that do not render the full Journey Card; the Journey Card field keeps the same guidance with the current-position summary.

`write_authority_summary` is returned when `include.write_authority=true`. When `include.journey_card=true`, the same current Write Authority Summary display may also appear in `journey_card.write_authority_summary`.

ValidatorResults emitted: optional `surface_capability_check`, optional `decision_gate_check`, optional `autonomy_boundary_check`.

Core checks/preconditions: optional residual-risk visibility read, optional projection freshness read.

Possible errors: `MCP_UNAVAILABLE`, `PROJECTION_STALE`.

Idempotency behavior: read-only; repeated requests do not mutate state.

### `harness.intake`

Purpose: create or resume a Task from user intent and classify it as advisor, direct, or work.

Allowed actor: `user`, `lead_agent`, `operator`.

Request schema:

```yaml
IntakeRequest:
  envelope: ToolEnvelope
  user_request: string
  requested_mode: advisor | direct | work | auto
  resume_policy: resume_active | create_new | supersede_active | reject_if_active
  acceptance_criteria: string[]
  constraints:
    allowed_paths: string[]
    non_goals: string[]
    sensitive_categories: string[]
  initial_context_refs: StateRecordRef[]
```

Response schema:

```yaml
IntakeResponse:
  base: ToolResponseBase
  task_id: string
  created: boolean
  resumed: boolean
  state: StateSummary
  next_action: string
  change_unit_id: string | null
```

State transition summary: creates or resumes a Task; sets `mode` and initial `lifecycle_phase`; may create an initial Change Unit for write-capable direct/work.

Stable EventRef values that may be returned: `task_superseded` when an existing Task is superseded.

Non-stable EventRef values that may be returned for implementation-local detail/audit: `task_intake_recorded`, `task_created`, `task_resumed`, `change_unit_created`.

Projection jobs enqueued: `TASK`; optionally `DOMAIN-LANGUAGE`, `MODULE-MAP`, or `INTERFACE-CONTRACT` if intake accepted design support records.

ValidatorResults emitted: `surface_capability_check`.

Core checks/preconditions: `state_envelope`, `active_task_policy`.

Possible errors: `STATE_CONFLICT`, `MCP_UNAVAILABLE`, `VALIDATOR_FAILED`, `CAPABILITY_INSUFFICIENT`.

Idempotency behavior: same key returns the same Task/resume decision; different payload with same key returns `STATE_CONFLICT`.

### `harness.next`

Purpose: return the next safe action, instruction bundle, and pending decisions for the current Task.

Allowed actor: `user`, `lead_agent`, `evaluator`, `operator`.

Request schema:

```yaml
NextRequest:
  envelope: ToolEnvelope
  task_id: string | null
  focus: status | shaping | decision | implementation | verification | qa | acceptance | reconcile
  include_instruction_bundle: boolean
```

Response schema:

```yaml
NextResponse:
  base: ToolResponseBase
  state: StateSummary | null
  next_action:
    action_kind: ask_user | prepare_write | implement | launch_verify | record_eval | record_manual_qa | request_acceptance | close_task | reconcile | idle
    summary: string
    required_tool: string | null
  recommended_playbooks: RecommendedPlaybook[]
  instruction_bundle:
    summary: string
    constraints: string[]
    relevant_refs: StateRecordRef[]
    artifact_refs: ArtifactRef[]
  pending_decisions: StateRecordRef[]
  judgment_context: JudgmentContext | null
  autonomy_boundary: AutonomyBoundarySummary | null
```

State transition summary: no state transition.

EventRef values that may be returned: none.

Projection jobs enqueued: none.

`pending_decisions` contains unresolved user-action Decision Packets. Deferred, blocked, or recently resolved packets that still affect the current phase or requested action appear through `judgment_context.active_decision_packet_refs`.

`recommended_playbooks` helps the caller choose a procedure for the returned next safe action. It is API/display guidance only, computed from current state and policy/playbook context. `playbook_id` remains a display/routing string identifier, not a canonical kernel enum. It must not update canonical state, append a `task_events` entry, enqueue projection jobs, satisfy `decision_gate`, `approval_gate`, `evidence_gate`, `verification_gate`, `qa_gate`, or `acceptance_gate`, or replace `prepare_write`, evidence, verification, QA, risk acceptance, result acceptance, or close. A playbook recommendation that would introduce user-owned judgment must route to a Decision Packet candidate/request path or existing Decision Packet before any affected write or close proceeds. `route.display_route` values are display routes, not public tool names and not instructions to call a state-changing tool.

When `focus=acceptance`, `judgment_context.acceptance_visibility` must be non-null. It must include the residual-risk summary, unaccepted close-relevant risk refs, evidence summary refs, verification status, QA status, acceptance status, and what acceptance does not replace. The context must distinguish `ResidualRiskSummary.status=none`, meaning no known close-relevant risk exists, from `not_visible`, meaning known close-relevant risk is still hidden. The context must make clear before any acceptance request that acceptance does not replace evidence sufficiency, verification, Manual QA, approval, scope, or residual-risk visibility.

ValidatorResults emitted: optional `surface_capability_check`, optional `decision_gate_check`, optional `autonomy_boundary_check`, optional `context_hygiene_check`.

Possible errors: `NO_ACTIVE_TASK`, `MCP_UNAVAILABLE`, `PROJECTION_STALE`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `RECONCILE_REQUIRED`.

Idempotency behavior: read-only; repeated requests do not mutate state.

### `harness.prepare_write`

Purpose: decide whether an intended product write is allowed before the agent writes.

Allowed actor: `lead_agent`, `operator`.

Request schema:

```yaml
PrepareWriteRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  intended_operation: string
  intended_paths: string[]
  intended_tools: string[]
  intended_commands:
    - command: string
      command_class: string
      writes_product_files: boolean
  intended_network:
    - target: string
      direction: read | write
  intended_secrets:
    - secret_handle: string
      access_kind: read | write
  sensitive_categories: string[]
  baseline_ref: string | null
```

Response schema:

```yaml
PrepareWriteResponse:
  base: ToolResponseBase
  decision: allowed | blocked | approval_required | decision_required | state_conflict
  state: StateSummary | null
  change_unit_id: string | null
  baseline_ref: string | null
  write_authorization_ref: StateRecordRef | null
  write_authorization: WriteAuthorizationSummary | null
  authorization_effect: none | would_create | created | returned
  active_decision_packet_refs: StateRecordRef[]
  blocked_reasons:
    - code: string
      message: string
      related_error: ErrorCode
  approval_request_candidate: ApprovalRequestCandidate | null
  decision_packet_candidate: DecisionPacketCandidate | null
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]

ApprovalRequestCandidate:
  sensitive_categories: string[]
  allowed_paths: string[]
  allowed_tools: string[]
  allowed_commands: string[]
  allowed_network_targets: string[]
  secret_scope: string[]
  baseline_ref: string | null
```

`approval_request_candidate` is present only when `decision=approval_required` or when Core can suggest a new approval request. Otherwise it is `null`. It is a non-mutating candidate for the `approval_scope` of a later `harness.request_user_decision(decision_kind=approval)` call; returning it from `prepare_write` does not create an Approval record, Decision Packet, Write Authorization, or `APR` projection job. If a UI, status response, or next-action response shows this payload before the approval request is committed, it must label it as candidate display, not as an `APR` projection.

When `dry_run=false` and `decision=allowed`, the response must include a non-null `write_authorization_ref`; the `write_authorization` summary may also be returned when the caller requests expanded payloads or the implementation supports it. `authorization_effect` is `created` when Core creates a new authorization.

`WriteAuthorizationSummary.basis_state_version` is the affected-scope state version Core used as the compatibility basis for the allowed write attempt. For MVP prepare-write product writes this is the Task State Version for `task_id`. It is replay and stale-detection audit metadata, not the resulting response `base.state_version`.

`authorization_effect=returned` is reserved for idempotent replay of the same committed `prepare_write` request and response with the same idempotency key, request hash, and `basis_state_version`. A distinct compatible request creates a distinct Write Authorization; compatibility does not make authorizations reusable. Core may stale, expire, or revoke older unconsumed authorizations if their compatibility basis changes.

When `dry_run=true` and the write would otherwise be allowed, Core returns `decision=allowed` with `authorization_effect=would_create`, but `write_authorization_ref` and `write_authorization` must be `null`, and no Write Authorization record, event, artifact, or projection job is created.

For `decision=blocked`, `decision=approval_required`, `decision=decision_required`, and `decision=state_conflict`, both authorization fields must be `null` and `authorization_effect=none`.

A Write Authorization is specific to the intended operation and the current state, baseline, active Change Unit scope, approval refs, Decision Packet refs, sensitive categories, and guarantee level. It is consumed by `harness.record_run` through `write_authorization_id`; it is not a reusable grant.

`active_decision_packet_refs` contains all Decision Packets relevant to the intended write, including pending, deferred, blocked, or recently resolved packets.

`decision_packet_candidate` is present when `decision=decision_required` and no compatible Decision Packet already exists. Its fields match `RequestUserDecisionRequest` after the envelope. It is a non-mutating candidate payload for a later `harness.request_user_decision` call; returning it from `prepare_write` does not create or update a Decision Packet.

State transition summary: may move Task to `executing`, `waiting_user`, or `blocked`; may create a Write Authorization when allowed or return the already committed response for idempotent replay; may set `scope_gate=pending/blocked`, `decision_gate=required/pending/blocked`, `approval_gate=required/expired`, or stale evidence/approval markers. `approval_gate=pending` starts when an approval-shaped Decision Packet and linked pending Approval record are created by `harness.request_user_decision(decision_kind=approval)`.

Stable EventRef values that may be returned: `prepare_write_allowed`, `write_authorization_created`, `write_authorization_returned`, `prepare_write_blocked`, `scope_required`, `decision_required`, `autonomy_boundary_exceeded`, `approval_required`, `baseline_stale_detected`, `capability_insufficient_detected`.

Projection jobs enqueued: `TASK`. `prepare_write` must not enqueue `APR` merely because it returned `decision=approval_required` or an `approval_request_candidate`; `APR` is reserved for the committed Approval record and approval-shaped Decision Packet lifecycle.

ValidatorResults emitted: `autonomy_boundary_check`, `decision_gate_check`, `decision_quality_check`, `feedback_loop_check`, `tdd_trace_required`, `codebase_stewardship_check`, applicable design-quality validators, `surface_capability_check`.

When `tdd_trace_required` applies, `prepare_write` can report a design-policy blocker for non-test implementation writes that have no actual RED evidence and no valid TDD waiver. Test-path writes whose intended operation is to create the failing RED check may still proceed when scope, baseline, approval, Autonomy Boundary, and other required checks pass. A RED target or plan can support that test-path write, but it must not satisfy the RED-evidence precondition for non-test implementation writes or Evidence Manifest coverage. The blocker is represented through validator results, blocked reasons, secondary errors/details as needed, and the primary `ToolError.code` selected by the API precedence table.

Core checks/preconditions: `state_envelope`, `active_task`, `active_change_unit`, `scope_coverage`, `changed_paths_intent`, `baseline_freshness`, `approval_scope`, and design preconditions that apply before write.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `BASELINE_STALE`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `VALIDATOR_FAILED`.

Idempotency behavior: repeated allowed/blocked decision with same payload returns the original decision and event refs; changed payload with same key returns `STATE_CONFLICT`.

#### Approval Lifecycle

Sensitive-change approval follows this recipe:

1. `harness.prepare_write` detects sensitive categories for the intended product write.
2. If no compatible granted Approval covers the scope, baseline, sensitive categories, paths, tools, commands, network targets, secret access, and capability requirements, `prepare_write` returns `decision=approval_required`, includes an `approval_request_candidate`, sets both Write Authorization fields to `null`, uses `authorization_effect=none`, and may update Task blockers plus enqueue `TASK`. It must not create an Approval record, Decision Packet, Write Authorization, or `APR` projection job for this non-mutating candidate.
3. The caller invokes `harness.request_user_decision` with `decision_kind=approval` and an `approval_scope` derived from the candidate and current intended write.
4. Core creates a canonical Decision Packet for the approval-shaped user judgment and a pending Approval record. The response includes both `decision_packet_ref` and `approval_id`, and this committed approval request enqueues `APR`.
5. The user or operator invokes `harness.record_user_decision` for that Decision Packet.
6. Core records the Decision Packet resolution, updates the linked Approval record, recomputes `approval_gate` as granted, denied, or expired, and enqueues `APR` again for the updated approval decision.
7. If the approval was granted, the caller retries `harness.prepare_write` with a fresh idempotency key and the current `expected_state_version`.
8. Only that retry may create a Write Authorization. It succeeds only if the approved scope, baseline, sensitive categories, paths, tools, commands, network targets, secret scope, Decision Packet refs, Approval refs, and capability checks remain compatible with the current intended write.

Approval authorizes sensitive categories inside the defined scope. Approval does not resolve user-owned judgment such as product trade-offs, design direction, architecture or material technical direction, unresolved security or product-security judgment, verification risk, QA waiver, final acceptance, or residual-risk acceptance. If the sensitive action also includes user-owned product, material technical, or architecture judgment, or unresolved security or product-security judgment, Core must require a separate compatible Decision Packet before `prepare_write` can return `allowed`. Approval is not Write Authorization; actual product writes still require an allowed `prepare_write` result and compatible `harness.record_run` consumption of the returned Write Authorization.

### `harness.record_run`

Purpose: record shaping, implementation, direct-result, or verification-input run data, including artifacts and evidence updates.

Allowed actor: `lead_agent`, `evaluator`, `operator`.

Request schema:

```yaml
RecordRunRequest:
  envelope: ToolEnvelope
  kind: shaping_update | implementation | direct | verification_input
  task_id: string
  change_unit_id: string | null
  run_id: string | null
  baseline_ref: string | null
  write_authorization_id: string | null
  summary: string
  artifact_inputs: ArtifactInput[]
  payload: RecordRunPayload

RecordRunPayload:
  shaping_update: ShapingUpdatePayload | null
  implementation: ImplementationPayload | null
  direct: DirectPayload | null
  verification_input: VerificationInputPayload | null

ShapingUpdatePayload:
  task_summary_update: string | null
  acceptance_criteria_updates:
    - criteria_id: string | null
      operation: add | update | remove
      statement: string
  change_unit_updates:
    - operation: create | update | select_active | complete | defer | supersede
      change_unit_id: string | null
      title: string | null
      purpose: string | null
      non_goals: string[]
      slice_type: vertical | enabling | cleanup | horizontal-exception | null
      horizontal_exception_reason: string | null
      follow_up_vertical_change_unit_id: string | null
      allowed_paths: string[]
      allowed_tools: string[]
      allowed_commands: string[]
      allowed_network_targets: string[]
      secret_scope: string[]
      sensitive_categories: string[]
      autonomy_profile: human_in_loop | afk_eligible | evaluator_only | read_only_advisor | null
      agent_may_do: string[]
      user_judgment_required: string[]
      afk_stop_conditions: string[]
      end_to_end_path: EndToEndPath | null
      validator_profile: string[]
      completion_conditions: string[]
      evaluator_focus: string[]
  design_record_refs: StateRecordRef[]
  pending_decision_refs: StateRecordRef[]
  feedback_loop_updates: FeedbackLoopUpdate[]

ImplementationPayload:
  observed_changes: ObservedChanges
  command_results: CommandResult[]
  evidence_updates: EvidenceUpdates
  tdd_trace_update: TddTraceUpdate | null

DirectPayload:
  observed_changes: ObservedChanges
  command_results: CommandResult[]
  evidence_updates: EvidenceUpdates
  self_check_summary: string
  escalation:
    value: none | escalate_to_work
    reason: string | null

VerificationInputPayload:
  evaluator_bundle_input: ArtifactInput | null
  evaluator_focus: string[]
  observed_changes: ObservedChanges
  command_results: CommandResult[]

ObservedChanges:
  changed_paths: string[]
  created_paths: string[]
  deleted_paths: string[]

CommandResult:
  command: string
  exit_code: integer
  artifact_inputs: ArtifactInput[]
  summary: string

EvidenceUpdates:
  acceptance_criteria:
    - criteria_id: string
      status: supported | unsupported | not_applicable
      supporting_refs: StateRecordRef[]
      artifact_inputs: ArtifactInput[]
  feedback_loop_updates: FeedbackLoopUpdate[]

FeedbackLoopUpdate:
  feedback_loop_id: string | null
  operation: create | update
  change_unit_id: string | null
  loop_kind: test | typecheck | lint | build | browser_smoke | manual_qa | tdd | eval | operational | alternate | null
  loop_profile: string | null
  planned_loop: string | null
  selected_loop_refs: StateRecordRef[]
  execution_refs: StateRecordRef[]
  artifact_inputs: ArtifactInput[]
  tdd_trace_refs: StateRecordRef[]
  manual_qa_record_refs: StateRecordRef[]
  evidence_manifest_refs: StateRecordRef[]
  status: defined | executed | waived | blocked | stale | null
  waiver_reason: string | null
  alternate_loop: string | null

TddTraceUpdate:
  tdd_trace_id: string | null
  status: required | recorded | waived | not_required
  red_inputs: ArtifactInput[]
  green_inputs: ArtifactInput[]
  refactor_inputs: ArtifactInput[]
  non_tdd_justification: string | null
```

The `payload` branch must match `kind`; all other branches must be `null` or absent. `ArtifactInput` values are resolved during the same Core transaction; response fields contain the committed `ArtifactRef` values. Change Unit creation and update for MVP happens through `kind=shaping_update` with `change_unit_updates`; `operation=create` creates a `change_units` record, and `operation=select_active` updates the Task's `active_change_unit_id`. `allowed_paths`, `allowed_tools`, `allowed_commands`, `allowed_network_targets`, `secret_scope`, and `sensitive_categories` are scope fields. `autonomy_profile`, `agent_may_do`, `user_judgment_required`, and `afk_stop_conditions` describe Autonomy Boundary judgment latitude only.

Evidence updates that attach `secret_omitted` artifacts may support only the acceptance criteria or completion conditions proven by the remaining visible nonsecret evidence. Evidence updates that attach `blocked` artifacts preserve the attempted capture as a committed metadata-only notice, but the blocked ref does not satisfy evidence that requires the forbidden raw payload; the related Evidence Manifest or gate remains unsupported, partial, blocked, or insufficient until a documented resolution supplies a valid path.

Feedback Loop creation and definition happen through `ShapingUpdatePayload.feedback_loop_updates`. Execution evidence and status updates happen through `EvidenceUpdates.feedback_loop_updates`, or through `harness.record_manual_qa` when Manual QA is the selected loop. `operation=create` creates a canonical `feedback_loops` row and returns a `StateRecordRef` with `record_kind=feedback_loop`; public callers normally leave `feedback_loop_id` null for Core assignment, while executable fixture/import runners may supply a deterministic collision-free `FBL-*` ID. `operation=update` requires `feedback_loop_id` to name an existing feedback-loop row for the same Task and compatible Change Unit. On update, null scalar fields leave stored values unchanged, and ref arrays plus artifact inputs are additive. A TDD Trace may be listed in `tdd_trace_refs` when TDD is selected, but it remains execution evidence and does not replace the Feedback Loop row. If a TDD waiver is recorded, `TddTraceUpdate.non_tdd_justification` records the reason and the related `FeedbackLoopUpdate.alternate_loop` or selected-loop refs record the alternate feedback loop that will supply evidence.

`write_authorization_id` references the compatible Write Authorization returned by `harness.prepare_write`. For `kind=implementation` and `kind=direct`, `write_authorization_id` is required unless the Run records no product write and Core classifies it as read-only evidence or shaping. For `kind=shaping_update`, `write_authorization_id` must be `null`; MVP does not support shaping updates that also record observed product writes, so those writes must be recorded as `kind=implementation` or `kind=direct` with a compatible authorization. For `kind=verification_input`, keep `write_authorization_id` `null`; verification input that creates product writes should normally be disallowed in MVP.

`runs.write_authorization_id` is populated only when a Run successfully consumes a compatible Write Authorization. A violation or audit Run that attempted to use an invalid, stale, missing, consumed, or scope-exceeded authorization must not populate `runs.write_authorization_id` as a consumed authorization. The attempted authorization ref, when useful for audit, should be recorded in validator findings, run violation payload, or `task_events.payload_json`. Such a violation Run may be recorded for audit or recovery if an observed product write already happened, but it must not satisfy evidence sufficiency, detached verification, QA, acceptance, or close readiness. The corresponding Write Authorization should remain unconsumed and may be marked stale, revoked, or expired according to the violation and compatibility basis.

Response schema:

```yaml
RecordRunResponse:
  base: ToolResponseBase
  run_id: string | null
  state: StateSummary
  write_authorization_ref: StateRecordRef | null
  evidence_manifest_ref: StateRecordRef | null
  updated_feedback_loop_refs: StateRecordRef[]
  run_summary_ref: StateRecordRef | null
  direct_result_ref: StateRecordRef | null
  registered_artifacts: ArtifactRef[]
  next_action: string
```

`run_id` is the committed Run ID when Core records a Run. It is `null` when Core rejects the request before any Run is committed, such as a missing Write Authorization for a write-capable implementation or direct Run. In those pre-commit rejection responses, `write_authorization_ref`, `evidence_manifest_ref`, `run_summary_ref`, and `direct_result_ref` remain `null`, while `registered_artifacts` and `updated_feedback_loop_refs` remain empty.

`write_authorization_ref` is non-null only when the committed Run successfully consumes a compatible Write Authorization.

Violation or audit Runs may have a non-null `run_id` only when Core deliberately records such a Run, for example after an observed product write already happened. Rejected pre-commit cases must not fabricate a Run ID.

State transition summary: shaping updates can keep `shaping`, move to `ready`, or move to `waiting_user`; implementation moves toward `verifying`; direct can become close-eligible or escalate to work; verification input records evaluator bundle context without proving detached verification.

Stable EventRef values that may be returned: `run_recorded`, `write_authorization_consumed`, `write_authorization_violation_detected`, `write_authorization_staled`, `write_authorization_revoked`, `write_authorization_expired`, `scope_violation_detected`, `evidence_manifest_updated`.

Non-stable EventRef values that may be returned for implementation-local detail/audit: `shaping_updated`, `implementation_recorded`, `direct_result_recorded`, `verification_input_recorded`, `artifact_registered`, `feedback_loop_updated`, `tdd_trace_updated`.

Violation or audit Runs may emit `write_authorization_violation_detected`, `write_authorization_staled`, `write_authorization_revoked`, `write_authorization_expired`, or `scope_violation_detected` for audit and recovery. Those Runs cannot satisfy evidence sufficiency, detached verification, QA, acceptance, or close readiness. Pre-commit rejection responses return no stable EventRef values from `record_run`.

Projection jobs enqueued for committed Run responses: `TASK`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`; `DIRECT-RESULT` for `kind=direct`; `TDD-TRACE` when updated. Pre-commit rejection responses enqueue no projection jobs.

ValidatorResults emitted: `decision_quality_check`, `autonomy_boundary_check`, `feedback_loop_check`, `tdd_trace_required`, `codebase_stewardship_check`, applicable design-quality validators, `surface_capability_check`.

Core checks/preconditions: `state_envelope`, `changed_paths`, `scope_coverage`, `approval_scope`, `baseline_freshness`, `artifact_integrity`, `evidence_sufficiency`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID`, `SCOPE_VIOLATION`, `APPROVAL_REQUIRED`, `APPROVAL_EXPIRED`, `ARTIFACT_MISSING`, `BASELINE_STALE`, `EVIDENCE_INSUFFICIENT`, `VALIDATOR_FAILED`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated request returns the same run, artifact records, evidence updates, events, and projection jobs; artifact inputs and resolved artifact refs must match the original payload.

### `harness.request_user_decision`

Purpose: create a structured Decision Packet for a user judgment that blocks progress, write, close, risk acceptance, waiver, or reconcile.

Allowed actor: `lead_agent`, `evaluator`, `operator`.

Request schema:

```yaml
RequestUserDecisionRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  decision_kind: approval | scope_confirmation | design_choice | architecture_choice | product_tradeoff | autonomy_boundary | verification_waiver | qa_waiver | acceptance | residual_risk_acceptance | reconcile
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary | null
  what_user_is_deciding: string
  what_agent_may_decide_without_user: string[]
  affected_gates:
    - scope_gate | decision_gate | approval_gate | design_gate | evidence_gate | verification_gate | qa_gate | acceptance_gate
  affected_acceptance_criteria:
    - criteria_id: string
      statement: string
  options: DecisionPacketOption[]
  recommendation: DecisionPacketRecommendation | null
  deferral_consequence: string
  user_context: DecisionPacketUserContext
  expires_at: string | null
  approval_scope: ApprovalScope | null
  reconcile_item_id: string | null
```

Core stores a canonical `DecisionPacket`. Minimal MVP implementations may omit `decision_requests`; public request and response schemas remain centered on Decision Packet, not Decision Request. If the implementation also creates or updates `decision_requests`, those rows are routing, interaction, idempotency replay, or legacy handoff metadata only, and they must link back to the canonical `decision_packet_id` before gate aggregation can consider their metadata. A `decision_request` row alone never satisfies `decision_gate`, approval, acceptance, waiver, residual-risk acceptance, or close. If `state_summary_at_request` is `null`, Core derives it from current state during the same transaction. The stored `state_summary_at_request` is a request-time snapshot and is not updated by later Task transitions. `approval_scope` is required when `decision_kind=approval`; for all other `decision_kind` values it must be `null` or omitted. `decision_kind=approval` is only the approval-shaped sensitive-change context and cannot resolve user-owned judgment such as product trade-offs, design direction, architecture or material technical direction, unresolved security or product-security judgment, QA waiver, verification risk, final acceptance, or residual-risk acceptance without separate compatible Decision Packets and gate updates. For `decision_kind=approval`, Core also creates a linked pending Approval record using the approval scope; the Approval is not granted until `harness.record_user_decision` resolves the Decision Packet. A `residual_risk_acceptance` packet must include the risk visibility context in `user_context.minimum_context` and relevant risk refs in `context.source_refs`.

Response schema:

```yaml
RequestUserDecisionResponse:
  base: ToolResponseBase
  decision_packet_id: string
  decision_packet_ref: StateRecordRef
  decision_packet: DecisionPacket
  approval_id: string | null
  reconcile_item_id: string | null
  state: StateSummary
  user_visible_summary: string
```

`pending_decisions` returned by status and next-action responses contain unresolved user-action `StateRecordRef` entries with `record_kind=decision_packet`. `active_decision_packet_refs` fields include all Decision Packets relevant to the current phase or requested action, including pending, deferred, blocked, or recently resolved packets.

State transition summary: records a pending Decision Packet and usually moves Task to `waiting_user`; decision-gate judgment, such as a user-owned product trade-off or material technical/architecture choice, sets `decision_gate=pending`; approval requests create a pending Approval record and set `approval_gate=pending`; scope confirmation sets `scope_gate=pending`; acceptance and residual-risk acceptance set or keep `acceptance_gate=pending` when acceptance is required.

Non-stable EventRef values that may be returned for implementation-local detail/audit: `decision_packet_created`, `user_decision_requested`, `approval_requested`, `scope_confirmation_requested`, `design_choice_requested`, `architecture_choice_requested`, `autonomy_boundary_decision_requested`, `verification_waiver_requested`, `qa_waiver_requested`, `acceptance_requested`, `residual_risk_acceptance_requested`, `reconcile_decision_requested`.

Projection jobs enqueued: `TASK`; `DEC` when standalone Decision Packet projection is enabled; `APR` only for `decision_kind=approval` after Core creates the canonical approval-shaped Decision Packet and linked pending Approval record; affected projection for reconcile.

ValidatorResults emitted: `decision_quality_check`, `autonomy_boundary_check` when the packet affects the active Change Unit boundary, `residual_risk_visibility_check` for risk-acceptance decisions.

Core checks/preconditions: `state_envelope`, `decision_packet_validity`, `approval_scope` for approval decisions, `reconcile_required` for reconcile decisions.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `DECISION_REQUIRED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_REQUIRED`, `RECONCILE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `PROJECTION_STALE`, `VALIDATOR_FAILED`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated request returns the same Decision Packet, related records, events, and projection jobs; a different packet payload with the same key returns `STATE_CONFLICT`.

### `harness.record_user_decision`

Purpose: record the user's answer to a pending Decision Packet and optionally record accepted residual risk.

Allowed actor: `user`, `operator`.

Request schema:

```yaml
RecordUserDecisionRequest:
  envelope: ToolEnvelope
  decision_packet_id: string
  decision_kind: approval | scope_confirmation | design_choice | architecture_choice | product_tradeoff | autonomy_boundary | verification_waiver | qa_waiver | acceptance | residual_risk_acceptance | reconcile
  selected_option_id: string
  decision: RecordUserDecisionPayload
  note: string
  waiver_reason: string | null
  accepted_risks: AcceptedRiskInput[]

RecordUserDecisionPayload:
  approval:
    value: granted | denied | expired
  scope_confirmation:
    value: confirmed | rejected | revise_scope
  design_choice:
    value: selected | rejected | defer
  architecture_choice:
    value: selected | rejected | defer
  product_tradeoff:
    value: selected | rejected | defer
  autonomy_boundary:
    value: accepted | rejected | revise_boundary | defer
  verification_waiver:
    value: waived | rejected
  qa_waiver:
    value: waived | rejected
  acceptance:
    value: accepted | rejected
  residual_risk_acceptance:
    value: accepted | rejected | defer
  reconcile:
    value: merge | reject | convert_to_note | create_decision | defer

AcceptedRiskInput:
  residual_risk_ref: StateRecordRef | null
  risk_summary: string
  accepted_scope: string[]
  acceptance_consequence: string
  follow_up_required: boolean
  follow_up: string | null
  evidence_refs: EvidenceRefs
```

The payload branch must match `decision_kind`; other branches must be absent. `accepted_risks` is allowed only when the Decision Packet and current Judgment Context made the close-relevant residual risk visible before the user decision. For `decision_kind=acceptance`, Core may record acceptance only when close-relevant residual risk is visible or `ResidualRiskSummary.status=none` confirms no known close-relevant risk. Core records the answer against the canonical `DecisionPacket` identified by `decision_packet_id`; any `decision_requests` row is updated only as routing/replay metadata and cannot satisfy `decision_gate`, approval, acceptance, waiver, residual-risk acceptance, or close without the linked compatible Decision Packet and owner-record updates. Core records accepted risk by updating Residual Risk records and returning residual-risk state refs; it does not treat risk acceptance as detached verification. `AcceptedRiskInput.residual_risk_ref=null` is allowed only when the current Decision Packet and Judgment Context already made that close-relevant risk visible to the user and include enough source and evidence context for Core to create or associate a Residual Risk record in the same committed transition. If visibility or context is absent, Core must reject or block instead of silently creating and accepting a hidden risk.

Response schema:

```yaml
RecordUserDecisionResponse:
  base: ToolResponseBase
  decision_packet_id: string
  decision_packet_ref: StateRecordRef
  state: StateSummary
  updated_records: StateRecordRef[]
  accepted_risk_refs: StateRecordRef[]
  next_action: string
```

`RecordUserDecisionResponse.accepted_risk_refs` contains only `StateRecordRef` entries with `record_kind=residual_risk`; there is no standalone accepted-risk record kind.

State transition summary: resolves, defers, rejects, or blocks the targeted Decision Packet; updates affected gates or reconcile item; approval grant/deny updates the linked Approval record and `approval_gate`, but does not create a Write Authorization; accepted scope updates `scope_gate`; user-resolved decision-gate judgment, such as a user-owned product trade-off or material technical/architecture choice, updates `decision_gate`; accepted Autonomy Boundary decisions may update the active Change Unit boundary; verification waiver updates `verification_gate=waived_by_user`; QA waiver updates `qa_gate`; acceptance records the user decision on the Decision Packet and updates `acceptance_gate`; accepted residual risk updates Residual Risk records and returns their refs without upgrading assurance; reconcile may create accepted state records.

Non-stable EventRef values that may be returned for implementation-local detail/audit: `user_decision_recorded`, `decision_packet_resolved`, `decision_packet_deferred`, `decision_packet_rejected`, `approval_granted`, `approval_denied`, `scope_confirmed`, `scope_rejected`, `design_choice_recorded`, `architecture_choice_recorded`, `autonomy_boundary_decision_recorded`, `verification_waiver_recorded`, `qa_waiver_recorded`, `acceptance_recorded`, `residual_risk_accepted`, `reconcile_resolved`.

Projection jobs enqueued: `TASK`; `DEC` when standalone Decision Packet projection is enabled; `APR` when the targeted Decision Packet is approval-shaped and the linked Approval record is updated; `MANUAL-QA` for QA waiver when represented as a QA record; affected design/task projections for reconcile. Decision Packet visibility still appears through `TASK` projections, status/next responses, judgment-context resources, and decision-packet resources.

ValidatorResults emitted: `decision_quality_check`, `autonomy_boundary_check`, `residual_risk_visibility_check`.

Core checks/preconditions: `state_envelope`, `pending_decision_packet_exists`, `approval_scope`, `qa_waiver_reason`, `reconcile_target_validity`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_UNRESOLVED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `SCOPE_VIOLATION`, `QA_REQUIRED`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `RECONCILE_REQUIRED`, `PROJECTION_STALE`, `VALIDATOR_FAILED`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated decision returns the same Decision Packet resolution, accepted-risk refs, updated records, and events; attempting to change an already-recorded decision with the same key returns `STATE_CONFLICT`.

### `harness.launch_verify`

Purpose: create a detached verification run or manual evaluator bundle.

Allowed actor: `lead_agent`, `operator`.

Request schema:

```yaml
LaunchVerifyRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  verification_mode: fresh_session | fresh_worktree | sandbox | manual_bundle
  evaluator_surface_id: string | null
  baseline_ref: string
  include_artifacts: ArtifactRef[]
  bundle_artifact_input: ArtifactInput | null
  evaluator_focus: string[]
```

`include_artifacts` references already registered evidence to include in or link from the bundle. `bundle_artifact_input` is optional; when it is `null`, Core assembles and registers the verification bundle. When it is present, Core validates and registers the supplied staged bundle instead. `secret_omitted` entries are included as refs plus omission notes or handles; `blocked` entries are included only as unavailable-input notices and may cause `EVIDENCE_INSUFFICIENT` unless the verification path records a replacement, waiver, Decision Packet outcome, accepted risk, or other documented resolution.

The returned `bundle_ref` is an `ArtifactRef`, usually with `kind=bundle` or `kind=manifest`. Its artifact link must point to an existing owner record such as the Task, launching Run, Evidence Manifest, Eval, or a rendered Task-scoped projection; it does not create a `verification_bundle` state record.

Response schema:

```yaml
LaunchVerifyResponse:
  base: ToolResponseBase
  evaluator_run_id: string | null
  bundle_ref: ArtifactRef
  state: StateSummary
  evaluator_instructions: string
  independence_expected:
    context: fresh_session | fresh_worktree | sandbox | manual_bundle
    write_capable: boolean
```

State transition summary: records verification launch, sets or keeps `verification_gate=pending`, and creates evaluator run/bundle references.

Non-stable EventRef values that may be returned for implementation-local detail/audit: `verification_launched`, `verification_bundle_created`, `evaluator_run_created`.

Projection jobs enqueued: `TASK`; optionally `EVIDENCE-MANIFEST`.

ValidatorResults emitted: `surface_capability_check`.

Core checks/preconditions: `state_envelope`, `evidence_sufficiency`, `baseline_freshness`, `artifact_integrity`, `same_session_verify_guard`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `EVIDENCE_INSUFFICIENT`, `BASELINE_STALE`, `ARTIFACT_MISSING`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `VALIDATOR_FAILED`.

Idempotency behavior: repeated request returns the same evaluator run and bundle ref; included artifact refs and bundle artifact input must match the original payload, and staged bundle contents must be byte-identical for the same key.

### `harness.record_eval`

Purpose: record a verification result and update verification gate/assurance when independence is valid.

Allowed actor: `evaluator`, `operator`.

Request schema:

```yaml
RecordEvalRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  evaluator_run_id: string | null
  target_run_id: string | null
  verdict: passed | failed | blocked | inconclusive
  checks_performed:
    - check_id: string
      result: passed | failed | skipped | blocked
      summary: string
  evidence_reviewed:
    state_refs: StateRecordRef[]
    artifact_refs: ArtifactRef[]
  independence:
    context: same_session | subagent_context | fresh_session | fresh_worktree | sandbox | manual_bundle
    write_capable: boolean
    baseline_reverified: boolean
    evaluator_surface_id: string
    parent_run_id: string | null
  blockers: string[]
  artifact_inputs: ArtifactInput[]
```

Core may derive `change_unit_id` from `target_run_id` or the evidence bundle when it is omitted, but supplying it explicitly improves projection and template alignment when the Eval applies to a Change Unit.

Eval evidence review must preserve artifact redaction semantics. A `secret_omitted` artifact can support an Eval finding only for visible nonsecret facts. A `blocked` artifact is reviewed as an unavailable input notice, not as raw evidence; an Eval that depends on the blocked payload must be `blocked` or `inconclusive`, or return `EVIDENCE_INSUFFICIENT`, until a valid replacement or documented resolution exists.

Response schema:

```yaml
RecordEvalResponse:
  base: ToolResponseBase
  eval_id: string
  state: StateSummary
  assurance_updated: boolean
  eval_ref: StateRecordRef
  registered_artifacts: ArtifactRef[]
  next_action: string
```

State transition summary: records Eval; passed detached verification can set `verification_gate=passed` and `assurance_level=detached_verified`; failed or blocked Eval moves gate to failed/blocked; same-session or invalid independence cannot upgrade assurance.

Stable EventRef values that may be returned: `eval_recorded`, `verification_passed`, `verify_not_detached_detected`.

Non-stable EventRef values that may be returned for implementation-local detail/audit: `verification_failed`, `verification_blocked`, `assurance_updated`.

Projection jobs enqueued: `TASK`, `EVAL`; optionally `EVIDENCE-MANIFEST`.

ValidatorResults emitted: `surface_capability_check`.

Core checks/preconditions: `state_envelope`, `same_session_verify_guard`, `baseline_freshness`, `artifact_integrity`, `evidence_sufficiency`, `approval_scope`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `VERIFY_NOT_DETACHED`, `EVIDENCE_INSUFFICIENT`, `BASELINE_STALE`, `ARTIFACT_MISSING`, `VALIDATOR_FAILED`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated request returns the same Eval and assurance decision; a changed verdict, independence payload, or artifact input with the same key returns `STATE_CONFLICT`.

### `harness.record_manual_qa`

Purpose: record an individual human QA outcome and update `qa_gate` when required QA is satisfied, failed, or waived.

Allowed actor: `user`, `operator`, `evaluator`.

Request schema:

```yaml
RecordManualQaRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  qa_profile: ui_quality | workflow | copy | accessibility | browser_smoke | performance_smoke | other
  performed_by: string
  result: passed | failed | waived
  findings:
    - severity: info | warning | error | blocker
      summary: string
      path: string | null
  artifact_inputs: ArtifactInput[]
  waiver_reason: string | null
  waiver_decision_packet_ref: StateRecordRef | null
  feedback_loop_ref: StateRecordRef | null
  next_action: rework | accept | waive | block | none
```

`change_unit_id` should be supplied when Manual QA applies to a Change Unit. It may be `null` for Task-level QA that is not scoped to a single Change Unit.

`RecordManualQaRequest.result` is the record-level result for an actual Manual QA record and is limited to `passed`, `failed`, or `waived`. Pending required QA is represented by the aggregate `qa_gate=pending`, not by `RecordManualQaRequest.result=pending`.

For `result=waived`, product/user risk or policy-required judgment requires a `qa_waiver` Decision Packet referenced by `waiver_decision_packet_ref`. `waiver_reason` alone is allowed only for a low-risk waiver when policy permits it.

When Manual QA is the selected Feedback Loop, `feedback_loop_ref` should reference the canonical `feedback_loops` row with `record_kind=feedback_loop`. Core records the Manual QA row, appends the resulting Manual QA ref and registered artifacts to that Feedback Loop, and updates its status to `executed`, `blocked`, or `waived` according to the QA result. This link updates execution evidence only; it does not create the selected-loop definition.

Manual QA artifact refs follow the same downstream rule as other evidence. `secret_omitted` QA artifacts may support observable workflow or UI findings while leaving omitted values unproven. `blocked` QA capture artifacts mark the screenshot, log, trace, or recording input unavailable; the QA record or aggregate `qa_gate` must show blocked, failed, pending, waived, or otherwise unresolved impact unless a replacement capture, waiver, Decision Packet outcome, accepted risk, or documented fallback resolves the QA path.

Response schema:

```yaml
RecordManualQaResponse:
  base: ToolResponseBase
  manual_qa_record_id: string
  state: StateSummary
  manual_qa_ref: StateRecordRef
  updated_feedback_loop_refs: StateRecordRef[]
  registered_artifacts: ArtifactRef[]
  next_action: string
```

State transition summary: records Manual QA; `passed` can set `qa_gate=passed`; `failed` sets `qa_gate=failed` and routes to rework/blocked; `waived` requires either a compatible `qa_waiver` Decision Packet or a policy-permitted low-risk waiver reason and sets `qa_gate=waived`. If required QA has not produced a satisfying record, or the latest relevant record does not satisfy policy, the aggregate gate remains `qa_gate=pending`.

Non-stable EventRef values that may be returned for implementation-local detail/audit: `manual_qa_recorded`, `qa_passed`, `qa_failed`, `qa_waived`, `artifact_registered`, `feedback_loop_updated`.

Projection jobs enqueued: `TASK`, `MANUAL-QA`; `DEC` when standalone Decision Packet projection is enabled and a waiver Decision Packet affects visibility; optionally `EVIDENCE-MANIFEST`. Waiver Decision Packet visibility still appears through `TASK` projections, status/next responses, judgment-context resources, and decision-packet resources.

ValidatorResults emitted: `manual_qa_required`, `decision_quality_check`, `residual_risk_visibility_check`.

Core checks/preconditions: `state_envelope`, `qa_waiver_reason`, `artifact_integrity`, `evidence_sufficiency`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `QA_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `ARTIFACT_MISSING`, `EVIDENCE_INSUFFICIENT`, `VALIDATOR_FAILED`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated request returns the same Manual QA record and gate update; waiver reason and artifact inputs must match.

### `harness.close_task`

Purpose: close, cancel, or supersede a Task after Core checks all close-relevant gates.

Allowed actor: `user`, `lead_agent`, `operator`.

Request schema:

```yaml
CloseTaskRequest:
  envelope: ToolEnvelope
  task_id: string
  intent: complete | cancel | supersede
  requested_close_reason: completed_verified | completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  user_note: string | null
  superseded_by_task_id: string | null
```

`CloseTaskRequest` does not carry accepted-risk refs. For `completed_with_risk_accepted`, Core reads already-recorded accepted state from close-relevant Residual Risk records and blocks if visible accepted residual-risk state is missing.

Response schema:

```yaml
CloseTaskResponse:
  base: ToolResponseBase
  closed: boolean
  state: StateSummary
  blockers:
    - code: ErrorCode
      message: string
      required_next_action: string
      related_refs: StateRecordRef[]
  final_report_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]
```

Close blockers include unresolved, missing, deferred-without-coverage, blocked, rejected, stale, or incompatible blocking Decision Packets, and known close-relevant residual risk that is not visible before any successful close. If no known close-relevant residual risk exists, `ResidualRiskSummary.status=none` satisfies residual-risk visibility and is not a close blocker. A risk-accepted close additionally requires visible and accepted Residual Risk refs. Acceptance, when required, can be recorded only after close-relevant residual risk is visible or confirmed as `ResidualRiskSummary.status=none`.

State transition summary: successful completion moves Task to `completed` with result and close reason; cancellation/supersession moves Task to `cancelled`; failed close leaves Task non-terminal and reports blockers.

Stable EventRef values that may be returned: `close_requested`, `task_closed`, `task_cancelled`, `task_superseded`, `risk_accepted_close_recorded`, `close_blocked`.

Projection jobs enqueued: `TASK`; latest required reports as needed for final freshness.

ValidatorResults emitted: `decision_gate_check`, `decision_quality_check`, `autonomy_boundary_check`, `feedback_loop_check`, `tdd_trace_required`, `codebase_stewardship_check`, `manual_qa_required`, `residual_risk_visibility_check`, `context_hygiene_check` when projection or context hygiene must be emitted as a ValidatorResult.

Core checks/preconditions: `state_envelope`, `active_run_absent`, `active_change_unit_complete`, `scope_coverage`, `approval_scope`, `design_gate_close`, `evidence_sufficiency`, `same_session_verify_guard`, `acceptance_required`, `projection_freshness`.

Possible errors: `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `EVIDENCE_INSUFFICIENT`, `VERIFY_NOT_DETACHED`, `QA_REQUIRED`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `PROJECTION_STALE`, `RECONCILE_REQUIRED`, `ARTIFACT_MISSING`, `BASELINE_STALE`, `VALIDATOR_FAILED`, `MCP_UNAVAILABLE`.

Idempotency behavior: repeated successful close returns the same terminal state and report refs; a second close with a different intent or close reason returns `STATE_CONFLICT`.
