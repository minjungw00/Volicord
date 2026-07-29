# State-Bound Write Ticket Validity Design

## Purpose

This design explains where the implementation constructs, evaluates, reuses,
invalidates, and consumes Write Tickets against current Core and Store facts.

## Design

`prepare_write` loads the current Task, Change Unit, scope, workspace,
approval, workflow-policy, and normalized path facts. The focused
`write_ticket/` owner keeps those responsibilities explicit.
`read_model.rs` acquires typed ticket, Task, normalized workflow-policy,
current UserAction-resolution, and evidence facts. `approval.rs` is the single
owner that constructs the canonical Write Ticket approval requirement, derives
the invariant-bearing current sensitive-approval set, and assesses a
Store-valid persisted approval basis as `NotRequired`, `Current`, or `Changed`
with a typed reason. `selection.rs` chooses among typed candidates, while
`current_validity.rs` converts the supplied assessment and current authority
facts into effective status and invalidation without Store access.
`summary.rs` maps an already evaluated ticket and supplied evidence to the
adapter-neutral summary without selecting a candidate or reevaluating policy.
`service.rs` narrowly coordinates the complete persisted-evaluation use case.
`planning.rs` evaluates a focused `PrepareWriteInput` and returns typed
semantic decision reasons, related record identities, candidate mutations,
and the distinct, unpersisted `PlannedWriteTicketDraft`. It does not receive a
public request envelope, dry-run intent, response state version, or durable ID
generator. For committed new issuance, the public method supplies the durable
ticket ID, approval-reference projection state version, and basis state
version. The typed non-empty approval basis from `approval.rs` constructs the
state-versioned `UserActionResolutionRef` values while materializing one
validated `PlannedWriteTicket`; that value supplies both the response
projection and fully typed Store insertion. Dry run ends at the method
boundary before materialization.
`semantic.rs` exposes the immutable ticket meaning shared by planned and
stored forms, but `WriteTicketEvaluationIdentity` keeps prospective and
persisted identity explicit.
For a protected Record Run, `write_ticket/admission.rs` accepts the exact typed
operation, Task, Change Unit, Git workspace, observation time,
observed-change, and current-policy facts it evaluates and returns the
admitted attempt scope or a typed semantic admission error.
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
- `approval.rs` alone constructs the current sensitive-approval identity set,
  the Write Ticket approval requirement, and the semantic assessment.
- Summary evaluation, reuse, Record Run admission, close readiness, and the CLI
  guard context consume that assessment or the evaluated ticket state derived
  from it; they do not compare project, `Task`, or UserAction resolution
  identities independently. Produced state version remains reference metadata.
- Store rejects approval-reference owner disagreement and duplicate full
  identities before semantic assessment.
- Relevant mismatch makes an active ticket unusable; unrelated state changes
  do not.
- Successful consumption is atomic with the protected committed mutation.

## Responsibility boundaries

Core methods own request-specific orchestration and response composition. For
Prepare Write, that includes dry-run selection, durable ID allocation,
state-versioned references, guarantee display, and conversion of typed
planning, Store, and UserAction failures to public `PlanError` branches. The
focused Write Ticket read boundary owns only typed fact acquisition. Approval
construction and assessment, selection, and current validity are focused pure
semantic policies. Summary projection accepts only evaluated typed state,
state-version and display facts, and evidence facts; it cannot read Store or
recompute workflow, UserAction, or approval policy. The narrow service
coordinates those owners and exposes evaluated ticket state to its consumers.
The Record Run planner supplies semantic facts and does not interpret stored
path JSON or construct ticket approval policy independently. Store keeps the
physical ticket row private and strictly
decodes status, validity basis, attempt scope, Product Repository path
collections, timestamps, and redundant owner coordinates before returning a
`StoredWriteTicket`. Relationships among physical fields are validated as
closed Write Ticket aggregate invariants, including approval-reference owner
agreement and unique full resolution identities. Core validates semantic planning
invariants while constructing an identity-free draft, then validates identity
and state-version-dependent invariants during method-owned
`PlannedWriteTicket` materialization. Those checks are separate from
Store-owned persisted physical validation. Planned issuance,
stored state, and projected post-consumption state share a semantic view but
retain their actual identities.
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
4. The Write Ticket read boundary loads typed candidates and the current facts
   required by each active candidate.
5. `approval.rs` constructs each candidate's current typed approval set and
   requirement, then produces the one semantic assessment. Current-validity
   policy consumes it, and pure selection policy applies the current precedence
   and tie-breaking rules.
6. State-summary projection, close readiness, and the CLI guard context receive
   evaluated ticket state derived from that assessment. Summary maps the
   selected ticket and supplied evidence facts without Store or policy access;
   close readiness and the CLI guard do not recompute approval currentness.
7. Write Ticket planning returns reuse or a new identity-free issuance draft,
   typed decision reasons, related semantic identities, and candidate
   mutations without inspecting dry-run intent or constructing public refs.
8. For dry run, `prepare_write` projects a preview and stops. For committed
   new issuance it allocates the durable ID, materializes one
   `PlannedWriteTicket`, and derives response projection and
   `WriteTicketInsert` from that value. Reuse reads a `StoredWriteTicket`.
9. For Record Run, `write_ticket/admission.rs` consumes the same approval
   assessment before applying operation-specific compatibility checks and
   returning the admitted scope.
10. Store commits ticket consumption with the protected mutation.

## Failure behavior

Missing current work, stale or corrupt policy, path normalization failure,
typed approval change, workspace mismatch, explicit revocation, incompatible
basis, or ambiguous ticket selection prevents reuse or consumption without a
partial protected effect. The assessment distinguishes newly required
approval, absent current resolution, changed approval scope, and a basis
resolution that is no longer current. Exact replay does not consume the ticket
again.
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
  typed fact acquisition, canonical approval requirement and current-set
  construction, typed approval assessment, pure candidate selection and
  current-validity evaluation, semantic issuance-draft planning, validated
  `PlannedWriteTicket` materialization, pure summary projection, narrow
  persisted-summary coordination, and protected Record Run admission.
- [`crates/volicord-core/src/write_ticket/tests/read_model_service.rs`](../../../../crates/volicord-core/src/write_ticket/tests/read_model_service.rs):
  focused Store-backed fact acquisition and persisted-summary service coverage.
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
