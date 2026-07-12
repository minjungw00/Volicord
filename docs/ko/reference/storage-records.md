# 저장소 기록

이 문서는 기준 범위의 영속 기록 계열과 위치, 관계 구조, 저장소 소유 값, 저장소 소유 JSON의 저장 위치를 담당합니다. 영속 기록은 나중에 `Volicord Runtime Home`에서 다시 읽을 수 있도록 커밋한 로컬 기록입니다.

영속 기록은 Volicord 기록에 대한 로컬 Core 저장소 권한입니다. 보안 보장, 외부 감사 보장, 위조 방지 주장, `Product Repository` 파일 쓰기 권한은 각 담당 문서에 남습니다.

## 담당 경계

이 문서가 담당합니다.

- 기준 범위 영속 기록 계열
- 그 기록 계열의 테이블, 파일, 아티팩트 저장소 위치
- 저장 범주와 관계 구조
- 저장소 소유 값 집합
- 저장소 소유 SQLite JSON `TEXT`의 저장 위치
- 커밋 전 기록 구조 검증 요구사항

이 문서는 담당하지 않습니다.

- 기준 SQLite DDL, 인덱스, 외래 키, 기준 SQL 원본, 제약: [저장소 DDL](storage-ddl.md)
- 메서드 분기별 영속 효과: [저장 효과](storage-effects.md)
- 아티팩트 스테이징, 승격, 연결, 본문 읽기, 보존, 무결성 생명주기: [아티팩트 저장소](storage-artifacts.md)
- `project_state.state_version`, 멱등성, 재실행, 이벤트, 잠금, 호환되지 않는 저장소 처리: [저장소 버전 관리](storage-versioning.md)
- API 요청 또는 응답 형태: [API 코어 스키마](api/schema-core.md), [API 상태 스키마](api/schema-state.md), [API 아티팩트 스키마](api/schema-artifacts.md), [API 판단 스키마](api/schema-judgment.md), [API 값 집합](api/schema-value-sets.md)
- API 메서드 동작: [API 메서드](api/methods.md)와 메서드 담당 문서
- 런타임 위치와 저장소 경계: [런타임 경계](runtime-boundaries.md)
- 보안 보장 수준과 보안 경계: [보안](security.md)

## 저장 위치

Volicord는 기준 범위 기록을 로컬 `Volicord Runtime Home` 하나와 등록된 프로젝트별 로컬 상태 데이터베이스 하나에 저장합니다. `volicord init`은 첫 실행 저장소 설정 중 선택된 Runtime Home과 설치 프로필을 마련하거나 재사용할 수 있습니다. 일반 사용자 흐름은 Runtime Home 경로를 다시 제공할 필요가 없습니다.

아래 트리는 관련 저장 기능을 사용한 뒤의 대표 배치입니다. 프로젝트 등록 직후의 초기 디렉터리 체크리스트가 아닙니다. 프로젝트 등록은 프로젝트 상태를 만들거나 열지만, 아티팩트 저장소 디렉터리는 필요할 때 늦게 만들어질 수 있습니다.

```text
~/.volicord/
  registry.sqlite
  diagnostics.sqlite   # 진단 세션을 관찰한 뒤 필요할 때 생성
  projects/
    prj_<internal>/
      state.sqlite
      artifacts/        # 아티팩트 저장소를 사용할 때 생성
        tmp/            # 아티팩트 스테이징이 일어날 때 생성
```

저장 위치:

- `registry.sqlite`는 Runtime Home 식별 정보, 설치 프로필, 프로젝트 등록 매핑과 별칭, Agent Connection, Connection Projects 멤버십, 호스트 훅 설치, 레지스트리 메타데이터를 저장합니다. 설치 프로필에는 선택된 `volicord` 명령, MCP 시작 명령, 실행 파일 디렉터리, 기본 연결 모드, 메타데이터, 타임스탬프가 포함됩니다. 프로젝트 등록에는 `project_internal_id`, 표시 이름, CLI 선택 별칭, Runtime Home 관계, 등록된 `repo_root`, `project_home`, 프로젝트 `state.sqlite` 경로, 상태, 메타데이터, 타임스탬프가 포함됩니다.
- `diagnostics.sqlite`는 필요할 때 생성되는 크기 제한 비권한 로컬 운영 진단 저장소입니다. `registry.sqlite` 및 모든 프로젝트 `state.sqlite`와 분리되며 어느 데이터베이스에도 외래 키를 두지 않습니다.
- `projects/{project_internal_id}/`는 등록된 프로젝트 하나에 대한 기본 Volicord 프로젝트 홈 형태입니다. `repo_root`와 같은 위치나 권한이 아닙니다.
- `state.sqlite`는 등록된 프로젝트의 로컬 Core 상태와 프로젝트 범위 호스트 관찰 기록을 저장합니다.
- `artifacts/`는 아티팩트 저장소를 사용할 때의 프로젝트 아티팩트 저장소이며, 아티팩트 저장소가 처음 필요할 때 늦게 만들어질 수 있습니다. `artifacts/tmp/`는 아티팩트 스테이징에 필요할 때 쓰는 임시 스테이징 공간이며 증거 권한이 아닙니다. 이 디렉터리도 스테이징이 일어날 때 늦게 만들어질 수 있습니다. 이 디렉터리들은 프로젝트 등록 직후에 반드시 존재할 필요가 없습니다.

아티팩트 경로 기준:

- `artifact_staging.tmp_path`는 `project_home` 기준 상대 경로로 저장합니다. 임시 스테이징 영역 아래의 스테이징 바이트 또는 알림은 `artifacts/tmp/<file>` 같은 형태를 사용합니다.
- `artifacts.body_path`는 보통 `project_home/artifacts`인 아티팩트 저장소 루트 기준 상대 경로로 저장합니다. 영속 본문은 `tmp/<file>` 같은 형태를 사용하며 `artifact_store_root.join(body_path)`로 해석합니다.

