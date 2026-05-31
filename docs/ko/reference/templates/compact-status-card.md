# Compact Status Card Template

## 사용 시점

매 턴 유지되는 Harness 맥락 묶음(context envelope)을 짧은 현재 상태 표시로 보여줄 때 Compact Status Card를 사용합니다. 여기에는 Task, 모드, 범위, 범위 밖, 다음 안전한 행동, 막힘 상태, 대기 중인 사용자 결정, 쓰기 권한, 수용 기준, 근거, 검증, 수동 QA, 잔여 위험, 보장 수준(guarantee level), 읽기용 보기 최신성(projection freshness), 최신 참조(ref)가 포함됩니다. 상태 확인, 다음 행동, 이어가기 턴에서 부담 없이 읽을 수 있게 유지하고, 평범한 상태 설명을 먼저 쓰며 정확한 Harness label은 경계를 분명히 할 때만 붙입니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 코어 권한 조각(v0.1 Core Authority Slice)에 필요한 최소 read-only status/next/blocker shape입니다. Card 또는 response text로 반환할 수 있으며 persisted state record가 아닙니다.

## 기준 기록

- 현재 Task 상태와 lifecycle phase
- scope와 out-of-bounds summary
- active Change Unit summary
- current 수용 기준 snapshot
- 대기 중인 Decision Packet summary
- Write Authority summary
- Write Authorization, Decision Packet, Approval, Evidence Manifest, Eval, 수동 QA, Acceptance Decision Packet, Residual Risk, artifact, redaction state, projection freshness 권한 claim을 표시할 때 필요한 source refs
- 연결 profile의 보장 수준
- 위험 summary
- design-quality 또는 stewardship summary
- evidence coverage summary
- verification summary
- 수동 QA summary
- 작업 수락 summary
- scope, approval, decision, design, evidence, verification, QA, acceptance gate
- close blocker, close reason, 수동 QA summary
- API error, close blocker, gate, ref에서 파생한 가장 먼저 해소할 막힘, 추가 막힘, 가장 작은 해소 방법 표시 summary
- 읽기용 보기 최신성(projection freshness)과 `source_state_version`
- state, baseline, evidence, MCP, capability freshness/blocker 표시 summary
- 최신 report, Evidence Manifest, Run, Eval, 수동 QA, ArtifactRef refs

이 card의 summary placeholder는 위 기록에서 파생한 표시 binding입니다. Decision, close-blocker, residual-risk, freshness summary는 ref 또는 명시적인 absence를 보여줘야 하며, 사용자 결정 맥락이나 권한을 만들지 않습니다.

## 렌더링 섹션

- task identity
- mode와 lifecycle phase
- scope와 out of bounds
- 수용 기준
- 다음 안전한 행동
- 확인됨 요약
- 남은 작업 또는 확인
- 가장 먼저 해소할 막힘, 소유자, 가장 작은 해소 방법
- 추가 막힘
- active Change Unit
- 사용자 decision
- authority source refs
- 쓰기 권한
- 보장 수준
- design과 stewardship
- 근거와 검증
- 수동 QA
- 잔여 위험
- 수락과 닫기 상태
- 읽기용 보기 최신성
- state/input 최신성과 capability 사용 가능 여부
- 최신 refs

## 전체 템플릿

````text
TASK-{id} {title}
표시 전용: 현재 상태를 보여주는 읽기용 보기이며 기준 상태나 쓰기 권한이 아닙니다.
모드: {mode} / {lifecycle_phase}
범위: {scope_summary|none}
범위 밖: {out_of_bounds_summary|none}
수용 기준: {acceptance_criteria_summary|none}
다음 안전한 행동: {next_safe_action}
확인됨: {checked_summary|none}
남은 것: {remaining_summary|none}
가장 먼저 해소할 막힘: {primary_blocker_label|none}
막힘 소유자: {primary_blocker_owner_label|none}
가장 작은 해소 방법: {smallest_unblocker|none}
추가 막힘: {secondary_blockers_summary|none}
Change Unit: {active_change_unit_summary|none}
필요한 판단: {blocking_decision_summary|none}
쓰기 권한: {write_authority_status}
Authority refs: write={write_authorization_ref|none}; decision={decision_packet_refs|none}; approval={approval_refs|none}; evidence={evidence_manifest_ref|none}; eval={eval_ref|none}; manual_qa={manual_qa_ref|none}; acceptance={acceptance_decision_ref|none}; residual_risk={residual_risk_refs|none}; artifacts={artifact_refs|none}; redaction={redaction_availability_summary|none}; freshness={projection_freshness}
보장 수준: {guarantee_level}; {guard_or_detection_summary}
권한 gate: scope={scope_gate}; approval={approval_gate}; decision={decision_gate}
Design/stewardship: {design_summary|none}; gate={design_gate}
근거: {evidence_summary|none}; gate={evidence_gate}
검증: {verification_summary|none}; gate={verification_gate}
수동 QA: {manual_qa_summary|not_required}; gate={qa_gate}
잔여 위험: status={residual_risk_status|none}; {residual_risk_summary|none}; refs={residual_risk_refs|none}
작업 수락: {acceptance_summary|not_required}; gate={acceptance_gate}
닫기 상태: blockers={close_blockers|none}; reason={close_reason|none}
닫기/assurance 표시: self_checked={self_check_refs|none}; detached_verified={eval_ref|none}; verification_waived={verification_waiver_ref|none}; qa_waived={manual_qa_waiver_ref|none}; risk_accepted_close={accepted_residual_risk_refs|none}
읽기용 보기 최신성(projection freshness): {current|stale|failed|unknown}; source_state_version={source_state_version|unknown}; {refresh_or_reconcile_needed|none}
상태/입력 최신성: {state_baseline_evidence_freshness_summary|current or none}
MCP/capability: {mcp_or_capability_summary|available}
최신 refs: report={latest_report_ref|none}; evidence={evidence_manifest_ref|none}; run/eval/QA={latest_check_refs|none}
````

