# Developer documentation

This is the source-code learning entry point for developers who want to
understand the current Rust implementation. It explains where to start, what to
read next, and where exact product contracts live.

These pages teach implementation structure. They do not define or override
public API behavior, request or response schemas, storage effects, security
guarantees, runtime boundaries, Core authority semantics, or product contracts.
For exact behavior, follow the links to the focused Reference owners.

Volicord is the local work authority record for AI-assisted product
work. Core is the local authority record for Volicord state.

## Reading order

1. Workspace and crate responsibilities: start with the
   [Codebase Tour](codebase-tour.md). It names every Cargo workspace member,
   the first source file to open, important symbols, relevant tests, and the
   next component to read.
2. Representative request flow: read the
   [Request Lifecycle](request-lifecycle.md). It follows `volicord.status`,
   `volicord.intake`, and `volicord.prepare_write` from MCP `tools/call` through
   Core and Store behavior to the MCP response wrapper.
3. Architecture and boundaries: use
   [Implementation Architecture](architecture.md) for the durable workspace
   shape, dependency direction, execution-flow maps, administrative CLI setup
   flow, and code-to-owner routing.
4. Implementation patterns: read
   [Implementation Design Patterns](design-patterns.md) for recurring
   structures such as `CoreService`, `MethodPolicy`, method planning,
   `CoreStorageMutation`, injected time, opaque IDs, controlled enums,
   canonical request hashing, and shared test fixtures.
5. Storage and transaction concepts: read
   [Storage and Transactions](storage-and-transactions.md) for Runtime Home,
   registry and project databases, `CoreProjectStore`, method planning,
   mutation values, atomic commit, replay, artifact staging, and failure
   boundaries. Route exact storage questions to [Storage](../reference/storage.md),
   [Storage Effects](../reference/storage-effects.md),
   [Storage Records](../reference/storage-records.md),
   [Storage DDL](../reference/storage-ddl.md),
   [Artifact Storage](../reference/storage-artifacts.md), and
   [Storage Versioning](../reference/storage-versioning.md).
6. Test strategy: use [Testing Strategy](testing-strategy.md) to choose among
   module unit tests, Core method tests, binary tests, MCP integration tests,
   conformance implementation tests, and `volicord-test-support`.
7. Durable decisions: use [Architecture Decisions](decisions/README.md) for
   focused explanations of the Core/adapter boundary, planning before atomic
   mutation commit, and the separation of `Volicord Runtime Home` from
   `Product Repository`.
8. Change workflow: use the [Implementation Guide](change-guide.md) when you
   are ready to classify a change, locate the owner document, inspect the
   implementation boundary, choose validation, and update affected developer
   explanation.
9. Exact Reference contracts: use the
   [Reference Index](../reference/README.md) and
   [API Methods](../reference/api/methods.md). Learning documents can explain
   how the current code is arranged, but Reference documents own exact method
   behavior, schemas, storage effects, security wording, runtime boundaries,
   error meaning, and Core authority semantics.

## Learning documents versus owners

| Question | Start here | Exact owner route |
|---|---|---|
| Which crate should I open first? | [Codebase Tour](codebase-tour.md) | [Implementation Architecture](architecture.md) owns guide-level implementation structure. |
| How does a method call move through MCP, Core, Store, and back? | [Request Lifecycle](request-lifecycle.md) | Method behavior is owned by [API Methods](../reference/api/methods.md) and the linked method owner. |
| Why does Core not depend on CLI or MCP? | [Implementation Architecture](architecture.md) and [Core and adapter dependency boundary](decisions/core-adapter-boundary.md) | Dependency-boundary guidance stays in developer-learning docs; public behavior still routes to Reference owners. |
| Why are planners separate from Store commit? | [Implementation Design Patterns](design-patterns.md) and [Planning before atomic mutation commit](decisions/plan-and-atomic-commit.md) | Exact method behavior and storage effects route to method and storage owners. |
| What storage mutation is committed? | [Request Lifecycle](request-lifecycle.md) and [Storage and Transactions](storage-and-transactions.md) | Exact storage effects route to [Storage Effects](../reference/storage-effects.md) and adjacent storage owners. |
| Which test layer should I use? | [Testing Strategy](testing-strategy.md) | Tests verify owner-defined facts but do not own product contracts. |
| What should I edit for a change? | [Implementation Guide](change-guide.md) | The focused Reference owner selected from [Reference Index](../reference/README.md) or `docs/doc-index.yaml`. |

## Source-reading shortcuts

For public method work, the shortest useful source path is:

1. [`crates/volicord-types/src/methods.rs`](../../../crates/volicord-types/src/methods.rs)
2. [`crates/volicord-mcp/src/adapter.rs`](../../../crates/volicord-mcp/src/adapter.rs)
3. [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs)
4. [`crates/volicord-core/src/methods/`](../../../crates/volicord-core/src/methods/)
5. [`crates/volicord-store/src/core_pipeline.rs`](../../../crates/volicord-store/src/core_pipeline.rs)
6. [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs)
7. [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs)

For agent host setup and operator behavior, start instead with
[`crates/volicord-cli/src/main.rs`](../../../crates/volicord-cli/src/main.rs),
then
[`crates/volicord-cli/src/connection_command.rs`](../../../crates/volicord-cli/src/connection_command.rs),
[`crates/volicord-cli/src/host_integration/`](../../../crates/volicord-cli/src/host_integration/),
and
[`crates/volicord-cli/src/registration.rs`](../../../crates/volicord-cli/src/registration.rs).
For local User Channel behavior, continue with
[`crates/volicord-cli/src/user_command.rs`](../../../crates/volicord-cli/src/user_command.rs).

## Boundary reminders

- Core-facing code is independent of CLI and MCP adapter crates.
- `volicord-mcp` may use Store directly for startup and session validation. That
  direct Store use is not alternate public-method semantics.
- `Volicord Runtime Home` and `Product Repository` are separate locations.
- Tests verify owner-defined facts, but tests and fixtures are not product
  contract owners.
- Learning pages should name source files and symbols, not unstable line
  numbers.
