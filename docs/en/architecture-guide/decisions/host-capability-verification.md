# Host-capability verification for credential delivery

## Context

The local-web User Channel handoff contains a bearer credential. The previous
adapter combined listener readiness with a client-declared
`model_invisible_user_surface=true` Boolean. A generic MCP client could make the
same declaration, while connection configuration verification retained no
exact host/client version, adapter profile, executable digest, live evidence,
or expiry history.

The declaration is useful negotiation input, but it cannot be the source of a
credential-delivery decision. Configuration, process markers, `clientInfo`, and
host lifecycle observations are also cooperative local facts rather than host
attestation. A persisted verification row cannot supply both the observed
evidence digest and the trusted expected value against which that digest is
checked.

## Decision

Volicord uses one centralized host-capability evaluator before materializing a
credential-bearing local-web `_meta` handoff. The evaluator requires the
managed non-generic stdio path, a ready listener lease, the exact client
declaration, and current persisted capability state pointing to an immutable,
unexpired `outcome=passed` verification. The verification must match the exact
Agent Connection, host/client version, adapter profile/version, managed
fingerprint, Volicord build, source revision, target and executable digest, and
bounded evidence-artifact digest.

The expected `evidence_artifact_sha256` would require trusted production
acquisition of the external `volicord-host-release-manifest-v1` defined by
[Host Release Evidence](../../reference/host-release-evidence.md). That manifest must bind the same capability, host/client,
adapter, Volicord build, source revision, target, and executable digest as the
current row, as well as the expected evidence-artifact digest. The evaluator
must verify the manifest and exact-match the row's `evidence_artifact_sha256`
against that expected value. A missing, unknown, malformed, unverified, or
mismatched manifest fails closed. The row's own digest, the build descriptor,
and a copied manifest value are not substitutes for this comparison.

For the built-in stdio adapter, a passing row represents one observed host
version rather than two independent runtime versions:
`host_version == client_version == clientInfo.version`, and that value must
match the live artifact's installed-host version. A passing `source_revision`
is exact lowercase 40- or 64-hex; `unknown` cannot pass. If the version equality
or source revision cannot be proved, publication must use a non-passing outcome.

The verification uses canonical UTC timestamps and must satisfy
`observed_at <= created_at` and
`observed_at < expires_at <= observed_at + 86,400 seconds`; a pass additionally
requires `created_at < expires_at`. Evaluation uses the half-open interval
`observed_at <= now < expires_at`. Twenty-four hours is the maximum freshness
window, not a default lifetime, identity proof, or attestation period.
Publishers may choose a shorter expiry.

The Registry stores immutable history in `host_capability_verifications` and a
single current pointer per connection/capability in `host_capability_state`.
Publishing a later failed, unavailable, or revoked observation moves the
pointer atomically; revocation is a new immutable `outcome=revoked` row, not a
mutation of the earlier row. Evaluation never searches backward for an older
passing row. A missing, malformed, expired, non-passing, or mismatched current
row fails closed to CLI inbox without a token, `_meta`, or project-time effect.

Exact duplicate publication with the same ID and content is idempotent and does
not move a current pointer that has since advanced. Reusing that ID with
different content conflicts.

V1 verification `metadata_json` is strict canonical `{}` only. Every allowed
evidence coordinate has a dedicated column, so an arbitrary member cannot
become an undeclared trust input or a place to retain sensitive host material.

Generic connections, user-managed clients, manual stdio, CLI verification
probes, Local HTTP transport, and invalid or unknown managed-launch markers are
categorically ineligible. Exact client and process values remain matching
inputs and do not become identity proof.

A live-host validation bootstrap must not use the sensitive bearer path to
prove itself. A future validation-only path may issue a non-secret challenge in
a separate host-delivery-verification `_meta` namespace, create no User Action
or token, and require bounded human confirmation of the host-owned surface and
model-visible absence. Evidence is produced only after the final executable
exists, so its digest must not be embedded back into that executable: doing so
would change the executable digest and create a recursive binding. A trusted
internal acquisition path must instead verify the external manifest described
above before publishing or evaluating a pass. The current adapter has no such
trusted acquisition path. Therefore production local-web eligibility remains
fail-closed, local web remains implemented but unverified, and CLI inbox is
used.

