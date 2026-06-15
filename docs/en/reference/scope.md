# Scope reference

This reference owns the Harness supported baseline scope boundary. Harness is the local work-authority product/system for AI-assisted product work. This page defines which capability families are inside the baseline, which remain outside it, and how reserved or profile-gated values affect scope.

This page is a stable reference contract rather than project narration or implementation planning.

<a id="owns-does-not-own"></a>
## Owns / does not own

This document owns:

- the supported baseline scope boundary
- included and excluded capability families
- the scope meaning of reserved and profile-gated values
- scope-level guarantee boundaries that other documents should summarize instead of repeating

This document does not own:

- baseline implementation reading paths; see the [Implementation Guide](../build/implementation-guide.md)
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
## Supported baseline scope

The baseline scope is intentionally narrow. A capability is inside the supported baseline only when this page includes the capability family and the relevant owner documents define its behavior, data shapes, and storage or security consequences.

<a id="what-supported-means"></a>
### What supported means

In this documentation set, supported means:

- The capability is part of the baseline contract that agents, implementers, and maintainers may rely on.
- The capability has one or more owner documents that define its behavior, schema shape, storage effect, security consequence, or terminology as needed.
- Other documents may summarize the capability, but they should route detailed questions to the owner documents listed here or in the [Reference Index](README.md).

Supported does not mean:

- every value name that appears in a schema or example is available as baseline behavior
- an out-of-scope capability becomes supported because a route page, example, schema note, or value set mentions it
- a security, conformance, storage, or projection guarantee exists without the relevant owner document defining that guarantee

<a id="included-capabilities"></a>
### Included capabilities