운영 프로젝트 기록에서 `project_home`은 프로젝트별 로컬 런타임 상태 위치를 담당합니다. 실행 가능한 프로젝트 상태 데이터베이스 경로는 검증된 프로젝트 홈에서 `project_home/state.sqlite`로 파생합니다. 저장된 `state_db_path`는 영속성과 진단을 위해 `registry.sqlite`에 남지만, Store가 정상 `ProjectRecord`를 반환하거나, 프로젝트별 상태를 열거나, Agent Connection 프로젝트 접근을 해석하거나, Core 실행에 들어가거나, MCP 프로젝트 가용성을 보고하기 전에 이 파생 경로와 일치해야 합니다. 일치하지 않는 등록은 진단을 위한 원시 레지스트리 내용으로 검사할 수 있지만, 운영 조회와 목록 조회는 그 행을 생략하거나 정상 프로젝트로 반환하지 말고 거절해야 합니다. 검사는 대체 `state_db_path`를 열거나, 만들거나, 초기화하거나, 복구하면 안 됩니다.

`Product Repository`는 `repo_root`로 등록되는 사용자 제품 파일 경계입니다. Volicord Runtime Home이 아니며, Core 권한 저장소가 아니고, 런타임 기록, 재실행 행, 판단, 쓰기 티켓, 호스트 훅 기록, Agent Connection 레지스트리 상태를 저장하는 위치도 아닙니다.

기준 SQLite 테이블 형태, 인덱스, 외래 키, 제약, 기준 SQL 원본은 [저장소 DDL](storage-ddl.md)이 담당합니다. 이 기록들의 현재 기준 SQLite 저장소 프로필은 `baseline_sqlite_v3`이며, 저장소 프로필과 호환되지 않는 저장소 경계 동작은 [저장소 버전 관리](storage-versioning.md)가 담당합니다.

Runtime Home 식별은 파일시스템 경로에만 의존하면 안 됩니다. 복사되거나 이동된 Runtime Home은 같은 저장된 `runtime_home_id`를 가질 수 있고, 새 Runtime Home은 새 식별자를 가져야 합니다. 이 식별자는 의심스러운 복사본, 중복 등록, 경로 변경을 감지하는 데 도움이 될 수 있지만 보안 보장은 아닙니다.

## API 스키마와 저장소 기록

API 스키마 형태와 저장소 기록 구조는 서로 다른 담당 문서가 맡습니다.

- API 스키마 담당 문서는 요청/응답 데이터 형태와 응답 분기를 정의합니다. 공개 API 값은 [API 값 집합](api/schema-value-sets.md)이 담당하고, 공개 `ErrorCode` 식별자와 의미는 [API 오류 코드](api/error-codes.md)가 담당합니다.
- 이 문서는 기준 범위 저장소 계약이 영속하는 항목을 정의합니다. 포함되는 항목은 기록 계열, 위치, 저장 범주, 관계 배치, 저장소 소유 값, 저장소 소유 JSON `TEXT`입니다.
- 비슷한 이름이 같은 권한을 만들지는 않습니다. `ArtifactRef`는 API 형태입니다. `artifacts`와 `artifact_links`는 저장소 기록입니다. `CloseReadinessBlocker` 형태는 [API 상태 스키마](api/schema-state.md)가 담당합니다. `blockers`는 저장소 기록 계열입니다.
- 응답 형태만으로 영속 여부가 증명되지 않습니다. 선택된 메서드 분기와 [저장 효과](storage-effects.md)가 호출이 기록을 만들거나, 바꾸거나, 관찰하거나, 건드리지 않는지를 정의합니다.
- 렌더링된 상태 카드, 판단 프롬프트, 실행/증거 요약, 닫기 준비 상태 출력, 에이전트 맥락 패킷은 기록 위에서 읽는 시점에 만들어지는 보기입니다. 템플릿 문구는 [템플릿 본문](template-bodies.md)이 담당하고, 상태 보기 권한은 [상태 보기 권한 참조](projection-and-templates.md)가 담당합니다.

## 영속 기록 계열

기준 범위 저장소는 이 기준 범위 저장소 계약이 정의한 기록 계열만 영속합니다. 다른 영속 기록 계열은 [범위](scope.md)와 영향받는 저장소 담당 문서가 지원을 정의해야 합니다.

