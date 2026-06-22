# 저장소 기록

이 문서는 기준 범위의 영속 저장소 기록 계열, 배치, 저장소 소유 값을 담당합니다. 영속 기록은 Core가 나중에 `Harness Runtime Home` 안에서 다시 읽을 수 있도록 커밋한 로컬 기록입니다.

영속 기록은 하네스 기록에 대한 로컬 Core 저장소 권한입니다.

보안 보장, 외부 감사 보장, 위조 방지 주장, `Product Repository` 쓰기 권한은 각 담당 문서에 남습니다.

## 담당 경계

이 문서가 담당합니다.

- 기준 범위 영속 기록 계열
- 그 기록 계열의 테이블, 파일, 아티팩트 저장소 위치
- 저장 범주와 관계 배치
- 저장소 소유 값 집합
- 저장소 소유 SQLite JSON `TEXT` 배치
- 커밋 전 기록 배치 검증 요구사항

이 문서가 담당하지 않는 것은 아래 항목입니다.

- 기준 SQLite DDL, 인덱스, 외래 키, 마이그레이션 테이블, 제약: [저장소 DDL](storage-ddl.md)
- 메서드 분기별 영속 효과: [저장 효과](storage-effects.md)
- 아티팩트 스테이징, 승격, 연결, 본문 읽기, 보존, 무결성 생명주기: [아티팩트 저장소](storage-artifacts.md)
- `project_state.state_version`, 멱등성, 재실행, 이벤트, 잠금, 마이그레이션 계약: [저장소 버전 관리](storage-versioning.md)
- API 요청 또는 응답 형태: [API 코어 스키마](api/schema-core.md), [API 상태 스키마](api/schema-state.md), [API 아티팩트 스키마](api/schema-artifacts.md), [API 판단 스키마](api/schema-judgment.md), [API 값 집합](api/schema-value-sets.md)
- API 메서드 동작: [API 메서드](api/methods.md)와 메서드 담당 문서
- 런타임 위치와 저장소 경계: [런타임 경계](runtime-boundaries.md)
- 보안 보장 수준과 보안 경계: [보안](security.md)

## 저장 위치

하네스는 기준 범위 기록을 로컬 `Harness Runtime Home` 하나와 등록된 프로젝트별 로컬 상태 데이터베이스 하나에 저장합니다. 기본 기준 루트는 `~/.harness`이며, 구현은 같은 역할을 하는 설정 루트를 선택할 수 있습니다.

```text
~/.harness/
  registry.sqlite
  projects/
    PRJ-0001/
      state.sqlite
      artifacts/
        tmp/
```

저장 위치의 의미는 아래와 같습니다.

- `registry.sqlite`는 런타임 홈 식별 정보, 프로젝트 등록 매핑, Agent Integration Profile 기록, 통합 프로젝트 멤버십, Host Installation 인벤토리, 레지스트리 메타데이터를 저장합니다. 프로젝트 등록에는 등록된 `repo_root`, `project_home`, 프로젝트 `state.sqlite` 경로, 상태가 포함됩니다.
- `projects/{project_id}/`는 등록된 프로젝트 하나의 하네스 프로젝트 홈입니다. `repo_root`와 같은 위치나 권한이 아닙니다.
- `state.sqlite`는 등록된 프로젝트의 프로젝트별 로컬 Core 상태를 저장합니다.
- `artifacts/`는 아티팩트 저장소를 사용할 때의 프로젝트 아티팩트 저장소입니다. `artifacts/tmp/`는 아티팩트 스테이징에 필요할 때 쓰는 임시 스테이징 공간이며 증거 권한이 아닙니다. 이 디렉터리들은 프로젝트 등록 직후에 반드시 존재할 필요가 없습니다.

