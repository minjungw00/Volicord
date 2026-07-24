CREATE TABLE runtime_home (
  singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
  runtime_home_id TEXT NOT NULL UNIQUE,
  runtime_home_path TEXT NOT NULL UNIQUE,
  registry_db_path TEXT NOT NULL UNIQUE,
  storage_profile TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE installation_profile (
  installation_id TEXT PRIMARY KEY,
  runtime_home_id TEXT NOT NULL UNIQUE,
  volicord_command TEXT NOT NULL,
  volicord_mcp_command TEXT NOT NULL,
  bin_dir TEXT NOT NULL,
  default_connection_mode TEXT NOT NULL CHECK (default_connection_mode IN ('read_only', 'workflow')),
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (runtime_home_id) REFERENCES runtime_home (runtime_home_id) ON DELETE RESTRICT
);

CREATE TABLE projects (
  project_internal_id TEXT PRIMARY KEY,
  project_name TEXT NOT NULL,
  project_alias TEXT NOT NULL UNIQUE,
  runtime_home_id TEXT NOT NULL,
  repo_root TEXT NOT NULL UNIQUE,
  project_home TEXT NOT NULL UNIQUE,
  state_db_path TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL DEFAULT 'active' CHECK (status = 'active'),
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (runtime_home_id) REFERENCES runtime_home (runtime_home_id)
);

CREATE TABLE project_aliases (
  alias TEXT PRIMARY KEY,
  project_internal_id TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (project_internal_id)
    REFERENCES projects (project_internal_id)
    ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_projects_repo_root ON projects (repo_root);
CREATE INDEX idx_projects_status ON projects (status);
CREATE INDEX idx_project_aliases_project
  ON project_aliases (project_internal_id);

CREATE TABLE agent_connections (
  connection_internal_id TEXT PRIMARY KEY,
  integration_instance_id TEXT NOT NULL CHECK (
    length(integration_instance_id) = 56
    AND substr(integration_instance_id, 1, 20) = 'connection_instance_'
    AND substr(integration_instance_id, 29, 1) = '-'
    AND substr(integration_instance_id, 34, 1) = '-'
    AND substr(integration_instance_id, 39, 1) = '-'
    AND substr(integration_instance_id, 44, 1) = '-'
    AND substr(integration_instance_id, 21, 8) NOT GLOB '*[^0-9a-f]*'
    AND substr(integration_instance_id, 30, 4) NOT GLOB '*[^0-9a-f]*'
    AND substr(integration_instance_id, 35, 4) NOT GLOB '*[^0-9a-f]*'
    AND substr(integration_instance_id, 40, 4) NOT GLOB '*[^0-9a-f]*'
    AND substr(integration_instance_id, 45, 12) NOT GLOB '*[^0-9a-f]*'
    AND substr(integration_instance_id, 35, 1) = '4'
    AND substr(integration_instance_id, 40, 1) GLOB '[89ab]'
  ),
  host_kind TEXT NOT NULL CHECK (host_kind = 'codex'),
  intent TEXT NOT NULL CHECK (intent IN ('personal', 'shared')),
  host_scope TEXT NOT NULL CHECK (host_scope IN ('user', 'project')),
  project_internal_id TEXT,
  server_name TEXT NOT NULL,
  config_target TEXT NOT NULL,
  mode TEXT NOT NULL CHECK (mode IN ('read_only', 'workflow')),
  enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  managed_fingerprint TEXT NOT NULL,
  integration_generation INTEGER NOT NULL DEFAULT 0 CHECK (integration_generation >= 0),
  verification_report_json TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_internal_id) REFERENCES projects (project_internal_id) ON DELETE RESTRICT,
  CHECK (host_kind = 'codex' AND host_scope IN ('user', 'project'))
);

CREATE TABLE connection_projects (
  connection_internal_id TEXT NOT NULL,
  project_internal_id TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (connection_internal_id, project_internal_id),
  FOREIGN KEY (connection_internal_id)
    REFERENCES agent_connections (connection_internal_id)
    ON DELETE RESTRICT
    DEFERRABLE INITIALLY DEFERRED,
  FOREIGN KEY (project_internal_id) REFERENCES projects (project_internal_id) ON DELETE RESTRICT
);

CREATE INDEX idx_connection_projects_project
  ON connection_projects (project_internal_id);
CREATE INDEX idx_agent_connections_enabled
  ON agent_connections (enabled);
CREATE INDEX idx_agent_connections_project
  ON agent_connections (project_internal_id);
CREATE UNIQUE INDEX idx_agent_connections_integration_instance
  ON agent_connections (integration_instance_id);
CREATE UNIQUE INDEX idx_agent_connections_target_project
  ON agent_connections (
    host_kind,
    intent,
    host_scope,
    project_internal_id,
    config_target,
    server_name
  )
  WHERE project_internal_id IS NOT NULL;
CREATE UNIQUE INDEX idx_agent_connections_target_unscoped
  ON agent_connections (
    host_kind,
    intent,
    host_scope,
    config_target,
    server_name
  )
  WHERE project_internal_id IS NULL;

CREATE TRIGGER agent_connections_integration_instance_immutable
BEFORE UPDATE OF integration_instance_id ON agent_connections
BEGIN
  SELECT RAISE(ABORT, 'agent_connections.integration_instance_id is immutable');
END;

CREATE TABLE diagnostic_findings (
  finding_id TEXT PRIMARY KEY CHECK (
    length(CAST(finding_id AS BLOB)) BETWEEN 1 AND 192
    AND substr(finding_id, 1, 1) GLOB '[a-z]'
    AND substr(finding_id, -1, 1) GLOB '[a-z0-9]'
    AND finding_id NOT GLOB '*[^a-z0-9_.:-]*'
  ),
  lifecycle TEXT NOT NULL CHECK (lifecycle IN ('occurrence', 'current_state')),
  current_identity_digest TEXT CHECK (
    current_identity_digest IS NULL
    OR (
      length(current_identity_digest) = 64
      AND current_identity_digest NOT GLOB '*[^0-9a-f]*'
    )
  ),
  current_subject_identity TEXT CHECK (
    current_subject_identity IS NULL
    OR (
      length(current_subject_identity) = 71
      AND substr(current_subject_identity, 1, 7) = 'sha256:'
      AND substr(current_subject_identity, 8) NOT GLOB '*[^0-9a-f]*'
    )
  ),
  diagnostic_scope_kind TEXT CHECK (
    diagnostic_scope_kind IS NULL
    OR diagnostic_scope_kind IN ('connection', 'project', 'runtime_home', 'installation', 'process')
  ),
  diagnostic_scope_identity TEXT CHECK (
    diagnostic_scope_identity IS NULL
    OR length(CAST(diagnostic_scope_identity AS BLOB)) BETWEEN 1 AND 1024
  ),
  current_state_status TEXT CHECK (
    current_state_status IS NULL
    OR current_state_status IN ('active', 'resolved')
  ),
  resolved_at TEXT,
  code TEXT NOT NULL CHECK (
    length(CAST(code AS BLOB)) BETWEEN 3 AND 192
    AND instr(code, '.') > 1
    AND code NOT GLOB '*[^a-z0-9_.]*'
  ),
  domain TEXT NOT NULL CHECK (
    length(CAST(domain AS BLOB)) BETWEEN 1 AND 128
    AND substr(domain, 1, 1) GLOB '[a-z]'
    AND domain NOT GLOB '*[^a-z0-9_]*'
  ),
  stage TEXT NOT NULL CHECK (
    length(CAST(stage AS BLOB)) BETWEEN 1 AND 128
    AND substr(stage, 1, 1) GLOB '[a-z]'
    AND stage NOT GLOB '*[^a-z0-9_]*'
  ),
  severity TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'error')),
  source TEXT NOT NULL CHECK (
    length(CAST(source AS BLOB)) BETWEEN 1 AND 128
    AND substr(source, 1, 1) GLOB '[a-z]'
    AND source NOT GLOB '*[^a-z0-9_]*'
  ),
  subject_json TEXT NOT NULL CHECK (
    json_valid(subject_json)
    AND json_type(subject_json) = 'object'
    AND length(CAST(subject_json AS BLOB)) <= 4096
  ),
  facts_json TEXT NOT NULL CHECK (
    json_valid(facts_json)
    AND json_type(facts_json) = 'object'
    AND length(CAST(facts_json AS BLOB)) <= 16384
  ),
  actions_json TEXT NOT NULL CHECK (
    json_valid(actions_json)
    AND json_type(actions_json) = 'array'
    AND length(CAST(actions_json AS BLOB)) <= 65536
  ),
  correlation_id TEXT CHECK (
    correlation_id IS NULL
    OR length(CAST(correlation_id AS BLOB)) BETWEEN 1 AND 192
  ),
  connection_internal_id TEXT CHECK (
    connection_internal_id IS NULL
    OR length(CAST(connection_internal_id AS BLOB)) BETWEEN 1 AND 192
  ),
  project_internal_id TEXT CHECK (
    project_internal_id IS NULL
    OR length(CAST(project_internal_id AS BLOB)) BETWEEN 1 AND 192
  ),
  runtime_session_id TEXT CHECK (
    runtime_session_id IS NULL
    OR length(CAST(runtime_session_id AS BLOB)) BETWEEN 1 AND 192
  ),
  integration_revision TEXT CHECK (
    integration_revision IS NULL
    OR (
      length(integration_revision) = 71
      AND substr(integration_revision, 1, 7) = 'sha256:'
      AND substr(integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
    )
  ),
  observed_at TEXT NOT NULL,
  UNIQUE (finding_id, runtime_session_id),
  CHECK (
    runtime_session_id IS NULL
    OR (connection_internal_id IS NOT NULL AND integration_revision IS NOT NULL)
  ),
  CHECK (
    (
      lifecycle = 'occurrence'
      AND current_identity_digest IS NULL
      AND current_subject_identity IS NULL
      AND diagnostic_scope_kind IS NULL
      AND diagnostic_scope_identity IS NULL
      AND current_state_status IS NULL
      AND resolved_at IS NULL
    )
    OR (
      lifecycle = 'current_state'
      AND current_identity_digest IS NOT NULL
      AND current_subject_identity IS NOT NULL
      AND diagnostic_scope_kind IS NOT NULL
      AND diagnostic_scope_identity IS NOT NULL
      AND current_state_status IS NOT NULL
      AND runtime_session_id IS NULL
      AND finding_id = 'finding.current.sha256:' || current_identity_digest
      AND (
        (current_state_status = 'active' AND resolved_at IS NULL)
        OR (current_state_status = 'resolved' AND resolved_at IS NOT NULL)
      )
    )
  )
);