| Baseline capability | Supported boundary | Owner documents |
|---|---|---|
| Plain-language intake and `Task` creation | A local `Task` can be started from plain-language user intent through the supported intake path. | [Intake method](api/method-intake.md), [Core Model](core-model.md) |
| Scope updates | `Task` and Change Unit scope can be updated through the supported scope-update path. | [Update-scope method](api/method-update-scope.md), [Core Model](core-model.md) |
| Status and close-readiness review | Status, evidence sufficiency, known blockers, and close-readiness state can be read through supported read paths. | [Status method](api/method-status.md), [Close-task method](api/method-close-task.md), [API State Schemas](api/schema-state.md), [Core Model](core-model.md) |
| Prepare-write authorization | `harness.prepare_write` can create an owner-scoped, single-use `Write Authorization` for one compatible product-file write attempt. | [Prepare-write method](api/method-prepare-write.md), [Storage Effects](storage-effects.md), [Security](security.md) |
| Local surface registration | Registered local surfaces can identify the selected surface and supported capabilities for scope checks. | [Agent Integration](agent-integration.md), [Surface Recipes](../use/surface-recipes.md), [Security](security.md) |
| Artifact staging and compatible artifact linking | New artifact bytes can enter the baseline through the supported staging path; compatible persisted artifact references can be linked when artifact owners allow it. | See [Artifact staging owners](#artifact-staging-owners). |
| Run and evidence recording | Runs and compact evidence summaries can be recorded for baseline work. | [Record-run method](api/method-record-run.md), [Storage Effects](storage-effects.md), [Core Model](core-model.md) |
| Focused user-owned judgment capture | User-owned judgments can be requested and recorded through supported judgment paths without substituting for Core-owned state, evidence, or close-readiness rules. | See [User-owned judgment owners](#user-judgment-owners). |
| Close attempts | `harness.close_task` can evaluate close readiness and attempt supported close outcomes while preserving evidence, final acceptance, residual-risk, and non-substitution boundaries. | [Close-task method](api/method-close-task.md), [Core Model](core-model.md), [API blocker routing](api/blocker-routing.md) |
| Read-time status display | Read-only status or derived display can summarize source state when the projection and template owners allow it. | [Projection Authority Reference](projection-and-templates.md), [Template Bodies](template-bodies.md), [API State Schemas](api/schema-state.md) |

<a id="artifact-staging-owners"></a>
Artifact staging owners:

- Method behavior: [Stage-artifact method](api/method-stage-artifact.md).
- API shapes: [API Artifact Schemas](api/schema-artifacts.md).
- Lifecycle and storage effects: [Artifact Storage](storage-artifacts.md) and [Storage Effects](storage-effects.md).

<a id="user-judgment-owners"></a>
User-owned judgment owners:

- Method behavior: [Request-user-judgment method](api/method-request-user-judgment.md) and [Record-user-judgment method](api/method-record-user-judgment.md).
- Product meaning: [Core Model](core-model.md).
- API shapes and values: [API Judgment Schemas](api/schema-judgment.md) and [API Value Sets](api/schema-value-sets.md).

<a id="excluded-from-baseline-scope"></a>
## Excluded from baseline scope

The following capability families are outside the supported baseline scope. They become supported only when this Scope reference includes them and the affected existing owner documents define their behavior.

Excluded capabilities:

- native artifact capture from surfaces
- persistent projection jobs, projection reconciliation, generated projection files, and managed projection repair
- expanded or additional evidence collection workflows
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

Scope rule:

- Capabilities listed here are outside the baseline scope.
- Excluded capabilities are not baseline requirements.
- Excluded capabilities become supported behavior only when this Scope reference and the affected existing owner documents explicitly define them as supported.
- Until then, excluded capabilities are not supported behavior.
- Sensitive-action approval does not by itself imply broad security monitoring, quarantine, QA gates, verification gates, command/network/secret observation, pre-tool blocking, isolation, or any other out-of-scope behavior.

Owner routing:

- API method behavior and schema shapes: [API Methods](api/methods.md), [API Schema Core](api/schema-core.md), [API State Schemas](api/schema-state.md), [API Artifact Schemas](api/schema-artifacts.md), and [API Judgment Schemas](api/schema-judgment.md).
- API value ownership, enum/status value semantics, and exact value names: [API Value Sets](api/schema-value-sets.md).
- Storage records/effects and artifact lifecycle: [Storage](storage.md), [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), and [Artifact Storage](storage-artifacts.md).
- Runtime and repository boundaries: [Runtime Boundaries](runtime-boundaries.md).
- Security boundary wording, guarantee levels, observation, blocking, isolation, and sensitive-action approval boundaries: [Security](security.md) and [Core Model](core-model.md).
- Conformance procedures and check criteria: [Conformance](conformance.md).
- Complete structured terminology metadata and bilingual wording controls: [`docs/terminology-map.yaml`](../../terminology-map.yaml).

<a id="reserved-and-profile-gated-values"></a>
## Reserved and profile-gated values

Some value names may exist as reserved values or profile-gated values without being supported user-visible capabilities.

Reserved value:

- A reserved value may appear as vocabulary, compatibility surface area, or a value-set entry.
- A reserved value does not activate behavior, create a default, satisfy close readiness, or create a guarantee by name alone.

Profile-gated value:

- A profile-gated value is available only when the relevant profile or gate is defined by the owner documents and this Scope reference includes the resulting capability.
- If either the profile/gate or the capability owner is missing, the value is not supported baseline behavior.

Point-of-use rule:

- Mark reserved and profile-gated values where they appear.
- Describe reserved and profile-gated values as default, required, supported, enforced, accepted, verified, close-ready, detective, sandboxed, or stronger-isolation behavior only when this page and the semantic owner both define that behavior.
- Otherwise, do not use those labels for the value.

Owner links:

- API value ownership, exact value names, value-set placement, and enum/status value semantics: [API Value Sets](api/schema-value-sets.md).
- Guarantee semantics and security boundaries, including stronger isolation claims: [Security](security.md).
- Structured terminology metadata for reserved and profile-gated values: [`docs/terminology-map.yaml`](../../terminology-map.yaml).

<a id="out-of-scope-capability-promotion"></a>
## Out-of-scope capability promotion

An out-of-scope capability becomes supported only when this Scope reference and the affected owner documents define a narrow supported contract.

Promotion must define:

- the supported behavior and fallback behavior
- the responsible owner documents
- API behavior, schemas, storage effects, runtime boundaries, security guarantees, conformance checks, and terminology updates when those areas are affected
- paired English and Korean documentation

If no applicable owner exists for the capability, promotion requires creating or designating that owner before the capability is described as supported. Do not route readers to a placeholder as if it were an existing owner.

Promotion boundary:

- Mentioning an excluded, reserved, or profile-gated capability in examples, route text, schema notes, value sets, or this reference does not promote it.

<a id="supported-guarantee-boundary"></a>
## Supported guarantee boundary

Baseline scope uses the guarantee level defined by [Security](security.md). Scope availability and security semantics must agree before a guarantee can be described as supported.

Supported boundary:

- The baseline guarantee boundary is `cooperative` unless this page and [Security](security.md) define another supported guarantee.
- `harness.prepare_write` and `Write Authorization` are product-file write compatibility mechanisms, not isolation or sandboxing guarantees.

Not included:

- The baseline scope does not provide stronger isolation guarantee semantics.
- Reserved or profile-gated guarantee labels do not expand the baseline scope.

Owner links:

- Guarantee semantics, detective wording, and security boundaries: [Security](security.md).
- Guarantee label value entries: [API Value Sets](api/schema-value-sets.md).
- Method behavior: [Prepare-write method](api/method-prepare-write.md), routed from [API Methods](api/methods.md).
- Core meaning: [Core Model](core-model.md).

<a id="owner-documents"></a>
## Owner documents

Use this page for supported baseline scope questions. For detailed questions outside the scope boundary, choose the applicable owner from the [Reference Index](README.md) or [`docs/doc-index.yaml`](../../doc-index.yaml). For API method behavior, start with [API Methods](api/methods.md).
