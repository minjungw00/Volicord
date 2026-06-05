# Reference Index

Use Reference when you need the exact owner contract for a schema, gate, state transition, DDL profile, projection rule, template body, security meaning, conformance rule, connector behavior, policy, or term.

These owner docs describe future Harness Server contracts for planning and review. They do not mean a server/runtime, Harness Runtime Home, conformance runner, generated projection system, or implementation exists in this repository today.

Do not read the whole Reference set by default. Choose the owner for the question in front of you, then follow its links only when that owner delegates a stricter detail.

## Canonical Contract Ownership Map

Use this map when a contract appears to fit more than one document. The owner is the only place to define exact fields, enum values, lifecycle states, DDL, request/response shapes, security guarantees, projection/template bodies, fixture assertions, validator IDs, or official terminology. Other docs should summarize the reader-visible consequence and link here or to the owner.

| Contract area | Canonical owner |
|---|---|
| Core state, gates, lifecycle, authority invariants, `prepare_write`, Write Authorization lifecycle, `record_run`, `close_task`, blockers, waivers, and non-substitution rules | [Core Model Reference](core-model.md) |
| Public MCP/API methods and per-method request/response behavior | [MVP API](api/mvp-api.md) for active MVP-1; [API Schema Later](api/schema-later.md) for later/profile-gated methods. |
| Shared API envelopes, common response shapes, read-only resource schemas, shared refs, `ArtifactRef`, `ValidatorResult`, API-owned staged value sets, and API error surfaces | [API Schema Core](api/schema-core.md) and [API Errors](api/errors.md). |
| Persisted tables, columns, indexes, check constraints, storage-owned JSON `TEXT`, runtime home layout, locks, migrations, artifact storage, projection-job storage, and validator-run storage | [Storage](storage.md). Storage hardening must reuse the lifecycle/value-set owner named for each field. |
| Local access posture, threat boundary, assets, guarantee-level meanings, and honest cooperative/detective/preventive/isolated wording | [Security Reference](security.md) |
| Surface behavior, connector fallback, agent-facing context contracts, connector capability profiles, generated manifests, Role Lens behavior, and surface-specific recipes | [Agent Integration Reference](agent-integration.md) and [Surface Cookbook](surface-cookbook.md) |
| Projections, compact views, projection freshness/failure behavior, managed blocks, human-editable sections, template classes, and artifact-ref rendering | [Projection And Templates Reference](projection-and-templates.md) |
| Full rendered template bodies, card bodies, and template display shapes | [Template Reference](templates/README.md) |
| Fixture bodies, fixture assertions, conformance scope, runner behavior, fixture profiles, suite metadata boundaries, current-phase fixture status, and the reduced Kernel Smoke queue | [Conformance Fixtures Reference](conformance-fixtures.md) |
| Operator behavior, diagnostics, staged operator surface, conformance run entrypoints, recovery/export/reconcile operations, and read-only docs-maintenance reporting entrypoints | [Operations And Conformance Reference](operations-and-conformance.md); use [Operations Profile](../later/operations-profile.md) for the later reader path. |
| Future scenario-family inventory, promotion criteria, suite-family labels, and catalog-only future candidates | [Future Fixtures](../later/future-fixtures.md) |
| Terminology, capitalization, official term wording, record-name orientation, and owner routing | [Glossary Reference](glossary.md) |
| Runtime spaces, Core process placement, Core-only canonical mutation authority, transaction ordering, artifact/projection/reconcile placement, and architecture-level recovery overview | [Runtime Architecture Reference](runtime-architecture.md) |
| Design-quality policies, policy-to-validator mapping, stable validator IDs, severity composition, waiver semantics, evidence expectations, and design-quality close impact | [Design Quality Policies](design-quality-policies.md) |
| Documentation drift rules, bilingual parity, strict-contract ownership rules, link hygiene, and translation guidance | [Authoring Guide](../maintain/authoring-guide.md), [Translation Guide](../maintain/translation-guide.md), [Korean Authoring Guide](../../ko/maintain/authoring-guide.md), and [Korean Translation Guide](../../ko/maintain/translation-guide.md). |

This map identifies strict contract owners. For known pre-implementation repair axes that cross owner families, use the [Authoring Guide repair-target owner map](../maintain/authoring-guide.md#pre-implementation-repair-target-owner-map). That map is docs-maintenance guidance only; it does not decide documentation acceptance, manual acceptance, runtime conformance, close readiness, or implementation readiness.

## Reader Shortcuts

- If you are implementing the future server, use [Implementation Overview](../build/implementation-overview.md), then [MVP-1 User Work Loop](../build/mvp-user-work-loop.md) -> [MVP API](api/mvp-api.md) -> [Storage](storage.md). Pull other Reference owners only for exact questions.
- If you are planning the first internal smoke, use [Engineering Checkpoint](../build/engineering-checkpoint.md), then [Core Model Reference](core-model.md), [MVP API](api/mvp-api.md), and [Storage](storage.md).
- If you are writing agent instructions, start with [Agent Guide](../use/agent-guide.md), then use [Agent Integration Reference](agent-integration.md) and [Surface Cookbook](surface-cookbook.md) only for connector-specific contracts.
- If you are checking an MVP-1 method, start with [MVP API](api/mvp-api.md). If you are checking shared refs or envelopes, use [API Schema Core](api/schema-core.md). For later methods, use [API Schema Later](api/schema-later.md) and keep them out of the MVP path unless promoted.
- If you are checking a persisted shape, start with [Storage](storage.md).
- If you are checking a `harness://` resource, start with the staged [Read-only resources](api/schema-core.md#read-only-resources) table before treating a URI as required for a delivery stage.
- If you are checking a user-facing wording claim, start with the owner of the underlying fact. Projection and template docs control display, but they do not create authority.
- If you are checking future assurance, operations, or fixture catalog material, use [Assurance Profile](../later/assurance-profile.md), [Operations Profile](../later/operations-profile.md), and [Future Fixtures](../later/future-fixtures.md). These are not the MVP implementation path.
