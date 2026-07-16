# Host Release Evidence

This document owns the exact-final-artifact release-validation contract for
managed Codex and Claude Code host features. It defines the release candidate,
cell, manifest, and independent-audit schemas; the fixed validation matrix;
canonical status derivation; freshness; gate verdicts; and privacy-preserving
managed-host session binding.

It does not define public Core API methods, production manifest acquisition,
host trust, operating-system isolation, or runtime storage. Administrative CLI
output is an auxiliary projection of this contract. Production local-web
eligibility remains governed by [Agent Connection](agent-connection.md) and
[MCP Transport](mcp-transport.md).

<a id="surface-stability"></a>
## Surface Stability

The four schema identifiers, their required fields, the fixed twelve-cell
matrix, `git_archive_tar_sha256_v1`, canonical status and verdict derivation,
the half-open freshness rule, and the managed-host session mapping are stable
release contracts. Adding a host, feature, status, digest algorithm, required
field, or verdict requires a versioned successor schema and a paired
architecture decision. File paths, private Rust types, process layout, and
human-readable rendering are implementation details unless this page names
them explicitly.

## Contract Identifiers

| Role | Exact identifier |
|---|---|
| Exact candidate descriptor | `volicord-release-candidate-v1` |
| One live-host matrix result | `volicord-host-release-cell-v3` |
| Canonical release-gate result | `volicord-host-release-manifest-v3` |
| Separate-process recalculation | `volicord-host-release-audit-v3` |
| Source archive digest algorithm | `git_archive_tar_sha256_v1` |
| Cell-input-set digest domain | `volicord-host-release-cell-inputs-v3` |

All four artifacts are canonical UTF-8 JSON objects with duplicate keys
rejected. Unknown fields are rejected. SHA-256 values are lowercase 64-hex,
timestamps are canonical UTC RFC 3339 with second precision, and schema
identifiers are exact strings. A producer creates each destination as a new
file and must fail if it already exists. A cell JSON file is at most 1 MiB; a
manifest or audit JSON file is at most 4 MiB; and one referenced cell evidence
artifact is at most 16 MiB. Paths named by these schemas must be absolute,
normalized, and external to the source checkout, Cargo target directory,
maintained documentation, and every Volicord Runtime Home.
Configured exclusion roots are normalized before overlap checks. A relative
`CARGO_TARGET_DIR` is resolved from the source checkout, while a relative
`VOLICORD_HOME` is resolved from the invoking process's current directory;
existing symlink prefixes and dot components do not weaken an exclusion.

<a id="external-release-path-policy"></a>
### Canonical external release-path policy

The candidate-descriptor producer, live-cell producer, gate, and independent
audit apply one canonical external-path evaluator to every release-artifact
input and create-new destination they consume. The common exclusion set contains
the canonical source checkout, resolved Cargo target directory, maintained `docs/`
directory, the explicit process Runtime Home, the default home-derived Runtime
Home, every additional caller-supplied Runtime Home, each disposable Runtime
Home bound to the current cell, and any registry-bearing ancestor discovered
for an artifact path. Candidate descriptors, `candidate_path`, cell
directories and outputs, evidence sidecars, manifests, and audit inputs and
outputs all use this evaluator.

Every accepted artifact path must have an exact UTF-8 representation. An
existing input or directory must be canonical and have no symlink component. A
create-new output must be absent and have an existing canonical, symlink-free
parent. A manifest or audit destination must also be outside the supplied
result root's `cells/` and `evidence/` directories so writing the summary
cannot mutate its own cell or evidence input set. If the producer binds the cell's disposable Runtime Home after
initially accepting a candidate or result path, it adds that root to the same
exclusion context and revalidates every retained path before acquiring the
result-root lease, launching a host, reading or executing the candidate again,
staging output, or publishing a final name.

A path-policy rejection, inability to acquire the cooperative result-root
lease, or a final destination already present during leased prevalidation is a
structural command error. It occurs before an authenticated host launch, or
before terminal publication for a static unsupported cell, and this producer
publishes no final release name. It is not a downgrade, an `ignored` cell, or an
audit exclusion. A producer-only source check, a parallel path policy, or
deferring the first rejection to the gate or audit is non-conforming.

<a id="append-only-live-cell-publication"></a>
### Append-only live-cell publication

A maintained matrix cell uses one new external result root. The result root
and its exact `cells/` and `evidence/` child directories must already exist, be
canonical, contain no symlink component, and satisfy the shared external-path
evaluator. The cell destination is a direct child of `RESULT_ROOT/cells`; an
implemented cell's derived evidence destination is a direct child of
`RESULT_ROOT/evidence`. The producer does not create, replace, rename, or
remove either directory.

After binding the disposable Runtime Home and revalidating all retained paths,
the producer opens and pins the result root and both child directories,
acquires one cooperative exclusive lease associated with that result root, and
holds the lease through host execution and terminal publication. While holding
the lease and before host launch, it requires the cell final name and, for an
implemented cell, the evidence final name to be absent. A static
`unsupported_by_host` cell performs the same leased cell-name check before its
host-free terminal publication. The lease serializes conforming producers;
final-name installation still uses atomic no-replace semantics so an
uncooperative concurrent writer cannot be overwritten. The stable private
coordination entry used for the lease also carries one bounded, synchronized
publication state. A new entry starts `clean`. After leased consistency checks
and before host launch, an implemented producer changes it to `active` and
synchronizes it; a static unsupported producer does so before its host-free
publication. Only after the final cell and owning directory are synchronized
does that producer begin replacing `active` with the complete exact `clean`
record and request its synchronization. A complete exact `clean` record
observed under a later lease is the authoritative publication commit marker.
An empty, partial, malformed, or `active` state is a structural failure. This
coordination entry is not a release artifact or an input to any release digest.

