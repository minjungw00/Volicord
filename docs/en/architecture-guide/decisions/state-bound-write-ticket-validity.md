# Write Ticket validity is bound to relevant work state

## Context

A Write Ticket exists to connect one proposed Product Repository change to the
Task, Change Unit, scope, baseline, workspace, and approvals that justified it.
Binding validity primarily to a project-wide state counter or a fixed short
lifetime makes unrelated activity revoke that connection. Read-only status,
Evidence recording, or an unrelated user action can then force another ticket
even though the proposed write basis did not change.

That churn increases agent calls without improving the authority boundary. It
also hides the useful question: which relevant fact changed?

## Decision

Bind Write Ticket validity to an explicit snapshot of the relevant work state.
The persisted basis identifies the Task, current Change Unit, scope revision,
baseline, workspace context when applicable, and approval records on which the
authorization depends.

Core compares that basis with current relevant state before use. A compatible,
unconsumed ticket may be reused for a covered write intent. A ticket is consumed
when a recorded product-file change uses it. Time alone is not the default
authority boundary; a project-owned policy may add an idle limit without
conflating it with approval expiry.

The exact basis fields, compatibility rules, invalidation reasons, effect
values, error precedence, and storage constraints remain owned by
[Core Model](../../reference/core-model.md),
[Prepare-write](../../reference/api/method-prepare-write.md),
[Record-run](../../reference/api/method-record-run.md), and the storage
Reference family.

## Consequences

- Read-only and unrelated authority activity no longer has to invalidate a
  compatible ticket.
- A rejection can name the relevant basis change instead of exposing only a
  generic stale counter.
- Ticket reuse reduces speculative prepare-write calls while preserving
  single-use linkage to a product-changing Run.
- Persistence needs enough structured basis data to validate compatibility and
  explain invalidation.
- The storage-profile transition revokes or marks old active tickets stale
  rather than pretending their missing basis can be reconstructed.

## Non-goals

- A Write Ticket is still not filesystem permission, a lock, user acceptance,
  or proof that a write occurred.
- This decision does not make tickets transferable across Tasks, Change Units,
  workspaces, or approval bases.
- It does not remove optimistic concurrency from public mutations.
- It does not define the public validity or error schema in this ADR.

## Rejected alternatives

- Keeping project-wide state equality as the primary rule was rejected because
  unrelated mutations cause false invalidation.
- Keeping a fixed short lifetime as the default was rejected because elapsed
  time does not identify a changed authority fact.
- Making a ticket indefinitely reusable was rejected because a recorded write
  needs one durable consumption relationship and later write phases require a
  fresh compatibility check.

## Relevant implementation

- [`crates/volicord-types/src/schema.rs`](../../../../crates/volicord-types/src/schema.rs):
  shared Write Ticket result and status projections.
- [`crates/volicord-core/src/methods/prepare_write.rs`](../../../../crates/volicord-core/src/methods/prepare_write.rs)
  and [`crates/volicord-core/src/methods/record_run.rs`](../../../../crates/volicord-core/src/methods/record_run.rs):
  basis comparison, reuse, issuance, and consumption planning.
- [`crates/volicord-store/src/schema/project.sql`](../../../../crates/volicord-store/src/schema/project.sql)
  and Store write-ticket accessors: persisted validity basis and consumption
  constraints.

## Related tests and Reference owners

- Write-ticket lifecycle coverage in
  [`crates/volicord-core/src/methods/tests/prepare_write.rs`](../../../../crates/volicord-core/src/methods/tests/prepare_write.rs),
  [`crates/volicord-core/src/methods/tests/record_run.rs`](../../../../crates/volicord-core/src/methods/tests/record_run.rs),
  [`crates/volicord-store/tests/storage_ddl_contract.rs`](../../../../crates/volicord-store/tests/storage_ddl_contract.rs),
  and [`tests/conformance/baseline.rs`](../../../../tests/conformance/baseline.rs).
- [Core Model](../../reference/core-model.md),
  [Prepare-write](../../reference/api/method-prepare-write.md),
  [Record-run](../../reference/api/method-record-run.md),
  [Storage Records](../../reference/storage-records.md),
  [Storage Effects](../../reference/storage-effects.md),
  [Storage DDL](../../reference/storage-ddl.md), and
  [Storage Versioning](../../reference/storage-versioning.md).
