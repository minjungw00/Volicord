# API 사용자 행동 스키마

이 문서는 Agent Connection이 요청하고 `User Channel`을 통해서만 해결하는 사용자
행동의 공통 공개 스키마를 담당합니다. 선택형 판단과 Evidence 관찰은 요청, 근거,
유효 상태, adapter-neutral resolution form, 불변 resolution envelope를 공유하지만
권한 의미는 구분됩니다.

## 폐쇄형 행동 계열

요청 측 `UserActionDraft`는 `action_type`을 판별자로 쓰는 폐쇄형 tagged union입니다.

```yaml
UserActionDraft:
  # choice variant
  action_type: choice
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | final_acceptance | residual_risk_acceptance | cancellation
  presentation: short
  question: string
  options: UserActionOptionInput[] | null
  context: UserActionContext
  affected_refs: StateRecordRef[]
  sensitive_action_scope: SensitiveActionScope | null

  # evidence-observation variant
  action_type: evidence_observation
  question: string
  context_summary: string
  target_candidates: EvidenceTarget[]
  artifact_candidate_ids: string[]
```

variant는 정확히 하나만 있어야 합니다. 알 수 없는 필드·판별자, choice와 관찰 필드의
혼합, 필수 variant 필드 누락은 커밋 전에 거부됩니다. 호출자는 `action_kind`를 제출하지
않습니다. Core가 `action_type`과 `judgment_kind`에서 8개 값의 영속 형태를 도출합니다.

`UserActionRequestBody`는 같은 `action_type`을 쓰는 대응 저장 union입니다. choice
variant는 호출자 선택지를 현재 Core 소유 `UserActionOption[]`으로 바꾸고, 관찰
variant는 아티팩트 ID를 정확한 정규 `artifact_candidates: ArtifactRef[]`로 바꿉니다.
판단별 선택지, context, 민감 범위 형태는 [API 판단 스키마](schema-judgment.md)가
담당합니다.

## 영속 요청과 근거

```yaml
UserActionRequest:
  user_action_request_id: string
  project_id: string
  task_id: string
  change_unit_id: string | null
  action_kind: string
  status: string
  body: UserActionRequestBody
  basis: UserActionBasis
  required_for: string[]
  user_action_resolution_ref: StateRecordRef | null
  expires_at: string | null
  created_at: string

UserActionBasisCoordinates:
  task_id: string
  change_unit_id: string | null
  scope_revision: integer
  baseline_ref: string | null
  created_at_state_version: integer
  compatibility_status: current | stale | superseded

UserActionBasis:
  # choice variant
  action_type: choice
  coordinates: UserActionBasisCoordinates
  close_basis_revision: integer | null
  result_refs: StateRecordRef[]
  residual_risk_ids: string[]
  sensitive_action_scope: SensitiveActionScope | null

  # evidence-observation variant
  action_type: evidence_observation
  coordinates: UserActionBasisCoordinates
  target_candidates: EvidenceTarget[]
  artifact_candidates: ArtifactRef[]
```

Core가 영속 `action_kind`, 근거, 정확한 아티팩트 ref, 호환성을 도출합니다. 호출자는
revision, baseline, canonical ref, 호환성, actor provenance, 캡처 시각을 제출할 수
없습니다.

Choice 요청의 모든 `affected_refs` 항목은 요청 프로젝트에 속해야 합니다. Task 범위
항목은 요청 Task에 속해야 합니다. `affected_refs`는 연산 relevance와 blocker 중첩에
참여하므로 Core가 정규화 전에 이를 검증합니다. `context.related_refs`는 표시와 감사
맥락으로만 남으며 `affected_refs`를 대신하지 않고 연산 blocker 중첩에도 참여하지
않습니다.

`required_for`는 비어 있지 않고 중복이 없으며 행동 종류와 호환되어야 합니다. 이 값은
변경 없이 저장되어 연산 relevance에 참여하며 Store나 어댑터가 항목을 조용히 추가하거나
버리면 안 됩니다.

닫힌 호환 행렬은 다음과 같습니다.

