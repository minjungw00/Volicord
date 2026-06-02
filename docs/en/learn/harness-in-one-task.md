# Harness in One Task

## Start With One Normal Request

This tutorial follows one feature request through the three user-visible work shapes:

- advice/read-only work;
- small direct change;
- tracked work.

Users can ask in ordinary language. The agent's job is to show the authority boundary: what is being attempted, what may change, what only the user can decide, what supports the claim, what still blocks close, and what risk remains.

This repository is documentation-only. The examples describe the intended future Harness behavior; they are not runtime transcripts from an implementation in this repo.

## Main Idea

Harness keeps AI-assisted product work grounded in local Core-owned state by tracking scope, user-owned judgments, evidence references, close readiness, acceptance, and residual risk outside the chat.

The task story below keeps returning to the same non-substitution rules:

- Chat is not state.
- Markdown projection is not state.
- Tool output is not user judgment.
- Sensitive-action approval is not work acceptance.
- Test pass is not manual QA.
- Self-check is not detached verification.
- "Proceed" or "looks good" does not automatically resolve every pending judgment.

## Step 1: Advice/Read-Only Planning

The user starts with:

```text
Before implementing, help me make the plan concrete.
```

The agent should not start by changing product files. It should inspect, separate facts from choices, and show a safe next step.

```text
What I can inspect:
- current login form structure;
- current session lifetime configuration;
- existing tests around login state.

What you likely need to decide:
- Product: should "remember me" keep the user signed in, remember the email, or both?
- Security: if it keeps the user signed in, what session lifetime is acceptable?

Safe next step:
Inspect auth shape and return a narrow plan. No product writes yet.
```

The authority boundary is important: the plan is useful, but it is not permission to implement and not acceptance of any risk.

## Step 2: Scope Becomes Concrete

After inspection, the agent reports:

```text
Current facts:
- Login form can add a checkbox without a layout rewrite.
- Session lifetime is controlled server-side.
- Tests already cover sign-in and sign-out.

Open choices:
- Product meaning of "remember me."
- Session lifetime and storage approach if the meaning is longer session.
```

The user replies:

```text
Separate the product decisions from the technical decisions.
```

The agent should show the split:

```text
Product decision:
Should "remember me" mean email memory, longer session, or both?

Technical decision:
If longer session is selected, should the implementation use the existing server-side session mechanism or introduce a separate persistent token?

Recommendation:
Choose product meaning first. Then decide whether the existing session mechanism is enough.
```

Tool output helped find facts. It did not make the user-owned decisions.

## Step 3: A Small Direct Change Stays Small

During planning, the user notices a separate typo:

```text
Keep this as a small change and tell me if the scope grows: rename the login button from "Sign in" to "Log in."
```

This is a small direct change if the label is local and the meaning stays the same.

```text
Scope:
Login button label and directly related copy test, if one exists.

Out of scope:
Authentication behavior, session lifetime, remember-me UI, localization strategy, and broader copy cleanup.
```

A compact result is enough:

```text
Changed the login button label to "Log in."
Checked the directly related copy path.
Small-change boundary held: no behavior or shared-contract change found.
No known close-relevant residual risk for this small change.
```

If the agent discovers that the label comes from a shared phrase used across account creation, password reset, and marketing pages, it should stop and explain that the scope grew. Small direct change is a work shape, not a license to ignore wider impact.

## Step 4: The Feature Becomes Tracked Work

The user returns to the feature:

```text
Add remember-me behavior to login.
```

Because this affects product behavior, security expectations, tests, and user experience, it should be tracked work.

The first useful boundary is:

```text
In scope:
Login form checkbox, chosen remember-me behavior, focused tests, and directly related copy.

Out of scope:
Passwordless login, account recovery, global session redesign, unrelated auth cleanup.

Still user-owned:
Meaning of "remember me"; acceptable session lifetime and storage risk if sessions are extended.
```

The agent then asks the named product question:

```text
Should "remember me" extend the login session on this device, remember the email address, or both?
```

If the user answers:

```text
Extend the login session on this device.
```

that answers the product meaning. It does not automatically answer the technical storage decision, manual QA expectation, residual risk, or final acceptance.

## Step 5: A Sensitive Step Is Not Acceptance

Suppose the chosen implementation needs to read a local secret or update a dependency lockfile. The agent may need permission for that sensitive step.

The prompt should be specific:

```text
Sensitive step permission needed:
May I update the dependency lockfile to add the session helper required by the chosen approach?

This permission only covers that step. It does not accept the final result, waive QA, or accept residual risk.
```

If the user says "proceed," that permission applies to the named sensitive step. It should not be stretched into acceptance of the completed feature.

## Step 6: Evidence Supports Claims

After implementation, the agent should connect claims to support:

```text
Claim:
Remembered sessions survive browser restart.

Evidence references:
- diff for login form and session behavior;
- focused test output for remembered and non-remembered sessions;
- implementation run notes.
```

Evidence is not the agent saying "done." It is the durable support that lets a future reader ask what backs the claim.

A Markdown summary can display the evidence references, but the Markdown text is not the evidence record.

## Step 7: Verification And Manual QA Stay Separate

A useful tracked-work status distinguishes checks:

```text
Automated check:
Focused session tests passed.

Verification:
Separate review confirmed remembered and non-remembered session behavior from current evidence.

Manual QA:
A person still needs to inspect the login screen flow, checkbox copy, keyboard behavior, and error-state layout.
```

The boundary matters:

- test pass is not manual QA;
- self-check is not detached verification;
- a QA waiver is not the same thing as QA passing.

If manual QA is waived, the skipped surface and close impact should be named. If verification is waived, the remaining verification risk should stay visible.

## Step 8: Residual Risk Before Acceptance

Before asking the user to accept the result, the agent should show known remaining uncertainty:

```text
Residual risk:
Remembered-session behavior was checked in the local browser path, but not across every supported browser policy combination.
```

If the user says:

```text
Looks good.
```

that phrase should not automatically accept every unresolved risk or judgment. The agent should connect it to the specific acceptance request:

```text
To close this tracked work, please confirm:
Do you accept the remember-me result with the shown residual risk?
```

Acceptance is the user's result judgment. It is separate from sensitive-action permission, test output, manual QA, and verification.

## Step 9: Close Readiness

A close-ready summary should be short and grounded:

```text
Close readiness:
- Scope stayed within login form, selected session behavior, focused tests, and related copy.
- Product meaning was decided: extend login session on this device.
- Evidence references cover remembered and non-remembered session behavior.
- Verification and manual QA are handled, or their waivers and risks are visible.
- Residual risk was shown.
- Final acceptance was requested for the named result and risk.
```

If anything is missing, the summary should name the smallest unblocker:

```text
Close blocked:
Manual QA for the login screen has not passed or been explicitly waived.
```

## What The User Should Learn

The learning path is not a feature list. It is an authority-boundary model:

- advice can guide work without authorizing writes;
- a small direct change can stay light while the boundary holds;
- tracked work makes decisions, evidence, QA, verification, acceptance, risk, and close readiness visible;
- chat, Markdown, tool output, tests, approvals, and self-checks are useful, but none of them silently substitutes for another authority.

## Where To Go Next

- Read [Harness in 15 Minutes](harness-in-15-minutes.md) for shorter examples.
- Read [Concepts](concepts.md) when internal labels start appearing.
- Read [User Guide](../use/user-guide.md) for the user-facing session flow.
- Use [Kernel Reference](../reference/kernel.md) only when you need exact future contracts.
