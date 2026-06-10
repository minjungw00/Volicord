# Start

## What Harness is

Harness is a future local work-authority server for AI-assisted product work. Its job is to keep the fragile basis of a task out of chat-only memory: scope, user-owned judgment, evidence, check expectations, close readiness, and residual risk.

Users should be able to speak normally:

```text
Make this plan concrete enough to implement.
Tell me if the scope is getting bigger.
Show what I need to decide and what you can verify.
Before you say it is done, show the evidence and residual risk.
```

The agent can answer in the same ordinary language. When a request hides product, technical, user-visible inspection, acceptance, or risk choices, Harness should make those choices visible.

The agent should request and record needed judgments, not decide them silently.

This repository is documentation-only. It is source material for a future Harness Server, not a running server or runtime implementation.

## What Harness is not

Harness is not:

- a prompt pack
- a chat script
- MCP itself
- an API wrapper
- a workflow engine
- a report generator
- a dashboard
- a hosted agent platform
- a Product Repository
- a Harness Runtime Home

Harness can record and display authority boundaries, but it must not claim enforcement that the current MVP does not provide. For the canonical security boundary, see [Security](reference/security.md).

Readable views, templates, status cards, and summaries are derived display. They help people read Harness state, but they do not become the authority record just because they are well written or manually edited.

## The problem it solves

AI-assisted work can move faster than the record around it:

- A small request grows.
- A product choice gets buried in implementation.
- A test pass starts sounding like proof of the whole user experience.
- A user says "looks good" and the agent treats every unresolved judgment as settled.

Harness exists to make those substitutions visible and invalid as authority.

It keeps the working basis explicit enough that a future reader can see:

- what was in scope
- what the user decided
- what evidence supports the claim
- what was verified
- what still needs human judgment
- whether the work can honestly close

## One ordinary task

A user might ask:

```text
Add remember-me behavior to login, but clarify the plan before changing files.
```

A useful agent response does not start by choosing hidden product and security behavior. It names a narrow goal, likely non-goals, the facts it can inspect, and the judgments the user still owns:

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

## What the user owns

The user owns:

- goals
- meaningful scope changes
- product behavior
- material technical direction
- user-visible inspection or quality expectations
- final acceptance
- residual-risk acceptance
- scoped permission for named sensitive actions when those actions are needed

The agent may compare options, inspect source, run checks, name evidence gaps, and explain consequences. It must not turn tool output, passing tests, generated summaries, or its own confidence into the user's judgment.

Broad phrases such as "go ahead" or "looks good" should be applied only to the specific pending choice they reasonably answer. They do not automatically grant new sensitive-action permission, accept the finished work, accept known residual risk, or settle another future judgment candidate.

## What the agent can do

The agent can:

- clarify requirements
- name non-goals
- propose a next safe work slice
- inspect the relevant files or sources
- show options with uncertainty
- implement inside the agreed scope
- run focused checks
- report evidence

When the work shape changes, the agent should say so. A typo fix can stay light. A bug fix can begin with inspection and a narrow repair.

A broad feature request should expose:

- the goal
- non-goals
- success criteria
- unknowns
- user-owned judgments
- evidence plan
- close blockers

When the agent cannot honestly proceed without a user-owned judgment, it should ask a specific question rather than treating silence or momentum as permission.

## What evidence means

Evidence is support for a claim. It can be a diff, test output, screenshot, log, source citation, review note, or artifact reference. Evidence should say what it supports and what it does not support.

Evidence shows what the work did, but it does not replace the user's final acceptance. Evidence is not user judgment. It is also not automatically a complete check of the user experience.

A test pass is not the same as user-visible review of copy, accessibility, visual layout, or the lived flow. Review can support acceptance, but it is not final acceptance.

If evidence is missing, stale, weak, or limited to the agent's own check, Harness should keep that visible instead of rounding it into "done."

## What close readiness means

Close readiness answers a simple question: can this work honestly finish now, and what still blocks it? The detailed current behavior belongs in [Active MVP Scope](reference/active-mvp-scope.md) and the Reference owners it links to.

If something is missing, close should name the smallest unblocker. Close-readiness order and blocker details belong to [Core Model](reference/core-model.md), [MVP API](reference/api/mvp-api.md), and [API Errors](reference/api/errors.md).

## Current MVP scope

The current MVP is intentionally narrow. For the canonical current scope, see [Active MVP scope](reference/active-mvp-scope.md).

Later candidates are not active requirements until an owner promotes them. If a detail is not in current scope, treat it as deferred even when it appears in examples or future-looking notes.

## Current MVP guarantee boundary

The current MVP guarantee boundary is modest. For canonical guarantee levels and security non-claims, see [Security](reference/security.md); for owner routing, use the [Reference Index](reference/README.md).

## Where to read next

- [User Guide](use/user-guide.md) for practical user and agent behavior.
- [Agent Guide](use/agent-guide.md) for agent-facing session guidance.
- [Active MVP Scope](reference/active-mvp-scope.md) for what is currently in and out of scope.
- [MVP Plan](build/mvp-plan.md) for repository status, handoff, and future implementation readiness boundaries.
- [Reference Index](reference/README.md) only when you need exact future contracts.
- [Authoring Guide](maintain/authoring-guide.md) before editing documentation.
