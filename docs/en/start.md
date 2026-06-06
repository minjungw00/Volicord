# Start

## What Harness Is

Harness is a future local work-authority server for AI-assisted product work. Its job is to keep the fragile basis of a task out of chat-only memory: scope, user-owned judgment, evidence, verification expectations, close readiness, and residual risk.

Users should be able to speak normally:

```text
Make this plan concrete enough to implement.
Tell me if the scope is getting bigger.
Show what I need to decide and what you can verify.
Before you say it is done, show the evidence and residual risk.
```

The agent can answer in the same ordinary language. When a request hides product, technical, QA, acceptance, or risk choices, Harness should make those choices visible instead of letting the agent infer them silently.

This repository is documentation-only today. It describes intended future Harness behavior, but it does not contain a running Harness Server or runtime implementation.

## What Harness Is Not

Harness is not a prompt pack, chat script, MCP itself, API wrapper, workflow engine, report generator, dashboard, hosted agent platform, Product Repository, or Harness Runtime Home.

Harness also is not an operating-system permission system, arbitrary-tool sandbox, tamper-proof store, default pre-tool blocker, or security-isolation boundary. It can record and display authority boundaries, but it must not claim enforcement that the current MVP does not provide.

Readable projections, templates, status cards, and summaries are derived display. They help people read Harness state, but they are not the Core authority record just because they are well written or manually edited.

## The Problem It Solves

AI-assisted work can move faster than the record around it. A small request grows. A product choice gets buried in implementation. A test pass starts sounding like proof of the whole user experience. A user says "looks good" and the agent treats every unresolved judgment as settled.

Harness exists to make those substitutions visible and invalid as authority. It keeps the working basis explicit enough that a future reader can see what was in scope, what the user decided, what evidence supports the claim, what was verified, what still needs human judgment, and whether the work can honestly close.

## One Ordinary Task

A user might ask:

```text
Add remember-me behavior to login, but clarify the plan before changing files.
```

A useful agent response does not start by choosing hidden product and security behavior. It names a narrow goal, likely non-goals, the facts it can inspect, and the choices the user still owns:

```text
Goal I heard:
Add remember-me behavior to login without redesigning authentication.

Out of scope unless you decide otherwise:
Password reset, signup, social login, and unrelated session maintenance.

I can inspect:
The login form, session settings, and focused tests.

You likely need to decide:
Whether "remember me" means a longer session, remembered email, or both, and what session risk is acceptable.

Safe next step:
Inspect and return a narrow plan. No product writes yet.
```

The user did not need to say an internal mode name. The agent clarified because the request touches product behavior, security expectations, checks, and user-owned judgment.

## What The User Owns

The user owns goals, meaningful scope changes, product behavior, material technical direction, QA expectations, final acceptance, and residual-risk acceptance. The user also owns scoped permission for named sensitive actions when those actions are needed.

The agent may recommend, compare options, inspect source, run checks, and explain consequences. It must not turn tool output, passing tests, generated summaries, or its own confidence into the user's judgment.

Broad phrases such as "go ahead" or "looks good" should be applied only to the specific pending choice they reasonably answer. They do not automatically grant new sensitive-action permission, waive QA, accept the finished work, or accept known residual risk.

## What The Agent Can Do

The agent can clarify requirements, name non-goals, propose a next safe work slice, inspect the relevant files or sources, recommend options with uncertainty, implement inside the agreed scope, run focused checks, and report evidence.

When the work shape changes, the agent should say so. A typo fix can stay light. A bug fix can begin with inspection and a narrow repair. A broad feature request should expose the goal, non-goals, success criteria, unknowns, user-owned judgments, evidence plan, and close blockers.

When the agent cannot honestly proceed without a user-owned judgment, it should ask a specific question rather than treating silence or momentum as permission.

## What Evidence Means

Evidence is support for a claim. It can be a diff, test output, screenshot, log, source citation, review note, or artifact reference. Evidence should say what it supports and what it does not support.

Evidence is not final acceptance. Evidence is not user judgment. Evidence is not automatically verification. A test pass is not Manual QA for copy, accessibility, visual layout, or the human experience. QA can support acceptance, but QA is not final acceptance.

If evidence is missing, stale, weak, or limited to the agent's own check, Harness should keep that visible instead of rounding it into "done."

## What Close Readiness Means

Close readiness answers a simple question: can this work honestly finish now, and what still blocks it?

A close-ready task should show that scope stayed bounded, required user-owned judgments were handled, evidence supports the stated result, checks or verification expectations are clear, required QA is passed or explicitly waived where allowed, final acceptance is handled when required, and known residual risk is visible and accepted only when the user actually accepts it.

If something is missing, close should name the smallest unblocker, such as a pending product decision, missing evidence, deferred QA, unaccepted residual risk, or final acceptance that has not happened yet.

## Current MVP Guarantee Boundary

The current MVP guarantee boundary is intentionally modest. Read MVP wording as cooperative guidance and limited detective visibility unless a specific future/profile mechanism is named, implemented, and proven.

In the current MVP, Harness does not claim OS-level permissions, arbitrary-tool isolation, tamper-proof local files, default pre-tool blocking, or broad security isolation. A future Write Authorization record is a cooperative scope check, not an operating-system permission or sandbox.

Preventive or isolated claims need a documented mechanism and proof path for the covered operation. Until then, Harness should say what it can record, guide, display, or detect without overstating what it can block.

## Where To Read Next

- [User Guide](use/user-guide.md) for practical user and agent behavior.
- [Agent Guide](use/agent-guide.md) for agent-facing session guidance.
- [MVP Plan](build/mvp-plan.md) for repository status, handoff, and future implementation readiness boundaries.
- [Reference Index](reference/README.md) only when you need exact future contracts.
- [Authoring Guide](maintain/authoring-guide.md) before editing documentation.
