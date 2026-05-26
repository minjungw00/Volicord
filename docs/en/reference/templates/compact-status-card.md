# Compact Status Card Template

## Used when

Use the compact status card when a short current-state display needs to show the always-on Harness context envelope: Task, mode, scope, out of bounds, next safe action, blocker status, pending user decision, write authority, evidence, verification, Manual QA, residual risk, guarantee level, projection freshness, and latest refs. Keep it small enough for status, next-action, and resume turns: ordinary-language state first, exact Harness labels only when they clarify the boundary.

## Source records

- current Task state and lifecycle phase
- scope and out-of-bounds summaries
- active Change Unit summary
- pending Decision Packet summary
- Write Authority summary
- connected profile guarantee level
- risk summary
- design-quality or stewardship summary
- evidence coverage summary
- verification summary
- Manual QA summary
- acceptance summary
- scope, approval, decision, design, evidence, verification, QA, and acceptance gates
- close blocker, close reason, and Manual QA summary
- primary blocker, secondary blocker, and smallest unblocker display summaries derived from API errors, close blockers, gates, and refs
- projection freshness and `source_state_version`
- state, baseline, evidence, MCP, and capability freshness/blocker display summaries
- latest report, Evidence Manifest, Run, Eval, Manual QA, and ArtifactRef refs

Summary placeholders in this card are display bindings derived from the records above. Decision, close-blocker, residual-risk, and freshness summaries should show refs or explicit absence; they do not create judgment context or authority.

## Rendered sections

- task identity
- mode and lifecycle phase
- scope and out of bounds
- next safe action
- primary blocker, owner, and smallest unblocker
- secondary blockers
- active Change Unit
- user decision
- write authority
- guarantee level
- design and stewardship
- evidence and verification
- Manual QA
- residual risk
- acceptance and close status
- projection freshness
- state/input freshness and capability availability
- latest refs

## Full template

````text
TASK-{id} {title}
Display only: current-state view, not canonical state or write authority.
Mode: {mode} / {lifecycle_phase}
Scope: {scope_summary|none}
Out of bounds: {out_of_bounds_summary|none}
Next safe action: {next_safe_action}
Primary blocker: {primary_blocker_label|none}
Blocker owner: {primary_blocker_owner_label|none}
Smallest unblocker: {smallest_unblocker|none}
Secondary blockers: {secondary_blockers_summary|none}
Change Unit: {active_change_unit_summary|none}
Decision needed: {blocking_decision_summary|none}
Write authority: {write_authority_status}
Guarantee: {guarantee_level}; {guard_or_detection_summary}
Authority gates: scope={scope_gate}; approval={approval_gate}; decision={decision_gate}
Design/stewardship: {design_summary|none}; gate={design_gate}
Evidence: {evidence_summary|none}; gate={evidence_gate}
Verification: {verification_summary|none}; gate={verification_gate}
Manual QA: {manual_qa_summary|not_required}; gate={qa_gate}
Residual risk: {residual_risk_summary|none}
Acceptance: {acceptance_summary|not_required}; gate={acceptance_gate}
Close status: blockers={close_blockers|none}; reason={close_reason|none}
Projection freshness (view only): {current|stale|failed|unknown}; source_state_version={source_state_version|unknown}; {refresh_or_reconcile_needed|none}
State/input freshness: {state_baseline_evidence_freshness_summary|current or none}
MCP/capability: {mcp_or_capability_summary|available}
Latest refs: report={latest_report_ref|none}; evidence={evidence_manifest_ref|none}; run/eval/QA={latest_check_refs|none}
````

## Notes

This template is a rendered card shape, not canonical state. Gate values remain owned by canonical state, guarantee level is display and risk context, and projection freshness is readable-view freshness only. Use the [projection/report boundary](../document-projection.md#projection-principles) for the exact non-authority rule.

Do not collapse display problems into one line. A stale projection means the readable card may lag. Stale state, baseline, or evidence means the underlying inputs moved or became insufficient. MCP or capability unavailable means the surface cannot reach or provide the required Harness/Core capability.

The primary blocker should come from the primary `ToolError` when an API response supplies one, or from the first close blocker when rendering a failed `harness.close_task` response. The owner label should say whether the next move is user-owned, agent-resolvable, or surface/system-owned, and should render as `none` or be omitted when there is no primary blocker. Secondary blockers should be grouped compactly and shown only when they change the next action, close readiness, or user judgment. These labels are display text, not new schema values or error codes.

Design/stewardship is separate from Close status. It may affect shaping, write blockers, close blockers, or Decision Packet needs, but it is not merely a close-status field.

This is not judgment-context. If user judgment is needed, render a separate decision prompt with options, recommendation, uncertainty, deferral effect, and relevant refs.

Close status should preserve the close-reason distinction. Render `completed_with_risk_accepted` as successful close with accepted residual risk, not as ordinary done, verified, or self-checked close. If final acceptance is the next action, the separate acceptance prompt must show evidence, verification, Manual QA, residual-risk visibility or `none`, and what acceptance does not replace.

Large records stay refs-first. Evidence, Run, Eval, Manual QA, artifacts, logs, screenshots, diffs, and large traces are not embedded by default.
