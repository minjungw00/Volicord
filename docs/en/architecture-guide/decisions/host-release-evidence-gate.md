# External host release evidence gate

## Context

Managed-host feature support combines static implementation facts with live
Codex and Claude Code behavior. A release binary exists before live-host
evidence can be produced, so embedding the resulting evidence digest would
change the artifact being proved. Ad hoc CLI output, fixtures, claimed status,
and results collected from different host versions cannot establish one exact
release claim.

The historical v1 cell, manifest, and audit evaluator owned implementation
disposition only by host kind. An exact reviewed-version table changes how a
cell is interpreted, so neither v1 artifacts nor their cell-input digest domain
can carry the new meaning.

The credential-bearing local-web path is more sensitive still. Its production
adapter currently has no trusted manifest-acquisition path. A release artifact
must therefore remain external validation evidence rather than a runtime trust
input.

## Decision

Volicord uses the versioned contract in
[Host Release Evidence](../../reference/host-release-evidence.md). One clean
source revision produces one exact-profile, exact-target final candidate at an
external immutable path. The source coordinate includes SHA-256 of raw
`git archive --format=tar <source_revision>` output under
`git_archive_tar_sha256_v1`; the candidate includes its own SHA-256 and exact
build environment. Before executing candidate-controlled bytes, the gate
hashes a held regular-file handle, copies verified bytes to a private
create-new executable, and runs only that copy with an empty ambient
environment. It then verifies the held bytes and final pathname identity are
unchanged; a post-execution stability mismatch is a failing invariant, while a
pre-execution descriptor or copy digest mismatch stops without a manifest or
candidate execution. These embedded build coordinates and archive checks provide
non-adversarial provenance and integrity; they are not a reproducible rebuild
or an attestation that arbitrary candidate bytes came from the named source.

Every gate evaluates a fixed twelve-cell matrix: six feature identifiers for
each of `codex` and `claude_code`, with one exact host availability coordinate
per host kind. A present unavailable-host matrix uses explicit null identity;
an implemented cell is `ignored` and remains a downgrade, while a static
unsupported cell is `not_applicable`. An implemented requested claim remains
eligible to fail even when identity is null; only an explicit exclusion sets
`requested_verified=false`.
The v2 evaluator validates each static disposition against a version-aware
owner table. For canonical Codex `host_version=0.144.4`,
`native_user_action`, `verified_tool_producer`, and
`registered_connection_observation` are implemented, while
`local_web_user_channel`, `record_final_output`, and
`detective_final_output` are unsupported by that host version. The exact raw
version-probe envelope is `codex-cli 0.144.4`, from which the cell stores
`0.144.4`. A null or unreviewed Codex version retains the host-kind fallback:
the first four features are implemented and the two final-output features are
unsupported. Claude Code retains its all-implemented host-kind fallback. This
is an exact reviewed-version table, not a minimum-version claim.
The canonical evaluator validates and recomputes coordinates, timestamps, and
digests and derives support status without trusting a producer's claimed
status. It derives the adapter profile from the feature (`record` only for
`record_final_output`, otherwise `detective`) and requires the adapter version
to equal the exact candidate `build_id`, including for static unsupported
cells. Freshness uses
`started_at <= recorded_at <= evaluated_at < started_at + 24h`. Results are
never aggregated across host versions.

An implemented cell becomes `verified` only from a complete, fresh,
coordinate-exact, digest-exact passing run. Present ignored, running, stale,
failed, or mismatched implemented cells become `implemented_unverified`.
Missing or malformed structural input prevents manifest creation rather than
being converted into a status. A statically owned `unsupported_by_host` result
remains `unsupported_by_host`. A requested verified claim that is not met
fails the gate; otherwise a matrix with an implemented-feature downgrade is an
explicit `pass_with_downgrades`. An explicit
`requested_verified=false` exclusion remains such a downgrade even when the
cell's evidence derives `verified`.

The gate creates a bounded external
`volicord-host-release-manifest-v2` file without overwriting. After the gate
process exits, a separate process independently reopens the source candidate,
the twelve original cell files, cell evidence, and manifest, recomputes their
SHA-256 values, invariants, statuses, findings, exclusions, and verdict, and
requires the original cells to equal the manifest's embedded raw cells. It
then creates a bounded external
`volicord-host-release-audit-v2` file without overwriting. Its cell-input-set
digest uses the `volicord-host-release-cell-inputs-v2` domain. The audit may not
delegate to a manifest-trusting display path. Administrative CLI output is
auxiliary only.

Managed Codex and Claude Code session correlation uses the domain-separated
SHA-256 mapping owned by Host Release Evidence. Managed MCP and hook paths use
the same opaque Volicord session ID, while the raw native session identifier is
never persisted. The `mhs_` namespace and its host/connection coordinates are
reserved and immutable; invalid markers create no durable diagnostic state,
and other native correlation identifiers are made opaque before persistence.
Missing or mismatched binding cannot produce Strong Evidence.

