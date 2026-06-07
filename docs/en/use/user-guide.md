# User Guide

Use normal language. Harness should help the agent keep scope, user-owned judgment, evidence, close readiness, and residual risk visible without making you learn internal presentation terms first.

This guide describes intended active user behavior for a future local Harness Server. This repository is still documentation-only; no Harness runtime/server implementation exists here yet. Exact state, API, schema, storage, and template contracts are owned by the [Reference Index](../reference/README.md).

## 1. Start in ordinary language

You should be able to ask for careful work in everyday terms:

```text
Help me make this plan concrete before implementation.
Ask me before deciding product behavior.
Before changing files, tell me what you expect to touch.
Show what you verified before calling it done.
Tell me what still blocks close.
```

You can set scope the same way:

```text
Add email login, but keep password reset and account creation out of scope.
Fix only typos in this document.
Keep this as a small copy edit unless it becomes a product or technical decision.
```

The agent should translate that into visible working context: the current goal, what is in scope, what is out of scope, what is already known, what only you can decide, what can safely happen next, and what would block close.

You do not need to say `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, or `task_events`. Those labels may appear only when they help explain a visible blocker, source reference, or owner contract.

## 2. Clarify scope before write

Before product files change, the agent should make the working scope plain:

- what you asked for and what the current goal is
- the smallest useful result
- what may change
- what is out of bounds
- likely paths, commands, tools, or external actions
- facts the agent already verified
- facts still unknown
- decisions that belong to you
- the next safe action

For anything larger than a tiny obvious edit, the agent should inspect available files, tests, docs, current Harness state, and accepted judgments before asking you. If the answer is discoverable, the agent should discover it. If the work is still too vague to implement safely, the agent should name the ambiguity and ask the one question that changes the next safe action.

In a Harness-connected session, product writes go through a pre-write scope check. In owner terms, the active path is `prepare_write`; a compatible response may create a single-use cooperative Write Authorization record for the intended operation. That record says the intended write matches current Harness state and active surface capability. It is not OS permission, sandboxing, tamper-proof enforcement, arbitrary-tool isolation, or proof that every tool was blocked before action.

## 3. Separate facts from user judgment

Facts are things the agent can inspect, verify, or cite. User judgments are choices you own.

| Item | Agent should do | It must not do |
|---|---|---|
| Repository facts | Read files, tests, docs, config, history, or current state when available. | Ask you to restate facts it can safely inspect. |
| Evidence | Show what supports a claim and what is missing. | Treat evidence as your decision or final acceptance. |
| Product judgment | Ask you about product behavior, copy, UX, user flow, and accessibility trade-offs. | Pick a material product direction silently. |
| Technical judgment | Ask you about architecture, dependency, migration, interface, security, privacy, retention, or compatibility choices that matter. | Hide a material technical decision as an implementation detail. |
| Scope judgment | Ask before expanding scope or removing a non-goal. | Treat enthusiasm as permission to broaden the task. |
| Close judgment | Ask for final acceptance or residual-risk acceptance only when the close basis is visible. | Turn "looks good" into every pending judgment. |

Harness preserves this boundary so the agent can recommend without replacing your judgment.

## 4. Ask one narrow judgment at a time

A useful judgment request asks for one decision and fits on one screen when possible. It should include:

- the exact decision
- realistic options
- a recommendation when helpful
- why the recommendation is reasonable
- uncertainty
- what can continue if you defer
- what remains blocked
- what the answer does not settle

Example:

```text
Judgment needed: choose failed-login feedback.

Options:
- Inline message near the form.
- Toast after failed submit.
- Modal that interrupts the flow.

Recommendation: inline message. It stays visible and fits the form context.

If deferred: backend error mapping can continue, but final UI behavior, copy, screenshots, and human QA remain blocked.

