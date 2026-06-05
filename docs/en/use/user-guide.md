# User Guide

## 1. What Harness helps with

Harness helps an AI-assisted work session keep track of the parts that should not disappear into chat: what the work is, what is in scope, what only you can decide, what evidence supports a claim, what was checked, and whether the work can honestly be closed.

It is meant to let you speak normally. You should not have to learn internal Harness labels before asking for help.

Status note: this guide describes the intended Harness-assisted user flow for a future local Harness Server. This repository is documentation-only today. It is not the user's Product Repository or a Harness Runtime Home, and no Harness runtime/server implementation exists here yet.

Harness is useful when you want the agent to:

- clarify blurry work before implementation
- keep small work small
- separate facts it can inspect from judgments only you can make
- show when scope is growing
- tie completion claims to evidence and checks
- keep product/UX judgment separate from technical judgment
- show what still blocks closing the work

## 2. Start in normal language

<a id="first-read-path"></a>
<a id="phrase-reference"></a>

Say what you want in ordinary words. These are enough:

```text
Help me clarify this before implementation.
Separate what I need to decide from what you can check.
Tell me if the scope is expanding.
Fix only typos in this document.
Before building login, show the product and technical decisions.
```

You can also add boundaries:

```text
Add email login, but keep password reset and account creation out of scope for now.
```

Or ask for a light touch:

```text
Keep this as a small copy change unless it turns into a product or technical decision.
```

The agent should translate your request into plain working facts: goal, scope, non-goals, facts it can inspect, choices you own, evidence needed, checks to run, and what would still block close.

## 3. What the agent should clarify first

<a id="what-the-agent-should-answer-first"></a>

For anything beyond an obvious tiny edit, the agent should first answer:

- What result are we trying to get?
- What is known to be in scope?
- What is known to be out of scope?
- What can the agent inspect before asking you?
- What decisions belong to you?
- What question, if any, blocks the next safe action?
- What can safely continue while a non-blocking question waits?

A useful first response is short and specific:

```text
Scope I heard: add email login only.
Out of scope for now: password reset, account creation, social login, and a full authentication redesign.

I can inspect: current login routes, session handling, auth tests, login form patterns, and validation helpers.

You may need to decide: password credentials, magic link, one-time code, or external identity provider; failed-login UX; security and privacy trade-offs.

Next safe action: inspect the current auth shape, then return with facts, open decisions, and the smallest safe implementation slice.
```

If the agent can answer something by reading the repo, docs, tests, prior saved context, or current files, it should inspect first. It should not turn agent-resolvable uncertainty into a user questionnaire.

## 4. How small work is handled

Small work should stay light. If you say:

```text
Fix only typos in this document.
```

the agent should keep the visible flow compact:

```text
Scope: typos in this document only.
Out of scope: wording changes, structure changes, terminology changes, and new examples.
I will edit only typo-level issues and tell you if I find anything broader.
```

After the work, a compact result is enough:

```text
Done.
Changed: typo fixes only.
Checked: diff review for unintended wording changes.
No scope expansion found.
```

Small work should stop being treated as small when it changes meaning, behavior, UX, public API, security/privacy posture, localization strategy, architecture, or QA expectations. When that happens, the agent should pause, show the scope change, and ask for the smallest necessary decision.

## 5. How larger work is clarified

Larger work needs more structure because product behavior, implementation choices, evidence, QA, and remaining risk can all matter.

If you say:

```text
Before building login, show the product and technical decisions.
```

the agent should separate planning from implementation:

```text
Product decisions likely needed:
- Which login experience: password, magic link, one-time code, or external identity provider.
- What failed-login feedback should look like.
- What recovery path and copy should users see.

Technical decisions likely needed:
- Session or token strategy.
- Password handling or identity-provider path.
- Rate limiting, account-enumeration risk, logging, redaction, and secret handling.
- Tests and manual QA expectations.

I will inspect the current auth code, UI patterns, tests, and docs before recommending an implementation path.
```

Clarification can take more than one turn. That is fine when each turn does one of these things: inspects available facts, names a blocking question, parks useful non-blocking questions, narrows the work, or proposes a safe split.

## 6. When the user must decide

<a id="judgment"></a>

The agent can choose routine implementation details inside an agreed scope: following local naming, reusing an existing helper, adding a focused test, or taking the conservative local pattern.

The agent should ask you when a choice affects:

