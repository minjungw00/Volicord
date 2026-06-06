# Judgment Request Template

## Used when

Use `judgment-request` when the user owns a choice that affects progress, scope, sensitive-action permission, QA waiver, verification-risk acceptance, final acceptance, residual-risk acceptance, or cancellation. This is the MVP-1 prompt shape for ordinary user-owned judgments.

Implementation tier: MVP-1 User Work Loop view. The full Decision Packet presentation is later/full-profile scope and lives in [../../later/index.md#later-template-candidates](../../later/index.md#later-template-candidates).

Boundary: this template displays a pending or recorded `user_judgment`; it does not create the judgment record by itself, grant Write Authorization, perform QA or verification, create QA evidence, record final acceptance, accept residual risk, accept verification risk, or close a Task.

## Source records

- pending or recorded `user_judgment`
- `judgment_kind`, `presentation`, and the locale-derived rendered judgment label
- exact question, rationale, recommendation, uncertainty, and no-decision consequence
- affected Task, Change Unit, write scope, close scope, sensitive-action scope, criteria, or other affected object
- options or selected outcome
- consequences, what the agent is not deciding, and why the agent cannot decide on the user's behalf
- minimal source refs needed to identify the affected work
- evidence, risk, approval, QA, verification, or close refs only when they affect the judgment

## Rendered sections

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

## Full template

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

## Notes

Small judgments should fit on one screen. Use `presentation=full` only when the active profile or complexity requires fuller trade-offs, recommendation, affected gates, evidence/risk refs, and deferral analysis.

Do not merge sensitive approval, product decision, technical decision, scope decision, QA waiver, verification-risk acceptance, final acceptance, residual-risk acceptance, or cancellation into one broad approval prompt. Chat phrases such as "yes, do it" satisfy a gate only when the scope, `judgment_kind`, affected object, and recorded user intent match the pending judgment.

The displayed `Type` label is rendered from `judgment_kind` and the user's locale. It is display text only; the canonical judgment category remains `judgment_kind`.
