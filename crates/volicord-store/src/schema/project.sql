CREATE TABLE project_state (
  project_id TEXT PRIMARY KEY,
  storage_profile TEXT NOT NULL,
  state_version INTEGER NOT NULL DEFAULT 0 CHECK (state_version >= 0),
  active_task_id TEXT,
  enforcement_profile_json TEXT NOT NULL DEFAULT '{"profile_id":"baseline_cooperative","guarantee_level":"cooperative","enabled_mechanisms":[],"source":"baseline_scope","status":"active"}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  FOREIGN KEY (project_id, active_task_id)
    REFERENCES tasks (project_id, task_id)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE tasks (
  project_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  created_by_actor_source TEXT NOT NULL,
  mode TEXT NOT NULL,
  requested_control_level TEXT NOT NULL CHECK (requested_control_level IN ('auto', 'observe', 'light', 'tracked', 'sensitive')),
  effective_control_level TEXT NOT NULL CHECK (effective_control_level IN ('observe', 'light', 'tracked', 'sensitive')),
  control_level_reason TEXT NOT NULL CHECK (length(trim(control_level_reason)) > 0),
  work_phase TEXT NOT NULL CHECK (work_phase IN ('shaping', 'implementation')),
  acceptance_policy TEXT NOT NULL CHECK (
    acceptance_policy IN ('required', 'not_required', 'policy_dependent')
  ),
  acceptance_policy_reason TEXT NOT NULL CHECK (length(trim(acceptance_policy_reason)) > 0),
  predecessor_task_id TEXT,
  lineage_relation TEXT CHECK (
    lineage_relation IS NULL OR lineage_relation IN (
      'continues', 'derived_from', 'split_from', 'replaces', 'implements_advice_from'
    )
  ),
  lineage_reason TEXT,
  carry_forward_json TEXT NOT NULL DEFAULT '[]',
  lifecycle_phase TEXT NOT NULL,
  result TEXT,
  title TEXT,
  summary TEXT,
  shaping_summary_json TEXT NOT NULL DEFAULT '{}',
  bounded_context_json TEXT NOT NULL DEFAULT '{}',
  autonomy_boundary_json TEXT NOT NULL DEFAULT '{}',
  scope_revision INTEGER NOT NULL DEFAULT 0 CHECK (scope_revision >= 0),
  close_basis_revision INTEGER NOT NULL DEFAULT 0 CHECK (close_basis_revision >= 0),
  close_basis_json TEXT,
  close_summary_json TEXT NOT NULL DEFAULT '{"close_reason":"none"}',
  current_change_unit_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  closed_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, task_id),
  UNIQUE (project_id, task_id, scope_revision),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, predecessor_task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, current_change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
    DEFERRABLE INITIALLY DEFERRED,
  CHECK (
    (predecessor_task_id IS NULL AND lineage_relation IS NULL AND lineage_reason IS NULL)
    OR (
      predecessor_task_id IS NOT NULL
      AND lineage_relation IS NOT NULL
      AND lineage_reason IS NOT NULL
      AND length(trim(lineage_reason)) > 0
      AND predecessor_task_id <> task_id
    )
  )
);

CREATE TABLE acceptance_criteria (
  project_id TEXT NOT NULL,
  acceptance_criterion_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  statement TEXT NOT NULL CHECK (length(trim(statement)) > 0),
  evidence_requirement TEXT NOT NULL CHECK (
    evidence_requirement IN ('required', 'optional', 'not_required')
  ),
  position INTEGER NOT NULL CHECK (position >= 0),
  status TEXT NOT NULL CHECK (status IN ('active', 'retired')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  retired_at TEXT,
  PRIMARY KEY (project_id, acceptance_criterion_id),
  UNIQUE (project_id, task_id, acceptance_criterion_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  CHECK (
    (status = 'active' AND retired_at IS NULL)
    OR (status = 'retired' AND retired_at IS NOT NULL)
  )
);

CREATE TABLE evidence_claims (
  project_id TEXT NOT NULL,
  evidence_claim_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  statement TEXT NOT NULL CHECK (length(trim(statement)) > 0),
  created_at TEXT NOT NULL,
  PRIMARY KEY (project_id, task_id, evidence_claim_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id)
);

CREATE TABLE change_units (
  project_id TEXT NOT NULL,
  change_unit_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('proposed', 'active', 'replaced', 'closed')),
  is_current INTEGER NOT NULL DEFAULT 0 CHECK (is_current IN (0, 1)),
  basis_state_version INTEGER NOT NULL CHECK (basis_state_version >= 0),
  scope_summary_json TEXT NOT NULL DEFAULT '{}',
  bounded_paths_json TEXT NOT NULL DEFAULT '[]',
  write_basis_json TEXT NOT NULL DEFAULT '{}',
  effect_contract_json TEXT NOT NULL DEFAULT 'null',
  lifecycle_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  closed_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, change_unit_id),
  UNIQUE (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id)
);

CREATE UNIQUE INDEX idx_change_units_one_current_active
  ON change_units (project_id, task_id)
  WHERE status = 'active' AND is_current = 1;

CREATE TABLE evidence_capture_intents (
  project_id TEXT NOT NULL,
  evidence_capture_intent_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT NOT NULL,
  scope_revision INTEGER NOT NULL CHECK (scope_revision >= 0),
  baseline_ref TEXT NOT NULL CHECK (length(trim(baseline_ref)) > 0),
  target_json TEXT NOT NULL,
  capture_kind TEXT NOT NULL CHECK (
    capture_kind IN (
      'verified_command_execution',
      'verified_tool_invocation'
    )
  ),
  capture_spec_json TEXT NOT NULL,
  input_sha256 TEXT NOT NULL CHECK (
    length(input_sha256) = 64 AND input_sha256 NOT GLOB '*[^0-9a-f]*'
  ),
  expected_outcome_json TEXT NOT NULL,
  requested_by_actor_source TEXT NOT NULL CHECK (
    length(trim(requested_by_actor_source)) > 0
  ),
  requesting_connection_internal_id TEXT NOT NULL CHECK (
    length(trim(requesting_connection_internal_id)) > 0
  ),
  session_context_json TEXT NOT NULL DEFAULT '{}',
  workspace_context_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, evidence_capture_intent_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
);

CREATE TABLE user_action_requests (
  project_id TEXT NOT NULL,
  user_action_request_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  action_kind TEXT NOT NULL CHECK (
    action_kind IN (
      'product_decision',
      'technical_decision',
      'scope_decision',
      'sensitive_approval',
      'final_acceptance',
      'residual_risk_acceptance',
      'cancellation',
      'evidence_observation'
    )
  ),
  request_json TEXT NOT NULL,
  basis_json TEXT NOT NULL,
  basis_status TEXT NOT NULL DEFAULT 'current'
    CHECK (basis_status IN ('current', 'stale', 'superseded')),
  required_for_json TEXT NOT NULL,
  requested_by_actor_source TEXT NOT NULL,
  source_method TEXT NOT NULL CHECK (
    source_method IN (
      'volicord.request_user_action',
      'volicord.record_shaping',
      'volicord.reconcile_changes'
    )
  ),
  source_idempotency_key TEXT NOT NULL CHECK (length(trim(source_idempotency_key)) > 0),
  requested_at TEXT NOT NULL,
  expires_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, user_action_request_id),
  UNIQUE (project_id, user_action_request_id, action_kind),
  UNIQUE (project_id, task_id, user_action_request_id, action_kind),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
);

CREATE TABLE user_action_resolutions (
  project_id TEXT NOT NULL,
  user_action_resolution_id TEXT NOT NULL,
  user_action_request_id TEXT NOT NULL,
  action_kind TEXT NOT NULL CHECK (
    action_kind IN (
      'product_decision',
      'technical_decision',
      'scope_decision',
      'sensitive_approval',
      'final_acceptance',
      'residual_risk_acceptance',
      'cancellation',
      'evidence_observation'
    )
  ),
  channel_kind TEXT NOT NULL CHECK (channel_kind = 'cli'),
  channel_submission_id TEXT NOT NULL CHECK (
    length(CAST(channel_submission_id AS BLOB)) BETWEEN 1 AND 256
    AND length(channel_submission_id) = length(CAST(channel_submission_id AS BLOB))
    AND channel_submission_id NOT GLOB '*[^!-~]*'
  ),
  resolution_json TEXT NOT NULL,
  resolved_by_actor_source TEXT NOT NULL CHECK (resolved_by_actor_source = 'local_user'),
  resolved_verification_basis TEXT NOT NULL CHECK (length(trim(resolved_verification_basis)) > 0),
  resolved_assurance_level TEXT NOT NULL CHECK (length(trim(resolved_assurance_level)) > 0),
  resolved_at TEXT NOT NULL,
  PRIMARY KEY (project_id, user_action_resolution_id),
  UNIQUE (project_id, user_action_request_id),
  UNIQUE (
    project_id,
    user_action_request_id,
    user_action_resolution_id,
    action_kind
  ),
  UNIQUE (project_id, channel_kind, channel_submission_id),
  FOREIGN KEY (project_id, user_action_request_id, action_kind)
    REFERENCES user_action_requests (
      project_id,
      user_action_request_id,
      action_kind
    )
);

