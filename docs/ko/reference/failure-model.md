# 실패 모델

이 문서는 Core, Store, 어댑터, 전송, 관리 명령, 진단에 공통으로 쓰는 제품 전체
실패 범주를 담당합니다. 정확한 범주 식별자와 의미 경계, 영속 상태 처리, 실패를
합성 값이나 기본 성공 값으로 바꾸지 않는 규칙을 정의합니다.

특정 API 응답 envelope, 전송 오류 형태, 메서드 저장 효과, 도메인별 사유 목록,
repair 명령은 정의하지 않습니다. 그 세부사항은 영향을 받는 표면의 집중 담당
문서에 남습니다.

<a id="surface-stability"></a>
## 표면 안정성

다섯 범주의 의미, machine-readable identifier, 영속 권한 및 정책 데이터의
fail-closed 규칙, 기본값 합치기 금지 규칙은 stable 계약입니다. 사람이 읽는 표시
문구와 도메인별 진단 세부사항은 집중 담당 문서가 달리 정하지 않는 한
diagnostic입니다.

## 정식 실패 범주

모든 기계 판독 실패 분류는 아래의 정확한 주 범주 식별자 중 하나를 사용합니다.

| 범주 | 식별자 | 의미 |
|---|---|---|
| `Rejected` | `rejected` | 요청 형태 또는 필수 맥락이 구조적으로 유효하지 않아 정책 평가나 성공 동작 분기로 진행하지 못했습니다. |
| `NotAllowed` | `not_allowed` | 구조적으로 유효한 요청과 완전한 필수 맥락이 정책 평가에 도달했지만 정책이 요청한 동작을 허용하지 않았습니다. |
| `Unavailable` | `unavailable` | 동작, 보조 capability, 필수 조회를 현재 수행할 수 없지만 사용 가능한 데이터가 영속 계약 손상을 확정하지는 않습니다. |
| `Degraded` | `degraded` | 핵심 동작은 계속할 수 있지만 명시적으로 식별된 검증, 진단, 보조 정보 구성 요소가 불완전합니다. |
| `Corrupt` | `corrupt` | 영속되었거나 신뢰되는 담당 데이터가 선언된 스키마, type, canonical encoding, 필드 간 계약을 위반합니다. |

기계 판독 결과나 진단은 정확한 범주 식별자를 담아야 합니다. 도메인이 같은 범주
안에서 원인을 구분한다면 도메인 담당 문서가 정의한 사유 식별자도 담아야 합니다.
사람이 읽는 문구만으로는 범주나 사유가 되지 않습니다. 사유 식별자가 범주의
의미를 암묵적으로 바꾸면 안 됩니다.

## 범주 선택 경계

`Rejected`와 `NotAllowed`는 정책 평가 여부로 구분합니다. 필수 맥락이 없거나
유효하지 않으면 `Rejected`이며 이를 정책상 비허용으로 표현하면 안 됩니다.
`NotAllowed`는 구조적으로 유효한 요청, 해석된 필수 맥락, 실제 정책 판단을
요구합니다. 메서드 담당 문서가 커밋되는 비허용 결과를 정의할 수는 있지만 범주
자체가 커밋을 허용하지는 않습니다.

`Unavailable`과 `Degraded`는 핵심 동작을 진실하게 계속할 수 있는지로 구분합니다.
필수 동작이나 조회를 수행할 수 없으면 `Unavailable`입니다. 핵심 동작은
유효하지만 이름이 붙은 보조 검증이나 정보 출처가 불완전하면 `Degraded`이며,
빠진 부분과 그 영향을 계속 보이게 해야 합니다.

지원되는 영속 또는 신뢰 계약을 따른다고 표시된 데이터가 그 계약을 위반하면
`Corrupt`입니다. 아직 영속 담당 상태가 되지 않은 신뢰할 수 없는 경계 입력이
잘못되었거나 알려지지 않은 경우는 `Corrupt`가 아니라 `Rejected`이며 지원되는
형태로 추정하지 않습니다.

