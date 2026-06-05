# Judgment Request Template

## Used when

Use `judgment-request` when the user owns a choice that affects progress, scope, sensitive-action permission, QA waiver, verification-risk acceptance, final acceptance, residual-risk acceptance, or cancellation. This is the MVP-1 prompt shape for ordinary user-owned judgments.

Implementation tier: MVP-1 User Work Loop view. The full Decision Packet presentation is later/full-profile scope and lives in [later-profile/decision-packet.md](later-profile/decision-packet.md).

Boundary: this template displays a pending or recorded `user_judgment`; it does not create the judgment record by itself, grant Write Authorization, perform QA or verification, create QA evidence, record final acceptance, accept residual risk, accept verification risk, or close a Task.

## Source records

- pending or recorded `user_judgment`
- `judgment_kind`, `presentation`, and the locale-derived rendered judgment label
- exact question, rationale, recommendation, uncertainty, and no-decision consequence
- affected Task, Change Unit, write scope, close scope, criteria, paths, gates, sensitive-action scope, or other affected object
- options or selected outcome
- consequences, what the agent is not deciding, and why the agent cannot decide on the user's behalf
- minimal source refs needed to identify the affected work
- evidence, risk, approval, QA, verification, or close refs only when they affect the judgment

## Rendered sections

- judgment request
- judgment kind
- exact question
- choices or selected outcome
- recommendation and rationale
- uncertainty
- affected scope
- no-decision consequence
- agent is not deciding
- why agent cannot decide
- next safe action or deferral effect
- refs

## Full template

````text
Judgment request: {short_title}
Judgment kind: {rendered_judgment_label} (`{judgment_kind}`)
Exact question: {question}
Choices: {choices_or_selected_outcome}
Recommendation: {recommendation|none}
Rationale: {rationale}
Uncertainty: {uncertainty}
Affected scope: task={task_ref}; change_unit={change_unit_ref|none}; write={write_scope_refs|none}; close={close_scope_refs|none}; object={affected_object_refs|none}
If you do not decide: {no_decision_consequence}
Agent is not deciding: {not_deciding}
Why the agent cannot decide: {why_agent_cannot_decide}
If deferred: {deferral_effect|not_applicable}
Next safe action after answer: {next_safe_action}
Refs: judgment={user_judgment_ref}; task={task_ref}; scope={scope_ref|none}; evidence={evidence_refs|none}; risk={risk_refs|none}
````

## Notes

Small judgments should fit on one screen. Use `presentation=full` only when the active profile or complexity requires fuller trade-offs, recommendation, affected gates, evidence/risk refs, and deferral analysis.

Do not merge sensitive approval, product decision, technical decision, scope decision, QA waiver, verification-risk acceptance, final acceptance, residual-risk acceptance, or cancellation into one broad approval prompt. Chat phrases such as "yes, do it" satisfy a gate only when the scope, `judgment_kind`, affected object, and recorded user intent match the pending judgment.
