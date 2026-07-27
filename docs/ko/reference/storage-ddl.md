# 저장소 DDL

이 문서는 [저장소 기록](storage-records.md)이 설명하는 단일 기준 저장소 배치의 물리
SQLite DDL 계약을 담당합니다. `registry.sqlite`, 프로젝트 `state.sqlite`, 물리
`StorageManifest` 배치를 구현할 수 있게 하되, manifest 정체성, 데이터베이스 열기 분류,
메서드 효과, 아티팩트 생명주기 규칙, 상태 버전 의미, API 스키마, 보안 보장을 이 문서로
옮기지 않습니다.

## 담당 경계

이 문서가 담당합니다.

- `registry.sqlite`와 프로젝트 `state.sqlite`의 기준 SQLite 테이블 형태
- 기준 인덱스, 외래 키, view, 물리 제약
- 물리 `StorageManifest` 운반 열과 엄격한 영속 표현
- `project_state.state_version`, 재실행 행, 현재 적용 Change Unit 고유성, 쓰기 티켓 기준 버전, 스테이징된 아티팩트 출처, 호스트 관찰 기록에 대한 SQLite 제약
- Runtime Home 등록 데이터와 프로젝트별 Core 상태 사이의 DDL 수준 분리
- `GeneratedSchemaMetadata`와 문서 projection을 파생하는 기준 SQL 입력

이 문서는 담당하지 않습니다.

- 기록 계열 목적, 저장 위치, 저장소 소유 값, JSON 배치 범주: [저장소 기록](storage-records.md)
- 메서드 분기별 저장 효과: [저장 효과](storage-effects.md)
- 아티팩트 스테이징, 승격, 연결, 본문 읽기, 보존, 무결성 생명주기: [아티팩트 저장소](storage-artifacts.md)
- `StorageManifest` 의미적 정체성, digest 생성, capability 의미, 정확한 열기 비교,
  실패 분류, 상태 버전, 멱등성, 이벤트, 잠금 동작: [저장소 버전 관리](storage-versioning.md)
- API 요청 또는 응답 스키마: [API 코어 스키마](api/schema-core.md)가 안내하는 API 스키마 담당 문서
- 런타임 위치 경계: [런타임 경계](runtime-boundaries.md)
- 보안 보장 수준: [보안](security.md)

<a id="surface-stability"></a>
## 표면 안정성

