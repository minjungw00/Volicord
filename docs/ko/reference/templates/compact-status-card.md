# Compact Status Card Template

## 권한 규칙

- Projection은 Core가 소유한 상태 기록과 아티팩트 참조에서 파생됩니다.
- Projection은 Core 상태가 아닙니다.
- 사용자가 Projection을 편집해도 그 내용이 자동으로 받아들여진 상태가 되지는 않습니다.
- Chat과 Markdown은 Core 상태를 덮어쓸 수 없습니다.

## 사용 시점

현재 Core 상태를 사용자가 읽기 쉽게, 또는 에이전트가 간결하게 참고할 수 있게 보여줄 때 Compact Status Card를 사용합니다. 이 card는 v0.2 MVP projection shape입니다. Core 상태와 ref에서 파생한 하나의 작은 card입니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: v0.2 User-Facing Harness MVP projection입니다. v0.1 Core status output은 이 card 대신 plain structured status/blocker output을 반환해도 됩니다. 이 template은 persisted state record가 아니며 full projection renderer support의 증거도 아닙니다.

Card는 평범한 말을 먼저 쓰고, 정확한 Harness label은 권한 경계를 분명히 할 때만 붙입니다. Status, 다음 행동, 이어가기 턴에서 부담 없이 읽을 만큼 작아야 합니다.

## 필수 내용

- 무엇을 하고 있는지
- 현재 범위와 하지 않을 일
- 대기 중인 사용자 판단
- 알려진 근거 또는 근거 gap
- close blocker
- 보이는 잔여 위험
- 다음 안전한 행동
- source/freshness ref

## 기준 기록과 ref

- 현재 Task 상태와 lifecycle phase
- 관련 있을 때 현재 scope, non-goal, active Change Unit summary
- 대기 중인 Decision Packet refs와 compact judgment summary
- 알려진 evidence refs와 evidence-gap summary
- close blocker refs와 있는 경우 close reason
- residual-risk refs 또는 명시적인 absence / 아직 보이지 않음 상태
- 현재 Core 상태에서 나온 next safe action
- 읽기용 보기 최신성(projection freshness)과 `source_state_version`
- 표시되는 claim을 뒷받침할 때 필요한 artifact refs와 redaction state
- 해당 claim이 card에 나타날 때만 Write Authorization, Approval, Evidence Manifest, Eval, 수동 QA, Acceptance Decision Packet, Residual Risk를 위한 optional authority refs

이 card의 summary placeholder는 위 기록에서 파생한 표시 binding입니다. Decision, evidence, close-blocker, residual-risk, freshness summary는 ref 또는 명시적인 absence를 보여줘야 하며 사용자 판단 맥락이나 권한을 만들지 않습니다.

Card에는 schema dump, DDL, event log, full artifact, full reference doc, full Evidence Manifest, full Eval body, full 수동 QA record, report body를 넣지 않습니다.

## 사용자 대상 framing

사용자가 읽을 때는 이 shape를 사용합니다. 각 줄은 짧고 쉽게 씁니다.

````text
TASK-{id} {title}
표시 전용: Core 상태와 ref에서 파생된 보기이며 Core 상태나 쓰기 권한이 아닙니다.
하는 일: {doing_summary}
현재 범위: {scope_summary|none}
하지 않을 일: {non_goals_summary|none}
대기 중인 사용자 판단: {pending_user_judgments_summary|none}
근거: {known_evidence_summary|none}
근거 gap: {evidence_gaps_summary|none}
Close blocker: {close_blockers_summary|none}
보이는 잔여 위험: {residual_risk_summary|none}
다음 안전한 행동: {next_safe_action}
Source/freshness: state={source_state_version|unknown}; refs={source_refs_summary|none}; rendered={updated_at|unknown}; freshness={projection_freshness}
````

## Agent compact framing

소비자가 agent context/reference payload일 때는 이 shape를 사용합니다. Public schema가 아니라 compact 목표를 보여주는 예시입니다.

````yaml
task: {task_id}
title: {title}
mode: {mode}
phase: {lifecycle_phase}
doing: {doing_summary}
scope_ref: {scope_ref|none}
non_goals_ref: {non_goals_ref|none}
pending_judgment_refs: {decision_packet_refs|none}
evidence_refs: {evidence_refs|none}
evidence_gaps: {evidence_gaps_summary|none}
close_blocker_refs: {close_blocker_refs|none}
residual_risk_refs: {residual_risk_refs|none}
next_safe_action: {next_safe_action}
freshness:
  source_state_version: {source_state_version|unknown}
  rendered_at: {updated_at|unknown}
  state: {current|stale|failed|unknown}