- product behavior, UX, copy, user flow, or accessibility trade-offs
- public API, module contracts, or compatibility
- architecture, dependencies, migrations, or data model direction
- security, privacy, audit, retention, redaction, or secrets
- scope expansion
- permission for a named sensitive step
- QA or verification expectations, including waivers
- accepting the finished result
- accepting a known remaining risk

The question should name the specific decision. It should not ask for broad approval when several different decisions are pending.

```text
Decision needed: choose the failed-login feedback pattern.
Options: inline message, toast, or modal.
Recommendation: inline message near the form because it stays visible and is easier to make accessible.
Can continue if deferred: backend validation work that does not claim final UI behavior.
Cannot close yet: final UX, copy, and human QA for the login screen.
```

## 7. Product/UX judgment examples

Product and UX judgments decide what the user experience should be. The agent can recommend, but it should not silently choose for you.

Examples:

| Situation | What you decide | What the agent can check |
|---|---|---|
| Login failure | Inline message, toast, modal, or another pattern. | Existing error UI, accessibility patterns, design-system components, and current copy. |
| Onboarding | Checklist, setup prompt, guided flow, or empty-state education. | Current screens, analytics notes if available, docs, and prior patterns. |
| Copy tone | Plain warning, softer guidance, or more specific error text. | Existing product voice, support docs, localization constraints, and account-enumeration risk. |
| Workflow friction | Require confirmation, allow undo, or proceed directly. | Existing destructive-action patterns, user roles, and current UI conventions. |

A good prompt shows options, recommendation, uncertainty, what can continue if you defer, and what cannot be honestly finished yet.

## 8. Technical judgment examples

Technical judgments decide material implementation direction. The agent should inspect first, then show the trade-off plainly.

Examples:

| Situation | What you decide | What the agent can check |
|---|---|---|
| Login architecture | Session cookie, bearer token, magic link, OAuth/OIDC, or provider integration. | Existing auth model, dependencies, tests, security notes, and deployment constraints. |
| Dependency choice | Add a package, avoid it, or replace an existing one. | Current dependency policy, licenses, maintenance status, bundle impact, and local alternatives. |
| Migration path | In-place change, staged migration, compatibility shim, or follow-up task. | Existing data shape, callers, tests, release constraints, and rollback options. |
| API contract | Preserve current contract, add a versioned path, or change callers together. | Existing callers, docs, tests, compatibility risks, and public surface area. |
| Verification expectation | Focused test, broader regression run, independent review, manual QA, or waiver. | Available tests, past failures, affected surfaces, and what remains untested. |

Technical judgment is not the same as permission for a sensitive step. For example, allowing the agent to install an auth helper package is not the same as deciding that package is the architecture direction.

## 9. How to read evidence and checks

<a id="the-four-display-groups"></a>

Evidence is support for a claim. Checks are actions that examine whether the claim is true.

Useful evidence can include changed paths, diffs, command output, test results, screenshots, logs, source links, inspection notes, or human QA notes. Useful checks can include focused tests, diff review, source lookup, browser inspection, accessibility checks, or an independent enough review when the work requires it.

Read evidence and checks separately:

| Plain item | What it means | What it does not replace |
|---|---|---|
| Evidence | Support for a completion or correctness claim. | Your decision, work acceptance, or remaining-risk acceptance. |
| Automated check | A test, command, or mechanical review of a specific behavior. | Human QA, broad confidence, or product judgment. |
| Human QA | A person inspected an experience where judgment matters. | Automated tests or screenshots alone. |
| Source lookup | The agent checked docs, code, or current files before claiming something. | A decision you own. |
| Missing evidence | The claim is not yet well supported. | A reason to invent confidence. |

If the agent says "done," it should also be able to say what changed, what supports that claim, what was checked, and what was not checked.

## 10. What to confirm before closing work

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

Tests can pass while close is still blocked. A UI change may still need human QA. A security-sensitive change may still need a risk decision. A broad "looks good" should only count when the agent has clearly named the thing you are accepting.

