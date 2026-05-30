# Harness in 15 Minutes

## What this document helps you do

Use six short scenarios to understand how Harness feels in ordinary AI-assisted work before you read the heavier Reference docs.

After reading it, you should be able to tell when a task can stay tiny, when it needs clarification, why user judgment can block work, what evidence is doing, why close can still be blocked, and why a readable Markdown report is not state.

## Read this when

Read this when you are new to Harness and want practical examples before learning exact gates, schemas, DDL, projection rules, or conformance fixtures.

## Before you read

No Harness background is required. If you want the longer mental model first, read [Overview](overview.md). If you want a full task story after this page, read [Harness in One Task](harness-in-one-task.md).

## Main idea

Harness keeps AI-assisted work followable by making a few things explicit: what is being attempted, what may change, what the user must decide, what supports the completion claim, what risk remains, and whether the work can close.

The examples below are onboarding examples, not schemas or new authority paths. Exact behavior stays with the Reference owners linked at the end.

## Scenario 1: Tiny docs change

The user says:

```text
Fix the typo in this install note.
```

The useful Harness shape is intentionally small:

- Scope: one named docs sentence or paragraph.
- Out of scope: meaning changes, link behavior changes, rendered output changes, contract changes, or adjacent cleanup.
- Change: edit the typo.
- Evidence: changed path plus a short self-check that the edit is spelling-only.
- Close: report the tiny result and whether anything forced escalation.

What the user should see is compact:

```text
Fixed the typo in `docs/install.md`.
Self-check: spelling-only, no meaning or contract change.
Closed as tiny direct. Residual risk: none known for this close.
```

Tiny direct is still under `direct`; it is not a separate mode and does not bypass user-owned judgment, security boundaries, evidence, scope, write permission, residual-risk visibility, or close rules. If the edit changes meaning, needs link/render proof, touches a strict Reference contract, or grows beyond changed-path plus self-check support, the same work should move to ordinary `direct` or `work`.