활성 연결 검증은 구성된 Codex 실행 파일을 찾고 version 명령을 실행하며 동작 probe를
아래에서 정의하는 다섯 가지 상태 모델로 보고합니다. 실행 파일을 찾거나 실행하지 못한
경우 일반 실패 범주 경계에서는 `Unavailable`, 연결 보고서에서는 실패한
`host_executable` check입니다. 관찰한 version이 달라지면 운영 관찰을 갱신합니다.

관리 connection command report는 필수 check가 하나 이상 실패했거나 blocked인 typed
운영 결과에 `failed`를 사용합니다. Host 관찰이 pending이면 `action_required`이며
`Degraded`, stale/broken 공개 상태, 예상하지 못한 런타임 오류로 표시하지 않습니다.
사용법 오류와 예상하지 못한 런타임 또는 직렬화 실패는 성공이나 action-required
보고서로 꾸미지 않고 CLI 오류 채널에 남깁니다.

어느 범주도 다른 범주의 의미를 포함하지 않습니다. 특히 다음 규칙을 지킵니다.

- `Unavailable`은 비어 있는 성공 결과가 아닙니다.
- `Degraded`는 완전한 검증이나 제한 없는 성공 상태가 아닙니다.
- `Corrupt`는 선택적인 값의 부재가 아닙니다.
- `NotAllowed`는 구조적 거부가 아닙니다.

## 영속 권한 및 정책 데이터

선언된 계약에 따라 decode하고 검증할 수 없는 영속 권한 또는 정책 데이터는
`Corrupt`이며 실패 시 닫히도록 처리합니다. 그 데이터에 의존하는 동작은 권한을
도출하거나, 정책 판단을 내리거나, 성공 효과를 기록하거나, 종속 담당 상태를
변경하기 전에 중단해야 합니다.

타입이 정해진 영속 JSON은 선언된 전체 type으로 decode해야 합니다. 문법 실패,
잘못된 최상위 형태, 알 수 없는 closed variant, 필수 필드 누락, 담당 문서가 추가
필드를 거절하는 곳의 추가 필드, 필드 간 invariant 위반은 모두 `Corrupt`입니다.
어느 경우도 빈 배열, 빈 객체, 값 부재, 호스트별 기본값으로 바꾸면 안 됩니다.

표시 전용 또는 보조 데이터가 없다고 해서 자동으로 손상은 아닙니다. 집중 담당
문서가 이를 `Unavailable` 또는 `Degraded`로 분류하고 핵심 동작을 계속할 수
있는지 정해야 합니다. 유효한 빈 배열이나 객체는 선언된 스키마가 그 정확한 값을
명시적으로 허용할 때만 유효한 빈 값입니다.

복구 가능한 상태는 담당 문서가 정의한 명시적 verify 또는 repair 흐름을 통해서만
다시 만들 수 있습니다. 읽기와 일반 실행은 실패를 분류하는 동안 데이터를 repair,
migration, 추정, 암묵적 교체하면 안 됩니다.

## 구조화된 Diagnostic Finding

공유 `DiagnosticFinding`은 제품 전체에서 사용하는 구조화된 diagnostic 단위입니다.
한도가 있는 안정적인 finding ID, namespaced code, domain, stage, severity, producer source,
typed subject, 안전하게 projection한 facts, 0개 이상의 cause reference와 권장 action,
관찰 timestamp, 선택적인 correlation, Connection, project, runtime-session, integration
revision 좌표를 담습니다. Domain 담당자는 closed code vocabulary와 오류를 finding으로
변환하는 규칙을 계속 담당합니다.