라벨은 [문서 정책](../maintain/documentation-policy.md#surface-stability-labels)의
어휘를 사용합니다.

| 표면 | 안정성 | 비고 |
|---|---|---|
| 기준 SQLite DDL, manifest 운반 열, 기준 SQL 블록, 테이블 제약, 인덱스, view, 외래 키, 공개 상태 시계인 `project_state.state_version`, 물리 정규 UTC 하한인 `project_state.updated_at` | `stable` | 받아들이는 manifest 하나를 위한 구현 가능한 저장소 DDL 계약입니다. UTC 하한은 두 번째 공개 상태 버전 필드가 아닙니다. |
| 물리 테이블 이름, 열 이름, 내부 ID, 생성된 호스트 관찰 행, `_json` 표현 열 | `internal` | 저장소 배치를 구현 가능하게 하는 세부사항입니다. 다른 집중 담당 문서가 노출하지 않는 한 일반 사용자 대상 선택자나 공개 API 인자가 아닙니다. |
| 테이블, 기록 참조, 논리 열, 손상 범주를 식별하는 안전한 저장소 또는 손상 진단 | `diagnostic` | 진단은 원본 저장 JSON, 비밀값, SQL 텍스트, 민감한 절대 경로를 노출하면 안 됩니다. |

## 연결과 트랜잭션 요구사항

SQLite 외래 키는 이 DDL 계약의 일부입니다. 이 데이터베이스들을 읽거나 쓰는 모든 연결은 아래 설정을 활성화해야 합니다.

```sql
PRAGMA foreign_keys = ON;
```

프로덕션 코드는 활성 `RuntimeHomeMutationContext`를 요구하는 crate-private
helper를 통해서만 `registry.sqlite` 또는 project `state.sqlite`를 쓰기 가능 상태로
엽니다. Project record와 database 경로는 그 context의 정확한 정규 Runtime Home에
속해야 합니다. 읽기 전용 helper는 분리되어 있고 변경 context가 필요하지 않습니다.
Setup 전용 staging database 생성은 배타 setup context를 요구하며 bootstrap 내부에
머뭅니다.

상태 변경 커밋을 위해 최신성, 쓰기 티켓 호환성 행, 스테이징, 재실행 행, 영속
정규 UTC 하한을 읽는 변경 트랜잭션은 `BEGIN IMMEDIATE` 또는 동등한 직렬화된 쓰기
경계를 사용해야 합니다.

담당 저장소 계약이 복구나 보존 경로를 정의하지 않는 한 권한 효력이 있는 행은 계속 주소 지정 가능해야 합니다. 레지스트리는 프로젝트 등록을 삭제할 때 그 등록이 소유한 비권한 별칭 행을 연쇄 삭제할 수 있습니다. 이 별칭 정리가 프로젝트별 Core 권한 기록 삭제를 뜻하면 안 됩니다.

`_json`으로 끝나는 SQLite `TEXT` 열은 JSON을 저장하는 표현 선택입니다. 권한, 생명주기, 범위, 증거, 완료, 닫기 준비 상태, 쓰기 호환성에 쓰이는 JSON은 타입이 지정된 담당 상태입니다. 타입을 아는 Core 코드는 커밋 전에 해당 API 스키마 담당 문서, 저장소 담당 문서, 또는 아티팩트 담당 문서에 맞게 이 열을 파싱하고 검증해야 합니다. 타입이 지정된 담당 상태를 디코드하지 못하는 경우는 손상이며 빈 객체, 빈 배열, `false` 값, 기본 열거형 값, 또는 "요구사항 없음" 해석으로 바꾸면 안 됩니다. SQL `NULL`은 담당 스키마가 그 필드를 명시적으로 선택 필드라고 표시할 때만 부재를 뜻할 수 있습니다. 선택 열의 형식이 잘못된 JSON도 부재가 아니라 손상입니다. 열린 표시 메타데이터는 권한이나 닫기 판단에 쓰이지 않을 때만 타입을 지정하지 않은 채로 둘 수 있습니다. 안전한 진단은 테이블, 기록 참조, 논리 열, 손상 범주를 식별할 수 있지만 원본 저장 JSON, 비밀값, SQL 텍스트, 민감한 절대 경로를 노출하면 안 됩니다. `'{}'`, `'[]'` 같은 SQLite 기본값은 API 필드를 선택 필드로 만들지 않습니다.

`project_state.state_version`은 유일한 공개 상태 시계입니다.
`project_state.updated_at`은 정규 Core UTC 시계를 위한 별도의 물리 하한이며 공개 충돌
버전이나 저장소 형식 식별자가 아닙니다. 기준 SQLite DDL은 다른 공개 상태 시계를
노출하지 않습니다.

물리 `write_tickets` 테이블은 제품 파일 쓰기 시도와 유효 `sensitive` 통제 아래의
정확한 승인 결속 비제품 동작에 대한 권한 기록을 저장합니다. 이 행은 Volicord 안에서
경계가 정해진 권한 의도와 호환성 상태를 기록합니다. OS 권한, 파일시스템 ACL,
샌드박싱, 네트워크 정책, 비밀값 격리, 전역 파일시스템 가로채기, 효과가 실제로
일어났다는 증명이 아닙니다.

<a id="physical-storage-manifest-placement"></a>
## 물리 `StorageManifest` 배치

기준 SQL에는 별도 manifest 테이블이나 숫자 스키마 버전 열이 없습니다. 전체 manifest는
아래 두 기존 운반 열에 둡니다.

| 데이터베이스 | 소유 행 | 운반 열 | 정확한 DDL 형태 |
|---|---|---|---|
| `registry.sqlite` | `singleton_id=1`로 선택하는 `runtime_home` 행 | `runtime_home.storage_profile` | SQL 기본값이 없는 `TEXT NOT NULL` |
| 프로젝트 `state.sqlite` | 해당 데이터베이스의 `project_id`에 대한 `project_state` 행 | `project_state.storage_profile` | SQL 기본값이 없는 `TEXT NOT NULL` |

`storage_profile`은 완전한 현재 `StorageManifest`의 결정적인 단일 정규 UTF-8 JSON
인코딩을 저장합니다. 객체에는 `contract_id`, `canonical_ddl_digest`,
`integrity_constraints_digest`, `enabled_capabilities`만 정확히 있어야 합니다. 필드가
누락되거나 알 수 없는 필드 또는 중복 필드가 있으면 유효하지 않습니다. Capability 배열은
[저장소 버전 관리](storage-versioning.md)가 담당하는 완전하고 정렬되었으며
중복이 없는 집합을 보존해야 합니다.

새 초기화는 레지스트리 운반 열과 새로 만드는 모든 프로젝트 운반 열에 같은 현재 manifest
값을 씁니다. 새 Runtime Home Registry는 같은 상위 directory의 staging directory에서만
만듭니다. Singleton과 최초 installation row를 commit하고 정확한 DDL inventory 및 manifest
carrier를 검증한 뒤에만 directory를 기존 대상을 교체하지 않는 원자적 방식으로 공개합니다.
Store는 권한 또는 정책 기록을 읽기 전에 각 운반 열을 독립적으로 엄격하게 디코드합니다.
영속 값이 현재 내장 manifest와 같고, 선택한 프로젝트 manifest가 레지스트리 manifest와
같아야 합니다. 정수를 파싱하거나, 버전을 비교하거나, 필드 존재 여부로 decoder를 고르거나,
다른 프로필을 시도하지 않습니다. 기존 carrier 검사는 읽기 전용이며 호환되지 않는
데이터베이스를 보존합니다. 정확한 열기 결과와 실패 범주는
[저장소 버전 관리](storage-versioning.md)가 담당합니다.

현재 프로젝트 schema는 source를 구분하는 host 상관관계를 `host_sessions`, `host_turns`,
`host_tool_invocations`, MCP 전용 `managed_mcp_sessions` table로 정규화합니다. 이 table의
제약과 phase discriminator가 있는 Guard column은 두 현재 schema digest 모두에 포함됩니다.
엄격한 open은 완전한 현재 manifest identity만 허용합니다.

Application은 저장 전에 서로 다른 `CodexMcpTurnMetadata`와 `CodexCommandHooks` marker를
선택하며 host-contract 담당자가 이 marker를 검토된 profile ID에 연결합니다. DDL은 그
결과인 한도가 있고 source로 구분된 좌표와 소유 profile/digest field만 저장합니다. Raw host
envelope를 저장하거나 column 존재 여부에서 decoder를 추론하지 않습니다.

## 기준 SQL 원본

실행 가능한 DDL 원본은 고정된 순서의
[`registry.sql`](../../../crates/volicord-store/src/schema/registry.sql)과
[`project.sql`](../../../crates/volicord-store/src/schema/project.sql)뿐입니다. 새 초기화는 비어
있는 SQLite 데이터베이스에만 이 원본을 적용합니다. 기존 데이터베이스는 정확한 현재
manifest와 물리 schema 검증을 통과할 때만 받아들입니다.

이 두 원본 파일은 바이트가 정확히 일치해야 하는 텍스트 계약입니다. 저장소의 기준 형식은
LF 바이트만 사용하고 끝에 LF 하나만 둡니다. 루트 `.gitattributes` 규칙은 Git 클라이언트의
줄바꿈 설정과 관계없이 Linux, macOS, native Windows, WSL2 체크아웃에서 같은 바이트를
강제합니다. `include_str!`가 `GeneratedSchemaMetadata`를 파생하는 정확한 원본 바이트를
포함하므로 스키마 줄바꿈 바이트를 바꾸면 `canonical_ddl_digest`,
`integrity_constraints_digest`, 그 결과인 `StorageManifest` 식별값이 바뀝니다. CRLF를
사용한 기준 스키마 원본은 유효하지 않으며 거부해야 합니다. 소비자는 스키마 식별값을
파생하기 전에 줄바꿈을 정규화하거나, CRLF를 치환하거나, 임의의 공백을 잘라 내지
않습니다. 기준 SQL을 변경할 때는 엄격한 바이트, 격리 체크아웃, 생성 메타데이터, 고정
digest 계약 테스트를 모두 통과해야 합니다.

결정적 추출기는 이 파일에서 테이블, 열, 인덱스, 제약, 두 스키마 digest를 파생해 하나의
공유 `GeneratedSchemaMetadata`를 만듭니다. 런타임 검증, manifest 생성, Store query
projection, fixture, DDL 계약 테스트, 문서 목록은 이 생성 아티팩트를 사용합니다. 어느
소비자도 두 번째 권위 있는 목록을 유지하지 않습니다.

아래 기준 SQL 블록은 점검되는 문서 projection이며 두 번째 DDL 원본이 아닙니다.
`docs-check`는 블록이 원본 파일과 정확히 같은지 요구하고, 집중 `storage_ddl_contract`
테스트는 실행 가능한 스키마 의미를 검증합니다. 기준 SQL에 없는 테이블, 열, 인덱스, view,
외래 키, `CHECK`, `UNIQUE`, 기본값, 그 밖의 물리 SQLite 객체는 받아들이는 배치에 포함되지
않습니다.

## `registry.sqlite`

`registry.sqlite`는 Runtime Home 식별 정보, 설치 프로필 기록, 프로젝트 등록, 프로젝트 별칭, Agent Connection 기록, Connection Projects 멤버십, 구조화된 진단 finding과 원인 edge, 권위 있는 MCP runtime session과 프로젝트 예약, 한도가 있는 채팅 내 통합 검증 run, 호스트 훅 설치 기록, 호스트 설정 목록을 저장합니다. 프로젝트별 Core 상태는 저장하지 않습니다.

<!-- canonical-storage-sql: registry start -->
```sql
CREATE TABLE runtime_home (
  singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
  runtime_home_id TEXT NOT NULL UNIQUE,
  publication_id TEXT NOT NULL UNIQUE CHECK (
    length(publication_id) = 61
    AND substr(publication_id, 1, 25) = 'runtime_home_publication_'
    AND substr(publication_id, 34, 1) = '-'
    AND substr(publication_id, 39, 1) = '-'
    AND substr(publication_id, 44, 1) = '-'
    AND substr(publication_id, 49, 1) = '-'
    AND substr(publication_id, 26, 8) NOT GLOB '*[^0-9a-f]*'
    AND substr(publication_id, 35, 4) NOT GLOB '*[^0-9a-f]*'
    AND substr(publication_id, 40, 4) NOT GLOB '*[^0-9a-f]*'
    AND substr(publication_id, 45, 4) NOT GLOB '*[^0-9a-f]*'
    AND substr(publication_id, 50, 12) NOT GLOB '*[^0-9a-f]*'
    AND substr(publication_id, 40, 1) = '4'
    AND substr(publication_id, 45, 1) GLOB '[89ab]'
  ),
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
      'unrelated_routed_tool',
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
```
<!-- canonical-storage-sql: registry end -->

레지스트리 제약:

- `runtime_home`은 단일 행 테이블입니다. `storage_profile` 열은 필수 manifest 운반 열이며 완전한 현재 `StorageManifest`를 저장합니다. 이 행은 Runtime Home 식별 정보, 고유한 `runtime_home_publication_` 접두 소문자 UUIDv4 publication provenance, Runtime Home 경로, 레지스트리 데이터베이스 경로, 메타데이터, 타임스탬프도 저장합니다. Publication ID는 준비 invocation 하나를 식별하며 credential, OS actor identity, 숫자 schema selector가 아닙니다. 저장된 `runtime_home_id`는 Runtime Home 기록을 식별하며 보안 보장이 아닙니다.
- `installation_profile`은 Runtime Home에 대해 선택된 `volicord` 명령, MCP 시작 명령, 실행 파일 디렉터리, 기본 연결 모드, 메타데이터, 타임스탬프를 저장합니다. `volicord init`이 이를 마련할 수 있습니다. 호스트 신뢰, 사용자 권한, 공개 API 상태가 아닙니다.
- `projects.project_internal_id`는 프로젝트 기록의 저장 기본 키입니다. `projects.project_name`은 표시 이름입니다. `projects.project_alias`는 CLI 선택 보조 값입니다. `projects.repo_root`는 저장소 루트 조회 키입니다. `projects.project_alias`, `projects.repo_root`, `projects.project_home`, `projects.state_db_path`는 고유합니다.
- `project_aliases`는 별칭을 `project_internal_id` 값에 매핑합니다. 별칭 행은 레지스트리 선택 보조 값이지 프로젝트별 Core 권한 기록이 아닙니다.
- `projects.state_db_path`는 저장 열로 유지됩니다. Store의 애플리케이션 수준 현재 등록 검증은 운영 `ProjectRecord` 조회나 목록 조회, 쓰기 가능한 프로젝트 상태 열기, Agent Connection 프로젝트 처리 경로, Core 실행, 프로젝트 Store 재사용, MCP 프로젝트 가용성 확인 전에 이 값이 `project_home/state.sqlite`와 같은지 확인해야 합니다.
- `projects.status`는 저장소 소유 값이며 유효한 값은 `active`뿐입니다.
- `agent_connections.connection_internal_id`는 Agent Connection 기록의 저장 기본 키입니다. 이 테이블은 고유하고 변경 불가능한 Store 생성 `integration_instance_id`, 호스트 종류, `intent`에 저장되는 연결 의도, 호스트 범위, 선택적 `project_internal_id`, 서버 이름, 설정 대상, 모드, 활성 상태, 관리 지문, Store 소유 integration generation, 선택적 정규 검증 보고서 JSON 값, 메타데이터, 타임스탬프를 저장합니다.
- `agent_connections.integration_instance_id`는 새 물리 행을 만들 때만 생성하는 엄격한 `connection_instance_` 접두 UUIDv4 lifecycle 좌표입니다. 고유 index는 현재 행 사이의 충돌을 막고 `agent_connections_integration_instance_immutable`은 갱신 시도를 거부합니다. 호환 replay와 모든 제자리 lifecycle 갱신은 이 값을 보존합니다. 물리 행 삭제는 이 값을 제거하며, 같은 결정적 Connection identity를 다시 만들어도 새 값을 받습니다.
- `agent_connections.intent`는 현재 `host_kind=codex` 계약에서 `personal` 또는 `shared`로 제한됩니다.
- 현재 Codex connection 계약은 `host_kind=codex`를 사용하고 connection intent에 따라 `host_scope`로 `user` 또는 `project`를 사용합니다.
- `agent_connections.mode`는 `read_only` 또는 `workflow`로 제한됩니다.
- `agent_connections.integration_generation`은 Connection integration revision의 Store 소유 비음수 입력입니다. 실제 mode 전환이 성공하면 mode와 소유한 모든 Guard manifest를 갱신하는 같은 Registry transaction에서 정확히 한 번 증가합니다. 같은 mode를 지정한 no-op에서는 증가하지 않습니다.
- Integration generation은 물리 Connection instance 하나 안의 revision을 구분하고 `integration_instance_id`는 물리 삭제와 재생성을 구분합니다. 두 값은 Store 소유 로컬 lifecycle 및 상관관계 좌표이며 호출자가 선택할 수 없습니다.
- `agent_connections.verification_report_json`은 완료된 보고서가 없으면 SQL null입니다. Null이 아닌 값은 파생 상태와 action을 포함하는 엄격한 정규 `ConnectionVerificationReport` 하나를 저장하며 값이 없는 선택 구성원은 명시적 null 대신 생략합니다. Store는 그 구성 요소를 독립적으로 영속 저장하지 않습니다.
- `connection_projects`는 Agent Connection 하나에 대한 명시적 프로젝트 허용 목록입니다. `connection_internal_id`와 `project_internal_id`로 멤버십을 저장합니다. 아직 멤버십이 남은 프로젝트나 연결 삭제는 제한됩니다.
- `managed_mcp_launch_leases`는 수명이 짧고 한 번만 쓰는 숨겨진 launcher 권한을 저장합니다. 예상 Connection, `codex` host kind, integration revision, managed launch fingerprint는 Store가 `issued`를 `consumed`로 바꾸고 `managed_host` runtime을 삽입하는 원자적 transaction 시점에도 현재 상태여야 합니다. Replay, 만료, 불일치, 취소는 runtime을 만들 수 없습니다. 한도가 있는 cleanup은 오래된 row를 만료 처리하거나 제거합니다. Lease는 evidence-integrity 좌표이지 OS actor credential이 아닙니다.
- `diagnostic_findings.lifecycle`은 정확히 `occurrence` 또는 `current_state`입니다. Occurrence row에는 current identity 및 status field가 없고 변경할 수 없습니다. Current row에는 64자 소문자 전체 identity digest, 검증된 `sha256:<64 lowercase hex>` `current_subject_identity`, scope kind와 완전한 scope identity, active/resolved status가 필요하고 runtime-session 좌표가 없어야 하며, ID는 정확히 `finding.current.sha256:`와 해당 digest를 이어 붙인 값이어야 합니다. Active row에는 `resolved_at`이 없고 resolved row에는 반드시 있어야 합니다. Unique digest index, active-scope index, lifecycle check, identity update trigger가 이 물리적 구분을 강제합니다. Trigger는 subject identity를 변경할 수 없게 유지하면서 `subject_json`은 교체 가능한 안전한 표시로 갱신할 수 있게 합니다. `facts_json`은 계속 16,384 byte 이하의 유효한 JSON object이며 `subject_json`과 `actions_json`도 한도가 있는 typed 표현입니다.
- `diagnostic_cause_edges`는 양 끝에 foreign key가 있는 고유한 finding-to-cause 쌍을 저장합니다. `diagnostic_cause_edges_acyclic`은 방향성 cycle을 완성하는 insert를 거부하고, cause-side index는 결정적인 역방향 조회와 제한된 순회를 지원합니다. 현재 상태 finding을 교체할 때는 이전 outgoing edge를 삭제하고 대체 edge를 row 교체와 같은 immediate transaction에서 삽입하며, 실패하면 이전 row와 edge 집합을 보존합니다.
- `mcp_runtime_sessions.attempted_client_name`과 `attempted_client_version`은 한도가 있는 파싱된 client 쌍입니다. `requested_protocol_version`은 client 입력이고 `selected_protocol_version`은 server가 선택한 initialize 결과이며, `negotiated_protocol_version`은 handshake 완료와 함께 있을 때만 존재하고 선택 revision과 같아야 합니다. `initialize_completed_at`, `initialized_notification_at`, `tools_list_observed_at`은 서로 구분되는 lifecycle milestone이며, `tools/list`는 initialize 완료 뒤 initialized notification보다 먼저 올 수 있습니다. `returned_tool_identities_json`은 해당 list observation의 정규 exact inventory이고, required set 검증에 성공한 경우에만 `required_tools_validated_at`이 존재합니다. 한도가 있는 MCP 도구 이름 `verification_tool_name`과 `verification_tool_observed_at`은 정확한 null-or-present 쌍이며, observation에는 같은 session의 required-tool validation이 필요하고 그보다 앞설 수 없습니다. `terminal_finding_id`는 같은 runtime의 구조화된 error finding 하나를 가리키는 foreign key이며 graceful close와 함께 있을 수 없습니다.
- `mcp_runtime_sessions.session_source`는 정확히 `managed_host`, `manual_cli`, `cli_preflight`, `integration_probe` 중 하나입니다. Lease-consumption transaction만 `managed_host`를 삽입할 수 있고 managed-session 조회는 나머지 세 값을 제외합니다.
- `guard_installations`는 프로젝트 범위의 안정적인 Guard 설치 identity 하나와 정규 typed Guard manifest를 저장합니다. Manifest는 row, Agent Connection, 프로젝트, 현재 integration revision, policy hash, runtime command, 전체 managed-file inventory, 필수 hook phase, 정확한 `host_contract_profile`, 결정적인 `host_contract_digest`에 결속됩니다. 현재 Guard 선택은 `codex-command-hooks`입니다. 파일 상태는 manifest와 현재 파일을 audit해 도출하고, 관찰 상태는 모든 필수 phase의 호환되는 현재 소유 `guard_events`를 요구합니다. 이 협력적 check는 OS 수준 집행이나 쓰기 방지를 제공하지 않습니다.
- `guard_integration_verification_runs`는 Connection, project, 현재 MCP runtime, native session과 turn, integration revision, Guard Installation, host-contract profile, hook-definition digest, policy digest로 이루어진 완전한 semantic 좌표마다 불변 managed-host attempt 하나를 저장합니다. 무조건 unique index는 terminal row도 포함하며 prompt 소유권은 별도 attempt가 prompt event 하나를 공유하지 못하게 합니다. Row는 semantic observation policy, bounded status-read 횟수, cleanup 경계, first-write acknowledgement, 일치한 event, 폐쇄형 상태, typed repair/retry field도 저장합니다. Coordinate, acknowledgement, terminal trigger는 identity 변경, 두 번째 acknowledgement, terminal 재활성화, terminal 교체를 막습니다. 현재 Codex semantic 계약은 허용 status read가 하나인 영속 synchronous policy를 사용합니다. `cleanup_after`는 보관 metadata이며 attempt expiry, polling 시간, retry eligibility가 아닙니다.
- `guard_probe_observations`는 폐쇄형 acquisition stage, 예상 agent-tool/callable identity, 선택적인 한도 내 관찰 callable, 선택적인 hook kind, verification ID의 존재 및 일치 flag, Guard Installation, integration revision, 관찰 시각만 저장합니다. Prompt나 제한 없는 hook/tool payload는 저장할 수 없습니다. Foreign key는 각 관찰을 하나의 verification run과 현재 installation에 결속하며, `hook_event_not_observed`는 Volicord 경계에서의 부재만 기록합니다. `unrelated_routed_tool`은 nonterminal trace이며 repair reason, proof, acknowledgement, retry 입력, root finding, status-read-budget effect가 아닙니다.
- 명시적 제거 또는 replacement 정리에 따른 Connection Project 폐기는 immediate transaction 하나에서 소유자 순서로 삭제하여 제한적인 Registry foreign key를 충족합니다. 선택한 project-session binding과 integration-verification run을 선택한 Guard Installation과 membership보다 먼저 삭제합니다. 여러 프로젝트가 있는 replacement 정리는 관련 없는 프로젝트 행과 connection 전체 runtime session을 유지합니다. 마지막 프로젝트 replacement 정리는 host 정리와 최종 재검증이 성공할 때까지 비활성 membership, binding, Guard Installation, pending-cleanup marker의 완전한 inventory를 유지한 뒤 프로젝트 소유 행과 membership만 삭제합니다. 명시적으로 마지막 membership을 제거할 때는 Connection 소유의 남은 binding, integration-verification run, Guard Installation을 모두 삭제한 뒤 `mcp_runtime_sessions`, `managed_mcp_launch_leases`, `agent_connections` 순서로 삭제하며, 구조화된 finding은 영속 이력 진단으로 남습니다. 어떤 경로도 `projects`, `runtime_home`, `installation_profile`, 프로젝트 `state.sqlite` 데이터베이스로 cascade하지 않습니다.

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
    source_method IN ('volicord.request_user_action', 'volicord.reconcile_changes')
  ),
  source_idempotency_key TEXT NOT NULL CHECK (length(trim(source_idempotency_key)) > 0),
  requested_at TEXT NOT NULL,
  expires_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, user_action_request_id),
  UNIQUE (project_id, user_action_request_id, action_kind),
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
  UNIQUE (project_id, channel_kind, channel_submission_id),
  FOREIGN KEY (project_id, user_action_request_id, action_kind)
    REFERENCES user_action_requests (
      project_id,
      user_action_request_id,
      action_kind
    )
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
  session_id TEXT,
  connection_internal_id TEXT NOT NULL,
  correlation_kind TEXT CHECK (correlation_kind = 'codex_hook_tool'),
  host_turn_id TEXT,
  host_tool_use_id TEXT,
  host_tool_name TEXT,
  task_id TEXT,
  status TEXT NOT NULL CHECK (status IN ('unresolved', 'resolved')),
  confidence TEXT NOT NULL CHECK (confidence IN ('confirmed', 'suspected')),
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
      correlation_kind IS NULL
      AND session_id IS NULL
      AND host_turn_id IS NULL
      AND host_tool_use_id IS NULL
      AND host_tool_name IS NULL
    )
    OR (
      correlation_kind = 'codex_hook_tool'
      AND session_id IS NOT NULL
      AND host_turn_id IS NOT NULL
      AND host_tool_use_id IS NOT NULL
      AND host_tool_name IS NOT NULL
    )
  ),
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
  FOREIGN KEY (
    project_id, session_id, connection_internal_id, host_turn_id,
    host_tool_use_id, host_tool_name
  ) REFERENCES host_tool_invocations (
    project_id, session_id, connection_internal_id, host_turn_id,
    host_tool_use_id, host_tool_name
  ),
  FOREIGN KEY (project_id, task_id) REFERENCES tasks (project_id, task_id)
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
CREATE INDEX idx_unrecorded_changes_status
  ON unrecorded_changes (project_id, status, detected_at);
