# Reference index

Use this human-readable index to choose the next owner document for a Harness reference question. For the exact machine-readable owner route, use [`docs/doc-index.yaml`](../../doc-index.yaml); it owns `doc_id`, paired paths, roles, owner scope, dependencies, normative level, and audience metadata.

This README is route-only. It does not define term meanings, terminology metadata, API behavior, error meaning, error precedence, response branch routing, blocker routing, storage effects, schema shapes, security guarantees, or Core authority semantics.

## Start Here

- Product/system boundaries: [Scope](scope.md), [Core Model](core-model.md), [Runtime Boundaries](runtime-boundaries.md), and [Security](security.md).
- API method behavior: [API Methods](api/methods.md), then the linked method owner.
- API schema families: [Schema Core](api/schema-core.md), [State Schemas](api/schema-state.md), [Artifact Schemas](api/schema-artifacts.md), [Judgment Schemas](api/schema-judgment.md), and [Value Sets](api/schema-value-sets.md).
- API error families: [API Errors](api/errors.md), which routes to error codes, precedence, response routing, blocker routing, and machine-readable details.
- Storage families: [Storage](storage.md), which routes to records, DDL, effects, artifacts, and versioning.
- Surface, projection, and display routes: [Agent Integration](agent-integration.md), [Surface Recipes](../use/surface-recipes.md), [Projection and Templates](projection-and-templates.md), and [Template Bodies](template-bodies.md).
- Quality and verification routes: [Conformance](conformance.md), [Design Quality](design-quality.md), and the relevant method or Core owner for the question.

## Common Crossings

- User-owned judgment meaning belongs in [Core Model](core-model.md); request and record method behavior belongs in [Request-user-judgment method](api/method-request-user-judgment.md) and [Record-user-judgment method](api/method-record-user-judgment.md); judgment-shaped API data belongs in [Judgment Schemas](api/schema-judgment.md).
- Close-readiness authority concepts belong in [Core Model](core-model.md); `harness.close_task` behavior belongs in [Close-Task Method](api/method-close-task.md); `CloseReadinessBlocker` shape belongs in [State Schemas](api/schema-state.md); blocker/API response boundary questions belong in [API Blocker Routing](api/blocker-routing.md).
- Public error code meaning belongs in [API Error Codes](api/error-codes.md); error precedence belongs in [API Error Precedence](api/error-precedence.md); response branch routing belongs in [API Error Routing](api/error-routing.md); machine-readable error details belong in [API Error Details](api/error-details.md).
- Terminology lookup starts with the [Glossary](glossary.md) for selected reader-facing terms and [`docs/terminology-map.yaml`](../../terminology-map.yaml) for structured terminology and identifier controls.

## Maintenance Routes

- Repository editing rules: [`AGENTS.md`](../../../AGENTS.md).
- Authoring rules: [Authoring Guide](../maintain/authoring-guide.md).
- Documentation checks: [Checks](../maintain/checks.md).
- English/Korean wording and Korean style: [Translation Guide](../maintain/translation-guide.md).
