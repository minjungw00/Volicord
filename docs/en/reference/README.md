# Reference Index

Use Reference when you need an exact owner contract. Reference owns contract lookup for future Harness Server planning; it is not the first-read tutorial and not the staged implementation plan.

These documents describe future Harness Server contracts for review. They do not mean a server/runtime, Harness Runtime Home, conformance runner, generated projection system, runtime data, or implementation exists in this repository today.

Do not read the whole Reference set by default. Choose the owner for the question in front of you, then follow links only when that owner delegates a stricter detail.

## Exact Contract Owners

The owner is the only place to define exact fields, enum values, lifecycle states, DDL, request/response shapes, security guarantees, projection/template bodies, fixture assertions, validator IDs, or official terminology. Other docs should summarize the reader-visible consequence and link to the owner.

| Contract area | Owner |
|---|---|
| Core authority, entities, gates, state transitions, `prepare_write`, Write Authorization, `record_run`, `close_task`, blockers, waivers, and non-substitution rules | [Core Model Reference](core-model.md) |
| Public MCP/API methods and active/later method ownership | [MVP API](api/mvp-api.md) for active MVP-1; [API Schema Later](../later/index.md#later-schema-candidates) for later/profile-gated methods |
| API schemas, envelopes, shared refs, `ArtifactRef`, `ValidatorResult`, staged value sets, read-only resources, public errors, idempotency, replay, and state conflicts | [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), and method owners above |
| Storage layout, SQLite DDL profiles, persisted tables, storage-owned JSON `TEXT`, locks, migrations, artifacts, baselines, projection-job storage, and validator-run storage | [Storage](storage.md) |
| Security assets, local access posture, trust boundaries, threat/control categories, guarantee-level meanings, and honest cooperative/detective/preventive/isolated wording | [Security Reference](security.md) |
| Agent integration, connector capability profiles, fallback behavior, context push/pull, generated manifests, Role Lens behavior, and compact surface recipes | [Agent Integration Reference](agent-integration.md) |
| Projection rules, readable views, authority boundaries, freshness/failure behavior, managed blocks, human-editable sections, active template bodies, template classes, card bodies, template display shapes, and artifact-ref rendering | [Projection And Templates Reference](projection-and-templates.md) |
| Current conformance status, what conformance means, future fixture shape, assertion authority, representative active examples, catalog-only future boundary, and metrics boundary | [Conformance Reference](conformance.md) |
| Names-only future fixture family candidates outside the MVP path | [Later Candidate Index: Future Fixture Families](../later/index.md#future-fixture-families) |
| Future operations/profile candidates outside active Reference scope, including diagnostics, recovery/export/reconcile, artifact checks, and future conformance run entrypoints | [Later Candidate Index: Operations Candidates](../later/index.md#operations-candidates). Runtime conformance meaning stays with [Conformance Reference](conformance.md), and docs-maintenance stays with Maintain docs. |
| Runtime boundary spaces, Product Repository / Harness Server / Runtime Home separation, Core-only canonical mutation authority, derived projection/status-card boundary, artifact storage boundary, recovery boundary, and current non-isolation claims | [Runtime Boundaries Reference](runtime-boundaries.md) |
| Active design-quality role, finding severity, close-blocker conditions, waiver boundary, evidence expectations, validator ID boundary, and later policy catalog boundary | [Design Quality](design-quality.md) |
| Terminology, capitalization, official term wording, record-name orientation, and owner routing | [Glossary Reference](glossary.md) |

Documentation authoring, translation, review, link hygiene, owner-boundary drift, and docs-maintenance checks are Maintain responsibilities: [Authoring Guide](../maintain/authoring-guide.md), [Translation Guide](../maintain/translation-guide.md), and [Documentation Checks](../maintain/documentation-checks.md).

## Reader Shortcuts

- Future server implementer: start with [MVP Plan](../build/mvp-plan.md). Return here only for exact owner contracts.
- First internal proof: use [MVP Plan: First internal smoke target](../build/mvp-plan.md#first-internal-smoke-target), then [Core Model Reference](core-model.md), [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), [Storage](storage.md), and [Security Reference](security.md) as needed.
- User or agent behavior wording: start with [User Guide](../use/user-guide.md) or [Agent Guide](../use/agent-guide.md), then use Reference only for the exact fact behind a visible blocker, judgment, write check, evidence gap, close result, or connector behavior.
- API question: start with [MVP API](api/mvp-api.md) for active methods, [API Schema Core](api/schema-core.md) for shared shapes, [API Errors](api/errors.md) for public errors, and [API Schema Later](../later/index.md#later-schema-candidates) for later/profile-gated material.
- Storage or DDL question: start with [Storage](storage.md).
- Security guarantee question: start with [Security Reference](security.md), then the exact API, storage, Core, connector, or conformance owner for the covered operation. Future operations candidates stay in [Later Candidate Index: Operations Candidates](../later/index.md#operations-candidates).
- Projection or template question: use [Projection And Templates Reference](projection-and-templates.md) for derived-display rules, active current MVP template bodies, card shapes, freshness, and authority boundaries.
- Future assurance, operations, or fixture catalog material: use [Assurance Profile](../later/index.md#assurance-candidates), [Operations Profile](../later/index.md#operations-candidates), and [Later Candidate Index: Future Fixture Families](../later/index.md#future-fixture-families). These are not the MVP implementation path unless promoted by an owner.

## Non-Owner Rule

If a Build, Use, Start, Maintain, or README page needs a strict contract, it should state the reader-visible consequence and link here or to the owner. It should not paste a full schema, DDL block, transition table, fixture mini-language, template body, enum table, validator table, projection table, threat catalog, or glossary definition.
