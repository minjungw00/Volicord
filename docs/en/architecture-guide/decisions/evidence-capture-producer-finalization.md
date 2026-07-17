# Evidence-Capture Intent And Producer Finalization

## Context

Evidence provenance must be bound before collection, yet a Run becomes
authoritative only when its complete owner-defined result is committed.
Observation records alone must not become producer authority.

## Decision

`volicord.prepare_evidence_capture` creates a bounded intent containing the
method-owned task, change-unit, source, selector, expiry, and policy
coordinates. A supported source records a digest-bound receipt against that
intent.

`volicord.record_run` revalidates the current intent, receipt, task,
change-unit, source, expiry, and evidence body. Producer finalization and the
Run commit happen in one Store transaction. A failed or rejected commit leaves
the intent and observation without producer authority.

Guard prompt capture remains observation only and cannot resolve a UserAction
or stand in for an evidence receipt.

## Consequences

- A receipt cannot be moved across projects, intents, sources, or work state.
- Replay returns the original immutable outcome and does not finalize twice.
- Fixtures prove parsing and state transitions, not support for an external
  artifact.
- Exact schemas and effects remain in the API and Storage owners.

See [Prepare Evidence Capture](../../reference/api/method-prepare-evidence-capture.md),
[Record Run](../../reference/api/method-record-run.md), and
[Storage Effects](../../reference/storage-effects.md).
