# External host release evidence gate

## Context

Managed-host feature support combines static implementation facts with live
Codex and Claude Code behavior. A release binary exists before live-host
evidence can be produced, so embedding the resulting evidence digest would
change the artifact being proved. Ad hoc CLI output, fixtures, claimed status,
and results collected from different host versions cannot establish one exact
release claim.

The historical v1 cell, manifest, and audit evaluator owned implementation
disposition only by host kind. V2 added an exact reviewed-version table but did
not bind a cell to the actual MCP client identity observed during managed
initialize. A copied or inferred version could therefore occupy the host
coordinate without proving which client ran the live cell. Adding required
client coordinates changes cell, manifest, audit, and digest meaning, so neither
v1 nor v2 artifacts or cell-input digest domains can carry the new contract.

The credential-bearing local-web path is more sensitive still. Its production
adapter currently has no trusted manifest-acquisition path. A release artifact
must therefore remain external validation evidence rather than a runtime trust
input.

The gate and audit already used one normalized source, Cargo target,
documentation, and Runtime Home exclusion context, but the live-cell producer
used separate source-only checks. That split allowed an authenticated host run
to create candidate, cell, or evidence paths that the gate would reject later.
Late rejection wastes the only host-native observation and violates the rule
that every release layer makes the same path-eligibility decision.

The earlier producer reserved final pathnames directly and attempted to delete
them during rollback. A pathname identity check followed by deletion cannot
prove that a concurrent replacement still names the producer-created inode.
Moving the name to a quarantine entry does not close the gap: another actor
can replace that quarantine name between the identity check and unlink, and
the quarantine move can itself displace a foreign replacement from the
expected final name. Release-cell publication therefore needs a protocol that
never deletes a published namespace entry.

The dedicated operating-system process group needed for cooperative host-turn
containment introduces a separate terminal job-control problem. If that group
remains in the background, an interactive host can be stopped when it reads
from the controlling terminal, so a timeout or absent source event would
describe the runner rather than the host feature. Reusing the runner's process
group would preserve terminal access but discard the containment coordinate.

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

The test-only package supplies a create-new descriptor producer for that
already staged final executable. It applies the same canonical external-path,
artifact-identity, build-coordinate, and source-coordinate checks used by the
gate, repeats candidate and source stability checks before writing, and refuses
an existing descriptor path. It does not build, stage, replace, or mutate the
external final executable; it makes only an ephemeral private copy for
verification. This closes the maintained workflow gap without weakening the
separate audit: the audit still reopens the source, candidate, cells, evidence,
and manifest and independently recalculates their release verdict.

The descriptor producer runs in the same unchanged runner and Git, Rust, and
Cargo toolchain environment that built the candidate. Its measured environment
strings are non-adversarial coordinates, not independent proof of that
precondition.

Every gate evaluates a fixed twelve-cell matrix: six feature identifiers for
each of `codex` and `claude_code`, with one exact host availability coordinate
per host kind. The host availability triple is independently all-string or
all-null. The top-level and environment client name/version quartet is also
independently all-string or all-null, and a non-null client requires a non-null
host. A present unavailable-host matrix uses null host and client coordinates;
an implemented cell is `ignored` and remains a downgrade. A static unsupported
cell is `not_applicable` and may retain non-null host availability while using
null client identity because its disposition may short-circuit before MCP
initialize. An implemented requested claim remains eligible to fail when
identity is missing; only an explicit exclusion sets
`requested_verified=false`.
The v3 evaluator validates each static disposition against the host-kind owner
table. Codex and Claude Code implement all six features. The exact reviewed
Codex raw version-probe envelope is `codex-cli 0.144.4`, from which the cell
stores the bare canonical `0.144.4`; every non-null Codex version passes the
shared bare parser and a raw probe envelope in `host_version` is structurally
invalid. A null, reviewed, or unreviewed version never selects or changes the
host-kind implementation disposition. The reviewed coordinate is not a
minimum-version claim.
Each non-null client pair comes only from the successful managed MCP
`initialize` used by that cell. It is not inferred from host kind, executable,
probe output, environment, configuration, protocol version, constants, later
tool metadata, or another cell. All non-null cells for one host use one exact
client pair. An implemented exact-live cell requires
`client_version == host_version`; missing identity derives
`client_identity_missing`, while a version or expected-identity mismatch
derives `client_identity_mismatch`, and either result is
`implemented_unverified`. Reviewed Codex `0.144.4` additionally requires
`codex-mcp-client`/`0.144.4`. Only the bounded name/version pair is retained for
the recorder; raw initialize or protocol/session/thread/turn payload is not
release evidence. The recorder compares bounded before/after observations in
the cell's bound clean disposable Runtime Home and accepts only the exact
managed baseline rows for the authenticated cell turn that were created or had
their metadata changed during that turn. An unchanged historical row for the
same connection is never client provenance, and connection-wide newest or
unique-value selection is rejected.
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