For the reviewed Codex version, managed stdio remains session-unbound until an
accepted tool call supplies the exact MCP client identity
`codex-mcp-client`/`0.144.4` and internally consistent per-call metadata:
`_meta.threadId` plus `session_id`, `thread_id`, and `turn_id` under
`_meta["x-codex-turn-metadata"]`. The flat and nested thread IDs must match;
the native `session_id`, not either thread ID, is the input to the reserved
mapping. The concrete thread is reduced to a separate domain-separated
process-local digest. The first valid call binds the stdio process once to both
coordinates, and every later call must match both; a new turn ID is allowed.
Missing, malformed, or mismatched metadata is rejected before tool dispatch or
managed durable effects. Ambient
`CODEX_THREAD_ID`, arrival order, timestamps, and nearest-session selection are
not binding authority. Existing feature assertion sets already require the
resulting exact session and connection scope, so this transport binding adds
no release assertion identifier.

The validation implementation is isolated in the test-only
`tests/release-validation` workspace package. It may reuse implementation-owned
evaluators, but production crates do not depend on it. Its maintained command
routes are owned by Host Release Evidence and the Maintain Validation page.

## Consequences

- A release claim is integrity-bound to one declared clean revision, one
  external final executable, one target, one exact profile, and exact host
  availability coordinates, subject to the non-adversarial provenance limit
  above.
- A producer cannot promote support by claiming a status or omitting an
  inconvenient cell.
- Stale or partial results remain visible as downgrades rather than being
  silently mixed with another run.
- The manifest and separate audit are durable release review inputs but do not
  create Core evidence, user authority, host attestation, or runtime trust.
- Production local-web manifest acquisition remains unavailable and therefore
  fail-closed; CLI inbox remains the supported fallback.
- Native session identifiers do not enter Volicord storage, diagnostics, or
  release evidence.

## Non-goals

- This decision does not add a public API method or production import command.
- It does not establish minimum Codex or Claude Code versions.
- It does not prove OS isolation, host identity, user identity, or absence of
  later host changes.
- It does not prove build reproducibility or attest source-to-binary provenance
  against a malicious candidate producer.
- It does not permit results from different host versions or candidates to be
  combined.
- It does not make external release artifacts trusted production inputs.

## Compatibility and migration

This decision advances the test-only cell, manifest, audit, and cell-input
digest contracts to v2 and tightens internal managed-host correlation; it does
not change a public Core API schema, public MCP method, SQLite DDL, or
storage-profile version. V1 cell, manifest, audit, and cell-input-domain inputs
remain historical and are rejected rather than imported, migrated, or
reinterpreted. The candidate stays `volicord-release-candidate-v1` and the
source archive algorithm stays `git_archive_tar_sha256_v1` because their
preimages did not change.

The reserved `mhs_` rules intentionally reject generic, cross-host, or
cross-connection preseeded values and invalid managed markers. No legacy alias,
fallback mapping, or decoder is added; compatible current observations are
recreated through the managed adapter. The batch remains within the current
workspace SemVer because it does not add or break a supported public API or
deployment surface; its externally stored v2 artifacts are opt-in release
validation output.

## Rejected alternatives

- Embedding live evidence in the candidate was rejected because rebuilding
  changes the exact executable digest and creates a recursive binding.
- Trusting `claimed_status`, CLI text, fixtures, or copied hashes was rejected
  because each can bypass canonical recalculation.
- Allowing a sparse or open-ended matrix was rejected because omission would
  hide unsupported or unverified features.
- Extending freshness through equality at 24 hours was rejected because the
  contract uses a precise half-open window.
- Aggregating the newest passing cell from each host version was rejected
  because no resulting claim would describe one tested host environment.
- Running the audit in the gate process was rejected because it would not
  provide process-separated reopening and recalculation.
- Persisting raw host session identifiers was rejected because correlation
  needs only the domain-separated opaque mapping.
- Reinterpreting v1 cells, manifests, audits, or the v1 cell-input digest under
  v2 semantics was rejected because a historical digest must retain one
  meaning.
- Binding Codex from `CODEX_THREAD_ID`, timing, arrival order, the newest open
  session, or proximity was rejected because concurrent and resumed sessions
  can produce indistinguishable but swapped pairings.

## Related owners and planned validation location

- [Host Release Evidence](../../reference/host-release-evidence.md)
- [Managed-host session/thread binding and per-call turn validation](managed-host-session-turn-binding.md)
- [Agent Connection](../../reference/agent-connection.md)
- [System Requirements](../../reference/system-requirements.md)
- [Security](../../reference/security.md)
- [Validation](../../maintain/validation.md)
- `tests/release-validation`

The package path above is the intended test-only implementation location. This
decision does not define its private module layout.
