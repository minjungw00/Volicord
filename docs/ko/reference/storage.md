# Storage

이 문서는 향후 하네스 저장소를 위한 참조 문서입니다. 이 저장소에 하네스 서버,
Runtime Home, 데이터베이스, 아티팩트 저장소, 마이그레이션 실행기, 생성된 Projection,
런타임 상태, 구현 완료 DDL이 있다는 뜻이 아닙니다. 현재 저장소 상태는
[MVP 계획](../build/mvp-plan.md#문서-수락-상태)이 담당합니다.

## 1. 담당하는 것 / 담당하지 않는 것

이 문서는 현재 활성 MVP의 영속 경계를 담당합니다.

- Runtime Home 식별과 프로젝트별 로컬 저장소 배치.
- 활성 영속 레코드와 저장소 테이블별 역할.
- 저장소가 소유하는 JSON `TEXT` 규칙.
- 아티팩트 영속성과 아티팩트 담당 연결.
- 이벤트와 멱등성 저장 의미.
- 상태 버전 관리 규칙.
- 잠금 정책과 마이그레이션 경계.
- 현재 활성 MVP 저장소와 later 후보 저장소의 경계.

저장소는 Harness record의 영속 위치와 상태 전이 기록 방식을 정의하지만,
변조 방지 저장소를 제공한다고 주장하지 않습니다.

이 문서는 아래 항목을 담당하지 않습니다.

- Core 생명주기, gate, blocker, Write Authorization, `record_run`, close 의미.
  [Core Model 참조](core-model.md)를 봅니다.
- 공개 MCP 요청/응답, 공유 스키마, active enum 값, 오류, 재실행
  동작. [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md),
  [API Errors](api/errors.md)를 봅니다.
- Projection 렌더링, 상태 보기 템플릿 본문, 보고서 형식, 대시보드,
  내보내기, reconcile 동작, 운영 진입점, 적합성 실행기, 향후 fixture 저장소.
- OS 권한, 샌드박스, 변조 방지 파일, 도구 실행 전 차단, 보안 격리 주장.
  [보안 참조](security.md)를 봅니다.

저장소는 Core가 커밋하고 담당 Core/API/storage 계약에 맞게 검증한 행에 대해서만
현재 하네스 레코드의 기준이 됩니다. 대화, 생성된 Markdown, 상태 카드, Projection,
커넥터 출력, 운영자 출력, 보고서 문장은 저장소 권한이 아닙니다. 저장소는 Core/API/storage
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

`registry.sqlite`는 Runtime Home 식별 정보와 최소 프로젝트 등록 데이터를
저장합니다. `project.yaml`은 정적 프로젝트 설정만 저장합니다.
`state.sqlite`는 프로젝트별 로컬 Core 상태를 저장합니다. 아티팩트 디렉터리는 Core가
아티팩트 등록 경계를 적용한 뒤 등록된 증거 바이트 또는 안전한 메타데이터를
저장합니다.

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

현재 활성 MVP는 활성 메서드 집합에 필요한 레코드만 영속화합니다.
`harness.intake`, `harness.status`, `harness.prepare_write`,
`harness.record_run`, `harness.request_user_judgment`,
`harness.record_user_judgment`, `harness.close_task`가 그 범위입니다.

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

다른 영속 테이블 계열은 현재 활성 MVP 범위가 아닙니다. 요구사항 구체화는
`tasks`, `change_units`, `user_judgments`, `evidence_summaries`, `blockers`를 통해
저장합니다. 별도의 커밋된 Discovery Brief, Shared Design, Question Queue, Assumption
Register, First Safe Change Unit Candidate 테이블을 만들지 않습니다. 증거는 간결한
증거 요약과 아티팩트 참조를 통해 저장합니다. 전체 Evidence Manifest 저장소를
요구하지 않습니다.

## 4. 테이블

아래 표는 active 저장소 테이블과 최소 저장 역할을 이름 붙입니다. 전체 DDL이 아니며 API
스키마를 복사하지 않습니다.

| 테이블 또는 파일 | 위치 | active 역할 | 주요 저장 필드 |
|---|---|---|---|
| Runtime Home 식별 정보 | `registry.sqlite` | 로컬 Runtime Home과 스키마/저장소 프로필을 식별합니다. | `runtime_home_id`, `schema_version`, `storage_profile`, `created_at`, `updated_at`. |
| 프로젝트 등록 | `registry.sqlite` | 등록된 프로젝트를 프로젝트별 로컬 저장소에 연결합니다. | `project_id`, `repo_root`, `project_home`, `display_name`, `status`, `created_at`, `updated_at`. |
| `project.yaml` | 프로젝트 디렉터리 | 정적 프로젝트 설정입니다. | `project_id`, `repo_root`, 표시/설정 기본값. |
| `project_state` | `state.sqlite` | 프로젝트별 로컬 상태 헤더, 상태 시계, 활성 Task 포인터, 기본 surface 포인터를 저장합니다. | `project_id`, `schema_version`, `storage_profile`, `state_version`, `active_task_id`, `default_surface_id`, `created_at`, `updated_at`. |
| `surfaces` | `state.sqlite` | `surface_id`, 기능 프로필, 로컬 접근 상태, 보장 표시를 해석하는 데 필요한 등록된 로컬/참조 접점 사실을 저장합니다. | `surface_id`, `project_id`, `surface_kind`, `capability_profile_json`, `local_access_posture`, `guarantee_level`, `status`, `created_at`, `updated_at`. |
| `tasks` | `state.sqlite` | 사용자 가치 단위, Task별 상태 시계, 현재 구체화 요약, 생명주기, 결과, 닫기 필드를 저장합니다. | `task_id`, `project_id`, `title`, `user_request`, `current_goal_summary`, `mode`, `lifecycle_phase`, `result`, `summary`, 구체화 JSON 열, `blocking_question`, `next_safe_action`, `active_change_unit_id`, `state_version`, `created_at`, `updated_at`, `closed_at`. |
| `change_units` | `state.sqlite` | 쓰기 호환성과 닫기 근거를 위한 현재 또는 제안된 범위 있는 작업 경계를 저장합니다. | `change_unit_id`, `task_id`, `scope_summary`, 범위 JSON 열, `baseline_ref`, `autonomy_boundary_json`, `status`, `created_at`, `updated_at`. |
| `user_judgments` | `state.sqlite` | 활성 `UserJudgment.judgment_kind` 값에 대한 사용자 소유 판단 기록을 저장합니다. | `user_judgment_id`, `task_id`, `change_unit_id`, `judgment_kind`, `presentation`, `status`, 요청/맥락 JSON 열, `question`, `resolution_json`, `expires_at`, `resolved_at`, `created_at`, `updated_at`. |
| `write_authorizations` | `state.sqlite` | `dry_run=false`인 `prepare_write`에서 `decision=allowed`일 때만 만들어지는 지속성 있는 단일 사용 협력형 Write Authorization입니다. | `write_authorization_id`, `task_id`, `change_unit_id`, `surface_id`, `status`, `basis_state_version`, `attempt_scope_json`, `consumed_by_run_id`, `expires_at`, `created_at`, `updated_at`, `consumed_at`. |
| `runs` | `state.sqlite` | 제품 쓰기가 있었다면 호환 승인 소비까지 포함하는 확정된 실행 또는 관찰 기록입니다. | `run_id`, `task_id`, `change_unit_id`, `write_authorization_id`, `surface_id`, `kind`, `status`, `product_write`, `baseline_ref`, `summary`, 관찰/증거 JSON 열, `created_at`, `completed_at`. |
| `artifacts` | `state.sqlite`와 아티팩트 저장소 | 아티팩트 무결성, 가림, 생산자, 보존, 가용성 사실을 가진 등록된 영속 증거 바이트 또는 안전한 메타데이터입니다. | `artifact_id`, `project_id`, `task_id`, `run_id`, `kind`, `uri`, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `retention_class`, `produced_by`, `status`, `created_at`, `updated_at`. |
| `artifact_links` | `state.sqlite` | 아티팩트와 그것이 뒷받침하는 active Core/API 레코드 사이의 담당 관계를 저장합니다. | `artifact_link_id`, `artifact_id`, `task_id`, `owner_record_kind`, `owner_record_id`, `relation`, `created_at`. |
| `evidence_summaries` | `state.sqlite` | 상태, 실행/증거 요약, 차단 사유, 닫기에 쓰는 간결한 증거 범위와 공백 기록을 저장합니다. | `evidence_summary_id`, `task_id`, `change_unit_id`, `status`, `coverage_items_json`, `summary`, `supporting_run_ids_json`, `supporting_artifact_link_ids_json`, `gap_blocker_ids_json`, `updated_at`. |
| `blockers` | `state.sqlite` | 다음 행동, 쓰기 호환성, 증거 공백, 닫기 준비, 복구를 위한 구조화된 차단 사유를 저장합니다. | `blocker_id`, `task_id`, `blocked_action`, `blocker_kind`, `status`, `message`, `owner_ref_json`, `related_refs_json`, `required_next_action`, `created_at`, `resolved_at`. |
| `task_events` | `state.sqlite` | 커밋된 Core 변경의 추가 전용 감사 및 순서 기록입니다. | `event_id`, `task_id`, `event_seq`, `event_type`, `state_version`, `actor_kind`, `surface_id`, `payload_json`, `created_at`. |
| `tool_invocations` | `state.sqlite` | `dry_run=false`인 상태 변경 도구 응답에 대한 커밋된 멱등성 replay 행입니다. | `invocation_id`, `project_id`, `tool_name`, `idempotency_key`, `request_hash`, `task_id`, `basis_state_version`, `response_json`, `status`, `created_at`. |

`surfaces`는 커넥터 marketplace나 넓은 커넥터 ecosystem 테이블이 아닙니다.
`surface_id`, 기능, 로컬 접근 상태, 보장 표시를 해석하는 데 필요한
active 로컬/참조 접점 등록입니다.

`display_label`은 active 저장소 식별 열이 아닙니다. 표시 라벨은
`judgment_kind` 같은 안정 식별자와 locale에서 파생합니다.

## 5. JSON TEXT 열

JSON을 저장하는 SQLite `TEXT` 기반 JSON TEXT 열은 저장 표현 선택입니다. 임의 JSON을
저장하라는 뜻이 아닙니다. Core는 커밋 전에 JSON을 파싱하고 검증해야 합니다.

API 형태로 저장되는 JSON은 [MVP API](api/mvp-api.md)와
[API Schema Core](api/schema-core.md)에 맞게 검증합니다. 저장소 전용 JSON은 이 문서
또는 이 문서가 이름 붙인 담당 문서에 맞게 검증합니다. `'{}'`, `'[]'` 같은 SQLite
기본값은 저장소 기본값일 뿐입니다. API 필드를 선택 사항으로 만들지 않습니다.

활성 JSON TEXT 열은 active 레코드에 필요한 간결한 담당 형태 데이터로 제한합니다.
예시는 다음과 같습니다.

- `surfaces.capability_profile_json`.
- `success_criteria_json`, `non_goals_json`, `affected_areas_json`,
  `affected_path_candidates_json`, `constraints_json`, `autonomy_boundary_json` 같은
  Task와 Change Unit의 구체화 열.
- `user_judgments`의 요청, 맥락, option, affected-ref, artifact-ref,
  `resolution_json` 열.
- `AuthorizedAttemptScope`를 저장하는 `write_authorizations.attempt_scope_json`.
- `runs`의 관찰된 시도와 증거 업데이트 JSON 열.
- `evidence_summaries.coverage_items_json`과 supporting/gap 참조 배열.
- `blockers.owner_ref_json`과 `blockers.related_refs_json`.
- `task_events.payload_json`.
- `tool_invocations.response_json`.

상태형 `TEXT` 값은 열린 문자열이 아니라 닫힌 담당 값 집합입니다. Active 값은
Core/API 담당 문서와 이 문서의 저장소 설명이 담당합니다. 방어적 `CHECK` 제약이나
lookup table을 사용할 수 있지만 Core 검증은 계속 필요합니다.

## 6. 아티팩트 참조

`ArtifactRef`는 등록된 영속 증거 바이트 또는 안전한 메타데이터를 위한 공개 API 형태입니다.
저장소는 `artifacts`와 `artifact_links`로 이를 구현합니다. 자세한 형태는
[API Schema Core: ArtifactRef](api/schema-core.md#artifactref)를 봅니다.

아티팩트가 증거로 쓰일 수 있으려면 저장소가 아래 사실을 가져야 합니다.

- 아티팩트 저장소 아래 등록된 바이트 또는 안전한 메타데이터 알림,
- `sha256`, `size_bytes`, `content_type` 같은 아티팩트 무결성 사실,
- `redaction_state`,
- 생산자와 보존 사실,
- 가용성 `status`,
- `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` 같은 active
  레코드로 가는 담당 연결.

`sha256`, `size_bytes`, `content_type`은 비교와 가용성 처리에 쓰는 아티팩트 무결성
사실입니다. 이 값들이 아티팩트 저장소를 변조 방지 저장소로 만들거나 암호학적 증거
보장 주장을 만들지는 않습니다.

`uri`는 보통 `harness-artifact://{project_id}/{artifact_id}` 형태로 Harness 저장소를 통해
해석됩니다. 호출자가 임의로 준 파일시스템 경로가 아닙니다. 원문 비밀값, 토큰, 민감한 전체
로그를 증거 바이트로 저장하면 안 됩니다. 대신 가림 처리된 바이트,
`secret_omitted` 또는 `blocked` 알림, 안전한 handle, 담당 문서가 허용한 안전한 표현을
저장합니다.

아티팩트 연결은 담당 기록을 만들지 않습니다. 그 자체로 gate를 충족하거나, 증거
충분성을 증명하거나, QA를 수행하거나, 최종 수락을 만들거나, 잔여 위험을 수락하거나,
Task를 close하지 않습니다.

## 7. 멱등성과 이벤트 의미

`task_events`는 커밋된 Core 변경을 순서대로 기록합니다. 감사와 순서 추적용 기록이지,
일반 동작에서 현재 상태를 재구성하는 출처가 아닙니다. `tasks`, `change_units`,
`user_judgments`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`,
`evidence_summaries`, `blockers` 같은 현재 행이 현재 상태입니다.

`tool_invocations`는 커밋된 non-dry-run 상태 변경 응답의 정확한 재실행을
저장합니다. 키 범위는 [API Errors: Idempotency](api/errors.md#idempotency)가 담당합니다.
같은 키와 요청 해시가 replay되면 Core는 이벤트를 추가하거나, 아티팩트를 등록하거나,
authorization을 소비하거나, 상태를 다시 바꾸지 않고 원래 커밋된 응답을
반환합니다. 같은 키가 다른 요청 해시로 재사용되면 Core는 API 담당 문서가 정의한 상태
충돌 동작을 반환합니다.

Dry run, 잘못된 요청, 커밋 전 검증 실패, 커밋 전 상태 충돌,
mutation을 만들지 않는 거부된 `record_run` 시도는 현재 행, `task_events`,
아티팩트, 증거 요약, Write Authorization, 닫기 상태, `tool_invocations` replay
행을 만들지 않습니다.

차단된 응답은 메서드 담당 문서가 허용한 blocker 또는 다른 mutation만 저장할 수 있습니다.
Blocker가 없다고 말하는 권한을 만들면 안 됩니다. 예를 들어 blocked `prepare_write`
response는 소비 가능한 `write_authorizations`를 만들지 않습니다.

## 8. 상태 버전 관리

상태 버전은 범위별 상태 시계입니다. Task 범위 변경은 `tasks.state_version`을 올립니다.
Core가 찾은 primary Task가 없는 프로젝트 범위 변경은 `project_state.state_version`을
올립니다.

상태를 바꾸는 API 호출은 커밋 전에 `ToolEnvelope.expected_state_version`을 영향받는
범위와 비교합니다. `ToolResponseBase.state_version`은 커밋된 변경에서는 영향받는
범위의 결과 버전이고, read-only와 dry-run 응답에서는 현재 읽을 수 있는 버전
또는 영향을 받을 버전입니다.

`write_authorizations.basis_state_version`은 Core가 시도를 허용할 때 사용한 상태 버전을
저장합니다. `write_authorizations.attempt_scope_json`은 나중에 `record_run`이 관찰된 사실과
비교할 승인된 시도 경계를 저장합니다. 최상위 `task_id`, `change_unit_id`,
`surface_id`, `basis_state_version` 열은 조회 필드입니다. 저장된 시도 범위가
호환성 경계로 남습니다.

`tool_invocations.basis_state_version`은 커밋된 변경 전에 호환성 기준으로 사용한
영향받는 범위의 버전을 저장합니다. `task_events.state_version`은 커밋된 이벤트의 결과
버전을 저장합니다.

## 9. 잠금 정책

Runtime 변경은 Core가 소유한 상태 변경 경로를 통해 직렬화합니다. 일반 SQLite
트랜잭션과 필요한 경우 프로세스/프로젝트 잠금을 사용합니다. 권한 배치는
[런타임 경계 참조](runtime-boundaries.md)가 담당합니다.

현재 활성 MVP는 `persistent_locks` 테이블을 요구하지 않습니다. 영속 잠금/복구
메타데이터는 담당 문서가 승격하기 전까지 later 운영 자료입니다.

잠금은 동시 상태 쓰기를 보호합니다. OS 샌드박스, 아티팩트 무결성 강제,
변조 방지 저장소, 권한 격리, 도구 실행 전 차단을 제공하지 않습니다.

## 10. 마이그레이션 경계

이 저장소에는 마이그레이션 실행기가 없고 마이그레이션할 런타임 데이터도 없습니다.
이 문서는 기존 런타임 데이터를 마이그레이션하는 단계를 정의하지 않습니다. 런타임 구현
전에는 유지보수자가 실제 DDL, 마이그레이션 메커니즘, 저장소 프로필, 제약 강화 동작을
별도로 수락해야 합니다.

현재 활성 마이그레이션 경계는 다음과 같습니다.

- Runtime Home 메타데이터와 `project_state`, 또는 유지보수자가 수락한 동등한 메커니즘에
  스키마/프로필 버전을 저장합니다.
- 커밋 전과 제약 강화 전에 담당 형태 JSON을 검증합니다.
- 담당 문서가 소유한 알 수 없는 status 또는 enum 값은 담당 문서가 정의하기 전까지
  유효하지 않은 값으로 취급합니다.
- `task_events`를 유지한다면 `task_events.event_seq` 순서를 보존합니다.
- 아티팩트 해시와 담당 연결을 보존하거나 영향을 받은 ref를 복구 대상으로 유효하지 않게
  표시합니다.
- 상태 카드, 간결한 상태 보기, Projection 최신성, 닫기 준비 상태, 보고서 문장은 현재
  레코드에서 파생합니다. 마이그레이션 권한이 아닙니다.

이 문서는 비활성 DDL 묶음, 마이그레이션 카탈로그, 프로필별 마이그레이션 세부사항을
의도적으로 제외합니다.

## 11. 현재 활성 MVP에서 제외되는 later 저장소

Profile-gated later 저장소는 담당 문서가 범위, 대체 동작, 향후 승격에 필요한
증명 경로 기대치와 함께 좁은 동작을 승격하기 전까지 현재 활성 MVP 밖에 있습니다. 참조 스키마에 존재한다는
사실만으로 저장소가 active가 되지 않습니다.

현재 활성 MVP는 아래 저장소를 제외합니다.

- Projection 작업, 영속 Projection cache, managed-output outbox, Projection 대시보드.
- validator-run 기록, conformance-runner 상태, fixture 실행 이력, 생성된
  conformance 아티팩트.
- Doctor suite, recover, export, release handoff, artifact dashboard, reconcile queue,
  operational report를 위한 operations-profile 저장소.
- 전체 Evidence Manifest 테이블, 상세 evidence 카탈로그, detached Eval, detached
  verification, 전체 Manual QA matrix, rich QA/waiver machinery.
- `user_judgments`와 `blockers`에서 분리된 rich Approval 테이블과 rich residual-risk lifecycle
  테이블.
- 대시보드, 메트릭, 분석, 팀 workflow, hosted connector registry, connector
  marketplace, connector analytics, cross-surface orchestration 저장소.
- Shared Design, Journey/Spine, Domain Language, Module Map, Interface Contract,
  stewardship, 장기 설계 지원 저장소.

활성 상태, 닫기 준비 상태, 실행/증거 요약, 다음 행동, 읽기용 카드, 보장 표시는 위 active
영속 레코드에서 파생합니다. 이 출력은 오래되었거나, 없거나, 실패했을 수 있고 다시 계산될
수 있습니다. 그래도 저장소 권한을 바꾸지 않습니다.
