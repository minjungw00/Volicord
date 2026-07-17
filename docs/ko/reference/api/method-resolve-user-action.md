<a id="volicordresolve_user_action"></a>

# `volicord.resolve_user_action` 참조

이 문서는 대기 `UserActionRequest`를 해결하는 유일한 공개 전이를 담당합니다. 직접
`User Channel` 메서드이며 Agent Connection MCP tool로 노출되지 않습니다.

## 요청

```yaml
ResolveUserActionRequest:
  envelope: ToolEnvelope
  user_action_request_id: string
  resolution: UserActionResolutionInput
  channel_submission_id: string
```

`resolution`은 `resolution_type=choice|evidence_observation`을 쓰는 폐쇄형 tagged
union이며 저장 요청 계열과 일치해야 합니다. 판단 해결 입력은 선택한 저장 선택지 ID와
선택적 사용자 note만 담습니다. Core는 저장 선택지에서 machine action과 outcome을
복사하고 저장 요청과 호환 근거에서 현재 수락 잔여 위험 ID를 도출합니다.
`judgment_kind`, 영속 `action_kind`, 민감 범위, 다른 권위 좌표는 요청 소유로 남습니다.
호출자는 rationale, risk 객체나 ID, answer branch, machine action, outcome을 제출할 수
없으며 Core는 사용자 rationale이나 합성 answer를 만들어 내지 않습니다. Evidence 관찰
입력은 사용자가 선택한 저장 대상, 선택한 후보 아티팩트 ID,
`supported` 또는 `contradicted`, 사용자 summary를 담으며 `observed_at`은 담지
않습니다.

호출은 인식된 User Channel verification basis와 함께 서버가 파생한
`actor_source=local_user`, `operation_category=user_only`여야 합니다. 요청은 저장
근거에 대해 유효하게 `pending`이고 만료되지 않았으며 현재 상태여야 합니다. Core는
커밋 전에 canonical 캡처 폼, 후보 집합, 현재 대상, 정확한 아티팩트 bytes, Task와
Change Unit, scope, baseline, 해당할 때의 닫기 근거를 다시 읽습니다.

채널 어댑터는 `ToolEnvelope.expected_state_version`을 명시적인 `null`로 보냅니다.
required-nullable envelope 필드이므로 생략은 유효하지 않습니다. host나 사용자가 요청 생성 시점 state version을 추측하지 않습니다. Core가
해결 preflight에서 현재 state version을 고정합니다. 커밋 전 상태가 바뀌면 transaction은
`STATE_VERSION_CONFLICT`를 반환합니다. 의미 freshness는 어댑터 제공 version이 아니라
근거 evaluator에서 계속 나옵니다.

`channel_submission_id`는 1~256 bytes의 불투명 채널 identity입니다. 모든 byte가
visible ASCII `0x21..=0x7e`여야 하며 공백, NUL, non-ASCII, 빈 값, 더 긴 값은
거부됩니다. Envelope의 `idempotency_key`는 이 값과 정확히 같아야 합니다. 로컬 CLI 어댑터가 identity를 만듭니다. 같은 요청, 채널, actor context,
submission ID, canonical 해결의 replay는 원래 커밋 응답을 반환합니다. 다른 해결로
재사용하면 거부합니다. 동시에 들어온 서로 다른 제출도 두 번째 해결을 만들 수
없습니다.

로컬 CLI 어댑터만 이 메서드에 진입할 수 있습니다. Agent Connection, MCP 어댑터,
Guard 관찰, 직접 Store 호출자는 resolution을 제출할 수 없습니다.

### 동작 시각

Core는 공통 preflight 뒤 해결 동작을 위해 프로젝트의 정규 Core UTC 시계를 정확히 한
번 샘플링합니다. 이 `operation_now`를 유효 요청 상태 파생, 요청 expiry 확인, 공개 및 저장
`resolved_at` 설정에 다시 사용합니다.

Core transaction은 더 늦은 정규 커밋 timestamp를 선택할 수 있지만 의미 있는
`resolved_at` 샘플을 바꾸면 안 됩니다. Transaction timestamp와 영속 하한 규칙은
[저장소 버전 관리](../storage-versioning.md#canonical-core-utc-clock)가 담당합니다.

## 결과와 효과

```yaml
ResolveUserActionResult:
  base: ToolResultBase
  user_action_request_ref: StateRecordRef
  user_action_resolution_ref: StateRecordRef
  user_action_request: UserActionRequest
  user_action_resolution: UserActionResolution
  derived_refs: StateRecordRef[]
  state: StateSummary
  next_actions: NextActionSummary[]
```

커밋은 적용되는 선택 또는 증거 관찰 본문을 닫힌 `resolution_json`에 담은 변경 불가능한
`user_action_resolutions` 행 하나를 삽입하여 공통 유효 상태 evaluator가 `resolved`를
반환하게 하고, 의존 blocker와 Task lifecycle을 갱신하고, authority event
하나와 user-only replay 결과를 저장하고, `state_version`을 한 번 증가시키는 작업을
원자적으로 수행합니다. 판단 해결은 담당자가 선택한 continuity record를 만들 수
있습니다. 증거 관찰 해결은 증거 coverage나 Run을 만들지 않습니다. 이후
`record_run`이 정확한 `user_action_resolution_ref`와 선택 아티팩트를 참조해야 합니다.
사용자 행동에서 파생된 모든 continuity record는 `rationale=null`을 저장합니다. 선택적
비공개 사용자 note는 변경 불가능한 resolution 본문에만 남으며 continuity, agent-safe,
status, export 파생, diagnostic projection으로 복사되면 안 됩니다.

Core는 바깥 resolution에 resolution 캡처 시각 하나를 제공합니다. 중첩 Evidence 관찰
본문에는 중복 identity나 timestamp가 없습니다. `status=resolved` 자체는
수락이 아닙니다. 판단 권한은 저장 선택지 action/outcome과 현재 근거에서만 나오고,
관찰 권한은 정확한 현재 `evidence_observation` 해결 본문에서만 나오며 최종 수락이나
다른 판단이 아닙니다.

Dry run, 잘못되거나 혼합된 payload, Agent Connection actor, stale 또는 superseded
근거, `now >= expires_at`, 바뀐 후보나 bytes, replay 충돌, 잘못된 채널 binding은
요청, 해결, event, replay, blocker, lifecycle, state version 효과 없이
거부됩니다. 정확한 replay, dry run, 거부, expiry 상태
갱신은 영속 정규 UTC 하한을 갱신하지 않습니다.

## 관련 담당 문서

- 공통 형태와 유효 상태: [API 사용자 행동 스키마](schema-user-action.md).
- 권한 의미: [Core 모델](../core-model.md).
- 효과: [저장소 효과](../storage-effects.md#volicordresolve_user_action).
- 채널 동작: [MCP 전송](../mcp-transport.md), [관리 CLI](../admin-cli.md).
- 시계 영속 규칙: [저장소 버전 관리](../storage-versioning.md#canonical-core-utc-clock).
