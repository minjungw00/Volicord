# Purpose and Principles

## What this document helps you do

This document explains why Harness exists, who it is for, what it values, how it thinks about common AI-assisted development failures, and what belongs inside the MVP boundary.

This is Learn documentation. It does not authorize runtime/server implementation, generated operational files, executable fixtures, or runtime data before maintainers explicitly accept the documentation set for first runtime-batch planning. The first implementation/proof target remains Kernel Smoke; Agency-Hardened MVP and post-MVP automation stay out of scope unless their owner docs promote and prove them.

## Read this when

Read this when you want the values and boundaries behind Harness before reading strict contracts or implementation plans.

## Before you read

[Overview](overview.md) is useful first if Harness is new to you. No implementation background is required.

## Main idea

Harness is a local work ledger and judgment router for AI-assisted product work. It records what may change, who must decide, what evidence exists, what risk remains, and whether the work can close.

The reason it works as a ledger is the agency-preserving local authority kernel principle: durable work facts stay in local state, artifact-backed evidence refs, and readable projections, while user-owned product and material technical judgment stays with the user.

## Purpose

Harness exists to make AI-assisted development followable without taking strategic judgment away from the user.

The user should be able to begin in ordinary language. The agent should be able to clarify, propose, implement, check, and report. But the important facts of the work should not live only in the chat transcript. Harness keeps those facts in local state, durable evidence, and readable projections so the task can be resumed, checked, reconciled, and closed from current state.

Harness is not ceremony for its own sake. It maintains a local work ledger for work that would otherwise become blurry, and it routes judgment when the work needs a user decision before it can continue or close.

Harness keeps that operating record around task state, Change Unit scope, user judgment, Write Authorization, evidence, verification, QA, Acceptance, Residual Risk, and close. The point is not to replace the user's tools, source control, tests, code review, or conversation, but to make the work followable from current local state.

## What Harness is not

Harness is not:

- a prompt pack
- a replacement for source control, tests, code review, or user judgment
- MCP itself
- a broad hosted agent platform

Harness may use agent instructions, MCP tools, hooks, sidecars, tests, reviews, and specs as useful inputs or surfaces. They do not become Harness authority just because they are nearby.

Harness differs from nearby pieces because authority lives in the local work record and user-owned judgment stays with the user. AGENTS.md / agent rules can guide agent behavior, MCP can expose tools and resources, specs can describe intent, hooks / sidecars can observe or enforce within their guarantee level, and tests / code review can check product work. Harness uses those surfaces as inputs or evidence, but it does not treat them as the ledger or the decision owner. For the detailed comparison table, use the [English documentation entrypoint](../README.md#comparison).

## Who it is for

Harness is for developers using AI agents to modify, verify, or explain product code. It is also for solo maintainers who need reliable resume behavior, technical leads who want local records of sensitive-action Approval and acceptance, connector authors integrating an agent surface with Harness, and documentation authors maintaining the current Harness model.

The shared need is simple: people want the speed of agent-assisted work without losing the ability to understand what happened, what is allowed, what was checked, and what still needs judgment.

## Core values

Harness keeps operational state and evidence local. The durable work record should not depend on a remote chat transcript.

Harness makes boundaries explicit. Scope, sensitive-action Approval, decisions, evidence, verification, Manual QA, acceptance, and residual risk are different questions, so the system records them separately.

Harness is honest about assurance. It should say what changed, what was checked, how independent that check was, what remains risky, and what decision is needed, instead of treating every review as equally strong.

Harness preserves strategic agency. The user keeps judgment over goals, scope, design direction, product trade-offs, material technical trade-offs, codebase stewardship, QA, acceptance, and residual-risk acceptance.

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

Second, Harness state is the operating record. It stores the task, scope, decisions, sensitive-action Approvals, evidence refs, verification, QA, acceptance, and residual risk needed to reason about the work.

Third, readable documents are projections and proposal areas. They help humans inspect and edit the story of the work, but accepted changes enter the operating record only through reconcile or another state-changing action. Projection freshness is a display fact, not a substitute for current state, evidence, tests, review, or acceptance.

## Failure model, rewritten as reader-facing problems

Without Harness, the work journey can disappear into chat. A later reader cannot tell what the current state is, what the next action should be, which decisions are open, or what evidence supports the result. Harness responds by recording task state, change scope, decisions, evidence, QA, acceptance, residual risk, and close status, then generating readable views from those records.

Without Harness, scope and Approval can drift. A small request can become a broad rewrite, or a sensitive action can happen without explicit sensitive-action Approval. Harness responds by requiring product writes to stay inside scoped Change Units and by requiring sensitive-action Approval for sensitive categories.

Without Harness, evidence can be too weak or too temporary. Tests, logs, screenshots, and run summaries can vanish with the session or remain as unstructured claims. Harness responds by tying evidence to the task and storing durable artifacts where evidence is required.

Without Harness, verification can overstate independence. The same agent that made a change can review its own work, and the system may treat that as independent assurance. Harness responds by separating self-checks from detached verification.

Without Harness, user-owned product judgment or material technical judgment can move from the user to the agent without anyone noticing. Design direction, trade-offs, codebase stewardship, QA judgment, acceptance, and risk acceptance can be hidden inside implementation. Harness responds with Decision Packets when user-owned judgment blocks progress.

Without Harness, local completion can hide long-term product damage. A task may pass tests while blurring domain language, crossing module boundaries, weakening interfaces, or leaving follow-up risk unnamed. Harness responds by keeping codebase stewardship, design trade-offs, QA findings, and residual risk visible in the work journey.

Without Harness, different judgments can collapse into one vague "done." Approval, verification, Manual QA, acceptance, and residual-risk acceptance answer different questions. Harness responds by recording them separately.

Without Harness, generated documents can be mistaken for operational truth. A stale report or edited note can look authoritative. Harness responds by treating Markdown as projection and by requiring reconcile before human edits become state.

## MVP boundary in plain language

The MVP is a proof of the local kernel and agency model, not a broad platform.

It should prove one local project, one reference agent surface, local runtime state, durable artifacts, public MCP tools, write gating, evidence, detached verification support, Manual QA, acceptance, projections, reconcile, recovery, export, and fixture-based conformance.

The MVP may be delivered in stages, but the final MVP still needs to prove the same authority and agency requirements. Early slices can be small as long as they do not redefine the boundary by hiding critical decisions, evidence, or close behavior.

## Non-goals

Harness is not merely a chat workflow, prompt pack, test harness, or evaluation harness.

Harness can integrate with MCP tools/connectors, hooks, guardrails, adapters, sidecars, and isolation layers, but those surfaces are not the source of Harness authority. They help agents read context, call Harness tools, capture evidence, or enforce/detect boundaries when the connected profile supports it.

Harness authority comes from Core and canonical local state around Task and Change Unit scope, Decision Packets, Approval, Write Authorization, evidence, verification, QA, acceptance, residual risk, and close.

Harness also does not replace the product repository, source control or version control system, test runner, code review process, or user judgment.

Harness does not treat chat history as durable state or generated Markdown reports as the operating record.

Harness does not aim to support every agent surface in the MVP.

Harness does not promise preventive enforcement where a connected agent surface can only cooperate or report after the fact.

Harness does not make a dashboard, team workflow platform, long-term analytics layer, broad connector ecosystem, or automatic parallel execution system part of the MVP.

Harness does not hide sensitive-action Approval, QA, verification, acceptance, or remaining risk behind a single "done" label.
