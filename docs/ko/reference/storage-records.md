# 저장소 기록

규칙:

- 이 문서는 현재 MVP 원천 설계에서 저장소 기록의 영속 범주와 저장 배치 기대를 담당합니다.
- 영속 기록은 Core가 커밋해 다시 읽을 수 있게 둔 로컬 기록입니다.

허용되지 않는 것:

- 이 문서는 이 저장소에 런타임 데이터베이스, 생성된 기록, 마이그레이션 파일, 구현 완료 DDL을 만들지 않습니다.
- 영속 기록은 변조 불가능성, 위조 방지, 외부 감사 보장을 뜻하지 않습니다.

담당 문서 링크:

- 보안 비주장과 보장 수준은 [보안](security.md)이 담당합니다.

## 저장소 기록과 API 스키마의 차이

저장소 기록과 API 스키마는 서로 다른 담당 문서 경계를 가집니다.

- API 스키마 파일은 요청/응답 데이터 형태, 공개 API 값, API 오류와 응답 분기를 정의합니다. 예를 들어 [API 코어 스키마](api/schema-core.md), [API 상태 스키마](api/schema-state.md), [API 아티팩트 스키마](api/schema-artifacts.md), [API 판단 스키마](api/schema-judgment.md), [API 값 집합](api/schema-value-sets.md)이 해당 경계를 담당합니다.
- 저장소 기록 문서는 영속 기록 범주, 파일/테이블 배치, 저장소가 소유하는 JSON `TEXT` 위치, 기록 간 관계와 검증 기대를 정의합니다.
- 같은 이름이 양쪽에 보여도 같은 물건이 아닙니다. `CloseReadinessBlocker`는 API 스키마가 정의하는 데이터 형태이고, `blockers`는 저장 행입니다. `ArtifactRef`는 API 스키마이고, `artifacts`와 `artifact_links`는 저장 기록입니다.
- 저장 배치는 템플릿 문구를 소유하지 않습니다. 상태 카드, 판단 요청, 실행/증거 요약, 닫기 결과, 에이전트 맥락 패킷의 사용자 표시 본문은 [템플릿 본문](template-bodies.md)이 담당하고, 읽기 전용 상태 보기 권한과 최신성 경계는 [상태 보기 권한 참조](projection-and-templates.md)가 담당합니다.
- 영속 기록과 보안 보장 수준은 서로 다른 경계입니다.

이 문서가 직접 담당하는 것은 아래 항목입니다.

- 런타임 홈 식별과 프로젝트별 로컬 저장소 배치 가정.
- 활성 영속 기록 범주와 테이블 수준 저장 역할.
- 향후 저장소 설계를 위한 기록 열 의미.
- 저장소 소유 기록 값과 상태 필드.
- 저장소가 소유하는 JSON `TEXT` 배치와 검증 기대.
- 기록 수준의 현재/이후 제외 경계.

이 문서가 담당하지 않는 것은 아래 항목입니다.

- 메서드별 저장 효과: [저장 효과](storage-effects.md)
- 아티팩트 스테이징, 승격, 연결, 본문 읽기, 보존, 무결성 생명주기: [아티팩트 저장소](storage-artifacts.md)
- `project_state.state_version`, 멱등성, 이벤트 의미, 잠금, 마이그레이션: [저장소 버전 관리](storage-versioning.md)
- API 스키마가 정의하는 요청 또는 응답 형태: [API 코어 스키마](api/schema-core.md), [API 상태 스키마](api/schema-state.md), [API 아티팩트 스키마](api/schema-artifacts.md)와 다른 API 스키마 담당 문서
- API 메서드 동작: [API 메서드](api/methods.md)와 메서드 담당 문서
- 런타임/제품 저장소/서버 경계: [런타임 경계](runtime-boundaries.md)

<a id="runtime-home에-속하는-것"></a>
## 런타임 홈에 속하는 것

규칙:

- 하네스는 로컬 런타임 홈 하나와 등록된 프로젝트별 로컬 상태 데이터베이스 하나를 사용합니다.
- 기본 기준 루트는 `~/.harness`입니다.

허용되는 것:

- 구현은 같은 역할을 하는 설정 루트를 선택할 수 있습니다.

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

런타임 홈에 속하는 경로와 파일의 의미는 저장 배치 가정에 포함됩니다.

- `~/.harness`로 표시한 루트는 하네스 운영 데이터 공간입니다. 제품 저장소가 아니며 파일시스템 권한을 부여하지 않습니다.
- `registry.sqlite`는 런타임 홈 식별 정보와 최소 프로젝트 등록을 저장합니다. 런타임 홈 레지스트리이지 프로젝트별 Task 상태가 아닙니다.
- `projects/{project_id}/`는 등록된 프로젝트 하나의 하네스 프로젝트 홈입니다. `repo_root`와 같은 뜻이 아닙니다.
- `project.yaml`은 정적 프로젝트 설정만 저장합니다.
- `state.sqlite`는 등록된 프로젝트의 프로젝트별 로컬 Core 상태를 저장합니다.
- `artifacts/`는 프로젝트 아티팩트 저장소입니다. `artifacts/tmp/`는 임시 스테이징 공간이며 증거 권한이 아닙니다.

규칙:

- 런타임 홈 식별은 파일시스템 경로에만 의존하면 안 됩니다.
- 복사되거나 이동된 런타임 홈은 같은 저장된 `runtime_home_id`를 가질 수 있습니다.
- 새 런타임 홈은 새 식별자를 가져야 합니다.
- 런타임 홈 파일은 로컬 운영 제어 데이터이고 민감한 지원 데이터를 담을 수 있습니다.

허용되는 것:

- 이 식별자는 의심스러운 복사본, 중복 등록, 경로 변경을 감지하는 데 도움이 됩니다.

허용되지 않는 것:

- 이 식별자는 보안 보장이 아닙니다.

담당 문서 링크:

- 보안 비주장과 보장 수준은 [보안](security.md)이 담당합니다.
- 위치 경계는 [런타임 경계](runtime-boundaries.md)가 담당합니다.

<a id="runtime-home에-속하지-않는-것"></a>
## 런타임 홈에 속하지 않는 것

런타임 홈은 하네스 운영 데이터 공간입니다. 자동으로 제품 저장소가 되지 않고, 제품 저장소에 대한 쓰기 권한이나 안전성 판단을 대신하지도 않습니다.

- 제품 소스 파일, 제품 저장소의 일반 작업 트리, 빌드 산출물은 런타임 홈에 속하지 않습니다.
- `repo_root`는 등록된 제품 저장소 경로이고, `projects/{project_id}/`는 하네스 프로젝트 홈입니다. 둘을 같은 위치나 같은 권한으로 취급하지 않습니다.
- `project.yaml`은 현재 Task 상태, 관문, Write Authorization 상태, 증거 충분성, 최종 수락, 잔여 위험 수락, 닫기 상태를 저장하면 안 됩니다.
- 생성된 상태 보기 본문, 템플릿 표시 문구, Evidence Manifest 본문, QA 기록, 수락 기록, 잔여 위험 기록, 닫기 기록은 이 저장소 기록 배치에 속하지 않습니다.
- 이 문서는 런타임 서버 구현, 제품 구현 코드, 운영 파일 생성, 실제 런타임 상태 생성을 시작한다는 허가가 아닙니다.

## 활성 영속 기록 범주

### 조건

현재 MVP는 활성 상태 변경 메서드 집합에 필요한 Core 영속 기록만 저장합니다. 대상은 아래 메서드와 의도 값입니다.

- `harness.intake`.
- `harness.update_scope`.
- `harness.prepare_write`.
- `harness.record_run`.
- `harness.request_user_judgment`.
- `harness.record_user_judgment`.
- 상태를 바꾸는 `harness.close_task` 의도 값.

`harness.status`와 `harness.close_task intent=check`는 읽기 전용입니다.

### 저장되는 것

활성 Core 영속 기록은 다음뿐입니다.

- `registry.sqlite`의 런타임 홈 식별 정보.
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

### 임시 저장 경계

활성 임시 저장 경계는 `artifact_staging` 또는 동등한 저장소 소유 스테이징 기록과 `artifacts/tmp/` 아래 안전한 임시 바이트 또는 알림입니다. 이는 저장 위치 설명일 뿐이며, 아티팩트 스테이징 생명주기, 출처, 소비, 승격은 [아티팩트 저장소](storage-artifacts.md)가 담당합니다.

### 저장되지 않는 것

그 밖의 영속 테이블 계열이나 임시 핸들 계열은 현재 MVP 범위가 아닙니다.

요구사항 구체화는 `tasks`, `change_units`, `user_judgments`, `evidence_summaries`, `blockers`를 통해 저장합니다. 아래 별도 커밋 테이블을 만들지 않습니다.

- Discovery Brief.
- Shared Design.
- Question Queue.
- Assumption Register.
- First Safe Change Unit Candidate.

증거는 간결한 증거 요약, Task 또는 Change Unit의 `CompletionPolicy`, 필수 범위 항목, 아티팩트 참조를 통해 저장합니다. 전체 Evidence Manifest 저장소를 요구하지 않습니다.

### 상태 보기 비주장

상태 보기는 현재 MVP에서 별도 테이블 계열로 영속 저장하지 않습니다. `status-card`, `judgment-request`, `run-evidence-summary`, `close-result`, `agent-context-packet`은 활성 기록 위에 읽는 시점에 만든 보기이며, 저장된 상태나 저장소 변경 경로가 아닙니다.

### 구체화 저장

최소 활성 구체화 정보도 기존 기록에 저장합니다. 여기에는 아래 항목이 포함됩니다.

- 현재 목표 요약.
- 활성 범위 요약.
- 허용 경로 또는 영향 영역.
- 범위 밖 항목.
- 수락 기준.
- 자율성 경계.
- 필요한 사용자 소유 판단.
- 필요할 때 막히는 질문 하나.
- 다음 안전한 행동 하나.
- `CompletionPolicy`.
- 필수 증거 기대 또는 증거 공백.
- 닫기 준비 상태.

빠졌거나 알 수 없는 항목은 `unknown`, 대기 중인 `user_judgments`, 증거 공백, `blockers`로 남깁니다. 저장소는 요청을 준비된 것처럼 보이게 만들려고 별도 활성 계획 테이블을 만들면 안 됩니다.

## 테이블 개요

아래 표는 현재 MVP의 활성 저장 기록 범주를 간결하게 보여 주고 범주별 세부 설명으로 연결합니다. 전체 DDL이 아니며 API 스키마나 렌더링된 템플릿 본문을 복사하지 않습니다. 세부 설명은 영속 기록이 저장하는 내용과 저장하지 않는 내용을 분리합니다.

