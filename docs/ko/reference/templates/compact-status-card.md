# Compact Status Card Template

## 사용 시점

상시 Harness context envelope를 짧은 현재 상태 표시로 보여줄 때 Compact Status Card를 사용합니다. 여기에는 Task 상태, next safe action, active Change Unit, 대기 중인 user decision, 쓰기 권한, guarantee level, gate, Manual QA, residual risk, projection freshness, latest refs가 포함됩니다.

## 기준 기록

- 현재 Task 상태와 lifecycle phase
- active Change Unit summary
- 대기 중인 Decision Packet summary
- Write Authority summary
- 연결 profile guarantee level
- 위험 summary
- scope, approval, decision, design, evidence, verification, QA, acceptance gate
- close blocker와 Manual QA summary
- projection freshness와 `source_state_version`
- 최신 report, Evidence Manifest, Run, Eval, Manual QA, ArtifactRef refs

## 렌더링 섹션

- task identity
- mode와 lifecycle phase
- next safe action
- active Change Unit
- 사용자 decision
- 쓰기 권한
- guarantee level
- gate summary
- Manual QA
- residual risk
- projection freshness
- latest refs

## 전체 템플릿

````text
TASK-{id} {title}
표시 전용: 현재 상태를 보여주는 읽기용 보기이며 기준 상태나 쓰기 권한이 아닙니다.
모드: {mode} / {lifecycle_phase}
다음 safe action: {next_safe_action}
Change Unit: {active_change_unit_summary|none}
Blocking decision: {blocking_decision_summary|none}
쓰기 권한: {write_authority_status}
Guarantee: {guarantee_level}; {guard_or_detection_summary}
Authority gates: scope={scope_gate}; approval={approval_gate}; decision={decision_gate}
Quality gates: design={design_gate}; evidence={evidence_gate}; verification={verification_gate}; QA={qa_gate}; acceptance={acceptance_gate}
Manual QA: {manual_qa_summary|not_required}
Close blockers: {close_blockers|none}
Residual risk: {residual_risk_summary|none}
Projection freshness (읽기용 view): {current|stale|failed|unknown}; source_state_version={source_state_version|unknown}; {refresh_or_reconcile_needed|none}
Latest refs: report={latest_report_ref|none}; evidence={evidence_manifest_ref|none}; run/eval/QA={latest_check_refs|none}
````

## 메모

이 template은 렌더링 결과인 카드 형태일 뿐 기준 상태가 아닙니다. Gate value는 기준 상태가 계속 담당하며, projection freshness는 읽기용 보기의 최신성만 뜻합니다. Task result, state freshness, evidence freshness, Approval, acceptance, 쓰기 권한이 아닙니다.

이것은 judgment-context가 아닙니다. 사용자 판단이 필요하면 options, recommendation, uncertainty, 미룰 때의 영향, relevant refs가 있는 decision prompt를 별도로 렌더링합니다.

큰 기록은 refs-first로 둡니다. Evidence, Run, Eval, Manual QA, artifact, log, screenshot, diff, large trace는 default로 embed하지 않습니다.
