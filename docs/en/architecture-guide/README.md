# Architecture Guide

The Architecture Guide is the source-code learning entry point for
implementers, reviewers, and source-code learners who need to understand the
current Rust workspace. It routes workspace structure, exact source-path
responsibilities, request flow, storage and transaction boundaries, design
patterns, test strategy, durable decisions, and implementation change workflow.

Use these pages to learn how the implementation is arranged and why durable
boundaries exist. Exact public API behavior, request or response schemas,
storage effects, security guarantees, runtime boundaries, Core authority
semantics, and other product contracts live in the focused Reference owners.

Volicord is the local work authority record for AI-assisted product
work. Core is the local authority record for Volicord state.

## Choose a reading path

You do not need to read every page in order. Start with the route that matches
your task.

| Goal | Route | What it answers |
|---|---|---|
| Learn the workspace | [Codebase Tour](codebase-tour.md) -> [Implementation Architecture](architecture.md) -> [Source Map](source-map.md) | Which crates to read, how dependencies point, and which module owns an implementation responsibility. |
| Follow an administrative workflow | [CLI Workflows](cli-workflows.md) -> [Source Map](source-map.md) | How setup, connection, host, guard, and diagnostic paths are assembled, then where each part lives. |
| Follow a public method call | [Request Lifecycle](request-lifecycle.md) -> [Implementation Design Patterns](design-patterns.md) -> [Storage and Transactions](storage-and-transactions.md) | How MCP, Core, and Store cooperate, which structures recur, and where persistence begins. |
| Plan a change | [Implementation Guide](change-guide.md) -> [Testing Strategy](testing-strategy.md) -> [Architecture Decisions](decisions/README.md) | Which owner and source area to inspect, which test layer to use, and why durable boundaries exist. |
| Check exact behavior | [Reference Index](../reference/README.md) -> [API Methods](../reference/api/methods.md) | Which focused Reference document owns the API, schema, storage, security, runtime, error, or Core authority detail. |

## Source-reading shortcuts

For complete source path responsibilities, use the [Source Map](source-map.md).
The shortcuts below are common first-open paths for frequent implementation
questions.

For public method work, the shortest useful source path is:

1. [`crates/volicord-types/src/methods.rs`](../../../crates/volicord-types/src/methods.rs)
2. [`crates/volicord-mcp/src/adapter.rs`](../../../crates/volicord-mcp/src/adapter.rs)
3. [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs)
4. [`crates/volicord-core/src/methods/`](../../../crates/volicord-core/src/methods/)
5. [`crates/volicord-store/src/core_pipeline.rs`](../../../crates/volicord-store/src/core_pipeline.rs)
6. [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs)
7. [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs)

For agent host setup and operator behavior, read
[CLI Workflows](cli-workflows.md) for the execution-flow boundaries, then start
with
[`crates/volicord-cli/src/main.rs`](../../../crates/volicord-cli/src/main.rs),
then
[`crates/volicord-cli/src/connection_command.rs`](../../../crates/volicord-cli/src/connection_command.rs),
[`crates/volicord-cli/src/connection_command/service.rs`](../../../crates/volicord-cli/src/connection_command/service.rs),
[`crates/volicord-cli/src/host_integration/`](../../../crates/volicord-cli/src/host_integration/),
[`crates/volicord-store/src/bootstrap.rs`](../../../crates/volicord-store/src/bootstrap.rs),
and
[`crates/volicord-store/src/agent_connections.rs`](../../../crates/volicord-store/src/agent_connections.rs).
For local User Channel behavior, continue with
[`crates/volicord-cli/src/user_command.rs`](../../../crates/volicord-cli/src/user_command.rs)
and
[`crates/volicord-core/src/methods/judgment.rs`](../../../crates/volicord-core/src/methods/judgment.rs).

## Boundary reminders

- Core-facing code is independent of CLI and MCP adapter crates.
- `volicord-mcp` may use Store directly for startup and session validation. That
  direct Store use is not alternate public-method semantics.
- `Volicord Runtime Home` and `Product Repository` are separate locations.
- Tests verify owner-defined facts, but tests and fixtures are not product
  contract owners.
- Learning pages should name source files and symbols, not unstable line
  numbers.
