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
- MVP-required `ProjectionKind` renderers for `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, and `DIRECT-RESULT`
- MVP-optional `ProjectionKind` renderers only where policy requires them, source records exist, or the user/operator enables them
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

Implement Change Unit records, Change Unit dependency metadata, gate records, baseline capture, artifact registration, `harness.prepare_write`, Write Authorization records, approval request/decision flow, shaping updates, autonomy boundary fields, minimal changed-path/scope/approval/baseline Core checks, and decision/autonomy validators.

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

Implement `harness.record_run`, run records, Write Authorization consumption, evidence manifest records, feedback loop checks, codebase stewardship checks, projection jobs, MVP-required TASK/APR/RUN-SUMMARY/EVIDENCE-MANIFEST/DIRECT-RESULT renderers, managed block hashes, and reconcile item creation for managed drift or human-editable proposals.

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

### JSON Field Validation Boundary

JSON `TEXT` columns in the reference DDL are MVP storage flexibility, not permission to persist arbitrary or partially parsed JSON. Before any Core commit writes or updates a JSON `TEXT` field, Core must parse the value, reject malformed JSON, and validate the parsed value against the field's owning shape.

For public API payloads and API-shaped stored payloads, the owning shape is the schema in [MCP API And Schemas](05-mcp-api-and-schemas.md). For storage-only fields, the owning shape is the reference storage contract in this document or the specific owner document named by this document. This boundary keeps public schemas in `05-mcp-api-and-schemas.md` and SQLite DDL in `06-reference-mvp.md`.

Malformed JSON is invalid state. Schema-incompatible JSON is invalid state. Fields with defaults such as `'[]'` or `'{}'` must continue to store valid JSON of the expected array or object shape, not a different JSON kind just because SQLite stores the column as `TEXT`.

Recommended hardening: where the deployed SQLite build supports JSON functions, migrations should add `CHECK (json_valid(column_name))` or equivalent generated checks for JSON `TEXT` columns. These checks are defense in depth and do not replace Core's shape validation before commit; the MVP DDL below does not need a full rewrite to show every check inline.

### Canonical Enum Hardening

Canonical enum columns use `TEXT` in the reference DDL for readability, but they are not open strings. Core validation remains authoritative. Database checks, lookup-table validation, generated checks, and migration assertions are defense in depth and should first cover the state fields that drive write, close, replay, and projection behavior.

Minimum enum hardening targets:

| Field(s) | Values to harden |
| --- | --- |
| `tasks.mode` | `advisor`, `direct`, `work` |
| `tasks.lifecycle_phase` | `intake`, `shaping`, `ready`, `executing`, `verifying`, `qa`, `waiting_user`, `blocked`, `completed`, `cancelled` |
| `tasks.result` | `none`, `advice_only`, `passed`, `failed`, `cancelled` |
| `tasks.close_reason` | `none`, `completed_verified`, `completed_self_checked`, `completed_with_risk_accepted`, `cancelled`, `superseded` |
| `tasks.assurance_level` | `none`, `self_checked`, `detached_verified` |
| `tasks.projection_status` | `current`, `stale`, `failed`, `unknown` |
| `task_gates.scope_gate` | `not_required`, `required`, `pending`, `passed`, `failed`, `blocked` |
| `task_gates.decision_gate` | `not_required`, `required`, `pending`, `resolved`, `deferred`, `blocked` |
| `task_gates.approval_gate` | `not_required`, `required`, `pending`, `granted`, `denied`, `expired` |
| `task_gates.design_gate` | `not_required`, `required`, `pending`, `passed`, `partial`, `waived`, `stale`, `blocked` |
| `task_gates.evidence_gate` | `not_required`, `none`, `partial`, `sufficient`, `stale`, `blocked` |
| `task_gates.verification_gate` | `not_required`, `required`, `pending`, `passed`, `failed`, `waived_by_user`, `blocked` |
| `task_gates.qa_gate` | `not_required`, `required`, `pending`, `passed`, `failed`, `waived` |
| `task_gates.acceptance_gate` | `not_required`, `required`, `pending`, `accepted`, `rejected` |
| `write_authorizations.status` | `allowed`, `consumed`, `expired`, `stale`, `revoked` |
| `decision_packets.status` | `proposed`, `pending_user`, `resolved`, `deferred`, `rejected`, `blocked`, `superseded` |
| `manual_qa_records.result` | `passed`, `failed`, `waived` |
| `evals.verdict` | `passed`, `failed`, `blocked`, `inconclusive` |
| `projection_jobs.status` | `pending`, `running`, `completed`, `failed`, `skipped` |

For new tables or rebuild migrations, representative inline hardening is `status TEXT NOT NULL CHECK (status IN (...))`. Existing SQLite tables may need a table rebuild, a small lookup table checked by Core before commit, or a migration-time assertion that rejects unknown values before tightening. Apply the same pattern to other status-like state fields as their owner enums are finalized, especially `approvals.status`, `runs.kind`, `runs.status`, `evidence_manifests.status`, `residual_risks.visibility_status`, `residual_risks.status`, `reconcile_items.status`, `validator_runs.status`, `validator_runs.guarantee_level`, and design-quality status columns. Do not invent database-only enum values; bind storage hardening to the kernel/API owner enum.

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
  basis_state_version INTEGER NOT NULL,
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
  -- Optional compatibility ref; leave null when decision_requests is omitted.
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

-- Optional compatibility/routing table for routing, interaction, replay, or legacy handoff metadata only.
-- Minimal MVP implementations may omit this table.
-- decision_packet_id may remain null for routing/replay staging; unlinked rows are non-authoritative.
-- Gate aggregation may consider a row only through a linked compatible decision_packet_id.
CREATE TABLE decision_requests (
  decision_request_id TEXT PRIMARY KEY,
  decision_packet_id TEXT REFERENCES decision_packets(decision_packet_id),
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
  -- Optional compatibility ref; leave null when decision_requests is omitted.
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
  event_seq INTEGER NOT NULL UNIQUE,
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
  source_state_version INTEGER,
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

`task_events` remains append-only event history inside `state.sqlite`; MVP does not introduce a separate event store. `task_events.event_seq` is the deterministic global append sequence for all events in the database. Core allocates it under the same write transaction as the state change, and Journey reconstruction, API event lists, and conformance ordering use ascending `event_seq`, never timestamps. `task_events.state_version` records the resulting version for the affected scope. For task events this is `tasks.state_version`; for project-level events with `task_id=null` this is `project_state.state_version`. Multiple events may share an affected-scope `state_version`; `event_seq` still defines their order.

`tool_invocations` stores request replay metadata needed to return the original committed response. Only committed, non-dry-run tool calls create or update `tool_invocations`; `dry_run=true` creates no replay row and does not consume the idempotency key for authoritative replay. Non-authoritative diagnostics, if an implementation keeps them, must not be stored in `tool_invocations` or used to replay state-changing responses. `tool_invocations.request_hash` stores the canonical request hash defined by the MCP API idempotency rules: canonical JSON, UTF-8, `tool_name`, schema-normalized request body and optional fields, sorted object keys, schema-ordered arrays unless explicitly order-insignificant, NFC Unicode strings, and envelope coverage that excludes only `request_id` and `idempotency_key`. `tool_invocations.state_version` stores the same primary affected-scope version returned in `ToolResponseBase.state_version`: Task State Version when Core resolves a primary Task, otherwise Project State Version. Reusing an idempotency key with a different `request_hash` returns `STATE_CONFLICT`.

`tasks.projection_version` is the TASK projection/template/job version used to prevent older TASK renders from replacing newer ones. It is not a state clock. `tasks.projected_version`, if retained, is only the TASK projection summary cache of the last rendered source state version. It must not be treated as the storage location for every task-related `ProjectionKind`.

`tasks.projection_status` is the TASK projection status summary. Per-kind projection freshness is tracked through `projection_jobs.source_state_version`, job status, managed hashes, and the relevant projection records or artifact refs for MVP-required `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, and `DIRECT-RESULT`; MVP-optional `MANUAL-QA`, `TDD-TRACE`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, and `INTERFACE-CONTRACT`; and enabled extension / appendix kinds such as `DEC`, `DESIGN`, `EXPORT`, and `JOURNEY-CARD`. `APR` freshness starts from committed Approval records and their approval-shaped Decision Packets, not from non-mutating `approval_request_candidate` payloads. Do not treat one Task field as owning all projection freshness.