For exact mode, evidence, and close behavior, use [Kernel Reference](../reference/kernel.md#mode), [Evidence Sufficiency Profiles](../reference/kernel.md#evidence-sufficiency-profiles), and [`close_task`](../reference/kernel.md#close_task).

## Scenario 2: Direct code change

The user says:

```text
Fix null date formatting in the invoice summary.
```

This is still small, but product code may change. Harness should keep the work narrow:

- Scope: the date formatting helper or caller and a focused test.
- Out of scope: invoice data model changes, localization strategy, billing behavior, or public API changes.
- Before write: the active scope must cover the intended paths, and Harness must allow the specific write attempt.
- Evidence: diff or patch summary plus the focused test, or a recorded reason no automated check applies.
- Close: usually self-checked if the task stays narrow and no required QA, detached verification, final acceptance, or residual-risk path applies.

The user-facing result can still be simple:

```text
Changed null invoice dates to render as "Not set."
Checked with `invoiceSummary.test`.
The write was allowed for the implementation run.
Closed self-checked; no close-relevant residual risk is known.
```

If the agent discovers the formatter is shared across exports, reporting, billing emails, and API responses, the task is no longer just a direct code fix. Harness should stop and shape the wider impact before product files change further.

The Reference docs name the scoped write boundary a Change Unit, and the write allow/deny decision Write Authorization. For exact write and evidence authority, use [Change Unit](../reference/kernel.md#change-unit), [Write Authorization](../reference/kernel.md#write-authorization), [`prepare_write`](../reference/kernel.md#prepare_write), and [Evidence Gate](../reference/kernel.md#evidence-gate).

## Scenario 3: Feature that needs clarification

The user says:

```text
Add remember-me behavior to login.
```

This sounds small, but it affects product behavior, security, session lifetime, UI, tests, and possibly storage. Harness should use Discovery before implementation planning:

```text
Goal: add remember-me behavior.
Need to clarify: extend session, remember email, or both.
Codebase-answerable: where session lifetime is configured today.
Possible first safe scoped slice: login checkbox, chosen session behavior, focused tests.
User question: Should remember-me extend the session on this device, prefill the email address, or both?
```

Discovery separates product, technical, security, QA, operational, and scope questions. It answers codebase-answerable questions from the repository and current Harness context, then asks the user only for decisions the codebase cannot answer.

Discovery is not sensitive-action permission (Approval), not permission to write product files, not evidence, not verification, not QA, not final acceptance (Acceptance), not residual-risk acceptance, not close, and not a new authority path. It is the clarification work that can make a first safe scoped slice visible.

For the user-facing flow, use [User Guide](../use/user-guide.md#first-read-path) and [Agent Session Flow](../use/agent-session-flow.md). For exact owner behavior behind the terms, use [Kernel Reference](../reference/kernel.md) and [MCP API And Schemas](../reference/mcp-api-and-schemas.md).

## Scenario 4: User judgment blocks work

During the login work, the agent reaches a user-owned UX choice:

```text
Failed-login feedback can be inline, a toast, or a modal.
```

This should not become a vague "approve?" prompt. The agent should show the real choice, options, recommendation, uncertainty, and deferral consequence:

```text
Judgment type: Product / UX
Why now: final UI behavior and tests need one failure-feedback pattern.
Options: inline message, toast, modal.
Recommendation: inline message near the form; it is persistent and accessible.
Uncertainty: confirm existing design-system error-message support.
Deferral consequence: API and state wiring can continue, but final UI behavior and Manual QA should wait.
```

If the decision is blocking, Harness records that user-owned judgment through the Decision Packet path. Chat text, a broad "go ahead," or readable report prose alone should not satisfy the decision unless it answers the specific recorded choice. A Decision Packet is also not sensitive-action Approval unless it is approval-shaped and linked to the Approval path.

For practical examples, read [Decision Packet Cookbook](../use/decision-packet-cookbook.md). For exact behavior, use [Decision Packet](../reference/kernel.md#decision-packet), [Decision Gate](../reference/kernel.md#decision-gate), [`harness.request_user_decision`](../reference/mcp-api-and-schemas.md#harnessrequest_user_decision), and [`harness.record_user_decision`](../reference/mcp-api-and-schemas.md#harnessrecord_user_decision).

## Scenario 5: Evidence and close blocker

The agent finishes a feature and says:

```text
The code is done and tests pass.
```

Harness may still block close if the close-relevant support is incomplete. That does not mean the work failed; it means the close basis is not complete yet.

Common examples:

- Evidence is partial because an acceptance criterion has no supporting ref.
- Verification is required but no compatible Eval exists.
- Manual QA is required for UI behavior and has not passed or been validly waived.
- Final acceptance is required but has not been requested with evidence, QA, verification, and residual-risk visibility.
- Known close-relevant residual risk exists but has not been made visible or accepted.

A useful close blocker names the smallest unblocker:

```text
Close blocked: Manual QA is still pending for the login error workflow.
Smallest unblocker: record Manual QA, or ask for a QA waiver Decision Packet that names the skipped check and, if close-relevant risk remains, routes residual-risk acceptance separately.
```

Waivers and residual-risk-accepted close paths should stay explicit. A verification waiver does not create detached verification. A QA waiver does not prove the UI was inspected. Residual-risk acceptance does not make the risk disappear.

For exact close and gate behavior, use [`close_task`](../reference/kernel.md#close_task), [Evidence Gate](../reference/kernel.md#evidence-gate), [Verification Gate](../reference/kernel.md#verification-gate), [QA Gate](../reference/kernel.md#qa-gate), [Acceptance Gate](../reference/kernel.md#acceptance-gate), and [Residual Risk](../reference/kernel.md#residual-risk).

## Scenario 6: A readable report is not state

A `TASK` Markdown report says:

```text
Evidence: partial
Next action: record Manual QA
source_state_version: 42
```

That report is useful, but it is not the operating record. The implementation term is projection: a readable view rendered from current state records and artifact refs.

If a human edits the report to say:

```text
Evidence: sufficient
```

that edit does not change the Evidence Manifest, gate state, Manual QA status, Acceptance state, Residual Risk, or close eligibility. Human-editable sections can become notes or reconcile input, but accepted state changes still need the owner Core/MCP path.

The practical rule is simple: read projections for orientation, refs, and freshness; use owner records and owner actions for authority. If a projection is stale or wrong, refresh or reconcile it instead of treating the Markdown as state.

For the exact projection boundary, use [Document Projection Reference](../reference/document-projection.md), especially [Projection in plain language](../reference/document-projection.md#projection-in-plain-language).

## Reference owners for this tour

| Topic | Owner for exact behavior |
|---|---|
| Task, Change Unit, Decision Packet, gates, evidence, verification, QA, Acceptance, Residual Risk, close | [Kernel Reference](../reference/kernel.md) |
| Public tool request and response shapes | [MCP API And Schemas](../reference/mcp-api-and-schemas.md) |
| Markdown projection authority and freshness | [Document Projection Reference](../reference/document-projection.md) |
| User-facing session flow and status reading | [User Guide](../use/user-guide.md), [Agent Session Flow](../use/agent-session-flow.md) |
| Practical Decision Packet examples | [Decision Packet Cookbook](../use/decision-packet-cookbook.md) |

## Where to go next

- Read [Harness in One Task](harness-in-one-task.md) for a fuller direct and work task story.
- Read [Decision Packet Cookbook](../use/decision-packet-cookbook.md) when a user-owned judgment blocks progress.
- Read [User Guide](../use/user-guide.md) when you are running a real session.
- Use Reference docs only when you need exact contracts.
