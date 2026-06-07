# Projection And Templates Reference

## Owns / Does not own

This document owns the projection and active template display contract for future Harness behavior. It is documentation source material only; it is not a runtime projection, runtime state, generated artifact, evidence record, QA record, final-acceptance record, residual-risk record, close record, or implementation-ready server plan.

This document owns:

- projection authority boundaries
- projection as derived display
- human-editable section rules
- managed block rules
- `ArtifactRef` rendering rules
- source/freshness display rules for compact views
- the active current MVP template set
- the full rendered bodies for the five active current MVP templates

This document does not own:

- Core state, lifecycle, gates, `prepare_write`, `record_run`, `close_task`, or user-judgment state changes; see [Core Model Reference](core-model.md)
- public MCP request/response schemas, `ProjectionKind`, `ArtifactRef`, or error shapes; see [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), and [API Errors](api/errors.md)
- SQLite DDL, storage layout, artifact storage, or projection job storage; see [Storage](storage.md)
- design-quality close-impact boundary; see [Design Quality](design-quality.md)
- operator command behavior as active Reference scope; future candidates stay in [Later Candidate Index: Operations Candidates](../later/index.md#operations-candidates)
- conformance fixture assertion behavior; see [Conformance Reference](conformance.md)
- connector context behavior; see [Agent Integration Reference](agent-integration.md)
- later candidate template bodies; they are not active documentation

## Authority boundary

Core-owned state and registered artifact refs are the authority. A projection, status card, Markdown report, rendered template, chat message, connector output, or agent context packet is display or support context only.

Templates cannot override Core state. A rendered view cannot authorize writes, create Write Authorization, satisfy gates, create evidence, perform verification, record QA, grant sensitive-action approval, record final acceptance, accept residual risk, create close readiness, close a Task, or mutate owner records. Those effects must come from the owner Core/API path.

Display labels are not canonical schema values. A localized label such as a user-readable judgment type is rendered from canonical fields such as `judgment_kind` and locale. If `display_label` appears in compatibility or response-only output, it remains display text and must not be treated as an enum value, storage value, API field owner, or schema category.

User edits to a projection are input only. They may become state only through an explicit owner path such as reconcile plus a Core state-changing action. Direct edits to managed text, front matter, displayed state, artifact refs, close status, acceptance status, residual-risk status, or template text do not become owner-recorded state.

## Projection is derived display

Projection is derived display from current Core-owned records and registered `ArtifactRef` metadata. It helps humans and agents read current work, source refs, blockers, evidence gaps, close blockers, freshness, and the next safe action. It is not a second state store.

The active MVP keeps projection small:

- four user-facing compact views: `status-card`, `judgment-request`, `run-evidence-summary`, and `close-result`
- one agent-facing support packet: `agent-context-packet`

These views may be rendered as compact text, cards, Markdown snippets, or structured payloads depending on the surface. They do not require persisted Markdown projection jobs or a full report renderer. The first internal smoke target may return plain structured status/blocker output instead of a rendered card.

Projection freshness is display over source clocks. A stale, failed, unknown, or too-broad readable view must not become the basis for state-changing work or close. If current state matters, retrieve current Core state or a current state-derived packet.

When a source record or ref is missing, render `none`, `unknown`, `not_required`, unavailable, or a blocking note. Do not invent placeholder state to satisfy a template.

Projection is also a privacy boundary. Render omission, redaction, blocked-artifact, and availability notes without reconstructing omitted or blocked raw values.

## Human-editable sections

Only explicitly marked human-editable sections are editable input. A common shape is:

```md
User Notes and Proposals:
-
```

Human-editable text can contain notes, questions, corrections, or proposals. It can propose changes to Task summary, scope, acceptance criteria, design notes, evidence notes, or other owner records, but the proposal is not the target record.

The state-changing path is explicit:

```text
human edit -> reconcile candidate -> explicit reconcile outcome -> Core state-changing action, rejection, deferral, or note
```

Until an owner path records a Core outcome, the human-editable text is not Task state, evidence, verification, QA, final acceptance, residual-risk acceptance, close readiness, or any other owner record.

Humans may not edit these directly into state:

- managed block content
- front matter fields such as `source_state_version`
- gate values, lifecycle phase, result, close reason, close status, or assurance level
- user judgment, sensitive-action approval, final-acceptance, residual-risk acceptance, close display text, or later/reserved QA waiver and verification-risk display text
- artifact identity, `sha256`, `size_bytes`, `content_type`, `redaction_state`, or artifact availability
- status cards, agent context packets, generated reports, or template bodies

## Managed block rules

Managed blocks are projector-owned Markdown regions:

```md
<!-- HARNESS:BEGIN managed -->
...
<!-- HARNESS:END managed -->
```

Rules:

- Managed block content is generated from committed Core-owned records and registered artifact refs.
- The projector may regenerate managed blocks.
- A managed block is display, not authority.
- Direct edits inside a managed block are drift, not owner-recorded state.
- If projection job storage is active, the projector records source state version, projection version or status, render timestamp, job status, and managed hash through the storage owner path.
- Active MVP compact views may instead carry read-time source/freshness text without a persisted projection job.
- Managed hash is computed over the projector-owned managed block body, excluding the marker lines, with normalized line endings and the projector's meaningful whitespace rules.
- Managed hash detects drift; it does not make Markdown state.
- If a managed block hash differs from the last projected hash before rendering, the projector reports drift or creates an owner-routed repair candidate. It does not silently accept the edited block.
- Regeneration must preserve unrelated human-editable sections.
- Rendering failure or stale source data must display `failed`, `stale`, `unknown`, or unavailable as appropriate. It must not roll back committed Core state, alter events, or change gate values.

Rendered views should include a short boundary notice near the top or managed summary: display only, derived from Core state and refs, not Write Authorization, not close result.

## ArtifactRef rendering

Large logs, diffs, traces, screenshots, recordings, bundles, export components, and sensitive artifact bodies are referenced by `ArtifactRef`, not embedded by default.

When useful to the reader or next action, render:

- artifact ref id
- owner relation or affected work ref
- artifact kind or source summary
- integrity metadata such as `sha256`, `size_bytes`, and `content_type`
- `redaction_state`
- availability state
- omission, redaction, or blocking note
- short reason the ref matters

Do not expand `secret_omitted`, `blocked`, unavailable, or redacted artifact bodies into Markdown. Do not reconstruct omitted raw values from metadata or surrounding prose.

A displayed `ArtifactRef` is a pointer to a registered artifact record. It is not, by itself, evidence sufficiency, verification, QA, final acceptance, residual-risk acceptance, or close readiness. If the ref lacks owner relation, integrity metadata, redaction state, or availability needed for the claim, show the gap.

## Active current MVP template set

The active current MVP template set is exactly:

| Audience | Template | Body |
|---|---|---|
| User-facing | `status-card` | [Status Card body](#status-card-body) |
| User-facing | `judgment-request` | [Judgment Request body](#judgment-request-body) |
| User-facing | `run-evidence-summary` | [Run / Evidence Summary body](#run--evidence-summary-body) |
| User-facing | `close-result` | [Close Result body](#close-result-body) |
| Agent-facing | `agent-context-packet` | [Agent Context Packet body](#agent-context-packet-body) |

The four user-facing outputs use ordinary language, source refs, and freshness only where they help the user decide, understand a blocker, inspect evidence, or understand close. They should not dump schemas, DDL, event logs, full artifacts, full report bodies, full evidence catalogs, or future catalogs.

The agent-facing packet is a separate audience. It carries only current, next-action-relevant refs, blockers, evidence gaps, close blockers, guarantee display level, and one next safe action. It is not user prose and is not authority.

## Status Card body

Use `status-card` when the active MVP needs a short user-visible current-state view. It shows what is happening now, what is in scope, what the user must decide, what evidence exists or is missing, what blocks close, and the next safe action.

Implementation tier: active MVP user work-loop view. The first internal smoke target may return plain structured status/blocker output instead of this card.

Boundary: this template is rendered display only. It is not Core state, not evidence, not approval, not final acceptance, not residual-risk acceptance, not Write Authorization, and not a close-readiness record. It must be rendered from current Core-owned state and refs, not stale chat.

Source records:

- current Task summary, work shape, and next safe action
- current scope, non-goals, and active Change Unit summary when useful to the user
- pending user judgments, rendered with user-readable labels
- active blockers and the plain reason progress or close is held
- current evidence summary, supporting refs, redaction or availability notes, and evidence gaps
- close blockers, final-acceptance need, residual-risk visibility, and residual-risk acceptance status when relevant
- design-quality routed action only when it changes the visible next step
- guarantee display level or unavailable capability status
- short source refs, render time, and freshness state

Rendered sections:

- work
- scope
- judgment
- blocked reason
- evidence
- checks
- close
- next safe action
- sources and freshness

Template:

````text
{task_id} {title}
Display only: derived from Core state and refs; not Core state or a Write Authorization.

Work: {work_shape}. {current_task_summary}
Scope: {scope_summary}
Out of scope: {non_goals|none}
Blocked because: {active_blocked_reason|none}
User must decide: {pending_user_judgments_with_localized_labels|none}
Evidence: {evidence_status}. {known_evidence_summary|none}
Evidence gaps: {evidence_gaps|none}
Checks: {check_summary|none}
Close: {close_readiness_summary}; blockers={close_blockers|none}
Design quality action: {design_quality_routed_action|none}
Residual risk: {residual_risk_visibility|none}
Next safe action: {next_safe_action}
Guarantee display: {guarantee_level_or_unavailable}; {guarantee_note}
Sources/freshness: {source_freshness_summary}
````

Notes:

- Keep this card readable for a user who does not know Harness internals.
- When a field has no source record, render `none`, `unknown`, `not_required`, or an explicit blocker instead of inventing state.
- Always render the guarantee display line. For active MVP default behavior, the note should say cooperative hold or detective reporting when that is the actual limit. If Core/MCP is unavailable, render the unavailable condition instead of a stale or guessed guarantee display value.
- Design-quality content should fit one line: the current routed action and, when blocking, the single next action.
- Agent-only refs and action-boundary details belong in [Agent Context Packet body](#agent-context-packet-body). Put a ref in the status card only when it helps the user decide, understand a blocker, or inspect source freshness.

## Judgment Request body

Use `judgment-request` when the user owns a choice that affects progress, scope, sensitive-action permission, final acceptance, residual-risk acceptance, or cancellation. This is the active MVP prompt shape for ordinary user-owned judgments. Later/reserved QA waiver and verification-risk prompts require a future promoted owner path before they become active values.

Implementation tier: active MVP user work-loop view. Full-format judgment presentation is later candidate scope and remains only a candidate in [Later Template Candidates](../later/index.md#later-template-candidates).

Boundary: this template displays a pending or recorded `user_judgment`; it does not create the judgment record by itself, grant Write Authorization, perform QA or verification, create QA evidence, record final acceptance, accept residual risk, accept verification risk, or close a Task.

Source records:

- pending or recorded `user_judgment`
- `judgment_kind`, `presentation`, and the locale-derived rendered judgment label
- exact question, rationale, recommendation, uncertainty, and no-decision consequence
- affected Task, Change Unit, write scope, close scope, sensitive-action scope, criteria, or other affected object
- options or selected outcome
- consequences, what the agent is not deciding, and why the agent cannot decide on the user's behalf
- minimal source refs needed to identify the affected work
- evidence, risk, approval, QA, verification, or close refs only when they affect the judgment

Rendered sections:

- judgment request
- localized judgment type
- exact question
- choices or selected outcome
- recommendation and rationale
- uncertainty
- affected work
- no-decision consequence
- what the agent is not deciding
- why the agent cannot decide
- next safe action or deferral effect
- refs

Template:

````text
Judgment request: {short_title}
Type: {localized_label_from_judgment_kind}
Question: {question}
Choices: {choices_or_selected_outcome}
Recommendation: {recommendation|none}
Why this matters: {rationale}
What is uncertain: {uncertainty}
Affected work: {affected_scope_summary}
If you do not decide: {no_decision_consequence}
What I will not decide for you: {not_deciding}
Why I need your answer: {why_agent_cannot_decide}
If deferred: {deferral_effect|not_applicable}
Next safe action after answer: {next_safe_action}
Refs: judgment={user_judgment_ref}; task={task_ref}; supporting={supporting_refs|none}
````

Notes:

- Small judgments should fit on one screen and use `presentation=short` in the active MVP. `presentation=full` and `Decision Packet` remain later candidate material until promoted by the owning user-judgment/template path.
- Do not merge sensitive approval, product decision, technical decision, scope decision, final acceptance, residual-risk acceptance, cancellation, or later/reserved QA waiver and verification-risk routes into one broad approval prompt.
- Chat phrases such as "yes, do it" satisfy a gate only when the scope, `judgment_kind`, affected object, and recorded user intent match the pending judgment.
- The displayed `Type` label is rendered from `judgment_kind` and the user's locale. It is display text only; the canonical judgment category remains `judgment_kind`.

<a id="run--evidence-summary-body"></a>

## Run / Evidence Summary body

Use `run-evidence-summary` after advice, a run, a check, or a change needs a minimal summary of what happened and what evidence now supports the current claim.

Implementation tier: active MVP user work-loop view. Detailed run reports and detailed evidence catalogs are later candidate scope.

Boundary: this template displays Run and evidence refs only. It is not the evidence itself, not a detailed evidence catalog, not verification, not QA, not final acceptance, not residual-risk acceptance, and not a close-readiness record.

Source records:

- Run refs and command/check summaries
- changed paths or no-file outcome
- consumed Write Authorization ref, no-write basis, or attempted invalid authorization context when relevant
- evidence refs, artifact refs, redaction, and availability notes
- completion claims, acceptance criteria, or close-relevant claims supported by the evidence
- evidence gaps, stale inputs, or unresolved support
- next safe evidence action

Rendered sections:

- run or action
- changed paths
- checks
- evidence refs
- supported claims
- gaps or stale support
- redaction and availability
- next evidence action

Template:

````text
Run/evidence summary
Display only: refs and summaries; not evidence, verification, QA, final acceptance, residual-risk acceptance, or close.

Action: {run_or_action_summary}
Changed paths: {changed_paths|none}
Checks: {checks_run_or_reason_not_run}
Write check: {write_check_summary|no product write}
Evidence: {evidence_status}. {evidence_summary}
Evidence refs: {evidence_refs|none}
Artifact refs: {artifact_ref_summary|none}
Redaction or availability: {redaction_availability_summary|none}
Supports: {supported_claims_or_criteria|none}
Still missing or stale: {evidence_gaps_or_stale_inputs|none}
Next safe evidence action: {next_evidence_action|none}
Sources/freshness: {source_freshness_summary}
````

Notes:

- Evidence sufficiency is coverage, not volume. If a claim has no current supporting ref, or a critical artifact ref lacks owner relation, integrity metadata, redaction state, or availability, show the gap and current evidence status instead of treating a long artifact list or report prose as sufficient support.
- Only a compatible consumed Write Authorization may be displayed as the product-write compatibility record for a product-write Run. Attempted invalid authorization refs may be shown only as violation/audit or validator-finding context, and they must not be rendered as a consumed Write Authorization or completion evidence.
- Keep this summary intentionally smaller than a full evidence report. Show the evidence refs and visible gaps needed for the user's next decision; do not expand full artifact inventories or raw artifact bodies.

## Close Result body

Use `close-result` when the user needs a compact close-readiness, close-blocker, or close-outcome display. It keeps acceptance, residual risk, evidence, artifact availability, self-check basis, and blockers separate.

Implementation tier: active MVP user work-loop view. Detailed continuity, release-handoff, or export reports are later candidate scope.

Boundary: this template displays close status. It does not close a Task, record final acceptance, accept residual risk, record verification or QA, create evidence, or change gate values. Only the Core close path can produce the close result.

Source records:

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

Rendered sections:

- close status
- scope
- evidence
- artifact availability and self-check basis
- judgment and acceptance
- residual risk
- blockers
- next safe action
- sources and freshness

Template:

````text
Close status: {ready|blocked|closed|not requested}
Display only: Core close state and owner refs remain authoritative.

Scope: {scope_summary}
Evidence: {evidence_status}. {evidence_summary}; gaps={evidence_gaps|none}
Artifact availability: {artifact_availability_summary}
Self-check basis: {self_check_summary|none}
Final acceptance: {final_acceptance_status}
Sensitive-action permission: {sensitive_permission_status|not_needed}
Design quality action: {design_quality_close_action|none}
Residual risk: {residual_risk_visibility}
Residual risk acceptance: {residual_risk_acceptance_status|not_needed}
Why close is blocked: {close_blockers|none}
Smallest unblocker: {smallest_unblocker|none}
Close basis or reason: {close_reason|not_applicable}
Next safe action: {next_safe_action|none}
Sources/freshness: {source_freshness_summary}
````

Notes:

- Do not collapse evidence summary, artifact availability, final acceptance, residual-risk visibility, residual-risk acceptance, blockers, design-quality routed actions, and readable-view freshness into one "done" line.
- Active MVP `close-result` output shows only active MVP close semantics; later assurance and detailed QA rows stay in later candidate scope.
- If close is blocked, name the primary blocker and the single next action, and keep secondary blockers visible only when they affect the next path.
- If the readable close view is stale or failed, fetch a current Core close result instead of closing from this template's prose.

## Agent Context Packet body

Use `agent-context-packet` when an agent needs compact, current context for the next safe action. It is optimized for accuracy, freshness, Core-derived refs, active blockers, unresolved user judgments, evidence gaps, close blockers, guarantee display level, and one next action, not for user-facing prose or full report detail.

Implementation tier: active MVP support view. It can be returned as a structured payload or prompt-sized text. It is not a required persisted Markdown projection.

Boundary: this packet is support context only. It cannot authorize writes, satisfy gates, create evidence, grant approval, record final acceptance, accept residual risk, create close readiness, or close a Task.

Source records:

- task and active Change Unit refs
- current state version and source refs
- active scope and non-goals
- unresolved user judgments
- active blockers
- evidence gaps
- close blockers
- residual-risk summary if active
- guarantee display level or unavailable capability status
- exactly one next safe action

Rendered sections:

- task and change unit refs
- state version and source refs
- active scope
- unresolved user judgments
- blockers
- next safe action
- evidence gaps
- close blockers
- residual-risk summary
- guarantee display level

Template:

````text
agent_context_packet:
  display_only: true
  authority: none; use current Core state for authority
  task_ref: {task_ref}
  change_unit_ref: {change_unit_ref|none}
  state_version: {source_state_version}
  source_refs: {source_refs}
  freshness: {freshness_state}
  active_scope: {scope_summary}
  unresolved_user_judgments: {pending_user_judgment_refs_with_kind_labels|none}
  blockers: {active_blockers|none}
  next_safe_action: {next_safe_action}
  evidence_gaps: {evidence_gaps|none}
  close_blockers: {close_blockers|none}
  residual_risk_summary: {residual_risk_summary_if_active|none}
  guarantee_level: {guarantee_level_or_unavailable}
````

Notes:

- Keep the packet one screen or less. It carries only current, next-action-relevant state.
- Do not include full schemas, full reference docs, full event logs, registered artifact file bodies, full report bodies, full templates, unrelated templates, full design-quality catalogs, or future catalog material by default.
- If the next action needs a fuller owner section, the agent should pull that owner section on demand instead of embedding it in the packet.
- The `guarantee_level` field is the required guarantee display context. If Core/MCP is unavailable, set it to the unavailable/capability condition and treat Harness-dependent state, write, evidence, acceptance, residual-risk, and close claims as unavailable until refreshed.

## Later template boundary

Later candidate template bodies are not active documentation and are not stored in this reference. Later template candidate names may appear only in [Later Template Candidates](../later/index.md#later-template-candidates), without bodies.

A later candidate listing does not create a current MVP requirement, active `ProjectionKind`, schema contract, runtime behavior, template body, generated Projection, evidence, verification, QA, final acceptance, residual-risk acceptance, close readiness, implementation task, or acceptance evidence.

To promote a later template, a future owner document must define a narrow scope, source records, fallback behavior, non-substitution rules, freshness behavior, proof-path expectations for future promotion, and exact owner placement. Until then, active current MVP output remains limited to the five templates in this document.