CREATE TABLE diagnostic_cause_edges (
  finding_id TEXT NOT NULL,
  cause_finding_id TEXT NOT NULL,
  PRIMARY KEY (finding_id, cause_finding_id),
  FOREIGN KEY (finding_id)
    REFERENCES diagnostic_findings (finding_id)
    ON DELETE CASCADE,
  FOREIGN KEY (cause_finding_id)
    REFERENCES diagnostic_findings (finding_id)
    ON DELETE RESTRICT,
  CHECK (finding_id <> cause_finding_id)
);

CREATE INDEX idx_diagnostic_findings_runtime_session
  ON diagnostic_findings (runtime_session_id, observed_at, finding_id)
  WHERE lifecycle = 'occurrence' AND runtime_session_id IS NOT NULL;
CREATE UNIQUE INDEX idx_diagnostic_findings_current_identity
  ON diagnostic_findings (current_identity_digest)
  WHERE lifecycle = 'current_state';
CREATE INDEX idx_diagnostic_findings_active_current_scope
  ON diagnostic_findings (
    diagnostic_scope_kind, diagnostic_scope_identity, observed_at, finding_id
  )
  WHERE lifecycle = 'current_state' AND current_state_status = 'active';
CREATE INDEX idx_diagnostic_findings_project
  ON diagnostic_findings (project_internal_id, observed_at, finding_id)
  WHERE project_internal_id IS NOT NULL;
