# 저장소 DDL

이 문서는 [저장소 기록](storage-records.md)이 설명하는 저장소 배치를 위한 기준 SQLite DDL 계약을 담당합니다. 기준 `registry.sqlite`와 프로젝트 `state.sqlite` 배치를 구현할 수 있게 하되, 메서드 효과, 아티팩트 생명주기 규칙, 상태 버전 의미, API 스키마, 보안 보장을 이 문서로 옮기지 않습니다.

## 담당 경계

이 문서가 담당합니다.

- `registry.sqlite`와 프로젝트 `state.sqlite`의 기준 SQLite 테이블 형태
- 기준 인덱스, 외래 키, 물리 제약
- `project_state.state_version`, 재실행 행, 현재 적용 Change Unit 고유성, 쓰기 티켓 기준 버전, 스테이징된 아티팩트 출처, 호스트 관찰 기록에 대한 SQLite 제약
- Runtime Home 등록 데이터와 프로젝트별 Core 상태 사이의 DDL 수준 분리
- 문서화된 기준 SQL 블록과 기준 SQL 원본 파일 사이의 일치

이 문서는 담당하지 않습니다.

- 기록 계열 목적, 저장 위치, 저장소 소유 값, JSON 배치 범주: [저장소 기록](storage-records.md)
- 메서드 분기별 저장 효과: [저장 효과](storage-effects.md)
- 아티팩트 스테이징, 승격, 연결, 본문 읽기, 보존, 무결성 생명주기: [아티팩트 저장소](storage-artifacts.md)
- 상태 버전, 멱등성, 이벤트, 잠금, 호환되지 않는 저장소 처리: [저장소 버전 관리](storage-versioning.md)
- API 요청 또는 응답 스키마: [API 코어 스키마](api/schema-core.md)가 안내하는 API 스키마 담당 문서
- 런타임 위치 경계: [런타임 경계](runtime-boundaries.md)
- 보안 보장 수준: [보안](security.md)

<a id="surface-stability"></a>
## 표면 안정성

