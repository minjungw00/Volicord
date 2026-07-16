# Task control levels and integration profiles are separate axes

## Context

The Record and Detective profiles describe how Volicord is connected to an
agent host. They select host-integration, observation, and diagnostic paths.
They do not describe the risk or authority needs of one `Task`.

Using one profile value for both concerns makes unrelated choices appear
coupled. A Record installation can handle sensitive work, while a Detective
installation can observe a read-only investigation. It also encourages an
agent to infer that a lighter host setup grants a lighter work boundary.

## Decision

Keep integration profile and Task control level as independent, visible axes.

- The integration profile remains an installation and Agent Connection
  concern. Host adapters and administrative setup own its configuration.
- The Task control level is a Core-owned work-state concern. Core derives and
  persists it from the caller request, project-owned policy, current scope, and
  facts that require escalation.
- Project policy may constrain or raise a requested level. An Agent Connection
  may carry the request, but it does not become the policy authority.
- When the normalized write-authority policy changes, the active Task is marked
  for policy reevaluation even if its stored control level and final-acceptance
  policy do not need to rise. Incompatible active Write Tickets are invalidated
  at that boundary.
- The next `volicord.prepare_write` resolves the current policy against the
  requested operation and paths, clears a satisfied reevaluation mark, and may
  raise the request to `sensitive` or require new sensitive-action approval.
- Adapters project both facts without translating one into the other.
- An active Task does not become less controlled merely because configuration
  or a later request becomes less restrictive.

The exact public types, values, derivation rules, persistence, and response
fields remain owned by [Core Model](../../reference/core-model.md),
[Intake](../../reference/api/method-intake.md), the public schema owners, and
the storage Reference family.

## Consequences

- Setup documentation can explain Record and Detective without implying a
  Task-risk grade.
- Task views can explain why a control level was selected independently of the
  host profile.
- Tests cover the cross-product of integration capability and Task control
  instead of treating profiles as a risk ladder.
- A changed normalized write authority creates an explicit reevaluation point
  for the active Task and invalidates incompatible active tickets; reapplying a
  normalized-equivalent authority preserves them.
- This binding uses the existing Write Ticket validity record. The storage
  profile remains `baseline_sqlite_v7`; no storage-profile transition is part
  of this behavior.
- Core, rather than generated host guidance, remains the place that resolves a
  requested level against project-owned constraints.

## Non-goals

- This decision does not define the Task control value set or escalation
  algorithm.
- It does not turn a control level into an OS permission, sandbox, or proof
  that an agent followed instructions.
- It does not rename or remove the Record or Detective profiles.
- It does not make the agent the owner of project workflow policy.
- It does not let a host profile, Guard result, or post-write final acceptance
  substitute for required pre-write approval.

## Rejected alternatives

- Adding more combined profile names was rejected because every new risk and
  host-capability combination would multiply the setup surface.
- Letting the agent choose a low-risk path without project policy was rejected
  because the requester of authority cannot also be its only policy source.
- One project-wide level with no Task record was rejected because risk can
  change across Tasks and because the reason for escalation must remain
  inspectable with the Task.

## Relevant implementation

- [`crates/volicord-types/src/values.rs`](../../../../crates/volicord-types/src/values.rs)
  and [`crates/volicord-types/src/methods.rs`](../../../../crates/volicord-types/src/methods.rs):
  shared value sets and public request/result shapes.
- [`crates/volicord-core/src/methods/intake.rs`](../../../../crates/volicord-core/src/methods/intake.rs):
  Task creation and Core-owned control selection.
- [`crates/volicord-cli/src/guard_integration/policy.rs`](../../../../crates/volicord-cli/src/guard_integration/policy.rs):
  managed project policy parsing and administrative integration.
- [`crates/volicord-core/src/policy/workflow.rs`](../../../../crates/volicord-core/src/policy/workflow.rs):
  current-policy control resolution for Task work.
- [`crates/volicord-store/src/workflow_records.rs`](../../../../crates/volicord-store/src/workflow_records.rs):
  normalized write-authority comparison, Task reevaluation marks, and active
  Write Ticket invalidation during policy apply.
- [`crates/volicord-store/src/schema/project.sql`](../../../../crates/volicord-store/src/schema/project.sql):
  durable project and Task state.

## Related tests and Reference owners

- Core method tests under
  [`crates/volicord-core/src/methods/tests/`](../../../../crates/volicord-core/src/methods/tests/)
  and cross-method coverage in
  [`tests/conformance/baseline.rs`](../../../../tests/conformance/baseline.rs).
- [Core Model](../../reference/core-model.md),
  [Intake](../../reference/api/method-intake.md),
  [API State Schemas](../../reference/api/schema-state.md),
  [API Value Sets](../../reference/api/schema-value-sets.md),
  [Administrative CLI](../../reference/admin-cli.md), and
  [Storage Records](../../reference/storage-records.md).
- [State-bound Write Ticket validity](state-bound-write-ticket-validity.md)
  records the complementary ticket-validity decision.
