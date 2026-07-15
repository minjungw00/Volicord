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
- `project_state.state_version`, 정규 Core UTC 시계와 그 영속 하한, 멱등성,
  재실행, 이벤트, 잠금, 호환되지 않는 저장소 처리: [저장소 버전 관리](storage-versioning.md)
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

- `registry.sqlite`는 Runtime Home 식별 정보, 설치 프로필, 프로젝트 등록 매핑과 별칭, Agent Connection, Connection Projects 멤버십, 호스트 역량 검증 이력과 현재 포인터, 호스트 훅 설치, 레지스트리 메타데이터를 저장합니다. 설치 프로필에는 선택된 `volicord` 명령, MCP 시작 명령, 실행 파일 디렉터리, 기본 연결 모드, 메타데이터, 타임스탬프가 포함됩니다. 프로젝트 등록에는 `project_internal_id`, 표시 이름, CLI 선택 별칭, Runtime Home 관계, 등록된 `repo_root`, `project_home`, 프로젝트 `state.sqlite` 경로, 상태, 메타데이터, 타임스탬프가 포함됩니다.
- `diagnostics.sqlite`는 필요할 때 생성되는 크기 제한 비권한 로컬 운영 진단 저장소입니다. `registry.sqlite` 및 모든 프로젝트 `state.sqlite`와 분리되며 어느 데이터베이스에도 외래 키를 두지 않습니다.
- `projects/{project_internal_id}/`는 등록된 프로젝트 하나에 대한 기본 Volicord 프로젝트 홈 형태입니다. `repo_root`와 같은 위치나 권한이 아닙니다.
- `state.sqlite`는 등록된 프로젝트의 로컬 Core 상태와 프로젝트 범위 호스트 관찰 기록을 저장합니다.
- `artifacts/`는 아티팩트 저장소를 사용할 때의 프로젝트 아티팩트 저장소이며, 아티팩트 저장소가 처음 필요할 때 늦게 만들어질 수 있습니다. `artifacts/tmp/`는 아티팩트 스테이징에 필요할 때 쓰는 임시 스테이징 공간이며 증거 권한이 아닙니다. 이 디렉터리도 스테이징이 일어날 때 늦게 만들어질 수 있습니다. 이 디렉터리들은 프로젝트 등록 직후에 반드시 존재할 필요가 없습니다.

아티팩트 경로 기준:

- `artifact_staging.tmp_path`는 `project_home` 기준 상대 경로로 저장합니다. 임시 스테이징 영역 아래의 스테이징 바이트 또는 알림은 `artifacts/tmp/<file>` 같은 형태를 사용합니다.
- `artifacts.body_path`는 보통 `project_home/artifacts`인 아티팩트 저장소 루트 기준 상대 경로로 저장합니다. 영속 본문은 `tmp/<file>` 같은 형태를 사용하며 `artifact_store_root.join(body_path)`로 해석합니다.

운영 프로젝트 기록에서 `project_home`은 프로젝트별 로컬 런타임 상태 위치를 담당합니다. 실행 가능한 프로젝트 상태 데이터베이스 경로는 검증된 프로젝트 홈에서 `project_home/state.sqlite`로 파생합니다. 저장된 `state_db_path`는 영속성과 진단을 위해 `registry.sqlite`에 남지만, Store가 정상 `ProjectRecord`를 반환하거나, 프로젝트별 상태를 열거나, Agent Connection 프로젝트 접근을 해석하거나, Core 실행에 들어가거나, MCP 프로젝트 가용성을 보고하기 전에 이 파생 경로와 일치해야 합니다. 일치하지 않는 등록은 진단을 위한 원시 레지스트리 내용으로 검사할 수 있지만, 운영 조회와 목록 조회는 그 행을 생략하거나 정상 프로젝트로 반환하지 말고 거절해야 합니다. 검사는 대체 `state_db_path`를 열거나, 만들거나, 초기화하거나, 복구하면 안 됩니다.

`Product Repository`는 `repo_root`로 등록되는 사용자 제품 파일 경계입니다. Volicord Runtime Home이 아니며, Core 권한 저장소가 아니고, 런타임 기록, 재실행 행, 판단, 쓰기 티켓, 호스트 훅 기록, Agent Connection 레지스트리 상태를 저장하는 위치도 아닙니다.

