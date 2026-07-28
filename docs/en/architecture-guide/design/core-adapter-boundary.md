# Core And Adapter Boundary Design

## Purpose

This design describes the current dependency and execution boundary among the
administrative command model, CLI and MCP adapters, Core policy and typed
result composition, Store, shared diagnostics, and repository tooling.

## Design

`volicord-command-model` contains the complete Clap declaration and
introspection model without command execution. Its typed invocation builders
derive command paths and option spellings from that same declaration.
`volicord-user-action-presentation` turns adapter-neutral UserAction facts into
the shared current CLI inbox item and CLI recovery instruction.
`volicord-cli` and `volicord-mcp` own their process, setup, transport, routing,
and final rendering concerns and call `volicord-core` through typed
interfaces.

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
protocol registry and wire-contract descriptors for repository checks but
remains outside runtime.

## Invariants

- Core normal and build dependencies target only groups classified as Core
  runtime dependencies. Core development dependencies may additionally target
  groups classified as Core development dependencies.
- Core owns no administrative command syntax, host launch arguments,
  host-specific configuration paths, presentation labels, rendered recovery
  instructions, or adapter protocol envelopes.
- Core entry points receive typed semantic authority rather than free-form
  actor or host labels.
- The command-model crate depends only on Clap and owns no execution.
- Canonical CLI invocation builders inspect and validate against the same Clap
  declaration used by the binary; adapters do not reconstruct command grammar.
- Shared UserAction presentation depends on the command model and shared types,
  not on Core, Store, CLI, or MCP.
- Method planners return typed fields and planned effects; the shared pipeline
  adds common branch facts only after the branch is known.
- Result, rejection, and preview metadata use distinct closed base types.
  Their fixed discriminants and effect facts are encoded by the shared types,
  and every public branch and base rejects unknown fields.
- One method declaration owns the request type, result type, exact public
  response family, semantic contract IDs, generated schemas, and replay
  eligibility. Core branch selection and stored-result validation consume that
  declaration rather than parallel method lists.
- A required infrastructure dependency that cannot produce a method result
  returns a typed neutral Core operational error outside every public response
  branch.
- `volicord-types` contains no MCP request, result, error, structured-content,
  tool-envelope, or JSON-RPC wire types.
- `volicord-mcp-wire` is reachable only from the matching MCP adapter and
  validation tooling or tests. Core, shared types, Store, UserAction service,
  CLI, and presentation packages cannot depend on it.
- Store does not derive method policy from adapter input.
- Adapters decode Core output through the selected method's exact response
  family before projection. Adapter projections cannot widen that family,
  widen a Core result, or replace typed diagnostic identity with rendered
  prose.
- Public errors are constructed through the invariant-preserving shared type.
  Core and adapters consume its code-derived category and do not keep an
  adapter-local public code/category mapping.
- Repository validation does not become a runtime dependency or compatibility
  path.

## Responsibility boundaries

Adapters select the host integration, validate host-specific values, derive
trusted local semantic context, and translate transport data. Core owns
authority-aware planning and policy evaluation. Store owns persistence and
atomicity. `volicord-types` owns dependency-safe shared shapes.
`volicord-host-contract` owns semantic host contracts.
`volicord-mcp-protocol` owns protocol profiles and semantic capabilities.
`volicord-mcp-wire` owns exact MCP fields, error identities, structured
content, JSON-RPC and tool envelopes, and generated MCP schemas.
`volicord-user-action-presentation` owns shared CLI-oriented UserAction
presentation. CLI owns terminal rendering, while MCP owns its protocol result
projection and maps neutral Core availability failures at that boundary.
`xtask` owns current repository validation and generation workflows.

## Execution flow

1. Command or transport syntax is parsed at its owning boundary.
2. The adapter resolves local Runtime Home, Connection, project, and operation
   context and constructs typed local-user or Agent Connection authority.
3. Core performs common preflight and method-specific planning with focused
   policy owners.
4. The selected branch remains typed as read-only, no-effect, dry-run,
   staging, or committed mutation.
5. Store reads or applies the planned effect. Operational unavailability
   returns through the neutral Core error path with typed operation, resource,
   and retryability facts.
6. Core composes public method results or returns adapter-neutral semantic
   facts for internal adapter reads.
7. Shared presentation derives any CLI inbox item or recovery instruction from
   those facts through a typed command-model invocation.
8. CLI renders local output and MCP constructs its own protocol projection
   without adding authority.

## Failure behavior

Syntax failures stay at the command or protocol boundary. Public rejections
remain structured Core responses. A required Store or infrastructure failure
that prevents any method result is a typed Core operational error and never a
public rejection or successful response. CLI maps that neutral failure to its
runtime diagnostic contract. MCP maps it to its capability-selected protocol
carrier and MCP-owned wire identity. Persisted owner-data corruption and
unexpected implementation failures retain their typed Store/Core/adapter routes.
Repository check failures are reported by `xtask` and do not trigger runtime
fallback behavior.

## Scope exclusions

This design does not define API behavior, CLI command semantics, schema
meaning, storage effects, or diagnostic code meanings. It does not expose
adapter internals through Core or use version-selected alternate module paths.

## Implementation routes

- [`crates/volicord-command-model/src/lib.rs`](../../../../crates/volicord-command-model/src/lib.rs):
  command syntax, visibility, traversal, synopses, canonical invocations, and
  typed inbox-resolution invocation construction.
- [`crates/volicord-user-action-presentation/src/lib.rs`](../../../../crates/volicord-user-action-presentation/src/lib.rs):
  shared CLI inbox, availability, and recovery-instruction presentation from
  adapter-neutral facts.
- [`Cargo.toml`](../../../../Cargo.toml) and
  [`xtask/src/architecture.rs`](../../../../xtask/src/architecture.rs):
  Core dependency eligibility and structural package-graph enforcement.
- [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs),
  [`methods/`](../../../../crates/volicord-core/src/methods/), and
  [`policy/`](../../../../crates/volicord-core/src/policy/): typed Core
  coordination, method planning, and focused policy.
- [`crates/volicord-types/src/methods.rs`](../../../../crates/volicord-types/src/methods.rs):
  fields-only and complete adapter-neutral public result declarations.
- [`crates/volicord-mcp-protocol/src/lib.rs`](../../../../crates/volicord-mcp-protocol/src/lib.rs):
  exact profile selection and semantic capabilities.
- [`crates/volicord-mcp-wire/src/`](../../../../crates/volicord-mcp-wire/src/):
  MCP wire values, envelopes, serialization, schemas, and contract
  descriptors.
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
