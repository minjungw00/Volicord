# 저장소 기록

이 문서는 기준 범위의 영속 저장소 기록 계열, 배치, 관계 배치, 저장소 소유 값, 저장소 소유 JSON 배치를 담당합니다. 영속 기록은 나중에 `Volicord Runtime Home` 안에서 다시 읽을 수 있도록 커밋한 로컬 기록입니다.

영속 기록은 Volicord 기록에 대한 로컬 Core 저장소 권한입니다. 보안 보장, 외부 감사 보장, 위조 방지 주장, `Product Repository` 파일 쓰기 권한은 각 담당 문서에 남습니다.

## 담당 경계

이 문서가 담당합니다.

- 기준 범위 영속 기록 계열
- 그 기록 계열의 테이블, 파일, 아티팩트 저장소 위치
- 저장 범주와 관계 배치
- 저장소 소유 값 집합
- 저장소 소유 SQLite JSON `TEXT` 배치
- 커밋 전 기록 배치 검증 요구사항

이 문서는 담당하지 않습니다.

- 기준 SQLite DDL, 인덱스, 외래 키, 마이그레이션 테이블, 제약: [저장소 DDL](storage-ddl.md)
- 메서드 분기별 영속 효과: [저장 효과](storage-effects.md)
- 아티팩트 스테이징, 승격, 연결, 본문 읽기, 보존, 무결성 생명주기: [아티팩트 저장소](storage-artifacts.md)
- `project_state.state_version`, 멱등성, 재실행, 이벤트, 잠금, 마이그레이션 계약: [저장소 버전 관리](storage-versioning.md)
- API 요청 또는 응답 형태: [API 코어 스키마](api/schema-core.md), [API 상태 스키마](api/schema-state.md), [API 아티팩트 스키마](api/schema-artifacts.md), [API 판단 스키마](api/schema-judgment.md), [API 값 집합](api/schema-value-sets.md)
- API 메서드 동작: [API 메서드](api/methods.md)와 메서드 담당 문서
- 런타임 위치와 저장소 경계: [런타임 경계](runtime-boundaries.md)
- 보안 보장 수준과 보안 경계: [보안](security.md)

## 저장 위치

Volicord는 기준 범위 기록을 로컬 `Volicord Runtime Home` 하나와 등록된 프로젝트별 로컬 상태 데이터베이스 하나에 저장합니다. `volicord init`은 첫 실행 저장소 설정 중 선택된 Runtime Home과 설치 프로필을 마련하거나 재사용할 수 있고, `volicord setup`은 그 프로필을 직접 준비하거나 복구합니다. 일반 사용자 흐름은 Runtime Home 경로를 다시 제공할 필요가 없습니다.

아래 트리는 관련 저장 기능을 사용한 뒤의 대표 배치입니다. 프로젝트 등록 직후의 초기 디렉터리 체크리스트가 아닙니다. 프로젝트 등록은 프로젝트 상태를 만들거나 열지만, 아티팩트 저장소 디렉터리는 필요할 때 늦게 만들어질 수 있습니다.

```text
~/.volicord/
  registry.sqlite
  projects/
    prj_<internal>/
      state.sqlite
      artifacts/        # 아티팩트 저장소를 사용할 때 생성
        tmp/            # 아티팩트 스테이징이 일어날 때 생성
```

저장 위치:

- `registry.sqlite`는 Runtime Home 식별 정보, 설치 프로필 기록, 프로젝트 등록 매핑, 프로젝트 alias, Agent Connection 기록, Connection Projects 멤버십, guard 설치 기록, registry 메타데이터를 저장합니다. 설치 프로필에는 선택된 `volicord` 명령, MCP 시작 명령, bin 디렉터리, 기본 연결 모드, 메타데이터, 타임스탬프가 포함됩니다. 프로젝트 등록에는 `project_internal_id`, 표시 이름, CLI 선택 alias, Runtime Home 관계, 등록된 `repo_root`, `project_home`, 프로젝트 `state.sqlite` 경로, 상태, 메타데이터, 타임스탬프가 포함됩니다.
- `projects/{project_internal_id}/`는 등록된 프로젝트 하나에 대한 기본 Volicord 프로젝트 홈 형태입니다. `repo_root`와 같은 위치나 권한이 아닙니다.
- `state.sqlite`는 등록된 프로젝트의 프로젝트별 로컬 Core 상태와 프로젝트 범위 guarded-operation 기록을 저장합니다.
- `artifacts/`는 아티팩트 저장소를 사용할 때의 프로젝트 아티팩트 저장소이며, 아티팩트 저장소가 처음 필요할 때 늦게 만들어질 수 있습니다. `artifacts/tmp/`는 아티팩트 스테이징에 필요할 때 쓰는 임시 스테이징 공간이며 증거 권한이 아닙니다. 이 디렉터리도 스테이징이 일어날 때 늦게 만들어질 수 있습니다. 이 디렉터리들은 프로젝트 등록 직후에 반드시 존재할 필요가 없습니다.

