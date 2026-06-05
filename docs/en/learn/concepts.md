# Concepts

## Start Here

Read this after [Overview](overview.md) and [One Task](one-task.md), or when a status summary starts using Harness vocabulary.

This page keeps only the minimum concepts for first-time readers. It describes intended future Harness behavior in a documentation-only repository. It does not mean runtime records already exist here.

## Seven Questions

Harness is easiest to read through seven ordinary questions:

| Question | Plain concept |
|---|---|
| What are we trying to do? | Work |
| What may change, and what is out of bounds? | Scope |
| What will count as a good enough result? | Success criteria |
| What must the user decide, accept, waive, or defer? | User-owned judgment |
| What supports the claim? | Evidence |
| What was checked, and what still needs checking? | Check, verification, or manual QA |
| Can this honestly finish, and what still blocks it? | Close readiness |

Users do not need to say these words exactly. They can ask in ordinary language:

```text
Make this plan concrete enough to implement.
Turn this feature idea into an implementable plan.
Ask me first about the parts only I can decide.
Before changing files, confirm which files you expect to touch.
Before you say it is done, show the evidence and remaining risks.
```

## Three Work Shapes

Harness should usually feel like one of three work shapes.

| Shape | Feels like | Boundary |
|---|---|---|
| Advice/read-only work | Explanation, planning, comparison, investigation, recommendation. | Advice can guide work, but it does not authorize product writes or accept risk. |
| Small direct change | A narrow typo, copy, or leaf fix with low risk. | Keep it light while the scope holds; stop if meaning, risk, UX, sensitive action, or shared-contract impact appears. |
| Tracked work | Meaningful scope, success criteria, user choices, evidence, QA, verification, acceptance, or residual risk. | Keep scope, judgments, evidence, checks, remaining risk, and close blockers visible. |

The agent should infer the shape from the request and explain when the shape changes. If enough information exists, tracked work should move toward the next safe implementation unit instead of staying in planning.

## Non-Substitution Rules

These are the rules that make the concepts matter:

| Rule | Meaning |
|---|---|
| Chat is not state. | Chat can coordinate, but it is not the durable operating record. |
| Readable summaries are not state. | A report can display status, but editing report text does not change the future Harness record. |
| Tool output is not user judgment. | Logs, diffs, tests, screenshots, and connector responses can inform choices; they do not make choices. |
| Sensitive-step permission is not final acceptance. | Permission for a named action does not accept the finished result. |
| Evidence is not verification. | Evidence supports a claim; verification says what was checked and how. |
| Test pass is not manual QA. | Automated checks do not prove copy, accessibility, visual quality, or user experience. |
| Residual-risk visibility is not residual-risk acceptance. | Showing a known remaining risk does not mean the user accepted it. |
| "Proceed" or "looks good" is not every answer. | A broad phrase should not resolve unrelated judgments by implication. |

## Labels You May See Later

Internal labels are lookup tools, not first-use concepts. Learn and Use pages should explain the plain situation first.

| Label | Plain reading |
|---|---|
| Task | The durable unit for a piece of work. |
| Change Unit | The bounded product-write scope. It does not authorize a write by itself. |
| Decision Packet | A fuller display for a specific user-owned judgment. |
| Approval | Permission for a named sensitive action, not final acceptance. |
| Write Authorization | The internal record/check behind pre-write scope confirmation. It is not OS permission or sandboxing. |
| Evidence Manifest | A detailed evidence list. |
| Verification / Manual QA / Acceptance / Residual Risk | Separate records or judgments that must not collapse into one "done." |
| Projection | A readable view derived from Harness records, not authority by itself. |

Exact definitions, API shapes, storage rules, and state transitions live in Reference docs, not in Learn.

## Stable Anchors For Older Links

These anchors keep older links from drifting. They do not make the terms required user vocabulary.

- <a id="task"></a>Task
- <a id="shared-design"></a>Shared Design
- <a id="change-unit"></a>Change Unit
- <a id="autonomy-boundary"></a>Autonomy Boundary
- <a id="decision-packet"></a>Decision Packet
- <a id="evidence"></a>Evidence
- <a id="approval"></a>Approval
- <a id="write-authorization"></a>Write Authorization
- <a id="gate"></a>Gate
- <a id="verification"></a>Verification
- <a id="manual-qa"></a>Manual QA
- <a id="acceptance"></a>Acceptance
- <a id="residual-risk"></a>Residual Risk
- <a id="projection"></a>Projection
- <a id="reconcile"></a>Reconcile

## Where Exact Rules Live

Use [Core Model Reference](../reference/core-model.md) for exact authority rules, [MVP API](../reference/api/mvp-api.md) for public future tool behavior, [API Schema Core](../reference/api/schema-core.md) for shared shapes, and [Projection And Templates Reference](../reference/projection-and-templates.md) for readable-view rules.