`write_authorizations` stores durable allow decisions from `prepare_write`. When `dry_run=false` and `prepare_write` returns `allowed`, Core creates a distinct `write_authorizations` row for a distinct compatible request and returns its ref. `write_authorizations.basis_state_version` stores the affected-scope state version Core used as the compatibility basis for the allowed write attempt; for MVP this is the Task State Version for `task_id`. It is audit metadata for idempotent replay, stale detection, and state-basis explanations, and it is not necessarily the resulting response `state_version` after the transaction appends events. `authorization_effect=returned` is reserved for idempotent replay of the same committed `prepare_write` request and response with the same idempotency key, request hash, and `basis_state_version`. A distinct compatible request creates a distinct Write Authorization; compatibility does not make authorizations reusable. Core may stale, expire, or revoke older unconsumed authorizations if their compatibility basis changes. `updated_at` changes whenever authorization status changes; status history remains in `task_events`.

Stored `write_authorizations` rows require non-null `basis_state_version`, including rows inserted from conformance fixture seeds. A fixture runner may derive the field from the seeded affected-scope state version before insert, but the stored row must still contain the value and must not treat it as the post-transaction `ToolResponseBase.state_version`.

Implementation and direct `record_run` calls consume a compatible unexpired authorization by recording `runs.write_authorization_id` and marking the authorization consumed with `consumed_by_run_id` and `consumed_at`. The reciprocal links `write_authorizations.consumed_by_run_id` and `runs.write_authorization_id` must point to each other in the same Core transaction. A mismatch is invalid state; `recover` must repair it or block affected close. Consuming an authorization does not make observed changes valid by itself; changed-path, tool, command, network, secret, Change Unit, approval, baseline, and Decision Packet validation still verifies the committed Run.

