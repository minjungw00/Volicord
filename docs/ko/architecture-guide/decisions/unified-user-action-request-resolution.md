# 통합 사용자 행동 요청과 해결

## 맥락

Volicord는 이전에 대기 판단을 `user_judgments`에 저장하고 판단 전용 host 및 CLI
경로로 답변을 받았으며, 사용자 증거 관찰은 별도 CLI-only 메서드와
`user_evidence_observations`로 즉시 기록했습니다. 최종 수락과 잔여 위험 수락은 판단
종류였지만 사용자 관찰에는 대기 요청, host-native 캡처 폼, 공통 채널 생명주기가
없었습니다.

## 결정

지원되는 모든 사용자 행동에 Core 소유 `UserActionRequest` 생명주기 하나와 변경
불가능한 일대일 `UserActionResolution` 하나를 사용합니다. 공개 메서드는
`agent_workflow` 생성용 `volicord.request_user_action`과 `user_only` 해결용
`volicord.resolve_user_action`입니다. 요청 메서드만 MCP tool입니다.

MCP tool은 명시적인 중첩 `request.operation=create|resume` union을 사용합니다.
`create`는 공개 mutation을 한 번 실행합니다. `resume`은 기존 직접 요청을 지정하고 같은
Agent Connection 접근 범위에서 정확한 원래 요청 결과를 읽습니다. 새 요청이나 authority
event를 만들지 않으며 행동을 해결하지도 않습니다. 어느 분기 뒤든 Core는 한 SQLite 읽기
snapshot에서 유효 상태와 agent-safe resolution을 다시 읽습니다. MCP 결과는 이
projection의 state version과 관찰 시각을 과거 요청 결과 및 이후의 일반 authority
receipt와 분리해 보존합니다.

요청 생성과 해결은 각각 정규 준비 동작 시각 샘플 하나를 상태, expiry, 의미 있는
timestamp에 사용합니다. 이후의 Core 커밋 timestamp는 이 샘플을 다시 쓰지 않습니다.
로컬 web token 검증은 같은 프로젝트 시계의 반열린 구간
`created_at <= now < expires_at`을 사용합니다.

닫힌 행동 종류는 일곱 판단 종류와 `evidence_observation`입니다. Tagged payload는 판단
action/outcome 권한과 관찰 relevance를 분리합니다. Core가 캡처 폼, 근거, 유효 상태,
후보 집합, 만료 결과를 한 번 파생합니다. MCP elicitation, prompt capture, 로컬 web
consent, CLI inbox 어댑터는 같은 폼을 렌더링하고 제출합니다. 일반 채널 어댑터는 크기가
제한되고 replay에 결속된 `channel_submission_id`를 제공하며 후보나 사용자 권한을 다시
계산하지 않습니다. Local-web consent에서는 Core가 프로젝트, 요청, bearer-token
credential, 예상 connection, 폐쇄형 canonical 완료 metadata에 대한 digest-only identity
도출과 정확한 재검증을 담당합니다. Mutation replay identity는 원문 token이 아니라 같은
connection과 metadata를 결합한 domain-separated token digest를 사용합니다.

지속 저장소는 `user_action_requests`, 닫힌 tagged evidence-observation 본문을
담는 변경 불가능한 일대일 `user_action_resolutions`, 요청에 결속된 로컬
채널 token을 사용합니다. 요청 행은 정확한 원천 메서드와 idempotency key도 저장합니다.
부분 고유성 규칙은 직접 요청에 원천 하나만 허용하면서 reconciliation 커밋 하나가 요청을
여러 개 만드는 것은 허용합니다. 기존 `user_judgments`,
`user_evidence_observations`, 판단 결속 token 형태는 제거합니다.

## 불변 조건과 결과

- Agent Connection은 요청을 만들 수 있지만 해결할 수 없습니다.
- 사용자는 검증된 `User Channel`에서 저장된 후보만 선택합니다.
- 해결 종류, 요청 종류, 근거, Task, Change Unit, scope, baseline, 대상, 아티팩트
  bytes, 만료, 채널 binding을 하나의 원자적 커밋 전에 다시 검증합니다.
- 요청 하나에는 변경 불가능한 해결이 최대 하나만 있습니다. 동시 제출이나 충돌
  replay가 해결을 갈라놓을 수 없습니다.
- Local-web 중복 제출은 token credential, 프로젝트/요청 좌표, 예상 connection, 폐쇄형
  완료 맥락, submission identity, canonical resolution이 모두 일치할 때만 원래의 안전한
  완료를 반환합니다. 손으로 만든 identity나 변경된 binding은 그 replay를 열 수 없습니다.
- `resolved`는 해결이 있다는 뜻이지 판단이 수락되었거나 관찰이 주장을 뒷받침한다는
  뜻이 아닙니다. 종류별 payload가 그 의미를 보존합니다.
