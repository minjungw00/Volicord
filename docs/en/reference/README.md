# Reference Index

Use Reference when you need an exact owner contract. Reference owns contract lookup for future Harness Server planning; it is not the first-read tutorial and not the staged implementation plan.

These documents describe future Harness Server contracts for review. They do not mean a server/runtime, Harness Runtime Home, conformance runner, generated projection system, runtime data, or implementation exists in this repository today.

Do not read the whole Reference set by default. Choose the owner for the question in front of you, then follow links only when that owner delegates a stricter detail.

## Exact Contract Owners

The owner is the only place to define exact fields, enum values, lifecycle states, DDL, request/response shapes, security guarantees, projection/template bodies, fixture assertions, validator IDs, or official terminology. Other docs should summarize the reader-visible consequence and link to the owner.

| Contract area | Owner |
|---|---|
| Core authority, entities, gates, state transitions, `prepare_write`, Write Authorization, `record_run`, `close_task`, blockers, waivers, and non-substitution rules | [Core Model Reference](core-model.md) |
| Public MCP/API methods and active/later method ownership | [MVP API](api/mvp-api.md) for active MVP-1; [API Schema Later](api/schema-later.md) for later/profile-gated methods |
| API schemas, envelopes, shared refs, `ArtifactRef`, `ValidatorResult`, staged value sets, read-only resources, public errors, idempotency, replay, and state conflicts | [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), and method owners above |
| Storage layout, SQLite DDL profiles, persisted tables, storage-owned JSON `TEXT`, locks, migrations, artifacts, baselines, projection-job storage, and validator-run storage | [Storage](storage.md) |
| Security assets, local access posture, trust boundaries, threat/control categories, guarantee-level meanings, and honest cooperative/detective/preventive/isolated wording | [Security Reference](security.md) |
| Agent integration, connector capability profiles, fallback behavior, context push/pull, generated manifests, Role Lens behavior, and surface-specific recipes | [Agent Integration Reference](agent-integration.md) and [Surface Cookbook](surface-cookbook.md) |
| Projection rules, readable views, authority boundaries, freshness/failure behavior, managed blocks, human-editable sections, template classes, and artifact-ref rendering | [Projection And Templates Reference](projection-and-templates.md) |
| Full rendered template bodies, card bodies, and template display shapes | [Template Reference](templates/README.md) |
| Conformance model, MVP behavior examples, future fixture body shape, future runner/assertion semantics, fixture profiles, suite metadata boundaries, current-phase fixture status, and Kernel Smoke authoring queue | [Conformance Fixtures Reference](conformance-fixtures.md) |
| Future scenario-family inventory, promotion criteria, suite-family labels, and catalog-only future candidates outside the MVP path | [Future Fixtures](../later/future-fixtures.md) |
| Operations, diagnostics, staged operator surface, recovery/export/reconcile operations, artifact checks, future conformance run entrypoints, and docs-maintenance reporting profile | [Operations And Conformance Reference](operations-and-conformance.md) |
| Runtime spaces, Core process placement, Core-only canonical mutation authority, transaction ordering, artifact/projection/reconcile placement, and architecture-level recovery overview | [Runtime Architecture Reference](runtime-architecture.md) |
| Design-quality policies, policy-to-validator mapping, stable validator IDs, severity composition, waiver semantics, evidence expectations, and design-quality close impact | [Design Quality Policies](design-quality-policies.md) |
| Terminology, capitalization, official term wording, record-name orientation, and owner routing | [Glossary Reference](glossary.md) |

Documentation authoring, translation, review, link hygiene, owner-boundary drift, and docs-maintenance checks are Maintain responsibilities: [Authoring Guide](../maintain/authoring-guide.md), [Translation Guide](../maintain/translation-guide.md), and [Documentation Checks](../maintain/documentation-checks.md).

## Reader Shortcuts

- Future server implementer: start with [MVP Plan](../build/mvp-plan.md). Return here only for exact owner contracts.
- First internal proof: use [MVP Plan: First internal smoke target](../build/mvp-plan.md#first-internal-smoke-target), then [Core Model Reference](core-model.md), [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), [Storage](storage.md), and [Security Reference](security.md) as needed.
- User or agent behavior wording: start with [User Guide](../use/user-guide.md) or [Agent Guide](../use/agent-guide.md), then use Reference only for the exact fact behind a visible blocker, judgment, write check, evidence gap, close result, or connector behavior.
- API question: start with [MVP API](api/mvp-api.md) for active methods, [API Schema Core](api/schema-core.md) for shared shapes, [API Errors](api/errors.md) for public errors, and [API Schema Later](api/schema-later.md) for later/profile-gated material.
- Storage or DDL question: start with [Storage](storage.md).
- Security guarantee question: start with [Security Reference](security.md), then the exact API, storage, Core, connector, operations, or conformance owner for the covered operation.
- Projection or template question: start with [Projection And Templates Reference](projection-and-templates.md); use [Template Reference](templates/README.md) only when the exact rendered body or card shape matters.
- Future assurance, operations, or fixture catalog material: use [Assurance Profile](../later/assurance-profile.md), [Operations Profile](../later/operations-profile.md), and [Future Fixtures](../later/future-fixtures.md). These are not the MVP implementation path unless promoted by an owner.

## Non-Owner Rule

If a Build, Use, Start, Maintain, or README page needs a strict contract, it should state the reader-visible consequence and link here or to the owner. It should not paste a full schema, DDL block, transition table, fixture mini-language, template body, enum table, validator table, projection table, threat catalog, or glossary definition.
