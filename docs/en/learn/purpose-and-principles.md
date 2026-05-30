# Purpose and Principles

## What this document helps you do

This document explains why Harness exists, who it is for, what it values, how it thinks about common AI-assisted development failures, and what belongs inside the MVP boundary.

## Read this when

Read this when you want the values and boundaries behind Harness before reading strict contracts or implementation plans.

## Before you read

[Overview](overview.md) is useful first if Harness is new to you. No implementation background is required.

## Main idea

One sentence version: Harness is a local authority record and judgment-routing layer for AI-assisted product work, keeping scope, user-owned judgments, evidence, verification, QA expectations, final acceptance, and residual-risk status outside fragile chat context.

One paragraph version: In practice, Harness gives the user and agent a local record of what work is in scope, which judgments belong to the user, what supports completion claims, what still needs verification or QA, whether final acceptance has been given, and what risk remains. Chat stays conversation. Markdown projections are readable views. Core-owned local state and artifact references are the source of operational truth. Harness may use agent instructions, MCP, reusable workflows, tests, reviews, and specs, but it is not identical to any of them.

## Purpose

Harness exists to make AI-assisted development followable without taking strategic judgment away from the user.

The user should be able to begin in ordinary language. The agent should be able to clarify, propose, implement, check, and report. But the important facts of the work should not live only in the chat transcript. Harness keeps those facts in local state, durable evidence, and readable projections so the task can be resumed, checked, reconciled, and closed from current state.

Harness is not ceremony for its own sake. It maintains a local record for work that would otherwise become blurry, and it routes judgment when the work needs a user decision before it can continue or close.

Harness keeps the operating record around work, scope, user judgment, evidence, verification, QA, final acceptance, remaining risk, and close readiness. The point is not to replace the user's tools, source control, tests, code review, or conversation, but to make the work followable from current local state.

## What Harness is not

Harness is not the same kind of thing as AGENTS.md or other agent instruction files, MCP, skills or reusable workflows, test runners, code review, or specs.

Agent instructions guide agent behavior. MCP provides a tool and resource integration boundary. Skills and reusable workflows package repeated behavior. Test runners execute checks. Code review examines changes. Specs describe planned intent. Harness may use any of these as inputs, surfaces, or evidence sources, but it does not treat them as the local operating record or the owner of user judgment.

