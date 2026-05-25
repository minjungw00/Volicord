# JOURNEY-CARD Template

## Used when

Use `JOURNEY-CARD` when a current-position card needs to show where the work is, what judgment is pending, the Autonomy Boundary, Write Authority Summary, next evidence, residual risk, gates, and projection freshness.

## Source records

- current Task state and gates
- active Change Unit
- Autonomy Boundary summary
- Write Authorization, approval, baseline, and guarantee refs
- active Decision Packet refs
- residual-risk summary and refs
- latest evidence, Eval, Manual QA, and report refs
- projection freshness inputs

Judgment, write-authority, close-impact, residual-risk, and freshness placeholders are display bindings derived from the records above. If a user decision is actually needed, render a Decision Packet or decision prompt rather than treating this card as the judgment-context source.

## Rendered sections

- current position and next action
- Judgment context
- Autonomy boundary
- Write Authority Summary
- Next evidence
- Residual risk
- Gates
- Projection freshness

## Full template

````text
TASK-{id} {title}
Display only: current-position view, not canonical state or write authority.
Where we are: {mode} / {lifecycle_phase} / {current_position}
Next action: {next_action}

Judgment context:
- pending decision: {decision_packet_ref|none}
- user deciding: {what_user_is_deciding|none}
- agent may decide: {what_agent_may_decide_without_user}

Autonomy boundary:
- profile: {autonomy_profile}
- agent may do: {agent_may_do}
- user judgment required: {user_judgment_required}
- AFK stop conditions: {afk_stop_conditions}

Write Authority Summary:
- active Change Unit: {active_change_unit_ref|none}
- write authorization: {write_authorization_ref|none}
- allowed paths: {allowed_paths}
- allowed tools: {allowed_tools}
- allowed commands: {allowed_commands}
- allowed network targets: {allowed_network_targets}
- secret scope: {secret_scope}
- sensitive categories: {sensitive_categories}
- approval status: {approval_status}
- baseline: {baseline_ref|none}
- guarantee: {guarantee_display}
- note: Autonomy Boundary is judgment latitude, not write authority.

Next evidence:
- action: {next_evidence_action}
- needed for: {evidence_needed_for}
- latest evidence: {latest_evidence_ref|none}
- omitted or blocked impact: {redaction_availability_summary|none}

Residual risk:
- status: {residual_risk_status}
- close impact: {residual_risk_close_impact}
- accepted residual-risk record refs: {accepted_residual_risk_record_refs|none}

Gates:
- scope: {scope_gate}
- decision: {decision_gate}
- approval: {approval_gate}
- evidence: {evidence_gate}
- verification: {verification_gate}
- Manual QA: {qa_gate display: pending|passed|failed|waived|not_required}
- acceptance: {acceptance_gate}

Projection freshness: {projection_freshness}; source_state_version={source_state_version|unknown} (view freshness, not task result)
````

## Notes

This template is a rendered shape, not canonical state. Persisted `JOURNEY-CARD` Markdown is optional; current-position Journey Card output in status, next, and significant resume flows remains a read/display surface.

When latest or next evidence includes `secret_omitted` or `blocked` artifact refs, this card should show only the availability impact. It must not include omitted values or blocked raw payload content.