For an implemented cell, `completed` means terminal recording, not success. A
classifiable selected attempt that binds an installed host and then fails is
represented by a strict committed `completed` cell with bounded evidence and
every unproven required assertion false only after child finalization,
after-turn baseline capture, and retained-integrity revalidation succeed. That
clean cell derives `implemented_unverified` and remains admissible matrix input.
It is distinct from a structural publication failure: incomplete child
finalization or after-turn baseline capture, retained-integrity failure,
failure to commit exact `clean`, publication I/O failure, or inability to
construct strict terminal bytes forbids a final cell and poisons the result
root.

For every interactive selected-host turn, the runner's process group must
initially own the controlling-terminal foreground. A bounded foreground
controller becomes ready in the dedicated containment group and transfers the
foreground to that exact group before the selected host can read. The host is
started in the same verified group and retains the existing ownership marker.
After the host exits and the direct child is reaped, the controller restores
the foreground to the original runner group and is itself reaped. After the
dedicated containment boundary reaches quiescence, the producer reapplies and
exactly verifies the complete pre-transfer terminal attributes before
after-turn baseline capture or terminal publication. Its readiness,
restoration, and reap waits are bounded. The
dedicated group and marker-retaining process discovery remain the cooperative
containment boundary. Controller readiness, liveness, transfer, restoration,
or reap failure, any host group mismatch, and terminal-attribute restoration or
verification failure forbid terminal publication and poison the result root.
This job-control protocol assumes an existing
controlling terminal; it neither creates nor claims a pseudo-terminal (PTY).

The gate creates a bounded external
`volicord-host-release-manifest-v3` file without overwriting. After the gate
process exits, a separate process independently reopens the source candidate,
the twelve original cell files, cell evidence, and manifest, recomputes their
SHA-256 values, invariants, statuses, findings, exclusions, and verdict, and
requires the original cells to equal the manifest's embedded raw cells. It
then creates a bounded external
`volicord-host-release-audit-v3` file without overwriting. Its cell-input-set
digest uses the `volicord-host-release-cell-inputs-v3` domain. The audit may not
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

The native UserAction cell treats the exact Task-bound Stop receipt as an
authority observation, not as a close-readiness or final-output claim. It
preserves and validates the Stop decision, reasons, close state, and complete
blocker set. Both an internally consistent ready `allow` and an internally
consistent Detective `allow` with `completion_claim_allowed=false` are eligible
only in the two clean fixture forms owned by Host Release Evidence:
full-coverage ready `allow`, or the exact active partial-coverage
`session_watch_unavailable` incomplete disclosure. Any other outcome fails the
cell. The LocalUser ready/blocker-free check remains a separate clean-fixture
precondition. A blocked close projection does not become close-ready and is not
compared byte-for-byte with that LocalUser receipt. The
two receipts must agree only on their shared Project, Task, state-version, and
latest-Run authority coordinates. Final-output display, authenticated replay,
and block-finalization remain separate matrix features.

The validation implementation is isolated in the test-only
`tests/release-validation` workspace package. It may reuse implementation-owned
evaluators, but production targets do not depend on it. Its maintained command
routes are owned by Host Release Evidence and the Maintain Validation page.
The candidate-descriptor producer, live-cell producer, gate, and audit directly
reuse that package's canonical external-path context. After adding the bound
disposable Runtime Home and revalidating retained paths, the live-cell producer
pins the precreated result root and
its `cells/` and `evidence/` directories and holds a cooperative exclusive
result-root lease before any host launch. Under that lease it prevalidates
absent final names and prior committed-pair consistency, synchronizes a bounded
private lease state from `clean` to `active`, stages and synchronizes complete
bounded bytes in private sibling files, publishes implemented-cell evidence
first with atomic no-replace semantics, and publishes the cell last as the
commit marker for its evidence pair. Static unsupported cells publish only the
cell. After synchronizing
the final cell and its directory, the producer writes the exact `clean` state;
that complete record, when observed under a later lease, is the authoritative
publication commit marker. `active`, empty, partial, and malformed states are
rejected. A post-write synchronization acknowledgement can be indeterminate,
so a reported error or termination after complete `clean` bytes become visible
may leave a committed clean root; a later process cannot infer whether the
producer observed success. Published final names and result directories are
never rolled back or deleted, and the maintained runbook abandons the root
after every reported publication error or abnormal exit. The gate and separate
audit hold a cooperative shared lease while reopening the cell set. No layer
maintains a source-only approximation or repairs a failed publication root.