CREATE TABLE shaping_checkpoints (
  project_id TEXT NOT NULL,
  shaping_checkpoint_id TEXT NOT NULL,
  predecessor_shaping_checkpoint_id TEXT,
  task_id TEXT NOT NULL,
  scope_revision INTEGER NOT NULL CHECK (scope_revision >= 0),
  baseline_ref TEXT,
  summary TEXT NOT NULL CHECK (length(trim(summary)) > 0),
  implementation_boundary TEXT,
  readiness TEXT NOT NULL CHECK (readiness IN ('blocked', 'ready', 'superseded')),
  source_refs_json TEXT NOT NULL DEFAULT '[]',
  evidence_refs_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  superseded_at TEXT,
  PRIMARY KEY (project_id, shaping_checkpoint_id),
  UNIQUE (project_id, task_id, shaping_checkpoint_id),
  UNIQUE (project_id, predecessor_shaping_checkpoint_id),
  FOREIGN KEY (project_id, task_id)
    REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, predecessor_shaping_checkpoint_id)
    REFERENCES shaping_checkpoints (project_id, task_id, shaping_checkpoint_id)
    DEFERRABLE INITIALLY DEFERRED,
  CHECK (
    predecessor_shaping_checkpoint_id IS NULL
    OR predecessor_shaping_checkpoint_id <> shaping_checkpoint_id
  ),
  CHECK (
    (readiness IN ('blocked', 'ready') AND superseded_at IS NULL)
    OR (readiness = 'superseded' AND superseded_at IS NOT NULL)
  ),
  CHECK (
    readiness <> 'ready'
    OR (
      baseline_ref IS NOT NULL
      AND length(trim(baseline_ref)) > 0
      AND implementation_boundary IS NOT NULL
      AND length(trim(implementation_boundary)) > 0
    )
  ),
  CHECK (baseline_ref IS NULL OR length(trim(baseline_ref)) > 0),
  CHECK (
    implementation_boundary IS NULL
    OR length(trim(implementation_boundary)) > 0
  )
);

CREATE UNIQUE INDEX idx_shaping_checkpoints_one_current
  ON shaping_checkpoints (project_id, task_id)
  WHERE readiness <> 'superseded';

CREATE TRIGGER trg_shaping_checkpoint_predecessor_immutable
BEFORE UPDATE OF predecessor_shaping_checkpoint_id ON shaping_checkpoints
WHEN NEW.predecessor_shaping_checkpoint_id IS NOT OLD.predecessor_shaping_checkpoint_id
BEGIN
  SELECT RAISE(ABORT, 'shaping checkpoint predecessor is immutable');
END;

CREATE TRIGGER trg_shaping_checkpoint_successor_requires_exact_predecessor
BEFORE INSERT ON shaping_checkpoints
WHEN NEW.predecessor_shaping_checkpoint_id IS NOT NULL
BEGIN
  SELECT CASE WHEN NOT EXISTS (
    SELECT 1
      FROM shaping_checkpoints AS predecessor
     WHERE predecessor.project_id = NEW.project_id
       AND predecessor.task_id = NEW.task_id
       AND predecessor.shaping_checkpoint_id = NEW.predecessor_shaping_checkpoint_id
       AND predecessor.readiness = 'superseded'
       AND predecessor.superseded_at = NEW.created_at
  ) THEN RAISE(ABORT, 'shaping checkpoint predecessor was not atomically superseded') END;
END;

CREATE TABLE shaping_checkpoint_gaps (
  project_id TEXT NOT NULL,
  shaping_checkpoint_id TEXT NOT NULL,
  shaping_gap_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  gap_kind TEXT NOT NULL CHECK (
    gap_kind IN (
      'goal_missing',
      'scope_boundary_missing',
      'non_goals_missing',
      'acceptance_criteria_missing',
      'autonomy_boundary_missing',
      'implementation_boundary_missing',
      'baseline_missing',
      'user_product_decision_required',
      'user_technical_decision_required',
      'user_scope_decision_required',
      'sensitive_approval_required'
    )
  ),
  summary TEXT NOT NULL CHECK (length(trim(summary)) > 0),
  affected_refs_json TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL CHECK (
    status IN ('current', 'accepted', 'rejected', 'deferred', 'applied')
  ),
  user_action_request_id TEXT,
  user_action_kind TEXT,
  PRIMARY KEY (project_id, shaping_checkpoint_id, shaping_gap_id),
  UNIQUE (
    project_id,
    shaping_checkpoint_id,
    shaping_gap_id,
    user_action_request_id,
    user_action_kind
  ),
  FOREIGN KEY (project_id, task_id, shaping_checkpoint_id)
    REFERENCES shaping_checkpoints (project_id, task_id, shaping_checkpoint_id),
  FOREIGN KEY (project_id, task_id, user_action_request_id, user_action_kind)
    REFERENCES user_action_requests (
      project_id,
      task_id,
      user_action_request_id,
      action_kind
    )
    DEFERRABLE INITIALLY DEFERRED,
  FOREIGN KEY (
    project_id,
    shaping_checkpoint_id,
    shaping_gap_id,
    user_action_request_id,
    user_action_kind
  ) REFERENCES shaping_checkpoint_user_actions (
    project_id,
    shaping_checkpoint_id,
    shaping_gap_id,
    user_action_request_id,
    action_kind
  ) DEFERRABLE INITIALLY DEFERRED,
  CHECK (
    (gap_kind = 'user_product_decision_required'
      AND user_action_kind = 'product_decision'
      AND user_action_request_id IS NOT NULL)
    OR (gap_kind = 'user_technical_decision_required'
      AND user_action_kind = 'technical_decision'
      AND user_action_request_id IS NOT NULL)
    OR (gap_kind = 'user_scope_decision_required'
      AND user_action_kind = 'scope_decision'
      AND user_action_request_id IS NOT NULL)
    OR (gap_kind = 'sensitive_approval_required'
      AND user_action_kind = 'sensitive_approval'
      AND user_action_request_id IS NOT NULL)
    OR (gap_kind IN (
        'goal_missing',
        'scope_boundary_missing',
        'non_goals_missing',
        'acceptance_criteria_missing',
        'autonomy_boundary_missing',
        'implementation_boundary_missing',
        'baseline_missing'
      )
      AND user_action_kind IS NULL
      AND user_action_request_id IS NULL)
  ),
  CHECK (status = 'current' OR user_action_request_id IS NOT NULL)
);

CREATE TABLE shaping_checkpoint_user_actions (
  project_id TEXT NOT NULL,
  shaping_checkpoint_id TEXT NOT NULL,
  shaping_gap_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  user_action_request_id TEXT NOT NULL,
  action_kind TEXT NOT NULL CHECK (
    action_kind IN (
      'product_decision',
      'technical_decision',
      'scope_decision',
      'sensitive_approval'
    )
  ),
  user_action_resolution_id TEXT,
  linked_at TEXT NOT NULL,
  resolved_at TEXT,
  PRIMARY KEY (project_id, shaping_checkpoint_id, shaping_gap_id),
  UNIQUE (project_id, user_action_request_id),
  UNIQUE (
    project_id,
    shaping_checkpoint_id,
    shaping_gap_id,
    user_action_request_id,
    action_kind
  ),
  FOREIGN KEY (
    project_id,
    shaping_checkpoint_id,
    shaping_gap_id,
    user_action_request_id,
    action_kind
  ) REFERENCES shaping_checkpoint_gaps (
    project_id,
    shaping_checkpoint_id,
    shaping_gap_id,
    user_action_request_id,
    user_action_kind
  ) DEFERRABLE INITIALLY DEFERRED,
  FOREIGN KEY (project_id, task_id, user_action_request_id, action_kind)
    REFERENCES user_action_requests (
      project_id,
      task_id,
      user_action_request_id,
      action_kind
    ),
  FOREIGN KEY (
    project_id,
    user_action_request_id,
    user_action_resolution_id,
    action_kind
  ) REFERENCES user_action_resolutions (
    project_id,
    user_action_request_id,
    user_action_resolution_id,
    action_kind
  ),
  CHECK (
    (user_action_resolution_id IS NULL AND resolved_at IS NULL)
    OR (user_action_resolution_id IS NOT NULL AND resolved_at IS NOT NULL)
  )
);

CREATE TABLE shaping_decision_applications (
  project_id TEXT NOT NULL,
  shaping_decision_application_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  source_checkpoint_id TEXT NOT NULL,
  source_gap_id TEXT NOT NULL,
  user_action_request_id TEXT NOT NULL,
  user_action_resolution_id TEXT NOT NULL,
  judgment_kind TEXT NOT NULL CHECK (
    judgment_kind IN (
      'product_decision',
      'technical_decision',
      'scope_decision',
      'sensitive_approval'
    )
  ),
  application_owner TEXT NOT NULL CHECK (
    application_owner IN (
      'volicord.update_scope',
      'volicord.advance_task',
      'volicord.record_shaping'
    )
  ),
  applied_scope_revision INTEGER NOT NULL CHECK (applied_scope_revision >= 0),
  applied_baseline_ref TEXT NOT NULL CHECK (length(trim(applied_baseline_ref)) > 0),
  applied_change_unit_id TEXT,
  applied_at TEXT NOT NULL,
  authority_status TEXT NOT NULL CHECK (
    authority_status IN ('current', 'stale', 'superseded')
  ),
  superseded_at TEXT,
  PRIMARY KEY (project_id, shaping_decision_application_id),
  UNIQUE (project_id, task_id, shaping_decision_application_id),
  UNIQUE (project_id, user_action_resolution_id, application_owner),
  FOREIGN KEY (project_id, task_id)
    REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, source_checkpoint_id)
    REFERENCES shaping_checkpoints (project_id, task_id, shaping_checkpoint_id),
  FOREIGN KEY (project_id, source_checkpoint_id, source_gap_id)
    REFERENCES shaping_checkpoint_gaps (
      project_id,
      shaping_checkpoint_id,
      shaping_gap_id
    ),
  FOREIGN KEY (
    project_id,
    source_checkpoint_id,
    source_gap_id,
    user_action_request_id,
    judgment_kind
  ) REFERENCES shaping_checkpoint_user_actions (
    project_id,
    shaping_checkpoint_id,
    shaping_gap_id,
    user_action_request_id,
    action_kind
  ),
  FOREIGN KEY (
    project_id,
    user_action_request_id,
    user_action_resolution_id,
    judgment_kind
  ) REFERENCES user_action_resolutions (
    project_id,
    user_action_request_id,
    user_action_resolution_id,
    action_kind
  ),
  FOREIGN KEY (project_id, task_id, applied_change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  CHECK (
    (authority_status = 'current' AND superseded_at IS NULL)
    OR (authority_status IN ('stale', 'superseded') AND superseded_at IS NOT NULL)
  )
);

