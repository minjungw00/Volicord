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
| `build_environment` | Exact `runner_os`, `runner_os_version`, `runner_arch`, `git_version`, `rustc_version`, and `cargo_version` strings. |
| `recorded_at` | Time at which the descriptor and all candidate digests were completed. |

The source checkout must be clean before building and remain at
`source_revision` through candidate creation. From that checkout, the producer
runs `git archive --format=tar <source_revision>` with no path prefix or extra
attributes and computes SHA-256 over the command's raw tar stdout. That byte
digest is `source_archive_sha256`; hashing a compressed archive, work tree,
directory listing, or Git bundle is not equivalent.

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

The only admissible non-null client identity is the exact pair observed from
the successful managed MCP `initialize` used by that cell. It must not be
inferred from `host_kind`, the host executable name, version-probe output,
environment or configuration, protocol version, known constants, later tool
metadata, or another cell. A recorder may read the bounded top-level
`client_name` and `client_version` retained in that managed session's
`session_watch_baselines.metadata_json`; it must not retain or consume the raw
initialize message or raw protocol, session, thread, turn, or tool-call payload
as release evidence.

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
the existing expected digest. Before creating either the cell or its evidence
artifact, the recorder reopens every retained key and requires the row to
exist with the exact managed session, connection, project, host, canonical
baseline ID, and expected metadata digest. Deletion, replacement at the same
key, or any metadata change not covered by such a later captured turn stops
recording and leaves both destinations absent. It is not converted into a null
client group or an `implemented_unverified` cell.

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
cargo run --locked -p volicord-release-validation-tests --bin host-release-gate -- --candidate CANDIDATE.json --cell-dir CELL_DIR --manifest-out MANIFEST.json
cargo run --locked -p volicord-release-validation-tests --bin host-release-audit -- --candidate CANDIDATE.json --cell-dir CELL_DIR --manifest MANIFEST.json --audit-out AUDIT.json
```

Every artifact and directory argument is an external absolute path subject to
this contract. The audit command is invoked as a separate process after the
gate process exits. Administrative CLI fallback and diagnostics may summarize
these artifacts, but they are auxiliary and cannot replace either command.

## Related Owners

- [System Requirements](system-requirements.md) owns environment applicability.
- [Agent Connection](agent-connection.md) owns runtime support and fallback.
- [MCP Transport](mcp-transport.md) owns managed stdio transport behavior.
- [Storage Records](storage-records.md) owns the bounded managed initialize
  identity placement in `session_watch_baselines.metadata_json`.
- [Security](security.md) owns trust and non-authority boundaries.
- [Administrative CLI](admin-cli.md) owns auxiliary operator projections.
- [Validation](../maintain/validation.md) owns maintainer execution and reports.
- [Host release evidence gate decision](../architecture-guide/decisions/host-release-evidence-gate.md)
  records why this contract is external and independently recalculated.
- [Managed-host session/thread binding and per-call turn validation decision](../architecture-guide/decisions/managed-host-session-turn-binding.md)
  records why Codex binding is call-scoped and fail-closed.
