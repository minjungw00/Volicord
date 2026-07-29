# State-Bound Write Ticket Validity Design

## Purpose

This design explains where the implementation constructs, evaluates, reuses,
invalidates, and consumes Write Tickets against current Core and Store facts.

## Design

`prepare_write` loads the current Task, Change Unit, scope, workspace,
approval, workflow-policy, and normalized path facts. Focused Core policy in
`policy/write_ticket.rs` constructs and evaluates the ticket basis.
`core_pipeline/write_tickets.rs` owns strict ticket reads and grouped mutation
application.

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

Core methods own request-specific planning. Core workflow, path, access, and
write-ticket policy modules own current authority evaluation over typed facts.
Store keeps the physical ticket row private and strictly decodes status,
validity basis, attempt scope, Product Repository path collections, timestamps,
and redundant owner coordinates before returning a typed record. Store also
owns ticket queries, invalidation persistence, and consumption mutation. Guard
supplies observations but does not widen the ticket basis.

## Execution flow

1. Common preflight validates actor, operation category, project, and request
   shape.
2. `prepare_write` loads the current work, policy, workspace, path, and
   approval facts.
3. Core policy computes the normalized current write-authority basis.
4. Store returns compatible ticket candidates and Core selects reuse or new
   issuance.
5. A later protected operation repeats the current compatibility evaluation.
6. Store commits ticket consumption with the protected mutation.

## Failure behavior

Missing current work, stale or corrupt policy, path normalization failure,
approval mismatch, workspace mismatch, explicit revocation, incompatible
basis, or ambiguous ticket selection prevents reuse or consumption without a
partial protected effect. Exact replay does not consume the ticket again.

## Scope exclusions

This design does not define Write Ticket product meaning, public request
fields, timeout policy, invalidation values, storage effects, user approval, or
OS write enforcement. A ticket is not actor identity or a transferable
capability.

## Implementation routes

- [`crates/volicord-core/src/methods/prepare_write.rs`](../../../../crates/volicord-core/src/methods/prepare_write.rs)
  and [`record_run.rs`](../../../../crates/volicord-core/src/methods/record_run.rs):
  issue/reuse and protected consumption planning.
- [`crates/volicord-core/src/policy/write_ticket.rs`](../../../../crates/volicord-core/src/policy/write_ticket.rs)
  and [`workflow.rs`](../../../../crates/volicord-core/src/policy/workflow.rs):
  typed current-basis evaluation.
- [`crates/volicord-types/src/product_path.rs`](../../../../crates/volicord-types/src/product_path.rs):
  shared typed product-path normalization and containment.
- [`crates/volicord-store/src/core_pipeline/write_tickets.rs`](../../../../crates/volicord-store/src/core_pipeline/write_tickets.rs):
  strict records, queries, and grouped mutation application.

## Reference owners

Exact behavior remains in [Core Model](../../reference/core-model.md),
[Prepare Write](../../reference/api/method-prepare-write.md),
[Record Run](../../reference/api/method-record-run.md),
[Storage Records](../../reference/storage-records.md),
[Storage Effects](../../reference/storage-effects.md),
[Storage Versioning](../../reference/storage-versioning.md), and
[Security](../../reference/security.md).