````

## 메모

이 template은 렌더링 결과인 카드 형태일 뿐 기준 상태가 아닙니다. Current source record와 ref에서 렌더링되며, 오래된 chat memory에서 렌더링하지 않습니다. Gate value는 기준 상태가 계속 담당하고, projection freshness는 읽기용 보기의 최신성만 뜻합니다. 정확한 권한 없음 규칙은 [projection/report 경계](../document-projection.md#projection-principles)를 사용합니다.

이 card의 status/next recommendation은 read-only guidance입니다. Decision Packet, `prepare_write`, 근거 수집, 검증, 수동 QA, reconcile, close attempt를 가리킬 수는 있지만, state를 mutate하거나, write를 허가하거나, gate를 충족하거나, 작업 수락을 기록하거나, 잔여 위험을 받아들이거나, Task를 close하지 않습니다.

Authority line은 refs-first여야 합니다. Card가 write allowed라고 말하면 Write Authorization ref를 cite합니다. 민감 동작 permission이 granted라고 말하면 Approval ref를 cite합니다. 근거가 sufficient라고 말하면 Evidence Manifest ref를 cite합니다. 분리 검증이 passed라고 말하면 Eval ref를 cite합니다. 수동 QA가 passed 또는 waived라고 말하면 수동 QA record 또는 waiver path를 cite합니다. 작업 수락이 recorded라고 말하면 Acceptance Decision Packet을 cite하고, 잔여 위험 수용이 recorded라고 말하면 accepted Residual Risk refs를 cite합니다. Source ref가 없으면 claim을 unsupported 또는 not yet recorded로 렌더링합니다.

Residual-risk display는 `status=none`과 `not_visible`을 구분해야 합니다. `status=none`은 requested action에 알려진 close-relevant 잔여 위험이 없다는 뜻이며 명시적인 empty risk-ref set과 함께 렌더링해야 합니다. `not_visible`은 알려진 close-relevant risk가 있지만 작업 수락 또는 close에 충분히 보이지 않았다는 뜻이므로 blocking risk refs 또는 risk가 hidden인 이유를 설명하는 refs를 보여줘야 합니다.

표시 문제를 한 줄로 뭉개지 않습니다. 오래된 projection(stale projection)은 읽기용 card가 뒤처졌을 수 있다는 뜻입니다. Stale state, baseline, evidence는 실제 입력이 이동했거나 부족해졌다는 뜻입니다. MCP 또는 필요한 기능이 unavailable이면 접점이 필요한 Harness/Core capability에 닿지 못하거나 제공하지 못한다는 뜻입니다.

가장 먼저 해소할 막힘은 API response가 제공하는 primary `ToolError`에서 가져오거나, failed `harness.close_task` response를 렌더링할 때는 첫 close blocker에서 가져와야 합니다. 소유자 라벨은 다음 움직임이 사용자 소유인지, 에이전트가 해소 가능한지, 접점/시스템 소유인지 보여줘야 하며, 가장 먼저 해소할 막힘이 없으면 `none`으로 렌더링하거나 생략합니다. 추가 막힘은 간결하게 묶고, 다음 행동, 닫기 준비 상태, 대기 중인 사용자 판단을 바꿀 때만 보여줍니다. 이 라벨들은 표시 문구일 뿐 새 schema value나 `ErrorCode`가 아닙니다.

이것은 사용자 판단 맥락이 아닙니다. 사용자 판단이 필요하면 `judgment_route`, `display_depth`, display-depth에 맞는 options 또는 chosen outcome, 관련 refs, 그리고 required일 때 higher-depth recommendation, uncertainty, deferral effect가 있는 judgment prompt를 별도로 렌더링합니다.

Close status는 close reason 구분을 보존해야 합니다. `completed_with_risk_accepted`는 accepted 잔여 위험이 있는 successful close로 렌더링하고, ordinary done, verified, self-checked close처럼 보여주면 안 됩니다. Self-checked, `detached_verified`, verification-waived, QA-waived, risk-accepted-close label은 ref 또는 명시적인 absence와 함께 별도 display slot에 둡니다. 작업 수락이 next action이면 별도 작업 수락 prompt가 근거, 검증, 수동 QA, 잔여 위험 표시 또는 `none`, 작업 수락이 대체하지 않는 것을 보여줘야 합니다.

큰 기록은 먼저 참조를 보여주는 방식(refs-first)으로 둡니다. Evidence, Run, Eval, 수동 QA, artifact, log, screenshot, diff, large trace는 기본적으로 본문에 포함하지 않습니다.