CREATE TABLE shaping_checkpoint_applications (
  project_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  shaping_checkpoint_id TEXT NOT NULL,
  shaping_decision_application_id TEXT NOT NULL,
  carried_from_checkpoint_id TEXT,
  linked_at TEXT NOT NULL,
  PRIMARY KEY (
    project_id,
    shaping_checkpoint_id,
    shaping_decision_application_id
  ),
  FOREIGN KEY (project_id, task_id, shaping_checkpoint_id)
    REFERENCES shaping_checkpoints (project_id, task_id, shaping_checkpoint_id),
  FOREIGN KEY (project_id, task_id, shaping_decision_application_id)
    REFERENCES shaping_decision_applications (
      project_id,
      task_id,
      shaping_decision_application_id
    ),
  FOREIGN KEY (project_id, task_id, carried_from_checkpoint_id)
    REFERENCES shaping_checkpoints (project_id, task_id, shaping_checkpoint_id),
  CHECK (
    carried_from_checkpoint_id IS NULL
    OR carried_from_checkpoint_id <> shaping_checkpoint_id
  )
);

CREATE TRIGGER trg_shaping_decision_application_requires_accepted_resolution
BEFORE INSERT ON shaping_decision_applications
BEGIN
  SELECT CASE WHEN NOT EXISTS (
    SELECT 1
      FROM shaping_checkpoint_gaps AS gap
      JOIN shaping_checkpoint_user_actions AS link
        ON link.project_id = gap.project_id
       AND link.shaping_checkpoint_id = gap.shaping_checkpoint_id
       AND link.shaping_gap_id = gap.shaping_gap_id
      JOIN user_action_requests AS request
        ON request.project_id = link.project_id
       AND request.user_action_request_id = link.user_action_request_id
      JOIN user_action_resolutions AS resolution
        ON resolution.project_id = link.project_id
       AND resolution.user_action_request_id = link.user_action_request_id
       AND resolution.user_action_resolution_id = link.user_action_resolution_id
     WHERE gap.project_id = NEW.project_id
       AND gap.task_id = NEW.task_id
       AND gap.shaping_checkpoint_id = NEW.source_checkpoint_id
       AND gap.shaping_gap_id = NEW.source_gap_id
       AND gap.status = 'accepted'
       AND link.user_action_request_id = NEW.user_action_request_id
       AND link.user_action_resolution_id = NEW.user_action_resolution_id
       AND link.action_kind = NEW.judgment_kind
       AND request.basis_status = 'current'
       AND json_extract(request.basis_json, '$.coordinates.scope_revision') = NEW.applied_scope_revision
       AND json_extract(request.basis_json, '$.coordinates.baseline_ref') = NEW.applied_baseline_ref
       AND json_extract(request.basis_json, '$.coordinates.change_unit_id') IS NEW.applied_change_unit_id
       AND json_extract(request.basis_json, '$.coordinates.compatibility_status') = 'current'
       AND json_extract(resolution.resolution_json, '$.resolution_type') = 'choice'
       AND json_extract(resolution.resolution_json, '$.machine_action') = 'accept'
       AND json_extract(resolution.resolution_json, '$.resolution_outcome') = 'accepted'
  ) THEN RAISE(ABORT, 'shaping application requires an exact accepted current resolution') END;
  SELECT CASE WHEN NOT EXISTS (
    SELECT 1
      FROM tasks AS task
     WHERE task.project_id = NEW.project_id
       AND task.task_id = NEW.task_id
       AND task.scope_revision = NEW.applied_scope_revision
       AND json_extract(task.shaping_summary_json, '$.baseline_ref') = NEW.applied_baseline_ref
       AND task.current_change_unit_id IS NEW.applied_change_unit_id
  ) THEN RAISE(ABORT, 'shaping application coordinates must match the current Task') END;
  SELECT CASE WHEN NOT (
    (NEW.judgment_kind = 'scope_decision'
      AND NEW.application_owner = 'volicord.update_scope')
    OR (NEW.judgment_kind IN ('product_decision', 'technical_decision', 'sensitive_approval')
      AND NEW.application_owner = CASE (
        SELECT mode FROM tasks
         WHERE project_id = NEW.project_id AND task_id = NEW.task_id
      ) WHEN 'advisor' THEN 'volicord.record_shaping'
        ELSE 'volicord.advance_task' END)
  ) THEN RAISE(ABORT, 'shaping application owner conflicts with decision policy') END;
  SELECT CASE WHEN NEW.authority_status <> 'current' OR NEW.superseded_at IS NOT NULL
    THEN RAISE(ABORT, 'new shaping application must be current') END;
END;

CREATE TRIGGER trg_shaping_decision_application_immutable
BEFORE UPDATE ON shaping_decision_applications
WHEN NEW.project_id IS NOT OLD.project_id
  OR NEW.shaping_decision_application_id IS NOT OLD.shaping_decision_application_id
  OR NEW.task_id IS NOT OLD.task_id
  OR NEW.source_checkpoint_id IS NOT OLD.source_checkpoint_id
  OR NEW.source_gap_id IS NOT OLD.source_gap_id
  OR NEW.user_action_request_id IS NOT OLD.user_action_request_id
  OR NEW.user_action_resolution_id IS NOT OLD.user_action_resolution_id
  OR NEW.judgment_kind IS NOT OLD.judgment_kind
  OR NEW.application_owner IS NOT OLD.application_owner
  OR NEW.applied_scope_revision IS NOT OLD.applied_scope_revision
  OR NEW.applied_baseline_ref IS NOT OLD.applied_baseline_ref
  OR NEW.applied_change_unit_id IS NOT OLD.applied_change_unit_id
  OR NEW.applied_at IS NOT OLD.applied_at
BEGIN
  SELECT RAISE(ABORT, 'shaping application identity and semantic coordinates are immutable');
END;

CREATE TRIGGER trg_shaping_decision_application_status_transition
BEFORE UPDATE OF authority_status ON shaping_decision_applications
WHEN NEW.authority_status <> OLD.authority_status
  AND NOT (
    OLD.authority_status = 'current'
    AND NEW.authority_status IN ('stale', 'superseded')
    AND NEW.superseded_at IS NOT NULL
  )
BEGIN
  SELECT RAISE(ABORT, 'invalid shaping application authority transition');
END;

CREATE TRIGGER trg_shaping_decision_application_invalidation_immutable
BEFORE UPDATE OF authority_status, superseded_at ON shaping_decision_applications
WHEN (OLD.authority_status <> 'current'
      AND (NEW.authority_status IS NOT OLD.authority_status
           OR NEW.superseded_at IS NOT OLD.superseded_at))
  OR (NEW.authority_status IS OLD.authority_status
      AND NEW.superseded_at IS NOT OLD.superseded_at)
BEGIN
  SELECT RAISE(ABORT, 'shaping application invalidation is immutable once recorded');
END;

CREATE TRIGGER trg_shaping_decision_application_delete_forbidden
BEFORE DELETE ON shaping_decision_applications
BEGIN
  SELECT RAISE(ABORT, 'shaping application audit records are immutable');
END;

CREATE TRIGGER trg_shaping_checkpoint_application_lineage
BEFORE INSERT ON shaping_checkpoint_applications
BEGIN
  SELECT CASE WHEN NOT EXISTS (
    SELECT 1 FROM shaping_decision_applications AS application
     WHERE application.project_id = NEW.project_id
       AND application.task_id = NEW.task_id
       AND application.shaping_decision_application_id = NEW.shaping_decision_application_id
       AND application.authority_status = 'current'
  ) THEN RAISE(ABORT, 'checkpoint lineage requires a current shaping application') END;
  SELECT CASE WHEN NEW.carried_from_checkpoint_id IS NULL AND NOT EXISTS (
    SELECT 1 FROM shaping_decision_applications AS application
     WHERE application.project_id = NEW.project_id
       AND application.shaping_decision_application_id = NEW.shaping_decision_application_id
       AND application.source_checkpoint_id = NEW.shaping_checkpoint_id
  ) THEN RAISE(ABORT, 'initial shaping application link must use its source checkpoint') END;
  SELECT CASE WHEN NEW.carried_from_checkpoint_id IS NOT NULL AND NOT EXISTS (
    SELECT 1
      FROM shaping_checkpoints AS successor
      JOIN shaping_checkpoint_applications AS predecessor_link
        ON predecessor_link.project_id = successor.project_id
       AND predecessor_link.task_id = successor.task_id
       AND predecessor_link.shaping_checkpoint_id = successor.predecessor_shaping_checkpoint_id
       AND predecessor_link.shaping_decision_application_id = NEW.shaping_decision_application_id
     WHERE successor.project_id = NEW.project_id
       AND successor.task_id = NEW.task_id
       AND successor.shaping_checkpoint_id = NEW.shaping_checkpoint_id
       AND successor.predecessor_shaping_checkpoint_id = NEW.carried_from_checkpoint_id
  ) THEN RAISE(ABORT, 'carried shaping application requires exact predecessor lineage') END;
