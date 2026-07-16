# Confirmed effects and heuristic observations have different authority

## Context

Detective integration receives a mixture of structured host events, direct
file-tool targets, shell commands, watcher snapshots, and repository diffs.
Those sources do not have equal precision. Treating a command name or partial
string parse as proof of a read-only or writing effect can either miss a real
write or hard-block ordinary work.

A shell parser embedded in a guard cannot reliably predict pipelines,
redirections, subshells, quoting, or script-internal effects. Post-action
observation can often establish facts that were unknowable beforehand.

## Decision

Carry effect classification together with observation confidence and source
detail. Pre-action hard denial is reserved for a deterministically identified
write target that conflicts with current scope, Write Ticket, or required user
authority. An uncertain or pathless possible effect produces a warning and is
checked again after execution.

Post-action reconciliation prefers structured changed paths, then bounded
watcher comparison and repository diff, before heuristic events. A suspected
Unrecorded Change remains distinguishable from a confirmed one so uncertainty
does not become a close blocker without corroboration.

The exact confidence and effect values, path assessment shape, reason codes,
blocker rules, and diagnostic output remain owned by the public schema, Guard,
Core, storage, and security Reference owners.

## Consequences

- Direct, structured out-of-scope edits can still be blocked before they run.
- Shell commands that cannot be proved safe or writing are not mislabeled as
  deterministic facts.
- Post-action evidence can promote or clear a suspected observation without
  rewriting the original source fact.
- Metrics can separate confirmed enforcement from heuristic warning quality.
- Command classification stays deliberately narrow instead of becoming a
  second shell implementation.

## Non-goals

- This decision does not promise complete observation of every filesystem or
  external effect.
- It does not make Detective an OS sandbox or identify the actor who changed a
  file.
- It does not classify every command or define the exact public value set in
  this ADR.
- Warning an unknown effect is not approval of that effect.

## Rejected alternatives

- Treating broad executable names as read-only was rejected because many of
  those programs contain writing, destructive, or script-running subcommands.
- Hard-blocking every unknown command was rejected because uncertainty is not
  proof of a policy violation and would create avoidable false positives.
- Parsing complete shell grammar in the guard was rejected because it still
  could not predict script internals or all runtime effects.
- Treating all post-action observations as equally authoritative was rejected
  because source precision must remain visible to close and diagnostics.

## Relevant implementation

- Guard command assessment under
  [`crates/volicord-cli/src/`](../../../../crates/volicord-cli/src/):
  host-event decoding, command classification, and pre/post-tool decisions.
- [`crates/volicord-store/src/session_watch.rs`](../../../../crates/volicord-store/src/session_watch.rs):
  bounded before/after repository observation.
- [`crates/volicord-store/src/guards.rs`](../../../../crates/volicord-store/src/guards.rs):
  persisted observations and Unrecorded Change state.
- [`crates/volicord-core/src/methods/reconcile_changes.rs`](../../../../crates/volicord-core/src/methods/reconcile_changes.rs):
  Core reconciliation of observed changes.

## Related tests and Reference owners

- [`crates/volicord-cli/tests/guard_command.rs`](../../../../crates/volicord-cli/tests/guard_command.rs),
  Store session-watch tests, Core reconcile-change tests, and
  [`tests/conformance/baseline.rs`](../../../../tests/conformance/baseline.rs).
- [Core Model](../../reference/core-model.md),
  [Administrative CLI](../../reference/admin-cli.md),
  [Reconcile Changes](../../reference/api/method-reconcile-changes.md),
  [API State Schemas](../../reference/api/schema-state.md),
  [API Value Sets](../../reference/api/schema-value-sets.md),
  [Storage Records](../../reference/storage-records.md), and
  [Security](../../reference/security.md).
