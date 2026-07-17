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