아티팩트 경로 기준:

- `artifact_staging.tmp_path`는 `project_home` 기준 상대 경로로 저장합니다. 임시 스테이징 영역 아래의 스테이징 바이트 또는 알림은 `artifacts/tmp/<file>` 같은 형태를 사용합니다.
- `artifacts.body_path`는 보통 `project_home/artifacts`인 아티팩트 저장소 루트 기준 상대 경로로 저장합니다. 지속 본문은 `tmp/<file>` 같은 형태를 사용하며 `artifact_store_root.join(body_path)`로 해석합니다.

운영 프로젝트 기록에서 `project_home`은 프로젝트별 로컬 런타임 상태 위치를 담당합니다. 실행 가능한 프로젝트 상태 데이터베이스 경로는 검증된 프로젝트 홈에서 `project_home/state.sqlite`로 파생합니다. 저장된 `state_db_path`는 영속성과 진단을 위해 `registry.sqlite`에 남지만, Store가 정상 `ProjectRecord`를 반환하거나, 프로젝트별 상태를 열거나 마이그레이션하거나, Agent Connection 프로젝트 접근을 해석하거나, Core 실행에 들어가거나, MCP 프로젝트 가용성을 보고하기 전에 이 파생 경로와 일치해야 합니다. 일치하지 않는 등록은 진단을 위한 원시 registry 내용으로 검사할 수 있지만, 운영 조회와 목록 조회는 그 행을 생략하거나 정상 프로젝트로 반환하지 말고 거절해야 합니다. 검사는 대체 `state_db_path`를 열거나, 만들거나, 마이그레이션하거나, 복구하면 안 됩니다.

`Product Repository`는 `repo_root`로 등록되는 사용자 제품 파일 경계입니다. Volicord Runtime Home이 아니며, Core 권한 저장소가 아니고, 런타임 기록, 재실행 행, 판단, Write Check, guard 기록, Agent Connection registry 상태를 저장하는 위치도 아닙니다.

기준 SQLite 테이블 형태, 인덱스, 외래 키, 마이그레이션 테이블, 제약은 [저장소 DDL](storage-ddl.md)이 담당합니다. 이 기록들의 현재 기준 SQLite 저장소 프로필은 `baseline_sqlite_v3`이며, 프로필/버전 경계 동작은 [저장소 버전 관리](storage-versioning.md)가 담당합니다.

Runtime Home 식별은 파일시스템 경로에만 의존하면 안 됩니다. 복사되거나 이동된 Runtime Home은 같은 저장된 `runtime_home_id`를 가질 수 있고, 새 Runtime Home은 새 식별자를 가져야 합니다. 이 식별자는 의심스러운 복사본, 중복 등록, 경로 변경을 감지하는 데 도움이 될 수 있지만 보안 보장은 아닙니다.

## API 스키마와 저장소 기록

API 스키마 형태와 저장소 기록 배치는 서로 다른 담당 문서가 맡습니다.

- API 스키마 담당 문서는 요청/응답 데이터 형태와 응답 분기를 정의합니다. 공개 API 값은 [API 값 집합](api/schema-value-sets.md)이 담당하고, 공개 `ErrorCode` 식별자와 의미는 [API 오류 코드](api/error-codes.md)가 담당합니다.
- 이 문서는 기준 범위 저장소 계약이 영속하는 항목을 정의합니다. 포함되는 항목은 기록 계열, 위치, 저장 범주, 관계 배치, 저장소 소유 값, 저장소 소유 JSON `TEXT`입니다.
- 비슷한 이름이 같은 권한을 만들지는 않습니다. `ArtifactRef`는 API 형태입니다. `artifacts`와 `artifact_links`는 저장소 기록입니다. `CloseReadinessBlocker` 형태는 [API 상태 스키마](api/schema-state.md)가 담당합니다. `blockers`는 저장소 기록 계열입니다.
- 응답 형태만으로 영속 여부가 증명되지 않습니다. 선택된 메서드 분기와 [저장 효과](storage-effects.md)가 호출이 기록을 만들거나, 바꾸거나, 관찰하거나, 건드리지 않는지를 정의합니다.
- 렌더링된 상태 카드, 판단 프롬프트, 실행/증거 요약, 닫기 준비 상태 출력, 에이전트 맥락 패킷은 기록 위에서 읽는 시점에 만들어지는 보기입니다. 템플릿 문구는 [템플릿 본문](template-bodies.md)이 담당하고, 상태 보기 권한은 [상태 보기 권한 참조](projection-and-templates.md)가 담당합니다.

