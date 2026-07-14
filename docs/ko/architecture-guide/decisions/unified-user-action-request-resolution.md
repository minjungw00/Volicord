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
Agent Connection 접근 범위에서 정확한 원래 agent-safe 요청 결과를 읽습니다. 새 요청이나
authority event를 만들지 않으며 행동을 해결하지도 않습니다. 저장된 Agent Workflow
결과는 `AgentSafeUserActionRequestSummary`만 담고 전체 요청, inbox 항목, 캡처 폼을 담지
않습니다. 어느 분기 뒤든 Core는 한 SQLite 읽기 snapshot에서 유효 상태와 agent-safe
resolution을 다시 읽습니다. MCP 결과는 이 projection의 state version과 관찰 시각을
과거 요청 결과 및 이후의 일반 authority receipt와 분리해 보존합니다.

User Channel presentation은 별도 projection입니다. Native elicitation은 프로토콜이
담당하는 사용자 입력 표면을 사용합니다. Local-web bearer URL은 초기화된 클라이언트가
loopback listener를 사용할 수 있고 초기화된 client가
`params.capabilities.experimental["io.volicord/user-channel"].model_invisible_user_surface`에
정확한 boolean `true`를 선언했을 때만 발급할 수 있습니다. 서버는 모델 맥락 밖에 남는다고 약속된
namespaced 최상위 tool-result `_meta` handoff에만 URL을 넣습니다. Capability가 없거나
false이거나 잘못된 형태이면 token을 발급하지 않고 CLI inbox를 선택합니다. User Channel
credential이나 credential을 포함한 URL은 Agent Connection `content`,
`structuredContent`, 호환·진단 text, 정확한 replay, operation-result byte에 들어가면 안
됩니다.

Loopback browser 제출에는 독립적인 전송 방어를 추가합니다. 모든 `POST /consent`는
listener 자체 origin과 scheme 및 authority가 같은, 문법적으로 유효한 `Origin` 하나만
담아야 합니다. 서버는 form body를 읽거나 token을 조회하거나 Core를 호출하기 전에
누락·빈 값·`null`·잘못된 형식·쉼표 결합·중복·다른 origin을 거부합니다. `GET
/consent`는 `Origin`을 요구하지 않지만, 제공된 origin은 같은 정확한 검증을 통과해야
합니다. 이 gate는 browser cross-origin 제출에 대한 심층 방어이며 사용자 인증도,
모델 비가시 credential 경계의 대체물도 아닙니다.

Listener 준비 상태는 시작 시점 marker가 아니라 실시간 process 상태입니다. Listener와
adapter는 준비 handle 하나를 공유하며 accept-loop 실패는 loop가 끝나기 전에 이를
unavailable로 표시합니다. 하나의 유효 가용성 evaluator가 Core projection 또는 adapter의
채널 선택 지점마다 이 상태와 협상된 capability를 결합합니다. Handoff materialization은
응답 예산 검증 뒤 협상된 capability를 이 evaluator에 전달하고, token 삽입과 handoff 구성이
끝날 때까지 준비된 listener의 공유 발급 lease를 유지합니다. Listener 무효화는 배타 lease를
획득합니다. 무효화가 먼저 순서화되면 `_meta`, token, clock 효과 없이 CLI로 fallback합니다.
발급이 먼저 순서화되면 이미 발급된 것으로 보며 이후 listener가 실패해도 제한된 token TTL을
유지합니다.

요청 생성과 해결은 각각 정규 준비 동작 시각 샘플 하나를 상태, expiry, 의미 있는
timestamp에 사용합니다. 이후의 Core 커밋 timestamp는 이 샘플을 다시 쓰지 않습니다.
로컬 web token 검증은 같은 프로젝트 시계의 반열린 구간
`created_at <= now < expires_at`을 사용합니다.

닫힌 행동 종류는 일곱 판단 종류와 `evidence_observation`입니다. Tagged payload는 판단
action/outcome 권한과 관찰 relevance를 분리합니다. Core가 캡처 폼, 근거, 유효 상태,
후보 집합, 만료 결과를 한 번 파생합니다. Native MCP elicitation, 협상된 모델 비가시
local web, CLI inbox를 포함한 별도 검증 User Channel renderer는 같은 form을 렌더링하고
제출합니다. Agent에 보이는 prompt capture는 안전한 대기 요약과 일반 CLI 안내만 받으며
완전한 form 표면이 아닙니다. 일반 채널 어댑터는 크기가
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
- Agent Connection은 대기 사용자 행동의 정규 요청 요약만 받습니다. 완전한 요청, 질문,
  맥락, 후보, 캡처 폼, 캡처 경로, 명령, URL, User Channel credential은 User Channel
  projection에만 남습니다.
- Listener 존재는 전달 capability가 아닙니다. Listener와 협상된 모델 비가시적 host
  표면을 모두 사용할 수 있을 때만 local web을 사용할 수 있으며 capability 누락이나
  협상 실패는 token 발급 없이 CLI로 fallback합니다.
- Local-web browser 제출은 request body, token, replay, resolution을 처리하기 전에
  정확한 same-origin 전송 gate를 통과해야 합니다. 거부는 token이나 Core 상태에 효과를
  남기지 않으며 같은 유효 token을 올바른 origin에서 다시 시도할 수 있습니다.
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
- 채널을 넘나드는 연속 작업은 byte 단위로 정확한 원래 안전 Agent Workflow 결과를
  재생하고 재생 여부를 표시합니다. 이어 별도로 관찰한 agent-safe 현재 projection을
  붙이며 새 idempotency key를 만들거나 완전한 User Channel 폼 또는 정확한 user-only
  resolution 응답을 노출하지 않습니다.
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

