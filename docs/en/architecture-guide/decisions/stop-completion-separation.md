# Session Stop and Task completion are separate outcomes

## Context

A host Stop event means that an agent session or turn is ending. It does not
mean that the current `Task` is complete. Treating every incomplete Task as a
reason to deny Stop couples host lifecycle control to Close Status and can make
a host retry the same Stop repeatedly. It also prevents an honest session exit
when the next actor is the user or work must resume later.

Volicord still needs to prevent a model from presenting blocked work as
complete. That is a completion-disclosure concern, not a reason to keep a host
process alive.

## Decision

Represent Stop permission and completion-claim permission as separate facts.
The managed Stop path allows the session to end, refreshes authoritative state
when possible, and records a bounded receipt that discloses the selected Task,
Close Status, next actor, and whether a completion claim is currently allowed.

If authority refresh fails, Stop still finishes and the result discloses that
the authority state could not be verified. Close blockers continue to suppress
completion claims. Deterministically confirmed out-of-scope or unauthorized
writes remain eligible for the separate pre-action enforcement path.

The exact Stop result, receipt persistence, blocker projection, failure
routing, and completion field remain owned by
[Administrative CLI](../../reference/admin-cli.md),
[Core Model](../../reference/core-model.md), the state schema, Agent Connection,
and storage Reference owners.

## Consequences

- Waiting for a user, missing Evidence, or another close blocker no longer
  traps the host in a Stop retry loop.
- Host adapters can acknowledge lifecycle completion while fixed UI and final
  output still disclose that the Task is incomplete.
- Live-host tests assert one completed Stop rather than instructing an operator
  to interrupt a second attempt.
- Stop receipts become diagnostic continuity records, not terminal Task
  mutations or substitutes for `close_task`.
- Pre-action write enforcement and completion disclosure can evolve without
  overloading one allow/deny bit.

## Non-goals

- Allowing Stop does not close, cancel, or supersede a Task.
- It does not waive Evidence, user judgment, acceptance, or residual-risk
  requirements.
- It does not weaken a deterministic pre-action write denial.
- A receipt does not prove that the host displayed it or that a model reported
  it faithfully.

## Rejected alternatives

- Denying Stop until Close Status is clear was rejected because session
  lifecycle and Task lifecycle have different next actors and time horizons.
- Treating host process exit as implicit close was rejected because it bypasses
  Core close authority and user-owned decisions.
- Allowing Stop without recording or disclosing the incomplete state was
  rejected because it would make honest continuation harder after the session
  ends.

## Relevant implementation

- [`crates/volicord-cli/src/guard_command.rs`](../../../../crates/volicord-cli/src/guard_command.rs)
  and Guard integration modules: host Stop handling and receipt projection.
- [`crates/volicord-core/src/methods/close_task.rs`](../../../../crates/volicord-core/src/methods/close_task.rs):
  authoritative Close Status separate from host lifecycle.
- [`crates/volicord-store/src/guards.rs`](../../../../crates/volicord-store/src/guards.rs):
  durable guard observations and diagnostic receipts.
- [`crates/volicord-mcp/src/stdio.rs`](../../../../crates/volicord-mcp/src/stdio.rs):
  authority refresh and adapter-visible next-action projection.

## Related tests and Reference owners

- [`crates/volicord-cli/tests/guard_command.rs`](../../../../crates/volicord-cli/tests/guard_command.rs),
  [`crates/volicord-cli/tests/live_host_smoke.rs`](../../../../crates/volicord-cli/tests/live_host_smoke.rs),
  and close coverage in
  [`tests/conformance/baseline.rs`](../../../../tests/conformance/baseline.rs).
- [Administrative CLI](../../reference/admin-cli.md),
  [Core Model](../../reference/core-model.md),
  [Close-task](../../reference/api/method-close-task.md),
  [API State Schemas](../../reference/api/schema-state.md),
  [Agent Connection](../../reference/agent-connection.md), and
  [Storage Records](../../reference/storage-records.md).