END;

CREATE TRIGGER trg_shaping_checkpoint_application_immutable
BEFORE UPDATE ON shaping_checkpoint_applications
BEGIN
  SELECT RAISE(ABORT, 'shaping checkpoint application lineage is immutable');
END;

CREATE TRIGGER trg_shaping_checkpoint_application_delete_forbidden
BEFORE DELETE ON shaping_checkpoint_applications
BEGIN
  SELECT RAISE(ABORT, 'shaping checkpoint application lineage is immutable');
END;

CREATE TRIGGER trg_shaping_checkpoint_live_user_action_not_detached
BEFORE UPDATE OF readiness ON shaping_checkpoints
WHEN OLD.readiness <> 'superseded' AND NEW.readiness = 'superseded'
BEGIN
  SELECT CASE WHEN EXISTS (
    SELECT 1
      FROM shaping_checkpoint_user_actions AS link
      JOIN shaping_checkpoint_gaps AS gap
        ON gap.project_id = link.project_id
       AND gap.shaping_checkpoint_id = link.shaping_checkpoint_id
       AND gap.shaping_gap_id = link.shaping_gap_id
      JOIN user_action_requests AS request
        ON request.project_id = link.project_id
       AND request.user_action_request_id = link.user_action_request_id
     WHERE link.project_id = OLD.project_id
       AND link.shaping_checkpoint_id = OLD.shaping_checkpoint_id
       AND request.basis_status = 'current'
       AND NOT EXISTS (
         SELECT 1
           FROM shaping_decision_applications AS application
           JOIN shaping_checkpoint_applications AS application_link
             ON application_link.project_id = application.project_id
            AND application_link.shaping_decision_application_id = application.shaping_decision_application_id
          WHERE application.project_id = link.project_id
            AND application.user_action_request_id = link.user_action_request_id
            AND application.authority_status = 'current'
            AND application_link.shaping_checkpoint_id = OLD.shaping_checkpoint_id
       )
  ) THEN RAISE(ABORT, 'live shaping UserAction authority cannot be detached') END;
END;

CREATE TRIGGER trg_shaping_gap_not_added_to_ready_checkpoint
BEFORE INSERT ON shaping_checkpoint_gaps
WHEN EXISTS (
  SELECT 1
    FROM shaping_checkpoints
   WHERE project_id = NEW.project_id
     AND shaping_checkpoint_id = NEW.shaping_checkpoint_id
     AND readiness = 'ready'
)
BEGIN
  SELECT RAISE(ABORT, 'ready shaping checkpoint cannot receive a gap');
END;

CREATE TRIGGER trg_shaping_gap_insert_is_current
BEFORE INSERT ON shaping_checkpoint_gaps
WHEN NEW.status <> 'current'
BEGIN
  SELECT RAISE(ABORT, 'inserted shaping gap must be current');
END;

CREATE TRIGGER trg_shaping_gap_disposition_transition
BEFORE UPDATE OF status ON shaping_checkpoint_gaps
WHEN NEW.status <> OLD.status
  AND NOT (
    (OLD.status = 'current' AND NEW.status IN ('accepted', 'rejected', 'deferred'))
    OR (OLD.status = 'accepted' AND NEW.status = 'applied')
  )
BEGIN
  SELECT RAISE(ABORT, 'invalid shaping gap disposition transition');
END;

CREATE TRIGGER trg_shaping_checkpoint_ready_has_no_current_gap
BEFORE UPDATE OF readiness ON shaping_checkpoints
WHEN NEW.readiness = 'ready'
BEGIN
  SELECT CASE WHEN EXISTS (
    SELECT 1
      FROM shaping_checkpoint_gaps
     WHERE project_id = NEW.project_id
       AND shaping_checkpoint_id = NEW.shaping_checkpoint_id
       AND status = 'current'
  ) THEN RAISE(ABORT, 'ready shaping checkpoint has a current gap') END;
END;

CREATE TRIGGER trg_shaping_gap_disposition_requires_matching_user_resolution
BEFORE UPDATE OF status ON shaping_checkpoint_gaps
WHEN NEW.status IN ('accepted', 'rejected', 'deferred')
BEGIN
  SELECT CASE WHEN NOT EXISTS (
    SELECT 1
      FROM shaping_checkpoint_user_actions AS link
      JOIN user_action_resolutions AS resolution
        ON resolution.project_id = link.project_id
       AND resolution.user_action_request_id = link.user_action_request_id
       AND resolution.user_action_resolution_id = link.user_action_resolution_id
     WHERE link.project_id = NEW.project_id
       AND link.shaping_checkpoint_id = NEW.shaping_checkpoint_id
       AND link.shaping_gap_id = NEW.shaping_gap_id
       AND json_extract(resolution.resolution_json, '$.resolution_type') = 'choice'
       AND (
         (NEW.status = 'accepted'
           AND json_extract(resolution.resolution_json, '$.machine_action') = 'accept'
           AND json_extract(resolution.resolution_json, '$.resolution_outcome') = 'accepted')
         OR (NEW.status = 'rejected'
           AND json_extract(resolution.resolution_json, '$.machine_action') = 'reject'
           AND json_extract(resolution.resolution_json, '$.resolution_outcome') = 'rejected')
         OR (NEW.status = 'deferred'
           AND json_extract(resolution.resolution_json, '$.machine_action') = 'defer'
           AND json_extract(resolution.resolution_json, '$.resolution_outcome') = 'deferred')
       )
  ) THEN RAISE(ABORT, 'shaping disposition requires a matching linked resolution') END;
END;

CREATE TRIGGER trg_shaping_gap_application_requires_accepted_gap
BEFORE UPDATE OF status ON shaping_checkpoint_gaps
WHEN NEW.status = 'applied'
BEGIN
  SELECT CASE WHEN OLD.status <> 'accepted' OR NOT EXISTS (
    SELECT 1
      FROM shaping_checkpoint_user_actions AS link
      JOIN user_action_resolutions AS resolution
        ON resolution.project_id = link.project_id
       AND resolution.user_action_request_id = link.user_action_request_id
       AND resolution.user_action_resolution_id = link.user_action_resolution_id
     WHERE link.project_id = NEW.project_id
       AND link.shaping_checkpoint_id = NEW.shaping_checkpoint_id
       AND link.shaping_gap_id = NEW.shaping_gap_id
       AND json_extract(resolution.resolution_json, '$.resolution_type') = 'choice'
       AND json_extract(resolution.resolution_json, '$.machine_action') = 'accept'
       AND json_extract(resolution.resolution_json, '$.resolution_outcome') = 'accepted'
       AND EXISTS (
         SELECT 1
           FROM shaping_decision_applications AS application
           JOIN shaping_checkpoint_applications AS application_link
             ON application_link.project_id = application.project_id
            AND application_link.shaping_decision_application_id = application.shaping_decision_application_id
          WHERE application.project_id = NEW.project_id
            AND application.source_checkpoint_id = NEW.shaping_checkpoint_id
            AND application.source_gap_id = NEW.shaping_gap_id
            AND application.user_action_request_id = link.user_action_request_id
            AND application.user_action_resolution_id = link.user_action_resolution_id
            AND application.authority_status = 'current'
            AND application_link.shaping_checkpoint_id = NEW.shaping_checkpoint_id
       )
  ) THEN RAISE(ABORT, 'applied shaping gap requires an exact accepted resolution') END;
END;

CREATE TRIGGER trg_applied_shaping_gap_is_terminal
BEFORE UPDATE OF status ON shaping_checkpoint_gaps
WHEN OLD.status = 'applied' AND NEW.status <> 'applied'
BEGIN
  SELECT RAISE(ABORT, 'applied shaping gap status is terminal');
END;

CREATE TABLE project_continuity_records (
  project_id TEXT NOT NULL,
  continuity_record_id TEXT NOT NULL,
  source_task_id TEXT NOT NULL,
  source_change_unit_id TEXT,
  kind TEXT NOT NULL CHECK (kind IN ('decision', 'obligation', 'known_limit', 'accepted_risk', 'constraint')),
  title TEXT NOT NULL CHECK (length(trim(title)) > 0),
  summary TEXT NOT NULL CHECK (length(trim(summary)) > 0),
  rationale TEXT CHECK (rationale IS NULL OR length(trim(rationale)) > 0),
  applies_to_paths_json TEXT NOT NULL DEFAULT '[]',
  applies_to_refs_json TEXT NOT NULL DEFAULT '[]',
  source_refs_json TEXT NOT NULL DEFAULT '[]',
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL CHECK (status IN ('active', 'superseded', 'closed')),
  supersedes_refs_json TEXT NOT NULL DEFAULT '[]',
  review_triggers_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, continuity_record_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, source_task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, source_task_id, source_change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
);