CREATE INDEX idx_unrecorded_changes_connection
  ON unrecorded_changes (project_id, connection_internal_id, status);
CREATE INDEX idx_unrecorded_changes_task
  ON unrecorded_changes (project_id, task_id, status);
CREATE TABLE expected_writes (
  project_id TEXT NOT NULL,
  expected_write_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  connection_internal_id TEXT NOT NULL,
  correlation_kind TEXT NOT NULL CHECK (correlation_kind = 'codex_hook_tool'),
  host_turn_id TEXT NOT NULL,
  host_tool_use_id TEXT NOT NULL,
  host_tool_name TEXT NOT NULL,
  guard_installation_id TEXT,
  pre_tool_guard_event_id TEXT NOT NULL,
  host_invocation_id TEXT,
  tool_name TEXT,
  command_kind TEXT NOT NULL CHECK (length(trim(command_kind)) > 0),
  path_policy TEXT NOT NULL CHECK (path_policy IN ('exact_paths')),
  expected_paths_json TEXT NOT NULL DEFAULT '[]',
  task_id TEXT NOT NULL,
  change_unit_id TEXT NOT NULL,
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
  FOREIGN KEY (
    project_id, session_id, connection_internal_id, host_turn_id,
    host_tool_use_id, host_tool_name
  ) REFERENCES host_tool_invocations (
    project_id, session_id, connection_internal_id, host_turn_id,
    host_tool_use_id, host_tool_name
  ),
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

```
<!-- canonical-storage-sql: project end -->

프로젝트 상태 제약:

- `project_state.storage_profile`은 필수 프로젝트 manifest 운반 열입니다. `runtime_home.storage_profile`과 같은 완전한 현재 `StorageManifest`를 저장합니다. 엄격한 애플리케이션 검증은 다른 형식을 고르는 대신 누락되었거나, 형식이 잘못되었거나, 현재와 다르거나, 레지스트리와 불일치하는 manifest를 거절합니다.
- `project_state.state_version`은 유일한 공개 상태 시계이며 [저장소 버전 관리](storage-versioning.md)에 따라 단조롭게 진행해야 합니다. 이것은 Core 상태 시계이지 스키마 버전이 아닙니다.
- `project_state.updated_at`은 정규 Core UTC 시계의 감소하지 않는 영속 하한입니다.
  Store application validation은 이 값을 정규 UTC 담당 상태로 strict parse하고 형식이
  잘못되면 닫힌 상태로 실패해야 합니다. 일반 Core 커밋은 정확히 하나의
  `committed_at` 값을 이 열, 이벤트 묶음의 모든 `authority_events.created_at`, 선택적
  재실행 행의 `tool_invocations.created_at`에 씁니다. Mutation application이 생성하는
  적용 가능한 Store transaction metadata인 `created_at`, `updated_at`, `retired_at`,
  `promoted_at`도 정확히 같은 값을 사용합니다. 의미 있는 동작 시각인
  `requested_at`, `resolved_at`, `closed_at`, `recorded_at`, `consumed_at`과 별도 담당
  사실인 `observed_at`, `started_at`은 담당 문서가 정의한 동작 샘플 또는 검증된 원천
  시각을 보존합니다. 이러한 교차 행 및 단조 제약은 transaction 요구사항이며
  table-local `CHECK` 제약만으로 표현되지 않습니다.
- Store application validation은 timestamp 열에 쓰는 모든 값을 정규 RFC 3339 UTC 담당
  상태로 strict validation합니다. TTL 파생 값은 checked 덧셈과 표현 가능성을 요구하며
  overflow 또는 표현 불가능한 값은 행, 하한, event, replay, state-version 효과 전에
  거부합니다.
- 최신 managed MCP session 또는 Guard event를 도출하는 Store 조회는 적용되는 RFC 3339
  timestamp를 strict parse하고 정규화한 뒤 UTC
  instant를 nanosecond 정밀도로 비교합니다. SQLite `julianday()`, timestamp 텍스트 순서,
  행 순서, 불투명 ID가 권한 순서를 정하면 안 됩니다. 같은 최대 instant는 함께 최신인
  집합입니다. 조회가 session 또는 Guard event 하나를 요구하는데 서로 다른 후보 여러
  개가 함께 최신이면 ID를 tie-breaker로 사용하지 않고 사용할 수 없는 담당 상태로 닫힌
  상태에서 실패합니다.
- `tasks.work_phase`와 `tasks.acceptance_policy`는 필수 제어 값이고 policy
  reason은 비어 있지 않습니다. Predecessor ID, lineage relation, 비어 있지
  않은 lineage reason은 모두 null이거나 모두 존재하고 외래 키로 같은
  프로젝트에 남으며 Task 자신을 가리킬 수 없습니다.
- `tasks.carry_forward_json`은 타입이 지정된 carry-forward disposition을
  저장합니다. 저장되어 있다는 사실만으로 predecessor 행, 판단, Evidence 집합,
  baseline, 쓰기 티켓을 현재 상태로 만들지 않습니다.
- `authority_events`는 커밋된 권한 이벤트마다 영속 이벤트 행 하나를 저장합니다. 같은 `state_version`을 가진 여러 이벤트 행은 하나의 커밋된 상태 전이에 속한 이벤트 배치입니다. `task_id`는 정확한 프로젝트 범위 `project_workflow_policy_applied` 이벤트를 제외하면 필수이고, 이 예외 이벤트는 `change_unit_id`도 null이어야 합니다. Task 범위 조회는 `task_id`가 null이 아닌 `authority_events` 행을 선택합니다.
- `authority_events.actor_source`, `tasks.created_by_actor_source`, `user_action_requests.requested_by_actor_source`, `user_action_resolutions.resolved_by_actor_source`, `evidence_capture_intents.requested_by_actor_source`, `evidence_capture_receipts.observed_by_actor_source`, `write_tickets.created_by_actor_source`, `runs.created_by_actor_source`, `artifact_staging.created_by_actor_source`, `evidence_observations.observed_by_actor_source`, `tool_invocations.actor_source`는 행위자 출처를 저장합니다.
- `authority_events.operation_category`와 `tool_invocations.operation_category`는 `read`, `agent_workflow`, `user_only`, `admin_local`, `local_recovery`로 제한됩니다.
- `authority_events.request_hash`는 커밋된 권한 이벤트의 요청 정체성을 저장합니다. `previous_event_hash`와 `event_hash`는 무결성 점검과 내보내기 상관을 위한 로컬 해시 체인을 저장하지만, 조작 방지 감사 보장을 뜻하지 않습니다.
- `user_action_requests`는 합성 lifecycle 상태를 저장하지 않습니다. Core가 resolution 존재 여부, `basis_status`, expiry, 현재 시각에서 유효한 `pending`, `resolved`, `stale`, `superseded`, `expired`를 파생합니다. `user_action_resolutions`는 변경 불가능하고 요청과 일대일입니다. 폐쇄형 `resolution_json`은 저장 선택지에서 파생한 choice action/outcome 또는 Core가 도출한 전체 Evidence 관찰 본문 중 하나를 담으며, 권한 의미에는 현재 근거, provenance, 메서드 담당자도 필요합니다.
- `user_action_resolutions.channel_submission_id`는 visible ASCII
  `0x21..=0x7e` 1~256 bytes로 제한됩니다. BLOB 길이가 byte 상한을 제공하고, 같은
  TEXT/BLOB 길이가 non-ASCII와 내장 NUL 형태를 거부하며, `GLOB` 검사가 visible 범위 밖의
  모든 byte를 거부합니다. Core도 replay 조회나 mutation 계획 전에 같은 상한을
  적용합니다.
- Store application validation은 요청, 근거, resolution JSON을 strict decode하고 저장 요청에서 adapter-neutral resolution form을 도출하며 폐쇄형 tag와 파생 `action_kind` 일치를 요구합니다. 대상과 아티팩트 후보는 각각 32개, note 성격 텍스트는 Unicode scalar value 1,000개, 관찰 summary는 4,000개, canonical 직렬화 폼은 32 KiB로 제한하며 초과 값을 자르지 않고 거부합니다.
- `write_tickets`는 소비 전까지 재사용 가능하고 상태에 묶인 호환성을 기록합니다. `basis_state_version`은 감사 순서이며 고유하거나 유효성 좌표가 아닙니다. 유효성은 `validity_basis_json`, status, 안정된 무효화 사유, 선택적 `idle_expires_at`에서 나옵니다. 고유 소비 인덱스는 소비 하나가 여러 Run으로 갈라지는 것을 계속 막습니다. Prefix 배열은 엄격하게 정규화된 repository-relative exact-or-descendant prefix이며 glob 문법 없음, 절대/빈 값/`..`/모호한 항목 거절, denied 우선, allowed 빈 배열은 product-file 쓰기 없음 규칙을 적용합니다.
- `project_workflow_policies`는 권위 있는 정규 데이터베이스 복사본과 `sha256:<64자리 소문자 16진수>` 지문만 저장하며 관리 파일/CLI/host 동작은 이 담당 문서 밖입니다. 변경된 fingerprint는 정확한 트랜잭션 `committed_at`, 한 번의 상태 버전 전진, 프로젝트 범위 정책 이벤트 하나와 함께 기록됩니다. 정규화된 쓰기 권한 fingerprint가 바뀌면 같은 트랜잭션에서 같은 수준의 권한 변경을 포함해 활성 Task 재평가 metadata 표시를 만들거나 갱신하고, 저장 결속이 없거나 다른 모든 활성 티켓과 표시된 Task의 모든 활성 티켓을 `explicit_revoke`로 무효화합니다. 개인정보를 제한한 workflow metric은 이 권한 데이터베이스가 아니라 별도의 비권한 `diagnostics.sqlite` 저장소에만 둡니다.
- `artifact_staging.created_by_actor_source`는 스테이징 출처를 기록합니다. 스테이징된 바이트와 알림은 아티팩트 담당 상태이며 그 자체로 증거 권한이 아닙니다.
- `evidence_capture_intents`는 만료되는 요청 하나를 정확한 현재 근거,
  verified-command 또는 verified-tool source input, connection/actor, workspace fact에
  결합합니다. `evidence_capture_receipts`는 intent당
  완전하고 content-bound된 안전한 receipt 및 staging handle을 정확히 하나만
  허용합니다. `evidence_capture_source_claims`는 해당 receipt가 사용한 정규화된 각 host
  invocation을 같은 트랜잭션에서 claim합니다. 프로젝트 범위 기본 키는 같은 원천 사실이 다른 intent나
  producer class를 충족하지 못하게 합니다. Host invocation claim ID는 정확한 connection과
  invocation 좌표를 정규화한 digest이므로 정확한 맥락이 다른
  host-local id가 충돌하지 않습니다. `evidence_producers`는 intent, receipt,
  observation, artifact의 일대일 finalization을 강제하고, 복합 외래 키로 intent가
  다른 receipt와 교차 결합되는 것을 막으며, producer를 Run 하나에 연결합니다. 이
  제약은 Core의 freshness, relevance, byte-integrity 검증을 대신하지 않습니다.
- Store 애플리케이션 검증은 모든 command 또는 tool receipt와 producer에 정확한
  `connection_id`와 nullable `host_invocation_id`가 있는지 요구합니다. Null이 아닌
  invocation ID에는 정확히 하나의 일치하는 배타적 host-invocation claim이 필요하고,
  null이면 invocation claim을 만들지 않습니다. 선택한 invocation은 intent 뒤에 생성되어야
  하며 receipt가 확정한 identifier, timestamp, digest는 모두 해당 source와 일치해야
  합니다. Receipt staging, receipt 삽입, 모든 claim은 함께
  commit되거나 rollback됩니다.
- Store application validation은
  `intent.created_at <= receipt.observed_at < intent.expires_at`, observation
  뒤이면서 intent expiry 전인 receipt 생성, intent expiry와 정확히 같은 receipt
  staging expiry를 강제합니다. 이런 cross-row 시간 관계는 table-local check만으로
  표현할 수 없습니다.
- Store application validation은 일반 staging을 만드는 transaction에서
  `project_state.updated_at`을 `artifact_staging.created_at` 이상으로 전진시킵니다.
  Evidence-capture fulfillment도 receipt, staging 행, source claim을 만드는
  transaction에서 receipt `created_at`을 기준으로 같은 효과를 적용합니다. 이러한 하한
  전용 효과는 `state_version`을 증가시키거나 이벤트 또는 재실행 행을 만들지 않습니다.
- `evidence_summaries.produced_at_state_version`은 삽입 또는 가장 최근 갱신의 결과
  권한 상태 버전을 저장합니다. 고유한
  `(project_id, task_id, produced_at_state_version)` 제약은 같은 `Task`의 요약 두 개가
  하나의 권한 순서 위치를 주장하지 못하게 합니다. 현재 요약 선택은 이 열만 사용하며
  timestamp와 불투명 ID는 권한 순서 tie-breaker가 아닙니다.
- `evidence_observations.source_kind`와 `assurance_level`은 협력적 에이전트 보고, 등록된 연결 관찰, 외부 도구 결과, 사용자 관찰, 재사용 증거, 미확인 주장을 구분합니다.
- `evidence_observations.metadata_json`은 엄격한 Core 파생 producer 앵커 및
  relevance 평가 JSON입니다. 사용자 행동의 로컬 사용자 relevance detail과 정확한
  현재 근거 좌표는 폐쇄형 `user_action_resolutions.resolution_json` Evidence 관찰
  본문에 남습니다.
- `tool_invocations`는 정확한 검증된 행위자 출처, 작업 범주, 선택적 검증 근거,
  선택적 정규 `git_workspace_context_json`을 포함해 재실행 행을 저장합니다. 재실행
  행은 호출자 권한이 아니며 현재 연결, Git 작업 공간, User Channel 요구사항을
  우회하지 않습니다.
- `host_sessions`, `host_turns`, `host_tool_invocations`는 프로젝트 로컬 host 상관관계를 정규화합니다. 복합 key는 프로젝트, Connection, session, turn, tool-use, tool-name 소유권을 보존합니다. Tool-use ID는 다른 turn이나 tool name으로 다시 결속할 수 없습니다. `managed_mcp_sessions`만 필수 `host_thread_id`와 선택적 `runtime_session_id`를 저장하며 partial unique index는 runtime attach 뒤에만 적용됩니다. Registry 예약 실패 뒤 또는 마지막 프로젝트 attach 전의 null runtime은 권한이 아닙니다.
- `guard_events`는 모든 관찰을 필수 typed hook phase, Guard 설치, 정확한 policy hash, integration revision에 결속합니다. `correlation_kind=codex_hook_prompt`는 session과 turn이 있고 tool field가 없는 `prompt_capture`에만 유효합니다. `correlation_kind=codex_hook_tool`은 session, turn, tool-use ID, tool name이 있는 `pre_tool` 또는 `post_tool`에만 유효합니다. Compatible event에는 이 정확한 형태 중 하나가 필요합니다. Hook row에는 thread column이 없습니다. 현재 소유권의 `compatible` event만 필수 phase를 충족하고, 현재 `malformed` 또는 `incompatible` event는 Guard observation check를 실패시키며, 이전 hash나 revision은 현재 check를 충족하지 않습니다. `decision`은 `allow`, `deny`, `warn`, `inject_context`로 제한되며 이 값은 OS 수준 집행 증명이 아니라 로컬 호스트 판단 요청을 기록합니다.
- `expected_writes.status`는 `pending` 또는 `matched`로 제한되고, `path_policy`는 `exact_paths`로 제한됩니다. 일치한 행은 일치한 Guard 관찰 이벤트, 일치 경로 JSON, `matched_at`을 가져야 하고, 대기 행은 이 일치 필드를 가지면 안 됩니다.
- `unrecorded_changes.status`는 `unresolved` 또는 `resolved`로 제한됩니다. 해결된 행은 해결 JSON, `resolved_at`, `resolved_by_actor_source`를 가져야 하고, 미해결 행은 이 해결 필드를 가지면 안 됩니다.

## 관련 담당 문서

- [저장소 기록](storage-records.md): 영속 기록 계열, 배치, 관계 배치, 저장소 소유 값, JSON 배치를 정의합니다.
- [저장 효과](storage-effects.md): 어떤 메서드 분기가 기록을 만들거나, 바꾸거나, 관찰하거나, 건드리지 않는지 정의합니다.
- [저장소 버전 관리](storage-versioning.md): `StorageManifest` 정체성과 digest, 활성화된
  capability, 정확한 열기 비교와 실패 분류, 생성된 스키마 메타데이터,
  `project_state.state_version` 시계, 정규 Core UTC 시계와 영속 하한, 멱등성, 재실행,
  이벤트, 잠금을 정의합니다.
- [Agent Connection](agent-connection.md): Agent Connection, Connection Projects, 현재 연결 맥락, 모드 제한, Agent Connection과 User Channel의 경계를 정의합니다.
- [보안](security.md): 보안 경계와 보장 수준을 정의합니다.