프로젝트 실행에서는 `project_home`이 프로젝트별 로컬 런타임 상태 위치를 담당합니다. 실행 가능한 프로젝트 상태 데이터베이스 경로는 검증된 프로젝트 홈에서 `project_home/state.sqlite`로 파생합니다. 저장된 `state_db_path`는 영속성과 진단을 위해 `registry.sqlite`에 남지만, Store 실행이 프로젝트별 상태를 열거나, 검사하거나, 마이그레이션하거나, 사용하기 전에 이 파생 경로와 일치해야 합니다. 일치하지 않는 등록은 진단을 위한 레지스트리 수준 조회와 목록에서는 계속 읽을 수 있지만 Core 실행, 접점 관리, setup 재사용, MCP 프로젝트 시작에는 적격하지 않습니다.

기준 SQLite 테이블 형태, 인덱스, 외래 키, 마이그레이션 테이블, 제약은 [저장소 DDL](storage-ddl.md)이 담당합니다.

런타임 홈 식별은 파일시스템 경로에만 의존하면 안 됩니다.

복사되거나 이동된 런타임 홈은 같은 저장된 `runtime_home_id`를 가질 수 있습니다. 새 런타임 홈은 새 식별자를 가져야 합니다.

이 식별자는 의심스러운 복사본, 중복 등록, 경로 변경을 감지하는 데 도움이 될 수 있습니다. 보안 보장은 아닙니다.

## API 스키마와 저장소 기록

API 스키마 형태와 저장소 기록 배치는 서로 다른 담당 문서가 맡습니다.

- API 스키마 담당 문서는 요청/응답 데이터 형태와 응답 분기를 정의합니다.
- 공개 API 값은 [API 값 집합](api/schema-value-sets.md)이 담당합니다.
- 공개 `ErrorCode` 식별자와 의미는 [API 오류 코드](api/error-codes.md)가 담당합니다.
- 이 문서는 기준 범위 저장소 계약이 영속하는 항목을 정의합니다.
- 포함되는 항목은 기록 계열, 위치, 저장 범주, 관계 배치, 저장소 소유 값, 저장소 소유 JSON `TEXT`입니다.
- 비슷한 이름이 같은 권한을 만들지는 않습니다.
- `ArtifactRef`는 API 형태입니다. `artifacts`와 `artifact_links`는 저장소 기록입니다.
- `CloseReadinessBlocker` 형태는 [API 상태 스키마](api/schema-state.md)가 담당합니다. `blockers`는 저장소 기록 계열입니다.
- 응답 형태만으로 영속 여부가 증명되지 않습니다.
- 선택된 메서드 분기와 [저장 효과](storage-effects.md)가 호출이 기록을 만들거나, 바꾸거나, 관찰하거나, 건드리지 않는지를 정의합니다.
- 렌더링된 상태 카드, 판단 프롬프트, 실행/증거 요약, 닫기 준비 상태 출력, 에이전트 맥락 패킷은 읽는 시점에 만들어지는 보기입니다.
- 이 보기들은 기록 위에서 만들어집니다.
- 템플릿 문구는 [템플릿 본문](template-bodies.md)이 담당합니다.
- 상태 보기 권한은 [상태 보기 권한 참조](projection-and-templates.md)가 담당합니다.

## 영속 기록 계열과 저장 범주

기준 범위 저장소는 이 기준 범위 저장소 계약이 정의한 기록 계열만 영속합니다.

다른 영속 기록 계열은 [범위](scope.md)와 영향받는 저장소 담당 문서가 지원을 정의해야 합니다.

