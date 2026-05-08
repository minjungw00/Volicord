# Reference MVP

## Document Role

This document owns the MVP implementation sequence and reference implementation details: SQLite DDL draft, migrations, lock policy, artifact directory layout, baseline capture format, projection job table, validator runner skeleton, reference surface behavior, and minimal operator tooling plan.

It does not own the public MCP schema source of truth. Implementers must use `05-mcp-api-and-schemas.md` for public request/response contracts.

## MVP Scope

The MVP is a kernel-authority and agency-conformance validation project, not a broad agent-surface integration project.

MVP includes:

- one local project registration
- one reference agent surface
- MCP server exposing the public tools from the API document
- `state.sqlite` current tables plus `state.sqlite.task_events`
- artifact registry and durable artifact files
- baseline capture
- `prepare_write` gate with scope, approval, baseline, capability checks, and durable Write Authorization records
- Journey/Decision skeleton for task continuity, Decision Packets, and user judgment routing
- shaping kernel support for Change Units, autonomy boundaries, dependency metadata, and end-to-end path intent
- approval, evidence, verification, Manual QA, and acceptance gate support
- decision, autonomy boundary, feedback loop, codebase stewardship, residual-risk visibility, and agency conformance checks
- TASK, APR, RUN-SUMMARY, EVIDENCE-MANIFEST, EVAL, DIRECT-RESULT projections
- optional minimal TDD-TRACE and MANUAL-QA projections where policy requires them
- detached verification bundle or manual evaluator instruction bundle
- doctor, recover, reconcile, export, and conformance smoke entrypoints

MVP excludes the later automation cataloged in `appendix/C-later-roadmap.md`, including broader surface expansion, richer capture automation, advanced orchestration, analytics, and team profile export/import.

Parallel orchestration automation remains later. MVP may store Change Unit dependency DAG metadata only when it is needed for serial shaping, write checks, close blockers, or visibility; it does not schedule parallel lanes, isolate concurrent baselines, or reconcile concurrent execution.

## Implementation Sequence

### MVP-0: Runtime Bootstrap

Create the runtime home, register one project, create `project.yaml`, initialize `registry.sqlite`, initialize `state.sqlite`, create artifact directories, and register the reference surface with a cooperative/detective capability profile.

Exit criteria:

- project appears in `registry.sqlite.projects`
- one `project_state` row exists for the registered project before any project-scoped mutation can use `expected_state_version`
- reference surface appears in `registry.sqlite.project_surfaces`
- project runtime directory contains `project.yaml`, `state.sqlite`, and artifact directories
- doctor can report project/runtime readiness

### MVP-1: Core State, Journey/Decision Skeleton, MCP Facade

Implement Core transaction wrapper, locks, state version checks, idempotency replay records, read resources, Journey Spine reconstruction, Decision Packet records, `decision_gate` aggregation, `harness.status`, `harness.intake`, and `harness.next`.

Exit criteria:

- active Task absent status works
- advisor Task can intake, run read-only, and close through Core
- Task status can expose current Journey/Decision state from committed records
- blocking user judgment can create or associate a Decision Packet and update `decision_gate`
- every state mutation updates current records and appends `state.sqlite.task_events` in one transaction

### MVP-2: Shaping Kernel, Write Gate, Approval, Baseline, Artifacts

Implement Change Unit records, Change Unit dependency metadata, gate records, baseline capture, artifact registration, `harness.prepare_write`, Write Authorization records, approval request/decision flow, shaping updates, autonomy boundary fields, and minimal changed-path/scope/approval/baseline/decision/autonomy validators.

Exit criteria:

- product write without active scoped Change Unit is blocked
- sensitive dependency or schema change requires approval
- intended work outside the active Autonomy Boundary is blocked or routed to a Decision Packet
- unresolved or incompatible blocking Decision Packets block affected writes
- allowed `prepare_write` creates a durable Write Authorization ref, while idempotent replay returns the already committed response
- approval scope drift can expire or block approval
- Change Unit shaping records end-to-end path intent, user-judgment requirements, AFK stop conditions, and dependency metadata when needed
- raw artifacts are stored with hash and redaction metadata

### MVP-3: Runs, Evidence, Feedback Loop, Projection, Reconcile

Implement `harness.record_run`, run records, Write Authorization consumption, evidence manifest records, feedback loop checks, codebase stewardship checks, projection jobs, TASK/APR/RUN-SUMMARY/EVIDENCE-MANIFEST/DIRECT-RESULT renderers, managed block hashes, and reconcile item creation for managed drift or human-editable proposals.

Exit criteria:

- implementation and direct runs register artifacts and update evidence
- implementation and direct runs consume a compatible Write Authorization and detect observed changes outside the authorization
- findings from runs, checks, QA inputs, or evaluator notes route back into state, evidence, a Decision Packet, a Change Unit update, or a close blocker
- codebase stewardship issues that affect scope, design, module boundaries, or user judgment are visible as validator results or blockers
- projection job failure is separate from state failure
- managed Markdown edits create reconcile items instead of mutating state

### MVP-4: Verification, Manual QA, Residual Risk, Close

Implement `harness.launch_verify`, `harness.record_eval`, `harness.record_manual_qa`, `harness.close_task`, verification independence checks, Manual QA aggregation, residual-risk visibility checks, decision gate close checks, and close blockers.

Exit criteria:

- work cannot close as `detached_verified` from same-session self-review
- verification waiver closes with `completed_with_risk_accepted`, not `detached_verified`
- required Manual QA and acceptance block close independently
- known close-relevant residual risk is visible before any successful close
- risk-accepted close additionally requires accepted Residual Risk refs
- acceptance, when required, can be recorded only after close-relevant residual risk is visible
- unresolved, stale, incompatible, or deferred-without-coverage blocking Decision Packets block close
- direct work can close self-checked unless policy or user requested detached verification

### MVP-5: Operator Smoke, Agency Conformance, Later-Boundary Checks

