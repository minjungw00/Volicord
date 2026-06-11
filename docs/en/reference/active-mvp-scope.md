# Active MVP scope reference

This reference is the canonical owner for detailed current MVP scope in the Harness planning documentation.

## Owns / Does not own

This document owns:

- the current MVP capability boundary
- included and excluded product-scope items
- the reserved-value, profile-gated, and later-candidate boundary where it affects active scope
- scope-level guarantee and non-claim wording that other documents should summarize instead of repeating

This document does not own:

- implementation readiness, server-coding handoff, maintainer acceptance status, or build sequencing; see [MVP Plan](../build/mvp-plan.md)
- API method behavior
- schema fields
- storage effects
- security proof
- template bodies
- connector behavior
- later-candidate details

Use this page when deciding whether a capability is part of the current MVP. Other route, build, README, later, and reference documents should link here for the detailed scope list.

## Current repository status

This repository is documentation-only source material for a future Harness Server. Runtime/server implementation has not started unless the maintainer handoff status in `docs/*/build/mvp-plan.md` explicitly says otherwise.

The repository is not the user's Product Repository and not a Harness Runtime Home. These docs do not create runtime state, generated projections, artifacts, evidence records, QA records, acceptance records, close records, residual-risk records, executable fixtures, conformance results, or implementation-complete behavior.

## Included in the active MVP

The current MVP scope is limited to:

- plain-language intake and Task creation
- scope updates
- status and close-readiness review
- prepare-write authorization
- local surface registration
- artifact staging
- run and evidence recording
- focused user judgment capture
- close attempts

The included scope is summarized below. Detail blocks keep the active meaning and owner links visible.

| Scope item | Primary owner |
|---|---|
| Plain-language intake and Task creation | [MVP API](api/mvp-api.md), [Core Model](core-model.md) |
| Scope updates | [MVP API](api/mvp-api.md), [Core Model](core-model.md) |
| Status and close-readiness review | [API State Schemas](api/schema-state.md), [Errors](api/errors.md), [Projection Authority Reference](projection-and-templates.md) |
| Prepare-write authorization | [MVP API](api/mvp-api.md), [Storage Effects](storage-effects.md), [Security](security.md) |
| Local surface registration | [Agent Integration](agent-integration.md), [Surface Recipes](../use/surface-recipes.md), [Security](security.md) |
| Artifact staging | [API Artifact Schemas](api/schema-artifacts.md), [Artifact Storage](storage-artifacts.md), [Storage Effects](storage-effects.md) |
| Run and evidence recording | [MVP API](api/mvp-api.md), [Storage Effects](storage-effects.md), [Core Model](core-model.md) |
| Focused user judgment capture | [Core Model](core-model.md), [API Judgment Schemas](api/schema-judgment.md), [API Value Sets](api/schema-value-sets.md) |
| Close attempts | [MVP API](api/mvp-api.md), [Core Model](core-model.md), [Errors](api/errors.md) |

Plain-language intake and Task creation:
- Active MVP meaning: A local task can be started from plain-language user intent through the active intake path.

Scope updates:
- Active MVP meaning: Task and Change Unit scope can be updated through the active scope-update path.

Status and close-readiness review:
- Active MVP meaning: Current status, evidence sufficiency, known blockers, and close-readiness review can be read.
- Not active: This read does not create a generated projection or runtime artifact.

Prepare-write authorization:
- Active MVP meaning: `harness.prepare_write` can create an owner-scoped, single-use `Write Authorization`.
- Condition: The authorization is for one compatible product-file write attempt.

Local surface registration:
- Active MVP meaning: Registered local surfaces can identify the active surface and its supported capabilities.
- Condition: Those facts are used only for current scope checks.

Artifact staging:
- Active MVP meaning: New artifact bytes can enter active scope through the active staging path.
- Condition: Existing artifacts can be linked only through compatible persisted artifact references.

Run and evidence recording:
- Active MVP meaning: Runs and compact evidence summaries can be recorded for active work.
- Condition: Compatible artifact promotion or linking is included only when the artifact owners allow it.

Focused user judgment capture:
- Active MVP meaning: User-owned judgments can be requested and recorded through the active judgment path.
- Included judgment paths: sensitive-action approval, final acceptance, residual-risk acceptance, and cancellation when the judgment owners allow them.