The new Registry shape is `baseline_sqlite_v6`. There is no v5 conversion,
relabeling, inferred pass, or synthetic history; an incompatible Runtime Home
must be recreated.

## Consequences

- A client declaration can no longer create a bearer URL by itself.
- Host or client upgrades, managed-configuration changes, executable changes,
  expiry, revocation, and later failed validation all remove eligibility.
- Projection, fallback selection, and final token materialization use the same
  evaluator; materialization rechecks current persisted state while holding the
  listener issuance lease.
- Build metadata remains a matching coordinate, not the trust source for the
  evidence digest. Exact-final-artifact release evidence stays external to the
  binary.
- CLI inbox remains the supported complete-form fallback.
- Each stored verification is bounded operational evidence; history remains
  append-only. It does not prove host isolation, current user identity, or
  later external-host behavior.
- Removing an Agent Connection cascades only its capability state and history.

## Non-goals

- This decision does not add a public Core API method or make an administrative
  validation command a public API method.
- It does not provide cryptographic host attestation, OS isolation, user
  authentication, or anti-forgery protection against the same local principal.
- It does not claim a positive Codex or Claude Code local-web result before an
  exact live-host artifact exists.
- It does not store a bearer URL or token, prompt, transcript, screenshot, raw
  host artifact, private operator data, or arbitrary verification metadata.

## Rejected alternatives

- Trusting the Boolean, `clientInfo`, environment markers, or process arguments
  was rejected because each is caller- or host-controlled cooperative input.
- Reusing connection `complete`, `last_verification_report_json`, or
  `guard_installations.host_capability_json` was rejected because those facts
  describe mutable configuration or hook health, not exact expiring delivery
  evidence and history.
- Allowing generic clients through an allowlist was rejected because a copied
  name or version does not establish the managed built-in host path.
- Treating fixtures or direct-wrapper output as live-host evidence was rejected
  because neither observes host-owned delivery or model-visible absence.
- Bootstrapping with a real bearer URL was rejected because it would expose the
  sensitive path before the eligibility invariant had been established.
- Treating a row's own `evidence_artifact_sha256` as its expected value was
  rejected because that makes the trust check self-asserted.
- Embedding the release evidence digest in the binary was rejected because the
  evidence is produced after final binary creation and rebuilding to embed it
  changes the executable digest being bound.
- Falling back to an older passing row was rejected because revocation, expiry,
  or a later failed observation must remain effective.

## Relevant implementation areas

- [`crates/volicord-store`](../../../../crates/volicord-store): Registry schema,
  immutable history, current pointer, validation, and exact-match evaluation.
- [`crates/volicord-mcp`](../../../../crates/volicord-mcp): initialize input
  retention, launch-profile binding, centralized evaluation, listener lease,
  fallback selection, and token materialization.
- [`crates/volicord-cli`](../../../../crates/volicord-cli): bounded diagnostic
  projection and future strict validation-artifact import.

## Related tests and Reference owners

Tests cover missing and non-passing state, expiry, current-pointer
supersession, selected binding mismatches, listener and budget races, replay
non-issuance, generic self-declaration without a current pass, and one exact
managed-host positive fixture. Launch-origin tests separately classify manual
stdio and CLI verification, while Local HTTP has transport-focused tests. The
suite does not yet publish an otherwise exact current pass and prove
non-issuance through each manual-stdio, CLI-verification, and Local HTTP path;
those exact-pass negative regressions are required before claiming those paths
as covered. Live-host validation remains a separate external cell and cannot
be replaced by fixtures.

Reference owners:

- [Host Release Evidence](../../reference/host-release-evidence.md) and the
  [external gate decision](host-release-evidence-gate.md)
- [Agent Connection](../../reference/agent-connection.md)
- [MCP Transport](../../reference/mcp-transport.md)
- [Administrative CLI](../../reference/admin-cli.md)
- [Security](../../reference/security.md)
- [Storage Records](../../reference/storage-records.md)
- [Storage DDL](../../reference/storage-ddl.md)
- [Storage Versioning](../../reference/storage-versioning.md)