`runs.write_authorization_id` is populated only when a Run successfully consumes a compatible Write Authorization. A violation or audit Run that attempted to use an invalid, stale, missing, consumed, or scope-exceeded authorization must not populate `runs.write_authorization_id`; store the attempted authorization ref in validator findings, run violation payload, or `task_events.payload_json` when useful. Such a Run may be recorded for audit or recovery if an observed product write already happened, but it must not satisfy evidence sufficiency, detached verification, QA, acceptance, or close readiness. The corresponding Write Authorization remains unconsumed and may be marked stale, revoked, or expired according to the violation and compatibility basis.

Write Authorizations are single-use for storage. The unique partial index on `runs.write_authorization_id` prevents more than one committed Run row from consuming the same authorization. Idempotent replay returns the original Run and response metadata; it does not insert a second Run row.

`decision_packets` is the canonical state table for blocking product judgment and the authority path for `decision_gate`. `decision_requests` is an optional interaction/routing compatibility table for implementation handoff, replay, or legacy request flow; a minimal MVP implementation may omit it. A `decision_request` alone never satisfies `decision_gate`, approval, acceptance, waiver, residual-risk acceptance, or close. `decision_requests` rows are never read by `decision_gate` aggregation except through a linked compatible `decision_packet_id`. Only compatible `decision_packets` plus currently detected blockers feed the `decision_gate` authority path. Core must create or associate a compatible Decision Packet when blocking product judgment exists. Approval decisions link to Decision Packets through `approvals.decision_packet_id`; `decision_request_id` may remain as routing metadata but is not the approval authority path. If `decision_requests` is kept, `decision_requests.decision_packet_id` may remain nullable for routing or replay staging, but unlinked rows are non-authoritative and must not be read by gate aggregation. If `decision_requests` is omitted, omit its indexes and leave nullable compatibility fields such as `approvals.decision_request_id` and `decision_packets.decision_request_id` empty.

`residual_risks` is the canonical table for close-relevant remaining uncertainty, accepted risk, follow-up requirements, and close impact. Accepted-risk identity in MVP is the `residual_risk_id`; there is no separate `accepted_risks` table or `ARISK-*` canonical record. `residual_risks.accepted_risk_json`, `status`, and `accepted_at` store accepted-risk metadata/state on the residual-risk row. Decision Packets may reference residual risks through `decision_packets.residual_risk_refs_json`; they must not bury the only canonical residual-risk payload inside the Decision Packet.

MVP final acceptance has no `acceptance_records` table. `record_user_decision(decision_kind=acceptance)` stores the user answer in the canonical Decision Packet path, including `decision_packets.decision_json` and `decided_at`, updates `task_gates.acceptance_gate`, and appends `state.sqlite.task_events` such as `acceptance_recorded`. Close reads that gate plus the relevant Decision Packet and event history; it does not look for a separate acceptance row.

