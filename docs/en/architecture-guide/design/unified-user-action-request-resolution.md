# Unified UserAction Request And Resolution Design

## Purpose

This design explains the current typed architecture for one durable
UserAction request lifecycle, one local-user resolution transition, and
separate agent-safe and User Channel projections.

## Design

Core uses strict shared `UserActionRequest` and `UserActionResolution` types
for all supported action families. `user_action/model.rs`,
`validation.rs`, `body.rs`, and `identity.rs` own semantic intent, pure
validation, canonical typed request construction, and stable source identity.
`service.rs` acquires current Store facts, while `materialization.rs` and
`persistence.rs` form the public request and exact Store mutation input.
`authority.rs`, `lifecycle.rs`, and `resolution.rs` own strict authority
decoding, semantic lifecycle interpretation, and typed resolution behavior.
`reader.rs` and `projection.rs` own adapter-neutral current and pending fact
reads. `methods/user_action.rs` owns public request and resolution
orchestration. Other Core methods, including `reconcile_changes.rs`, invoke
these shared responsibility owners directly and decide how typed results
participate in their own operations.

Store's `core_pipeline/user_actions.rs` owns effective-status reads, coherent
inbox resolution snapshots, immutable resolution insertion, and grouped
mutation application.

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
- Core fact results contain semantic coordinates, lifecycle status,
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
adapter-neutral resolution form, basis, and summary shapes, with no CLI
presentation helpers. The Core `user_action` modules own reusable semantic
validation, construction, identity, materialization, authority interpretation,
lifecycle policy, reads, and adapter-neutral semantic facts. Individual method
modules own request-specific operation and response composition. Store owns
strict records and snapshot consistency. The command model owns canonical CLI
invocation construction.
`volicord-user-action-presentation` owns the typed `Cli*` projection and CLI
JSON Schemas. CLI owns direct typed terminal rendering; MCP owns the bounded
protocol projection and adapter-specific failure mapping.

## Execution flow

1. A Core method supplies typed semantic action intent and current domain facts
   to the UserAction service.
2. Pure validation returns typed validated intent for the semantic combination
   and current coordinates.
3. Store-aware service code acquires the remaining current domain facts.
4. Pure body construction produces the canonical typed request body and basis.
5. Identity and materialization allocate the request ID and return a typed
   public request, effective Store record, and mutation plan.
6. The calling method decides how that result participates in its operation and
   response, or returns the explicit resume projection.
7. Store persists the request with the normal Core commit.
8. MCP returns only the agent-safe summary and continuation.
9. CLI requests neutral pending facts for one Task.
10. Store reads the effective records from one project snapshot and Core
   returns typed lifecycle and resolution-availability facts.
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
- [`crates/volicord-core/src/methods/user_action.rs`](../../../../crates/volicord-core/src/methods/user_action.rs)
  and [`lib.rs`](../../../../crates/volicord-core/src/lib.rs):
  direct public-method orchestration and the public adapter-neutral fact
  surface.
- [`crates/volicord-core/src/user_action/`](../../../../crates/volicord-core/src/user_action/):
  responsibility-owned semantic model, validation, typed body and identity
  construction, Store-aware service, materialization and persistence mapping,
  authority and lifecycle interpretation, resolution, neutral reads,
  projection, and summaries.
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
