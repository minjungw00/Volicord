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

Use one shared capability-first evaluator for the six feature IDs and the
final-output subcapabilities defined by the Agent Connection and API value-set
owners. Shared types own the closed identifiers, runtime probe facts, reviewed
host/version/client evidence coordinates, canonical Codex version grammar, and
the single-feature support-state precedence over exact evidence and current
readiness. CLI diagnostics, MCP delivery eligibility, and release validation
must consume those results rather than reconstructing host-kind fallbacks,
version gates, or feature-state precedence. An unfamiliar version is
`implemented_unverified`, not `unsupported_by_host`; a successful applicable
runtime probe can establish `verified`, a failed probe yields a degraded or
temporarily unavailable state, and only demonstrated host absence yields
`unsupported_by_host`.

The CLI aggregates the shared single-feature results across the six-feature
matrix and each final-output profile. Exact live evidence and current runtime
readiness remain separate inputs. Configuration is reported only through
`configured` and `configuration_verified`; it never promotes support. MCP must
reject a demonstrated unsupported local-web surface before consulting
persisted verification and must repeat the capability check when issuing a
delivery lease. An exact client version remains evidence metadata, not the
primary runtime eligibility gate.

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
- MCP, CLI diagnostics, and release validation cannot assign different
  capability dispositions to the same current probe and evidence facts.

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
- Using a reviewed exact-version matrix as the primary runtime gate was
  rejected because a newer compatible host would be mislabeled unsupported
  before its actual capabilities were probed. Reviewed versions remain
  regression and release-evidence coordinates.

## Relevant implementation areas

- [`crates/volicord-types/src/host_feature_support.rs`](../../../../crates/volicord-types/src/host_feature_support.rs):
  closed identifiers, reviewed host/version/client evidence facts, canonical
  parsing, capability-probe facts, and single-feature state precedence.
- [`crates/volicord-cli/src/host_integration/capability_status.rs`](../../../../crates/volicord-cli/src/host_integration/capability_status.rs):
  profile-specific final-output and six-feature diagnostic aggregation over
  shared capability and single-feature support results.
- [`crates/volicord-mcp/src/adapter.rs`](../../../../crates/volicord-mcp/src/adapter.rs):
  initial and issuance-time local-web eligibility checks over the same capability
  result.
- [`tests/release-validation`](../../../../tests/release-validation):
  exact-artifact claim evaluation over the same capability result.
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