CREATE TABLE write_tickets (
  project_id TEXT NOT NULL,
  write_ticket_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT NOT NULL,
  basis_state_version INTEGER NOT NULL CHECK (basis_state_version > 0),
  status TEXT NOT NULL CHECK (status IN ('active', 'consumed', 'invalidated', 'revoked')),
  validity_basis_json TEXT NOT NULL,
  allowed_path_prefixes_json TEXT NOT NULL DEFAULT '[]',
  denied_path_prefixes_json TEXT NOT NULL DEFAULT '[]',
  attempt_scope_json TEXT NOT NULL DEFAULT '{}',
  created_by_actor_source TEXT NOT NULL,
  created_by_user_action_resolution_id TEXT,
  idle_expires_at TEXT,
  invalidation_reason TEXT CHECK (
    invalidation_reason IS NULL OR invalidation_reason IN (
      'scope_revision_changed', 'change_unit_changed', 'baseline_changed',
      'workspace_changed', 'approval_basis_changed', 'idle_timeout',
      'task_closed', 'explicit_revoke'
    )
  ),
  consumed_by_run_id TEXT,
  consumed_at TEXT,
  revoked_at TEXT,
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, write_ticket_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, created_by_user_action_resolution_id)
    REFERENCES user_action_resolutions (project_id, user_action_resolution_id),
  FOREIGN KEY (project_id, consumed_by_run_id)
    REFERENCES runs (project_id, run_id)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE UNIQUE INDEX idx_write_tickets_consumed_run
  ON write_tickets (project_id, consumed_by_run_id)
  WHERE consumed_by_run_id IS NOT NULL;

CREATE TABLE runs (
  project_id TEXT NOT NULL,
  run_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  write_ticket_id TEXT,
  kind TEXT NOT NULL,
  status TEXT NOT NULL,
  summary_json TEXT NOT NULL DEFAULT '{}',
  observed_changes_json TEXT NOT NULL DEFAULT '{}',
  evidence_updates_json TEXT NOT NULL DEFAULT '[]',
  write_ticket_effect_json TEXT NOT NULL DEFAULT '{}',
  scope_revision INTEGER NOT NULL CHECK (scope_revision >= 0),
  created_by_actor_source TEXT NOT NULL,
  started_at TEXT,
  completed_at TEXT,
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, run_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, write_ticket_id)
    REFERENCES write_tickets (project_id, write_ticket_id)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE UNIQUE INDEX idx_runs_write_ticket
  ON runs (project_id, write_ticket_id)
  WHERE write_ticket_id IS NOT NULL;

CREATE TABLE artifact_staging (
  project_id TEXT NOT NULL,
  handle_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  created_by_actor_source TEXT NOT NULL,
  artifact_json TEXT NOT NULL DEFAULT '{}',
  safe_metadata_json TEXT NOT NULL DEFAULT '{}',
  tmp_path TEXT,
  sha256 TEXT,
  size_bytes INTEGER CHECK (size_bytes IS NULL OR size_bytes >= 0),
  content_type TEXT,
  redaction_state TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('staged', 'consumed', 'expired', 'discarded')),
  expires_at TEXT NOT NULL,
  consumed_by_run_id TEXT,
  promoted_artifact_id TEXT,
  consumed_at TEXT,
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, handle_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, consumed_by_run_id)
    REFERENCES runs (project_id, run_id)
    DEFERRABLE INITIALLY DEFERRED,
  FOREIGN KEY (project_id, promoted_artifact_id)
    REFERENCES artifacts (project_id, artifact_id)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE UNIQUE INDEX idx_artifact_staging_promoted_artifact
  ON artifact_staging (project_id, promoted_artifact_id)
  WHERE promoted_artifact_id IS NOT NULL;

CREATE TABLE evidence_capture_receipts (
  project_id TEXT NOT NULL,
  evidence_capture_receipt_id TEXT NOT NULL,
  evidence_capture_intent_id TEXT NOT NULL,
  staging_handle_id TEXT NOT NULL,
  capture_kind TEXT NOT NULL CHECK (
    capture_kind IN (
      'verified_command_execution',
      'verified_tool_invocation'
    )
  ),
  input_sha256 TEXT NOT NULL CHECK (
    length(input_sha256) = 64 AND input_sha256 NOT GLOB '*[^0-9a-f]*'
  ),
  result_sha256 TEXT NOT NULL CHECK (
    length(result_sha256) = 64 AND result_sha256 NOT GLOB '*[^0-9a-f]*'
  ),
  expected_outcome_json TEXT NOT NULL,
  observed_outcome_json TEXT NOT NULL,
  source_refs_json TEXT NOT NULL DEFAULT '[]',
  observed_by_actor_source TEXT NOT NULL CHECK (
    length(trim(observed_by_actor_source)) > 0
  ),
  observed_at TEXT NOT NULL,
  completeness TEXT NOT NULL CHECK (completeness = 'complete'),
  limitations_json TEXT NOT NULL DEFAULT '[]',
  safe_receipt_json TEXT NOT NULL,
  safe_receipt_sha256 TEXT NOT NULL CHECK (
    length(safe_receipt_sha256) = 64 AND safe_receipt_sha256 NOT GLOB '*[^0-9a-f]*'
  ),
  safe_receipt_size_bytes INTEGER NOT NULL CHECK (safe_receipt_size_bytes >= 0),
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, evidence_capture_receipt_id),
  UNIQUE (project_id, evidence_capture_intent_id),
  UNIQUE (
    project_id,
    evidence_capture_intent_id,
    evidence_capture_receipt_id
  ),
  UNIQUE (project_id, staging_handle_id),
  FOREIGN KEY (project_id, evidence_capture_intent_id)
    REFERENCES evidence_capture_intents (project_id, evidence_capture_intent_id),
  FOREIGN KEY (project_id, staging_handle_id)
    REFERENCES artifact_staging (project_id, handle_id)
);

CREATE TABLE evidence_capture_source_claims (
  project_id TEXT NOT NULL,
  source_claim_kind TEXT NOT NULL CHECK (
    source_claim_kind = 'host_invocation'
  ),
  source_claim_id TEXT NOT NULL CHECK (length(trim(source_claim_id)) > 0),
  evidence_capture_intent_id TEXT NOT NULL,
  evidence_capture_receipt_id TEXT NOT NULL,
  capture_kind TEXT NOT NULL CHECK (
    capture_kind IN (
      'verified_command_execution',
      'verified_tool_invocation'
    )
  ),
  claimed_at TEXT NOT NULL,
  CHECK (
    source_claim_kind != 'host_invocation'
    OR (
      length(source_claim_id) = 64
      AND source_claim_id NOT GLOB '*[^0-9a-f]*'
    )
  ),
  PRIMARY KEY (project_id, source_claim_kind, source_claim_id),
  FOREIGN KEY (
    project_id,
    evidence_capture_intent_id,
    evidence_capture_receipt_id
  ) REFERENCES evidence_capture_receipts (
    project_id,
    evidence_capture_intent_id,
    evidence_capture_receipt_id
  )
);

CREATE TABLE artifacts (
  project_id TEXT NOT NULL,
  artifact_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  producer_run_id TEXT,
  source_staging_handle_id TEXT,
  uri TEXT NOT NULL,
  body_path TEXT,
  sha256 TEXT,
  size_bytes INTEGER CHECK (size_bytes IS NULL OR size_bytes >= 0),
  content_type TEXT,
  integrity_status TEXT NOT NULL DEFAULT 'verified'
    CHECK (integrity_status IN ('verified', 'corrupt')),
  redaction_state TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('available', 'missing', 'integrity_failed', 'unavailable')),
  retention_json TEXT NOT NULL DEFAULT '{}',
  producer_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, artifact_id),
  CHECK (
    integrity_status <> 'verified'
    OR (
      content_type IS NOT NULL
      AND length(trim(content_type)) > 0
      AND sha256 IS NOT NULL
      AND length(sha256) = 64
      AND sha256 NOT GLOB '*[^0-9a-f]*'
      AND size_bytes IS NOT NULL
      AND size_bytes >= 0
    )
  ),
  CHECK (
    body_path IS NULL
    OR (
      length(trim(body_path)) > 0
      AND body_path NOT GLOB '/*'
      AND body_path NOT GLOB '[A-Za-z]:*'
      AND instr(body_path, '\') = 0
      AND body_path <> '..'
      AND body_path NOT GLOB '../*'
      AND body_path NOT GLOB '*/../*'
      AND body_path NOT GLOB '*/..'
      AND body_path <> 'artifacts'
      AND body_path NOT GLOB 'artifacts/*'
    )
  ),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, producer_run_id) REFERENCES runs (project_id, run_id),
  FOREIGN KEY (project_id, source_staging_handle_id)
    REFERENCES artifact_staging (project_id, handle_id)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE UNIQUE INDEX idx_artifacts_source_staging
  ON artifacts (project_id, source_staging_handle_id)
  WHERE source_staging_handle_id IS NOT NULL;

CREATE TABLE artifact_links (
  project_id TEXT NOT NULL,
  artifact_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  owner_record_kind TEXT NOT NULL CHECK (
    owner_record_kind IN ('task', 'change_unit', 'run', 'user_action_request', 'user_action_resolution', 'evidence_summary', 'evidence_observation', 'evidence_producer', 'blocker')
  ),
  owner_record_id TEXT NOT NULL,
  created_by_run_id TEXT,
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, artifact_id, owner_record_kind, owner_record_id),
  FOREIGN KEY (project_id, artifact_id) REFERENCES artifacts (project_id, artifact_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, created_by_run_id) REFERENCES runs (project_id, run_id)
);

