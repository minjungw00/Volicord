# 저장소 기록

이 문서는 현재 MVP 원천 설계의 지속 저장 기록 배치를 담당합니다. 문서 원천 자료일 뿐이며 이 저장소에 런타임 데이터베이스, 생성된 기록, 마이그레이션 파일, 구현 완료 DDL을 만들지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- Runtime Home 식별과 프로젝트별 로컬 저장소 배치 가정.
- 활성 지속 기록 범주와 테이블 수준 저장 역할.
- 향후 저장소 설계를 위한 기록 열 의미.
- 저장소가 소유하는 JSON `TEXT` 배치와 검증 기대.
- 기록 수준의 active/later 제외 경계.

이 문서는 담당하지 않습니다.

- 메서드별 저장 효과: [저장 효과](storage-effects.md)
- 아티팩트 스테이징, 승격, 연결, 본문 읽기, 보존, 무결성 생명주기: [아티팩트 저장소](storage-artifacts.md)
- `project_state.state_version`, 멱등성, 이벤트 의미, 잠금, 마이그레이션: [저장소 버전 관리](storage-versioning.md)
- API 요청 또는 응답 스키마: [API 코어 스키마](api/schema-core.md), [API 상태 스키마](api/schema-state.md), [API 아티팩트 스키마](api/schema-artifacts.md)와 다른 API 스키마 담당 문서
- API 메서드 동작: [MVP API](api/mvp-api.md)
- 런타임/저장소/서버 경계: [런타임 경계](runtime-boundaries.md)

## Runtime Home 배치

하네스는 로컬 Runtime Home 하나와 등록된 프로젝트별 로컬 상태 데이터베이스 하나를 사용합니다. 기본 기준 루트는 `~/.harness`입니다. 구현은 같은 역할을 하는 설정 루트를 선택할 수 있습니다.

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

경로의 의미도 저장소 가정에 포함됩니다.

- `~/.harness`로 표시한 Runtime Home 루트는 하네스 운영 데이터 공간입니다. Product Repository가 아니며 파일시스템 권한을 부여하지 않습니다.
- `registry.sqlite`는 Runtime Home 식별 정보와 최소 프로젝트 등록을 저장합니다. Runtime Home 레지스트리이지 프로젝트별 Task 상태가 아닙니다.
- `projects/{project_id}/`는 등록된 프로젝트 하나의 하네스 프로젝트 홈입니다. `repo_root`와 같은 뜻이 아닙니다.
- `project.yaml`은 정적 프로젝트 설정만 저장합니다.
- `state.sqlite`는 등록된 프로젝트의 프로젝트별 로컬 Core 상태를 저장합니다.
- `artifacts/`는 프로젝트 아티팩트 저장소입니다. `artifacts/tmp/`는 임시 스테이징 공간이며 증거 권한이 아닙니다.

`project.yaml`은 현재 Task 상태, gate, Write Authorization 상태, 증거 충분성, 최종 수락, 잔여 위험 수락, 닫기 상태를 저장하면 안 됩니다.

Runtime Home 식별은 파일시스템 경로에만 의존하면 안 됩니다. 복사되거나 이동된 Runtime Home은 같은 저장된 `runtime_home_id`를 가질 수 있습니다. 새 Runtime Home은 새 id를 가져야 합니다. 이 id는 의심스러운 복사본, 중복 등록, 경로 변경을 감지하는 데 도움이 되지만 보안 보장은 아닙니다.

Runtime Home 파일은 로컬 운영 제어 데이터이고 민감한 지원 데이터를 담을 수 있습니다. 보안 비주장과 보장 수준은 [보안](security.md)이 담당하고, 위치 경계는 [런타임 경계](runtime-boundaries.md)가 담당합니다.

## 지속 기록 범주

현재 MVP는 활성 상태 변경 메서드 집합에 필요한 Core 기록만 지속 저장합니다. 대상은 `harness.intake`, `harness.update_scope`, `harness.prepare_write`, `harness.record_run`, `harness.request_user_judgment`, `harness.record_user_judgment`, 상태를 바꾸는 `harness.close_task` intent입니다. `harness.status`와 `harness.close_task intent=check`는 읽기 전용입니다.

