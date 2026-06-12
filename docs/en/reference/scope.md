# Scope reference

This reference owns the current Harness MVP capability boundary.

## Owns / Does Not Own

This document owns:

- the current MVP capability boundary
- included and excluded product-scope items
- reserved and profile-gated value boundaries where they affect active scope
- scope-level guarantee and non-claim wording that other documents should summarize instead of repeating

This document does not own:

- implementation sequencing; see the [Implementation Guide](../build/implementation-guide.md)
- API method behavior
- schema fields
- storage effects
- security proof
- template bodies
- connector behavior
- detailed specifications for out-of-scope capabilities

Use this page when deciding whether a capability is part of the current MVP. Route, build, README, and reference documents should link here for the scope boundary instead of repeating the detailed list.

## Included in the Current MVP

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

| Scope item | Primary owner |
|---|---|
| Plain-language intake and Task creation | [Intake method](api/method-intake.md), [Core Model](core-model.md) |
| Scope updates | [Update-scope method](api/method-update-scope.md), [Core Model](core-model.md) |
| Status review | [Status method](api/method-status.md), [API State Schemas](api/schema-state.md), [Projection Authority Reference](projection-and-templates.md) |
| Close-readiness review | [Close-task method](api/method-close-task.md), [API State Schemas](api/schema-state.md), [Errors](api/errors.md) |
| Prepare-write authorization | [Prepare-write method](api/method-prepare-write.md), [Storage Effects](storage-effects.md), [Security](security.md) |
| Local surface registration | [Agent Integration](agent-integration.md), [Surface Recipes](../use/surface-recipes.md), [Security](security.md) |
| Artifact staging | See [Artifact staging owners](#artifact-staging-owners). |
| Run and evidence recording | [Record-run method](api/method-record-run.md), [Storage Effects](storage-effects.md), [Core Model](core-model.md) |
| Focused user judgment capture | See [User judgment owners](#user-judgment-owners). |
| Close attempts | [Close-task method](api/method-close-task.md), [Core Model](core-model.md), [Errors](api/errors.md) |

Plain-language intake and Task creation:
- Active meaning: A local task can be started from plain-language user intent through the active intake path.

Scope updates:
- Active meaning: Task and Change Unit scope can be updated through the active scope-update path.

Status and close-readiness review:
- Active meaning: Current status, evidence sufficiency, known blockers, and close-readiness review can be read.
- Not included: The read does not create a generated projection or runtime artifact.

Prepare-write authorization:
- Active meaning: `harness.prepare_write` can create an owner-scoped, single-use `Write Authorization`.
- Condition: The authorization is for one compatible product-file write attempt.

Local surface registration:
- Active meaning: Registered local surfaces can identify the active surface and its supported capabilities.
- Condition: Those facts are used only for current scope checks.

<a id="artifact-staging-owners"></a>
Artifact staging owners:
- Method behavior: [Stage-artifact method](api/method-stage-artifact.md).
- API shapes: [API Artifact Schemas](api/schema-artifacts.md).
- Lifecycle and storage effects: [Artifact Storage](storage-artifacts.md) and [Storage Effects](storage-effects.md).

Artifact staging:
- Active meaning: New artifact bytes can enter active scope through the active staging path.
- Condition: Existing artifacts can be linked only through compatible persisted artifact references.

Run and evidence recording:
- Active meaning: Runs and compact evidence summaries can be recorded for active work.
- Condition: Compatible artifact promotion or linking is included only when the artifact owners allow it.

<a id="user-judgment-owners"></a>
User judgment owners:
- Method behavior: [User-judgment methods](api/method-user-judgment.md).
- Product meaning: [Core Model](core-model.md).
- API shapes and values: [API Judgment Schemas](api/schema-judgment.md) and [API Value Sets](api/schema-value-sets.md).

Focused user judgment capture:
- Active meaning: User-owned judgments can be requested and recorded through the active judgment path.
- Included judgment paths: sensitive-action approval, final acceptance, residual-risk acceptance, and cancellation when the judgment owners allow them.

Close attempts:
- Active meaning: `harness.close_task` can check close readiness and attempt supported close outcomes.
- Required boundary: Evidence, final acceptance, residual-risk, and non-substitution boundaries remain intact.

Status display boundary:
- Current scope: Read-time status or derived display is active only as part of status and close-readiness review.
- Not included: Persistent projection jobs, generated projection files, and managed projection repair.

## Excluded from the Current MVP

The current MVP is intentionally narrow.

Not included:

- native artifact capture and `captured_artifact`
- projection reconcile, persistent projection jobs, and managed block drift repair
- full `Evidence Manifest`
- Manual QA workflow, `qa_gate`, and `verification_gate`
- command, network, and secret access observation
- command, network, and secret pre-tool blocking
- preventive guarantees and `isolated` guarantee semantics
- hosted dashboards
- connector marketplaces
- export or handoff formats
- executable fixture runners
- generated conformance artifacts
- operations profiles

Does not imply:
- Excluded capabilities are not active requirements.
- Approving a sensitive action does not create active observation or blocking unless an owner promotes that capability.

Owner links:
- Security non-claims, guarantee levels, and observation boundaries: [Security](security.md).
- Value names and reserved values: [API Value Sets](api/schema-value-sets.md).

## Reserved and Profile-Gated Values

Some value names may be reserved values or profile-gated values without being active user-visible capabilities.

Does not imply:
- Reserved or profile-gated guarantee labels do not expand the current MVP scope.
- Appearance in examples or schemas does not activate behavior.
- Appearance in a value set does not make a guarantee available.
- Appearance in a value set does not make the value a default current-MVP value.

Owner links:
- Exact guarantee label value entries: [API Value Sets](api/schema-value-sets.md).
- Guarantee semantics, including non-claims for `isolated`: [Security](security.md).

## Out-of-Scope Capability Activation

An out-of-scope capability remains inactive until this Scope reference and the affected owner documents define a narrow active contract with fallback behavior, proof expectations, and paired English/Korean documentation.

Does not imply:
- Mentioning an excluded or reserved capability in examples, route text, schema notes, or this reference does not promote it and does not make it a current MVP requirement.

## Current Guarantee Boundary

Current scope:
- The current MVP guarantee boundary is `cooperative` by default.
- `harness.prepare_write` and `Write Authorization` remain product-file write compatibility mechanisms.

Not included:
- The current MVP does not provide `isolated` guarantee semantics.

Does not imply:
- Reserved or profile-gated guarantee labels do not expand the current MVP scope.

Owner links:
- Guarantee semantics, detective wording, promotion rules for `preventive` and `isolated`, and security non-claims: [Security](security.md).
- Guarantee label value entries: [API Value Sets](api/schema-value-sets.md).
- Method behavior: [Prepare-write method](api/method-prepare-write.md), routed from [API Methods](api/methods.md).
- Core meaning: [Core Model](core-model.md).

## Documentation Tree Boundary

The documentation tree stores maintained product and system documentation.

It does not store:
- runtime state
- generated projections
- generated artifacts
- evidence records
- QA records
- acceptance records
- close records
- residual-risk records
- executable fixtures
- conformance results
- product implementation outputs

Owner links:
- Implementation routing: [Implementation Guide](../build/implementation-guide.md).
- Runtime, repository, and server boundaries: [Runtime Boundaries](runtime-boundaries.md).

## Owner Links

| Need | Owner |
|---|---|
| Implementation routing | [Implementation Guide](../build/implementation-guide.md) |
| Core authority, Task state, and user-owned judgment boundaries | [Core Model](core-model.md) |
| API method behavior | [API Methods](api/methods.md) and method owner documents |
| API schemas and value sets | [API schema owners in the Reference Index](README.md#api-and-schema-owners) |
| Public errors and close-readiness blocker behavior | [Errors](api/errors.md) |
| Storage records, effects, artifact lifecycle, versioning, and locks | [Storage owners in the Reference Index](README.md#storage-owners) |
| Runtime, repository, and server boundaries | [Runtime Boundaries](runtime-boundaries.md) |
| Security claims and non-claims | [Security](security.md) |
| Surface and connector behavior | [Agent Integration](agent-integration.md), [Surface Recipes](../use/surface-recipes.md) |
| Projection authority and source-state/freshness boundaries | [Projection Authority Reference](projection-and-templates.md) |
| Template bodies for readable displays | [Template Bodies](template-bodies.md) |
| Out-of-scope and reserved capability boundaries | [Scope](scope.md) |
| Product terminology | [Glossary](glossary.md), [Translation Guide](../maintain/translation-guide.md), [docs/terminology-map.yaml](../../terminology-map.yaml) |
