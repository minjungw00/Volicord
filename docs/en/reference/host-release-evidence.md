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
| One live-host matrix result | `volicord-host-release-cell-v1` |
| Canonical release-gate result | `volicord-host-release-manifest-v1` |
| Separate-process recalculation | `volicord-host-release-audit-v1` |
| Source archive digest algorithm | `git_archive_tar_sha256_v1` |

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
differently named, malformed, or cross-host-availability cell is a structural
command error and produces no manifest. All six cells for one host kind share
one availability coordinate: either all name the same exact `host_version` and
non-null executable digest, or all use explicit `null` for the host version and
executable digest. Results from different versions or availability coordinates
must never be aggregated into one host result. A new host version requires a
new complete twelve-cell manifest.

`volicord-host-release-cell-v1` contains exactly these required members:

| Member | Contract |
|---|---|
| `schema` | Exact value `volicord-host-release-cell-v1`. |
| `candidate_id`, `binary_sha256`, `source_revision`, `target_triple`, `release_profile` | Exact copies of the candidate coordinates. |
| `host_kind`, `host_version` | One fixed host kind and either the exact installed host version observed by the cell or explicit `null` when that host is unavailable. The member is always required. |
| `adapter_profile`, `adapter_version` | Exact managed adapter coordinates. |
| `feature` | One of the six fixed feature identifiers. |
| `implementation_disposition` | `implemented` or `unsupported_by_host`; this is owner-reviewed static input, not a live result. |
| `requested_verified` | Boolean release claim requested for this exact host availability/feature. An implemented cell defaults to `true`; explicit `false` is a release exclusion and downgrade. Static unsupported cells require `false`. |
| `claimed_status` | Producer claim using `HostFeatureSupportStatus`; retained only for mismatch reporting and never trusted. |
| `run_state` | `completed`, `running`, `ignored`, or `not_applicable`; only static `unsupported_by_host` may use `not_applicable`. |
| `started_at`, `recorded_at` | Cell start and immutable result-recording times. |
| `environment` | Exact `runner_os`, `runner_os_version`, `runner_arch`, required-nullable `host_executable_sha256` and `host_version`, and all host/adapter coordinates used by the run. The three top-level/environment host identity fields are either all non-null or all null. |
| `assertions` | Non-empty bounded array of stable assertion IDs with `passed` booleans and optional bounded finding codes. |
| `evidence_artifact_path`, `evidence_artifact_sha256` | External create-new bounded evidence file and SHA-256; both remain required for an implemented cell, including an unavailable ignored cell, and both are `null` only for static `unsupported_by_host`. |

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

An honestly unrun implemented cell is represented by a present cell with null
host identity, `run_state=ignored`, required failing assertions, and a bounded
evidence artifact. It derives `implemented_unverified`; `requested_verified=true`
therefore fails the gate, while explicit `requested_verified=false` permits only
`pass_with_downgrades`. A null static unsupported cell uses
`run_state=not_applicable`, null evidence, and `requested_verified=false`.
Claim and downgrade keys use the literal `unavailable` for a null version
segment. An absent or malformed cell or evidence file is not this honest
downgrade representation: it is a structural command error and produces no
manifest. A completed implemented cell passes only when every required
assertion passes and its evidence artifact exists, is within the size bound,
and matches its recorded digest.

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
   `completed`, fresh, coordinate-exact, digest-exact, and all assertions pass.
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

`volicord-host-release-manifest-v1` contains exactly:

- `schema`, with exact value `volicord-host-release-manifest-v1`;
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

`volicord-host-release-audit-v1` contains exactly:

- `schema`, with exact value `volicord-host-release-audit-v1`;
- `manifest_path`, `manifest_sha256`, `cell_directory`,
  `cell_inputs_sha256`, `candidate_path`, and `candidate_sha256`;
- `started_at` and `evaluated_at` for the separate audit process;
- `invariant_results`, with stable invariant IDs and pass/fail values;
- `recalculated_cells`, with required-nullable `host_version` and all twelve
  derived statuses and finding codes;
- `findings`, a sorted bounded list of mismatches or invalid inputs;
- `exclusions`, a sorted bounded list of checks intentionally not performed,
  each with a non-empty reason;
- `recalculated_verdict`, and `audit_verdict` as `pass` or `fail`.

`cell_directory` is the exact external absolute input-directory string.
`cell_inputs_sha256` is SHA-256 of this preimage: the ASCII domain
`volicord-host-release-cell-inputs-v1` followed by NUL, then, for each of the
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
fails before durable diagnostics or protocol state is created.

The raw native identifier and raw native event, tool-call, capture, turn, or
invocation identifiers exist only long enough to validate, hash, or replace
them with domain-separated opaque values. They are never persisted, logged,
rendered in diagnostics, attached to evidence, or placed in release artifacts.
A missing native identifier, invalid value, different mapping coordinate, or
mismatch between managed MCP and hook observations cannot create Strong
Evidence and must remain an explicit missing/mismatch finding. Implementations
must not silently mint a replacement session ID or correlate across host kinds
or registered connections.

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
- [Security](security.md) owns trust and non-authority boundaries.
- [Administrative CLI](admin-cli.md) owns auxiliary operator projections.
- [Validation](../maintain/validation.md) owns maintainer execution and reports.
- [Host release evidence gate decision](../architecture-guide/decisions/host-release-evidence-gate.md)
  records why this contract is external and independently recalculated.