## 영속 기록 계열

기준 범위 저장소는 이 기준 범위 저장소 계약이 정의한 기록 계열만 영속합니다. 다른 영속 기록 계열은 [범위](scope.md)와 영향받는 저장소 담당 문서가 지원을 정의해야 합니다.

| 저장 영역 | 기록 계열 | 저장 범주 | 배치 요약 |
|---|---|---|---|
| `registry.sqlite` | Runtime Home 식별 정보 | 런타임 식별 | 저장된 `runtime_home_id` 하나, Runtime Home 경로, registry 데이터베이스 경로, 스키마/저장 프로필, 메타데이터, 타임스탬프. |
| `registry.sqlite` | 설치 프로필 | 실행 파일 프로필 | `volicord init` 또는 `volicord setup`이 마련한 선택된 `volicord` 명령, MCP 시작 명령, bin 디렉터리, 기본 연결 모드, 메타데이터, 타임스탬프. |
| `registry.sqlite` | 프로젝트 등록과 alias | 프로젝트 매핑 | `project_internal_id`, 표시 이름, CLI 선택 alias, Runtime Home 관계, 고유한 `repo_root`, 위치를 담당하는 `project_home`, 실행 시 `project_home/state.sqlite`와 일치해야 하는 저장된 `state_db_path`, 상태, 메타데이터, alias에서 내부 식별 정보로 가는 매핑. |
| `registry.sqlite` | Agent Connection | MCP 호스트 연결 단위 | 지속되는 `connection_internal_id`, 호스트 종류, 연결 의도, 호스트 범위, 선택적 `project_internal_id`, 내부 서버 이름, 설정 대상, 모드, 활성 상태, 관리 fingerprint, 검증 요약 상태, 검증 보고서 JSON, 사용자 동작 JSON, 메타데이터, 타임스탬프. |
| `registry.sqlite` | Connection Projects | 연결 프로젝트 허용 목록 | `connection_internal_id`와 `project_internal_id`를 사용하는 Agent Connection과 등록된 프로젝트 사이의 명시적 다대다 멤버십. |
| `registry.sqlite` | Guard installation | Guard 설정과 호스트 capability 기록 | Runtime Home, Agent Connection, 선택적 프로젝트 범위, 호스트 종류, guard 모드, 호스트 capability JSON, 설치 health, 타임스탬프, 메타데이터. |
| `state.sqlite` | `project_state` | 프로젝트 상태 헤더 | 저장 프로필, `state_version`, 현재 적용 `Task` 포인터, 프로젝트 강제 프로필. |
| `state.sqlite` | `agent_sessions` | Guarded Agent Session | Agent Connection 하나에 대한 프로젝트 범위 세션, 선택적 guard 설치, 호스트 종류, guard 모드, 시작/종료 타임스탬프, 메타데이터. |
| `state.sqlite` | `guard_events` | Guard decision 이벤트 | 연결 및 선택적 세션 또는 설치에 묶이는 프로젝트 범위 guard 이벤트입니다. decision, subject JSON, result JSON, 타임스탬프, 메타데이터를 포함합니다. |
| `state.sqlite` | `prompt_captures` | Prompt capture | 세션에 대한 프로젝트 범위 prompt capture입니다. 연결, capture kind, prompt hash, 선택적 prompt text, 타임스탬프, 메타데이터를 포함합니다. |
| `state.sqlite` | `expected_writes` | 예상 Product Repository 쓰기 | 허용된 guarded pre-tool 쓰기가 만드는 프로젝트 범위 expected-write 상관 기록입니다. 연결/세션 식별 정보, 선택적 호스트 invocation 식별 정보, 정확한 경로 정책, active task/Change Unit/Write Check 근거, 타임스탬프, 매칭된 post-tool 메타데이터를 포함합니다. |
| `state.sqlite` | `unrecorded_changes` | 기록되지 않은 Product Repository 변경 | Core run 또는 담당자가 정의한 다른 기록과 아직 연결되지 않은 관찰된 Product Repository 변경에 대한 프로젝트 범위 미해결 또는 해결 기록. |
| `state.sqlite` | `tasks` | 작업 단위 상태 | 사용자 가치 작업 단위, 구체화 요약, 범위와 닫기 근거 리비전, nullable 현재 닫기 근거, 생명주기/결과/종료 닫기 요약, 현재 적용 `CompletionPolicy`, 현재 적용 Change Unit 포인터, 생성자 행위자 출처. |
| `state.sqlite` | `change_units` | 범위 있는 작업 경계 | 범위 요약, 쓰기 근거, Change Unit 생명주기, 소유 `Task` 관계. |
| `state.sqlite` | `user_judgments` | 사용자 소유 판단 상태 | 근거 스냅샷, 요청 맥락, 선택지, 민감 동작 범위, 해결 기계 동작과 결과, 판단 이유 메타데이터, User Channel 행위자 출처, 검증 근거, 보장 수준을 포함하는 대기, 해결됨, 오래됨, 대체됨, 만료됨 사용자 소유 판단. |
| `state.sqlite` | `project_continuity_records` | 프로젝트 연속성 맥락 | 원천 `Task`가 닫힌 뒤에도 주소 지정할 수 있게 남는 프로젝트 수준 결정, 의무, 알려진 한계, 수락된 잔여 위험, 제약. |
| `state.sqlite` | `write_checks` | Core 상태 쓰기 호환성 | 단일 사용 Write Check, 기준 버전, 시도 범위, 만료, 행위자 출처, 선택적 원천 판단, 소비 상태. |
| `state.sqlite` | `runs` | 실행 또는 관찰 기록 | 커밋된 실행 또는 관찰 기록, 선택적 호환 Write Check 소비, 행위자 출처, 간결한 증거 갱신. |
| `state.sqlite`와 `artifacts/tmp/` | `artifact_staging` | 임시 아티팩트 스테이징 | 스테이징된 핸들 메타데이터, 생성자 행위자 출처, 안전한 스테이징 사실, 임시 바이트 또는 알림. |
| `state.sqlite`와 아티팩트 저장소 | `artifacts` | 영속 아티팩트 기록 | 영속 아티팩트 메타데이터 또는 본문 위치, 콘텐츠 타입, SHA-256, 크기, 무결성 상태, 가림 처리, 보존, 생산자, 가용성 사실. |
| `state.sqlite` | `artifact_links` | 아티팩트 소유 관계 | 아티팩트와 기준 범위 Core/API 기록 계열 사이의 소유 관계. |
| `state.sqlite` | `evidence_summaries` | 증거 요약 | 간결한 증거 범위, 뒷받침 참조, 공백 참조. |
| `state.sqlite` | `evidence_observations` | 증거 관찰 | 하나의 보고되었거나 관찰된 증거 주장에 대한 지속 출처 기록입니다. 출처 종류, 보장 수준, 관찰자 행위자 출처, 도구 메타데이터, 입력 참조, 출력 아티팩트 참조, 한계, 타임스탬프를 포함합니다. |
| `state.sqlite` | `blockers` | 차단 사유 상태 | 다음 행동, 쓰기 호환성, 증거 공백, 닫기 준비 상태, 복구를 위한 구조화된 차단 사유 상태. |
| `state.sqlite` | `task_events` | 이벤트 흐름 | 커밋된 Core 변경의 추가 전용 순서와 감사 흐름. |
| `state.sqlite` | `tool_invocations` | 재실행 행 | [저장 효과](storage-effects.md)가 재실행 생성을 정의한 경우의 커밋된 `dry_run=false` Core 메서드 결과 재실행 행. 행위자 출처와 작업 범주를 포함합니다. |

