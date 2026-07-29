# Observation Confidence Boundary Design

## Purpose

This design explains how structured path facts, uncertain host observations,
reconciliation state, and operational diagnostics remain separate in the
current Guard and Store architecture.

## Design

Guard decoding produces typed host-neutral outcomes. Exact compatible path
facts may enter the owned pre-action policy path; uncertain command or
repository observations remain suspected until a deterministic post-action
comparison confirms their Product Repository paths. Store persists
Unrecorded Change state separately from shared structured diagnostics.

Diagnostic identity is selected by a closed domain diagnostic kind and shared
typed subject identity. Occurrence findings are insert-only; current
conditions use an immutable `CurrentDiagnosticKey` plus a replaceable
snapshot. Renderers select typed fields and actions rather than classifying
summary prose.

## Invariants

- Suspected observations do not silently become confirmed authority.
- Suppression removes only exact owner-defined matches.
- Partial or unavailable observation is not reported as complete.
- Prompt capture records observation and never a user answer.
- Diagnostic lifecycle, identity, cause edges, and actions are typed before
  persistence and rendering.
- A diagnostic finding describes a condition; it does not replace
  reconciliation or close-readiness state.

## Responsibility boundaries

CLI Guard modules decode host input and project host output. Core policy owns
write and reconciliation interpretation. Store owns Guard events,
Unrecorded Changes, structured finding lifecycle, and cause graphs. Store
strictly decodes persisted observation status, confidence, Product Repository
paths, typed objects, actors, and timestamps before Core receives a
reconciliation record.
`volicord-types` owns dependency-safe diagnostic identity and report shapes;
CLI and MCP domain modules own exhaustive conversion from their failures.

## Execution flow

1. A Guard adapter decodes one host event into a typed neutral outcome.
2. Compatible structured facts reach the applicable policy path; incompatible
   input remains an observation with no fabricated policy result.
3. Post-action comparison records suspected or confirmed Unrecorded Change
   state.
4. Reconciliation evaluates deterministic coverage or requests user action.
5. Domain failures project typed diagnostic findings.
6. Store inserts occurrences or reconciles current-condition snapshots and
   validates bounded cause graphs.

## Failure behavior

Unavailable persistence, incomplete suppression, invalid host payloads,
unknown diagnostic identity, missing cause rows, cycles, and corrupt stored
facts remain explicit typed failures. The implementation does not replace
missing observation with an empty successful result or infer a remediation
from human-readable text.

## Scope exclusions

This design does not claim actor identity, OS sandboxing, complete
observability, write prevention, or correctness. It does not define public
confidence values, reconciliation effects, diagnostic codes, or Guard
suppression contracts.

## Implementation routes

- [`crates/volicord-cli/src/guard_command/`](../../../../crates/volicord-cli/src/guard_command/)
  and [`operational_diagnostics/`](../../../../crates/volicord-cli/src/operational_diagnostics/):
  typed Guard adaptation and CLI diagnostic projection.
- [`crates/volicord-core/src/methods/reconcile_changes.rs`](../../../../crates/volicord-core/src/methods/reconcile_changes.rs):
  current reconciliation planning.
- [`crates/volicord-store/src/guards.rs`](../../../../crates/volicord-store/src/guards.rs)
  and [`core_pipeline/reconciliation.rs`](../../../../crates/volicord-store/src/core_pipeline/reconciliation.rs):
  observations and reconciliation records.
- [`crates/volicord-store/src/diagnostic_findings/`](../../../../crates/volicord-store/src/diagnostic_findings/)
  and [`crates/volicord-types/src/diagnostics.rs`](../../../../crates/volicord-types/src/diagnostics.rs):
  lifecycle-aware persistence and shared typed identity.

## Reference owners

Exact behavior remains in
[Guard Suppression](../../reference/guard-suppression.md),
[Reconcile Changes](../../reference/api/method-reconcile-changes.md),
[Storage Records](../../reference/storage-records.md),
[Failure Model](../../reference/failure-model.md), and
[Security](../../reference/security.md).
