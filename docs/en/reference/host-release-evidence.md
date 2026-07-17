# Host Release Evidence

This document owns first-release support evidence for exact finalized Codex
artifacts. It defines `CodexReleaseCell`, the checked-in support manifest,
independent platform cells, required release-validation scenarios, and honest
cell execution status.

It does not define managed host configuration, receipt semantics, runtime trust,
or operating-system prerequisites. Those contracts remain with their focused
owners. Release-validation fixtures and results are test and release evidence;
they are never production runtime trust inputs.

<a id="surface-stability"></a>
## Surface Stability

The labels below use the
[surface-stability vocabulary](../maintain/documentation-policy.md#surface-stability-labels).
The `CodexReleaseCell` shape, exact artifact-and-capability matching,
`unsupported_host_artifact`, four independent platform cells, and cell status
meanings are `stable`. Test runner modules and fixture layout below those
boundaries are `internal`.

<a id="codex-release-cell"></a>

## `CodexReleaseCell`

The first release records a strict cell with this exact closed shape:

```yaml
CodexReleaseCell:
  artifact_digest: string
  platform: PlatformEnvironment
  observed_capabilities: CodexCapability[]
  integration_profile: record
  validation_evidence: CodexReleaseValidationEvidence
```

`artifact_digest` is the raw 64-lowercase-hex SHA-256 of the exact finalized
Codex executable bytes exercised by this cell. `platform` uses the closed
`PlatformEnvironment` set, and `observed_capabilities` uses the closed
`CodexCapability` set, both owned by
[Agent Connection](agent-connection.md#platform-environment). A first-release
cell carries exactly `FirstReleaseCodexCapabilities` in its required canonical
order. `integration_profile` is exactly `record`.

Every member is required. Unknown members, duplicate JSON keys, malformed
digests, or a value outside an owned closed set invalidate the cell.
`validation_evidence.artifact_digest`, `platform`,
`observed_capabilities`, and `integration_profile` must exactly equal the
owning cell coordinates. Evidence cannot widen or repair them.

<a id="codex-release-validation-evidence"></a>

## `CodexReleaseValidationEvidence`

The nested evidence and runner coordinates have these exact closed shapes and
field order:

```yaml
CodexReleaseValidationEvidence:
  status: passed | failed | unavailable
  artifact_digest: string
  platform: PlatformEnvironment
  observed_capabilities: CodexCapability[]
  integration_profile: record
  volicord_artifact_digest: string
  runner: CodexReleaseRunnerCoordinate
  scenario_results: CodexReleaseScenarioResult[]
  evidence_digest: string
  observed_at: string

CodexReleaseRunnerCoordinate:
  runner_id: string
  target_triple: string
  architecture: x86_64 | aarch64
  os_release: string
  environment_image: string

CodexReleaseScenarioResult:
  scenario_id: CodexReleaseScenarioId
  status: passed | failed | unavailable | not_run
  reason: string | null
  evidence_digest: string | null
  observed_at: string | null
```

Every member is required, including nullable members; unknown members and
duplicate JSON keys are invalid. `volicord_artifact_digest` and every non-null
scenario `evidence_digest` are raw 64-lowercase-hex SHA-256 values.
`evidence_digest` at the evidence-object level is also raw 64-lowercase-hex.
All non-null timestamps are canonical RFC 3339 UTC. Runner strings are
nonempty, control-free UTF-8: `runner_id` and `target_triple` are at most 256
bytes, while `os_release` and `environment_image` are at most 512 bytes. The
runner fields identify the exact execution environment; another cell's runner
coordinates cannot be copied or inferred. The WSL2 cell's
`environment_image` names its pinned Ubuntu LTS distribution image.

`reason` is null for `passed` and otherwise a nonempty machine-readable code
matching `[a-z][a-z0-9_]{0,127}`. A `passed` or `failed` scenario has a
non-null digest and timestamp. An `unavailable` scenario has a non-null reason
and timestamp and may have a null digest only when no bounded evidence artifact
could be produced. A `not_run` scenario has a non-null reason and null digest
and timestamp.

The evidence `status` is `passed` only when every required scenario is
`passed`. It is `failed` when at least one scenario is `failed`. It is
`unavailable` only when no scenario failed, at least one scenario is
`unavailable`, and a qualifying attempt could not complete. Later scenarios
that could not run remain explicit `not_run` results. No top-level `not_run`
evidence object exists: a platform with no qualifying attempt has no manifest
cell.

<a id="codex-release-scenario-catalog"></a>

### Closed Scenario Catalog

Every non-WSL2 cell contains each base scenario exactly once in this order:

```text
fresh_install
runtime_home_creation
personal_managed_binding
shared_managed_binding
receipt_create_and_validate
configuration_drift_detection
repair_after_drift
safe_uninstall
symlink_and_canonical_path
codex_restart
project_move
record_write_workflow
suppression_unavailable
unsupported_host
unsupported_host_artifact
```

The WSL2 cell appends each of these scenarios exactly once in this order:

```text
wsl_shutdown_restart
wsl2_ext4_project
wsl2_drvfs_rejection
wsl2_cross_topology_rejection
wsl1_rejection
wsl2_native_windows_receipt_reuse_rejection
```

Unknown, duplicate, missing, or out-of-order scenario IDs invalidate the
evidence. The personal and shared scenarios separately exercise the two
first-release connection intents.

<a id="release-evidence-digest"></a>

### Evidence Digest

Evidence uses the exact `u32be`, `u64be`, `blob`, `string`, `list`, and
`record` primitives from
[Agent Connection](agent-connection.md#canonical-binding-encoding). Nullable
values add this primitive:

```text
nullable(null)  = 0x00
nullable(value) = 0x01 || blob(value_encoding)
```

`canonical_evidence_without_digest_bytes` is the `record` encoding of
`CodexReleaseValidationEvidence` in the declared order with the
`evidence_digest` field omitted. Nested records use their declared order, and
`scenario_results` preserves the catalog order.

```text
evidence_digest = lowercase_hex(sha256(
  "volicord.codex-release-validation-evidence\0"
  || canonical_evidence_without_digest_bytes
))
```

The runner computes this digest after producing all scenario results. Review
recomputes it; JSON serializer order, omitted nulls, default values, and
hand-edited evidence are invalid.

## Exact finalized artifact evidence

The cell hashes the executable after every publisher-controlled change to its
bytes, including signing, stripping, packaging extraction, or other
post-processing. Validation runs the exact bytes named by `artifact_digest`.
Before and after the scenario suite, the runner reopens the executable and
requires the same byte digest. A command name, path, version range, package
label, build identifier, or separately rebuilt executable cannot substitute for
the finalized bytes.

A support claim is valid only for the exact combination of:

- `artifact_digest`
- `platform`
- `observed_capabilities=FirstReleaseCodexCapabilities`
- `integration_profile=record`
- `validation_evidence.status=passed`

Support never propagates from one artifact to another, from one capability to
another, or from one platform to another. A capability observed for one cell
does not become a general Codex capability claim.

An artifact is registered for support only when one `passed` cell exactly
matches the current `ProcessBinding.executable_digest`,
`PlatformEnvironment`, `integration_profile=record`, and complete canonical
`CodexCapability` set. The receipt's `executable_digest`, platform, profile,
`required_capabilities`, and `verified_capabilities` must equal those same
cell coordinates. An unknown digest, any platform/profile/capability mismatch,
or a digest present only in a non-passing cell returns machine-readable reason
`unsupported_host_artifact`. The implementation must not infer support from a
command name, broad version range, neighboring artifact, fixture, subset, or
superset capability match.

## Canonical checked-in manifest

The single support source is:

```text
tests/release-validation/contracts/codex-release-manifest.json
```

It is a strict UTF-8 JSON array containing zero through four actually
runner-generated and reviewed `CodexReleaseCell` objects. At most one cell may
name each platform, and present cells appear in `linux`, `macos`,
`native_windows`, `wsl2` order. A newly introduced or not-yet-executed source
may therefore be `[]`. An absent platform has the derived release status
`not_run`; the source must not fabricate a placeholder cell, digest, runner
coordinate, or evidence object.

Only an actual qualifying attempt can produce a `failed` or `unavailable`
cell, and review must accept its generated evidence before it enters the
source. Only `passed` cells participate in production support lookup. No other
source file, fixture, generated constant, documentation table, or runtime
database may carry a second support list. Runtime code may consume a build
projection derived from this source, but the projection must be reproducibly
checked against it and must not add an artifact, platform, status, or
capability.

Update one platform entry as one reviewed operation:

1. Finalize that platform's distributable Codex artifact and calculate
   `artifact_digest` from the final bytes.
2. Execute that platform's complete release-validation cell against those exact
   bytes. The runner emits the cell and bounded evidence for every required
   scenario.
3. Recheck the artifact digest and the exact platform, profile, capability,
   runner, scenario, and evidence-digest bindings.
4. Review the generated cell and replace the existing entry for that platform,
   if any, while preserving canonical platform order. Do not hand-author an
   unattempted cell, copy another platform's result, or retain a historical
   compatibility entry.
5. Re-evaluate the manifest. A four-platform release is eligible only when it
   contains exactly one current `passed` cell for each of the four platforms,
   all four cells carry `FirstReleaseCodexCapabilities`, and all refer to the
   current release candidate artifacts.

Changing an artifact byte, platform coordinate, capability set, profile, or
validation evidence requires a new run of that exact cell. Editing the manifest
cannot promote evidence that the runner did not produce.

## Explicit test-only descriptor

Unit and integration tests that do not execute a finalized Codex artifact use
an explicit descriptor separated from `CodexReleaseCell`:

```yaml
TestOnlyCodexDescriptor:
  test_only: true
  fixture_id: string
  artifact_digest: string
  platform: linux | macos | native_windows | wsl2
  observed_capabilities: CodexCapability[]
```

The marker must be the exact boolean `true`. This descriptor may exercise
parsing, routing, negative cases, and adapter projections in test builds. It is
rejected by the checked-in manifest loader and every production support lookup,
cannot produce `validation_evidence.status=passed`, and cannot register a host
artifact or capability. A test fixture, test-only injection, copied manifest
entry, or repository test pass is not runtime trust and is not finalized
artifact evidence.

## Independent platform cells

A release-eligible matrix contains four independent passed cells:

| Platform | Required environment boundary |
|---|---|
| `linux` | Native Linux runner and Linux artifact. Its result says nothing about WSL2. |
| `macos` | Native macOS runner and macOS artifact. Linux or Unix-like behavior cannot stand in for it. |
| `native_windows` | Native Windows runner and native Windows artifact. WSL paths, processes, bindings, and receipts are ineligible. |
| `wsl2` | The pinned Ubuntu LTS WSL2 environment described below, with a WSL2 artifact and every component inside that environment. |

Each cell runs, records, and reports its own artifact, environment, capability,
and evidence coordinates. Passing `linux` does not pass `wsl2`; passing
`native_windows` does not pass `wsl2`; and a pass on any one artifact does not
support the artifact used by another cell. A missing or non-passing cell blocks
the four-platform release claim rather than being inferred from another cell.

### WSL2 cell boundary

The WSL2 cell uses one Ubuntu LTS image pinned by the manifest evidence. Codex,
Volicord, the Product Repository, and the Volicord Runtime Home all run inside
the same WSL2 distribution. The Product Repository and Runtime Home use the
distribution's Linux ext4 filesystem.

The WSL2 cell rejects:

- WSL1
- a Windows-hosted Codex process paired with WSL2 Volicord or project state
- a WSL2 Codex process paired with a native Windows Volicord process, project,
  or Runtime Home
- Product Repositories or Runtime Homes under `/mnt/*` or another DrvFS mount
- reuse of native Windows bindings, process identity, or verification receipts
- a distribution or Ubuntu LTS image not named by the current cell evidence

Windows paths, PIDs, environment values, and receipts are never converted or
treated as equivalent to WSL2 values. A WSL shutdown or restart invalidates any
process-bound evidence whose owner requires a live process and makes an expired
or mismatched receipt stale; the cell must observe that rejection before it can
record a fresh receipt.

## Required release-validation scenarios

Every platform cell exercises the same domain scenario set through its own
platform and Codex adapter boundary:

- fresh installation
- Volicord Runtime Home creation and validation
- personal managed Codex binding installation
- shared managed Codex binding installation
- verification receipt creation and current validation
- configuration drift detection
- repair after supported drift
- safe uninstall
- symlink and canonical-path handling
- Codex restart and stale-receipt rejection
- Product Repository move and stale binding or receipt rejection
- one complete Record-profile write workflow
- conservative `suppression unavailable` behavior without hiding observed paths
- rejection of an unsupported host
- `unsupported_host_artifact` for an unregistered or platform-mismatched
  artifact

The WSL2 cell additionally exercises WSL shutdown and restart, an ext4 Product
Repository, `/mnt/*` Product Repository rejection, native-Windows/WSL2
cross-topology rejection, WSL1 rejection, and native Windows receipt reuse
rejection.

Shared scenarios construct one canonical domain setup and assert one canonical
domain outcome. Platform modules supply only platform-specific setup,
filesystem, process, and projection assertions. A platform-specific shortcut
must not remove or weaken a shared scenario.

## Cell execution status

`validation_evidence.status` is exactly one of:

| Status | Meaning | Release effect |
|---|---|---|
| `passed` | The exact finalized artifact ran in the exact cell environment, every required scenario passed, evidence is complete, and all bindings remain exact. | Registers only this artifact, platform, profile, and observed capability set for support. |
| `failed` | The cell ran far enough to classify at least one required assertion as failed, or an artifact/evidence integrity check failed. | Does not register support and blocks the four-platform release claim. |
| `unavailable` | A required runner, host, credential, environment, or other execution prerequisite was unavailable, so the cell could not establish a pass or failure for the complete suite. | Does not register support and blocks the four-platform release claim. |

`not_run` is the derived platform status when the manifest has no cell for that
platform; it is not a `validation_evidence.status` value and does not authorize
a placeholder cell. Scenario results may use `not_run` under the cross-field
rules above. `unavailable` and derived `not_run` are never reported,
summarized, or counted as `passed`. A repository unit test, fixture result,
another platform's pass, or an older artifact's evidence cannot change those
meanings.

## Release-validation target layout

The maintained target structure is:

```text
tests/release-validation/
  contracts/
    codex-release-manifest.json
  fixtures/
  scenarios/
  hosts/
    codex/
  platforms/
    linux/
    macos/
    windows/
    wsl2/
```

`contracts/` owns strict manifest parsing and exact support lookup.
`fixtures/` contains only explicit test-only descriptors and bounded test
inputs. `scenarios/` owns shared domain setup and canonical outcomes.
`hosts/codex/` owns actual Codex launch and observation behavior. Each
`platforms/` module owns only its platform environment and adapter-specific
assertions. No single platform module, fixture, or live-host test file owns the
whole release contract.

<a id="executable-release-cell-gate"></a>
## Executable release-cell gate

The repository-native candidate producer and blocking gate are the
`codex-release-cell-gate` binary in the `volicord-release-validation-tests`
package. It has no manifest override. It loads both the build-embedded bytes
and the on-disk bytes at the canonical manifest path and requires them to parse
to the same strict manifest.

```sh
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --status
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --capture-candidate PLATFORM
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --platform linux
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --platform macos
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --platform native_windows
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --platform wsl2
```

`--status` reports the four actual or derived manifest statuses and does not
execute a cell. `--capture-candidate` executes one qualifying attempt and uses
create-new semantics to write an external, strictly parsed, one-cell candidate
array. It exits unsuccessfully after retaining a `failed` or `unavailable`
candidate, and it never edits or promotes the canonical manifest. `--platform`
is the blocking replay gate and succeeds only when that platform already has an
exact checked-in `passed` cell. An absent entry fails as `not_run`; a checked-in
`failed` or `unavailable` entry also fails. Therefore the honest current `[]`
source can produce review candidates but cannot pass publication until all four
candidates have been reviewed and checked in.

The producer and gate perform these checks in order:

1. It rejects a process outside the selected independent runner boundary.
2. It derives the actual runner coordinate. Capture records it; blocking replay
   requires it to exactly equal the cell's `runner` value.
3. It hashes the actual Codex and Volicord executable bytes. Blocking replay
   requires the two digests recorded by the cell.
4. It executes `--version` from both exact paths.
5. It delegates every platform-owned scenario exactly once, in canonical
   order, to the provisioned scenario driver. A passing driver must emit the
   strict semantic evidence document below. The runner validates its canonical
   fixture setup, repository-selected boundary execution, repository-owned
   domain outcome, matching adapter projection, and bounded cleanup before it
   wraps that evidence. It independently recomputes every canonical payload
   digest and verifies the digest links between the four records. The
   deterministic wrapper binds the scenario definition, both artifact digests,
   platform, driver digest, capabilities, and Record profile. Missing,
   additional, renamed, catalog-reordered, opaque, self-selected, or mismatched
   evidence fails.
6. It reopens and rehashes both executable paths after the complete catalog and
   requires their bytes to be unchanged.
7. Capture computes the canonical cell evidence digest and writes only the new
   external candidate path. Blocking replay requires every current scenario to
   pass and each deterministic evidence digest to equal the reviewed checked-in
   result.

The native Linux boundary rejects WSL and container process boundaries. The
macOS and native Windows boundaries require their corresponding native process.
The WSL2 boundary deliberately runs the gate and scenario coordinator on a
native Windows supervisor so that the coordinator can survive
`wsl_shutdown_restart`. The selected product environment is exactly
`Ubuntu-24.04`; the gate verifies a WSL2 kernel, `ID=ubuntu`,
`VERSION_ID=24.04`, and the matching `WSL_DISTRO_NAME`. Codex, Volicord, the
cell work root, the Product Repositories created below it, and `VOLICORD_HOME`
remain inside that one distribution on ext4. The Windows supervisor is test
harness infrastructure, not a substitute native-Windows product component.

### Producer and gate inputs

The release runner provisions these exact environment variables before capture
or blocking replay:

| Variable | Required value |
|---|---|
| `VOLICORD_CODEX_RELEASE_CODEX_PATH` | Canonical symlink-free path of the exact finalized Codex executable. It is a host path for native cells and an absolute Linux ext4 path inside the selected distribution for WSL2. |
| `VOLICORD_CODEX_RELEASE_VOLICORD_PATH` | Canonical symlink-free path of the exact Volicord executable named by `volicord_artifact_digest`, using the same native-or-WSL2 path rule. |
| `VOLICORD_CODEX_RELEASE_SCENARIO_DRIVER` | Canonical host path of the platform-provisioned scenario driver. For WSL2 it is a native Windows coordinator capable of surviving and verifying a distribution shutdown and restart. |
| `VOLICORD_CODEX_RELEASE_EVIDENCE_DIR` | Existing, empty, canonical host directory outside the source checkout, Cargo target directory, maintained docs, Product Repository, and Runtime Home. |
| `VOLICORD_CODEX_RELEASE_WORK_ROOT` | Existing, empty work root outside repository-owned paths. For WSL2 it is an absolute ext4 directory inside the selected distribution. |
| `VOLICORD_HOME` | Absent child of the cell work root, created only by the `runtime_home_creation` scenario. |
| `VOLICORD_CODEX_RELEASE_ENVIRONMENT_IMAGE` | Exact environment-image coordinate recorded in the checked-in cell. |
| `RUNNER_NAME` | Actual runner-service identity; it must equal `runner.runner_id`. |
| `VOLICORD_CODEX_RELEASE_WSL2_DISTRIBUTION` | WSL2 only; exactly `Ubuntu-24.04`. It is rejected for native cells. |
| `VOLICORD_CODEX_RELEASE_CANDIDATE_CELL_PATH` | Capture only; an absent absolute path with an existing canonical parent outside repository-owned, evidence, work, and Runtime Home roots. The producer writes a strict one-cell JSON array with create-new semantics. Blocking replay does not read this variable. |

The gate derives the architecture and target triple from the native process, or
from `uname -m` inside the selected WSL2 distribution. It derives `os_release`
from `/proc/sys/kernel/osrelease` on Linux and WSL2,
`sw_vers -productVersion` on macOS, and `cmd.exe /D /C ver` on native Windows.
Those derived values and the provisioned environment-image coordinate must
exactly equal the checked-in runner coordinate.

For every scenario, the driver receives the exact scenario, platform,
executables, work root, Runtime Home, and two fresh output paths. Its command
shape is:

```text
SCENARIO_DRIVER
  --scenario SCENARIO_ID
  --fixture FIXTURE
  --boundary BOUNDARY
  --projection PROJECTION
  --expected-outcome OUTCOME_CODE
  --platform PLATFORM
  --codex CODEX_PATH
  --volicord VOLICORD_PATH
  --work-root WORK_ROOT
  --runtime-home RUNTIME_HOME
  --evidence-output NEW_DRIVER_EVIDENCE_PATH
  --outcome-output NEW_OUTCOME_JSON_PATH
  [--wsl2-distribution Ubuntu-24.04]
```

The outcome document has exactly `scenario_id`, `status`, `reason`, and
`observed_at`. `status` is `passed`, `failed`, `unavailable`, or `not_run`.
`passed` requires a null reason and a canonical UTC observation time;
`failed` and `unavailable` require a machine-readable reason and time;
`not_run` requires a reason and null time. Passed and failed outcomes require a
bounded driver evidence file, `not_run` forbids one, and unavailable may
include one.

A `passed` evidence file is strict JSON with exactly this shape:

```json
{
  "contract": "volicord.release_scenario_evidence",
  "scenario_id": "fresh_install",
  "platform": "linux",
  "state_setup": {
    "canonical_project_state": {
      "fixture_id": "fresh_install",
      "fixture": "no_installation",
      "platform": "linux"
    },
    "canonical_project_state_digest": "0412001a986fb601aaec49e5ca491f034735eae9d2b79fc3a1f172ac73268725",
    "validated": true
  },
  "boundary_execution": {
    "canonical_invocation": {
      "scenario_id": "fresh_install",
      "platform": "linux",
      "boundary": "cli",
      "canonical_project_state_digest": "0412001a986fb601aaec49e5ca491f034735eae9d2b79fc3a1f172ac73268725"
    },
    "invocation_digest": "e123ca5a50d1b8362a8bc8a9a6366692f3901a09184e014da44eaa1e3a1d9fde",
    "completed": true
  },
  "domain_outcome": {
    "canonical_outcome": {
      "scenario_id": "fresh_install",
      "expectation": "complete_successfully",
      "disposition": "completed",
      "outcome_code": "installation_completed",
      "invocation_digest": "e123ca5a50d1b8362a8bc8a9a6366692f3901a09184e014da44eaa1e3a1d9fde",
      "observed_paths_preserved": null
    },
    "canonical_outcome_digest": "9690aa98449e2944b9477d3ffb6496a918556a15432173cdc48b4a432cee19af",
    "validated": true
  },
  "adapter_projection": {
    "canonical_projection": {
      "scenario_id": "fresh_install",
      "projection": "cli_json",
      "outcome_code": "installation_completed",
      "canonical_outcome_digest": "9690aa98449e2944b9477d3ffb6496a918556a15432173cdc48b4a432cee19af"
    },
    "canonical_projection_digest": "ca3f422db0290cd0bdd328afd8ca794bca9e7f1eee0a509382e1ddff2dfd0a48",
    "validated": true
  },
  "cleanup_complete": true
}
```

The four nested `canonical_*` objects are retained canonical, nonvolatile
payloads. Each adjacent digest is the bare lowercase SHA-256 of that object's
canonical JSON. The invocation embeds the recomputed project-state digest, the
domain outcome embeds the recomputed invocation digest, and the projection
embeds the recomputed domain-outcome digest. The gate recomputes all four
digests and links before comparing every payload to the selected repository
definition. `validated`, `completed`, and `cleanup_complete` must be true, but
those flags do not replace the payload, digest, link, or semantic checks.

The repository owns this exact scenario mapping:

| Scenario | Fixture | Boundary / projection | Outcome code | Expectation / disposition |
|---|---|---|---|---|
| `fresh_install` | `no_installation` | `cli` / `cli_json` | `installation_completed` | `complete_successfully` / `completed` |
| `runtime_home_creation` | `runtime_home_absent` | `cli` / `cli_json` | `runtime_home_created` | `complete_successfully` / `completed` |
| `personal_managed_binding` | `personal_binding_absent` | `cli` / `cli_json` | `personal_managed_binding_installed` | `complete_successfully` / `completed` |
| `shared_managed_binding` | `shared_binding_absent` | `cli` / `cli_json` | `shared_managed_binding_installed` | `complete_successfully` / `completed` |
| `receipt_create_and_validate` | `current_managed_binding` | `managed_host` / `managed_host_state` | `receipt_current` | `complete_successfully` / `completed` |
| `configuration_drift_detection` | `drifted_managed_configuration` | `managed_host` / `managed_host_state` | `configuration_drift_detected` | `complete_successfully` / `completed` |
| `repair_after_drift` | `repairable_managed_configuration_drift` | `cli` / `cli_json` | `configuration_repaired` | `complete_successfully` / `completed` |
| `safe_uninstall` | `installed_managed_binding` | `cli` / `cli_json` | `managed_binding_removed` | `complete_successfully` / `completed` |
| `symlink_and_canonical_path` | `symlinked_managed_path` | `platform` / `platform_result` | `canonical_path_rules_enforced` | `complete_successfully` / `completed` |
| `codex_restart` | `restarted_codex_process` | `managed_host` / `managed_host_state` | `stale_receipt_rejected` | `complete_successfully` / `completed` |
| `project_move` | `moved_product_repository` | `managed_host` / `managed_host_state` | `moved_project_binding_rejected` | `complete_successfully` / `completed` |
| `record_write_workflow` | `record_workflow_ready` | `mcp_stdio` / `mcp_structured_content` | `record_write_completed` | `complete_successfully` / `completed` |
| `suppression_unavailable` | `suppression_provider_unavailable` | `core` / `core_response` | `observed_paths_preserved` | `preserve_observed_paths_when_suppression_unavailable` / `warning` |
| `unsupported_host` | `unsupported_host_selected` | `cli` / `cli_json` | `unsupported_host_rejected` | `reject_unsupported_host` / `rejected` |
| `unsupported_host_artifact` | `unregistered_host_artifact` | `managed_host` / `managed_host_state` | `unsupported_host_artifact_rejected` | `reject_unsupported_host_artifact` / `rejected` |
| `wsl_shutdown_restart` | `stale_wsl2_process_and_receipt` | `platform` / `platform_result` | `stale_wsl2_process_and_receipt_rejected` | `reject_stale_wsl2_process_and_receipt` / `rejected` |
| `wsl2_ext4_project` | `wsl2_ext4_topology` | `platform` / `platform_result` | `wsl2_ext4_accepted` | `accept_wsl2_ext4` / `completed` |
| `wsl2_drvfs_rejection` | `wsl2_drvfs_topology` | `platform` / `platform_result` | `wsl2_drvfs_rejected` | `reject_wsl2_drvfs` / `rejected` |
| `wsl2_cross_topology_rejection` | `wsl2_cross_topology` | `platform` / `platform_result` | `wsl2_cross_topology_rejected` | `reject_wsl2_cross_topology` / `rejected` |
| `wsl1_rejection` | `wsl1_environment` | `platform` / `platform_result` | `wsl1_rejected` | `reject_wsl1` / `rejected` |
| `wsl2_native_windows_receipt_reuse_rejection` | `native_windows_receipt_in_wsl2` | `managed_host` / `managed_host_state` | `native_windows_receipt_reuse_rejected` | `reject_native_windows_receipt_reuse` / `rejected` |

Only `suppression_unavailable` has
`observed_paths_preserved: true`; every other scenario requires null. An
opaque success flag, an unknown field, a prefixed or stale digest, a broken
digest link, or a driver-selected alternative fixture, boundary, projection,
expectation, disposition, or outcome code cannot satisfy the gate.

The runner supplies `VOLICORD_CODEX_RELEASE_ARTIFACT_DIGEST`,
`VOLICORD_CODEX_RELEASE_VOLICORD_DIGEST`,
`VOLICORD_CODEX_RELEASE_SCENARIO_DRIVER_DIGEST`,
`VOLICORD_CODEX_RELEASE_CAPABILITIES`, and
`VOLICORD_CODEX_RELEASE_INTEGRATION_PROFILE=record`. The repository scenario
catalog owns the exact fixture, boundary, projection, expectation,
disposition, and outcome code. The driver owns execution through that selected
boundary, adapter-specific observation, and bounded cleanup; it reports the
retained canonical records through the closed evidence schema. The runner owns
strict outcome and evidence parsing, canonical digest recomputation, digest-link
validation, semantic comparison with the catalog, catalog completeness, fresh
and exact output placement, deterministic evidence wrapping, retention of each
bounded driver file, timeout, and executable and driver stability. The
observation time is recorded in the candidate result but excluded from the
deterministic per-scenario envelope, so a later blocking replay can reproduce
the reviewed digest while still recording a fresh time. A successful driver
exit without protocol-complete output is a failure. Driver stdout and stderr are
suppressed so prompts, transcripts, credentials, and tokens do not become
workflow output.

## Trust and owner boundaries

The checked-in manifest and its validation evidence support a release decision;
they do not attest a user, sign a receipt, grant Core authority, prove host
isolation, or become a runtime credential. Production runtime trust comes only
from the current managed binding, current Store state, and the verification
receipt contracts owned by their runtime owners. Release fixtures are never
loaded as production trust inputs.

Adjacent owners:

- First-release product scope: [Scope](scope.md).
- Operating-system and WSL2 prerequisites: [System Requirements](system-requirements.md).
- External descriptor and shared Git object-ID rules:
  [External Contracts](external-contracts.md).
- Managed binding and receipt semantics: [Agent Connection](agent-connection.md).
- Install, verify, repair, and uninstall behavior: [Administrative CLI](admin-cli.md).
- Host trust and non-guarantees: [Security](security.md).
- Unsupported-contract category meaning: [Failure Model](failure-model.md).
- Maintained validation commands and release reporting: [Validation](../maintain/validation.md).