CREATE INDEX idx_diagnostic_cause_edges_cause
  ON diagnostic_cause_edges (cause_finding_id, finding_id);

CREATE TRIGGER diagnostic_cause_edges_acyclic
BEFORE INSERT ON diagnostic_cause_edges
BEGIN
  SELECT CASE WHEN EXISTS (
    WITH RECURSIVE causes(finding_id) AS (
      SELECT cause_finding_id
        FROM diagnostic_cause_edges
       WHERE finding_id = NEW.cause_finding_id
      UNION
      SELECT edge.cause_finding_id
        FROM diagnostic_cause_edges AS edge
        JOIN causes ON edge.finding_id = causes.finding_id
    )
    SELECT 1 FROM causes WHERE finding_id = NEW.finding_id
  ) THEN RAISE(ABORT, 'diagnostic cause cycle') END;
END;

CREATE TRIGGER diagnostic_occurrence_findings_immutable
BEFORE UPDATE ON diagnostic_findings
WHEN OLD.lifecycle = 'occurrence'
BEGIN
  SELECT RAISE(ABORT, 'diagnostic occurrence findings are immutable');
END;

CREATE TRIGGER diagnostic_current_identity_immutable
BEFORE UPDATE OF
  finding_id,
  lifecycle,
  current_identity_digest,
  current_subject_identity,
  diagnostic_scope_kind,
  diagnostic_scope_identity,
  code,
  domain,
  stage,
  source
