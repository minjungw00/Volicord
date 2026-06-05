# Harness in One Task

## Start With One Normal Request

This page follows one future Harness-assisted task. It is a learning story, not a runtime transcript. This repository is documentation-only and does not contain a running Harness Server.

The user asks:

```text
Build a login feature. Make the plan concrete enough to implement before changing files.
```

The user does not need to know internal labels. The agent should make the work understandable through plain questions: what is the goal, what is not a goal, what will count as success, what is still unknown, what does the user need to decide, what evidence will support the result, what was checked, and what still blocks close?

## 1. Clarify Before Writing

The agent should not start by changing files when the request still hides product, technical, or security choices.

Good agent response:

```text
This is larger than a direct edit, so I need to clarify before implementation.

Goal I heard:
Add a login capability that can be implemented as a narrow first slice.

Non-goals I will not assume:
Signup, password reset, social login, global auth redesign, hard-coded secrets, or broad UI rewrites.

Success criteria to shape:
The first slice lets the chosen user type sign in, reports failure in the chosen UI pattern, has focused checks, and keeps excluded auth work out of scope.

What I can inspect:
- current auth routes and middleware;
- current session handling;
- existing login, logout, and user tests;
- UI patterns for forms, errors, and save feedback.

What you likely need to decide:
- login experience: email/password, magic link, OAuth/OIDC, social login, or another path;
- technical direction: server-side session cookie, JWT/bearer token, external identity provider, or a hybrid;
- product behavior: signup, password reset, failed-login feedback, and account recovery scope;
- QA expectations for the user-facing login flow.

Safe next step:
Inspect the current auth shape and return a narrow plan. No product writes yet.
```

Bad agent response:

```text
I will build login with JWTs, social login, reset, signup, and modal errors, then mark it done when tests pass.
```

The good response is useful because it separates the broad request into goal, non-goals, success criteria, facts the agent can inspect, and decisions the user owns. The bad response silently chooses architecture, UX, and scope.

## 2. Separate Facts From User Judgment

After inspection, the agent can summarize:

```text
Confirmed facts:
- The app already has server-side session middleware.
- The existing UI uses inline form errors and toast messages for non-blocking success feedback.
- Tests already cover logout but not failed-login copy or account lockout.

Still yours to decide:
- whether this first slice is email/password only;
- whether sessions should stay server-side or move to JWT/bearer tokens;
- whether OAuth/OIDC or social login belongs in this task or a later one;
- whether failed-login feedback should be inline, toast, or modal.
```

Tool output can inform the choice. It does not make the user's choice.

## 3. Name The Scope

Once the user chooses an email/password first slice with server-side session cookies, the agent should name the work boundary before implementing:

```text
In scope:
Email/password login, server-side session cookie integration, inline failed-login message, focused tests, and directly related copy.

Out of scope:
Account creation, password reset, OAuth/OIDC, social login, global session redesign, unrelated auth cleanup, and hard-coded secrets.

Still user-owned if it matters:
Session lifetime, secret handling policy, Manual QA expectation, residual-risk acceptance, and final acceptance.
```

When product writes are involved, this bounded first slice is the next safe Change Unit. The user did not have to say that label; the agent should create the scoped work boundary once enough information exists.

If the user says "Show a message after clicking Save" inside this work, the agent should treat that as a product choice, not a hidden default. A toast may be right for non-blocking success feedback; inline text may be right for field-specific errors; a modal should be chosen only when interrupting the flow is intentional.

## 4. Ask Specific Questions

Harness should not turn every uncertainty into a heavy ceremony. It should ask the specific question that blocks the next safe action or the user-owned judgment needed for honest progress.

```text
Decision needed:
Which login session architecture should this first slice use?

Options:
- Server-side session cookie for first-party web login.
- JWT or bearer token handled by the client.
- OAuth/OIDC identity provider with a local session or token strategy when needed.
- Social login provider integration with account-linking implications.

Recommendation:
Use the existing server-side session cookie for the first slice unless you already need third-party identity, non-browser clients, or SSO.

Uncertainty:
I still need to confirm session expiration and rotation behavior before claiming the implementation is complete.
```