활성 Core 지속 기록은 다음뿐입니다.

- `registry.sqlite`의 Runtime Home 식별 정보.
- `registry.sqlite`의 최소 프로젝트 등록.
- `project.yaml`의 정적 프로젝트 설정.
- `project_state`.
- `surfaces`. 단, 활성 API 요청 래퍼, 기능 표시, 로컬 접근 상태에 필요한 등록된 로컬/참조 접점 사실로 제한합니다.
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

활성 임시 저장 경계는 `artifact_staging` 또는 동등한 저장소 소유 스테이징 기록과 `artifacts/tmp/` 아래 안전한 임시 바이트 또는 알림입니다. 이는 저장 위치 설명일 뿐이며, 아티팩트 스테이징 생명주기, 출처, 소비, 승격은 [아티팩트 저장소](storage-artifacts.md)가 담당합니다.

그 밖의 지속 테이블 계열이나 임시 핸들 계열은 현재 MVP 범위가 아닙니다. 요구사항 구체화는 `tasks`, `change_units`, `user_judgments`, `evidence_summaries`, `blockers`를 통해 저장합니다. 별도의 커밋된 Discovery Brief, Shared Design, Question Queue, Assumption Register, First Safe Change Unit Candidate 테이블을 만들지 않습니다. 증거는 간결한 증거 요약, Task 또는 Change Unit의 `CompletionPolicy`, 필수 범위 항목, 아티팩트 참조를 통해 저장합니다. 전체 Evidence Manifest 저장소를 요구하지 않습니다.

Projection은 현재 MVP에서 별도 테이블 계열로 지속 저장하지 않습니다. `status-card`, `judgment-request`, `run-evidence-summary`, `close-result`, `agent-context-packet`은 활성 기록 위에 읽는 시점에 만든 보기이며, 저장된 상태나 저장소 변경 경로가 아닙니다.

최소 활성 구체화 정보도 기존 기록들에 저장합니다. 여기에는 현재 목표 요약, 활성 범위 요약, 허용 경로 또는 영향 영역, 범위 밖 항목, 수락 기준, 자율성 경계, 필요한 사용자 소유 판단, 필요할 때 막히는 질문 하나, 다음 안전한 행동 하나, `CompletionPolicy`, 필수 증거 기대 또는 증거 공백, 닫기 준비 상태가 포함됩니다. 빠졌거나 알 수 없는 항목은 `unknown`, 대기 중인 `user_judgments`, 증거 공백, `blockers`로 남깁니다. 저장소는 요청을 준비된 것처럼 보이게 만들려고 별도 활성 계획 테이블을 만들면 안 됩니다.

## 테이블 개요

아래 표는 활성 저장소 테이블과 최소 저장 역할을 이름 붙입니다. 전체 DDL이 아니며 API 스키마를 복사하지 않습니다.

