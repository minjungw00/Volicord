# Implementation guide

This guide helps implementers classify an implementation change, locate the applicable contract owner, find the implementation boundary, and select validation. Product meaning remains in the canonical Reference owners.

This is a guide-level reading path. It does not define or override baseline scope, API behavior, schemas, storage effects, security guarantees, runtime boundaries, error behavior, close-readiness rules, connector behavior, conformance authority, or Core authority semantics. Use [`docs/doc-index.yaml`](../../doc-index.yaml) for the exact machine-readable owner route and the [Reference Index](../reference/README.md) for reader-facing owner navigation.

Harness is the local work-authority product/system for AI-assisted product work. Core is the local authority record for Harness state.

## Implementation Change Classification

Start with the closest row, then add adjacent owners when the change crosses API, storage, runtime, security, or Core authority boundaries. The table is a routing aid, not a full owner registry.

| Change type | First contract owner route | Architecture or code route | Useful validation area |
|---|---|---|---|
| Public API method implementation | [Scope](../reference/scope.md), then [API Methods](../reference/api/methods.md) and the linked method owner; add schema, error, storage, and security owners as touched. | [Implementation Architecture](architecture.md) sections on Core pipeline, Store boundary, effect paths, and code-to-owner routing; `crates/harness-core/src/methods/`, `crates/harness-core/src/pipeline.rs`, and `crates/harness-core/src/policy/`. | Method and Core tests in `crates/harness-core/src/methods/tests.rs`; `tests/conformance/baseline.rs`; `tests/integration/mcp_surface.rs` when adapter exposure is affected. |
| Common Core pipeline or Core policy | [Core Model](../reference/core-model.md), [API Schema Core](../reference/api/schema-core.md), [API Errors](../reference/api/errors.md), and [Storage Effects](../reference/storage-effects.md) when persistence is involved; add [Agent Integration](../reference/agent-integration.md) or [Security](../reference/security.md) when access or guarantee wording is involved. | Architecture sections on Core pipeline and Store boundary, effect and commit boundaries, and implementation invariants; `crates/harness-core/src/pipeline.rs` and `crates/harness-core/src/policy/`. | Core method tests, conformance scenarios that assert the touched owner-defined fact, and integration tests when MCP or Store boundaries are affected. |
| Shared types, schema representations, identifiers, or value sets | API schema owners: [Schema Core](../reference/api/schema-core.md), [State Schemas](../reference/api/schema-state.md), [Artifact Schemas](../reference/api/schema-artifacts.md), [Judgment Schemas](../reference/api/schema-judgment.md), and [Value Sets](../reference/api/schema-value-sets.md); use method owners for method-specific request or result meaning. | The workspace shape and source-module map in [Implementation Architecture](architecture.md); `crates/harness-types/src/methods.rs`, `schema.rs`, `values.rs`, `ids.rs`, and `canonical.rs`. | Type and serialization unit tests, method tests using the shape, and conformance coverage for owner-defined value behavior. |
| Storage effects, records, transactions, or migrations | Storage owner family: [Storage](../reference/storage.md), [Storage Effects](../reference/storage-effects.md), [Storage Records](../reference/storage-records.md), [Storage DDL](../reference/storage-ddl.md), [Artifact Storage](../reference/storage-artifacts.md), and [Storage Versioning](../reference/storage-versioning.md). | Architecture sections on Store boundary, effect and commit boundaries, and source-module map; `crates/harness-store/src/`, especially `core_pipeline.rs`, `migrations.rs`, `sqlite.rs`, and `artifacts.rs`. | Store unit tests, Core method tests for committed effects, `tests/conformance/baseline.rs`, and `tests/integration/mcp_surface.rs` for cross-layer storage effects. |
| MCP startup, binding, transport, or tool dispatch | [MCP Transport](../reference/mcp-transport.md); add [Agent Integration](../reference/agent-integration.md) for verified surface context and [API Methods](../reference/api/methods.md) for the supported public method set exposed through tools. | Architecture operational paths and MCP/Core execution flow; `crates/harness-mcp/src/lib.rs` and `crates/harness-mcp/src/main.rs`. | `crates/harness-mcp/tests/binary_transport.rs` and `tests/integration/mcp_surface.rs`. |
| Administrative CLI setup, registration, or host configuration | [Administrative CLI](../reference/admin-cli.md); add [Runtime Boundaries](../reference/runtime-boundaries.md) and [MCP Transport](../reference/mcp-transport.md) for Runtime Home, Product Repository, process, and host configuration boundaries. | Architecture administrative CLI setup flow; `crates/harness-cli/src/`, especially `local_mcp_command.rs`, `setup.rs`, `wizard.rs`, `host_config.rs`, and `registration.rs`. | `crates/harness-cli/tests/binary_admin.rs`; Store setup tests when bootstrap or migration behavior is touched. |
| Tests, fixtures, and test-support facilities | The owner of each asserted fact; [Conformance](../reference/conformance.md) only for documentation-level conformance scenario meaning and assertion routing. | Architecture test topology; `crates/harness-test-support/`, `tests/conformance/`, `tests/integration/`, and colocated crate tests. | The touched test package or crate tests, plus owner-focused documentation checks when the test exposes a missing contract owner. |

