# Unified UserAction Request And Resolution Design

## Purpose

This design explains the current typed architecture for one durable
UserAction request lifecycle, one local-user resolution transition, and
separate agent-safe and User Channel projections.

## Design

Core uses strict shared `UserActionRequest` and `UserActionResolution` types
for all supported action families. The dedicated
`volicord-user-action-service` crate owns semantic intent, validation,
canonical typed body construction, stable source identity, authority
normalization, lifecycle interpretation, resolution, persistence planning,
materialization, continuity facts, and adapter-neutral semantic projections.
It accepts small typed operation and persistence contexts and returns
service-owned typed results and errors. It does not import Core method,
pipeline, response, CLI, MCP, command-model, or presentation machinery.

`methods/user_action.rs` owns public request and resolution orchestration.
`methods/user_action_read.rs` owns Core admission checks and originating-result
replay around service-owned neutral facts. Other Core methods, including
`reconcile_changes.rs`, invoke the same service crate and decide how its typed
results participate in their own method plans and responses.

Store's `core_pipeline/user_actions.rs` owns effective-status reads, coherent
inbox resolution snapshots, immutable resolution insertion, grouped mutation
application, and focused decoding from physical JSON and stored values into
typed request and resolution records.

The MCP adapter calls the request/resume path, rereads
`CurrentUserActionFacts`, and constructs its own safe protocol projection. The
CLI consumes `PendingUserActionFacts`. `volicord-user-action-presentation`
projects the semantic `UserActionResolutionForm` into the current
`CliUserActionInboxItem`, closed availability and capture-path states, CLI JSON
Schema, and recovery instruction. Its commands come from a typed
`volicord-command-model` invocation derived from the actual Clap declaration.

## Invariants

- One request has zero or one immutable resolution.
- Request creation/resume and resolution are separate Core operations.
- Callers provide typed semantic intent and current domain facts; they do not
  construct canonical request JSON.
- Canonical request bodies and bases remain typed until the Store boundary.
- Effective status is derived from the stored resolution and current basis.
- Agent-facing projections never include the complete resolving form or
  user-only resolution body.
- Service fact results contain semantic coordinates, lifecycle status,
  availability, and safe resolution facts; they contain no command strings,
  presentation labels, CLI capture metadata, rendered instructions, or
  MCP-named envelopes.
- CLI resolution commands are derived from the same Clap declaration that
  parses them.
- The local inbox reads one coherent Store snapshot before planning
  resolution.
- Resolution replay cannot fork immutable authority state.

## Responsibility boundaries

`volicord-types` owns dependency-safe request, immutable resolution,
adapter-neutral resolution form, basis, summary shapes, product paths, and the
semantic `StateRecordRef` constructor, with no CLI presentation helpers.
`volicord-user-action-service` owns reusable UserAction semantics and depends
only on shared types, Store, and focused utility libraries. Core allocates
current IDs and timestamps, verifies invocation context, invokes the service,
participates in the Store mutation pipeline, and maps service errors and
results into request-specific responses. Store owns physical persistence,
strict row decoding, and snapshot consistency. The command model owns
canonical CLI invocation construction.
`volicord-user-action-presentation` owns the typed `Cli*` projection and CLI
JSON Schemas. CLI owns direct typed terminal rendering; MCP owns the bounded
protocol projection and adapter-specific failure mapping.

## Execution flow

1. A Core method supplies typed semantic action intent and current domain facts
   to the UserAction service.
2. Pure validation returns typed validated intent for the semantic combination
   and current coordinates.
3. The service acquires the remaining current domain facts through typed Store
   readers.
4. Pure body construction produces the canonical typed request body and basis.
5. Core supplies the durable request ID and operation identity. Service
   identity and materialization return a typed public request, effective Store
   record, and mutation plan.
6. The calling method decides how that result participates in its operation and
   response, or returns the explicit resume projection.
7. Store persists the request with the normal Core commit.
8. MCP returns only the agent-safe summary and continuation.
9. CLI requests neutral pending facts for one Task.
10. Store decodes effective typed records from one project snapshot and the
    service returns typed lifecycle and resolution-availability facts through
    the Core admission boundary.
11. Shared presentation creates the typed local CLI inbox item and obtains a
    canonical typed resolution invocation from the command model.
12. Core validates the selected local-user answer and plans one immutable
    resolution.
13. Store commits the resolution, derived records, event, and replay response
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
- [`crates/volicord-user-action-service/src/`](../../../../crates/volicord-user-action-service/src/):
  semantic model, typed errors, validation, canonical body construction,
  identity, Store-aware service, persistence planning, materialization,
  authority and lifecycle interpretation, resolution, continuity facts,
  neutral projections, and summaries.
- [`crates/volicord-core/src/methods/user_action.rs`](../../../../crates/volicord-core/src/methods/user_action.rs),
  [`user_action_read.rs`](../../../../crates/volicord-core/src/methods/user_action_read.rs),
  and [`user_action_continuity.rs`](../../../../crates/volicord-core/src/methods/user_action_continuity.rs):
  public-method orchestration, Core-owned admission and replay, error mapping,
  and persistence of service-derived continuity drafts.
- [`crates/volicord-core/src/methods/reconcile_changes.rs`](../../../../crates/volicord-core/src/methods/reconcile_changes.rs):
  reconciliation-specific orchestration that consumes the UserAction service.
- [`crates/volicord-store/src/core_pipeline/user_actions.rs`](../../../../crates/volicord-store/src/core_pipeline/user_actions.rs):
  strict reads, effective status, coherent snapshots, and mutations.
- [`crates/volicord-command-model/src/lib.rs`](../../../../crates/volicord-command-model/src/lib.rs)
  and [`crates/volicord-user-action-presentation/src/lib.rs`](../../../../crates/volicord-user-action-presentation/src/lib.rs):
  typed canonical CLI invocations and shared CLI-oriented presentation.
- [`crates/volicord-cli/src/user_command.rs`](../../../../crates/volicord-cli/src/user_command.rs)
  and [`crates/volicord-mcp/src/user_action_projection.rs`](../../../../crates/volicord-mcp/src/user_action_projection.rs):
  terminal and MCP protocol projections.

## Reference owners

Exact behavior remains in
[Request User Action](../../reference/api/method-request-user-action.md),
[Resolve User Action](../../reference/api/method-resolve-user-action.md),
[User Action Schemas](../../reference/api/schema-user-action.md),
[Core Model](../../reference/core-model.md),
[Agent Connection](../../reference/agent-connection.md),
[Administrative CLI](../../reference/admin-cli.md), and
[Storage Records](../../reference/storage-records.md).
