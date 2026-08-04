# API state schemas

This document owns state-shaped API schemas for the baseline scope. It covers
common state references, current-position summaries, observation health,
project continuity, write tickets, evidence, and close-readiness data.

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

## Find a schema

| Data you need | Start here |
|---|---|
| State references, current `Task` position, lifecycle, and shaping readiness | [State references](#state-references) |
| Unrecorded changes and project continuity | [Unrecorded change reconciliation shapes](#unrecorded-change-reconciliation-shapes) |
| Status cards, next actions, and write tickets | [Current-position display shapes](#current-position-display-shapes) |
| Evidence, observations, and Run summaries | [Evidence and run snapshot shapes](#evidence-and-run-snapshot-shapes) |
| Close basis, residual risk, blockers, validators, and guarantees | [Close readiness and validation shapes](#close-readiness-and-validation-shapes) |

## Boundary

State schemas describe API data shapes only. A state-shaped field does not choose a response branch or create persistence, Core transitions, replay rows, `authority_events`, artifact effects, write-ticket effects, or a `state_version` increment.

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
- Record identity is exactly the tuple (`project_id`, `record_kind`, `record_id`). `task_id` is nullable `Task` context and is not part of record identity.
- `produced_at_state_version` is the nullable `project_state.state_version` of the aggregate projection that produced the reference. It is a projection-freshness cue, not record identity, a record revision, a close-basis revision, authority, or an optimistic-concurrency token. The same logical record can therefore appear with different `produced_at_state_version` values without becoming a different record.
- When a method response emits a ref from its current projection, non-null `produced_at_state_version` matches the `project_state.state_version` used for that response projection, even when the referenced record last changed earlier. Exact replay retains the originally stored response and its original projection-freshness values.
- A `StateRecordRef` never supplies concurrency input. A caller that invokes a mutation uses the current project clock in `ToolEnvelope.expected_state_version` when the method owner requires it.

It is a public reference, not an embedded storage row.

```schema
StateRecordRef:
  record_kind: string
  record_id: string
  project_id: string
  task_id: string | null
  produced_at_state_version: integer | null

UserActionResolutionRef:
  record_kind: user_action_resolution
  record_id: string
  project_id: string
  task_id: string
  produced_at_state_version: integer
```

`UserActionResolutionRef` is the dedicated approval-reference shape. Its
`record_kind` is fixed to `user_action_resolution`, and its required
`project_id`, `task_id`, and `record_id` form the full resolution identity.
Its required `produced_at_state_version` is the concrete project state version
of the projection that produced the reference and does not participate in that
identity.

Owner links:
- `record_kind` values: [record and reference values](schema-value-sets.md#record-and-reference-values)
- request-level optimistic concurrency: [`ToolEnvelope`](schema-core.md#tool-envelope)
- project state clock: [Storage Versioning](../storage-versioning.md)
- storage record families and values: [Storage Records](../storage-records.md)
- storage table names and DDL: [Storage DDL](../storage-ddl.md)

### Non-authoritative source references

`SourceRef` records caller-supplied context or provenance that is not a
Core-owned state-record reference. Its `source_kind` tag selects exactly one
`source` body:

```schema
SourceRef:
  source_kind: repository_file | git_commit | git_diff | command | external_uri | user_context
  source: RepositoryFileSource | GitCommitSource | GitDiffSource | CommandSource | ExternalUriSource | UserContextSource

RepositoryFileSource:
  repository_path: string
  baseline_commit_sha: string
  content_sha256: string
  line_range: SourceLineRange | null

SourceLineRange:
  start_line: integer
  end_line: integer

GitCommitSource:
  commit_sha: string

GitDiffSource:
  base_commit_sha: string
  head_commit_sha: string
  diff_artifact_ref: ArtifactRef | null

CommandSource:
  invocation_id: string
  command_summary: string
  exit_code: integer
  output_artifact_ref: ArtifactRef | null

ExternalUriSource:
  uri: string
  retrieved_at: string
  content_sha256: string

UserContextSource:
  context_id: string
```

Validation and authority boundary:
- `repository_path` is a Product Repository-relative source locator. Core rejects absolute paths, Windows drive prefixes, backslashes, and lexically escaping `..` segments, then removes `.` and non-escaping `..` segments without filesystem or symlink resolution. A line range is one-based and inclusive, starts at `1` or later, and does not end before it starts.
- Git object ids are full lowercase hexadecimal SHA-1 or SHA-256 ids of exactly `40` or `64` characters. Content hashes are lowercase `64`-character SHA-256 hexadecimal strings.
- `command_summary` is a non-empty redacted display summary, not executable input. Artifact refs, when present, are canonical same-project, same-Task refs selected by the method owner.
- `external_uri` is an absolute `http` or `https` URI without user information, and `retrieved_at` is RFC 3339. `user_context.context_id` is a non-empty opaque correlation id, not message content, actor identity, or User Channel provenance.
- `SourceRef` is context or provenance only. It is not record identity, scope, baseline selection, a user-owned judgment, approval, a write ticket, evidence sufficiency, final acceptance, residual-risk acceptance, close readiness, a guarantee, or a concurrency token.
- Core validates and stores the submitted shape. It does not read or hash a referenced file, resolve Git objects, execute a command, fetch a URI, or resolve message content. Submitted hashes, object ids, timestamps, exit codes, summaries, and context ids remain reported facts. Verified bytes continue to use `ArtifactRef` and Artifact Storage.

## `StateSummary`

`StateSummary` is the compact current-position state returned by supported methods that need to show the current `Task` path.

```schema
StateSummary:
  project_id: string
  state_version: integer
  task_ref: StateRecordRef | null
  mode: string | null
  requested_control_level: string | null
  effective_control_level: string | null
  control_level_reason: string | null
  project_policy: ProjectWorkflowPolicySummary | null
  work_phase: string | null
  acceptance_policy: string | null
  acceptance_policy_reason: string | null
  lineage: TaskLineageSummary | null
  lifecycle: TaskLifecycleState | null
  scope_revision: integer
  goal_summary: string | null
  scope_summary: string | null
  non_goals: string[]
  acceptance_criteria: AcceptanceCriterion[]
  autonomy_boundary: string | null
  active_change_unit_ref: StateRecordRef | null
  effect_contract: ChangeUnitEffectContract | null
  baseline_ref: string | null
  workspace_context: WorkspaceContext | null
  workflow: WorkflowProjection
  pending_user_action_summaries: AgentSafeUserActionRequestSummary[]
  blocker_refs: StateRecordRef[]
  write_ticket_summary: WriteTicketStateSummary | null
  evidence_summary: EvidenceSummary | null
  evidence_gate: EvidenceGateSummary | null
  close_state: string | null
  close_blockers: CloseReadinessBlocker[]
  guarantee_display: GuaranteeDisplay | null
```

Meaning:
- `StateSummary` is a compact response shape for state references, summaries, and close-readiness fields.
- Method `include` flags may select only part of this shape. When a method owner says a projection is not selected, include-controlled fields such as `evidence_summary`, `evidence_gate`, `close_state`, `close_blockers`, or `guarantee_display` are omitted instead of being returned as null or empty. A returned empty array means the projection was computed and found empty.
- `mode`, `work_phase`, `acceptance_policy`, and `close_state` are controlled
  value strings when present. `acceptance_policy_reason` records why Core chose
  the Task-owned final-acceptance policy; it is not an approval or waiver.
- `requested_control_level` preserves `auto` or the caller's explicit request.
  `effective_control_level` is the upward-only Core decision. The free-form
  `control_level_reason` explains the decision without becoming authority.
  `project_policy` identifies the exact authoritative policy copy used.
- `lineage` is the Task's one canonical predecessor edge and its carry-forward
  audit. `scope_revision` is the current Task scope revision.
- `goal_summary`, `scope_summary`, `non_goals`, and `autonomy_boundary` are
  free-form display strings. `acceptance_criteria` contains the current
  canonical criterion records for the Task; retired criteria are not projected
  as current criteria.
- `effect_contract` is the current Change Unit's optional extra effect contract. `null` means no extra Change Unit effect contract is recorded; it must not be described as broad safety or unrestricted execution.
- `baseline_ref` is an opaque baseline identifier.
- `workspace_context` is the optional verified Git coordinate captured for the
  current Change Unit baseline. Its paths and hashes are local authority facts,
  not portable repository identity or a security guarantee.
- `pending_user_action_summaries` lists effectively pending user actions
  relevant to the response view using only request ID, `status=pending`, and
  `next_actor=user`. Core still evaluates required-for target, action kind,
  Task, Change Unit, affected refs, and current basis internally to determine
  whether a request blocks an operation; `StateSummary` does not expose those
  request details.
- For an existing pending action in an Agent-facing result,
  `StateSummary.blocker_refs`, `CloseReadinessBlocker.related_refs`,
  `NextActionSummary.required_refs`, and summary-card ref collections exclude
  `record_kind=user_action_request`. Other blocker and authority record kinds
  remain governed by their owners. The request identity is supplied only by
  `AgentSafeUserActionRequestSummary`.

Does not imply:
- `StateSummary` field presence does not define whether a method committed.

Owner links:
- Task, lineage, workspace, and close values: [task lifecycle values](schema-value-sets.md#task-lifecycle-values)
- Commit decision branch: [Common response branches](schema-core.md#common-response)
- Method-specific commit behavior: method owner documents routed from [API Methods](methods.md)

<a id="task-lineage-workspace-and-authority-receipt"></a>
### Task lineage, workspace, and authority receipt

```schema
TaskLineageSummary:
  predecessor_task_ref: StateRecordRef
  relation: string
  creation_reason: string
  carry_forward: CarryForwardDisposition[]

CarryForwardDisposition:
  kind: string
  status: string
  source_refs: StateRecordRef[]

TaskFlowItem:
  task_ref: StateRecordRef
  predecessor_task_ref: StateRecordRef | null
  relation: string | null
  mode: string
  work_phase: string
  lifecycle_phase: string

WorkspaceContext:
  vcs: string
  git_common_dir: string
  worktree_id: string
  branch_ref: string | null
  head_sha: string | null
  workspace_fingerprint: string

ProjectWorkflowPolicySummary:
  policy_schema: string
  policy_version: integer
  policy_fingerprint: string
  source: string

AuthorityReceipt:
  project_id: string
  state_version: integer
  task_ref: StateRecordRef
  change_unit_ref: StateRecordRef | null
  scope_revision: integer
  latest_run_ref: StateRecordRef | null
  product_file_write_observed: boolean
  evidence_gate: EvidenceGateSummary | null
  close_state: string
  close_blockers: CloseReadinessBlocker[]
  completion_claim_allowed: boolean
  next_actor: string
```

Meaning:

- `TaskLineageSummary` records exactly one predecessor relation. `applied`
  carry-forward becomes newly validated Task input; `reference_only` preserves
  predecessor context without making its authority current.
- `TaskFlowItem[]` is a full-status projection over the connected predecessor
  component. It is derived display, not a new parent-goal record.
- `ProjectWorkflowPolicySummary.policy_schema` is
  `volicord.workflow_policy`; `policy_version` is monotonic per project and
  `policy_fingerprint` is the SHA-256 of canonical policy JSON. `source` is
  provenance for the authoritative database copy, not a file-loading contract.
- `AuthorityReceipt.completion_claim_allowed` is derived, never caller supplied.
  It is true only for a valid completion basis with no blockers and false when
  no active Task is available or authority refresh fails.
- `WorkspaceContext` uses the canonical Git common-directory and linked-
  worktree identity shared by local integration and Core write checks. A null
  branch represents detached HEAD. Non-Git repositories return null context.
- `AuthorityReceipt` is Core-generated from one freshly read project state
  version. Its blocker list is complete even when optional status projections
  are omitted. `product_file_write_observed` describes the latest recorded Run,
  not every historical Run. `next_actor` is a compact actor classification;
  current method progression remains in the tagged `StateSummary.workflow`.
  The receipt does not itself commit, close, accept, or prove product
  correctness.

<a id="unrecorded-change-reconciliation-shapes"></a>
## Unrecorded change reconciliation shapes

`UnrecordedChangeFinding` is the public finding shape returned by `volicord.reconcile_changes` for unresolved unrecorded Product Repository changes.

`UnrecordedChangeResolutionSummary` is the public summary shape for findings resolved by one reconciliation call.

```schema
UnrecordedChangeFinding:
  unrecorded_change_ref: StateRecordRef
  status: string
  summary: string
  observed_paths: string[]
  detected_at: string
  next_action: NextActionSummary

UnrecordedChangeResolutionSummary:
  unrecorded_change_ref: StateRecordRef
  resolution_basis: string
  resolved_by_actor_source: string
  capture_basis: string
  user_action_resolution_ref: StateRecordRef | null
  resolved_at: string
```

Meaning:

- `unrecorded_change_ref` uses `StateRecordRef` with `record_kind=unrecorded_change`.
- `status` is a controlled value string.
- Every unresolved finding is backed by a complete observed non-empty unmatched
  repository delta and is a close blocker. Baseline, outcome, delta, and
  unmatched-delta meaning belongs to
  [Repository Observation](../repository-observation.md).
- `summary`, `capture_basis`, and `next_action.label` are display strings, not proof of correctness.
- `observed_paths` contains a non-empty canonical Product Repository relative
  path set decoded from the exact observation's unmatched delta. It does not
  include prompt text, command text, shell arguments, or full sensitive content.
- `resolution_basis` classifies why the finding became resolved.
- `resolved_by_actor_source=system` means Core verified a deterministic basis; `resolved_by_actor_source=local_user` means a compatible User Channel judgment supplied the authority.
- `user_action_resolution_ref` is non-null only for user-owned acceptance resolution.

These shapes do not prove product correctness, test sufficiency, review completion, final acceptance, residual-risk acceptance, or security. Resolution behavior and caller restrictions belong to [`volicord.reconcile_changes`](method-reconcile-changes.md).

Owner links:

- Resolution behavior: [`volicord.reconcile_changes`](method-reconcile-changes.md).
- Resolution basis and status values: [API Value Sets](schema-value-sets.md#unrecorded-change-resolution-basis-values).
- Storage record preservation: [Storage Records](../storage-records.md).

<a id="project-continuity-shapes"></a>
## Project continuity shapes

`ProjectContinuityRecord` is the full API state shape for one durable project-level continuity record. `ProjectContinuitySummary` is the compact status-view shape.

```schema
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

ContinuityPageRequest:
  page_size: integer
  cursor: ContinuityCursor | null

ContinuityCursor:
  updated_at: string
  continuity_record_id: string

ProjectContinuityPage:
  items: ProjectContinuitySummary[]
  page_info: ContinuityPageInfo

ContinuityPageInfo:
  total_count: integer
  returned_count: integer
  truncated: boolean
  next_cursor: ContinuityCursor | null
```

Meaning:
- Project continuity records preserve durable project-level context such as decisions, obligations, known limits, accepted residual risks, and constraints after the source `Task` closes.
- `source_task_id` and `source_change_unit_id` identify where the record originated. They do not make the source Task or Change Unit current again.
- `applies_to_paths`, `applies_to_refs`, `source_refs`, `artifact_refs`, `supersedes_refs`, and `review_triggers` are bounded context for later review. Empty arrays mean the record has no entries for that field.
- `ProjectContinuitySummary` is selected by method owners as a read view; it is not the full persisted record.
- `ContinuityPageRequest.page_size` is an integer from 1 through 64.
  `ContinuityCursor` is a closed ordering object, not a record lookup or
  authority reference. Both members are required, use the exact canonical
  stored values, and identify an exclusive position in `updated_at DESC,
  continuity_record_id DESC` order.
- `ProjectContinuityPage.items` contains at most the requested page size.
  `total_count` counts every active record in the selected project before the
  cursor, `returned_count` equals `items.len`, and `truncated` is true exactly
  when a later item exists. `next_cursor` is non-null exactly when `truncated`
  is true and copies the last item's stored `updated_at` and
  `continuity_record_id`.

Does not imply:
- A project continuity record is not current Task authority, evidence, write ticket, final acceptance, close readiness, residual-risk acceptance for a future close basis, or a blocker waiver.
- `status=active` means the continuity record is live project context. It does not mean the record is currently applicable to every Task or that its source decision remains sufficient for a new authority check.

Owner links:
- `kind` and `status` values: [project continuity values](schema-value-sets.md#project-continuity-values)
- Storage family and JSON placement: [Storage Records](../storage-records.md)
- Method-specific creation effects: [Storage Effects](../storage-effects.md)

## `ChangeUnitEffectContract`

`ChangeUnitEffectContract` is the optional effect-boundary object recorded on a Change Unit.

```schema
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
- It does not replace user-owned judgment, sensitive-action approval, evidence, write ticket, final acceptance, close readiness, or residual-risk acceptance.

Owner links:
- Effect value strings: [method-local values](schema-value-sets.md#method-local-values)
- Product Repository path normalization: [Runtime Boundaries](../runtime-boundaries.md#product-repository-api-path-normalization)
- Method behavior that records the contract: [`volicord.update_scope`](method-update-scope.md)
- Method behavior that applies the product-file write boundary: [`volicord.prepare_write`](method-prepare-write.md)

## `Task` lifecycle state

`TaskLifecycleState` is the API shape for Task lifecycle fields that may appear inside `StateSummary` or close results.

```schema
TaskLifecycleState:
  lifecycle_phase: string
  close_reason: string
  result: string
  closed_at: string | null
```

Owner links:
- Supported values for `lifecycle_phase`, `close_reason`, and `result`: [task lifecycle values](schema-value-sets.md#task-lifecycle-values)
- Product meaning of lifecycle areas: [Core Model task lifecycle](../core-model.md#6-task-lifecycle)

## `WorkflowProjection` and shaping checkpoints

`StateSummary.workflow` is the single tagged progression authority. Its `kind` is one of `no_active_task`, `shaping_required`, `awaiting_user_action`, `decision_recovery_required`, `ready_to_apply_decisions`, `ready_for_change_unit`, `ready_to_finalize_advice`, `ready_for_implementation`, `implementation`, `close_review`, or `terminal`.

```schema
WorkflowProjection:
  kind: string
  next_actor: string
  required_action: string | null
  allowed_actions: string[]
  required_refs: StateRecordRef[]
  expected_state_version: integer
  blocking_reason: string | null
  checkpoint: ShapingCheckpointSummary | null
  action_catalog: WorkflowActionCatalog

WorkflowActionCatalog:
  required_method: string | null
  actions: WorkflowActionIntent[]

WorkflowActionIntent:
  method: string
  role: required | allowed
  expected_state_version: integer
  fixed_authority_coordinates: WorkflowActionAuthorityCoordinates
  required_refs: StateRecordRef[]

ShapingUserActionDraft:
  action: UserActionDraft
  expires_at: string | null

ShapingGapInput:
  gap_kind: string
  summary: string
  affected_refs: StateRecordRef[]
  user_action: ShapingUserActionDraft | null

ShapingCheckpointOperation:
  # initial variant
  operation: create_initial

  # replacement variant
  operation: replace_current
  expected_current_checkpoint_id: string
  retired_non_authorizing_request_refs: StateRecordRef[]
  carry_forward_application_refs: StateRecordRef[]
  stale_authority_actions: StaleShapingAuthorityAction[]

StaleShapingAuthorityAction:
  # retirement variant
  action: retire
  stale_application_ref: StateRecordRef

  # reauthorization variant
  action: reauthorize
  stale_application_ref: StateRecordRef
  successor_gap: ShapingGapInput

ShapingCheckpoint:
  shaping_checkpoint_id: string
  predecessor_checkpoint_id: string | null
  project_id: string
  task_id: string
  scope_revision: integer
  baseline_ref: string | null
  summary: string
  implementation_boundary: string | null
  readiness: string
  source_refs: SourceRef[]
  evidence_refs: StateRecordRef[]
  created_at: string
  superseded_at: string | null

ShapingCheckpointSummary:
  checkpoint_ref: StateRecordRef
  predecessor_checkpoint_ref: StateRecordRef | null
  readiness: string
  scope_revision: integer
  baseline_ref: string | null
  implementation_boundary: string | null
  current_application_refs: StateRecordRef[]
  gaps: ShapingCheckpointGap[]
  pending_decision_refs: StateRecordRef[]
  unresolved_application_owners: string[]
  decision_recovery_requirements: ShapingDecisionRecoveryRequirement[]

ShapingCheckpointGap:
  shaping_gap_id: string
  gap_kind: string
  application_owner: string | null
  summary: string
  affected_refs: StateRecordRef[]
  status: string
  decision_authority_state: string | null
  user_action_request_ref: StateRecordRef | null
  user_action_resolution_ref: StateRecordRef | null
  reauthorizes_application_ref: StateRecordRef | null

ShapingDecisionRecoveryRequirement:
  shaping_gap_id: string
  user_action_request_ref: StateRecordRef
  user_action_resolution_ref: StateRecordRef | null
  disposition: string
  reason: string

ShapingDecisionApplication:
  shaping_decision_application_id: string
  project_id: string
  task_id: string
  source_checkpoint_id: string
  source_gap_id: string
  user_action_request_id: string
  user_action_resolution_id: string
  judgment_kind: string
  application_owner: string
  applied_scope_revision: integer
  applied_baseline_ref: string
  applied_change_unit_id: string | null
  applied_at: string
  authority_status: string
  stale_at: string | null
  superseded_at: string | null

ShapingAuthorityReauthorization:
  shaping_authority_reauthorization_id: string
  project_id: string
  task_id: string
  stale_application_id: string
  stale_user_action_request_id: string
  successor_checkpoint_id: string
  successor_gap_id: string | null
  successor_user_action_request_id: string | null
  outcome: string
  created_at: string
```

`ShapingCheckpoint` is the first-class durable record returned by
`volicord.record_shaping_checkpoint`; the workflow embeds its current compact summary and
gap projections. A replacement carries the exact predecessor identity and the
complete explicit `current_application_refs` set through strict
checkpoint-application lineage. `ShapingDecisionApplication`, rather than an
`applied` gap alone, is the durable authority record. Its immutable source and
application coordinates preserve audit history while `authority_status` owns
explicit current, stale, or superseded invalidation. `ShapingGapInput.user_action` is non-null exactly for a
user-owned gap and carries the compatible typed draft that Core materializes
and links atomically. Readiness, gap kinds, gap statuses, workflow kinds, and
blocking reasons use the closed sets in [API Value Sets](schema-value-sets.md).
`ShapingAuthorityReauthorization` is immutable audit lineage. A `retired`
outcome has null successor gap/request identities; a `reissued` outcome has
both and always points to a fresh unresolved request.
`ShapingCheckpointOperation` is one closed tagged union. Replacement requires
the complete current compatible carry-forward set and the complete stale
application action set. `retire` ends one stale authority path without a
successor request. `reauthorize` creates a fresh successor gap and unresolved
request whose `reauthorizes_application_ref` names the stale application; it
does not carry the old accepted resolution into the new request.

Checkpoint readiness is structural and is independent from decision
application. `application_owner` is non-null exactly for a user-owned gap.
`unresolved_application_owners` is the unique stable set of owners for accepted
decisions not yet applied. It can be non-empty while `readiness=ready`.
`decision_recovery_requirements` identifies each exact rejected, deferred, or
expired request, available immutable resolution, authority disposition, and
typed reason. Its presence selects `decision_recovery_required` with
`next_actor=agent` and `required_action=volicord.record_shaping_checkpoint` even when
structural readiness is `ready`.
`ready_to_apply_decisions` is selected only when that set includes
`volicord.update_scope`. Work advance-owned decisions proceed toward a Change
Unit or `ready_for_implementation`. Advisor finalization-owned decisions
proceed toward a non-write Change Unit and `ready_to_finalize_advice`; only a
current checkpoint-backed close basis selects `close_review`.

The workflow projection selects at most one required method from current progression state. Its tagged `required_action`, not the position of a top-level action or blocker array entry, is progression authority. Close blockers retain their local remediation actions but never choose this required action. User-owned current gaps always carry an exact current UserAction request ref; their chat presentation never resolves it. Progression consumes the Store-owned current effective shaping authority graph for `advance_task`, `finalize_advice`, shaping-owned scope update, write preparation, Run recording, close readiness, and mutation rejection. A compatible application carried from an ancestor remains `applied` without copying its source gap. A stale application grants no authority and appears only as a current recovery obligation: in `advisor|work` shaping it selects `shaping_required`, `next_actor=agent`, `required_action=volicord.record_shaping_checkpoint`, `blocking_reason=application_authority_stale`, and its exact recovery refs. An implementation-phase update that would create this condition is rejected before mutation and names `volicord.close_task` as the close/supersede recovery instead of returning the Task to shaping. A contradiction inside the current graph uses `inconsistent_authority_state`. Superseded request, resolution, application, and checkpoint refs remain immutable audit history and never enter current `required_refs` or progression merely because they exist.

`action_catalog` contains one neutral action intent for every Task-state-bound
method in `allowed_actions` and no intent for any other method. Entries are
ordered by canonical method name. `required_method` is the Task-state-bound
`required_action`, or null when the required action is read-only or belongs to
the User Channel. The required entry has `role=required`; all other entries
have `role=allowed`. Duplicate methods, a missing required entry, a method and
coordinate-kind mismatch, or noncanonical ordering are invalid. Every entry
uses the workflow's state version and Core-owned fixed authority coordinates
from the same current Task snapshot. Initial checkpoint coordinates preserve
an actual null baseline; replacement coordinates carry the exact current and
predecessor checkpoint refs, retirement refs, compatible application refs, and
stale-application refs. Other coordinates bind the exact current Task,
checkpoint, Change Unit, scope revision, baseline, resolution, and method-local
authority facts that apply. MCP may project each neutral intent into an
executable method-specific action form, but MCP forms and their input slots are
not Core state.

Workflow mutation rejection details embed this same complete tagged
`WorkflowProjection`; they do not reconstruct progression from the received
payload. `allowed_actions`, blocker refs, exact Task mode/work phase, and the
single recovery owner are read from current authority. A rejected request does
not make its embedded `expected_state_version` a committed replay result: a
later replay is evaluated against then-current authority and returns its then-
current workflow.

Owner links:
- Method behavior and durable effects: method owner documents routed from [API Methods](methods.md) and [Storage Effects](../storage-effects.md)

<a id="current-position-display-shapes"></a>
## Current-position display shapes

```schema
SummaryCard:
  task: string
  recording: string
  profile: string
  write_ticket: string
  evidence: string
  user_action: string
  changes: string
  close_status: string
  transport: string
  next: string
  next_action: NextActionSummary | null
  guarantee: string

NextActionSummary:
  action_kind: string
  owner_method: string | null
  allowed_operation_categories: string[]
  label: string
  blocking_question: string | null
  expected_state_version: integer | null
  required_refs: StateRecordRef[]

WriteTicketStateSummary:
  status: string
  write_ticket_ref: StateRecordRef | null
  basis_state_version: integer | null
  validity_basis: WriteTicketValidityBasis | null
  invalidation_reason: string | null
  idle_expires_at: string | null
  intended_paths: string[]
  consumed_by_run_ref: StateRecordRef | null
  observation_refs: StateRecordRef[]
  guarantee_display: GuaranteeDisplay | null

WriteTicketAttemptScope:
  task_id: string
  change_unit_id: string
  intended_operation: string
  intended_paths: string[]
  product_file_write_intended: boolean
  sensitive_categories: string[]
  baseline_ref: string | null

WriteTicketPathPatterns:
  allowed: string[]
  denied: string[]

WriteTicketValidityBasis:
  task_id: string
  change_unit_id: string
  scope_revision: integer
  baseline_ref: string | null
  workspace_context_sha256: string | null
  write_authority_fingerprint: string
  approval_basis_refs: UserActionResolutionRef[]

WriteTicketScope:
  task_id: string
  change_unit_id: string
  intended_operation: string
  product_file_write_intended: boolean
  sensitive_categories: string[]
  baseline_ref: string | null

WriteTicket:
  write_ticket_id: string
  write_ticket_ref: StateRecordRef
  state: string
  scope: WriteTicketScope
  path_patterns: WriteTicketPathPatterns
  observed_paths: string[]
  basis_state_version: integer
  validity_basis: WriteTicketValidityBasis
  invalidation_reason: string | null
  idle_expires_at: string | null
  guarantee_display: GuaranteeDisplay | null

WriteDecisionReason:
  category: string
  code: string
  message: string
  related_refs: StateRecordRef[]
```

Meaning:
- `SummaryCard` is the stable compact summary shape for major user-facing status views. It uses public display strings for `Task`, Recording, Profile, Write Ticket, Evidence, User Judgment, Changes, Close Status, Transport, one Next action, and a concise Guarantee line.
- Display strings returned in `SummaryCard` and the returned
  `NextActionSummary.label` and `NextActionSummary.blocking_question` fields are
  normative public API values for that response. CLI or MCP adapters may wrap
  those values, but command syntax, transport framing, terminal or Markdown
  styling, and adapter-only explanatory copy are adapter presentation and are
  not Core-returned display strings.
- When evidence or close projection is selected, `SummaryCard.evidence` is the exact `EvidenceGateSummary.state` value owned by [API Value Sets](schema-value-sets.md#evidence-gate-values). It does not independently infer a state from staged input or `EvidenceSummary.evidence_state`.
- `SummaryCard.next` is a display hint only. `SummaryCard.next_action` may carry a matching structured `NextActionSummary` and may be omitted when no structured action applies. Neither field is workflow authority; close blockers remain independent facts.
- `SummaryCard` is a summary of other owner-selected state fields, not a second authority record. It must not add internal identifiers unless an identifier is needed for the displayed next action.
- For an already-existing pending user action, `SummaryCard.user_action`,
  `SummaryCard.next`, method `status_summary`, blocker messages, and every other
  display/template string stay generic. They may say that user action is
  pending and identify the User Channel as next actor, but must not reconstruct
  request question, options, context, form, path, command, URL, or credential.
- `SummaryCard.guarantee` is concise display wording for the summarized view. It must not claim correctness proof, test sufficiency proof, review completion, or OS-level enforcement unless another owner explicitly provides that guarantee.
- `NextActionSummary` is the canonical blocker-local or preview remediation shape. Its valid fields are `action_kind`, `owner_method`, `allowed_operation_categories`, `label`, `blocking_question`, `expected_state_version`, and `required_refs`.
- Successful method results expose tagged `workflow` progression. `CloseReadinessBlocker.next_actions` remains local to its blocker and has no cross-blocker selection role.
- `allowed_operation_categories` names the owner-supported invocation categories for the action. It does not prove that the current connection can dispatch the action, does not grant user authority, and is empty when no supported API method invocation is identified.
- `expected_state_version` is always present and nullable. For an API mutation action that consumes optimistic concurrency, it contains the current `project_state.state_version` from the projection that produced the action and maps directly to `ToolEnvelope.expected_state_version` for that invocation. It is `null` for read actions, `user_only` actions, actions without a single owner method, and owner-method actions that do not consume optimistic concurrency.
- `expected_state_version` is a retryable concurrency input, not identity or authority. It can become stale after another committed mutation; callers refresh current state after `STATE_VERSION_CONFLICT`. Neither `required_refs` nor any ref's `produced_at_state_version` supplies or overrides this token.
- A blocker-local or preview action that uses stale `action` or `reason` fields is not a valid `NextActionSummary`.
- For an already-existing pending user action, `NextActionSummary.label` and
  the owning blocker message use generic User Channel guidance,
  `blocking_question=null`, and `required_refs` contains no
  `user_action_request` ref. The request ID and pending/next-actor facts come
  only from `AgentSafeUserActionRequestSummary`; next-action text must not
  reconstruct the question, options, context, form, capture path, command, URL,
  or credential. The distinct pre-request `missing_final_acceptance` action may
  carry the question and Task/current-basis refs needed for the Agent to create
  the request. After creation, the pending rule applies.
- `WriteTicketStateSummary.status` is a controlled value string.
- When a stored-active ticket has a missing or mismatched policy-authority
  binding, current projection treats it as effectively
  `status=invalidated,invalidation_reason=explicit_revoke`. That fail-closed
  projection does not make the ticket an active candidate and does not rewrite
  a historical consumed ticket.
- `WriteTicketStateSummary.consumed_by_run_ref` is non-null only when the summarized write ticket has been consumed by a recorded Run.
- `WriteTicketStateSummary.observation_refs` lists evidence observation refs created by that consuming Run when those refs are available; it is empty when the write ticket is not consumed or the consuming Run created no observations.
- `WriteTicketAttemptScope` is the one-attempt boundary captured by the write ticket.
- `WriteTicketAttemptScope` is not ordinary write approval, sensitive-action approval, final acceptance, residual-risk acceptance, or broad user approval.
- `WriteTicket` is the ticket-first authority record returned by `volicord.prepare_write` when a committed allowed decision issues or reuses a compatible ticket.
- `WriteTicket.state` is a controlled value string.
- `WriteTicket.path_patterns.allowed` and `WriteTicket.path_patterns.denied` are normalized repository-relative path prefixes captured by the ticket decision. A prefix matches its exact path or descendants; wildcard and glob grammar is not supported. Absolute, empty, `..`-containing, or ambiguous entries are invalid; denied prefixes win, and an empty allowed list permits no product-file writes.
- `WriteTicket.validity_basis`, consumption state, optional idle timeout, and
  invalidation reason determine validity. `basis_state_version` records audit
  order only; an unrelated state-version increment never invalidates a ticket.
- `WriteTicketValidityBasis.approval_basis_refs` contains only
  `UserActionResolutionRef` values. Each value names one full project, `Task`,
  and UserAction-resolution identity owned by the ticket, and duplicate
  identities are invalid. Currentness compares that full identity; it never
  compares an unscoped resolution ID. Each reference carries a concrete
  `produced_at_state_version`, which is metadata rather than identity.
- `WriteTicketValidityBasis.write_authority_fingerprint` is canonical-JSON
  SHA-256 with the `sha256:` prefix over the exact normalized object
  `{schema:"volicord.write_authority",default_direct_control,default_work_control,light:{enabled,max_intended_paths,allowed_path_patterns,denied_path_patterns,final_acceptance},write_ticket:{idle_timeout_minutes}}`.
  The values come from the corresponding `workflow` policy fields, and both
  pattern arrays are sorted and deduplicated before canonicalization.
  Every policy field not listed in that object is excluded, including
  host, connection, MCP, integration-binding, and outer policy
  metadata fields. This digest is narrower than and is not interchangeable with
  the whole canonical-policy `policy_fingerprint`. Pattern order and duplicate
  entries therefore do not change the digest, and canonically different
  policies with the same normalized write-authority object preserve ticket
  compatibility. When no project policy exists, the normalized input uses
  `default_direct_control=tracked`, `default_work_control=tracked`,
  `light.enabled=false`, `light.max_intended_paths=3`, empty allowed and denied
  pattern arrays, `light.final_acceptance=policy_dependent`, and
  `write_ticket.idle_timeout_minutes=null`. Every canonical ticket carries the
  current digest. A missing or null binding is corrupt stored data, not a
  compatibility form.
- `WriteTicket.observed_paths` is empty in the baseline. Codex Record Guard
  observations are recorded through repository-observation and Unrecorded
  Change records rather than written back into the ticket.
- `WriteTicket.guarantee_display` discloses current guarantee wording. It does
  not claim OS-level filesystem enforcement.
- `WriteDecisionReason` is used by `PrepareWriteResult.write_decision_reasons`.

`NextActionSummary` field classifications:

| Field | Classification | Rule |
|---|---|---|
| `action_kind` | Controlled action category. | Uses the [next-action values](schema-value-sets.md#next-action-values). It is not a method-name value. |
| `owner_method` | Method-name value or `null`. | Names the API method that owns the next action when one supported public method applies. Use `null` when no single owner method applies. |
| `allowed_operation_categories` | Controlled operation-category values. | Lists the owner-supported invocation categories for this action. Uses `[]` when `owner_method=null` or no supported API invocation path is identified. |
| `label` | Free-form display string. | Human- and agent-facing display text, not a canonical value. For an existing pending user action it is generic User Channel guidance and carries no request detail. |
| `blocking_question` | Free-form display string or `null`. | The question to resolve before the action can proceed, or `null` when no blocking question is needed. It is always `null` for an existing pending user action; the pre-request creation exception is described above. |
| `expected_state_version` | Project state clock value or `null`. | Maps to `ToolEnvelope.expected_state_version` for a mutation action that consumes optimistic concurrency. Uses `null` for read, `user_only`, or no-concurrency actions. |
| `required_refs` | `StateRecordRef[]`. | Records required for the next action. Use `[]` when there are no required refs. Refs identify records and context; they never supply the concurrency token. Existing pending-user-action entries exclude the request ref. |

`WriteTicketAttemptScope` field classifications:

| Field | Classification | Rule |
|---|---|---|
| `task_id` | Opaque identifier. | Identifies the Task for the captured attempt boundary. |
| `change_unit_id` | Opaque identifier. | Identifies the Change Unit for the captured attempt boundary. |
| `intended_operation` | Free-form exact operation coordinate. | Stores the prepare-write value after trimming outer whitespace while preserving case and interior text. A method that compares `performed_operation` uses exact equality; this coordinate is not proof that an external action occurred. |
| `intended_paths` | Normalized Product Repository path strings. | Product Repository relative paths after API-level path normalization. |
| `product_file_write_intended` | Boolean. | Indicates whether the captured attempt intended a product-file write. |
| `sensitive_categories` | Opaque sensitive-category classification strings. | Not an exhaustive public enum unless an affected method or profile owner publishes a narrower local list. |
| `baseline_ref` | Opaque baseline identifier or `null`. | Names the baseline identifier captured for the attempt boundary when present. |

`WriteTicket` field classifications:

| Field | Classification | Rule |
|---|---|---|
| `write_ticket_id` | Opaque identifier. | Identifies the write ticket authority record. |
| `write_ticket_ref` | `StateRecordRef`. | References the same write ticket with `record_kind=write_ticket`. |
| `state` | Controlled state value. | Uses the [method-local values](schema-value-sets.md#method-local-values) owned for `WriteTicket.state`. |
| `scope` | `WriteTicketScope`. | Captures the Task, Change Unit, operation, sensitive categories, product-write flag, and baseline used for ticket issuance. |
| `path_patterns` | `WriteTicketPathPatterns`. | Captures allowed and denied normalized Product Repository path patterns for the ticket decision. |
| `observed_paths` | Normalized Product Repository path strings. | Lists observed paths only when an owner-defined Guard path has connected observations to the ticket. Use `[]` when no observations are connected. |
| `basis_state_version` | State-clock value. | Audit ordering captured at issue or reuse; never a ticket-validity coordinate. |
| `validity_basis` | `WriteTicketValidityBasis`. | Exact Task, Change Unit, scope, baseline, workspace, project write-authority, and approval coordinates used for state-bound reuse and invalidation. |
| `invalidation_reason` | Controlled invalidation reason or `null`. | Stable reason recorded when the ticket is invalidated. |
| `idle_expires_at` | UTC timestamp or `null`. | Optional project-policy idle boundary. `null` means no idle timeout; there is no fixed default lifetime. |
| `guarantee_display` | `GuaranteeDisplay | null`. | Human-display guarantee wording scoped by [Security](../security.md). |

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
- `WriteTicket.state` and `WriteTicketStateSummary.status` values: [method-local values](schema-value-sets.md#method-local-values)
- `WriteDecisionReason.category` values: [state and blocker values](schema-value-sets.md#state-and-blocker-values)
- `WriteDecisionReason.code` value-set boundary: [opaque and method-scoped string fields](schema-value-sets.md#opaque-and-method-scoped-string-fields)
- `WriteDecisionReason.code` production and local meaning: method owner documents, including [`volicord.prepare_write`](method-prepare-write.md)
- Write-ticket issuance behavior: [`volicord.prepare_write`](method-prepare-write.md)
- Write-ticket product meaning and approval boundaries: [Core Model](../core-model.md)
- Public `ErrorCode` values are separate: [API error codes](error-codes.md)

## Evidence and run snapshot shapes

```schema
AcceptanceCriterionInput:
  statement: string
  evidence_requirement: string

AcceptanceCriterionReplacement:
  acceptance_criterion_id: string | null
  statement: string
  evidence_requirement: string

AcceptanceCriterion:
  acceptance_criterion_id: string
  statement: string
  evidence_requirement: string

EvidenceTarget:
  target_kind: acceptance_criterion | supplemental_claim
  acceptance_criterion_id: string  # acceptance_criterion only
  evidence_claim_id: string        # supplemental_claim only
  statement: string                # supplemental_claim only

EvidenceCaptureSpec:
  capture_kind: verified_command_execution | verified_tool_invocation
  command_sha256: string                       # verified_command_execution only
  command_label: string                        # verified_command_execution only; normalized, 1..256 UTF-8 bytes
  expected_exit_code: integer | null           # verified_command_execution only
  tool_name: string                            # verified_tool_invocation only; trimmed, 1..256 UTF-8 bytes
  tool_input_sha256: string                    # verified_tool_invocation only
  expected_success: boolean | null             # verified_tool_invocation only

EvidenceCaptureIntent:
  capture_intent_id: string
  project_id: string
  task_id: string
  change_unit_id: string
  scope_revision: integer
  baseline_ref: string
  target: EvidenceTarget
  capture: EvidenceCaptureSpec
  input_sha256: string
  expected_outcome: object
  requested_by_actor_source: string
  workspace_context: object
  created_at: string
  expires_at: string

EvidenceCaptureReceipt:
  capture_receipt_id: string
  capture_intent_id: string
  capture_intent_ref: StateRecordRef
  producer_kind: string
  project_id: string
  task_id: string
  change_unit_id: string
  scope_revision: integer
  baseline_ref: string
  target: EvidenceTarget
  input_sha256: string
  result_sha256: string
  expected_outcome: object
  observed_outcome: object
  source_refs: StateRecordRef[]
  connection_id: string
  host_invocation_id: string | null
  staged_receipt_handle: StagedArtifactHandle
  complete: boolean
  limitations: string[]
  redaction_state: string
  observed_by_actor_source: string
  observed_at: string
  recorded_at: string

EvidenceProducer:
  evidence_producer_id: string
  capture_receipt_id: string
  capture_intent_id: string
  capture_intent_ref: StateRecordRef
  producer_kind: string
  project_id: string
  task_id: string
  change_unit_id: string
  scope_revision: integer
  baseline_ref: string
  target: EvidenceTarget
  input_sha256: string
  result_sha256: string
  expected_outcome: object
  observed_outcome: object
  source_refs: StateRecordRef[]
  connection_id: string
  host_invocation_id: string | null
  receipt_artifact_refs: ArtifactRef[]
  complete: boolean
  limitations: string[]
  redaction_state: string
  observed_by_actor_source: string
  observed_at: string
  finalized_at: string
  run_ref: StateRecordRef
  observation_ref: StateRecordRef

EvidenceSummary:
  evidence_state: string
  status: string
  coverage_items: EvidenceCoverageItem[]
  artifact_refs: ArtifactRef[]
  observation_refs: StateRecordRef[]
  updated_by_run_ref: StateRecordRef | null

EvidenceGateSummary:
  state: string

EvidenceCoverageItem:
  target: EvidenceTarget
  coverage_state: string
  supporting_run_refs: StateRecordRef[]
  observation_refs: StateRecordRef[]
  supporting_artifact_refs: ArtifactRef[]
  gap_refs: StateRecordRef[]

EvidenceCoverageUpdate:
  target: EvidenceTarget
  coverage_state: string
  provenance: EvidenceUpdateProvenance | null
  supporting_run_refs: StateRecordRef[]
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
  source_refs: SourceRef[]
  limitations: string[]

EvidenceObservation:
  observation_id: string
  project_id: string
  task_id: string
  change_unit_id: string | null
  run_ref: StateRecordRef | null
  target: EvidenceTarget
  source_kind: string
  assurance_level: string
  producer_anchor: EvidenceProducerAnchor
  relevance_assessment: EvidenceRelevanceAssessment
  observed_by_actor_source: string | null
  tool_name: string | null
  tool_invocation_id: string | null
  tool_metadata: object
  input_refs: StateRecordRef[]
  source_refs: SourceRef[]
  output_artifact_refs: ArtifactRef[]
  limitations: string[]
  observed_at: string
  recorded_at: string

EvidenceProducerAnchor:
  producer_kind: string
  producer_ref: StateRecordRef | null
  output_artifact_refs: ArtifactRef[]
  verification_basis: string | null

EvidenceRelevanceAssessment:
  status: string
  assessment_ref: StateRecordRef | null
  assessed_by_actor_source: string | null

EvidenceObservationInput:
  target: EvidenceTarget
  source_kind: string
  assurance_level: string
  observed_by_actor_source: string | null
  tool_name: string | null
  tool_invocation_id: string | null
  tool_metadata: object
  input_refs: StateRecordRef[]
  source_refs: SourceRef[]
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
- `AcceptanceCriterionInput` is used by intake and never accepts an ID.
  `AcceptanceCriterionReplacement` is used only in a non-null update-scope
  replacement set. A current ID from the same `Task` preserves identity, `null` requests a
  new Core-generated ID, and omission from the replacement set retires the
  previous current criterion. Unknown, retired, cross-Task, and duplicate IDs
  are invalid.
- `AcceptanceCriterion.acceptance_criterion_id` is an opaque Core-generated
  identifier. Its `statement` is display text, while `evidence_requirement`
  selects `required`, `optional`, or `not_required`.
- `EvidenceTarget` is a strict tagged union. The `acceptance_criterion` variant
  contains only `acceptance_criterion_id`. The `supplemental_claim` variant
  contains caller-assigned Task-scoped `evidence_claim_id` and a non-empty
  immutable `statement`. Variant fields must not be mixed.
- `EvidenceCaptureSpec` is a strict tagged union. Its caller-supplied lowercase
  64-character digest fields bind exact command or tool input. Expected-outcome
  members are nullable in the typed shape and use method-owned omission
  defaults on MCP.
- `EvidenceCaptureIntent` is the immutable, expiring current-basis request. Its
  `requested_by_actor_source` and `workspace_context` are Core-derived basis
  fields, not caller-selected attribution. Its public ref uses
  `record_kind=evidence_capture_intent`.
- `EvidenceCaptureReceipt` is an immutable durable source-fulfillment fact record.
  Its associated staging handle and staged receipt bytes are transient. Its
  registered connection and host invocation identity, outcome, completeness,
  limitations, redaction state, observer, and times are source facts. The
  receipt is not a `StateRecordRef` and does not advance Core state.
- `EvidenceProducer` is the immutable Core-finalized authority record created
  one-to-one with the consuming Run observation. Its receipt artifacts, Run
  ref, and observation ref bind source bytes, producer, and relevance. Its
  public ref uses `record_kind=evidence_producer`.
- `EvidenceSummary.evidence_state`, when present, is an evidence display state. It is omitted for coverage-gap summaries that do not yet have attached evidence or a current close-basis evidence ref.
- `EvidenceGateSummary` is the canonical derived evidence-gate projection for the current active criteria and close-evaluation basis. Core computes it once from criterion requirements and coverage plus current evidence provenance, freshness, artifact availability, and evidence-related close blockers. `StateSummary`, status and close results, and `SummaryCard.evidence` copy that result; they do not recalculate it independently. It is not a stored authority record or an `AuthorityReceipt`.
- `EvidenceSummary.status`, `EvidenceCoverageItem.coverage_state`,
  `EvidenceCoverageUpdate.coverage_state`, `EvidenceUpdateProvenance.source_kind`,
  `EvidenceUpdateProvenance.assurance_level`, `EvidenceObservation.source_kind`,
  `EvidenceObservation.assurance_level`, `EvidenceObservationInput.source_kind`,
  `EvidenceObservationInput.assurance_level`, and `RunSummary.kind` are
  controlled value strings.
- `RunSummary.summary`, acceptance-criterion statements, and supplemental claim
  statements are free-form display strings. They are never evidence identity.
- `EvidenceCoverageUpdate.provenance` is optional on request input and is
  omitted from committed `EvidenceCoverageItem` values after Core creates or
  links the corresponding target-matching `EvidenceObservation`. A `supported`
  update must have a target-matching observation input, a usable
  target-matching observation ref, or this provenance object.
- `supporting_run_refs` accepts same-Task Run refs. `observation_refs`,
  `supporting_artifact_refs`, and `gap_refs` preserve target-specific
  observation, artifact, and gap relations.
- `EvidenceSummary.observation_refs` and `EvidenceCoverageItem.observation_refs` list `StateRecordRef` values for committed evidence observations that Core relates to the summary or target.
- `EvidenceObservation` is a durable provenance record for one evidence target.
  `producer_anchor` separately identifies the Core-validated producer record and
  its exact outputs; `relevance_assessment` separately identifies whether an
  authority source assessed those outputs as supporting the target. Byte
  integrity, producer provenance, basis freshness, target identity, and claim
  relevance remain distinct checks.
- An `evidence_observation` `UserActionResolution` is the User Channel-owned,
  target- and basis-bound relevance record. Its closed nested observation body
  binds exact canonical artifact refs. An exact stored relevance of `supported`
  or `contradicted` establishes user-observed producer provenance while
  preserving that same status as the separate relevance assessment. The
  committed observation uses the enclosing resolution's `resolved_at` as
  `observed_at`, not the caller's `EvidenceObservationInput.observed_at`. A
  contradicted observation is negative relevance and cannot satisfy supported
  coverage, evidence sufficiency, or validated reuse that establishes
  `supported`; it is not a judgment resolution or final acceptance. Its public
  shape is owned by [API User Action Schemas](schema-user-action.md).
- `source_refs` uses `SourceRef`. `input_refs` remains a separate `StateRecordRef[]`; a source ref never becomes a Core state ref or close-basis result ref.
- `EvidenceObservationInput` is the request-side shape accepted by `volicord.record_run`; Core fills `observation_id`, project and Task coordinates, `run_ref`, `recorded_at`, and the observer actor source when it commits. Request-side source and assurance values are provenance claims, not caller-granted assurance.
- Only coverage for a current criterion with
  `evidence_requirement=required` participates in close authority. Required
  criteria reject `coverage_state=not_applicable`; `optional`, `not_required`,
  supplemental, and retired targets remain non-authoritative for close.
- Submitted `observed_by_actor_source` does not select the committed actor. Core
  derives it from a validated producer record when present and otherwise from
  the verified invocation; a submitted value cannot raise trust or impersonate
  another actor source.
- Core derives committed `source_kind` and `assurance_level` from verified anchors. An unanchored direct `connection_observation`, `user_observation`, `external_tool`, or caller-declared `reused_evidence` input is committed as `agent_report` / `cooperative_report`. These fields never by themselves prove product correctness, grant user authority, satisfy final acceptance, satisfy residual-risk acceptance, or raise `GuaranteeDisplay.level`.
- A current complete capture receipt consumed through exactly one
  `evidence_capture_intent` input ref can establish an authority-owned verified
  command or verified tool producer. A
  direct `external_tool` or `connection_observation` input without that anchor
  remains cooperative even when its artifacts have verified bytes.
- `user_observation` requires a current `evidence_observation`
  `UserActionResolution`, exact output equality, an exact stored
  `relevance_status` of `supported` or `contradicted`, verified local-user
  provenance, and a matching Task, Change Unit, scope, baseline, and target.
  Core preserves that exact status in `relevance_assessment` and derives
  `observed_at` from the resolution's `resolved_at`; only `supported` may
  satisfy coverage or sufficiency or qualify for validated reuse that
  establishes `supported`.
- `reused_evidence` is Core-derived only after every recursive observation's
  strict persisted producer/relevance metadata, exact outputs, target, current
  basis, source Run, and inherited assurance are revalidated.
- `unverified_claim` and `unverified` preserve an asserted claim without verified observation and are not sufficient evidence by themselves.
- `tool_metadata` is descriptive metadata and must not be treated as authority, approval, or a storage effect.
- `ObservedChanges.changed_paths` are path strings.
- `ObservedChanges.sensitive_categories` are opaque sensitive-category classification strings unless an affected method or profile owner publishes a narrower local list.
- `ObservedChanges.baseline_ref` is an opaque baseline identifier.

Owner links:
- `ArtifactRef`: [API Artifact Schemas](schema-artifacts.md)
- evidence, `coverage_state`, evidence observation, and run-kind values: [state and blocker values](schema-value-sets.md#state-and-blocker-values), [evidence observation values](schema-value-sets.md#evidence-observation-values), and [method-local values](schema-value-sets.md#method-local-values)
- evidence observation actor values: [actor values](schema-value-sets.md#actor-values)
- Evidence sufficiency meaning: [Core Model evidence and run authority](../core-model.md#9-evidence-and-run-authority)
- Method behavior: method owner documents routed from [API Methods](methods.md)

<a id="close-readiness-and-validation-shapes"></a>
## Close readiness and validation shapes

```schema
CurrentCloseBasis:
  close_basis_revision: integer
  scope_revision: integer
  task_id: string
  change_unit_id: string
  baseline_ref: string | null
  result_summary: string
  result_refs: StateRecordRef[]
  evidence_refs: StateRecordRef[]
  evidence_summary_ref: StateRecordRef | null
  residual_risks: ResidualRisk[]
  sensitive_categories: string[]
  sensitive_action_requirements: SensitiveActionRequirement[]
  recovery_constraints: string[]
  source_run_ref: StateRecordRef | null
  shaping_checkpoint_ref: StateRecordRef | null
  shaping_decision_application_refs: StateRecordRef[]
  updated_at: string

SensitiveActionRequirement:
  action_kind: string
  normalized_paths: string[]
  sensitive_categories: string[]
  baseline_ref: string | null
  change_unit_id: string
  source_run_ref: StateRecordRef
  source_write_ticket_ref: StateRecordRef

ResidualRisk:
  risk_id: string
  summary: string
  consequence: string
  acceptance_required: boolean
  source_refs: StateRecordRef[]

RiskAcceptanceCoverage:
  risk_id: string
  accepted: boolean
  accepted_by_user_action_resolution_refs: StateRecordRef[]
  missing_reason: string | null

CloseReadinessBlocker:
  category: string
  code: string
  message: string
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

GuaranteeDisclosure:
  guarantee_class: string
  guarantees: string[]
  non_guarantees: string[]
```

Meaning:
- `CurrentCloseBasis` is the current result and residual-risk state used by close-readiness responses. It is not a terminal close summary.
- `close_basis_revision` and `scope_revision` are internal current-state coordinates surfaced for compatibility checks. They are not caller-selected authority.
- `ResidualRisk.risk_id` is an opaque Core-generated identifier. `ResidualRisk.summary` and `ResidualRisk.consequence` are display strings and do not authorize text matching.
- `result_refs`, `evidence_refs`, `source_run_ref`, `shaping_checkpoint_ref`,
  `shaping_decision_application_refs`, `source_refs`,
  `evidence_summary_ref`, and `accepted_by_user_action_resolution_refs` use
  `StateRecordRef`.
- `sensitive_categories` are opaque sensitive-category classification strings unless an affected method or profile owner publishes a narrower local list.
- `sensitive_action_requirements` are Core-derived close requirements from committed Runs and consumed write tickets. Category-only caller input cannot establish or erase these requirements.
- `recovery_constraints` and `RiskAcceptanceCoverage.missing_reason` are display strings. Current close-readiness results use `acceptance_required` when required acceptance is absent and may use `stale_acceptance` when a non-current residual-risk acceptance exists but does not cover the current residual-risk `risk_id` values.
- `RiskAcceptanceCoverage` reports whether the current residual-risk requirements are covered by compatible user-action resolutions. It does not report evidence sufficiency or final acceptance.
- `CloseReadinessBlocker` is a data shape for close-readiness findings.
- `CloseReadinessBlocker.category` is a controlled value string.
- `CloseReadinessBlocker.code` is an owner-defined blocker code. It is not an exhaustive global public enum unless the blocker or method owner publishes a narrower local list.
- `CloseReadinessBlocker.message`, `ValidatorResult.message`, and `GuaranteeDisplay.basis` are free-form display strings.
- `ValidatorResult.validator_id` is a reporting label unless the value-set owner publishes a supported stable value.
- `ValidatorResult.status`, `ValidatorResult.severity`, and `GuaranteeDisplay.level` are controlled value strings.
- `GuaranteeDisclosure` is the result-interpretation disclosure returned by public result bases and diagnostic outputs when a reader might otherwise overinterpret the result.
- `GuaranteeDisclosure.guarantee_class` and `GuaranteeDisclosure.non_guarantees` are controlled value strings. `GuaranteeDisclosure.guarantees` are concise display statements.
- `GuaranteeDisplay` describes the current capability display for a status or compatibility view. It does not replace `GuaranteeDisclosure`.

These shapes do not define close-readiness meaning, response routing, or persistence behavior.

Close-basis reference rules:
- Direct/work bases have a non-null exact compatible `source_run_ref`, a null
  `shaping_checkpoint_ref`, and no shaping decision application refs. Advisor
  bases have a null `source_run_ref`, the exact current shaping checkpoint,
  and the exact set of current checkpoint application refs.
- Caller-supplied direct/work close-assessment refs accepted into `CurrentCloseBasis.result_refs` or `ResidualRisk.source_refs` are limited to result/evidence record kinds `run`, `artifact`, `evidence_summary`, and `change_unit` unless an owner document explicitly adds another kind. Advisor finalization accepts current same-Task `change_unit`, `artifact`, and `evidence_summary` result refs and `artifact` or `evidence_summary` evidence refs; it does not accept a Run.
- `project_state`, `write_ticket`, `user_action_request`,
  `user_action_resolution`, `blocker`, `task_event`, and `task` are not
  caller-supplied result refs for a close basis unless an owner document
  explicitly adds them.
- Every accepted ref must exist, belong to the same project and `Task`, and be canonicalized by Core. Core never treats caller-supplied `produced_at_state_version` metadata as authority or concurrency input.
- Artifact refs used for close evidence must be linked to the Task and have `integrity_status=verified` plus current-byte verification at use time under [Artifact Storage](../storage-artifacts.md).
- Evidence refs must identify the current Task evidence summary. Run refs used as current close-basis result refs must identify a recorded current Run compatible with the current Task, current Change Unit, current scope revision, compatible baseline, and recorded status. Historical Runs are audit records unless a current Run explicitly reuses their `verified` artifacts or evidence and records that reuse.
- Core may add the current Run, current Change Unit, and current EvidenceSummary refs when constructing the canonical close basis.

Guarantee display rules:
- `GuaranteeDisplay` is derived from the project enforcement profile, verified invocation context, enabled enforcement mechanisms, and supported baseline scope.
- `capability_refs` is the implemented field name for references that justify the display; in the baseline connection architecture it should cite invocation binding, Agent Connection, or observation facts when such refs are available.
- A cooperative `agent_report` Run or observation is not displayed as
  externally observed unless a separate supporting record justifies that
  display.

Owner links:
- Close-readiness meaning and non-substitution rules: [Core Model close readiness](../core-model.md#close_task)
- Current close basis creation: [`volicord.record_run`](method-record-run.md)
  for direct/work and [`volicord.finalize_advice`](method-finalize-advice.md) for
  advisor
- Judgment compatibility and accepted-risk input: [API Judgment Schemas](schema-judgment.md)
- Response branch behavior, close-readiness evaluation order, and response-only blocked outcomes: [`volicord.check_close` and `volicord.close_task`](method-close-task.md)
- Close-readiness blocker/API response routing semantics: [API blocker routing](blocker-routing.md)
- Supported `CloseReadinessBlocker.category`, `ValidatorResult.status`, `ValidatorResult.severity`, and `GuaranteeDisplay.level` values: [API Value Sets](schema-value-sets.md#state-and-blocker-values)
- Security guarantee meaning: [Security](../security.md)

## Related owners

- [API Schema Core](schema-core.md) for `ToolEnvelope`, effect-specific result
  metadata, method-specific result bases, `ToolRejectedBase`,
  `ToolDryRunBase`, `ToolRejectedResponse`, and `ToolDryRunResponse`.
- [API Value Sets](schema-value-sets.md#state-and-blocker-values) for exact `CloseReadinessBlocker.category` values and neighboring state values.
- [API Methods](methods.md) and method owner documents for the methods that return these schemas.
- [API Artifact Schemas](schema-artifacts.md) for `ArtifactRef`.
- [API User Action Schemas](schema-user-action.md) for durable action requests
  and adapter-neutral resolution forms.
- [Storage Effects](../storage-effects.md) for persistence and state-effect consequences.
