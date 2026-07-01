# API state schemas

This document owns API state-shaped schemas for the baseline scope. It defines public response shapes for `StateSummary`, `StateRecordRef`, lifecycle state as API data, state-related snapshots, `ProjectContinuityRecord`, `ProjectContinuitySummary`, `ShapingReadiness`, `ChangeUnitEffectContract`, and display shapes such as `NextActionSummary`, `WriteCheckStateSummary`, `WriteCheckSummary`, `WriteCheckAttemptScope`, `EvidenceSummary`, `EvidenceObservation`, `GuardHealthSummary`, `UnrecordedChangeFinding`, `UnrecordedChangeResolutionSummary`, `CurrentCloseBasis`, `ResidualRisk`, `RiskAcceptanceCoverage`, `CloseReadinessBlocker`, `ValidatorResult`, and `GuaranteeDisplay`.

## Owner boundary

This document owns state-shaped API fields, nesting, references, summaries, snapshots, display shapes, and the boundary between field presence and response effects. Neighboring contracts remain with these owners:

| Neighboring contract | Owner |
|---|---|
| Common envelopes and response branches | [API Schema Core](schema-core.md) |
| Supported enum-like values | [API Value Sets](schema-value-sets.md) |
| Method behavior | [API Methods](methods.md) and method owner documents |
| Public error semantics | [API error codes](error-codes.md) and [API error routing](error-routing.md) |
| Core lifecycle and close-readiness product meaning | [Core Model](../core-model.md) |
| Storage records and persistence effects | [Storage Records](../storage-records.md) and [Storage Effects](../storage-effects.md) |

## Boundary

State schemas describe API data shapes only. A state-shaped field does not choose a response branch or create persistence, Core transitions, replay rows, `task_events`, artifact effects, `Write Check` effects, or a `state_version` increment.

State projections must be truthful about computed state:
- A `null` or omitted field means the method did not select a value, the value is unavailable, or the owning schema explicitly allows absence. It must not be replaced with an empty value that implies "computed and none."
- Empty arrays such as `close_blockers: []` or `risk_acceptance_coverage: []` mean the relevant computation ran and found no entries.
- Mutation results and `volicord.status` projections must describe the same current state where their schemas overlap.
- Computed blockers use the same close-readiness calculation as the shared close-readiness engine; method owners decide only whether a branch persists an effect.

