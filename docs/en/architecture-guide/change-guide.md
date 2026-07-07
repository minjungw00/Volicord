# Implementation guide

This guide gives a practical workflow for making a narrow implementation
change in the Rust workspace. Product meaning remains in the focused Reference
owners. This guide does not define or override baseline scope, API behavior,
schemas, storage effects, security guarantees, runtime boundaries, error
behavior, close-readiness rules, connector behavior, conformance authority, or
Core authority semantics.

Use [Architecture Guide](README.md) when learning the source, the
[Codebase Tour](codebase-tour.md) for first files and symbols,
[Request Lifecycle](request-lifecycle.md) for representative method traces,
[Implementation Design Patterns](design-patterns.md) for recurring structures,
[Storage and Transactions](storage-and-transactions.md) for Store boundaries,
and [Testing Strategy](testing-strategy.md) for test-layer choice. Use
[`docs/doc-index.yaml`](../../doc-index.yaml) for machine-readable owner
routing and the [Reference Index](../reference/README.md) for reader-facing
owner navigation.

Volicord is the local work authority record for AI-assisted product
work. Core is the local authority record for Volicord state.

## Practical Sequence

1. Classify the requested change.

   Decide whether the change touches shared types, Store behavior, Core method
   behavior, MCP adapter behavior, setup workflow, connection provisioning,
   guard hook lifecycle, guard integration files, host adapters, test fixtures,
   or Architecture Guide only. If it crosses more than one boundary, keep the
   questions separate.

2. Locate the current implementation path.

   Use [Implementation Architecture](architecture.md) for the workspace and
   module map, then open the closest source and tests from the routing table
   below. Confirm the named symbols still exist before editing.

3. Identify the exact Reference owner.

   Use the [Reference Index](../reference/README.md) or
   [`docs/doc-index.yaml`](../../doc-index.yaml). Method behavior starts at
   [API Methods](../reference/api/methods.md); storage questions start at
   [Storage](../reference/storage.md); runtime-location questions start at
   [Runtime Boundaries](../reference/runtime-boundaries.md).

4. Implement the narrow change.

   Change the crate or module that owns the implementation responsibility. Keep
   Core-facing code independent of CLI and MCP adapter crates. Do not encode
   new API behavior, schema meaning, storage effects, security guarantees, or
   Core authority semantics only in code, tests, fixtures, examples, generated
   output, or comments.

5. Choose the appropriate test layer.

   Use [Testing Strategy](testing-strategy.md) to pick the smallest layer that
   protects the changed behavior, then add broader tests only when the change
   crosses layers.

6. Update affected Architecture Guide explanation.

   If the durable source shape, dependency direction, execution flow, Store
   boundary, test topology, or change workflow changed, update the relevant
   Architecture Guide page in both languages. Keep exact product contracts in Reference
   owners.

7. Run validation.

   For Rust implementation edits, default to `cargo fmt`,
   `cargo clippy --all-targets --all-features`, and
   `cargo test --all-targets --all-features`. For documentation edits, run the
   applicable Maintain checks for structure, links/indexes, language parity,
   and terminology. Report any skipped command with a reason.

8. Report owner gaps instead of inventing behavior.

   If the implementation needs behavior that no owner defines, stop the product
   meaning change and report the owner gap or update the proper Reference owner
   first. Do not fill the gap in a README, guide, test, fixture, adapter,
   generated output, or implementation comment.

## Change-Type Routing