| 테이블 또는 파일 | 위치 | 현재 MVP 역할 | 주요 저장 필드 |
|---|---|---|---|
| Runtime Home 식별 정보 | `registry.sqlite` | 로컬 Runtime Home과 스키마/저장소 프로필을 식별합니다. | `runtime_home_id`, `schema_version`, `storage_profile`, `created_at`, `updated_at`. |
| 프로젝트 등록 | `registry.sqlite` | 등록된 프로젝트를 프로젝트별 로컬 저장소에 연결합니다. | `project_id`, `repo_root`, `project_home`, `display_name`, `status`, `created_at`, `updated_at`. |
| `project.yaml` | 프로젝트 디렉터리 | 정적 프로젝트 설정입니다. | `project_id`, `repo_root`, 표시/설정 기본값. |
| `project_state` | `state.sqlite` | 프로젝트별 로컬 상태 헤더, 단일 공개 프로젝트 전체 상태 시계, 활성 Task 포인터, 기본 접점 포인터를 저장합니다. | `project_id`, `schema_version`, `storage_profile`, `state_version`, `active_task_id`, `default_surface_id`, `created_at`, `updated_at`. |
| `surfaces` | `state.sqlite` | API 접근에 쓸 로컬 접점 맥락을 확인하기 위한 `LocalSurfaceRegistration` 사실을 저장합니다. 이 행은 등록 데이터이지 현재 호출자가 신뢰된다는 실시간 증명이 아닙니다. | `project_id`, `surface_id`, `surface_instance_id`, `transport_kind`, `transport_binding_fingerprint`, `access_secret_hash`, `capability_profile_hash`, `capability_profile_json`, `status`, `local_access_posture`, `registered_at`, `last_verified_at`, `updated_at`. |
| `tasks` | `state.sqlite` | 사용자 가치 단위, 구체화 요약, 생명주기, 결과, 다음 행동, Task 수준 활성 `CompletionPolicy`, 닫기 필드를 저장합니다. | `task_id`, `project_id`, `title`, `user_request`, `current_goal_summary`, `mode`, `lifecycle_phase`, `close_reason`, `result`, `summary`, 구체화 JSON 열, `completion_policy_json`, `blocking_question`, `next_safe_action`, `active_change_unit_id`, `created_at`, `updated_at`, `closed_at`. |
| `change_units` | `state.sqlite` | 쓰기 호환성, Change Unit 수준 `CompletionPolicy`, 닫기 근거를 위한 현재 또는 제안된 범위 있는 작업 경계를 저장합니다. | `change_unit_id`, `task_id`, `scope_summary`, 허용 경로 또는 영향 영역을 담는 범위 JSON 열, `baseline_ref`, `autonomy_boundary_json`, `completion_policy_json`, `status`, `created_at`, `updated_at`. |
| `user_judgments` | `state.sqlite` | 사용자 소유 판단 기록을 저장하며, 필요하면 별도 민감 동작 승인 범위도 저장합니다. | `user_judgment_id`, `task_id`, `change_unit_id`, `judgment_kind`, `presentation`, `status`, 요청/맥락 JSON 열, `question`, `sensitive_action_scope_json`, `resolution_json`, `expires_at`, `resolved_at`, `created_at`, `updated_at`. |
| `write_authorizations` | `state.sqlite` | `dry_run=false`인 `prepare_write`에서 `decision=allowed`일 때만 만들어지는 지속성 있는 단일 사용 협력형 Write Authorization입니다. | `write_authorization_id`, `task_id`, `change_unit_id`, `surface_id`, `status`, `basis_state_version`, `attempt_scope_json`, `consumed_by_run_id`, `expires_at`, `created_at`, `updated_at`, `consumed_at`. |
| `runs` | `state.sqlite` | 제품 쓰기가 있었다면 호환되는 Write Authorization 소비까지 포함하는 커밋된 실행 또는 관찰 기록입니다. | `run_id`, `task_id`, `change_unit_id`, `write_authorization_id`, `surface_id`, `kind`, `status`, `product_write`, `baseline_ref`, `summary`, 관찰/증거 JSON 열, `created_at`, `completed_at`. |
| `artifact_staging` | `state.sqlite`와 `artifacts/tmp/` | `harness.stage_artifact`가 만들고 나중에 `harness.record_run`이 한 번만 소비할 수 있는 임시 안전 바이트 또는 안전한 알림입니다. | `handle_id`, `project_id`, `task_id`, `created_by_surface_id`, `created_by_surface_instance_id`, `display_name`, `relation_hint`, `tmp_uri`, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `status`, `consumed_by_run_id`, `promoted_artifact_id`, `expires_at`, `created_at`, `consumed_at`. |
| `artifacts` | `state.sqlite`와 아티팩트 저장소 | 무결성, 가림 처리, 생산자, 보존, 가용성 사실을 가진 지속 증거 바이트 또는 안전한 메타데이터입니다. | `artifact_id`, `project_id`, `task_id`, `run_id`, `kind`, `uri`, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `retention_class`, `produced_by`, `status`, `created_at`, `updated_at`. |
| `artifact_links` | `state.sqlite` | 아티팩트와 그것이 뒷받침하는 활성 Core/API 기록 사이의 담당 관계를 저장합니다. | `artifact_link_id`, `artifact_id`, `task_id`, `owner_record_kind`, `owner_record_id`, `relation`, `created_at`. |
| `evidence_summaries` | `state.sqlite` | 상태, 실행/증거 요약, 차단 사유, 닫기에 쓰는 간결한 증거 범위와 공백 기록을 저장합니다. | `evidence_summary_id`, `task_id`, `change_unit_id`, `status`, `coverage_items_json`, `summary`, `supporting_run_ids_json`, `supporting_artifact_link_ids_json`, `gap_blocker_ids_json`, `updated_at`. |
| `blockers` | `state.sqlite` | 다음 행동, 쓰기 호환성, 증거 공백, 닫기 준비 상태, 복구를 위한 구조화된 차단 사유 상태를 저장합니다. `CloseReadinessBlocker`는 API 데이터 형태이며 그 자체가 행이나 저장 신호가 아닙니다. | `blocker_id`, `task_id`, `blocked_action`, `blocker_kind`, `status`, `message`, `owner_ref_json`, `related_refs_json`, `required_next_action`, `created_at`, `resolved_at`. |
| `task_events` | `state.sqlite` | 커밋된 Core 변경의 추가 전용 감사 및 순서 기록입니다. | `event_id`, `project_id`, `task_id`, `event_seq`, `event_type`, `state_version`, `actor_kind`, `surface_id`, `payload_json`, `created_at`. |
| `tool_invocations` | `state.sqlite` | API 메서드별 상태 효과 행이 재실행 행 생성을 허용한, 커밋된 `dry_run=false` Core `MethodResult` 응답만 저장하는 재실행 행입니다. | `invocation_id`, `project_id`, `tool_name`, `idempotency_key`, `request_hash`, `task_id`, `basis_state_version`, `response_json`, `status`, `created_at`. |

