<a id="volicordrequest_user_action"></a>

# `volicord.request_user_action` 참조

이 문서는 대기 `UserActionRequest` 하나를 만드는 agent-workflow 메서드를
담당합니다. 일곱 판단 종류와 `evidence_observation`에 대한 유일한 공개 요청
메서드입니다.

## 요청

```yaml
RequestUserActionRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  action: UserActionDraft
  required_for: string[]
  expires_at: string | null
```

`action`은 폐쇄형 `UserActionDraft` union입니다. 판별자는
`action_type=choice|evidence_observation`이며 choice variant는 중첩된 7개 값의
`judgment_kind`를 담습니다. 호출자는 8개 값의 영속 `action_kind`를 별도로 제출할 수
없습니다. Choice payload는 [API 판단 스키마](schema-judgment.md)가 담당합니다. Evidence
관찰 variant는 다음을 담습니다.

```yaml
UserActionEvidenceObservationDraft:
  action_type: evidence_observation
  question: string
  context_summary: string
  target_candidates: EvidenceTarget[]
  artifact_candidate_ids: string[]
```

이 메서드는 `operation_category=agent_workflow`, 필요한 경우 현재 호환 Task와 Change
Unit, 커밋 요청 idempotency key, 현재 `expected_state_version`을 요구합니다. 에이전트는
사용자에게 보이는 후보를 제안하지만 해결, 사용자 actor, relevance 판정, 선택한
선택지·대상·아티팩트, 캡처 시각을 제출할 수 없습니다.

`change_unit_id`, `required_for`, `expires_at`은 공통 요청 필드입니다. Evidence 관찰
요청에서 `expires_at`은 null이어야 하며 Core가 15분 만료를 부여합니다. 관찰 요청에서
Core는 현재 대상 식별자와 정확한 지속 아티팩트 bytes를 확인하고 후보
참조를 정규화하며 Task, Change Unit, scope revision, baseline, state version을
캡처하고 15분 만료를 설정합니다. 판단 요청에서는 판단 담당 계약에 따라 판단 근거와
Core 소유 권한 선택지를 파생합니다.

### 동작 시각

Core는 공통 preflight 뒤 이 준비된 동작을 위해 프로젝트의 정규 Core UTC 시계를
정확히 한 번 샘플링합니다. 이 `operation_now`는 모든 현재 시각 판단, 공개 요청의
`created_at`, 저장 `requested_at`, 명시적 choice expiry 검증, 파생 15분 Evidence 관찰
expiry에 사용됩니다. 호스트 timestamp나 호출자 시계는 이러한 판단의 입력이 아닙니다.

Null이 아닌 호출자 제공 choice `expires_at`은 정규 4자리 RFC 3339 UTC 표현이 가능한
시각으로 정규화되어야 하고 `operation_now`보다 늦어야 합니다. Core는 dry-run 계획과
커밋 계획 전에 같은 검증을 적용합니다. 표현할 수 없는 명시적 expiry는 요청, event,
replay 행, state-version 변경, 영속 시계 하한 갱신 없이 거부됩니다.

15분 파생과 다른 모든 담당 TTL은 checked timestamp 덧셈을 사용하고 정규 RFC 3339 UTC
결과를 요구합니다. Overflow 또는 표현 불가능한 결과는 요청, event, replay 행,
state-version 변경, 영속 시계 하한 갱신 없이 커밋 전에 거부됩니다.