## 기록 배치 규칙

### 식별자와 소유 관계

기준 범위 기록은 불투명하고 안정적인 식별자를 기본 키 또는 동등한 고유 키로 사용합니다. 고유성은 담당 기록 계열의 소유 범위 안에서 적용됩니다.

- Runtime Home 식별 정보는 그 Runtime Home의 `runtime_home_id` 하나를 저장합니다.
- 프로젝트 등록에는 고유한 `project_internal_id`, 고유한 프로젝트 alias, 고유한 저장소 루트, 고유한 프로젝트 홈, 고유한 상태 데이터베이스 경로가 필요합니다. `project_name`은 표시 이름이고 `project_alias`는 CLI 선택 보조 값입니다.
- Agent Connection 식별 정보는 `connection_internal_id`별로 고유합니다.
- Connection Projects 멤버십은 `connection_internal_id`와 `project_internal_id`의 조합별로 고유하며, 하나의 연결이 등록된 프로젝트를 주소 지정할 수 있게 하는 유일한 registry 멤버십입니다.
- Guard installation 식별 정보는 `guard_installation_id`별로 고유합니다. 프로젝트 범위 guard 설치는 등록된 프로젝트와 그 프로젝트에 대한 Connection Projects 멤버십을 가진 Agent Connection을 이름 붙여야 합니다.
- 프로젝트 범위 행은 등록된 프로젝트에 속합니다.
- Guard 세션, guard 이벤트, prompt capture, expected write, unrecorded change는 프로젝트별 `state.sqlite` 하나에 속하며 그 기록을 관찰했거나 만든 Agent Connection을 이름 붙입니다.
- `Task` 범위 행은 자신을 소유한 `tasks` 행과 같은 프로젝트와 같은 `Task`에 속합니다.
- 현재 적용 포인터와 소유 참조는 같은 프로젝트의 기록을 가리켜야 합니다.
- `Task` 하나에는 현재 적용 Change Unit이 최대 하나만 있습니다.
- 소비된 Write Check 행, 소비된 스테이징 핸들, 승격된 스테이징 아티팩트, 아티팩트 소유 연결, 재실행 키 같은 단일 사용 관계는 여러 커밋 의미로 갈라지면 안 됩니다.

