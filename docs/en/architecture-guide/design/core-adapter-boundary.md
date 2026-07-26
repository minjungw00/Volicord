# Core And Adapter Boundary Design

## Purpose

This design describes the current dependency and execution boundary among the
administrative command model, CLI and MCP adapters, Core policy and typed
result composition, Store, shared diagnostics, and repository tooling.

## Design

`volicord-command-model` contains the complete Clap declaration and
introspection model without command execution. `volicord-cli` and
`volicord-mcp` own their process, setup, transport, routing, and rendering
concerns and call `volicord-core` through typed interfaces.

Core owns common preflight, verified invocation context, method planning,
focused modules under `policy/`, replay handling, branch selection, and final
public result composition. `InvocationContext` carries an
`InvocationAuthority` consisting of either a typed local
`UserActionChannelKind` or an opaque `ValidatedAgentSession`; actor identity and
verification basis are derived from that authority. Public result declarations in
`volicord-types::methods` produce fields-only planning types and complete
result types from one declaration, so planners do not construct an incomplete
common envelope.

Store owns strict persisted-record decoding, aggregate reads, grouped mutation
application, and transaction mechanics. Shared diagnostic identity and report
shapes live in `volicord-types`; domain conversion, persistence, and rendering
remain with their domain owners. `xtask` consumes the command model and MCP
protocol registry for repository checks but remains outside runtime.

## Invariants

- Core normal and build dependencies target only groups classified as Core
  runtime dependencies. Core development dependencies may additionally target
  groups classified as Core development dependencies.
- Core owns no administrative command syntax, host launch arguments,
  host-specific configuration paths, or adapter rendering.
- Core entry points receive typed semantic authority rather than free-form
  actor or host labels.
- The command-model crate depends only on Clap and owns no execution.
- Method planners return typed fields and planned effects; the shared pipeline
  adds common branch facts only after the branch is known.
- Store does not derive method policy from adapter input.
- Adapter projections cannot widen a Core result or replace typed diagnostic
  identity with rendered prose.
- Repository validation does not become a runtime dependency or compatibility
  path.

## Responsibility boundaries

Adapters select the host integration, validate host-specific values, derive
trusted local semantic context, and translate transport data. Core owns
authority-aware planning and policy evaluation. Store owns persistence and
atomicity. `volicord-types` owns dependency-safe shared shapes.
`volicord-host-contract` and `volicord-mcp-protocol` own external-wire profiles.
`xtask` owns current repository validation and generation workflows.

## Execution flow

1. Command or transport syntax is parsed at its owning boundary.
2. The adapter resolves local Runtime Home, Connection, project, and operation
   context and constructs typed local-user or Agent Connection authority.
3. Core performs common preflight and method-specific planning with focused
   policy owners.
4. The selected branch remains typed as read-only, no-effect, dry-run,
   staging, or committed mutation.
5. Store reads or applies the planned effect.
6. Core composes the complete typed result; the adapter projects it for CLI or
   MCP without adding authority.

## Failure behavior

Syntax failures stay at the command or protocol boundary. Public rejections
remain structured Core responses. Persisted owner-data failures and unexpected
implementation failures retain their typed Store/Core/adapter routes.
Repository check failures are reported by `xtask` and do not trigger runtime
fallback behavior.

## Scope exclusions

This design does not define API behavior, CLI command semantics, schema
meaning, storage effects, or diagnostic code meanings. It does not expose
adapter internals through Core or use version-selected alternate module paths.

## Implementation routes

- [`crates/volicord-command-model/src/lib.rs`](../../../../crates/volicord-command-model/src/lib.rs):
  command syntax, visibility, traversal, synopses, and canonical invocations.
- [`Cargo.toml`](../../../../Cargo.toml) and
  [`xtask/src/architecture.rs`](../../../../xtask/src/architecture.rs):
  Core dependency eligibility and structural package-graph enforcement.
- [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs),
  [`methods/`](../../../../crates/volicord-core/src/methods/), and
  [`policy/`](../../../../crates/volicord-core/src/policy/): typed Core
  coordination, method planning, and focused policy.
- [`crates/volicord-types/src/methods.rs`](../../../../crates/volicord-types/src/methods.rs):
  fields-only and complete public result declarations.
- [`crates/volicord-store/src/core_pipeline/`](../../../../crates/volicord-store/src/core_pipeline/):
  project Store facade, aggregate ownership, and commit coordination.
- [`crates/volicord-mcp/src/`](../../../../crates/volicord-mcp/src/),
  [`crates/volicord-cli/src/`](../../../../crates/volicord-cli/src/), and
  [`xtask/src/`](../../../../xtask/src/): adapter and repository-tooling
  responsibilities.

## Reference owners

Exact behavior remains in [API Methods](../../reference/api/methods.md),
[Core Model](../../reference/core-model.md),
[MCP Transport](../../reference/mcp-transport.md),
[Administrative CLI](../../reference/admin-cli.md),
[Storage](../../reference/storage.md), and
[Failure Model](../../reference/failure-model.md).
