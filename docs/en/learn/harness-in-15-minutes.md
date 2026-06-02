# Harness in 15 Minutes

## Start with familiar requests

You can ask for a typo fix, a focused code change, help clarifying a feature, a user-owned decision prompt, missing evidence, or the current close blockers. The agent should translate that ordinary request into a small visible shape: scope, what it can inspect, what it needs from you, what evidence matters, and what you can expect to see next.

Harness preserves the facts that tend to get lost in chat: scope, user-owned judgment, evidence, verification, QA expectations, final acceptance, residual risk, and close readiness. These six short scenarios show how those facts should feel in ordinary AI-assisted work.

## Read this when

Read this when you are new to Harness and want practical examples before exact Reference details.

## Before you read

No Harness background is required, but the primary path starts with [Overview](overview.md). If you want a full task story after this page, read [Harness in One Task](harness-in-one-task.md).

## Main idea

Harness keeps AI-assisted work followable by making a few things explicit: what is being attempted, what may change, what the user must decide, what supports the completion claim, what risk remains, and whether the work can close.

Users should still speak normally. Requests such as "review this feature idea and ask the questions needed before implementation," "make a small copy change, but tell me if it becomes a broader product decision," or "before changing code, separate the product decisions from the technical decisions" are enough for the agent to choose the right Harness shape.

