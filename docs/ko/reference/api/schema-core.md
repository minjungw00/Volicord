# API 코어 스키마

의미:
- 이 문서는 기준 범위의 공통 API 요청 래퍼와 응답 분기 스키마를 담당합니다.
- 참조 문서일 뿐입니다.

의미하지 않는 것:
- 메서드 동작, 저장 효과, 상태 스냅샷, 아티팩트 생명주기, 사용자 소유 판단 의미, 공개 오류 의미, 활성 값 집합을 정의하지 않습니다.

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
- 활성 메서드 이름, `response_kind` 값, `effect_kind` 값, 접근 등급, 그 밖의 enum 형태 값: [API 값 집합](schema-value-sets.md)
- 공개 오류 코드, 우선순위, 오류 의미: [API 오류](errors.md)
- 저장소 기록과 효과: [저장소 기록](../storage-records.md), [저장 효과](../storage-effects.md)

## 스키마 표기 규칙

의미:
- 이 문서의 스키마 블록은 계획 표기입니다.
- 후보 API 계약 형태만 설명합니다.

의미하지 않는 것:
- 스키마 블록은 생성된 코드가 아닙니다.

표기:
- `string | null`은 필드가 존재하며 `null`일 수 있다는 뜻입니다.
- `Type[]`는 해당 타입의 배열입니다.

담당 문서 링크:
- 필드 값 집합: 이 문서가 자유 형식 텍스트나 불투명 식별자라고 말하지 않는 한 [API 값 집합](schema-value-sets.md)에 둡니다.

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
- `task_id`는 요청 수준의 선택적 Task 선택자입니다.
- `expected_state_version`은 상태 변경 메서드가 쓰는 프로젝트 전체 상태 시계를 가리킵니다.

우선순위:
- 메서드별 `task_id` 필드가 있으면 [공통 요청 래퍼와 응답 분기 경로](methods.md#공통-요청-규칙)가 설명하는 대로 그 필드가 우선합니다.

담당 문서 링크:
- 충돌 동작: [상태 버전 충돌](errors.md#state-conflict-behavior)
- 저장소 버전 동작: [저장소 버전 관리](../storage-versioning.md)

<a id="common-response"></a>
## 공통 응답 분기

공개 메서드 응답은 정확히 하나의 분기를 사용합니다.

- 메서드별 `MethodResult`
- `ToolRejectedResponse`
- 선택된 상태 효과 동작이나 저장소 스테이징 동작에 유효한 미리보기 분기가 있을 때의 `ToolDryRunResponse`

의미:
- `MethodResult`는 [API 메서드](methods.md)가 안내하는 메서드 담당 문서가 정의하는 메서드별 성공 또는 커밋 결과 분기입니다.
- 모든 구체 메서드 결과는 `base: ToolResultBase`를 담고 그 뒤에 해당 메서드의 결과 필드만 둡니다.

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
- 활성 `response_kind`와 `effect_kind` 값: [응답과 효과 값](schema-value-sets.md#응답과-효과-값)
- 공통 분기 읽기 규칙: [공통 요청 래퍼와 응답 분기 경로](methods.md#공통-요청-규칙)
- 메서드별 상태 효과: 메서드 담당 문서
- 공개 오류 우선순위: [API 오류](errors.md)

## `dry_run` 요약 형태

의미:
- `DryRunSummary`, `PlannedEffect`, `PlannedBlocker`는 공통 `dry_run` 분기 보조 형태입니다.
- 설명용 미리보기 데이터일 뿐입니다.

의미하지 않는 것:
- 기록을 만들지 않습니다.
- 참조를 예약하지 않습니다.
- 핸들을 소비하지 않습니다.
- 재실행 행을 만들지 않습니다.
- `state_version`을 올리지 않습니다.

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
- `PlannedBlocker.source_kind` 값: [상태와 차단 사유 값](schema-value-sets.md#상태와-차단-사유-값)
- `ToolError.code`에 쓰는 공개 `ErrorCode` 값: [API 오류](errors.md)

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

담당 문서 링크:
- 공개 오류 코드 집합, 오류 세부사항 의미, 주 오류 우선순위: [API 오류](errors.md)
