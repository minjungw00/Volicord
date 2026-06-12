# Scope Reference

This reference owns the Harness supported baseline scope boundary. It defines which capability families are inside the baseline, which remain outside it, and how reserved or profile-gated values affect scope.

This page is a stable reference contract rather than project narration or implementation planning.

<a id="owns-does-not-own"></a>
## Owns / Does Not Own

This document owns:

- the supported baseline scope boundary
- included and excluded capability families
- the scope meaning of reserved and profile-gated values
- scope-level guarantee and non-claim wording that other documents should summarize instead of repeating

This document does not own:

- implementation sequencing; see the [Implementation Guide](../build/implementation-guide.md)
- API method behavior
- schema fields
- storage records or effects
- runtime and repository boundary detail
- security guarantee semantics or proof requirements
- conformance procedures
- template bodies
- connector behavior
- detailed specifications for out-of-scope capabilities

Use this page when deciding whether a capability is part of the supported baseline scope. Route, build, README, and reference documents should link here for the scope boundary instead of repeating the detailed list.

<a id="supported-baseline-scope"></a>
## Supported Baseline Scope

The baseline scope is intentionally narrow. A capability is inside the supported baseline only when this page includes the capability family and the relevant owner documents define its behavior, data shapes, and storage or security consequences.

<a id="what-supported-means"></a>
### What Supported Means

In this documentation set, supported means:

- The capability is part of the baseline contract that agents, implementers, and maintainers may rely on.
- The capability has one or more owner documents that define its behavior, schema shape, storage effect, security consequence, or terminology as needed.
- Other documents may summarize the capability, but they should route detailed questions to the owner documents listed here or in the [Reference Index](README.md).

Supported does not mean:

- every value name that appears in a schema or example is available as baseline behavior
- an out-of-scope capability becomes supported because a route page, example, schema note, or value set mentions it
- a security, conformance, storage, or projection guarantee exists without the relevant owner document defining that guarantee

<a id="included-capabilities"></a>
### Included Capabilities

