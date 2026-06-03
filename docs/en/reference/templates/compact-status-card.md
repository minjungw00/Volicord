# Compact Status Card Template

## Authority rule

- Projection is derived from Core-owned state records and artifact references.
- Projection is not Core state.
- User edits to a projection are input only; they are not automatically accepted state.
- Chat and Markdown cannot override Core state.

## Used when

Use the compact status card when a short current-state display needs to make Core state readable for a user or compact for an agent. It is the v0.2 MVP projection shape: one small card derived from Core state and refs.

Boundary: projection template only; it does not authorize runtime/server implementation or generated operational outputs. Shared phase and projection rules live in [Template Reference](README.md#used-when).

Implementation tier: v0.2 User-Facing Harness MVP projection. v0.1 Core status output may still return plain structured status/blocker output instead of this card. This template is not a persisted state record and is not evidence of full projection renderer support.

The card should use ordinary language first and exact Harness labels only where they clarify the authority boundary. It should be small enough for status, next-action, and resume turns.

## Required contents

- what we are doing
- current scope and non-goals
- pending user judgments
- known evidence or evidence gaps
- close blockers
- visible residual risk
- next safe action
- source/freshness references

## Source records and refs

- current Task state and lifecycle phase
- current scope, non-goals, and active Change Unit summary when relevant
- pending Decision Packet refs and compact judgment summaries
- known evidence refs and evidence-gap summaries
- close blocker refs and close reason when present
- residual-risk refs or explicit absence / not-yet-visible status
- next safe action from current Core state
- projection freshness and `source_state_version`
- artifact refs and redaction state when needed to support a displayed claim
- optional authority refs for Write Authorization, Approval, Evidence Manifest, Eval, Manual QA, Acceptance Decision Packet, and Residual Risk only when the card displays the related claim

Summary placeholders in this card are display bindings derived from the records above. Decision, evidence, close-blocker, residual-risk, and freshness summaries should show refs or explicit absence; they do not create user judgment context or authority.

Do not include schema dumps, DDL, event logs, full artifacts, full reference docs, full Evidence Manifests, full Eval bodies, full Manual QA records, or report bodies in the card.

## User-facing framing

Use this shape when the reader is the user. Keep each line short and readable.

````text
TASK-{id} {title}
Display only: derived from Core state and refs; not Core state and not write authority.
Doing: {doing_summary}
Scope now: {scope_summary|none}
Non-goals: {non_goals_summary|none}
Pending user judgments: {pending_user_judgments_summary|none}
Evidence: {known_evidence_summary|none}
Evidence gaps: {evidence_gaps_summary|none}
Close blockers: {close_blockers_summary|none}
Visible residual risk: {residual_risk_summary|none}
Next safe action: {next_safe_action}
Sources/freshness: state={source_state_version|unknown}; refs={source_refs_summary|none}; rendered={updated_at|unknown}; freshness={projection_freshness}
````

## Agent compact framing

Use this shape when the consumer is an agent context/reference payload. This is not a public schema; it is an example of the compactness target.

````yaml
task: {task_id}
title: {title}
mode: {mode}
phase: {lifecycle_phase}
doing: {doing_summary}
scope_ref: {scope_ref|none}
non_goals_ref: {non_goals_ref|none}
pending_judgment_refs: {decision_packet_refs|none}
evidence_refs: {evidence_refs|none}
evidence_gaps: {evidence_gaps_summary|none}
close_blocker_refs: {close_blocker_refs|none}
residual_risk_refs: {residual_risk_refs|none}
next_safe_action: {next_safe_action}
freshness:
  source_state_version: {source_state_version|unknown}
  rendered_at: {updated_at|unknown}
  state: {current|stale|failed|unknown}
````

## Notes

This template is a rendered card shape, not canonical state. It is rendered from current source records and refs, not stale chat memory. Gate values remain owned by canonical state, and projection freshness is readable-view freshness only. Use the [projection/report boundary](../document-projection.md#projection-principles) for the exact non-authority rule.

Status/next recommendations in this card are read-only guidance. They may point to a Decision Packet, `prepare_write`, evidence collection, verification, QA, reconcile, or close attempt, but they do not mutate state, authorize writes, satisfy gates, accept results, accept residual risk, or close the Task.

Authority lines must be refs-first. If the card says writes are allowed, cite the Write Authorization ref. If it says sensitive-action permission was granted, cite the Approval ref. If it says evidence is sufficient, cite the Evidence Manifest ref. If it says detached verification passed, cite the Eval ref. If it says Manual QA passed or was waived, cite the Manual QA record or waiver path. If it says work acceptance was recorded, cite the Acceptance Decision Packet; if it says residual-risk acceptance was recorded, cite accepted Residual Risk refs. If the source ref is absent, render the claim as unsupported or not yet recorded.

Residual-risk display must distinguish `status=none` from `not_visible`. `status=none` means no known close-relevant residual risk exists for the requested action and should render with an explicit empty risk-ref set. `not_visible` means known close-relevant risk exists but is not yet visible enough for acceptance or close, and should show the blocking risk refs or the refs that explain why the risk is hidden.

Do not collapse display problems into one line. A stale projection means the readable card may lag. Stale state, baseline, or evidence means the underlying inputs moved or became insufficient. MCP or capability unavailable means the surface cannot reach or provide the required Harness/Core capability.

The primary blocker should come from the primary `ToolError` when an API response supplies one, or from the first close blocker when rendering a failed `harness.close_task` response. The owner label should say whether the next move is user-owned, agent-resolvable, or surface/system-owned, and should render as `none` or be omitted when there is no primary blocker. Secondary blockers should be grouped compactly and shown only when they change the next action, close readiness, or pending user judgment. These labels are display text, not new schema values or error codes.

This is not user judgment context. If a user judgment is needed, render a separate judgment prompt with `judgment_route`, `display_depth`, display-depth-appropriate options or chosen outcome, relevant refs, and higher-depth recommendation, uncertainty, or deferral effect when required.

Close status should preserve the close-reason distinction. Render `completed_with_risk_accepted` as successful close with accepted residual risk, not as ordinary done, verified, or self-checked close. Keep self-checked, `detached_verified`, verification-waived, QA-waived, and risk-accepted-close labels on separate display slots with refs or explicit absence. If work acceptance is the next action, the separate acceptance prompt must show evidence, verification, Manual QA, residual-risk visibility or `none`, and what acceptance does not replace.

Large records stay refs-first. Evidence, Run, Eval, Manual QA, artifacts, logs, screenshots, diffs, and large traces are not embedded by default.
