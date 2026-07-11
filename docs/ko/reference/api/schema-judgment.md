# API 판단 스키마

이 문서는 기준 범위의 사용자 소유 판단 API 스키마를 담당합니다. 스키마는 사용자 소유 판단 형태의 API 데이터를 정의하지만 그 자체로 사용자 결정을 기록하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- `UserJudgment`
- `JudgmentInboxItem`
- `UserChannelAvailability`
- `UserJudgmentCandidate`
- `UserJudgmentOptionInput`
- `UserJudgmentOption`
- `UserJudgmentContext`
- `JudgmentBasis`
- `UserJudgmentResolution`
- `JudgmentRationale`
- `JudgmentResolutionOutcome`
- `RecordUserJudgmentPayload`
- `SensitiveActionScope`
- `AcceptedRiskInput`
- 사용자 소유 판단의 스키마 필드와 중첩 구조

이 문서는 담당하지 않습니다.

- 사용자 소유 판단의 제품 의미와 비대체 규칙: [Core 모델](../core-model.md)
- 판단 요청 메서드 동작: [사용자 소유 판단 요청 메서드](method-request-user-judgment.md)
- 판단 기록 메서드 동작: [사용자 소유 판단 기록 메서드](method-record-user-judgment.md)
- 지원되는 판단 종류 값, 상태 값, 표시 형식 값, 필요 판단 위치 값, 해결 결과 값: [API 값 집합](schema-value-sets.md)
- 최종 수락이나 잔여 위험 수락의 닫기 효과: [Core 모델](../core-model.md), [Task 닫기 메서드](method-close-task.md)
- 판단 누락, 미해결, 거절, 만료에 대한 공개 오류 의미: [API 오류 코드](error-codes.md)

## 경계

판단 스키마는 사용자가 소유한 선택의 필드 구조를 보존합니다. 제품 판단, 기술 판단, 범위 판단, 민감 동작 승인, 최종 수락, 잔여 위험 수락, 취소 판단, 지원되지 않는 판단 범주의 동작 계약이 아닙니다. 그 의미는 Core와 메서드 담당 문서에 둡니다.

`UserJudgmentCandidate`는 대기 중인 판단이 아닙니다.

`UserJudgment`와 `UserJudgmentCandidate`는 서로 다른 형태입니다. 각 형태가 응답에 나타나는 조건은 메서드 담당 문서가 정의합니다.

`UserJudgmentOptionInput`과 `UserJudgmentOption`은 서로 다른 형태입니다. `UserJudgmentOptionInput`은 메서드가 호출자 작성 선택지를 허용하는 곳에서만 쓰는 호출자 요청 입력입니다. `UserJudgmentOption`은 Core가 소유한 상태 또는 출력입니다.

`RecordUserJudgmentPayload`는 현재 적용 범위, 증거, 쓰기 티켓, 닫기 결과, 넓은 승인에 대한 스키마가 아닙니다.

`JudgmentRationale`은 설명 메타데이터입니다. 사용자가 볼 수 있는 이유와 검토 맥락을 보존하지만 권한 출처가 아니며, 선택된 선택지, 결과, 행위자 출처, 근거 호환성을 덮어쓸 수 없습니다.

<a id="userjudgment"></a>
## `UserJudgment`

```yaml
UserJudgment:
  judgment_id: string
  project_id: string
  task_id: string
  change_unit_id: string | null
  judgment_kind: string
  status: string
  presentation: string
  question: string
  options: UserJudgmentOption[]
  context: UserJudgmentContext
  affected_refs: StateRecordRef[]
  basis: JudgmentBasis
  required_for: string[]
  resolution: UserJudgmentResolution | null
  expires_at: string | null
  created_at: string
  resolved_at: string | null
```

