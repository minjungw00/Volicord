# Write Ticket validity is bound to relevant work state

## Context

A Write Ticket exists to connect one proposed Product Repository change to the
Task, Change Unit, scope, baseline, workspace, and approvals that justified it.
That connection also depends on the project-owned workflow policy under which
the write was authorized. If the policy changes which paths qualify for Light
work, which control level applies, whether pre-write approval is required, or
how long a ticket remains usable, an earlier ticket must not carry the old
authorization across the new boundary.

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
authorization depends. It also stores the normalized
`write_authority_fingerprint` derived from the current authoritative project
policy.

The fingerprint is intentionally narrower than the whole canonical policy. Its
canonical basis contains the `volicord-write-authority-v1` schema discriminator,
the direct and work control defaults, the Light enabled flag, path-count limit,
allowed and denied path patterns, Light final-acceptance policy, and Write
Ticket idle timeout. Pattern arrays are sorted and deduplicated before
canonicalization, and the canonical JSON is stored as a lowercase
`sha256:<hex>` digest. Detective behavior and repository,
connection, MCP, host-hook, and other integration bindings do not participate
because Core does not consult them to authorize a write.

Core compares that basis with current relevant state before use. A compatible,
unconsumed ticket may be reused for a covered write intent. A ticket is consumed
when a recorded product-file change uses it. Time alone is not the default
authority boundary; a project-owned policy may add an idle limit without
conflating it with approval expiry.

Applying any policy whose normalized write authority differs from the prior
one atomically marks the active Task for reevaluation and invalidates
incompatible active tickets with `explicit_revoke`. This is conservative for
both tightening and relaxation: compatibility is not inferred across a changed
fingerprint. The active-Task mark is created even when the stored control and
acceptance levels do not need to rise, so the next `volicord.prepare_write`
still evaluates the requested operation and paths under the current policy.
A normalized-equivalent write authority does not invalidate tickets merely
because the whole policy document changed or was reapplied.

The cooperative Guard path excludes a missing or mismatched policy binding from
active candidates. Core independently performs the same current-policy check
in `volicord.record_run`, and Store checks it again inside the consumption
transaction. A new prepare-write decision can therefore raise control to
`sensitive` and require a new sensitive-action approval. Final acceptance after
the write is a separate user judgment and cannot replace that pre-write
approval.

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
- An active legacy ticket with no current policy binding fails closed and must
  be reissued. A consumed historical ticket remains inspectable instead of
  being retroactively rewritten.
- The binding fits in the existing `validity_basis_json` record. The storage
  profile remains `baseline_sqlite_v7`; this decision requires no offline-copy
  storage upgrade.

## Non-goals

- A Write Ticket is still not filesystem permission, a lock, user acceptance,
  an OS sandbox, a tamper-proof audit log, or proof that a write was correct or
  occurred.
- This decision does not make tickets transferable across Tasks, Change Units,
  workspaces, policy authorities, or approval bases.
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
- [`crates/volicord-store/src/workflow_records.rs`](../../../../crates/volicord-store/src/workflow_records.rs):
  normalized write-authority derivation, policy-apply reevaluation, and atomic
  active-ticket invalidation.
- [`crates/volicord-cli/src/guard_command/context.rs`](../../../../crates/volicord-cli/src/guard_command/context.rs)
  and [`crates/volicord-cli/src/guard_command/write_ticket.rs`](../../../../crates/volicord-cli/src/guard_command/write_ticket.rs):
  cooperative Guard candidate selection and stale-policy diagnostics.
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