| 저장 영역 | 기록 계열 | 저장 범주 | 배치 요약 |
|---|---|---|---|
| `registry.sqlite` | 런타임 홈 식별 정보 | 런타임 식별 | 저장된 `runtime_home_id` 하나, 스키마/저장 프로필, 로컬 레지스트리 메타데이터. |
| `registry.sqlite` | 프로젝트 등록 | 프로젝트 매핑 | 등록된 프로젝트 식별자를 `repo_root`, 위치를 담당하는 `project_home`, 실행 시 `project_home/state.sqlite`와 일치해야 하는 저장된 `state_db_path`에 연결합니다. |
| `registry.sqlite` | Agent Integration Profile | 코딩 에이전트 통합 바인딩 | 지속되는 통합 식별자, 상호작용 역할, 묶인 접점 식별자, 활성 상태, 선택적 기본 프로젝트, 통합 메타데이터. |
| `registry.sqlite` | 통합 프로젝트 멤버십 | 통합 프로젝트 허용 목록 | Agent Integration Profile과 등록된 프로젝트 사이의 명시적 다대다 멤버십. |
| `registry.sqlite` | Host Installation | 호스트 설정 인벤토리 | 설정되었거나 내보낸 코딩 에이전트 호스트 항목의 호스트 종류, 호스트 범위, 서버 이름, 설정 대상, 관리 fingerprint, 마지막 검증 상태, 설치 메타데이터. |
| `state.sqlite` | `project_state` | 프로젝트 상태 헤더 | 저장 프로필, `state_version`, 현재 적용 `Task` 포인터, 기본 접점 포인터, 프로젝트 강제 프로필. |
| `state.sqlite` | `surfaces` | 접점 사실 | API 요청 래퍼 호환성, 행위자 출처 역할, 기능 표시, 로컬 접근 상태에 필요한 등록된 로컬 접점 사실. |
| `state.sqlite` | `tasks` | 작업 단위 상태 | 사용자 가치 작업 단위, 구체화 요약, 범위와 닫기 근거 리비전, nullable 현재 닫기 근거, 생명주기/결과/종료 닫기 요약, 현재 적용 `CompletionPolicy`, 현재 적용 Change Unit 포인터. |
| `state.sqlite` | `change_units` | 범위 있는 작업 경계 | 범위 요약, 쓰기 근거, 호환성 닫기 근거 저장소, Change Unit 생명주기, 소유 `Task` 관계. |
| `state.sqlite` | `user_judgments` | 사용자 소유 판단 상태 | 근거 스냅샷, 근거 상태, 선택된 선택지, 기계 동작, 해결 결과, 해결 행위자, 있을 때 확인된 행위자 출처, 필요한 경우 민감 동작 승인 범위를 포함하는 대기, 해결됨, 오래됨, 대체됨, 만료됨, 레거시 미결속 사용자 소유 판단. |
| `state.sqlite` | `write_authorizations` | 협력형 쓰기 권한 | 단일 사용 `Write Authorization`, 기준 버전, 시도 범위, 만료, 소비 상태. |
| `state.sqlite` | `runs` | 실행 또는 관찰 기록 | 커밋된 실행 또는 관찰 기록, 호환되는 승인 소비, 간결한 증거 갱신. |
| `state.sqlite`와 `artifacts/tmp/` | `artifact_staging` | 임시 아티팩트 스테이징 | 스테이징된 핸들 메타데이터, 안전한 스테이징 사실, 임시 바이트 또는 알림. |
| `state.sqlite`와 아티팩트 저장소 | `artifacts` | 영속 아티팩트 기록 | 영속 아티팩트 메타데이터 또는 본문 위치, 콘텐츠 타입, SHA-256, 크기, 무결성 상태, 가림 처리, 보존, 생산자, 가용성 사실. |
| `state.sqlite` | `artifact_links` | 아티팩트 소유 관계 | 아티팩트와 기준 범위 Core/API 기록 계열 사이의 소유 관계. |
| `state.sqlite` | `evidence_summaries` | 증거 요약 | 간결한 증거 범위, 뒷받침 참조, 공백 참조. |
| `state.sqlite` | `blockers` | 차단 사유 상태 | 다음 행동, 쓰기 호환성, 증거 공백, 닫기 준비 상태, 복구를 위한 구조화된 차단 사유 상태. |
| `state.sqlite` | `task_events` | 이벤트 흐름 | 커밋된 Core 변경의 추가 전용 순서와 감사 흐름. |
| `state.sqlite` | `tool_invocations` | 재실행 행 | [저장 효과](storage-effects.md)가 재실행 생성을 정의한 경우의 커밋된 `dry_run=false` Core 메서드 결과 재실행 행. |

## 기록 배치 규칙

### 식별자와 소유 관계

기준 범위 기록은 불투명하고 안정적인 식별자를 기본 키 또는 동등한 고유 키로 사용합니다. 고유성은 담당 기록 계열의 소유 범위 안에서 적용됩니다.

