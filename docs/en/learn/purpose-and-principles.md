# Purpose and Principles

## Start Here

Read this when you want the thesis, values, and non-goals behind Harness before reading strict contracts or future implementation plans.

Harness is a local work-authority server for AI-assisted product work. Its job is to keep fragile conversation context from becoming the source of truth. It preserves the local basis for scope, user-owned judgment, evidence, verification expectations, work acceptance, close readiness, and residual risk, and routes decisions back to the user when the agent should not decide.

This page is a principles document. It is not an implementation status report and not a claim that the Harness Server already exists. This repository currently contains documentation only.

## Purpose

Harness exists to make AI-assisted product work followable while preserving user judgment.

The user should be able to begin in ordinary language. The agent should be able to clarify, inspect, recommend, implement when appropriate, check, and report. But authority over the work should not leak into chat phrasing, generated Markdown, connector output, test logs, or agent confidence.

Harness keeps the local work-authority record small and explicit. It tracks the work boundary, the choices the user owns, the evidence references behind claims, the checking and verification expectations, the remaining close blockers, the user's work acceptance when required, and any residual risk that still matters.

## Core Principles

Harness keeps authority local. The durable work record should not depend on a remote chat transcript or a generated report.

Harness separates unlike things. Scope, sensitive-action permission, product judgment, technical judgment, evidence, verification, manual QA, work acceptance, and residual-risk acceptance answer different questions.

Harness preserves user agency. The user owns goals, scope, product direction, material technical trade-offs, security/privacy judgment, QA expectations, work acceptance, and residual-risk acceptance.

Harness is honest about support. It should say what was checked, what kind of check it was, what evidence supports the claim, what remains unverified, and what still needs a person.

Harness keeps small work small. A typo or narrow leaf fix should not become ceremony. It should also stop being treated as small when scope, meaning, risk, UX, public behavior, sensitive action, or shared-contract impact appears.

Harness describes guarantees by capability, not aspiration. A cooperative surface can ask an agent to hold; a detective surface can report drift; only a proven preventive path should be described as blocking before action.

Harness complements ordinary engineering practice. Source control remains the history of product files. Tests remain executable checks. Code review remains review. Product specifications remain product specifications. Harness records the authority boundary around AI-assisted work.

## What Harness Is Not

| Harness is not | Harness does |
|---|---|
| A prompt pack or chat script. | Keeps work authority outside prompts and conversation. |
| MCP itself or an API wrapper. | May use MCP/API surfaces as implementation mechanisms, but they are not the product thesis. |
| A workflow engine, report generator, or dashboard. | Records the basis for work and can derive readable views from that record. |
| A hosted agent platform. | Is designed around a local Harness Server / Installation. |
| A sandbox or OS permission system. | Preserves authority boundaries without claiming OS-level isolation or arbitrary-tool permission control. |

Harness can use instructions, MCP/API surfaces, reusable workflows, test output, review notes, reports, dashboards, and specs as surfaces or evidence sources. They do not become Harness authority by being useful.

## Strategic Thesis

AI-assisted work can move quickly without pushing the user out of judgment when the authority boundary is explicit.

That thesis has three consequences.

First, chat is coordination, not durable state. It can propose, explain, and summarize. It should not decide by implication that work may write, close, accept risk, or resolve every pending judgment.

Second, Core-owned state is the operating record. It stores the facts needed to reason about scope, user-owned judgment, evidence references, close readiness, work acceptance, and residual risk.

Third, readable Markdown is a view, not the record. A report can help a person inspect the work, but editing report prose does not change the saved evidence, work acceptance, QA status, verification status, risk state, or close readiness.

## Failure Model

Harness is designed around recurring failures in AI-assisted work.

Scope becomes implicit. A request starts narrow, then expands through "while you are there" changes. Harness responds by making the current boundary visible and requiring scope growth to be named.

User judgment is silently replaced. The agent makes a product, UX, architecture, security, QA, work acceptance, or risk choice as though it were an implementation detail. Harness responds by routing the named user-owned judgment back to the user.

Evidence, verification, QA, and completion claims collapse into one "done." Harness responds by keeping support types distinct: evidence references support claims, tests check what they check, manual QA covers human inspection, and detached verification is stronger than self-check.

Readable surfaces look authoritative. A chat summary, tool output, or Markdown report may sound final. Harness responds with the non-substitution rules: those surfaces can inform the record, but they do not replace Core-owned state.

## Work Shapes

Harness should be visible to users as a small set of work shapes, not as a vocabulary quiz.

Advice/read-only work is for explanation, comparison, planning, investigation, or recommendation. It can cite sources and inspect current context, but it does not imply product writes, work acceptance, or residual-risk acceptance.

Small direct change is for narrow, clear edits. It can stay light when scope and result are obvious. It should stop and reshape when the work grows beyond the small boundary.

Tracked work is for meaningful product changes, multi-step delivery, user-owned decisions, evidence mapping, QA, verification, work acceptance, or residual risk. It keeps blockers visible until close readiness is clear.

Users can ask for these shapes in ordinary language:

```text
Before implementing, help me make the plan concrete.
Separate the product decisions from the technical decisions.
Keep this as a small change and tell me if the scope grows.
Show me what still prevents closing this work.
```

## MVP Boundary

The MVP boundary is about proving the local work-authority model, not building a broad platform.

The first future slices should prove that ordinary AI-assisted work can be represented as local scope, user-owned judgment, evidence references, close readiness, work acceptance, and residual risk without confusing those records with chat, Markdown, tool output, or product files.

Broader automation, richer projections, connector ecosystems, hosted workflows, dashboards, analytics, and large conformance suites are outside the first user-value slice thesis. They may become useful later only if they preserve the authority boundary instead of hiding it.

## Non-Goals

Harness does not replace the product repository, version control, tests, review, product specifications, user judgment, or team process.

Harness does not replace MCP, become a workflow engine, generate reports as the source of truth, provide a dashboard as the product center, host agents, or provide sandbox/OS permission control.

Harness does not treat chat history as state.

Harness does not treat generated Markdown as state.

Harness does not treat tool output as user judgment.

Harness does not turn sensitive-action approval into work acceptance.

Harness does not turn a test pass into manual QA.

Harness does not turn self-check into detached verification.

Harness does not treat "proceed" or "looks good" as resolving every pending judgment unless the specific judgment has actually been answered.