## 첫 스키마 무결성 계약

이 섹션은 향후 SQLite 첫 스키마 설계를 위한 저장 계약입니다. 전체 DDL, 마이그레이션 파일, 또는 런타임 구현이 시작되었다는 증거가 아닙니다.

첫 SQLite 스키마에서는 커밋된 행이 아래 식별자, 값 집합, 관계, 트랜잭션 경계를 보존해야 합니다. 구현은 `CHECK` 제약, 조회 테이블, 생성 열, 트리거, Core 쪽 검증 중 알맞은 방식을 고를 수 있습니다. 그래도 Core/API/저장소 담당 문서의 검증은 계속 필요합니다.

필수 식별성과 고유 제약은 다음과 같습니다.

- 활성 테이블은 불투명하고 안정적인 id를 기본 키 또는 동등한 고유 키로 사용합니다. 대상은 `project_id`, `surface_id`, `surface_instance_id`, `task_id`, `change_unit_id`, `user_judgment_id`, `write_authorization_id`, `run_id`, `handle_id`, `artifact_id`, `artifact_link_id`, `evidence_summary_id`, `blocker_id`, `event_id`, `invocation_id`입니다.
- Runtime Home 식별 정보는 해당 Runtime Home의 `runtime_home_id` 하나를 저장합니다.
- 프로젝트 등록에는 고유한 `project_id`와 고유한 `project_home`이 필요합니다. 향후 담당 문서가 다중 등록 동작을 정의하기 전까지 `repo_root` 하나에는 활성 등록 하나만 둡니다.
- `project_state.project_id`는 등록된 프로젝트마다 한 행입니다.
- `surfaces`에는 고유한 `(project_id, surface_id)`가 필요합니다. 저장된 `surface_instance_id`는 그 접점 행이 선택한 등록 로컬 인스턴스를 식별합니다.
- `tasks`에는 고유한 `(project_id, task_id)`가 필요합니다.
- `change_units`에는 고유한 `(task_id, change_unit_id)`가 필요하며, Task마다 `status=active`인 Change Unit은 최대 하나입니다.
- `write_authorizations.consumed_by_run_id`, `runs.write_authorization_id`, `artifact_staging.consumed_by_run_id`, `artifact_staging.promoted_artifact_id`는 null이 아닐 때 고유합니다.
- `artifact_staging`에는 고유한 `(project_id, handle_id)`가 필요합니다. 핸들이 존재하는 동안 `tmp_uri` 또는 동등한 스테이징 객체 식별자도 프로젝트 안에서 고유해야 합니다.
- `artifact_links`에는 `(artifact_id, owner_record_kind, owner_record_id, relation)`과 동등한 고유 규칙이 필요합니다.
- `artifacts.uri`는 저장하는 경우 프로젝트 안에서 고유해야 하며 같은 `artifact_id`로 해석되어야 합니다.
- `task_events`에는 고유한 `event_id`와 영향을 받는 프로젝트/Task 범위 안에서 단조 증가하는 고유 `event_seq`가 필요합니다.
- `tool_invocations`에는 `(project_id, tool_name, idempotency_key)` 고유 재실행 키가 필요합니다. `request_hash`는 그 행에 저장하는 충돌 판별자이며, 같은 멱등 키가 여러 커밋 응답으로 갈라지게 하는 두 번째 고유 키가 아니어야 합니다.

