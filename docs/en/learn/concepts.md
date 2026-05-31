# Concepts

## What this document helps you do

This document gives you the smallest vocabulary for using Harness in ordinary work. It starts with public user-facing words, then lists the internal implementation terms you may see in reference docs, status cards, or API-shaped examples.

Harness should be usable even when you do not know the internal labels. You can talk about work, scope, judgment, evidence, close readiness, and risk; the agent translates that into Harness records and procedures when precision is needed.

The exact kernel, runtime, MCP API, and document-rendering contracts live in the reference path.

In the primary reader path, read this after [Overview](overview.md) and [User Guide](../use/user-guide.md). This page is a vocabulary bridge, not another product overview or task tutorial.

## Read this when

Read this when Harness terms are starting to appear in examples, status summaries, or reference specs and you want the plain meaning first.

## Before you read

[Overview](overview.md) is recommended first. No schema or implementation knowledge is required.

## Main idea

Start with ordinary questions:

- What work are we trying to do?
- What is in scope, and what is out of scope?
- What judgment belongs to the user?
- What evidence supports the claim that the work is done?
- What still affects close readiness?
- What risk remains?

Harness gives those questions exact implementation names so agents, tools, and records can agree. The names matter in references and APIs, but the plain questions come first.

## Public Vocabulary

Use these words in user-facing docs, prompts, and status summaries.

| Public term | Plain meaning |
|---|---|
| Work | The thing the user wants completed, answered, investigated, or decided. |
| Scope | What may change, what must stay out of bounds, and where the agent should stop before continuing. |
| Judgment | A choice the user owns, such as a product direction, important technical trade-off, sensitive action, QA waiver, accepting the result, or risk decision. |
| Evidence | Durable support for a claim, such as changed paths, diffs, test output, logs, screenshots, run summaries, QA notes, or verification results. |
| Close readiness | What still has to be true before the work can finish: checks, QA, final acceptance, risk visibility, and blockers. |
| Risk | Known uncertainty, limitation, skipped check, trade-off, or possible consequence that should stay visible instead of disappearing behind "done." |

User-facing docs should explain the easy concept first. Add the exact internal term in parentheses only when it helps the reader understand a real stop, boundary, or reference link.

## User-Facing Display Groups

When Harness status is shown to users, the many internal details should usually appear as four readable display groups:

| Display group | Plain question | What it usually shows |
|---|---|---|
| Scope | What may change? | The agreed work area, out-of-bounds items, and whether the next intended action fits. |
| Judgment | What must the user decide? | Product, technical, security, QA, final acceptance, risk, permission, or scope choices that need the user. |
| Evidence | What supports completion claims? | The current support, missing support, stale support, and evidence refs when they matter. |
| Close readiness | What still prevents finish or close? | Verification, Manual QA, accepting the result, visible risk, accepted risk when relevant, and close blockers. |

