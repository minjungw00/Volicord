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
- 공통 `response_kind`와 `effect_kind` 필드

이 문서는 담당하지 않습니다.

- 메서드 동작: [API 메서드](methods.md)와 메서드 담당 문서
- 상태와 현재 위치 스키마: [API 상태 스키마](schema-state.md)
- 아티팩트 스키마: [API 아티팩트 스키마](schema-artifacts.md)
- 사용자 소유 판단 스키마: [API 판단 스키마](schema-judgment.md)
- 지원되는 메서드 이름, `response_kind` 값, `effect_kind` 값, 접근 등급, 그 밖의 enum 형태 값: [API 값 집합](schema-value-sets.md)
- 공개 오류 코드, 우선순위, 오류 의미: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md)
- 저장소 기록과 효과: [저장소 기록](../storage-records.md), [저장 효과](../storage-effects.md)

## 스키마 표기 규칙

의미:
- 이 문서의 스키마 블록은 공개 API 형태를 나타내는 계약 표기입니다.
- 필드 존재와 중첩 구조를 설명하며, 메서드별 동작은 설명하지 않습니다.

의미하지 않는 것:
- 스키마 블록은 생성된 코드가 아닙니다.

표기:
- `string`은 JSON 스칼라 형태만 나타냅니다. 이 표기만으로 자유 형식 텍스트라는 뜻이 되지 않습니다.
- `string | null`은 필드가 존재하며 `null`일 수 있다는 뜻입니다.
- `Type[]`는 그 타입의 배열입니다.

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

의미하지 않는 것:
- 더 좁은 메서드별 요청 규칙을 덮어쓰지 않습니다.

담당 문서 링크:
- 메서드별 요청 규칙: [API 메서드](methods.md)가 안내하는 메서드 담당 문서.

```yaml
ToolEnvelope:
  project_id: string
  task_id: string | null
  actor_kind: string
  surface_id: string
  request_id: string
  idempotency_key: string | null
  expected_state_version: integer | null
  dry_run: boolean
  locale: string | null
```

의미:
- `task_id`는 요청 수준의 선택적 `Task` 선택자입니다.
- `actor_kind`는 제어 값 문자열입니다.
- `expected_state_version`은 프로젝트 전체 상태 시계 값을 담는 요청 수준 필드입니다.
- `project_id`, `task_id`, `surface_id`, `request_id`, `idempotency_key`는 불투명 식별자입니다.
- `locale`은 로캘 태그 문자열이며 하네스가 제어하는 값 집합이 아닙니다.

의미하지 않는 것:
- 이 필드 목록은 충돌 동작, 저장소 버전 관리, 메서드별 선택자 우선순위를 정의하지 않습니다.

담당 문서 링크:
- `actor_kind` 값: [행위자 값](schema-value-sets.md#actor-values)
- 메서드별 요청 동작: [API 메서드](methods.md)가 안내하는 메서드 담당 문서
- 충돌 동작: [상태 버전 충돌](error-precedence.md#state-conflict-behavior)
- 저장소 버전 동작: [저장소 버전 관리](../storage-versioning.md)

<a id="common-response"></a>
## 공통 응답 분기

공개 메서드 응답은 정확히 하나의 분기를 사용합니다.

- 메서드별 `MethodResult`
- `ToolRejectedResponse`
- 메서드 담당 문서가 `dry_run` 미리보기 분기를 정의할 때의 `ToolDryRunResponse`

의미:
- `MethodResult`는 [API 메서드](methods.md)가 안내하는 메서드 담당 문서가 정의하는 메서드별 결과 분기입니다.
- 모든 구체 메서드 결과는 `base: ToolResultBase`를 담고 그 뒤에 그 메서드의 결과 필드만 둡니다.

의미하지 않는 것:
- `MethodResult`는 하나의 구체 스키마 이름이 아닙니다.

```yaml
ToolResultBase:
  response_kind: string
  effect_kind: string
  dry_run: boolean
  state_version: integer | null
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

의미하지 않는 것:
- `ToolRejectedResponse`와 `ToolDryRunResponse`는 `task_ref`, `run_summary`, `staged_artifact_handle`, `write_authorization_ref`, `user_judgment_ref`, `decision`, `close_state` 같은 결과 전용 필드를 담지 않습니다.

담당 문서 링크:
- 지원되는 `response_kind`와 `effect_kind` 값: [응답과 효과 값](schema-value-sets.md#response-and-effect-values)
- 공통 분기 읽기 규칙: [공통 응답 분기](#common-response)
- 메서드별 상태 효과: 메서드 담당 문서
- 공개 오류 우선순위: [API 오류 우선순위](error-precedence.md)

## `dry_run` 요약 형태

의미:
- `DryRunSummary`, `PlannedEffect`, `PlannedBlocker`는 공통 `dry_run` 분기 보조 형태입니다.
- 설명용 미리보기 데이터 형태일 뿐입니다.

의미하지 않는 것:
- 이 문서는 기록 생성, 참조 예약, 핸들 소비, 재실행 행, `state_version` 효과를 정의하지 않습니다.

```yaml
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

```yaml
ToolError:
  code: string
  message: string
  retryable: boolean
  details: object | null

EventRef:
  event_id: string
  event_kind: string
```

의미:
- `ToolError`는 `ToolRejectedResponse.errors`와 미리보기 가능한 `DryRunSummary.would_errors`가 사용하는 형태입니다.
- `ToolError.code`는 공개 `ErrorCode` 값입니다.
- `ToolError.message`는 자유 형식 표시 문자열입니다.
- `EventRef.event_id`는 불투명 이벤트 식별자입니다.
- `EventRef.event_kind`는 불투명 이벤트 분류 문자열입니다. 전달하고 경로를 안내할 만큼 안정적이지만, 이 문서는 빠짐없는 공개 `event_kind` 값 집합을 공개하지 않습니다.

담당 문서 링크:
- 공개 오류 코드 집합: [API 오류 코드](error-codes.md)
- 오류 세부사항 의미: [API 오류 세부사항](error-details.md)
- 주 오류 우선순위: [API 오류 우선순위](error-precedence.md)
- `EventRef.event_kind`의 불투명 경계: [불투명 문자열과 메서드 범위 문자열 필드](schema-value-sets.md#opaque-and-method-scoped-string-fields)
