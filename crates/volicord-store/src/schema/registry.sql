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
  host_kind TEXT NOT NULL CHECK (host_kind IN ('codex', 'claude_code', 'generic')),
  intent TEXT NOT NULL CHECK (intent IN ('personal', 'shared', 'global')),
  host_scope TEXT NOT NULL CHECK (host_scope IN ('user', 'project', 'local', 'export')),
  project_internal_id TEXT,
  server_name TEXT NOT NULL,
  config_target TEXT NOT NULL,
  mode TEXT NOT NULL CHECK (mode IN ('read_only', 'workflow')),
  enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  managed_fingerprint TEXT NOT NULL,
  last_verification_status TEXT NOT NULL DEFAULT 'not_verified'
    CHECK (last_verification_status IN ('not_verified', 'complete', 'action_required', 'failed')),
  last_verification_report_json TEXT NOT NULL DEFAULT '{}',
  last_user_actions_json TEXT NOT NULL DEFAULT '[]',
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_internal_id) REFERENCES projects (project_internal_id) ON DELETE RESTRICT,
  CHECK (
    (host_kind = 'codex' AND host_scope IN ('user', 'project'))
    OR (host_kind = 'claude_code' AND host_scope IN ('local', 'project', 'user'))
    OR (host_kind = 'generic' AND host_scope = 'export')
  )
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
CREATE UNIQUE INDEX idx_agent_connections_target_global
  ON agent_connections (
    host_kind,
    intent,
    host_scope,
    config_target,
    server_name
  )
  WHERE project_internal_id IS NULL;

CREATE TABLE host_capability_verifications (
  verification_internal_id TEXT PRIMARY KEY,
  connection_internal_id TEXT NOT NULL,
  capability TEXT NOT NULL
    CHECK (capability = 'model_invisible_user_surface'),
  outcome TEXT NOT NULL
    CHECK (outcome IN ('passed', 'failed', 'unavailable', 'revoked')),
  host_kind TEXT NOT NULL
    CHECK (host_kind IN ('codex', 'claude_code', 'generic')),
  host_version TEXT NOT NULL CHECK (length(trim(host_version)) > 0),
  client_name TEXT NOT NULL CHECK (length(trim(client_name)) > 0),
  client_version TEXT NOT NULL CHECK (length(trim(client_version)) > 0),
  adapter_profile TEXT NOT NULL
    CHECK (adapter_profile = 'mcp_user_channel_local_web_v1'),
  adapter_version TEXT NOT NULL CHECK (length(trim(adapter_version)) > 0),
  managed_fingerprint TEXT NOT NULL CHECK (length(trim(managed_fingerprint)) > 0),
  volicord_build_id TEXT NOT NULL CHECK (length(trim(volicord_build_id)) > 0),
  source_revision TEXT NOT NULL CHECK (length(trim(source_revision)) > 0),
  target_triple TEXT NOT NULL CHECK (length(trim(target_triple)) > 0),
  executable_sha256 TEXT NOT NULL
    CHECK (length(executable_sha256) = 64 AND executable_sha256 NOT GLOB '*[^0-9a-f]*'),
  evidence_artifact_sha256 TEXT NOT NULL
    CHECK (
      length(evidence_artifact_sha256) = 64
      AND evidence_artifact_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
  observed_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}' CHECK (metadata_json = '{}'),
  created_at TEXT NOT NULL,
  FOREIGN KEY (connection_internal_id)
    REFERENCES agent_connections (connection_internal_id)
    ON DELETE CASCADE,
  UNIQUE (connection_internal_id, capability, verification_internal_id),
  CHECK (outcome != 'passed' OR host_kind IN ('codex', 'claude_code')),
  CHECK (outcome != 'passed' OR host_version = client_version),
  CHECK (
    outcome != 'passed'
    OR (
      length(source_revision) IN (40, 64)
      AND source_revision NOT GLOB '*[^0-9a-f]*'
    )
  )
);

CREATE TABLE host_capability_state (
  connection_internal_id TEXT NOT NULL,
  capability TEXT NOT NULL
    CHECK (capability = 'model_invisible_user_surface'),
  current_verification_internal_id TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (connection_internal_id, capability),
  FOREIGN KEY (connection_internal_id)
    REFERENCES agent_connections (connection_internal_id)
    ON DELETE CASCADE,
  FOREIGN KEY (
    connection_internal_id,
    capability,
    current_verification_internal_id
  ) REFERENCES host_capability_verifications (
    connection_internal_id,
    capability,
    verification_internal_id
  ) ON DELETE CASCADE
);

CREATE INDEX idx_host_capability_verifications_connection
  ON host_capability_verifications (connection_internal_id, capability, observed_at);
CREATE INDEX idx_host_capability_verifications_outcome_expiry
  ON host_capability_verifications (outcome, expires_at);
CREATE INDEX idx_host_capability_state_current
  ON host_capability_state (current_verification_internal_id);

CREATE TABLE guard_installations (
  guard_installation_id TEXT PRIMARY KEY,
  runtime_home_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  project_internal_id TEXT,
  host_kind TEXT NOT NULL CHECK (length(trim(host_kind)) > 0),
  guard_mode TEXT NOT NULL CHECK (guard_mode IN ('record', 'detective')),
  host_capability_json TEXT NOT NULL DEFAULT '{}',
  installation_status TEXT NOT NULL
    CHECK (installation_status IN (
      'absent',
      'configured',
      'reload_required',
      'active',
      'degraded',
      'stale',
      'broken'
    )),
  installed_at TEXT,
  last_checked_at TEXT NOT NULL,
  first_seen_at TEXT,
  last_seen_at TEXT,
  last_seen_phase TEXT,
  observed_host_kind TEXT,
  observed_policy_hash TEXT,
  observed_binary_version TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
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
CREATE INDEX idx_guard_installations_status
  ON guard_installations (installation_status);
CREATE UNIQUE INDEX idx_guard_installations_scope_project
  ON guard_installations (connection_internal_id, project_internal_id, guard_mode)
  WHERE project_internal_id IS NOT NULL;
CREATE UNIQUE INDEX idx_guard_installations_scope_global
  ON guard_installations (connection_internal_id, guard_mode)
  WHERE project_internal_id IS NULL;
