# Unified UserAction Request And Resolution Design

## Purpose

This design explains the current typed architecture for one durable
UserAction request lifecycle, one local-user resolution transition, and
separate agent-safe and User Channel projections.

## Design

Core uses strict shared `UserActionRequest` and `UserActionResolution` types
for all supported action families. `methods/user_action.rs` plans create,
resume, inbox projection, and resolution paths. Store's
`core_pipeline/user_actions.rs` owns effective-status reads, coherent inbox
resolution snapshots, immutable resolution insertion, and grouped mutation
application.

The MCP adapter calls only the request/resume path and projects
`AgentSafeUserActionRequestSummary`. The CLI inbox uses the nonserialized
`UserChannelInboxProjection` boundary, which includes the complete trusted
capture form for local presentation.

## Invariants

- One request has zero or one immutable resolution.
- Request creation/resume and resolution are separate Core operations.
- Effective status is derived from the stored resolution and current basis.
- Agent-facing projections never include the complete resolving form or
  user-only resolution body.
- The local inbox reads one coherent Store snapshot before planning
  resolution.
- Resolution replay cannot fork immutable authority state.

## Responsibility boundaries

`volicord-types` owns dependency-safe request, resolution, form, and summary
shapes. Core owns lifecycle and basis policy plus method planning. Store owns
strict records and snapshot consistency. CLI owns local presentation; MCP owns
the bounded agent projection.

## Execution flow

1. Core creates a request with its current basis and capture form or returns
   the explicit resume projection.
2. Store persists the request with the normal Core commit.
3. MCP returns only the agent-safe summary and continuation.
4. CLI requests the current inbox projection for one Task.
5. Store reads the effective record and pending form from one project
   snapshot.
6. Core validates the selected local-user answer and plans one immutable
   resolution.
7. Store commits the resolution, derived records, event, and replay response
   atomically.

## Failure behavior

Malformed stored variants, missing basis or form, stale coordinates, expiry,
existing conflicting resolution, invalid choice, or provenance mismatch fails
without partial derived state. Read-only status calculation does not mutate a
record merely because time has advanced.

## Scope exclusions

This design does not define action kinds, forms, option semantics, effective
status values, public method fields, delivery support, or authority meaning.
It does not make prompt capture or MCP transport a resolution channel.

## Implementation routes

- [`crates/volicord-types/src/schema.rs`](../../../../crates/volicord-types/src/schema.rs)
  and [`methods.rs`](../../../../crates/volicord-types/src/methods.rs):
  shared shapes and public result composition.
- [`crates/volicord-core/src/methods/user_action.rs`](../../../../crates/volicord-core/src/methods/user_action.rs)
  and [`lib.rs`](../../../../crates/volicord-core/src/lib.rs):
  request/resolution planning and internal projection boundaries.
- [`crates/volicord-store/src/core_pipeline/user_actions.rs`](../../../../crates/volicord-store/src/core_pipeline/user_actions.rs):
  strict reads, effective status, coherent snapshots, and mutations.
- [`crates/volicord-cli/src/user_command.rs`](../../../../crates/volicord-cli/src/user_command.rs)
  and [`crates/volicord-mcp/src/user_action_projection.rs`](../../../../crates/volicord-mcp/src/user_action_projection.rs):
  local and agent-facing projections.

## Reference owners

Exact behavior remains in
[Request User Action](../../reference/api/method-request-user-action.md),
[Resolve User Action](../../reference/api/method-resolve-user-action.md),
[User Action Schemas](../../reference/api/schema-user-action.md),
[Core Model](../../reference/core-model.md),
[Agent Connection](../../reference/agent-connection.md),
[Administrative CLI](../../reference/admin-cli.md), and
[Storage Records](../../reference/storage-records.md).
