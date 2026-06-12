# API 판단 스키마

이 문서는 기준 범위의 사용자 소유 판단 API 스키마를 담당합니다. 참조 문서일 뿐이며 그 자체로 사용자 결정을 기록하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- `UserJudgment`
- `UserJudgmentCandidate`
- `UserJudgmentOption`
- `UserJudgmentContext`
- `UserJudgmentResolution`
- `RecordUserJudgmentPayload`
- `SensitiveActionScope`
- `AcceptedRiskInput`
- 사용자 소유 판단의 스키마 의미

이 문서는 담당하지 않습니다.

- 사용자 소유 판단의 제품 의미와 비대체 규칙: [Core 모델](../core-model.md)
- 판단 요청과 기록 메서드 동작: [사용자 판단 메서드](method-user-judgment.md)
- 활성 판단 종류 값, 상태 값, 표시 형식 값, 필요 판단 위치 값: [API 값 집합](schema-value-sets.md)
- 최종 수락이나 잔여 위험 수락의 닫기 효과: [Core 모델](../core-model.md), [Task 닫기 메서드](method-close-task.md)
- 판단 누락, 미해결, 거절, 만료에 대한 공개 오류 의미: [API 오류](errors.md)

## 경계

판단 스키마는 사용자가 소유한 선택의 구조를 보존합니다. 넓은 승인이 제품 판단, 기술 판단, 범위 판단, 민감 동작 승인, 최종 수락, 잔여 위험 수락, 취소 판단, 이후 QA 면제, 이후 검증 위험 수락을 대신하게 만들지 않습니다.

`UserJudgmentCandidate`는 대기 중인 판단이 아닙니다.

조건: 대기 중인 `UserJudgment`는 `harness.request_user_judgment`가 커밋된 뒤에만 존재합니다.

효과: 기록된 답변은 해당 대기 중인 판단과 그 `judgment_kind`만 해결합니다.

비주장:
- 활성 범위를 조용히 바꾸지 않습니다.
- 증거를 만들지 않습니다.
- Write Authorization을 만들지 않습니다.
- 잔여 위험을 수락하지 않습니다.
- Task를 닫지 않습니다.

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
  required_for: string
  resolution: UserJudgmentResolution | null
  expires_at: string | null
  created_at: string
  resolved_at: string | null
```

`judgment_kind`, `status`, `presentation`, `required_for` 값은 [판단 값](schema-value-sets.md#판단-값)이 담당합니다. 제품 의미는 [Core 모델의 사용자 소유 판단](../core-model.md#4-사용자-소유-판단)이 담당합니다.

## `UserJudgmentCandidate`

`UserJudgmentCandidate`는 다음 안전한 경로에 사용자 소유 판단이 필요할 때 다른 메서드가 반환하는 집중된 질문 후보입니다. 화면에 보여 줄 수 있지만, `harness.request_user_judgment`가 커밋하기 전까지 지속 판단이 아닙니다.

```yaml
UserJudgmentCandidate:
  judgment_kind: string
  presentation: string
  question: string
  options: UserJudgmentOption[]
  context: UserJudgmentContext
  affected_refs: StateRecordRef[]
  required_for: string
  expires_at: string | null
```

## 선택지와 맥락 형태

```yaml
UserJudgmentOption:
  option_id: string
  label: string
  description: string
  consequence: string
  is_default: boolean

UserJudgmentContext:
  summary: string
  related_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]
  visible_risks: AcceptedRiskInput[]
  constraints: string[]
```

`option_id`는 해당 판단 안에서만 유효합니다. 화면에 보이는 라벨은 표시 텍스트이며 기준 스키마 값이 아닙니다.

## 해결과 답변 요청 본문

```yaml
UserJudgmentResolution:
  selected_option_id: string
  answer: RecordUserJudgmentPayload
  note: string | null
  accepted_risks: AcceptedRiskInput[]
  resolved_by_actor_kind: string

RecordUserJudgmentPayload:
  product_decision: object | null
  technical_decision: object | null
  scope_decision: object | null
  sensitive_action_scope: SensitiveActionScope | null
  final_acceptance: object | null
  residual_risk_acceptance: object | null
  cancellation: object | null
```

`selected_option_id`와 `note`는 요청 수준이자 해결 수준의 필드입니다. `RecordUserJudgmentPayload`는 이 둘을 반복하면 안 됩니다. 메서드 담당 문서가 더 좁은 구조를 명시적으로 허용하지 않는 한 활성 `judgment_kind`에 맞는 판단별 요청 본문 분기 하나만 채워야 합니다.

## `SensitiveActionScope`

`SensitiveActionScope`는 사용자가 승인할지 판단해야 하는 이름 붙은 민감 단계를 설명합니다. `AuthorizedAttemptScope`도 아니고, Write Authorization도 아니며, 보안 권한도 아닙니다. [보안](../security.md)을 확인하세요.

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

민감 동작 승인은 쓰기 호환성, Run 기록, 닫기 전에 필요할 수 있습니다. 하지만 제품 파일 쓰기에 대한 `harness.prepare_write` 경로를 대신하지 않습니다.

## `AcceptedRiskInput`

`AcceptedRiskInput`은 기록하려는 판단 안에서 사용자가 수락할 수 있는 보이는 잔여 위험을 이름 붙입니다.

```yaml
AcceptedRiskInput:
  risk_id: string | null
  summary: string
  consequence: string
  related_refs: StateRecordRef[]
  accepted_for_close: boolean
```

수락된 위험은 이름 붙은 보이는 위험과 요청된 판단에만 적용됩니다. 검증, 증거 충분성, QA, 최종 수락, 결과에 위험이 없다는 증명이 아닙니다.

## 관련 담당 문서

- [Core 모델](../core-model.md): 사용자 소유 판단 의미와 비대체 규칙.
- [사용자 판단 메서드](method-user-judgment.md): `harness.request_user_judgment`, `harness.record_user_judgment`.
- [API 값 집합](schema-value-sets.md): `judgment_kind`, `presentation`, `required_for`, 상태, 선택지 표시 경계.
- [API 상태 스키마](schema-state.md): `StateRecordRef`.
- [API 아티팩트 스키마](schema-artifacts.md): `ArtifactRef`.
- [범위 참조](../scope.md): 예약된 판단 경로와 활성 경계 확인.
