# Host Release Evidence

This document owns the strict separation between runtime Codex support policy
and exact finalized-artifact release evidence. It defines the embedded
`CodexSupportCatalog`, the external `CodexReleaseEvidenceManifest`, independent
platform cells, required release-validation scenarios, and honest cell
execution status.

It does not define managed host configuration, receipt semantics, runtime trust,
or operating-system prerequisites. Those contracts remain with their focused
owners. Release-validation fixtures and results are test and release evidence;
they are never production runtime trust inputs.

<a id="surface-stability"></a>
## Surface Stability

The labels below use the
[surface-stability vocabulary](../maintain/documentation-policy.md#surface-stability-labels).
The `CodexSupportCatalog`, `CodexReleaseEvidenceManifest`, exact
artifact-and-capability matching, `unsupported_host_artifact`, four independent
platform cells, and cell status meanings are `stable`. Test runner modules and
fixture layout below those boundaries are `internal`.

<a id="codex-support-catalog"></a>

## `CodexSupportCatalog`

The executable embeds one strict runtime policy catalog with this exact closed
shape and field order:

```yaml
CodexSupportCatalog:
  contract_id: volicord.codex-support-catalog
  entries: CodexSupportEntry[]

CodexSupportEntry:
  codex_artifact_digest: string
  platform_environment: PlatformEnvironment
  platform_release_coordinate: PlatformReleaseCoordinate
  integration_profile: record
  verified_capabilities: CodexCapability[]
```

`codex_artifact_digest` is the raw 64-lowercase-hex SHA-256 of the exact
finalized Codex executable bytes authorized by runtime policy. A first-release
entry uses the exact platform and release coordinate owned by
[Agent Connection](agent-connection.md#platform-environment), exactly
`integration_profile=record`, and exactly `FirstReleaseCodexCapabilities` in
canonical order. Entries are unique and appear in `linux`, `macos`,
`native_windows`, `wsl2` order. The catalog may contain zero through four
entries. An empty catalog rejects every Codex artifact.

The catalog contains no Volicord executable digest, validation result, scenario
status, evidence path, workflow run identifier, release-cell timestamp, or any
other value derived from the final Volicord executable bytes. Unknown members,
duplicate JSON keys, malformed digests, noncanonical field order, or values
outside owned closed sets invalidate the catalog. Runtime lookup reads only this
embedded catalog and requires an exact digest, platform, release coordinate,
profile, and complete capability match. It never reads release evidence.

The catalog identity is independent of release evidence. It uses the canonical
`record` and `list` encodings declared below:

```text
support_catalog_identity_digest = lowercase_hex(sha256(
  "volicord.codex-support-catalog\0"
  || record(contract_id, entries)
))
```

Each entry and nested platform release coordinate is encoded in its declared
field order. Changing an external Volicord artifact digest, validation result,
runner, scenario result, evidence path, workflow coordinate, or timestamp
cannot change this identity.

<a id="codex-release-evidence-manifest"></a>

## `CodexReleaseEvidenceManifest`

Release evidence remains external to the executable and uses this exact closed
shape and field order:

```yaml
CodexReleaseEvidenceManifest:
  contract_id: volicord.codex-release-evidence-manifest
  entries: CodexReleaseEvidenceEntry[]

CodexReleaseEvidenceEntry:
  codex_artifact_digest: string
  platform_environment: PlatformEnvironment
  observed_capabilities: CodexCapability[]
  integration_profile: record
  validation_evidence: CodexReleaseValidationEvidence
```

Every member is required. The outer coordinates bind the exact Codex artifact,
platform environment, observed capability set, and profile exercised by the
release cell. `validation_evidence.codex_artifact_digest`,
`platform_environment`, `observed_capabilities`, and `integration_profile`
must exactly equal the owning entry. Evidence cannot widen or repair them.

<a id="codex-release-validation-evidence"></a>

## `CodexReleaseValidationEvidence`

The nested evidence and runner coordinates have these exact closed shapes and
field order:

```yaml
CodexReleaseValidationEvidence:
  validation_result: passed | failed | unavailable
  codex_artifact_digest: string
  platform_environment: PlatformEnvironment
  observed_capabilities: CodexCapability[]
  integration_profile: record
  volicord_artifact_digest: string
  runner: CodexReleaseEvidenceRunner
  scenario_results: CodexReleaseScenarioResult[]
  evidence_digest: string
  observed_at: string

CodexReleaseEvidenceRunner:
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

Every member is required, including nullable members; unknown members,
duplicate JSON keys, and noncanonical field order are invalid.
`codex_artifact_digest`, `volicord_artifact_digest`, and every non-null scenario
`evidence_digest` are raw 64-lowercase-hex SHA-256 values.
`evidence_digest` at the evidence-object level is also raw 64-lowercase-hex.
All non-null timestamps are canonical RFC 3339 UTC. Runner strings are
nonempty, control-free UTF-8: `runner_id` and `target_triple` are at most 256
bytes, while `os_release` and `environment_image` are at most 512 bytes. The
runner fields identify the target and exact execution environment; another
cell's runner coordinates cannot be copied or inferred. The WSL2 cell's
`environment_image` names its pinned Ubuntu LTS distribution image.

`reason` is null for `passed` and otherwise a nonempty machine-readable code
matching `[a-z][a-z0-9_]{0,127}`. A `passed` or `failed` scenario has a
non-null digest and timestamp. An `unavailable` scenario has a non-null reason
and timestamp and may have a null digest only when no bounded evidence artifact
could be produced. A `not_run` scenario has a non-null reason and null digest
and timestamp.

The evidence `validation_result` is `passed` only when every required scenario is
`passed`. It is `failed` when at least one scenario is `failed`. It is
`unavailable` only when no scenario failed, at least one scenario is
`unavailable`, and a qualifying attempt could not complete. Later scenarios
that could not run remain explicit `not_run` results. No top-level `not_run`
evidence object exists: a platform with no qualifying attempt has no manifest
entry.

<a id="codex-release-scenario-catalog"></a>

### Closed Scenario Catalog

Every non-WSL2 evidence entry contains each base scenario exactly once in this
order:

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

Evidence uses the exact `u32be`, `blob`, `string`, `list`, and
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

## Exact policy matching and finalized-artifact evidence

The release cell hashes each executable after every publisher-controlled change to its
bytes, including signing, stripping, packaging extraction, or other
post-processing. Validation runs the exact Codex bytes named by
`codex_artifact_digest` and exact Volicord bytes named by
`volicord_artifact_digest`. Before and after the scenario suite, the runner
reopens both executables and requires the same byte digests. A command name,
path, version range, package label, build identifier, or separately rebuilt
executable cannot substitute for the finalized bytes.

Runtime support requires only an exact `CodexSupportEntry` match. Release
eligibility additionally requires one external evidence entry with the same
Codex coordinates, the exact exercised Volicord digest, complete runner and
scenario metadata, and `validation_result=passed`.

Support never propagates from one artifact to another, from one capability to
another, or from one platform to another. A capability observed for one cell
does not become a general Codex capability claim.

An artifact is registered for runtime support only when one support-catalog
entry exactly
matches the current `ProcessBinding.executable_digest`,
`PlatformEnvironment`, `PlatformReleaseCoordinate`,
`integration_profile=record`, and complete canonical `CodexCapability` set.
The receipt's `executable_digest`, platform, release coordinate, profile,
`required_capabilities`, and `verified_capabilities` must equal those same
policy coordinates. An unknown digest or any platform/release-coordinate/
profile/capability mismatch returns machine-readable reason
`unsupported_host_artifact`. The implementation must not infer support from a
command name, broad version range, neighboring artifact, fixture, subset, or
superset capability match.

<a id="canonical-checked-in-contracts"></a>

## Canonical checked-in contracts

The runtime support-policy source is:

```text
crates/volicord-types/contracts/codex-support-catalog.json
```

Only this file is embedded. It must never gain release results, Volicord
digests, runner metadata, evidence locations, workflow identifiers, or
release-cell timestamps. No fixture, generated constant, documentation table,
runtime database, or release-evidence file may act as a second runtime support
list.

The release-evidence source is external to every Volicord executable:

```text
tests/release-validation/contracts/codex-release-evidence-manifest.json
```

It contains zero through four actually runner-generated and reviewed evidence
entries. At most one entry names each platform, and entries appear in `linux`,
`macos`, `native_windows`, `wsl2` order. A not-yet-executed source therefore has
`entries: []`. An absent platform has derived status `not_run`; the source must
not fabricate a placeholder entry, digest, runner coordinate, or evidence
object. Only an actual qualifying attempt can produce `failed` or
`unavailable` evidence.

Production code must not embed this external manifest through `include_bytes!`,
generated Rust, build-script environment variables, compiled constants, or an
equivalent mechanism. Release validation reads it from its canonical on-disk
path, strictly parses it, and cross-checks every evidence entry against the
embedded support catalog. Evidence for a Codex artifact absent from the catalog
is invalid even when its recorded result is `passed`.

Update one platform as one reviewed operation:

1. Finalize the platform's Codex artifact and calculate
   `codex_artifact_digest` from its final bytes.
2. Add or replace the exact runtime policy entry without any release result or
   Volicord digest, then build and finalize the Volicord artifact that embeds
   that catalog.
3. Execute the complete release-validation cell against those exact Codex and
   Volicord bytes. The runner emits bounded external evidence for every required
   scenario.
4. Recheck both artifact digests and the exact platform, profile, capability,
   runner, target, scenario, and evidence-digest bindings.
5. Review the generated evidence and replace the external entry for that
   platform while preserving canonical order. Do not hand-author an unattempted
   cell, copy another platform's result, relabel a result, or retain a historical
   compatibility entry.
6. Re-evaluate the external manifest against the embedded catalog. A
   four-platform release is eligible only when it contains one current passing
   evidence entry for each platform and every entry exactly matches runtime
   policy.

Changing either artifact byte, platform coordinate, capability set, profile, or
validation evidence requires a new run of that exact cell. Editing either
contract cannot promote evidence that the runner did not produce.

## Explicit test-only descriptor

Unit and integration tests that do not execute a finalized Codex artifact use
an explicit descriptor separated from both production contracts:

```yaml
TestOnlyCodexDescriptor:
  test_only: true
  fixture_id: string
  codex_artifact_digest: string
  platform_environment: linux | macos | native_windows | wsl2
  observed_capabilities: CodexCapability[]
```

The marker must be the exact boolean `true`. This descriptor may exercise
parsing, routing, negative cases, and adapter projections in test builds. It is
rejected by both contract loaders and every production support lookup, cannot
produce `validation_result=passed`, and cannot register a host artifact or
capability. A test fixture, test-only injection, copied entry, or repository
test pass is not runtime trust and is not finalized-artifact evidence.

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

The WSL2 cell uses one Ubuntu LTS image pinned by both runtime policy and the
external evidence. Codex,
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

`validation_evidence.validation_result` is exactly one of:

| Status | Meaning | Release effect |
|---|---|---|
| `passed` | The exact finalized Codex and Volicord artifacts ran in the exact cell environment, every required scenario passed, evidence is complete, and all bindings remain exact. | Satisfies release evidence only for this exact policy entry and Volicord digest. Runtime support still comes from the embedded catalog. |
| `failed` | The cell ran far enough to classify at least one required assertion as failed, or an artifact/evidence integrity check failed. | Blocks the four-platform release claim and does not change runtime policy. |
| `unavailable` | A required runner, host, credential, environment, or other execution prerequisite was unavailable, so the cell could not establish a pass or failure for the complete suite. | Blocks the four-platform release claim and does not change runtime policy. |

`not_run` is the derived platform status when the evidence manifest has no
entry for that platform; it is not a
`validation_evidence.validation_result` value and does not authorize a
placeholder entry. Scenario results may use `not_run` under the cross-field
rules above. `unavailable` and derived `not_run` are never reported,
summarized, or counted as `passed`. A repository unit test, fixture result,
another platform's pass, or an older artifact's evidence cannot change those
meanings.

## Release-validation target layout

The maintained target structure is:

```text
crates/volicord-types/
  contracts/
    codex-support-catalog.json
  src/
    codex_support_catalog.rs
    codex_release_evidence.rs

tests/release-validation/
  contracts/
    codex-release-evidence-manifest.json
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

The shared types crate owns strict runtime-catalog parsing, embedding, exact
support lookup, and strict external-evidence parsing without embedding the
external source. The release-validation `contracts/` route owns the canonical
external path and cross-contract validation.
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
package. It has no contract override. It requires the embedded support catalog
to equal the on-disk support-catalog source, loads the release-evidence manifest
only from its canonical external path, and cross-checks every evidence entry
against runtime policy.

```sh
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --status
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --capture-candidate PLATFORM
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --platform linux
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --platform macos
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --platform native_windows
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --platform wsl2
```

`--status` reports the four actual or derived external-evidence statuses and does not
execute a cell. `--capture-candidate` executes one qualifying attempt and uses
create-new semantics to write an external, strictly parsed, one-entry candidate
manifest. The candidate's Codex coordinates must already exist in the embedded
support catalog. It exits unsuccessfully after retaining a `failed` or
`unavailable` candidate, and it never edits or promotes either canonical
contract. `--platform`
is the blocking replay gate and succeeds only when that platform already has an
exact checked-in `passed` evidence entry matching runtime policy. An absent
entry fails as `not_run`; a checked-in `failed` or `unavailable` entry also
fails. Therefore the honest current `entries: []` sources fail closed and cannot
pass publication.

The producer and gate perform these checks in order:

1. It rejects a process outside the selected independent runner boundary.
2. It derives the actual runner coordinate. Capture records it; blocking replay
   requires it to exactly equal the evidence entry's `runner` value.
3. It hashes the actual Codex and Volicord executable bytes. Blocking replay
   requires the two digests recorded by external evidence.
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
| `VOLICORD_CODEX_RELEASE_ENVIRONMENT_IMAGE` | Exact environment-image coordinate for the support entry during capture and the external evidence entry during blocking replay. |
| `RUNNER_NAME` | Actual runner-service identity; it must equal `runner.runner_id`. |
| `VOLICORD_CODEX_RELEASE_WSL2_DISTRIBUTION` | WSL2 only; exactly `Ubuntu-24.04`. It is rejected for native cells. |
| `VOLICORD_CODEX_RELEASE_CANDIDATE_CELL_PATH` | Capture only; an absent absolute path with an existing canonical parent outside repository-owned, evidence, work, and Runtime Home roots. The producer writes a strict one-entry evidence manifest with create-new semantics. Blocking replay does not read this variable. |

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

The runner supplies `VOLICORD_CODEX_RELEASE_CODEX_ARTIFACT_DIGEST`,
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

The embedded support catalog governs exact runtime policy. The external
release-evidence manifest supports a release decision and never becomes an
executable input. Neither contract attests a user, signs a receipt, grants Core
authority, proves host isolation, or becomes a runtime credential. Production
runtime trust comes only from the current managed binding, current Store state,
the embedded support policy, and the verification receipt contracts owned by
their runtime owners. Release fixtures and evidence are never loaded as
production trust inputs.

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
