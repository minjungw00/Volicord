# Host feature support-state evaluation

## Context

Host integration previously collapsed three different facts into Boolean names
such as `supported` and `verified`: implementation presence, generated-file
configuration checks, and actual-host evidence. A configuration fixture could
therefore appear to verify behavior that no exact installed host and final
Volicord artifact had demonstrated. Stale results could also outlive the binary,
host version, adapter profile, or runtime prerequisite they observed.

The same ambiguity affected native user action, local-web delivery, evidence
producer paths, registered connection observations, and both final-output
profiles. Doctor, connection status, and release validation could interpret the
same facts differently.

## Decision

Use one typed `HostFeatureSupportStatus` evaluator for the six feature IDs and
precedence defined by the Agent Connection and API value-set owners. The
evaluator keeps implementation, exact live evidence, and current runtime
readiness separate. Configuration is reported only through `configured` and
`configuration_verified`; it never promotes support.

Doctor, connection status, and the release feature matrix consume one
evaluation result. Exact replay re-runs the evaluator. Evidence must bind the
current final Volicord artifact, source revision, build and target, installed
host and version, adapter profile, connection identity, evidence artifact, and
freshness interval. Missing, stale, expired, malformed, or mismatched evidence
cannot be treated as a transient runtime outage.

Final-output diagnostics use `support_status`, configuration facts, an exact
profile-specific `required_subcapabilities` list, and a map containing only the
applicable `authority_display`, `authenticated_exact_replay`, and
`block_finalization` states. Best-effort non-sensitive display may run when its
implementation and configuration are present, while the typed state remains
unverified or unsupported. It does not establish a support or release claim.

Stored `host_capability_json` moves from internal schema v1 to v2. V2 records
explicit implementation and configuration facts rather than the ambiguous
`final_output_authority_disclosure_supported` Boolean. An old v1 record is
invalid current diagnostic input and is repaired by rerunning the supported
init workflow. No fallback inference is made from its old Boolean.

## Consequences

- A feature can be implemented without being described as verified.
- A known absent host-owned surface is distinguishable from a temporary outage.
- Configuration checks remain useful without becoming live-host evidence.
- A host, binary, adapter, evidence, or freshness mismatch downgrades the exact
  feature rather than inheriting a historical pass.
- Final-output Record and Detective claims expose their distinct required
  subcapabilities while continuing to share fresh authority projection code.

## Public and diagnostic compatibility

This is an intentional pre-major `0.9.0` diagnostic contract change. The old
`supported` and `verified` final-output fields and
`native_host_output_adapter_verified` name are removed, not retained as aliases.
The configuration-only replacement is
`native_host_output_adapter_config_verified`. Consumers must read
`support_status` and must not infer it from configuration fields.

The stored v2 capability JSON change has no v1 compatibility decoder or
synthetic migration. Rerunning init regenerates current managed records and
files. Public Core method inputs do not gain a host-support selector, and the
support state does not create Core authority or a new public method.

## Non-goals

- This decision does not claim that current Codex or Claude Code live cells
  passed.
- It does not turn fixtures, generated wrappers, ignored tests, or historical
  result files into actual-host evidence.
- It does not provide host attestation, user identity, OS enforcement, or a
  security proof.
- It does not make best-effort display equivalent to supported replay or block
  finalization.

## Rejected alternatives

- Keeping separate `supported`, `configured`, and `verified` Booleans was
  rejected because their meanings overlapped and configuration could masquerade
  as behavior verification.
- Treating missing evidence as `temporarily_unavailable` was rejected because a
  temporary state presupposes exact current evidence.
- Retaining v1 aliases or inferring v2 state from old capability JSON was
  rejected because it would preserve the ambiguous contract.
- Letting every command recompute support independently was rejected because
  precedence, freshness, and subcapability aggregation would drift.

## Relevant implementation areas

- [`crates/volicord-cli/src/host_integration/`](../../../../crates/volicord-cli/src/host_integration/):
  typed values, host baseline facts, and centralized evaluation.
- [`crates/volicord-cli/src/connection_command.rs`](../../../../crates/volicord-cli/src/connection_command.rs)
  and [`doctor_command.rs`](../../../../crates/volicord-cli/src/doctor_command.rs):
  diagnostic consumers.
- [`crates/volicord-cli/src/guard_integration/`](../../../../crates/volicord-cli/src/guard_integration/):
  v2 capability metadata and configuration audit facts.
- [`crates/volicord-cli/tests/live_host_smoke.rs`](../../../../crates/volicord-cli/tests/live_host_smoke.rs):
  bounded exact-artifact evidence inputs kept separate from product status.

## Reference owners

Exact status values and shapes remain in [API Value Sets](../../reference/api/schema-value-sets.md)
and [API State Schemas](../../reference/api/schema-state.md). Evaluation,
baseline host state, replay, and final-output subcapabilities remain in
[Agent Connection](../../reference/agent-connection.md). Administrative output
and same-identity repair versus migration rejection remain in
[Administrative CLI](../../reference/admin-cli.md). The exact closed stored v2
capability shape, semantic relations, and owner binding remain in
[Storage Records](../../reference/storage-records.md); the defensive projected
required-phase rule remains in API State Schemas. Environment prerequisites remain in
[System Requirements](../../reference/system-requirements.md).
