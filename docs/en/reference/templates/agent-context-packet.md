# Agent Context Packet Template

## Used when

Use `agent-context-packet` when an agent needs compact, current context for the next safe action. It is optimized for accuracy, freshness, Core-derived refs, allowed action boundaries, and blockers, not for user-facing prose or full report detail.

Implementation tier: MVP-1 support view. It can be returned as a structured payload or prompt-sized text. It is not a required persisted Markdown projection.

Boundary: this packet is support context only. It cannot authorize writes, satisfy gates, create evidence, grant approval, record final acceptance, accept residual risk, create close readiness, or close a Task.

## Source records

- task id, task summary, work shape, lifecycle, and state version
- active Change Unit ref, scope summary, non-goals, and allowed paths/tools/commands
- active blockers and blocker refs
- active user judgments, pending judgment refs, and judgment request refs
- write authority summary, including Write Authorization refs when present
- evidence summary, evidence refs, Run refs, ArtifactRefs, `redaction_state`, and evidence gaps
- close blockers, residual-risk status, final-acceptance need/status, and relevant owner refs
- guarantee level, MCP/Core availability, source clocks, and freshness state
- exactly one compact next action plus owner document or owner-section pointers needed for that action

## Rendered sections

- current task
- active Change Unit and allowed action boundary
- active user judgments
- blockers
- write authority
- evidence state
- close and residual-risk state
- next safe action
- freshness and source refs
- pull-on-demand pointers

## Full template

````text
agent_context_packet:
  display_only: true
  authority: none; use current Core state for authority
  task_id: {task_id}
  task_summary: {task_summary}
  state_version: {source_state_version}
  work_shape: {work_shape}
  active_change_unit: {change_unit_ref|none}
  scope: {scope_summary}
  non_goals: {non_goals|none}
  allowed_paths: {allowed_paths|none}
  allowed_tools: {allowed_tools|none}
  allowed_commands: {allowed_commands|none}
  active_blockers: {active_blockers|none}
  active_user_judgments: {active_user_judgment_refs|none}
  pending_judgments: {pending_user_judgment_refs|none}
  write_authority: {write_authority_summary|none}
  evidence_summary: {evidence_summary}
  evidence_refs: {evidence_refs_and_gaps}
  design_quality: {design_quality_routed_action|none}
  close: {close_blockers_and_acceptance_state}
  residual_risk_status: {residual_risk_status}
  next_safe_action: {next_safe_action}
  guarantee_level: {guarantee_level_or_unavailable}
  sources:
    refs: {source_refs}
    freshness: {freshness_state}
    rendered_at: {updated_at}
  pull_if_needed: {owner_section_refs_for_next_action|none}
````

## Notes

Keep the packet one screen or less. Do not include full schemas, full reference docs, full historical event logs, registered artifact file bodies, full report bodies, full templates, unrelated templates, full design-quality catalogs, or future catalog material by default.

The `guarantee_level` field is required context. If Core/MCP is unavailable, set it to the unavailable/capability condition and treat Harness-dependent state, write, evidence, acceptance, residual-risk, and close claims as unavailable until refreshed.