## Default Implementation Reading Order

Use this order unless the change has a narrower owner route already selected:

1. Check supported scope in [Scope](../reference/scope.md).
2. Use [`docs/doc-index.yaml`](../../doc-index.yaml) to locate the canonical owner for each contract question.
3. Read the focused Reference owner for exact meaning.
4. Use [Implementation Architecture](architecture.md) to locate the implementation boundary and execution flow.
5. Inspect the relevant source and tests.
6. Compare code, tests, and documentation against the owner-defined contract.
7. Run validation appropriate to the changed layer.

An implementation change may require more than one owner. For example, a method change can touch method behavior, schema shape, storage effect, runtime boundary, error routing, security wording, and conformance assertions. Keep each question with its focused owner instead of treating this guide as a combined contract.

## Code And Document Disagreement

When implementation and documentation appear to disagree, classify the disagreement before deciding what to edit:

- If a guide-level source-structure description differs from current stable code, correct [Implementation Architecture](architecture.md) to match the implementation structure.
- If code differs from API, schema, storage, security, error, scope, or Core authority owners, do not treat code as the new contract.
- Resolve product-meaning differences through the applicable owner and implementation, not in a route page, README, Use page, or this guide.
- If tests, fixtures, examples, or conformance scenario prose are the only place a behavior is expressed, treat that as a contract-owner gap.
- If no owner can be identified, report the owner gap rather than placing the contract in this guide.

Do not infer a product decision from the mismatch itself. The owner route identifies where the decision belongs.

## Inputs That Are Not Contract Owners

These inputs are useful while implementing, but they do not define product contracts.

| Input | Legitimate use | Owner boundary |
|---|---|---|
| Use pages, including [User Guide](../use/user-guide.md), [Agent Guide](../use/agent-guide.md), [Judgment Examples](../use/judgment-examples.md), and [Surface Recipes](../use/surface-recipes.md) | Understand workflow intent, reader decisions, connector context, and surface expectations. | API payloads, storage effects, access boundaries, security guarantees, close-readiness rules, and error behavior route back to Reference owners. |
| Examples | Understand a representative branch, compact shape, or scenario. | Examples are not full schemas, value-set definitions, storage-effect definitions, or implementation shortcuts. |
| Conformance scenarios | Identify coverage prompts and assertion routing. | Scenario prose and scenario IDs do not own the asserted product fact; the fact routes to Scope or the focused Reference owner. |
| Tests, fixtures, and test-support helpers | Verify owner-defined behavior, set up disposable Runtime Home state, and exercise cross-layer paths. | A test assertion, fixture shape, or helper API must not be the only source for a product contract. |
| Generated output, logs, rendered reports, and current implementation behavior | Diagnose behavior and compare observed implementation against owners. | Runtime output and observed code behavior do not become API, storage, security, Core authority, or conformance contracts, and generated or scratch artifacts do not belong in maintained documentation. |

## Implementation Completion Check

Use this as an implementation and documentation-maintenance check. It is not product acceptance, runtime conformance, close readiness, QA completion, security proof, or residual-risk acceptance.

- Scope and the canonical owner or owner family are identified for each changed behavior.
- The architecture boundary and code area are identified through [Implementation Architecture](architecture.md).
- Code, tests, and documentation are aligned with the owner-defined contract, or an owner gap is reported.
- Paired English and Korean documentation are updated when product meaning changes.
- Appropriate tests or documentation checks are run, or skipped with a reason.
- No behavior is defined only in code, a test, a fixture, an example, generated output, or this guide.
- No scratch notes, generated reports, runtime homes, SQLite files, fixture output, logs, or other transient artifacts are left in maintained documentation.
