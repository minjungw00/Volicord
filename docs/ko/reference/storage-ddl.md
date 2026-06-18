# 저장소 DDL

이 문서는 [저장소 기록](storage-records.md)이 설명하는 저장소 배치를 위한 기준 SQLite DDL 계약을 담당합니다. 기준 `registry.sqlite`와 프로젝트 `state.sqlite` 배치를 구현할 수 있게 하되, 메서드 효과, 아티팩트 생명주기 규칙, 상태 버전 의미, API 스키마, 보안 보장을 이 문서로 옮기지 않습니다.

## 담당 경계

이 문서가 담당합니다.

- `registry.sqlite`와 프로젝트 `state.sqlite`의 기준 SQLite 테이블 형태
- 기준 인덱스, 외래 키, 마이그레이션 테이블, 물리 제약
- `project_state.state_version`, 재실행 행, 현재 적용 Change Unit 고유성, `Write Authorization` 기준 버전, 스테이징된 아티팩트 출처에 대한 SQLite 제약
- 런타임 홈 등록 데이터와 프로젝트별 Core 상태 사이의 DDL 수준 분리

이 문서가 담당하지 않는 것은 아래 항목입니다.

- 기록 계열 목적, 저장 위치, 저장소 소유 값, JSON 배치 범주: [저장소 기록](storage-records.md)
- 메서드 분기별 저장 효과: [저장 효과](storage-effects.md)
- 아티팩트 스테이징, 승격, 연결, 본문 읽기, 보존, 무결성 생명주기: [아티팩트 저장소](storage-artifacts.md)
- 상태 버전, 멱등성, 이벤트, 잠금, 마이그레이션 의미: [저장소 버전 관리](storage-versioning.md)
- API 요청 또는 응답 스키마: [API 코어 스키마](api/schema-core.md)가 안내하는 API 스키마 담당 문서
- 런타임 위치 경계: [런타임 경계](runtime-boundaries.md)
- 보안 보장 수준: [보안](security.md)

## 연결과 트랜잭션 요구사항

SQLite 외래 키는 이 DDL 계약의 일부입니다. 이 데이터베이스들을 읽거나 쓰는 모든 연결은 아래 설정을 활성화해야 합니다.

```sql
PRAGMA foreign_keys = ON;
```

상태 변경 커밋을 위해 최신성, 권한 부여, 스테이징, 재실행 행을 읽는 변경 트랜잭션은 `BEGIN IMMEDIATE` 또는 동등한 직렬화된 쓰기 경계를 사용해야 합니다.

기준 테이블은 `ON DELETE CASCADE`를 사용하지 않습니다. 담당 저장소 또는 마이그레이션 계약이 복구나 보존 경로를 정의하지 않는 한 권한 행은 계속 주소 지정 가능해야 합니다.

`_json`으로 끝나는 SQLite `TEXT` 열은 JSON을 저장하는 표현 선택입니다. 타입을 아는 Core 코드는 커밋 전에 해당 API 스키마 담당 문서, 저장소 담당 문서, 또는 아티팩트 담당 문서에 맞게 이 열을 파싱하고 검증해야 합니다. `'{}'`, `'[]'` 같은 SQLite 기본값은 API 필드를 선택 필드로 만들지 않습니다.

`project_state.state_version`은 기준 범위의 유일한 공개 상태 시계입니다. 기준 SQLite DDL은 `tasks.state_version`을 만들면 안 됩니다.

## `registry.sqlite`

`registry.sqlite`는 런타임 홈 식별 정보와 프로젝트 등록만 저장합니다. 프로젝트별 Core 상태는 저장하지 않습니다.

```sql
CREATE TABLE schema_migrations (
  database_kind TEXT NOT NULL CHECK (database_kind = 'registry'),
  version INTEGER NOT NULL CHECK (version > 0),
  name TEXT NOT NULL,
  storage_profile TEXT NOT NULL,
  applied_at TEXT NOT NULL,
  checksum_sha256 TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (database_kind, version)
);

CREATE TABLE runtime_home (
  singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
  runtime_home_id TEXT NOT NULL UNIQUE,
  storage_profile TEXT NOT NULL,
  schema_version INTEGER NOT NULL CHECK (schema_version > 0),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE projects (
  project_id TEXT PRIMARY KEY,
  runtime_home_id TEXT NOT NULL,
  repo_root TEXT NOT NULL,
  project_home TEXT NOT NULL UNIQUE,
  state_db_path TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'active' CHECK (status = 'active'),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  FOREIGN KEY (runtime_home_id) REFERENCES runtime_home (runtime_home_id)
);

CREATE INDEX idx_projects_repo_root ON projects (repo_root);
CREATE INDEX idx_projects_status ON projects (status);
```