안전한 요청 projection과 모델 비가시적 handoff는 같은 미출시 `0.8.0` clean-break batch
안의 보정이므로 별도 SemVer나 저장소 profile을 만들지 않습니다. DDL 변경도 없습니다.
저장된 모든 공개 메서드 결과는 replay 또는 operation-result paging 전에 현재의
닫힌 결과 타입과 strict 대조합니다. 따라서 보정 전 전체 form 형태를 사용하는
저장 요청 결과, `pending_user_action_inbox_items`를 담은 닫기 결과, 폐기된
`StateSummary` 대기 행동 projection을 담은 결과는 모두 사용할 수 없습니다.
이 결과를 다시 쓰거나 변환하지 않습니다. 보정 전 local-web
token에는 필수 `delivery_surface=model_invisible_user_surface` 생성 marker가 없으므로
수정된 코드에서는 영구적으로 사용할 수 없습니다. GET과 POST는 표시나 효과 없이
닫힌 상태로 실패합니다. 그 행은 upgrade하지 않으며 대기 행동은 CLI 같은 다른
유효한 User Channel로 계속 해결할 수 있습니다. 안전하지 않은 projection을 복구하는
호환 별칭은 두지 않습니다.

같은 replay 보정은 일반 value normalization 전에 중복 원문 JSON object member,
non-result 분기, committed state version 불일치를 거부합니다. Preflight replay, commit
transaction 안에서 발견한 replay, resume, operation-result paging에 모두 적용합니다. 이런
기존 행은 사용할 수 없으며 normalize, redact, rewrite, upgrade하지 않습니다. DDL, 저장
프로필, 호환 decoder, 별도 SemVer 변경을 추가하지 않습니다.

Browser `POST /consent`에 same-origin `Origin`을 요구하는 것도 같은 미출시 batch 안의
보정입니다. 공개 메서드 스키마, DDL, 저장 프로필을 바꾸지 않으며 별도 SemVer 변경이
필요하지 않습니다. Browser form 제출은 이미 `Origin`을 제공합니다. 이를 생략하던
non-browser 호출자는 이제 전송 담당 `403 ORIGIN_NOT_ALLOWED` 응답을 받으며, 계약에 맞는
same-origin 요청이나 다른 User Channel을 사용해야 합니다. 거부는 token 조회 전에
일어나므로 다른 면에서 유효한 token을 소비하거나 무효화하지 않습니다.

시작 뒤 listener 저하 추적은 process 내부 adapter 보정입니다. 공개 schema, DDL, 저장
프로필, migration, SemVer 변경을 추가하지 않습니다. 대기 요청은 CLI로 계속 복구할 수
있으며 이미 발급된 token은 기존의 제한된 TTL과 검증 규칙을 유지합니다. 기존 공개
programmatic adapter builder는 source compatibility를 유지하지만 fail-closed shim으로
바뀝니다. Listener가 소유한 guard 없는 base URL은 더 이상 local web을 활성화하지 않습니다.
지원되는 process entry point는 managed 경로를 통해 local-web 동작을 유지합니다. 이 동작
보강은 현재 미출시 `0.8.0` 수정 batch에 포함되며 별도 SemVer 변경을 추가하지 않습니다.
외부 tracked-listener API는 자체 공개 lifetime 계약이 생길 때까지 보류합니다.

수정된 Store와 adapter fence는 수정 전 process를 교체하거나 재시작한 뒤에만 적용됩니다.
계속 실행 중인 이전 process는 이미 발급한 raw credential을 보유할 수 있고 기존 token
TTL로만 제한됩니다. 수정된 fence에 의존하기 전에 이전 process를 교체해야 합니다.

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
- Fallback text에서 bearer 문자열만 제거하면 full-detail 결과, status/close projection,
  resume, 정확한 operation-result 조회를 통한 전체 User Channel 폼 접근이 남습니다.
- Listener 시작을 안전한 host 표면의 증거로 취급하면 모델에게 노출할 수 있는
  클라이언트에 권한 효력이 있는 bearer credential을 발급하게 됩니다.
- 시작 때 만든 listener context를 accept loop 실패 뒤에도 유지하는 방안은 죽은 User
  Channel을 위한 credential 포함 handoff를 발급할 수 있으므로 거부했습니다.
- Atomic 준비 flag를 확인한 뒤 token을 삽입하는 방안은 두 동작 사이에 무효화가 일어날 수
  있으므로 거부했습니다. 발급 lease가 대신 하나의 순서 지점을 제공합니다.
- 호출자가 제공한 base URL이나 lifetime assertion을 준비 상태로 취급하는 방안은 listener가
  계속 소유되고 호출 가능한지 증명하지 못하므로 거부했습니다.
- 일반 MCP content에 URL을 넣고 사용하지 말라고 안내하면 채널 경계를 강제하지 않고
  권한 우회를 그대로 유지합니다.
- `Origin`이 없을 때 검사를 선택적으로 적용하는 방안은 browser 방어가 공격자가 통제할
  수 있는 header 생략에 의존하게 되므로 거부했습니다.
- Browser 요청 보호에 bearer token만으로 충분하다고 보는 방안은 token 보유와
  same-origin 요청 출처가 서로 다른 방어이고 어느 쪽도 모델 비가시 전달 불변 조건을
  대신하지 못하므로 거부했습니다.
- 구체적인 결과를 검사하기 전에 저장 byte를 일반 JSON tree로 parse하는 방안은 중복
  member가 축약되어 안전하지 않은 원문 byte가 현재의 안전한 형태처럼 보일 수 있으므로
  거부했습니다.
- 공통 rejected 또는 dry-run 응답 분기를 replay 행으로 허용하는 방안은 replay 저장소가
  commit된 non-dry-run 메서드 결과만 담으므로 거부했습니다.

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