기준 SQLite 테이블 형태, 인덱스, 외래 키, 제약, 기준 SQL 원본은 [저장소 DDL](storage-ddl.md)이 담당합니다. 이 기록들의 현재 기준 SQLite 저장소 프로필은 `baseline_sqlite_v6`이며, 저장소 프로필과 호환되지 않는 저장소 경계 동작은 [저장소 버전 관리](storage-versioning.md)가 담당합니다.

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
| `registry.sqlite` | `host_capability_verifications` | 변경 불가능한 호스트 역량 검증 이력 | 정확한 연결·역량, 결과, 호스트·클라이언트 버전, 어댑터 프로필, 관리 지문, Volicord 빌드·source·target·실행 파일 다이제스트, 크기가 제한된 증거 아티팩트 다이제스트, 관찰·만료 기간, 엄격한 정규 `{}` 메타데이터, 생성 시각. |
| `registry.sqlite` | `host_capability_state` | 현재 호스트 역량 포인터 | 연결과 역량마다 현재의 변경 불가능한 검증 행 하나를 가리키며 이후 통과·실패·사용 불가·취소 관찰로 원자적으로 교체됩니다. |
| `registry.sqlite` | 호스트 훅 설치 | 호스트 훅 설정과 호스트 역량 기록 | Runtime Home, Agent Connection, 선택적 프로젝트 범위, 호스트 종류, 통합 모드, 호스트 역량 JSON, 설치 생명주기 상태, 관찰된 훅 메타데이터, 타임스탬프, 메타데이터. |
| `state.sqlite` | `project_state` | 프로젝트 상태 헤더 | 저장 프로필, `state_version`, 현재 적용 `Task` 포인터, 프로젝트 강제 프로필, 정규 Core UTC 시계의 영속 하한인 `updated_at`. |
| `state.sqlite` | `agent_sessions` | 관찰된 에이전트 세션 | Agent Connection 하나에 대한 프로젝트 범위 세션, 선택적 호스트 훅 설치, 호스트 종류, 통합 프로필, 시작/종료 타임스탬프, 메타데이터. |
| `state.sqlite` | `guard_events` | 호스트 훅 판단 이벤트 | 연결 및 선택적 세션 또는 설치에 묶이는 프로젝트 범위 호스트 훅 이벤트입니다. 판단 값, 대상 JSON, 결과 JSON, 타임스탬프, 메타데이터를 포함합니다. |
| `state.sqlite` | `prompt_captures` | 프롬프트 캡처 | 세션에 대한 프로젝트 범위 프롬프트 캡처입니다. 연결, 캡처 종류, 프롬프트 해시, 선택적 프롬프트 본문, 타임스탬프, 메타데이터를 포함합니다. |
| `state.sqlite` | `expected_writes` | 예상 Product Repository 쓰기 | 허용된 `detective` 도구 실행 전 쓰기가 만드는 프로젝트 범위 예상 쓰기 상관 기록입니다. 연결/세션 식별 정보, 선택적 호스트 호출 식별 정보, 정확한 경로 정책, 현재 적용 `Task`/Change Unit/쓰기 티켓 근거, 타임스탬프, 일치한 도구 실행 후 메타데이터를 포함합니다. |
| `state.sqlite` | `unrecorded_changes` | 미기록 Product Repository 변경 | Core 실행 또는 담당 문서가 정의한 다른 기록과 아직 연결되지 않은 관찰된 Product Repository 변경에 대한 프로젝트 범위 미해결 또는 해결 기록. |
| `state.sqlite` | `session_watch_baselines` | 세션 감시 기준선 | 등록된 Product Repository 또는 감시 경로 집합에 대한 프로젝트 범위 세션 감시 상태와 기준선 스냅샷입니다. 유효한 제외 항목, 스냅샷 다이제스트 메타데이터, 간결한 스냅샷 항목, 관리 MCP 기준선의 크기가 제한된 실제 initialize 클라이언트 정체성을 포함합니다. |
| `state.sqlite` | `session_watch_observations` | 세션 감시 관찰 | 이후의 안전한 스냅샷을 기준선과 비교해 얻은 프로젝트 범위 `detective` 관찰입니다. 관찰된 변경 경로, 선택적 예상 쓰기 또는 쓰기 티켓 상관 관계, 기존 미기록 변경 행에 대한 선택적 연결을 포함합니다. |
| `state.sqlite` | `tasks` | 작업 단위 상태 | 모드와 work phase, Task 소유 acceptance policy와 이유, 선택적 predecessor 관계와 carry-forward 감사, 구체화 요약, 범위와 닫기 근거 리비전, `null` 허용 현재 닫기 근거, 생명주기/결과/종료 요약, 현재 Change Unit 포인터, 생성자 actor source를 가진 사용자 가치 단위. |
| `state.sqlite` | `acceptance_criteria` | 수락 기준 | Core가 생성한 기준 identity, 소유 `Task`, 문장, 증거 요구 수준, 교체 순서, 활성/폐기 상태, 타임스탬프. |
| `state.sqlite` | `evidence_claims` | 보충 증거 주장 | 호출자가 부여한 `Task` 범위 주장 identity와 비어 있지 않은 불변 문장 하나. |
| `state.sqlite` | `change_units` | 범위 있는 작업 경계 | 범위 요약, 쓰기 근거, Change Unit 생명주기, 소유 `Task` 관계. |
| `state.sqlite` | `evidence_capture_intents` | 증거 캡처 intent | 현재 Task/Change Unit/scope/baseline/target/workspace, 정확한 capture spec과 command/tool input digest 또는 Core가 파생한 connection source-selector digest, 요청 connection과 actor, 예상 outcome, timestamp에 결합된 만료되는 불변 요청. |
| `state.sqlite` | `user_action_requests` | 사용자 행동 요청 | 폐쇄형 행동 요청 JSON, Core 파생 근거와 호환성, required-for 대상, 요청 actor, 원천 메서드/idempotency 관계, expiry를 담습니다. 캡처 폼과 유효 lifecycle 상태는 합성 열로 저장하지 않고 파생합니다. |
| `state.sqlite` | `user_action_resolutions` | 변경 불가능한 User Channel resolution | 요청당 최대 하나이며 폐쇄형 종류 일치 본문, channel kind와 크기가 제한된 visible-ASCII submission replay identity, local-user provenance, verification basis, assurance, Core 캡처 시각을 담습니다. Choice 사실 또는 전체 관찰 detail은 본문에 남습니다. |
| `state.sqlite` | `user_action_channel_tokens` | User Channel fallback token | 요청, connection, expiry, capture basis, 정확한 fallback 종류·`delivery_surface=model_invisible_user_surface`·endpoint·canonical-form digest를 담은 폐쇄형 생성 metadata에 결속된 hash-only 일회성 local-web token. |
| `state.sqlite` | `project_continuity_records` | 프로젝트 연속성 맥락 | 원천 `Task`가 닫힌 뒤에도 주소 지정할 수 있게 남는 프로젝트 수준 결정, 의무, 알려진 한계, 수락된 잔여 위험, 제약. |
| `state.sqlite` | `write_tickets` | 쓰기 티켓 권한 | 단일 사용 쓰기 티켓 권한 기록, 기준 버전, 시도 범위, 만료, 행위자 출처, 선택적 원천 판단, 소비 상태를 저장하는 물리 테이블입니다. |
| `state.sqlite` | `runs` | 실행 또는 관찰 기록 | 커밋된 실행 또는 관찰 기록, 선택적 호환 쓰기 티켓 소비, 행위자 출처, 간결한 증거 갱신. |
| `state.sqlite`와 `artifacts/tmp/` | `artifact_staging` | 임시 아티팩트 스테이징 | 스테이징된 핸들 메타데이터, 생성자 행위자 출처, 안전한 스테이징 사실, 임시 바이트 또는 알림. |
| `state.sqlite`와 `artifacts/tmp/` | `evidence_capture_receipts` | transient staging을 가진 영속 evidence-source fact receipt | Capture intent마다 정확한 source/result digest, 관찰 outcome, 등록 source 좌표, 한계, timestamp를 가진 불변의 완전하고 content-bound된 redacted safe receipt와 transient staging handle 하나. Staged bytes를 승격한 뒤에도 receipt 행은 계속 주소 지정할 수 있습니다. |
| `state.sqlite` | `evidence_capture_source_claims` | 배타적 증거 source claim | Receipt 하나가 소비한 각 host invocation, guard event, session-watch observation을 프로젝트 범위에서 정규화한 identity와 정확한 intent/receipt 쌍, capture kind, claim timestamp. |
| `state.sqlite`와 아티팩트 저장소 | `artifacts` | 영속 아티팩트 기록 | 영속 아티팩트 메타데이터 또는 본문 위치, 콘텐츠 타입, SHA-256, 크기, 무결성 상태, 가림 처리, 보존, 생산자, 가용성 사실. |
| `state.sqlite` | `artifact_links` | 아티팩트 소유 관계 | 아티팩트와 기준 범위 Core/API 기록 계열 사이의 소유 관계. |
| `state.sqlite` | `evidence_summaries` | 증거 요약 | 간결한 증거 범위, 뒷받침 참조, 공백 참조, 현재 행 값을 만든 결과 프로젝트 상태 버전. |
| `state.sqlite` | `evidence_observations` | 증거 관찰 | 대상 하나에 대한 영속 provenance 레코드입니다. Core 파생 source/assurance, producer 앵커, 분리된 relevance 평가, 정확한 출력, 관찰자, ref, 한계, 타임스탬프를 포함합니다. |
| `state.sqlite` | `evidence_producers` | finalization된 증거 producer | Run 하나와 현재 근거에 결합되고 canonical producer JSON을 가진 불변 일대일 intent/receipt/observation/artifact 권한 레코드. |
| `state.sqlite` | `blockers` | 차단 사유 상태 | 다음 행동, 쓰기 호환성, 증거 공백, 닫기 준비 상태, 복구를 위한 구조화된 차단 사유 상태. |
| `state.sqlite` | `authority_events` | 권한 이벤트 흐름 | 커밋된 Core 권한 변경의 추가 전용 순서와 로컬 감사 흐름. |
| `state.sqlite` | `tool_invocations` | 재실행 및 정확한 동작 결과 행 | [저장 효과](storage-effects.md)가 재실행 생성을 정의한 경우의 커밋된 `dry_run=false` Core 메서드 결과 재실행 행입니다. 변경 불가능한 `response_json`, 행위자 출처, 작업 범주, 선택적 검증 근거, 검증된 호출에서 포착한 선택적 정규 Git 작업 공간 맥락을 포함합니다. 조회할 수 있는 `operation_category=agent_workflow` 행은 `OperationResultRef`가 가리키는 저장 원본이기도 합니다. |

관리 Codex 또는 Claude Code 상관관계가 세션 식별 정보를 제공할 때 저장소에는
[호스트 릴리스 증거](host-release-evidence.md)가 정의한 불투명
`managed_host_session_id`만 전달됩니다. `mhs_` namespace는 이 매핑에만 예약됩니다. 기존
매핑 세션은 정확히 같은 등록 연결과 호스트 종류에서만 재사용할 수 있습니다. 다른 연결
또는 호스트에서 재사용하려는 시도는 기존 행을 바꾸지 않고 거부하며 generic 또는 manual
경로는 이 namespace를 미리 심을 수 없습니다. 원본 native session, event, tool-call,
capture, turn, invocation identifier는 저장 기록, JSON 메타데이터, 로그 payload, 진단 세션
식별자가 될 수 없습니다. 관리 수집은 영속화 전에 상관관계 identifier를 domain 분리
불투명 값으로 바꿉니다. 매핑이 없거나 잘못됐거나 일치하지 않으면 그 상태로 남기며,
저장소가 대체 값을 만들거나 잘못된 marker의 진단 상태를 만들면 안 됩니다.