Implement minimal doctor, recover, reconcile, export, artifact integrity check, fixture-based conformance smoke, and agency conformance smoke for Journey visibility, explicit product judgment, Autonomy Boundary respect, and residual-risk visibility.

Exit criteria:

- conformance smoke covers no-active-task status, advisor close, direct close, approval-required block, decision-required block, autonomy-boundary block, Write Authorization required and invalid cases, evidence-insufficient close block, same-session verification guard, residual-risk visibility, feedback-loop routing, codebase-stewardship finding visibility, projection failure separation, reconcile required, and MCP-unavailable write hold
- agency conformance checks verify the user can follow the Journey, see unresolved decisions, see what the agent may do without asking, and see close-relevant residual risk before acceptance
- parallel orchestration automation remains later; any MVP dependency DAG support is metadata-only
- export includes state snapshots, report projections, artifact refs, and redaction status

## Runtime Storage

The reference storage uses SQLite for registry and per-project state. The DDL is a draft implementation contract; field names may gain indexes or migration helpers, but table ownership and authority boundaries should remain stable.

`task_spine_entries` is the physical MVP table for public `journey_spine_entry` records and Journey Spine Entry wording. Public MCP/API naming remains `journey_spine_entry`; the table name preserves the task-local implementation shape.

### `project.yaml`

`project.yaml` stores static project configuration only. It must not store current Task state.

```yaml
project_id: PRJ-0001
display_name: my-app
repo_root: /abs/path/to/my-app
default_agent_surface: reference

agent_surfaces:
  reference:
    enabled: true
    capability_profile_id: SURF-PROFILE-0001

default_checks:
  lint: []
  test: []
  build: []

design_quality:
  vertical_slice_default: true
  tdd_required_for: []
  manual_qa_default_for: []

network_policy:
  default_write: deny
  allowed_read_domains: []
  allowed_write_targets: []

secret_policy:
  env_allowlist: []
  allow_secret_access_without_approval: false
```

### `registry.sqlite`