## 메모

이 template은 렌더링 결과인 카드 형태일 뿐 기준 상태가 아닙니다. Current source record와 ref에서 렌더링되며, 오래된 chat memory에서 렌더링하지 않습니다. Gate value는 기준 상태가 계속 담당하고, guarantee level은 표시와 위험 맥락입니다. Projection freshness는 읽기용 보기의 최신성만 뜻합니다. 정확한 권한 없음 규칙은 [projection/report 경계](../document-projection.md#projection-principles)를 사용합니다.

이 card의 status/next recommendation은 read-only guidance입니다. Decision Packet, `prepare_write`, 근거 수집, 검증, 수동 QA, reconcile, close attempt를 가리킬 수는 있지만, state를 mutate하거나, write를 허가하거나, gate를 충족하거나, 작업 수락을 기록하거나, 잔여 위험을 받아들이거나, Task를 close하지 않습니다.

Authority line은 refs-first여야 합니다. Card가 write allowed라고 말하면 Write Authorization ref를 cite합니다. 근거가 sufficient라고 말하면 Evidence Manifest ref를 cite합니다. 분리 검증이 passed라고 말하면 Eval ref를 cite합니다. 수동 QA가 passed 또는 waived라고 말하면 수동 QA record 또는 waiver path를 cite합니다. 작업 수락 또는 잔여 위험 수용이 recorded라고 말하면 Acceptance Decision Packet 또는 Residual Risk refs를 cite합니다. Source ref가 없으면 claim을 unsupported 또는 not yet recorded로 렌더링합니다.

Residual-risk display는 `status=none`과 `not_visible`을 구분해야 합니다. `status=none`은 requested action에 알려진 close-relevant 잔여 위험이 없다는 뜻이며 명시적인 empty risk-ref set과 함께 렌더링해야 합니다. `not_visible`은 알려진 close-relevant risk가 있지만 작업 수락 또는 close에 충분히 보이지 않았다는 뜻이므로 blocking risk refs 또는 risk가 hidden인 이유를 설명하는 refs를 보여줘야 합니다.

표시 문제를 한 줄로 뭉개지 않습니다. 오래된 projection(stale projection)은 읽기용 card가 뒤처졌을 수 있다는 뜻입니다. Stale state, baseline, evidence는 실제 입력이 이동했거나 부족해졌다는 뜻입니다. MCP 또는 필요한 기능이 unavailable이면 접점이 필요한 Harness/Core capability에 닿지 못하거나 제공하지 못한다는 뜻입니다.

가장 먼저 해소할 막힘은 API response가 제공하는 primary `ToolError`에서 가져오거나, failed `harness.close_task` response를 렌더링할 때는 첫 close blocker에서 가져와야 합니다. 소유자 라벨은 다음 움직임이 사용자 소유인지, 에이전트가 해소 가능한지, 접점/시스템 소유인지 보여줘야 하며, 가장 먼저 해소할 막힘이 없으면 `none`으로 렌더링하거나 생략합니다. 추가 막힘은 간결하게 묶고, 다음 행동, 닫기 준비 상태, 대기 중인 사용자 결정을 바꿀 때만 보여줍니다. 이 라벨들은 표시 문구일 뿐 새 schema value나 `ErrorCode`가 아닙니다.

Design/stewardship은 닫기 상태(Close status)와 별개입니다. Shaping, write blocker, close blocker, Decision Packet 필요성에 영향을 줄 수 있지만 단순한 close-status field가 아닙니다.

이것은 사용자 결정 맥락이 아닙니다. 사용자 결정이 필요하면 결정 유형, decision profile, profile에 맞는 options 또는 chosen outcome, 관련 refs, 그리고 required일 때 full-profile recommendation, uncertainty, deferral effect가 있는 decision prompt를 별도로 렌더링합니다.

Close status는 close reason 구분을 보존해야 합니다. `completed_with_risk_accepted`는 accepted 잔여 위험이 있는 successful close로 렌더링하고, ordinary done, verified, self-checked close처럼 보여주면 안 됩니다. Self-checked, `detached_verified`, verification-waived, QA-waived, risk-accepted-close label은 ref 또는 명시적인 absence와 함께 별도 display slot에 둡니다. 작업 수락이 next action이면 별도 작업 수락 prompt가 근거, 검증, 수동 QA, 잔여 위험 표시 또는 `none`, 작업 수락이 대체하지 않는 것을 보여줘야 합니다.

큰 기록은 먼저 참조를 보여주는 방식(refs-first)으로 둡니다. Evidence, Run, Eval, 수동 QA, artifact, log, screenshot, diff, large trace는 기본적으로 본문에 포함하지 않습니다.
