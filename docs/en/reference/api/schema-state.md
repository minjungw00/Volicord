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
| State references, current Task position, lifecycle, and shaping readiness | [State references](#state-references) |
| Administrative host-feature support diagnostics | [Host feature support diagnostics](#host-feature-support-diagnostics) |
| Host-hook observation and session-watch coverage | [Guard health summary](#guard-health-summary) |
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
- Record identity is exactly the tuple (`project_id`, `record_kind`, `record_id`). `task_id` is nullable Task context and is not part of record identity.
- `produced_at_state_version` is the nullable `project_state.state_version` of the aggregate projection that produced the reference. It is a projection-freshness cue, not record identity, a record revision, a close-basis revision, authority, or an optimistic-concurrency token. The same logical record can therefore appear with different `produced_at_state_version` values without becoming a different record.
- When a method response emits a ref from its current projection, non-null `produced_at_state_version` matches the `project_state.state_version` used for that response projection, even when the referenced record last changed earlier. Exact replay retains the originally stored response and its original projection-freshness values.
- A `StateRecordRef` never supplies concurrency input. A caller that invokes a mutation uses the current project clock in `ToolEnvelope.expected_state_version` when the method owner requires it.

It is a public reference, not an embedded storage row.

```yaml
StateRecordRef:
  record_kind: string
  record_id: string
  project_id: string
  task_id: string | null
  produced_at_state_version: integer | null
```

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

```yaml
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

`StateSummary` is the compact current-position state returned by supported methods that need to show the current Task path.

```yaml
StateSummary:
  project_id: string
  state_version: integer
  task_ref: StateRecordRef | null
  mode: string | null
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
  shaping_readiness: ShapingReadiness | null
  pending_user_action_summaries: AgentSafeUserActionRequestSummary[]
  blocker_refs: StateRecordRef[]
  write_ticket_summary: WriteTicketStateSummary | null
  evidence_summary: EvidenceSummary | null
  evidence_gate: EvidenceGateSummary | null
  close_state: string | null
  close_blockers: CloseReadinessBlocker[]
  guard_health: GuardHealthSummary | null
  guarantee_display: GuaranteeDisplay | null
```

Meaning:
- `StateSummary` is a compact response shape for state references, summaries, and close-readiness fields.
- Method include flags may select only part of this shape. When a method owner says a projection is not selected, include-controlled fields such as `evidence_summary`, `evidence_gate`, `close_state`, `close_blockers`, `guard_health`, or `guarantee_display` are omitted instead of being returned as null or empty. A returned empty array means the projection was computed and found empty.
- `mode`, `work_phase`, `acceptance_policy`, and `close_state` are controlled
  value strings when present. `acceptance_policy_reason` records why Core chose
  the Task-owned final-acceptance policy; it is not an approval or waiver.
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

```yaml
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
  next_actor: string
  next_action: NextActionSummary | null
```

Meaning:

- `TaskLineageSummary` records exactly one predecessor relation. Applied
  carry-forward becomes newly validated Task input; `reference_only` preserves
  predecessor context without making its authority current.
- `TaskFlowItem[]` is a full-status projection over the connected predecessor
  component. It is derived display, not a new parent-goal record.
- `WorkspaceContext` uses the canonical Git common-directory and linked-
  worktree identity shared by local integration and Core write checks. A null
  branch represents detached HEAD. Non-Git repositories return null context.
- `AuthorityReceipt` is Core-generated from one freshly read project state
  version. Its blocker list is complete even when optional status projections
  are omitted. `product_file_write_observed` describes the latest recorded Run,
  not every historical Run. The receipt does not itself commit, close, accept,
  or prove product correctness.

<a id="host-feature-support-diagnostics"></a>
## Host feature support diagnostics

Every administrative support evaluation first exposes the same exact six-key
map:

```yaml
HostFeatureSupportMap:
  native_user_action: HostFeatureSupportStatus
  local_web_user_channel: HostFeatureSupportStatus
  verified_tool_producer: HostFeatureSupportStatus
  registered_connection_observation: HostFeatureSupportStatus
  record_final_output: HostFeatureSupportStatus
  detective_final_output: HostFeatureSupportStatus
```

All six keys are required and no additional feature key is allowed. A final-
output diagnostic is profile-specific detail alongside that map, not a
replacement for it:

```yaml
FinalOutputAuthorityDisclosureDiagnostic:
  support_status: HostFeatureSupportStatus
  configured: boolean
  configuration_verified: boolean
  required_subcapabilities: string[]
  subcapabilities: object<string, HostFeatureSupportStatus>

DoctorHostFeatureSupportRow:
  connection_id: string
  host_kind: string
  selected_profile: string | null
  host_feature_support: HostFeatureSupportMap
  final_output_authority_disclosure: FinalOutputAuthorityDisclosureDiagnostic | null
```

Record emits exactly `authority_display` and `authenticated_exact_replay` in
both `required_subcapabilities` and `subcapabilities`. Detective emits those
two plus `block_finalization`. No non-applicable key is emitted. The aggregate
`support_status` uses the precedence owned by
[Agent Connection](../agent-connection.md#host-feature-support-state).
`configured` and `configuration_verified` are independent configuration facts
and never imply `verified`.

The machine-readable projections are exact:

- connection status always places `HostFeatureSupportMap` at
  `states.host_feature_support`; it places profile detail at
  `states.final_output_authority_disclosure` only for an exact `record` or
  `detective` installation profile. Otherwise it preserves the raw
  `states.selected_profile` value, mirrors it at
  `states.control_surface.selected_profile`, and emits null detail rather than
  defaulting to Record. The `host_hook` config/audit object does not duplicate
  either typed field;
- `connection add --dry-run --json` has no exact installed profile, so its
  planned state preserves `selected_profile=not_configured`, keeps the complete
  map, emits null profile detail, and does not duplicate either typed field in
  `host_hook`;
- doctor places one `DoctorHostFeatureSupportRow` per readable stored Agent
  Connection at `states.host_feature_support_by_connection`, ordered by
  `connection_id`; each row contains exactly the five fields shown above;
- every terminal release-feature matrix cell, regardless of passed,
  incomplete, failed, or unavailable result, places the complete
  `HostFeatureSupportMap` at `host_feature_support` and exact selected-profile
  detail at `final_output_authority_disclosure`, or null detail when no exact
  profile exists. Post-init cells copy the product-produced init projection;
  preflight-unavailable cells use the centralized default projection with
  configuration facts false, without erasing or reclassifying static support
  status. The create-new recorder emits only terminal cell artifacts and does
  not persist a provisional `result=running` shape. A terminal
  `result=failed_before_completion` artifact uses the recorder's exact profile
  hint and the default projection, or null detail when no exact profile is
  available; it never defaults to Record.

Doctor emits an empty `host_feature_support_by_connection` array when the
Registry has no readable connection rows; it does not synthesize a feature map
from an unreadable connection. `selected_profile` and the profile detail are
both null when no exact profile can be selected for that connection. This
administrative diagnostic schema is not added to Core method results merely by
being defined here.

## Guard health summary

`GuardHealthSummary` is the compact detective host-hook and observation-state projection returned by close-readiness and status views when the method owner selects it. The `guard_*` field names are schema identifiers for internal host observation records and hook-related implementation state; they are not a public security mode or security boundary.

```yaml
ControlSurfaceSummary:
  selected_profile: string
  host_hooks_active: boolean
  session_watcher_active: boolean
  cooperative_pre_tool_warning_available: boolean
  cooperative_pre_tool_denial_available: boolean
  unrecorded_changes_detectable: boolean
  actor_identity_provable: boolean
  os_enforced: boolean

GuardHealthSummary:
  selected_profile: string
  control_surface: ControlSurfaceSummary
  guard_installation_id: string | null
  guard_installation_status: string
  guard_configuration_status: string
  guard_observation_status: string
  effective_guard_status: string
  generated_config_verified: boolean
  native_host_output_adapter_config_verified: boolean
  hook_path_safety: string
  hook_commands_cwd_independent: boolean
  hook_commands_subdirectory_safe: boolean
  cooperative_pre_tool_warning_available: boolean
  cooperative_pre_tool_denial_available: boolean
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
  mcp_connection_healthy: boolean
  mcp_connection_status: string | null
  session_watch_status: string
  last_session_watch_checked_at: string | null
  session_watch_baseline_created_at: string | null
  session_watch_coverage_start_at: string | null
  session_watch_coverage_basis: string | null
  session_watch_partial_coverage_warning: string | null
  session_watch_detail: string | null
  session_watch_scan_summary: SessionWatchScanSummary | null
  unresolved_unrecorded_change_count: integer
  missing_or_stale_write_ticket: boolean
  write_ticket_path_scope_violation: boolean

SessionWatchScanSummary:
  files_scanned: integer
  files_skipped: integer
  unreadable_paths_count: integer
  degraded_reasons: string[]
  degraded_reason_counts: object
  skipped_paths_sample: string[]
  skipped_paths_truncated: boolean
  default_excluded_paths: string[]
  max_file_size_bytes: integer
  max_file_count: integer
  follows_symlinks: boolean
  not_full_filesystem_monitoring: boolean

CoverageSummary:
  active_profile: string
  host_hook_state: string
  session_watcher_state: string
  coverage_started_at: string | null
  last_snapshot_at: string | null
  watcher_scan_summary: SessionWatchScanSummary | null
  unresolved_unrecorded_change_count: integer
  non_guarantees: NonGuarantee[]
```

Meaning:
- `selected_profile` and `guard_installation_status` are controlled value strings.
- `control_surface` is the public observation summary of what Volicord can currently observe or decide. It reports the selected profile, whether host hooks and a session watcher are active, whether cooperative pre-tool warning or denial is available, whether unrecorded changes can be detected, whether actor identity can be proven, and whether OS enforcement is provided.
- `guard_installation_id`, when non-null, is an opaque internal host-hook installation identifier.
- `guard_configuration_status`, `guard_observation_status`, and `effective_guard_status` separate file/config health, runtime hook observation, and the effective detective-profile close-readiness status. Stored `configured` and `active` installation rows both project configured configuration health; for `detective`, that configured health plus a current matching observation projects effective `active`. A same-identity setup refresh may therefore leave the lifecycle row `configured` while a preserved matching observation keeps effective health active. `reload_required`, `degraded`, `stale`, and `broken` remain non-active.
- `generated_config_verified`, `native_host_output_adapter_config_verified`, `hook_path_safety`, `hook_commands_cwd_independent`, `hook_commands_subdirectory_safe`, `cooperative_pre_tool_warning_available`, `cooperative_pre_tool_denial_available`, `post_tool_correlation_available`, `bash_shell_mutation_coverage`, `direct_file_write_matcher_coverage`, `bypass_detection_active`, `prompt_capture_available`, and `local_web_consent_available` expose capability or configuration facts for the selected profile. `native_host_output_adapter_config_verified` is configuration-only close gating and does not claim host-feature support or live delivery. Detective host hooks require verified generated config, native host output configuration, `hook_path_safety=ok`, cwd-independent and subdirectory-safe required hook commands, required lifecycle phases, Bash/shell and direct file-write matcher coverage, a matching policy hash, and a current matching host-hook observation. Unrecorded-change detection requires an active session watch; a partial coverage warning remains visible in `session_watch_partial_coverage_warning`. A setup diagnostic that cannot observe a runtime-only capability reports that capability as false.
- `guard_hook_observed` reports whether a current matching host-hook observation is recorded for the selected internal host-hook installation record. A current matching observation has a parseable timestamp, an observed host and policy hash matching the current installation and its exact `volicord-host-hook-capability-v2` capability, and a known observation phase configured by that capability's current lifecycle commands; any missing, malformed, unknown, or mismatched fact fails closed. When a connection-wide projection aggregates one Agent Connection across Connection Projects, this field is true only when every applicable Detective installation has a current matching observation.
- `last_guard_observed_at` is the latest stored internal host-hook installation observation timestamp, or `null` when no observation is recorded. Its value reports the latest stored timestamp even when that observation is no longer current; observation currentness is represented by `guard_hook_observed` and the effective guard state.
- `last_guard_event_at` is the latest host-hook event timestamp available to the projection, or `null` when no host-hook event is available.
- `host_kind`, `observed_hook_phase`, `observed_host_kind`, `expected_policy_hash`, `observed_policy_hash`, and `observed_binary_version` report the selected installation and latest stored observation metadata when available.
- `required_hook_phases` and `missing_required_hook_phases` report required host-hook configuration completeness. This public projection deliberately fails closed: a required phase is missing when it is absent from `required_hook_phases` or listed in `missing_required_hook_phases`. By contrast, a valid stored `volicord-host-hook-capability-v2` Detective record always declares the canonical five required phases and represents degradation only by listing a duplicate-free subset in `missing_required_hooks`; [Storage Records](../storage-records.md) owns that stored contract. The projection retains the absent-or-listed rule so corrupt or independently constructed input cannot become complete by omission. Missing required phases keep effective detective health non-active even when a valid hook event has been observed.
- `prompt_capture_status` reports the machine-readable prompt-capture availability state for the selected connection. `prompt_capture_available=true` only when that state allows verification-code chat commands; it does not mean raw prompt text is included.
- `prompt_capture_available` reports whether prompt-capture verification-code chat commands may be shown or recorded for the selected connection. It does not include prompt text.
- `local_web_consent_available=true` only when the current adapter invocation's
  centralized evaluator observes a managed non-generic stdio host, a ready
  loopback listener, the exact boolean `true` declaration at
  `params.capabilities.experimental["io.volicord/user-channel"].model_invisible_user_surface`,
  and current exact-match persisted host-capability state with an unexpired
  `outcome=passed` result whose interval satisfies
  `observed_at <= created_at < expires_at <= observed_at + 86,400 seconds`;
  the row's `evidence_artifact_sha256` must also exactly match the expected
  digest in a separately verified exact-final-artifact release evidence
  manifest or receipt outside the executable, bound to the same capability,
  host/client, adapter, build, source, target, and executable digest;
  current
  evaluation uses `observed_at <= now < expires_at`. Twenty-four hours is the
  maximum freshness window, not a default lifetime or attestation period. An
  omitted, false, wrong-typed, malformed, or wrong-
  namespace declaration, or absent, non-passing, expired, revoked, corrupt, or
  mismatched verification, means `false`. This fact does not mean a token was
  issued, a form was rendered, a user was identified, or model isolation was
  proved. Status and check-close use the same evaluator and never issue a token
  merely to report availability; setup diagnostics that cannot observe every
  runtime and persisted input also report `false`.
  A missing, unknown, malformed, unverified, or mismatched manifest also means
  `false`; the row and build metadata cannot self-supply its expected digest.
  The current adapter has no trusted manifest acquisition path, so production
  projections report this capability as unavailable.
- `mcp_connection_healthy` and `mcp_connection_status` summarize the tracked Agent Connection verification state when that state is available.
- `session_watch_status` reports whether the session-level Product Repository watcher is `disabled`, `active`, `degraded`, `unavailable`, or `pending_project_selection` for the selected connection or session.
- `last_session_watch_checked_at` is the latest watcher baseline status update timestamp, or `null` when no session-watch baseline is available.
- `session_watch_baseline_created_at` is the stored baseline creation timestamp, or `null` when no session-watch baseline is available.
- `session_watch_coverage_start_at` is the timestamp from which the watcher baseline can claim coverage for the selected session, or `null` when no coverage start is available.
- `session_watch_coverage_basis` is `mcp_start`, `first_project_selection`, `method_boundary`, or `null`.
- `session_watch_partial_coverage_warning` is a human-readable warning when Product Repository changes before the recorded coverage start are outside watcher coverage.
- `session_watch_detail` is a short diagnostic detail for the selected watcher state, or `null` when no detail is available.
- `session_watch_scan_summary` reports the selected watcher scan footprint when available. It includes files scanned, files skipped, unreadable path count, degraded reason counts, a sample of skipped paths, default policy exclusions, file-size and file-count limits, `follows_symlinks=false`, and `not_full_filesystem_monitoring=true`.
- `unresolved_unrecorded_change_count` is a count of unresolved unrecorded Product Repository changes. It does not expose prompt text, command text, or path lists.
- `missing_or_stale_write_ticket` reports whether host-hook events detected missing, indeterminate, ambiguous, or stale write-ticket readiness.
- `write_ticket_path_scope_violation` reports whether host-hook events observed a Product Repository path outside the active write-ticket scope.
- `CoverageSummary` is a concise, derived coverage view selected by status and close-readiness results. `active_profile` is the current `record` or `detective` profile; `host_hook_state` is `observed`, `not_observed`, `unsupported`, or `degraded`; and `session_watcher_state` is `active`, `inactive`, `unsupported`, or `degraded`.
- `coverage_started_at` is the session-watch coverage start timestamp when the runtime tracks one, or `null` when unavailable. `last_snapshot_at` is the latest watcher baseline or snapshot-status timestamp when tracked, or `null` when unavailable.
- `CoverageSummary.watcher_scan_summary` mirrors `GuardHealthSummary.session_watch_scan_summary` when coverage is selected.
- `CoverageSummary.unresolved_unrecorded_change_count` mirrors the unresolved unrecorded Product Repository change count used by close readiness.
- `CoverageSummary.non_guarantees` must include `NotActorAttributionProof`, `NotFullFilesystemMonitoring`, and `NotFullWritePrevention` when coverage is reported.

Does not imply:
- `control_surface` is not proof of correctness, review completion, test sufficiency, OS-level enforcement, or write prevention.
- `GuardHealthSummary` is not evidence of product correctness, test sufficiency, OS enforcement, sandboxing, security isolation, or final acceptance.
- An active `effective_guard_status` does not replace evidence, artifact integrity, user-owned judgment, write-ticket, final acceptance, or residual-risk acceptance requirements.
- Session watch status and coverage metadata do not mean Volicord prevented a write, monitored the full filesystem, identified the actor who changed a file, stored file contents, or provided OS-level enforcement.
- When `session_watch_partial_coverage_warning` is non-null, Product Repository changes before `session_watch_coverage_start_at` remain outside session-watch coverage.
- `record` profile remains cooperative. Unresolved unrecorded-change findings still block close when detective control-surface health reports them.
- `detective` profile does not prevent all writes, monitor the full filesystem, identify the actor who changed a file, isolate the network, or provide a sandbox.

Owner links:
- `selected_profile`, `host_hook_state`, `session_watcher_state`, `hook_path_safety`, `guard_installation_status`, `guard_configuration_status`, `guard_observation_status`, `effective_guard_status`, `prompt_capture_status`, `session_watch_status`, and `session_watch_coverage_basis` values: [state and blocker values](schema-value-sets.md#state-and-blocker-values)
- Close-readiness `guard_*` blockers and method-local codes: [`volicord.check_close` and `volicord.close_task`](method-close-task.md)
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
  user_action_resolution_ref: StateRecordRef | null
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
- `user_action_resolution_ref` is non-null only for user-owned acceptance resolution.

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
- A project continuity record is not current Task authority, evidence, write ticket, final acceptance, close readiness, residual-risk acceptance for a future close basis, or a blocker waiver.
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
- It does not replace user-owned judgment, sensitive-action approval, evidence, write ticket, final acceptance, close readiness, or residual-risk acceptance.

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
- `ShapingReadiness` is an API view shape over Task, Change Unit, pending user action, evidence summary, blocker, and next-action fields.
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
  user_action_request_candidate_ref: StateRecordRef | null
```

Meaning:
- `ShapingGap` can reference a blocker or an owner-proposed user-action request
  candidate by shape; the candidate ref is not itself a resolution.
- `user_owned_blocker_kind` and `ShapingGap.gap_kind` are opaque readiness classification strings. They are not exhaustive public value sets unless an affected owner publishes narrower values.
- `ShapingGap.message` is a free-form display string.

Owner links:
- Method behavior and durable effects: method owner documents routed from [API Methods](methods.md) and [Storage Effects](../storage-effects.md)

<a id="current-position-display-shapes"></a>
## Current-position display shapes

```yaml
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
  presentation_role: string
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
  expires_at: string | null
  control_surface: ControlSurfaceSummary | null
  guarantee_display: GuaranteeDisplay | null

WriteDecisionReason:
  category: string
  code: string
  message: string
  related_refs: StateRecordRef[]
```

Meaning:
- `SummaryCard` is the stable compact summary shape for major user-facing status views. It uses public display strings for Task, Recording, Profile, Write Ticket, Evidence, User Judgment, Changes, Close Status, Transport, one Next action, and a concise Guarantee line.
- When evidence or close projection is selected, `SummaryCard.evidence` is the exact `EvidenceGateSummary.state` value owned by [API Value Sets](schema-value-sets.md#evidence-gate-values). It does not independently infer a state from staged input or `EvidenceSummary.evidence_state`.
- `SummaryCard.next` is the single display next action selected for the summary. Use `none` only when the owner-selected view knows no next action. `SummaryCard.next_action` may carry the matching structured `NextActionSummary` and may be omitted when no structured action applies. When a structured action applies, the summary selects the action whose `presentation_role=primary`; array position is not a selection contract.
- `SummaryCard` is a summary of other owner-selected state fields, not a second authority record. It must not add internal identifiers unless an identifier is needed for the displayed next action.
- For an already-existing pending user action, `SummaryCard.user_action`,
  `SummaryCard.next`, method `status_summary`, blocker messages, and every other
  display/template string stay generic. They may say that user action is
  pending and identify the User Channel as next actor, but must not reconstruct
  request question, options, context, form, path, command, URL, or credential.
- `SummaryCard.guarantee` is concise display wording for the summarized view. It must not claim correctness proof, test sufficiency proof, review completion, or OS-level enforcement unless another owner explicitly provides that guarantee.
- `NextActionSummary` is the canonical next-action display shape. Its valid fields are `presentation_role`, `action_kind`, `owner_method`, `allowed_operation_categories`, `label`, `blocking_question`, `expected_state_version`, and `required_refs`.
- Every non-empty top-level `next_actions` collection has exactly one `presentation_role=primary`. Remaining entries use `additional`. Close readiness is the explicit nested exception: `blockers[*].next_actions` flattened across one close-readiness result is one projection unit with exactly one primary, so an individual later blocker list can contain only additional actions. A singular `next_action` uses `primary`.
- `additional` is a presentation role, not an optionality claim. An additional action can still be required to clear another blocker.
- `allowed_operation_categories` names the owner-supported invocation categories for the action. It does not prove that the current connection can dispatch the action, does not grant user authority, and is empty when no supported API method invocation is identified.
- `expected_state_version` is always present and nullable. For an API mutation action that consumes optimistic concurrency, it contains the current `project_state.state_version` from the projection that produced the action and maps directly to `ToolEnvelope.expected_state_version` for that invocation. It is `null` for read actions, `user_only` actions, actions without a single owner method, and owner-method actions that do not consume optimistic concurrency.
- `expected_state_version` is a retryable concurrency input, not identity or authority. It can become stale after another committed mutation; callers refresh current state after `STATE_VERSION_CONFLICT`. Neither `required_refs` nor any ref's `produced_at_state_version` supplies or overrides this token.
- A `next_actions` entry that uses stale `action` or `reason` fields is not a valid `NextActionSummary`.
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
- `WriteTicketStateSummary.consumed_by_run_ref` is non-null only when the summarized write ticket has been consumed by a recorded Run.
- `WriteTicketStateSummary.observation_refs` lists evidence observation refs created by that consuming Run when those refs are available; it is empty when the write ticket is not consumed or the consuming Run created no observations.
- `WriteTicketAttemptScope` is the one-attempt boundary captured by the write ticket.
- `WriteTicketAttemptScope` is not ordinary write approval, sensitive-action approval, final acceptance, residual-risk acceptance, or broad user approval.
- `WriteTicket` is the ticket-first authority record returned by `volicord.prepare_write` when a committed allowed decision issues a write ticket.
- `WriteTicket.state` is a controlled value string.
- `WriteTicket.path_patterns.allowed` and `WriteTicket.path_patterns.denied` are normalized Product Repository path patterns captured by the ticket decision.
- `WriteTicket.observed_paths` is empty in the baseline. Detective host-hook and watcher observations are recorded through host-observation and unrecorded-change records rather than written back into the ticket.
- `WriteTicket.control_surface` and `WriteTicket.guarantee_display` disclose the current Volicord observation summary and guarantee wording. They do not claim OS-level filesystem enforcement.
- `WriteDecisionReason` is used by `PrepareWriteResult.write_decision_reasons`.

`NextActionSummary` field classifications:

| Field | Classification | Rule |
|---|---|---|
| `presentation_role` | Controlled presentation-role value. | Uses `primary` or `additional` from the [next-action values](schema-value-sets.md#next-action-values). It does not describe optionality. |
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
| `intended_operation` | Free-form intent string. | Describes the intended operation without creating a controlled value set. |
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
| `observed_paths` | Normalized Product Repository path strings. | Lists observed paths only when an owner-defined detective path has connected observations to the ticket. Use `[]` when no observations are connected. |
| `basis_state_version` | State-clock value. | The `project_state.state_version` basis committed with the ticket. |
| `expires_at` | UTC timestamp or `null`. | Ticket expiration used as a Volicord compatibility condition, not as OS-level enforcement. |
| `control_surface` | `ControlSurfaceSummary | null`. | Disclosure of the current Volicord control surface. |
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

```yaml
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

ConnectionObservationSourceSelector:
  source_kind: guard_event | session_watcher
  event_kind: pre_tool | post_tool | prompt_capture | stop  # guard_event only

EvidenceCaptureSpec:
  capture_kind: verified_command_execution | verified_tool_invocation | registered_connection_observation
  command_sha256: string                       # verified_command_execution only
  command_label: string                        # verified_command_execution only; normalized, 1..256 UTF-8 bytes
  expected_exit_code: integer | null           # verified_command_execution only
  tool_name: string                            # verified_tool_invocation only; trimmed, 1..256 UTF-8 bytes
  tool_input_sha256: string                    # verified_tool_invocation only
  expected_success: boolean | null             # verified_tool_invocation only
  source_selector: ConnectionObservationSourceSelector  # registered_connection_observation only
  expected_complete: boolean | null            # registered_connection_observation only

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
  session_id: string | null
  guard_installation_id: string | null
  guard_event_ids: string[]
  watch_observation_refs: string[]
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
  session_id: string | null
  guard_installation_id: string | null
  guard_event_ids: string[]
  watch_observation_refs: string[]
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
  replacement set. A same-Task current ID preserves identity, `null` requests a
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
- `ConnectionObservationSourceSelector` is a strict tagged union. The guard
  branch requires one closed `event_kind`; the session-watcher branch rejects
  that field and has no additional caller-owned coordinate. Unknown, extra,
  and mixed branch fields are invalid. `session_start` is excluded because the
  exact intent-bound session necessarily started before the intent and cannot
  supply a post-intent observation.
- `EvidenceCaptureSpec` is a strict tagged union. Its caller-supplied lowercase
  64-character digest fields bind exact command or tool input. For a registered
  connection observation, Core derives `input_sha256` from canonical
  `source_selector` JSON; future event/observation identity, time, raw-event
  digest, and snapshot digest are not intent fields. Expected-outcome members
  are nullable in the typed shape and use method-owned omission defaults on
  MCP.
- `EvidenceCaptureIntent` is the immutable, expiring current-basis request. Its
  `requested_by_actor_source` and `workspace_context` are Core-derived basis
  fields, not caller-selected attribution. Its public ref uses
  `record_kind=evidence_capture_intent`.
- `EvidenceCaptureReceipt` is an immutable durable source-fulfillment fact record.
  Its associated staging handle and staged receipt bytes are transient.
  Its registered connection/session/guard/watch coordinates, exact source
  identity, observation time and raw-event or snapshot/selection digests,
  outcome, completeness, limitations, redaction state, observer, and times are
  source facts. The receipt is not a `StateRecordRef` and does not advance Core
  state.
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
  links the corresponding target-matching `EvidenceObservation`. A supported
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
  criteria reject `coverage_state=not_applicable`; optional, `not_required`,
  supplemental, and retired targets remain non-authoritative for close.
- Submitted `observed_by_actor_source` does not select the committed actor. Core
  derives it from a validated producer record when present and otherwise from
  the verified invocation; a submitted value cannot raise trust or impersonate
  another actor source.
- Core derives committed `source_kind` and `assurance_level` from verified anchors. An unanchored direct `connection_observation`, `user_observation`, `external_tool`, or caller-declared `reused_evidence` input is committed as `agent_report` / `cooperative_report`. These fields never by themselves prove product correctness, grant user authority, satisfy final acceptance, satisfy residual-risk acceptance, or raise `GuaranteeDisplay.level`.
- A current complete capture receipt consumed through exactly one
  `evidence_capture_intent` input ref can establish an authority-owned verified
  command, verified tool, or registered connection-observation producer. A
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
  control_surface: ControlSurfaceSummary | null
  can_resolve_in_chat: boolean
  outside_chat_action_required: boolean
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
- `result_refs`, `source_run_ref`, `source_refs`, `evidence_summary_ref`, and `accepted_by_user_action_resolution_refs` use `StateRecordRef`.
- `sensitive_categories` are opaque sensitive-category classification strings unless an affected method or profile owner publishes a narrower local list.
- `sensitive_action_requirements` are Core-derived close requirements from committed Runs and consumed write tickets. Category-only caller input cannot establish or erase these requirements.
- `recovery_constraints` and `RiskAcceptanceCoverage.missing_reason` are display strings. Current close-readiness results use `acceptance_required` when required acceptance is absent and may use `stale_acceptance` when a non-current residual-risk acceptance exists but does not cover the current residual-risk `risk_id` values.
- `RiskAcceptanceCoverage` reports whether the current residual-risk requirements are covered by compatible user-action resolutions. It does not report evidence sufficiency or final acceptance.
- `CloseReadinessBlocker` is a data shape for close-readiness findings.
- `CloseReadinessBlocker.category` is a controlled value string.
- `CloseReadinessBlocker.code` is an owner-defined blocker code. It is not an exhaustive global public enum unless the blocker or method owner publishes a narrower local list.
- `CloseReadinessBlocker.control_surface` may be present on `guard_*` connection-capability blockers to report the observation summary at the time the blocker was computed. It is absent for blockers that do not derive from `GuardHealthSummary` hook-state facts.
- `can_resolve_in_chat` reports whether the blocker can be resolved through a chat-mediated user path when the method owner knows that path.
- `outside_chat_action_required` reports whether the owner knows that the next action requires a terminal, host, filesystem, or setup action outside chat.
- `can_resolve_in_chat` and `outside_chat_action_required` are independent disclosures, not logical complements. `false` for both means that neither path claim was established; it does not mean that no action is required.
- `CloseReadinessBlocker.message`, `ValidatorResult.message`, and `GuaranteeDisplay.basis` are free-form display strings.
- `ValidatorResult.validator_id` is a reporting label unless the value-set owner publishes a supported stable value.
- `ValidatorResult.status`, `ValidatorResult.severity`, and `GuaranteeDisplay.level` are controlled value strings.
- `GuaranteeDisclosure` is the result-interpretation disclosure returned by public result bases and diagnostic outputs when a reader might otherwise overinterpret the result.
- `GuaranteeDisclosure.guarantee_class` and `GuaranteeDisclosure.non_guarantees` are controlled value strings. `GuaranteeDisclosure.guarantees` are concise display statements.
- `GuaranteeDisplay` describes the current capability display for a status or compatibility view. It does not replace `GuaranteeDisclosure`.

These shapes do not define close-readiness meaning, response routing, or persistence behavior.

Close-basis reference rules:
- Caller-supplied close-assessment refs accepted into `CurrentCloseBasis.result_refs` or `ResidualRisk.source_refs` are limited to result/evidence record kinds `run`, `artifact`, `evidence_summary`, and `change_unit` unless an owner document explicitly adds another kind.
- `project_state`, `write_ticket`, `user_action_request`,
  `user_action_resolution`, `blocker`, `task_event`, and `task` are not
  caller-supplied result refs for a close basis unless an owner document
  explicitly adds them.
- Every accepted ref must exist, belong to the same project and Task, and be canonicalized by Core. Core never treats caller-supplied `produced_at_state_version` metadata as authority or concurrency input.
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
- Response branch behavior, close-readiness evaluation order, and response-only blocked outcomes: [`volicord.check_close` and `volicord.close_task`](method-close-task.md)
- Close-readiness blocker/API response routing semantics: [API blocker routing](blocker-routing.md)
- Supported `CloseReadinessBlocker.category`, `ValidatorResult.status`, `ValidatorResult.severity`, and `GuaranteeDisplay.level` values: [API Value Sets](schema-value-sets.md#state-and-blocker-values)
- Security guarantee meaning: [Security](../security.md)

## Related owners

- [API Schema Core](schema-core.md) for `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, and `ToolDryRunResponse`.
- [API Value Sets](schema-value-sets.md#state-and-blocker-values) for exact close-readiness blocker category values and neighboring state values.
- [API Methods](methods.md) and method owner documents for the methods that return these schemas.
- [API Artifact Schemas](schema-artifacts.md) for `ArtifactRef`.
- [API User Action Schemas](schema-user-action.md) for durable action requests
  and capture forms.
- [Storage Effects](../storage-effects.md) for persistence and state-effect consequences.
