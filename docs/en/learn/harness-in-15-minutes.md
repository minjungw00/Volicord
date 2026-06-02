# Harness in 15 Minutes

## Start With Ordinary Requests

Harness should not require users to speak in internal labels. These requests are enough:

```text
Before implementing, help me make the plan concrete.
Separate the product decisions from the technical decisions.
Keep this as a small change and tell me if the scope grows.
Show me what still prevents closing this work.
```

Harness keeps AI-assisted product work grounded in local Core-owned state by tracking scope, user-owned judgments, evidence references, close readiness, acceptance, and residual risk outside the chat.

This page shows the product feel in short scenarios. It is not an API guide and not a claim that this documentation repository already contains a running Harness Server.

## The Tour In One Idea

Every scenario below is about one question: which surface is allowed to carry authority?

Chat can coordinate. Tool output can support claims. Markdown can summarize. Product files can contain the work result. Core-owned state is where Harness keeps scope, user-owned judgments, evidence references, close readiness, acceptance, and residual risk.

## Scenario 1: Advice/Read-Only Work

The user says:

```text
Before implementing, help me make the plan concrete.
```

The useful shape is read-only:

- inspect relevant files, docs, or sources;
- separate known facts from recommendations;
- name product decisions and technical decisions separately;
- propose safe next work;
- do not treat the recommendation as permission to write product files.

The user-facing response should stay plain:

```text
I inspected the login flow and session setup.

Known facts:
- The login form already has a persistent error area.
- Session lifetime is configured in one server-side module.

Decisions still yours:
- Product: should "remember me" mean email memory, longer session, or both?
- Security: if session lifetime changes, what risk is acceptable?

Safe next step:
Choose the product meaning first, then keep implementation scoped to login form, selected session behavior, and focused tests.
```

The boundary: advice is not acceptance, cited output is not user judgment, and a plan is not write authority.

## Scenario 2: Small Direct Change

The user says:

```text
Keep this as a small change: rename the profile button from "Save" to "Update profile."
```

The useful shape is intentionally narrow:

- scope: profile button copy and a directly related copy test, if one exists;
- out of scope: save behavior, profile redesign, localization strategy, broader copy cleanup;
- evidence: changed path plus focused check or self-check;
- escalation point: stop if the label is shared across unrelated flows or changes meaning.

A compact result should be enough when the work stays small:

```text
Changed the profile button label to "Update profile."
Checked the directly related copy path.
Small-change boundary held: no behavior, contract, or broader copy change found.
No known close-relevant residual risk for this change.
```

The boundary: small work can stay light, but it must stop being treated as small when the scope grows.

## Scenario 3: Tracked Work

The user says:

```text
Add remember-me behavior to the login flow.
```

This sounds simple, but it may change authentication behavior, session lifetime, UI copy, storage, tests, and security expectations. The useful shape is tracked work:

- clarify the product meaning before implementation;
- separate the user-owned choices from codebase-answerable facts;
- keep the scope visible;
- connect completion claims to evidence references;
- show verification and manual QA expectations when they matter;
- show residual risk before final acceptance or close.

The agent can ask in ordinary language:

```text
Should "remember me" extend the login session on this device, remember the email address, or both?
```

If the answer is "extend the session," the next summary should name the boundary:

```text
Scope: login checkbox, selected session behavior, focused tests, and directly related copy.
Out of scope: passwordless login, account recovery, and global session redesign.
Still needed before close: evidence for remembered and non-remembered sessions, human QA for the login screen, and visibility into remaining session-risk limits.
```

The boundary: the agent should not hide a product/security choice inside implementation just because the code can be changed.

## Scenario 4: User Judgment Is Specific

During a tracked task, the agent finds a UX trade-off:

```text
Failed-login feedback can be inline, a toast, or a modal.
```

A good prompt does not ask for broad approval:

```text
Decision needed: failed-login feedback pattern.
Why now: final UI behavior and human QA depend on this choice.
Options: inline message near the form, toast, or modal.
Recommendation: inline message near the form because it stays visible and is easier to make accessible.
Uncertainty: confirm the design system's existing error-message support.
If deferred: backend validation can continue, but final UI behavior and human QA should wait.
```

The boundary: "go ahead" only answers this decision if it clearly selects or defers this named choice. It should not also accept residual risk, waive QA, or accept the finished work.

## Scenario 5: Close Readiness Is Not "Tests Passed"

The agent says:

```text
The code is done and tests pass.
```

That may still be insufficient to close. A useful close-readiness response names the missing support:

```text
Close still blocked:
- Human QA is pending for the login error workflow.
- Residual session-risk limits have not been shown for acceptance.

Smallest unblocker:
Record the QA result, or explicitly waive that QA with the skipped surface and close impact named. Then show the residual risk before asking for acceptance.
```

The boundary: test pass is not manual QA, self-check is not detached verification, and work acceptance is not the same thing as permission for a sensitive action.

## Scenario 6: Markdown Is A View

A generated Markdown status says:

```text
Evidence: partial
Next action: record human QA
source_state_version: 42
```

That report is useful for reading status. It is not the state.

If a person edits the Markdown to say:

```text
Evidence: sufficient
```

the edit does not change saved evidence references, QA status, acceptance, residual risk, or close readiness. The future Harness system should treat that edit as a note or reconciliation input, not as authority by itself.

The boundary: Markdown projection is not state.

## Reference Owners

This tour intentionally avoids API detail. For exact future contracts, use:

| Need | Owner |
|---|---|
| Scope, user-owned judgment, evidence, verification, QA, acceptance, residual risk, and close rules | [Kernel Reference](../reference/kernel.md) |
| Public tool shapes and schema details | [MCP API and Schemas](../reference/mcp-api-and-schemas.md) |
| Readable document boundaries and freshness | [Document Projection Reference](../reference/document-projection.md) |
| User-facing session behavior | [User Guide](../use/user-guide.md) and [Agent Session Flow](../use/agent-session-flow.md) |

## Where To Go Next

- Read [Harness in One Task](harness-in-one-task.md) for a fuller walkthrough.
- Read [Concepts](concepts.md) when internal labels start appearing.
- Read [User Guide](../use/user-guide.md) when you want the user-facing session flow.