주요 관계 제약은 다음과 같습니다.

- 프로젝트 범위 행은 프로젝트 등록에 속합니다.
- 활성 Task 포인터와 기본 접점 포인터는 같은 프로젝트의 행을 가리켜야 합니다.
- `tasks.active_change_unit_id`는 같은 Task의 `change_units` 행을 가리킵니다.
- `change_units`, `user_judgments`, `write_authorizations`, `runs`, `evidence_summaries`, `blockers`, `task_events`, `tool_invocations` 같은 Task 범위 행은 같은 프로젝트/Task 범위를 가리킵니다.
- `runs.write_authorization_id`는 값이 있으면 소비된 `write_authorizations` 행을 가리키며 같은 Task, Change Unit, 접점, 호환되는 시도 범위와 맞아야 합니다.
- `artifact_staging.consumed_by_run_id`와 `artifact_staging.promoted_artifact_id`는 값이 있으면 소비하는 `harness.record_run` 트랜잭션이 만든 같은 프로젝트 같은 Task의 `runs`와 `artifacts` 행을 가리킵니다.
- `artifact_links.artifact_id`는 `artifacts`를 가리키며, `artifact_links.task_id`는 아티팩트와 담당 관계가 속한 같은 Task를 가리킵니다.
- `supporting_run_ids_json`, `supporting_artifact_link_ids_json`, `gap_blocker_ids_json`, `related_refs_json`, `response_json` 같은 JSON 참조 배열은 단순 미검증 텍스트 참조가 될 수 없습니다. SQLite가 직접 외래 키로 표현할 수 없는 관계라도 커밋 전에 파싱하고 같은 프로젝트/Task와 담당 관계를 확인해야 합니다.

일반적인 현재 MVP Core 동작은 권한 행을 하드 삭제하지 않습니다. 상태 또는 생명주기 필드로 행을 이동시키고, 이벤트를 추가하며, 재실행과 아티팩트 메타데이터를 감사와 복구에 사용할 수 있게 유지합니다. 외래 키는 권한 행에 대해 기본적으로 `RESTRICT` 또는 동등한 담당 검증을 사용해야 합니다. Task를 완료, 취소, 대체해도 `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`, `evidence_summaries`, `blockers`, `task_events`, `tool_invocations`가 연쇄 삭제되면 안 됩니다.

소비되지 않았거나 만료된 `artifact_staging` 행과 `artifacts/tmp/`의 스테이징 바이트 또는 알림은 `expired` 또는 `discarded`로 표시할 수 있고, 등록 전 임시 바이트는 정리할 수 있습니다. 이것들은 증거 권한이 아니기 때문입니다. `artifacts` 행이 커밋된 뒤의 보존 삭제, 프로젝트 해체, 파괴적 정리는 일반적인 현재 MVP 변경 동작 밖이며 담당 문서가 정의한 경로가 필요합니다.