| 저장 영역 | 기록 계열 | 저장 범주 | 배치 요약 |
|---|---|---|---|
| `diagnostics.sqlite` | `diagnostic_sessions` | 크기 제한 로컬 운영 세션 | 세션, 선택적 연결 및 프로젝트 식별 정보, 전송, 선택적 호스트 종류, 기록 생성 패키지/빌드 식별 정보, 시작/갱신 타임스탬프. |
| `diagnostics.sqlite` | `diagnostic_events` | 내용을 담지 않는 운영 관찰 | 세션 관계, 이벤트/도구 범주, 지연과 바이트 카운터, 검증/재시도/Core/재실행 플래그, 선택적 User Channel 또는 대체 경로 범주, 관찰된 제품 쓰기 수, 권한 상태 새로 고침 실패 플래그, 범주형 결과, 타임스탬프. |
| `registry.sqlite` | Runtime Home 식별 정보 | 런타임 식별 | 저장된 `runtime_home_id` 하나, Runtime Home 경로, 레지스트리 데이터베이스 경로, 스키마/저장 프로필, 메타데이터, 타임스탬프. |
| `registry.sqlite` | 설치 프로필 | 실행 파일 프로필 | `volicord init`이 마련한 선택된 `volicord` 명령, MCP 시작 명령, 실행 파일 디렉터리, 기본 연결 모드, 메타데이터, 타임스탬프. |
| `registry.sqlite` | 프로젝트 등록과 별칭 | 프로젝트 매핑 | `project_internal_id`, 표시 이름, CLI 선택 별칭, Runtime Home 관계, 고유한 `repo_root`, 위치를 담당하는 `project_home`, 실행 시 `project_home/state.sqlite`와 일치해야 하는 저장된 `state_db_path`, 상태, 메타데이터, 별칭에서 내부 식별 정보로 가는 매핑. |
| `registry.sqlite` | Agent Connection | MCP 호스트 연결 단위 | 영속 `connection_internal_id`, 호스트 종류, 연결 의도, 호스트 범위, 선택적 `project_internal_id`, 내부 서버 이름, 설정 대상, 모드, 활성 상태, 관리 지문, 검증 요약 상태, 검증 보고서 JSON, 사용자 동작 JSON, 메타데이터, 타임스탬프. |
| `registry.sqlite` | Connection Projects | 연결 프로젝트 허용 목록 | `connection_internal_id`와 `project_internal_id`를 사용하는 Agent Connection과 등록된 프로젝트 사이의 명시적 다대다 멤버십. |
| `registry.sqlite` | 호스트 훅 설치 | 호스트 훅 설정과 호스트 역량 기록 | Runtime Home, Agent Connection, 선택적 프로젝트 범위, 호스트 종류, 통합 모드, 호스트 역량 JSON, 설치 생명주기 상태, 관찰된 훅 메타데이터, 타임스탬프, 메타데이터. |
| `state.sqlite` | `project_state` | 프로젝트 상태 헤더 | 저장 프로필, `state_version`, 현재 적용 `Task` 포인터, 프로젝트 강제 프로필. |
| `state.sqlite` | `agent_sessions` | 관찰된 에이전트 세션 | Agent Connection 하나에 대한 프로젝트 범위 세션, 선택적 호스트 훅 설치, 호스트 종류, 통합 프로필, 시작/종료 타임스탬프, 메타데이터. |
| `state.sqlite` | `guard_events` | 호스트 훅 판단 이벤트 | 연결 및 선택적 세션 또는 설치에 묶이는 프로젝트 범위 호스트 훅 이벤트입니다. 판단 값, 대상 JSON, 결과 JSON, 타임스탬프, 메타데이터를 포함합니다. |
| `state.sqlite` | `prompt_captures` | 프롬프트 캡처 | 세션에 대한 프로젝트 범위 프롬프트 캡처입니다. 연결, 캡처 종류, 프롬프트 해시, 선택적 프롬프트 본문, 타임스탬프, 메타데이터를 포함합니다. |
| `state.sqlite` | `expected_writes` | 예상 Product Repository 쓰기 | 허용된 `detective` 도구 실행 전 쓰기가 만드는 프로젝트 범위 예상 쓰기 상관 기록입니다. 연결/세션 식별 정보, 선택적 호스트 호출 식별 정보, 정확한 경로 정책, 현재 적용 `Task`/Change Unit/쓰기 티켓 근거, 타임스탬프, 일치한 도구 실행 후 메타데이터를 포함합니다. |
| `state.sqlite` | `unrecorded_changes` | 미기록 Product Repository 변경 | Core 실행 또는 담당 문서가 정의한 다른 기록과 아직 연결되지 않은 관찰된 Product Repository 변경에 대한 프로젝트 범위 미해결 또는 해결 기록. |
| `state.sqlite` | `session_watch_baselines` | 세션 감시 기준선 | 등록된 Product Repository 또는 감시 경로 집합에 대한 프로젝트 범위 세션 감시 상태와 기준선 스냅샷입니다. 유효한 제외 항목, 스냅샷 다이제스트 메타데이터, 간결한 스냅샷 항목을 포함합니다. |
| `state.sqlite` | `session_watch_observations` | 세션 감시 관찰 | 이후의 안전한 스냅샷을 기준선과 비교해 얻은 프로젝트 범위 `detective` 관찰입니다. 관찰된 변경 경로, 선택적 예상 쓰기 또는 쓰기 티켓 상관 관계, 기존 미기록 변경 행에 대한 선택적 연결을 포함합니다. |
| `state.sqlite` | `tasks` | 작업 단위 상태 | 모드와 work phase, Task 소유 acceptance policy와 이유, 선택적 predecessor 관계와 carry-forward 감사, 구체화 요약, 범위와 닫기 근거 리비전, `null` 허용 현재 닫기 근거, 생명주기/결과/종료 요약, 현재 Change Unit 포인터, 생성자 actor source를 가진 사용자 가치 단위. |
| `state.sqlite` | `acceptance_criteria` | 수락 기준 | Core가 생성한 기준 identity, 소유 `Task`, 문장, 증거 요구 수준, 교체 순서, 활성/폐기 상태, 타임스탬프. |
| `state.sqlite` | `evidence_claims` | 보충 증거 주장 | 호출자가 부여한 `Task` 범위 주장 identity와 비어 있지 않은 불변 문장 하나. |
| `state.sqlite` | `change_units` | 범위 있는 작업 경계 | 범위 요약, 쓰기 근거, Change Unit 생명주기, 소유 `Task` 관계. |
| `state.sqlite` | `user_judgments` | 사용자 소유 판단 상태 | 근거 스냅샷, 요청 맥락, 선택지, 민감 동작 범위, 해결 기계 동작과 결과, 판단 이유 메타데이터, User Channel 행위자 출처, 검증 근거, 보장 수준을 포함하는 대기, 해결됨, 오래됨, 대체됨, 만료됨 사용자 소유 판단. |
| `state.sqlite` | 로컬 웹 동의 토큰 | User Channel 대체 입력 토큰 | 대기 사용자 판단을 위해 해시만 저장하는 일회성 토큰 메타데이터입니다. 프로젝트, 연결, 판단, `capture_basis`, 상태, 만료, 생성/완료 메타데이터로 범위가 정해집니다. |
| `state.sqlite` | `project_continuity_records` | 프로젝트 연속성 맥락 | 원천 `Task`가 닫힌 뒤에도 주소 지정할 수 있게 남는 프로젝트 수준 결정, 의무, 알려진 한계, 수락된 잔여 위험, 제약. |
| `state.sqlite` | `write_tickets` | 쓰기 티켓 권한 | 단일 사용 쓰기 티켓 권한 기록, 기준 버전, 시도 범위, 만료, 행위자 출처, 선택적 원천 판단, 소비 상태를 저장하는 물리 테이블입니다. |
| `state.sqlite` | `runs` | 실행 또는 관찰 기록 | 커밋된 실행 또는 관찰 기록, 선택적 호환 쓰기 티켓 소비, 행위자 출처, 간결한 증거 갱신. |
| `state.sqlite`와 `artifacts/tmp/` | `artifact_staging` | 임시 아티팩트 스테이징 | 스테이징된 핸들 메타데이터, 생성자 행위자 출처, 안전한 스테이징 사실, 임시 바이트 또는 알림. |
| `state.sqlite`와 아티팩트 저장소 | `artifacts` | 영속 아티팩트 기록 | 영속 아티팩트 메타데이터 또는 본문 위치, 콘텐츠 타입, SHA-256, 크기, 무결성 상태, 가림 처리, 보존, 생산자, 가용성 사실. |
| `state.sqlite` | `artifact_links` | 아티팩트 소유 관계 | 아티팩트와 기준 범위 Core/API 기록 계열 사이의 소유 관계. |
| `state.sqlite` | `evidence_summaries` | 증거 요약 | 간결한 증거 범위, 뒷받침 참조, 공백 참조. |
| `state.sqlite` | `evidence_observations` | 증거 관찰 | 대상 하나에 대한 영속 provenance 레코드입니다. Core 파생 source/assurance, producer 앵커, 분리된 relevance 평가, 정확한 출력, 관찰자, ref, 한계, 타임스탬프를 포함합니다. |
| `state.sqlite` | `user_evidence_observations` | User Channel 증거 관찰 | 현재 Task/Change Unit/scope/baseline 하나와 정확한 정규 아티팩트 출력에 결합된 로컬 사용자 소유 대상 relevance 레코드입니다. |
| `state.sqlite` | `blockers` | 차단 사유 상태 | 다음 행동, 쓰기 호환성, 증거 공백, 닫기 준비 상태, 복구를 위한 구조화된 차단 사유 상태. |
| `state.sqlite` | `authority_events` | 권한 이벤트 흐름 | 커밋된 Core 권한 변경의 추가 전용 순서와 로컬 감사 흐름. |
| `state.sqlite` | `tool_invocations` | 재실행 행 | [저장 효과](storage-effects.md)가 재실행 생성을 정의한 경우의 커밋된 `dry_run=false` Core 메서드 결과 재실행 행입니다. 행위자 출처, 작업 범주, 검증된 호출에서 포착한 선택적 정규 Git 작업 공간 맥락을 포함합니다. |