- 런타임 홈 식별 정보는 그 런타임 홈의 `runtime_home_id` 하나를 저장합니다.
- 프로젝트 등록에는 고유한 프로젝트 식별자와 고유한 프로젝트 홈이 필요합니다.
- Agent Integration Profile 식별 정보는 `integration_id`별로 고유하며 어댑터 호출에 사용할 등록된 코딩 에이전트 접점 식별자 하나를 묶습니다.
- 통합 프로젝트 멤버십은 `(integration_id, project_id)`별로 고유합니다. 프로필 기본 프로젝트가 있으면 그 프로젝트도 멤버십 집합에 있어야 합니다.
- Host Installation 인벤토리는 관리되는 호스트 대상, 호스트 범위, 서버 이름 조합에 대해 고유합니다. 관리되는 설정 상태를 기록하지만 외부 호스트 설정의 원천은 아닙니다.
- 프로젝트 범위 행은 등록된 프로젝트에 속합니다.
- `Task` 범위 행은 자신을 소유한 `tasks` 행과 같은 프로젝트와 같은 `Task`에 속합니다.
- 현재 적용 포인터, 기본 접점 포인터, 소유 참조는 같은 프로젝트의 기록을 가리켜야 합니다.
- `Task` 하나에는 현재 적용 Change Unit이 최대 하나만 있습니다.
- 단일 사용 관계는 여러 커밋 의미로 갈라지면 안 됩니다.
- 단일 사용 관계에는 소비된 `Write Authorization` 행, 소비된 스테이징 핸들, 승격된 스테이징 아티팩트, 아티팩트 소유 연결, 재실행 키가 포함됩니다.

### 현재 행, 이벤트 행, 재실행 행

현재 기록 계열은 일반 읽기에 쓰는 현재 Core 상태를 담습니다.

`task_events`는 커밋된 Core 변경의 추가 전용 순서와 감사 흐름입니다.

`tool_invocations`는 [저장 효과](storage-effects.md)가 재실행 생성을 정의한 경우에만 커밋된 재실행 행을 저장합니다.

상태 버전 동작, 멱등성, 이벤트 의미, 재실행 충돌 처리, 잠금, 마이그레이션 계약은 [저장소 버전 관리](storage-versioning.md)가 담당합니다.

### 관계 검증

저장소는 커밋 전에 저장 관계를 검증해야 합니다. 검증에는 아래 항목이 포함됩니다.

- 같은 프로젝트와 같은 `Task` 소유 관계
- 현재 적용 포인터 대상
- 호환되는 `Write Authorization` 소비
- 아티팩트 스테이징 소비와 승격 대상
- 아티팩트 소유 관계
- Agent Integration Profile의 기본 프로젝트 멤버십과 활성 상태 일관성
- 기존 Agent Integration Profile을 참조하는 Host Installation
- SQLite가 직접 외래 키로 표현할 수 없는 JSON 참조 배열

### 권한 행 보존

일반적인 기준 범위 Core 동작은 생명주기 또는 상태 전환을 통해 권한 행을 보존합니다.

`Task`를 완료, 취소, 대체하면 관련 생명주기/상태 의미가 바뀝니다. 그래도 커밋된 권한 행은 감사와 복구를 위해 계속 주소 지정 가능해야 합니다.

이 보존 규칙은 아래 기록 계열에 적용됩니다.

- `tasks`
- `change_units`
- `user_judgments`
- `write_authorizations`
- `runs`
- `artifacts`
- `artifact_links`
- `evidence_summaries`
- `blockers`
- `task_events`
- `tool_invocations`

아티팩트별 임시/영속 보존 규칙은 [아티팩트 저장소](storage-artifacts.md)가 담당합니다.

### 현재 닫기 근거와 레거시 상태

현재 닫기 근거는 `tasks` 계열에 저장되는 Task 소유 현재 상태입니다. 성공한 종료 닫기 결과를 위해 저장되는 종료 닫기 요약과 다릅니다.