## 저장소 소유 값과 JSON

닫힌 현재 MVP 저장소 값 집합은 테이블 수준 지속 제약입니다. API 스키마 값을 반영하는 행은 API 스키마 담당 문서와 정확히 맞아야 합니다. 저장소 소유로 표시된 행은 공개 API 스키마 본문이 아니라 저장 동작을 정의합니다. 알 수 없는 값은 커밋 전에 실패합니다.

| 필드 | 현재 MVP 값 | 저장소 규칙 |
|---|---|---|
| 프로젝트 등록 `status` | `active` | 기준 현재 MVP에는 등록된 활성 프로젝트만 있습니다. 비활성화/등록 해제 동작은 승격되기 전까지 이후 후보입니다. |
| `surfaces.transport_kind` | `local_mcp_stdio`, `local_http` | 등록 매칭을 위한 저장된 로컬 전송 범주입니다. 소켓이나 프로토콜 설정 명세가 아닙니다. |
| `surfaces.local_access_posture` | `registered_local`, `unavailable`, `mismatch`, `revoked` | API 호환성 확인을 위한 저장된 등록 상태입니다. 의미는 API 스키마 담당 문서와 맞습니다. |
| `surfaces.status` | `active`, `disabled`, `stale`, `revoked` | 저장된 접점 등록 사용 가능성입니다. 의미는 API 스키마 담당 문서와 맞습니다. |
| `tasks.lifecycle_phase` | `shaping`, `ready`, `executing`, `waiting_user`, `blocked`, `completed`, `cancelled`, `superseded` | 지속 Task 생명주기입니다. `intake`는 저장 값이 아니며 `superseded`는 종료 값입니다. |
| `tasks.close_reason` | `none`, `completed_self_checked`, `completed_with_risk_accepted`, `cancelled`, `superseded` | 생명주기와 결과에서 분리된 지속 닫기 세부값입니다. |
| `tasks.result` | `none`, `advice_only`, `completed`, `cancelled`, `superseded` | 지속되는 큰 결과입니다. 실패한 Run, violation, 차단된 닫기, 증거 공백은 담당 기록에 남습니다. |
| `change_units.status` | `proposed`, `active`, `replaced`, `closed` | 쓰기 호환성과 닫기 근거를 위한 저장소 소유 활성 Change Unit 생명주기입니다. |
| `write_authorizations.status` | `active`, `consumed`, `expired`, `stale`, `revoked` | 지속 승인 생명주기입니다. 저장소가 지속 저장과 전이 규칙을 담당합니다. |
| `artifact_staging.status` | `staged`, `consumed`, `expired`, `discarded` | 저장소 소유 임시 핸들 생명주기입니다. `harness.record_run`이 소비할 수 있는 값은 `staged`뿐이며, 종료 값은 `staged`로 돌아갈 수 없습니다. |
| `artifacts.status` | `available`, `missing`, `integrity_failed`, `unavailable` | 저장소 소유 아티팩트 가용성 상태입니다. 가림 처리와 차단된 페이로드 처리는 `redaction_state`에 남습니다. |
| `artifact_links.owner_record_kind` | `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` | 지속 담당 관계 판별자입니다. 저장소가 같은 프로젝트/같은 Task 담당 행 조회와 관계 검증을 담당합니다. |
| `blockers.status` | `active`, `resolved`, `superseded` | 저장소 소유 차단 사유 행 상태입니다. 공개 닫기 차단 사유 형태는 API 담당 문서에 남습니다. |
| `tool_invocations.status` | `committed` | 재실행 행은 메서드별 상태 효과 행이 재실행 행 생성을 허용한 커밋된 `dry_run=false` Core `MethodResult` 응답에만 존재합니다. |

`tasks.mode`, `runs.kind`, `runs.status`, `user_judgments.status`, `evidence_summaries.status` 같은 다른 지속 status형 API 필드는 [API 값 집합](api/schema-value-sets.md)과 Core/API 메서드 담당 문서를 기준으로 검증합니다. 저장소는 색인과 제약을 둘 수 있지만, 이 문서가 공개 스키마 값을 다시 정의하지 않습니다.