Common status messages should be direct and non-alarming. Exact condition behavior is owned by [API Errors: MVP-1 guarantee and status taxonomy](../reference/api/errors.md#mvp-1-guarantee-and-status-taxonomy), but as a user you can read them this way:

| Message | What it means for you |
|---|---|
| Harness/Core authority is unavailable. | The agent cannot confirm current Harness state, evidence, work acceptance, residual-risk acceptance, or close readiness. It can reconnect, diagnose, or continue outside Harness only if you explicitly choose that mode. |
| Local access is unavailable or denied. | The current surface cannot inspect or change the requested local files or system path. |
| Current state or view may be stale. | The agent should refresh before relying on the status, baseline, projection, or pre-write scope check. |
| This is outside the current Harness stage or surface. | The requested behavior is not available in the current stage/profile; the agent should offer a supported fallback. |
| This is outside the current scope. | The agent should narrow the action or ask whether you want to change the scope. |
| User judgment is needed. | A decision belongs to you before the affected action can continue. |
| Evidence is insufficient. | The claim needs more support before the agent can honestly rely on it. |
| Close is blocked. | The work cannot be closed yet; the agent should show the smallest unblocker. |
| Residual risk remains. | A known remaining risk must be visible, and some close paths require you to accept that risk separately. |

## 11. Everyday scenario examples

These examples are written as ordinary user requests. They are examples of intended future Harness behavior, not runtime records from this documentation repository.

### "Build a login feature."

User request:

```text
Build a login feature.
```

Good agent response:

```text
This is larger than a direct edit, so I need to clarify before implementation. I will inspect the existing auth routes, session handling, login UI, tests, and docs first.

Likely decisions for you:
- product flow: email/password, magic link, OAuth/OIDC, social login, or another login path;
- technical direction: server-side session cookie, JWT/bearer token, external identity provider, or a hybrid;
- scope: account creation, password reset, rate limiting, audit logging, and recovery copy.

Next safe action: inspect the current auth shape and come back with facts, a recommended narrow slice, and the decisions that still block writing.
```

Why this is good: it does not turn an ambiguous feature request into silent architecture and UX choices. It lets the agent inspect facts first and keeps session-cookie, JWT, OAuth/OIDC, and social-login decisions visible.

Bad agent response:

```text
I will build the full login system with JWTs, social login, reset, signup, and modals, then mark it done when tests pass.
```

Why this is bad: it expands scope, chooses architecture and product behavior for the user, and treats tests as enough for completion.

### "Show a message after clicking Save."

User request:

```text
Show a message after clicking Save.
```

Good agent response:

```text
I will check the existing save feedback patterns first. The product choice is where the message should appear: inline near the form, as a toast, or as a modal.

Recommendation: use a toast for a non-blocking success message, and use inline text for form-specific errors. I can wire the save state now, but final UI behavior and QA depend on that choice.
```

Why this is good: it treats inline, toast, and modal as a user-visible UX judgment instead of hiding the decision in implementation.

Bad agent response:

```text
I will add a modal after every Save click because it is the most noticeable.
```

Why this is bad: it picks a disruptive UX without checking local patterns or asking whether the message should block the flow.

### "Fix this bug."

User request:

```text
Fix this bug.
```

Good agent response:

```text
I need the failing behavior or a repro before I can safely fix it. I will first inspect the error, nearby tests, recent changes, and affected files. If the bug is obvious from those sources, I will make the smallest fix and show the evidence. If the symptom is still ambiguous, I will ask for the missing repro detail.
```

Why this is good: it asks for clarification only after checking what the agent can discover, and it keeps the fix narrow until the bug is understood.

Bad agent response:

```text
I will refactor the whole module and call it fixed once the test suite passes.
```

Why this is bad: it turns an ambiguous bug into broad work and may hide missing evidence about the actual failure.

### "Only fix typos in this document."

User request:

```text
Only fix typos in this document.
```

Good agent response:

```text
Scope: typo fixes in this document only.
Out of scope: wording changes, structure changes, terminology changes, and new examples.
I will make only typo-level edits and do a quick diff review for unintended meaning changes.
```

Why this is good: it uses a low procedure budget and keeps small direct work small.

Bad agent response:

```text
I will rewrite the section, add examples, update terminology, create a full evidence report, and ask for acceptance.
```

Why this is bad: it ignores the user's narrow scope and adds ceremony the small edit does not need.

### "Looks good, go ahead."

User request:

```text
Looks good, go ahead.
```

Good agent response:

```text
I will apply that to the one pending choice: use the toast for the Save success message. This does not grant sensitive-action approval, accept the finished work, waive QA, or accept any residual risk.
```

Why this is good: it maps broad language only to the active, clearly named judgment.

Bad agent response:

```text
Invalid: I would treat that as all write permission, work acceptance, QA waiver, and residual-risk acceptance.
```

Why this is bad: "go ahead" is not automatically every approval and acceptance path.

### "I will test it later."

User request:

```text
I will test it later.
```

Good agent response:

```text
I will record this as Manual QA deferred, not passed. The implementation can remain available for your later test, but close is still blocked unless the active path allows a QA waiver and any remaining risk is shown and accepted separately.
```

Why this is good: it keeps evidence gaps and residual risk visible instead of converting future testing into proof.

Bad agent response:

```text
Sounds good. I will mark QA complete and close the task.
```

Why this is bad: future testing is not evidence that testing passed, and it does not accept remaining risk.

### "Clean up all these files."

User request:

```text
Clean up all these files.
```

Good agent response:

```text
"Clean up" could mean formatting, dead-code removal, renaming, deleting files, or changing structure. I will inspect the files first and propose a narrow cleanup slice. Before deleting files, changing public interfaces, or broadening scope, I will ask for the specific decision.
```

Why this is good: it clarifies a large ambiguous request before broad or destructive work.

Bad agent response:

```text
I will delete anything that looks unused, rename files as I see fit, and reorganize the folder.
```

Why this is bad: it treats vague cleanup as permission for destructive and architectural changes.

### "Can I put the API key in the code?"

User request:

```text
Can I put the API key in the code?
```

Good agent response:

```text
No. Do not put a raw API key in source code. Use the project's existing secret path if one exists, such as environment variables, a local secret file excluded from version control, or a secret manager. I will inspect the repo's config and docs before recommending the exact path.

Security note: Harness can make this security/privacy judgment visible and preserve the decision path, but early local Harness wording should not claim OS-level secret isolation or tamper-proof storage unless that exact mechanism is present.
```

Why this is good: it gives a clear security/privacy recommendation, checks local practice, and does not overclaim enforcement.

Bad agent response:

```text
Yes. Hard-code it for now and Harness will keep it safe.
```

Why this is bad: it creates a secret leak risk and overclaims what Harness guarantees.

## 12. What Harness does not guarantee

Harness makes AI-assisted work easier to inspect and route. It does not replace the surrounding engineering process.

It does not:

- replace source control, tests, review, product specs, or team process
- prove a product decision is correct
- turn tool output into user judgment
- turn test pass into human QA
- turn permission for a sensitive step into work acceptance
- treat generated readable summaries as operating state
- automatically change OS permissions
- sandbox arbitrary tools
- make local files tamper-proof
- provide pre-execution blocking unless a specific proven blocking control is named
- provide security isolation unless the exact separation boundary is named and proven

If the agent says something is blocked, read that as "we cannot honestly proceed or close under the current record" unless it also names an actual preventive control. Early local Harness wording should be cooperative or detective unless a stronger mechanism is explicitly documented.

## 13. Advanced internal terms, only after the main flow

You can skip this section until an agent or reference page shows one of these labels. They are useful for precision, but they should not be the first way a normal task is explained.

| Internal label | Plain meaning |
|---|---|
| Discovery | The internal name for requirements clarification before implementation planning. Users can simply ask for clarification in ordinary language. |
| Change Unit | The bounded work area that may change for a product-write attempt. It is scope, not broad permission. |
| Decision Packet | A fuller presentation for a specific user-owned judgment. It should not be required for every small choice. |
| Write Authorization | A cooperative internal record/check that a specific write attempt fits the current scope and recorded permissions. It is not OS permission, sandboxing, tamper-proof enforcement, or generic approval. |
| Evidence Manifest | A fuller record that maps completion claims or criteria to evidence references when that profile is active. Small work may only need a short evidence summary. |
| Projection | A readable summary derived from saved records. It helps orientation, but it is not the operating record itself. |
| Gate | An internal readiness or compatibility condition. User-facing status should show the blocker or check first. |
| User Judgment | The saved record behind a named user decision, such as product/UX judgment, technical judgment, sensitive-step permission, work acceptance, or remaining-risk acceptance. |
| Approval | Permission for a named sensitive action. It is not product judgment, work acceptance, or remaining-risk acceptance. |
| Manual QA | Human inspection for UX, copy, accessibility, visual quality, workflow, or other human-judgment surfaces. |
| Acceptance | The user's work-acceptance judgment when the work path requires it. |
| Residual Risk | Known remaining uncertainty, limitation, trade-off, or consequence. |
| `task_events` | Low-level event history for implementers and diagnostics. |

These labels do not collapse into each other. A decision is not a write check. Permission for a sensitive action is not work acceptance. Work acceptance does not erase remaining risk. A readable summary is not state. Passing tests does not mean human QA happened.

For exact contracts, use the Reference docs when needed: [Core Model Reference](../reference/core-model.md), [MVP API](../reference/api/mvp-api.md), [Agent Integration Reference](../reference/agent-integration.md), and [Projection And Templates Reference](../reference/projection-and-templates.md).
