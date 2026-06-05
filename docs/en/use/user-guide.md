# User Guide

## 1. Start with ordinary requests

<a id="first-read-path"></a>
<a id="phrase-reference"></a>

Harness is meant to let you ask for help in normal language. You should not have to say "Discovery," "Change Unit," "Decision Packet," or any other internal label to get careful behavior.

Useful requests can be as simple as:

```text
Make this plan concrete enough to implement.
Turn this feature idea into an implementable plan.
Ask me first about the parts only I can decide.
Before changing files, confirm which files you expect to touch.
Before you say it is done, show the evidence and remaining risks.
```

You can also set a boundary:

```text
Add email login, but keep password reset and account creation out of scope for now.
```

Or keep the work intentionally small:

```text
Fix only typos in this document.
Keep this as a small copy change unless it turns into a product or technical decision.
```

Status note: this guide describes the intended Harness-assisted user flow for a future local Harness Server. This repository is documentation-only today. It is not the user's Product Repository or a Harness Runtime Home, and no Harness runtime/server implementation exists here yet.

This Use guide stays at the visible user and agent behavior layer. Exact enum values, DDL, full state transitions, API schemas, and projection/template bodies are owned by the [Reference Index](../reference/README.md).

## 2. What Harness does with a normal request

Harness turns ordinary requests into a visible working basis. You do not need to name these parts, but the agent should preserve them.

| What you ask for | What Harness should route internally | What you should see |
|---|---|---|
| "Make this plan concrete enough to implement." | Intake and requirement shaping, stored through the active Task, proposed or active Change Unit, and user-decision boundaries. | Original request, current goal, success criteria, confirmed facts, likely scope, non-goals, remaining uncertainties, user-owned decisions, and the next safe implementation unit. |
| "Ask me before deciding the UX." | Focused user judgment through the user-owned judgment path. | One specific question with choices, recommendation, uncertainty, and consequence if deferred. |
| "Keep reset out of scope." | Active scope, internally represented by the bounded work area or Change Unit when a product write is involved. | What may change, what is out of bounds, and when scope expansion needs a decision. |
| "Before changing files, tell me what you will touch." | A pre-write scope check. In owner terms, product writes go through `prepare_write`; an allowed response may create one single-use cooperative Write Authorization record when Harness is connected. | Intended paths or operation, scope match or mismatch, pending decisions, stale or unavailable authority, and the smallest unblocker. |
| "Show what happened and what proves it." | A run/evidence recording path. In owner terms, meaningful execution is summarized through `record_run` and evidence refs when that path is active. | What ran or changed, what supports the claim, what is missing, and what was not checked. |
| "Can we call this done?" | Close readiness. In owner terms, `close_task` returns close blockers or a close result. | Whether close is available, why it is blocked or available, what risk remains, and the smallest unblocker. |

Readable summaries help you understand the work. They are not the operating record themselves. Editing a status summary, a generated report, or chat text does not create a user decision, pre-write scope check, Write Authorization, evidence, final acceptance, residual-risk acceptance, or close readiness. When Harness says a write is allowed or blocked, read that as a compatibility result against current Harness state and active surface capability, not as a physical operating-system permission result.

## 3. What the agent should clarify first

<a id="what-the-agent-should-answer-first"></a>

For anything beyond an obvious tiny edit, the agent should first answer:

- What did you originally ask for, and what is the current goal?
- What result are we trying to get?
- What success criteria would make the result implementable and checkable?
- What is in scope?
- What is out of scope?
- What can the agent inspect before asking you?
- What facts are confirmed, and what uncertainty remains?
- What decisions belong to you?
- What question blocks the next safe action or user-owned judgment, if any?
- What can safely continue while non-blocking questions wait?
- What is the next safe implementation unit, or the next Change Unit when product writes are involved?

A useful first response is short:

```text
Scope I heard: add email login only.
Out of scope for now: password reset, account creation, social login, and a full authentication redesign.

Success criteria to shape: a user can sign in with the chosen email-login flow, failure feedback follows the chosen UI pattern, and the slice does not add reset or signup.

I can inspect: current login routes, session handling, auth tests, login form patterns, and validation helpers.

Likely decisions for you: password credentials, magic link, one-time code, or external identity provider; failed-login feedback; security and privacy trade-offs.

Next safe action: inspect the current auth shape, then return with confirmed facts, open decisions, and the smallest safe implementation slice.
```

If the answer is available in files, tests, docs, saved Harness context, or current project state, the agent should inspect first. It should not turn agent-resolvable uncertainty into a questionnaire.

