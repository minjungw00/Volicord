# Roadmap

## What this document helps you do

This document collects future Harness candidates that are not yet part of Build-owned staged delivery. It lets readers see possible later directions without treating them as current requirements, authority paths, acceptance paths, QA paths, verification paths, or runtime guarantees.

This is roadmap documentation. It is not an MVP-1 requirement and it is not implemented runtime behavior. It does not authorize runtime/server implementation, generated operational files, executable fixtures, fixture files, projections, databases, or runtime data before documentation acceptance and a separate implementation-planning readiness decision.

## Read this when

- You want to know which ideas are intentionally outside staged delivery.
- You are checking whether a future capability is ready to be promoted into a stage plan.
- You need to keep useful future ideas non-authoritative until an owner explicitly scopes and proves them.

## Before you read

Staged delivery is owned by [Build: MVP-1 User Work Loop](build/mvp-user-work-loop.md). For current handoff and implementation planning, start with [Build: Implementation Overview](build/implementation-overview.md#maintainer-handoff-summary), then check [Implementation decisions needed before server coding](build/mvp-user-work-loop.md#implementation-decisions-needed-before-server-coding), [Build: Engineering Checkpoint](build/engineering-checkpoint.md), and [Build: MVP-1 User Work Loop](build/mvp-user-work-loop.md). For exact contracts, use the Reference docs.

Current stage names are:

- Engineering Checkpoint
- MVP-1 User Work Loop
- Assurance Profile
- Operations Profile
- Roadmap

## Main idea

Roadmap items are candidates, not staged-delivery commitments. Listing an item here does not create authority, conformance, implementation readiness, user acceptance, QA completion, verification satisfaction, residual-risk acceptance, security guarantees, or runtime behavior.

A roadmap candidate stays outside Engineering Checkpoint through Operations Profile unless a future owner document explicitly promotes it. When promoted, it must still preserve user-owned judgment, route durable state and artifacts through Core-owned authority paths, keep evidence/verification/QA/work acceptance/residual risk separate, and use honest security wording for the capability actually proven.

Assurance Profile and Operations Profile have their own later buckets. Roadmap is for expansion candidates such as dashboard, hosted workflows, team workflows, broader connectors, automation, metrics, orchestration, remote/shared profiles, and higher guarantee claims that have not been promoted.

## Roadmap Boundary

This document does not own kernel invariants, public MCP schemas, storage profiles, fixture profile exits, stage-required API surface, operator surface, or implementation checklists. Those details belong in Build and Reference owner docs.

Roadmap candidates may be useful as read-only displays, metadata, artifact candidates, fixture candidates, prototypes, or planning notes only when the relevant owner docs allow that limited use. They must not become a shortcut around Core-owned state, `task_events`, artifact refs, user judgments, Manual QA, Eval records, work acceptance, residual-risk acceptance, projection freshness, close readiness, or implementation readiness.

Any durable artifact registration, evidence attachment, state change, gate result, QA record, verification result, acceptance record, or residual-risk record must go through an existing Core/MCP owner path or a future promoted owner contract. Being listed here is never an authority path.

## Promotion Criteria

A candidate cannot enter staged delivery unless a future owner decision defines and proves all of the following:

- an explicit future-version or future-stage owner decision with narrow scope
- preservation of user-owned judgment, including work acceptance, residual-risk acceptance, product judgment, material technical judgment, and QA waiver judgment where relevant
- no bypass of Core authority, Core-owned state, artifact refs, gate semantics, close semantics, or owner-record lifecycles
- stage-appropriate security guarantee wording that matches the Security Reference; preventive or isolation claims require a proven covered mechanism and fallback
- clear evidence, verification, QA, work acceptance, and residual-risk implications, including what the candidate can assist and what it must not satisfy
- exact contracts and owner-doc placement for new API, storage, artifact, projection, fixture, operator, connector, or UI behavior
- redaction, secret/PII handling, test-environment, and artifact-retention rules when the candidate captures or stores runtime surfaces
- a fixture or conformance target for the promoted behavior
- fallback behavior for unsupported surfaces, missing capability, unavailable tools, stale data, or partial capture
- no dependency on treating projections, dashboards, indexes, connector output, or generated documents as canonical state
- an early-stage inflation check showing that the candidate does not add requirements to Engineering Checkpoint through Operations Profile or make unsupported surfaces fail earlier stages by default

If any criterion is missing, the item remains a roadmap candidate.

## Candidate Inventory

These examples describe candidate areas only. They do not add stage requirements and do not relax the promotion criteria above.

| Candidate area | Boundary before promotion |
|---|---|
| Dashboard, hosted workflows, artifact dashboard, richer cards, richer visualizations | May display Core-derived state or projections. Must not become authority, implementation readiness, close readiness, work acceptance, residual-risk acceptance, QA completion, verification satisfaction, projection freshness, workflow routing, or metric interpretation. |
| Browser capture automation | May collect screenshots, console logs, network traces, accessibility snapshots, and workflow recordings as artifact candidates. Must not replace human Manual QA judgment, work acceptance, profile-required detached verification, redaction policy, or the existing Manual QA/artifact path. |
| Cross-surface verification | May route verification bundles to another agent surface or evaluator environment after promotion. Must not record an Eval, satisfy verification, raise assurance, accept a result, or close a Task without Core-owned return records and any independence semantics required by the active profile. |
| Broader connectors, connector marketplace, hosted UI, hosted/remote runtime | May extend surfaces later. Must not widen MCP exposure, create authority, bypass Core, replace the local reference proof, imply remote/runtime guarantees, or make unsupported surfaces fail earlier stages by default. |
| Native hooks, preventive guard expansion, advanced sidecar watcher | May strengthen guard display, artifact capture, command observation, or file-write observation where a surface proves the mechanism. Must not claim pre-execution blocking, OS isolation, tamper-proof storage, or arbitrary-tool control by label alone. Observations route through Core records, validators, artifact registration, or reconcile before affecting state. |
| Context Index, local derived metrics, long-term metrics | May provide read-only retrieval or diagnostics. Must not authorize writes, create Write Authorization, resolve user judgments, grant Approval, satisfy gates, create evidence, record verification or QA, refresh projections, declare readiness, accept risk, accept results, upgrade assurance, or close Tasks. |
| Team workflows, permissions, shared profiles, export/import, orchestration, parallel lanes | May coordinate future work. Must not become required for staged delivery, single-project local authority, user acceptance, QA, verification, residual-risk acceptance, or close. |
| Advanced exports, release/deployment/canary/rollback/merge/production-monitoring automation | May become future integration work. Release handoff remains a report/export boundary unless owner docs promote more; deployment, merge, rollback, and production authority stay external until explicitly scoped and proven. |
| Advanced validators and language or interface checks | May become future stewardship or diagnostic coverage. Must not become early-stage fixture failure, acceptance, QA, or close criteria until owner docs define the exact policy, severity, waiver, and fixture behavior. |

Use [Build: MVP-1 User Work Loop](build/mvp-user-work-loop.md#later-profiles-not-to-build-yet) for the staged-delivery boundary, and use this page only to track candidates that remain outside staged delivery until promoted.