Owner links:
- Response branch selection: [Common response branches](schema-core.md#common-response)
- Method behavior and effects: [API Methods](methods.md) and method owner documents

<a id="state-references"></a>
## State references

Meaning:
- `StateRecordRef` is the common public reference shape for Core-owned records that appear in API responses.
- `record_kind` is a controlled value string.
- `record_id`, `project_id`, and `task_id` are opaque identifiers.

It is a public reference, not an embedded storage row.

```yaml
StateRecordRef:
  record_kind: string
  record_id: string
  project_id: string
  task_id: string | null
  state_version: integer | null
```

Owner links:
- `record_kind` values: [record and reference values](schema-value-sets.md#record-and-reference-values)
- storage record families and values: [Storage Records](../storage-records.md)
- storage table names and DDL: [Storage DDL](../storage-ddl.md)

## `StateSummary`

`StateSummary` is the compact current-position state returned by supported methods that need to show the current Task path.

```yaml
StateSummary:
  project_id: string
  state_version: integer
  task_ref: StateRecordRef | null
  mode: string | null
  lifecycle: TaskLifecycleState | null
  goal_summary: string | null
  scope_summary: string | null
  non_goals: string[]
  acceptance_criteria: string[]
  autonomy_boundary: string | null
  active_change_unit_ref: StateRecordRef | null
  effect_contract: ChangeUnitEffectContract | null
  baseline_ref: string | null
  shaping_readiness: ShapingReadiness | null
  pending_user_judgment_refs: StateRecordRef[]
  blocker_refs: StateRecordRef[]
  write_check_summary: WriteCheckStateSummary | null
  evidence_summary: EvidenceSummary | null
  close_state: string | null
  close_blockers: CloseReadinessBlocker[]
  guard_health: GuardHealthSummary | null
  guarantee_display: GuaranteeDisplay | null
```

Meaning:
- `StateSummary` is a compact response shape for state references, summaries, and close-readiness fields.
- Method include flags may select only part of this shape. When a method owner says a projection is not selected, include-controlled fields such as `evidence_summary`, `close_state`, `close_blockers`, `guard_health`, or `guarantee_display` are omitted instead of being returned as null or empty. A returned empty array means the projection was computed and found empty.
- `mode` and `close_state` are controlled value strings when present.
- `goal_summary`, `scope_summary`, `non_goals`, `acceptance_criteria`, and `autonomy_boundary` are free-form display strings.
- `effect_contract` is the current Change Unit's optional extra effect contract. `null` means no extra Change Unit effect contract is recorded; it must not be described as broad safety or unrestricted execution.
- `baseline_ref` is an opaque baseline identifier.
- `pending_user_judgment_refs` lists current pending judgments relevant to the response view. A pending judgment is operation-blocking only when its `required_for` target, judgment kind, Task, Change Unit, affected refs, and basis are compatible with that operation.

Does not imply:
- `StateSummary` field presence does not define whether a method committed.

Owner links:
- `mode` and `close_state` values: [task lifecycle values](schema-value-sets.md#task-lifecycle-values)
- Commit decision branch: [Common response branches](schema-core.md#common-response)
- Method-specific commit behavior: method owner documents routed from [API Methods](methods.md)

## Guard health summary

`GuardHealthSummary` is the compact guard-health projection returned by close-readiness and status views when the method owner selects it.

```yaml
GuardHealthSummary:
  guard_mode: string
  guard_strength: string
  guard_installation_id: string | null
  guard_installation_status: string
  guard_configuration_status: string
  guard_observation_status: string
  effective_guard_status: string
  generated_config_verified: boolean
  native_host_output_adapter_verified: boolean
  hook_path_safety: string
  hook_commands_cwd_independent: boolean
  hook_commands_subdirectory_safe: boolean
  pre_tool_blocking_available: boolean
  post_tool_correlation_available: boolean
  bash_shell_mutation_coverage: boolean
  direct_file_write_matcher_coverage: boolean
  bypass_detection_active: boolean
  guard_hook_observed: boolean
  last_guard_observed_at: string | null
  last_guard_event_at: string | null
  host_kind: string | null
  observed_hook_phase: string | null
  observed_host_kind: string | null
  expected_policy_hash: string | null
  observed_policy_hash: string | null
  observed_binary_version: string | null
  required_hook_phases: string[]
  missing_required_hook_phases: string[]
  prompt_capture_status: string
  prompt_capture_available: boolean
  local_web_consent_available: boolean
  managed_distribution_verified: boolean
  mcp_connection_healthy: boolean
  mcp_connection_status: string | null
  session_watch_status: string
  last_session_watch_checked_at: string | null
  session_watch_baseline_created_at: string | null
  session_watch_coverage_start_at: string | null
  session_watch_coverage_basis: string | null
  session_watch_partial_coverage_warning: string | null
  session_watch_detail: string | null
  unresolved_unrecorded_change_count: integer
  missing_or_stale_write_readiness: boolean
```

Meaning:
- `guard_mode` and `guard_installation_status` are controlled value strings.
- `guard_strength` is the derived guard-strength label for the selected connection or session. It reports the strongest currently supported guard path from recorded mode, hook health, hook command path safety, runtime observation health, session watcher status, prompt-capture availability, local web consent availability, and managed-distribution verification.
- `guard_installation_id`, when non-null, is an opaque guard-installation identifier.
- `guard_configuration_status`, `guard_observation_status`, and `effective_guard_status` separate file/config health, runtime hook observation, and the effective guarded close-readiness status.
- `generated_config_verified`, `native_host_output_adapter_verified`, `hook_path_safety`, `hook_commands_cwd_independent`, `hook_commands_subdirectory_safe`, `pre_tool_blocking_available`, `post_tool_correlation_available`, `bash_shell_mutation_coverage`, `direct_file_write_matcher_coverage`, `bypass_detection_active`, `prompt_capture_available`, `local_web_consent_available`, and `managed_distribution_verified` expose the capability facts behind the label. `host_hook_guarded` requires verified generated config, native host output, `hook_path_safety=ok`, cwd-independent and subdirectory-safe required hook commands, required lifecycle phases, Bash/shell and direct file-write matcher coverage, a matching policy hash, and a current runtime guard observation. `bypass_detection_active=true` requires an active session watch; a partial coverage warning remains visible in `session_watch_partial_coverage_warning`. A setup diagnostic that cannot observe a runtime-only capability reports that capability as false.
- `guard_hook_observed` reports whether a current matching host guard hook observation is recorded for the selected guard installation.
- `last_guard_observed_at` is the latest stored guard-installation observation timestamp, or `null` when no observation is recorded.
- `last_guard_event_at` is the latest guard-event timestamp available to the projection, or `null` when no guard event is available.
- `host_kind`, `observed_hook_phase`, `observed_host_kind`, `expected_policy_hash`, `observed_policy_hash`, and `observed_binary_version` report the selected installation and latest stored observation metadata when available.
- `required_hook_phases` and `missing_required_hook_phases` report required guard hook configuration completeness. A required phase is missing when it is absent from `required_hook_phases` or listed in `missing_required_hook_phases`. Missing required phases keep effective guard health non-active even when a valid hook event has been observed.
- `prompt_capture_status` reports the machine-readable prompt-capture availability state for the selected connection. `prompt_capture_available=true` only when that state allows verification-code chat commands; it does not mean raw prompt text is included.
- `prompt_capture_available` reports whether prompt-capture verification-code chat commands may be shown or recorded for the selected connection. It does not include prompt text.
- `local_web_consent_available` reports whether the current adapter invocation can offer the loopback local web consent fallback for User Channel recovery.
- `managed_distribution_verified` reports whether managed mode is backed by verified managed-distribution metadata. It is false for ordinary project-local guarded hook files.
- `mcp_connection_healthy` and `mcp_connection_status` summarize the tracked Agent Connection verification state when that state is available.
- `session_watch_status` reports whether the session-level Product Repository watcher is `disabled`, `active`, `degraded`, `unavailable`, or `pending_project_selection` for the selected connection or session.
- `last_session_watch_checked_at` is the latest watcher baseline status update timestamp, or `null` when no session-watch baseline is available.
- `session_watch_baseline_created_at` is the stored baseline creation timestamp, or `null` when no session-watch baseline is available.
- `session_watch_coverage_start_at` is the timestamp from which the watcher baseline can claim coverage for the selected session, or `null` when no coverage start is available.
- `session_watch_coverage_basis` is `mcp_start`, `first_project_selection`, `method_boundary`, or `null`.
- `session_watch_partial_coverage_warning` is a human-readable warning when Product Repository changes before the recorded coverage start are outside watcher coverage.
- `session_watch_detail` is a short diagnostic detail for the selected watcher state, or `null` when no detail is available.
- `unresolved_unrecorded_change_count` is a count of unresolved unrecorded Product Repository changes. It does not expose prompt text, command text, or path lists.
- `missing_or_stale_write_readiness` reports whether guard events detected missing or stale write readiness.

Does not imply:
- `guard_strength` is not proof of correctness, review completion, test sufficiency, OS-level enforcement, or write prevention.
- `GuardHealthSummary` is not evidence of product correctness, test sufficiency, OS enforcement, sandboxing, security isolation, or final acceptance.
- An active guard summary does not replace evidence, artifact integrity, user-owned judgment, `Write Check`, final acceptance, or residual-risk acceptance requirements.
- Session watch status and coverage metadata do not mean Volicord prevented a write, identified the actor who changed a file, stored file contents, or provided OS-level enforcement.
- When `session_watch_partial_coverage_warning` is non-null, Product Repository changes before `session_watch_coverage_start_at` remain outside session-watch coverage.
- `mcp_only` mode remains cooperative except that unresolved watcher-created unrecorded-change findings block close while an active session watch is selected.

Owner links:
- `guard_mode`, `guard_strength`, `hook_path_safety`, `guard_installation_status`, `guard_configuration_status`, `guard_observation_status`, `effective_guard_status`, `prompt_capture_status`, `session_watch_status`, and `session_watch_coverage_basis` values: [state and blocker values](schema-value-sets.md#state-and-blocker-values)
- Close-readiness guard blockers and method-local codes: [`volicord.close_task`](method-close-task.md)
- Agent Connection meaning: [Agent Connection](../agent-connection.md)

<a id="unrecorded-change-reconciliation-shapes"></a>
## Unrecorded change reconciliation shapes

`UnrecordedChangeFinding` is the public finding shape returned by `volicord.reconcile_changes` for unresolved unrecorded Product Repository changes.

`UnrecordedChangeResolutionSummary` is the public summary shape for findings resolved by one reconciliation call.

```yaml
UnrecordedChangeFinding:
  unrecorded_change_ref: StateRecordRef
  status: string
  summary: string
  observed_paths: string[]
  detected_at: string
  can_resolve_in_chat: boolean
  next_action: NextActionSummary

UnrecordedChangeResolutionSummary:
  unrecorded_change_ref: StateRecordRef
  resolution_basis: string
  resolved_by_actor_source: string
  capture_basis: string
  user_judgment_ref: StateRecordRef | null
  resolved_at: string
```

Meaning:

- `unrecorded_change_ref` uses `StateRecordRef` with `record_kind=unrecorded_change`.
- `status` is a controlled value string.
- `summary`, `capture_basis`, and `next_action.label` are display strings, not proof of correctness.
- `observed_paths` contains Product Repository relative paths when Core can safely decode them. It does not include prompt text, command text, shell arguments, or full sensitive content.
- `can_resolve_in_chat` reports whether the finding can proceed through a chat-mediated user path selected by the method owner.
- `resolution_basis` classifies why the finding became resolved.
- `resolved_by_actor_source=system` means Core verified a deterministic basis; `resolved_by_actor_source=local_user` means a compatible User Channel judgment supplied the authority.
- `user_judgment_ref` is non-null only for user-owned acceptance resolution.

These shapes do not prove product correctness, test sufficiency, review completion, final acceptance, residual-risk acceptance, or security. Resolution behavior and caller restrictions belong to [`volicord.reconcile_changes`](method-reconcile-changes.md).

Owner links:

- Resolution behavior: [`volicord.reconcile_changes`](method-reconcile-changes.md).
- Resolution basis and status values: [API Value Sets](schema-value-sets.md#unrecorded-change-resolution-basis-values).
- Storage record preservation: [Storage Records](../storage-records.md).

<a id="project-continuity-shapes"></a>
## Project continuity shapes

`ProjectContinuityRecord` is the full API state shape for one durable project-level continuity record. `ProjectContinuitySummary` is the compact status-view shape.

```yaml
ProjectContinuityRecord:
  continuity_record_id: string
  project_id: string
  source_task_id: string
  source_change_unit_id: string | null
  kind: string
  title: string
  summary: string
  rationale: string | null
  applies_to_paths: string[]
  applies_to_refs: StateRecordRef[]
  source_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]
  status: string
  supersedes_refs: StateRecordRef[]
  review_triggers: string[]
  created_at: string
  updated_at: string

ProjectContinuitySummary:
  continuity_record_ref: StateRecordRef
  kind: string
  status: string
  title: string
  summary: string
  source_task_ref: StateRecordRef
  source_change_unit_ref: StateRecordRef | null
  review_triggers: string[]
```

Meaning:
- Project continuity records preserve durable project-level context such as decisions, obligations, known limits, accepted residual risks, and constraints after the source `Task` closes.
- `source_task_id` and `source_change_unit_id` identify where the record originated. They do not make the source Task or Change Unit current again.
- `applies_to_paths`, `applies_to_refs`, `source_refs`, `artifact_refs`, `supersedes_refs`, and `review_triggers` are bounded context for later review. Empty arrays mean the record has no entries for that field.
- `ProjectContinuitySummary` is selected by method owners as a read view; it is not the full persisted record.

Does not imply:
- A project continuity record is not current Task authority, evidence, `Write Check`, final acceptance, close readiness, residual-risk acceptance for a future close basis, or a blocker waiver.
- `status=active` means the continuity record is live project context. It does not mean the record is currently applicable to every Task or that its source decision remains sufficient for a new authority check.

Owner links:
- `kind` and `status` values: [project continuity values](schema-value-sets.md#project-continuity-values)
- Storage family and JSON placement: [Storage Records](../storage-records.md)
- Method-specific creation effects: [Storage Effects](../storage-effects.md)

## `ChangeUnitEffectContract`

`ChangeUnitEffectContract` is the optional effect-boundary object recorded on a Change Unit.

```yaml
ChangeUnitEffectContract:
  allowed_effects: string[]
  forbidden_effects: string[]
  allowed_paths: string[]
  expected_outputs: string[]
  invariants: string[]
  evidence_expectations: string[]
  sensitive_action_expectations: string[]
```

Meaning:
- `allowed_effects` and `forbidden_effects` classify effects that the current Change Unit permits or forbids as Core state.
- `allowed_paths` lists Product Repository relative paths that further narrow product-file writes when present.
- `expected_outputs`, `invariants`, `evidence_expectations`, and `sensitive_action_expectations` are structured expectation strings. They help users and agents understand the intended output and evidence boundary without creating a workflow engine.
- An empty array means that part of the contract adds no extra restriction or expectation.

Does not imply:
- `ChangeUnitEffectContract` is not a runtime sandbox, command interceptor, network blocker, operating-system permission system, or development-methodology state machine.
- It does not replace user-owned judgment, sensitive-action approval, evidence, `Write Check`, final acceptance, close readiness, or residual-risk acceptance.

Owner links:
- Effect value strings: [method-local values](schema-value-sets.md#method-local-values)
- Product Repository path normalization: [Runtime Boundaries](../runtime-boundaries.md#product-repository-api-path-normalization)
- Method behavior that records the contract: [`volicord.update_scope`](method-update-scope.md)
- Method behavior that applies the product-file write boundary: [`volicord.prepare_write`](method-prepare-write.md)

## Task lifecycle state

`TaskLifecycleState` is the API shape for Task lifecycle fields that may appear inside `StateSummary` or close results.

```yaml
TaskLifecycleState:
  lifecycle_phase: string
  close_reason: string
  result: string
  closed_at: string | null
```

Owner links:
- Supported values for `lifecycle_phase`, `close_reason`, and `result`: [task lifecycle values](schema-value-sets.md#task-lifecycle-values)
- Product meaning of lifecycle areas: [Core Model task lifecycle](../core-model.md#6-task-lifecycle)

## `ShapingReadiness`

Meaning:
- `ShapingReadiness` is an API view shape over Task, Change Unit, pending judgment, evidence summary, blocker, and next-action fields.
- Its boolean fields and `gaps` array expose readiness-shaped data for the current state.

```yaml
ShapingReadiness:
  goal_summary_known: boolean
  scope_boundary_known: boolean
  non_goals_known: boolean
  affected_area_or_paths_known: boolean
  acceptance_criteria_known: boolean
  autonomy_boundary_known: boolean
  first_change_unit_known: boolean
  user_owned_blocker_kind: string | null
  next_safe_action: NextActionSummary | null
  gaps: ShapingGap[]

ShapingGap:
  gap_kind: string
  message: string
  blocker_ref: StateRecordRef | null
  user_judgment_candidate_ref: StateRecordRef | null
```

Meaning:
- `ShapingGap` can reference a blocker or user-judgment candidate by shape.
- `user_owned_blocker_kind` and `ShapingGap.gap_kind` are opaque readiness classification strings. They are not exhaustive public value sets unless an affected owner publishes narrower values.
- `ShapingGap.message` is a free-form display string.

Owner links:
- Method behavior and durable effects: method owner documents routed from [API Methods](methods.md) and [Storage Effects](../storage-effects.md)

<a id="current-position-display-shapes"></a>
## Current-position display shapes

```yaml
NextActionSummary:
  action_kind: string
  owner_method: string | null
  label: string
  blocking_question: string | null
  required_refs: StateRecordRef[]

WriteCheckStateSummary:
  status: string
  write_check_ref: StateRecordRef | null
  basis_state_version: integer | null
  intended_paths: string[]
  consumed_by_run_ref: StateRecordRef | null
  observation_refs: StateRecordRef[]
  guarantee_display: GuaranteeDisplay | null

WriteCheckSummary:
  write_check_ref: StateRecordRef
  status: string
  attempt_scope: WriteCheckAttemptScope
  basis_state_version: integer
  expires_at: string | null

WriteCheckAttemptScope:
  task_id: string
  change_unit_id: string
  intended_operation: string
  intended_paths: string[]
  product_file_write_intended: boolean
  sensitive_categories: string[]
  baseline_ref: string | null

WriteDecisionReason:
  category: string
  code: string
  message: string
  related_refs: StateRecordRef[]
```

Meaning:
- `NextActionSummary` is the canonical next-action display shape. Its valid fields are `action_kind`, `owner_method`, `label`, `blocking_question`, and `required_refs`.
- A `next_actions` entry that uses stale `action` or `reason` fields is not a valid `NextActionSummary`.
- `WriteCheckStateSummary.status` and `WriteCheckSummary.status` are controlled value strings.
- `WriteCheckStateSummary.consumed_by_run_ref` is non-null only when the summarized `Write Check` has been consumed by a recorded Run.
- `WriteCheckStateSummary.observation_refs` lists evidence observation refs created by that consuming Run when those refs are available; it is empty when the `Write Check` is not consumed or the consuming Run created no observations.
- `WriteCheckAttemptScope` is the one-attempt boundary captured by a `Write Check`.
- `WriteCheckAttemptScope` is not ordinary write approval, sensitive-action approval, final acceptance, residual-risk acceptance, or broad user approval.
- `WriteDecisionReason` is used by `PrepareWriteResult.write_decision_reasons`.

`NextActionSummary` field classifications:

| Field | Classification | Rule |
|---|---|---|
| `action_kind` | Controlled action category. | Uses the [next-action values](schema-value-sets.md#next-action-values). It is not a method-name value. |
| `owner_method` | Method-name value or `null`. | Names the API method that owns the next action when one supported public method applies. Use `null` when no single owner method applies. |
| `label` | Free-form display string. | Human- and agent-facing display text, not a canonical value. |
| `blocking_question` | Free-form display string or `null`. | The question to resolve before the action can proceed, or `null` when no blocking question is needed. |
| `required_refs` | `StateRecordRef[]`. | Records required for the next action. Use `[]` when there are no required refs. |

`WriteCheckAttemptScope` field classifications:

| Field | Classification | Rule |
|---|---|---|
| `task_id` | Opaque identifier. | Identifies the Task for the captured attempt boundary. |
| `change_unit_id` | Opaque identifier. | Identifies the Change Unit for the captured attempt boundary. |
| `intended_operation` | Free-form intent string. | Describes the intended operation without creating a controlled value set. |
| `intended_paths` | Normalized Product Repository path strings. | Product Repository relative paths after API-level path normalization. |
| `product_file_write_intended` | Boolean. | Indicates whether the captured attempt intended a product-file write. |
| `sensitive_categories` | Opaque sensitive-category classification strings. | Not an exhaustive public enum unless an affected method or profile owner publishes a narrower local list. |
| `baseline_ref` | Opaque baseline identifier or `null`. | Names the baseline identifier captured for the attempt boundary when present. |

`WriteDecisionReason` field classifications:

| Field | Classification | Rule |
|---|---|---|
| `category` | Controlled category value. | Uses the `WriteDecisionReason.category` values owned by [API Value Sets](schema-value-sets.md#state-and-blocker-values). |
| `code` | Method-scoped opaque reason code. | Not a global exhaustive enum. A method owner may define local codes, but example codes do not become global values. |
| `message` | Free-form display string. | Human- and agent-facing display text, not a canonical value. |
| `related_refs` | `StateRecordRef[]`. | Records related to the decision reason. Use `[]` when there are no related refs. |

`WriteDecisionReason` is distinct from `CloseReadinessBlocker`.

Owner links:
- `action_kind` values: [next-action values](schema-value-sets.md#next-action-values)
- `owner_method` values: [method name values](schema-value-sets.md#method-name-values)
- `WriteCheckStateSummary.status` and `WriteCheckSummary.status` values: [method-local values](schema-value-sets.md#method-local-values)
- `WriteDecisionReason.category` values: [state and blocker values](schema-value-sets.md#state-and-blocker-values)
- `WriteDecisionReason.code` value-set boundary: [opaque and method-scoped string fields](schema-value-sets.md#opaque-and-method-scoped-string-fields)
- `WriteDecisionReason.code` production and local meaning: method owner documents, including [`volicord.prepare_write`](method-prepare-write.md)
- `Write Check` creation behavior: [`volicord.prepare_write`](method-prepare-write.md)
- `Write Check` product meaning and approval boundaries: [Core Model](../core-model.md)
- Public `ErrorCode` values are separate: [API error codes](error-codes.md)

## Evidence and run snapshot shapes

```yaml
EvidenceSummary:
  status: string
  completion_policy: CompletionPolicy
  coverage_items: EvidenceCoverageItem[]
  artifact_refs: ArtifactRef[]
  observation_refs: StateRecordRef[]
  updated_by_run_ref: StateRecordRef | null

CompletionPolicy:
  evidence_required: boolean
  required_claims: string[]

EvidenceCoverageItem:
  claim: string
  required_for_close: boolean
  coverage_state: string
  provenance: EvidenceUpdateProvenance | null
  supporting_refs: StateRecordRef[]
  observation_refs: StateRecordRef[]
  supporting_artifact_refs: ArtifactRef[]
  gap_refs: StateRecordRef[]

EvidenceUpdateProvenance:
  source_kind: string
  assurance_level: string
  observed_at: string | null
  tool_name: string | null
  tool_invocation_id: string | null
  tool_metadata: object
  limitations: string[]

EvidenceObservation:
  observation_id: string
  project_id: string
  task_id: string
  change_unit_id: string | null
  run_ref: StateRecordRef | null
  claim: string
  source_kind: string
  assurance_level: string
  observed_by_actor_source: string | null
  tool_name: string | null
  tool_invocation_id: string | null
  tool_metadata: object
  input_refs: StateRecordRef[]
  output_artifact_refs: ArtifactRef[]
  limitations: string[]
  observed_at: string
  recorded_at: string

EvidenceObservationInput:
  claim: string
  source_kind: string
  assurance_level: string
  observed_by_actor_source: string | null
  tool_name: string | null
  tool_invocation_id: string | null
  tool_metadata: object
  input_refs: StateRecordRef[]
  output_artifact_refs: ArtifactRef[]
  limitations: string[]
  observed_at: string

RunSummary:
  run_ref: StateRecordRef
  kind: string
  summary: string
  observed_changes: ObservedChanges
  artifact_refs: ArtifactRef[]

ObservedChanges:
  changed_paths: string[]
  product_file_write_observed: boolean
  sensitive_categories: string[]
  baseline_ref: string | null
```

Meaning:
- `EvidenceSummary.status`, `EvidenceCoverageItem.coverage_state`, `EvidenceUpdateProvenance.source_kind`, `EvidenceUpdateProvenance.assurance_level`, `EvidenceObservation.source_kind`, `EvidenceObservation.assurance_level`, `EvidenceObservationInput.source_kind`, `EvidenceObservationInput.assurance_level`, and `RunSummary.kind` are controlled value strings.
- `CompletionPolicy.required_claims`, `EvidenceCoverageItem.claim`, `EvidenceObservation.claim`, `EvidenceObservationInput.claim`, and `RunSummary.summary` are free-form claim or display strings.
- `EvidenceCoverageItem.provenance` is optional on request input and is omitted from committed evidence summaries after Core creates or links the corresponding `EvidenceObservation`. A supported evidence update for a close-relevant claim must have a matching observation input, a usable observation ref, or this provenance object so Core can create an observation.
- `EvidenceSummary.observation_refs` and `EvidenceCoverageItem.observation_refs` list `StateRecordRef` values for committed evidence observations that Core relates to the summary or claim.
- `EvidenceObservation` is a durable provenance record for one reported or observed evidence claim. It records source, assurance, observer actor source, optional tool metadata, input refs, output artifact refs, limitations, and observation timestamps.
- `EvidenceObservationInput` is the request-side shape accepted by `volicord.record_run`; Core fills `observation_id`, project and Task coordinates, `run_ref`, and `recorded_at` when it commits.
- `observed_by_actor_source`, when present, must be an `ActorSource` value. When it is null in an observation input, Core may fill it from the verified invocation context.
- `source_kind` and `assurance_level` describe provenance and observation assurance. They do not by themselves prove product correctness, grant user authority, satisfy final acceptance, satisfy residual-risk acceptance, or raise `GuaranteeDisplay.level`.
- `user_observation` records a user-attributed observation, not final acceptance or any other authority-bearing user judgment.
- `external_tool` and `external_tool_result` record an external tool result. They are not a product correctness proof without the applicable evidence, artifact, close-readiness, and security owners.
- `unverified_claim` and `unverified` preserve an asserted claim without verified observation and are not sufficient evidence by themselves.
- `tool_metadata` is descriptive metadata and must not be treated as authority, approval, or a storage effect.
- `ObservedChanges.changed_paths` are path strings.
- `ObservedChanges.sensitive_categories` are opaque sensitive-category classification strings unless an affected method or profile owner publishes a narrower local list.
- `ObservedChanges.baseline_ref` is an opaque baseline identifier.

Owner links:
- `ArtifactRef`: [API Artifact Schemas](schema-artifacts.md)
- evidence, coverage, evidence observation, and run-kind values: [state and blocker values](schema-value-sets.md#state-and-blocker-values), [evidence observation values](schema-value-sets.md#evidence-observation-values), and [method-local values](schema-value-sets.md#method-local-values)
- evidence observation actor values: [actor values](schema-value-sets.md#actor-values)
- Evidence sufficiency meaning: [Core Model evidence and run authority](../core-model.md#9-evidence-and-run-authority)
- Method behavior: method owner documents routed from [API Methods](methods.md)

<a id="close-readiness-and-validation-shapes"></a>
## Close readiness and validation shapes

```yaml
CurrentCloseBasis:
  close_basis_revision: integer
  scope_revision: integer
  task_id: string
  change_unit_id: string
  baseline_ref: string | null
  result_summary: string
  result_refs: StateRecordRef[]
  evidence_summary_ref: StateRecordRef | null
  residual_risks: ResidualRisk[]
  sensitive_categories: string[]
  sensitive_action_requirements: SensitiveActionRequirement[]
  recovery_constraints: string[]
  source_run_ref: StateRecordRef
  updated_at: string

SensitiveActionRequirement:
  action_kind: string
  normalized_paths: string[]
  sensitive_categories: string[]
  baseline_ref: string | null
  change_unit_id: string
  source_run_ref: StateRecordRef
  source_write_check_ref: StateRecordRef

ResidualRisk:
  risk_id: string
  summary: string
  consequence: string
  acceptance_required: boolean
  source_refs: StateRecordRef[]

RiskAcceptanceCoverage:
  risk_id: string
  accepted: boolean
  accepted_by_judgment_refs: StateRecordRef[]
  missing_reason: string | null

CloseReadinessBlocker:
  category: string
  code: string
  message: string
  guard_strength: string | null
  can_resolve_in_chat: boolean
  terminal_action_required: boolean
  related_refs: StateRecordRef[]
  next_actions: NextActionSummary[]

ValidatorResult:
  validator_id: string
  status: string
  severity: string | null
  message: string
  related_refs: StateRecordRef[]

GuaranteeDisplay:
  level: string
  basis: string
  capability_refs: StateRecordRef[]
```

Meaning:
- `CurrentCloseBasis` is the current result and residual-risk state used by close-readiness responses. It is not a terminal close summary.
- `close_basis_revision` and `scope_revision` are internal current-state coordinates surfaced for compatibility checks. They are not caller-selected authority.
- `ResidualRisk.risk_id` is an opaque Core-generated identifier. `ResidualRisk.summary` and `ResidualRisk.consequence` are display strings and do not authorize text matching.
- `result_refs`, `source_run_ref`, `source_refs`, `evidence_summary_ref`, and `accepted_by_judgment_refs` use `StateRecordRef`.
- `sensitive_categories` are opaque sensitive-category classification strings unless an affected method or profile owner publishes a narrower local list.
- `sensitive_action_requirements` are Core-derived close requirements from committed Runs and consumed `Write Check` records. Category-only caller input cannot establish or erase these requirements.
- `recovery_constraints` and `RiskAcceptanceCoverage.missing_reason` are display strings. Current close-readiness results use `acceptance_required` when required acceptance is absent and may use `stale_acceptance` when a non-current residual-risk acceptance exists but does not cover the current residual-risk `risk_id` values.
- `RiskAcceptanceCoverage` reports whether the current residual-risk requirements are covered by compatible judgments. It does not report evidence sufficiency or final acceptance.
- `CloseReadinessBlocker` is a data shape for close-readiness findings.
- `CloseReadinessBlocker.category` is a controlled value string.
- `CloseReadinessBlocker.code` is an owner-defined blocker code. It is not an exhaustive global public enum unless the blocker or method owner publishes a narrower local list.
- `CloseReadinessBlocker.guard_strength` may be present on guard-derived connection-capability blockers to report the selected guard-health label at the time the blocker was computed. It is absent for blockers that do not derive from guard health.
- `can_resolve_in_chat` reports whether the blocker can be resolved through a chat-mediated user path when the method owner knows that path.
- `terminal_action_required` reports whether the next action requires a terminal, host, filesystem, or setup action outside chat.
- `CloseReadinessBlocker.message`, `ValidatorResult.message`, and `GuaranteeDisplay.basis` are free-form display strings.
- `ValidatorResult.validator_id` is a reporting label unless the value-set owner publishes a supported stable value.
- `ValidatorResult.status`, `ValidatorResult.severity`, and `GuaranteeDisplay.level` are controlled value strings.

These shapes do not define close-readiness meaning, response routing, or persistence behavior.

Close-basis reference rules:
- Caller-supplied close-assessment refs accepted into `CurrentCloseBasis.result_refs` or `ResidualRisk.source_refs` are limited to result/evidence record kinds `run`, `artifact`, `evidence_summary`, and `change_unit` unless an owner document explicitly adds another kind.
- `project_state`, `write_check`, `user_judgment`, `blocker`, `task_event`, and `task` are not caller-supplied result refs for a close basis unless an owner document explicitly adds them.
- Every accepted ref must exist, belong to the same project and Task, and be canonicalized by Core. Core never preserves caller-supplied `state_version` metadata as authority.
- Artifact refs used for close evidence must be linked to the Task and have `integrity_status=verified` plus current-byte verification at use time under [Artifact Storage](../storage-artifacts.md).
- Evidence refs must identify the current Task evidence summary. Run refs used as current close-basis result refs must identify a recorded current Run compatible with the current Task, current Change Unit, current scope revision, compatible baseline, and recorded status. Historical Runs are audit records unless a current Run explicitly reuses their verified artifacts or evidence and records that reuse.
- Core may add the current Run, current Change Unit, and current EvidenceSummary refs when constructing the canonical close basis.

Guarantee display rules:
- `GuaranteeDisplay` is derived from the project enforcement profile, verified invocation context, enabled enforcement mechanisms, and supported baseline scope.
- `capability_refs` is the implemented field name for references that justify the display; in the baseline connection architecture it should cite invocation binding, Agent Connection, or observation facts when such refs are available.
- A cooperative-only deployment must not claim `detective`.
- `detective` requires supported enforcement or observation facts for the observed scope, not host instructions, connection mode, or generated text alone.
- A cooperative `agent_report` Run or observation is not displayed as `detective` or externally observed unless a separate supporting observation justifies that display.

Owner links:
- Close-readiness meaning and non-substitution rules: [Core Model close readiness](../core-model.md#close_task)
- Current close basis creation: [`volicord.record_run`](method-record-run.md)
- Judgment compatibility and accepted-risk input: [API Judgment Schemas](schema-judgment.md)
- Response branch behavior, close-readiness evaluation order, and committed blocked outcomes: [`volicord.close_task`](method-close-task.md)
- Close-readiness blocker/API response routing semantics: [API blocker routing](blocker-routing.md)
- Supported `CloseReadinessBlocker.category`, `CloseReadinessBlocker.guard_strength`, `ValidatorResult.status`, `ValidatorResult.severity`, and `GuaranteeDisplay.level` values: [API Value Sets](schema-value-sets.md#state-and-blocker-values)
- Security guarantee meaning: [Security](../security.md)

## Related owners

- [API Schema Core](schema-core.md) for `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, and `ToolDryRunResponse`.
- [API Value Sets](schema-value-sets.md#state-and-blocker-values) for exact close-readiness blocker category values and neighboring state values.
- [API Methods](methods.md) and method owner documents for the methods that return these schemas.
- [API Artifact Schemas](schema-artifacts.md) for `ArtifactRef`.
- [API Judgment Schemas](schema-judgment.md) for `UserJudgmentCandidate`.
- [Storage Effects](../storage-effects.md) for persistence and state-effect consequences.