- 관찰 캡처 시각은 Core 시각이며 에이전트와 채널 폼이 제출하지 않습니다.
- 요청과 해결 계획은 각각 정규 Core 시각 샘플 하나를 다시 사용하며 어댑터와 호스트
  timestamp가 시간 권한을 제공하지 않습니다.
- 판단 캡처는 저장 선택지 ID와 선택적 note만 제출합니다. Core는 저장 상태에서
  구조화된 권한 사실을 파생하며 사용자 rationale을 만들어 내지 않습니다.
- 해결 어댑터는 요청 시점 state version을 추측하지 않습니다. Core가 preflight에서
  현재 상태를 고정하고 transaction이 preflight-to-commit race를 감지합니다.
- 사용자 증거 해결은 producer/relevance 레코드일 뿐이며 이후 `record_run`이 참조해야
  증거 coverage가 사용할 수 있습니다.
- 자유 형식 사용자 텍스트는 user-only 결과에 남습니다. agent-safe MCP projection은
  구조화된 선택 결과와 ref만 노출합니다.
- 채널을 넘나드는 연속 작업은 byte 단위로 정확한 원래 Agent Workflow 결과를 재생하고
  재생 여부를 표시합니다. 이어 별도로 관찰한 agent-safe 현재 projection을 붙이며 새
  idempotency key를 만들거나 정확한 user-only resolution 응답을 노출하지 않습니다.
- 현재 상태, 안전한 resolution, 과거 resolution 파생 ref는 하나의 Core/Store
  snapshot에서 읽습니다. 관찰 state version과 시각을 함께 내보내므로 이후 더 최신
  authority receipt가 만들어져도 freshness가 명시적입니다.

## 호환성과 migration

첫 major 이전의 의도된 clean break입니다. 기존 세 공개 메서드와 요청/응답 스키마,
MCP 이름, CLI `inbox answer`와 직접 observe 형태, record kind, 테이블, 별칭을 유지하지
않습니다. 공개 계약 batch 버전은 `0.8.0`입니다. 중첩 MCP create/resume union은 같은
clean-break batch에서 이전의 평면 create-only 인자를 대체하며 모호한 평면 호환 decoder를
두지 않습니다. 잔여 위험 coverage의 정확한 권한
ref 이름도 `accepted_by_user_action_resolution_refs`로 통일합니다.

저장소 profile은 `baseline_sqlite_v5`입니다. v4-to-v5 변환이나 legacy 읽기 경로는
없습니다. v4 Runtime Home은 호환되지 않으며 다시 만들어야 합니다.

## 거부한 대안

- `request_user_observation`만 추가하면 판단과 관찰 생명주기가 계속 분리되어 같은 근본
  원인이 남습니다.
- 기존 메서드를 wrapper로 유지하면 첫 major 이전에 경쟁하는 stable 경로와 legacy
  호환 코드가 생깁니다.
- 어댑터가 폼을 만들거나 사용자 답변을 추론하게 두면 권한이 Core에서 분리되고
  projection drift가 생길 수 있습니다.
- 관찰 해결을 untyped JSON으로만 저장하면 증거 재사용에 필요한 대상 관계 확인을
  잃습니다.
- 이후 retry를 새 create 호출로 처리하면 어댑터가 새로 생성한 idempotency key에 의존해
  사용자 요청을 중복할 수 있습니다.
- 공개 요청 내용에서 대체 idempotency key를 결정적으로 만들면 의도적으로 구분한 요청을
  합치며, 다른 채널이 해결한 뒤 정확한 과거 결과를 식별하지도 못합니다.
- 평면 create 필드 옆에 선택적 resume ID를 두면 폐쇄형 operation union 대신 혼합되거나
  모호한 요청을 허용하게 됩니다.

## 구현과 테스트

공유 형태는 `volicord-types`, 상태·근거·캡처 폼 평가는 Core, 원자적 요청-해결-token
transaction은 Store가 맡고 MCP와 CLI는 어댑터로 남습니다. 지속 테스트는 strict
tagged decoding, 모든 행동 종류, 후보와 크기 경계, actor 거부, stale/만료 경계,
멱등 channel submission, 동시 해결, rollback, 채널 fallback 동등성, 비공개 텍스트
redaction, 이후 `record_run` 증거 경로를 검증합니다.

정확한 동작은 [Core 모델](../../reference/core-model.md), [API 사용자 행동
스키마](../../reference/api/schema-user-action.md), [사용자 행동 요청
메서드](../../reference/api/method-request-user-action.md), [사용자 행동 해결
메서드](../../reference/api/method-resolve-user-action.md), [저장소 버전
관리](../../reference/storage-versioning.md)가 담당합니다.
시계 선택 근거는 [정규 Core UTC 시계](canonical-core-utc-clock.md)에 기록합니다.