CREATE TABLE evidence_summaries (
  project_id TEXT NOT NULL,
  evidence_summary_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  produced_at_state_version INTEGER NOT NULL CHECK (produced_at_state_version >= 0),
  status TEXT NOT NULL,
  coverage_json TEXT NOT NULL DEFAULT '[]',
  supporting_refs_json TEXT NOT NULL DEFAULT '[]',
  gap_refs_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, evidence_summary_id),
  UNIQUE (project_id, task_id, produced_at_state_version),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
);

CREATE TABLE evidence_observations (
  project_id TEXT NOT NULL,
  evidence_observation_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  run_id TEXT,
  acceptance_criterion_id TEXT,
  evidence_claim_id TEXT,
  source_kind TEXT NOT NULL CHECK (
    source_kind IN ('agent_report', 'connection_observation', 'external_tool', 'user_observation', 'reused_evidence', 'unverified_claim')
  ),
  assurance_level TEXT NOT NULL CHECK (
    assurance_level IN ('cooperative_report', 'registered_connection_observed', 'external_tool_result', 'user_observed', 'unverified')
  ),
  observed_by_actor_source TEXT,
  tool_name TEXT,
  tool_invocation_id TEXT,
  tool_metadata_json TEXT NOT NULL DEFAULT '{}',
  input_refs_json TEXT NOT NULL DEFAULT '[]',
  source_refs_json TEXT NOT NULL DEFAULT '[]',
  output_artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  limitations_json TEXT NOT NULL DEFAULT '[]',
  observed_at TEXT NOT NULL,
  recorded_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, evidence_observation_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, run_id)
    REFERENCES runs (project_id, run_id)
    DEFERRABLE INITIALLY DEFERRED,
  FOREIGN KEY (project_id, task_id, acceptance_criterion_id)
    REFERENCES acceptance_criteria (project_id, task_id, acceptance_criterion_id),
  FOREIGN KEY (project_id, task_id, evidence_claim_id)
    REFERENCES evidence_claims (project_id, task_id, evidence_claim_id),
  CHECK (
    (acceptance_criterion_id IS NOT NULL AND evidence_claim_id IS NULL)
    OR (acceptance_criterion_id IS NULL AND evidence_claim_id IS NOT NULL)
  )
);

CREATE TABLE evidence_producers (
  project_id TEXT NOT NULL,
  evidence_producer_id TEXT NOT NULL,
  evidence_capture_intent_id TEXT NOT NULL,
  evidence_capture_receipt_id TEXT NOT NULL,
  evidence_observation_id TEXT NOT NULL,
  artifact_id TEXT NOT NULL,
  run_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT NOT NULL,
  scope_revision INTEGER NOT NULL CHECK (scope_revision >= 0),
  baseline_ref TEXT NOT NULL CHECK (length(trim(baseline_ref)) > 0),
  producer_kind TEXT NOT NULL CHECK (
    producer_kind IN (
      'verified_command_execution',
      'verified_tool_invocation'
    )
  ),
  canonical_producer_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, evidence_producer_id),
  UNIQUE (project_id, evidence_capture_intent_id),
  UNIQUE (project_id, evidence_capture_receipt_id),
  UNIQUE (project_id, evidence_observation_id),
  UNIQUE (project_id, artifact_id),
  FOREIGN KEY (project_id, evidence_capture_intent_id)
    REFERENCES evidence_capture_intents (project_id, evidence_capture_intent_id),
  FOREIGN KEY (project_id, evidence_capture_receipt_id)
    REFERENCES evidence_capture_receipts (project_id, evidence_capture_receipt_id),
  FOREIGN KEY (
    project_id,
    evidence_capture_intent_id,
    evidence_capture_receipt_id
  ) REFERENCES evidence_capture_receipts (
    project_id,
    evidence_capture_intent_id,
    evidence_capture_receipt_id
  ),
  FOREIGN KEY (project_id, evidence_observation_id)
    REFERENCES evidence_observations (project_id, evidence_observation_id),
  FOREIGN KEY (project_id, artifact_id) REFERENCES artifacts (project_id, artifact_id),
  FOREIGN KEY (project_id, run_id) REFERENCES runs (project_id, run_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
);

CREATE TABLE blockers (
  project_id TEXT NOT NULL,
  blocker_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  status TEXT NOT NULL CHECK (status IN ('active', 'resolved', 'superseded')),
  category TEXT NOT NULL,
  code TEXT NOT NULL,
  owner_refs_json TEXT NOT NULL DEFAULT '[]',
  related_refs_json TEXT NOT NULL DEFAULT '[]',
  detail_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  resolved_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, blocker_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
);

CREATE TABLE authority_events (
  project_id TEXT NOT NULL,
  event_seq INTEGER NOT NULL CHECK (event_seq > 0),
  event_id TEXT NOT NULL,
  state_version INTEGER NOT NULL CHECK (state_version > 0),
  event_type TEXT NOT NULL,
  actor_source TEXT NOT NULL,
  operation_category TEXT NOT NULL CHECK (operation_category IN ('read', 'agent_workflow', 'user_only', 'admin_local', 'local_recovery')),
  task_id TEXT,
  change_unit_id TEXT,
  payload_json TEXT NOT NULL DEFAULT '{}',
  request_hash TEXT NOT NULL,
  previous_event_hash TEXT,
  event_hash TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (project_id, event_seq),
  UNIQUE (project_id, event_id),
  UNIQUE (project_id, event_hash),
  CHECK (length(trim(event_hash)) > 0),
  CHECK (previous_event_hash IS NULL OR length(trim(previous_event_hash)) > 0),
  CHECK (
    (event_type = 'project_workflow_policy_applied'
      AND task_id IS NULL AND change_unit_id IS NULL)
    OR (event_type <> 'project_workflow_policy_applied' AND task_id IS NOT NULL)
  ),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, previous_event_hash)
    REFERENCES authority_events (project_id, event_hash)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE tool_invocations (
  project_id TEXT NOT NULL,
  tool_name TEXT NOT NULL,
  idempotency_key TEXT NOT NULL,
  request_hash TEXT NOT NULL,
  basis_state_version INTEGER NOT NULL CHECK (basis_state_version >= 0),
  committed_state_version INTEGER NOT NULL CHECK (committed_state_version > basis_state_version),
  status TEXT NOT NULL DEFAULT 'committed' CHECK (status = 'committed'),
  actor_source TEXT NOT NULL,
  operation_category TEXT NOT NULL CHECK (operation_category IN ('read', 'agent_workflow', 'user_only', 'admin_local', 'local_recovery')),
  verification_basis TEXT,
  git_workspace_context_json TEXT,
  response_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (project_id, tool_name, idempotency_key),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id)
);

CREATE INDEX idx_project_state_active_task
  ON project_state (project_id, active_task_id);

CREATE INDEX idx_tasks_lifecycle
  ON tasks (project_id, lifecycle_phase, result);

CREATE INDEX idx_tasks_current_change_unit
  ON tasks (project_id, current_change_unit_id);

CREATE INDEX idx_acceptance_criteria_task_status
  ON acceptance_criteria (project_id, task_id, status, position);

CREATE INDEX idx_evidence_claims_task
  ON evidence_claims (project_id, task_id);

CREATE INDEX idx_change_units_task_status
  ON change_units (project_id, task_id, status);

CREATE INDEX idx_evidence_capture_intents_task_expiry
  ON evidence_capture_intents (project_id, task_id, expires_at);

CREATE INDEX idx_evidence_capture_intents_connection_expiry
  ON evidence_capture_intents (
    project_id,
    requesting_connection_internal_id,
    expires_at
  );

CREATE INDEX idx_user_action_requests_task_basis_expiry
  ON user_action_requests (project_id, task_id, basis_status, expires_at);
CREATE INDEX idx_user_action_requests_task_kind
  ON user_action_requests (project_id, task_id, action_kind, requested_at);
CREATE INDEX idx_user_action_resolutions_request
  ON user_action_resolutions (project_id, user_action_request_id);

CREATE UNIQUE INDEX idx_user_action_requests_direct_origin
  ON user_action_requests (project_id, source_idempotency_key)
  WHERE source_method = 'volicord.request_user_action';

CREATE INDEX idx_project_continuity_records_status
  ON project_continuity_records (project_id, status, kind, updated_at);

CREATE INDEX idx_project_continuity_records_source_task
  ON project_continuity_records (project_id, source_task_id);

CREATE INDEX idx_write_tickets_task_status
  ON write_tickets (project_id, task_id, status);

CREATE INDEX idx_runs_task_created
  ON runs (project_id, task_id, created_at);

CREATE INDEX idx_artifact_staging_task_status
  ON artifact_staging (project_id, task_id, status);

CREATE INDEX idx_artifact_staging_actor_source
  ON artifact_staging (project_id, created_by_actor_source);

CREATE INDEX idx_evidence_capture_receipts_created
  ON evidence_capture_receipts (project_id, created_at);

CREATE INDEX idx_evidence_capture_source_claims_receipt
  ON evidence_capture_source_claims (
    project_id,
    evidence_capture_receipt_id,
    source_claim_kind,
    source_claim_id
  );

CREATE INDEX idx_artifacts_task_status
  ON artifacts (project_id, task_id, status);

