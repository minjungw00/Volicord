# Reference index

Use this human-readable index to choose the next owner document for a Volicord reference question. For the exact machine-readable owner route, use [`docs/doc-index.yaml`](../../doc-index.yaml); it owns `doc_id`, maintained paths, document kind, focused `canonical_for` scope, maintenance `owner_area`, `created_on`, `last_updated_on`, `last_verified_on`, `applies_to`, dependencies, normative level, primary audience, reader journeys, and translation policy metadata.

This README is route-only. It does not define term meanings, terminology metadata, API behavior, error meaning, error precedence, response branch routing, blocker routing, storage effects, schema shapes, security guarantees, or Core authority semantics.

## Start Here

- Environment prerequisites before installation: [System Requirements](system-requirements.md).
- Executable preparation and verification tutorial: [Installation](../user-guide/installation.md).
- Product/system boundaries: [Scope](scope.md), [Core Model](core-model.md), [Runtime Boundaries](runtime-boundaries.md), and [Security](security.md).
- First-run agent host setup: [Quickstart](../user-guide/quickstart.md) for the shortest success path, then [Agent Host Setup](../user-guide/agent-host-setup.md) for the complete operator guide and [Multi-Repository Agent Setup](../user-guide/multi-repository-agent-setup.md) for one user-scope Agent Connection serving multiple repositories.
- Setup failures and recovery: [Agent Host Troubleshooting](../user-guide/agent-host-troubleshooting.md).
- Local executable contracts: [Administrative CLI](admin-cli.md) for `volicord` administrative commands and Runtime Home selection, and [MCP Transport](mcp-transport.md) for `volicord mcp --stdio` startup, preflight, response wrapping, and shutdown.
- API method behavior: [API Methods](api/methods.md), then the linked method owner.
- API schema families: [Schema Core](api/schema-core.md), [State Schemas](api/schema-state.md), [Artifact Schemas](api/schema-artifacts.md), [Judgment Schemas](api/schema-judgment.md), and [Value Sets](api/schema-value-sets.md).
- API error families: [API Errors](api/errors.md), which routes to error codes, precedence, response routing, blocker routing, and machine-readable details.
- Storage families: [Storage](storage.md), which routes to records, DDL, effects, artifacts, and versioning.
- Connection, projection, and display routes: [Agent Connection Reference](agent-connection.md) for Agent Connection, Connection Projects, and current connection context, [Runtime Boundaries](runtime-boundaries.md) for User Channel and runtime-location boundaries, [Security](security.md) for operation-category non-guarantees, [Projection and Templates](projection-and-templates.md), and [Template Bodies](template-bodies.md).
- Quality and verification routes: [Conformance](conformance.md), [Design Quality](design-quality.md), and the relevant method or Core owner for the question.

## Common Crossings

- User-owned judgment meaning belongs in [Core Model](core-model.md); request and record method behavior belongs in [Request-user-judgment method](api/method-request-user-judgment.md) and [Record-user-judgment method](api/method-record-user-judgment.md); judgment-shaped API data belongs in [Judgment Schemas](api/schema-judgment.md).
- Close-readiness authority concepts belong in [Core Model](core-model.md); `volicord.check_close` and `volicord.close_task` behavior belongs in the [Close Method](api/method-close-task.md); `CloseReadinessBlocker` shape belongs in [State Schemas](api/schema-state.md); blocker/API response boundary questions belong in [API Blocker Routing](api/blocker-routing.md).
- Write ticket meaning and non-substitution rules belong in [Core Model](core-model.md); issue and consumption behavior belongs in [Prepare-write method](api/method-prepare-write.md) and [Record-run method](api/method-record-run.md); persistence effects belong in [Storage Effects](storage-effects.md); security non-guarantees belong in [Security](security.md).
- Judgment Inbox CLI behavior belongs in [Administrative CLI](admin-cli.md); User Channel versus Agent Connection boundaries belong in [Agent Connection Reference](agent-connection.md); inbox item shape belongs in [Judgment Schemas](api/schema-judgment.md); host prompt and local consent URL transport behavior belongs in [MCP Transport](mcp-transport.md).
- Local HTTP loopback behavior belongs in [MCP Transport](mcp-transport.md); command-line startup belongs in [Administrative CLI](admin-cli.md); guarantee limits and non-guarantees belong in [Security](security.md).
- Public error code meaning belongs in [API Error Codes](api/error-codes.md); error precedence belongs in [API Error Precedence](api/error-precedence.md); response branch routing belongs in [API Error Routing](api/error-routing.md); machine-readable error details belong in [API Error Details](api/error-details.md).
- Administrative `volicord` commands are local bootstrap commands, not public Volicord API methods; `volicord mcp --stdio` exposes the public method set through MCP stdio without owning a second method list.
- Terminology lookup starts with the [Glossary](glossary.md) for selected reader-facing terms and [`docs/terminology-map.yaml`](../../terminology-map.yaml) for structured terminology and identifier controls.

## Maintenance Routes

- Repository editing rules: [`AGENTS.md`](../../../AGENTS.md).
- Documentation governance: [Documentation Policy](../maintain/documentation-policy.md).
- Documentation validation: [Validation](../maintain/validation.md).
- English/Korean wording and Korean style: [Translation Policy](../maintain/translation-policy.md).