Close attempts:
- Active MVP meaning: `harness.close_task` can check close readiness and attempt supported close outcomes.
- Required boundary: Evidence, final acceptance, residual-risk, and non-substitution boundaries remain intact.

Read-time status or derived display is active only as part of status and close-readiness review. Persistent projection jobs, generated projection files, and managed projection repair are not active scope.

## Excluded from the active MVP

The active MVP is intentionally narrow. For canonical security non-claims and guarantee levels, see [Security](security.md). While this repository remains documentation-only, the active MVP is also not a runtime implementation.

Later candidates do not create active requirements.

The active MVP excludes:

- native artifact capture and `captured_artifact`
- projection reconcile, persistent projection jobs, and managed block drift repair
- full `Evidence Manifest`
- Manual QA workflow, `qa_gate`, and `verification_gate`
- command, network, and secret access observation
- command/network/secret pre-tool blocking
- preventive guarantees and `isolated` guarantee semantics
- hosted dashboards
- connector marketplaces
- export or handoff formats
- executable fixture runners
- generated conformance artifacts
- operations profiles

Approving a sensitive action does not create active observation or blocking unless an owner promotes that capability. Security and observation boundaries belong to [Security](security.md) and the relevant later-candidate owners.

## Reserved and profile-gated values

Some value names may be reserved values or profile-gated values without being active user-visible capabilities. Reserved or profile-gated guarantee labels do not expand the current MVP scope. Their appearance in examples, schemas, or later-candidate tables does not activate behavior, make a guarantee available, or make the value a default current-MVP value.

Exact guarantee label value entries belong to [API Value Sets](api/schema-value-sets.md). Guarantee semantics, including non-claims for `isolated`, belong to [Security](security.md).

## Later candidates

[Later Candidate Index](../later/index.md) owns deferred candidate names and promotion boundaries. A later candidate remains inert until an owner document promotes a narrow capability with scope, fallback behavior, proof expectations, and paired English/Korean documentation.

Mentioning a later candidate in examples, route text, schema notes, or this reference does not promote it and does not make it an active MVP requirement.

## Current guarantee boundary

The current MVP guarantee boundary is `cooperative` by default. The current MVP does not provide `isolated` guarantee semantics. Reserved or profile-gated guarantee labels do not expand the current MVP scope.

For guarantee semantics, detective wording, promotion rules for `preventive` and `isolated`, and security non-claims, see [Security](security.md). For guarantee label value entries, see [API Value Sets](api/schema-value-sets.md).

`harness.prepare_write` and `Write Authorization` remain product-file write compatibility mechanisms. Their method behavior belongs to [MVP API](api/mvp-api.md), and their Core meaning belongs to [Core Model](core-model.md).

## Documentation-only boundary

Editing this document or any linked reference document does not implement the Harness Server, create runtime state, run conformance, generate projections, stage artifacts, record evidence, accept QA, accept residual risk, close tasks, or authorize server coding.

Implementation readiness and maintainer handoff status stay in [MVP Plan](../build/mvp-plan.md). If that plan does not explicitly authorize runtime work, this repository remains documentation-only.

## Owner links

| Need | Owner |
|---|---|
| Implementation readiness and maintainer handoff status | [MVP Plan](../build/mvp-plan.md) |
| Core authority, Task state, and user-owned judgment boundaries | [Core Model](core-model.md) |
| API method behavior | [MVP API](api/mvp-api.md) |
| API schemas and value sets | [API schema owners in the Reference Index](README.md#api-and-schema-owners) |
| Public errors and close-readiness blocker behavior | [Errors](api/errors.md) |
| Storage records, effects, artifact lifecycle, versioning, and locks | [Storage owners in the Reference Index](README.md#storage-owners) |
| Runtime, repository, and server boundaries | [Runtime Boundaries](runtime-boundaries.md) |
| Security claims and non-claims | [Security](security.md) |
| Surface and connector behavior | [Agent Integration](agent-integration.md), [Surface Recipes](../use/surface-recipes.md) |
| Projection authority and source-state/freshness boundaries | [Projection Authority Reference](projection-and-templates.md) |
| Template bodies for readable displays | [Template Bodies](template-bodies.md) |
| Later candidates and promotion boundaries | [Later Candidate Index](../later/index.md) |
| Product terminology | [Glossary](glossary.md), [Translation Guide](../maintain/translation-guide.md), [docs/terminology-map.yaml](../../terminology-map.yaml) |
