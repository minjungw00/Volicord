# State-Bound Write Ticket Validity Design

## Purpose

This design explains where the implementation constructs, evaluates, reuses,
invalidates, and consumes Write Tickets against current Core and Store facts.

## Design

`prepare_write` loads the current Task, Change Unit, scope, workspace,
approval, workflow-policy, and normalized path facts. The focused
`write_ticket/` owner keeps those responsibilities explicit.
`read_model.rs` acquires typed stored-ticket, Task, normalized
workflow-policy, current UserAction-resolution, and evidence facts.
`current_validity.rs` first converts each Store-validated record into either a
terminal stored evaluation or an active stored candidate. Invalidated,
consumed, and revoked records finish at that boundary; only an active
candidate can request current authority and approval facts. It then converts
that candidate into either `ReusableStoredWriteTicket` or an invalidated
stored state. `approval.rs` is the single owner that constructs the canonical
Write Ticket approval requirement, derives the invariant-bearing current
sensitive-approval set, and assesses a Store-valid persisted approval basis as
`NotRequired`, `Current`, or `Changed` with a typed reason.
`selection.rs` chooses only among complete `StoredWriteTicketEvaluation`
values. `summary.rs` has distinct planned and stored projection inputs, so a
planned issuance never impersonates a stored evaluation. `service.rs`
coordinates this persisted flow, partitions terminal and active records before
loading current facts, selects one stored result, and loads evidence only for
the selection.
`planning.rs` evaluates a focused `PrepareWriteInput` and returns typed
semantic decision reasons, related record identities, common facts, candidate
mutation facts, and exactly one `PrepareWriteTicketPlan` branch:
`Issue(PlannedWriteTicketDraft)`, `Reuse(ReusableStoredWriteTicket)`, or
`NoTicket(WriteDecisionPathFacts)`. The issue draft and reusable stored ticket
each expose one immutable `WriteTicketPathScope`; only no-ticket carries
decision paths not attached to a ticket. The planner does not receive a public
request envelope, dry-run intent, response state version, or durable ID
generator.

The public method projects dry run from that closed planning branch. A
committed call converts it to one `MaterializedPrepareWriteTicket`: issued,
reused, or none. For issuance, the method supplies the durable ticket ID,
approval-reference projection state version, and basis state version. The
typed non-empty approval basis from `approval.rs` constructs the state-versioned
`UserActionResolutionRef` values while materializing one validated
`PlannedWriteTicket`. The issued plan supplies nested and top-level response
facts and the fully typed Store insertion. Reused response facts come from the
reusable stored ticket, while none carries no ticket identity or insertion.
`semantic.rs` exposes only the immutable ticket meaning shared by planned and
stored forms. Planned and stored lifecycle identities remain in their own
types; every evaluated stored state has a mandatory `WriteTicketId`.
For a protected Record Run, the planner first converts the selected physical
active record into an active candidate. Record Run evaluation checks the exact
typed operation, Task, Change Unit, Git workspace, observed-change, and
current-policy facts to produce an exact-attempt compatibility proof while
current-validity evaluation proves a `ReusableStoredWriteTicket`.
`write_ticket/admission.rs` combines those matching proofs into
`AdmissibleStoredWriteTicket`, which is the only ticket type carried into
mutation planning and consumption.
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
- Every stored evaluation has a non-optional persisted `WriteTicketId`.
- Planned issuance is never a variant of a stored lifecycle evaluation.
- Prepare Write ticket planning and materialization each select exactly one
  closed issue/reuse/no-ticket branch.
- Planned and stored tickets each own one invariant-bearing
  `WriteTicketPathScope`; no parallel ticket path arrays accompany the branch.
- The issued plan is the one source for response and persistence, reuse is the
  one source for its response, and only no-ticket owns unattached decision
  paths.