## 기록 배치 규칙

### 식별자와 소유 관계

기준 범위 기록은 불투명하고 안정적인 식별자를 기본 키 또는 동등한 고유 키로 사용합니다. 고유성은 담당 기록 계열의 소유 범위 안에서 적용됩니다.

- Runtime Home 식별 정보는 그 Runtime Home의 `runtime_home_id` 하나를 저장합니다.
- 프로젝트 등록에는 고유한 `project_internal_id`, 고유한 프로젝트 별칭, 고유한 저장소 루트, 고유한 프로젝트 홈, 고유한 상태 데이터베이스 경로가 필요합니다. `project_name`은 표시 이름이고 `project_alias`는 CLI 선택 보조 값입니다.
- Agent Connection 식별 정보는 `connection_internal_id`별로 고유합니다.
- Connection Projects 멤버십은 `connection_internal_id`와 `project_internal_id`의 조합별로 고유하며, 하나의 연결이 등록된 프로젝트를 주소 지정할 수 있게 하는 유일한 레지스트리 멤버십입니다.
- 호스트 역량 검증 식별 정보는 `verification_internal_id`별로 고유하며 각 이력 행은 Agent
  Connection 하나와 정확한 역량 하나에 속합니다. `host_capability_state`는 같은 연결과
  역량의 행만 가리킬 수 있습니다. 현재의 통과하지 않은 행은 더 오래된 통과 행으로
  fallback하지 못하게 합니다. 정규 UTC 구간 값은 `observed_at <= created_at`과
  `observed_at < expires_at <= observed_at + 86,400 seconds`를 만족해야 하며 통과 행은
  `created_at < expires_at`도 만족해야 합니다. 행은 `observed_at <= now < expires_at`에서만
  최신입니다. 24시간은 기본 수명이나 attestation 기간이 아니라 최대 최신성 구간입니다.
  통과하는 내장 stdio 행은
  `host_version = client_version`을 요구하고 그 단일 버전은 정확한 런타임
  `clientInfo.version`, 실제 아티팩트의 설치 호스트 버전과 모두 같아야 합니다.
  `source_revision`은 정확한 소문자 40자리 또는 64자리 16진수이며 `unknown`은 통과할 수
  없습니다.
- 정확히 같은 검증 ID와 내용을 게시하는 것은 멱등입니다. 그 이력 행이 더 이상 현재 행이
  아니면 중복 게시는 더 새로운 포인터를 뒤로 옮기지 않습니다. 같은 ID에 다른 내용을
  사용하면 충돌합니다.
- 비활성 connection은 일반적으로 membership을 유지할 수 있으며, 그것만으로 마이그레이션 cleanup 기록이 되지 않습니다. 마지막 project 호스트 마이그레이션 cleanup은 `project_id`와 `replacement_connection_id`를 담은 정확한 `agent_connections.metadata_json.pending_host_cleanup` 객체로만 식별합니다. Cleanup transaction은 호스트 폐기와 membership 제거 전에 이 marker, 비활성 상태, 보존된 membership 하나가 모두 일치하는지 검증해야 합니다.
- `agent_connections.metadata_json.pending_host_cleanup`은 Store 소유 복구 상태입니다. 일반 Agent Connection 등록과 갱신 입력은 이 예약 키를 거절해야 하고, 일반 활성화·비활성화 또는 Connection Projects membership 변경은 marker가 있는 행을 거절해야 합니다. 마이그레이션은 marker가 있는 행을 요청 대상으로 활성화하면 안 됩니다. 마이그레이션 전환과 cleanup 작업은 project membership을 다시 검증할 때만 대체 inventory의 marker를 다시 연결하거나 제거할 수 있습니다.
- `pending_host_cleanup` 값에 구성원이 빠졌거나, 추가됐거나, 비어 있거나, 형식이 잘못되면 재개 가능한 cleanup이 아닙니다. Doctor는 이를 유효하지 않은 예약 marker로 보고해야 하며 cleanup과 마이그레이션 발견은 유효한 inventory로 해석하면 안 됩니다.
- 호스트 훅 설치 식별 정보는 `guard_installation_id`별로 고유합니다. 프로젝트 범위 호스트 훅 설치는 등록된 프로젝트와 그 프로젝트에 대한 Connection Projects 멤버십을 가진 Agent Connection을 이름 붙여야 합니다.
- 로컬 웹 동의 토큰 식별 정보는 하나의 프로젝트 상태 데이터베이스 안에 저장된 domain-separated 토큰 해시입니다. 원문 토큰은 저장하면 안 됩니다. 대기 토큰은 프로젝트, 선택된 Agent Connection, 대기 `UserActionRequest`, `capture_basis`, 만료를 이름 붙여야 합니다. 토큰 소비와 대응하는 `UserActionResolution` 삽입은 하나의 프로젝트 상태 트랜잭션 또는 동등한 원자적 작업이어야 합니다.
- `user_action_resolutions.channel_submission_id`는 visible ASCII
  `0x21..=0x7e` 1~256 bytes이며 프로젝트와 channel kind 안에서 고유합니다. Local-web
  값은 프로젝트, 요청, bearer-token credential, 예상 connection, 폐쇄형 완료 metadata에
  결속된 digest-only identity입니다. 대응 replay request hash는 별도로 domain-separated
  token digest, 예상 connection, 타입이 지정된 canonical metadata를 포함합니다. 어떤
  영속 레코드도 원문 token이나 내부 binding 객체를 저장하지 않습니다.
- 모든 `user_action_requests` 행은 정확한 `source_method`와
  `source_idempotency_key`를 저장합니다. 직접 `volicord.request_user_action` 원천은
  프로젝트마다 요청 하나에만 대응하므로 같은 Agent Connection이 두 번째 요청을 만들지
  않고 정확한 원래 결과를 재개할 수 있습니다. `volicord.reconcile_changes` 커밋 하나는
  요청을 여러 개 만들 수 있으므로 그 행들은 reconciliation idempotency key를 의도적으로
  공유할 수 있습니다.
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

일반 Core 권한 커밋에서 `project_state.updated_at`, 이벤트 묶음의 모든
`authority_events.created_at`, 선택적 재실행 행의 `tool_invocations.created_at`은
transaction timestamp 하나와 정확히 같은 값을 저장합니다. 이 timestamp는 준비된
동작 시각 샘플보다 이르지 않습니다. 이전 하한과 같을 수 있으므로 서로 다른
`state_version` 값마다 timestamp도 반드시 달라진다는 뜻은 아닙니다.

Core mutation application이 해당 transaction에서 직접 생성하는 Store transaction
metadata도 적용되는 `created_at`, `updated_at`, `retired_at`, `promoted_at` 값을
포함해 정확히 같은 transaction timestamp를 사용합니다. 이 규칙은 의미 있는 동작
시각인 `requested_at`, `resolved_at`, `closed_at`, `recorded_at`, `consumed_at`이나
입력·관찰 담당 사실인 `observed_at`, `started_at`을 바꾸지 않습니다. 이 값들은 각
담당 문서가 정의한 동작 샘플 하나 또는 담당자가 검증한 원천 시각을 보존합니다.

커밋된 모든 `evidence_summaries` 삽입 또는 갱신은 transaction의 결과
`project_state.state_version`을 `produced_at_state_version`에 저장합니다. `Task`의
현재 Evidence Summary는 `created_at`이나 `updated_at`이 가장 큰 행이 아니라
`produced_at_state_version`이 가장 큰 행입니다. UTC timestamp는 담당자가 정의한
시간 의미를 유지하며 권한 커밋 순서를 대신하는 값으로 사용하면 안 됩니다. 불투명
record ID도 권한 순서 key가 아닙니다.

