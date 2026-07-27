# API 판단 스키마

이 문서는 공통 사용자 행동 스키마 안에 중첩되는 선택형 판단 payload를 담당합니다.
별도의 영속 판단 생명주기를 담당하지 않습니다. 요청 identity, 유효 상태, 근거, adapter-neutral resolution form,
만료, 채널 경로, 불변 resolution identity는 [API 사용자 행동
스키마](schema-user-action.md)가 담당합니다.

## 경계

일곱 `judgment_kind` 값은 `product_decision`, `technical_decision`,
`scope_decision`, `sensitive_approval`, `final_acceptance`,
`residual_risk_acceptance`, `cancellation`입니다. 이 값은
`action_type=choice` 안에만 나타납니다. `evidence_observation`은 다른 사용자 행동
계열이며 판단이 아닙니다.

## Choice 요청 payload

```schema
UserActionDraft:
  action_type: choice
  judgment_kind: string
  presentation: short
  question: string
  options: UserActionOptionInput[] | null
  context: UserActionContext
  affected_refs: StateRecordRef[]
  sensitive_action_scope: SensitiveActionScope | null

UserActionOptionInput:
  option_id: string
  label: string
  description: string
  consequence: string
  is_default: boolean

UserActionOption:
  option_id: string
  label: string
  description: string
  consequence: string
  machine_action: accept | reject | defer
  resolution_outcome: accepted | rejected | deferred
  is_default: boolean

UserActionContext:
  summary: string
  related_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]
  visible_risks: AcceptedRiskInput[]
  constraints: string[]
```

호출자 작성 선택지는 `product_decision`과 `technical_decision`에서만 받고 machine
action이나 outcome을 담지 않습니다. 권한 효력이 있는 종류는 Core가 선택지와 mapping을
만듭니다. `accept`는 `accepted`, `reject`는 `rejected`, `defer`는 `deferred`에만
대응합니다. label이나 자유 형식 텍스트가 mapping을 뒤집거나 권한을 부여할 수 없습니다.

공통 choice `UserActionBasis`는 현재 close-basis revision, result ref, 잔여 위험 ID,
민감 동작 범위를 담습니다. 이 좌표는 Core 파생이며 resolution 입력이 아닙니다.

## Choice resolution payload

```schema
UserActionResolutionInput:
  resolution_type: choice
  selected_option_id: string
  note: string | null

UserActionResolutionBody:
  resolution_type: choice
  selected_option_id: string
  machine_action: accept | reject | defer
  resolution_outcome: accepted | rejected | deferred
  note: string | null
  accepted_risk_ids: string[]
```

사용자는 저장 선택지 ID와 Unicode scalar value 최대 1,000개의 선택적 note만
제출합니다. Core는 저장 선택지에서 machine action과 outcome을 복사하고 저장 요청과
호환 근거에서 현재 수락 잔여 위험 ID를 도출합니다. `judgment_kind`와 영속
`action_kind`는 요청에서 가져옵니다. 민감 범위와 다른 권한 좌표는 resolution에
중복하지 않고 요청 근거에 남습니다.

호출자는 machine action, outcome, risk 객체, accepted-risk ID, answer branch, 민감
범위, rationale을 제출하거나 덮어쓸 수 없습니다. Core는 캡처하지 않은 rationale이나
합성 사용자 답변을 만들어 내지 않습니다.

수락한 선택지는 근거가 계속 현재 상태이고 불변 resolution에 호환
`local_user` User Channel provenance가 있을 때만 종류별 요구사항을 만족합니다. 거절과
연기는 영속 사용자 선택이지만 승인, 수락, 권한 부여, 면제, 닫기를 만들지 않습니다.

## `SensitiveActionScope`

```schema
SensitiveActionScope:
  action_kind: string
  description: string
  intended_paths: string[]
  sensitive_categories: string[]
  command_or_tool_summary: string | null
  network_or_host_summary: string | null
  secret_or_credential_summary: string | null
  capability_claim: string
  expires_at: string | null
```

이는 범위가 정해진 민감 동작 맥락이며 write ticket, OS 권한, 보안 경계, 최종 수락,
Evidence가 아닙니다.

<a id="acceptedriskinput"></a>
## `AcceptedRiskInput`

```schema
AcceptedRiskInput:
  risk_id: string
  summary: string
  consequence: string
  related_refs: StateRecordRef[]
  accepted_for_close: boolean
```

보이는 위험은 요청 context와 근거에 속합니다. Choice resolution은 Core가 도출한 정확한
현재 ID만 저장하며 이 객체를 중복하지 않습니다. 잔여 위험 수락은 위험이 남지 않았음을
증명하지 않습니다.

## 관련 담당 문서

- [API 사용자 행동 스키마](schema-user-action.md).
- [`volicord.request_user_action`](method-request-user-action.md).
- [`volicord.resolve_user_action`](method-resolve-user-action.md).
- 판단과 비대체 의미를 담당하는 [Core 모델](../core-model.md).