Before changing `clean` to `active`, a later producer requires every existing
cell-directory entry to be a bounded strict-valid committed cell, requires
every evidence-directory entry to be named exactly once by one such cell with
matching bytes and digest, and rejects a private stage, orphan evidence,
missing evidence, or already complete cell set. This consistency scan does not
make remnants admissible and does not repair them; it durably enforces
fresh-root recovery across producer processes.

Once terminal bytes are known, an implemented-cell producer creates at most
one private create-new stage file in each pinned owning directory. It
completely writes, bounds, and synchronizes the evidence stage, calculates the
recorded evidence digest from those held bytes, then completely writes, bounds,
and synchronizes the cell stage that names that final evidence path and digest.
It atomically installs the evidence final name first with no-replace semantics
and synchronizes the evidence directory. It installs the cell final name last
with the same semantics and synchronizes the cell directory. A strict-valid
final cell is the only commit marker for that cell/evidence pair. A static
`unsupported_by_host` cell has null evidence and stages and publishes only its
final cell. No producer publishes a provisional `running` cell.

A selected feature failure or selected-host child-process failure that the
producer can classify into strict bounded terminal bytes is not a publication
failure, provided that the direct child was reaped, the maintained cooperative
process-containment boundary was made quiescent, and the after-turn managed
baseline, retained identity, candidate digest, and publication domain were all
revalidated. The maintained boundary is the dedicated operating-system process
group assigned when the selected host is launched, supplemented, where the
runner supports it, by discovery of processes that retain the turn's inherited
ownership marker after leaving that group. Quiescence requires the direct child
to be reaped, the process group to have no live member, and every discoverable
marker-retaining process to have terminated. This is cooperative containment,
not an adversarial sandbox: a host adapter that both daemonizes outside the
assigned group and removes the inherited marker is outside the validated runner
profile and must not be claimed as live-host verified.

An interactive selected host that reads from a controlling terminal has an
additional job-control invariant. The runner's original process group must
initially own that terminal's foreground. Before the selected child can read,
a bounded foreground controller must become ready in the dedicated operating-
system process group, transfer the controlling-terminal foreground to that
exact group, and keep the restoration path available. The producer starts the
selected host in that same group, verifies its membership, and retains the
turn ownership marker. After the direct child exits and is reaped, the bounded
controller restores the foreground to the original runner process group and
is itself reaped. After the dedicated group and marker boundary reach
quiescence, the producer reapplies and exactly verifies the complete terminal
attributes captured before transfer. Foreground restoration gates group
signaling; quiescence and attribute restoration must both finish before
after-turn baseline capture or terminal publication. Controller readiness,
restoration, and reap waits are all bounded. The dedicated group remains the containment group;
foreground transfer does not replace the ownership-marker supplement. This
invariant neither creates nor attests a pseudo-terminal (PTY); the runner must
already have a controlling terminal. The validated runner profile requires the
terminal's `TOSTOP` local mode to be disabled and excludes operator-initiated
job-control suspension of the selected turn, such as `Ctrl-Z`; a suspended or
resumed turn is not release evidence. Controller readiness, liveness, or reap
failure, a group mismatch, enabled `TOSTOP`, or foreground transfer,
verification, foreground restoration, or terminal-attribute restoration failure
forbids final-name publication, poisons the result root, and cannot be
represented by a non-passing cell.

If the producer then
publishes those evidence and cell bytes and
commits the exact `clean` record, the non-passing cell is admissible matrix
input and the result root remains structurally usable. It must not be replaced
or retried in that root. Failure to reap the direct child, verify that
cooperative containment boundary as quiescent, or establish the after-turn
baseline, any retained-baseline or identity replacement, a producer exit before
exact `clean`, a publication I/O failure, or inability to construct strict
terminal bytes instead forbids final-name publication, poisons the result root,
and requires the fresh-root recovery below. None of those integrity failures is
converted into an `implemented_unverified` cell. A runner on which the
maintained producer cannot establish the dedicated process group, or a reviewed
host profile known to violate the cooperative-containment precondition, must
reject the selected live attempt structurally rather than publish a cell.

Publication is append-only. The producer never unlinks, replaces, renames
away, or rolls back a published final name and never removes the result root or
its `cells/` or `evidence/` directory. On an I/O error, abnormal termination,
or concurrent-name loss, it also does not perform check-then-delete cleanup.
A failed implemented-cell attempt may leave up to two bounded private stages,
an installed evidence final name without a producer cell, or both installed
final names if failure occurs after the cell rename but before publication
commit is acknowledged. A failed static unsupported attempt may likewise leave
a bounded private cell stage or its installed final cell. An installed final
cell under an `active`, empty, partial, or malformed coordination state is not
an admissible committed cell.

An error or termination before the complete exact `clean` record becomes
observable leaves an `active`, empty, or malformed state and poisons that
result root. A write or synchronization acknowledgement can be indeterminate:
an error or termination after the full `clean` bytes become observable may
leave a committed clean root even though the producer did not observe a
successful return. A later process cannot infer that return; it treats only the
exact `clean` record as the state commit and still independently checks the
complete cell/evidence set. This is not repair or remnant adoption. The
producer reports every observed failure and does not retry, and the maintained
operator runbook conservatively abandons the root after any reported
publication error or abnormal termination. Recovery uses a fresh external
result root with newly precreated `cells/` and `evidence/` directories and
reruns the complete twelve-cell matrix; prior cells, private stages, orphan
evidence, and installed final names from the abandoned root are not copied,
adopted, or synthesized.