## Consequences

- A release claim is integrity-bound to one declared clean revision, one
  external final executable, one target, one exact profile, and exact host
  availability coordinates, subject to the non-adversarial provenance limit
  above.
- A producer cannot promote support by claiming a status or omitting an
  inconvenient cell.
- Stale or partial results remain visible as downgrades rather than being
  silently mixed with another run.
- Missing, inferred, or mismatched managed client identity cannot produce a
  verified live cell.
- The manifest and separate audit are durable release review inputs but do not
  create Core evidence, user authority, host attestation, or runtime trust.
- Production local-web manifest acquisition remains unavailable and therefore
  fail-closed; CLI inbox remains the supported fallback.
- Native session identifiers do not enter Volicord storage, diagnostics, or
  release evidence.
- The exact maintained partial-coverage Detective incomplete disclosure remains
  visible in native UserAction evidence instead of being rewritten as
  close-ready; other Stop outcomes fail the clean fixture.
- Forbidden or non-canonical release paths fail at the producer before an
  authenticated host run or release-artifact write, and the gate and audit
  independently enforce the same decision when reopening inputs.
- Cooperative producers serialize per result root; an uncooperative concurrent
  name can make publication fail but cannot be overwritten or later deleted by
  the producer.
- Interactive selected hosts keep terminal input and containment aligned: the
  dedicated containment group owns the controlling-terminal foreground only
  for the bounded host turn, after which the original runner group is restored.
- A committed implemented cell can become visible only after its complete
  evidence final name; static unsupported cells remain cell-only.
- A crash or I/O failure may leave bounded private stages, orphan evidence, or
  already installed final names under a non-clean state. This deliberate
  append-only residue is safer than rollback deletion; no installed cell is
  admissible unless the coordination state is exactly `clean`.
- Exact `clean` is the observable state-commit point. `active`, empty, partial,
  and malformed records preserve ineligibility across process death even when
  no other filesystem remnant is visible; the record is coordination state,
  not release evidence or a digest input.
- Recovery uses a fresh result root and reruns the complete twelve-cell matrix.

## Non-goals

- This decision does not add a public API method or production import command.
- It does not establish minimum Codex or Claude Code versions.
- It does not prove OS isolation, host identity, user identity, or absence of
  later host changes.
- It does not create, provision, or attest a pseudo-terminal (PTY).
- The cooperative lease is not hostile same-user exclusion or a claim about
  non-conforming writers or non-standard network-filesystem lock semantics.
- It does not prove build reproducibility or attest source-to-binary provenance
  against a malicious candidate producer.
- It does not permit results from different host versions or candidates to be
  combined.
- It does not make external release artifacts trusted production inputs.

## Compatibility and migration

This decision advances the test-only cell, manifest, audit, and cell-input
digest contracts to v3 and binds live cells to actual managed initialize
identity; it does
not change a public Core API schema, public MCP method, SQLite DDL, or
storage-profile version. V1 and v2 cell, manifest, audit, and cell-input-domain
inputs remain historical and are rejected rather than imported, migrated, or
reinterpreted. The candidate stays `volicord-release-candidate-v1` and the
source archive algorithm stays `git_archive_tar_sha256_v1` because their
preimages did not change.

The reserved `mhs_` rules intentionally reject generic, cross-host, or
cross-connection preseeded values and invalid managed markers. No legacy alias,
fallback mapping, or decoder is added; compatible current observations are
recreated through the managed adapter. The batch remains within the current
workspace SemVer because it does not add or break a supported public API or
deployment surface; its externally stored v3 artifacts are opt-in release
validation output.

The maintained candidate command serializes the existing candidate-v1 contract
from an already staged final executable. It changes no field, digest preimage,
schema identifier, accepted candidate, public API, or deployment behavior, so
it does not advance the candidate schema or workspace SemVer.

Using the shared external-path evaluator is a conformance correction to the
existing v3 path contract. It does not change any artifact field, digest
preimage, schema identifier, or valid artifact, so the candidate v1 and release
artifact v3 identifiers remain current. Producer outputs previously accepted
at forbidden paths were already invalid and rejected by the gate or audit; no
migration or compatibility fallback is provided.