영속 finding에는 서로 다른 두 lifecycle이 있습니다. 발생형 finding은 runtime, process,
protocol 또는 그 밖의 event 성격 관찰 하나를 기록하며 insert-only finding 경로를
사용합니다. Runtime과 상관된 발생형 finding은 변경할 수 없습니다. 현재 상태 운영
finding은 정확한 주체 하나에 대한 교체 가능한 snapshot입니다. 안정적인 ID는 Connection
좌표와 diagnostic code에 정규 subject kind 및 reference의 한도가 있는 소문자 digest를
결합합니다. 정확한 주체는 typed `subject` 값에 남고 ID에 노출되지 않으므로 같은 code를
가진 서로 다른 주체를 구분하면서 관리 경로를 누출하지 않습니다. 같은 주체를 다시
관찰하면 facts, actions, 관찰 시각, revision 좌표, 나가는 cause edge를 원자적으로
교체합니다. 현재 상태 upsert는 입력 finding에 runtime session이 있거나 기존 runtime-session
finding을 교체하려는 경우를 모두 거부합니다.

Safe facts는 저장하거나 렌더링하기 전에 한도를 검증합니다. Typed projection은 민감한
key를 가리고 text 크기, collection 크기, nesting depth를 제한합니다. Raw environment map,
request body, tool argument set, credential, 제한 없는 child-process output은 diagnostic
fact가 아닙니다. Producer는 이런 입력을 finding에 옮기는 대신 한도가 있는 안전한 요약을
제공해야 합니다.

Registry는 공유 finding을 구조화된 column과 한도가 있는 subject, facts, action JSON으로
저장합니다. Cause reference는 별도 edge입니다. 모든 edge는 기존 finding을 가리켜야 하며,
중복 edge와 self edge는 거부하고, 검증된 graph 삽입과 Registry constraint가 cycle을
거부하며, 한도가 있는 traversal은 결정적입니다. Finding graph 삽입은 transaction
하나입니다. 유효하지 않은 node, 없는 cause, 중복 edge, cycle이 있으면 부분 graph나
dangling edge가 남지 않습니다. MCP terminal finding 삽입과 runtime-session 연결도 같은
방식으로 transaction 하나에서 수행할 수 있습니다.

Root cause 선택은 이 typed cause edge만 사용합니다. Summary를 parsing하거나 stage 또는
enum 순서를 비교하거나 첫 번째 실패 check를 고르지 않습니다. 선택 결과는 ID로 정렬해
결정적이며, 서로 독립인 root를 모두 유지하고 선택한 ancestor가 이미 설명하는 downstream
증상은 제외합니다. 따라서 여러 선택 finding이 같은 ancestor로 모여도 그 ancestor는 한
번만 나타납니다. Traversal은 cause edge 32개로 제한합니다. 알 수 없는 reference, cycle,
한도를 넘는 경로가 있으면 root를 추정하지 않고 선택을 거부합니다.
`DiagnosticReport.root_cause_ids`는 report finding에서 계산한 결과이며 caller가 별도로
선택해서 제공할 수 없습니다.

`DiagnosticReport`는 현재 사용하는 유일한 lossless diagnostic JSON envelope입니다.
`schema_version`은 `2`입니다. Typed `operation`, 집계 `status`, 생성 timestamp, 선택적인
Connection context, 전체 check 배열, 한도가 있는 finding graph, 계산한 root-cause ID,
중복 제거한 report action, operation별 typed detail, report limit을 담습니다. Report
action은 namespaced code, 한도가 있는 summary, 해당 action이 복구하는 정확한 root ID를
담습니다. 다른 schema version, 알 수 없는 최상위 구성원, 중복 check 또는 finding ID,
잘못된 cause graph, 계산 결과와 다른 supplied root list, 중복 action code, root가 아닌
finding을 가리키는 action은 역직렬화에서 거부합니다. 예전 connection-report schema를
위한 두 번째 분기는 없습니다.