| Change type | First implementation path | First Reference owner route | Useful test layer | Architecture Guide explanation to check |
|---|---|---|---|---|
| Shared request or value type | `crates/volicord-types/src/methods.rs`, `schema.rs`, `values.rs`, `ids.rs`, or `canonical.rs` | API schema owners and [Value Sets](../reference/api/schema-value-sets.md); method owner for method-specific meaning | `volicord-types` unit tests; Core or MCP tests when the shape affects method planning or adapter exposure | [Codebase Tour](codebase-tour.md), [Design Patterns](design-patterns.md), and [Testing Strategy](testing-strategy.md) |
| Store behavior | `crates/volicord-store/src/core_pipeline.rs`, `core_pipeline/mutation_apply.rs`, `schema.rs`, `schema/*.sql`, `sqlite.rs`, `bootstrap.rs`, or `artifacts.rs` | [Storage](../reference/storage.md), [Storage Effects](../reference/storage-effects.md), [Storage Records](../reference/storage-records.md), [Storage DDL](../reference/storage-ddl.md), [Artifact Storage](../reference/storage-artifacts.md), [Storage Versioning](../reference/storage-versioning.md) | Store unit tests; Core method tests for public effects; conformance or MCP integration when cross-layer behavior changes | [Storage and Transactions](storage-and-transactions.md), [Implementation Architecture](architecture.md), and decision records |
| Core method behavior | `crates/volicord-core/src/methods/`, `pipeline.rs`, and `policy/` | [API Methods](../reference/api/methods.md), then the linked method owner; add schema, error, storage, Core model, or security owners as touched | `crates/volicord-core/src/methods/tests/mod.rs`; pipeline tests; conformance for cross-method baseline scenarios | [Request Lifecycle](request-lifecycle.md), [Design Patterns](design-patterns.md), and [Storage and Transactions](storage-and-transactions.md) |
| MCP adapter behavior | `crates/volicord-mcp/src/lib.rs`, `adapter.rs`, `routing.rs`, `tool_registry.rs`, `stdio.rs`, `local_http.rs`, `local_web_consent.rs`, and the `volicord mcp` dispatch in `crates/volicord-cli/src/main.rs` | [MCP Transport](../reference/mcp-transport.md); [Agent Connection](../reference/agent-connection.md) for verified connection context; [API Methods](../reference/api/methods.md) for public tool set | `crates/volicord-mcp/src/tests.rs`, `mcp_transport`, and `tests/integration/mcp_connection.rs` | [Request Lifecycle](request-lifecycle.md), [Architecture Decisions](decisions/README.md), and [Testing Strategy](testing-strategy.md) |
| Setup workflow and output | `crates/volicord-cli/src/setup_command.rs`, `setup_command/workflow.rs`, `setup_command/discovery.rs`, `setup_command/linking.rs`, `setup_command/shell_startup.rs`, `setup_command/interactive.rs`, `setup_command/output.rs`, and `doctor_command.rs` when diagnostics share setup facts | [Administrative CLI](../reference/admin-cli.md), with [Runtime Boundaries](../reference/runtime-boundaries.md), [MCP Transport](../reference/mcp-transport.md), and [Security](../reference/security.md) for adjacent process, location, and non-guarantee boundaries | Setup module tests and `binary_admin`; Store setup tests when bootstrap, registry, inspection, or schema initialization behavior changes | [Implementation Architecture](architecture.md), [Codebase Tour](codebase-tour.md), and [Testing Strategy](testing-strategy.md) |
| Connection provisioning, status, and output | `crates/volicord-cli/src/connection_command.rs`, `connection_command/service.rs`, `selection.rs`, `verification.rs`, `mcp_process.rs`, `connection_command/output/`, and `registration.rs` | [Administrative CLI](../reference/admin-cli.md), with [Agent Connection](../reference/agent-connection.md), [Runtime Boundaries](../reference/runtime-boundaries.md), and [MCP Transport](../reference/mcp-transport.md) for adjacent concerns | CLI module tests and `binary_admin`; `mcp_transport` or `mcp_connection` when preflight or MCP-visible behavior must be observed | [Implementation Architecture](architecture.md) and [Runtime Home and Product Repository separation](decisions/runtime-home-and-product-repository.md) |
| Guard integration files, capability records, and audit facts | `crates/volicord-cli/src/guard_integration/`, `connection_command/service.rs` for application and recording, `connection_command/output/` for status rendering, and `doctor_command.rs` for diagnostic consumption | [Administrative CLI](../reference/admin-cli.md#guard-hook-commands), with [Runtime Boundaries](../reference/runtime-boundaries.md), [Storage Records](../reference/storage-records.md), [MCP Transport](../reference/mcp-transport.md), and [Security](../reference/security.md) for diagnostic and non-guarantee boundaries | `binary_admin` guarded init/status tests, colocated guard integration tests, and doctor tests; avoid treating audit output as a security proof or approval record | [Implementation Architecture](architecture.md), [Codebase Tour](codebase-tour.md), and [Testing Strategy](testing-strategy.md) |
| Guard hook lifecycle behavior and host-native rendering | `crates/volicord-cli/src/guard_command.rs`, `guard_command/envelope.rs`, `tool_observation.rs`, `mutation.rs`, `prompt_command.rs`, `prompt_capture.rs`, `write_ticket.rs`, `render.rs`, and `guard_command/phase/` | [Administrative CLI](../reference/admin-cli.md#guard-hook-commands), with [Storage Records](../reference/storage-records.md), [Core Model](../reference/core-model.md), [Runtime Boundaries](../reference/runtime-boundaries.md), and [Security](../reference/security.md) for adjacent facts | `guard_command` tests; Core method or conformance tests when a hook path depends on owner-defined Core behavior | [Implementation Architecture](architecture.md), [Codebase Tour](codebase-tour.md), and [Testing Strategy](testing-strategy.md) |
| Host config adapters | `crates/volicord-cli/src/host_integration/`, especially `host_integration/codex/`, `host_integration/claude_code/`, `config_edit.rs`, `contracts.rs`, `generic.rs`, and `verification.rs` | [Administrative CLI](../reference/admin-cli.md), [Agent Connection](../reference/agent-connection.md), [Runtime Boundaries](../reference/runtime-boundaries.md), [MCP Transport](../reference/mcp-transport.md), and [Security](../reference/security.md) | Host adapter module tests and `binary_admin`; `guard_command` when host-native hook rendering changes; `mcp_transport` when launch/preflight paths change | [Implementation Architecture](architecture.md), [Codebase Tour](codebase-tour.md), and host-related decision records |
| Test fixture behavior | `crates/volicord-test-support/src/lib.rs`, `tests/conformance/`, `tests/integration/`, `crates/volicord-cli/tests/support/`, or colocated test helpers | The owner of each asserted fact; [Conformance](../reference/conformance.md) only for conformance scenario meaning and assertion routing | The consuming package's tests plus focused fixture tests | [Testing Strategy](testing-strategy.md) and [Codebase Tour](codebase-tour.md) |
| CLI integration test support | `crates/volicord-cli/tests/support/assertions.rs`, `binary_fixture.rs`, `fake_hosts.rs`, `fake_mcp.rs`, `guard_fixture.rs`, and `json.rs` | The owner of each asserted setup, connection, guard, MCP, or host-adapter fact | `binary_admin`, `guard_command`, `mcp_transport`, or the consuming CLI test target that uses the helper | [Testing Strategy](testing-strategy.md) and [Codebase Tour](codebase-tour.md) |
| Architecture Guide only | `docs/en/architecture-guide/`, `docs/ko/architecture-guide/`, and route metadata | The Architecture Guide page's `doc-index.yaml` owner scope; Reference owners only when exact behavior is being changed | Documentation checks; Cargo commands only when requested or needed for source verification | The paired page, [Architecture Guide](README.md), and `docs/doc-index.yaml` |

## Disagreement Handling

When implementation and documentation appear to disagree, classify the
disagreement before editing:

- If guide-level source-structure description differs from stable code, update
  the Architecture Guide page that owns that explanation.
- If code differs from API, schema, storage, security, error, scope, runtime, or
  Core authority owners, do not treat code as the new contract.
- If tests, fixtures, examples, or conformance scenario prose are the only
  place a behavior is expressed, treat that as an owner gap.
- If no owner can be identified, report the owner gap rather than placing the
  product rule in this guide.

Do not infer a product decision from a mismatch. The owner route identifies
where the decision belongs.

## Completion Check

Use this as an implementation and documentation-maintenance check. It is not
product acceptance, runtime conformance, close readiness, QA completion,
security proof, or residual-risk acceptance.

- Each changed behavior has a focused owner or an owner-gap report.
- The implementation path and boundary were identified before editing.
- Tests were selected for the changed layer.
- Architecture Guide documentation was updated when durable source structure,
  execution flow, storage boundary, or test strategy changed.
- Paired English and Korean documentation stayed aligned when maintained
  documents changed.
- No scratch notes, generated reports, runtime homes, SQLite files, fixture
  output, logs, or other transient artifacts remain in maintained documentation.
