# Storage

이 문서는 향후 하네스 저장소를 위한 참조 문서입니다. 이 저장소에 하네스 서버,
Runtime Home, 데이터베이스, 아티팩트 저장소, 마이그레이션 실행기, 생성된 Projection,
런타임 상태, 구현 완료 DDL이 있다는 뜻이 아닙니다. 현재 저장소 상태는
[MVP 계획](../build/mvp-plan.md#문서-수락-상태)이 담당합니다.

## 1. 담당하는 것 / 담당하지 않는 것

이 문서는 현재 MVP의 영속 경계를 담당합니다.

- Runtime Home 식별과 프로젝트별 로컬 저장소 배치.
- 활성 영속 레코드와 저장소 테이블별 역할.
- 저장소가 소유하는 JSON `TEXT` 규칙.
- 아티팩트 영속성과 아티팩트 담당 연결.
- 이벤트와 멱등성 저장 의미.
- 상태 버전 관리 규칙.
- 잠금 정책과 마이그레이션 경계.
- 현재 MVP 저장소와 이후 후보 저장소의 경계.

저장소는 하네스 기록의 영속 위치와 상태 전이 기록 방식을 정의하지만,
변조 방지 저장소를 제공한다고 주장하지 않습니다.

이 문서는 아래 항목을 담당하지 않습니다.

- Core 생명주기, 관문, 차단 사유, Write Authorization, `record_run`, 닫기 의미.
  [Core Model 참조](core-model.md)를 봅니다.
- 공개 MCP 요청/응답, 공유 스키마, 활성 enum 값, 오류, 재실행
  동작. [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md),
  [API Errors](api/errors.md)를 봅니다.
- Projection 렌더링, 상태 보기 템플릿 본문, 보고서 형식, 대시보드,
  내보내기, reconcile 동작, 운영 진입점, 적합성 실행기, 향후 fixture 저장소.
- OS 권한, 샌드박스, 변조 방지 파일, 도구 실행 전 차단, 보안 격리 주장.
  [보안 참조](security.md)를 봅니다.

저장소는 Core가 커밋하고 담당 Core/API/저장소 계약에 맞게 검증한 행에 대해서만
현재 하네스 레코드의 기준이 됩니다. 대화, 생성된 Markdown, 상태 카드, Projection,
커넥터 출력, 운영자 출력, 보고서 문장은 저장소 권한이 아닙니다. 저장소는 Core/API/저장소
계약을 넘어서는 보안 격리, 암호학적 증거 보장 주장, 변조 방지 저장소 주장을 만들지 않습니다.

## 2. Runtime Home 식별

하네스는 로컬 Runtime Home 하나와 등록된 프로젝트별 로컬 상태 데이터베이스 하나를
사용합니다. 기본 기준 루트는 `~/.harness`입니다. 구현은 같은 역할을 하는 설정 루트를
선택할 수 있습니다.

기준 배치는 다음과 같습니다.

```text
~/.harness/
  registry.sqlite
  projects/
    PRJ-0001/
      project.yaml
      state.sqlite
      artifacts/
        tmp/
        diffs/
        logs/
        screenshots/
        checkpoints/
```

경로의 의미도 저장 계약에 포함됩니다.

- `~/.harness`로 표시한 Runtime Home 루트는 하네스 운영 데이터 공간입니다. Product
  Repository가 아니며 파일시스템 권한을 부여하지 않습니다.
- `registry.sqlite`는 Runtime Home 식별 정보와 최소 프로젝트 등록을 저장합니다.
  Runtime Home 레지스트리이지 프로젝트별 Task 상태가 아닙니다.
- `projects/{project_id}/` 아래 프로젝트 디렉터리는 등록된 프로젝트 하나의 Harness
  프로젝트 홈입니다. `repo_root`와 같은 뜻이 아닙니다.
- `project.yaml`은 정적 프로젝트 설정만 저장합니다.
- `state.sqlite`는 등록된 프로젝트의 프로젝트별 로컬 Core 상태를 저장합니다.
- `artifacts/`는 프로젝트 아티팩트 저장소입니다. 그 아래 경로는 Core가 아티팩트 등록
  경계를 적용한 뒤에만 등록된 증거 바이트 또는 안전한 메타데이터를 저장합니다.
  `artifacts/tmp/`는 스테이징 공간이며 증거 권한이 아닙니다.

`project.yaml`은 현재 Task 상태, gate, Write Authorization 상태, 증거 충분성,
최종 수락, 잔여 위험 수락, 닫기 상태를 저장하면 안 됩니다.

Runtime Home 식별은 파일시스템 경로에만 의존하면 안 됩니다. 복사되거나 이동된
Runtime Home은 같은 저장된 `runtime_home_id`를 가질 수 있습니다. 새 Runtime Home은 새 id를
가져야 합니다. 이 id는 의심스러운 복사본, 중복 등록, 경로 변경을 감지하는 데
도움이 됩니다. 하지만 저장소를 변조 방지 상태로 만들지는 않습니다.

Runtime Home 파일은 로컬 운영 제어 데이터이고 민감한 지원 데이터를 담을 수 있습니다.
넓은 읽기 접근은 비밀값, PII, 토큰, 로그, 스크린샷, diff, 아티팩트 내용을 노출할 수
있습니다. 넓은 쓰기 접근은 변조와 증거 오염 위험입니다. 파일 권한, 소유자 확인,
해시, 진단은 방어적 확인입니다. 그 자체로 OS 수준 샌드박스, 임의 도구 제어,
변조 방지 저장소, 실행 전 차단, 보안 격리를 만들지 않습니다.

## 3. 활성 영속 레코드

현재 MVP는 활성 상태 변경 메서드 집합에 필요한 Core 기록만 영속화합니다.
`harness.intake`, `harness.update_scope`, `harness.prepare_write`,
`harness.record_run`, `harness.request_user_judgment`,
`harness.record_user_judgment`, `harness.close_task`가 그 범위입니다.
`harness.status`와 `harness.close_task intent=check`는 읽기 전용입니다.
`harness.stage_artifact`는 활성 로컬 아티팩트 유틸리티이지만 임시 스테이징
바이트 또는 알림과 `StagedArtifactHandle`만 만듭니다. 현재 Core 기록을 만들거나
프로젝트 상태 시계를 올리지 않습니다.

활성 영속 레코드는 다음뿐입니다.

- `registry.sqlite`의 Runtime Home 식별 정보.
- `registry.sqlite`의 최소 프로젝트 등록.
- `project.yaml`의 정적 프로젝트 설정.
- `project_state`.
- `surfaces`. 단, 활성 API 봉투 구조, 기능 표시, 로컬 접근 상태에 필요한
  등록된 로컬/참조 접점 사실로 제한합니다.
- `tasks`.
- `change_units`.
- `user_judgments`.
- `write_authorizations`.
- `runs`.
- `artifacts`.
- `artifact_links`.
- `evidence_summaries`.
- `blockers`.
- `task_events`.
- `tool_invocations`.

다른 영속 테이블 계열은 현재 MVP 범위가 아닙니다. 요구사항 구체화는
`tasks`, `change_units`, `user_judgments`, `evidence_summaries`, `blockers`를 통해
저장합니다. 별도의 커밋된 Discovery Brief, Shared Design, Question Queue, Assumption
Register, First Safe Change Unit Candidate 테이블을 만들지 않습니다. 증거는 간결한
증거 요약과 아티팩트 참조를 통해 저장합니다. 전체 Evidence Manifest 저장소를
요구하지 않습니다.

최소 활성 구체화 정보도 이 기존 기록들에 저장합니다. 여기에는 현재 목표 요약,
활성 범위 요약, 허용 경로 또는 영향 영역, 범위 밖 항목, 수락 기준, 자율성 경계,
필요한 사용자 소유 판단, 필요할 때 막히는 질문 하나, 다음 안전한 행동 하나, 증거 기대
또는 증거 공백, 닫기 차단 사유가 포함됩니다. 빠졌거나 알 수 없는 항목은 `unknown`,
대기 중인 `user_judgments`, 증거 공백, `blockers`로 남깁니다. 저장소는 요청을 준비된 것처럼
보이게 만들려고 별도 활성 계획 테이블을 만들면 안 됩니다.

## 4. 테이블

아래 표는 활성 저장소 테이블과 최소 저장 역할을 이름 붙입니다. 전체 DDL이 아니며 API
스키마를 복사하지 않습니다.

| 테이블 또는 파일 | 위치 | 현재 MVP 역할 | 주요 저장 필드 |
|---|---|---|---|
| Runtime Home 식별 정보 | `registry.sqlite` | 로컬 Runtime Home과 스키마/저장소 프로필을 식별합니다. | `runtime_home_id`, `schema_version`, `storage_profile`, `created_at`, `updated_at`. |
| 프로젝트 등록 | `registry.sqlite` | 등록된 프로젝트를 프로젝트별 로컬 저장소에 연결합니다. | `project_id`, `repo_root`, `project_home`, `display_name`, `status`, `created_at`, `updated_at`. |
| `project.yaml` | 프로젝트 디렉터리 | 정적 프로젝트 설정입니다. | `project_id`, `repo_root`, 표시/설정 기본값. |
| `project_state` | `state.sqlite` | 프로젝트별 로컬 상태 헤더, 단일 공개 프로젝트 상태 시계, 활성 Task 포인터, 기본 접점 포인터를 저장합니다. | `project_id`, `schema_version`, `storage_profile`, `state_version`, `active_task_id`, `default_surface_id`, `created_at`, `updated_at`. |
| `surfaces` | `state.sqlite` | API 접근에 쓸 로컬 접점 맥락을 확인하기 위한 `LocalSurfaceRegistration` 사실을 저장합니다. 이 행은 등록 데이터이지 현재 호출자가 신뢰된다는 실시간 증명이 아닙니다. | `project_id`, `surface_id`, `surface_instance_id`, `transport_kind`, `transport_binding_fingerprint`, `access_secret_hash`, `capability_profile_hash`, `capability_profile_json`, `status`, `local_access_posture`, `registered_at`, `last_verified_at`, `updated_at`. |
| `tasks` | `state.sqlite` | 사용자 가치 단위, 현재 구체화 요약, 생명주기, 결과, 다음 행동, 닫기 필드를 저장합니다. | `task_id`, `project_id`, `title`, `user_request`, `current_goal_summary`, `mode`, `lifecycle_phase`, `close_reason`, `result`, `summary`, 구체화 JSON 열, `blocking_question`, `next_safe_action`, `active_change_unit_id`, `created_at`, `updated_at`, `closed_at`. |
| `change_units` | `state.sqlite` | 쓰기 호환성과 닫기 근거를 위한 현재 또는 제안된 범위 있는 작업 경계를 저장합니다. | `change_unit_id`, `task_id`, `scope_summary`, 허용 경로 또는 영향 영역을 담는 범위 JSON 열, `baseline_ref`, `autonomy_boundary_json`, `status`, `created_at`, `updated_at`. |
| `user_judgments` | `state.sqlite` | 활성 `UserJudgment.judgment_kind` 값에 대한 사용자 소유 판단 기록을 저장합니다. | `user_judgment_id`, `task_id`, `change_unit_id`, `judgment_kind`, `presentation`, `status`, 요청/맥락 JSON 열, `question`, `resolution_json`, `expires_at`, `resolved_at`, `created_at`, `updated_at`. |
| `write_authorizations` | `state.sqlite` | `dry_run=false`인 `prepare_write`에서 `decision=allowed`일 때만 만들어지는 지속성 있는 단일 사용 협력형 Write Authorization입니다. | `write_authorization_id`, `task_id`, `change_unit_id`, `surface_id`, `status`, `basis_state_version`, `attempt_scope_json`, `consumed_by_run_id`, `expires_at`, `created_at`, `updated_at`, `consumed_at`. |
| `runs` | `state.sqlite` | 제품 쓰기가 있었다면 호환 승인 소비까지 포함하는 확정된 실행 또는 관찰 기록입니다. | `run_id`, `task_id`, `change_unit_id`, `write_authorization_id`, `surface_id`, `kind`, `status`, `product_write`, `baseline_ref`, `summary`, 관찰/증거 JSON 열, `created_at`, `completed_at`. |
| `artifacts` | `state.sqlite`와 아티팩트 저장소 | 아티팩트 무결성, 가림, 생산자, 보존, 가용성 사실을 가진 등록된 영속 증거 바이트 또는 안전한 메타데이터입니다. | `artifact_id`, `project_id`, `task_id`, `run_id`, `kind`, `uri`, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `retention_class`, `produced_by`, `status`, `created_at`, `updated_at`. |
| `artifact_links` | `state.sqlite` | 아티팩트와 그것이 뒷받침하는 활성 Core/API 레코드 사이의 담당 관계를 저장합니다. | `artifact_link_id`, `artifact_id`, `task_id`, `owner_record_kind`, `owner_record_id`, `relation`, `created_at`. |
| `evidence_summaries` | `state.sqlite` | 상태, 실행/증거 요약, 차단 사유, 닫기에 쓰는 간결한 증거 범위와 공백 기록을 저장합니다. | `evidence_summary_id`, `task_id`, `change_unit_id`, `status`, `coverage_items_json`, `summary`, `supporting_run_ids_json`, `supporting_artifact_link_ids_json`, `gap_blocker_ids_json`, `updated_at`. |
| `blockers` | `state.sqlite` | 다음 행동, 쓰기 호환성, 증거 공백, 닫기 준비, 복구를 위한 구조화된 차단 사유를 저장합니다. | `blocker_id`, `task_id`, `blocked_action`, `blocker_kind`, `status`, `message`, `owner_ref_json`, `related_refs_json`, `required_next_action`, `created_at`, `resolved_at`. |
| `task_events` | `state.sqlite` | 커밋된 Core 변경의 추가 전용 감사 및 순서 기록입니다. | `event_id`, `project_id`, `task_id`, `event_seq`, `event_type`, `state_version`, `actor_kind`, `surface_id`, `payload_json`, `created_at`. |
| `tool_invocations` | `state.sqlite` | `dry_run=false`인 상태 변경 도구 응답에 대한 커밋된 재실행 행입니다. | `invocation_id`, `project_id`, `tool_name`, `idempotency_key`, `request_hash`, `task_id`, `basis_state_version`, `response_json`, `status`, `created_at`. |

### 첫 스키마 무결성 계약

이 섹션은 향후 SQLite 첫 스키마 설계를 위한 저장 계약입니다. 전체 DDL,
마이그레이션 파일, 또는 런타임 구현이 시작되었다는 증거가 아닙니다.

첫 SQLite 스키마에서는 아래 하위 섹션을 최소 영속 제약으로 다룹니다. 구현은 `CHECK`
제약, 조회 테이블, 생성 열, 트리거, Core 쪽 검증 중 알맞은 방식을 고를 수 있습니다.
하지만 커밋된 행은 이 식별자, 값 집합, 관계, 트랜잭션 경계를 보존해야 합니다. 이 문서와
API/Core 담당 문서가 공개 스키마 값이나 메서드 효과에서 어긋나면 DDL을 수락하기 전에
담당 문서를 먼저 바로잡아야 합니다.

필수 식별성과 고유 제약은 다음과 같습니다.

- 활성 테이블은 불투명하고 안정적인 id를 기본 키 또는 동등한 고유 키로
  사용합니다. 대상은 `project_id`, `surface_id`, `surface_instance_id`, `task_id`,
  `change_unit_id`, `user_judgment_id`, `write_authorization_id`, `run_id`,
  `artifact_id`, `artifact_link_id`, `evidence_summary_id`, `blocker_id`, `event_id`,
  `invocation_id`입니다.
- Runtime Home 식별 정보는 해당 Runtime Home의 `runtime_home_id` 하나를 저장합니다.
- 프로젝트 등록에는 고유한 `project_id`와 고유한 `project_home`이 필요합니다. 향후
  담당 문서가 다중 등록 동작을 정의하기 전까지 `repo_root` 하나에는 활성 등록 하나만
  둡니다.
- `project_state.project_id`는 등록된 프로젝트마다 한 행입니다.
- `surfaces`에는 고유한 `(project_id, surface_id)`가 필요합니다. 저장된
  `surface_instance_id`는 그 접점 행이 선택한 등록 로컬 인스턴스를 식별합니다.
  요청이 그 접점에 의존하려면 서버가 파생한 확인된 맥락과 이 값이 맞아야 합니다.
- `tasks`에는 고유한 `(project_id, task_id)`가 필요합니다.
- `change_units`에는 고유한 `(task_id, change_unit_id)`가 필요하며, Task마다
  `status=active`인 Change Unit은 최대 하나입니다.
- `write_authorizations.consumed_by_run_id`는 null이 아닐 때 고유합니다.
  `runs.write_authorization_id`도 null이 아닐 때 고유합니다. 둘을 함께 사용해
  한 번만 소비되는 관계를 보존합니다.
- `artifact_links`에는 같은 담당 관계가 중복되지 않도록
  `(artifact_id, owner_record_kind, owner_record_id, relation)`과 동등한 고유 규칙이
  필요합니다.
- `artifacts.uri`를 파생하지 않고 저장한다면 프로젝트 안에서 고유해야 하며 같은
  `artifact_id`로 해석되어야 합니다.
- `task_events`에는 고유한 `event_id`가 필요하고, 영향을 받는 범위 안에서
  `event_seq`가 단조 증가하며 고유해야 합니다. Task 범위 이벤트는
  `(project_id, task_id, event_seq)`, `task_id`가 null인 프로젝트 범위 이벤트는
  `(project_id, event_seq)` 범위입니다.
- `tool_invocations`에는 `(project_id, tool_name, idempotency_key)`에 대한 고유 재실행
  키가 필요합니다. `request_hash`는 그 행에 저장하는 충돌 판별자입니다. 같은
  `idempotency_key`가 여러 커밋 응답으로 갈라질 수 있게 `request_hash`를 별도 고유
  키에 넣으면 안 됩니다.

주요 외래 키 관계는 다음과 같습니다.

- `project_state.project_id`, `surfaces.project_id`, `tasks.project_id`,
  `artifacts.project_id`, `task_events.project_id`, `tool_invocations.project_id`는 프로젝트
  등록에 속합니다.
- `project_state.active_task_id`는 값이 있으면 같은 프로젝트의 열린 `tasks` 행을
  가리킵니다. `project_state.default_surface_id`는 값이 있으면 같은 프로젝트의
  `surfaces` 행을 가리킵니다.
- `tasks.active_change_unit_id`는 같은 Task의 `change_units` 행을 가리킵니다. Task가 아직
  shaping 중이거나 쓰기 가능하지 않으면 null일 수 있습니다.
- `change_units.task_id`, `user_judgments.task_id`, `write_authorizations.task_id`,
  `runs.task_id`, `evidence_summaries.task_id`, `blockers.task_id`, Task 범위
  `task_events.task_id`는 `tasks`를 가리킵니다.
- `user_judgments.change_unit_id`, `write_authorizations.change_unit_id`,
  `runs.change_unit_id`, `evidence_summaries.change_unit_id`는 값이 있으면 같은 Task의
  `change_units` 행을 가리킵니다.
- `write_authorizations.surface_id`와 `runs.surface_id`는 같은 프로젝트의 `surfaces` 행을
  가리킵니다.
- `runs.write_authorization_id`는 값이 있으면 소비된 `write_authorizations` 행을
  가리킵니다. 같은 Task, Change Unit, surface, 호환되는 attempt scope와 맞아야 합니다.
- `artifacts.task_id`는 담당 Task를 가리킵니다. `artifacts.run_id`는 값이 있으면 같은
  Task의 `runs` 행을 가리킵니다.
- `artifact_links.artifact_id`는 `artifacts`를 가리키고, `artifact_links.task_id`는
  아티팩트와 담당 관계가 속한 같은 Task를 가리킵니다.
- `tool_invocations.task_id`는 값이 있으면 영향을 받은 Task에 해당하는 같은 프로젝트의
  `tasks` 행을 가리킵니다.
- `supporting_run_ids_json`, `supporting_artifact_link_ids_json`, `gap_blocker_ids_json`,
  `related_refs_json`, `response_json` 같은 JSON 참조 배열은 검증하지 않은 원시 텍스트
  참조일 수 없습니다. SQLite가 직접 외래 키로 표현할 수 없는 관계라도 커밋 전에
  파싱하고 같은 `project_id`/`task_id` 범위와 담당 관계에 맞는지 확인해야 합니다.

`cascade delete`(연쇄 삭제) 정책은 다음과 같습니다.

- 현재 MVP의 일반 Core 동작은 권한 행을 물리 삭제하지 않습니다. 상태 또는 생명주기
  필드를 바꾸고, 이벤트를 추가하며, 재실행과 아티팩트 메타데이터를 감사와 복구를 위해
  남깁니다.
- 권한 행의 외래 키는 기본적으로 `RESTRICT` 또는 동등한 담당 문서 검증을 사용해야
  합니다. Task를 completed, cancelled, superseded 상태로 옮겨도 `tasks`,
  `change_units`, `user_judgments`, `write_authorizations`, `runs`, `artifacts`,
  `artifact_links`, `evidence_summaries`, `blockers`, `task_events`,
  `tool_invocations`를 연쇄 삭제하면 안 됩니다.
- `artifacts/tmp/`의 스테이징 바이트, 알림, 임시 핸들은 등록 전에는 증거 권한이
  아니므로 정리할 수 있습니다. 그러나 `artifacts` 행이 커밋된 뒤 보존 정책에 따른
  삭제, 프로젝트 해체, 파괴적 정리는 일반 현재 MVP 상태 변경 밖이며 담당 문서가
  정의한 경로가 필요합니다.
- 향후 보존 또는 마이그레이션 경로는 아티팩트 해시, 담당 연결, 이벤트, 재실행 행을
  보존하거나 영향을 받은 참조를 복구 대상으로 유효하지 않게 표시해야 합니다. 현재
  기록이 아직 이름 붙이는 증거 뒷받침을 조용히 연쇄 삭제하면 안 됩니다.

닫힌 현재 MVP 저장 값 집합은 테이블 수준 영속 제약입니다. Schema Core 값을 그대로
보여 주는 행은 Schema Core와 정확히 일치해야 합니다. 아래에서 저장소 소유로 표시한 행은
공개 API 스키마 본문이 아니라 저장소 동작을 정의합니다. 알 수 없는 값은 커밋 전에
실패해야 합니다.

| 필드 | 현재 MVP 값 | 저장 규칙 |
|---|---|---|
| 프로젝트 등록 `status` | `active` | 기준 현재 MVP에는 등록된 `active` 프로젝트만 있습니다. 비활성화/등록 해제 동작은 담당 문서가 승격하기 전까지 이후 후보입니다. |
| `surfaces.transport_kind` | `local_mcp_stdio`, `local_http` | 등록 일치를 위한 저장된 로컬 transport 범주입니다. 소켓 설정이나 암호 프로토콜 명세가 아닙니다. |
| `surfaces.local_access_posture` | `registered_local`, `unavailable`, `mismatch`, `revoked` | API 호환성 확인을 위한 저장된 등록 태세입니다. 의미는 아래에 있으며 Schema Core의 `LocalSurfaceRegistration.local_access_posture`와 같습니다. |
| `surfaces.status` | `active`, `disabled`, `stale`, `revoked` | 저장된 접점 등록의 사용 가능성입니다. 의미는 아래에 있으며 Schema Core의 `LocalSurfaceRegistration.status`와 같습니다. |
| `tasks.lifecycle_phase` | `shaping`, `ready`, `executing`, `waiting_user`, `blocked`, `completed`, `cancelled`, `superseded` | 지속 저장되는 Task 생명주기입니다. `intake`는 저장 값이 아니며 `superseded`는 종료 값입니다. |
| `tasks.close_reason` | `none`, `completed_self_checked`, `completed_with_risk_accepted`, `cancelled`, `superseded` | 생명주기와 결과와는 별도로 저장되는 닫기 세부 사유입니다. |
| `tasks.result` | `none`, `advice_only`, `completed`, `cancelled`, `superseded` | 저장되는 굵은 결과입니다. 실패한 Run, violation, 차단된 닫기, 증거 공백은 각 담당 기록에 남깁니다. |
| `change_units.status` | `proposed`, `active`, `replaced`, `closed` | 쓰기 호환성과 닫기 근거를 위한 저장소 소유 활성 Change Unit 생명주기입니다. |
| `write_authorizations.status` | `active`, `consumed`, `expired`, `stale`, `revoked` | 오래 남는 Write Authorization 생명주기입니다. Schema Core가 같은 공개 요약 값을 노출하며, 저장소는 영속 방식과 전이 규칙을 담당합니다. |
| `artifacts.status` | `available`, `missing`, `integrity_failed`, `unavailable` | 저장소가 소유하는 아티팩트 가용성 상태입니다. 가림 처리와 차단된 페이로드 처리는 `redaction_state`에 남습니다. |
| `artifacts.redaction_state` | `none`, `redacted`, `secret_omitted`, `blocked` | Schema Core의 `ArtifactRef.redaction_state` 값을 지속 저장합니다. 해시와 크기는 숨겨진 원본이 아니라 커밋된 안전한 바이트 또는 안전한 알림을 설명합니다. |
| `artifact_links.owner_record_kind` | `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` | 지속 저장되는 담당 관계 판별자입니다. 값은 `ArtifactRelationOwner.record_kind`와 같고, 저장소는 같은 `project_id`/`task_id`에 속한 담당 기록 조회와 relation 검증을 담당합니다. |
| `blockers.status` | `active`, `resolved`, `superseded` | 저장소가 소유하는 `blockers` 행 상태입니다. 공개 닫기 차단 사유 형태는 API가 담당합니다. |
| `tool_invocations.status` | `committed` | 재실행 행은 커밋된 non-dry-run 상태 변경 응답에만 존재합니다. `dry_run`과 커밋 전 실패에는 재실행 행이 없습니다. |

그 밖의 지속 저장되는 상태형 API 필드, 예를 들어 `tasks.mode`, `runs.kind`,
`runs.status`, `user_judgments.status`, `evidence_summaries.status`는
[API Schema Core](api/schema-core.md#current-mvp-value-sets)와 Core/API 메서드 담당 문서에
맞게 검증합니다. 저장소가 인덱스나 제약을 둘 수는 있지만, 이 문서는 공개 스키마 값을
다시 정의하지 않습니다.

`harness.intake` 이후에는 `harness.update_scope`가 활성 Task 범위 필드의 커밋된 갱신을 담당합니다.
여기에는 목표 요약, 범위 경계, 범위 밖 항목, 수락 기준, 자율성 경계, baseline 참조,
`tasks.active_change_unit_id`가 포함됩니다. 이 메서드는 Task의 활성 `change_units` 행을
만들거나 교체할 수 있습니다. `user_judgments`에 해결된 `scope_decision`이 있으면 관련
참조로 연결할 수 있지만, `harness.record_user_judgment`가 활성 범위 필드나 Change Unit
필드를 직접 바꾸지는 않습니다.

`change_units.status` 전이는 닫혀 있습니다.

- 쓰기가 가능한 intake 또는 구체화 후보는 새 행에서 `proposed`로 시작할 수 있습니다.
- `proposed`는 해당 Task의 현재 쓰기 호환성 근거로 만드는 담당 경로를 통해서만
  `active`가 될 수 있습니다.
- `proposed`는 활성화 전에 다른 후보가 대체하면 `replaced`가 될 수 있습니다.
- `active`는 `harness.update_scope`가 해당 Task의 다른 활성 Change Unit을 선택할 때
  `replaced`가 됩니다.
- `closed`가 아닌 Change Unit은 담당 Task가 completed, cancelled, superseded 상태가 될 때
  `closed`가 될 수 있습니다.
- `closed`는 현재 MVP 저장소에서 종료 상태입니다.

`write_authorizations.status` 전이도 닫혀 있습니다.

- `dry_run=false`인 `harness.prepare_write`가 `decision=allowed`일 때만 정확히 하나의
  `status=active` 행을 만듭니다. `active`는 지속 저장되는 `open` 의미의 상태입니다. 저장 값
  `open`은 없습니다.
- `active`는 호환되는 `harness.record_run`이 소비할 때만 `consumed`가 됩니다. 이때
  저장소는 `consumed_by_run_id`와 `consumed_at`을 설정합니다.
- `active`는 `harness.update_scope` 또는 다른 담당 경로가 활성 Task, Change Unit,
  baseline, 범위 경계, 수락 근거, 자율성 경계, 상태 버전을 바꿔 현재 근거와 더
  이상 맞지 않을 때 `stale`이 됩니다.
- `active`는 소비하지 않고 Write Authorization을 무효화하는 명시적 담당 경로를 통해서만
  `revoked`가 됩니다.
- `active`는 저장된 `expires_at` 경계가 지났거나 담당 경로가 시간 제한 Write Authorization을
  만료 처리할 때 `expired`가 됩니다. `expired`는 Schema Core가 노출하므로 현재 MVP의
  활성 값입니다. 다만 이전에 `active`였던 행의 종료 상태입니다. 저장소는 이미
  만료된 행을 소비 가능한 Write Authorization처럼 만들면 안 됩니다.
- `consumed`, `stale`, `revoked`, `expired`는 다시 `active`가 될 수 없습니다. 호출자는
  정확한 동작에 대해 호환되는 새 `harness.prepare_write` 결과를 받아야 합니다.
- 차단된 `prepare_write`, `dry_run`, 잘못된 요청, 커밋 전 실패는 소비 가능한
  `write_authorizations` 행을 만들지 않습니다.

`surfaces`는 커넥터 마켓플레이스나 넓은 커넥터 생태계 테이블이 아닙니다.
`surface_id`, 접점 인스턴스 식별자, 로컬 transport binding, `capability_profile_hash`,
로컬 접근 태세, 등록 상태를 해석하는 데 필요한 활성 `LocalSurfaceRegistration` 사실을
저장합니다.

`surfaces` 행은 필요하지만 API 신뢰를 증명하기에는 충분하지 않습니다. 이 행은 등록
근거를 저장합니다. 상태 변경 API가 커밋되거나 아티팩트 본문을 읽기 전에는 서버가 로컬
transport/session/binding에서 요청별 `VerifiedSurfaceContext`를 파생해야 합니다.
`ToolEnvelope.surface_id`는 비교할 행을 고르는 선택자입니다. 그 자체로 권한을 증명하지 않습니다.

Product Repository 파일, Projection, 생성된 Markdown, 대화 텍스트, 에이전트 기억은
`surfaces` 행을 만들거나, 바꾸거나, 새로 고칠 수 없습니다. 등록 새로 고침은 로컬 접점
맥락을 확인하고 현재 등록 상태를 쓰는 담당 경로를 통해서만 저장된 등록 사실을 바꿉니다.

`surfaces.local_access_posture`는 닫힌 현재 MVP 값 집합입니다.

| 값 | 저장 의미 |
|---|---|
| `registered_local` | 성공한 로컬 등록 또는 확인을 통해 이 저장된 접점 등록을 현재 API 호환성 확인의 등록된 로컬 태세로 볼 수 있습니다. |
| `unavailable` | 이 등록으로 필요한 MCP/Core 또는 접점 도달 가능성을 현재 확인할 수 없습니다. |
| `mismatch` | 관찰된 호출자나 transport binding이 저장된 로컬 접점 등록과 맞지 않습니다. |
| `revoked` | 이 등록의 로컬 접근이 명시적으로 철회되었고 사용할 수 없습니다. |

`surfaces.status`는 닫힌 현재 MVP 값 집합입니다.

| 값 | 저장 의미 |
|---|---|
| `active` | 저장된 접점 행을 현재 API 접근 확인에 사용할 수 있습니다. |
| `disabled` | 행은 보존하지만 현재 API 접근에 쓰면 안 됩니다. |
| `stale` | 현재 API 접근에서 행에 의존하기 전에 새로 고쳐야 합니다. |
| `revoked` | 접점 등록이 현재 API 접근에 더 이상 유효하지 않습니다. |

알 수 없는 `surfaces.transport_kind`, `surfaces.local_access_posture`, `surfaces.status` 값은 유효하지 않습니다. 상태 변경 API 호출은 커밋 전에 같은 프로젝트의 `surfaces` 행이 `status=active`이고 요청한 접근 분류에 대해 서버가 파생한 `VerifiedSurfaceContext.verified=true`여야 합니다. 아티팩트 본문 읽기도 `access_class=artifact_read`에 대해 같은 확인된 맥락을 요구합니다. 읽기 전용 상태 경로는 unavailable, mismatch, stale, disabled, revoked, insufficient-capability 접점에 대해 표시해도 안전한 진단을 반환할 수 있지만, 그 진단을 Core 상태로 바꾸거나 아티팩트 본문을 노출하면 안 됩니다.

`display_label`은 활성 저장소 식별 열이 아닙니다. 표시 라벨은
`judgment_kind` 같은 안정 식별자와 locale에서 파생합니다.

`tasks.lifecycle_phase`, `tasks.close_reason`, `tasks.result`는 서로 다른 Core
개념을 저장합니다. `CloseTaskResponse.close_state`는 응답 수준의 닫기 상태이지
지속 저장되는 `tasks` 열이 아닙니다. `tasks.lifecycle_phase`에는 `intake`를 저장하면 안 됩니다.
종료 생명주기 값은 `completed`, `cancelled`, `superseded`입니다. `tasks.result`에는
`passed`나 `failed`를 저장하면 안 됩니다. 실패한 Run, Projection, 아티팩트, validator,
증거 공백, 차단된 닫기, 닫기 차단 사유는 각 담당 기록이나 현재 Task 상태에 남습니다. 커밋된 supersession이 활성
포인터를 바꿀 때 `project_state.active_task_id`는 `harness.close_task`의
`superseding_task_id` 규칙을 따라야 하며, superseded된 Task를 계속 가리키면 안 됩니다.

## 5. JSON TEXT 열

JSON을 저장하는 SQLite `TEXT` 기반 JSON TEXT 열은 저장 표현 선택입니다. 임의 JSON을
저장하라는 뜻이 아닙니다. Core는 커밋 전에 JSON을 파싱하고 검증해야 합니다.

API 형태로 저장되는 JSON은 [MVP API](api/mvp-api.md)와
[API Schema Core](api/schema-core.md)에 맞게 검증합니다. 저장소 전용 JSON은 이 문서
또는 이 문서가 이름 붙인 담당 문서에 맞게 검증합니다. `'{}'`, `'[]'` 같은 SQLite
기본값은 저장소 기본값일 뿐입니다. API 필드를 선택 사항으로 만들지 않습니다.
잘못된 JSON, 담당 문서가 소유하지 않은 필드, 알 수 없는 enum 값, 틀린 스칼라 타입,
경계 없는 배열, 호환되는 `project_id`/`task_id` 범위 밖 기록을 가리키는 JSON은 커밋 전에 실패해야
합니다. SQLite `json_valid`, `CHECK` 제약, 생성 열, 조회 테이블은 저장 표현을
강화할 수 있지만 Core/API/storage 담당 검증을 대신하지 않습니다.

활성 JSON TEXT 열은 활성 레코드에 필요한 간결한 담당 형태 데이터로 제한합니다.
예시는 다음과 같습니다.

- `surfaces.capability_profile_json`.
- `success_criteria_json`, `acceptance_criteria_json`, `scope_boundary_json`,
  `non_goals_json`, `affected_areas_json`, `affected_path_candidates_json`,
  `constraints_json`, `autonomy_boundary_json` 같은
  Task와 Change Unit의 구체화 열.
- `user_judgments`의 요청, 맥락, 선택지, 영향받는 참조, 아티팩트 참조,
  `resolution_json` 열.
- `AuthorizedAttemptScope`를 저장하는 `write_authorizations.attempt_scope_json`.
- `runs`의 관찰된 시도와 증거 업데이트 JSON 열.
- `evidence_summaries.coverage_items_json`과 뒷받침/공백 참조 배열.
- `blockers.owner_ref_json`과 `blockers.related_refs_json`.
- `task_events.payload_json`.
- `tool_invocations.response_json`.

Task와 Change Unit의 구체화 JSON은 간결한 요약과 경계 있는 목록만 저장합니다.
별도의 Discovery Brief, Question Queue, Assumption Register, 전체 설계 아티팩트,
생성된 Projection 본문, Evidence Manifest 본문, QA 기록, 수락 기록, 잔여 위험 기록,
닫기 기록을 다른 이름으로 저장하면 안 됩니다.

상태형 `TEXT` 값은 열린 문자열이 아니라 닫힌 담당 값 집합입니다. 활성 값은
Core/API 담당 문서와 이 문서의 저장소 설명이 담당합니다. 방어적 `CHECK` 제약이나
조회 테이블을 사용할 수 있지만 Core 검증은 계속 필요합니다.

## 6. 아티팩트 참조

`ArtifactRef`는 등록된 영속 증거 바이트 또는 안전한 메타데이터를 위한 공개 API 형태입니다.
저장소는 `artifacts`와 `artifact_links`로 이를 구현합니다. 자세한 형태는
[API Schema Core: ArtifactRef](api/schema-core.md#artifactref)를 봅니다.

아티팩트 등록은 활성 담당 문서가 문서화한 `ArtifactInput` 출처인 `staged_artifact`
또는 `existing_artifact`만 받습니다. `staged_artifact` 입력은 활성
`harness.stage_artifact` 유틸리티가 만든 `StagedArtifactHandle`을 가져야 하며,
저장소가 아티팩트 행을 커밋하기 전에 담당 경로가 이를 해석해야 합니다.
`existing_artifact` 입력은 이미 등록된 `ArtifactRef`를 가리켜야 하며, 같은 프로젝트에
속하고 호환되는 담당 관계를 가져야 합니다. 호출자가 임의로 준 파일시스템 경로, 임의
로컬 경로 문자열, 권한 주장으로서의 원시 로그, `captured_artifact` 핸들, 원시 캡처
어댑터 출력, 접점 자체 캡처 주장은 현재 MVP의 등록 권한이 아닙니다.

임시 스테이징은 아티팩트 권한이 아닙니다. 스테이징 경계는 최소한 `handle_id`,
`project_id`, `task_id`, `sha256`, `size_bytes`, `content_type`, `redaction_state`,
`expires_at`, 이미 소비되었는지 여부를 추적해야 합니다. `harness.stage_artifact`는
안전한 바이트 또는 안전한 알림을 `artifacts/tmp/` 아래에 쓸 수 있지만 `artifacts` 행,
`artifact_links` 행, `evidence_summaries` 행, `task_events` 행, `tool_invocations`
재실행 행, `project_state.state_version` 증가를 만들지 않습니다. 만료되지 않았고 같은
프로젝트/같은 Task에 속하며 아직 소비되지 않은 핸들을 소비해 지속 `ArtifactRef`로
승격할 수 있는 경로는 호환되는 `harness.record_run`뿐입니다. 만료된 스테이징 핸들,
범위가 맞지 않는 핸들, 이미 소비된 핸들, 다른 Task의 핸들은 변경 전에 거부해야 합니다.

`existing_artifact`를 등록할 때는 가용성, 무결성 사실, `redaction_state`, 담당 관계가
새 용도와 계속 호환될 때만 기존 아티팩트 행을 재사용합니다. 새 담당 관계를 위해
`artifact_links` 행을 추가할 수 있지만, 고유 제약과 동일 `project_id`/`task_id` 규칙을 따라야
합니다. 바이트를 복제하거나, 무결성 확인을 건너뛰거나, 원시 아티팩트 경로를 권한으로
사용하면 안 됩니다.

아티팩트가 증거로 쓰일 수 있으려면 저장소가 아래 사실을 가져야 합니다.

- 아티팩트 저장소 아래 등록된 바이트 또는 안전한 메타데이터 알림,
- `sha256`, `size_bytes`, `content_type` 같은 아티팩트 무결성 사실,
- `redaction_state`,
- 생산자와 보존 사실,
- 가용성 `status`,
- `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` 같은 활성
  레코드로 가는 담당 연결.

`artifacts.status`는 가용성 상태입니다.

| 값 | 저장 의미 |
|---|---|
| `available` | 등록된 안전한 바이트 또는 안전한 메타데이터 알림이 있고 저장된 무결성 메타데이터와 맞습니다. |
| `missing` | 아티팩트 행은 남아 있지만 등록된 바이트 또는 안전한 메타데이터 알림을 찾을 수 없습니다. |
| `integrity_failed` | 사용할 수 있는 바이트 또는 메타데이터가 `sha256`, `size_bytes` 같은 저장된 무결성 사실과 맞지 않습니다. |
| `unavailable` | 아티팩트 저장소 또는 필요한 조회 경로가 등록된 바이트나 안전한 메타데이터 알림을 현재 제공할 수 없습니다. |

`artifacts.redaction_state`는 [API Schema Core](api/schema-core.md#artifactref)의 활성
`ArtifactRef.redaction_state` 값을 사용합니다. `blocked`는 가림/생략 상태이지 아티팩트
가용성 `status`가 아닙니다. 커밋된 안전한 알림 또는 가림 처리된 바이트가 있고 무결성
정보를 갖추었다면 `blocked`, `secret_omitted`, `redacted` 아티팩트도
`artifacts.status=available`일 수 있습니다.

`sha256`, `size_bytes`, `content_type`은 비교와 가용성 처리에 쓰는 아티팩트 무결성
사실입니다. 이 값들이 아티팩트 저장소를 변조 방지 저장소로 만들거나 암호학적 증거
보장 주장을 만들지는 않습니다.

`artifact_links`가 다형적 담당 테이블이어도 아티팩트 담당 관계 무결성은 필수입니다.
저장소는 `owner_record_kind`가 `task`, `change_unit`, `run`, `user_judgment`,
`evidence_summary`, `blocker` 중 하나인지, `owner_record_id`가 대응 활성 테이블에
존재하는지, 담당 기록이 같은 `project_id`와 `task_id`에 속하는지, relation이 아티팩트
사용 방식과 호환되는지 검증해야 합니다. 유효한 담당 연결이 없는 원시 `artifact_id`는
증거 뒷받침이 아닙니다.

`owner_record_kind=run`이면 담당 Run은 같은 Task에 속해야 하며 `artifacts.run_id` 값이
있을 때 그 값과 호환되어야 합니다. `owner_record_kind=change_unit`, `user_judgment`,
`evidence_summary`, `blocker`이면 담당 행은 같은 Task에 속해야 합니다. 그 담당 행이 나중에
`superseded`되거나 해결되면, 연결된 아티팩트가 그 담당 관계보다 오래 증거 뒷받침으로
남아 있으면 안 됩니다.

`uri`는 보통 `harness-artifact://{project_id}/{artifact_id}` 형태로 Harness 저장소를 통해
해석됩니다. 호출자가 임의로 준 파일시스템 경로가 아닙니다. 원문 비밀값, 토큰, 민감한 전체
로그를 증거 바이트로 저장하면 안 됩니다. 대신 가림 처리된 바이트,
`secret_omitted` 또는 `blocked` 알림, 안전한 핸들, 담당 문서가 허용한 안전한 표현을
저장합니다.

원시 아티팩트 경로 읽기는 기본으로 허용되지 않습니다. 아티팩트 메타데이터나 본문을
읽으려면 등록된 `ArtifactRef`, 같은 프로젝트의 일치하는 `task_id`, 필요한
`artifact_links` 담당 관계, 호출자의 접근 분류에 필요한 가림/가용성 상태가 있어야 합니다.
아티팩트 저장소 아래 로컬 경로, artifact `uri`, 스테이징 경로, 복사된 파일만으로는
아티팩트 바이트를 읽거나 신뢰할 수 없습니다.

아티팩트 연결은 담당 기록을 만들지 않습니다. 그 자체로 gate를 충족하거나, 증거
충분성을 증명하거나, QA를 수행하거나, 최종 수락을 만들거나, 잔여 위험을 수락하거나,
Task를 닫지 않습니다.

## 7. 멱등성과 이벤트 의미

`task_events`는 커밋된 Core 변경을 순서대로 기록합니다. 감사와 순서 추적용 기록이지,
일반 동작에서 현재 상태를 재구성하는 출처가 아닙니다. `tasks`, `change_units`,
`user_judgments`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`,
`evidence_summaries`, `blockers` 같은 현재 행이 현재 상태입니다.

`task_events`는 현재 MVP의 일반 동작에서 append-only입니다. Event가 커밋된 뒤에는
Core가 이력 내용을 바꾸기 위해 그 행을 수정하거나 삭제하면 안 됩니다. 정정이나
복구는 담당 경로를 통한 새 이벤트와 현재 행 업데이트로 기록합니다. 멱등 재실행,
`dry_run`, 잘못된 요청, 커밋 전 실패는 이벤트를 추가하지 않습니다.

새로 커밋되는 non-dry-run 상태 변경에서는 현재 행 쓰기, `task_events` 추가, 프로젝트 전체
상태 버전 증가, `tool_invocations` 재실행 행 삽입이 하나의 트랜잭션으로 커밋되어야 합니다.
어느 하나라도 실패하면 부분적인 권한 행, 이벤트, 아티팩트 등록, Write Authorization 소비, 증거
업데이트, 닫기 효과, 재실행 행이 남지 않아야 합니다.

`tool_invocations`는 커밋된 non-dry-run 상태 변경 응답의 정확한 재실행을
저장합니다. 키 범위는 [API Errors: Idempotency](api/errors.md#idempotency)가 담당합니다.
같은 키와 요청 해시가 재실행되면 Core는 이벤트를 추가하거나, 아티팩트를 등록하거나,
Write Authorization을 소비하거나, 상태를 다시 바꾸지 않고 원래 커밋된 응답을
반환합니다. 같은 키가 다른 요청 해시로 재사용되면 Core는
[API Errors](api/errors.md#state-conflict-behavior)가 정의한 `STATE_CONFLICT`를
반환합니다. 저장소 고유 키는
`(project_id, tool_name, idempotency_key)`입니다. `request_hash`는 그 행에 저장하는
충돌 판별자입니다.

`dry_run`, 잘못된 요청, 커밋 전 검증 실패, 커밋 전 상태 충돌,
`harness.status`와 `harness.close_task intent=check` 같은 읽기 전용 호출,
상태 변경을 만들지 않는 거부된 `record_run` 시도는 현재 행, `task_events`,
아티팩트, 증거 요약, Write Authorization, 닫기 상태, `tool_invocations` 재실행
행, 상태 버전 증가를 만들지 않습니다.

차단된 응답은 API 메서드별 상태 효과 표가 허용한 차단 사유 또는 다른 상태 변경만 저장할 수 있습니다.
차단 사유가 없거나 부족하다고 지적한 권한을 만들면 안 됩니다. 예를 들어 차단된 `prepare_write`
응답은 소비 가능한 `write_authorizations`를 만들지 않습니다. API 담당 문서가 커밋된
차단 응답에 차단 사유 또는 다른 현재 행 상태 변경 저장을 허용하면, 그 응답은 이벤트,
재실행 행, 상태 버전 목적에서도 커밋된 non-dry-run 상태 변경입니다.

## 8. 상태 버전 관리

현재 MVP에는 공개 상태 시계가 하나만 있습니다. 바로
`project_state.state_version`입니다. 이 값은 프로젝트 전체에 적용되며, 공개 API
변경에서 승인, 충돌, 최신성, 동시성 판단에 쓰는 유일한 활성 기준입니다. `task_id`는
여전히 담당 Task, 차단 사유, 닫기 상태, 증거, 사용자 판단을 찾는 데 중요하지만 별도
상태 시계를 고르지는 않습니다.

새 non-dry-run 상태 변경 API 호출은 커밋 전에
`ToolEnvelope.expected_state_version`을 현재 `project_state.state_version`과 비교합니다.
값이 맞지 않으면 `STATE_CONFLICT`를 반환하고 현재 기록, 이벤트, 아티팩트, 증거 요약,
Write Authorization, 닫기 상태, 재실행 행, 상태 버전 증가를 만들지 않습니다. 현재 MVP의
공개 호출은 둘 이상의 공개 `expected_state_version`을 요구하거나 받지 않습니다.

커밋된 모든 non-dry-run 상태 변경은 `project_state.state_version`을 정확히 1 올립니다.
메서드 담당 문서가 차단 사유나 다른 현재 행 변경 저장을 허용한 커밋된 차단 응답도
같은 규칙을 따릅니다. 하나의 공개 호출이 Task 생명주기 필드와 프로젝트 수준 필드를
함께 바꿀 수 있습니다. 예를 들어 `harness.close_task intent=supersede`가
`tasks.lifecycle_phase`와 `project_state.active_task_id`를 함께 바꾸더라도 여전히 하나의
상태 변경이며 프로젝트 전체 버전 증가는 정확히 한 번만 일어납니다.

`harness.status`, `harness.close_task intent=check`, `dry_run` 호출, 잘못된 요청, 커밋 전
검증 실패, 커밋 전 상태 충돌, 멱등 재실행은 `project_state.state_version`을 올리지
않습니다. `ToolResponseBase.state_version`은 항상 프로젝트 전체 버전을 반환합니다.
커밋된 상태 변경에서는 커밋 뒤 결과 버전이고, 읽기 전용과 `dry_run` 응답에서는 그
응답이 관찰한 현재 프로젝트 전체 버전입니다.

활성 첫 스키마에서는 `tasks.state_version`을 생략해야 합니다. 구현이 레거시 또는
프로토타입 `tasks.state_version` 열을 만나더라도 그 값은 비활성 메타데이터일 뿐입니다.
승인, `STATE_CONFLICT`, 오래된 상태 판단, Write Authorization, 멱등성, 잠금, 동시성
기준으로 쓰면 안 됩니다.

`write_authorizations.basis_state_version`은 Core가 권한을 준비할 때 사용한 프로젝트 전체
`project_state.state_version`을 저장합니다. 오래된 Write Authorization인지 판단할 때는
이 저장값을 현재 프로젝트 전체 상태 버전과 비교합니다. Task별 시계와 비교하지 않습니다.
`write_authorizations.attempt_scope_json`은 나중에 `record_run`이 관찰된 사실과 비교할
승인된 시도 경계를 저장합니다. 최상위 `task_id`, `change_unit_id`, `surface_id`,
`basis_state_version` 열은 조회 필드입니다. 저장된 시도 범위가 호환성 경계로 남습니다.

`tool_invocations.basis_state_version`은 호출이 커밋 전 관찰한 프로젝트 전체 상태 버전을
저장합니다. `task_events.state_version`은 커밋된 이벤트 뒤의 결과 프로젝트 전체 버전을
저장합니다.

## 9. 잠금 정책

Runtime 변경은 Core가 소유한 상태 변경 경로를 통해 직렬화합니다. 일반 SQLite
트랜잭션과 필요한 경우 프로세스/프로젝트 잠금을 사용합니다. 권한 배치는
[런타임 경계 참조](runtime-boundaries.md)가 담당합니다.

현재 MVP는 `persistent_locks` 테이블을 요구하지 않습니다. 영속 잠금/복구
메타데이터는 담당 문서가 승격하기 전까지 이후 운영 자료입니다.

잠금은 동시 상태 쓰기를 보호합니다. OS 샌드박스, 아티팩트 무결성 강제,
변조 방지 저장소, 권한 격리, 도구 실행 전 차단을 제공하지 않습니다.

## 10. 마이그레이션 경계

이 저장소에는 마이그레이션 실행기가 없고 마이그레이션할 런타임 데이터도 없습니다.
이 문서는 기존 런타임 데이터를 마이그레이션하는 단계를 정의하지 않습니다. 런타임 구현
전에는 유지보수자가 실제 DDL, 마이그레이션 메커니즘, 저장소 프로필, 제약 강화 동작을
별도로 수락해야 합니다.

현재 마이그레이션 경계는 다음과 같습니다.

- Runtime Home 메타데이터와 `project_state`, 또는 유지보수자가 수락한 동등한 메커니즘에
  스키마/프로필 버전을 저장합니다.
- 향후 마이그레이션은 수락되기 전에 출발 버전, 대상 버전, 저장소 프로필, 담당 문서,
  되돌림 또는 복구 기대치를 선언해야 합니다.
- 향후 마이그레이션은 `registry.sqlite` 또는 `state.sqlite` 하나를 기준으로 트랜잭션 안에서
  실행해야 하며, 런타임 구현 전에 중단 상태 복구 규칙을 분명히 가져야 합니다.
- 커밋 전과 제약 강화 전에 담당 형태 JSON을 검증합니다.
- 담당 문서가 소유한 알 수 없는 상태 또는 enum 값은 담당 문서가 정의하기 전까지
  유효하지 않은 값으로 취급합니다.
- null 허용 필드, 외래 키, enum 검사, JSON 검증을 강화할 때는 기존 행을 먼저
  검증하거나 담당 문서가 정의한 복구 상태로 라우팅해야 합니다.
- `task_events`를 유지한다면 `task_events.event_seq` 순서를 보존합니다.
- 아티팩트 해시와 담당 연결을 보존하거나 영향을 받은 ref를 복구 대상으로 유효하지 않게
  표시합니다.
- 커밋된 `tool_invocations` 재실행 행을 보존해 마이그레이션 뒤 멱등성이 갈라지지 않게
  합니다.
- 상태 카드, 간결한 상태 보기, Projection 최신성, 닫기 준비 상태, 보고서 문장은 현재
  레코드에서 파생합니다. 마이그레이션 권한이 아닙니다.

이 문서는 비활성 DDL 묶음, 마이그레이션 카탈로그, 프로필별 마이그레이션 세부사항을
의도적으로 제외합니다.

## 11. 현재 MVP에서 제외되는 이후 저장소

profile-gated 이후 후보 저장소는 담당 문서가 범위, 대체 동작, 향후 승격에 필요한
증명 경로 기대치와 함께 좁은 동작을 승격하기 전까지 현재 MVP 밖에 있습니다. 참조 스키마에 존재한다는
사실만으로 저장소가 활성화되지 않습니다.

현재 MVP는 아래 저장소를 제외합니다.

- Projection 작업, 영속 Projection 캐시, managed-output outbox, Projection 대시보드.
- validator-run 기록, conformance-runner 상태, fixture 실행 이력, 생성된
  conformance 아티팩트.
- `doctor` 묶음, recover, export, release handoff, artifact dashboard, reconcile queue,
  운영 보고서를 위한 운영 프로필 저장소.
- 전체 Evidence Manifest 테이블, 상세 증거 카탈로그, detached Eval, 분리형
  검증, 전체 수동 QA 행렬, 풍부한 QA/waiver 장치.
- `user_judgments`와 `blockers`에서 분리된 상세 Approval 테이블과 상세 잔여 위험 생명주기
  테이블.
- 대시보드, 메트릭, 분석, 팀 작업 흐름, 호스팅 커넥터 등록소, 커넥터
  마켓플레이스, 커넥터 분석, 접점 간 조율 저장소.
- Shared Design, Journey/Spine, Domain Language, Module Map, Interface Contract,
  stewardship, 장기 설계 지원 저장소.

활성 상태, 닫기 준비 상태, 실행/증거 요약, 다음 행동, 읽기용 카드, 보장 표시는 위 활성
영속 레코드에서 파생합니다. 이 출력은 오래되었거나, 없거나, 실패했을 수 있고 다시 계산될
수 있습니다. 그래도 저장소 권한을 바꾸지 않습니다.
