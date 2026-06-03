# Concepts

## Start Here

Read this after [Overview](overview.md) when Harness terms start appearing in examples, status summaries, or reference docs.

This page gives the smallest vocabulary for the learning path. It starts with six ordinary user-facing concepts, then marks implementation labels as optional/internal. Users should not need those labels to ask for useful Harness behavior.

This page describes vocabulary for the design/review documentation and future Harness behavior. It does not mean these runtime records exist in this repository today.

## Main Idea

Harness vocabulary is built around authority boundaries.

Start with six ordinary questions:

- What work are we trying to do?
- What is in scope, and what is out of scope?
- What judgment or thing to decide belongs to the user?
- What evidence supports the claim?
- What check or verification has happened, or is still needed?
- What still prevents close?

The reference docs give exact names to records and APIs so future implementations can be precise. In learning docs, the plain questions come first.

## Six Plain Concepts

Use these concepts first in user-facing explanations.

| Concept | Plain meaning |
|---|---|
| Work | The thing the user wants completed, answered, investigated, or decided. |
| Scope | What may change, what stays out, and where the agent should stop before continuing. |
| Judgment or thing to decide | A choice the user owns, such as product direction, UX behavior, architecture trade-off, security/privacy call, permission for a sensitive step, QA waiver, work acceptance, or acceptance of a named remaining risk. |
| Evidence | Durable support for a claim: changed paths, diffs, logs, test output, screenshots, inspection notes, or other artifacts. |
| Check or verification | An ordinary confirmation such as a test, diff review, inspection, or source lookup; when the work needs a stronger boundary, a recorded correctness check or human QA expectation. |
| Close | The visible answer to "can this honestly finish?" It includes blockers, required work acceptance, and remaining risk when they matter. |

## User-Visible Work Shapes

Harness should usually feel like one of three work shapes.

| Work shape | Use when | What to show |
|---|---|---|
| Advice/read-only work | The user asks for explanation, planning, comparison, investigation, or recommendation. | What was inspected or cited, what is fact versus recommendation, and what decisions still belong to the user. |
| Small direct change | The user asks for a narrow edit with clear scope and low risk. | The small scope, changed paths, focused check or self-check, and whether anything made the scope grow. |
| Tracked work | The work has meaningful scope, user-owned judgment, evidence, QA, verification, work acceptance, or residual risk. | Scope, pending judgments, evidence, check/verification state, close blockers, work acceptance, and remaining risk. |

The user does not need to name these shapes. The agent should infer them from the task and explain scope growth when it happens.

## How Users Can Speak

Ordinary language is enough:

```text
Help me clarify the plan before implementation.
Show what I need to decide and what you can check yourself.
Tell me if the scope is getting bigger.
Show what still blocks closing this work.
What evidence supports the completion claim?
Show the remaining risks before I accept.
Inspect the current auth shape before recommending sessions, magic links, or OAuth/OIDC.
```

The agent may mention an internal label only when it helps explain a real boundary or reference link. The user should not have to start with that label.

## Non-Substitution Rules

These rules keep one kind of signal from being treated as another kind of authority.

| Rule | Meaning |
|---|---|
| Chat is not state. | Chat can coordinate and summarize, but it is not the durable operating record. |
| Readable report is not state. | A readable report can display state, but editing it does not change Core-owned state. Exact internal labels for readable views belong in optional/internal or Reference sections. |
| Tool output is not user judgment. | A log, diff, connector response, or test result can inform a decision; it cannot make the user's decision. |
| Sensitive-action approval is not work acceptance. | Permission to do a named sensitive step does not mean the completed result is accepted. |
| Test pass is not manual QA. | Automated checks do not prove human experience, copy, accessibility, or visual quality. |
| Self-check is not detached verification. | The same agent/session reviewing its own work can be useful, but it is weaker than an independent enough check. |
| "Proceed" or "looks good" does not automatically resolve every pending judgment. | A general phrase must not be stretched to cover unrelated product, technical, QA, work acceptance, or risk decisions. |

## Display Questions

User-facing status should normally group details under the same six concepts:

