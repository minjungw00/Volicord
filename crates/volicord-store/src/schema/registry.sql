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

CREATE TABLE mcp_runtime_sessions (
  runtime_session_id TEXT PRIMARY KEY,
  connection_internal_id TEXT NOT NULL,
  session_source TEXT NOT NULL CHECK (session_source IN ('managed_host', 'cli_preflight')),
  connection_integration_revision TEXT NOT NULL CHECK (
    length(connection_integration_revision) = 71
    AND substr(connection_integration_revision, 1, 7) = 'sha256:'
    AND substr(connection_integration_revision, 8) NOT GLOB '*[^0-9a-f]*'
  ),
  observed_host_executable_version TEXT,
  client_name TEXT,
  client_version TEXT,
  negotiated_protocol_version TEXT,
  process_id INTEGER NOT NULL CHECK (process_id > 0),
  process_started_at TEXT NOT NULL,
  initialize_completed_at TEXT,
  initialized_notification_at TEXT,
  tools_list_observed_at TEXT,
  required_tools_present INTEGER CHECK (required_tools_present IN (0, 1)),
  last_safe_read_only_tool_call_at TEXT,
  last_observed_at TEXT NOT NULL,
  terminal_protocol_failure_code TEXT,
  terminal_protocol_failure_details TEXT,
  graceful_close_at TEXT,
  UNIQUE (runtime_session_id, connection_internal_id),
  FOREIGN KEY (connection_internal_id)
    REFERENCES agent_connections (connection_internal_id)
    ON DELETE RESTRICT,
  CHECK (
    (client_name IS NULL AND client_version IS NULL)
    OR (client_name IS NOT NULL AND client_version IS NOT NULL)
  ),
  CHECK (
    (initialize_completed_at IS NULL AND client_name IS NULL)
    OR (initialize_completed_at IS NOT NULL AND client_name IS NOT NULL)
  ),
  CHECK (
    (initialized_notification_at IS NULL AND negotiated_protocol_version IS NULL)
    OR (initialized_notification_at IS NOT NULL AND negotiated_protocol_version IS NOT NULL)
  ),
  CHECK (
    (tools_list_observed_at IS NULL AND required_tools_present IS NULL)
    OR (tools_list_observed_at IS NOT NULL AND required_tools_present IS NOT NULL)
  ),
  CHECK (initialized_notification_at IS NULL OR initialize_completed_at IS NOT NULL),
  CHECK (last_safe_read_only_tool_call_at IS NULL OR initialized_notification_at IS NOT NULL),
  CHECK (
    (terminal_protocol_failure_code IS NULL AND terminal_protocol_failure_details IS NULL)
    OR terminal_protocol_failure_code IS NOT NULL
  ),
  CHECK (terminal_protocol_failure_code IS NULL OR graceful_close_at IS NULL),
  CHECK (last_observed_at >= process_started_at),
  CHECK (initialize_completed_at IS NULL OR initialize_completed_at >= process_started_at),
  CHECK (initialized_notification_at IS NULL OR initialized_notification_at >= initialize_completed_at),
  CHECK (tools_list_observed_at IS NULL OR tools_list_observed_at >= initialize_completed_at),
  CHECK (last_safe_read_only_tool_call_at IS NULL OR last_safe_read_only_tool_call_at >= initialized_notification_at),
  CHECK (terminal_protocol_failure_code IS NULL OR last_observed_at >= process_started_at),
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
    last_safe_read_only_tool_call_at
  )
  WHERE session_source = 'managed_host'
    AND initialized_notification_at IS NOT NULL
    AND required_tools_present = 1
    AND last_safe_read_only_tool_call_at IS NOT NULL;

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
