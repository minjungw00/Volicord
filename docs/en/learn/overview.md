# Overview

## Start Here

Read this first if Harness is new to you.

Harness is a local work-authority server for AI-assisted product work. Its job is to keep fragile work criteria out of chat-only memory. Scope, user-owned judgment, evidence, checks, final acceptance, close readiness, and residual risk should not live only in a confident message or a generated report.

Users can speak normally:

```text
Make this plan concrete enough to implement.
Help me clarify the plan before implementation.
Tell me if the scope is getting bigger.
Show what I need to decide and what you can verify.
```

The agent should answer in the same ordinary language. When requirements are blurry, it should clarify before acting by naming the goal, non-goals, success criteria, unknowns, user-owned decisions, and next safe implementation unit. When the agent should not decide, Harness routes that judgment back to the user instead of letting tool output, tests, or summaries silently decide.

This repository is documentation-only today. It describes intended future Harness behavior, but no Harness Server or runtime implementation exists here yet. This repository is not a Product Repository and not a Harness Runtime Home.

## Why Harness Exists

AI-assisted work can move faster than the record around it. A small request grows. A product choice gets buried in implementation. A test pass is treated as proof of the whole experience. A user says "looks good" and the agent assumes every unresolved judgment is settled.

Harness exists to stop those substitutions. It keeps the working basis local and explicit enough that a future reader can see what was in scope, what the user decided, what evidence supports the claim, what was checked, what still needs human judgment, and whether the work can honestly close.

## How It Feels

In one task, Harness should feel less like a procedure and more like a careful working memory.

The user might ask:

```text
Add remember-me behavior to login, but clarify the plan before changing files.
```

A useful agent response names the boundary:

```text
Goal I heard:
Add remember-me behavior to login without redesigning authentication.

Out of scope unless you decide otherwise:
Password reset, signup, social login, and unrelated session cleanup.

Success criteria to shape:
The chosen remember-me behavior is clear to the user, covered by focused checks, and does not silently widen the login scope.

I can inspect the login form, session settings, and focused tests.

You likely need to decide:
- whether "remember me" means longer session, remembered email, or both;
- what session risk is acceptable if the login lasts longer.

Safe next step:
Inspect and return a narrow plan. No product writes yet.
```

The user did not need to say an internal mode name. The agent clarified because the request affected product behavior, security expectations, tests, and user-owned judgment.

## What Harness Keeps Separate

Harness is built around a few separations. Learn these before learning any internal labels.

| Keep separate | Why it matters |
|---|---|
| Scope | The agent should know what may change and where to stop. |
| User-owned judgment | Product, technical, scope, QA waiver, final acceptance, and residual-risk choices stay with the user when they matter. |
| Evidence | Evidence supports claims; it does not make the user's decision. |
| Checks and verification | A test, review, inspection, or stronger verification says what was checked, not that every human concern is done. |
| Manual QA | Human experience, copy, accessibility, and visual quality may need human inspection even when tests pass. |
| Final acceptance | Accepting the finished result is different from permitting a sensitive step. |
| Residual risk | Known remaining uncertainty should be visible before final acceptance or close. |
| Close readiness | Close should say what is handled and what still blocks an honest finish. |

The learning rule is simple: one kind of support should not silently substitute for another.

## Authority Boundary

Harness separates useful surfaces from the record that carries authority.

| Surface | Good for | Not enough for |
|---|---|---|
| Chat | Coordination, explanation, questions, summaries. | Durable state or broad implied acceptance. |
| Tool output | Logs, diffs, tests, screenshots, connector responses. | User judgment by itself. |
| Product files | The actual product result. | Harness operating state. |
| Readable summaries | Human-readable status and reports. | The source of authority if edited by hand. |
| Local Harness record | The future operating record for scope, judgments, evidence, checks, close, and risk. | A replacement for tests, review, product specs, or source control. |

Reference docs give the future record exact implementation names. First-time readers do not need those names.

## Harness Is Not

| Harness is not | Harness does |
|---|---|
| A prompt pack or chat script. | Keeps work authority outside prompts and conversation. |
| MCP itself or an API wrapper. | May use MCP/API surfaces as mechanisms. |
| A workflow engine, report generator, or dashboard. | Records the basis for work and can derive readable views from that record. |
| A hosted agent platform. | Is designed around a local Harness Server / Installation. |
| A sandbox or OS permission system. | Does not claim OS-level isolation or arbitrary-tool permission control. |

## First Learn Path

The Learn path is intentionally short:

1. [Overview](overview.md): what Harness is and why it exists.
2. [One Task](one-task.md): how one user work loop feels.
3. [Concepts](concepts.md): the minimum concepts and optional labels.

After that, use [User Guide](../use/user-guide.md) for practical session behavior or [Reference](../reference/README.md) only when exact future contracts are needed. [Purpose and Principles](purpose-and-principles.md) is an optional thesis check for reviewers.