## 기록 배치 규칙

### 식별자와 소유 관계

기준 범위 기록은 불투명하고 안정적인 식별자를 기본 키 또는 동등한 고유 키로 사용합니다. 고유성은 담당 기록 계열의 소유 범위 안에서 적용됩니다.

- Runtime Home 식별 정보는 그 Runtime Home의 `runtime_home_id` 하나를 저장합니다.
- 프로젝트 등록에는 고유한 `project_internal_id`, 고유한 프로젝트 별칭, 고유한 저장소 루트, 고유한 프로젝트 홈, 고유한 상태 데이터베이스 경로가 필요합니다. `project_name`은 표시 이름이고 `project_alias`는 CLI 선택 보조 값입니다.
- Agent Connection 식별 정보는 `connection_internal_id`별로 고유합니다.
- Connection Projects 멤버십은 `connection_internal_id`와 `project_internal_id`의 조합별로 고유하며, 하나의 연결이 등록된 프로젝트를 주소 지정할 수 있게 하는 유일한 레지스트리 멤버십입니다.
- 호스트 훅 설치 식별 정보는 `guard_installation_id`별로 고유합니다. 프로젝트 범위 호스트 훅 설치는 등록된 프로젝트와 그 프로젝트에 대한 Connection Projects 멤버십을 가진 Agent Connection을 이름 붙여야 합니다.
- 로컬 웹 동의 토큰 식별 정보는 하나의 프로젝트 상태 데이터베이스 안에 저장된 토큰 해시입니다. 원문 토큰은 저장하면 안 됩니다. 대기 토큰은 프로젝트, 선택된 Agent Connection, 대기 판단, `capture_basis`, 만료를 이름 붙여야 합니다. 토큰 소비와 대응하는 사용자 판단 해결은 하나의 프로젝트 상태 트랜잭션 또는 동등한 원자적 작업이어야 합니다.
- 프로젝트 범위 행은 등록된 프로젝트에 속합니다.
- 에이전트 세션, 호스트 훅 이벤트, 프롬프트 캡처, 예상 쓰기, 미기록 변경, 세션 감시 기준선, 세션 감시 관찰은 프로젝트별 `state.sqlite` 하나에 속하며 그 기록을 관찰했거나 만든 Agent Connection을 이름 붙입니다.
- `Task` 범위 행은 자신을 소유한 `tasks` 행과 같은 프로젝트와 같은 `Task`에 속합니다.
- Task에는 같은 프로젝트의 predecessor가 최대 하나 있습니다. Predecessor ID,
  관계, 비어 있지 않은 이유는 모두 없거나 모두 있어야 하고 self-predecessor
  edge는 거부됩니다. `carry_forward_json`은 명시적 disposition 감사 기록이며
  현재 권한 검사를 우회하지 않습니다.
- `AcceptanceCriterionId`는 Core가 생성하며 프로젝트 안에서 고유합니다. 같은 `Task` 복합 키는 대상 외래 키를 지원합니다. 기준이 폐기되면 그 행은 폐기 상태로 남고 활성 identity로 다시 쓰이지 않습니다.
- `EvidenceClaimId`는 호출자가 부여하며 소유 `Task` 안에서만 고유합니다. 다른 `Task`에서는 같은 철자를 독립적으로 사용할 수 있지만, 기존 같은 `Task` ID의 문장은 바꿀 수 없습니다.
- 각 증거 관찰은 같은 `Task` 수락 기준 또는 보충 증거 주장 중 정확히 하나를 이름 붙입니다. 두 대상 열을 모두 `null`로 두거나 둘 다 채울 수 없습니다.
- 현재 적용 포인터와 소유 참조는 같은 프로젝트의 기록을 가리켜야 합니다.
- `Task` 하나에는 현재 적용 Change Unit이 최대 하나만 있습니다.
- 소비된 쓰기 티켓 행, 소비된 스테이징 핸들, 승격된 스테이징 아티팩트, 아티팩트 소유 연결, 재실행 키 같은 단일 사용 관계는 여러 커밋 의미로 갈라지면 안 됩니다.

### 현재 행, 이벤트 행, 재실행 행

