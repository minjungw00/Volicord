# Compact Status Card Template

## 사용 시점

매 턴 유지되는 Harness 맥락 묶음(context envelope)을 짧은 현재 상태 표시로 보여줄 때 Compact Status Card를 사용합니다. 여기에는 Task, 모드, 범위, 범위 밖, 다음 안전한 행동, 막힘 상태, 대기 중인 사용자 판단, 쓰기 권한, 근거, 검증, Manual QA, 남은 위험, 보장 수준(guarantee level), 읽기용 보기 최신성(projection freshness), 최신 참조(ref)가 포함됩니다. 상태 확인, 다음 행동, 이어가기 턴에서 부담 없이 읽을 수 있게 유지하고, 평범한 상태 설명을 먼저 쓰며 정확한 Harness label은 경계를 분명히 할 때만 붙입니다.

## 기준 기록

- 현재 Task 상태와 lifecycle phase
- scope와 out-of-bounds summary
- active Change Unit summary
- 대기 중인 Decision Packet summary
- Write Authority summary
- 연결 profile의 보장 수준
- 위험 summary
- design-quality 또는 stewardship summary
- evidence coverage summary
- verification summary
- Manual QA summary
- 결과 수락 summary
- scope, approval, decision, design, evidence, verification, QA, acceptance gate
- close blocker, close reason, Manual QA summary
- API error, close blocker, gate, ref에서 파생한 가장 먼저 해소할 막힘, 추가 막힘, 가장 작은 해소 방법 표시 summary
- 읽기용 보기 최신성(projection freshness)과 `source_state_version`
- state, baseline, evidence, MCP, capability freshness/blocker 표시 summary
- 최신 report, Evidence Manifest, Run, Eval, Manual QA, ArtifactRef refs

이 card의 summary placeholder는 위 기록에서 파생한 표시 binding입니다. Decision, close-blocker, residual-risk, freshness summary는 ref 또는 명시적인 absence를 보여줘야 하며, 판단 맥락(judgment context)이나 권한을 만들지 않습니다.

## 렌더링 섹션

- task identity
- mode와 lifecycle phase
- scope와 out of bounds
- 다음 안전한 행동
- 가장 먼저 해소할 막힘, 소유자, 가장 작은 해소 방법
- 추가 막힘
- active Change Unit
- 사용자 decision
- 쓰기 권한
- 보장 수준
- design과 stewardship
- 근거와 검증
- Manual QA
- 남은 위험
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
다음 안전한 행동: {next_safe_action}
가장 먼저 해소할 막힘: {primary_blocker_label|none}
막힘 소유자: {primary_blocker_owner_label|none}
가장 작은 해소 방법: {smallest_unblocker|none}
추가 막힘: {secondary_blockers_summary|none}
Change Unit: {active_change_unit_summary|none}
필요한 판단: {blocking_decision_summary|none}
쓰기 권한: {write_authority_status}
보장 수준: {guarantee_level}; {guard_or_detection_summary}
권한 gate: scope={scope_gate}; approval={approval_gate}; decision={decision_gate}
Design/stewardship: {design_summary|none}; gate={design_gate}
근거: {evidence_summary|none}; gate={evidence_gate}
검증: {verification_summary|none}; gate={verification_gate}
Manual QA: {manual_qa_summary|not_required}; gate={qa_gate}
남은 위험: {residual_risk_summary|none}
수락: {acceptance_summary|not_required}; gate={acceptance_gate}
닫기 상태: blockers={close_blockers|none}; reason={close_reason|none}
읽기용 보기 최신성(projection freshness): {current|stale|failed|unknown}; source_state_version={source_state_version|unknown}; {refresh_or_reconcile_needed|none}
상태/입력 최신성: {state_baseline_evidence_freshness_summary|current or none}
MCP/capability: {mcp_or_capability_summary|available}
최신 refs: report={latest_report_ref|none}; evidence={evidence_manifest_ref|none}; run/eval/QA={latest_check_refs|none}
````

## 메모

이 template은 렌더링 결과인 카드 형태일 뿐 기준 상태가 아닙니다. Gate value는 기준 상태가 계속 담당하고, guarantee level은 표시와 위험 맥락입니다. Projection freshness는 읽기용 보기의 최신성만 뜻합니다. 정확한 권한 없음 규칙은 [projection/report 경계](../document-projection.md#projection-principles)를 사용합니다.

표시 문제를 한 줄로 뭉개지 않습니다. 오래된 projection(stale projection)은 읽기용 card가 뒤처졌을 수 있다는 뜻입니다. Stale state, baseline, evidence는 실제 입력이 이동했거나 부족해졌다는 뜻입니다. MCP 또는 필요한 기능이 unavailable이면 접점이 필요한 Harness/Core capability에 닿지 못하거나 제공하지 못한다는 뜻입니다.

가장 먼저 해소할 막힘은 API response가 제공하는 primary `ToolError`에서 가져오거나, failed `harness.close_task` response를 렌더링할 때는 첫 close blocker에서 가져와야 합니다. 소유자 라벨은 다음 움직임이 사용자 소유인지, 에이전트가 해소 가능한지, 접점/시스템 소유인지 보여줘야 하며, 가장 먼저 해소할 막힘이 없으면 `none`으로 렌더링하거나 생략합니다. 추가 막힘은 간결하게 묶고, 다음 행동, 닫기 준비 상태, 사용자 판단을 바꿀 때만 보여줍니다. 이 라벨들은 표시 문구일 뿐 새 schema value나 `ErrorCode`가 아닙니다.

Design/stewardship은 닫기 상태(Close status)와 별개입니다. Shaping, write blocker, close blocker, Decision Packet 필요성에 영향을 줄 수 있지만 단순한 close-status field가 아닙니다.

이것은 judgment-context가 아닙니다. 사용자 판단이 필요하면 선택지, 추천안, 불확실성, 결정을 미룰 때의 영향, 관련 refs가 있는 decision prompt를 별도로 렌더링합니다.

Close status는 close reason 구분을 보존해야 합니다. `completed_with_risk_accepted`는 accepted residual risk가 있는 successful close로 렌더링하고, ordinary done, verified, self-checked close처럼 보여주면 안 됩니다. Final acceptance가 next action이면 별도 acceptance prompt가 evidence, verification, Manual QA, residual-risk visibility 또는 `none`, acceptance가 대체하지 않는 것을 보여줘야 합니다.

큰 기록은 먼저 참조를 보여주는 방식(refs-first)으로 둡니다. Evidence, Run, Eval, Manual QA, artifact, log, screenshot, diff, large trace는 기본적으로 본문에 포함하지 않습니다.