레지스트리 제약:

- `runtime_home`은 단일 행 테이블입니다. 저장된 `runtime_home_id`는 런타임 홈 기록을 식별하며 보안 보장이 아닙니다.
- `projects.project_home`은 고유합니다. `repo_root`는 조회를 위해 인덱스를 두지만 프로젝트 식별을 대신하지 않습니다.
- `projects.status`는 저장소 소유 값이며 기준 범위에서 유효한 값은 `active`뿐입니다.
- `schema_migrations`는 적용된 레지스트리 스키마 버전을 기록합니다. 마이그레이션 실행 의미는 [저장소 버전 관리](storage-versioning.md)가 담당합니다.

## 프로젝트 `state.sqlite`

등록된 프로젝트마다 프로젝트별 `state.sqlite`가 하나 있습니다. 이 데이터베이스는 그 프로젝트의 Core 상태를 저장하며, 외래 키와 인덱스가 같은 프로젝트 관계를 강제할 수 있도록 프로젝트 범위 행에 `project_id`를 반복해 저장합니다.

```sql
CREATE TABLE schema_migrations (
  database_kind TEXT NOT NULL CHECK (database_kind = 'project_state'),
  version INTEGER NOT NULL CHECK (version > 0),
  name TEXT NOT NULL,
  storage_profile TEXT NOT NULL,
  applied_at TEXT NOT NULL,
  checksum_sha256 TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (database_kind, version)
);

CREATE TABLE project_state (
  project_id TEXT PRIMARY KEY,
  storage_profile TEXT NOT NULL,
  schema_version INTEGER NOT NULL CHECK (schema_version > 0),
  state_version INTEGER NOT NULL DEFAULT 0 CHECK (state_version >= 0),
  active_task_id TEXT,
  default_surface_id TEXT,
  default_surface_instance_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  CHECK (
    (default_surface_id IS NULL AND default_surface_instance_id IS NULL)
    OR (default_surface_id IS NOT NULL AND default_surface_instance_id IS NOT NULL)
  ),
  FOREIGN KEY (project_id, active_task_id)
    REFERENCES tasks (project_id, task_id)
    DEFERRABLE INITIALLY DEFERRED,
  FOREIGN KEY (project_id, default_surface_id, default_surface_instance_id)
    REFERENCES surfaces (project_id, surface_id, surface_instance_id)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE surfaces (
  project_id TEXT NOT NULL,
  surface_id TEXT NOT NULL,
  surface_instance_id TEXT NOT NULL,
  surface_kind TEXT NOT NULL,
  display_name TEXT,
  capability_profile_json TEXT NOT NULL DEFAULT '{}',
  local_access_json TEXT NOT NULL DEFAULT '{}',
  registered_at TEXT NOT NULL,
  last_seen_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, surface_id, surface_instance_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id)
);

CREATE TABLE tasks (
  project_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  created_by_surface_id TEXT NOT NULL,
  created_by_surface_instance_id TEXT NOT NULL,
  mode TEXT NOT NULL,
  lifecycle_phase TEXT NOT NULL,
  result TEXT,
  title TEXT,
  summary TEXT,
  shaping_summary_json TEXT NOT NULL DEFAULT '{}',
  bounded_context_json TEXT NOT NULL DEFAULT '[]',
  autonomy_boundary_json TEXT NOT NULL DEFAULT '{}',
  close_summary_json TEXT NOT NULL DEFAULT '{}',
  completion_policy_json TEXT NOT NULL DEFAULT '{}',
  current_change_unit_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  closed_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, task_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, created_by_surface_id, created_by_surface_instance_id)
    REFERENCES surfaces (project_id, surface_id, surface_instance_id),
  FOREIGN KEY (project_id, task_id, current_change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE change_units (
  project_id TEXT NOT NULL,
  change_unit_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('proposed', 'active', 'replaced', 'closed')),
  is_current INTEGER NOT NULL DEFAULT 0 CHECK (is_current IN (0, 1)),
  basis_state_version INTEGER CHECK (basis_state_version >= 0),
  scope_summary_json TEXT NOT NULL DEFAULT '{}',
  bounded_paths_json TEXT NOT NULL DEFAULT '[]',
  write_basis_json TEXT NOT NULL DEFAULT '{}',
  close_basis_json TEXT NOT NULL DEFAULT '{}',
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

CREATE TABLE user_judgments (
  project_id TEXT NOT NULL,
  judgment_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  judgment_kind TEXT NOT NULL,
  status TEXT NOT NULL,
  request_json TEXT NOT NULL DEFAULT '{}',
  context_json TEXT NOT NULL DEFAULT '{}',
  options_json TEXT NOT NULL DEFAULT '[]',
  affected_refs_json TEXT NOT NULL DEFAULT '[]',
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  sensitive_action_scope_json TEXT NOT NULL DEFAULT '{}',
  resolution_json TEXT,
  requested_by_surface_id TEXT NOT NULL,
  requested_by_surface_instance_id TEXT NOT NULL,
  requested_at TEXT NOT NULL,
  resolved_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, judgment_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, requested_by_surface_id, requested_by_surface_instance_id)
    REFERENCES surfaces (project_id, surface_id, surface_instance_id)
);

CREATE TABLE write_authorizations (
  project_id TEXT NOT NULL,
  write_authorization_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  basis_state_version INTEGER NOT NULL CHECK (basis_state_version > 0),
  status TEXT NOT NULL CHECK (status IN ('active', 'consumed', 'expired', 'stale', 'revoked')),
  attempt_scope_json TEXT NOT NULL DEFAULT '{}',
  created_by_surface_id TEXT NOT NULL,
  created_by_surface_instance_id TEXT NOT NULL,
  created_by_judgment_id TEXT,
  expires_at TEXT NOT NULL,
  consumed_by_run_id TEXT,
  consumed_at TEXT,
  revoked_at TEXT,
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, write_authorization_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, created_by_surface_id, created_by_surface_instance_id)
    REFERENCES surfaces (project_id, surface_id, surface_instance_id),
  FOREIGN KEY (project_id, created_by_judgment_id)
    REFERENCES user_judgments (project_id, judgment_id),
  FOREIGN KEY (project_id, consumed_by_run_id)
    REFERENCES runs (project_id, run_id)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE UNIQUE INDEX idx_write_authorizations_consumed_run
  ON write_authorizations (project_id, consumed_by_run_id)
  WHERE consumed_by_run_id IS NOT NULL;

CREATE TABLE runs (
  project_id TEXT NOT NULL,
  run_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  write_authorization_id TEXT,
  kind TEXT NOT NULL,
  status TEXT NOT NULL,
  summary_json TEXT NOT NULL DEFAULT '{}',
  observed_changes_json TEXT NOT NULL DEFAULT '{}',
  evidence_updates_json TEXT NOT NULL DEFAULT '[]',
  authorization_effect_json TEXT NOT NULL DEFAULT '{}',
  created_by_surface_id TEXT NOT NULL,
  created_by_surface_instance_id TEXT NOT NULL,
  started_at TEXT,
  completed_at TEXT,
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, run_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, write_authorization_id)
    REFERENCES write_authorizations (project_id, write_authorization_id)
    DEFERRABLE INITIALLY DEFERRED,
  FOREIGN KEY (project_id, created_by_surface_id, created_by_surface_instance_id)
    REFERENCES surfaces (project_id, surface_id, surface_instance_id)
);

CREATE UNIQUE INDEX idx_runs_write_authorization
  ON runs (project_id, write_authorization_id)
  WHERE write_authorization_id IS NOT NULL;

CREATE TABLE artifact_staging (
  project_id TEXT NOT NULL,
  handle_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  created_by_surface_id TEXT NOT NULL,
  created_by_surface_instance_id TEXT NOT NULL,
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
  FOREIGN KEY (project_id, created_by_surface_id, created_by_surface_instance_id)
    REFERENCES surfaces (project_id, surface_id, surface_instance_id),
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
  redaction_state TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('available', 'missing', 'integrity_failed', 'unavailable')),
  retention_json TEXT NOT NULL DEFAULT '{}',
  producer_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, artifact_id),
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
    owner_record_kind IN ('task', 'change_unit', 'run', 'user_judgment', 'evidence_summary', 'blocker')
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
  status TEXT NOT NULL,
  coverage_json TEXT NOT NULL DEFAULT '[]',
  supporting_refs_json TEXT NOT NULL DEFAULT '[]',
  gap_refs_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, evidence_summary_id),
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

CREATE TABLE task_events (
  project_id TEXT NOT NULL,
  event_seq INTEGER NOT NULL CHECK (event_seq > 0),
  event_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  state_version INTEGER NOT NULL CHECK (state_version > 0),
  event_kind TEXT NOT NULL,
  event_payload_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  PRIMARY KEY (project_id, event_seq),
  UNIQUE (project_id, event_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
);

CREATE TABLE tool_invocations (
  project_id TEXT NOT NULL,
  tool_name TEXT NOT NULL,
  idempotency_key TEXT NOT NULL,
  request_hash TEXT NOT NULL,
  basis_state_version INTEGER NOT NULL CHECK (basis_state_version >= 0),
  committed_state_version INTEGER NOT NULL CHECK (committed_state_version > basis_state_version),
  status TEXT NOT NULL DEFAULT 'committed' CHECK (status = 'committed'),
  surface_id TEXT,
  surface_instance_id TEXT,
  access_class TEXT,
  verification_basis TEXT,
  replay_context_status TEXT NOT NULL CHECK (replay_context_status IN ('verified', 'legacy_unverified')),
  response_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (project_id, tool_name, idempotency_key),
  CHECK (
    (
      replay_context_status = 'verified'
      AND surface_id IS NOT NULL
      AND surface_instance_id IS NOT NULL
      AND access_class IS NOT NULL
    )
    OR (
      replay_context_status = 'legacy_unverified'
    )
  ),
  FOREIGN KEY (project_id, surface_id, surface_instance_id)
    REFERENCES surfaces (project_id, surface_id, surface_instance_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id)
);
```

