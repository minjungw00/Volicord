# Host Release Evidence

This document owns the strict separation between runtime Codex support policy
and exact finalized-artifact release evidence. It defines the embedded
`CodexSupportCatalog`, the external `CodexReleaseEvidenceManifest`, exact
target/environment cells, immutable raw Volicord build artifacts, required
release-validation scenarios, honest cell execution status, and the
build-once publication boundary.

It does not define managed host configuration, receipt semantics, runtime trust,
or operating-system prerequisites. Those contracts remain with their focused
owners. Release-validation fixtures and results are test and release evidence;
they are never production runtime trust inputs.

<a id="surface-stability"></a>
## Surface Stability

The labels below use the
[surface-stability vocabulary](../maintain/documentation-policy.md#surface-stability-labels).
The `CodexSupportCatalog`, `CodexReleaseEvidenceManifest`, exact
artifact-and-capability matching, `unsupported_host_artifact`, five published
targets, six independent target/environment cells, build-artifact metadata,
same-bytes publication rule, and cell status meanings are `stable`. Test runner
modules and fixture layout below those boundaries are `internal`.

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
  target_triple: ReleaseTargetTriple
  platform_environment: PlatformEnvironment
  platform_release_coordinate: PlatformReleaseCoordinate
  integration_profile: record
  verified_capabilities: CodexCapability[]
```

`codex_artifact_digest` is the raw 64-lowercase-hex SHA-256 of the exact
finalized Codex executable bytes authorized by runtime policy. A first-release
entry uses the exact published target, platform, and release coordinate owned by
[Agent Connection](agent-connection.md#platform-environment), exactly
`integration_profile=record`, and exactly `FirstReleaseCodexCapabilities` in
canonical order. Its deterministic identity is exactly
`(codex_artifact_digest, target_triple, platform_environment, integration_profile)`.
Entries are unique and ordered by that identity. The catalog may contain zero
through six entries. Every entry must map to a required cell in the release-target
contract below; an operating-system name alone is never a support identity. An
empty catalog rejects every Codex artifact.

The catalog contains no Volicord executable digest, validation result, scenario
status, evidence path, workflow run identifier, release-cell timestamp, or any
other value derived from the final Volicord executable bytes. Unknown members,
duplicate JSON keys, malformed digests, noncanonical field order, or values
outside owned closed sets invalidate the catalog. Runtime lookup reads only this
embedded catalog and requires an exact digest, target triple, platform
environment, release coordinate, profile, and complete capability match. It
never reads release evidence.

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
  target_triple: ReleaseTargetTriple
  platform_environment: PlatformEnvironment
  observed_capabilities: CodexCapability[]
  integration_profile: record
  validation_evidence: CodexReleaseValidationEvidence
```

Every member is required. The outer coordinates bind the exact Codex artifact,
target triple, platform environment, observed capability set, and profile
exercised by the release cell. `validation_evidence.codex_artifact_digest`,
`target_triple`, `platform_environment`, `observed_capabilities`, and `integration_profile`
must exactly equal the owning entry. Evidence cannot widen or repair them.

<a id="codex-release-validation-evidence"></a>

## `CodexReleaseValidationEvidence`

The nested evidence and runner coordinates have these exact closed shapes and
field order:

```yaml
CodexReleaseValidationEvidence:
  validation_result: passed | failed | unavailable
  codex_artifact_digest: string
  target_triple: ReleaseTargetTriple
  platform_environment: PlatformEnvironment
  observed_capabilities: CodexCapability[]
  integration_profile: record
  volicord_artifact_digest: string
  source_revision: string
  runner: CodexReleaseEvidenceRunner
  scenario_results: CodexReleaseScenarioResult[]
  evidence_digest: string
  observed_at: string

CodexReleaseEvidenceRunner:
  runner_id: string
  target_triple: ReleaseTargetTriple
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
`evidence_digest` are raw 64-lowercase-hex SHA-256 values. `source_revision`
is the raw lowercase 40- or 64-hex Git object ID from which the exact Volicord
artifact was built.
`evidence_digest` at the evidence-object level is also raw 64-lowercase-hex.
All non-null timestamps are canonical RFC 3339 UTC. Runner strings are
nonempty, control-free UTF-8: `runner_id` is at most 256 bytes, while
`os_release` and `environment_image` are at most 512 bytes. The outer, nested,
and runner `target_triple` values must be the same closed exact target, and the
runner architecture must match it. Another cell's runner coordinates cannot be
copied or inferred. The WSL2 cell's
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

Each published Volicord target is built exactly once for one source revision.
The build job completes every publisher-controlled executable-byte operation,
including any signing, stripping, or other post-processing, before calculating
the digest and uploading the raw executable. Validation runs the exact Codex bytes named by
`codex_artifact_digest` and exact Volicord bytes named by
`volicord_artifact_digest`. The evidence `source_revision` must equal the
source revision in that Volicord artifact's build metadata. Before and after
the scenario suite, the runner
reopens both executables and requires the same byte digests. A command name,
path, version range, package label, build identifier, or separately rebuilt
executable cannot substitute for the finalized bytes.

Packaging occurs only after validation and may change archive, checksum, and
metadata bytes, but it must not process or replace the executable. After
creating each archive, publication extracts it and requires the contained
executable SHA-256 to equal the validated raw-binary digest. Any executable-byte
change requires a new build artifact and new validation; it cannot be repaired
by editing metadata or evidence.

Runtime support requires only an exact `CodexSupportEntry` match. Release
eligibility additionally requires one external evidence entry with the same
Codex coordinates, the exact exercised Volicord digest, complete runner and
scenario metadata, and `validation_result=passed`.

Support never propagates from one artifact to another, from one capability to
another, or from one platform to another. A capability observed for one cell
does not become a general Codex capability claim.

<a id="release-build-artifact-flow"></a>

## Release build artifact and publication flow

For each published target, the build matrix uploads exactly one immutable
artifact named
`volicord-build-TARGET-RUN_ID-RUN_ATTEMPT`. It contains only the raw
`volicord` or `volicord.exe`, `volicord.sha256`, and
`build-metadata.json`. The metadata has this exact closed shape:

```yaml
contract_id: volicord.release-build-artifact
target_triple: ReleaseTargetTriple
source_revision: string
binary_name: volicord | volicord.exe
binary_sha256: string
```

`source_revision` is the raw lowercase 40- or 64-hex Git object ID checked out
by the workflow. `binary_sha256` is the raw lowercase SHA-256 of the named
executable. `volicord.sha256` contains the same digest and binary name. Unknown
fields, an unexpected file, a mismatched target, revision, binary name, digest
record, or executable digest invalidate the artifact.

Every required release-cell job downloads the exact artifact for its target,
verifies the metadata and executable digest before execution, sets
`VOLICORD_CODEX_RELEASE_VOLICORD_PATH` to that downloaded executable, and runs
one qualifying capture. The compatibility variable must not select a
runner-installed or independently built Volicord executable. Native Linux and
WSL2 both consume the same `x86_64-unknown-linux-gnu` build artifact. The WSL2
supervisor copies those bytes from the downloaded artifact to a distinct ext4
path inside `Ubuntu-24.04`, verifies the digest inside WSL2 before and after the
cell, and produces a distinct WSL2 manifest.

Each successful cell uploads one artifact named
`volicord-release-evidence-TARGET-PLATFORM-RUN_ID-RUN_ATTEMPT`. It contains
exactly `release-evidence.json` and `scenario-evidence/`. The manifest contains
one exact `passed` entry whose `volicord_artifact_digest` equals the raw build
digest. The scenario directory contains every deterministic scenario envelope
and retained driver payload required by that entry. Publication recomputes the
manifest evidence digest, each scenario-envelope digest, each retained canonical
driver-payload digest, and the raw executable digest.

The production publish job depends on all six required cells and the five-target
build matrix. It downloads only artifacts from the same workflow attempt,
rejects missing or extra targets or cells, and requires every cell to be
`passed` for the corresponding raw build digest. It then packages the raw files
without invoking a Volicord build and verifies the extracted archive member
against the same digest before upload. Existing release assets are not replaced
by a rerun. A skipped or queued self-hosted job, missing Codex artifact, missing
or incomplete evidence, unavailable WSL2 execution, digest drift, or absent
target blocks publication.

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

The single machine-readable release-target contract is:

```text
tests/release-validation/contracts/release-targets.json
```

It has `contract_id=volicord.release-targets`, a duplicate-free
`published_targets` array, and a duplicate-free `required_cells` array. Every
required cell contains `target_triple`, `platform_environment`, and
`integration_profile=record`. Validation rejects an unpublished cell target, a
published target without a cell, an unknown target or environment, and a
target/environment mismatch. Release and CI workflow values that GitHub Actions
cannot load from this JSON are consistency-tested against it, including the
binary packaging matrix.

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

It contains zero through six actually runner-generated and reviewed evidence
entries. Entries are unique and ordered by the exact
`(codex_artifact_digest, target_triple, platform_environment, integration_profile)`
identity. A not-yet-executed source therefore has
`entries: []`. An absent required cell has derived status `not_run`; the source must
not fabricate a placeholder entry, digest, runner coordinate, or evidence
object. Only an actual qualifying attempt can produce `failed` or
`unavailable` evidence.

Production code must not embed this external manifest through `include_bytes!`,
generated Rust, build-script environment variables, compiled constants, or an
equivalent mechanism. Release validation reads it from its canonical on-disk
path, strictly parses it, and cross-checks every evidence entry against the
embedded support catalog. Evidence for a Codex artifact absent from the catalog
is invalid even when its recorded result is `passed`.

Maintain and release the support set in this exact operator order:

1. Obtain the exact finalized Codex artifacts intended for support.
2. Generate deterministic proposed support-catalog entries from those actual
   files. The generator hashes the supplied bytes, normalizes closed target and
   capability values, and emits an entry without a release result or Volicord
   digest.
3. Review and commit the canonical support catalog.
4. Build each published Volicord target once from one source revision after the
   catalog is embedded, and retain each exact raw executable plus digest metadata.
5. Run every required release cell against those exact Codex and Volicord
   binaries. The runner emits bounded external evidence for every required
   scenario and binds the source revision.
6. Verify the complete bundle of catalog, release-target contract, build
   artifacts, digest metadata, manifests, and retained evidence. The verifier
   emits the external verified release index only after every required cell
   passes and every exact binding is unambiguous.
7. Publish the same validated binary bytes together with the verified external
   release index. Packaging may wrap those bytes but must not replace or process
   the executable.

Review generated entries and evidence rather than hand-authoring digests or
passing results. Do not fabricate an unattempted cell, copy another cell's
result, relabel a result, or retain a historical compatibility entry.

Changing either artifact byte, platform coordinate, capability set, profile, or
validation evidence requires a new run of that exact cell. Editing either
contract cannot promote evidence that the runner did not produce.

<a id="verified-release-index"></a>
## `VerifiedReleaseIndex`

The complete-bundle verifier emits one external canonical JSON index with this
exact closed shape and field order:

```yaml
VerifiedReleaseIndex:
  contract_id: volicord.verified-release-index
  source_revision: string
  support_catalog_identity_digest: string
  published_artifacts: VerifiedPublishedArtifact[]
  release_evidence: VerifiedReleaseEvidenceReference[]

VerifiedPublishedArtifact:
  target_triple: ReleaseTargetTriple
  binary_name: volicord | volicord.exe
  binary_sha256: string
  build_artifact: string

VerifiedReleaseEvidenceReference:
  target_triple: ReleaseTargetTriple
  platform_environment: PlatformEnvironment
  integration_profile: record
  codex_artifact_digest: string
  observed_capabilities: CodexCapability[]
  volicord_artifact_digest: string
  evidence_digest: string
  evidence_manifest_sha256: string
  evidence_artifact: string
```

`source_revision` uses the same Git object-ID rule as build and cell evidence.
Every digest is the raw lowercase SHA-256 of the named bytes or canonical
record. `published_artifacts` follows `published_targets` order and
`release_evidence` follows `required_cells` order from the verified release-target
contract. Artifact names include the exact workflow run and attempt. The
manifest digest and canonical evidence digest together reference the exact
external evidence used for approval.

The verifier serializes the index deterministically for identical inputs and
uses create-new output semantics. The index is suitable for attachment to a
release but remains external to Volicord executables. Production code must not
embed or consume it as a `CodexSupportCatalog`, runtime support input, receipt,
credential, or Core authority record.

## Explicit test-only descriptor

Unit and integration tests that do not execute a finalized Codex artifact use
an explicit descriptor separated from both production contracts:

```yaml
TestOnlyCodexDescriptor:
  test_only: true
  fixture_id: string
  codex_artifact_digest: string
  target_triple: ReleaseTargetTriple
  platform_environment: linux | macos | native_windows | wsl2
  observed_capabilities: CodexCapability[]
```

The marker must be the exact boolean `true`. This descriptor may exercise
parsing, routing, negative cases, and adapter projections in test builds. It is
rejected by both contract loaders and every production support lookup, cannot
produce `validation_result=passed`, and cannot register a host artifact or
capability. A test fixture, test-only injection, copied entry, or repository
test pass is not runtime trust and is not finalized-artifact evidence.

<a id="independent-platform-cells"></a>
## Independent target/environment cells

A release-eligible matrix contains these six independent passed cells for all
five published binary targets:

| Target triple | Platform environment | Required environment boundary |
|---|---|---|
| `x86_64-unknown-linux-gnu` | `linux` | Native x86-64 Linux runner and the x86-64 Linux artifact. |
| `aarch64-unknown-linux-gnu` | `linux` | Native AArch64 Linux runner and the AArch64 Linux artifact. |
| `aarch64-apple-darwin` | `macos` | Native Apple Silicon macOS runner and the Apple Silicon artifact. |
| `x86_64-apple-darwin` | `macos` | Native Intel x86-64 macOS runner and the Intel artifact. |
| `x86_64-pc-windows-msvc` | `native_windows` | Native x86-64 Windows runner and native Windows artifact. |
| `x86_64-unknown-linux-gnu` | `wsl2` | The same eligible x86-64 Linux binary may be used, but it runs inside the pinned WSL2 environment and produces distinct WSL2 evidence. |

Each cell runs, records, and reports its own target, artifact, environment,
capability, and evidence coordinates. Passing native `linux` does not pass `wsl2`; passing
`native_windows` does not pass `wsl2`; and a pass on any one artifact does not
support another target. Intel macOS cannot satisfy Apple Silicon, Linux x86-64
cannot satisfy Linux AArch64, and neither direction is allowed. A missing or
non-passing cell blocks the complete release claim rather than being inferred
from another cell.

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

Every required cell exercises the same domain scenario set through its own
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
| `failed` | The cell ran far enough to classify at least one required assertion as failed, or an artifact/evidence integrity check failed. | Blocks the complete six-cell release claim and does not change runtime policy. |
| `unavailable` | A required runner, host, credential, environment, or other execution prerequisite was unavailable, so the cell could not establish a pass or failure for the complete suite. | Blocks the complete six-cell release claim and does not change runtime policy. |

`not_run` is the derived cell status when the evidence manifest has no entry
for that exact target/environment/profile cell; it is not a
`validation_evidence.validation_result` value and does not authorize a
placeholder entry. Scenario results may use `not_run` under the cross-field
rules above. `unavailable` and derived `not_run` are never reported,
summarized, or counted as `passed`. A repository unit test, fixture result,
another cell's pass, or an older artifact's evidence cannot change those
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
    release-targets.json
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
to equal the on-disk support-catalog source, loads the release-target and
release-evidence contracts only from their canonical external paths, and
cross-checks every support and evidence entry against an actual required cell.

```sh
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --status
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --generate-support-entry --codex-path CODEX_PATH --target TARGET --platform PLATFORM --profile record --capabilities managed_stdio_mcp,personal_managed_binding,record_workflow,shared_managed_binding
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --capture-candidate TARGET --platform PLATFORM
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --verify-build-artifact --build-artifact-dir BUILD_DIR --source-revision REVISION --target TARGET
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --verify-cell-evidence --build-artifact-dir BUILD_DIR --evidence-artifact-dir EVIDENCE_DIR --source-revision REVISION --target TARGET --platform PLATFORM
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --verify-publish-evidence --build-root BUILD_ROOT --evidence-root EVIDENCE_ROOT --source-revision REVISION --run-id RUN_ID --run-attempt RUN_ATTEMPT --verified-index-output NEW_INDEX_PATH
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --target x86_64-unknown-linux-gnu --platform linux
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --target aarch64-unknown-linux-gnu --platform linux
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --target aarch64-apple-darwin --platform macos
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --target x86_64-apple-darwin --platform macos
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --target x86_64-pc-windows-msvc --platform native_windows
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --target x86_64-unknown-linux-gnu --platform wsl2
```

`--status` reports the six actual or derived external-evidence statuses and does not
execute a cell. `--generate-support-entry` hashes the actual file at `CODEX_PATH`,
normalizes and validates the closed target, environment, Record profile, and
capability values, and writes one compact canonical JSON entry to standard
output. It contains no release result or Volicord digest and does not edit the
catalog. `--capture-candidate` executes one qualifying attempt and uses
create-new semantics to write an external, strictly parsed, one-entry candidate
manifest. The candidate's Codex coordinates must already exist in the embedded
support catalog. It exits unsuccessfully after retaining a `failed` or
`unavailable` candidate, and it never edits or promotes either canonical
contract. `--target TARGET --platform PLATFORM` is the blocking replay gate and
succeeds only when that exact required cell already has an
exact checked-in `passed` evidence entry matching runtime policy. An absent
entry fails as `not_run`; a checked-in `failed` or `unavailable` entry also
fails. The release workflow uses one capture per current build artifact rather
than replaying the scenario catalog a second time. `--verify-build-artifact`
checks the downloaded raw executable before that capture;
`--verify-cell-evidence` rehashes it and verifies the complete retained cell
evidence afterward. `--verify-publish-evidence` requires the exact five builds
and six passing cell artifacts from one workflow attempt and uses create-new
semantics to write `NEW_INDEX_PATH` only after the complete bundle passes. In
production it rejects an empty support catalog, missing or duplicate evidence,
source or digest mismatches, and every support entry that cannot map to and be
used by exactly one required release cell. The current contract has no
supported non-release-environment marker, so such an entry is invalid. The
honest current empty support catalog blocks capture and publication; an empty
checked-in evidence source still blocks checked-in replay.

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
   target triple, platform environment, driver digest, capabilities, and Record profile. Missing,
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
| `VOLICORD_CODEX_RELEASE_SOURCE_REVISION` | Raw lowercase 40- or 64-hex Git object ID for the exact source revision that produced the Volicord build. Capture records it and verification requires it to match build metadata. |
| `VOLICORD_CODEX_RELEASE_CODEX_PATH` | Canonical symlink-free path of the exact finalized Codex executable. It is a host path for native cells and an absolute Linux ext4 path inside the selected distribution for WSL2. |
| `VOLICORD_CODEX_RELEASE_VOLICORD_PATH` | Canonical symlink-free path of the exact Volicord executable named by `volicord_artifact_digest`, using the same native-or-WSL2 path rule. In the release workflow this is set only to the downloaded raw build artifact, or to its digest-identical WSL2 ext4 copy. A runner-installed Volicord executable is not eligible. |
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
