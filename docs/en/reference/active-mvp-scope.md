# Active MVP scope reference

This reference is the canonical owner for detailed current MVP scope in the Harness planning documentation.

## What this document owns

This document owns the current MVP capability boundary, the included and excluded scope at the product-scope level, the profile-gated and later-candidate boundary as it affects active scope, and the scope-level guarantee and non-claim wording that other documents should summarize instead of repeating.

Use this page when deciding whether a capability is part of the current MVP. Other route, build, README, later, and reference documents should link here for the detailed scope list.

## What this document does not own

This document does not own implementation readiness, server-coding handoff, maintainer acceptance status, or build sequencing; those belong in [MVP Plan](../build/mvp-plan.md).

It also does not own API method behavior, schema fields, storage effects, security proof, template bodies, connector behavior, or later-candidate details. Those details belong to the owner documents linked below.

## Current repository status

This repository is documentation-only source material for a future Harness Server. Runtime/server implementation has not started unless the maintainer handoff status in `docs/*/build/mvp-plan.md` explicitly says otherwise.

The repository is not the user's Product Repository and not a Harness Runtime Home. These docs do not create runtime state, generated projections, artifacts, evidence records, QA records, acceptance records, close records, residual-risk records, executable fixtures, conformance results, or implementation-complete behavior.

## Included in the active MVP

The current MVP scope is limited to plain-language intake and Task creation, scope updates, status and close-readiness review, prepare-write authorization, local surface registration, artifact staging, run and evidence recording, focused user judgment capture, and close attempts.

The included scope is:

| Scope item | Active MVP meaning | Primary owner |
|---|---|---|
| Plain-language intake and Task creation | A local task can be started from plain-language user intent through the active intake path. | [MVP API](api/mvp-api.md), [Core Model](core-model.md) |
| Scope updates | Task and Change Unit scope can be updated through the active scope-update path. | [MVP API](api/mvp-api.md), [Core Model](core-model.md) |
| Status and close-readiness review | Current status, evidence sufficiency, known blockers, and close-readiness review can be read without creating a generated projection or runtime artifact. | [API State Schemas](api/schema-state.md), [Errors](api/errors.md), [Projection Authority Reference](projection-and-templates.md) |
| Prepare-write authorization | `harness.prepare_write` can create an owner-scoped, single-use `Write Authorization` for a compatible product-file write attempt. | [MVP API](api/mvp-api.md), [Storage Effects](storage-effects.md), [Security](security.md) |
| Local surface registration | Registered local surfaces can identify the active surface and its supported capabilities for current scope checks. | [Agent Integration](agent-integration.md), [Surface Recipes](../use/surface-recipes.md), [Security](security.md) |
| Artifact staging | New artifact bytes can enter active scope only through the active staging path, and existing artifacts can be linked only through compatible persisted artifact references. | [API Artifact Schemas](api/schema-artifacts.md), [Artifact Storage](storage-artifacts.md), [Storage Effects](storage-effects.md) |
| Run and evidence recording | Runs and compact evidence summaries can be recorded for active work, including compatible artifact promotion or linking when the artifact owners allow it. | [MVP API](api/mvp-api.md), [Storage Effects](storage-effects.md), [Core Model](core-model.md) |
| Focused user judgment capture | User-owned judgments can be requested and recorded through the active judgment path, including sensitive-action approval, final acceptance, residual-risk acceptance, and cancellation when the judgment owners allow them. | [Core Model](core-model.md), [API Judgment Schemas](api/schema-judgment.md), [API Value Sets](api/schema-value-sets.md) |
| Close attempts | `harness.close_task` can check close readiness and attempt supported close outcomes while preserving evidence, final acceptance, residual-risk, and non-substitution boundaries. | [MVP API](api/mvp-api.md), [Core Model](core-model.md), [Errors](api/errors.md) |

Read-time status or derived display is active only as part of status and close-readiness review. Persistent projection jobs, generated projection files, and managed projection repair are not active scope.

## Excluded from the active MVP