| Baseline capability | Supported boundary | Primary owners |
|---|---|---|
| Plain-language intake and `Task` creation | A local `Task` can be started from plain-language user intent through the supported intake path. | [Intake method](api/method-intake.md), [Core Model](core-model.md) |
| Scope updates | `Task` and Change Unit scope can be updated through the supported scope-update path. | [Update-scope method](api/method-update-scope.md), [Core Model](core-model.md) |
| Status and close-readiness review | Status, evidence sufficiency, known blockers, and close-readiness state can be read through supported read paths. | [Status method](api/method-status.md), [Close-task method](api/method-close-task.md), [API State Schemas](api/schema-state.md), [Core Model](core-model.md) |
| Prepare-write authorization | `harness.prepare_write` can create an owner-scoped, single-use `Write Authorization` for one compatible product-file write attempt. | [Prepare-write method](api/method-prepare-write.md), [Storage Effects](storage-effects.md), [Security](security.md) |
| Local surface registration | Registered local surfaces can identify the selected surface and supported capabilities for scope checks. | [Agent Integration](agent-integration.md), [Surface Recipes](../use/surface-recipes.md), [Security](security.md) |
| Artifact staging and compatible artifact linking | New artifact bytes can enter the baseline through the supported staging path; compatible persisted artifact references can be linked when artifact owners allow it. | See [Artifact staging owners](#artifact-staging-owners). |
| Run and evidence recording | Runs and compact evidence summaries can be recorded for baseline work. | [Record-run method](api/method-record-run.md), [Storage Effects](storage-effects.md), [Core Model](core-model.md) |
| Focused user judgment capture | User-owned judgments can be requested and recorded through supported judgment paths without substituting for Core-owned state, evidence, or close-readiness rules. | See [User judgment owners](#user-judgment-owners). |
| Close attempts | `harness.close_task` can evaluate close readiness and attempt supported close outcomes while preserving evidence, final acceptance, residual-risk, and non-substitution boundaries. | [Close-task method](api/method-close-task.md), [Core Model](core-model.md), [Errors](api/errors.md) |
| Read-time status display | Read-only status or derived display can summarize source state when the projection and template owners allow it. | [Projection Authority Reference](projection-and-templates.md), [Template Bodies](template-bodies.md), [API State Schemas](api/schema-state.md) |

<a id="artifact-staging-owners"></a>
Artifact staging owners:

- Method behavior: [Stage-artifact method](api/method-stage-artifact.md).
- API shapes: [API Artifact Schemas](api/schema-artifacts.md).
- Lifecycle and storage effects: [Artifact Storage](storage-artifacts.md) and [Storage Effects](storage-effects.md).

<a id="user-judgment-owners"></a>
User judgment owners:

- Method behavior: [User-judgment methods](api/method-user-judgment.md).
- Product meaning: [Core Model](core-model.md).
- API shapes and values: [API Judgment Schemas](api/schema-judgment.md) and [API Value Sets](api/schema-value-sets.md).

<a id="excluded-from-baseline-scope"></a>
## Excluded from Baseline Scope

The following capability families are outside the supported baseline unless this Scope reference and the affected owner documents promote them.

Not included:

- native artifact capture from surfaces
- persistent projection jobs, projection reconciliation, generated projection files, and managed projection repair
- full evidence-manifest production
- manual QA and external verification workflows
- command, network, and secret access observation
- command, network, and secret pre-tool blocking
- pre-tool isolation, sandboxing, or stronger isolation guarantee semantics
- hosted dashboards
- connector marketplaces
- export or transfer packages
- executable fixture runners
- generated conformance artifacts
- operations profiles

Does not imply:

- Excluded capabilities are not baseline requirements.
- Approval of a sensitive action does not create observation, blocking, isolation, QA, or verification behavior unless the relevant owners define that capability as supported.

Owner links:

- Security non-claims, guarantee levels, and observation boundaries: [Security](security.md).
- Value names and reserved values: [API Value Sets](api/schema-value-sets.md).
- Conformance procedures and checks: [Conformance](conformance.md).

<a id="reserved-and-profile-gated-values"></a>
## Reserved and Profile-Gated Values

Some value names may exist as reserved values or profile-gated values without being supported user-visible capabilities.

Reserved value:

- A reserved value may appear as vocabulary, compatibility surface area, or a value-set entry.
- A reserved value does not activate behavior, create a default, satisfy close readiness, or create a guarantee by name alone.

Profile-gated value:

- A profile-gated value is available only when the relevant profile or gate is defined by the owner documents and this Scope reference includes the resulting capability.
- If either the profile/gate or the capability owner is missing, the value is not supported baseline behavior.

Point-of-use rule:

- Mark reserved and profile-gated values where they appear.
- Do not describe them as default, required, supported, enforced, accepted, verified, close-ready, detective, sandboxed, or stronger-isolation behavior unless this page and the semantic owner both define that behavior.

Owner links:

- Exact value names and value-set placement: [API Value Sets](api/schema-value-sets.md).
- Guarantee semantics and non-claims, including stronger isolation claims: [Security](security.md).
- Product terminology for reserved and profile-gated values: [Glossary](glossary.md).

<a id="out-of-scope-capability-promotion"></a>
## Out-of-Scope Capability Promotion

An out-of-scope capability becomes supported only when this Scope reference and the affected owner documents define a narrow supported contract.

Promotion must define:

- the supported behavior and fallback behavior
- the responsible owner documents
- API behavior, schemas, storage effects, runtime boundaries, security guarantees, conformance checks, and terminology updates when those areas are affected
- paired English and Korean documentation

If no current owner exists for the capability, promotion requires creating or designating that owner before the capability is described as supported. Do not route readers to a placeholder as if it were a current owner.

Does not imply:

- Mentioning an excluded, reserved, or profile-gated capability in examples, route text, schema notes, value sets, or this reference does not promote it.

<a id="supported-guarantee-boundary"></a>
## Supported Guarantee Boundary

Baseline scope uses the guarantee level defined by [Security](security.md). Scope availability and security semantics must agree before a guarantee can be described as supported.

Supported boundary:

- The baseline guarantee boundary is `cooperative` unless this page and [Security](security.md) define another supported guarantee.
- `harness.prepare_write` and `Write Authorization` are product-file write compatibility mechanisms, not isolation or sandboxing guarantees.

Not included:

- The baseline scope does not provide stronger isolation guarantee semantics.
- Reserved or profile-gated guarantee labels do not expand the baseline scope.

Owner links:

- Guarantee semantics, detective wording, and security non-claims: [Security](security.md).
- Guarantee label value entries: [API Value Sets](api/schema-value-sets.md).
- Method behavior: [Prepare-write method](api/method-prepare-write.md), routed from [API Methods](api/methods.md).
- Core meaning: [Core Model](core-model.md).

<a id="owner-documents"></a>
## Owner Documents

Use these owners for detailed contract questions. This table routes responsibility; it does not duplicate those contracts.

| Contract area | Owner documents |
|---|---|
| Baseline scope, excluded scope, and reserved/profile-gated scope availability | [Scope Reference](scope.md) |
| Core authority, `Task` state, close readiness, and user-owned judgment boundaries | [Core Model](core-model.md) |
| Public API method list and method routing | [API Methods](api/methods.md) |
| Method-specific API behavior | [Intake](api/method-intake.md), [Update Scope](api/method-update-scope.md), [Status](api/method-status.md), [Prepare Write](api/method-prepare-write.md), [Stage Artifact](api/method-stage-artifact.md), [Record Run](api/method-record-run.md), [User Judgment](api/method-user-judgment.md), [Close Task](api/method-close-task.md) |
| Shared API envelopes, response branches, schemas, and value sets | [API Schema Core](api/schema-core.md), [API State Schemas](api/schema-state.md), [API Artifact Schemas](api/schema-artifacts.md), [API Judgment Schemas](api/schema-judgment.md), [API Value Sets](api/schema-value-sets.md) |
| Public errors and close-readiness blocker routing | [Errors](api/errors.md) |
| Storage records, effects, artifact lifecycle, versioning, and locks | [Storage](storage.md), [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), [Artifact Storage](storage-artifacts.md), [Storage Versioning](storage-versioning.md) |
| Runtime and repository boundaries | [Runtime Boundaries](runtime-boundaries.md) |
| Security claims, access-boundary wording, and guarantee semantics | [Security](security.md) |
| Surface and connector behavior | [Agent Integration](agent-integration.md), [Surface Recipes](../use/surface-recipes.md) |
| Projection authority, source-state freshness, and template routing | [Projection Authority Reference](projection-and-templates.md), [Template Bodies](template-bodies.md) |
| Conformance procedures and check expectations | [Conformance](conformance.md) |
| Implementation entry routing | [Implementation Guide](../build/implementation-guide.md) |
| Product terminology and bilingual terminology controls | [Glossary](glossary.md), [Translation Guide](../maintain/translation-guide.md), [docs/terminology-map.yaml](../../terminology-map.yaml) |
