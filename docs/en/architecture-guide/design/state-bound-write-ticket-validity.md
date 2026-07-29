# State-Bound Write Ticket Validity Design

## Purpose

This design explains where the implementation constructs, evaluates, reuses,
invalidates, and consumes Write Tickets against current Core and Store facts.

## Design

`prepare_write` loads the current Task, Change Unit, scope, workspace,
approval, workflow-policy, and normalized path facts. The focused
`write_ticket/` owner acquires and normalizes current facts, evaluates policy,
plans issue or reuse, and projects the typed outcome through its `facts.rs`,
`policy.rs`, `planning.rs`, and `projection.rs` modules.
`planning.rs` owns the distinct, unpersisted `PlannedWriteTicket`. For new
issuance, the same validated plan supplies both the response projection and
the fully typed Store insertion; a dry-run plan has no durable ticket ID and
cannot supply an insertion.
For a protected Record Run, `write_ticket/admission.rs` accepts the typed
operation, Task, Change Unit, invocation, observed-change, and current policy
facts and returns the admitted attempt scope or a semantic admission error.
`core_pipeline/write_tickets.rs` solely owns the physical ticket table,
columns, row projection, canonical decoder, persisted invariants, strict
normal and transaction-scoped reads, focused typed authority views, and
grouped mutation application. The decoder returns the opaque
`StoredWriteTicket`; Core and adapters can inspect semantic accessors but
cannot construct, mutate, or destructure its private fields.

Ticket lookup occurs only after structural preconditions. Reuse compares the
stored basis with the current typed facts rather than relying on an unrelated
global state counter. A protected Run mutation consumes the selected ticket in
the same Store commit as the Run and its associated effects.

## Invariants

- A ticket is bound to one current work and write-authority basis.
- Ticket IDs, stored status, or age alone do not establish current validity.
- Structural request and current Change Unit validation precede lookup.
- Reuse does not widen paths, approvals, operations, or authority.
- Relevant mismatch makes an active ticket unusable; unrelated state changes
  do not.
- Successful consumption is atomic with the protected committed mutation.

## Responsibility boundaries

Core methods own request-specific orchestration and response composition. The
focused Write Ticket owner owns reusable fact acquisition, policy evaluation,
issuance or reuse planning, Record Run admission, and projection over typed
facts. The Record Run planner supplies semantic facts and does not interpret
stored path JSON or construct ticket policy independently. Store keeps the
physical ticket row private and strictly decodes status, validity basis,
attempt scope, Product Repository path collections, timestamps, and redundant
owner coordinates before returning a `StoredWriteTicket`. Relationships among
physical fields are validated as closed Write Ticket aggregate invariants.
Core validates semantic planning invariants while constructing a
`PlannedWriteTicket`; those checks are separate from Store-owned persisted
physical validation. Projection has explicit paths for a plan, a stored
ticket, and projected post-consumption state instead of mutating a stored
record.
Store also owns ticket queries, invalidation persistence, and consumption
mutation. Workflow-policy persistence receives only a focused typed authority
view produced from the validated record; it never queries or decodes a ticket
row. Guard supplies observations but does not widen the ticket basis.

## Execution flow

1. Common preflight validates actor, operation category, project, and request
   shape.
2. `prepare_write` loads the current work, policy, workspace, path, and
   approval facts.
3. Core policy computes the normalized current write-authority basis.
4. Store returns compatible ticket candidates and Core selects reuse or new
   issuance.
5. New issuance creates one `PlannedWriteTicket`; response projection and
   `WriteTicketInsert` derive from it. Dry-run keeps the plan ID-less and
   performs no Store insertion, while reuse reads a `StoredWriteTicket`.
6. For Record Run, `write_ticket/admission.rs` repeats the current
   compatibility evaluation from typed operation facts and returns the admitted
   scope.
7. Store commits ticket consumption with the protected mutation.

## Failure behavior

Missing current work, stale or corrupt policy, path normalization failure,
approval mismatch, workspace mismatch, explicit revocation, incompatible
basis, or ambiguous ticket selection prevents reuse or consumption without a
partial protected effect. Exact replay does not consume the ticket again.
Malformed physical fields and persisted cross-field disagreement are Store
corruption. Expiry, path, operation, or current-policy mismatch evaluated from
an internally valid typed ticket is a semantic policy outcome, not persisted
corruption.

## Scope exclusions

This design does not define Write Ticket product meaning, public request
fields, timeout policy, invalidation values, storage effects, user approval, or
OS write enforcement. A ticket is not actor identity or a transferable
capability.

## Implementation routes

- [`crates/volicord-core/src/methods/prepare_write.rs`](../../../../crates/volicord-core/src/methods/prepare_write.rs)
  and [`record_run.rs`](../../../../crates/volicord-core/src/methods/record_run.rs):
  request-specific issue/reuse and protected-consumption orchestration.
- [`crates/volicord-core/src/write_ticket/`](../../../../crates/volicord-core/src/write_ticket/)
  and [`workflow.rs`](../../../../crates/volicord-core/src/policy/workflow.rs):
  typed fact acquisition, current-basis evaluation, `PlannedWriteTicket`
  construction, protected Record Run admission, and explicit planned, stored,
  or projected-consumption projection.
- [`crates/volicord-core/src/write_ticket/tests/record_run_admission.rs`](../../../../crates/volicord-core/src/write_ticket/tests/record_run_admission.rs):
  focused Record Run ticket admission and no-effect rejection coverage.
- [`crates/volicord-types/src/product_path.rs`](../../../../crates/volicord-types/src/product_path.rs):
  shared typed product-path normalization and containment.
- [`crates/volicord-store/src/core_pipeline/write_tickets.rs`](../../../../crates/volicord-store/src/core_pipeline/write_tickets.rs):
  physical ownership, canonical decoding into opaque `StoredWriteTicket`
  values, typed insertion serialization, authority views, queries, and grouped
  mutation application.
- [`crates/volicord-store/src/workflow_records.rs`](../../../../crates/volicord-store/src/workflow_records.rs):
  workflow-policy persistence and semantic evaluation over the typed Write
  Ticket authority view.

## Reference owners

Exact behavior remains in [Core Model](../../reference/core-model.md),
[Prepare Write](../../reference/api/method-prepare-write.md),
[Record Run](../../reference/api/method-record-run.md),
[Storage Records](../../reference/storage-records.md),
[Storage Effects](../../reference/storage-effects.md),
[Storage Versioning](../../reference/storage-versioning.md), and
[Security](../../reference/security.md).