- Terminal stored states cannot enter active currentness or admission logic.
- Reuse accepts only `ReusableStoredWriteTicket`; protected mutation accepts
  only `AdmissibleStoredWriteTicket`.
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
construction and assessment, stored-only selection, and current validity are
focused pure semantic policies. Planned summary projection accepts a
`PlannedWriteTicket`; stored summary projection accepts a
`StoredWriteTicketEvaluation` plus supplied evidence. Neither can read Store
or recompute workflow, UserAction, or approval policy. The narrow service
coordinates terminal pre-evaluation, active-only current fact loading,
selection, evidence loading, and stored projection. The Record Run planner
passes a typed reusable ticket and the matching exact-attempt compatibility
proof to admission and retains only the returned admissible ticket; it does not
interpret stored path JSON or construct ticket approval policy independently.
Store keeps the physical ticket row private and strictly
decodes status, validity basis, attempt scope, Product Repository path
collections, timestamps, and redundant owner coordinates before returning a
`StoredWriteTicket`. Relationships among physical fields are validated as
closed Write Ticket aggregate invariants, including approval-reference owner
agreement and unique full resolution identities. Core validates semantic planning
invariants while constructing an identity-free draft, then validates identity
and state-version-dependent invariants during method-owned
`PlannedWriteTicket` materialization. Those checks are separate from
Store-owned persisted physical validation. `WriteTicketPathScope` validates
typed path uniqueness and allowed/denied disjointness before either form is
exposed, while Core and Store retain their lifecycle-specific cross-field
checks. Planned issuance and each stored lifecycle state share immutable
semantic facts only. Stored evaluation,
selection, currentness, reuse, admission, consumption, and summary projection
retain their actual persisted identity.
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
4. The Write Ticket read boundary pre-evaluates every stored record. Terminal
   records become complete typed outcomes immediately; active records become
   `ActiveStoredWriteTicketCandidate` values.
5. Only when active candidates exist does the service load current Task,
   workflow-policy, and UserAction facts. `approval.rs` produces the semantic
   assessment for each active candidate, and current-validity policy produces a
   reusable or invalidated active outcome.
6. Pure selection considers only the resulting stored evaluations. After
   selection the service loads its evidence and projects the stored summary.
   Close readiness and the CLI guard receive the same evaluated stored state
   and do not recompute approval currentness.
7. Write Ticket planning returns common facts, mutation facts, and one closed
   issue, reuse, or no-ticket branch without inspecting dry-run intent or
   constructing public refs.
8. For dry run, `prepare_write` projects a preview from that branch and stops.
   For commit it preserves the branch as issued, reused, or none. Issued
   allocates the durable ID, materializes one `PlannedWriteTicket`, and derives
   nested result, top-level identity and paths, planned summary, and
   `WriteTicketInsert` from that value. Reused derives its result and stored
   summary from `ReusableStoredWriteTicket`; none derives only decision paths.
9. For Record Run, evaluation produces an exact-attempt compatibility proof
   while current-validity evaluation proves a `ReusableStoredWriteTicket`.
   `write_ticket/admission.rs` combines those matching proofs and returns
   `AdmissibleStoredWriteTicket`.
10. Store commits consumption of that admissible ticket with the protected
    mutation.

## Failure behavior

Missing current work, stale or corrupt policy, path normalization failure,
typed approval change, workspace mismatch, explicit revocation, incompatible
basis, or ambiguous ticket selection prevents reuse or consumption without a
partial protected effect. The assessment distinguishes newly required
approval, absent current resolution, changed approval scope, and a basis
resolution that is no longer current. Exact replay does not consume the ticket
again.
Malformed physical fields and persisted cross-field disagreement are Store
corruption. A checked Core state-conversion failure is an invariant failure,
not a panic-based narrowing path. Expiry, path, operation, or current-policy
mismatch evaluated from an internally valid typed ticket is a semantic policy
outcome, not persisted corruption.

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
  construction, typed approval assessment, terminal pre-evaluation,
  active-only current-validity evaluation, stored-only selection, semantic
  issuance-draft planning, validated `PlannedWriteTicket` materialization,
  separate planned and stored summary projection, narrow persisted-summary
  coordination, and reusable-to-admissible Record Run admission.
- [`crates/volicord-core/src/write_ticket/tests/read_model_service.rs`](../../../../crates/volicord-core/src/write_ticket/tests/read_model_service.rs):
  focused Store-backed fact acquisition and persisted-summary service coverage.
- [`crates/volicord-core/src/write_ticket/tests/record_run_admission.rs`](../../../../crates/volicord-core/src/write_ticket/tests/record_run_admission.rs):
  focused Record Run ticket admission and no-effect rejection coverage.
- [`crates/volicord-types/src/product_path.rs`](../../../../crates/volicord-types/src/product_path.rs):
  shared typed product-path normalization and containment plus immutable
  `WriteTicketPathScope` uniqueness and disjointness.
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