These groups are a reading aid, not a replacement for the kernel gate taxonomy. They do not create schema fields, gate values, recompute inputs, authority paths, or close rules. Strict gate behavior stays in [Kernel Reference](../reference/kernel.md#gates), public API behavior stays in [MCP API and Schemas](../reference/mcp-api-and-schemas.md), and readable document rendering rules stay in [Document Projection Reference](../reference/document-projection.md).

## How Users Can Speak

Users do not need a command language. These are enough:

```text
Add email login. Keep password reset and account creation out of scope.
Clarify the plan before implementation.
Ask what you need before changing code.
Help plan better onboarding; inspect what exists and separate product choices from facts.
Inspect our auth shape before recommending sessions, magic links, or OAuth/OIDC.
Show what is blocking this work.
What evidence supports the completion claim?
Show close readiness before I accept.
Show the remaining risks.
```

The agent may answer with internal labels when they clarify what Harness recorded or why the work is stopped. For example, it may say "the scope (Change Unit) does not include the new auth file" or "this needs a user judgment (Decision Packet)." Users should not need to begin with those labels.

## Internal Implementation Terms

These terms are exact implementation names used by references, APIs, schemas, records, or status refs. Users do not need to use them in prompts.

| Internal term | Plain-language explanation |
|---|---|
| Task | The durable unit for the work the user wants completed, answered, investigated, or decided. |
| Discovery | The internal name for the agent's requirements-clarification posture before implementation planning when goals, value, non-goals, acceptance criteria, user-owned judgments, QA expectations, uncertainty, or safe next work need shaping. Users can ask for this in ordinary language. |
| Shared Design | A recorded shared understanding of goal, value, scope, non-goals, assumptions, decisions, and safe next-work shape for blurry work. |
| Change Unit | The bounded work scope for product writes. It names what may change, but does not authorize a write by itself. |
| Autonomy Boundary | The choices the agent may make inside the active scope without asking the user again. |
| Decision Packet | The recorded path for a specific user-owned judgment that blocks progress, write, waiver, acceptance, risk handling, or close. |
| Approval | Permission for a named sensitive action. It is not final acceptance and does not prove correctness. |
| Write Authorization | The Harness result that a specific product-write attempt may proceed now, after scope and other checks. |
| Evidence Manifest | A record mapping completion conditions or acceptance criteria to the evidence that supports them. |
| Verification | A recorded check of correctness, with stronger meaning when it happens across an independent boundary. |
| Manual QA | Human inspection for quality that needs human judgment, such as UI, UX, copy, accessibility, workflow, or visual output. |
| Acceptance | The user's judgment that the result is acceptable when the task path requires final acceptance. |
| Residual Risk | Known remaining uncertainty, limitation, or trade-off after the work. |
| Projection | A readable view rendered from Harness state, such as a report or Journey Card. It displays state but does not replace it. |
| Reconcile | The deliberate path for handling human edits or drift in a readable view. |
| `task_events` | The internal event log table for task state changes. It is a reference/schema term, not user-facing vocabulary. |

Reference docs contain many more terms because they must be precise about records, APIs, storage, and conformance. Learn and Use docs should normally stay with the public vocabulary unless an internal label explains a real boundary.

## Implementation Term Lookup

These compact anchors keep older links stable without making users learn the implementation vocabulary first.

- <a id="task"></a>Task: internal durable unit for work.
- <a id="shared-design"></a>Shared Design: recorded shared understanding for blurry work.
- <a id="change-unit"></a>Change Unit: bounded product-write scope.
- <a id="autonomy-boundary"></a>Autonomy Boundary: choices the agent may make inside scope.
- <a id="decision-packet"></a>Decision Packet: path for recording a specific user-owned judgment.
- <a id="evidence"></a>Evidence: support for a completion or correctness claim.
- <a id="approval"></a>Approval: permission for a named sensitive action.
- <a id="write-authorization"></a>Write Authorization: one-attempt allowance for a compatible write.
- <a id="verification"></a>Verification: recorded correctness check.
- <a id="manual-qa"></a>Manual QA: human inspection where human judgment matters.
- <a id="acceptance"></a>Acceptance: final acceptance when required by the task path.
- <a id="residual-risk"></a>Residual Risk: known remaining uncertainty, limitation, or trade-off.
- <a id="projection"></a>Projection: readable view rendered from Harness state, not authority.
- <a id="reconcile"></a>Reconcile: path for handling readable-view drift or human edits.

## Important Separations

These separations keep Harness from turning a casual phrase into the wrong authority:

- Scope is not write permission. A scoped work area can exist before a specific write is allowed.
- Judgment is not broad approval. The user should be asked for the named choice that matters.
- Approval is only permission for a sensitive action. It is not final acceptance, evidence, QA, verification, or risk acceptance.
- Evidence is not the agent saying "done." It is recorded support for the claim.
- A readable report is not the operating record. It should point back to the records and evidence it summarizes.
- Close readiness is not a new gate. It is a user-facing summary of what still blocks finish or close.
- Accepting risk is not proof that the work is correct. It means the remaining uncertainty was visible and accepted for this work.

## Where Exact Rules Live

Strict kernel definitions live in [Kernel Reference](../reference/kernel.md), public API definitions live in [MCP API and Schemas](../reference/mcp-api-and-schemas.md), and readable document rules live in [Document Projection Reference](../reference/document-projection.md).
