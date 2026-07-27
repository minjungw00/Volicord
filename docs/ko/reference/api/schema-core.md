# API 코어 스키마

이 문서는 기준 범위 공개 API에서 공통으로 쓰는 API 요청 래퍼(`ToolEnvelope`)와 공유 스키마 요소를 담당합니다. 여기에는 아래의 공통 응답 분기 모델, 공통 보조 형태, 스키마 표기 규칙이 포함됩니다.

인접 계약은 각 담당 문서에 둡니다. 메서드 동작은 [API 메서드](methods.md), 저장 효과는 [저장 효과](../storage-effects.md), Core 권한은 [Core 모델](../core-model.md), 런타임 경계는 [런타임 경계](../runtime-boundaries.md), 표시 문구와 템플릿 본문은 [템플릿 본문](../template-bodies.md)을 따릅니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- API 스키마 담당 문서에서 쓰는 스키마 표기 규칙
- `ToolEnvelope`
- 공통 메서드 결과 분기 모델
- `ToolResultBase`
- `ToolRejectedResponse`
- `ToolDryRunResponse`
- `ToolError`
- `EventRef`
- `OperationResultRef`
- 공통 `response_kind`와 `effect_kind` 필드
- 전송별 wrapping 이전의 정식 공개 결과 본문

이 문서는 담당하지 않습니다.

- 메서드 동작: [API 메서드](methods.md)와 메서드 담당 문서
- 상태와 현재 위치 스키마: [API 상태 스키마](schema-state.md)
- 아티팩트 스키마: [API 아티팩트 스키마](schema-artifacts.md)
- 사용자 소유 판단 스키마: [API 판단 스키마](schema-judgment.md)
- 지원되는 메서드 이름, `response_kind` 값, `effect_kind` 값,
  `FailureCategory` 값, 작업 범주(`operation_category`) 값, 그 밖의 enum 형태 값:
  [API 값 집합](schema-value-sets.md)
- 공개 오류 코드, 우선순위, 오류 의미: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md)
- 제품 전체 실패 범주 의미: [실패 모델](../failure-model.md)
- 저장소 기록과 효과: [저장소 기록](../storage-records.md), [저장 효과](../storage-effects.md)
- MCP revision별 도구 정의, 결과 carrier, 오류 flag: [MCP 전송](../mcp-transport.md)

`volicord-types`는 이 문서가 설명하는 adapter-neutral 공개 schema 계열을 구현합니다.
여기에서 생성하는 공개 메서드 schema에는 MCP 요청 wrapper, MCP 오류 identity,
structured-content union, 도구 정의 envelope, JSON-RPC field가 없습니다. 생성 wire
schema와 정확한 직렬화 규칙은 `volicord-mcp-wire`가 담당하고 semantic profile 선택은
`volicord-mcp-protocol`이 담당합니다.

Core 소유 MCP 도구의 정규 `AgentToolId` identity는 기존 `MethodName` domain을 재사용하고
안정적인 MCP wire 이름을 투영합니다. Adapter utility도 같은 폐쇄형 identity catalog에
속합니다. 운영 `ToolVerificationRole::ManagedHostRoundTrip`과
`AgentToolId::LIST_PROJECTS`의 결합은 이 catalog의 컴파일 시점 metadata이며, 별도의 Core
method identity를 정의하지 않습니다.