### 현재 행, 이벤트 행, 재실행 행

현재 기록 계열은 일반 읽기에 쓰는 현재 Core 상태를 담습니다. `task_events`는 커밋된 Core 변경의 추가 전용 순서와 감사 흐름입니다. `tool_invocations`는 [저장 효과](storage-effects.md)가 재실행 생성을 정의한 경우에만 커밋된 재실행 행을 저장합니다.

상태 버전 동작, 멱등성, 이벤트 의미, 재실행 충돌 처리, 잠금, 마이그레이션 계약은 [저장소 버전 관리](storage-versioning.md)가 담당합니다.

### 관계 검증

저장소는 커밋 전에 저장 관계를 검증해야 합니다. 검증에는 아래 항목이 포함됩니다.

- 같은 프로젝트와 같은 `Task` 소유 관계
- 현재 적용 포인터 대상
- 호환되는 Write Check 소비
- 아티팩트 스테이징 소비와 승격 대상
- 아티팩트 소유 관계
- Agent Connection 라우팅을 위한 Connection Projects 멤버십과 활성 상태 일관성
- guard 설치, Agent Session, guard 이벤트, prompt capture, expected write, unrecorded change의 프로젝트 및 연결 범위
- SQLite가 직접 외래 키로 표현할 수 없는 JSON 참조 배열

### 권한 행 보존

일반적인 기준 범위 Core 동작은 생명주기 또는 상태 전환을 통해 권한 행을 보존합니다. `Task`를 완료, 취소, 대체하면 관련 생명주기/상태 의미가 바뀝니다. 그래도 커밋된 권한 행은 감사와 복구를 위해 계속 주소 지정 가능해야 합니다.

이 보존 규칙은 `tasks`, `change_units`, `user_judgments`, `project_continuity_records`, `write_checks`, `runs`, `artifacts`, `artifact_links`, `evidence_summaries`, `evidence_observations`, `blockers`, `task_events`, `tool_invocations`, `agent_sessions`, `guard_events`, `prompt_captures`, `expected_writes`, `unrecorded_changes`에 적용됩니다. 아티팩트별 임시/영속 보존 규칙은 [아티팩트 저장소](storage-artifacts.md)가 담당합니다.

### Guarded Operation 기록

Guarded-operation 기록은 호스트 통합 상태에 대한 로컬 권한 사실을 보존합니다. 이 기록은 Core와 Store 코드가 작업을 정직하게 진행하거나 닫을 수 있는지 판단하는 데 도움이 될 수 있습니다. 그러나 OS 수준 sandboxing, 파일시스템 ACL, 외부 정책 집행, 위조 방지 증명, 쓰기 방지 증명이 아닙니다.