The agent should ask only questions that block the next safe action or a user-owned judgment. If the requirement is too vague to implement safely, it should challenge that strongly and name the ambiguity instead of pretending the work is ready. Once enough information exists, shaping should end in a safe implementation unit, not another planning loop.

For MVP-1, this clarification does not create a separate Shared Design record, Discovery Brief record, Question Queue record, Assumption Register record, or First Safe Change Unit Candidate record. The result should become an updated Task summary, a proposed or active Change Unit when product writes are near, and focused user-decision prompts when the choice belongs to you.

## 4. Keep small work small

Small work should stay light. If you say:

```text
Fix only typos in this document.
```

the agent should keep the flow compact:

```text
Scope: typos in this document only.
Out of scope: wording changes, structure changes, terminology changes, and new examples.
I will edit only typo-level issues and tell you if I find anything broader.
```

Afterward, a compact result is enough:

```text
Done.
Changed: typo fixes only.
Checked: diff review for unintended wording changes.
No scope expansion found.
```

Small work stops being small when it changes meaning, behavior, UX, public API, security/privacy posture, localization strategy, architecture, QA expectations, or any decision you own. Then the agent should pause, show the scope change, and ask the smallest necessary question.

## 5. User decisions are specific

<a id="judgment"></a>

The agent can choose routine implementation details inside an agreed scope, such as following local naming, reusing an existing helper, adding a focused test, or taking the conservative local pattern.

The agent should ask you when a choice affects:

- product behavior, UX, copy, user flow, or accessibility trade-offs
- public API, module contracts, or compatibility
- architecture, dependencies, migrations, data model, security, privacy, audit, retention, redaction, or secrets
- scope expansion or non-goal removal
- permission for a named sensitive step
- QA waiver or verification-risk acceptance
- final acceptance of the finished result
- acceptance of a known remaining risk
- cancellation

The question should be focused. It should name the decision, choices, recommendation, rationale, uncertainty, affected scope, what happens if you defer, and what the agent is not deciding for you.

```text
Decision needed: choose the failed-login feedback pattern.
Options: inline message, toast, or modal.
Recommendation: inline message near the form because it stays visible and is easier to make accessible.
Can continue if deferred: backend validation work that does not claim final UI behavior.
Cannot close yet: final UX, copy, and human QA for the login screen.
```

A broad "yes, do it," "looks good," or "go ahead" applies only to the one active, clearly named decision. It does not automatically grant sensitive-action approval, accept the finished work, waive QA, accept verification risk, change scope, cancel the task, or accept residual risk.

## 6. Before files change

Before a product write in a Harness-connected session, the agent should confirm that the intended write still fits the current scope, state, and active surface capability. Internally, that is the `prepare_write` / Write Authorization path.

You should see:

- intended paths or operation summary
- why the write fits, or why it does not
- pending product, technical, scope, sensitive-action, QA, verification-risk, final-acceptance, or residual-risk decisions
- stale state, stale baseline, or unavailable Core/Harness authority
- current guarantee level, or a clear unavailable/capability condition
- the smallest action that would unblock the write

An allowed result means the intended write is compatible with the current Harness state and active surface capability. A blocked result means the Harness protocol, state, or capability does not allow that claim to proceed. This check is not OS permission, sandboxing, tamper-proof enforcement, arbitrary-tool isolation, or proof that Harness can prevent every tool from acting. In owner terms, the stored boundary is `AuthorizedAttemptScope`: operation, paths, tools, commands and command classes, product-file-write intent, network targets, secret scope, sensitive categories, baseline, Task, Change Unit, state, surface, related judgments, and guarantee level. A Write Authorization is a single-use cooperative record for that stored boundary. If any part changes or cannot be observed on the active surface, the check should be refreshed or treated as unverified/blocked before writing.

If Core or Harness authority cannot answer, the agent should say that. It should not claim current write compatibility or a Write Authorization from old chat, cached summaries, stale projections, or user enthusiasm.

## 7. After execution, read evidence and checks separately

<a id="the-four-display-groups"></a>

Evidence is support for a claim. Checks are actions that examine whether the claim is true.

Useful evidence can include changed paths, diffs, command output, test results, screenshots, logs, source links, inspection notes, or human QA notes. Useful checks can include focused tests, diff review, source lookup, browser inspection, accessibility checks, or independent enough review when the work requires it.

After meaningful work, the agent should summarize the execution and evidence. Internally, that means the future `record_run` path or a run/evidence summary when that path is active.

Read the summary this way:

| Plain item | What it means | What it does not replace |
|---|---|---|
| Evidence | Support for a completion or correctness claim. | Your decision, final acceptance, or remaining-risk acceptance. |
| Automated check | A test, command, or mechanical review of a specific behavior. | Human QA, broad confidence, or product decision. |
| Human QA | A person inspected an experience where judgment matters. | Automated tests or screenshots alone. |
| Source lookup | The agent checked docs, code, or current files before claiming something. | A decision you own. |
| Missing evidence | The claim is not yet well supported. | A reason to invent confidence. |

When evidence uses artifacts, expect refs plus integrity and redaction facts, not pasted large or sensitive bodies. Raw secrets, tokens, and full sensitive logs should be redacted, omitted, blocked, or represented by safe handles instead of stored or pasted.

If the agent says "done," it should also be able to say what changed, what supports that claim, what was checked, and what was not checked.

## 8. Before completion, ask what still blocks close

Before closing larger work, ask:

```text
Show what changed, what was checked, what risks remain, and what still blocks close.
```

The agent should show:

- scope that actually stayed in bounds
- decisions you made and decisions still unresolved
- changed paths or no-file result
- evidence supporting each important completion claim
- checks that passed, failed, were skipped, or were not applicable
- human QA expectations and results, when the work needs them
- whether you need to accept the finished result
- known remaining risks and whether they were accepted
- the smallest action that would unblock close

Tests can pass while close is still blocked. A UI change may still need human QA. A security-sensitive change may still need a risk decision. A broad "looks good" counts only when the agent has clearly named what you are accepting.

In owner terms, `close_task` should return blockers or a close result. In user terms, the agent should not claim completion until evidence, required checks, user decisions, final acceptance, residual-risk visibility, and close blockers have been handled or honestly reported.

## 9. Understand the MVP guarantee limits

Status cards and pre-write checks should show the active guarantee level, or a clear unavailable/capability condition when Core or required MCP cannot answer.

For the early MVP path:

| Guarantee level | What it means | What it does not mean |
|---|---|---|
| Cooperative | The agent is instructed to hold, ask, refresh, or proceed through the Harness record path. | Harness is not automatically stopping every tool at the OS level. |
| Detective | Harness or a surface can report a mismatch after observing state, output, or a recorded action. | The action was not necessarily blocked before it happened. |
| Preventive | A specific proven mechanism blocks a covered action before it occurs. | Do not assume this unless the exact mechanism and covered operation are named. |
| Isolated | A documented separation boundary exists. | Do not assume broader OS sandboxing, arbitrary-tool isolation, or tamper-proof storage. |

If the agent says an action is blocked, read that as "we cannot honestly proceed or close under the current Harness record" unless it also names an actual preventive control.

Common messages should be plain:

| Message | What it means for you |
|---|---|
| Harness/Core authority is unavailable. | The agent cannot confirm current Harness state, evidence, final acceptance, residual-risk acceptance, or close readiness. It can reconnect, diagnose, or continue outside Harness only if you explicitly choose that mode. |
| Local access is unavailable or denied. | The current surface cannot inspect or change the requested local files or system path. |
| Current state or view may be stale. | The agent should refresh before relying on the status, baseline, readable view, or pre-write scope check. |
| This is outside the current scope. | The agent should narrow the action or ask whether you want to change the scope. |
| User judgment is needed. | A specific decision belongs to you before the affected action can continue. |
| Evidence is insufficient. | The claim needs more support before the agent can honestly rely on it. |
| Close is blocked. | The work cannot be closed yet; the agent should show the smallest unblocker. |
| Residual risk remains. | A known remaining risk must be visible, and some close paths require you to accept that risk separately. |

## 10. Everyday scenario examples

These examples are ordinary-language examples of intended future Harness behavior. They are not runtime records, generated evidence, acceptance records, or conformance outputs from this documentation repository.

### "Build a login feature."

Good response:

```text
This is larger than a direct edit, so I need to clarify before implementation. I will inspect the existing auth routes, session handling, login UI, tests, and docs first.

Likely decisions for you:
- product flow: email/password, magic link, OAuth/OIDC, social login, or another login path;
- technical direction: server-side session cookie, JWT/bearer token, external identity provider, or a hybrid;
- scope: account creation, password reset, rate limiting, audit logging, recovery copy, and Manual QA expectations.

Next safe action: inspect the current auth shape and come back with facts, a recommended narrow slice, and the decisions that still block writing.
```

Bad response:

```text
I will build the full login system with JWTs, social login, reset, signup, and modals, then mark it done when tests pass.
```