`volicord.begin_integration_verification`, `volicord.guard_probe`,
`volicord.get_integration_verification`은 이 catalog의 Connection-integration 구성원입니다.
공개 요청/결과 스키마는 [MCP 전송](../mcp-transport.md#in-chat-integration-verification-schemas)이
담당하며 공유 tagged `IntegrationVerificationWorkflowState`, typed tool 지향 alternative,
phase 관찰, 한도 있는 finding을 포함합니다. `ToolEnvelope`, Core method 스키마, Task 상태는
이를 담당하지 않습니다. Adapter 효과는
[저장 효과](../storage-effects.md#connection-integration-verification-effects)가 담당합니다.

## 스키마 표기 규칙

의미:
- 이 문서의 스키마 블록은 공개 API 형태를 나타내는 계약 표기입니다.
- 필드 존재와 중첩 구조를 설명하며, 메서드별 동작은 설명하지 않습니다.

의미하지 않는 것:
- 스키마 블록은 생성된 코드가 아닙니다.

표기:
- `string`은 JSON 스칼라 형태만 나타냅니다. 이 표기만으로 자유 형식 텍스트라는 뜻이 되지 않습니다.
- `T | null`은 필드가 반드시 존재해야 하며 `T` 또는 JSON `null`을 담을 수 있다는 뜻입니다.
- 선택 필드는 담당 스키마나 메서드 필드 참고가 명시적으로 선택 필드라고 표시할 때만 생략할 수 있습니다.
- 선택성과 null 허용성은 서로 독립적인 속성입니다.
- `Type[]`는 그 타입의 배열입니다.
- JSON Schema 검증과 타입 지정 디코더 동작은 같은 페이로드를 받아들이고 거절해야 합니다.
- 표준 멱등성 해싱은 원시 JSON 형식이 아니라 성공적으로 디코드된 타입 지정 요청을 사용합니다.

문자열 형태 필드 분류:
- 제어 값 문자열은 연결된 값 집합 담당 문서의 지원 값을 사용해야 합니다.
- 불투명 식별자 또는 분류 문자열은 전달, 비교, 상관관계 확인, 더 좁은 담당 문서 경로 안내에 쓸 만큼 안정적입니다. 다만 담당 문서가 값 목록을 공개하지 않는 한 빠짐없는 공개 enum이 아닙니다.
- 자유 형식 표시 문자열은 사람이 읽는 표시 텍스트입니다. 기준 스키마 값, 오류 코드, 차단 사유 코드, 저장소 식별자가 아닙니다.

담당 문서 링크:
- 제어 값 문자열: 스키마나 메서드 담당 문서가 더 좁은 담당 문서로 연결하지 않는 한 [API 값 집합](schema-value-sets.md)에 둡니다.
- 공개 오류 코드: [API 오류 코드](error-codes.md).
- API 예시는 관련 스키마 담당 문서가 해당 필드를 명시적으로 자유 형식 표시 문자열, 불투명 식별자, 또는 불투명 분류 문자열로 정의하지 않는 한 [API 값 집합](schema-value-sets.md)의 지원되는 enum 형태 값을 사용해야 합니다.

<a id="tool-envelope"></a>
## `ToolEnvelope`

의미:
- `ToolEnvelope`는 공개 메서드가 사용하는 공통 요청 래퍼입니다.
- `ToolEnvelope`에 표시된 모든 필드는 필수 래퍼 멤버입니다. `T | null`로 타입이 지정된 멤버도 반드시 있어야 하며 JSON `null`을 담을 수 있습니다.

의미하지 않는 것:
- 더 좁은 메서드별 요청 규칙을 덮어쓰지 않습니다.
- `actor_source`, `operation_category`, 검증 근거, 그 밖의 호출 출처를 담지 않습니다.

담당 문서 링크:
- 메서드별 요청 규칙: [API 메서드](methods.md)가 안내하는 메서드 담당 문서.

```schema
ToolEnvelope:
  project_id: string
  task_id: string | null
  request_id: string
  idempotency_key: string | null
  expected_state_version: integer | null
  dry_run: boolean
  locale: string | null
```

의미:
- `task_id`는 null 허용 요청 수준 `Task` 선택자입니다. 필드는 존재하며 값은 null일 수 있습니다.
- `expected_state_version`은 `project_state.state_version` 값을 담는 요청 수준 낙관적 동시성 필드입니다.
- `idempotency_key`는 null 허용 불투명 식별자입니다. 메서드 담당 문서가 null이 아닌 값이 필요한 때를 정의합니다.
- `expected_state_version`은 null 허용입니다. 메서드와 저장소 담당 문서가 null이 아닌 값이 필요한 때를 정의합니다.
- 상태 보기가 제공한 `NextActionSummary.expected_state_version`이 null이 아니면 해당 행동의 담당 메서드 호출에서 이 필드로 직접 매핑합니다. 다음 행동의 값이 null이어도 메서드 담당 문서가 별도로 요구하는 null이 아닌 토큰 요건이 면제되지는 않습니다.
- `StateRecordRef.produced_at_state_version`은 상태 보기의 최신성 메타데이터이며 `ToolEnvelope.expected_state_version`을 대신하지 않습니다.
- `project_id`, `task_id`, `request_id`, `idempotency_key`는 null이 아닐 때 불투명 식별자입니다.
- `locale`은 null 허용 로캘 태그 문자열이며 Volicord가 제어하는 값 집합이 아닙니다.
- 행위자 출처와 작업 범주는 공개 요청 필드가 아니라 [Agent Connection](../agent-connection.md)이 설명하는 어댑터/Core 로직에서 파생됩니다.

의미하지 않는 것:
- 이 필드 목록은 충돌 동작, 저장소 버전 관리, 메서드별 선택자 우선순위를 정의하지 않습니다.

담당 문서 링크:
- 행위자 출처 값: [행위자 출처 값](schema-value-sets.md#actor-source-values)
- 상태 보기가 제공하는 다음 행동 토큰과 상태 참조 최신성: [API 상태 스키마의 상태 참조](schema-state.md#state-references), [현재 위치 표시 형태](schema-state.md#current-position-display-shapes)
- 메서드별 요청 동작: [API 메서드](methods.md)가 안내하는 메서드 담당 문서
- 충돌 동작: [상태 버전 충돌](error-precedence.md#state-conflict-behavior)
- 저장소 버전 동작: [저장소 버전 관리](../storage-versioning.md)

<a id="common-response"></a>
## 공통 응답 분기

이 응답 schema는 전송 carrier와 독립된 정식 공개 결과 본문을 정의합니다. MCP 어댑터는
같은 객체를 유지하고 선택한 protocol profile이 허용하는 carrier로 projection합니다.
`toolResult`, `content`, `structuredContent`, MCP `isError`는
[MCP 전송](../mcp-transport.md)이 담당하는 전송 필드입니다. 이 필드는 API 결과 본문의
필드를 추가, 제거, 재해석하지 않으며 Core 분기 의미도 바꾸지 않습니다.

공개 메서드 응답은 정확히 하나의 분기를 사용합니다.

- 메서드별 `MethodResult`
- `ToolRejectedResponse`
- 메서드 담당 문서가 `dry_run` 미리보기 분기를 정의할 때의 `ToolDryRunResponse`

의미:
- `MethodResult`는 [API 메서드](methods.md)가 안내하는 메서드 담당 문서가 정의하는 메서드별 결과 분기입니다.
- 모든 구체 메서드 결과는 `base: ToolResultBase`를 담고 그 뒤에 그 메서드의 결과 필드만 둡니다.
- `response_kind=result`와 성공한 전송은 완료 주장을 허가하지 않습니다. Task 범위
  workflow 결과는 현재 `AuthorityReceipt.completion_claim_allowed`를 사용하며 거절,
  dry-run, refresh 실패, 활성 Task 없음 분기는 완료 권한이 아닙니다.

의미하지 않는 것:
- `MethodResult`는 하나의 구체 스키마 이름이 아닙니다.

```schema
ToolResultBase:
  response_kind: string
  effect_kind: string
  dry_run: boolean
  state_version: integer | null
  disclosure: GuaranteeDisclosure
  events: EventRef[]

ToolRejectedResponse:
  base: ToolResultBase
  errors: ToolError[]

ToolDryRunResponse:
  base: ToolResultBase
  dry_run_summary: DryRunSummary
```

의미:
- 메서드별 결과 필드는 그 메서드 결과 분기에만 둡니다.
- `ToolResultBase.disclosure`는 기계가 읽을 수 있는 공개 정보입니다. 응답 분기를 해석할 때 무엇을 보장하고 보장하지 않는지 설명합니다.

의미하지 않는 것:
- `ToolRejectedResponse`와 `ToolDryRunResponse`는 `task_ref`, `run_summary`, `staged_artifact_handle`, `write_ticket_ref`, `user_action_resolution_ref`, `decision`, `close_state` 같은 결과 전용 필드를 담지 않습니다.
- `ToolResultBase.disclosure`는 OS 샌드박싱, 네트워크 격리, 악성 코드 방어, 변조 불가능 감사 로그, 전체 쓰기 방지, 전체 파일시스템 감시, 행위자 귀속 증명, 정확성 증명, 테스트 충분성 증명, 인간 검토 대체를 만들지 않습니다.
- `effect_kind`와 표시 문구는 `completion_claim_allowed=false`, 닫기 차단 사유,
  누락된 권한을 덮어쓸 수 없습니다.

담당 문서 링크:
- 지원되는 `response_kind`와 `effect_kind` 값: [응답과 효과 값](schema-value-sets.md#response-and-effect-values)
- 공개 형태: [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes)
- 공개 값 집합: [API 값 집합](schema-value-sets.md#state-and-blocker-values)
- 공통 분기 읽기 규칙: [공통 응답 분기](#common-response)
- 메서드별 상태 효과: 메서드 담당 문서
- 공개 오류 우선순위: [API 오류 우선순위](error-precedence.md)

## `dry_run` 요약 형태

의미:
- `DryRunSummary`, `PlannedEffect`, `PlannedBlocker`는 공통 `dry_run` 분기 보조 형태입니다.
- 설명용 미리보기 데이터 형태일 뿐입니다.

의미하지 않는 것:
- 이 문서는 기록 생성, 참조 예약, 핸들 소비, 재실행 행, `state_version` 효과를 정의하지 않습니다.

```schema
DryRunSummary:
  planned_effects: PlannedEffect[]
  would_blockers: PlannedBlocker[]
  would_errors: ToolError[]
  next_actions: NextActionSummary[]
  diagnostics: string[]

PlannedEffect:
  target_kind: string
  action: string
  description: string

PlannedBlocker:
  source_kind: string
  category: string
  code: string
  message: string
  related_refs: StateRecordRef[]
```

담당 문서 링크:
- `NextActionSummary`와 `StateRecordRef`: [API 상태 스키마](schema-state.md)
- `PlannedBlocker.source_kind` 값: [상태와 차단 사유 값](schema-value-sets.md#state-and-blocker-values)
- `PlannedBlocker.category` 값 담당 경로: [상태와 차단 사유 값](schema-value-sets.md#state-and-blocker-values)
- `ToolError.code`에 쓰는 공개 `ErrorCode` 값: [API 오류 코드](error-codes.md)

`PlannedEffect.target_kind`와 `PlannedEffect.action`은 메서드 담당 문서가 특정 `dry_run` 분기에서 더 좁게 정의하지 않는 한 불투명 미리보기 분류 문자열입니다. `PlannedEffect.description`과 `DryRunSummary.diagnostics[]` 항목은 자유 형식 표시 문자열입니다.

`PlannedBlocker.category`는 `PlannedBlocker.source_kind`가 이름 붙이는 차단 사유 계열의 범주 집합을 사용합니다. `source_kind=write_decision`에는 쓰기 결정 범주를, `source_kind=close_readiness`에는 닫기 차단 사유 범주를 사용합니다. `PlannedBlocker.code`는 메서드 담당 문서가 더 좁은 로컬 코드 목록을 명시적으로 정의하지 않는 한 불투명 미리보기 사유 코드입니다. `PlannedBlocker.message`는 자유 형식 표시 문자열입니다.

<a id="shared-support-shapes"></a>

## 공통 보조 형태

```schema
ToolError:
  category: FailureCategory
  code: ErrorCode
  message: string
  retryable: boolean
  details: object | null

EventRef:
  event_id: string
  event_kind: string

OperationResultRef:
  project_id: string
  source_method: string
  source_idempotency_key: string
  committed_state_version: integer
  response_sha256: string
  response_size_bytes: integer
```

의미:
- `ToolError`는 `ToolRejectedResponse.errors`와 미리보기 가능한 `DryRunSummary.would_errors`가 사용하는 형태입니다.
- 표시된 모든 `ToolError` 필드는 필수입니다. `details`는 반드시 있어야 하며 null일
  수 있습니다.
- `ToolError.category`는 지원되는 `FailureCategory` 식별자입니다. 더 좁은 공개 코드와
  도메인 사유와 독립적으로 실패를 분류합니다.
- `ToolError.code`는 공개 `ErrorCode` 값입니다.
- `ToolError.message`는 자유 형식 표시 문자열입니다.
- `EventRef.event_id`는 불투명 이벤트 식별자입니다.
- `EventRef.event_kind`는 불투명 이벤트 분류 문자열입니다. 전달하고 경로를 안내할 만큼 안정적이지만, 이 문서는 빠짐없는 공개 `event_kind` 값 집합을 공개하지 않습니다.

<a id="operation-result-retrieval"></a>

`OperationResultRef` 의미:

- `source_method`는 커밋 replay 행에 응답을 저장한 공개 변경 메서드의 정확한
  이름입니다.
- `source_idempotency_key`는 그 커밋 호출의 불투명 idempotency 식별자이며 새 변경의
  idempotency key로 사용할 수 없습니다.
- `response_sha256`은 정확한 저장 UTF-8 응답 byte의 SHA-256에 리터럴 `sha256:`
  접두사와 소문자 16진수 64자리를 붙인 값입니다.
- `response_size_bytes`는 같은 UTF-8 byte의 정확한 길이입니다.
- 전체 형태는 bearer가 아닌 조회 locator입니다. 접근은
  [`volicord.get_operation_result`](method-get-operation-result.md#volicordget_operation_result)가
  page마다 다시 확인합니다.
- `OperationResultRef`는 `StateRecordRef`, authority receipt, 쓰기 티켓,
  artifact나 Evidence ref, 재시도 credential, 권한 token이 아닙니다. 과거 응답이나
  state version이 현재 값이라고 주장하지 않습니다.

담당 문서 링크:
- `FailureCategory` 값: [실패 범주 값](schema-value-sets.md#failure-category-values)
- 실패 범주 의미와 선택 경계: [실패 모델](../failure-model.md)
- 공개 오류 코드 집합: [API 오류 코드](error-codes.md)
- 오류 세부사항 의미: [API 오류 세부사항](error-details.md)
- 주 오류 우선순위: [API 오류 우선순위](error-precedence.md)
- `EventRef.event_kind`의 불투명 경계: [불투명 문자열과 메서드 범위 문자열 필드](schema-value-sets.md#opaque-and-method-scoped-string-fields)
- `OperationResultRef` 저장 불변성과 정확한 byte 출처: [저장 버전 관리](../storage-versioning.md#exact-operation-result-retrieval)