Artifact staging, 등록된 evidence-capture receipt 이행, 로컬 User Channel token
발급은 Core 권한 커밋이 아니라 저장소 소유 시간 효과입니다. 각 효과는 자신의
`created_at` 이상으로 `project_state.updated_at`을 같은 transaction에서 갱신하지만
권한 이벤트나 재실행 행을 만들지 않고 `state_version`도 증가시키지 않습니다. 정확한
재실행, 거부, dry run, 읽기 전용 경로는 더 늦은 하한을 영속화하지 않습니다. 전체
시계 규칙과 bootstrap 보존 규칙은 [저장소 버전 관리](storage-versioning.md#canonical-core-utc-clock)가
담당합니다.

<a id="exact-operation-result-storage"></a>
#### 정확한 동작 결과 저장

조회할 수 있는 `operation_category=agent_workflow` Core 커밋에서
`tool_invocations.response_json`은 멱등 재실행과 읽기 전용
`volicord.get_operation_result` 페이지 조회에 함께 쓰는
변경 불가능한 정확한 직렬화 메서드 결과입니다. `OperationResultRef`는 이 기존
행을 가리키며 별도 기록 계열을 만들거나 응답을 페이지 기록으로 복사하지
않습니다. 페이지는 저장 바이트를 다시 쓰거나 자르거나 다시 계산하지 않고 UTF-8
경계에 맞는 연속 부분을 읽습니다.

저장된 행위자와 프로젝트는 계속 그 행의 소유 경계에 속합니다. 조회가 그 경계를
넓히지 않으며 과거 응답 바이트는 현재 Core 권한이 아닙니다. 정확한
`volicord.resolve_user_action` 응답과 그 비공개 사용자 텍스트를 포함한
`operation_category=user_only` 행은 Agent Connection 결과 조회 대상이 아닙니다.
`volicord.stage_artifact`에는 재실행 행이 없으므로 `OperationResultRef`도
없습니다.

이 조회 기능은 현재 `tool_invocations` 행과 `response_json`을 재사용합니다. 새
테이블, 열, 영속 페이지 기록, 기록 계열, 저장소 마이그레이션을 추가하지 않습니다.

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
- evidence-capture intent/receipt의 일회성 fulfillment와 producer의 일대일 intent,
  receipt, observation, artifact, Run 관계
- capture class별 정확한 source 형태와 각 원천 invocation, guard event, watcher
  observation에 대한 배타적 정규화 claim. Staging, receipt, claim은 함께 commit되거나
  rollback됩니다.
- `intent.created_at <= observed_at < intent.expires_at`, observation 뒤이면서 expiry
  전인 receipt 생성, intent expiry와 정확히 같은 staging expiry로 이루어진
  evidence-capture source 시간 관계
- Agent Connection 처리 경로를 위한 Connection Projects 멤버십과 활성 상태 일관성
- 호스트 훅 설치, 에이전트 세션, 호스트 훅 이벤트, 프롬프트 캡처, 예상 쓰기, 미기록 변경, 세션 감시 기준선, 세션 감시 관찰의 프로젝트 및 연결 범위
- 엄격하게 저장된 entry, scope, path, algorithm, digest로 정규 재구성할 수 있는
  session-watch baseline 및 observation snapshot. 저장된 `observed_paths_json`과
  `change_summary_json`은 다시 계산한 diff와 같아야 합니다.
- SQLite가 직접 외래 키로 표현할 수 없는 JSON 참조 배열

### 권한 행 보존

일반적인 기준 범위 Core 동작은 생명주기 또는 상태 전환을 통해 권한 행을 보존합니다. `Task`를 완료, 취소, 대체하면 관련 생명주기/상태 의미가 바뀝니다. 그래도 커밋된 권한 행은 감사와 복구를 위해 계속 주소 지정 가능해야 합니다.

이 보존 규칙은 `tasks`, `change_units`, `evidence_capture_intents`, `evidence_capture_receipts`, `evidence_capture_source_claims`, `user_action_requests`, `user_action_resolutions`, `user_action_channel_tokens`, `project_continuity_records`, `write_tickets`, `runs`, `artifacts`, `artifact_links`, `evidence_summaries`, `evidence_observations`, `evidence_producers`, `blockers`, `authority_events`, `tool_invocations`, `agent_sessions`, `guard_events`, `prompt_captures`, `expected_writes`, `unrecorded_changes`, `session_watch_baselines`, `session_watch_observations`에 적용됩니다. Receipt의 staging handle과 staged bytes만 transient artifact lifecycle을 따릅니다. 아티팩트별 임시/영속 보존 규칙은 [아티팩트 저장소](storage-artifacts.md)가 담당합니다.

### 호스트 관찰 기록

호스트 관찰 기록은 호스트 통합 상태에 대한 로컬 권한 사실을 보존합니다. Core와 Store는 이 기록을 근거로 작업을 계속하거나 닫을 수 있는지 판단할 수 있습니다. 그러나 이 기록은 OS 샌드박스, 파일시스템 ACL, 외부 정책 집행, 위조 방지, 행위자 신원, 쓰기 방지를 증명하지 않습니다.

`agent_sessions`, `guard_events`, `prompt_captures`, `expected_writes`, `unrecorded_changes`, `session_watch_baselines`, `session_watch_observations`는 모두 프로젝트 로컬 행입니다. 서로 다른 프로젝트의 `state.sqlite` 데이터베이스 사이로 새면 안 됩니다.

호스트 관찰 조회나 projection이 `agent_sessions`, `guard_events`,
`session_watch_baselines`, `session_watch_observations`의 `latest` 또는 가장 최근 권한
사실을 도출할 때는 적용되는 정규 RFC 3339 timestamp를 strict parse하고 UTC instant로
정규화한 뒤 nanosecond 정밀도로 비교합니다. 저장 timestamp 텍스트, SQLite
`julianday()`, 행 삽입 순서, 불투명 record ID는 권한 순서 key가 아닙니다. 가장 큰 시각이
같은 행은 함께 최신인 집합이며, 불투명 ID로 그중 하나를 더 최신이라고 선택하면 안
됩니다.

함께 최신인 `guard_events`의 닫기 및 보안 관련 issue predicate는 전체 집합에서
보수적으로 합산합니다. 함께 최신인 event 중 하나라도 issue를 가지면 그 issue가 남으며
`allow` 또는 issue가 없는 다른 event가 이를 숨길 수 없습니다. Consumer가 권한으로 쓸
Agent Session, session-watch baseline, session-watch observation 하나를 요구할 때 서로 다른
후보 여러 개가 함께 최신이면 선택이 모호하므로, 좁은 담당 문서가 집합 기반 집계를
명시하지 않는 한 사용할 수 없는 담당 상태로 닫힌 상태에서 실패합니다.

`guard_installations`는 Runtime Home, Agent Connection, 선택적 프로젝트 범위별 설정 생명주기, 관찰된 훅 메타데이터, 호스트 역량을 기록합니다.

- `configured`는 파일이나 메타데이터가 설치되어 있지만 설정 생명주기 행 자체가 새 활성 관찰을 주장하지 않는다는 뜻입니다. 동일 identity 설정을 갱신하면 이전 관찰 메타데이터가 보존될 수 있고, 그 관찰은 현재 호스트·정책·단계·capability와 계속 일치할 수 있습니다. 따라서 소비자는 생명주기 상태에서 관찰 부재를 추론하지 않고 이 사실들을 평가합니다. `reload_required`는 호스트 reload와 현재 일치하는 관찰이 여전히 필요하다는 뜻이며, 보존된 관찰 메타데이터는 진단용으로만 남습니다.
- `active`는 기록된 프로젝트, Agent Connection, 호스트 종류, 통합 프로필, 정책 해시와 일치하는 유효한 호스트 훅을 Volicord가 관찰했다는 뜻입니다. OS 수준 집행이나 샌드박싱을 증명하지 않습니다.

#### 닫힌 호스트 훅 capability v2 기록

`guard_installations.host_capability_json`은 닫힌 내부
`volicord-host-hook-capability-v2` 계약을 사용합니다. 최상위 객체에는 다음 18개 구성원만
정확히 있어야 합니다.

- `schema`, `policy_hash`, `selected_profile`, `connection_intent`
- `final_output_authority_disclosure_implementation_available`,
  `native_host_output_adapter`,
  `native_host_output_adapter_config_verified`,
  `bash_shell_mutation_coverage`, `direct_file_write_matcher_coverage`
- `host_capabilities`, `required_hook_phases`, `missing_required_hooks`,
  `prompt_capture`
- `files`, `host_hook_commands`, `hook_root_resolution`, `hook_path_safety`,
  `commands`

`schema`는 정확히 `volicord-host-hook-capability-v2`이고, `policy_hash`는 비어 있지 않은
문자열이며, `selected_profile`은 `record` 또는 `detective`이고,
`connection_intent`는 `personal`, `shared`, `global` 중 하나입니다.
`native_host_output_adapter`는 `none`, `codex`, `claude-code` 중 하나입니다.
`final_output_authority_disclosure_implementation_available`은 어댑터가 `codex` 또는
`claude-code`일 때만 참이고, 그때 반드시 참입니다.
`native_host_output_adapter_config_verified=true`도 구현된 두 어댑터에서만 허용됩니다. 두
적용 범위 구성원과 `prompt_capture`는 boolean입니다. `record` capability의
`prompt_capture`는 `false`입니다.

`host_capabilities`는 `stdio_mcp`, `http_mcp`, `session_start_hook`,
`pre_tool_hook`, `post_tool_hook`, `user_prompt_submit_hook`, `stop_hook`,
`rule_file_support`, `project_local_configuration`이라는 정확한 boolean 구성원만 갖는 닫힌
객체입니다. `commands`는 `session_start`, `pre_tool`, `post_tool`,
`prompt_capture`, `stop`만 정확히 갖는 닫힌 map입니다. 각 값은 정확히
`{command,args}`이며, `command`는 비어 있지 않은 문자열이고 `args`는 문자열 배열입니다.

`required_hook_phases`와 `missing_required_hooks`는 중복이 없는 배열입니다.
`detective`에서 `required_hook_phases`는 정확히 `session_start_hook`,
`pre_tool_hook`, `post_tool_hook`, `user_prompt_submit_hook`, `stop_hook`이며,
`missing_required_hooks`는 이 집합의 부분집합입니다. `record`에서는 두 배열이 모두
비어 있습니다. 이 정규 저장 규칙은 [API 상태 스키마](api/schema-state.md)의
`GuardHealthSummary`가 닫힌 상태에서 실패하기 위해 적용하는 “부재 또는 명시적 나열”
완전성 projection과 구별됩니다.

각 `host_hook_commands[]` 항목은 `host_kind`, `phase`, `purpose`, `policy_key`,
`command_shape`, `command`, `args`, `expected_wrapper_path`,
`expected_phase_wrapper_path`, `root_resolution_basis`,
`hook_command_path_basis`, `cwd_independent`, `subdirectory_safe`,
`wrapper_resolution_status`, `verification`만 정확히 갖는 닫힌 객체입니다.
`verification`은 정확히 `{basis_verified_by,host_contract_source}`입니다.
`command_shape`은 `args=null`인 `shell_command_string` 또는 문자열 배열 `args`를 갖는
`exec_form`입니다. 루트 근거는 `git_work_tree` 또는 `claude_project_dir`, 경로 근거는
`git_root_runtime` 또는 `claude_project_dir`입니다. 래퍼 상태는 `ok`,
`relative_path_unsafe`, `wrapper_missing`, `wrapper_not_executable`,
`dispatch_missing`, `placeholder_unsupported`, `absolute_path_stale`,
`policy_hash_mismatch`, `host_output_mismatch`, `authority_mismatch`,
`metadata_missing` 중 하나입니다. 단계는 중복 없이 정확한 phase-to-policy-key 대응을
사용합니다. `purpose`는 `detective`에서 `detective_guard`, `record`에서
`final_output_authority_disclosure`이고 모든 항목은 비어 있지 않은 같은 호스트 종류를
사용합니다. `detective` 기록에는 필수 단계 중 누락 목록에 없는 단계만 정확히 있습니다.
`record` 기록에는 항목이 없거나 `stop_hook` 하나만 있습니다.

소유자 바인딩은 생성된 호스트 명령 자체도 요구합니다. Codex는 `args=null`인
`shell_command_string`만 정확히 사용합니다. Detective 명령은 정확히
`sh -c 'root=$(git rev-parse --show-toplevel) || exit $?; exec
"$root/.codex/hooks/volicord-dispatch.sh" <command-name>'`이며 마지막 인자는 해당 항목의
`policy_key`에서 대응한 명령 이름입니다. Record의 `stop` 명령은 dispatch 대신 정확한 단계
wrapper를 가리키며 단계 인자가 없습니다. Claude Code는 정확히 `exec_form`, 빈 `args` 배열,
`${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-<command-name>.sh`를 사용합니다. Volicord 직접
호출, 절대 wrapper 경로, 잘못된 단계 인자, 다른 셸 형식, 이전 형식은 정확한 v2 소유자
바인딩 명령이 아닙니다.

호스트 훅 명령이 없으면 `hook_root_resolution`과 `hook_path_safety`는 모두 `null`입니다.
그 밖에는 `hook_root_resolution`이 정확히
`{basis,all_cwd_independent,all_subdirectory_safe,overall_status,phases}`이고, 각
`phases[]` 항목은 정확히
`{phase,root_resolution_basis,hook_command_path_basis,cwd_independent,subdirectory_safe,wrapper_resolution_status}`입니다.
`hook_path_safety`는 정확히
`{overall_status,all_cwd_independent,all_subdirectory_safe,commands}`이고, 각
`commands[]` 항목은 정확히
`{phase,hook_command_path_basis,cwd_independent,subdirectory_safe,wrapper_resolution_status}`입니다.
두 배열은 `host_hook_commands`를 일대일로 projection해야 하며 집계 boolean, basis,
`ok` 또는 `relative_path_unsafe` 상태가 기초 항목과 정확히 같아야 합니다.

각 `files[]` 값은 닫힌 ownership-tagged union입니다. 모든 variant는 문자열 `kind`,
`path`, `status`, `content_hash`, `ownership`을 가집니다. `kind`는
`volicord_policy`, `git_info_exclude`, `host_mcp_config`, `host_hook_config`,
`host_hook_dispatch`, `host_hook_wrapper`, `host_rule_instruction`,
`agents_managed_block` 중 하나이고, `status`는 `planned_create`, `planned_update`,
`unchanged`, `created`, `updated` 중 하나입니다. `managed_json`은 공통 구성원 다섯 개만
가집니다. `managed_block`은 `managed_marker_start`, `managed_marker_end`만 정확히
추가합니다. `managed_json_projection`은 `managed_projection`,
`managed_projection_json`만 정확히 추가합니다. `managed_script`는 `managed_marker`와
boolean `executable_required`를 추가한 뒤, 정확히 `managed_script_role=codex_dispatch`,
`host_kind`, `phase`를 갖는 `host_hook_dispatch`이거나, 정확히
`managed_script_command`, `host_kind`, `phase`, `purpose`, `connection_id`,
`guard_installation_id`, `policy_hash`, `host_output`을 갖는 `host_hook_wrapper`입니다.

JSON 형태만으로는 권한이 충분하지 않습니다. Capability 프로필과 의도는 소유
`guard_installations` 행 및 Agent Connection과 일치해야 하고, 행과 연결의 호스트 종류도
일치해야 합니다. 어댑터, 모든 호스트 훅 명령의 호스트 종류, 모든 managed-script의 호스트
종류도 해당 소유 호스트와 일치해야 합니다. 저장소 인벤토리도 정확한 소유 프로젝트의
정규화된 절대 `repo_root`에 바인딩됩니다. `volicord_policy`는 정확히
`.volicord/policy.json`, `agents_managed_block`은 정확히 `AGENTS.md`, Claude Code
`host_mcp_config`는 정확히 `.mcp.json`입니다. 훅 설정은 Codex의
`.codex/hooks.json`, 개인 Claude Code 연결의 `.claude/settings.local.json`, 그 밖의
Claude Code 연결의 `.claude/settings.json` 중 해당 경로여야 합니다. 단계 wrapper는 정확히
`.codex/hooks/volicord-<command-name>.sh` 또는
`.claude/hooks/volicord-<command-name>.sh`, Codex Detective dispatch는 정확히
`.codex/hooks/volicord-dispatch.sh`, 규칙 지시는 정확히
`.codex/rules/volicord.rules` 또는 `.claude/rules/volicord.md`여야 합니다. 명령의
wrapper 경로도 같은 정규 경로를 가리켜야 합니다.

프로젝트 범위 capability에서는 최상위 `commands` 항목 다섯 개가 모두 비어 있지 않은 같은
실행 파일을 사용합니다. 인자는 정확히 `_hook`, 단계에 대응하는 명령 이름, `--repo`와
정규화된 소유 루트, `--connection`과 소유 연결 ID, `--guard-installation`과 소유 설치 ID,
`--host`와 공개 소유 호스트 레이블, `--integration-profile`과 소유 프로필에 이어 생성된
출력 쌍을 사용합니다. 해당 출력 쌍은 Detective의 각 어댑터에 대해
`--host-output codex` 또는 `--host-output claude-code`이고, 그 밖에는
`--output volicord-json`입니다. 정책 해시는 이 명령을 포함한 정책을 대상으로 계산하므로 이
정책 명령에는 `--policy-hash`가 없습니다. 모든 관리 wrapper 명령은 같은 실행 파일과 정확한
소유 좌표를 다시 사용해야 합니다. Detective wrapper는 호스트 출력 쌍 앞에 capability의
`policy_hash`를 추가하고, Record의 `stop` wrapper는 `_final-output`, 같은 소유 좌표,
capability의 `policy_hash`, 정확한 호스트 출력 쌍을 사용합니다. 생성기가 적용한 셸 단어
인용도 이 정확한 명령 텍스트의 일부이며 호환 대체 경로는 없습니다.

연결 worktree에서 해석한 공통 Git
디렉터리의 `info/exclude`가 worktree 밖에 있을 수 있으므로 `git_info_exclude`만
`repo_root` 아래 경로 규칙의 예외이며, 이 예외는 임의 경로 제거 권한을 부여하지 않습니다.
프로젝트가 없는 capability는 호스트 훅 명령이나 `git_info_exclude`를 포함한 저장소
인벤토리를 가질 수 없습니다. 필수 닫힌 최상위 `commands` map은 계속 형태 검증을 받지만
저장소 권한은 아니며 프로젝트 명령 사실로 소비해서는 안 됩니다. 운영 쓰기는 형태, 의미
관계, 소유자 바인딩이 불일치하면
거부합니다. 기존 불일치 행은 제한된 원시 검사를 통한 진단에는 계속 보이지만, Store,
Core, 최종 출력, 연결, Doctor의 사실 소비자는 guard-event 증거 이행을 포함해 capability
사실을 사용하지 않고 닫힌 상태에서 실패합니다.

제거된 `final_output_authority_disclosure_supported` boolean은 v2 구성원이 아닙니다. 현재
최종 출력 구성원 세 개는 구현, `native_host_output_adapter`, 생성 설정 감사 사실을 서로
구분합니다. 어느 것도 `HostFeatureSupportStatus`, 정확한 실제 호스트 증거,
`verified` projection 권한이 아닙니다. 닫힌 어느 수준에서든 구성원이 빠지거나 알 수 없는
구성원 또는 제거된 구성원이 있거나, v1 스키마이면 현재 capability 입력으로 유효하지
않습니다. 조회 경로는 v1을 v2로 decode하거나 이전 boolean을 새 필드로 복사하거나 거기서
지원 상태를 추론하면 안 됩니다. 지원되는 동일 신원 복구와 마이그레이션 거부 동작은
[관리 CLI](admin-cli.md)가 담당합니다.

`expected_writes`는 쓰기 상관관계를 결정적으로 기록합니다.

- 대기 행은 탐지형 도구 실행 전 경로가 프로젝트, 연결, 세션, 시간, 경로, `Task`, Change Unit, `active` 쓰기 티켓 좌표로 제한된 예상 쓰기 하나를 허용했다는 뜻입니다.
- 매칭된 행은 도구 실행 후 관찰을 그 예상 쓰기와 연결했다는 뜻입니다. 제품 정확성, 행위자 신원, OS 수준 쓰기 방지를 증명하지 않습니다.
- 매칭되지 않았거나 모호하거나 쓰기 티켓 범위를 벗어난 Product Repository 변경은 미해결 `unrecorded_changes` 행을 만듭니다.

미해결 `unrecorded_changes` 행은 관찰된 Product Repository 변경에 담당 문서가 정의한 조정이 아직 필요하다는 뜻입니다. 행을 해결하면 그 행을 보존하면서 로컬 해결 근거, 행위자 출처, 캡처 근거, 해결 시각, 선택적으로 연결된 사용자 행동 resolution을 기록합니다.

`session_watch_baselines`와 `session_watch_observations`는 탐지형 세션 단위 Product Repository 감시를 지원합니다. 샌드박스, 파일시스템 권한 경계, 쓰기 전 차단, 파일을 바꾼 주체나 이유에 대한 증명이 아닙니다.

- 기준선은 감시 가용성, 등록된 저장소 루트 또는 감시 경로 집합, 적용된 제외 항목, 결정적 스냅샷 다이제스트 메타데이터를 저장합니다.
- 관리 MCP 결속이 기준선을 구체화하면 `metadata_json`은 최상위 `client_name`과
  `client_version`을 모두 저장합니다. 성공한 `clientInfo.name`과 `clientInfo.version`의
  정확한 값이며, 각각 1바이트 이상 256 UTF-8 바이트 이하이고 공백이 아닌 문자를 하나
  이상 포함하며 제어 문자가 없는지 검증한 뒤 그 밖에는 정확한 문자열을 그대로
  보존합니다. 릴리스 기록기 좌표이며 클라이언트 신원 증명이나 사용자 권한이 아닙니다.
  호스트 종류, 실행 파일이나 probe 텍스트, 설정, 프로토콜 버전, 상수, 요청 메타데이터,
  다른 세션에서 어느 필드도 추론할 수 없습니다. 원본 initialize 요청, 그 밖의 initialize
  파라미터, 원본 프로토콜·세션·thread·turn·tool-call payload는 이 행에 저장하지 않습니다.
- 관리 기준선 하나는 클라이언트 쌍 하나만 보존합니다. 정확히 같은 쌍을 다시 관찰하는
  것만 멱등입니다. 기존 관리 기준선의 쌍이 없거나 일부만 있거나 다르면 결속 충돌이며
  성공한 관리 클라이언트 provenance가 아닙니다. 저장소 소비자는 값을 채우거나 교체하거나
  추론하지 않고 닫힌 상태로 실패해야 합니다.
- 기존 `metadata_json` 열을 사용합니다. 테이블, 열, 인덱스, trigger, 저장 profile 버전을
  추가하지 않으므로 저장소 DDL 계약은 바뀌지 않습니다.
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

스키마에는 프롬프트, 경로, 파일 본문, 오류 세부 정보, 비밀값, 사용자 행동 질문이나
캡처 양식, 선택 note, Evidence 관찰 summary 열이 없습니다. 크기가 제한된 도구 필드는 임의 요청 텍스트가 아니라 식별 정보만
받습니다. 내용을 담는 상세 추적은 지원하지 않습니다. 향후 상세 추적을 추가하려면
이 테이블을 넓히는 대신 별도의 명시적 opt-in, 보존, 가림 계약이 필요합니다.

진단 쓰기 때 보존 한도를 적용합니다. 7일보다 오래된 세션을 제거하고, 세션은 최대
64개, 세션별 이벤트는 최대 1,024개를 유지합니다. 시간 기반 보존은 타임스탬프 텍스트의
사전식 순서가 아니라 해석한 시간 값을 비교합니다. 이 데이터베이스의 부재, 손상,
비호환 버전, 읽기 전용 상태, 쓰기 실패는 MCP, guard, Core, User Channel 결과에 치명적이지
않습니다. 진단은 `state_version`, 증거, 보장 수준, 닫기 준비 상태, 사용자 행동, 권한 이벤트,
재실행 행을 갱신하면 안 되며 권한 번들 내보내기는 이 데이터베이스를 제외합니다.

### 현재 닫기 근거

현재 닫기 근거는 `tasks` 계열에 저장되는 Task 소유 현재 상태입니다. 성공한 종료 닫기 결과를 위해 저장되는 종료 닫기 요약과 다릅니다.

권위 있는 현재 `CurrentCloseBasis` 기록은 Task 소유 닫기 근거 좌표와 함께 해석하는 `tasks.close_basis_json`입니다.

기존 열린 Task는 종료 닫기 요약 JSON을 현재 닫기 근거로 자동 변환하지 않습니다. 현재 닫기 근거가 없다는 사실은 빈 생성 근거가 아니라 `tasks.close_basis_json`의 부재로 표현합니다. Change Unit 기록은 현재 `CurrentCloseBasis` 권한을 저장하거나 만족하지 않습니다.

저장된 사용자 행동 요청에는 닫힌 요청 본문과 `UserActionBasis`가 필요합니다. 해결된
요청에는 완전한 닫힌 해결 본문, 행위자 출처, 검증 근거, 보장 수준이 필요합니다. 이
사실이 빠진 행은 감사 호환 권한 기록이 아니라 유효하지 않은 소유자 상태입니다.

`user_action_channel_tokens.created_metadata_json`은 정확히
`{fallback_kind, delivery_surface, endpoint, form_digest}`로 strict decode되어야 합니다.
필수 값은 `fallback_kind=local_web_consent`,
`delivery_surface=model_invisible_user_surface`, `endpoint=/consent`이며 digest는 저장된 닫힌
요청에서 도출한 canonical form과 일치해야 합니다. Metadata가 누락됐거나 추가됐거나,
타입이 잘못됐거나, 값이 일치하지 않으면 사용할 수 없습니다. 특히 `delivery_surface`가
없는 수정 전 행은 수정된 코드에서 영구적으로 사용할 수 없습니다. 이때 local-web GET,
POST, token 소비, resolution은 form을 표시하지 않고 닫힌 상태로 실패하며 token,
프로젝트, UTC 하한, 사용자 행동 상태를 바꾸지 않습니다. 이런 행은 upgrade하지 않으며
대기 행동은 CLI 같은 다른 유효 User Channel로 계속 해결할 수 있습니다.

`user_action_resolutions` 행 하나가 존재해도 요청 근거가 계속 현재 상태일 때만 유효
`status=resolved`가 됩니다. stale 또는 superseded 근거는 이 불변 행보다 우선합니다.
Resolution 존재 자체는 승인이나 증거 뒷받침을 뜻하지 않습니다. 현재 권한 효력이 있는
선택 사용에는 선택한 저장 선택지, 파생 machine action/outcome, 적용 가능한 User Channel
provenance, 현재 근거가 필요합니다.
증거 관찰 사용에는 닫힌 해결 본문에 저장된 중첩 선택 대상, 정확한 정규 아티팩트 ref,
relevance 상태, 비어 있지 않은 관찰 요약, 현재 정확한 아티팩트 bytes가 필요합니다.
선택적 사용자 note는 비공개 설명 텍스트이며 rationale이나 권한이 아닙니다. 종류별 사실이
빠지면 유효하지 않은 담당 상태입니다.

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
| `user_action_requests.action_kind` | 일곱 판단 종류와 `evidence_observation` |
| `user_action_requests.basis_status` | `current`, `stale`, `superseded` |
| `user_action_requests.source_method` | `volicord.request_user_action`, `volicord.reconcile_changes` |
| `user_action_channel_tokens.status` | `pending`, `consumed`, `expired` |
| `project_continuity_records.kind` | `decision`, `obligation`, `known_limit`, `accepted_risk`, `constraint` |
| `project_continuity_records.status` | `active`, `superseded`, `closed` |
| `artifact_staging.status` | `staged`, `consumed`, `expired`, `discarded` |
| `artifacts.status` | `available`, `missing`, `integrity_failed`, `unavailable` |
| `artifacts.integrity_status` | `verified`, `corrupt` |
| `artifact_links.owner_record_kind` | `task`, `change_unit`, `run`, `user_action_request`, `user_action_resolution`, `evidence_summary`, `evidence_observation`, `evidence_producer`, `blocker` |
| `evidence_capture_intents.capture_kind`, `evidence_capture_receipts.capture_kind`, `evidence_producers.producer_kind` | `verified_command_execution`, `verified_tool_invocation`, `registered_connection_observation` |
| `evidence_capture_receipts.completeness` | `complete` |
| `evidence_capture_source_claims.source_claim_kind` | `host_invocation`, `guard_event`, `session_watch_observation` |
| `evidence_observations.source_kind` | `agent_report`, `connection_observation`, `external_tool`, `user_observation`, `reused_evidence`, `unverified_claim` |
| `evidence_observations.assurance_level` | `cooperative_report`, `registered_connection_observed`, `external_tool_result`, `user_observed`, `unverified` |
| `blockers.status` | `active`, `resolved`, `superseded` |
| `tool_invocations.status` | `committed` |
| `authority_events.operation_category`와 `tool_invocations.operation_category` | `read`, `agent_workflow`, `user_only`, `admin_local`, `local_recovery` |

공개 API 값을 반영하는 행은 [API 값 집합](api/schema-value-sets.md), 관련 스키마 담당 문서, 메서드 담당 문서와 정확히 맞아야 합니다. 이 문서는 `tasks.mode`, `tasks.lifecycle_phase`, `tasks.result`, `runs.kind`, `runs.status`, `evidence_summaries.status` 같은 필드의 공개 API 값을 다시 정의하지 않습니다. 공개 API 값은 [API 값 집합](api/schema-value-sets.md), [API 상태 스키마](api/schema-state.md), 메서드 담당 문서를 봅니다.

행에 저장된 `evidence_observations.source_kind` / `assurance_level` 조합은 enum 값만으로
충분한 강한 출처가 되지 않습니다. Core는 메서드 소유 파생 뒤에 이 조합을 기록하고,
요청 멤버를 신뢰하지 않고 확인된 호출에서 `observed_by_actor_source`를 가져옵니다. 현재
닫기 평가와 재사용 평가는 대상, `Task`와 Change Unit, 출처 실행 기록, 현재 범위 리비전과
기준선, 정확한 현재 출력 바이트, 타입이 지정된 producer 앵커, 별도의 relevance 평가를
다시 검증하고 입증되지 않으면 차단합니다. Capture intent와 완전한 receipt 경로는
authority-owned 외부 도구 또는 등록 연결 producer를 finalization할 수 있습니다. 그
정확한 앵커가 없는 직접 주장은 아티팩트 바이트를 사용할 수 있고 검증된 상태여도
협력적으로 남습니다. `user_observation` 행은 정확한 출력과
`relevance_status=supported`인 현재 `evidence_observation` `user_action_resolutions` 레코드와 일치 detail을 가리켜야
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
| 호스트 역량 검증 | V1 `metadata_json`은 엄격한 정규 `{}`만 허용합니다. 허용되는 모든 증거 좌표에는 전용 열이 있으며 bearer URL이나 token, prompt, transcript, screenshot, 원문 호스트 아티팩트, 비공개 운영자 데이터, 임의 또는 추가 구성원은 유효하지 않습니다. |
| 호스트 훅 설치 | 로컬 호스트 훅 설정 상태를 위한 닫힌 내부 `volicord-host-hook-capability-v2` JSON과 메타데이터입니다. V2는 구현 가용성과 설정 검증을 분리합니다. V1은 현재 입력으로 유효하지 않으며 추론 없이 init으로만 복구합니다. 이 기록은 typed 호스트 지원, 정확한 실제 증거, OS 집행 증명이 아닙니다. |
| `agent_sessions` | 프로젝트 범위 에이전트 세션에 대한 비권한 메타데이터. |
| `guard_events` | 로컬 호스트 판단 요청의 호스트 훅 대상 JSON, 결과 JSON, 메타데이터. |
| `prompt_captures` | 캡처된 프롬프트 기록의 비권한 메타데이터. 프롬프트 본문은 `null`을 허용하는 별도 텍스트 열입니다. |
| `expected_writes` | `detective` 예상 쓰기 상관관계를 위한 예상 경로 배열, 쓰기 티켓 ID 배열, 일치 경로 배열, 메타데이터. |
| `unrecorded_changes` | 미기록 Product Repository 변경의 관찰 경로 배열, 탐지 JSON, 해결 JSON, 메타데이터. 해결 JSON은 간결한 해결 근거, 캡처 근거, 해결 메서드, 선택적 연결 사용자 행동 resolution 참조를 저장합니다. 전체 민감 명령이나 프롬프트 내용을 저장하면 안 됩니다. |
| `session_watch_baselines` | 세션 감시 기준선을 위한 감시 경로 배열, 유효한 제외 배열, 스냅샷 항목 배열, 메타데이터. 관리 MCP 메타데이터에는 크기가 제한된 최상위 `client_name`과 `client_version` initialize 정체성만 포함하며 원본 initialize 또는 프로토콜·세션·thread·turn payload는 포함하지 않습니다. 스냅샷 항목은 경로, 종류, 크기, 해시 또는 건너뛴 이유 메타데이터만 저장하며 파일 내용을 저장하지 않습니다. |
| `session_watch_observations` | 세션 감시 관찰을 위한 관찰된 변경 경로 배열, 간결한 변경 요약 JSON, 스냅샷 항목 배열, 메타데이터. 스냅샷과 변경 요약은 행위자 식별, 의도, 제품 정확성, 닫기 준비 상태를 증명하지 않습니다. |
| `tasks` | 구체화 요약, 제한된 목록, 자율성 경계, carry-forward disposition, 현재 닫기 근거, 종료 닫기 요약, 생명주기 요약. Acceptance policy, work phase, lineage edge identity는 전용 열을 사용하고 수락 기준과 보충 증거 주장은 각 정규 관계형 테이블을 사용합니다. |
| `change_units` | 범위 요약, 제한된 목록, 쓰기 근거 요약, 선택적 효과 계약 데이터, 생명주기 지원 데이터. |
| `user_action_requests` | 폐쇄형 요청, required-for 대상, Core 파생 근거, 요청 actor, 정확한 원천 메서드/idempotency 관계, expiry. |
| `user_action_resolutions` | 폐쇄형 불변 resolution 본문, channel kind와 크기가 제한된 visible-ASCII submission ID, 파생 actor/verification/assurance, Core 캡처 시각, 선택적 비공개 note, choice 또는 Evidence 관찰 detail. Local-web 행은 파생 digest identity만 저장하며 원문 token은 저장하지 않습니다. |
| `user_action_channel_tokens` | 요청 결합 local-web hash-token lifecycle, capture basis, 폐쇄형 delivery-surface 생성 metadata. |
| `project_continuity_records` | 오래 유지하는 프로젝트 맥락을 위한 적용 대상 경로, 적용 대상 참조, 원천 참조, 아티팩트 참조, 대체된 참조, 검토 트리거, 비권한 메타데이터. |
| `write_tickets` | 쓰기 티켓 시도 범위와 비권한 메타데이터. |
| `runs` | 요약, 관찰된 변경, 증거 갱신, 쓰기 티켓 효과 데이터, 비권한 메타데이터. |
| `artifact_staging` | 스테이징된 아티팩트 데이터, 안전 메타데이터, 비권한 메타데이터. |
| `evidence_capture_intents` | 정확한 target/capture JSON, command/tool input digest 또는 Core가 파생한 connection source-selector digest, 예상 outcome, 등록 session 및 Git workspace 근거, actor/connection provenance, 만료, 비권한 메타데이터. Connection capture JSON에는 미래 source ID, observation timestamp, snapshot digest, raw-event digest가 없습니다. |
| `evidence_capture_receipts` | 정확한 예상/관찰 outcome, source ref, 한계, 크기가 제한된 safe receipt JSON과 digest/size, 메타데이터의 등록 source 좌표, 비권한 메타데이터. Safe receipt는 redacted이며 raw command, environment, stdout, stderr, tool input, tool response, secret, 크기 제한 없는 host payload를 포함하지 않습니다. |
| `artifacts` | 보존, 생산자, 비권한 메타데이터. |
| `artifact_links` | 비권한 메타데이터. |
| `evidence_summaries` | 결과 권한 상태 버전, 증거 범위, 뒷받침 참조, 공백 참조, 비권한 메타데이터. |
| `evidence_observations` | 증거 관찰 하나의 도구 메타데이터, Core 기록 입력 ref, 권한 효력이 없는 `SourceRef` JSON, 출력 아티팩트 ref, 한계, 타입이 지정된 Core 파생 producer/relevance 권한 메타데이터입니다. `source_refs_json`은 권한을 만들지 않습니다. |
| `evidence_producers` | 엄격한 canonical `EvidenceProducer` JSON과 relational one-to-one 권한 key 및 verification-basis 메타데이터. |
| `blockers` | 차단 사유 소유 참조, 관련 참조, 세부 정보, 비권한 메타데이터. |
| `authority_events` | 커밋된 Core 권한 변경의 이벤트 페이로드. |
| `tool_invocations` | 재실행과 조회할 수 있는 정확한 동작 결과 페이지에 쓰는 변경 불가능한 커밋 응답, 그리고 정확한 재실행 호환성에 사용하는 검증된 행위자 출처, 작업 범주, 선택적 검증 근거, 선택적 정규 Git 작업 공간 맥락 JSON. |

`Task`와 Change Unit 구체화 JSON은 간결한 요약과 제한된 목록만 저장합니다. 추가 영속 기록 계열을 만들지 않습니다.

## 관련 담당 문서

- [저장 효과](storage-effects.md): 어떤 메서드 분기가 기록을 만들거나, 바꾸거나, 관찰하거나, 건드리지 않는지 정의합니다.
- [저장소 DDL](storage-ddl.md): 기준 SQLite 테이블 형태, 인덱스, 외래 키, 제약, 기준 SQL 원본을 정의합니다.
- [아티팩트 저장소](storage-artifacts.md): 아티팩트 스테이징, 승격, 연결, 본문 읽기, 보존, 무결성 생명주기를 정의합니다.
- [저장소 버전 관리](storage-versioning.md): 상태 버전 시계, 정규 Core UTC 시계와
  영속 하한, 멱등성, 재실행, 이벤트, 잠금, 호환되지 않는 저장소 처리를 정의합니다.
- [Agent Connection](agent-connection.md): Agent Connection, Connection Projects, 모드로 제한되는 MCP 도구 접근, User Channel 경계를 정의합니다.
- [API 코어 스키마](api/schema-core.md), [API 상태 스키마](api/schema-state.md), [API 아티팩트 스키마](api/schema-artifacts.md), [API 사용자 행동 스키마](api/schema-user-action.md), [API 판단 스키마](api/schema-judgment.md), [API 값 집합](api/schema-value-sets.md): API 형태와 공개 API 값을 정의합니다.
- [API 메서드](api/methods.md)와 메서드 담당 문서: 기록을 사용하는 공개 메서드 동작을 정의합니다.
- [런타임 경계](runtime-boundaries.md): `Product Repository`, Volicord 설치 또는 런타임 프로세스, `Volicord Runtime Home` 위치 경계를 정의합니다.
- [상태 보기 권한 참조](projection-and-templates.md)와 [템플릿 본문](template-bodies.md): 읽는 시점의 상태 보기 권한과 렌더링된 템플릿 본문을 정의합니다.
- [보안](security.md): 보안 경계와 보장 수준을 정의합니다.
