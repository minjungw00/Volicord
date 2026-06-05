# Close Result Template

## Used when

Use `close-result` when the user or agent needs a compact close-readiness, close-blocker, or close-outcome display. It keeps acceptance, residual risk, evidence, checks, and blockers separate.

Implementation tier: MVP-1 User Work Loop view. Detailed continuity, Journey, direct-result, release-handoff, or export reports are later/full-profile templates.

Boundary: this template displays close status. It does not close a Task, record work acceptance, accept residual risk, waive QA or verification, create evidence, or change gate values. Close authority remains with the Core close path.

## Source records

- current Task state and close attempt or close-readiness result
- scope and changed-scope summary
- evidence refs and evidence gaps
- check, verification, Manual QA, and waiver status when relevant
- work-acceptance user judgment refs when required
- residual-risk visibility and residual-risk acceptance refs when relevant
- close blockers and smallest unblockers
- source state version, freshness, and capability status

## Rendered sections

- close status
- scope
- evidence
- check or verification
- judgment and acceptance
- residual risk
- blockers
- next safe action
- sources and freshness

## Full template

````text
Close result: {ready|blocked|closed|not_requested}
Display only: Core close state and owner refs remain authoritative.

Scope: {scope_summary}
Evidence: status={evidence_summary.status}; summary={evidence_summary.summary}; gaps={evidence_gaps|none}
Check or verification: {check_verification_manual_qa_summary}
Judgment and acceptance: work_acceptance={work_acceptance_status}; sensitive_action_permission={sensitive_permission_status|not_applicable}
Residual risk visibility: {residual_risk_visibility}
Residual risk acceptance: {residual_risk_acceptance_status|not_applicable}
Blockers: {close_blockers|none}
Smallest unblocker: {smallest_unblocker|none}
Close basis or reason: {close_reason|not_applicable}
Next safe action: {next_safe_action|none}
Sources/freshness: state={source_state_version}; refs={source_refs}; rendered={updated_at}; freshness={freshness_state}
````

## Notes

Do not collapse evidence, verification, Manual QA, work acceptance, residual-risk visibility, residual-risk acceptance, blockers, and readable-view freshness into one "done" line. If close is blocked, name the primary blocker and keep secondary blockers visible when they affect the next path. If the readable close view is stale or failed, fetch a current Core close result instead of closing from this template's prose.