JSON을 저장하는 SQLite `TEXT` 열은 저장 표현 선택일 뿐이며 임의 JSON을 저장해도 된다는 뜻이 아닙니다. Core는 커밋 전에 JSON을 파싱하고 검증해야 합니다. API 형태의 저장 JSON은 API 스키마 담당 문서를 기준으로 검증합니다. 저장소 전용 JSON은 이 문서나 이 문서가 가리키는 담당 문서를 기준으로 검증합니다. `'{}'`, `'[]'` 같은 SQLite 기본값은 저장 기본값일 뿐이며 API 필드를 optional로 만들지 않습니다.

활성 JSON `TEXT` 열은 활성 기록에 필요한 간결한 담당 형태 데이터로 제한합니다.

- `surfaces.capability_profile_json`.
- `success_criteria_json`, `acceptance_criteria_json`, `scope_boundary_json`, `non_goals_json`, `affected_areas_json`, `affected_path_candidates_json`, `constraints_json`, `autonomy_boundary_json`, `completion_policy_json` 같은 Task와 Change Unit 구체화 열.
- `user_judgments` 요청, 맥락, 선택지, 영향 참조, 아티팩트 참조, `sensitive_action_scope_json`, `resolution_json` 열.
- `write_authorizations.attempt_scope_json`.
- `runs`의 관찰된 시도와 증거 업데이트 JSON 열.
- `evidence_summaries.coverage_items_json`과 supporting/gap 참조 배열.
- `blockers.owner_ref_json`과 `blockers.related_refs_json`.
- `task_events.payload_json`.
- `tool_invocations.response_json`.

Task와 Change Unit 구체화 JSON은 간결한 요약과 제한된 목록만 저장합니다. 독립 Discovery Brief, Question Queue, Assumption Register, 전체 설계 아티팩트, 생성된 상태 보기 본문, Evidence Manifest 본문, QA 기록, 수락 기록, 잔여 위험 기록, 닫기 기록을 다른 이름으로 저장하면 안 됩니다.

## Active / Later 경계

프로필로 제한된 이후 후보 저장소는 담당 문서가 범위, 대체 동작, 향후 승격에 필요한 증명 경로 기대치와 함께 좁은 동작을 승격하기 전까지 현재 MVP 밖에 있습니다. 참조 스키마에 존재한다는 사실만으로 저장소가 활성화되지 않습니다.

현재 MVP는 Projection 작업, 영속 Projection 캐시, managed-output outbox, conformance-runner 상태, fixture 실행 이력, 운영 프로필 저장소, `captured_artifact` 핸들, 접점 자체 캡처 저장소, 캡처 어댑터 출력 테이블, 전체 Evidence Manifest 테이블, 상세 증거 카탈로그, 분리형 검증, 전체 수동 QA 행렬, 풍부한 QA/waiver 장치, `user_judgments`와 `blockers`에서 분리된 상세 승인 또는 잔여 위험 생명주기 테이블, 대시보드, 분석, 호스팅 커넥터 등록소, 접점 간 조율 저장소, 장기 설계 지원 저장소를 제외합니다.

활성 상태, 닫기 준비 상태, 실행/증거 요약, 다음 행동, 읽기용 카드, `agent-context-packet`, 보장 표시는 활성 지속 기록 위에 읽는 시점에 파생하는 보기입니다. 이 출력은 오래되었거나, 없거나, 실패했을 수 있고 다시 계산될 수 있습니다. 그래도 저장소 권한을 바꾸지 않습니다.

## 관련 담당 문서

- [저장 효과](storage-effects.md): 어떤 메서드가 기록을 만들거나, 바꾸거나, 관찰하거나, 건드리지 않는지.
- [아티팩트 저장소](storage-artifacts.md): 아티팩트 전용 저장 생명주기.
- [저장소 버전 관리](storage-versioning.md): 시계, 멱등성, 잠금, 마이그레이션 의미.
- [MVP API](api/mvp-api.md): 기록을 사용하는 공개 메서드 동작.