| 행동 종류 | 호환되는 `required_for` 값 |
|---|---|
| `product_decision`, `technical_decision` | `scope_update`, `prepare_write`, `record_run`, `close_complete`, `close_supersede`, `informational` |
| `scope_decision` | `scope_update`, `prepare_write`, `record_run`, `close_complete`, `close_supersede`, `informational` |
| `sensitive_approval` | `prepare_write`, `record_run`, `close_complete`, `close_supersede`, `informational` |
| `final_acceptance`, `residual_risk_acceptance` | `close_complete`, `informational` |
| `cancellation` | `close_cancel`, `informational` |
| `evidence_observation` | `record_run`, `close_complete`, `informational` |

요청 validator와 operation-blocker projection은 이 단일 행렬을 사용합니다.
`informational`은 그 자체로 Task를 대기 상태로 유지하거나 연산을 막지 않습니다.

유효 상태는 `pending`, `resolved`, `stale`, `superseded`, `expired`입니다. 하나의 Core
evaluator가 현재 근거 호환성, 불변 resolution 존재 여부, expiry, 현재 Core 시각에서
상태를 도출합니다. stale 또는 superseded 근거는 저장 resolution보다 우선하며, 현재
요청에 resolution이 있으면 `resolved`입니다. 그 밖에 대기인 현재 요청만 `expired`가 될
수 있습니다. 만료는 `created_at <= now < expires_at`을 사용하므로
`now >= expires_at`이면 해결할 수 없습니다. 조회는 만료 표시를 위해 상태를 변경하지
않습니다.

Choice 요청은 호출자의 명시적인 nullable `expires_at`을 보존합니다. `null`은 시간
deadline이 없다는 뜻이며 근거 무효화는 계속 적용됩니다. Evidence 관찰 요청은 호출자
deadline을 받지 않고 Core가 15분 만료를 부여합니다.

### 정규 시각 샘플링

UserAction lifecycle의 모든 `now`는 호스트나 호출자 시계가 아니라 프로젝트 범위 정규
Core UTC 시계의 샘플입니다. 각 요청 또는 해결 동작은 공통 preflight 뒤
`operation_now`를 정확히 한 번 샘플링하고 그 동작의 모든 상태, 근거, 만료, timestamp
판단에 다시 사용합니다.

- 요청 생성은 이 샘플 하나를 `UserActionRequest.created_at`으로 노출하고
  `user_action_requests.requested_at`으로 저장합니다. Evidence 관찰 만료는 같은 샘플의
  정확히 15분 뒤이며, 명시적인 choice expiry도 이 샘플을 기준으로 검증하고 그 자체가
  정규 4자리 RFC 3339 UTC로 표현 가능한 시각으로 정규화되어야 합니다. Dry run과
  커밋은 명시적 expiry에 같은 검증을 적용합니다. 모든 파생 expiry는 checked 덧셈과
  같은 표현 가능성 규칙을 사용하며 유효하지 않은 명시적 값이나 overflow는 효과 없이
  거부됩니다.
- 해결은 해결 동작 샘플 하나로 유효 상태를 파생하고 요청과 채널을 검증하며
  `UserActionResolution.resolved_at`을 기록합니다.
- `current_projection_observed_at`은 projection이 이름 붙인 읽기 snapshot 하나의 정규
  Core 시각 샘플입니다. 이 값을 관찰하는 것만으로 더 늦은 프로젝트 시각 하한을
  영속화하지 않습니다.