The append-only publication protocol is another conformance correction within
the existing v3 producer contract. It changes no serialized member, allowed
value, digest preimage, status derivation, verdict, schema identifier, or
strict-valid completed artifact. It therefore does not advance candidate v1,
release artifact v3, the workspace SemVer, public API schemas, MCP methods,
SQLite DDL, or storage-profile versions. Current gate and audit commands do,
however, require the maintained `RESULT_ROOT/cells` and sibling `evidence/`
layout plus an exact clean coordination record. A pre-protocol unleased v3
artifact remains schema-valid as an individual artifact but is not an accepted
input set for those commands. There is no migration, lease synthesis, or
adoption path; rerun the complete twelve-cell matrix in a fresh result root.

Separating native UserAction receipt observation from close readiness is also
a conformance correction within v3. The exact assertion set, serialized cell
members, digest preimages, and final-output feature contracts do not change, so
the release artifact v3 identifiers and workspace SemVer remain current.
Previously generated evidence that claimed whole-receipt equality across
LocalUser and Agent Connection contexts must be regenerated; it is not
reinterpreted or migrated.

Clarifying terminal failed-cell recording and the durable source barrier is
likewise a v3 conformance correction. It changes no serialized member or
allowed value in the four versioned release schemas, no assertion identifier,
and no v3 evaluator cell-input digest-preimage definition, status derivation,
or verdict. The producer's referenced diagnostic evidence bytes and their
digest may change and must be regenerated; they are not a fifth versioned
schema. The four current schema identifiers and workspace SemVer therefore
remain unchanged. A producer that waited for host-process exit or omitted a
classifiable failed cell must be rerun against a fresh exact candidate and
result root.

The same correction closes the previously undefined Claude Code installed-
version probe envelope. Only a successful stdout-only canonical line can bind
the existing exact `host_version` coordinate; merged streams, selected or
trimmed lines, malformed UTF-8, and non-success exits cannot. Existing cells
whose Claude coordinate came from those ambiguous forms must be regenerated,
but the coordinate's 1–1,024-byte contract and every versioned release-schema
member remain unchanged.

Controlling-terminal foreground transfer and restoration are likewise a v3
producer conformance correction. They change no candidate, cell, manifest, or
audit member, digest preimage, assertion identifier, status derivation, or
verdict. Evidence from an interactive run whose dedicated group never owned
the terminal foreground, or whose original runner foreground was not restored,
must be regenerated in a fresh result root; no compatibility fallback converts
that structurally invalid run into a non-passing cell.

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
- Reinterpreting v1 or v2 cells, manifests, audits, or cell-input digests under
  v3 semantics was rejected because a historical digest must retain one
  meaning.
- Inferring client identity from host kind, a version probe, configuration,
  protocol version, constants, or another cell was rejected because none is the
  actual client observed for that live run.
- Binding Codex from `CODEX_THREAD_ID`, timing, arrival order, the newest open
  session, or proximity was rejected because concurrent and resumed sessions
  can produce indistinguishable but swapped pairings.
- Keeping a smaller source-only validator in the live producer was rejected
  because it permits host work and filesystem effects that the canonical gate
  and audit must later discard.
- Assigning the interactive host to a dedicated process group without also
  transferring the controlling-terminal foreground was rejected because a
  background group can be stopped on terminal input and produce a runner-
  induced timeout.
- Keeping the interactive host in the runner's foreground process group was
  rejected because it removes the dedicated containment coordinate and weakens
  group-wide cleanup.
- Direct final-file reservation followed by rollback deletion was rejected
  because a concurrent replacement can be deleted after the identity check.
- Quarantine rename followed by identity check and unlink was rejected because
  the quarantine name can itself be replaced and the move can displace a
  foreign final-name replacement.
- Publishing the cell before its evidence was rejected because a visible cell
  would falsely act as a commit marker for incomplete evidence.
- Cleaning remnants and retrying in the same result root was rejected because
  no automatic cleanup can safely recover every concurrent or crash state.
- Requiring native UserAction evidence to manufacture a Stop `allow`, ready
  close state, or byte-identical LocalUser receipt was rejected because close
  blockers are invocation-context projections and those requirements conflate
  native elicitation with the separately owned final-output features.
- Treating `completed` as a passing result, or waiting for Stop, turn, or
  process completion after a producer source-observation event is durably
  persisted, was rejected because it would hide failed assertions and can let
  the short-lived capture intent expire for an unrelated lifecycle reason.
- Merging version-probe stdout and stderr, trimming lines, or selecting one
  plausible line was rejected because those transformations cannot preserve an
  exact installed-host availability coordinate or distinguish ambiguous output.

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