Machine consumer는 관찰 결과를 구조적으로 구분합니다. 관찰 부재는 생략한 optional 값
또는 담당자가 정의한 typed `observation_state=absent`, 관찰한 빈 collection은 값이 있는
`[]`, 관찰 실패는 cause finding이 있는 `failed` check, prerequisite 때문에 막힌 관찰은
해당 root ID가 있는 `blocked` check입니다. Producer는 consumer가 parsing해야 하는 사람용
summary로 이 상태를 encode하면 안 됩니다. 렌더러는 typed fact를 골라 라벨을 붙일 수
있지만 산문에서 cause edge나 action category를 만들 수 없습니다.

Connection 검증은 이 cause graph를 정확히 다섯 가지 check 상태로 사용합니다. `passed`는
check가 성공적으로 끝났다는 뜻입니다. `pending`은 필요한 외부 관찰 또는 사용자가
일으키는 event가 아직 없고 이를 막는 실패 prerequisite도 없다는 뜻입니다. `failed`는
check 자체가 실패를 관찰했다는 뜻입니다. `blocked`는 prerequisite finding이 실패해
check를 실행하거나 관찰할 수 없었다는 뜻입니다. `not_applicable`은 해당 Connection 또는
profile에 check가 적용되지 않는다는 뜻입니다. Blocked check는 해소된 root finding ID를
담습니다. Root 기반 action은 중복을 제거하며, blocker가 해소되기 전에는 blocked
downstream 관찰을 위한 action을 만들지 않습니다. 기준 check dependency와 report 집계
규칙은 [Agent Connection](agent-connection.md)이 담당합니다.

Registry를 열기 전에 실패하면 공유 stderr fallback envelope은 아래 정확한 형태의 한도가
있는 단일 행 하나뿐입니다.

```text
VOLICORD_DIAGNOSTIC_V1 <bounded-json>
```

JSON은 정확히 현재 공유 `DiagnosticFinding` 하나입니다. Format과 parse는 공유 필드 검증,
safe-fact 한도, 정확한 prefix, 단일 행 형태, 전체 envelope byte 한도를 집행합니다. 이
fallback은 환경 dump를 허용하지 않으며 두 번째 diagnostic model을 만들지 않습니다.

## 합성 값과 기본값으로 합치기 금지

다음 값을 사용해 실패를 일반 값으로 바꾸면 안 됩니다.

- 계약에 실제로 없던 빈 문자열, 배열, 객체, 0 값
- 합성 식별자, placeholder record, 조작한 timestamp, 조작한 capability
- decode 또는 조회 실패 뒤 고른 기본 enum variant
- fallback 호스트, 어댑터, decoder, 외부 계약, 저장소 형태
- 현재 계약으로 취급한 과거 alias 또는 deprecated 형태
- 사람용 문구로만 실패를 표시하는 성공 응답

`unwrap_or_default()` 같은 구현 편의 기능은 이 계약을 바꾸지 않습니다. 기본값은
영속 전에 담당 문서가 정의한 typed construction 경로에서 만들어졌고, 그 결과로
저장된 값 자체가 선언된 전체 계약을 통과할 때만 유효합니다.

## 효과와 응답 처리 경로

실패 범주 자체는 상태 변경, 재시도 가능성, HTTP 또는 JSON-RPC 상태, CLI 종료
코드, API 응답 분기, 표시 문구를 정의하지 않습니다. 영향을 받는 메서드,
어댑터, 전송, CLI, 저장 효과 담당 문서는 이 범주 의미를 보존하면서 해당 표시
형태를 정의합니다.

이웃 담당 문서:

- 공개 API 응답 분기와 공개 코드: [API 오류](api/errors.md).
- 영속 기록 계약: [저장 기록](storage-records.md).
- Runtime Home 배치와 Registry 이전 stderr fallback 경계:
  [런타임 경계](runtime-boundaries.md).
- 메서드별 저장 효과와 효과 없음 분기: [저장 효과](storage-effects.md).
- 관리 검증 및 repair 명령: [관리 CLI](admin-cli.md).