이후 Core transaction은 `operation_now`보다 이르지 않은 정규 커밋 timestamp를
선택합니다. 더 늦을 수 있지만 요청의 의미 있는 생성·요청 시각을 다시 쓰지 않습니다.
커밋 시각 저장소 metadata는
[저장소 버전 관리](../storage-versioning.md#canonical-core-utc-clock)를 따릅니다.

### MCP create와 resume 연산

MCP에 보이는 어댑터 인자는 생성과 연속 작업을 하나의 엄격한 중첩 operation union으로
감쌉니다. 위의 create-only Core 요청 형태는 바꾸지 않습니다.

```yaml
McpRequestUserActionArguments:
  project_selector: string | null
  detail: summary | workflow | full
  request:
    # create variant
    operation: create
    task_id: string
    change_unit_id: string | null
    action: UserActionDraft
    required_for: string[]
    expires_at: string | null

    # resume variant
    operation: resume
    user_action_request_id: string
```

정확히 한 variant만 허용합니다. operation 누락·미지원 값, 평면 create 필드, create와
resume 필드 혼합은 Core 전에 거부됩니다. create variant는 완전한
`RequestUserActionRequest`를 만들며 프로젝트 상태에 쓸 수 있어야 합니다.

resume variant는 두 번째 공개 mutation이 아니라 읽기 전용 연속 작업입니다.
`volicord.request_user_action`이 직접 만든 요청만 가리킬 수 있고, 같은 활성 workflow
Agent Connection actor 범위와 허용 프로젝트를 요구하며, 같은 `operation_result_ref`와 함께
byte 단위로 정확한 원래 agent-safe Agent Workflow 응답을 재생합니다. 재생 결과는 정규
요청 요약만 담고 전체 요청, inbox 항목, 캡처 폼, 캡처 경로, User Channel credential을
담지 않습니다. 요청, event, replay 행, prompt, token, resolution, state-version 증가를
만들지 않습니다. 다른 connection 또는 `volicord.reconcile_changes`가 만든 요청은 이
분기에서 사용할 수 없습니다. 이후 관계없는 Git 변경이나 authority 상태 변경도 과거
결과를 다시 쓰거나 무효화하지 않습니다. 어떤 중첩 단계의 중복 JSON object member, non-result branch, commit 좌표 불일치,
그 밖의 현재 폐쇄 result 계약 위반을 담은 저장 응답은 재생하지 않고 corrupt로
처리합니다. Resume은 직접 replay와 같은 원문 committed-result gate를 적용하며 gate가
실패하면 저장 byte 없이 `PERSISTED_DATA_CORRUPT`를 반환합니다.

create 또는 resume 뒤 어댑터는 Core에 별도 현재 agent-safe projection을 요청합니다.
Core는 상태, 선택적 안전 resolution, 정확한 과거 resolution 파생 ref, 관찰 anchor를 한
SQLite 읽기 snapshot에서 읽습니다. 어댑터는 별도의 입력 표면을 열지 않고 이
projection을 반환합니다. 대기 중인 행동은 `volicord inbox`에서만 전달하고
해결합니다. 정확한 결과는 resume일 때만
`agent_workflow_result_replayed=true`로 표시됩니다.
`current_projection_observed_at`은 그 읽기 snapshot의 정규 Core 시각 샘플이며
projection을 읽었다는 이유만으로 영속화하지 않습니다.

## 결과와 효과

```yaml
RequestUserActionResult:
  base: ToolResultBase
  user_action_request_summary: AgentSafeUserActionRequestSummary
  blocker_refs: StateRecordRef[]
  state: StateSummary
```

커밋 호출은 `user_action_requests` 행 하나를 삽입하고 authority event 하나와 정확한
replay 결과를 저장하며 `state_version`을 한 번 증가시킵니다. 현재 유효한 대기 요청에
정보성이 아닌 `required_for`가 있으면 현재 종료되지 않은 Task를 `waiting_user`로
바꿀 수 있습니다. 멱등 replay는 저장 요청을 다시 정규화하지 않고 원래 agent-safe 요청
요약을 반환합니다. 이 요약은 요청 ID, 과거 `pending` 상태, `next_actor=user`만 담고 ref,
행동 종류, 만료 시각, 질문, 맥락, 본문, 근거, 후보, 폼, 채널 경로, 명령, URL,
credential은 생략합니다. 영속 정규 UTC 하한도 갱신하지 않습니다.

Dry run은 지속 ref를 반환하지 않고 효과가 없습니다. 유효하지 않은 후보, 크기 초과
폼, stale 상태, 잘못된 operation category, 비호환 근거, 사용할 수 없는 아티팩트
bytes, 지원하지 않는 tagged payload는 커밋 전에 거부됩니다.
Dry run과 거부 어느 쪽도 영속 정규 UTC 하한을 갱신하지 않습니다.

MCP는 쓸 수 있는 `workflow` Agent Connection에 create를 노출합니다. 프로젝트 저장소가
읽기 가능 전용으로 저하된 workflow connection도 resume은 발견하고 사용할 수 있지만
create는 Core mutation 전에 거부됩니다. `read_only` Agent Connection은 어느 분기도 쓸
수 없습니다. 어댑터는 새로 생성되어 계속 pending인 요청에만 canonical 캡처 폼을 지원
User Channel로 렌더링할 수 있으며 Agent Connection 호출 자체는 행동을 해결하지
않고 그 폼도 받지 않습니다. Resume은 정확한 안전 replay와 현재 안전 projection만
반환하고 User Channel을 열지 않습니다.

## 관련 담당 문서

- 공통 형태와 제한: [API 사용자 행동 스키마](schema-user-action.md).
- 판단 payload: [API 판단 스키마](schema-judgment.md).
- 해결: [`volicord.resolve_user_action`](method-resolve-user-action.md).
- 효과: [저장소 효과](../storage-effects.md#volicordrequest_user_action).
- 시계 영속 규칙: [저장소 버전 관리](../storage-versioning.md#canonical-core-utc-clock).