| 저장 기록 범주 | 목적 | 세부사항 |
|---|---|---|
| 런타임 홈 식별 정보 | 로컬 런타임 홈과 저장 프로필 식별 | [런타임 홈 식별 정보](#runtime-home-identity) 참고 |
| 프로젝트 등록 | 등록된 프로젝트와 프로젝트별 로컬 저장소 연결 | [프로젝트 등록](#project-registration) 참고 |
| `project.yaml` | 정적 프로젝트 설정 | [`project.yaml`](#projectyaml) 참고 |
| `project_state` | 현재 프로젝트 상태, 버전, 활성 포인터 저장 | [`project_state`](#project_state) 참고 |
| `surfaces` | API 접근 점검에 쓰는 등록된 로컬 접점 사실 저장 | [`surfaces`](#surfaces) 참고 |
| `tasks` | 작업 단위, 구체화, 생명주기, 닫기 상태 저장 | [`tasks`](#tasks) 참고 |
| `change_units` | 쓰기와 닫기 기준이 되는 범위 경계 저장 | [`change_units`](#change_units) 참고 |
| `user_judgments` | 사용자 소유 판단과 민감 동작 승인 기록 저장 | [`user_judgments`](#user_judgments) 참고 |
| `write_authorizations` | 단일 사용 협력형 Write Authorization 기록 저장 | [`write_authorizations`](#write_authorizations) 참고 |
| `runs` | 커밋된 실행 또는 관찰 기록 저장 | [`runs`](#runs) 참고 |
| `artifact_staging` | 임시 스테이징 아티팩트 핸들 저장 | [`artifact_staging`](#artifact_staging) 참고 |
| `artifacts` | 등록된 영속 아티팩트 메타데이터 또는 바이트 저장 | [`artifacts`](#artifacts) 참고 |
| `artifact_links` | 아티팩트와 지원 대상 Core/API 기록의 담당 관계 저장 | [`artifact_links`](#artifact_links) 참고 |
| `evidence_summaries` | 간결한 증거 범위와 공백 기록 저장 | [`evidence_summaries`](#evidence_summaries) 참고 |
| `blockers` | 구조화된 차단 사유 상태 저장 | [`blockers`](#blockers) 참고 |
| `task_events` | 커밋된 Core 변경의 추가 전용 감사 및 순서 기록 저장 | [`task_events`](#task_events) 참고 |
| `tool_invocations` | 커밋된 Core 메서드 재실행 행 저장 | [`tool_invocations`](#tool_invocations) 참고 |

## 저장 기록 범주 세부사항

<a id="runtime-home-identity"></a>
### 런타임 홈 식별 정보

목적:
- 로컬 런타임 홈과 스키마/저장 프로필을 식별합니다.

저장 위치:
- `registry.sqlite`.

포함하는 것:
- `runtime_home_id`.
- `schema_version`과 `storage_profile`.
- `created_at`과 `updated_at`.

포함하지 않는 것:
- 프로젝트별 Task 상태.
- 제품 저장소 내용이나 권한.
- 변조 불가능성을 증명하는 자료.

담당 문서 링크:
- [런타임 경계](runtime-boundaries.md).
- [보안](security.md).

<a id="project-registration"></a>
### 프로젝트 등록

목적:
- 등록된 프로젝트를 프로젝트별 로컬 저장소에 연결합니다.

저장 위치:
- `registry.sqlite`.

포함하는 것:
- `project_id`.
- `repo_root`와 `project_home`.
- `display_name`과 `status`.
- `created_at`과 `updated_at`.

포함하지 않는 것:
- 현재 Task 생명주기 상태.
- 제품 저장소 파일 내용.
- 기준 현재 MVP를 넘어서는 다중 등록 동작.

담당 문서 링크:
- [런타임 경계](runtime-boundaries.md).
- [저장소 버전 관리](storage-versioning.md).

<a id="projectyaml"></a>
### `project.yaml`

목적:
- 등록된 프로젝트 하나의 정적 프로젝트 설정을 저장합니다.

저장 위치:
- 런타임 홈 아래 프로젝트 디렉터리.

포함하는 것:
- `project_id`.
- `repo_root`.
- 표시와 설정 기본값.

포함하지 않는 것:
- 현재 Task 상태, 관문, Write Authorization 상태.
- 증거 충분성, 최종 수락, 잔여 위험 수락, 닫기 상태.
- 렌더링된 템플릿 문구.

담당 문서 링크:
- [런타임 경계](runtime-boundaries.md).
- [저장소 버전 관리](storage-versioning.md).

<a id="project_state"></a>
### `project_state`

목적:
- 프로젝트별 로컬 상태 헤더, 공개 프로젝트 전체 상태 시계, 활성 Task 포인터, 기본 접점 포인터를 저장합니다.

저장 위치:
- `state.sqlite`.

포함하는 것:
- `project_id`.
- `schema_version`, `storage_profile`, `state_version`.
- `active_task_id`와 `default_surface_id`.
- `created_at`과 `updated_at`.

포함하지 않는 것:
- 아티팩트 바이트.
- API 요청 또는 응답 본문.
- 렌더링된 템플릿 문구.
- 변조 불가능성을 증명하는 자료.

담당 문서 링크:
- [저장소 버전 관리](storage-versioning.md).
- [API 상태 스키마](api/schema-state.md).

<a id="surfaces"></a>
### `surfaces`

목적:
- API 접근에 쓸 로컬 접점 맥락을 확인하기 위한 `LocalSurfaceRegistration` 사실을 저장합니다.

저장 위치:
- `state.sqlite`.

포함하는 것:
- `project_id`, `surface_id`, `surface_instance_id`.
- `transport_kind`, `transport_binding_fingerprint`, `access_secret_hash`.
- `capability_profile_hash`와 `capability_profile_json`.
- `status`, `local_access_posture`, `registered_at`, `last_verified_at`, `updated_at`.

포함하지 않는 것:
- 현재 호출자가 신뢰된다는 실시간 증명.
- 호출자가 제공한 권한 주장.
- 호스팅 커넥터 등록소 상태.

담당 문서 링크:
- [에이전트 통합](agent-integration.md).
- [API 메서드](api/methods.md).
- [보안](security.md).

<a id="tasks"></a>
### `tasks`

목적:
- 사용자 가치 작업 단위, 구체화 요약, 생명주기, 결과, 다음 행동, Task 수준 활성 `CompletionPolicy`, 닫기 필드를 저장합니다.

저장 위치:
- `state.sqlite`.

포함하는 것:
- `task_id`, `project_id`, `title`, `user_request`, `current_goal_summary`.
- `mode`, `lifecycle_phase`, `close_reason`, `result`, `summary`.
- 구체화 JSON 열과 `completion_policy_json`.
- `blocking_question`, `next_safe_action`, `active_change_unit_id`.
- `created_at`, `updated_at`, `closed_at`.

포함하지 않는 것:
- 별도 커밋된 Discovery Brief, Question Queue, Assumption Register, First Safe Change Unit Candidate.
- 전체 Evidence Manifest 저장소.
- 렌더링된 `status-card` 또는 `close-result` 본문.

담당 문서 링크:
- [Core Model](core-model.md).
- [API 상태 스키마](api/schema-state.md).
- [템플릿 본문](template-bodies.md).

<a id="change_units"></a>
### `change_units`

목적:
- 쓰기 호환성, Change Unit 수준 `CompletionPolicy`, 닫기 근거를 위한 현재 또는 제안된 범위 있는 작업 경계를 저장합니다.

저장 위치:
- `state.sqlite`.

포함하는 것:
- `change_unit_id`, `task_id`, `scope_summary`.
- 허용 경로 또는 영향 영역을 담는 범위 JSON 열.
- `baseline_ref`, `autonomy_boundary_json`, `completion_policy_json`.
- `status`, `created_at`, `updated_at`.

포함하지 않는 것:
- 별도 Shared Design 또는 First Safe Change Unit Candidate 테이블.
- 제품 저장소 diff 바이트.
- Write Authorization record.

담당 문서 링크:
- [Core Model](core-model.md).
- [저장 효과](storage-effects.md).
- [API 메서드](api/methods.md).

<a id="user_judgments"></a>
### `user_judgments`

목적:
- 사용자 소유 판단 기록을 저장하며, 필요하면 별도 민감 동작 승인 범위도 저장합니다.

저장 위치:
- `state.sqlite`.

포함하는 것:
- `user_judgment_id`, `task_id`, `change_unit_id`.
- `judgment_kind`, `presentation`, `status`.
- 요청/맥락 JSON 열, `question`, `sensitive_action_scope_json`.
- `resolution_json`, `expires_at`, `resolved_at`, `created_at`, `updated_at`.

포함하지 않는 것:
- Core 소유 상태 권한이나 아티팩트 권한.
- 아티팩트 바이트.
- 기록된 판단 범위를 넘어서는 포괄 승인.

담당 문서 링크:
- [API 판단 스키마](api/schema-judgment.md).
- [Core Model](core-model.md).
- [API 메서드](api/methods.md).

<a id="write_authorizations"></a>
### `write_authorizations`

목적:
- `dry_run=false`인 `prepare_write`에서 `decision=allowed`일 때만 만들어지는 영속적인 단일 사용 협력형 Write Authorization을 저장합니다.

저장 위치:
- `state.sqlite`.

포함하는 것:
- `write_authorization_id`, `task_id`, `change_unit_id`, `surface_id`.
- `status`, `basis_state_version`, `attempt_scope_json`.
- `consumed_by_run_id`, `expires_at`, `created_at`, `updated_at`, `consumed_at`.

포함하지 않는 것:
- 제품 저장소 쓰기 자체.
- 증거 충분성이나 최종 수락.
- 재사용 가능한 권한이나 예방형 보안 보장.

담당 문서 링크:
- [저장 효과](storage-effects.md).
- [API 메서드](api/methods.md).
- [보안](security.md).

<a id="runs"></a>
### `runs`

목적:
- 제품 쓰기가 있었다면 호환되는 Write Authorization 소비까지 포함하는 커밋된 실행 또는 관찰 기록을 저장합니다.

저장 위치:
- `state.sqlite`.

포함하는 것:
- `run_id`, `task_id`, `change_unit_id`, `write_authorization_id`, `surface_id`.
- `kind`, `status`, `product_write`, `baseline_ref`, `summary`.
- 관찰/증거 JSON 열.
- `created_at`, `completed_at`.

포함하지 않는 것:
- 아티팩트 바이트.
- 렌더링된 실행/증거 요약 문구.
- 최종 수락이나 잔여 위험 수락 자체.

담당 문서 링크:
- [저장 효과](storage-effects.md).
- [API 상태 스키마](api/schema-state.md).
- [Core Model](core-model.md).

<a id="artifact_staging"></a>
### `artifact_staging`

목적:
- `harness.stage_artifact`가 만들고 나중에 `harness.record_run`이 한 번만 소비할 수 있는 임시 안전 바이트 또는 안전한 알림을 저장합니다.

저장 위치:
- `state.sqlite`와 `artifacts/tmp/` 아래 안전한 임시 바이트 또는 알림.

포함하는 것:
- `handle_id`, `project_id`, `task_id`, `created_by_surface_id`, `created_by_surface_instance_id`.
- `display_name`, `relation_hint`, `tmp_uri`, `sha256`, `size_bytes`, `content_type`, `redaction_state`.
- `status`, `consumed_by_run_id`, `promoted_artifact_id`, `expires_at`, `created_at`, `consumed_at`.

포함하지 않는 것:
- 영속 `ArtifactRef` 권한.
- 증거 충분성이나 닫기 준비 상태.
- 접점 간 스테이징 아티팩트 핸드오프.

담당 문서 링크:
- [아티팩트 저장소](storage-artifacts.md).
- [API 아티팩트 스키마](api/schema-artifacts.md).
- [API 메서드](api/methods.md).

<a id="artifacts"></a>
### `artifacts`

목적:
- 무결성, 가림 처리, 생산자, 보존, 가용성 사실을 가진 등록된 영속 증거 바이트 또는 안전한 메타데이터를 저장합니다.

저장 위치:
- `state.sqlite`와 프로젝트 아티팩트 저장소.

포함하는 것:
- `artifact_id`, `project_id`, `task_id`, `run_id`.
- `kind`, `uri`, `sha256`, `size_bytes`, `content_type`, `redaction_state`.
- `retention_class`, `produced_by`, `status`, `created_at`, `updated_at`.

포함하지 않는 것:
- 증거 충분성 자체.
- 렌더링된 증거 요약 본문.
- 제한 없는 본문 읽기 권한.

담당 문서 링크:
- [아티팩트 저장소](storage-artifacts.md).
- [API 아티팩트 스키마](api/schema-artifacts.md).
- [보안](security.md).

<a id="artifact_links"></a>
### `artifact_links`

목적:
- 아티팩트와 그것이 뒷받침하는 활성 Core/API 기록 사이의 담당 관계를 저장합니다.

저장 위치:
- `state.sqlite`.

포함하는 것:
- `artifact_link_id`, `artifact_id`, `task_id`.
- `owner_record_kind`, `owner_record_id`, `relation`.
- `created_at`.

포함하지 않는 것:
- 아티팩트 바이트.
- 담당 기록 본문이나 API 스키마 객체.
- 아티팩트가 충분한 증거라는 증명.

담당 문서 링크:
- [아티팩트 저장소](storage-artifacts.md).
- [API 값 집합](api/schema-value-sets.md).
- [Core Model](core-model.md).

<a id="evidence_summaries"></a>
### `evidence_summaries`

목적:
- 상태, 실행/증거 요약, 차단 사유, 닫기에 쓰는 간결한 증거 범위와 공백 기록을 저장합니다.

저장 위치:
- `state.sqlite`.

포함하는 것:
- `evidence_summary_id`, `task_id`, `change_unit_id`.
- `status`, `coverage_items_json`, `summary`.
- `supporting_run_ids_json`, `supporting_artifact_link_ids_json`, `gap_blocker_ids_json`.
- `updated_at`.

포함하지 않는 것:
- 전체 Evidence Manifest 저장소.
- 전체 수동 QA 행렬.
- 최종 수락.
- 렌더링된 `run-evidence-summary` 본문.

담당 문서 링크:
- [Core Model](core-model.md).
- [API 상태 스키마](api/schema-state.md).
- [템플릿 본문](template-bodies.md).

<a id="blockers"></a>
### `blockers`

목적:
- 다음 행동, 쓰기 호환성, 증거 공백, 닫기 준비 상태, 복구를 위한 구조화된 차단 사유 상태를 저장합니다.

저장 위치:
- `state.sqlite`.

포함하는 것:
- `blocker_id`, `task_id`, `blocked_action`, `blocker_kind`, `status`.
- `message`, `owner_ref_json`, `related_refs_json`, `required_next_action`.
- `created_at`, `resolved_at`.

포함하지 않는 것:
- 저장 행이나 영속 신호로서의 `CloseReadinessBlocker`.
- 닫기 준비 상태 전체 개념.
- 렌더링된 템플릿 문구.

담당 문서 링크:
- [Core Model](core-model.md).
- [API 상태 스키마](api/schema-state.md).
- [API 오류](api/errors.md).

<a id="task_events"></a>
### `task_events`

목적:
- 커밋된 Core 변경의 추가 전용 감사 및 순서 기록을 저장합니다.

저장 위치:
- `state.sqlite`.

포함하는 것:
- `event_id`, `project_id`, `task_id`, `event_seq`.
- `event_type`, `state_version`, `actor_kind`, `surface_id`.
- `payload_json`, `created_at`.

포함하지 않는 것:
- 제품 저장소 diff 바이트.
- 렌더링된 템플릿 본문.
- 외부 감사 보장이나 변조 방지 보장.

담당 문서 링크:
- [저장소 버전 관리](storage-versioning.md).
- [저장 효과](storage-effects.md).
- [보안](security.md).

<a id="tool_invocations"></a>
### `tool_invocations`

목적:
- 메서드별 상태 효과 행이 재실행 행 생성을 허용한, 커밋된 `dry_run=false` Core `MethodResult` 응답만 저장합니다.

저장 위치:
- `state.sqlite`.

포함하는 것:
- `invocation_id`, `project_id`, `tool_name`, `idempotency_key`.
- `request_hash`, `task_id`, `basis_state_version`.
- `response_json`, `status`, `created_at`.

포함하지 않는 것:
- 재실행 효과가 없는 `dry_run` 또는 읽기 전용 응답.
- API 스키마 정의.
- 같은 멱등 키를 여러 커밋 응답으로 갈라지게 하는 권한.

담당 문서 링크:
- [저장소 버전 관리](storage-versioning.md).
- [API 메서드](api/methods.md).
- [API 코어 스키마](api/schema-core.md).

## 첫 스키마 무결성 계약

규칙:

- 이 섹션은 향후 SQLite 첫 스키마 설계를 위한 저장 계약입니다.
- 첫 SQLite 스키마에서는 커밋된 행이 아래 식별자, 값 집합, 관계, 트랜잭션 경계를 보존해야 합니다.

허용되는 것:

- 구현은 `CHECK` 제약, 조회 테이블, 생성 열, 트리거, Core 쪽 검증 중 알맞은 방식을 고를 수 있습니다.

허용되지 않는 것:

- 이 섹션은 전체 DDL, 마이그레이션 파일, 또는 런타임 구현이 시작되었다는 증거가 아닙니다.
- 구현 방식 선택이 Core/API/저장소 담당 문서의 검증을 대체하지 않습니다.

필수 식별성과 고유 제약은 다음과 같습니다.

- 활성 테이블은 불투명하고 안정적인 식별자를 기본 키 또는 동등한 고유 키로 사용합니다.

  식별자 필드는 아래 항목을 포함합니다.
  - `project_id`
  - `surface_id`
  - `surface_instance_id`
  - `task_id`
  - `change_unit_id`
  - `user_judgment_id`
  - `write_authorization_id`
  - `run_id`
  - `handle_id`
  - `artifact_id`
  - `artifact_link_id`
  - `evidence_summary_id`
  - `blocker_id`
  - `event_id`
  - `invocation_id`
- 런타임 홈 식별 정보는 해당 런타임 홈의 `runtime_home_id` 하나를 저장합니다.
- 프로젝트 등록에는 고유한 `project_id`와 고유한 `project_home`이 필요합니다. 향후 담당 문서가 다중 등록 동작을 정의하기 전까지 `repo_root` 하나에는 활성 등록 하나만 둡니다.
- `project_state.project_id`는 등록된 프로젝트마다 한 행입니다.
- `surfaces`에는 고유한 `(project_id, surface_id)`가 필요합니다. 저장된 `surface_instance_id`는 그 접점 행이 선택한 등록 로컬 인스턴스를 식별합니다.
- `tasks`에는 고유한 `(project_id, task_id)`가 필요합니다.
- `change_units`에는 고유한 `(task_id, change_unit_id)`가 필요하며, Task마다 `status=active`인 Change Unit은 최대 하나입니다.
- `write_authorizations.consumed_by_run_id`, `runs.write_authorization_id`, `artifact_staging.consumed_by_run_id`, `artifact_staging.promoted_artifact_id`는 `null`이 아닐 때 고유합니다.
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

### 권한 행 삭제 비주장

규칙:

- 일반적인 현재 MVP Core 동작은 권한 행을 하드 삭제하지 않습니다.
- 행은 상태 또는 생명주기 필드로 이동합니다.
- Core는 새 이벤트를 추가합니다.
- 재실행과 아티팩트 메타데이터는 감사와 복구에 사용할 수 있게 유지합니다.
- 외래 키는 권한 행에 대해 기본적으로 `RESTRICT` 또는 동등한 담당 검증을 사용해야 합니다.

Task를 완료, 취소, 대체해도 아래 행이 연쇄 삭제되면 안 됩니다.

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

### 임시 스테이징 예외

허용되는 것:

- 소비되지 않았거나 만료된 `artifact_staging` 행과 `artifacts/tmp/`의 스테이징 바이트 또는 알림은 `expired` 또는 `discarded`로 표시할 수 있습니다.
- 등록 전 임시 바이트는 정리할 수 있습니다.

예외:

- 이 예외가 허용되는 이유는 해당 행과 임시 바이트가 증거 권한이 아니기 때문입니다.

허용되지 않는 것:

- `artifacts` 행이 커밋된 뒤의 보존 삭제, 프로젝트 해체, 파괴적 정리는 일반적인 현재 MVP 변경 동작 밖이며 담당 문서가 정의한 경로가 필요합니다.

## 저장소 소유 값 요약

규칙:

- 저장 값 집합은 현재 MVP의 테이블 수준 영속 제약입니다.
- API 스키마 값을 반영하는 행은 API 스키마 담당 문서와 정확히 맞아야 합니다.
- 저장소 소유로 표시된 행은 공개 API 스키마 본문이 아니라 저장 동작을 정의합니다.

허용되지 않는 것:

- 알 수 없는 값은 커밋할 수 없습니다.

| 필드 | 목적 | 세부사항 |
|---|---|---|
| 프로젝트 등록 `status` | 활성 프로젝트 등록 기준선 | [프로젝트 등록 `status`](#project-registration-status) 참고 |
| `surfaces.transport_kind` | 저장된 로컬 전송 범주 | [`surfaces.transport_kind`](#surfacestransport_kind) 참고 |
| `surfaces.local_access_posture` | 저장된 등록 상태 | [`surfaces.local_access_posture`](#surfaceslocal_access_posture) 참고 |
| `surfaces.status` | 저장된 접점 등록 사용 가능성 | [`surfaces.status`](#surfacesstatus) 참고 |
| `tasks.lifecycle_phase` | 영속 Task 생명주기 | [`tasks.lifecycle_phase`](#taskslifecycle_phase) 참고 |
| `tasks.close_reason` | 영속 닫기 세부값 | [`tasks.close_reason`](#tasksclose_reason) 참고 |
| `tasks.result` | 영속되는 상위 수준 Task 결과 | [`tasks.result`](#tasksresult) 참고 |
| `change_units.status` | 활성 Change Unit 생명주기 | [`change_units.status`](#change_unitsstatus) 참고 |
| `write_authorizations.status` | 영속 승인 생명주기 | [`write_authorizations.status`](#write_authorizationsstatus) 참고 |
| `artifact_staging.status` | 임시 핸들 생명주기 | [`artifact_staging.status`](#artifact_stagingstatus) 참고 |
| `artifacts.status` | 아티팩트 가용성 상태 | [`artifacts.status`](#artifactsstatus) 참고 |
| `artifact_links.owner_record_kind` | 담당 관계 판별자 | [`artifact_links.owner_record_kind`](#artifact_linksowner_record_kind) 참고 |
| `blockers.status` | 차단 사유 행 상태 | [`blockers.status`](#blockersstatus) 참고 |
| `tool_invocations.status` | 커밋된 재실행 행 상태 | [`tool_invocations.status`](#tool_invocationsstatus) 참고 |

<a id="project-registration-status"></a>
### 프로젝트 등록 `status`

값:
- `active`

저장 규칙:
- 기준 현재 MVP에는 등록된 활성 프로젝트만 있습니다.
- 비활성화/등록 해제 동작은 승격되기 전까지 이후 후보입니다.

담당 문서 링크:
- [런타임 경계](runtime-boundaries.md).
- [저장소 버전 관리](storage-versioning.md).

<a id="surfacestransport_kind"></a>
### `surfaces.transport_kind`

값:
- `local_mcp_stdio`
- `local_http`

저장 규칙:
- 등록 매칭을 위한 저장된 로컬 전송 범주입니다.
- 소켓이나 프로토콜 설정 명세가 아닙니다.

담당 문서 링크:
- [에이전트 통합](agent-integration.md).
- [API 메서드](api/methods.md).
- [보안](security.md).

<a id="surfaceslocal_access_posture"></a>
### `surfaces.local_access_posture`

값:
- `registered_local`
- `unavailable`
- `mismatch`
- `revoked`

저장 규칙:
- API 호환성 확인에 쓰는 저장된 등록 상태입니다.
- 의미는 API 스키마 담당 문서와 맞습니다.

담당 문서 링크:
- [API 코어 스키마](api/schema-core.md).
- [에이전트 통합](agent-integration.md).
- [보안](security.md).

<a id="surfacesstatus"></a>
### `surfaces.status`

값:
- `active`
- `disabled`
- `stale`
- `revoked`

저장 규칙:
- 저장된 접점 등록 사용 가능성입니다.
- 의미는 API 스키마 담당 문서와 맞습니다.

담당 문서 링크:
- [API 코어 스키마](api/schema-core.md).
- [에이전트 통합](agent-integration.md).
- [보안](security.md).

<a id="taskslifecycle_phase"></a>
### `tasks.lifecycle_phase`

값:
- `shaping`
- `ready`
- `executing`
- `waiting_user`
- `blocked`
- `completed`
- `cancelled`
- `superseded`

저장 규칙:
- 영속 Task 생명주기를 나타냅니다.
- `intake`는 저장되는 생명주기 값이 아닙니다.
- `superseded`는 종료 상태입니다.

담당 문서 링크:
- [Task 생명주기 값](api/schema-value-sets.md#task-lifecycle-values).
- [API 상태 스키마](api/schema-state.md).
- [Core 모델](core-model.md).

<a id="tasksclose_reason"></a>
### `tasks.close_reason`

값:
- `none`
- `completed_self_checked`
- `completed_with_risk_accepted`
- `cancelled`
- `superseded`

저장 규칙:
- 생명주기와 결과에서 분리된 영속 닫기 세부값입니다.

담당 문서 링크:
- [Task 생명주기 값](api/schema-value-sets.md#task-lifecycle-values).
- [API 상태 스키마](api/schema-state.md).
- [Core 모델](core-model.md).

<a id="tasksresult"></a>
### `tasks.result`

값:
- `none`
- `advice_only`
- `completed`
- `cancelled`
- `superseded`

저장 규칙:
- 영속되는 상위 수준 결과입니다.
- 실패한 Run, 위반, 차단된 닫기, 증거 공백은 담당 기록에 남습니다.

담당 문서 링크:
- [Task 생명주기 값](api/schema-value-sets.md#task-lifecycle-values).
- [API 상태 스키마](api/schema-state.md).
- [Core 모델](core-model.md).

<a id="change_unitsstatus"></a>
### `change_units.status`

값:
- `proposed`
- `active`
- `replaced`
- `closed`

저장 규칙:
- 저장소 소유 활성 Change Unit 생명주기입니다.
- 이 생명주기는 쓰기 호환성과 닫기 근거에 쓰입니다.

담당 문서 링크:
- [Core 모델](core-model.md).
- [저장 효과](storage-effects.md).
- [API 메서드](api/methods.md).

<a id="write_authorizationsstatus"></a>
### `write_authorizations.status`

값:
- `active`
- `consumed`
- `expired`
- `stale`
- `revoked`

저장 규칙:
- 영속 승인 생명주기입니다.
- 저장소가 영속 저장과 전이 규칙을 담당합니다.

담당 문서 링크:
- [저장 효과](storage-effects.md).
- [API 메서드](api/methods.md).
- [보안](security.md).

<a id="artifact_stagingstatus"></a>
### `artifact_staging.status`

값:
- `staged`
- `consumed`
- `expired`
- `discarded`

저장 규칙:
- 저장소 소유 임시 핸들 생명주기입니다.
- `harness.record_run`이 소비할 수 있는 값은 `staged`뿐입니다.
- 종료 값은 `staged`로 돌아갈 수 없습니다.

담당 문서 링크:
- [아티팩트 저장소](storage-artifacts.md).
- [API 아티팩트 스키마](api/schema-artifacts.md).
- [API 메서드](api/methods.md).

<a id="artifactsstatus"></a>
### `artifacts.status`

값:
- `available`
- `missing`
- `integrity_failed`
- `unavailable`

저장 규칙:
- 저장소 소유 아티팩트 가용성 상태입니다.
- 가림 처리와 차단된 페이로드 처리는 `redaction_state`에 남습니다.

담당 문서 링크:
- [아티팩트 저장소](storage-artifacts.md).
- [API 아티팩트 스키마](api/schema-artifacts.md).
- [보안](security.md).

<a id="artifact_linksowner_record_kind"></a>
### `artifact_links.owner_record_kind`

값:
- `task`
- `change_unit`
- `run`
- `user_judgment`
- `evidence_summary`
- `blocker`

저장 규칙:
- 영속 담당 관계 판별자입니다.
- 저장소가 같은 프로젝트/같은 Task 담당 행 조회와 관계 검증을 담당합니다.

담당 문서 링크:
- [아티팩트 저장소](storage-artifacts.md).
- [기록과 참조 값](api/schema-value-sets.md#record-and-reference-values).
- [Core 모델](core-model.md).

<a id="blockersstatus"></a>
### `blockers.status`

값:
- `active`
- `resolved`
- `superseded`

저장 규칙:
- 저장소 소유 차단 사유 행 상태입니다.
- 공개 닫기 차단 사유 형태는 API 담당 문서에 남습니다.

담당 문서 링크:
- [Core 모델](core-model.md).
- [API 상태 스키마](api/schema-state.md).
- [API 오류](api/errors.md).

<a id="tool_invocationsstatus"></a>
### `tool_invocations.status`

값:
- `committed`

저장 규칙:
- 재실행 행은 메서드별 상태 효과 행이 재실행 행 생성을 허용한 커밋된 `dry_run=false` Core `MethodResult` 응답에만 존재합니다.

담당 문서 링크:
- [저장소 버전 관리](storage-versioning.md).
- [API 메서드](api/methods.md).
- [API 코어 스키마](api/schema-core.md).

### 담당하지 않는 값

규칙:

- 다른 영속 상태형 API 필드는 [API 값 집합](api/schema-value-sets.md)과 Core/API 메서드 담당 문서를 기준으로 검증합니다.

예:

- `tasks.mode`.
- `runs.kind`.
- `runs.status`.
- `user_judgments.status`.
- `evidence_summaries.status`.

허용되는 것:

- 저장소는 색인과 제약을 둘 수 있습니다.

허용되지 않는 것:

- 이 문서는 공개 스키마 값을 다시 정의하지 않습니다.

## 저장소 소유 JSON

### JSON 저장 조건

JSON을 저장하는 SQLite `TEXT` 열은 저장 표현 선택일 뿐이며 임의 JSON을 저장해도 된다는 뜻이 아닙니다.

- Core는 커밋 전에 JSON을 파싱하고 검증해야 합니다.
- API 형태의 저장 JSON은 API 스키마 담당 문서를 기준으로 검증합니다.
- 저장소 전용 JSON은 이 문서나 이 문서가 가리키는 담당 문서를 기준으로 검증합니다.
- `'{}'`, `'[]'` 같은 SQLite 기본값은 저장 기본값일 뿐이며 API 필드를 선택 필드로 만들지 않습니다.

### 저장되는 JSON

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

### 저장되지 않는 JSON

규칙:

- Task와 Change Unit 구체화 JSON은 간결한 요약과 제한된 목록만 저장합니다.

허용되지 않는 것:

아래 항목을 다른 이름으로 저장하면 안 됩니다.

- 독립 Discovery Brief.
- Question Queue.
- Assumption Register.
- 전체 설계 아티팩트.
- 생성된 상태 보기 본문.
- Evidence Manifest 본문.
- QA 기록.
- 수락 기록.
- 잔여 위험 기록.
- 닫기 기록.

## 현재/이후 경계

### 조건

규칙:

- 프로필로 제한된 이후 후보 저장소는 현재 MVP 밖에 있습니다.

예외:

- 담당 문서가 범위, 대체 동작, 향후 승격에 필요한 증명 경로 기대치와 함께 좁은 동작을 승격하면 해당 범위에서만 달라질 수 있습니다.

허용되지 않는 것:

- 참조 스키마에 존재한다는 사실만으로 저장소가 활성화되지 않습니다.

### 저장되지 않는 것

현재 MVP는 아래 저장소를 제외합니다.

- 상태 보기 작업.
- 영속 상태 보기 캐시.
- 관리 출력 outbox.
- 적합성 실행기 상태.
- 픽스처 실행 이력.
- 운영 프로필 저장소.
- `captured_artifact` 핸들.
- 접점 자체 캡처 저장소.
- 캡처 어댑터 출력 테이블.
- 전체 Evidence Manifest 테이블.
- 상세 증거 카탈로그.
- 분리형 검증.
- 전체 수동 QA 행렬.
- 풍부한 QA/면제 장치.
- `user_judgments`와 `blockers`에서 분리된 상세 승인 또는 잔여 위험 생명주기 테이블.
- 대시보드.
- 분석.
- 호스팅 커넥터 등록소.
- 접점 간 조율 저장소.
- 장기 설계 지원 저장소.

### 비주장

허용되는 것:

- 활성 상태, 닫기 준비 상태, 실행/증거 요약, 다음 행동, 읽기용 카드, `agent-context-packet`, 보장 표시는 활성 영속 기록 위에 읽는 시점에 파생하는 보기입니다.

예외:

- 이 출력은 오래되었거나, 없거나, 실패했을 수 있고 다시 계산될 수 있습니다.

허용되지 않는 것:

- 이런 출력은 저장소 권한을 바꾸지 않습니다.

## 관련 담당 문서

- [저장 효과](storage-effects.md): 어떤 메서드가 기록을 만들거나, 바꾸거나, 관찰하거나, 건드리지 않는지.
- [아티팩트 저장소](storage-artifacts.md): 아티팩트 전용 저장 생명주기.
- [저장소 버전 관리](storage-versioning.md): 시계, 멱등성, 잠금, 마이그레이션 의미.
- [API 메서드](api/methods.md)와 메서드 담당 문서: 기록을 사용하는 공개 메서드 동작.
- [API 코어 스키마](api/schema-core.md), [API 상태 스키마](api/schema-state.md), [API 아티팩트 스키마](api/schema-artifacts.md), [API 판단 스키마](api/schema-judgment.md), [API 값 집합](api/schema-value-sets.md): 요청/응답 형태와 공개 API 값.
- [템플릿 본문](template-bodies.md): 사용자에게 보이는 상태 카드, 판단 요청, 실행/증거 요약, 닫기 결과, 에이전트 맥락 패킷의 표시 본문.
- [상태 보기 권한 참조](projection-and-templates.md): 읽기 전용 상태 보기 권한, 원천 기록, 최신성 경계.
- [런타임 경계](runtime-boundaries.md): 런타임 홈, 제품 저장소, 서버 경계.
- [보안](security.md): 보안 비주장, 보장 수준, 변조 방지처럼 이 문서가 주장하지 않는 보안 의미.