Does not settle: login architecture, account recovery, final acceptance, or residual-risk acceptance.
```

A broad reply such as "go ahead," "approved," or "looks good" applies only to the one active, clearly named judgment. It does not automatically grant sensitive-action approval, waive QA, accept verification risk, accept the final result, change scope, cancel the task, or accept residual risk.

The compact active path is a judgment request through the `user_judgment` owner path. A full-format Decision Packet is later candidate presentation material for complex judgments; it is not required for ordinary user decisions.

## 5. Treat sensitive action approval separately

Sensitive action approval is permission for a named action, not approval of the whole plan.

Examples include installing or updating dependencies, running commands with destructive potential, touching secrets, accessing restricted systems, deploying, sending network requests outside the current scope, or making a change with privacy/security impact.

A good sensitive-action prompt should name:

- the exact action
- the reason it is needed
- the scope and time window
- what command, tool, host, dependency, secret, or path is covered
- what safer alternative exists, if any
- what the approval does not settle

Approving a dependency install does not mean the dependency is the right architecture. Approving a deployment command does not mean the product result is accepted. Approving secret access does not accept residual risk. Each of those needs its own judgment when relevant.

## 6. Record evidence after meaningful action

After meaningful work, the agent should summarize what happened and what supports the claim. In owner terms, the active path may use `record_run` and evidence references when that path is available.

Useful evidence can include changed paths, diffs, command output, test results, screenshots, logs, source links, inspection notes, and human QA notes. The summary should say:

- what ran or changed
- what claim each item supports
- what passed, failed, was skipped, or was not applicable
- what evidence is missing, stale, redacted, omitted, blocked, or insufficient
- what was not verified

Evidence does not replace your judgment. Tests do not replace human QA when human inspection is required. A screenshot does not prove accessibility. A generated summary does not become operational truth. Raw secrets, tokens, and full sensitive logs should be redacted, omitted, blocked, or represented by safe handles.

## 7. Review blockers before close

Before larger work is called done, ask:

```text
Show what changed, what was verified, what residual risk is visible, and what still blocks close.
```

The agent should show:

- whether scope stayed in bounds
- user judgments already made
- unresolved user judgments
- changed paths or no-file result
- evidence supporting important completion claims
- checks and their status
- human QA expectation and result, when relevant
- final acceptance need
- residual risk visibility and acceptance need
- the smallest unblocker

Tests can pass while close is still blocked. A UI change can need human QA. A security-sensitive change can need a risk decision. Missing evidence remains a blocker until it is gathered, waived through an allowed path, or honestly reported as unresolved.

In owner terms, `close_task` returns blockers or a close result. In user terms, the agent should not claim close while required scope, evidence, verification, QA, user judgment, final acceptance, residual-risk handling, or close blockers remain unresolved.

## 8. Accept final result separately from residual risk

Final acceptance means you accept the result you can see. Residual-risk acceptance means you accept a named residual risk that is still visible. They are separate judgments.

The agent should ask for final acceptance only after the close basis is visible: scope, result, evidence, checks, known gaps, QA status, and blockers. The prompt should name exactly what result you are accepting.

The agent should ask for residual-risk acceptance only when a known residual risk is visible and the active close path requires that judgment. The prompt should name the risk, affected area, consequence, evidence gap or uncertainty, and any safer alternative.

"Looks good" may be final acceptance only when the agent has clearly asked for final acceptance of a named result. It is not residual-risk acceptance unless the risk was named and the prompt asked for that judgment.

## 9. Read current MVP guarantees honestly

Early Harness behavior is mostly cooperative and detective unless a specific owner-documented mechanism proves more.

| Guarantee level | What it means | What it does not mean |
|---|---|---|
| Cooperative | The agent is instructed to hold, ask, refresh, or proceed through the Harness record path. | Harness is not automatically stopping every tool at the OS level. |
| Detective | Harness or a surface can report a mismatch after observing state, output, or recorded action. | The action was not necessarily blocked before it happened. |
| Preventive | A specific proven mechanism blocks a covered action before it occurs. | Do not assume this unless the exact mechanism and operation are named. |
| Isolated | A documented separation boundary exists. | Do not assume broader OS sandboxing, arbitrary-tool isolation, or tamper-proof storage. |

Common status messages should be read plainly:

| Message | Meaning |
|---|---|
| Harness/Core authority is unavailable. | The agent cannot confirm current Harness state, judgments, evidence, write compatibility, final acceptance, residual-risk acceptance, or close readiness. |
| Current state may be stale. | The agent should refresh before relying on state, baseline, readable view, or pre-write scope check. |
| Out of scope. | The agent should narrow the action or ask whether you want to change scope. |
| User judgment needed. | A specific user-owned decision blocks the affected action. |
| Evidence insufficient. | The claim needs more support before it can be relied on. |
| Close blocked. | The work cannot honestly be closed yet. |

If an agent says something is blocked, read that as "we cannot honestly proceed or close under the current Harness record" unless it also names a proven preventive control.

## 10. Examples

### Clarify a feature

```text
User: Build a login feature.

Good agent behavior:
- inspect existing auth routes, session handling, login UI, tests, and docs first;
- separate scope, non-goals, facts, unknowns, and user-owned choices;
- ask one blocking product or technical judgment before writing;
- propose the smallest safe login slice.
```

### Keep a tiny edit tiny

```text
User: Fix only typos in this document.

Good agent behavior:
- treat the scope as typo fixes only;
- avoid wording, structure, terminology, and example changes;
- report the changed file and a diff review for unintended meaning changes.
```

### Split dependency choice from install approval

```text
User: Add a chart library.

Good agent behavior:
- ask the technical judgment about whether a new dependency is the right direction;
- ask a separate sensitive-action approval before installing or updating packages;
- record evidence after the install and implementation checks.
```

### Close honestly

```text
User: Can we call this done?

Good agent behavior:
- show scope, evidence, checks, QA status, final acceptance need, and residual risk;
- name blockers before close;
- ask final acceptance and residual-risk acceptance separately when both are relevant.
```

For compact judgment prompt patterns, see [Judgment Examples](judgment-examples.md).
