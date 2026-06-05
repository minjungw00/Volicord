# Close Result Template

## Used when

Use `close-result` when the user needs a compact close-readiness, close-blocker, or close-outcome display. It keeps acceptance, residual risk, evidence, artifact availability, self-check basis, and blockers separate.

Implementation tier: MVP-1 User Work Loop view. Detailed continuity, Journey, direct-result, release-handoff, or export reports are later/full-profile templates.

Boundary: this template displays close status. It does not close a Task, record final acceptance, accept residual risk, record verification or Manual QA, create evidence, or change gate values. Close authority remains with the Core close path.

## Source records

- current Task state and close attempt or close-readiness result
- scope and changed-scope summary
- evidence refs and evidence gaps
- self-check summary when it is part of the active evidence summary
- artifact availability for close-relevant evidence refs
- final-acceptance user judgment refs when required
- residual-risk visibility and residual-risk acceptance refs when relevant
- design-quality routed actions when they affect close, limited to the active MVP blocking set unless a later profile is active
- close availability, close blockers, and smallest unblockers
- source state version, freshness, and capability status

## Rendered sections

- close status
- scope
- evidence
- artifact availability and self-check basis
- judgment and acceptance
- residual risk
- blockers
- next safe action
- sources and freshness

## Full template

````text
Close availability: {ready|blocked|closed|not_requested}
Display only: Core close state and owner refs remain authoritative.

Scope: {scope_summary}
Evidence: status={evidence_summary.status}; summary={evidence_summary.summary}; gaps={evidence_gaps|none}
Artifacts: {artifact_availability_summary}
Self-check basis: {self_check_summary|none}
Judgment and acceptance: final_acceptance={final_acceptance_status}; sensitive_action_permission={sensitive_permission_status|not_applicable}
Design quality: {design_quality_close_action|none}
Residual risk visibility: {residual_risk_visibility}
Residual risk acceptance: {residual_risk_acceptance_status|not_applicable}
Why close is unavailable: {close_blockers|none}
Smallest unblocker: {smallest_unblocker|none}
Close basis or reason: {close_reason|not_applicable}
Agent can safely do next: {next_safe_action|none}
Sources/freshness: state={source_state_version}; refs={source_refs}; rendered={updated_at}; freshness={freshness_state}
````

## Notes

Do not collapse evidence summary, artifact availability, final acceptance, residual-risk visibility, residual-risk acceptance, blockers, design-quality routed actions, and readable-view freshness into one "done" line. MVP-1 close-result output does not display detached verification or Manual QA rows; later/profile templates may add them when the owner profile is active. If close is blocked, name the primary blocker and the single next action, and keep secondary blockers visible only when they affect the next path. If the readable close view is stale or failed, fetch a current Core close result instead of closing from this template's prose.