The gate and audit never repair or clean a result root. Before reopening cells,
each acquires and retains a cooperative shared lease for that result root; an
active or stale-`active` producer state, or an invalid lease entry, is a
structural command failure. They
accept exactly twelve final `.json` entries in the supplied cell directory and
follow only the evidence paths named by those strict-valid cells. A missing
final cell, an additional cell-directory entry including a private stage, or
malformed or mismatched referenced evidence is a structural command failure.
An unreferenced private evidence stage or orphan evidence file is outside the
input set and cannot satisfy a missing cell. These remnants are neither
downgrades nor audit exclusions.

The v3 gate and audit reject the historical v1 and v2 cell, manifest, audit,
and cell-input digest-domain identifiers. They do not import, migrate, or
reinterpret those artifacts under v3 rules. The candidate remains
`volicord-release-candidate-v1`, and the source archive algorithm remains
`git_archive_tar_sha256_v1`, because neither candidate nor archive preimage
semantics changed.

## Exact Release Candidate

`volicord-release-candidate-v1` contains exactly these required members:

| Member | Contract |
|---|---|
| `schema` | Exact value `volicord-release-candidate-v1`. |
| `candidate_id` | Non-empty opaque identifier unique within the release run. |
| `candidate_path` | External absolute path to the one final executable tested by every cell. |
| `source_revision` | Lowercase 40- or 64-hex commit object ID. |
| `source_clean` | Must be `true`; a dirty or untracked source tree is ineligible. |
| `source_archive_algorithm` | Exact value `git_archive_tar_sha256_v1`. |
| `source_archive_sha256` | SHA-256 of the deterministic source archive below. |
| `target_triple` | Exact Cargo target triple used for the candidate. |
| `release_profile` | Exact maintained Cargo profile name `release`; an approximate profile class or any other profile is invalid. |
| `binary_sha256` | SHA-256 of the bytes at `candidate_path`. |
| `build_environment` | Exact `runner_os`, `runner_os_version`, `runner_arch`, `git_version`, `rustc_version`, and `cargo_version` strings. Candidate v1 accepts each as 1 through 512 control-free UTF-8 bytes. |
| `recorded_at` | Time at which the descriptor and all candidate digests were completed. |

The source checkout must be clean before building and remain at
`source_revision` through candidate creation. From that checkout, the producer
runs `git archive --format=tar <source_revision>` with no path prefix or extra
attributes and computes SHA-256 over the command's raw tar stdout. That byte
digest is `source_archive_sha256`; hashing a compressed archive, work tree,
directory listing, or Git bundle is not equivalent.

The maintained descriptor producer consumes an already staged final executable
and an absent external descriptor path:

```sh
cargo run --locked -p volicord-release-validation-tests --bin host-release-candidate -- --candidate-id CANDIDATE_ID --candidate-path CANDIDATE_BINARY --candidate-out CANDIDATE.json
```

It validates both paths with the canonical external-path evaluator before
executing candidate-controlled bytes, derives the clean HEAD and raw source
archive digest, hashes and privately inspects the exact executable, records the
bounded runner and toolchain coordinates, and then repeats the executable and
source stability checks. The producer trims command-output boundaries and emits
each coordinate as 1 through 512 control-free UTF-8 bytes with at least one
non-whitespace character. It creates `CANDIDATE.json` only after those checks
pass. The output path must not exist and is never overwritten. This command
does not build the candidate or stage, replace, or mutate the external final
executable. Any required publisher-side staging or post-processing must finish
before invocation. The command's only copy is the ephemeral private
verification copy described below, which is not `candidate_path` or a release
artifact.

The descriptor producer runs in the same unchanged runner and Git, Rust, and
Cargo toolchain environment that built the candidate. The `build_environment`
strings are measurements of that producer process under this precondition;
they remain the non-adversarial, independently unattested coordinates described
below. A candidate moved from a different build environment, or a toolchain
changed between build and descriptor creation, is ineligible.

Before executing candidate-controlled bytes, the gate opens a regular file at
`candidate_path`, hashes that held handle, and requires an exact descriptor
digest match. It copies the held bytes to a private create-new executable,
requires the copy digest to match, closes the writer, and executes only the
private copy with the ambient environment cleared. After execution it requires
the held digest and the final pathname's digest and file identity to remain
unchanged through the `candidate_binary_final_stable` invariant; a stability
mismatch produces a fail manifest. A descriptor or private-copy digest mismatch
is instead a command error and produces no manifest, and candidate-controlled
bytes are never executed after that pre-execution mismatch.

The embedded `--version` build metadata and source-archive checks are
non-adversarial provenance and coordinate-integrity checks. The gate does not
rebuild the candidate, prove a reproducible build, or attest that the named
source revision produced arbitrary candidate bytes.

The private candidate's `--version` output must parse to these exact build
invariants: package version equal to the gate package's inherited workspace
SemVer, `git_commit=source_revision`, `tree=clean`,
`metadata_source=environment`, `target=target_triple`, `profile=release`,
`profile_class=release`, and `profile_exact=true`. Failure of any one is a gate
invariant failure. The descriptor's build-environment strings remain recorded
non-adversarial coordinates; they are not independently attested by this
comparison.