현재 기록 계열은 일반 읽기에 쓰는 현재 Core 상태를 담습니다. `authority_events`는 커밋된 Core 권한 변경의 추가 전용 순서와 로컬 감사 흐름입니다. 각 권한 이벤트 행은 `event_id`, `project_id`, 결과 `state_version`, `event_type`, `actor_source`, `operation_category`, `payload_json`, `request_hash`, `previous_event_hash`, `event_hash`, `created_at`을 저장합니다. 이벤트 해시는 로컬 무결성 점검과 내보내기 상관을 위한 필드이며, 로컬 SQLite 저장소는 조작 방지 감사 로그가 아닙니다. `tool_invocations`는 [저장 효과](storage-effects.md)가 재실행 생성을 정의한 경우에만 커밋된 재실행 행을 저장합니다.

상태 버전 동작, 멱등성, 이벤트 의미, 재실행 충돌 처리, 잠금, 마이그레이션 계약은 [저장소 버전 관리](storage-versioning.md)가 담당합니다.

### 권한 번들 내보내기

`volicord export authority-bundle`의 관리 CLI 동작은 [관리 CLI](admin-cli.md#authority-bundle-export)가
담당합니다. 저장소 기록은 그 내보내기의 저장소 행 기준을 담당합니다. 번들의
`records.jsonl`은 선택된 프로젝트의 기준 `state.sqlite` 기록 계열을 저장소 행으로
표현하며, 복사된 영속 아티팩트 본문은 현재 로컬 아티팩트 저장소에서 읽을 수 있는
바이트가 있는 `artifacts` 행에 대한 보조 내보내기 파일입니다.

내보내기 번들은 저장소 행 이름, 열 이름, 저장 값, 저장소 소유 JSON `TEXT` 값을 내보낸
기록 데이터로 보존합니다. SHA-256 체크섬 파일은 내보낸 파일에 라벨을 붙입니다.
이 번들은 Runtime Home을 변조 방지 저장소로 바꾸지 않고, Runtime Home이 내보내기 전에
한 번도 수정되지 않았음을 증명하지 않으며, 정확성, 테스트 충분성, 검토 완료, 배포,
최종 수락, 잔여 위험 수락을 증명하지 않습니다.

### 관계 검증

저장소는 커밋 전에 저장 관계를 검증해야 합니다. 검증에는 아래 항목이 포함됩니다.

- 같은 프로젝트와 같은 `Task` 소유 관계
- 현재 적용 포인터 대상
- 호환되는 쓰기 티켓 소비
- 아티팩트 스테이징 소비와 승격 대상
- 아티팩트 소유 관계
- Agent Connection 처리 경로를 위한 Connection Projects 멤버십과 활성 상태 일관성
- 호스트 훅 설치, 에이전트 세션, 호스트 훅 이벤트, 프롬프트 캡처, 예상 쓰기, 미기록 변경, 세션 감시 기준선, 세션 감시 관찰의 프로젝트 및 연결 범위
- SQLite가 직접 외래 키로 표현할 수 없는 JSON 참조 배열

### 권한 행 보존

일반적인 기준 범위 Core 동작은 생명주기 또는 상태 전환을 통해 권한 행을 보존합니다. `Task`를 완료, 취소, 대체하면 관련 생명주기/상태 의미가 바뀝니다. 그래도 커밋된 권한 행은 감사와 복구를 위해 계속 주소 지정 가능해야 합니다.

이 보존 규칙은 `tasks`, `change_units`, `user_judgments`, `user_evidence_observations`, `project_continuity_records`, `write_tickets`, `runs`, `artifacts`, `artifact_links`, `evidence_summaries`, `evidence_observations`, `blockers`, `authority_events`, `tool_invocations`, `agent_sessions`, `guard_events`, `prompt_captures`, `expected_writes`, `unrecorded_changes`, `session_watch_baselines`, `session_watch_observations`에 적용됩니다. 아티팩트별 임시/영속 보존 규칙은 [아티팩트 저장소](storage-artifacts.md)가 담당합니다.

### 호스트 관찰 기록

호스트 관찰 기록은 호스트 통합 상태에 대한 로컬 권한 사실을 보존합니다. Core와 Store는 이 기록을 근거로 작업을 계속하거나 닫을 수 있는지 판단할 수 있습니다. 그러나 이 기록은 OS 샌드박스, 파일시스템 ACL, 외부 정책 집행, 위조 방지, 행위자 신원, 쓰기 방지를 증명하지 않습니다.

`agent_sessions`, `guard_events`, `prompt_captures`, `expected_writes`, `unrecorded_changes`, `session_watch_baselines`, `session_watch_observations`는 모두 프로젝트 로컬 행입니다. 서로 다른 프로젝트의 `state.sqlite` 데이터베이스 사이로 새면 안 됩니다.

`guard_installations`는 Runtime Home, Agent Connection, 선택적 프로젝트 범위별 설정 생명주기, 관찰된 훅 메타데이터, 호스트 역량을 기록합니다.

- `configured`와 `reload_required`는 파일이나 메타데이터가 설치되었지만, 일치하는 호스트 훅 관찰은 아직 기록되지 않았다는 뜻입니다.
- `active`는 기록된 프로젝트, Agent Connection, 호스트 종류, 통합 프로필, 정책 해시와 일치하는 유효한 호스트 훅을 Volicord가 관찰했다는 뜻입니다. OS 수준 집행이나 샌드박싱을 증명하지 않습니다.

`expected_writes`는 쓰기 상관관계를 결정적으로 기록합니다.

- 대기 행은 탐지형 도구 실행 전 경로가 프로젝트, 연결, 세션, 시간, 경로, `Task`, Change Unit, `active` 쓰기 티켓 좌표로 제한된 예상 쓰기 하나를 허용했다는 뜻입니다.
- 매칭된 행은 도구 실행 후 관찰을 그 예상 쓰기와 연결했다는 뜻입니다. 제품 정확성, 행위자 신원, OS 수준 쓰기 방지를 증명하지 않습니다.
- 매칭되지 않았거나 모호하거나 쓰기 티켓 범위를 벗어난 Product Repository 변경은 미해결 `unrecorded_changes` 행을 만듭니다.

미해결 `unrecorded_changes` 행은 관찰된 Product Repository 변경에 담당 문서가 정의한 조정이 아직 필요하다는 뜻입니다. 행을 해결하면 그 행을 보존하면서 로컬 해결 근거, 행위자 출처, 캡처 근거, 해결 시각, 선택적으로 연결된 사용자 판단을 기록합니다.

`session_watch_baselines`와 `session_watch_observations`는 탐지형 세션 단위 Product Repository 감시를 지원합니다. 샌드박스, 파일시스템 권한 경계, 쓰기 전 차단, 파일을 바꾼 주체나 이유에 대한 증명이 아닙니다.

- 기준선은 감시 가용성, 등록된 저장소 루트 또는 감시 경로 집합, 적용된 제외 항목, 결정적 스냅샷 다이제스트 메타데이터를 저장합니다.
- 관찰은 이후의 안전한 스냅샷을 기준선과 비교해 찾은 변경 제품 경로를 저장합니다. 예상 쓰기, 쓰기 티켓, 미기록 변경의 선택적 상관 참조도 포함할 수 있습니다.
- 관찰을 예상 쓰기나 일치하는 `active` 쓰기 티켓 하나에 연결하는 것은 결정적 상관관계일 뿐입니다.
- 관찰을 `unrecorded_changes` 행에 연결하면 로컬 조정 맥락을 기록합니다. 그 자체로 닫기 차단 사유를 만들지는 않습니다.

<a id="local-diagnostics-store"></a>
### 로컬 진단 저장소

`diagnostics.sqlite`는 독립된 로컬 운영 저장소이며 Core, 레지스트리, 증거,
User Channel, 호스트 관찰 권한 데이터베이스가 아닙니다. 스키마 버전은 이 저장소에만
적용됩니다. `diagnostic_sessions.session_id`는 데이터베이스 내부의 연쇄 외래 키로 각
이벤트를 소유하지만, `registry.sqlite`나 프로젝트 `state.sqlite`와 데이터베이스 간
관계를 두지 않습니다. 연결 및 프로젝트 식별 정보는 권한 효력이 없는 상관 라벨일
뿐입니다.

기본 로컬 수집은 크기가 제한된 집계와 범주형 관찰만 기록합니다.

- 이벤트 종류: `mcp_tool_call`, `guard_hook`, `session`
- 결과: `success`, `rejected`, `validation_failure`, `tool_error`,
  `transport_error`, `unavailable`
- 선택적 확인 User Channel 범주: `mcp_elicitation`, `prompt_capture`,
  `local_web_consent`, `cli_inbox`
- 선택적 대기 대체 경로 범주: `prompt_capture`, `local_web_consent`, `cli_inbox`
- 호출, 지연, 요청/응답 바이트, 검증 실패, 재시도, Core 도달, Core 커밋,
  재실행, 관찰된 제품 쓰기, 권한 상태 새로 고침 실패 카운터

스키마에는 프롬프트, 경로, 파일 본문, 오류 세부 정보, 비밀값, 판단 질문·답변·이유·메모
열이 없습니다. 크기가 제한된 도구 필드는 임의 요청 텍스트가 아니라 식별 정보만
받습니다. 내용을 담는 상세 추적은 지원하지 않습니다. 향후 상세 추적을 추가하려면
이 테이블을 넓히는 대신 별도의 명시적 opt-in, 보존, 가림 계약이 필요합니다.

진단 쓰기 때 보존 한도를 적용합니다. 7일보다 오래된 세션을 제거하고, 세션은 최대
64개, 세션별 이벤트는 최대 1,024개를 유지합니다. 시간 기반 보존은 타임스탬프 텍스트의
사전식 순서가 아니라 해석한 시간 값을 비교합니다. 이 데이터베이스의 부재, 손상,
비호환 버전, 읽기 전용 상태, 쓰기 실패는 MCP, guard, Core, User Channel 결과에 치명적이지
않습니다. 진단은 `state_version`, 증거, 보장 수준, 닫기 준비 상태, 판단, 권한 이벤트,
재실행 행을 갱신하면 안 되며 권한 번들 내보내기는 이 데이터베이스를 제외합니다.

### 현재 닫기 근거

현재 닫기 근거는 `tasks` 계열에 저장되는 Task 소유 현재 상태입니다. 성공한 종료 닫기 결과를 위해 저장되는 종료 닫기 요약과 다릅니다.

권위 있는 현재 `CurrentCloseBasis` 기록은 Task 소유 닫기 근거 좌표와 함께 해석하는 `tasks.close_basis_json`입니다.

기존 열린 Task는 종료 닫기 요약 JSON을 현재 닫기 근거로 자동 변환하지 않습니다. 현재 닫기 근거가 없다는 사실은 빈 생성 근거가 아니라 `tasks.close_basis_json`의 부재로 표현합니다. Change Unit 기록은 현재 `CurrentCloseBasis` 권한을 저장하거나 만족하지 않습니다.

저장된 판단에는 `JudgmentBasis`가 필요합니다. 해결된 저장 판단에는 완전한 기계 판독 가능 해결, 구조화된 설명용 판단 이유 메타데이터, 행위자 출처, 검증 근거, 보장 수준이 필요합니다. 이 사실이 빠진 행은 감사 호환 권한 기록이 아니라 유효하지 않은 소유자 상태입니다.

저장된 판단 권한에서 `user_judgments.status='resolved'`는 답변이 있다는 사실을 기록합니다. 사용자가 승인했다는 뜻이 아닙니다. 현재 판단을 권한 근거로 사용하려면 선택된 선택지, 저장된 `resolution_machine_action`, 저장된 `resolution_outcome`, 적용 가능한 User Channel 행위자 출처, 메서드가 정의한 호환성이 필요합니다. 판단 이유 메타데이터는 답변의 이유와 맥락을 보존하지만 그 자체가 권한, 증거, 수락, 닫기 준비 상태, 잔여 위험 수락은 아닙니다. 결과, 기계 동작, 적용 가능한 행위자 출처, 검증 근거, 보장 수준의 부재는 유효하지 않은 소유자 상태이며 절대 수락이 아닙니다.

### 프로젝트 연속성 기록

`project_continuity_records`는 커밋된 Core 효과에서 비롯된 오래 유지할 프로젝트 수준 맥락을 보존합니다. 기준 기록은 결정, 의무, 알려진 한계, 수락된 잔여 위험, 제약을 나타낼 수 있습니다.

원천 `Task`와 선택적 원천 Change Unit은 연속성 기록이 어디에서 비롯되었는지를 식별합니다. 그 원천 경로를 다시 현재 상태로 만들지는 않습니다. `status='active'`는 기록을 살아 있는 프로젝트 맥락으로 보이게 하고, `superseded`와 `closed`는 감사와 복구를 위해 기록을 계속 주소 지정할 수 있게 둡니다.

프로젝트 연속성 기록은 새 작업의 현재 권한이 아닙니다. 이후 쓰기, `Run`, 판단 요구사항, 닫기 준비 상태 확인, 최종 수락, 잔여 위험 수락, 차단 사유 결정은 여전히 현재 담당자가 정의한 Core 상태와 호환성 규칙을 사용해야 합니다.

## 저장소 소유 값

닫힌 저장소 소유 값 집합은 영속 제약입니다. 알 수 없는 값은 커밋할 수 없습니다.

| 저장 필드 | 기준 범위 값 |
|---|---|
| 프로젝트 등록 `status` | `active` |
| `installation_profile.default_connection_mode` | `read_only`, `workflow` |
| Agent Connection `host_kind` | `codex`, `claude_code`, `generic` |
| Agent Connection `intent` | `personal`, `shared`, `global` |
| Agent Connection `host_scope` | `host_kind` 조합에 따른 `user`, `project`, `local`, `export` |
| Agent Connection `mode` | `workflow`, `read_only` |
| Agent Connection `enabled` | `0`, `1` |
| Agent Connection `last_verification_status` | `not_verified`, `complete`, `action_required`, `failed` |
| 호스트 훅 설치 `guard_mode` | `record`, `detective` |
| 호스트 훅 설치 `installation_status` | `absent`, `configured`, `reload_required`, `active`, `degraded`, `stale`, `broken` |
| `agent_sessions.guard_mode` | `record`, `detective` |
| `guard_events.decision` | `allow`, `deny`, `warn`, `inject_context` |
| `expected_writes.path_policy` | `exact_paths` |
| `expected_writes.status` | `pending`, `matched` |
| `unrecorded_changes.status` | `unresolved`, `resolved` |
| `session_watch_baselines.status` | `disabled`, `active`, `degraded`, `unavailable` |
| `session_watch_baselines.scope_kind` | `repository`, `path_set` |
| `session_watch_observations.observation_status` | `unresolved`, `linked` |
| `change_units.status` | `proposed`, `active`, `replaced`, `closed` |
| `change_units.is_current` | `0`, `1` |
| `write_tickets.status` | `active`, `consumed`, `expired`, `stale`, `revoked` |
| `user_judgments.status` | `pending`, `resolved`, `stale`, `superseded`, `expired` |
| `user_judgments.basis_status` | `current`, `stale`, `superseded` |
| `user_judgments.resolution_machine_action` | 완전한 해결 그룹의 `accept`, `reject`, `defer` |
| `user_judgments.resolution_outcome` | 완전한 해결 그룹의 `accepted`, `rejected`, `deferred` |
| `local_web_consent_tokens.status` | `pending`, `consumed`, `expired` |
| `project_continuity_records.kind` | `decision`, `obligation`, `known_limit`, `accepted_risk`, `constraint` |
| `project_continuity_records.status` | `active`, `superseded`, `closed` |
| `artifact_staging.status` | `staged`, `consumed`, `expired`, `discarded` |
| `artifacts.status` | `available`, `missing`, `integrity_failed`, `unavailable` |
| `artifacts.integrity_status` | `verified`, `corrupt` |
| `artifact_links.owner_record_kind` | `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `evidence_observation`, `blocker` |
| `evidence_observations.source_kind` | `agent_report`, `connection_observation`, `external_tool`, `user_observation`, `reused_evidence`, `unverified_claim` |
| `evidence_observations.assurance_level` | `cooperative_report`, `registered_connection_observed`, `external_tool_result`, `user_observed`, `unverified` |
| `user_evidence_observations.relevance_status` | `supported`, `contradicted` |
| `blockers.status` | `active`, `resolved`, `superseded` |
| `tool_invocations.status` | `committed` |
| `authority_events.operation_category`와 `tool_invocations.operation_category` | `read`, `agent_workflow`, `user_only`, `admin_local`, `local_recovery` |

공개 API 값을 반영하는 행은 [API 값 집합](api/schema-value-sets.md), 관련 스키마 담당 문서, 메서드 담당 문서와 정확히 맞아야 합니다. 이 문서는 `tasks.mode`, `tasks.lifecycle_phase`, `tasks.result`, `runs.kind`, `runs.status`, `evidence_summaries.status` 같은 필드의 공개 API 값을 다시 정의하지 않습니다. 공개 API 값은 [API 값 집합](api/schema-value-sets.md), [API 상태 스키마](api/schema-state.md), 메서드 담당 문서를 봅니다.

행에 저장된 `evidence_observations.source_kind` / `assurance_level` 조합은 enum 값만으로
충분한 강한 출처가 되지 않습니다. Core는 메서드 소유 파생 뒤에 이 조합을 기록하고,
요청 멤버를 신뢰하지 않고 확인된 호출에서 `observed_by_actor_source`를 가져옵니다. 현재
닫기 평가와 재사용 평가는 대상, `Task`와 Change Unit, 출처 실행 기록, 현재 범위 리비전과
기준선, 정확한 현재 출력 바이트, 타입이 지정된 producer 앵커, 별도의 relevance 평가를
다시 검증하고 입증되지 않으면 차단합니다. 기준 구현에는 authority-owned 외부 도구 또는
등록 연결 producer 경로가 없으므로 해당 직접 주장은 아티팩트 바이트를 사용할 수 있고
검증된 상태여도 협력적 상태로 남습니다. `user_observation` 행은 정확한 출력과
`relevance_status=supported`를 가진 현재 `user_evidence_observations` 레코드를 가리켜야
합니다. `reused_evidence` 행은 원래 증거 관찰 하나만 가리켜야 하며 Core는 그 identity,
승계한 보장 수준, 출력, producer, relevance를 재귀적으로 다시 검증합니다. 설명용 도구
메타데이터, raw guard payload, 아티팩트 무결성, `source_refs_json`은 producer 또는
relevance 레코드를 대신할 수 없습니다.

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
| 호스트 훅 설치 | 로컬 호스트 훅 설정 상태를 위한 호스트 역량 JSON과 메타데이터입니다. OS 집행 증명이 아닙니다. |
| `agent_sessions` | 프로젝트 범위 에이전트 세션에 대한 비권한 메타데이터. |
| `guard_events` | 로컬 호스트 판단 요청의 호스트 훅 대상 JSON, 결과 JSON, 메타데이터. |
| `prompt_captures` | 캡처된 프롬프트 기록의 비권한 메타데이터. 프롬프트 본문은 `null`을 허용하는 별도 텍스트 열입니다. |
| `expected_writes` | `detective` 예상 쓰기 상관관계를 위한 예상 경로 배열, 쓰기 티켓 ID 배열, 일치 경로 배열, 메타데이터. |
| `unrecorded_changes` | 미기록 Product Repository 변경의 관찰 경로 배열, 탐지 JSON, 해결 JSON, 메타데이터. 해결 JSON은 간결한 해결 근거, 캡처 근거, 해결 메서드, 선택적 연결 사용자 판단 참조를 저장합니다. 전체 민감 명령이나 프롬프트 내용을 저장하면 안 됩니다. |
| `session_watch_baselines` | 세션 감시 기준선을 위한 감시 경로 배열, 유효한 제외 배열, 스냅샷 항목 배열, 메타데이터. 스냅샷 항목은 경로, 종류, 크기, 해시 또는 건너뛴 이유 메타데이터만 저장하며 파일 내용을 저장하지 않습니다. |
| `session_watch_observations` | 세션 감시 관찰을 위한 관찰된 변경 경로 배열, 간결한 변경 요약 JSON, 스냅샷 항목 배열, 메타데이터. 스냅샷과 변경 요약은 행위자 식별, 의도, 제품 정확성, 닫기 준비 상태를 증명하지 않습니다. |
| `tasks` | 구체화 요약, 제한된 목록, 자율성 경계, carry-forward disposition, 현재 닫기 근거, 종료 닫기 요약, 생명주기 요약. Acceptance policy, work phase, lineage edge identity는 전용 열을 사용하고 수락 기준과 보충 증거 주장은 각 정규 관계형 테이블을 사용합니다. |
| `change_units` | 범위 요약, 제한된 목록, 쓰기 근거 요약, 선택적 효과 계약 데이터, 생명주기 지원 데이터. |
| `user_judgments` | 판단 요청, 맥락, 선택지, 영향 참조, 아티팩트 참조, 근거 스냅샷, 민감 동작 범위, 기계 판독 가능 해결, 설명용 판단 이유 메타데이터. |
| `project_continuity_records` | 오래 유지하는 프로젝트 맥락을 위한 적용 대상 경로, 적용 대상 참조, 원천 참조, 아티팩트 참조, 대체된 참조, 검토 트리거, 비권한 메타데이터. |
| `write_tickets` | 쓰기 티켓 시도 범위와 비권한 메타데이터. |
| `runs` | 요약, 관찰된 변경, 증거 갱신, 쓰기 티켓 효과 데이터, 비권한 메타데이터. |
| `artifact_staging` | 스테이징된 아티팩트 데이터, 안전 메타데이터, 비권한 메타데이터. |
| `artifacts` | 보존, 생산자, 비권한 메타데이터. |
| `artifact_links` | 비권한 메타데이터. |
| `evidence_summaries` | 증거 범위, 뒷받침 참조, 공백 참조, 비권한 메타데이터. |
| `evidence_observations` | 증거 관찰 하나의 도구 메타데이터, Core 기록 입력 ref, 권한 효력이 없는 `SourceRef` JSON, 출력 아티팩트 ref, 한계, 타입이 지정된 Core 파생 producer/relevance 권한 메타데이터입니다. `source_refs_json`은 권한을 만들지 않습니다. |
| `user_evidence_observations` | 현재 근거 좌표, 대상 identity, relevance, 정확한 아티팩트 ref, 로컬 사용자 actor, 검증 근거, 요약, 타임스탬프입니다. |
| `blockers` | 차단 사유 소유 참조, 관련 참조, 세부 정보, 비권한 메타데이터. |
| `authority_events` | 커밋된 Core 권한 변경의 이벤트 페이로드. |
| `tool_invocations` | 커밋된 재실행 응답과 재실행 호환성에 사용하는 선택적 정규 Git 작업 공간 맥락 JSON. |

`Task`와 Change Unit 구체화 JSON은 간결한 요약과 제한된 목록만 저장합니다. 추가 영속 기록 계열을 만들지 않습니다.

## 관련 담당 문서

- [저장 효과](storage-effects.md): 어떤 메서드 분기가 기록을 만들거나, 바꾸거나, 관찰하거나, 건드리지 않는지 정의합니다.
- [저장소 DDL](storage-ddl.md): 기준 SQLite 테이블 형태, 인덱스, 외래 키, 제약, 기준 SQL 원본을 정의합니다.
- [아티팩트 저장소](storage-artifacts.md): 아티팩트 스테이징, 승격, 연결, 본문 읽기, 보존, 무결성 생명주기를 정의합니다.
- [저장소 버전 관리](storage-versioning.md): 상태 버전 시계, 멱등성, 재실행, 이벤트, 잠금, 호환되지 않는 저장소 처리를 정의합니다.
- [Agent Connection](agent-connection.md): Agent Connection, Connection Projects, 모드로 제한되는 MCP 도구 접근, User Channel 경계를 정의합니다.
- [API 코어 스키마](api/schema-core.md), [API 상태 스키마](api/schema-state.md), [API 아티팩트 스키마](api/schema-artifacts.md), [API 판단 스키마](api/schema-judgment.md), [API 값 집합](api/schema-value-sets.md): API 형태와 공개 API 값을 정의합니다.
- [API 메서드](api/methods.md)와 메서드 담당 문서: 기록을 사용하는 공개 메서드 동작을 정의합니다.
- [런타임 경계](runtime-boundaries.md): `Product Repository`, Volicord 설치 또는 런타임 프로세스, `Volicord Runtime Home` 위치 경계를 정의합니다.
- [상태 보기 권한 참조](projection-and-templates.md)와 [템플릿 본문](template-bodies.md): 읽는 시점의 상태 보기 권한과 렌더링된 템플릿 본문을 정의합니다.
- [보안](security.md): 보안 경계와 보장 수준을 정의합니다.