| Concept | Question |
|---|---|
| Work | What are we trying to do now? |
| Scope | What may change, and what is out of bounds? |
| Judgment | What must the user decide, accept, waive, or defer? |
| Evidence | What supports the claim, and what support is missing or stale? |
| Check or verification | What was checked, what still needs checking, and whether a stronger review boundary or human QA is needed? |
| Close | What still prevents finish or close? |

These display questions are a reading aid. They do not create schema fields, readiness labels, authority paths, or close rules. Exact rules live in Reference docs.

## Optional/Internal Labels

The labels below are implementation or reference labels. Users do not need to use them in prompts. Learn and Use docs should introduce them only when they clarify a real boundary.

| Optional/internal label | Plain explanation |
|---|---|
| Task | Internal durable unit for the work the user wants completed, answered, investigated, or decided. |
| Discovery | Optional/internal label for clarifying blurry work before implementation planning. Users can simply ask, "help me clarify the plan before implementation." |
| Shared Design | Internal record of shared understanding for goal, value, scope, non-goals, assumptions, decisions, and safe next work. |
| Change Unit | Optional/internal label for bounded product-write scope. It names what may change, but does not authorize a write by itself. |
| Autonomy Boundary | Internal label for choices the agent may make inside the active scope without asking again. |
| Decision Packet | Optional/internal label for the recorded path for a specific user-owned judgment. Users can simply answer the named decision. |
| Approval | Permission for a named sensitive action. It is not work acceptance for the completed work. |
| Write Authorization | Internal record that a specific product-write attempt is compatible with current Harness authority. |
| Evidence Manifest | Internal record mapping completion conditions or acceptance criteria to evidence references. |
| Eval | Internal verification result record. |
| Gate | Internal readiness or compatibility condition. User-facing status should usually show the blocker or check first. |
| Verification | Recorded correctness check, with stronger meaning when it crosses an independent enough boundary. |
| Manual QA | Human inspection for UX, copy, accessibility, visual quality, workflow, or other human-judgment surfaces. |
| Acceptance | User's work acceptance judgment when required. |
| Residual Risk | Known remaining uncertainty, limitation, trade-off, or consequence. |
| Projection | Readable view derived from Harness state. It displays state but does not replace it. |
| Journey Card / Journey Spine | Later continuity display. It can help orientation when enabled and fresh, but it is not authority by itself. |
| Reconcile | Deliberate path for handling human edits or drift in a readable view. |
| task_events | Low-level internal event history for implementers and diagnostics. |

## Stable Anchors For Older Links

These anchors keep links stable. They do not make the terms required user vocabulary.

- <a id="task"></a>Task: optional/internal label for the durable work unit.
- <a id="shared-design"></a>Shared Design: optional/internal label for recorded shared understanding.
- <a id="change-unit"></a>Change Unit: optional/internal label for bounded product-write scope.
- <a id="autonomy-boundary"></a>Autonomy Boundary: optional/internal label for in-scope agent choices.
- <a id="decision-packet"></a>Decision Packet: optional/internal label for recording a specific user-owned judgment.
- <a id="evidence"></a>Evidence: support for a completion or correctness claim.
- <a id="approval"></a>Approval: permission for a named sensitive action.
- <a id="write-authorization"></a>Write Authorization: optional/internal label for a compatible write attempt.
- <a id="gate"></a>Gate: optional/internal label for a readiness or compatibility condition.
- <a id="verification"></a>Verification: recorded correctness check.
- <a id="manual-qa"></a>Manual QA: human inspection where human judgment matters.
- <a id="acceptance"></a>Acceptance: work acceptance judgment when required.
- <a id="residual-risk"></a>Residual Risk: known remaining uncertainty, limitation, trade-off, or consequence.
- <a id="projection"></a>Projection: readable view derived from state, not authority itself.
- <a id="reconcile"></a>Reconcile: path for readable-view drift or human edits.

## Where Exact Rules Live

Strict kernel definitions live in [Kernel Reference](../reference/kernel.md), public API definitions live in [MCP API and Schemas](../reference/mcp-api-and-schemas.md), and readable document rules live in [Document Projection Reference](../reference/document-projection.md).