프로젝트 상태의 기준 인덱스:

```sql
CREATE INDEX idx_project_state_active_task
  ON project_state (project_id, active_task_id);

CREATE INDEX idx_surfaces_last_seen
  ON surfaces (project_id, last_seen_at);

CREATE INDEX idx_tasks_lifecycle
  ON tasks (project_id, lifecycle_phase, result);

CREATE INDEX idx_tasks_current_change_unit
  ON tasks (project_id, current_change_unit_id);

CREATE INDEX idx_change_units_task_status
  ON change_units (project_id, task_id, status);

CREATE INDEX idx_user_judgments_task_status
  ON user_judgments (project_id, task_id, status);

CREATE INDEX idx_write_authorizations_task_status
  ON write_authorizations (project_id, task_id, status);

CREATE INDEX idx_runs_task_created
  ON runs (project_id, task_id, created_at);

CREATE INDEX idx_artifact_staging_task_status
  ON artifact_staging (project_id, task_id, status);

CREATE INDEX idx_artifact_staging_surface
  ON artifact_staging (project_id, created_by_surface_id, created_by_surface_instance_id);

CREATE INDEX idx_artifacts_task_status
  ON artifacts (project_id, task_id, status);

CREATE INDEX idx_artifact_links_owner
  ON artifact_links (project_id, owner_record_kind, owner_record_id);

CREATE INDEX idx_evidence_summaries_task_status
  ON evidence_summaries (project_id, task_id, status);

CREATE INDEX idx_blockers_task_status
  ON blockers (project_id, task_id, status);

CREATE INDEX idx_task_events_task_seq
  ON task_events (project_id, task_id, event_seq);
```