The examples below are onboarding examples, not exact contracts. Exact behavior stays with the Reference owners linked at the end.

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
Closed as a small change. No known close-relevant risk for this change.
```

A tiny fix still has boundaries. If the edit changes meaning, needs link or render proof, touches a strict contract, or grows beyond changed-path plus self-check support, the same work should get the extra structure that the broader scope needs.

For exact mode, evidence, and close behavior, use [Kernel Reference](../reference/kernel.md#mode), [Evidence Sufficiency Profiles](../reference/kernel.md#evidence-sufficiency-profiles), and [`close_task`](../reference/kernel.md#close_task).

## Scenario 2: Small code change

The user says:

```text
Fix null date formatting in the invoice summary.
```

This is still small, but product code may change. Harness should keep the work narrow:

- Scope: the date formatting helper or caller and a focused test.
- Out of scope: invoice data model changes, localization strategy, billing behavior, or public API changes.
- Before writing: the intended paths should fit the active scope, and the write check should allow this focused change.
- Evidence: diff or patch summary plus the focused test, or a recorded reason no automated check applies.
- Close: usually self-checked if the task stays narrow and no required QA, detached verification, final acceptance, or residual-risk path applies.

The user-facing result can still be simple:

```text
Changed null invoice dates to render as "Not set."
Checked with `invoiceSummary.test`.
Write check: allowed for this focused change.
Closed self-checked; no close-relevant residual risk is known.
```

If the agent discovers the formatter is shared across exports, reporting, billing emails, and API responses, the task is no longer just a small code change. Harness should stop and shape the wider impact before product files change further.

Reference docs define the scoped write boundary and write-check record more strictly. For exact write and evidence authority, use [Change Unit](../reference/kernel.md#change-unit), [Write Authorization](../reference/kernel.md#write-authorization), [`prepare_write`](../reference/kernel.md#prepare_write), and [Evidence Gate](../reference/kernel.md#evidence-gate).

## Scenario 3: Feature that needs clarification

The user says:

```text
Add remember-me behavior to login.
```

This sounds small, but it affects product behavior, security, session lifetime, UI, tests, and possibly storage. The agent should clarify the request before implementation planning:

```text
Goal: add remember-me behavior.
Need to clarify: extend session, remember email, or both.
Codebase-answerable: where session lifetime is configured today.
Possible scoped next work: inspect session handling, then split into login checkbox, chosen session behavior, and focused tests once the user-owned meaning is decided.
User question: Should remember-me extend the session on this device, prefill the email address, or both?
```

This clarification separates product, technical, security, QA, operational, and scope questions. It answers codebase-answerable questions from the repository and current Harness context, then asks the user only for decisions the codebase cannot answer.

Reference docs call this posture Discovery. It is clarification, not permission to write product files, not evidence, not verification, not QA, not final acceptance, not residual-risk acceptance, not close, and not a new authority path.

For the user-facing flow, use [User Guide](../use/user-guide.md#start-with-ordinary-requests) and [Agent Session Flow](../use/agent-session-flow.md). For exact owner behavior behind the terms, use [Kernel Reference](../reference/kernel.md) and [MCP API And Schemas](../reference/mcp-api-and-schemas.md).

## Scenario 4: User judgment blocks work

During the login work, the agent reaches a user-owned UX choice:

```text
Failed-login feedback can be an inline layer, a toast, or a modal.
```

This should not become a vague "approve?" prompt. Because this is a meaningful UX trade-off, the agent should show real options, recommendation, uncertainty, and deferral consequence:

```text
Decision needed: failed-login feedback pattern.
Why now: final UI behavior and tests need one failure-feedback pattern.
Options: inline message near the form, toast, or modal.
Recommendation: inline layer near the form; it is persistent and accessible.
Uncertainty: confirm existing design-system error-message support.
Deferral consequence: API and state wiring can continue, but final UI behavior and human QA should wait.
```

If the decision is blocking, Harness records that user-owned judgment through the documented decision path. Chat text, a broad "go ahead," or readable report prose alone should not satisfy the decision unless it answers the specific recorded choice. Permission for a sensitive step is separate from this product/UX choice.

For practical examples, read [Decision Packet Cookbook](../use/decision-packet-cookbook.md). For exact behavior, use [Decision Packet](../reference/kernel.md#decision-packet), [Decision Gate](../reference/kernel.md#decision-gate), [`harness.request_user_decision`](../reference/mcp-api-and-schemas.md#harnessrequest_user_decision), and [`harness.record_user_decision`](../reference/mcp-api-and-schemas.md#harnessrecord_user_decision).

## Scenario 5: Evidence and close blocker

The agent finishes a feature and says:

```text
The code is done and tests pass.
```

Harness may still block close if the close-relevant support is incomplete. That does not mean the work failed; it means the close basis is not complete yet.

Common examples:

- Evidence is partial because an acceptance criterion has no evidence link.
- Verification is required but no compatible independent check exists.
- Human QA is required for UI behavior and has not passed or been validly waived.
- Final acceptance is required but has not been requested with evidence, QA, verification, and residual-risk visibility.
- Known close-relevant residual risk exists but has not been made visible or accepted.

A useful close blocker names the smallest unblocker:

```text
Close blocked: human QA is still pending for the login error workflow.
Smallest unblocker: record the QA result, or ask for a QA waiver that names the skipped check and, if close-relevant risk remains, routes residual-risk acceptance separately.
```

Waivers and residual-risk-accepted close paths should stay explicit. A verification waiver does not create detached verification. A QA waiver does not prove the UI was inspected. Residual-risk acceptance does not make the risk disappear.

For exact close and gate behavior, use [`close_task`](../reference/kernel.md#close_task), [Evidence Gate](../reference/kernel.md#evidence-gate), [Verification Gate](../reference/kernel.md#verification-gate), [QA Gate](../reference/kernel.md#qa-gate), [Acceptance Gate](../reference/kernel.md#acceptance-gate), and [Residual Risk](../reference/kernel.md#residual-risk).

## Scenario 6: A readable report is not state

A Markdown status report says:

```text
Evidence: partial
Next action: record human QA
source_state_version: 42
```

That report is useful, but it is not the operating record. The implementation term is projection: a readable view rendered from current state records and related artifacts.

If a human edits the report to say:

```text
Evidence: sufficient
```

that edit does not change the saved evidence, gate state, human QA status, acceptance state, residual risk, or close eligibility. Human-editable sections can become notes or reconcile input, but accepted state changes still need the owner Core/MCP path.

The practical rule is simple: read projections for orientation, evidence links, related artifacts, and freshness; use owner records and owner actions for authority. If a projection is stale or wrong, refresh or reconcile it instead of treating the Markdown as state.

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

- Read [Harness in One Task](harness-in-one-task.md) for a fuller small-change and tracked-work task story.
- Read [Decision Packet Cookbook](../use/decision-packet-cookbook.md) when a user-owned judgment blocks progress.
- Read [User Guide](../use/user-guide.md) when you are running a real session.
- Use Reference docs only when you need exact contracts.