CREATE INDEX idx_artifact_links_owner
  ON artifact_links (project_id, owner_record_kind, owner_record_id);

CREATE INDEX idx_evidence_summaries_task_status
  ON evidence_summaries (project_id, task_id, status);

CREATE INDEX idx_evidence_observations_task_target
  ON evidence_observations (
    project_id,
    task_id,
    acceptance_criterion_id,
    evidence_claim_id
  );

CREATE INDEX idx_evidence_observations_run
  ON evidence_observations (project_id, run_id);
CREATE INDEX idx_evidence_producers_task_run
  ON evidence_producers (project_id, task_id, run_id);

CREATE INDEX idx_blockers_task_status
  ON blockers (project_id, task_id, status);

CREATE INDEX idx_authority_events_task_seq
  ON authority_events (project_id, task_id, event_seq);
CREATE INDEX idx_authority_events_state_version
  ON authority_events (project_id, state_version, event_seq);
CREATE INDEX idx_authority_events_hash_chain
  ON authority_events (project_id, previous_event_hash, event_hash);
CREATE TABLE host_sessions (
  project_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  project_integration_revision TEXT NOT NULL CHECK (
    length(project_integration_revision) = 71
    AND substr(project_integration_revision, 1, 7) = 'sha256:'
    AND substr(project_integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  host_session_id TEXT NOT NULL CHECK (length(trim(host_session_id)) > 0),
  first_observed_at TEXT NOT NULL,
  last_observed_at TEXT NOT NULL,
  PRIMARY KEY (project_id, session_id),
  UNIQUE (project_id, session_id, connection_internal_id),
  CHECK (last_observed_at >= first_observed_at),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id)
);

CREATE TRIGGER host_sessions_project_integration_revision_immutable
BEFORE UPDATE OF project_integration_revision ON host_sessions
BEGIN
  SELECT RAISE(ABORT, 'host_sessions.project_integration_revision is immutable');
END;

CREATE TABLE host_turns (
  project_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  host_turn_id TEXT NOT NULL CHECK (length(trim(host_turn_id)) > 0),
  first_observed_at TEXT NOT NULL,
  last_observed_at TEXT NOT NULL,
  PRIMARY KEY (project_id, session_id, host_turn_id),
  UNIQUE (project_id, session_id, connection_internal_id, host_turn_id),
  CHECK (last_observed_at >= first_observed_at),
  FOREIGN KEY (project_id, session_id, connection_internal_id)
    REFERENCES host_sessions (project_id, session_id, connection_internal_id)
);

CREATE TABLE host_tool_invocations (
  project_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  host_turn_id TEXT NOT NULL,
  host_tool_use_id TEXT NOT NULL CHECK (length(trim(host_tool_use_id)) > 0),
  host_tool_name TEXT NOT NULL CHECK (length(trim(host_tool_name)) > 0),
  first_observed_at TEXT NOT NULL,
  last_observed_at TEXT NOT NULL,
  PRIMARY KEY (project_id, session_id, host_tool_use_id),
  UNIQUE (
    project_id, session_id, connection_internal_id, host_turn_id,
    host_tool_use_id, host_tool_name
  ),
  CHECK (last_observed_at >= first_observed_at),
  FOREIGN KEY (project_id, session_id, connection_internal_id, host_turn_id)
    REFERENCES host_turns (project_id, session_id, connection_internal_id, host_turn_id)
);

CREATE TABLE managed_mcp_sessions (
  project_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  runtime_session_id TEXT CHECK (
    runtime_session_id IS NULL OR length(trim(runtime_session_id)) > 0
  ),
  connection_internal_id TEXT NOT NULL,
  host_thread_id TEXT NOT NULL CHECK (length(trim(host_thread_id)) > 0),
  last_host_turn_id TEXT NOT NULL CHECK (length(trim(last_host_turn_id)) > 0),
  first_observed_at TEXT NOT NULL,
  last_observed_at TEXT NOT NULL,
  PRIMARY KEY (project_id, session_id),
  CHECK (last_observed_at >= first_observed_at),
  FOREIGN KEY (project_id, session_id, connection_internal_id)
    REFERENCES host_sessions (project_id, session_id, connection_internal_id),
  FOREIGN KEY (project_id, session_id, connection_internal_id, last_host_turn_id)
    REFERENCES host_turns (project_id, session_id, connection_internal_id, host_turn_id)
);

CREATE TABLE guard_events (
  project_id TEXT NOT NULL,
  guard_event_id TEXT NOT NULL,
  session_id TEXT,
  connection_internal_id TEXT NOT NULL,
  correlation_kind TEXT CHECK (
    correlation_kind IN ('codex_hook_prompt', 'codex_hook_tool')
  ),
  host_turn_id TEXT,
  host_tool_use_id TEXT,
  host_tool_name TEXT,
  guard_installation_id TEXT NOT NULL,
  policy_hash TEXT NOT NULL CHECK (
    length(policy_hash) = 71
    AND substr(policy_hash, 1, 7) = 'sha256:'
    AND substr(policy_hash, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  integration_revision TEXT NOT NULL CHECK (
    length(integration_revision) = 71
    AND substr(integration_revision, 1, 7) = 'sha256:'
    AND substr(integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  event_kind TEXT NOT NULL CHECK (event_kind IN ('pre_tool', 'post_tool', 'prompt_capture')),
  contract_status TEXT NOT NULL CHECK (contract_status IN ('compatible', 'malformed', 'incompatible')),
  decision TEXT NOT NULL CHECK (decision IN ('allow', 'deny', 'warn', 'inject_context')),
  subject_json TEXT NOT NULL DEFAULT '{}',
  result_json TEXT NOT NULL DEFAULT '{}',
  occurred_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, guard_event_id),
  UNIQUE (
    project_id, guard_event_id, session_id, connection_internal_id,
    host_turn_id, host_tool_use_id, host_tool_name
  ),
  CHECK (
    (
      correlation_kind IS NULL
      AND session_id IS NULL
      AND host_turn_id IS NULL
      AND host_tool_use_id IS NULL
      AND host_tool_name IS NULL
    )
    OR (
      correlation_kind = 'codex_hook_prompt'
      AND event_kind = 'prompt_capture'
      AND session_id IS NOT NULL
      AND host_turn_id IS NOT NULL
      AND host_tool_use_id IS NULL
      AND host_tool_name IS NULL
    )
    OR (
      correlation_kind = 'codex_hook_tool'
      AND event_kind IN ('pre_tool', 'post_tool')
      AND session_id IS NOT NULL
      AND host_turn_id IS NOT NULL
      AND host_tool_use_id IS NOT NULL
      AND host_tool_name IS NOT NULL
    )
  ),
  CHECK (contract_status != 'compatible' OR correlation_kind IS NOT NULL),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, session_id, connection_internal_id, host_turn_id)
    REFERENCES host_turns (project_id, session_id, connection_internal_id, host_turn_id),
  FOREIGN KEY (
    project_id, session_id, connection_internal_id, host_turn_id,
    host_tool_use_id, host_tool_name
  ) REFERENCES host_tool_invocations (
    project_id, session_id, connection_internal_id, host_turn_id,
    host_tool_use_id, host_tool_name
  )
);

CREATE TABLE prompt_captures (
  project_id TEXT NOT NULL,
  prompt_capture_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  host_turn_id TEXT NOT NULL,
  capture_kind TEXT NOT NULL,
  prompt_sha256 TEXT NOT NULL,
  prompt_text TEXT,
  captured_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, prompt_capture_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, session_id, connection_internal_id, host_turn_id)
    REFERENCES host_turns (project_id, session_id, connection_internal_id, host_turn_id)
);

CREATE TABLE unrecorded_changes (
  project_id TEXT NOT NULL,
  unrecorded_change_id TEXT NOT NULL,
  repository_observation_id TEXT NOT NULL,
  task_id TEXT,
  status TEXT NOT NULL CHECK (status IN ('unresolved', 'resolved')),
  summary TEXT NOT NULL CHECK (length(trim(summary)) > 0),
  observed_paths_json TEXT NOT NULL,
  unmatched_delta_digest TEXT NOT NULL CHECK (
    length(unmatched_delta_digest) = 71
    AND substr(unmatched_delta_digest, 1, 7) = 'sha256:'
    AND substr(unmatched_delta_digest, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  detection_json TEXT NOT NULL DEFAULT '{}',
  resolution_json TEXT,
  detected_at TEXT NOT NULL,
  resolved_at TEXT,
  resolved_by_actor_source TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, unrecorded_change_id),
  UNIQUE (project_id, repository_observation_id, unmatched_delta_digest),
  CHECK (observed_paths_json != '[]'),
  CHECK (
    (
      status = 'unresolved'
      AND resolution_json IS NULL
      AND resolved_at IS NULL
      AND resolved_by_actor_source IS NULL
    )
    OR (
      status = 'resolved'
      AND resolution_json IS NOT NULL
      AND resolved_at IS NOT NULL
      AND resolved_by_actor_source IS NOT NULL
    )
  ),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, repository_observation_id)
    REFERENCES repository_observations (project_id, repository_observation_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id)
);

CREATE INDEX idx_unrecorded_changes_status
  ON unrecorded_changes (project_id, status, detected_at);
CREATE INDEX idx_unrecorded_changes_observation
  ON unrecorded_changes (project_id, repository_observation_id);
CREATE INDEX idx_unrecorded_changes_task
  ON unrecorded_changes (project_id, task_id, status);
CREATE TABLE expected_writes (
  project_id TEXT NOT NULL,
  expected_write_id TEXT NOT NULL,
  repository_observation_id TEXT NOT NULL,
  command_kind TEXT NOT NULL CHECK (length(trim(command_kind)) > 0),
  path_policy TEXT NOT NULL CHECK (path_policy IN ('exact_paths')),
  expected_paths_json TEXT NOT NULL CHECK (expected_paths_json != '[]'),
  task_id TEXT NOT NULL,
  change_unit_id TEXT NOT NULL,
  write_ticket_ids_json TEXT NOT NULL CHECK (write_ticket_ids_json != '[]'),
  basis_state_version INTEGER NOT NULL CHECK (basis_state_version >= 0),
  status TEXT NOT NULL CHECK (status IN ('pending', 'matched')),
  matched_paths_json TEXT,
  created_at TEXT NOT NULL,
  matched_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, expected_write_id),
  UNIQUE (project_id, repository_observation_id),
  CHECK (
    (
      status = 'pending'
      AND matched_paths_json IS NULL
      AND matched_at IS NULL
    )
    OR (
      status = 'matched'
      AND matched_paths_json IS NOT NULL
      AND matched_paths_json != '[]'
      AND matched_at IS NOT NULL
    )
  ),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, repository_observation_id)
    REFERENCES repository_observations (project_id, repository_observation_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id)
);

