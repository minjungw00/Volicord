# Concepts

## What this document helps you do

This document introduces the smallest concept set you need before reading Harness reference specs. Each concept starts with a plain example, then gives a tighter definition.

The kernel, runtime, MCP API, and document projection references now live in the reference path.

## Read this when

Read this when Harness terms are starting to appear in examples, status summaries, or reference specs and you want the smallest useful vocabulary.

## Before you read

[Overview](overview.md) is recommended first. No schema or implementation knowledge is required.

## Main idea

Harness vocabulary names a small work journey: request, scope, judgment, support, checks, acceptance, remaining risk, and close.

## Gate display groups

When Harness status is shown to users, the many internal gate details should usually appear as four readable display groups:

- Scope: what may change.
- Judgment: what the user must decide.
- Evidence: what supports completion claims.
- Close Readiness: verification, Manual QA, acceptance, residual risk, and close blockers.

These groups are a reading aid, not a replacement for the kernel gate taxonomy. They do not create schema fields, gate values, recompute inputs, authority paths, or close rules. Strict gate behavior stays in [Kernel Reference](../reference/kernel.md#gates), public API behavior stays in [MCP API and Schemas](../reference/mcp-api-and-schemas.md), and projection rendering rules stay in [Document Projection Reference](../reference/document-projection.md).

## The smallest concept set

Harness is easiest to understand if you start with the work journey:

- A user asks for a Task.
- A Shared Design records the goal, scope, assumptions, and first safe shape when the work needs shaping.
- Product writes happen inside a Change Unit.
- Important claims need Evidence.
- Sensitive actions need Approval, and product writes need Write Authorization.
- Checks create Verification, and human inspection may create Manual QA.
- The user gives Acceptance when the task path requires it.
- Remaining uncertainty is Residual Risk.
- Readable documents are Projections, and human edits become state through Reconcile.

These concepts are intentionally small here. Strict kernel definitions live in [Kernel Reference](../reference/kernel.md), public API definitions live in [MCP API and Schemas](../reference/mcp-api-and-schemas.md), and projection rules live in [Document Projection Reference](../reference/document-projection.md). Some concepts span several references; the notes below name the split without copying the owner maps.

## Task

A user says, "Add email login and show a helpful error when the password is wrong." The chat may include many turns, but the work still needs one durable unit that says what the user wants done and what state the work is in.

A Task is the user value unit: the thing the user wants completed, answered, investigated, or decided. Harness uses the Task to keep status, next action, blockers, evidence, QA, acceptance, and close behavior connected.

Reference: [Kernel Reference](../reference/kernel.md).

## Shared Design

A user asks for "better onboarding." Before implementation hardens into a plan, the agent and user need a shared understanding of the goal, non-goals, acceptance criteria, assumptions, affected screens, domain terms, module or interface impact, and first safe slice.

Shared Design is that recorded understanding. It helps turn blurry work into a safe first Change Unit, but it is not sensitive-action Approval, Write Authorization, final acceptance, QA judgment, or residual-risk acceptance.

If shaping reveals a choice the user owns, such as public API direction, domain-language meaning, module boundary movement, architecture direction, or a known-risk waiver, that choice routes through a Decision Packet. Shared Design can point to the decision; it does not decide it by itself.

References: [Shared Design](../reference/glossary.md#shared-design) and [Design Quality Policies](../reference/design-quality-policies.md#shared-design-shared_design).

## Change Unit

The email login task may require changes to the login form, one API call, and session handling. That is a bounded slice. If the work suddenly starts rewriting the whole authentication system, the scope has changed and should be visible.

A Change Unit is the bounded product-write scope for a Task. It names the part of the product that may change so the agent, user, and Harness can tell whether a write is inside the agreed work.

For a tiny docs, copy, or focused test edit, the Change Unit may be generated from the request and stay very small. The important point is still the same: direct work can be light, but product writes still happen inside an active scope.

Reference: [Kernel Reference](../reference/kernel.md).

## Autonomy Boundary

Inside the email login Change Unit, the agent might decide to reuse an existing helper, split a private function, or add a focused test without asking again. That is different from deciding whether to use JWTs, change a public API, or accept a security trade-off.

An Autonomy Boundary describes the judgment the agent may exercise inside the Change Unit. Change Unit scope answers "what work surface may change?" Autonomy Boundary answers "what choices may the agent make there without another user decision?" Neither one is Write Authorization.

Reference: [Kernel Reference](../reference/kernel.md).

## Decision Packet

The agent finds several reasonable failed-login choices: inline message, toast, or modal/layer for the interaction; generic, specific, or hybrid wording for the copy. Another task might need a choice between session cookie, JWT, and social login, or between a compatible public API extension and a breaking cleanup. The agent should not quietly choose the product, security, compatibility, or maintenance trade-off if that choice blocks progress.

A Decision Packet records a specific user-owned decision when that judgment blocks progress, write, close, waiver, acceptance, residual-risk acceptance, product direction, material technical direction, scope, design trade-off, stewardship judgment, or public commitment. It should ask the user to decide the named issue, not to give broad approval. Exact record quality and public fields live in the references below.

References: [Decision Packet](../reference/kernel.md#decision-packet), [Decision Gate](../reference/kernel.md#decision-gate), and [`harness.request_user_decision`](../reference/mcp-api-and-schemas.md#harnessrequest_user_decision).

## Evidence

The agent says the login flow works. Useful support might include the diff, the test output, a screenshot of the error state, and a note about the manual browser check. Without those records, "works" is only a chat claim.

Evidence is recorded support for claims about the work. It can include diffs, logs, tests, screenshots, run summaries, evaluation records, Manual QA records, or other durable artifacts tied to the task.

This is the plain concept. Strict behavior lives across [Kernel Reference](../reference/kernel.md) for Evidence Gate, Evidence Manifest, and evidence sufficiency; [MCP API and Schemas](../reference/mcp-api-and-schemas.md) and [Storage And DDL](../reference/storage-and-ddl.md) for artifact registration, `ArtifactRef`, and storage integrity; and [Operations and Conformance Reference](../reference/operations-and-conformance.md) for conformance proof.

## Approval

The task needs a new dependency, a network call, or access to a sensitive file. Even if the change is useful, the user may need to approve that category of action before it proceeds.

Approval answers whether a sensitive action may proceed inside a defined scope. Approval is not the same as accepting the final result, choosing a design trade-off, or accepting residual risk.

For example, sensitive-action Approval to install a package is not the same as deciding that package is the architecture direction. Sensitive-action Approval to access a secret is not permission to expose secret values in Evidence, projections, exports, logs, screenshots, or summaries.

Reference: [Kernel Reference](../reference/kernel.md).

## Write Authorization

The agent is ready to edit the login code. Harness needs to check whether there is an active Change Unit, whether the target path is in scope, whether any required sensitive-action Approval exists, and whether any blocking decision must be resolved first.

Write Authorization is the Harness decision that a product write may proceed now. In the current spec language, `prepare_write` is the product-write decision point.

Strict behavior lives across [Kernel Reference](../reference/kernel.md) for write-gate semantics and state effects, [MCP API and Schemas](../reference/mcp-api-and-schemas.md) for the public `prepare_write` shape, and [Agent Integration Reference](../reference/agent-integration.md) for connected-surface behavior.

## Verification

The agent runs tests after editing the login flow. That is useful, but it is not the same as an independent check by another session, tool path, or evaluator bundle.

Verification records how the result was checked and how independent that check was. Harness separates self-checks from detached verification so confidence is not confused with independence.

Strict behavior lives across [Kernel Reference](../reference/kernel.md) for verification gate, assurance, and verification independence semantics; [MCP API and Schemas](../reference/mcp-api-and-schemas.md) for Eval and verification tool schemas; and [Operations and Conformance Reference](../reference/operations-and-conformance.md) for conformance fixtures.

## Manual QA

A test can pass while the error message is confusing, clipped on mobile, or visually inconsistent. A human may need to look at the result and record what they saw.

Manual QA is human inspection of the experiential result where that matters, especially UI, UX, copy, accessibility, visual output, product taste, and other judgment-heavy outcomes.

If Manual QA is waived, the waiver should still name the skipped surface, accepted risk, follow-up, and close impact. A waiver is a recorded judgment, not a test result.

Strict behavior lives across [Design Quality Policies](../reference/design-quality-policies.md) for Manual QA requirements and waivers; [Kernel Reference](../reference/kernel.md) for QA Gate semantics; [MCP API and Schemas](../reference/mcp-api-and-schemas.md) for Manual QA record and tool shape; and [Operations and Conformance Reference](../reference/operations-and-conformance.md) for conformance proof.

## Acceptance

The work may be implemented and checked, but the user still needs to decide whether the result satisfies the request and whether the remaining trade-offs are acceptable.

Acceptance is the user's judgment that the task result can be accepted. It is separate from Approval, Verification, Manual QA, and Residual Risk.

Reference: [Kernel Reference](../reference/kernel.md).

## Residual Risk

The login flow is done, but rate limiting was not added in this task, or the detached verifier could not run in the current environment. That remaining uncertainty should be named instead of disappearing behind "done."

Residual Risk is known remaining uncertainty, limitation, or trade-off after the work. When task close depends on accepting that risk, the user's residual-risk acceptance must be explicit.

Reference: [Kernel Reference](../reference/kernel.md).

## Projection

Harness can generate a readable task report or Journey Card from recorded state. A user can read it quickly, but editing that report should not silently rewrite the operating record.

A Projection is a human-readable rendering of Harness state records and artifact references. Markdown reports, Journey Cards, and Journey Spine views are projections; they display state but do not replace it.

Strict behavior lives across [Document Projection Reference](../reference/document-projection.md) for projection authority, managed blocks, and freshness; [MCP API and Schemas](../reference/mcp-api-and-schemas.md) for `ProjectionKind` and projection refs; and [Template Reference](../reference/templates/README.md) for rendered template bodies and display card shapes.

## Reconcile

A user edits a notes section in a generated report and proposes a different next action. Harness should not pretend the operational state changed just because a Markdown line changed. The proposal needs a deliberate path into state.

Reconcile is the explicit path for turning human-editable notes, proposals, or projection drift into accepted state changes, rejected proposals, notes, decisions, or deferred items.

Strict behavior lives across [Document Projection Reference](../reference/document-projection.md) for human-editable input, drift, and reconcile items; [MCP API and Schemas](../reference/mcp-api-and-schemas.md) for public reconcile decision shapes; and the relevant owner reference for any state record a reconcile outcome changes.