기준 어휘는 [문서 정책](../maintain/documentation-policy.md#surface-stability-labels)을 확인하세요. 이 섹션에서 `stable`은 문서화된 호환성 표면을 뜻합니다. `beta`는 지원되지만 세부사항이 바뀔 수 있음을 뜻합니다. `internal`은 구현 또는 생성된 통합 세부사항이며 일반 사용자 입력 표면이 아님을 뜻합니다. `diagnostic`은 문제 해결이나 상태 보고 표면이며 산문 또는 진단 문구가 안정적인 API 계약이 아님을 뜻합니다.

| 표면 | 안정성 | 비고 |
|---|---|---|
| 기준 SQLite DDL, 기준 SQL 블록, 테이블 제약, 인덱스, 외래 키, 공개 기준 상태 시계인 `project_state.state_version` | `stable` | 현재 기준 프로필을 위한 구현 가능한 저장소 DDL 계약입니다. |
| 물리 테이블 이름, 열 이름, 내부 ID, 생성된 호스트 관찰 행, `_json` 표현 열 | `internal` | 저장소 배치를 구현 가능하게 하는 세부사항입니다. 다른 집중 담당 문서가 노출하지 않는 한 일반 사용자 대상 선택자나 공개 API 인자가 아닙니다. |
| 테이블, 기록 참조, 논리 열, 손상 범주를 식별하는 안전한 저장소 또는 손상 진단 | `diagnostic` | 진단은 원본 저장 JSON, 비밀값, SQL 텍스트, 민감한 절대 경로를 노출하면 안 됩니다. |

## 연결과 트랜잭션 요구사항

SQLite 외래 키는 이 DDL 계약의 일부입니다. 이 데이터베이스들을 읽거나 쓰는 모든 연결은 아래 설정을 활성화해야 합니다.

```sql
PRAGMA foreign_keys = ON;
```

상태 변경 커밋을 위해 최신성, 쓰기 티켓 호환성 행, 스테이징, 재실행 행을 읽는 변경 트랜잭션은 `BEGIN IMMEDIATE` 또는 동등한 직렬화된 쓰기 경계를 사용해야 합니다.

담당 저장소 계약이 복구나 보존 경로를 정의하지 않는 한 권한 효력이 있는 행은 계속 주소 지정 가능해야 합니다. 레지스트리는 프로젝트 등록을 삭제할 때 그 등록이 소유한 비권한 별칭 행을 연쇄 삭제할 수 있습니다. 이 별칭 정리가 프로젝트별 Core 권한 기록 삭제를 뜻하면 안 됩니다.

`_json`으로 끝나는 SQLite `TEXT` 열은 JSON을 저장하는 표현 선택입니다. 권한, 생명주기, 범위, 증거, 완료, 닫기 준비 상태, 쓰기 호환성에 쓰이는 JSON은 타입이 지정된 담당 상태입니다. 타입을 아는 Core 코드는 커밋 전에 해당 API 스키마 담당 문서, 저장소 담당 문서, 또는 아티팩트 담당 문서에 맞게 이 열을 파싱하고 검증해야 합니다. 타입이 지정된 담당 상태를 디코드하지 못하는 경우는 손상이며 빈 객체, 빈 배열, `false` 값, 기본 열거형 값, 또는 "요구사항 없음" 해석으로 바꾸면 안 됩니다. SQL `NULL`은 담당 스키마가 그 필드를 명시적으로 선택 필드라고 표시할 때만 부재를 뜻할 수 있습니다. 선택 열의 형식이 잘못된 JSON도 부재가 아니라 손상입니다. 열린 표시 메타데이터는 권한이나 닫기 판단에 쓰이지 않을 때만 타입을 지정하지 않은 채로 둘 수 있습니다. 안전한 진단은 테이블, 기록 참조, 논리 열, 손상 범주를 식별할 수 있지만 원본 저장 JSON, 비밀값, SQL 텍스트, 민감한 절대 경로를 노출하면 안 됩니다. `'{}'`, `'[]'` 같은 SQLite 기본값은 API 필드를 선택 필드로 만들지 않습니다.

`project_state.state_version`은 기준 범위의 유일한 공개 상태 시계입니다. 기준 SQLite DDL은 `tasks.state_version`, 저장소 `schema_version` 열, 마이그레이션 이력 테이블을 만들면 안 됩니다.

물리 `write_tickets` 테이블은 제품 파일 쓰기 시도에 대한 쓰기 티켓 권한 기록을 저장합니다. 이 행은 Volicord 안에서 권한 있는 쓰기 의도와 호환성 상태를 기록합니다. OS 권한, 파일시스템 ACL, 샌드박싱, 네트워크 정책, 비밀값 격리, 전역 파일시스템 가로채기, 쓰기가 실제로 일어났다는 증명이 아닙니다.

## 기준 SQL 원본

실행 가능한 기준 SQL 원본은 [`registry.sql`](../../../crates/volicord-store/src/schema/registry.sql)과 [`project.sql`](../../../crates/volicord-store/src/schema/project.sql)입니다. Runtime Home 초기화는 비어 있는 SQLite 데이터베이스에 이 원본을 적용합니다. 이 원본과 호환되지 않는 저장소 형태를 가진 기존 Runtime Home은 분명하게 실패하고 Runtime Home 재생성을 요구합니다. 기준 저장소는 이전 버전 저장소의 마이그레이션 경로를 정의하지 않습니다.

`docs-check`는 아래 기준 SQL 블록이 해당 원본 파일과 정확히 일치하는지 검증합니다. 집중 `storage_ddl_contract` 테스트는 실행 가능한 스키마 의미를 검증합니다.

## `registry.sqlite`

`registry.sqlite`는 Runtime Home 식별 정보, 설치 프로필 기록, 프로젝트 등록, 프로젝트 별칭, Agent Connection 기록, Connection Projects 멤버십, 호스트 훅 설치 기록, 호스트 설정 목록을 저장합니다. 프로젝트별 Core 상태는 저장하지 않습니다.

<!-- canonical-storage-sql: registry start -->
```sql
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
```
<!-- canonical-storage-sql: registry end -->

레지스트리 제약:

- `runtime_home`은 단일 행 테이블입니다. Runtime Home 식별 정보, Runtime Home 경로, 레지스트리 데이터베이스 경로, 저장소 프로필, 메타데이터, 타임스탬프를 저장합니다. 저장된 `runtime_home_id`는 Runtime Home 기록을 식별하며 보안 보장이 아닙니다.
- `installation_profile`은 Runtime Home에 대해 선택된 `volicord` 명령, MCP 시작 명령, 실행 파일 디렉터리, 기본 연결 모드, 메타데이터, 타임스탬프를 저장합니다. `volicord init`이 이를 마련할 수 있습니다. 호스트 신뢰, 사용자 권한, 공개 API 상태가 아닙니다.
- `projects.project_internal_id`는 프로젝트 기록의 저장 기본 키입니다. `projects.project_name`은 표시 이름입니다. `projects.project_alias`는 CLI 선택 보조 값입니다. `projects.repo_root`는 저장소 루트 조회 키입니다. `projects.project_alias`, `projects.repo_root`, `projects.project_home`, `projects.state_db_path`는 고유합니다.
- `project_aliases`는 별칭을 `project_internal_id` 값에 매핑합니다. 별칭 행은 레지스트리 선택 보조 값이지 프로젝트별 Core 권한 기록이 아닙니다.
- `projects.state_db_path`는 저장 열로 유지됩니다. Store의 애플리케이션 수준 현재 등록 검증은 운영 `ProjectRecord` 조회나 목록 조회, 쓰기 가능한 프로젝트 상태 열기, Agent Connection 프로젝트 처리 경로, Core 실행, 프로필 재사용, MCP 프로젝트 가용성 확인 전에 이 값이 `project_home/state.sqlite`와 같은지 확인해야 합니다.
- `projects.status`는 저장소 소유 값이며 기준 범위에서 유효한 값은 `active`뿐입니다.
- `agent_connections.connection_internal_id`는 Agent Connection 기록의 저장 기본 키입니다. 이 테이블은 호스트 종류, `intent`에 저장되는 연결 의도, 호스트 범위, 선택적 `project_internal_id`, 서버 이름, 설정 대상, 모드, 활성 상태, 관리 지문, 검증 요약 상태, 검증 보고서 JSON, 사용자 동작 JSON, 메타데이터, 타임스탬프를 저장합니다.
- `agent_connections.intent`는 `personal`, `shared`, `global`로 제한됩니다.
- `agent_connections.host_scope`는 `host_kind`와 함께 제한됩니다. Codex는 `user`와 `project`를 지원하고, Claude Code는 `local`, `project`, `user`를 지원하며, `generic` 기록은 `export`로 제한됩니다.
- `agent_connections.mode`는 `read_only` 또는 `workflow`로 제한됩니다.
- `agent_connections.last_verification_report_json`은 최신 검증 보고서 JSON 객체를 저장합니다. `agent_connections.last_user_actions_json`은 최신 사용자 동작 JSON 배열을 저장합니다.
- `connection_projects`는 Agent Connection 하나에 대한 명시적 프로젝트 허용 목록입니다. `connection_internal_id`와 `project_internal_id`로 멤버십을 저장합니다. 아직 멤버십이 남은 프로젝트나 연결 삭제는 제한됩니다.
- `guard_installations`는 Runtime Home 하나, Agent Connection 하나, 선택적 프로젝트 범위에 대한 로컬 호스트 훅 설정 생명주기 상태와 호스트 역량을 저장합니다. 내부 `guard_mode` 값은 `record`, `detective`입니다. `installation_status` 값은 `absent`, `configured`, `reload_required`, `active`, `degraded`, `stale`, `broken`입니다. 이 행은 호스트 관찰을 위한 로컬 권한 기록이며 OS 수준 집행 증명이나 쓰기 방지 증명이 아닙니다.

## 프로젝트 `state.sqlite`

등록된 프로젝트마다 프로젝트별 `state.sqlite`가 하나 있습니다. 이 데이터베이스는 그 프로젝트의 Core 상태를 저장하며, 외래 키와 인덱스가 같은 프로젝트 관계를 강제할 수 있도록 프로젝트 범위 행에 `project_id`를 반복해 저장합니다.

<!-- canonical-storage-sql: project start -->
```sql
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
  bounded_context_json TEXT NOT NULL DEFAULT '[]',
  autonomy_boundary_json TEXT NOT NULL DEFAULT '{}',
  scope_revision INTEGER NOT NULL DEFAULT 0 CHECK (scope_revision >= 0),
  close_basis_revision INTEGER NOT NULL DEFAULT 0 CHECK (close_basis_revision >= 0),
  close_basis_json TEXT,
  close_summary_json TEXT NOT NULL DEFAULT '{}',
  current_change_unit_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  closed_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, task_id),
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
  basis_state_version INTEGER CHECK (basis_state_version >= 0),
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
      'verified_tool_invocation',
      'registered_connection_observation'
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

CREATE TABLE user_judgments (
  project_id TEXT NOT NULL,
  judgment_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  judgment_kind TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('pending', 'resolved', 'stale', 'superseded', 'expired')),
  request_json TEXT NOT NULL DEFAULT '{}',
  context_json TEXT NOT NULL DEFAULT '{}',
  options_json TEXT NOT NULL DEFAULT '{"options":[]}',
  affected_refs_json TEXT NOT NULL DEFAULT '[]',
  artifact_refs_json TEXT NOT NULL DEFAULT '[]',
  sensitive_action_scope_json TEXT NOT NULL DEFAULT '{}',
  basis_json TEXT NOT NULL,
  basis_status TEXT NOT NULL DEFAULT 'current'
    CHECK (basis_status IN ('current', 'stale', 'superseded')),
  resolution_outcome TEXT
    CHECK (resolution_outcome IS NULL OR resolution_outcome IN ('accepted', 'rejected', 'deferred')),
  resolution_machine_action TEXT
    CHECK (resolution_machine_action IS NULL OR resolution_machine_action IN ('accept', 'reject', 'defer')),
  resolution_json TEXT,
  resolution_rationale_json TEXT,
  requested_by_actor_source TEXT NOT NULL,
  resolved_by_actor_source TEXT,
  resolved_verification_basis TEXT,
  resolved_assurance_level TEXT,
  requested_at TEXT NOT NULL,
  resolved_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, judgment_id),
  CHECK (
    (
      status IN ('pending', 'expired')
      AND resolution_outcome IS NULL
      AND resolution_machine_action IS NULL
      AND resolution_json IS NULL
      AND resolution_rationale_json IS NULL
      AND resolved_by_actor_source IS NULL
      AND resolved_verification_basis IS NULL
      AND resolved_assurance_level IS NULL
      AND resolved_at IS NULL
    )
    OR (
      status = 'resolved'
      AND resolution_outcome IS NOT NULL
      AND resolution_machine_action IS NOT NULL
      AND resolution_json IS NOT NULL
      AND resolution_rationale_json IS NOT NULL
      AND resolved_by_actor_source IS NOT NULL
      AND resolved_verification_basis IS NOT NULL
      AND resolved_assurance_level IS NOT NULL
      AND resolved_at IS NOT NULL
    )
    OR (
      status IN ('stale', 'superseded')
      AND (
        (
          resolution_outcome IS NULL
          AND resolution_machine_action IS NULL
          AND resolution_json IS NULL
          AND resolution_rationale_json IS NULL
          AND resolved_by_actor_source IS NULL
          AND resolved_verification_basis IS NULL
          AND resolved_assurance_level IS NULL
          AND resolved_at IS NULL
        )
        OR (
          resolution_outcome IS NOT NULL
          AND resolution_machine_action IS NOT NULL
          AND resolution_json IS NOT NULL
          AND resolution_rationale_json IS NOT NULL
          AND resolved_by_actor_source IS NOT NULL
          AND resolved_verification_basis IS NOT NULL
          AND resolved_assurance_level IS NOT NULL
          AND resolved_at IS NOT NULL
        )
      )
    )
  ),
  CHECK (
    resolution_machine_action IS NULL
    OR (
      (resolution_machine_action = 'accept' AND resolution_outcome = 'accepted')
      OR (resolution_machine_action = 'reject' AND resolution_outcome = 'rejected')
      OR (resolution_machine_action = 'defer' AND resolution_outcome = 'deferred')
    )
  ),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id)
);

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
  change_unit_id TEXT,
  basis_state_version INTEGER NOT NULL CHECK (basis_state_version > 0),
  status TEXT NOT NULL CHECK (status IN ('active', 'consumed', 'expired', 'stale', 'revoked')),
  attempt_scope_json TEXT NOT NULL DEFAULT '{}',
  created_by_actor_source TEXT NOT NULL,
  created_by_judgment_id TEXT,
  expires_at TEXT NOT NULL,
  consumed_by_run_id TEXT,
  consumed_at TEXT,
  revoked_at TEXT,
  created_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, write_ticket_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, created_by_judgment_id)
    REFERENCES user_judgments (project_id, judgment_id),
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
      'verified_tool_invocation',
      'registered_connection_observation'
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
    source_claim_kind IN (
      'host_invocation',
      'guard_event',
      'session_watch_observation'
    )
  ),
  source_claim_id TEXT NOT NULL CHECK (length(trim(source_claim_id)) > 0),
  evidence_capture_intent_id TEXT NOT NULL,
  evidence_capture_receipt_id TEXT NOT NULL,
  capture_kind TEXT NOT NULL CHECK (
    capture_kind IN (
      'verified_command_execution',
      'verified_tool_invocation',
      'registered_connection_observation'
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
    owner_record_kind IN ('task', 'change_unit', 'run', 'user_judgment', 'evidence_summary', 'evidence_observation', 'evidence_producer', 'blocker')
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

CREATE TABLE user_evidence_observations (
  project_id TEXT NOT NULL,
  user_evidence_observation_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  change_unit_id TEXT NOT NULL,
  scope_revision INTEGER NOT NULL CHECK (scope_revision >= 0),
  baseline_ref TEXT NOT NULL CHECK (length(trim(baseline_ref)) > 0),
  acceptance_criterion_id TEXT,
  evidence_claim_id TEXT,
  relevance_status TEXT NOT NULL CHECK (relevance_status IN ('supported', 'contradicted')),
  output_artifact_refs_json TEXT NOT NULL,
  summary TEXT NOT NULL CHECK (length(trim(summary)) > 0),
  observed_by_actor_source TEXT NOT NULL CHECK (observed_by_actor_source = 'local_user'),
  verification_basis TEXT NOT NULL CHECK (length(trim(verification_basis)) > 0),
  observed_at TEXT NOT NULL,
  recorded_at TEXT NOT NULL,
  PRIMARY KEY (project_id, user_evidence_observation_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
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
      'verified_tool_invocation',
      'registered_connection_observation'
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
  task_id TEXT NOT NULL,
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
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id),
  FOREIGN KEY (project_id, task_id, change_unit_id)
    REFERENCES change_units (project_id, task_id, change_unit_id),
  FOREIGN KEY (project_id, previous_event_hash)
    REFERENCES authority_events (project_id, event_hash)
    DEFERRABLE INITIALLY DEFERRED
);

CREATE VIEW task_events AS
SELECT
  project_id,
  event_seq,
  event_id,
  task_id,
  change_unit_id,
  state_version,
  event_type AS event_kind,
  payload_json AS event_payload_json,
  created_at
FROM authority_events;

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

CREATE INDEX idx_user_judgments_task_status
  ON user_judgments (project_id, task_id, status);

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
CREATE INDEX idx_user_evidence_observations_task_target
  ON user_evidence_observations (
    project_id,
    task_id,
    acceptance_criterion_id,
    evidence_claim_id,
    recorded_at
  );

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
CREATE TABLE agent_sessions (
  project_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  guard_installation_id TEXT,
  host_kind TEXT NOT NULL CHECK (length(trim(host_kind)) > 0),
  guard_mode TEXT NOT NULL CHECK (guard_mode IN ('record', 'detective')),
  started_at TEXT NOT NULL,
  ended_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, session_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id)
);

CREATE TABLE guard_events (
  project_id TEXT NOT NULL,
  guard_event_id TEXT NOT NULL,
  session_id TEXT,
  connection_internal_id TEXT NOT NULL,
  guard_installation_id TEXT,
  event_kind TEXT NOT NULL,
  decision TEXT NOT NULL CHECK (decision IN ('allow', 'deny', 'warn', 'inject_context')),
  subject_json TEXT NOT NULL DEFAULT '{}',
  result_json TEXT NOT NULL DEFAULT '{}',
  occurred_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, guard_event_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, session_id) REFERENCES agent_sessions (project_id, session_id)
);

CREATE TABLE prompt_captures (
  project_id TEXT NOT NULL,
  prompt_capture_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  capture_kind TEXT NOT NULL,
  prompt_sha256 TEXT NOT NULL,
  prompt_text TEXT,
  captured_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, prompt_capture_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, session_id) REFERENCES agent_sessions (project_id, session_id)
);

CREATE TABLE unrecorded_changes (
  project_id TEXT NOT NULL,
  unrecorded_change_id TEXT NOT NULL,
  session_id TEXT,
  connection_internal_id TEXT NOT NULL,
  task_id TEXT,
  status TEXT NOT NULL CHECK (status IN ('unresolved', 'resolved')),
  summary TEXT NOT NULL CHECK (length(trim(summary)) > 0),
  observed_paths_json TEXT NOT NULL DEFAULT '[]',
  detection_json TEXT NOT NULL DEFAULT '{}',
  resolution_json TEXT,
  detected_at TEXT NOT NULL,
  resolved_at TEXT,
  resolved_by_actor_source TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, unrecorded_change_id),
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
  FOREIGN KEY (project_id, session_id) REFERENCES agent_sessions (project_id, session_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id)
);

CREATE INDEX idx_agent_sessions_connection
  ON agent_sessions (project_id, connection_internal_id);
CREATE INDEX idx_agent_sessions_open
  ON agent_sessions (project_id, connection_internal_id)
  WHERE ended_at IS NULL;
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
CREATE INDEX idx_unrecorded_changes_status
  ON unrecorded_changes (project_id, status, detected_at);
CREATE INDEX idx_unrecorded_changes_connection
  ON unrecorded_changes (project_id, connection_internal_id, status);
CREATE INDEX idx_unrecorded_changes_task
  ON unrecorded_changes (project_id, task_id, status);
CREATE TABLE expected_writes (
  project_id TEXT NOT NULL,
  expected_write_id TEXT NOT NULL,
  session_id TEXT,
  connection_internal_id TEXT NOT NULL,
  guard_installation_id TEXT,
  pre_tool_guard_event_id TEXT NOT NULL,
  host_invocation_id TEXT,
  tool_name TEXT,
  command_kind TEXT NOT NULL CHECK (length(trim(command_kind)) > 0),
  path_policy TEXT NOT NULL CHECK (path_policy IN ('exact_paths')),
  expected_paths_json TEXT NOT NULL DEFAULT '[]',
  task_id TEXT NOT NULL,
  change_unit_id TEXT,
  write_ticket_ids_json TEXT NOT NULL DEFAULT '[]',
  basis_state_version INTEGER NOT NULL CHECK (basis_state_version >= 0),
  status TEXT NOT NULL CHECK (status IN ('pending', 'matched')),
  matched_post_tool_guard_event_id TEXT,
  matched_paths_json TEXT,
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  matched_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, expected_write_id),
  CHECK (
    (
      status = 'pending'
      AND matched_post_tool_guard_event_id IS NULL
      AND matched_paths_json IS NULL
      AND matched_at IS NULL
    )
    OR (
      status = 'matched'
      AND matched_post_tool_guard_event_id IS NOT NULL
      AND matched_paths_json IS NOT NULL
      AND matched_at IS NOT NULL
    )
  ),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, session_id) REFERENCES agent_sessions (project_id, session_id),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id)
);

CREATE INDEX idx_expected_writes_pending_connection
  ON expected_writes (project_id, connection_internal_id, status, created_at);
CREATE INDEX idx_expected_writes_session
  ON expected_writes (project_id, session_id, status, created_at);
CREATE INDEX idx_expected_writes_host_invocation
  ON expected_writes (project_id, connection_internal_id, host_invocation_id, status)
  WHERE host_invocation_id IS NOT NULL;
CREATE INDEX idx_expected_writes_task
  ON expected_writes (project_id, task_id, status);
CREATE TABLE session_watch_baselines (
  project_id TEXT NOT NULL,
  watch_baseline_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  guard_installation_id TEXT,
  status TEXT NOT NULL CHECK (status IN ('disabled', 'active', 'degraded', 'unavailable')),
  scope_kind TEXT NOT NULL CHECK (scope_kind IN ('repository', 'path_set')),
  repo_root TEXT NOT NULL CHECK (length(trim(repo_root)) > 0),
  watched_paths_json TEXT NOT NULL DEFAULT '[]',
  exclusions_json TEXT NOT NULL DEFAULT '[]',
  snapshot_algorithm TEXT NOT NULL CHECK (length(trim(snapshot_algorithm)) > 0),
  snapshot_digest TEXT NOT NULL CHECK (length(trim(snapshot_digest)) > 0),
  snapshot_entries_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, watch_baseline_id),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id),
  FOREIGN KEY (project_id, session_id) REFERENCES agent_sessions (project_id, session_id)
);

CREATE TABLE session_watch_observations (
  project_id TEXT NOT NULL,
  watch_observation_id TEXT NOT NULL,
  watch_baseline_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  expected_write_id TEXT,
  unrecorded_change_id TEXT,
  observation_status TEXT NOT NULL CHECK (observation_status IN ('unresolved', 'linked')),
  observed_paths_json TEXT NOT NULL DEFAULT '[]',
  change_summary_json TEXT NOT NULL DEFAULT '{}',
  snapshot_algorithm TEXT NOT NULL CHECK (length(trim(snapshot_algorithm)) > 0),
  snapshot_digest TEXT NOT NULL CHECK (length(trim(snapshot_digest)) > 0),
  snapshot_entries_json TEXT NOT NULL DEFAULT '[]',
  observed_at TEXT NOT NULL,
  linked_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, watch_observation_id),
  CHECK (
    (
      observation_status = 'unresolved'
      AND unrecorded_change_id IS NULL
      AND linked_at IS NULL
    )
    OR (
      observation_status = 'linked'
      AND unrecorded_change_id IS NOT NULL
      AND linked_at IS NOT NULL
    )
  ),
  FOREIGN KEY (project_id, watch_baseline_id)
    REFERENCES session_watch_baselines (project_id, watch_baseline_id),
  FOREIGN KEY (project_id, session_id) REFERENCES agent_sessions (project_id, session_id),
  FOREIGN KEY (project_id, expected_write_id)
    REFERENCES expected_writes (project_id, expected_write_id),
  FOREIGN KEY (project_id, unrecorded_change_id)
    REFERENCES unrecorded_changes (project_id, unrecorded_change_id)
);

CREATE INDEX idx_session_watch_baselines_session
  ON session_watch_baselines (project_id, session_id, status);
CREATE INDEX idx_session_watch_baselines_status
  ON session_watch_baselines (project_id, status, updated_at);
CREATE INDEX idx_session_watch_observations_unresolved
  ON session_watch_observations (project_id, session_id, observation_status, observed_at);
CREATE INDEX idx_session_watch_observations_baseline
  ON session_watch_observations (project_id, watch_baseline_id, observed_at);
CREATE INDEX idx_session_watch_observations_expected_write
  ON session_watch_observations (project_id, expected_write_id)
  WHERE expected_write_id IS NOT NULL;
CREATE INDEX idx_session_watch_observations_unrecorded_change
  ON session_watch_observations (project_id, unrecorded_change_id)
  WHERE unrecorded_change_id IS NOT NULL;
CREATE TABLE local_web_consent_tokens (
  project_id TEXT NOT NULL,
  token_hash TEXT NOT NULL CHECK (length(token_hash) = 64),
  connection_internal_id TEXT NOT NULL,
  judgment_id TEXT NOT NULL,
  capture_basis TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending', 'consumed', 'expired')),
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  consumed_at TEXT,
  completed_at TEXT,
  created_metadata_json TEXT NOT NULL DEFAULT '{}',
  completion_metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, token_hash),
  FOREIGN KEY (project_id) REFERENCES project_state (project_id) ON DELETE RESTRICT,
  FOREIGN KEY (project_id, judgment_id)
    REFERENCES user_judgments (project_id, judgment_id)
    ON DELETE RESTRICT,
  CHECK (
    (
      status = 'pending'
      AND consumed_at IS NULL
      AND completed_at IS NULL
    )
    OR (
      status = 'consumed'
      AND consumed_at IS NOT NULL
      AND completed_at IS NOT NULL
    )
    OR (
      status = 'expired'
      AND consumed_at IS NULL
      AND completed_at IS NULL
    )
  )
);

CREATE INDEX idx_local_web_consent_tokens_judgment
  ON local_web_consent_tokens (project_id, judgment_id, status);
CREATE INDEX idx_local_web_consent_tokens_connection
  ON local_web_consent_tokens (project_id, connection_internal_id, status, expires_at);
CREATE INDEX idx_local_web_consent_tokens_expiry
  ON local_web_consent_tokens (project_id, status, expires_at);
```
<!-- canonical-storage-sql: project end -->

프로젝트 상태 제약:

- `project_state.state_version`은 기준 범위의 유일한 공개 상태 시계이며 [저장소 버전 관리](storage-versioning.md)에 따라 단조롭게 진행해야 합니다. 이것은 Core 상태 시계이지 스키마 버전이 아닙니다.
- `tasks.work_phase`와 `tasks.acceptance_policy`는 필수 제어 값이고 policy
  reason은 비어 있지 않습니다. Predecessor ID, lineage relation, 비어 있지
  않은 lineage reason은 모두 null이거나 모두 존재하고 외래 키로 같은
  프로젝트에 남으며 Task 자신을 가리킬 수 없습니다.
- `tasks.carry_forward_json`은 타입이 지정된 carry-forward disposition을
  저장합니다. 저장되어 있다는 사실만으로 predecessor 행, 판단, Evidence 집합,
  baseline, 쓰기 티켓을 현재 상태로 만들지 않습니다.
- `authority_events`는 커밋된 권한 이벤트마다 영속 이벤트 행 하나를 저장합니다. 같은 `state_version`을 가진 여러 이벤트 행은 하나의 커밋된 상태 전이에 속한 이벤트 배치입니다.
- `authority_events.actor_source`, `tasks.created_by_actor_source`, `user_judgments.requested_by_actor_source`, `user_judgments.resolved_by_actor_source`, `evidence_capture_intents.requested_by_actor_source`, `evidence_capture_receipts.observed_by_actor_source`, `user_evidence_observations.observed_by_actor_source`, `write_tickets.created_by_actor_source`, `runs.created_by_actor_source`, `artifact_staging.created_by_actor_source`, `evidence_observations.observed_by_actor_source`, `tool_invocations.actor_source`는 행위자 출처를 저장합니다.
- `authority_events.operation_category`와 `tool_invocations.operation_category`는 `read`, `agent_workflow`, `user_only`, `admin_local`, `local_recovery`로 제한됩니다.
- `authority_events.request_hash`는 커밋된 권한 이벤트의 요청 정체성을 저장합니다. `previous_event_hash`와 `event_hash`는 무결성 점검과 내보내기 상관을 위한 로컬 해시 체인을 저장하지만, 조작 방지 감사 보장을 뜻하지 않습니다.
- 사용자 판단 행은 권한 효력이 있는 판단 해결에 대한 User Channel 출처를 저장합니다. `status='resolved'`는 답변이 존재한다는 사실을 기록할 뿐이며, 승인 의미는 저장된 기계 동작, 결과, 근거, 출처, 메서드 담당 문서에서 나옵니다.
- `local_web_consent_tokens`는 대기 사용자 판단을 위한 해시된 일회성 로컬 웹 동의 토큰을 저장합니다. 원문 토큰은 저장하지 않습니다. 토큰 소비는 대응하는 사용자 판단 해결과 같은 프로젝트 상태 트랜잭션에서 커밋해야 합니다. 이 행은 임시 User Channel 캡처 메타데이터이며 그 자체로 Core 판단 권한이 아닙니다.
- `write_tickets`는 단일 사용 쓰기 티켓 호환성을 기록합니다. `write_tickets.consumed_by_run_id`와 `runs.write_ticket_id`의 고유 인덱스는 쓰기 티켓 소비 하나가 여러 실행으로 갈라지는 것을 막습니다.
- `artifact_staging.created_by_actor_source`는 스테이징 출처를 기록합니다. 스테이징된 바이트와 알림은 아티팩트 담당 상태이며 그 자체로 증거 권한이 아닙니다.
- `evidence_capture_intents`는 만료되는 요청 하나를 정확한 현재 근거, source input,
  connection/actor, workspace fact에 결합합니다. `evidence_capture_receipts`는 intent당
  완전하고 content-bound된 안전한 receipt 및 staging handle을 정확히 하나만
  허용합니다. `evidence_capture_source_claims`는 해당 receipt의 정규화된 host
  invocation, guard event, session-watch observation 원천 사실을 모두 같은
  트랜잭션에서 claim합니다. 프로젝트 범위 기본 키는 같은 원천 사실이 다른 intent나
  producer class를 충족하지 못하게 합니다. Host invocation claim id는 connection,
  session, installation, invocation 좌표를 정규화한 digest이므로 정확한 맥락이 다른
  host-local id가 충돌하지 않습니다. `evidence_producers`는 intent, receipt,
  observation, artifact의 일대일 finalization을 강제하고, 복합 외래 키로 intent가
  다른 receipt와 교차 결합되는 것을 막으며, producer를 Run 하나에 연결합니다. 이
  제약은 Core의 freshness, relevance, byte-integrity 검증을 대신하지 않습니다.
- Store 애플리케이션 검증은 capture class별로 누락되거나 추가된 source 좌표를
  거부합니다. Command receipt는 host invocation 하나를 요구하고 claim합니다. Tool
  receipt는 정확한 session과 installation, 서로 다른 guard event 두 개, host
  invocation 하나를 요구하며 세 원천 사실을 모두 claim합니다. Guard connection
  receipt는 guard event 하나만 claim하고 watcher receipt는 session-watch observation
  하나만 claim합니다. Receipt staging, receipt 삽입, 모든 claim은 함께 commit되거나
  rollback됩니다.
- Store application validation은
  `intent.created_at <= receipt.observed_at < intent.expires_at`, observation
  뒤이면서 intent expiry 전인 receipt 생성, intent expiry와 정확히 같은 receipt
  staging expiry를 강제합니다. 이런 cross-row 시간 관계는 table-local check만으로
  표현할 수 없습니다.
- `evidence_observations.source_kind`와 `assurance_level`은 협력적 에이전트 보고, 등록된 연결 관찰, 외부 도구 결과, 사용자 관찰, 재사용 증거, 미확인 주장을 구분합니다.
- `evidence_observations.metadata_json`은 엄격한 Core 파생 producer 앵커 및
  relevance 평가 JSON입니다. `user_evidence_observations`는 별도의 로컬 사용자
  relevance 레코드와 정확한 현재 근거 좌표를 저장합니다.
- `tool_invocations`는 행위자 출처, 작업 범주, 선택적 정규 `git_workspace_context_json`을 포함해 재실행 행을 저장합니다. 재실행 행은 호출자 권한이 아니며 현재 연결, Git 작업 공간, User Channel 요구사항을 우회하지 않습니다.
- `agent_sessions`, `guard_events`, `prompt_captures`, `expected_writes`, `unrecorded_changes`, `session_watch_baselines`, `session_watch_observations`는 프로젝트별 호스트 관찰 및 세션 감시 기록입니다. 연결 범위를 위해 `connection_internal_id`를 반복해 저장하고, 프로젝트별 키를 사용해 기록이 프로젝트 사이로 새지 않게 합니다.
- `guard_events.decision`은 `allow`, `deny`, `warn`, `inject_context`로 제한됩니다. 이 값은 로컬 호스트 판단 요청을 기록하며 OS 수준 집행 증명이 아닙니다.
- `expected_writes.status`는 `pending` 또는 `matched`로 제한되고, `path_policy`는 `exact_paths`로 제한됩니다. 일치한 행은 일치한 도구 실행 후 호스트 훅 이벤트, 일치 경로 JSON, `matched_at`을 가져야 하고, 대기 행은 이 일치 필드를 가지면 안 됩니다.
- `unrecorded_changes.status`는 `unresolved` 또는 `resolved`로 제한됩니다. 해결된 행은 해결 JSON, `resolved_at`, `resolved_by_actor_source`를 가져야 하고, 미해결 행은 이 해결 필드를 가지면 안 됩니다.
- `session_watch_baselines.status`는 `disabled`, `active`, `degraded`, `unavailable`로 제한되고, `scope_kind`는 `repository` 또는 `path_set`으로 제한됩니다.
- `session_watch_observations.observation_status`는 `unresolved` 또는 `linked`로 제한됩니다. 연결된 행은 `unrecorded_change_id`와 `linked_at`을 가져야 하고, 미해결 행은 이 연결 필드를 가지면 안 됩니다.

## 관련 담당 문서

- [저장소 기록](storage-records.md): 영속 기록 계열, 배치, 관계 배치, 저장소 소유 값, JSON 배치를 정의합니다.
- [저장 효과](storage-effects.md): 어떤 메서드 분기가 기록을 만들거나, 바꾸거나, 관찰하거나, 건드리지 않는지 정의합니다.
- [저장소 버전 관리](storage-versioning.md): `project_state.state_version` 시계, 멱등성, 재실행, 이벤트, 잠금, 호환되지 않는 저장소 처리를 정의합니다.
- [Agent Connection](agent-connection.md): Agent Connection, Connection Projects, 현재 연결 맥락, 모드 제한, Agent Connection과 User Channel의 경계를 정의합니다.
- [보안](security.md): 보안 경계와 보장 수준을 정의합니다.