CREATE INDEX idx_expected_writes_observation
  ON expected_writes (project_id, repository_observation_id, status);
CREATE INDEX idx_expected_writes_task
  ON expected_writes (project_id, task_id, status);
CREATE TABLE repository_observations (
  project_id TEXT NOT NULL,
  repository_observation_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  host_turn_id TEXT NOT NULL,
  host_tool_use_id TEXT NOT NULL,
  host_tool_name TEXT NOT NULL,
  guard_installation_id TEXT NOT NULL,
  observer_contract_digest TEXT NOT NULL CHECK (
    length(observer_contract_digest) = 71
    AND substr(observer_contract_digest, 1, 7) = 'sha256:'
    AND substr(observer_contract_digest, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  pre_tool_guard_event_id TEXT,
  post_tool_guard_event_id TEXT,
  state TEXT NOT NULL CHECK (state IN ('open', 'complete', 'unavailable')),
  pre_snapshot_json TEXT,
  pre_snapshot_digest TEXT CHECK (
    pre_snapshot_digest IS NULL
    OR (
      length(pre_snapshot_digest) = 71
      AND substr(pre_snapshot_digest, 1, 7) = 'sha256:'
      AND substr(pre_snapshot_digest, 8) NOT GLOB '*[^0-9a-f]*'
    )
  ),
  post_snapshot_json TEXT,
  post_snapshot_digest TEXT CHECK (
    post_snapshot_digest IS NULL
    OR (
      length(post_snapshot_digest) = 71
      AND substr(post_snapshot_digest, 1, 7) = 'sha256:'
      AND substr(post_snapshot_digest, 8) NOT GLOB '*[^0-9a-f]*'
    )
  ),
  delta_json TEXT,
  delta_digest TEXT CHECK (
    delta_digest IS NULL
    OR (
      length(delta_digest) = 71
      AND substr(delta_digest, 1, 7) = 'sha256:'
      AND substr(delta_digest, 8) NOT GLOB '*[^0-9a-f]*'
    )
  ),
  unavailable_reason TEXT CHECK (
    unavailable_reason IS NULL OR unavailable_reason IN (
      'invalid_observer_limits',
      'invalid_repository_root',
      'not_git_repository',
      'git_layout_unavailable',
      'git_command_unavailable',
      'git_command_failed',
      'process_timeout',
      'git_output_limit_exceeded',
      'process_input_limit_exceeded',
      'candidate_path_limit_exceeded',
      'total_hash_bytes_limit_exceeded',
      'file_size_limit_exceeded',
      'serialization_depth_limit_exceeded',
      'serialization_size_limit_exceeded',
      'invalid_relative_path',
      'non_utf8_path',
      'path_outside_repository',
      'inaccessible_path',
      'unsupported_path_state',
      'unstable_repository',
      'repository_identity_changed',
      'observer_contract_mismatch',
      'git_object_unavailable',
      'invocation_denied',
      'missing_open_observation',
      'post_tool_not_observed',
      'managed_session_terminated'
    )
  ),
  started_at TEXT NOT NULL,
  completed_at TEXT,
  terminal_result_json TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, repository_observation_id),
  UNIQUE (
    project_id, session_id, connection_internal_id, host_turn_id,
    host_tool_use_id, host_tool_name
  ),
  CHECK (
    (
      state = 'open'
      AND pre_tool_guard_event_id IS NOT NULL
      AND pre_snapshot_json IS NOT NULL
      AND pre_snapshot_digest IS NOT NULL
      AND post_tool_guard_event_id IS NULL
      AND post_snapshot_json IS NULL
      AND post_snapshot_digest IS NULL
      AND delta_json IS NULL
      AND delta_digest IS NULL
      AND unavailable_reason IS NULL
      AND completed_at IS NULL
      AND terminal_result_json IS NULL
    )
    OR (
      state = 'complete'
      AND pre_tool_guard_event_id IS NOT NULL
      AND pre_snapshot_json IS NOT NULL
      AND pre_snapshot_digest IS NOT NULL
      AND post_tool_guard_event_id IS NOT NULL
      AND post_snapshot_json IS NOT NULL
      AND post_snapshot_digest IS NOT NULL
      AND delta_json IS NOT NULL
      AND delta_digest IS NOT NULL
      AND unavailable_reason IS NULL
      AND completed_at IS NOT NULL
      AND terminal_result_json IS NOT NULL
    )
    OR (
      state = 'unavailable'
      AND ((pre_snapshot_json IS NULL AND pre_snapshot_digest IS NULL)
        OR (pre_snapshot_json IS NOT NULL AND pre_snapshot_digest IS NOT NULL))
      AND (pre_snapshot_json IS NULL OR pre_tool_guard_event_id IS NOT NULL)
      AND post_snapshot_json IS NULL
      AND post_snapshot_digest IS NULL
      AND delta_json IS NULL
      AND delta_digest IS NULL
      AND unavailable_reason IS NOT NULL
      AND completed_at IS NOT NULL
      AND terminal_result_json IS NOT NULL
    )
  ),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (
    project_id, session_id, connection_internal_id, host_turn_id,
    host_tool_use_id, host_tool_name
  ) REFERENCES host_tool_invocations (
    project_id, session_id, connection_internal_id, host_turn_id,
    host_tool_use_id, host_tool_name
  ),
  FOREIGN KEY (
    project_id, pre_tool_guard_event_id, session_id, connection_internal_id,
    host_turn_id, host_tool_use_id, host_tool_name
  ) REFERENCES guard_events (
    project_id, guard_event_id, session_id, connection_internal_id,
    host_turn_id, host_tool_use_id, host_tool_name
  ),
  FOREIGN KEY (
    project_id, post_tool_guard_event_id, session_id, connection_internal_id,
    host_turn_id, host_tool_use_id, host_tool_name
  ) REFERENCES guard_events (
    project_id, guard_event_id, session_id, connection_internal_id,
    host_turn_id, host_tool_use_id, host_tool_name
  )
);

CREATE INDEX idx_host_sessions_connection
  ON host_sessions (project_id, connection_internal_id, last_observed_at);
CREATE INDEX idx_host_turns_session
  ON host_turns (project_id, session_id, last_observed_at);
CREATE INDEX idx_host_tool_invocations_session
  ON host_tool_invocations (project_id, session_id, last_observed_at);
CREATE INDEX idx_managed_mcp_sessions_runtime
  ON managed_mcp_sessions (project_id, runtime_session_id, last_observed_at);
CREATE UNIQUE INDEX idx_managed_mcp_sessions_runtime_binding
  ON managed_mcp_sessions (project_id, runtime_session_id)
  WHERE runtime_session_id IS NOT NULL;
CREATE INDEX idx_guard_events_session
  ON guard_events (project_id, session_id, occurred_at);
CREATE INDEX idx_guard_events_connection
  ON guard_events (project_id, connection_internal_id, occurred_at);
CREATE INDEX idx_guard_events_decision
  ON guard_events (project_id, decision, occurred_at);
CREATE INDEX idx_prompt_captures_session
  ON prompt_captures (project_id, session_id, captured_at);
CREATE INDEX idx_prompt_captures_connection
  ON prompt_captures (project_id, connection_internal_id, captured_at);
CREATE INDEX idx_repository_observations_state
  ON repository_observations (project_id, state, started_at);
CREATE INDEX idx_repository_observations_connection
  ON repository_observations (project_id, connection_internal_id, state);
CREATE TABLE project_workflow_policies (
  project_id TEXT PRIMARY KEY,
  policy_schema TEXT NOT NULL CHECK (policy_schema = 'volicord.workflow_policy'),
  policy_version INTEGER NOT NULL CHECK (policy_version > 0),
  policy_json TEXT NOT NULL,
  policy_fingerprint TEXT NOT NULL CHECK (
    length(policy_fingerprint) = 71
    AND substr(policy_fingerprint, 1, 7) = 'sha256:'
    AND substr(policy_fingerprint, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  source TEXT NOT NULL CHECK (length(trim(source)) > 0),
  applied_at TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES project_state (project_id)
);