`guard_installations`는 Runtime Home, Agent Connection, 선택적 프로젝트 범위별 설정 생명주기 상태, 관찰된 hook 메타데이터, 호스트 capability를 기록합니다. `configured`와 `reload_required`는 파일 또는 메타데이터가 설치되었지만 일치하는 guard hook이 아직 관찰되지 않았다는 뜻입니다. `active`는 기록된 프로젝트, Agent Connection, 호스트 종류, guard 모드, policy hash와 일치하는 유효한 guard hook을 Volicord가 관찰했다는 뜻입니다. OS 수준 집행이나 sandboxing을 증명하지 않습니다. `agent_sessions`, `guard_events`, `prompt_captures`, `expected_writes`, `unrecorded_changes`는 프로젝트별 로컬 행이며 프로젝트 `state.sqlite` 데이터베이스 사이로 새면 안 됩니다. 대기 중인 `expected_writes` 행은 guarded pre-tool이 프로젝트, 연결, 세션, 시간, 경로, Task, Change Unit, Write Check 좌표로 제한된 구체적 예상 쓰기를 허용했다는 뜻입니다. 매칭된 행은 post-tool 관찰이 그 예상 쓰기와 상관되었다는 뜻이며 제품 정확성 증명이 아닙니다. 미해결 `unrecorded_changes` 행은 관찰된 Product Repository 변경이 아직 담당자가 정의한 조정을 필요로 한다는 뜻입니다. 그 행을 해결하면 로컬 resolution basis, 행위자 출처, capture basis, 해결 타임스탬프, 선택적 연결 사용자 판단을 기록하고 행은 보존됩니다.

### 현재 닫기 근거

현재 닫기 근거는 `tasks` 계열에 저장되는 Task 소유 현재 상태입니다. 성공한 종료 닫기 결과를 위해 저장되는 종료 닫기 요약과 다릅니다.

권위 있는 현재 `CurrentCloseBasis` 기록은 Task 소유 닫기 근거 좌표와 함께 해석하는 `tasks.close_basis_json`입니다.

기존 열린 Task는 종료 닫기 요약 JSON을 현재 닫기 근거로 자동 변환하지 않습니다. 현재 닫기 근거가 없다는 사실은 빈 생성 근거가 아니라 `tasks.close_basis_json`의 부재로 표현합니다. Change Unit 기록은 현재 `CurrentCloseBasis` 권한을 저장하거나 만족하지 않습니다.

저장된 판단에는 `JudgmentBasis`가 필요합니다. 해결된 저장 판단에는 완전한 기계 판독 가능 해결, 구조화된 설명용 판단 이유 메타데이터, 행위자 출처, 검증 근거, 보장 수준이 필요합니다. 이 사실이 빠진 행은 감사 호환 권한 기록이 아니라 유효하지 않은 소유자 상태입니다.

저장된 판단 권한에서 `user_judgments.status='resolved'`는 답변이 있다는 사실을 기록합니다. 사용자가 승인했다는 뜻이 아닙니다. 현재 권한을 지니는 판단 사용에는 선택된 선택지, 저장된 `resolution_machine_action`, 저장된 `resolution_outcome`, 적용 가능한 User Channel 행위자 출처, 메서드가 정의한 호환성이 필요합니다. 판단 이유 메타데이터는 답변의 이유와 맥락을 보존하지만 그 자체가 권한, 증거, 수락, 닫기 준비 상태, 잔여 위험 수락은 아닙니다. 결과, 기계 동작, 적용 가능한 행위자 출처, 검증 근거, 보장 수준의 부재는 유효하지 않은 소유자 상태이며 절대 수락이 아닙니다.

### 프로젝트 연속성 기록

`project_continuity_records`는 커밋된 Core 효과에서 비롯된 오래 유지할 프로젝트 수준 맥락을 보존합니다. 기준 기록은 결정, 의무, 알려진 한계, 수락된 잔여 위험, 제약을 나타낼 수 있습니다.

원천 `Task`와 선택적 원천 Change Unit은 연속성 기록이 어디에서 비롯되었는지를 식별합니다. 그 원천 경로를 다시 현재 상태로 만들지는 않습니다. `status='active'`는 기록을 살아 있는 프로젝트 맥락으로 보이게 하고, `superseded`와 `closed`는 감사와 복구를 위해 기록을 계속 주소 지정할 수 있게 둡니다.

프로젝트 연속성 기록은 새 작업의 현재 권한이 아닙니다. 이후 쓰기, Run, 판단 요구사항, 닫기 준비 상태 확인, 최종 수락, 잔여 위험 수락, 차단 사유 결정은 여전히 현재 담당자가 정의한 Core 상태와 호환성 규칙을 사용해야 합니다.

## 저장소 소유 값

닫힌 저장소 소유 값 집합은 영속 제약입니다. 알 수 없는 값은 커밋할 수 없습니다.

