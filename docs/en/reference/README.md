# Reference index

Use this index to choose the Reference page for a CLI, API, storage, runtime, security, terminology, quality, or scope question. Exact product contracts live in the focused Reference pages linked below; this README routes readers to those owners and does not define the contracts itself.

This README is route-only. It does not define term meanings, terminology metadata, API behavior, error meaning, error precedence, response branch routing, blocker routing, storage effects, schema shapes, security guarantees, or Core authority semantics.

## Start Here

- Environment prerequisites before installation: [System Requirements](system-requirements.md).
- Managed-host behavioral verification: [Agent Connection](agent-connection.md); ordinary release integrity: [Validation](../maintain/validation.md).
- Executable preparation and verification tutorial: [Installation](../user-guide/installation.md).
- Product/system boundaries: [Scope](scope.md), [Core Model](core-model.md), [Runtime Boundaries](runtime-boundaries.md), and [Security](security.md).
- External-format compatibility, exact adapter selection, and shared Git object-ID validation: [External Contracts](external-contracts.md).
- First-run agent host setup: [Quickstart](../user-guide/quickstart.md) for the shortest success path, then [Agent Host Setup](../user-guide/agent-host-setup.md) for the complete operator guide and [Multi-Repository Agent Setup](../user-guide/multi-repository-agent-setup.md) for one user-scope Agent Connection serving multiple repositories.
- Setup failures and recovery: [Agent Host Troubleshooting](../user-guide/agent-host-troubleshooting.md).
- Local executable contracts: [Administrative CLI](admin-cli.md) for `volicord` administrative commands and Runtime Home selection, and [MCP Transport](mcp-transport.md) for `volicord mcp preflight`, manual `volicord mcp serve`, managed startup, response wrapping, and shutdown.
- API method behavior: [API Methods](api/methods.md), then the linked method owner.
- API schema families: [Schema Core](api/schema-core.md), [State Schemas](api/schema-state.md), [Artifact Schemas](api/schema-artifacts.md), [User Action Schemas](api/schema-user-action.md), [Judgment Schemas](api/schema-judgment.md), and [Value Sets](api/schema-value-sets.md).
- API error families: [API Errors](api/errors.md), which routes to error codes, precedence, response routing, blocker routing, and machine-readable details.
- Product-wide failure categories and persisted-data failure boundaries: [Failure Model](failure-model.md).
- Conservative recorded-change suppression outcomes and diagnostics: [Guard Recorded-Change Suppression](guard-suppression.md).
- Storage families: [Storage](storage.md), which routes to records, DDL, effects, artifacts, and versioning.
- Connection, projection, and display routes: [Agent Connection Reference](agent-connection.md) for Agent Connection, Connection Projects, and current connection context, [Runtime Boundaries](runtime-boundaries.md) for User Channel and runtime-location boundaries, [Security](security.md) for operation-category non-guarantees, [Projection and Templates](projection-and-templates.md), and [Template Bodies](template-bodies.md).
- Quality and verification routes: [Conformance](conformance.md), [Design Quality](design-quality.md), [Agent Connection](agent-connection.md) for behavioral host observations, and the relevant method or Core owner for the question.

## Common Crossings

- User-owned action and judgment meaning belongs in [Core Model](core-model.md); request and resolution method behavior belongs in [Request-user-action method](api/method-request-user-action.md) and [Resolve-user-action method](api/method-resolve-user-action.md); the shared request, resolution, status, inbox, and capture-form shapes belong in [User Action Schemas](api/schema-user-action.md), while nested choice-judgment payloads belong in [Judgment Schemas](api/schema-judgment.md).
- Close-readiness authority concepts belong in [Core Model](core-model.md); `volicord.check_close` and `volicord.close_task` behavior belongs in the [Close Method](api/method-close-task.md); `CloseReadinessBlocker` shape belongs in [State Schemas](api/schema-state.md); blocker/API response boundary questions belong in [API Blocker Routing](api/blocker-routing.md).
- Write ticket meaning and non-substitution rules belong in [Core Model](core-model.md); policy application and Guard candidate behavior belong in [Administrative CLI](admin-cli.md); issue, current-policy reevaluation, and reuse belong in [Prepare-write method](api/method-prepare-write.md); consumption and the independent current-policy check belong in [Record-run method](api/method-record-run.md); the `write_authority_fingerprint` field and scope belong in [State Schemas](api/schema-state.md); persistence and storage-profile boundaries belong in [Storage Effects](storage-effects.md) and [Storage Versioning](storage-versioning.md); security non-guarantees belong in [Security](security.md).
- User-action inbox CLI behavior belongs in [Administrative CLI](admin-cli.md); User Channel versus Agent Connection boundaries belong in [Agent Connection Reference](agent-connection.md); inbox item shape belongs in [User Action Schemas](api/schema-user-action.md).
- Public error code meaning belongs in [API Error Codes](api/error-codes.md); error precedence belongs in [API Error Precedence](api/error-precedence.md); response branch routing belongs in [API Error Routing](api/error-routing.md); machine-readable error details belong in [API Error Details](api/error-details.md).
- Shared Git object-ID validation and canonicalization belong in [External Contracts](external-contracts.md); the cross-surface distinction among rejection, policy non-allow, unavailability, degradation, and corruption belongs in [Failure Model](failure-model.md). API response projection remains with the API error owners.
- Recorded-change suppression outcome, scan budget, fail-safe paths, and reason identifiers belong in [Guard Recorded-Change Suppression](guard-suppression.md).
- Administrative `volicord` commands are local bootstrap commands, not public Volicord API methods; `volicord mcp serve` exposes the public method set through manual MCP stdio without owning a second method list.
- Terminology lookup starts with the [Glossary](glossary.md) for selected reader-facing terms and [`docs/terminology-map.yaml`](../../terminology-map.yaml) for structured terminology and identifier controls.

## Contributor / Maintenance Routes

- Repository editing rules: [`AGENTS.md`](../../../AGENTS.md).
- Machine-readable owner metadata: [`docs/doc-index.yaml`](../../doc-index.yaml).
- Documentation governance: [Documentation Policy](../maintain/documentation-policy.md).
- Documentation validation: [Validation](../maintain/validation.md).
- English/Korean wording and Korean style: [Translation Policy](../maintain/translation-policy.md).
