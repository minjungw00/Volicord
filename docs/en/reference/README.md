# Reference Index

Use Reference when you need the exact owner contract for a schema, gate, state transition, DDL profile, projection rule, template body, security meaning, conformance rule, connector behavior, policy, or term.

Do not read the whole Reference set by default. Choose the owner for the question in front of you, then follow its links only when that owner delegates a stricter detail.

## Owner-Contract Map

| Question | Contract owner |
|---|---|
| What is the authoritative Core state behavior? | [Kernel Reference](kernel.md) owns entities, gates, state transitions, write authority, `prepare_write`, `record_run`, `close_task`, and close semantics. |
| What is the public API or schema shape? | [MCP API And Schemas](mcp-api-and-schemas.md) owns staged public tools/resources, request and response envelopes, shared refs, errors, idempotency, state conflict behavior, and validator result schema. |
| Where is runtime state stored? | [Storage And DDL](storage-and-ddl.md) owns runtime layout, DDL profiles, storage JSON, locks, artifacts, migrations, baselines, projection jobs, and validator storage. |
| How do readable documents work? | [Document Projection Reference](document-projection.md) owns projection rules, freshness, managed blocks, and authority boundaries; [Template Reference](templates/README.md) owns rendered Markdown shapes. |
| What security guarantee can Harness claim? | [Security Threat Model Reference](security-threat-model.md) owns assets, trust boundaries, threats, controls, guarantee levels, and honest security wording. |
| How should agent surfaces integrate? | [Agent Integration Reference](agent-integration.md) owns connector profiles, context push/pull, fallback behavior, and generated-manifest boundaries; [Surface Cookbook](surface-cookbook.md) owns surface recipes. |
| What do operators and conformance authors use? | [Operations And Conformance Reference](operations-and-conformance.md) owns operator behavior and conformance run entrypoints; [Conformance Fixtures Reference](conformance-fixtures.md) owns fixture mechanics and Kernel Smoke queue. |
| Where do later fixture scenarios live? | [Future Fixture Catalog](future-fixture-catalog.md) owns detailed future scenarios, coverage maps, and catalog-only candidates. |
| What governs design-quality checks? | [Design Quality Policies](design-quality-policies.md) owns policies, validator IDs, severity composition, waiver semantics, and close impact. |
| What does a term mean? | [Glossary Reference](glossary.md) owns public/internal terminology definitions and owner routing. |
| How do runtime pieces fit together? | [Runtime Architecture Reference](runtime-architecture.md) owns runtime spaces, Core transaction placement, architecture flow, artifacts, projection/reconcile placement, and recovery overview. |

## Reader Shortcuts

- If you are implementing the future server, start in [Build](../build/implementation-overview.md), then come here for the specific owner contract.
- If you are integrating an agent, start with [Agent Session Flow](../use/agent-session-flow.md), then use [Agent Integration Reference](agent-integration.md) and [Surface Cookbook](surface-cookbook.md).
- If you are checking a schema, start with [MCP API And Schemas](mcp-api-and-schemas.md) or [Storage And DDL](storage-and-ddl.md), depending on whether the contract is API-facing or persisted.
- If you are checking a `harness://` resource, start with the staged [Read-only resources](mcp-api-and-schemas.md#read-only-resources) table before treating a URI as required for a delivery stage.
- If you are checking a user-facing wording claim, start with the owner of the underlying fact. Projection and template docs control display, but they do not create authority.