## 제약 참고사항

`project_state.state_version`:

- `project_state.state_version`은 기준 범위의 유일한 공개 상태 시계입니다.
- `tasks.state_version`을 만들면 안 됩니다.
- `task_events.state_version`은 커밋된 이벤트 뒤의 결과 프로젝트 전체 버전을 저장합니다.
- `tool_invocations.basis_state_version`은 원래 커밋된 상태 변경이 커밋 전에 관찰한 프로젝트 전체 버전을 저장합니다.

현재 적용 Change Unit:

- `idx_change_units_one_current_active`는 `Task`마다 `status='active'`이고 `is_current=1`인 현재 적용 Change Unit 행이 최대 하나만 있도록 합니다.
- `tasks.current_change_unit_id`가 설정되어 있으면 타입을 아는 Core 코드는 그 포인터가 `status='active'`이고 `is_current=1`인 같은 행을 가리키는지 확인해야 합니다.

접점 로컬 접근 허용:

- `surfaces.local_access_json`은 등록된 로컬 접근 허용의 기준 저장 위치입니다.
- 선호되는 허용 필드는 `authorized_access_classes: string[]`입니다. `access_class: string`은 하위 호환을 위한 단일 값 대체 필드입니다.
- `verification_basis: string`은 허용이 어떻게 성립했는지 설명하는 신뢰된 등록 메타데이터입니다.
- `surfaces.capability_profile_json`은 역량 선언이며 접근 등급 허용으로 취급하면 안 됩니다.