권위 있는 현재 `CurrentCloseBasis` 기록은 Task 소유 닫기 근거 좌표와 함께 해석하는 `tasks.close_basis_json`입니다. `change_units.close_basis_json`은 물리 호환성 저장소로 유지되며 현재 닫기 근거 권한으로 해석하면 안 됩니다.

기존 열린 Task는 종료 닫기 요약 JSON이나 레거시 요약 JSON을 현재 닫기 근거로 자동 변환하지 않습니다. 현재 닫기 근거가 없다는 사실은 빈 생성 근거가 아니라 `tasks.close_basis_json`의 부재로 표현합니다. `change_units.close_basis_json`의 존재, 부재, 내용은 현재 `CurrentCloseBasis` 권한을 만족하지 않습니다.

저장된 근거가 없거나, 기계 판독 가능한 해결 결과가 없거나, 필요한 확인된 행위자 출처가 없는 판단은 감사를 위해 보존됩니다. 이 판단은 주소 지정 가능한 이력 판단 기록으로 남지만 현재 닫기, 쓰기, 범위 결정, 민감 승인, 취소, 최종 수락, 잔여 위험 수락 요구사항을 만족할 수 없습니다.

저장된 판단 권한에서 `user_judgments.status='resolved'`는 답변이 있다는 사실을 기록합니다. 사용자가 승인했다는 뜻이 아닙니다. 현재 권한을 지니는 판단 사용에는 선택된 선택지, 저장된 `resolution_machine_action`, 저장된 `resolution_outcome`, 적용 가능한 행위자 출처, 적용 가능한 확인된 해결 접점 출처가 필요합니다. 레거시 행은 감사 전용 읽기를 위해 빠진 권한 사실을 보존할 수 있습니다. 결과, 기계 동작, 적용 가능한 행위자 출처, 확인된 해결 접점 출처의 부재는 절대 수락이 아닙니다.

## 저장소 소유 값

닫힌 저장소 소유 값 집합은 영속 제약입니다. 알 수 없는 값은 커밋할 수 없습니다.

| 저장 필드 | 기준 범위 값 |
|---|---|
| 프로젝트 등록 `status` | `active` |
| Agent Integration Profile `interaction_role` | `agent` |
| Agent Integration Profile `enabled` | `0`, `1` |
| Host Installation `host_kind` | `codex`, `claude_code`, `generic` |
| Host Installation `host_scope` | `user`, `project`, `local`, `export` |
| Host Installation `last_verified_status` | `not_verified`, `complete`, `action_required`, `partial_failure`, `failed` |
| `change_units.status` | `proposed`, `active`, `replaced`, `closed` |
| `write_authorizations.status` | `active`, `consumed`, `expired`, `stale`, `revoked` |
| `user_judgments.basis_status` | `current`, `stale`, `superseded`, `legacy_unbound` |
| `user_judgments.resolution_machine_action` | 값이 있을 때 `accept`, `reject`, `defer` |
| `user_judgments.resolution_outcome` | 값이 있을 때 `accepted`, `rejected`, `deferred`, `blocked` |
| `artifact_staging.status` | `staged`, `consumed`, `expired`, `discarded` |
| `artifacts.status` | `available`, `missing`, `integrity_failed`, `unavailable` |
| `artifacts.integrity_status` | `verified`, `legacy_unknown`, `corrupt` |
| `artifact_links.owner_record_kind` | `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` |
| `blockers.status` | `active`, `resolved`, `superseded` |
| `tool_invocations.status` | `committed` |
| `tool_invocations.replay_context_status` | `verified`, `legacy_unverified` |

공개 API 값을 반영하는 행은 [API 값 집합](api/schema-value-sets.md), 관련 스키마 담당 문서, 메서드 담당 문서와 정확히 맞아야 합니다.

이 문서는 아래 필드의 공개 API 값을 다시 정의하지 않습니다.

- `tasks.mode`
- `tasks.lifecycle_phase`
- `tasks.result`
- `runs.kind`
- `runs.status`
- `user_judgments.status`
- `evidence_summaries.status`

공개 API 값은 [API 값 집합](api/schema-value-sets.md), [API 상태 스키마](api/schema-state.md), 메서드 담당 문서를 봅니다.

