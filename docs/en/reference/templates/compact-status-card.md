# Compact Status Card Template

## Used when

Use the compact status card when a short current-state display needs to show the always-on Harness context envelope: task state, next safe action, active Change Unit, pending user decision, write authority, guarantee level, gates, Manual QA, residual risk, projection freshness, and latest refs.

## Source records

- current Task state and lifecycle phase
- active Change Unit summary
- pending Decision Packet summary
- Write Authority summary
- connected profile guarantee level
- risk summary
- scope, approval, decision, design, evidence, verification, QA, and acceptance gates
- close blocker and Manual QA summary
- projection freshness and `source_state_version`
- latest report, Evidence Manifest, Run, Eval, Manual QA, and ArtifactRef refs

## Rendered sections

- task identity
- mode and lifecycle phase
- next safe action
- active Change Unit
- user decision
- write authority
- guarantee level
- gate summary
- Manual QA
- residual risk
- projection freshness
- latest refs

## Full template

````text
TASK-{id} {title}
Display only: current-state view, not canonical state or write authority.
Mode: {mode} / {lifecycle_phase}
Next safe action: {next_safe_action}
Change Unit: {active_change_unit_summary|none}
Blocking decision: {blocking_decision_summary|none}
Write authority: {write_authority_status}
Guarantee: {guarantee_level}; {guard_or_detection_summary}
Authority gates: scope={scope_gate}; approval={approval_gate}; decision={decision_gate}
Quality gates: design={design_gate}; evidence={evidence_gate}; verification={verification_gate}; QA={qa_gate}; acceptance={acceptance_gate}
Manual QA: {manual_qa_summary|not_required}
Close blockers: {close_blockers|none}
Residual risk: {residual_risk_summary|none}
Projection freshness (view only): {current|stale|failed|unknown}; source_state_version={source_state_version|unknown}; {refresh_or_reconcile_needed|none}
Latest refs: report={latest_report_ref|none}; evidence={evidence_manifest_ref|none}; run/eval/QA={latest_check_refs|none}
````

## Notes

This template is a rendered card shape, not canonical state. Gate values remain owned by canonical state, and projection freshness is readable-view freshness only; it is not Task result, state freshness, evidence freshness, approval, acceptance, or write authority.

This is not judgment-context. If user judgment is needed, render a separate decision prompt with options, recommendation, uncertainty, deferral effect, and relevant refs.

Large records stay refs-first. Evidence, Run, Eval, Manual QA, artifacts, logs, screenshots, diffs, and large traces are not embedded by default.