`judgment_kind`, `status`, `presentation`, `required_for`, `machine_action`, `resolution_outcome` 값은 [판단 값](schema-value-sets.md#judgment-values)이 담당합니다. 제품 의미는 [Core 모델의 사용자 소유 판단](../core-model.md#4-user-owned-judgment)이 담당합니다.

`status=resolved`는 답변이 기록되었다는 뜻입니다. 그 자체로 승인, 수락, 권한 부여, 범위 결정 권한, 최종 수락, 잔여 위험 수락, 민감 승인, 취소 권한을 뜻하지 않습니다. 선택된 선택지에서 저장된 `resolution.machine_action`과 `resolution.resolution_outcome`만 기계 판독 가능한 권한 결과를 지닐 수 있습니다.

`judgment_id`, `project_id`, `task_id`, `change_unit_id`는 불투명 식별자입니다. `question`은 자유 형식 표시 문자열입니다.

저장되고 반환되는 판단에는 `basis`가 필요합니다. 근거가 없는 저장 판단은 유효하지 않은 소유자 상태입니다.

<a id="judgmentinboxitem"></a>
## `JudgmentInboxItem`

`JudgmentInboxItem`은 사용자 행동이 필요한 대기 판단을 보여 주는 사용자용 상태
보기입니다. 이 형태는 답변을 기록하지 않으며 지속되는 `UserJudgment`를 대체하지
않습니다.

```yaml
JudgmentInboxItem:
  judgment_id: string
  judgment_ref: StateRecordRef
  project_id: string
  task_id: string
  change_unit_id: string | null
  question: string
  context_summary: string
  choices: JudgmentInboxChoice[]
  answer_constraints:
    choice_required: boolean
    note_allowed: boolean
    note_max_chars: integer
  required: boolean
  requirement_status: "required" | "optional"
  required_for: string[]
  status: string
  answer_path_availability: UserChannelAvailability
  preferred_capture_path: JudgmentCapturePath | null
  fallbacks: JudgmentCapturePath[]
  expires_at: string | null

JudgmentInboxChoice:
  choice_id: string
  label: string
  description: string
  consequence: string
  is_default: boolean

JudgmentCapturePath:
  kind: string
  label: string
  available: boolean
  command: string | null
  url: string | null
  capture_basis: string | null
  expires_at: string | null
  detail: string | null

UserChannelAvailability:
  paths: UserChannelPathAvailability[]
  recommended_path_kind: string | null
  recommended_path_label: string | null
  recommendation: string | null

UserChannelPathAvailability:
  kind: string
  label: string
  available: boolean
  status: string
  capture_basis: string | null
  detail: string | null
```

`required=true`와 `requirement_status=required`는 `required_for`에
`informational`이 아닌 작업 대상이 하나 이상 있다는 뜻입니다.
`informational`만 있는 항목은 `required=false`와
`requirement_status=optional`을 사용합니다. 이 항목이 대기 상태이거나 현재
호환되는 근거를 가졌다는 사실만으로 필수 항목 또는 작업 차단 항목이 되지는
않습니다. `required_for`에 `informational`과 정보성 외 작업 대상이 함께 있으면
그 작업 대상에 필요한 항목입니다.

`choices`는 사용자에게 보이는 선택지 식별자와 라벨을 노출하며 내부 `machine_action`이나 `resolution_outcome` 필드는 노출하지 않습니다. 기계 동작과 결과는 지속되는 `UserJudgmentOption`과 기록된 해결에 남습니다.

`answer_path_availability`는 이 대기 판단에 대해 지원되는 User Channel 경로의 현재
사용 가능 상태를 보고합니다. 사용할 수 없는 경로도 포함할 수 있습니다. 예를 들어
호스트 프롬프트 입력을 사용할 수 없더라도 다른 경로가 사용 가능한지는 계속 확인할
수 있습니다. 현재 경로 종류에는 `mcp_elicitation`, `prompt_capture`,
`local_web_consent`, `cli`가 있습니다.

`preferred_capture_path`는 현재 어댑터 맥락에서 가장 적합한 User Channel 답변 경로를
가리킵니다. 현재 경로 종류에는 `mcp_elicitation`, `prompt_capture`,
`local_web_consent`, `cli`가 있습니다. `fallbacks`는 사용할 수 있는 다른 경로를
나열합니다. 사용할 수 있다면 로컬
`volicord inbox answer <judgment-id> --choice <choice>` 명령도 포함합니다.

`recommended_path_kind`, `recommended_path_label`, `recommendation`은 현재 상태
보기에 충분한 정보가 있을 때 선호하는 답변 방법을 알려 줍니다. 사용자가 어디에서
답할 수 있는지 안내할 뿐, 답변을 기록하지는 않습니다.

## `JudgmentBasis`

`JudgmentBasis`는 판단이 현재 요구사항을 만족할 수 있는지 정하는 데 쓰는 Core 파생 상태 스냅샷입니다.

```yaml
JudgmentBasis:
  task_id: string
  change_unit_id: string | null
  scope_revision: integer
  close_basis_revision: integer | null
  baseline_ref: string | null
  result_refs: StateRecordRef[]
  residual_risk_ids: string[]
  sensitive_action_scope: SensitiveActionScope | null
  created_at_state_version: integer
  compatibility_status: string
```

Core는 판단을 만들 때 현재 상태에서 `JudgmentBasis`를 만듭니다. `JudgmentBasis`는 서버가 파생한 지속 상태이며 공개 요청 필드가 아닙니다. 호출자는 `basis`, `scope_revision`, `close_basis_revision`, 현재 닫기 근거 데이터, 세션 바인딩 데이터를 제출하지 않습니다.

`compatibility_status` 값은 [판단 값](schema-value-sets.md#judgment-values)이 담당합니다. `stale`과 `superseded` 판단은 감사에 필요할 때 저장된 채 남을 수 있지만 현재 닫기, 쓰기, 민감 승인 요구사항을 만족하는 데 사용할 수 없습니다.

<a id="userjudgmentcandidate"></a>
## `UserJudgmentCandidate`

`UserJudgmentCandidate`는 제안된 집중 질문의 후보 형태입니다. `judgment_id`, `status`, `resolution`, `created_at`, `resolved_at` 필드가 없습니다.

```yaml
UserJudgmentCandidate:
  judgment_kind: string
  presentation: string
  question: string
  options: UserJudgmentOption[]
  context: UserJudgmentContext
  affected_refs: StateRecordRef[]
  required_for: string[]
  expires_at: string | null
```

<a id="userjudgmentoptioninput"></a>
## 선택지와 맥락 형태

```yaml
UserJudgmentOptionInput:
  option_id: string
  label: string
  description: string
  consequence: string
  is_default: boolean

UserJudgmentOption:
  option_id: string
  label: string
  description: string
  consequence: string
  machine_action: string
  resolution_outcome: string
  is_default: boolean

UserJudgmentContext:
  summary: string
  related_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]
  visible_risks: AcceptedRiskInput[]
  constraints: string[]
```

`option_id`는 그 판단 안에서만 유효합니다. `label`, `description`, `consequence`, `summary`, `constraints` 항목은 자유 형식 표시 문자열입니다. 화면에 보이는 라벨은 표시 텍스트이며 기준 스키마 값이 아닙니다.

`UserJudgmentOptionInput`은 메서드 담당 문서가 호출자 작성 선택지를 허용할 때 쓰는 사용자 지정 선택지 요청 형태입니다. 이 형태에는 `machine_action`이나 `resolution_outcome`이 없습니다. 공개 요청이 `options` 안에 이 필드를 담으면 유효하지 않습니다.

`UserJudgmentOption`은 현재 Core가 소유한 선택지 상태와 출력 형태입니다. 공개 API의
선택지는 `null`이 아닌 `machine_action`과 `resolution_outcome`을 포함합니다.
`machine_action=accept`는 `resolution_outcome=accepted`로,
`machine_action=reject`는 `resolution_outcome=rejected`로 매핑됩니다.
`machine_action=defer`는 메서드나 의미 담당 문서가 연기를 허용하는 곳에서만
`resolution_outcome=deferred`로 매핑됩니다. `blocked`는
`JudgmentResolutionOutcome` 값이 아닙니다.

권한 효력이 있는 판단 종류에서 호출자는 요청 입력에 보이는 라벨과 기계 결과 사이의 매핑을 작성하지 않습니다. Core가 권한 선택지의 동작, 결과, 현지화된 라벨, 결과 설명을 만듭니다. 선택지 라벨이나 설명 문구가 기계 판독 가능한 동작이나 결과를 뒤집으면 안 됩니다. 지속 선택지 상태는 명시적인 동작과 결과 필드가 있는 현재 구조화된 선택지 객체를 사용합니다.

<a id="resolution-and-answer-payload"></a>
## 판단 결과와 답변 요청 본문

```yaml
UserJudgmentResolution:
  selected_option_id: string
  machine_action: string
  resolution_outcome: string
  answer: RecordUserJudgmentPayload
  rationale: JudgmentRationale
  note: string | null
  accepted_risks: AcceptedRiskInput[]
  resolved_by_actor_source: string

RecordUserJudgmentPayload:
  product_decision: object | null
  technical_decision: object | null
  scope_decision: object | null
  sensitive_action_scope: SensitiveActionScope | null
  final_acceptance: object | null
  residual_risk_acceptance: object | null
  cancellation: object | null

JudgmentRationale:
  summary: string
  selected_reason: string | null
  considered_alternatives: string[]
  rejected_alternatives: string[]
  assumptions: string[]
  tradeoffs: string[]
  uncertainties: string[]
  review_triggers: string[]
  related_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]
```

`selected_option_id`, `rationale`, `note`는 요청 수준이자 해결 수준의 필드입니다. `selected_option_id`는 판단 선택지 집합 안에서만 유효합니다. `note`는 자유 형식 표시 문자열입니다.

`JudgmentRationale.summary`는 필수이며 간결하지만 비어 있으면 안 됩니다. `selected_reason`, 대안, 가정, 절충, 불확실성, 검토 트리거, 관련 참조, 아티팩트 참조는 사용자에게 보이는 의도와 검토 맥락을 보존합니다. 수락된 제품 판단, 기술 판단, 범위 판단, 최종 수락, 취소, 민감 승인, 잔여 위험 수락에는 비어 있지 않은 `selected_reason`과 `tradeoffs`, `review_triggers` 항목이 각각 하나 이상 필요합니다. 거절되거나 연기된 판단은 메서드 담당 문서가 더 많은 세부사항을 요구하지 않는 한 간결한 근거를 사용할 수 있습니다.

`machine_action`과 `resolution_outcome`은 선택된 `UserJudgmentOption`에서 복사됩니다. 선택된 선택지의 저장 동작과 결과가 기준이며 동작/결과 매핑과 일치해야 합니다. `answer` 안의 결과, 결정, 수락 필드는 선택된 선택지와 일치해야 합니다. 자유 형식 답변 텍스트는 권한을 부여할 수 없습니다.

판단 이유 텍스트는 권한을 부여하거나, 쓰기 티켓을 만들거나, 증거 요구사항을 만족하거나, 최종 수락을 성립시키거나, 잔여 위험을 수락하거나, 오래된 판단을 현재 것으로 만들거나, 어떤 선택지가 선택되었는지를 바꿀 수 없습니다.

`resolved_by_actor_source`는 `ActorSource` 값 집합을 사용합니다. [행위자 출처
값](schema-value-sets.md#actor-source-values)을 보세요. 이 필드는 자유 형식 호출자
귀속이 아니라 파생된 출처를 기록합니다. 사용자 판단을 권한 효력이 있는 결과로
기록하려면 호환 User Channel 출처와 함께
`resolved_by_actor_source=local_user`가 필요합니다.

권한 효력이 있는 판단 해결 규칙:
- `judgment_kind=scope_decision`, `final_acceptance`, `residual_risk_acceptance`, `sensitive_approval`, `cancellation`은 현재 권한 요구사항을 만족하려면 선택된 Core 생성 권한 선택지, `machine_action=accept`, `resolution_outcome=accepted`, `resolved_by_actor_source=local_user`, 호환 User Channel 출처, 호환되는 현재 근거가 필요합니다.
- `resolution_outcome=rejected` 또는 `deferred`는 지속되는 사용자 결정이지만 어떤 것도 승인, 수락, 권한 부여, 면제, 닫기를 만들지 않습니다. `blocked`는 판단 결과가 아니며 권한 요구사항을 만족할 수 없습니다.
- 기계 판독 가능한 동작이나 결과 또는 필요한 User Channel 출처가 없는 결과 기록은 유효하지 않은 소유자 상태이며 현재 권한 요구사항을 만족할 수 없습니다.

형태 규칙:
- 선택된 `judgment_kind`에 맞는 판단별 요청 본문 분기 하나만 채웁니다.

담당 문서 예외:
- 메서드 담당 문서가 더 좁은 요청 본문 구조를 명시적으로 정의할 수 있습니다.

판단별 요청 본문 객체 안의 문자열 필드는 메서드 담당 문서가 더 좁은 로컬 코드 목록이나 값 목록을 명시적으로 정의하지 않는 한 그 요청 본문 구조 안에서만 유효합니다. 전역 API 값 집합이 아닙니다.

허용되지 않는 것:
- `RecordUserJudgmentPayload`에는 `selected_option_id`, `rationale`, `note`가 없습니다.

## `SensitiveActionScope`

`SensitiveActionScope`는 이름 붙은 민감 동작 승인 맥락의 스키마 형태입니다. `WriteTicketAttemptScope`도 아니고, 쓰기 티켓도 아니며, 보안 권한도 아닙니다. [보안](../security.md)을 확인하세요.

```yaml
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

`SensitiveActionScope`의 존재는 민감 동작 승인이 필요한 위치를 정의하지 않습니다. 이 형태가 나타나는 위치는 메서드 담당 문서가 정의하며, 제품 파일 쓰기에 대한 `volicord.prepare_write` 경로를 대신하지 않습니다.

`SensitiveActionScope.action_kind`와 `sensitive_categories[]`는 영향받는 메서드나 프로필 담당 문서가 더 좁은 로컬 목록을 공개하지 않는 한 불투명 민감 동작 분류 문자열입니다. `description`, `command_or_tool_summary`, `network_or_host_summary`, `secret_or_credential_summary`, `capability_claim`은 표시 또는 주장 문자열입니다. 기준 값 집합이나 보안 권한이 아닙니다.

`volicord.request_user_judgment`에서 `sensitive_action_scope`는 `null`을 허용하는
선택적 공개 요청 필드입니다. `judgment_kind=sensitive_approval`일 때 `null`이 아닌
값이 필요한지는 메서드 담당 문서가 정의합니다. `SensitiveActionScope`가
`JudgmentBasis` 안에 나타날 때는 서버가 파생한 지속 상태이며, 호출자가 제출한
근거 데이터가 아닙니다.

<a id="acceptedriskinput"></a>
## `AcceptedRiskInput`

`AcceptedRiskInput`은 판단 요청 본문 안에서 보이는 잔여 위험의 이름을 담는 형태입니다.

```yaml
AcceptedRiskInput:
  risk_id: string
  summary: string
  consequence: string
  related_refs: StateRecordRef[]
  accepted_for_close: boolean
```

이 형태는 검증, 증거 충분성, QA, 최종 수락, 결과에 위험이 없다는 증명이 아닙니다. 잔여 위험의 의미는 [Core 모델](../core-model.md)이 담당합니다.

`risk_id`는 현재 닫기 근거에 있는 정확한 불투명 위험 식별자입니다. 닫기를 위해 잔여 위험을 수락할 때 필수입니다. `summary`, `consequence`, `related_refs`는 사용자와 감사 기록을 위한 맥락이며 텍스트 일치를 권한으로 만들지 않습니다.

## 관련 담당 문서

- [Core 모델](../core-model.md): 사용자 소유 판단 의미와 비대체 규칙.
- [사용자 소유 판단 요청 메서드](method-request-user-judgment.md): `volicord.request_user_judgment`.
- [사용자 소유 판단 기록 메서드](method-record-user-judgment.md): `volicord.record_user_judgment`.
- [API 값 집합](schema-value-sets.md): `judgment_kind`, `presentation`, `required_for`, 상태, 행위자 값, 선택지 표시 경계.
- [API 상태 스키마](schema-state.md): `StateRecordRef`.
- [API 아티팩트 스키마](schema-artifacts.md): `ArtifactRef`.
- [범위 참조](../scope.md): 예약된 판단 경로와 기준 범위 경계 확인.