## 저장소 소유 JSON

JSON을 저장하는 SQLite `TEXT` 열은 저장 표현 선택일 뿐이며 임의 JSON을 저장해도 된다는 뜻이 아닙니다.

규칙:

- Core는 커밋 전에 JSON을 파싱하고 검증해야 합니다.
- API 형태의 저장 JSON은 API 스키마 담당 문서를 기준으로 검증합니다.
- 저장소 전용 JSON은 이 저장소 계약이나 참조된 저장소 담당 문서를 기준으로 검증합니다.
- `'{}'`, `'[]'` 같은 SQLite 기본값은 저장 기본값일 뿐이며 API 필드를 선택 필드로 만들지 않습니다.

| 기록 계열 | JSON `TEXT` 범주 |
|---|---|
| Agent Integration Profile | 권한, 접근 허용, 프로젝트 선택, 호스트 신뢰 증명에 쓰지 않는 통합 메타데이터. |
| Host Installation | 권한, 호스트 신뢰 증명, 외부 호스트 설정의 대체물로 쓰지 않는 설치 메타데이터. |
| `surfaces` | 접점 기능 프로필 데이터. |
| `tasks` | 구체화 요약, 제한된 목록, 자율성 경계, 현재 닫기 근거, 종료 닫기 요약, 생명주기 요약, `CompletionPolicy`. |
| `change_units` | 범위 요약, 제한된 목록, 쓰기 근거 요약, 호환성 닫기 근거 저장소, 생명주기 지원 데이터. |
| `user_judgments` | 판단 요청, 맥락, 선택지, 영향 참조, 아티팩트 참조, 근거 스냅샷, 민감 동작 승인 범위, 선택된 선택지, 기계 동작, 해결 결과, 행위자 출처, 해결 데이터. |
| `write_authorizations` | `Write Authorization` 시도 범위. |
| `runs` | 관찰과 증거 갱신 데이터. |
| `evidence_summaries` | 증거 범위와 공백 참조. |
| `blockers` | 차단 사유 소유 참조와 관련 참조. |
| `task_events` | 커밋된 Core 변경의 이벤트 페이로드. |
| `tool_invocations` | 커밋된 재실행 응답. |

`Task`와 Change Unit 구체화 JSON은 간결한 요약과 제한된 목록만 저장합니다. 추가 영속 기록 계열을 만들지 않습니다.

## 관련 담당 문서

- [저장 효과](storage-effects.md): 어떤 메서드 분기가 기록을 만들거나, 바꾸거나, 관찰하거나, 건드리지 않는지 정의합니다.
- [저장소 DDL](storage-ddl.md): 기준 SQLite 테이블 형태, 인덱스, 외래 키, 마이그레이션 테이블, 제약을 정의합니다.
- [아티팩트 저장소](storage-artifacts.md): 아티팩트 스테이징, 승격, 연결, 본문 읽기, 보존, 무결성 생명주기를 정의합니다.
- [저장소 버전 관리](storage-versioning.md): 상태 버전, 멱등성, 재실행, 이벤트, 잠금, 마이그레이션 계약을 정의합니다.
- [API 코어 스키마](api/schema-core.md), [API 상태 스키마](api/schema-state.md), [API 아티팩트 스키마](api/schema-artifacts.md), [API 판단 스키마](api/schema-judgment.md), [API 값 집합](api/schema-value-sets.md): API 형태와 공개 API 값을 정의합니다.
- [API 메서드](api/methods.md)와 메서드 담당 문서: 기록을 사용하는 공개 메서드 동작을 정의합니다.
- [런타임 경계](runtime-boundaries.md): `Product Repository`, Harness 설치 또는 런타임 프로세스, `Harness Runtime Home` 위치 경계를 정의합니다.
- [상태 보기 권한 참조](projection-and-templates.md)와 [템플릿 본문](template-bodies.md): 읽는 시점의 상태 보기 권한과 렌더링된 템플릿 본문을 정의합니다.
- [보안](security.md): 보안 경계와 보장 수준을 정의합니다.