Core 커밋 timestamp는 `operation_now`보다 늦을 수 있지만 담당 문서가 정의한 이러한
의미 있는 timestamp를 다시 쓰면 안 됩니다. 물리 하한과 커밋 규칙은
[저장소 버전 관리](../storage-versioning.md#canonical-core-utc-clock)가 담당합니다.

## Resolution 입력과 불변 본문

`UserActionResolutionInput`은 `resolution_type`을 판별자로 쓰는 별도 폐쇄형 union입니다.

```yaml
UserActionResolutionInput:
  # choice variant
  resolution_type: choice
  selected_option_id: string
  note: string | null

  # evidence-observation variant
  resolution_type: evidence_observation
  target: EvidenceTarget
  artifact_ids: string[]
  relevance_status: supported | contradicted
  summary: string

UserActionResolutionBody:
  # choice variant
  resolution_type: choice
  selected_option_id: string
  machine_action: accept | reject | defer
  resolution_outcome: accepted | rejected | deferred
  note: string | null
  accepted_risk_ids: string[]

  # evidence-observation variant
  resolution_type: evidence_observation
  observation: UserActionEvidenceObservation

UserActionEvidenceObservation:
  target: EvidenceTarget
  output_artifact_refs: ArtifactRef[]
  relevance_status: supported | contradicted
  summary: string
```

Choice 사용자는 저장 선택지 ID와 선택적 note만 제출합니다. Core는 해당 저장 선택지에서
machine action과 outcome을 복사하고 저장 요청과 호환 근거에서 현재 수락 잔여 위험 ID를
도출합니다. 호출자는 `judgment_kind`, `action_kind`, machine action, outcome, risk 객체,
answer branch, 민감 범위, rationale을 제출할 수 없습니다. Core는 캡처하지 않은 rationale
또는 합성 사용자 답변을 만들어 내지 않습니다.

Evidence 관찰에서 사용자는 저장된 대상 후보 하나와 저장된 아티팩트 후보의 비어 있지
않은 고유 부분집합을 고릅니다. Core는 선택을 정규화하고 선택된 각 아티팩트 후보를 현재
권한 효력이 있는 아티팩트 identity와 최신성에 맞춰 검증합니다. 검증을 통과하면 불변
resolution은 중첩 `created_by_run_ref` version을 포함해 저장 요청 후보의 정확한
`ArtifactRef` 값을 보존하며 나중의 현재 상태 보기에서 다시 만들거나 새 version으로
올리지 않습니다. 요청 `UserActionBasis`만
Task, Change Unit, scope, baseline 좌표를 담당합니다. Resolution identity, project/Task,
channel, actor provenance, assurance, verification basis, 캡처 시각은 바깥
`UserActionResolution`에만 있습니다. 중첩 관찰은 선택 대상, artifact ref, relevance,
summary만 담으며 좌표를 중복하거나 고아 observation identity를 만들지 않습니다.

```yaml
UserActionResolution:
  user_action_resolution_id: string
  user_action_request_id: string
  project_id: string
  task_id: string
  action_kind: string
  body: UserActionResolutionBody
  resolved_by_actor_source: local_user
  resolved_verification_basis: cli_direct_user_channel
  resolved_assurance_level: string
  channel_kind: cli
  channel_submission_id: string
  resolved_at: string
```

resolution의 `action_kind`는 요청에서 복사하며 resolution 입력이 아닙니다. `resolved`는
불변 User Channel resolution 하나가 존재한다는 뜻일 뿐입니다. Choice 수락과 Evidence
relevance는 서로 대신하지 않습니다.

## 크기 제한

Choice 선택지, 관찰 대상 후보, 관찰 아티팩트 후보는 각각 최대 32개입니다. 제품·기술
결정은 고유 ID와 최대 하나의 기본값을 가진 비어 있지 않은 호출자 선택지가 필요하고,
권한을 담는 choice 종류는 호출자 선택지를 거부하고 Core 소유 선택지를 사용합니다.
관찰 후보 목록과 선택 아티팩트 목록은 비어 있지 않고 고유해야 합니다. 질문과 context
summary도 공백이면 안 됩니다. 사용자 note 성격 텍스트는 Unicode
scalar value 1,000개, 관찰 `summary`는 4,000개, canonical 직렬화 행동 또는
adapter-neutral resolution form은 32 KiB로 제한됩니다. Core는 요청 커밋과
resolution 커밋 전에 확인하고,
어댑터는 렌더링 또는 수신 전에 다시 확인하며 잘라내어 유효하게 만들지 않습니다.

`ResolveUserActionRequest.channel_submission_id`와 `UserActionResolution`에 보존되는 값은
visible ASCII `0x21..=0x7e` 1~256 bytes입니다. 빈 값, 공백, NUL, non-ASCII, 더 긴 값은
유효하지 않습니다. 공개 JSON Schema는 이에 맞는 비어 있지 않은 최대 길이와
visible-ASCII 형태를 표현하고, Core는 replay 조회나 커밋 전에 정확한 byte 상한을
검증합니다.

<a id="resolution-form"></a>
## Adapter-neutral resolution form

```yaml
AgentSafeUserActionRequestSummary:
  user_action_request_id: string
  status: pending
  next_actor: user

UserActionResolutionForm:
  # choice variant
  form_type: choice
  choices: UserActionResolutionChoice[]
  note_allowed: boolean
  note_max_chars: integer

  # evidence-observation variant
  form_type: evidence_observation
  target_candidates: EvidenceTarget[]
  artifact_candidates: ArtifactRef[]
  relevance_options: [supported, contradicted]
  summary_max_chars: integer

UserActionResolutionChoice:
  choice_id: string
  label: string
  description: string
  consequence: string
  is_default: boolean
```

`AgentSafeUserActionRequestSummary`는 Agent Connection 결과에서 허용되는 유일한 대기
요청 projection입니다. 요청 식별자, 과거 pending 상태, 다음 actor인 사용자만 담습니다.
요청 ref, 행동 종류, 만료 시각, 요청 본문, 근거, 질문, 맥락, 후보, resolution form,
캡처 경로, 명령, URL, User Channel credential은 담지 않습니다. 현재의 pending이 아닌
상태는 이 과거 대기 요약이 아니라 별도로 갱신한 현재 projection에 속합니다.

이는 알 수 없거나 추가된 필드가 없는 정확한 세 필수 필드의 닫힌 객체입니다.
`user_action_request_id`는 비어 있지 않은 크기 제한 identifier 계약을 만족해야 하고
`status`, `next_actor`는 각각 literal `pending`, `user`여야 합니다. 필드가 누락되거나
추가되거나, 타입이나 literal 값이 잘못되면 일반 출력, replay, resume,
operation-result eligibility에서 유효하지 않습니다.

`UserActionResolutionForm`은 저장 요청 본문에서
`UserActionRequestBody.resolution_form()`으로 도출하는 닫힌 의미 projection입니다.
정확한 선택지 또는 Evidence 후보, 폐쇄형 relevance 값, canonical 입력 한도만
복사합니다. Channel availability, CLI label, command, terminal 또는 Markdown
layout, protocol field, credential, adapter status는 담지 않습니다. Adapter는 인자,
산문, adapter 로컬 상태에서 후보를 다시 만들면 안 됩니다.

정확한 CLI inbox 문서, channel availability, capture path, CLI JSON schema는
[관리 CLI](../admin-cli.md#user-channel-commands)가 담당합니다. MCP는 neutral current
fact를 자체 safe protocol projection으로 변환합니다. 요청을 만들거나 재개할 수 있지만
resolution form을 받거나 제출할 수 없습니다.


정확한 MCP 복합 응답, 간결한 projection, 안전한 resolution, wire 직렬화는
[MCP 전송](../mcp-transport.md#user-action-wire-projection)이 담당합니다. 이 공개 schema
담당자는 해당 projection이 소비하는 adapter-neutral 요청, resolution, 참조, 현재 상태
fact만 제공합니다.

## 관련 담당 문서

- [사용자 행동 요청 메서드](method-request-user-action.md).
- [사용자 행동 해결 메서드](method-resolve-user-action.md).
- [API 판단 스키마](schema-judgment.md).
- Evidence 대상과 ref를 담당하는 [API 상태 스키마](schema-state.md).
- 권한과 비대체 의미를 담당하는 [Core 모델](../core-model.md).
- 정규 시계, 영속 하한, 커밋 timestamp를 담당하는
  [저장소 버전 관리](../storage-versioning.md#canonical-core-utc-clock).
