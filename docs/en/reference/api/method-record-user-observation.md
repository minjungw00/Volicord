<a id="volicordrecord_user_observation"></a>

# `volicord.record_user_observation` reference

## What this document owns

This document owns the public contract for the User Channel-only
`volicord.record_user_observation` transition: its request, result, authority
checks, freshness rules, and method-specific effects.

It does not redefine `ArtifactRef`, `EvidenceTarget`, common response branches,
storage DDL, or close-readiness policy.

## Purpose

`volicord.record_user_observation` records a user's target-bound assessment of
exact, already-persistent artifact bytes. It creates a
`UserEvidenceObservation`; it does not create or resolve a `UserJudgment`, grant
final acceptance, or record a Run.

The method is a direct User Channel surface. It is available through the local
CLI as `volicord inbox observe` and is not exposed as an Agent Connection MCP
tool.

## Request schema

```yaml
RecordUserObservationRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string
  target: EvidenceTarget
  relevance_status: supported | contradicted
  artifact_ids: string[]
  summary: string
  observed_at: string
```

All members are required. `artifact_ids` must be non-empty. Core resolves each
ID to its canonical current `ArtifactRef`; caller-supplied hashes, sizes, or
producer labels are not accepted by this method.

## Access and validation

The verified invocation must use `actor_source=local_user` and
`operation_category=user_only`. A committed request also requires a non-null
`idempotency_key` and current `expected_state_version`.

Core requires:

- the current Task and active Change Unit named by the request;
- a current Task baseline;
- a current acceptance criterion or an existing same-Task supplemental claim;
- at least one same-Task persistent artifact whose stored body bytes are still
  available and integrity-verified;
- a non-empty summary and an `observed_at` value that is not in the future.

Core records the current `scope_revision`, baseline, exact canonical artifact
refs, verified local-user actor, and the actual User Channel verification basis.
Changing any of those current coordinates makes the record ineligible as a
Strong observation for the new basis.

## Success result

```yaml
RecordUserObservationResult:
  base: ToolResultBase
  user_observation_ref: StateRecordRef
  user_observation: UserEvidenceObservation
```

A committed result increments `project_state.state_version` once, writes one
`user_evidence_observations` row, emits
`user_evidence_observation_recorded`, and creates the ordinary committed replay
row. It does not update evidence coverage by itself. A later
`volicord.record_run` must reference `user_observation_ref`, the same target,
and the exact canonical artifact outputs before Core derives
`user_observation` / `user_observed` provenance.

## Dry run and rejection

Dry run performs the same request and authority checks but creates no record,
event, replay row, or state-version change. Stale state, non-user invocation,
missing or changed artifact bytes, stale Task coordinates, unknown targets,
and invalid relevance input reject without effect.

## Authority boundaries

- This record establishes user observation and target relevance only for the
  exact stored bytes and basis it names.
- It does not prove that an external tool produced those bytes.
- It is evidence provenance, not a user-owned judgment, approval, final
  acceptance, residual-risk acceptance, or correctness proof.
- `record_run` and close checks re-read this producer record, revalidate byte
  integrity and exact output identity, and require `relevance_status=supported`.
  A missing, contradicted, stale, corrupt, or mismatched record is weak.

## Related owners

- Evidence derivation: [`volicord.record_run`](method-record-run.md).
- Evidence shapes: [API State Schemas](schema-state.md#evidence-and-run-snapshot-shapes).
- Values: [API Value Sets](schema-value-sets.md#evidence-observation-values).
- Exact effects: [Storage Effects](../storage-effects.md#volicordrecord_user_observation).
- Storage rows: [Storage Records](../storage-records.md).