```sql
CREATE TABLE projects (
  project_id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  repo_root TEXT NOT NULL,
  repo_fingerprint TEXT NOT NULL,
  runtime_path TEXT NOT NULL,
  project_yaml_path TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE project_surfaces (
  surface_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(project_id),
  surface_kind TEXT NOT NULL,
  display_name TEXT NOT NULL,
  capability_profile_id TEXT NOT NULL,
  guarantee_level TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  mcp_config_ref TEXT,
  last_seen_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE connector_manifests (
  manifest_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(project_id),
  surface_id TEXT NOT NULL REFERENCES project_surfaces(surface_id),
  manifest_version INTEGER NOT NULL,
  generated_paths_json TEXT NOT NULL,
  managed_hash TEXT NOT NULL,
  capability_profile_json TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

### `state.sqlite`

```sql
CREATE TABLE project_state (
  project_id TEXT PRIMARY KEY,
  state_version INTEGER NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE tasks (
  task_id TEXT PRIMARY KEY,
  state_version INTEGER NOT NULL,
  mode TEXT NOT NULL,
  lifecycle_phase TEXT NOT NULL,
  result TEXT NOT NULL,
  close_reason TEXT NOT NULL,
  assurance_level TEXT NOT NULL,
  title TEXT NOT NULL,
  current_summary TEXT NOT NULL DEFAULT '',
  acceptance_criteria_json TEXT NOT NULL DEFAULT '[]',
  active_change_unit_id TEXT,
  active_run_id TEXT,
  latest_evidence_manifest_id TEXT,
  latest_eval_id TEXT,
  latest_manual_qa_record_id TEXT,
  projection_version INTEGER NOT NULL DEFAULT 0,
  projected_version INTEGER NOT NULL DEFAULT 0,
  projection_status TEXT NOT NULL DEFAULT 'unknown',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE task_gates (
  task_id TEXT PRIMARY KEY REFERENCES tasks(task_id),
  scope_gate TEXT NOT NULL,
  decision_gate TEXT NOT NULL,
  approval_gate TEXT NOT NULL,
  design_gate TEXT NOT NULL,
  evidence_gate TEXT NOT NULL,
  verification_gate TEXT NOT NULL,
  qa_gate TEXT NOT NULL,
  acceptance_gate TEXT NOT NULL,
  waiver_json TEXT NOT NULL DEFAULT '{}',
  updated_at TEXT NOT NULL
);

CREATE TABLE change_units (
  change_unit_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  title TEXT NOT NULL,
  purpose TEXT NOT NULL,
  non_goals_json TEXT NOT NULL DEFAULT '[]',
  slice_type TEXT NOT NULL,
  autonomy_profile TEXT NOT NULL,
  agent_may_do_json TEXT NOT NULL DEFAULT '[]',
  user_judgment_required_json TEXT NOT NULL DEFAULT '[]',
  afk_stop_conditions_json TEXT NOT NULL DEFAULT '[]',
  end_to_end_path_json TEXT NOT NULL DEFAULT '{}',
  horizontal_exception_reason TEXT,
  follow_up_vertical_change_unit_id TEXT,
  allowed_paths_json TEXT NOT NULL DEFAULT '[]',
  allowed_tools_json TEXT NOT NULL DEFAULT '[]',
  allowed_commands_json TEXT NOT NULL DEFAULT '[]',
  allowed_network_json TEXT NOT NULL DEFAULT '[]',
  secret_scope_json TEXT NOT NULL DEFAULT '[]',
  sensitive_categories_json TEXT NOT NULL DEFAULT '[]',
  validator_profile_json TEXT NOT NULL DEFAULT '[]',
  completion_conditions_json TEXT NOT NULL DEFAULT '[]',
  evaluator_focus_json TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE baselines (
  baseline_ref TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT,
  repo_head TEXT NOT NULL,
  branch TEXT NOT NULL,
  dirty INTEGER NOT NULL,
  tree_hash TEXT NOT NULL,
  included_paths_json TEXT NOT NULL DEFAULT '[]',
  ignored_paths_json TEXT NOT NULL DEFAULT '[]',
  diff_artifact_id TEXT REFERENCES artifacts(artifact_id),
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE write_authorizations (
  write_authorization_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT NOT NULL REFERENCES change_units(change_unit_id),
  baseline_ref TEXT REFERENCES baselines(baseline_ref),
  intended_operation TEXT NOT NULL,
  intended_paths_json TEXT NOT NULL DEFAULT '[]',
  intended_tools_json TEXT NOT NULL DEFAULT '[]',
  intended_commands_json TEXT NOT NULL DEFAULT '[]',
  intended_network_json TEXT NOT NULL DEFAULT '[]',
  intended_secrets_json TEXT NOT NULL DEFAULT '[]',
  sensitive_categories_json TEXT NOT NULL DEFAULT '[]',
  approval_refs_json TEXT NOT NULL DEFAULT '[]',
  decision_packet_refs_json TEXT NOT NULL DEFAULT '[]',
  guarantee_level TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  expires_at TEXT,
  consumed_by_run_id TEXT,
  consumed_at TEXT
);

CREATE TABLE runs (
  run_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT,
  kind TEXT NOT NULL,
  actor_kind TEXT NOT NULL,
  surface_id TEXT NOT NULL,
  baseline_ref TEXT,
  write_authorization_id TEXT REFERENCES write_authorizations(write_authorization_id),
  summary TEXT NOT NULL DEFAULT '',
  observed_changes_json TEXT NOT NULL DEFAULT '{}',
  command_results_json TEXT NOT NULL DEFAULT '[]',
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL,
  started_at TEXT NOT NULL,
  completed_at TEXT
);

CREATE TABLE approvals (
  approval_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT,
  decision_request_id TEXT,
  decision_packet_id TEXT REFERENCES decision_packets(decision_packet_id),
  status TEXT NOT NULL,
  sensitive_categories_json TEXT NOT NULL DEFAULT '[]',
  allowed_paths_json TEXT NOT NULL DEFAULT '[]',
  allowed_tools_json TEXT NOT NULL DEFAULT '[]',
  allowed_commands_json TEXT NOT NULL DEFAULT '[]',
  allowed_network_targets_json TEXT NOT NULL DEFAULT '[]',
  secret_scope_json TEXT NOT NULL DEFAULT '[]',
  baseline_ref TEXT,
  expires_at TEXT,
  decision_note TEXT,
  created_at TEXT NOT NULL,
  decided_at TEXT
);

CREATE TABLE decision_requests (
  decision_request_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT,
  decision_kind TEXT NOT NULL,
  status TEXT NOT NULL,
  prompt TEXT NOT NULL,
  options_json TEXT NOT NULL DEFAULT '[]',
  recommendation TEXT,
  approval_scope_json TEXT NOT NULL DEFAULT '{}',
  reconcile_item_id TEXT,
  expires_at TEXT,
  decided_option_id TEXT,
  decision_json TEXT NOT NULL DEFAULT '{}',
  note TEXT,
  waiver_reason TEXT,
  created_at TEXT NOT NULL,
  decided_at TEXT
);

CREATE TABLE decision_packets (
  decision_packet_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT,
  decision_request_id TEXT,
  decision_kind TEXT NOT NULL,
  status TEXT NOT NULL,
  question TEXT NOT NULL,
  options_json TEXT NOT NULL DEFAULT '[]',
  recommendation_json TEXT NOT NULL DEFAULT '{}',
  affected_scope_json TEXT NOT NULL DEFAULT '{}',
  autonomy_boundary_json TEXT NOT NULL DEFAULT '{}',
  context_refs_json TEXT NOT NULL DEFAULT '[]',
  context_artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  residual_risk_refs_json TEXT NOT NULL DEFAULT '[]',
  decision_json TEXT NOT NULL DEFAULT '{}',
  superseded_by_decision_packet_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  decided_at TEXT
);

CREATE TABLE residual_risks (
  residual_risk_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT,
  source_record_kind TEXT NOT NULL,
  source_record_id TEXT NOT NULL,
  related_decision_packet_id TEXT REFERENCES decision_packets(decision_packet_id),
  affected_scope_json TEXT NOT NULL DEFAULT '{}',
  affected_acceptance_criteria_json TEXT NOT NULL DEFAULT '[]',
  visibility_status TEXT NOT NULL,
  accepted_risk_json TEXT NOT NULL DEFAULT '{}',
  follow_up_requirement_json TEXT NOT NULL DEFAULT '{}',
  close_impact TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  accepted_at TEXT
);

CREATE TABLE shared_designs (
  shared_design_id TEXT PRIMARY KEY,
  task_id TEXT REFERENCES tasks(task_id),
  change_unit_id TEXT,
  first_change_unit_id TEXT REFERENCES change_units(change_unit_id),
  title TEXT NOT NULL,
  design_kind TEXT NOT NULL,
  goal TEXT NOT NULL,
  non_goals_json TEXT NOT NULL DEFAULT '[]',
  acceptance_criteria_json TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL,
  scope_json TEXT NOT NULL DEFAULT '{}',
  assumptions_json TEXT NOT NULL DEFAULT '[]',
  resolved_questions_json TEXT NOT NULL DEFAULT '[]',
  domain_impact_refs_json TEXT NOT NULL DEFAULT '[]',
  module_impact_refs_json TEXT NOT NULL DEFAULT '[]',
  interface_impact_refs_json TEXT NOT NULL DEFAULT '[]',
  options_json TEXT NOT NULL DEFAULT '[]',
  selected_option_json TEXT NOT NULL DEFAULT '{}',
  rejected_options_json TEXT NOT NULL DEFAULT '[]',
  decision_packet_refs_json TEXT NOT NULL DEFAULT '[]',
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE task_spine_entries (
  task_spine_entry_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT,
  sequence_no INTEGER NOT NULL,
  entry_kind TEXT NOT NULL,
  lifecycle_phase TEXT,
  actor_kind TEXT NOT NULL,
  source_record_kind TEXT,
  source_record_id TEXT,
  summary TEXT NOT NULL DEFAULT '',
  refs_json TEXT NOT NULL DEFAULT '[]',
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(task_id, sequence_no)
);

CREATE TABLE change_unit_dependencies (
  change_unit_dependency_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT NOT NULL REFERENCES change_units(change_unit_id),
  depends_on_change_unit_id TEXT NOT NULL REFERENCES change_units(change_unit_id),
  dependency_kind TEXT NOT NULL,
  status TEXT NOT NULL,
  merge_risk TEXT NOT NULL,
  visibility_note TEXT NOT NULL DEFAULT '',
  close_impact TEXT NOT NULL,
  rationale TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE evidence_manifests (
  evidence_manifest_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT,
  baseline_ref TEXT,
  criteria_json TEXT NOT NULL DEFAULT '[]',
  changed_files_json TEXT NOT NULL DEFAULT '[]',
  supporting_refs_json TEXT NOT NULL DEFAULT '[]',
  stale_if_json TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE evals (
  eval_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT,
  evaluator_run_id TEXT,
  target_run_id TEXT,
  verdict TEXT NOT NULL,
  checks_json TEXT NOT NULL DEFAULT '[]',
  evidence_reviewed_json TEXT NOT NULL DEFAULT '[]',
  independence_json TEXT NOT NULL DEFAULT '{}',
  blockers_json TEXT NOT NULL DEFAULT '[]',
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL
);

CREATE TABLE manual_qa_records (
  manual_qa_record_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT,
  qa_profile TEXT NOT NULL,
  performed_by TEXT NOT NULL,
  result TEXT NOT NULL,
  findings_json TEXT NOT NULL DEFAULT '[]',
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  waiver_reason TEXT,
  waiver_decision_packet_id TEXT REFERENCES decision_packets(decision_packet_id),
  residual_risk_refs_json TEXT NOT NULL DEFAULT '[]',
  next_action TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE artifacts (
  artifact_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  run_id TEXT,
  kind TEXT NOT NULL,
  relative_path TEXT NOT NULL,
  sha256 TEXT NOT NULL,
  size_bytes INTEGER NOT NULL,
  content_type TEXT NOT NULL,
  redaction_state TEXT NOT NULL,
  produced_by TEXT NOT NULL,
  retention_class TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE artifact_links (
  artifact_link_id TEXT PRIMARY KEY,
  artifact_id TEXT NOT NULL REFERENCES artifacts(artifact_id),
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  record_kind TEXT NOT NULL,
  record_id TEXT NOT NULL,
  relation_kind TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE task_events (
  event_id TEXT PRIMARY KEY,
  task_id TEXT,
  state_version INTEGER NOT NULL,
  event_type TEXT NOT NULL,
  actor_kind TEXT NOT NULL,
  surface_id TEXT,
  request_id TEXT,
  idempotency_key TEXT,
  payload_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL
);

CREATE TABLE tool_invocations (
  invocation_id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  task_id TEXT,
  tool_name TEXT NOT NULL,
  request_id TEXT NOT NULL,
  idempotency_key TEXT NOT NULL,
  request_hash TEXT NOT NULL,
  response_json TEXT NOT NULL DEFAULT '{}',
  state_version INTEGER NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  completed_at TEXT,
  UNIQUE(project_id, tool_name, idempotency_key)
);

CREATE TABLE projection_jobs (
  projection_job_id TEXT PRIMARY KEY,
  task_id TEXT,
  projection_kind TEXT NOT NULL,
  target_ref TEXT NOT NULL,
  projection_version INTEGER NOT NULL,
  status TEXT NOT NULL,
  attempts INTEGER NOT NULL DEFAULT 0,
  output_path TEXT,
  managed_hash TEXT,
  error_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE reconcile_items (
  reconcile_item_id TEXT PRIMARY KEY,
  task_id TEXT,
  source_kind TEXT NOT NULL,
  source_path TEXT,
  source_hash TEXT,
  target_record_kind TEXT,
  target_record_id TEXT,
  proposed_change_json TEXT NOT NULL DEFAULT '{}',
  status TEXT NOT NULL,
  decision_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  resolved_at TEXT
);

CREATE TABLE domain_terms (
  domain_term_id TEXT PRIMARY KEY,
  term TEXT NOT NULL,
  meaning TEXT NOT NULL,
  code_representation TEXT,
  not_this_json TEXT NOT NULL DEFAULT '[]',
  related_terms_json TEXT NOT NULL DEFAULT '[]',
  source_ref TEXT,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE module_map_items (
  module_map_item_id TEXT PRIMARY KEY,
  module_path TEXT NOT NULL,
  responsibility TEXT NOT NULL,
  public_interface_json TEXT NOT NULL DEFAULT '[]',
  dependencies_json TEXT NOT NULL DEFAULT '[]',
  test_boundary TEXT,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE interface_contracts (
  interface_contract_id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  owner_module TEXT NOT NULL,
  change_type TEXT NOT NULL,
  inputs_json TEXT NOT NULL DEFAULT '[]',
  outputs_json TEXT NOT NULL DEFAULT '[]',
  errors_json TEXT NOT NULL DEFAULT '[]',
  compatibility_impact TEXT NOT NULL,
  callers_impacted_json TEXT NOT NULL DEFAULT '[]',
  boundary_tests_json TEXT NOT NULL DEFAULT '[]',
  review_status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE tdd_traces (
  tdd_trace_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(task_id),
  change_unit_id TEXT,
  status TEXT NOT NULL,
  red_refs_json TEXT NOT NULL DEFAULT '[]',
  green_refs_json TEXT NOT NULL DEFAULT '[]',
  refactor_refs_json TEXT NOT NULL DEFAULT '[]',
  non_tdd_justification TEXT,
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE validator_runs (
  validator_run_id TEXT PRIMARY KEY,
  task_id TEXT,
  change_unit_id TEXT,
  run_id TEXT,
  validator_id TEXT NOT NULL,
  validator_kind TEXT NOT NULL,
  status TEXT NOT NULL,
  guarantee_level TEXT NOT NULL,
  findings_json TEXT NOT NULL DEFAULT '[]',
  blocked_reasons_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL
);

CREATE TABLE locks (
  lock_id TEXT PRIMARY KEY,
  scope TEXT NOT NULL,
  owner TEXT NOT NULL,
  acquired_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  heartbeat_at TEXT NOT NULL
);
```

`project_state.state_version` is the project-scoped state clock. Core initializes exactly one `project_state` row for the registered project during runtime bootstrap, before any project-scoped mutation can compare `expected_state_version` with `project_state.state_version`.

`tasks.state_version` is the task-scoped state clock. Task-scoped mutations compare `expected_state_version` with the Core-resolved primary Task's `tasks.state_version`; project-scoped mutations with no resolved primary Task compare it with `project_state.state_version`.

`task_events` remains append-only event history inside `state.sqlite`; MVP does not introduce a separate event store. `task_events.state_version` records the resulting version for the affected scope. For task events this is `tasks.state_version`; for project-level events with `task_id=null` this is `project_state.state_version`.

`tool_invocations` stores request replay metadata needed to return the original committed response. `tool_invocations.state_version` stores the same primary affected-scope version returned in `ToolResponseBase.state_version`: Task State Version when Core resolves a primary Task, otherwise Project State Version. Reusing an idempotency key with a different `request_hash` returns `STATE_CONFLICT`.

`tasks.projection_status` is the TASK projection status summary. Per-kind projection freshness is tracked through `projection_jobs` and through the relevant projection records or artifact refs for APR, RUN-SUMMARY, EVIDENCE-MANIFEST, EVAL, DIRECT-RESULT, optional MANUAL-QA, optional TDD-TRACE, and other enabled projection kinds. Do not treat one Task field as owning all projection freshness.

`write_authorizations` stores durable allow decisions from `prepare_write`. When `dry_run=false` and `prepare_write` returns `allowed`, Core creates a distinct `write_authorizations` row for a distinct compatible request and returns its ref. `authorization_effect=returned` is reserved for idempotent replay of the same committed `prepare_write` request and response with the same idempotency key, request hash, and state basis. A distinct compatible request creates a distinct Write Authorization; compatibility does not make authorizations reusable. Core may stale, expire, or revoke older unconsumed authorizations if their compatibility basis changes. `updated_at` changes whenever authorization status changes; status history remains in `task_events`.

Implementation and direct `record_run` calls consume a compatible unexpired authorization by recording `runs.write_authorization_id` and marking the authorization consumed with `consumed_by_run_id` and `consumed_at`. The reciprocal links `write_authorizations.consumed_by_run_id` and `runs.write_authorization_id` must point to each other in the same Core transaction. A mismatch is invalid state; `recover` must repair it or block affected close. Consuming an authorization does not make observed changes valid by itself; changed-path, tool, command, network, secret, Change Unit, approval, baseline, and Decision Packet validation still verifies the committed Run.

`runs.write_authorization_id` is populated only when a Run successfully consumes a compatible Write Authorization. A violation or audit Run that attempted to use an invalid, stale, missing, consumed, or scope-exceeded authorization must not populate `runs.write_authorization_id`; store the attempted authorization ref in validator findings, run violation payload, or `task_events.payload_json` when useful. Such a Run may be recorded for audit or recovery if an observed product write already happened, but it must not satisfy evidence sufficiency, detached verification, QA, acceptance, or close readiness. The corresponding Write Authorization remains unconsumed and may be marked stale, revoked, or expired according to the violation and compatibility basis.

Write Authorizations are single-use for storage. The unique partial index on `runs.write_authorization_id` prevents more than one committed Run row from consuming the same authorization. Idempotent replay returns the original Run and response metadata; it does not insert a second Run row.

`decision_packets` is the canonical state table for blocking product judgment and the authority path for `decision_gate`. `decision_requests` is only an interaction/routing compatibility table for implementation handoff, replay, or legacy request flow. A `decision_request` alone never satisfies `decision_gate`, and `decision_gate` is never recomputed from `decision_requests` alone. Only compatible `decision_packets` plus currently detected blockers feed the `decision_gate` authority path. Core must create or associate a compatible Decision Packet when blocking product judgment exists. Approval decisions link to Decision Packets through `approvals.decision_packet_id`; `decision_request_id` may remain as routing metadata but is not the approval authority path.

`residual_risks` is the canonical table for close-relevant remaining uncertainty, accepted risk, follow-up requirements, and close impact. Decision Packets may reference residual risks through `decision_packets.residual_risk_refs_json`; they must not bury the only canonical residual-risk payload inside the Decision Packet.

`artifact_links` is the queryable many-to-many attachment table for artifacts. Use it to attach artifacts to `run`, `decision_packet`, `shared_design`, `residual_risk`, `evidence_manifest`, `tdd_trace`, `manual_qa_record`, `eval`, and `export` records. Existing `artifact_refs_json` fields may preserve ordered or record-local context, but multi-record artifact reuse and artifact integrity checks should use `artifact_links`.

Manual QA waiver rule: a QA waiver with product/user risk requires a compatible `qa_waiver` Decision Packet linked by `manual_qa_records.waiver_decision_packet_id` and any close-relevant risk refs in `manual_qa_records.residual_risk_refs_json`; otherwise the waiver is blocked.

`change_unit_dependencies` is MVP DAG metadata for shaping, ordering, and close visibility. It is not a parallel orchestration scheduler and does not authorize multiple active implementation lanes.

`baselines` stores BaselineCapture records in state with repo head, branch, dirty flag, tree hash, included/ignored paths, optional diff artifact, and status. `baseline_ref` fields in other tables refer to `baselines.baseline_ref`.

Recommended indexes:

```sql
CREATE INDEX idx_task_events_task_version ON task_events(task_id, state_version);
CREATE INDEX idx_decision_requests_task_status ON decision_requests(task_id, status);
CREATE INDEX idx_decision_packets_task_status ON decision_packets(task_id, status);
CREATE INDEX idx_residual_risks_task_status ON residual_risks(task_id, status);
CREATE INDEX idx_shared_designs_task_status ON shared_designs(task_id, status);
CREATE INDEX idx_task_spine_entries_task_seq ON task_spine_entries(task_id, sequence_no);
CREATE INDEX idx_change_unit_dependencies_task ON change_unit_dependencies(task_id, change_unit_id);
CREATE INDEX idx_baselines_task_change_unit ON baselines(task_id, change_unit_id);
CREATE INDEX idx_write_authorizations_task_status ON write_authorizations(task_id, status);
CREATE INDEX idx_write_authorizations_change_unit ON write_authorizations(change_unit_id);
CREATE INDEX idx_approvals_decision_packet ON approvals(decision_packet_id);
CREATE INDEX idx_projection_jobs_status ON projection_jobs(status, projection_version);
CREATE INDEX idx_artifacts_task_run ON artifacts(task_id, run_id);
CREATE INDEX idx_artifact_links_artifact ON artifact_links(artifact_id);
CREATE INDEX idx_artifact_links_record ON artifact_links(record_kind, record_id);
CREATE INDEX idx_runs_task_status ON runs(task_id, status);
CREATE INDEX idx_runs_write_authorization ON runs(write_authorization_id);
CREATE UNIQUE INDEX uq_runs_write_authorization_consumed
ON runs(write_authorization_id)
WHERE write_authorization_id IS NOT NULL;
CREATE INDEX idx_evals_task_change_unit ON evals(task_id, change_unit_id);
CREATE INDEX idx_manual_qa_records_task_change_unit ON manual_qa_records(task_id, change_unit_id);
CREATE INDEX idx_reconcile_items_status ON reconcile_items(status);
```

`task_events` is append-only by application policy. Recovery may append compensating events; it should not rewrite historical rows.

Reference MVP Write Authorization event vocabulary:

```text
write_authorization_created
write_authorization_returned
write_authorization_consumed
write_authorization_expired
write_authorization_staled
write_authorization_revoked
write_authorization_violation_detected
```

`scope_violation_detected` may be appended when a Run observes a general scope violation; it is not part of the Write Authorization lifecycle vocabulary.

## Migration And Versioning

MVP uses integer schema versions recorded in a small internal migration ledger:

```sql
CREATE TABLE schema_migrations (
  database_name TEXT NOT NULL,
  version INTEGER NOT NULL,
  applied_at TEXT NOT NULL,
  checksum TEXT NOT NULL,
  PRIMARY KEY (database_name, version)
);
```

Migrations must be forward-only for MVP. A failed migration leaves the project unavailable until doctor/recover reports whether the failure is repairable.

## Lock Policy

State-changing operations acquire a lock at the narrowest practical scope:

| Operation | Lock scope |
|---|---|
| project registration | project |
| task intake/close | task |
| shaping update | task and affected Change Unit |
| decision packet create/resolve | task and affected Decision Packet |
| residual risk create/update/accept | task and affected residual risk |
| baseline capture | task and affected Change Unit |
| prepare_write | task and active Change Unit; write authorization when allowed |
| record_run | task and run; write authorization when one is consumed |
| projection render | projection job |
| artifact registration | artifact path |
| artifact link registration | artifact and target record |
| reconcile decision | reconcile item and affected task/design record |

If a lock is expired, the next operation may take it after appending a recovery event. If `expected_state_version` is stale for the relevant task or project scope, the operation returns `STATE_CONFLICT` before mutation.

## Artifact Directory Layout

Reference layout:

```text
~/.harness/
  registry.sqlite
  projects/
    PRJ-0001/
      project.yaml
      state.sqlite
      artifacts/
        bundles/
        diffs/
        logs/
        screenshots/
        checkpoints/
        manifests/
        qa/
        tdd/
        designs/
        prototypes/
        architecture/
        decisions/
        exports/
        tmp/
```

Artifact filenames should include enough stable identity to avoid collisions:

```text
{task_id}/{run_id-or-record_id}/{artifact_id}-{kind}.{ext}
```

Markdown reports in the Product Repository are not raw artifacts by default. If an export needs a report snapshot, it can store that snapshot as an export component artifact while preserving the distinction between the report projection and raw evidence.

### Artifact Kind Storage Notes

The `artifacts.kind` field names durable evidence files. It does not make the artifact file the owner of the corresponding state record.

| Artifact kind | Reference storage note |
|---|---|
| `design_probe` | Store exploratory design findings, sketches, or probe outputs under `artifacts/designs/`; accepted structure belongs in `shared_designs`, design support records, or Task/Change Unit state. |
| `prototype` | Store prototype diffs, screenshots, logs, or throwaway proof artifacts under `artifacts/prototypes/`; product code remains in the Product Repository and committed harness meaning remains in state records. |
| `architecture_scan` | Store module scans, dependency snapshots, boundary findings, or stewardship evidence under `artifacts/architecture/`; accepted module/interface facts remain in their owner records. |
| `decision_context` | Store compact context bundles for user judgment under `artifacts/decisions/`; Decision Packet status and outcome remain in `state.sqlite`. |

### Artifact Registration Contract

Artifact registration is part of the Core transition that records the producing Run, Decision Packet context, Shared Design, Journey Spine Entry, Eval, Manual QA record, verification bundle, or export component.

MVP registration steps:

1. Accept a connector-captured or operator-supplied file only from a staging path under the project artifact `tmp/` directory or from an approved capture adapter.
2. Apply redaction or omission before hashing. Raw secrets must not be copied into durable artifact storage.
3. Move or copy the stored bytes into the artifact directory using `{task_id}/{run_id-or-record_id}/{artifact_id}-{kind}.{ext}` under the matching kind directory.
4. Compute `sha256`, `size_bytes`, `content_type`, and `redaction_state` from the stored bytes.
5. Insert the `artifacts` row and required `artifact_links` rows in the same Core transaction that records the related state record and appends `task_events`.
6. Return an `ArtifactRef` whose `uri` resolves through the artifact registry row.
7. If the file move succeeds but the transaction fails, leave the file in `tmp/` or mark it orphaned for `recover`; do not create a committed artifact ref or artifact link.

`redaction_state` implementation:

| State | Stored artifact bytes |
|---|---|
| `none` | original non-sensitive evidence |
| `redacted` | redacted evidence; the unredacted original is not retained by the harness |
| `secret_omitted` | evidence with secret values omitted or replaced by handles |
| `blocked` | a small metadata-only notice artifact explaining that capture was blocked; no forbidden content is stored |

Artifact integrity failures return `ARTIFACT_MISSING` or a validator failure and mark related evidence or projection freshness stale according to the kernel rules.

## Baseline Capture

Baseline capture records the repository state used by write, approval, evidence, and verification checks.

MVP stores each capture in `baselines`; `baseline_ref` is the primary key used by Runs, approvals, evidence manifests, verification bundles, and validators. If a dirty diff is captured, `baselines.diff_artifact_id` points to the registered diff artifact and an `artifact_links` row attaches it to the baseline context.

```yaml
BaselineCapture:
  baseline_ref: BASE-0001
  project_id: PRJ-0001
  task_id: TASK-0001
  change_unit_id: CU-0001
  repo_root: /abs/path/to/repo
  vcs:
    kind: git
    head: string
    branch: string
    dirty: boolean
    diff_artifact_ref: ArtifactRef | null
  file_snapshot:
    included_paths: string[]
    ignored_paths: string[]
    tree_hash: string
  approval_scope_refs: string[]
  captured_at: string
```

Baseline is stale when relevant HEAD, dirty diff, allowed path contents, approval scope, or verification bundle inputs no longer match the captured baseline. Stale baseline can mark approval, evidence, or verification stale depending on the affected records.

## Verification Bundle Shape

`harness.launch_verify` creates a bundle artifact for detached verification or manual evaluator handoff. The bundle is raw evidence metadata, not an Eval verdict.

Minimum bundle contents:

```text
verify-bundle/
  manifest.json
  task-summary.json
  change-unit.json
  baseline.json
  evidence-manifest.json
  approvals.json
  decision-packets.json
  residual-risks.json
  run-refs.json
  artifact-refs.json
  artifact-links.json
  design-refs.json
  journey-spine-entries.json
  evaluator-instructions.md
```

The manifest records task id, Change Unit id, baseline ref, source state version, included artifact ids, redaction summary, evaluator focus, and the expected independence context. The bundle may include copied raw artifacts when retention and redaction policy allow it; otherwise it includes artifact refs that the evaluator can resolve through the harness.

Launching verification sets or keeps `verification_gate=pending`. Only `harness.record_eval` can record the verdict and update assurance.

## Projection Jobs

Projection jobs are the durable outbox between committed state and Product Repository Markdown files. The `projection_jobs` table above owns job persistence.

For MVP, Decision Packet visibility is rendered through `TASK` projections, status/next responses, judgment-context resources, and decision-packet read resources. A standalone `DEC` projection is optional unless the standalone Decision Packet projection feature is enabled. This document does not define DEC template text.

MVP job lifecycle:

```text
pending -> running -> completed
pending -> running -> failed -> pending
pending -> skipped
```

Rules:

- never render an older projection version over a newer one
- preserve human-editable sections
- compare managed hash before overwrite
- create a reconcile item for managed drift
- keep projection failure separate from Task result

### Projection Worker Execution

The reference projector executes pending jobs after the Core transaction commits.

MVP worker steps:

1. Select the oldest `pending` job for the target projection and acquire the projection-job lock.
2. Mark the job `running` and read the latest state records, artifact refs, and previous managed hash.
3. If the job's `projection_version` is older than the target's current projection version, mark it `skipped`.
4. Render the managed block from committed records and artifact refs.
5. If the existing managed block hash differs from the last recorded hash, create or update a `reconcile_items` row, mark the job `skipped`, and set the projection status to `stale`.
6. Preserve human-editable sections and write the projection through a temporary file plus atomic rename.
7. Record the new managed hash, output path, projected version, and `completed` status.
8. On render or write failure, mark the job `failed`, keep state result unchanged, and surface projection freshness as `failed` or `stale`.

Projection refresh retries `failed` jobs by creating or resetting a `pending` job with a newer attempt count. It must not overwrite a projection whose managed block has drifted until reconcile resolves the drift.

## Reference Surface Behavior

The reference surface is the single MVP agent integration target. It demonstrates the kernel without claiming broad surface support.

Required reference behavior:

- can read repository rules and harness instructions
- can call MCP tools and resources
- calls `harness.intake` before tracked work
- calls `harness.prepare_write` before product writes
- records runs through `harness.record_run`
- uses artifact refs for diffs/logs/bundles
- requests approval/scope/user decisions through MCP
- treats Decision Packets as the state path for blocking product judgment
- respects the active Change Unit Autonomy Boundary and AFK stop conditions
- launches or prepares verification through `harness.launch_verify`
- does not claim detached verification from same-session self-review
- keeps feedback findings and close-relevant residual risk visible through state-backed records or validator results
- holds product writes when MCP is unavailable

Default guarantee display is cooperative/detective. Preventive or isolated claims require an implemented guard or isolation path and a passing capability precondition.

## Validator Runner Skeleton

MVP validators use one shared result shape from the API document. The runner is intentionally small:

```text
run_validators(context, validator_ids):
  results = []
  for validator_id in validator_ids:
    load validator definition
    read only the state/artifact/repo inputs declared by the validator
    execute validator
    normalize output to ValidatorResult
    persist result in validator_runs
    results.append(result)
  return results
```

Stable MVP validator IDs:

| Validator | Purpose |
|---|---|
| `decision_gate_check` | blocking Decision Packets are present, compatible, and resolved or validly deferred for the requested operation |
| `decision_quality_check` | Decision Packets include enough context, options, recommendation status, trade-offs, and affected-scope refs for user judgment |
| `autonomy_boundary_check` | intended work stays inside the active Change Unit Autonomy Boundary or routes to user judgment |
| `feedback_loop_check` | test, eval, QA, or operational findings have an explicit state route to rework, decision, risk, evidence, or close |
| `tdd_trace_required` | required TDD evidence or allowed waiver exists |
| `codebase_stewardship_check` | codebase health, module boundary, dependency, or maintainability concerns that need judgment are visible before write or close |
| `residual_risk_visibility_check` | close-relevant residual risks are recorded and visible before acceptance or risk-accepted close |
| `shared_design_alignment` | active Change Unit and runs align with the Shared Design contract or record a compatible decision |
| `vertical_slice_shape` | required vertical slice or exception is recorded |
| `domain_language_consistency` | domain-language terms that affect the change are consistent or routed to design judgment |
| `module_interface_review` | module/interface review requirement is met |
| `manual_qa_required` | required QA is passed or validly waived |
| `context_hygiene_check` | required context, projection freshness, managed hashes, and user-visible summaries are consistent enough for the requested operation |

Core precondition checks such as active Task, active Change Unit, changed paths, approval scope, baseline freshness, artifact integrity, evidence sufficiency, verification independence, same-session verification guard, projection freshness, and surface capability may still run before or beside these validators. They should not be emitted as alternate design/agency validator IDs in MVP conformance.

Compatibility aliases:

| Older ID | Stable ID |
|---|---|
| `tdd_trace` | `tdd_trace_required` |
| `module_boundary_review` | `module_interface_review` |
| `docs_consistency` | `context_hygiene_check` |
| `projection_freshness` | `context_hygiene_check` |

These aliases are old compatibility inputs only; MVP conformance must emit the stable IDs above.

### Evidence and Verification Profile Implementation Notes

The evidence sufficiency precondition reads only committed records and registered artifacts. Inputs are: Task, `task_gates`, Change Units, Decision Packets, Residual Risks, Shared Designs, Journey Spine Entries, Runs, approvals, Evidence Manifests, Evals, Manual QA records, artifacts, artifact links, and the relevant baseline ref. It computes whether the applicable Evidence Profile is absent, partial, sufficient, stale, or blocked, then updates or blocks through Core according to the kernel rules.

The verification independence precondition reads `evals.independence_json`, `evaluator_run_id`, `target_run_id`, evaluator and target `surface_id`, `baseline_ref`, bundle artifact refs, and `actor_kind`. It confirms whether the Eval profile is `same_session`, `subagent_context`, `fresh_session`, `fresh_worktree`, `sandbox`, or `manual_bundle`, and whether that profile can support detached assurance for the target close path.

No additional evidence/verification profile DDL is required beyond the MVP tables above. Existing JSON fields hold profile metadata: `change_units.autonomy_profile`, `change_units.agent_may_do_json`, `change_units.user_judgment_required_json`, `change_units.afk_stop_conditions_json`, `change_units.end_to_end_path_json`, `decision_packets.context_refs_json`, `decision_packets.context_artifact_refs_json`, `decision_packets.residual_risk_refs_json`, `residual_risks.accepted_risk_json`, `residual_risks.follow_up_requirement_json`, `evidence_manifests.criteria_json`, `evidence_manifests.supporting_refs_json`, `evidence_manifests.stale_if_json`, `evals.evidence_reviewed_json`, `evals.independence_json`, `evals.artifact_refs_json`, `runs.observed_changes_json`, `runs.command_results_json`, `runs.artifact_refs_json`, `approvals.*_json`, `manual_qa_records.findings_json`, `manual_qa_records.residual_risk_refs_json`, and `validator_runs.findings_json`.

If an implementation cannot derive an input above from existing fields, add `TODO_IMPLEMENT` naming the exact table and field before changing DDL.

| MVP stage | Hardening coverage |
|---|---|
| MVP-1 | Journey/Decision skeleton, Decision Packet records, `decision_gate` aggregation |
| MVP-2 | shaping kernel, `prepare_write`, Write Authorization creation, scope, approval, baseline, decision/autonomy write checks, artifact registration |
| MVP-3 | `record_run`, Write Authorization consumption and violation detection, evidence manifest, `feedback_loop_check`, `codebase_stewardship_check`, projection/reconcile |
| MVP-4 | verification independence, Manual QA, `residual_risk_visibility_check`, acceptance, close blockers |
| MVP-5 | conformance and agency conformance fixtures for the hardened rules, including write authorization required and invalid cases |

Validator failure must be visible as state, blocked reasons, or close blockers. It must not be hidden in prose-only agent output.

Conformance fixture assertion semantics are owned by [Operations And Conformance](11-operations-and-conformance.md#fixture-assertion-semantics). The reference runner must implement those assertion modes against captured Core state, `task_events`, validator results, artifact registry/file integrity, projection job or freshness state, and returned error codes; it must not pass fixtures by matching rendered Markdown or agent prose alone.

## Minimal CLI Plan

The MVP CLI is an operator/debug surface over the same Core logic. It should not become a second API with different state semantics.

Minimum entrypoints:

- connect one local project and reference surface
- start or print MCP server connection information
- doctor project/runtime/MCP/artifacts/projections
- refresh projections
- reconcile pending items
- recover interrupted runs, stale projections, and artifact registry mismatch
- export a Task bundle
- run conformance smoke fixtures

Detailed operator procedures are owned by the operations and conformance document.

## Export Bundle Shape

Exports package state snapshots, projection snapshots, and artifact refs for review or archival.

```text
export/
  manifest.json
  state/
    task.json
    decision-packets.json
    residual-risks.json
    shared-designs.json
    task-spine-entries.json
    change-unit-dependencies.json
    baselines.json
    artifact-links.json
    runs.json
    approvals.json
    evidence-manifest.json
    evals.json
    manual-qa.json
  projections/
    TASK.md
    APR-*.md
    RUN-SUMMARY-*.md
    EVIDENCE-MANIFEST-*.md
    EVAL-*.md
    DIRECT-RESULT-*.md
  artifacts/
    ...
```

Raw secret values, unredacted sensitive logs, and PII are omitted or redacted before export.