The active MVP is not an OS permission control system. It is not a sandbox. It is not tamper-proof storage. It is not a full security isolation layer. It is not a runtime implementation while this repository remains documentation-only. Later candidates do not create active requirements.

The active MVP excludes native artifact capture, `captured_artifact`, projection reconcile, persistent projection jobs, managed block drift repair, full `Evidence Manifest`, Manual QA workflow, `qa_gate`, `verification_gate`, command observation, network observation, secret access observation, command/network/secret pre-tool blocking, preventive guarantees, isolated guarantees, hosted dashboards, connector marketplaces, export or handoff formats, executable fixture runners, generated conformance artifacts, and operations profiles.

Approving a command, dependency change, host, network access, secret handle, deployment, destructive action, or system access does not mean Harness can observe or block that action in the current MVP.

## Profile-gated values

Some value names may be reserved or profile-gated without being active user-visible capabilities. Reserved or profile-gated names do not extend the current MVP by appearing in examples, schemas, or later-candidate tables.

Exact value-set placement belongs to [API Value Sets](api/schema-value-sets.md). Security and guarantee meaning belongs to [Security](security.md).

## Later candidates

[Later Candidate Index](../later/index.md) owns deferred candidate names and promotion boundaries. A later candidate remains inert until an owner document promotes a narrow capability with scope, fallback behavior, proof expectations, and paired English/Korean documentation.

Mentioning a later candidate in examples, route text, schema notes, or this reference does not promote it and does not make it an active MVP requirement.

## Current guarantee boundary

The current MVP guarantee boundary is cooperative by default. Harness can record and display authority boundaries, compatibility checks, evidence summaries, user-owned judgment, and close-readiness findings, but that display is not OS enforcement, arbitrary-tool isolation, tamper-proof storage, or broad security isolation.

Detective wording is allowed only for the covered observable scope after the relevant active capability check has passed. Preventive or isolated wording requires a separately documented and proven mechanism before it can be active.

`harness.prepare_write` and `Write Authorization` are cooperative product-file write compatibility mechanisms. They do not grant operating-system permission, sandbox arbitrary tools, prove all tools were blocked before action, or make local files tamper-proof.

## Documentation-only boundary

Editing this document or any linked reference document does not implement the Harness Server, create runtime state, run conformance, generate projections, stage artifacts, record evidence, accept QA, accept residual risk, close tasks, or authorize server coding.

Implementation readiness and maintainer handoff status stay in [MVP Plan](../build/mvp-plan.md). If that plan does not explicitly authorize runtime work, this repository remains documentation-only.

## Owner links

| Need | Owner |
|---|---|
| Implementation readiness and maintainer handoff status | [MVP Plan](../build/mvp-plan.md) |
| Core authority, Task state, and user-owned judgment boundaries | [Core Model](core-model.md) |
| API method behavior | [MVP API](api/mvp-api.md) |
| API schemas and value sets | [API Schema Core](api/schema-core.md), [API State Schemas](api/schema-state.md), [API Artifact Schemas](api/schema-artifacts.md), [API Judgment Schemas](api/schema-judgment.md), [API Value Sets](api/schema-value-sets.md) |
| Public errors and close-readiness blocker behavior | [Errors](api/errors.md) |
| Storage records, effects, artifact lifecycle, versioning, and locks | [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), [Artifact Storage](storage-artifacts.md), [Storage Versioning](storage-versioning.md) |
| Runtime, repository, and server boundaries | [Runtime Boundaries](runtime-boundaries.md) |
| Security claims and non-claims | [Security](security.md) |
| Surface and connector behavior | [Agent Integration](agent-integration.md), [Surface Recipes](../use/surface-recipes.md) |
| Projection authority and source-state/freshness boundaries | [Projection Authority Reference](projection-and-templates.md) |
| Template bodies for readable displays | [Template Bodies](template-bodies.md) |
| Later candidates and promotion boundaries | [Later Candidate Index](../later/index.md) |
| Product terminology | [Glossary](glossary.md), [Translation Guide](../maintain/translation-guide.md), [docs/terminology-map.yaml](../../terminology-map.yaml) |