A broad "go ahead" should not be stretched to cover unrelated product decision, technical decision, QA waiver, final acceptance, or residual-risk acceptance.

## 5. Keep Sensitive Permission Narrow

If the chosen approach requires a sensitive step, the prompt should say exactly what permission covers.

```text
Sensitive step permission needed:
May I update the dependency lockfile to add the named session helper for this task?

This only covers that dependency action. It does not decide the architecture by itself, accept the finished result, waive QA, or accept residual risk.
```

Permission for a named step is not final acceptance.

If the user asks:

```text
Can I put the API key in the code?
```

the agent should answer plainly:

```text
No. Do not put a raw API key in source code. I will check the project's existing secret path and recommend that instead, such as an environment variable, ignored local secret file, platform secret, or secret manager.
```

That answer is a security/privacy judgment. It should not claim Harness provides OS-level secret isolation or tamper-proof storage unless that exact mechanism exists.

## 6. Show Evidence And Checks

After implementation, the agent should connect claims to support:

```text
Claim:
Email/password login creates a server-side session and shows inline feedback for failed login.

Evidence:
- changed login form, auth route, and session middleware paths;
- focused test output for valid login, invalid login, and logout;
- diff review showing password reset, signup, OAuth/OIDC, and social login were not added.

Checks:
- focused automated tests passed;
- login flow still needs human QA for copy, keyboard behavior, screen-reader announcement, and visual layout.
```

Evidence, verification, and Manual QA are related but separate. A test pass does not prove the human experience, and a self-check is weaker than a genuinely separate verification path.

If the user says:

```text
I will test it later.
```

the agent should not hide the gap:

```text
I will show Manual QA as deferred, not passed. The implementation can wait for your later check, but close remains blocked unless the active path allows a QA waiver and any visible residual risk is handled separately.
```

## 7. Show Residual Risk Before Acceptance

Before asking the user to accept the work, the agent should show known remaining uncertainty:

```text
Residual risk:
The focused tests cover the main login path, but manual browser QA has not confirmed small-screen layout, keyboard flow, or screen-reader announcements.
```

If the user says:

```text
Looks good, go ahead.
```

the agent should connect that phrase to the named judgment:

```text
I can apply "go ahead" to the one pending choice if you mean the inline failed-login message. It does not accept the finished work, waive Manual QA, grant new sensitive-step permission, or accept the residual risk shown above.
```

Final acceptance is the user's result judgment. It is separate from sensitive-step permission, evidence, tests, Manual QA, verification, and residual-risk visibility.

## 8. Close Honestly

A useful close summary is short and grounded:

```text
Close readiness:
- Scope stayed within email/password login, server-side sessions, inline failed-login feedback, focused tests, and related copy.
- User-owned product and technical choices were named.
- Evidence supports the main login and logout behavior.
- Manual QA is handled, or its waiver and impact are visible.
- Residual risk was shown before final acceptance.
```

If something is missing, close should name the smallest unblocker:

```text
Close blocked:
Manual QA for the login screen has not passed or been explicitly waived.
```

## Smaller Requests Should Stay Smaller

The same behavior scales down.

```text
User request: Only fix typos in this document.
Good agent response: I will edit typo-level issues only and do a diff review for unintended meaning changes.
Bad agent response: I will rewrite the section and add new examples.
```

```text
User request: Fix this bug.
Good agent response: I will inspect the failing behavior and nearby tests first, then make the smallest fix or ask for the missing repro detail.
Bad agent response: I will refactor the whole module and call it fixed if tests pass.
```

```text
User request: Clean up all these files.
Good agent response: I will inspect first and propose a narrow cleanup slice before deleting, renaming, or reorganizing.
Bad agent response: I will delete unused-looking files and restructure the folder.
```

## What To Remember

Harness should make one task easier to trust, not harder to start. Users speak normally. Agents clarify when needed. User-owned judgments stay with the user. Evidence, verification, Manual QA, final acceptance, residual risk, and close readiness stay distinct.

## Where To Go Next

- Read [Concepts](concepts.md) for the minimum vocabulary.
- Read [User Guide](../use/user-guide.md) for practical session behavior.
- Use [Core Model Reference](../reference/core-model.md) only when exact future contracts are needed.