ON diagnostic_findings
WHEN OLD.lifecycle = 'current_state'
BEGIN
  SELECT RAISE(ABORT, 'diagnostic current identity is immutable');
END;

CREATE TABLE managed_mcp_launch_leases (
  launch_lease_id TEXT PRIMARY KEY CHECK (
    length(launch_lease_id) = 53
    AND substr(launch_lease_id, 1, 17) = 'mcp_launch_lease_'
    AND substr(launch_lease_id, 26, 1) = '-'
    AND substr(launch_lease_id, 31, 1) = '-'
    AND substr(launch_lease_id, 36, 1) = '-'
    AND substr(launch_lease_id, 41, 1) = '-'
    AND substr(launch_lease_id, 18, 8) NOT GLOB '*[^0-9a-f]*'
    AND substr(launch_lease_id, 27, 4) NOT GLOB '*[^0-9a-f]*'
    AND substr(launch_lease_id, 32, 4) NOT GLOB '*[^0-9a-f]*'
    AND substr(launch_lease_id, 37, 4) NOT GLOB '*[^0-9a-f]*'
    AND substr(launch_lease_id, 42, 12) NOT GLOB '*[^0-9a-f]*'
    AND substr(launch_lease_id, 32, 1) = '4'
    AND substr(launch_lease_id, 37, 1) GLOB '[89ab]'
  ),
  connection_internal_id TEXT NOT NULL,
  host_kind TEXT NOT NULL CHECK (host_kind = 'codex'),
  expected_integration_revision TEXT NOT NULL CHECK (
    length(expected_integration_revision) = 71
    AND substr(expected_integration_revision, 1, 7) = 'sha256:'
    AND substr(expected_integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  expected_launch_fingerprint TEXT NOT NULL CHECK (
    length(CAST(expected_launch_fingerprint AS BLOB)) BETWEEN 1 AND 1024
  ),
  issued_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  consumed_at TEXT,
  terminal_state TEXT NOT NULL CHECK (
    terminal_state IN ('issued', 'consumed', 'cancelled', 'expired')
  ),
  FOREIGN KEY (connection_internal_id)
    REFERENCES agent_connections (connection_internal_id)
    ON DELETE RESTRICT,
  CHECK (expires_at > issued_at),
  CHECK (
    (terminal_state = 'consumed' AND consumed_at IS NOT NULL)
    OR (terminal_state <> 'consumed' AND consumed_at IS NULL)
  ),
  CHECK (consumed_at IS NULL OR consumed_at >= issued_at),
  CHECK (consumed_at IS NULL OR consumed_at < expires_at)
);

CREATE INDEX idx_managed_mcp_launch_leases_cleanup
  ON managed_mcp_launch_leases (
    connection_internal_id, terminal_state, expires_at
  );


CREATE TABLE mcp_runtime_sessions (
  runtime_session_id TEXT PRIMARY KEY,
  connection_internal_id TEXT NOT NULL,
  session_source TEXT NOT NULL CHECK (
    session_source IN ('managed_host', 'manual_cli', 'cli_preflight', 'integration_probe')
  ),
  connection_integration_revision TEXT NOT NULL CHECK (
    length(connection_integration_revision) = 71
    AND substr(connection_integration_revision, 1, 7) = 'sha256:'
    AND substr(connection_integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  observed_host_executable_version TEXT,
  attempted_client_name TEXT,
  attempted_client_version TEXT,
  requested_protocol_version TEXT,
  selected_protocol_version TEXT,
  negotiated_protocol_version TEXT,
  process_id INTEGER NOT NULL CHECK (process_id > 0),
  process_started_at TEXT NOT NULL,
  initialize_completed_at TEXT,
  initialized_notification_at TEXT,
  tools_list_observed_at TEXT,
  returned_tool_identities_json TEXT CHECK (
    returned_tool_identities_json IS NULL
    OR (
      json_valid(returned_tool_identities_json)
      AND json_type(returned_tool_identities_json) = 'array'
    )
  ),
  required_tools_present INTEGER CHECK (required_tools_present IN (0, 1)),
  required_tools_validated_at TEXT,
  verification_tool_name TEXT CHECK (
    verification_tool_name IS NULL
    OR (
      length(CAST(verification_tool_name AS BLOB)) BETWEEN 1 AND 128
      AND length(verification_tool_name) = length(CAST(verification_tool_name AS BLOB))
      AND verification_tool_name NOT GLOB '*[^A-Za-z0-9_.-]*'
    )
  ),
  verification_tool_observed_at TEXT,
  last_observed_at TEXT NOT NULL,
  terminal_finding_id TEXT,
  graceful_close_at TEXT,
  UNIQUE (runtime_session_id, connection_internal_id),
  FOREIGN KEY (connection_internal_id)
    REFERENCES agent_connections (connection_internal_id)
    ON DELETE RESTRICT,
  FOREIGN KEY (terminal_finding_id, runtime_session_id)
    REFERENCES diagnostic_findings (finding_id, runtime_session_id)
    ON DELETE RESTRICT,
  CHECK (
    (attempted_client_name IS NULL AND attempted_client_version IS NULL)
    OR (attempted_client_name IS NOT NULL AND attempted_client_version IS NOT NULL)
  ),
  CHECK (
    (initialize_completed_at IS NULL AND selected_protocol_version IS NULL)
    OR (initialize_completed_at IS NOT NULL AND selected_protocol_version IS NOT NULL)
  ),
  CHECK (selected_protocol_version IS NULL OR requested_protocol_version IS NOT NULL),
  CHECK (selected_protocol_version IS NULL OR attempted_client_name IS NOT NULL),
  CHECK (
    (initialized_notification_at IS NULL AND negotiated_protocol_version IS NULL)
    OR (initialized_notification_at IS NOT NULL AND negotiated_protocol_version IS NOT NULL)
  ),
  CHECK (
    (
      tools_list_observed_at IS NULL
      AND returned_tool_identities_json IS NULL
      AND required_tools_present IS NULL
      AND required_tools_validated_at IS NULL
    )
    OR (
      tools_list_observed_at IS NOT NULL
      AND returned_tool_identities_json IS NOT NULL
      AND required_tools_present = 0
      AND required_tools_validated_at IS NULL
    )
    OR (
      tools_list_observed_at IS NOT NULL
      AND returned_tool_identities_json IS NOT NULL
      AND required_tools_present = 1
      AND required_tools_validated_at IS NOT NULL
    )
  ),
  CHECK (
    (verification_tool_name IS NULL AND verification_tool_observed_at IS NULL)
    OR (verification_tool_name IS NOT NULL AND verification_tool_observed_at IS NOT NULL)
  ),
  CHECK (initialized_notification_at IS NULL OR initialize_completed_at IS NOT NULL),
  CHECK (negotiated_protocol_version IS NULL OR negotiated_protocol_version = selected_protocol_version),
  CHECK (tools_list_observed_at IS NULL OR initialize_completed_at IS NOT NULL),
  CHECK (required_tools_validated_at IS NULL OR required_tools_validated_at >= tools_list_observed_at),
  CHECK (verification_tool_observed_at IS NULL OR required_tools_validated_at IS NOT NULL),
  CHECK (terminal_finding_id IS NULL OR graceful_close_at IS NULL),
  CHECK (last_observed_at >= process_started_at),
  CHECK (initialize_completed_at IS NULL OR initialize_completed_at >= process_started_at),
  CHECK (initialized_notification_at IS NULL OR initialized_notification_at >= initialize_completed_at),
  CHECK (tools_list_observed_at IS NULL OR tools_list_observed_at >= initialize_completed_at),
  CHECK (verification_tool_observed_at IS NULL OR verification_tool_observed_at >= initialized_notification_at),
  CHECK (verification_tool_observed_at IS NULL OR verification_tool_observed_at >= required_tools_validated_at),
  CHECK (terminal_finding_id IS NULL OR last_observed_at >= process_started_at),
  CHECK (graceful_close_at IS NULL OR graceful_close_at >= process_started_at)
);

CREATE INDEX idx_mcp_runtime_sessions_current_revision
  ON mcp_runtime_sessions (
    connection_internal_id,
    session_source,
    connection_integration_revision,
    last_observed_at
  );
CREATE INDEX idx_mcp_runtime_sessions_successful_managed
  ON mcp_runtime_sessions (
    connection_internal_id,
    connection_integration_revision,
    verification_tool_observed_at
  )
  WHERE session_source = 'managed_host'
    AND initialized_notification_at IS NOT NULL
    AND required_tools_validated_at IS NOT NULL
    AND verification_tool_name IS NOT NULL
    AND verification_tool_observed_at IS NOT NULL;

CREATE TABLE mcp_runtime_project_session_bindings (
  runtime_session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  project_internal_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  project_integration_revision TEXT NOT NULL CHECK (
    length(project_integration_revision) = 71
    AND substr(project_integration_revision, 1, 7) = 'sha256:'
    AND substr(project_integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  host_session_id TEXT NOT NULL,
  bound_at TEXT NOT NULL,
  PRIMARY KEY (runtime_session_id, host_session_id),
  UNIQUE (project_internal_id, session_id),
  FOREIGN KEY (runtime_session_id, connection_internal_id)
    REFERENCES mcp_runtime_sessions (runtime_session_id, connection_internal_id)
    ON DELETE RESTRICT,
  FOREIGN KEY (project_internal_id)
    REFERENCES projects (project_internal_id)
    ON DELETE RESTRICT,
  FOREIGN KEY (connection_internal_id, project_internal_id)
    REFERENCES connection_projects (connection_internal_id, project_internal_id)
    ON DELETE RESTRICT
);

CREATE INDEX idx_mcp_runtime_project_bindings_project
  ON mcp_runtime_project_session_bindings (
    project_internal_id, connection_internal_id, project_integration_revision, bound_at
  );

CREATE TABLE guard_installations (
  guard_installation_id TEXT PRIMARY KEY,
  runtime_home_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  project_internal_id TEXT NOT NULL,
  manifest_json TEXT NOT NULL CHECK (json_valid(manifest_json) AND json_type(manifest_json) = 'object'),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (runtime_home_id) REFERENCES runtime_home (runtime_home_id) ON DELETE RESTRICT,
  FOREIGN KEY (connection_internal_id)
    REFERENCES agent_connections (connection_internal_id)
    ON DELETE RESTRICT,
  FOREIGN KEY (project_internal_id) REFERENCES projects (project_internal_id) ON DELETE RESTRICT
);

CREATE INDEX idx_guard_installations_connection
  ON guard_installations (connection_internal_id);
CREATE INDEX idx_guard_installations_project
  ON guard_installations (project_internal_id);
CREATE UNIQUE INDEX idx_guard_installations_scope_project
  ON guard_installations (connection_internal_id, project_internal_id);

CREATE TABLE guard_integration_verification_runs (
  verification_id TEXT PRIMARY KEY CHECK (
    length(CAST(verification_id AS BLOB)) BETWEEN 1 AND 192
    AND substr(verification_id, 1, 19) = 'guard_verification_'
  ),
  connection_internal_id TEXT NOT NULL,
  project_internal_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  runtime_session_id TEXT NOT NULL,
  host_session_id TEXT NOT NULL,
  host_turn_id TEXT NOT NULL,
  integration_revision TEXT NOT NULL CHECK (
    length(integration_revision) = 71
    AND substr(integration_revision, 1, 7) = 'sha256:'
    AND substr(integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  guard_installation_id TEXT NOT NULL,
  host_contract_profile TEXT NOT NULL CHECK (
    host_contract_profile = 'codex-command-hooks'
  ),
  hook_definition_digest TEXT NOT NULL CHECK (
    length(hook_definition_digest) = 71
    AND substr(hook_definition_digest, 1, 7) = 'sha256:'
    AND substr(hook_definition_digest, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  policy_digest TEXT NOT NULL CHECK (
    length(policy_digest) = 71
    AND substr(policy_digest, 1, 7) = 'sha256:'
    AND substr(policy_digest, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  expected_probe_tool TEXT NOT NULL CHECK (
    expected_probe_tool = 'volicord.guard_probe'
  ),
  expected_host_callable_name TEXT NOT NULL CHECK (
    length(CAST(expected_host_callable_name AS BLOB)) BETWEEN 1 AND 64
    AND expected_host_callable_name NOT GLOB '*[^A-Za-z0-9_]*'
  ),
  observation_policy_kind TEXT NOT NULL CHECK (
    observation_policy_kind IN ('synchronous', 'deferred')
  ),
  observation_deadline_at TEXT,
  allowed_status_reads INTEGER NOT NULL CHECK (
    allowed_status_reads BETWEEN 1 AND 255
  ),
  status_read_count INTEGER NOT NULL DEFAULT 0 CHECK (
    status_read_count BETWEEN 0 AND allowed_status_reads
  ),
  created_at TEXT NOT NULL,
  cleanup_after TEXT NOT NULL,
  status TEXT NOT NULL CHECK (
    status IN ('awaiting_probe', 'awaiting_observation', 'complete', 'repair_required')
  ),
  probe_acknowledged_at TEXT,
  completed_at TEXT,
  matched_prompt_event_id TEXT NOT NULL,
  matched_pre_tool_event_id TEXT,
  matched_post_tool_event_id TEXT,
  repair_reason TEXT CHECK (
    repair_reason IS NULL
    OR repair_reason IN (
      'hook_event_not_observed',
      'hook_payload_incompatible',
      'callable_identity_mismatch',
      'verification_id_mismatch',
      'session_mismatch',
      'turn_mismatch',
      'tool_use_mismatch',
      'integration_revision_changed',
      'hook_definition_changed',
      'policy_changed',
      'observation_deadline_exceeded'
    )
  ),
  retry_policy TEXT CHECK (
    retry_policy IS NULL
    OR retry_policy IN (
      'no_automatic_retry',
      'new_turn_required',
      'host_reload_required',
      'hook_review_required',
      'repair_required'
    )
  ),
  terminal_finding_code TEXT CHECK (
    terminal_finding_code IS NULL
    OR (
      length(CAST(terminal_finding_code AS BLOB)) BETWEEN 1 AND 128
      AND substr(terminal_finding_code, 1, 1) GLOB '[a-z]'
      AND terminal_finding_code NOT GLOB '*[^a-z0-9_]'
    )
  ),
  terminal_finding_summary TEXT CHECK (
    terminal_finding_summary IS NULL
    OR length(CAST(terminal_finding_summary AS BLOB)) BETWEEN 1 AND 4096
  ),
  FOREIGN KEY (runtime_session_id, connection_internal_id)
    REFERENCES mcp_runtime_sessions (runtime_session_id, connection_internal_id)
    ON DELETE RESTRICT,
  FOREIGN KEY (connection_internal_id, project_internal_id)
    REFERENCES connection_projects (connection_internal_id, project_internal_id)
    ON DELETE RESTRICT,
  FOREIGN KEY (guard_installation_id)
    REFERENCES guard_installations (guard_installation_id)
    ON DELETE RESTRICT,
  CHECK (cleanup_after > created_at),
  CHECK (
    (observation_policy_kind = 'synchronous' AND observation_deadline_at IS NULL)
    OR (
      observation_policy_kind = 'deferred'
      AND (
        (status = 'awaiting_probe' AND observation_deadline_at IS NULL)
        OR observation_deadline_at > probe_acknowledged_at
      )
    )
  ),
  CHECK (probe_acknowledged_at IS NULL OR probe_acknowledged_at >= created_at),
  CHECK (
    (status = 'awaiting_probe' AND probe_acknowledged_at IS NULL
      AND completed_at IS NULL AND repair_reason IS NULL AND retry_policy IS NULL
      AND terminal_finding_code IS NULL AND terminal_finding_summary IS NULL)
    OR (status = 'awaiting_observation' AND probe_acknowledged_at IS NOT NULL
      AND completed_at IS NULL AND repair_reason IS NULL AND retry_policy IS NULL
      AND terminal_finding_code IS NULL AND terminal_finding_summary IS NULL)
    OR (status = 'complete' AND probe_acknowledged_at IS NOT NULL
      AND completed_at IS NOT NULL AND repair_reason IS NULL AND retry_policy IS NULL
      AND terminal_finding_code IS NULL AND terminal_finding_summary IS NULL
      AND matched_pre_tool_event_id IS NOT NULL AND matched_post_tool_event_id IS NOT NULL)
    OR (status = 'repair_required' AND completed_at IS NOT NULL
      AND repair_reason IS NOT NULL AND retry_policy IS NOT NULL
      AND terminal_finding_code IS NOT NULL AND terminal_finding_summary IS NOT NULL)
  ),
  CHECK (
    (matched_pre_tool_event_id IS NULL AND matched_post_tool_event_id IS NULL)
    OR matched_pre_tool_event_id IS NOT NULL
  )
);

CREATE UNIQUE INDEX idx_guard_integration_verification_coordinate
  ON guard_integration_verification_runs (
    connection_internal_id, project_id, runtime_session_id, host_session_id,
    host_turn_id, integration_revision, guard_installation_id,
    host_contract_profile, hook_definition_digest, policy_digest
  );
CREATE UNIQUE INDEX idx_guard_integration_verification_prompt_attempt
  ON guard_integration_verification_runs (project_internal_id, matched_prompt_event_id);
CREATE INDEX idx_guard_integration_verification_project
  ON guard_integration_verification_runs (
    project_internal_id, connection_internal_id, created_at, verification_id
  );

CREATE TRIGGER guard_integration_verification_coordinate_immutable
BEFORE UPDATE OF
  connection_internal_id, project_internal_id, project_id, runtime_session_id,
  host_session_id, host_turn_id, integration_revision, guard_installation_id,
  host_contract_profile, hook_definition_digest, policy_digest
ON guard_integration_verification_runs
BEGIN
  SELECT RAISE(ABORT, 'guard integration verification coordinate is immutable');
END;

CREATE TRIGGER guard_integration_verification_probe_ack_immutable
BEFORE UPDATE OF probe_acknowledged_at
ON guard_integration_verification_runs
WHEN OLD.probe_acknowledged_at IS NOT NULL
BEGIN
  SELECT RAISE(ABORT, 'guard integration verification probe acknowledgement is immutable');
END;

CREATE TRIGGER guard_integration_verification_terminal_immutable
BEFORE UPDATE ON guard_integration_verification_runs
WHEN OLD.status IN ('complete', 'repair_required')
BEGIN
  SELECT RAISE(ABORT, 'guard integration verification terminal state is immutable');
END;

CREATE TABLE guard_probe_observations (
  observation_id TEXT PRIMARY KEY CHECK (
    length(CAST(observation_id AS BLOB)) BETWEEN 1 AND 192
  ),
  verification_id TEXT NOT NULL,
  guard_event_id TEXT CHECK (
    guard_event_id IS NULL
    OR length(CAST(guard_event_id AS BLOB)) BETWEEN 1 AND 192
  ),
  stage TEXT NOT NULL CHECK (
    stage IN (
      'probe_acknowledged',
      'hook_event_not_observed',
      'hook_payload_incompatible',
      'callable_identity_unknown',
      'callable_identity_mismatch',
      'verification_id_mismatch',
      'session_mismatch',
      'turn_mismatch',
      'tool_use_mismatch',
      'pre_tool_matched',
      'post_tool_matched'
    )
  ),
  expected_agent_tool_id TEXT NOT NULL CHECK (
    expected_agent_tool_id = 'volicord.guard_probe'
  ),
  expected_host_callable_name TEXT NOT NULL CHECK (
    length(CAST(expected_host_callable_name AS BLOB)) BETWEEN 1 AND 64
    AND expected_host_callable_name NOT GLOB '*[^A-Za-z0-9_]*'
  ),
  observed_callable_name TEXT CHECK (
    observed_callable_name IS NULL
    OR length(CAST(observed_callable_name AS BLOB)) BETWEEN 1 AND 256
  ),
  hook_event_kind TEXT CHECK (
    hook_event_kind IS NULL OR hook_event_kind IN ('pre_tool', 'post_tool')
  ),
  verification_id_present INTEGER NOT NULL CHECK (
    verification_id_present IN (0, 1)
  ),
  verification_id_matches INTEGER NOT NULL CHECK (
    verification_id_matches IN (0, 1)
  ),
  guard_installation_id TEXT NOT NULL,
  integration_revision TEXT NOT NULL CHECK (
    length(integration_revision) = 71
    AND substr(integration_revision, 1, 7) = 'sha256:'
    AND substr(integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  observed_at TEXT NOT NULL,
  CHECK (verification_id_matches = 0 OR verification_id_present = 1),
  FOREIGN KEY (verification_id)
    REFERENCES guard_integration_verification_runs (verification_id)
    ON DELETE CASCADE,
  FOREIGN KEY (guard_installation_id)
    REFERENCES guard_installations (guard_installation_id)
    ON DELETE RESTRICT
);

CREATE INDEX idx_guard_probe_observations_verification
  ON guard_probe_observations (verification_id, observed_at, observation_id);