For the detailed role comparison table, use the [English documentation entrypoint](../README.md#comparison).

## Who it is for

Harness is for developers using AI agents to modify, verify, or explain product code. It is also for solo maintainers who need reliable resume behavior, technical leads who want local records of sensitive-action permission and final acceptance, connector authors integrating an agent surface with Harness, and documentation authors maintaining the current Harness model.

The shared need is simple: people want the speed of agent-assisted work without losing the ability to understand what happened, what is allowed, what was checked, and what still needs judgment.

## Core values

Harness keeps operational state and evidence local. The durable work record should not depend on a remote chat transcript.

Harness makes boundaries explicit. Scope, permission for sensitive actions (Approval in the Reference docs), decisions, evidence, verification, human QA, final acceptance (Acceptance), and remaining risk (Residual Risk) are different questions, so the system records them separately.

Harness is honest about assurance. It should say what changed, what was checked, how independent that check was, what remains risky, and what decision is needed, instead of treating every review as equally strong.

Harness preserves strategic agency. The user keeps judgment over goals, scope, design direction, product trade-offs, material technical trade-offs, codebase stewardship, QA, final acceptance, and residual-risk acceptance.

Harness keeps the work journey followable. A reader should be able to reconstruct current state, next action, decisions, evidence, and blockers without relying on chat memory.

Harness keeps design-quality policy visible without turning stewardship defaults into kernel invariants. The exact policy contracts live in [Design Quality Policies](../reference/design-quality-policies.md).

Harness prefers a small, buildable MVP. The first implementation should prove the authority and agency model with concrete fixtures before growing into broader automation.

Harness treats projections with humility. Markdown reports help humans read status and propose changes, but they do not silently become operational truth.

Harness describes capability by actual guarantee level, not by product name. If an agent surface can only cooperate or report after the fact, Harness should say that plainly.

Harness complements ordinary engineering discipline. Source control remains the history of product files, tests remain executable checks, code review remains human and team review, and user-owned product or material technical judgment remains with the user.

## Strategic thesis

The strategic thesis is simple: AI-assisted work can move quickly while the user keeps meaningful judgment.

In plain terms: AI agents can move quickly without pushing the user out of the driver's seat when the system keeps the work journey explicit, keeps durable truth small, and records user-owned product or material technical judgment at the moments where judgment matters.

This thesis has three practical consequences.

First, chat is an operating surface, not durable state. It is where people and agents coordinate, but it is not the record that decides whether work can write or close.

Second, Harness state is the operating record. It stores the work, scope, user-owned decisions, sensitive-action permission, evidence refs, verification, QA, final acceptance, and residual risk needed to reason about the work.

Third, readable documents are projections and proposal areas. They help humans inspect and edit the story of the work, but accepted changes enter the operating record only through an explicit state-changing path. Projection freshness is a display fact, not a substitute for current state, evidence, tests, review, or final acceptance.

## Failure model, rewritten as reader-facing problems

Harness focuses on four reader-facing failures.

Scope drifts or becomes implicit. A small request can become a broad rewrite, or the reason a path is in scope can disappear into conversation. Harness responds by recording the current work boundary and making scope changes visible before they are treated as part of the task.

User-owned judgment is silently replaced by agent judgment. Product direction, important technical trade-offs, QA expectations, final acceptance, and residual-risk acceptance can be hidden inside implementation. Harness responds by routing those judgments back to the user when they block progress, write, close, waiver, or final acceptance.

Evidence, verification, QA, and completion claims get mixed. A test result can be treated like proof of UX quality, a self-check can be treated like independent verification, or a waiver can be treated like the skipped check happened. Harness responds by recording evidence, verification, QA, final acceptance, and remaining risk as separate questions.

Chat or Markdown output is mistaken for operational truth. A stale report, edited note, or confident chat summary can look authoritative. Harness responds by treating chat as conversation, Markdown as projection, and Core-owned local state plus artifact references as the source of operational truth.

## MVP boundary in plain language

The staged MVP model separates the first runnable kernel slice from the first user-facing MVP. It proves the local authority record and agency model, not a broad platform.

v0.1 Core Authority Slice should prove one local project, one reference agent surface, local runtime state, public MCP tools, write gating, one recorded Run, one evidence link, one structured blocker/status response, and fixture-based Kernel Smoke conformance.

v0.2 User-Facing Harness MVP is the first product MVP: it makes ordinary requests visible as scope, user-owned judgment, evidence, close readiness, final acceptance, and residual-risk boundaries. v0.3 and v0.4 then add detached verification support, Manual QA, stewardship, projection/reconcile depth, recovery, export, release handoff, and the remaining hardening requirements. Early slices can be small as long as they do not redefine the boundary by hiding critical decisions, evidence, or close behavior.

## Non-goals

Harness is not merely a chat workflow, prompt pack, test harness, evaluation harness, dashboard, or hosted agent platform.

Harness can integrate with agent instructions, MCP tools/connectors, skills, reusable workflows, hooks, guardrails, adapters, sidecars, tests, reviews, specs, and isolation layers, but those surfaces are not the source of Harness authority. They help agents read context, call Harness tools, capture evidence, run checks, or enforce/detect boundaries when the connected profile supports it.

Harness authority comes from Core-owned local state and artifact references around work, scope, user-owned judgment, evidence, verification, QA, final acceptance, remaining risk, and close readiness. The exact implementation names for those records live in the Reference docs.

Harness also does not replace the product repository, source control or version control system, test runner, code review process, or user judgment.

Harness does not treat chat history as durable state or generated Markdown reports as the operating record.

Harness does not aim to support every agent surface in the MVP.

Harness does not promise preventive enforcement where a connected agent surface can only cooperate or report after the fact.

Harness does not make a dashboard, team workflow platform, long-term analytics layer, broad connector ecosystem, or automatic parallel execution system part of the MVP.

Harness does not hide sensitive-action permission, QA, verification, final acceptance, or remaining risk behind a single "done" label.