`artifact_links` is the queryable many-to-many attachment table for artifacts. Use it to attach artifacts to `run`, `decision_packet`, `shared_design`, `residual_risk`, `evidence_manifest`, `tdd_trace`, `manual_qa_record`, `eval`, and `export` records. Existing `artifact_refs_json` fields may preserve ordered or record-local context, but multi-record artifact reuse and artifact integrity checks should use `artifact_links`.

Manual QA waiver rule: a QA waiver with product/user risk requires a compatible `qa_waiver` Decision Packet linked by `manual_qa_records.waiver_decision_packet_id` and any close-relevant risk refs in `manual_qa_records.residual_risk_refs_json`; otherwise the waiver is blocked.

`change_unit_dependencies` is MVP DAG metadata for shaping, ordering, and close visibility. It is not a parallel orchestration scheduler and does not authorize multiple active implementation lanes.

`baselines` stores BaselineCapture records in state with repo head, branch, dirty flag, tree hash, included/ignored paths, optional diff artifact, and status. `baseline_ref` fields in other tables refer to `baselines.baseline_ref`.

Recommended indexes:

```sql
CREATE INDEX idx_task_events_task_version ON task_events(task_id, state_version);
CREATE INDEX idx_task_events_task_seq ON task_events(task_id, event_seq);
CREATE INDEX idx_decision_requests_task_status ON decision_requests(task_id, status); -- optional; omit when decision_requests is omitted
CREATE INDEX idx_decision_requests_packet ON decision_requests(decision_packet_id); -- optional; omit when decision_requests is omitted
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

`task_events` is append-only by application policy. `event_seq` is monotonically allocated and never reused. Recovery appends compensating events with new `event_seq` values; it should not rewrite historical rows or historical order.

Deterministic event order is ascending `task_events.event_seq`. `state_version` is an affected-scope concurrency/result clock, and `created_at` is audit metadata; neither field is sufficient for conformance ordering when several events share a state version or timestamp.

Reference MVP event storage follows the [Kernel Stable Event Catalog](03-kernel-spec.md#stable-event-catalog). Stable events remain rows in `state.sqlite.task_events`; no separate event store is introduced. The Write Authorization lifecycle vocabulary remains:

```text
write_authorization_created
write_authorization_returned
write_authorization_consumed
write_authorization_expired
write_authorization_staled
write_authorization_revoked
write_authorization_violation_detected
```

`scope_violation_detected` may be appended when a Run observes a general scope violation; it is stable for conformance assertions but is not part of the Write Authorization lifecycle vocabulary. Tool-specific event names that are not in the kernel catalog are optional or illustrative extension events and must not be required by MVP fixtures.

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

`tree_hash` is computed from a deterministic tree manifest after ignore rules have excluded ignored paths. Each entry uses a normalized relative POSIX path with no leading `./`. Path strings are normalized to Unicode NFC before sorting and hashing, and paths are sorted after normalization. Regular file content is hashed as bytes exactly as stored, without line-ending normalization, and the entry includes the content hash, file size, and executable bit where available. Symlink entries hash the link target rather than dereferenced content unless the implementation explicitly disallows symlinks and records that exclusion or block. The final manifest is serialized canonically before hashing so equivalent snapshots produce the same `tree_hash`.

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

Projection jobs are the durable outbox between committed state and Product Repository Markdown files. The `projection_jobs` table above owns job persistence and the canonical per-projection `source_state_version` metadata.

`projection_jobs.projection_version` is the projection/template/job version; it is not an affected-scope state clock. `projection_jobs.source_state_version` is the affected-scope state clock used as the render source for that projection job. It may be null for pending jobs and jobs that fail before the source state is resolved; completed successful renders must record it.

For sensitive approvals, `prepare_write` with `decision=approval_required` may enqueue `TASK` for changed task state or blockers, but it must not enqueue `APR` for the returned `approval_request_candidate`. The `APR` job is enqueued by `harness.request_user_decision(decision_kind=approval)` after it creates the canonical approval-shaped Decision Packet and linked pending Approval record, and by `harness.record_user_decision` when it updates that Approval decision.

For MVP, Decision Packet visibility is rendered through `TASK` projections, status/next responses, judgment-context resources, and decision-packet read resources. A standalone `DEC` projection is optional unless the standalone Decision Packet projection feature is enabled. Persisted `JOURNEY-CARD` Markdown is optional; current-position Journey Card output in status, next, and significant resume flows remains an agency-conformance requirement. This document does not define extension template text.

The job lifecycle below applies to every enqueued `ProjectionKind`. MVP smoke must cover the MVP-required tier; MVP-optional jobs are covered when policy, records, or operator settings enable them. Extension / appendix jobs such as `DEC`, `DESIGN`, `EXPORT`, and `JOURNEY-CARD` are not required for MVP smoke unless the corresponding feature is enabled.

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

`managed_hash` is computed only from the projector-owned managed block body after projector canonicalization; the `HARNESS:BEGIN` and `HARNESS:END` marker lines are excluded from the hash input. The projector normalizes line endings to LF before hashing and preserves meaningful whitespace according to the projection rules for that block. `managed_hash` is a drift-detection value; it does not make a Markdown projection canonical state.

### Projection Worker Execution

The reference projector executes pending jobs after the Core transaction commits.

MVP worker steps:

1. Select the oldest `pending` job for the target projection and acquire the projection-job lock.
2. Mark the job `running`, read the latest state records, artifact refs, and previous managed hash, and resolve `source_state_version` from the affected-scope state clock.
3. If the job's `projection_version` is older than the target's current projection/template/job version, mark it `skipped`.
4. Render the front matter and managed block from committed records and artifact refs, including `source_state_version`.
5. If the existing managed block hash differs from the last recorded hash, create or update a `reconcile_items` row, mark the job `skipped`, and set the projection status to `stale`.
6. Preserve human-editable sections and write the projection through a temporary file plus atomic rename.
7. Record the new managed hash, output path, `projection_jobs.projection_version`, `projection_jobs.source_state_version`, and `completed` status. For the `TASK` projection summary only, update `tasks.projected_version` with that same source state version as a Task-level summary cache.
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

MVP validators use one shared result shape from the API document. The runner is intentionally small.

Minimal validator rollout uses the [MVP Severity Defaults](08-design-quality-policy-pack.md#mvp-severity-defaults) matrix as the default severity router. The runner may initially implement shallow checks for each stable ID, but when the matrix marks an applicable policy as `blocking before write`, `close blocker`, or `Decision Packet required`, it must emit a gate/blocker-compatible result instead of downgrading the finding to `warning`. `warning` and `not_required` defaults remain valid only while the policy contract, user request, sensitive category, public commitment, residual risk, and conformance fixture do not raise severity.

Minimal runner shape:

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
| `surface_capability_check` | connected surface capability is sufficient for the requested operation or reported honestly through capability findings |

Core precondition checks such as active Task, active Change Unit, changed paths, approval scope, baseline freshness, artifact integrity, evidence sufficiency, verification independence, same-session verification guard, and projection freshness may still run before or beside these validators. They should not be emitted as alternate design/agency validator IDs in MVP conformance. Capability checks that emit `ValidatorResult` use the stable `surface_capability_check` ID; capability may also appear in blocked reasons and guarantee display without creating additional validator IDs.

Compatibility aliases:

| Older ID | Stable ID |
|---|---|
| `tdd_trace` | `tdd_trace_required` |
| `module_boundary_review` | `module_interface_review` |
| `docs_consistency` | `context_hygiene_check` |
| `projection_freshness` | `context_hygiene_check` |

These aliases are old compatibility inputs for legacy validator outputs or legacy validator IDs only; MVP conformance must emit the stable IDs above. The `projection_freshness` alias maps older validator output to `context_hygiene_check`; new MVP fixture assertions for mechanical projection freshness should use `expected_state.checks.projection_freshness`.

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

Conformance fixture assertion semantics are owned by [Operations And Conformance](11-operations-and-conformance.md#fixture-assertion-semantics), and stable `expected_events` names are owned by the [Kernel Stable Event Catalog](03-kernel-spec.md#stable-event-catalog). The reference runner must implement those assertion modes against captured Core state, `task_events`, validator results, artifact registry/file integrity, projection job or freshness state, and returned error codes; it must not pass fixtures by matching rendered Markdown or agent prose alone.

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