The exact final executable is built once, copied to `candidate_path`, and never
rebuilt, patched, stripped, signed in place, or replaced during the gate. A
publisher that needs a post-processing step performs it before calculating the
candidate descriptor. Gate and audit commands run from the same checkout at
`source_revision` and require it to remain clean while they recalculate the
declared archive and source coordinates.

## Fixed Twelve-Cell Matrix

One manifest contains exactly one cell for every Cartesian pair below:

- `host_kind`: `codex`, `claude_code`
- `feature`: `native_user_action`, `local_web_user_channel`,
  `verified_tool_producer`, `registered_connection_observation`,
  `record_final_output`, `detective_final_output`

The result is twelve present JSON cell files. A duplicate, omitted, additional,
differently named, malformed, or partially null required field group is a
structural command error and produces no manifest. All six cells for one host
kind share one host-availability coordinate: either all name the same exact
`host_version` and non-null executable digest, or all use explicit `null` for
the host version and executable digest. Among cells that carry a non-null client
identity, all six use at most one exact `client_name` and `client_version` pair.
A statically `unsupported_by_host` cell may use null client identity even when
the host-availability coordinate is non-null because its static disposition may
short-circuit before MCP initialize. Results from different non-null host
versions, executables, or client identities must never be aggregated into one
host result. A new host version or client identity requires a new complete
twelve-cell manifest.

`volicord-host-release-cell-v3` contains exactly these required members:

| Member | Contract |
|---|---|
| `schema` | Exact value `volicord-host-release-cell-v3`. |
| `candidate_id`, `binary_sha256`, `source_revision`, `target_triple`, `release_profile` | Exact copies of the candidate coordinates. |
| `host_kind`, `host_version` | One fixed host kind and either the exact installed host version observed by the cell or explicit `null` when that host is unavailable. The member is always required. |
| `client_name`, `client_version` | Required-nullable exact `clientInfo.name` and `clientInfo.version` observed from this cell's successful managed MCP `initialize`. Each non-null value is 1 through 256 UTF-8 bytes, contains at least one non-whitespace character, contains no control character, and is otherwise preserved exactly. Static unsupported cells may use explicit `null` without initializing MCP. |
| `adapter_profile`, `adapter_version` | Exact managed adapter coordinates. |
| `feature` | One of the six fixed feature identifiers. |
| `implementation_disposition` | `implemented` or `unsupported_by_host`; this is owner-reviewed static input, not a live result. |
| `requested_verified` | Boolean release claim requested for this exact host availability/feature. An implemented cell defaults to `true`; explicit `false` is a release exclusion and downgrade. Static unsupported cells require `false`. |
| `claimed_status` | Producer claim using `HostFeatureSupportStatus`; retained only for mismatch reporting and never trusted. |
| `run_state` | `completed`, `running`, `ignored`, or `not_applicable`; only static `unsupported_by_host` may use `not_applicable`. |
| `started_at`, `recorded_at` | Cell start and immutable result-recording times. |
| `environment` | Exact `runner_os`, `runner_os_version`, `runner_arch`, required-nullable `host_executable_sha256`, `host_version`, `client_name`, and `client_version`, and all host/adapter coordinates used by the run. Its duplicate identity values exactly match the top-level values. |
| `assertions` | Non-empty bounded array of stable assertion IDs with `passed` booleans and optional bounded finding codes. |
| `evidence_artifact_path`, `evidence_artifact_sha256` | External create-new bounded evidence file and SHA-256; both remain required for an implemented cell, including an unavailable ignored cell, and both are `null` only for static `unsupported_by_host`. |

The existing host-availability group—top-level `host_version`,
`environment.host_version`, and `environment.host_executable_sha256`—is either
all strings or all explicit `null`. Separately, the four client members—
top-level and `environment` copies of `client_name` and `client_version`—are
either all strings or all explicit `null`. Non-null client members require a
non-null host-availability group. When present, each `environment` client value
must exactly equal its top-level copy. Omission of a required-nullable member, a
partial-null group, a wrong type, or non-null client identity with null host
availability is a structural error rather than a downgrade. A duplicated-value
mismatch is a coordinate mismatch, and a copied or inferred identity is not an
observed coordinate.

The v3 evaluator validates `implementation_disposition` against the exact
host-version-aware owner table rather than accepting the producer's value as
an independent fact. For Codex, exact canonical `host_version=0.144.4` is
reviewed as `implemented` for `native_user_action`,
`verified_tool_producer`, and `registered_connection_observation`, and as
`unsupported_by_host` for `local_web_user_channel`, `record_final_output`, and
`detective_final_output`. The exact installed probe output is
`codex-cli 0.144.4`; cells store only the parsed canonical bare coordinate
`0.144.4`, not the probe envelope. Every non-null Codex `host_version` must pass
the shared canonical bare-version parser. A raw probe envelope such as
`codex-cli 0.144.4` in `host_version` is a structural error.
For absent or unreviewed Codex versions, the host-kind fallback keeps the
first four features `implemented` and both final-output features
`unsupported_by_host`; lack of exact evidence leaves those implemented cells
`implemented_unverified`. Claude Code keeps its host-kind fallback of all six
features `implemented`. A new reviewed version table requires an owner change
and a complete new twelve-cell manifest.