### "Show a message after clicking Save."

Good response:

```text
I will check the existing save feedback patterns first. The product choice is where the message should appear: inline near the form, as a toast, or as a modal.

Recommendation: use a toast for a non-blocking success message, and use inline text for form-specific errors. I can wire the save state now, but final UI behavior and QA depend on that choice.
```

Bad response:

```text
I will add a modal after every Save click because it is the most noticeable.
```

### "Fix this bug."

Good response:

```text
I will inspect the failing behavior, nearby tests, recent changes, and affected files first. If the bug is clear from those sources, I will make the smallest fix and show the evidence. If the symptom is still ambiguous, I will ask for the missing repro detail.
```

Bad response:

```text
I will refactor the whole module and call it fixed once the test suite passes.
```

### "Looks good, go ahead."

Good response:

```text
I will apply that to the one pending choice: use the toast for the Save success message. This does not grant sensitive approval, accept the finished work, waive QA, accept verification risk, change scope, cancel the task, or accept any residual risk.
```

Bad response:

```text
Great. I will treat that as write permission, final acceptance, QA waiver, verification-risk acceptance, scope change, and residual-risk acceptance.
```

### "I will test it later."

Good response:

```text
I will show Manual QA as deferred, not passed. The implementation can remain available for your later test, but close is still blocked unless the active path allows a QA waiver and any remaining risk is shown and accepted separately.
```

Bad response:

```text
Sounds good. I will mark QA complete and close the task.
```

### "Can I put the API key in the code?"

Good response:

```text
No. Do not put a raw API key in source code. I will inspect the repo's config and docs before recommending the exact secret path, such as an environment variable, an ignored local secret file, a platform secret, or a secret manager.

Security note: Harness can make this security/privacy judgment visible and preserve the decision path, but early local Harness wording should not claim OS-level secret isolation or tamper-proof storage unless that exact mechanism is present.
```

Bad response:

```text
Yes. Hard-code it for now and Harness will keep it safe.
```

## 11. Advanced internal terms, only after the main flow

You can skip this section until an agent or Reference page shows one of these labels. They are useful for precision, but they should not be the first way a normal task is explained.

| Internal label | Plain meaning |
|---|---|
| Discovery | The internal name for requirements clarification before implementation planning. Users can simply ask for clarification in ordinary language. In MVP-1, its output is stored through Task, Change Unit, and User Judgment paths, not a committed Shared Design record. |
| Shared Design | A later/profile design-support record or projection label. In active MVP-1, requirements shaping uses Task, Change Unit, and User Judgment instead. |
| Change Unit | The bounded work area that may change for a product-write attempt. It is scope, not broad permission. |
| Decision Packet | A fuller presentation for a specific user-owned judgment. It should not be required for every small choice. |
| Write Authorization | A single-use cooperative internal record/check for one stored `AuthorizedAttemptScope`, the boundary Core compares during `record_run`. It is not OS permission, sandboxing, tamper-proof enforcement, isolation, a permission token, or generic approval. |
| Evidence Manifest | A fuller record that maps completion claims or criteria to evidence references when that profile is active. Small work may only need a short evidence summary. |
| Projection | A readable summary derived from saved records. It helps orientation, but it is not the operating record itself. |
| Gate | An internal readiness or compatibility condition. User-facing status should show the blocker or check first. |
| User Judgment | The saved record behind a named user decision, such as product decision, technical decision, scope decision, sensitive approval, QA waiver, verification-risk acceptance, final acceptance, residual-risk acceptance, or cancellation. |
| Approval | Permission for a named sensitive action. It is not product decision, technical decision, scope decision, final acceptance, QA waiver, verification-risk acceptance, or residual-risk acceptance. |
| Manual QA | Human inspection for UX, copy, accessibility, visual quality, workflow, or other human-judgment surfaces. |
| Acceptance | The user's final acceptance judgment when the work path requires it. |
| Residual Risk | Known remaining uncertainty, limitation, trade-off, or consequence. |
| `task_events` | Low-level event history for implementers and diagnostics. |

These labels do not collapse into each other. A decision is not a write check. Permission for a sensitive action is not final acceptance. Final acceptance does not erase remaining risk. A QA waiver is not QA evidence. A readable summary is not state. Passing tests does not mean human QA happened.

For exact contracts, use the Reference docs when needed: [Core Model Reference](../reference/core-model.md), [MVP API](../reference/api/mvp-api.md), [Agent Integration Reference](../reference/agent-integration.md), and [Projection And Templates Reference](../reference/projection-and-templates.md).