| 저장 필드 | 기준 범위 값 |
|---|---|
| 프로젝트 등록 `status` | `active` |
| Agent Connection `host_kind` | `codex`, `claude_code`, `generic` |
| Agent Connection `intent` | `personal`, `shared`, `global` |
| Agent Connection `host_scope` | `host_kind` 조합에 따른 `user`, `project`, `local`, `export` |
| Agent Connection `mode` | `workflow`, `read_only` |
| Agent Connection `enabled` | `0`, `1` |
| Agent Connection `last_verification_status` | `not_verified`, `complete`, `action_required`, `failed` |
| Guard installation `guard_mode` | `mcp_only`, `guarded`, `managed` |
| Guard installation `installation_status` | `absent`, `configured`, `reload_required`, `active`, `degraded`, `stale`, `broken` |
| `agent_sessions.guard_mode` | `mcp_only`, `guarded`, `managed` |
| `guard_events.decision` | `allow`, `deny`, `warn`, `inject_context` |
| `expected_writes.path_policy` | `exact_paths` |
| `expected_writes.status` | `pending`, `matched` |
| `unrecorded_changes.status` | `unresolved`, `resolved` |
| `change_units.status` | `proposed`, `active`, `replaced`, `closed` |
| `write_checks.status` | `active`, `consumed`, `expired`, `stale`, `revoked` |
| `user_judgments.status` | `pending`, `resolved`, `stale`, `superseded`, `expired` |
| `user_judgments.basis_status` | `current`, `stale`, `superseded` |
| `user_judgments.resolution_machine_action` | 완전한 해결 그룹의 `accept`, `reject`, `defer` |
| `user_judgments.resolution_outcome` | 완전한 해결 그룹의 `accepted`, `rejected`, `deferred` |
| `project_continuity_records.kind` | `decision`, `obligation`, `known_limit`, `accepted_risk`, `constraint` |
| `project_continuity_records.status` | `active`, `superseded`, `closed` |
| `artifact_staging.status` | `staged`, `consumed`, `expired`, `discarded` |
| `artifacts.status` | `available`, `missing`, `integrity_failed`, `unavailable` |
| `artifacts.integrity_status` | `verified`, `corrupt` |
| `artifact_links.owner_record_kind` | `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `evidence_observation`, `blocker` |
| `evidence_observations.source_kind` | `agent_report`, `connection_observation`, `external_tool`, `user_observation`, `reused_evidence`, `unverified_claim` |
| `evidence_observations.assurance_level` | `cooperative_report`, `registered_connection_observed`, `external_tool_result`, `user_observed`, `unverified` |
| `blockers.status` | `active`, `resolved`, `superseded` |
| `tool_invocations.status` | `committed` |
| `tool_invocations.operation_category` | `read`, `agent_workflow`, `user_only`, `admin_local` |

공개 API 값을 반영하는 행은 [API 값 집합](api/schema-value-sets.md), 관련 스키마 담당 문서, 메서드 담당 문서와 정확히 맞아야 합니다. 이 문서는 `tasks.mode`, `tasks.lifecycle_phase`, `tasks.result`, `runs.kind`, `runs.status`, `evidence_summaries.status` 같은 필드의 공개 API 값을 다시 정의하지 않습니다. 공개 API 값은 [API 값 집합](api/schema-value-sets.md), [API 상태 스키마](api/schema-state.md), 메서드 담당 문서를 봅니다.

## 저장소 소유 JSON

JSON을 저장하는 SQLite `TEXT` 열은 저장 표현 선택일 뿐이며 임의 JSON을 저장해도 된다는 뜻이 아닙니다.

규칙:

- Core는 커밋 전에 JSON을 파싱하고 검증해야 합니다.
- API 형태의 저장 JSON은 API 스키마 담당 문서를 기준으로 검증합니다.
- 저장소 전용 JSON은 이 저장소 계약이나 참조된 저장소 담당 문서를 기준으로 검증합니다.
- `'{}'`, `'[]'` 같은 SQLite 기본값은 저장 기본값일 뿐이며 API 필드를 선택 필드로 만들지 않습니다.

| 기록 계열 | JSON `TEXT` 범주 |
|---|---|
| 설치 프로필 | 호스트 신뢰 결정, 사용자 판단, 공개 API 스키마가 아닌 설치 프로필 메타데이터. |
| Agent Connection | 권한, 호스트 신뢰 증명, 외부 호스트 설정의 대체물로 쓰지 않는 검증 보고서 JSON, 사용자 동작 JSON, 메타데이터. |
| Guard installation | 로컬 guard 설정 health를 위한 호스트 capability JSON과 메타데이터입니다. OS 집행 증명이 아닙니다. |
| `agent_sessions` | 프로젝트 범위 Agent Session에 대한 비권한 메타데이터. |
| `guard_events` | 로컬 guard decision 이벤트의 guard subject JSON, result JSON, 메타데이터. |
| `prompt_captures` | 캡처된 prompt 기록의 비권한 메타데이터. Prompt text는 직접 nullable text 열입니다. |
| `unrecorded_changes` | 기록되지 않은 Product Repository 변경의 관찰 경로 배열, detection JSON, resolution JSON, 메타데이터. Resolution JSON은 간결한 resolution basis, capture basis, 해결 메서드, 선택적 연결 사용자 판단 참조를 저장하며, 전체 민감 명령이나 prompt 내용을 저장하면 안 됩니다. |
| `tasks` | 구체화 요약, 제한된 목록, 자율성 경계, 현재 닫기 근거, 종료 닫기 요약, 생명주기 요약, `CompletionPolicy`. |
| `change_units` | 범위 요약, 제한된 목록, 쓰기 근거 요약, 선택적 효과 계약 데이터, 생명주기 지원 데이터. |
| `user_judgments` | 판단 요청, 맥락, 선택지, 영향 참조, 아티팩트 참조, 근거 스냅샷, 민감 동작 범위, 기계 판독 가능 해결, 설명용 판단 이유 메타데이터. |
| `project_continuity_records` | 오래 유지하는 프로젝트 맥락을 위한 적용 대상 경로, 적용 대상 참조, 원천 참조, 아티팩트 참조, 대체된 참조, 검토 트리거, 비권한 메타데이터. |
| `write_checks` | Write Check 시도 범위와 비권한 메타데이터. |
| `runs` | 요약, 관찰된 변경, 증거 갱신, Write Check 효과 데이터, 비권한 메타데이터. |
| `artifact_staging` | 스테이징된 아티팩트 데이터, 안전 메타데이터, 비권한 메타데이터. |
| `artifacts` | 보존, 생산자, 비권한 메타데이터. |
| `artifact_links` | 비권한 메타데이터. |
| `evidence_summaries` | 증거 범위, 뒷받침 참조, 공백 참조, 비권한 메타데이터. |
| `evidence_observations` | 하나의 증거 관찰에 대한 도구 메타데이터, 입력 참조, 출력 아티팩트 참조, 한계, 비권한 메타데이터. |
| `blockers` | 차단 사유 소유 참조, 관련 참조, 세부 정보, 비권한 메타데이터. |
| `task_events` | 커밋된 Core 변경의 이벤트 페이로드. |
| `tool_invocations` | 커밋된 재실행 응답. |

`Task`와 Change Unit 구체화 JSON은 간결한 요약과 제한된 목록만 저장합니다. 추가 영속 기록 계열을 만들지 않습니다.

## 관련 담당 문서

- [저장 효과](storage-effects.md): 어떤 메서드 분기가 기록을 만들거나, 바꾸거나, 관찰하거나, 건드리지 않는지 정의합니다.
- [저장소 DDL](storage-ddl.md): 기준 SQLite 테이블 형태, 인덱스, 외래 키, 마이그레이션 테이블, 제약을 정의합니다.
- [아티팩트 저장소](storage-artifacts.md): 아티팩트 스테이징, 승격, 연결, 본문 읽기, 보존, 무결성 생명주기를 정의합니다.
- [저장소 버전 관리](storage-versioning.md): 상태 버전, 멱등성, 재실행, 이벤트, 잠금, 마이그레이션 계약을 정의합니다.
- [Agent Connection](agent-connection.md): Agent Connection, Connection Projects, 모드로 제한되는 MCP 도구 접근, User Channel 경계를 정의합니다.
- [API 코어 스키마](api/schema-core.md), [API 상태 스키마](api/schema-state.md), [API 아티팩트 스키마](api/schema-artifacts.md), [API 판단 스키마](api/schema-judgment.md), [API 값 집합](api/schema-value-sets.md): API 형태와 공개 API 값을 정의합니다.
- [API 메서드](api/methods.md)와 메서드 담당 문서: 기록을 사용하는 공개 메서드 동작을 정의합니다.
- [런타임 경계](runtime-boundaries.md): `Product Repository`, Volicord 설치 또는 런타임 프로세스, `Volicord Runtime Home` 위치 경계를 정의합니다.
- [상태 보기 권한 참조](projection-and-templates.md)와 [템플릿 본문](template-bodies.md): 읽는 시점의 상태 보기 권한과 렌더링된 템플릿 본문을 정의합니다.
- [보안](security.md): 보안 경계와 보장 수준을 정의합니다.