This exact-version table is a release-gate evidence coordinate, not an ordinary
runtime feature gate. Runtime support remains capability-probe-first under
[Agent Connection](agent-connection.md#host-feature-support-state). A different
or newer valid installed version is `implemented_unverified` for an implemented
surface until its probes and fresh evidence establish another state; it is not
`unsupported_by_host` solely because it lacks a row here. A release claim for
that version still requires an owner change and its own complete twelve-cell
manifest.

The only admissible non-null client identity is the exact pair observed from
the successful managed MCP `initialize` used by that cell. It must not be
inferred from `host_kind`, the host executable name, version-probe output,
environment or configuration, protocol version, known constants, later tool
metadata, or another cell. A recorder may read the bounded top-level
`client_name` and `client_version` retained in that managed session's
`session_watch_baselines.metadata_json`; it must not retain or consume the raw
initialize message or raw protocol, session, thread, turn, or tool-call payload
as release evidence.

Before any authenticated cell host process starts, the recorder must
monotonically bind the exact prepared Agent Connection ID obtained from that
cell's initialization result. Rebinding the same exact ID is idempotent. A
missing, malformed, or conflicting binding is a terminal structural recorder
failure: the producer must not launch the host process or publish either final
name. A static unsupported or unavailable-host path that launches no host may
remain unbound.

Before the authenticated cell host turn, the recorder establishes a bounded
before-observation of the exact managed baselines in the cell's bound, clean,
disposable Runtime Home. It establishes the corresponding after-observation
before final cell recording. It may accept client identity only from baseline
rows whose opaque managed session, connection, project, and host coordinates
match that authenticated turn and that were either created during the turn or
had `metadata_json` change between those observations by recording that turn's
successful managed `initialize`. A row that existed with unchanged metadata at
both observations is historical and is never evidence for the cell, even when
it belongs to the same connection and contains the expected pair. The recorder
must not search connection-wide history and substitute its sole or newest
identity. All qualifying rows must expose one identical exact pair. No
qualifying row leaves the client group null; a partial, malformed, or divergent
qualifying result stops cell recording rather than being filled, replaced, or
inferred.

For every qualifying row, the after-observation retains the exact
`{project_id, watch_baseline_id}` key and the SHA-256 of the exact
`metadata_json` bytes as that row's expected after-turn digest. Session and
connection fields are not duplicated into that key: the canonical
baseline ID binds the validated opaque session, while the reopened row and
metadata digest bind the exact connection, project, and host coordinates. An
additional authenticated turn in the same cell may advance an already retained
key only
when that turn's before-observation contains the same expected digest; its
after-observation then replaces the expected digest. An unchanged replay keeps
the existing expected digest. Before entering publication, the recorder
reopens every retained key and requires the row to exist with the exact
managed session, connection, project, host, canonical baseline ID, and
expected metadata digest. Deletion, same-key replacement, or a metadata change
not covered by a later captured turn stops recording before this producer
publishes either final name. It does not remove or replace a concurrently
present name and does not convert the failure into a null client group or an
`implemented_unverified` cell.

Every cell with non-null client identity for one host kind uses one exact client
pair. Every implemented exact-live cell can derive `verified` only when
`client_version == host_version`. The reviewed Codex
`host_version=0.144.4` coordinate additionally requires
`client_name=codex-mcp-client` and `client_version=0.144.4`.

The canonical `adapter_profile` is `record` only for
`record_final_output`; it is `detective` for the other five features,
including `detective_final_output`. `adapter_version` is the exact `build_id`
parsed from the private candidate's validated `--version` output. The
top-level and `environment` copies of both coordinates must equal those
canonical values for every cell, including static `unsupported_by_host` cells;
matching arbitrary duplicated strings are not coordinate-exact.

The `assertions` array is bytewise sorted by `assertion_id` and contains exactly
the set selected by disposition and feature:

| Disposition or feature | Exact assertion IDs |
|---|---|
| `unsupported_by_host` | `static_unsupported_by_host` |
| `native_user_action` | `actual_host_session`, `authority_receipt_observed`, `native_user_selector_observed`, `operator_choice_confirmed`, `same_connection_resume` |
| `local_web_user_channel` | `actual_host_session`, `browser_submission_observed`, `host_owned_surface_observed`, `model_visible_payload_absence_observed`, `same_connection_resume`, `strong_evidence_close_chain`, `trusted_capability_current` |
| `verified_tool_producer` | `actual_host_tool_event`, `capture_receipt_bound`, `criterion_coverage_projected`, `exact_session_connection_actor_scope_baseline`, `intent_precedes_source`, `negative_rejections_zero_effect`, `strong_producer_chain` |
| `registered_connection_observation` | `actual_host_connection_event`, `capture_receipt_bound`, `criterion_coverage_projected`, `exact_session_connection_actor_scope_baseline`, `intent_precedes_source`, `negative_rejections_zero_effect`, `strong_producer_chain` |
| `record_final_output` | `actual_host_session`, `authenticated_exact_replay_observed`, `authority_display_observed` |
| `detective_final_output` | `actual_host_session`, `authenticated_exact_replay_observed`, `authority_display_observed`, `block_finalization_observed` |

For `verified_tool_producer`, the live-harness source-observation barrier is
durable persistence of the complete `post_tool` event matched to the immutable
intent and post-intent exact `pre_tool` event whose decision was not `deny`. A
Stop event or decision, close-readiness result, model response completion,
host-turn completion, or host-process exit is not part of that barrier. For
`registered_connection_observation` selected by Stop, the source-observation
barrier is durable persistence of the exact post-intent Stop event; its
completion-claim flag and always-allow termination result are captured source
outcomes, not process-exit prerequisites.
Because guard-event rows are append-only, an observed lone `post_tool` or an
observed incomplete `post_tool` is a terminal source-shape failure: neither can
become the exact pair through a later append. Only no candidate or one
non-denied exact `pre_tool` remains a pending pair state. The bounded deadline
performs one final persisted inspection so a just-committed exact pair is not
relabelled as a timeout.
The maintained harness performs mismatch rejection and exact receipt capture
immediately after the applicable barrier and before intent expiry, without
waiting for the host turn or process to exit. Receipt capture is the separate
source-fulfillment transaction. Lifecycle signals neither extend the intent
window nor substitute for a missing source event.

For these two producer cells, `negative_rejections_zero_effect` has a narrow
v3 meaning. It proves one actual-host mismatch probe and equality of the
capture-owned Core tables, selected immutable intent and source rows, Project
clock and version, and bounded whole artifact-store file set sampled around
that rejected command: reversed pre/post references for the tool producer, or
the retained pre-intent, wrong-session Stop reference for the connection
producer. Concurrent host-lifecycle session and watch rows are outside that
snapshot and are revalidated separately after the turn. It does not prove that
missing or mismatched invocation identity,
actor, scope revision, baseline, connection, session, or freshness was exercised
as an independent actual-host case. Fixture-only tables can protect the shared
predicate but cannot be reported as those actual-host observations. The v3
sidecar has no per-case provenance field, so a cell or gate verdict must not be
cited as evidence for any broader negative matrix.

For `native_user_action`, `authority_receipt_observed` means that the live cell
observed the complete fresh receipt stored by the exact authenticated,
same-connection, Task-bound Stop event. The receipt must bind the selected
Project, Task, current `state_version`, and exact consuming Run, and the stored
Stop decision, completion-claim flag, reasons, close state, and complete blocker
set must be internally consistent with it. Stop termination is always `allow`.
The maintained clean fixture admits exactly two completion outcomes: ready with
`completion_claim_allowed=true` under full `mcp_start` coverage and no warning
or blocker, and `completion_claim_allowed=false` with only
`close_readiness_blocked` plus the exact `session_watch_unavailable` blocker
under active partial `first_project_selection` or `method_boundary` coverage.
Neither outcome asks the host to retry Stop. Any other outcome
fails this cell even when its receipt is truthful. The fresh LocalUser status
must separately be ready and blocker-free as a clean-fixture sanity
precondition; it is not the receipt that satisfies this assertion. A LocalUser
CLI status receipt and an Agent Connection Stop receipt share authority
coordinates but are invocation-context projections; their close state and
blocker set need not be equal, so whole-receipt equality across those contexts
is not an assertion.

The legacy-named `block_finalization_observed` assertion means the host showed
completion-claim suppression separately while allowing termination; it never
means Stop denial or retry. That native-cell observation does not satisfy `authority_display_observed`,
`authenticated_exact_replay_observed`, or `block_finalization_observed` and
does not promote either final-output feature. Those assertions remain owned
only by their corresponding final-output cells.

An honestly unrun implemented cell is represented by a present cell with
`run_state=ignored`, required failing assertions, and a bounded evidence
artifact. An unavailable host uses a null host-availability group and therefore
a null client group. An available host for which no successful managed
initialize identity was observed keeps the non-null host-availability group and
uses a null client group. Either cell derives `implemented_unverified`;
`requested_verified=true`
therefore fails the gate, while explicit `requested_verified=false` permits only
`pass_with_downgrades`. A static unsupported cell uses
`run_state=not_applicable`, null evidence, and `requested_verified=false`; its
client group may remain null whether its host-availability group is null or
non-null.
Claim and downgrade keys use the literal `unavailable` for a null version
segment. An absent or malformed cell or evidence file is not this honest
downgrade representation: it is a structural command error and produces no
manifest. A completed implemented cell passes only when every required
assertion passes and its evidence artifact exists, is within the size bound,
and matches its recorded digest.

An implemented cell with a null client group includes
`client_identity_missing` in its derived finding codes. A non-null client group
includes `client_identity_mismatch` when its duplicate copies differ, its
`client_version` differs from `host_version`, or its reviewed Codex pair differs
from `codex-mcp-client`/`0.144.4`. Either finding forces
`implemented_unverified`; it is never repaired from another cell or an inferred
value. A duplicate-copy mismatch also fails the
`all_cell_environment_coordinates_exact` invariant. A static unsupported cell
may keep a null client group without either
finding and still derives `unsupported_by_host`. Divergent non-null identities
for one host fail the `single_host_client_identity_per_host` invariant.

For an implemented cell, `run_state=completed` means that the selected attempt
reached terminal classification and the recorder produced immutable terminal
bytes; it does not mean that the feature passed. When an installed host was
bound but a classifiable feature, source-observation, capture, or producer-chain
attempt failed before every required assertion was determined, and all
publication-integrity revalidation still succeeded, the producer records
bounded evidence, marks every unproven required assertion `passed=false` with
bounded finding codes, and claims no more than `implemented_unverified`.
Child-reaping, after-baseline, retained-identity, candidate-integrity, and
publication failures remain structural failures with no final cell. `ignored`
is reserved for an implemented host path that was not run. `running` is never
emitted as a terminal cell. A strict committed `completed` cell with one or
more failed required assertions is the canonical representation of a failed
selected attempt and derives `implemented_unverified`.

## Canonical Evaluation And Freshness

The canonical evaluator receives one candidate, exactly twelve raw cells, and
one `evaluated_at`. It validates structure and recomputes all reachable file
digests before deriving each status. It never accepts `claimed_status` as an
input to the derived status.

A live cell is fresh only when:

```text
started_at <= recorded_at <= evaluated_at < started_at + 24h
```

The candidate must also precede the cell:
`candidate.recorded_at <= cell.started_at`. A cell that starts before its
candidate descriptor was completed is not exact live evidence and is
downgraded.

The interval is half-open. Equality at the 24-hour boundary is stale. A
future, reversed, malformed, or more precise non-canonical timestamp is
invalid. Evaluation of cells from different host versions as one host result
is invalid even when all other coordinates match.

Derivation is deterministic:

1. A statically reviewed `unsupported_by_host` disposition derives
   `unsupported_by_host`; live absence or failure cannot promote or relabel it.
2. An `implemented` cell derives `verified` only when it is present,
   `completed`, fresh, coordinate-exact, client-identity-exact, digest-exact,
   and all assertions pass.
3. A present `ignored` or `running`, stale, failed, or mismatched implemented
   cell derives `implemented_unverified`. Missing or malformed structural input
   prevents manifest creation instead of being synthesized into a status.
4. Configuration and current runtime prerequisites remain orthogonal. They may
   produce `temporarily_unavailable` only through the existing runtime
   evaluator; the release gate does not manufacture that status.
5. A difference between `claimed_status` and the derived status is a finding
   and the derived status wins.

The gate verdict is `fail` when any cell with `requested_verified=true` does
not derive `verified`, or when candidate/manifest invariants fail. If every
requested verified claim is satisfied but at least one implemented feature is
unverified or explicitly excluded with `requested_verified=false`, the verdict
is explicitly `pass_with_downgrades`. An exclusion remains a downgrade even if
its evidence independently derives `verified`. Only an invariant-clean matrix
with every implemented cell verified and none explicitly excluded is `pass`.

## Release Manifest

`volicord-host-release-manifest-v3` contains exactly:

- `schema`, with exact value `volicord-host-release-manifest-v3`;
- `candidate`, the complete validated candidate object;
- `evaluated_at`;
- `cells`, exactly twelve objects containing each raw cell, `derived_status`,
  and stable `finding_codes`;
- `requested_verified_claims`, the sorted exact host/version/feature keys for
  which `requested_verified=true`;
- `downgrades`, the sorted implemented cells either not derived as `verified`
  or explicitly excluded with `requested_verified=false`;
- `invariant_findings`, a sorted bounded list;
- `verdict`, derived as `pass`, `pass_with_downgrades`, or `fail`.

The gate creates the manifest only after evaluating all cells. A caller cannot
override its status, findings, downgrades, or verdict. JSON key ordering is not
semantic; array ordering named as sorted above is semantic and uses bytewise
ascending UTF-8 order.

## Independent Audit

The audit runs in a new process that did not create the candidate, cell files,
or manifest. It independently strict-reads exactly twelve original `.json`
files from the supplied cell directory instead of trusting the raw cells
embedded in the manifest. It opens immutable inputs from their external paths
and recomputes the manifest SHA-256, candidate SHA-256, source archive SHA-256,
cell-input and cell-evidence SHA-256 values, all structural invariants, every
derived status, and the gate verdict. It must not call a mode that merely reads
the manifest's claimed status or verdict.

`volicord-host-release-audit-v3` contains exactly:

- `schema`, with exact value `volicord-host-release-audit-v3`;
- `manifest_path`, `manifest_sha256`, `cell_directory`,
  `cell_inputs_sha256`, `candidate_path`, and `candidate_sha256`;
- `started_at` and `evaluated_at` for the separate audit process;
- `invariant_results`, with stable invariant IDs and pass/fail values;
- `recalculated_cells`, with required-nullable `host_version`, `client_name`,
  and `client_version`, and all twelve derived statuses and finding codes;
- `findings`, a sorted bounded list of mismatches or invalid inputs;
- `exclusions`, a sorted bounded list of checks intentionally not performed,
  each with a non-empty reason;
- `recalculated_verdict`, and `audit_verdict` as `pass` or `fail`.

`cell_directory` is the exact external absolute input-directory string.
`cell_inputs_sha256` is SHA-256 of this preimage: the ASCII domain
`volicord-host-release-cell-inputs-v3` followed by NUL, then, for each of the
twelve cells ordered by bytewise ascending exact UTF-8 absolute path, the path
byte length as unsigned 64-bit big-endian, the exact path bytes, and the raw
32-byte SHA-256 of the cell file's exact bytes. The audit requires the reopened
cell inputs to equal the manifest's raw cells; a coherent rewrite of only the
manifest fails `cell_inputs_match_manifest`. The final pathname digest recorded
as `candidate_sha256` must also equal the descriptor digest; otherwise
`audit_candidate_binary_digest_exact` fails and the audit verdict is `fail`.

`audit_verdict=pass` requires no findings, no exclusions affecting a requested
verified claim or invariant, exact agreement with the manifest, and a
recalculated manifest verdict other than `fail`. An audit destination is also
create-new and external. The manifest and audit are release evidence, not
Volicord runtime trust inputs, Core evidence, User Channel authority, host
attestation, or permission to publish.

## Managed-Host Session Binding

Codex and Claude Code native session identifiers are accepted only from the
managed adapter path. The native value must be valid UTF-8, 1 through 256 bytes,
and match `[A-Za-z0-9._:-]+`; whitespace, controls, empty values, and every
other byte are rejected.

For the reviewed Codex coordinate, the installed-host probe envelope is
exactly `codex-cli 0.144.4`, while MCP initialize must report exact
`clientInfo.name=codex-mcp-client` and `clientInfo.version=0.144.4`. A launch
with managed Codex provenance starts session-unbound. Only an otherwise valid
known-tool call may supply the binding, through exact request metadata:

- `_meta.threadId` is a valid native identifier;
- `_meta["x-codex-turn-metadata"]` is an object whose `session_id`,
  `thread_id`, and `turn_id` are valid native identifiers; and
- `_meta.threadId` equals the nested `thread_id`.

The nested `session_id` is the native session value used by the mapping below.
The concrete `thread_id` may differ from `session_id`, including for a
subagent, but its flat and nested copies must agree. Volicord reduces that
thread value to a separate domain-separated process-local digest. The first
valid call binds the managed stdio process exactly once to both the mapped root
session and that thread digest. Every later call must carry valid metadata that
matches both; a later turn may use a different valid `turn_id`. Missing,
malformed, or mismatched metadata is rejected without rebinding and before
tool dispatch. Ambient or configured `CODEX_THREAD_ID`, timing, arrival order,
and a nearest or most-recent session are not binding inputs.

For a validated value, Volicord calculates:

```text
digest = SHA-256(
  b"volicord-managed-host-session-v1\0" ||
  host_kind_utf8 || b"\0" || connection_internal_id_utf8 || b"\0" ||
  native_session_id_utf8
)
managed_host_session_id = "mhs_" || lowercase_hex(digest)
```

The same mapped `managed_host_session_id` is used for managed MCP observations
and host-hook observations. The `mhs_` namespace is reserved for this managed
mapping. Its registered-connection and host-kind coordinates are immutable;
generic or manual paths cannot preseed or reuse it. An invalid managed marker
fails before durable diagnostics or protocol state is created. While a Codex
launch is session-unbound, successful startup, initialize, and tools-list facts
may be retained only as bounded process-local state. They create no durable
managed session, lifecycle, diagnostic, tool, capture, token, or watch effect.
The first valid binding materializes the applicable retained lifecycle facts
once in canonical order before recording the accepted call. Session-watch
coverage starts at binding and remains explicitly partial; deferred lifecycle
facts do not backdate repository observation. A rejected binding attempt
creates none of those effects, and a later valid call may retry.

The raw native identifier and raw native event, tool-call, capture, turn, or
invocation identifiers exist only long enough to validate, hash, or replace
them with domain-separated opaque values. They are never persisted, logged,
rendered in diagnostics, attached to evidence, or placed in release artifacts.
A missing native identifier, invalid value, different mapping coordinate, or
mismatch between managed MCP and hook observations cannot create Strong
Evidence and must remain an explicit missing/mismatch finding. Implementations
must not silently mint a replacement session ID or correlate across host kinds
or registered connections. Per-call binding is enforced through the existing
feature assertion sets; it does not add a release assertion identifier.

## Command Routes

The implementation package is `tests/release-validation` with Cargo package
name `volicord-release-validation-tests`. Its exact validation routes are:

```sh
cargo test -p volicord-release-validation-tests
cargo run --locked -p volicord-release-validation-tests --bin host-release-candidate -- --candidate-id CANDIDATE_ID --candidate-path CANDIDATE_BINARY --candidate-out CANDIDATE.json
cargo run --locked -p volicord-release-validation-tests --bin host-release-gate -- --candidate CANDIDATE.json --cell-dir CELL_DIR --manifest-out MANIFEST.json
cargo run --locked -p volicord-release-validation-tests --bin host-release-audit -- --candidate CANDIDATE.json --cell-dir CELL_DIR --manifest MANIFEST.json --audit-out AUDIT.json
```

Every artifact and directory argument is an external absolute path subject to
this contract. The candidate command runs after the final executable is staged
and before any cell starts. The audit command is invoked as a separate process
after the gate process exits. Administrative CLI fallback and diagnostics may
summarize these artifacts, but they are auxiliary and cannot replace the
candidate, gate, or audit command.

## Related Owners

- [System Requirements](system-requirements.md) owns environment applicability.
- [Agent Connection](agent-connection.md) owns runtime support and fallback.
- [MCP Transport](mcp-transport.md) owns managed stdio transport behavior.
- [API State Schemas](api/schema-state.md) owns the `AuthorityReceipt` and
  close-readiness projection shapes.
- [`close_task`](api/method-close-task.md) owns close-readiness blocker codes,
  categories, and resolution meaning.
- [Storage Records](storage-records.md) owns the bounded managed initialize
  identity placement in `session_watch_baselines.metadata_json`.
- [Security](security.md) owns trust and non-authority boundaries.
- [Administrative CLI](admin-cli.md#guard-hook-commands) owns hidden Stop-event
  behavior and auxiliary operator projections.
- [Validation](../maintain/validation.md) owns maintainer execution and reports.
- [Host release evidence gate decision](../architecture-guide/decisions/host-release-evidence-gate.md)
  records why this contract is external and independently recalculated.
- [Managed-host session/thread binding and per-call turn validation decision](../architecture-guide/decisions/managed-host-session-turn-binding.md)
  records why Codex binding is call-scoped and fail-closed.
