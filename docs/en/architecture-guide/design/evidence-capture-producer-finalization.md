# Evidence-Capture Producer Finalization Design

## Purpose

This design explains the current implementation boundary between capture
intent, source fulfillment, receipt validation, evidence fact acquisition,
producer finalization, and the Run commit.

## Design

`volicord.prepare_evidence_capture` is planned in Core and commits a bounded
capture intent. CLI fulfillment code executes or correlates the owned source
path, while Store writes the immutable receipt and its content-bound staging
data. `record_run` loads strict current intent and receipt facts through
`recording/authority.rs`, using `evidence_facts.rs` for typed receipt facts and
the recording artifact planner for verified receipt promotion. It passes the
typed authority to `recording/evidence.rs`, which evaluates it with the focused
evidence policy modules and includes observation and producer finalization in
the typed Record Run mutation plan.

Observation and producer authority remain distinct. A stored source
observation is not a producer until the current Core policy accepts the full
binding and the Run commit succeeds.

## Invariants

- An intent is bound to its current project, Task, Change Unit, scope,
  workspace, target, source, Connection, digest, and time coordinates.
- A receipt is immutable, bounded, and content-bound to one intent and source
  claim.
- Evidence facts are acquired once and passed as typed policy inputs.
- Producer finalization and Run persistence succeed or roll back together.
- Replay returns the original result and does not finalize a producer twice.
- Guard prompt capture remains observation and never substitutes for a receipt
  or user-owned resolution.

## Responsibility boundaries

The public Core method validates the request and composes the method response.
`recording/authority.rs` coordinates Store-backed capture fact resolution;
`recording/evidence.rs` and `recording/artifact.rs` convert accepted typed facts
into observation, producer, promotion, and link plans. `evidence_facts.rs` owns
reusable typed fact acquisition, the top-level Core `artifact.rs` owns reusable
artifact-source verification, and Core evidence policy modules own provenance,
binding, target, relevance, and close-readiness evaluation over typed facts.
CLI fulfillment owns command or tool-source collection. Store owns intent,
receipt, staging, producer, and Run persistence.

## Execution flow

1. Core plans and commits an evidence-capture intent.
2. The supported local source revalidates the intent and records its bounded
   receipt through Store.
3. The recording authority owner loads the current typed intent, receipt,
   artifact, and target facts.
4. The recording evidence owner calls focused pure Core policy modules to
   evaluate provenance and binding without giving those policies Store access.
5. The recording planner emits typed Run, evidence, producer, and artifact
   mutations.
6. Store applies producer finalization and Run state in one immediate
   transaction.

## Failure behavior

Missing, expired, stale, cross-boundary, reused, corrupt, digest-mismatched, or
incomplete inputs fail before producer authority is created. A failed
transaction leaves the intent and observation without a finalized producer.
The implementation does not downgrade a malformed current owner record into a
weaker successful source.

## Scope exclusions

This design does not define which sources are publicly supported, evidence
eligibility, receipt fields, exact storage effects, or proof strength. It does
not treat fixtures, command output, or host observations as external
attestation.

## Implementation routes

- [`crates/volicord-core/src/methods/prepare_evidence_capture.rs`](../../../../crates/volicord-core/src/methods/prepare_evidence_capture.rs)
  and [`record_run.rs`](../../../../crates/volicord-core/src/methods/record_run.rs):
  request-specific method orchestration and response composition.
- [`crates/volicord-core/src/recording/authority.rs`](../../../../crates/volicord-core/src/recording/authority.rs),
  [`evidence.rs`](../../../../crates/volicord-core/src/recording/evidence.rs),
  and [`artifact.rs`](../../../../crates/volicord-core/src/recording/artifact.rs):
  Record Run capture resolution and typed observation, producer, promotion, and
  link planning.
- [`crates/volicord-core/src/evidence_facts.rs`](../../../../crates/volicord-core/src/evidence_facts.rs)
  and [`artifact.rs`](../../../../crates/volicord-core/src/artifact.rs):
  shared typed fact acquisition and artifact-source verification.
- [`crates/volicord-core/src/policy/`](../../../../crates/volicord-core/src/policy/):
  focused evidence provenance, binding, relevance, target, and close-readiness
  policy.
- [`crates/volicord-store/src/evidence_capture.rs`](../../../../crates/volicord-store/src/evidence_capture.rs)
  and [`core_pipeline/evidence.rs`](../../../../crates/volicord-store/src/core_pipeline/evidence.rs):
  intent, receipt, producer, and grouped mutation persistence.
- [`crates/volicord-cli/src/evidence_command.rs`](../../../../crates/volicord-cli/src/evidence_command.rs):
  local source fulfillment.

## Reference owners

Exact behavior remains in
[Prepare Evidence Capture](../../reference/api/method-prepare-evidence-capture.md),
[Record Run](../../reference/api/method-record-run.md),
[Core Model](../../reference/core-model.md),
[Storage Records](../../reference/storage-records.md),
[Storage Effects](../../reference/storage-effects.md), and
[Security](../../reference/security.md).