멱등 재실행 행:

- 재실행 고유 키는 정확히 `(project_id, tool_name, idempotency_key)`입니다.
- `request_hash`는 공개 요청 충돌 판별자로 저장하지만 고유 키의 일부가 아니며 호출 맥락을 흡수하지 않습니다.
- `tool_invocations.response_json`은 [저장 효과](storage-effects.md)가 재실행 행 생성을 정의한 커밋된 재실행 응답만 저장합니다.
- 새로 쓰는 재실행 행은 `replay_context_status='verified'`를 사용하고 파생된 `VerifiedSurfaceContext`의 완전한 `surface_id`, `surface_instance_id`, `access_class` 값을 저장합니다.
- `verification_basis`는 진단용으로 재실행 행에 저장할 수 있지만 호출자 권한이 아닙니다.
- 확인된 맥락이 없는 기존 재실행 행은 `replay_context_status='legacy_unverified'`와 null 또는 불완전한 맥락 필드로 표현할 수 있습니다. 재실행 적격성은 [저장소 버전 관리](storage-versioning.md)가 담당합니다.

`Write Authorization` 기준 버전:

- `write_authorizations.basis_state_version`은 권한 생성 커밋 뒤 결과 `project_state.state_version`을 저장합니다.
- `basis_state_version`은 별도 상태 시계가 아니며 `tasks` 행과 비교하면 안 됩니다.
- `idx_runs_write_authorization`과 `idx_write_authorizations_consumed_run`은 단일 권한 소비가 여러 실행으로 갈라지는 것을 막습니다.

스테이징된 아티팩트 출처:

- `artifact_staging.created_by_surface_id`와 `artifact_staging.created_by_surface_instance_id`는 필수이며 `surfaces`에 외래 키로 연결됩니다.
- 스테이징 핸들을 소비하기 전에는 저장된 접점 출처, 같은 프로젝트, 같은 `Task`, 만료, 생명주기 상태, `sha256`, `size_bytes`, `redaction_state`를 검증해야 합니다.
- `artifact_staging`과 `artifacts`의 고유 인덱스는 하나의 스테이징 핸들이 여러 아티팩트 행으로 승격되는 것을 막습니다.

마이그레이션 기록:

- 각 데이터베이스는 자체 `schema_migrations` 테이블을 가집니다.
- `schema_migrations`는 적용된 스키마 버전, 이름, 저장소 프로필, 적용 시각, 선택적 체크섬, 저장소 소유 메타데이터를 기록합니다.
- 마이그레이션 의미, 복구 동작, 지원되는 마이그레이션 경로는 [저장소 버전 관리](storage-versioning.md)가 담당합니다.

외래 키 한계:

- 직접 표현할 수 있는 같은 프로젝트와 같은 `Task` 관계는 복합 외래 키를 사용합니다.
- `artifact_links.owner_record_kind`는 닫힌 저장소 소유 값 집합이지만 `owner_record_id`는 다형입니다. 타입을 아는 Core 코드는 `owner_record_kind`가 이름 붙인 테이블에 담당 행이 존재하고 같은 `project_id`와 `task_id`에 속하는지 검증해야 합니다.
- JSON 참조 배열은 SQLite 외래 키로 표현할 수 없습니다. 타입을 아는 Core 코드는 커밋 전에 이 참조들을 검증해야 합니다.

## 관련 담당 문서

- [저장소 기록](storage-records.md): 기록 계열, 위치, 저장소 소유 값, JSON 배치 범주.
- [저장 효과](storage-effects.md): 어떤 메서드 분기가 저장소를 만들거나, 바꾸거나, 관찰하거나, 건드리지 않는지.
- [아티팩트 저장소](storage-artifacts.md): 스테이징 핸들 생명주기, 승격, 연결, 본문 읽기, 보존, 무결성 경계.
- [저장소 버전 관리](storage-versioning.md): 상태 시계 의미, 멱등성과 재실행 의미, 이벤트 의미, 잠금, 마이그레이션 의미.
- [API 코어 스키마](api/schema-core.md)와 같은 스키마 담당 문서: 공개 API 형태와 API 소유 값 의미.
