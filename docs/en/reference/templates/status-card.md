# Status Card Template

## Used when

Use `status-card` when MVP-1 needs a short user-visible current-state view. It shows what is happening now, what is in scope, what the user must decide, what evidence exists or is missing, what blocks close, and the next safe action.

Implementation tier: MVP-1 User Work Loop view. Engineering Checkpoint may return plain structured status/blocker output instead of this card.

Boundary: this template is rendered display only. It is not Core state, not evidence, not approval, not final acceptance, not residual-risk acceptance, not Write Authorization, and not close readiness authority. It must be rendered from current Core-owned state and refs, not stale chat.

## Source records

- current Task state, work shape, lifecycle, and next safe action
- scope, non-goals, active Change Unit summary, and stop conditions when relevant
- pending `user_judgment` refs, what the user must decide, and compact judgment summaries
- active blockers and why work is blocked when blocked
- run refs, `evidence_ref` refs, ArtifactRefs, `redaction_state`, and evidence gaps
- close blockers, final-acceptance need/status, residual-risk visibility, and residual-risk acceptance refs when relevant
- design-quality routed action when relevant, using the active MVP impact classes rather than the full policy catalog
- guarantee level and capability/fallback status
- `source_state_version`, render time, and freshness state

## Rendered sections

- work
- scope
- judgment
- blocked reason
- evidence
- check or verification
- close
- next safe action
- sources and freshness

## Full template

````text
{task_id} {title}
Display only: derived from Core state and refs; not Core state or write authority.

Work: {work_shape}. {current_task_summary}
Scope: {scope_summary}
Out of scope: {non_goals|none}
Blocked because: {active_blocked_reason|none}
User must decide: {pending_user_judgments|none}
Evidence: status={evidence_summary.status}; summary={known_evidence_summary|none}
Evidence gaps: {evidence_gaps|none}
Check or verification: {check_or_verification_summary|none}
Close availability: {close_readiness_summary}; why unavailable={close_blockers|none}
Design quality: {design_quality_routed_action|none}
Remaining risk: {residual_risk_visibility|none}
Agent can safely do next: {next_safe_action}
Guarantee: {guarantee_level_or_unavailable}; {guarantee_note}
Sources/freshness: state={source_state_version}; refs={source_refs}; rendered={updated_at}; freshness={freshness_state}
````

## Notes

Keep this card readable. Do not dump schemas, DDL, event logs, full artifacts, full report bodies, full templates, future catalogs, detailed Evidence Manifest bodies, detailed Eval bodies, or full Manual QA records.

When a field has no source record, render `none`, `unknown`, `not_required`, or an explicit blocker instead of inventing state.

Always render the guarantee line. For MVP-1 default behavior, the note should say cooperative hold or detective reporting when that is the actual limit. If Core/MCP is unavailable, render the unavailable condition instead of a stale or guessed guarantee.

Design-quality content should fit one line: the current routed action and, when blocking, the single next action. Do not list full domain-language, module/interface, TDD, stewardship, feedback-loop, Manual QA, or detached-verification catalogs in the MVP-1 status card.
