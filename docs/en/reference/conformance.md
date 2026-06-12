# Conformance reference

## Boundary

This reference defines conformance terminology and document-level criteria. Runtime conformance outputs, generated reports, executable fixtures, and runner artifacts belong outside the documentation tree.

This document owns documentation-level conformance meaning, candidate fixture shape, assertion authority boundaries, and a compact scenario index. It does not define API branches, storage effects, access classes, artifact promotion, security guarantees, or close-readiness behavior.

For the canonical baseline scope, see [Scope](scope.md). Implementation routing is described in the [Implementation Guide](../build/implementation-guide.md).

## Conformance item summary

| Item | Boundary | Details |
|---|---|---|
| Documentation criteria | Supported documentation criteria | [Details](#documentation-criteria) |
| Internal smoke target | Implementation-owned check target | [Details](#internal-smoke-target) |
| Fixture shape | Out-of-scope candidate shape | [Details](#fixture-shape-boundary) |
| Executable fixtures | Out of documentation scope | [Details](#executable-fixtures) |
| Runtime conformance report | Out-of-scope capability | [Details](#runtime-conformance-report) |

<a id="documentation-criteria"></a>
### Documentation criteria

Boundary:
- Supported reference criteria for documentation maintenance.

Execution boundary:
- No runtime execution. These criteria do not run a Harness Server, execute a conformance suite, or create runtime records.

Owner:
- `docs/en/reference/conformance.md`

Not allowed:
- Do not treat documentation criteria as runtime conformance results, acceptance evidence, or implementation routing.

<a id="internal-smoke-target"></a>
### Internal smoke target

Boundary:
- Implementation-owned check target.

Execution boundary:
- Execution behavior belongs to the implementation owner.

Owner:
- `build/implementation-guide.md`

Not allowed:
- Do not describe this as a documentation-owned conformance suite.

<a id="fixture-shape-boundary"></a>
### Fixture shape boundary

Boundary:
- Out-of-scope candidate format documented by this reference.

Execution boundary:
- The candidate shape is not a fixture file, runner input, or suite entry point.

Owner:
- `docs/en/reference/conformance.md`

Not allowed:
- Do not describe the candidate shape as fixture files, runner input, or an executable conformance suite.

<a id="executable-fixtures"></a>
### Executable fixtures

Boundary:
- Out of documentation scope.

Execution boundary:
- Executable fixture material requires a runner owner and owner-promoted fixtures.

Owner:
- Runner owner and the owners that promote fixtures.

Not allowed:
- Do not add fixture bodies, runner output, generated runtime objects, or runtime results to this documentation repository.

<a id="runtime-conformance-report"></a>
### Runtime conformance report

Boundary:
- Out-of-scope capability.

Execution boundary:
- Conformance runners, suite entry points, generated conformance reports, and runtime conformance results belong outside documentation.

Owner:
- [Scope Reference](scope.md)
- [Policy and conformance: conformance run entrypoint](scope.md)

Not allowed:
- Do not present metrics, generated prose, rendered reports, or documentation-check labels as conformance authority or runtime proof.

When this page says "must", "required", or "always", it is naming a documentation criterion or a requirement for a server or runner contract. It is not a claim that documentation creates executable checks.

## What conformance means

For a server, conformance means executable checks can compare one owner-defined action with owner-defined authority records. Documentation checks are separate maintenance aids for links, terminology, owner boundaries, active/out-of-scope wording, security wording, and bilingual parity.

A runtime conformance check must judge only facts made authoritative by an owner document. It must not treat generated prose, agent summaries, rendered reports, status wording, documentation-check labels, or projections as authority unless a specific owner promotes that fact.

## Documentation boundary

The following are runtime or implementation outputs, not documentation repository contents:

- Harness Server runtime or Harness Runtime Home data
- executable fixture files or a fixture directory
- a conformance runner or `harness conformance run` implementation
- generated conformance reports, generated runtime artifacts, projections, operational files, or runtime state
- runtime results for baseline behavior or out-of-scope capabilities
- runtime proof of preventive blocking, OS permission control, arbitrary-tool sandboxing, tamper-proof storage, security isolation, or profile-gated `preventive` / `isolated` guarantee claims

Examples on this page explain conformance concepts, but they do not create runtime state, acceptance evidence, close readiness, residual-risk acceptance, generated reports, or implementation routing.

## Fixture shape

Fixture shape is an out-of-scope candidate format, not documentation-owned fixture files. A promoted fixture should be a compact structured record with these parts:

| Part | Details |
|---|---|
| `scenario_id` | See [`scenario_id`](#fixture-scenario-id) |
| authority context | See [Authority context](#fixture-authority-context) |
| action | See [Action](#fixture-action) |
| expected assertions | See [Expected assertions](#fixture-expected-assertions) |
| owner links | See [Owner links](#fixture-owner-links) |

<a id="fixture-scenario-id"></a>
### `scenario_id`

Purpose:
- Stable identifier for the behavior under review.

<a id="fixture-authority-context"></a>
### Authority context

Purpose:
- Names the facts needed before the action.

Expected content:
- Task, Change Unit, state version, surface, owner refs, Core state, storage rows, artifact refs, and capability facts.

<a id="fixture-action"></a>
### Action

Purpose:
- Describes one public Core, API, or operator request.

Owner link:
- The request must use the owner request schema.

<a id="fixture-expected-assertions"></a>
### Expected assertions

Purpose:
- Names the structured facts an owner-promoted fixture may compare.

Expected content:
- Response facts, owner-state effects, storage or artifact facts, blocker facts, error facts, guarantee-display facts, and required absence of forbidden side effects.

<a id="fixture-owner-links"></a>
### Owner links

Purpose:
- Routes exact values and meaning to their canonical owners.

Owner links:
- API, Core, Storage, Security, Agent Integration, artifact, and policy owners.

An owner-promoted materialized fixture must use public owner schemas. It must not invent fixture-only enum values, pseudo-fields, localized display labels as state, prose-only expectations, or out-of-scope-only values.

## Assertion authority

Assertion authority is the narrow set of facts an owner-promoted fixture may judge. Authority comes from owner-defined facts, not from scenario prose or generated summaries.

Fixture assertions may reference owner-defined response facts, Core state, storage effects, artifact facts, public `ErrorCode` values, structured blockers, guarantee-display facts, and required absence of forbidden side effects.

Exact assertion detail stays with these owners:

| Assertion area | Canonical owner |
|---|---|
| API methods and response branch behavior | [API Methods](api/methods.md) and method owner documents |
| Common response branches and `dry_run` preview shapes | [API Schema Core](api/schema-core.md) |
| State summaries, blockers, evidence, and close-readiness structures | [API State Schemas](api/schema-state.md) |
| `ArtifactRef`, `ArtifactInput`, and `StagedArtifactHandle` shapes | [API Artifact Schemas](api/schema-artifacts.md) |
| API value sets, including `access_class` values | [API Value Sets](api/schema-value-sets.md) |
| Public errors and precedence | [API Errors](api/errors.md) |
| Storage effects, no-effect branches, and state-version effects | [Storage Effects](storage-effects.md) |
| Artifact staging, promotion, persistence, and body-read lifecycle | [Artifact Storage](storage-artifacts.md) |
| Security non-claims and guarantee levels | [Security](security.md) |
| Runtime location and documentation boundaries | [Runtime Boundaries](runtime-boundaries.md) |

## Representative scenario index

These scenario IDs are compact documentation criteria for owner-promoted fixture design. They are not fixture bodies, runtime results, generated runtime objects, or an implementation plan. Use the owner links above for exact branch, storage, access, artifact, security, and close-readiness contracts.

- `BASELINE-registered-surface-mismatch-blocks-mutation`
  See [registered surface mismatch](#scenario-baseline-registered-surface-mismatch-blocks-mutation).
- `BASELINE-verified-local-surface-allows-owner-mutation`
  See [verified local surface](#scenario-baseline-verified-local-surface-allows-owner-mutation).
- `BASELINE-single-access-class-per-public-request`
  See [single access class](#scenario-baseline-single-access-class-per-public-request).
- `BASELINE-detective-display-capability-gated`
  See [`detective` display](#scenario-baseline-detective-display-capability-gated).
- `BASELINE-shaping-readiness-gap-blocks-or-asks`
  See [shaping readiness gap](#scenario-baseline-shaping-readiness-gap-blocks-or-asks).
- `BASELINE-project-state-version-stale-mutation-rejected`
  See [stale mutation](#scenario-baseline-project-state-version-stale-mutation-rejected).
- `BASELINE-dry-run-pre-commit-failure-rejected`
  See [`dry_run` pre-commit failure](#scenario-baseline-dry-run-pre-commit-failure-rejected).
- `BASELINE-status-close-blockers-read-only`
  See [read-only close blockers](#scenario-baseline-status-close-blockers-read-only).
- `BASELINE-sensitive-approval-records-sensitive-action-scope`
  See [sensitive approval scope](#scenario-baseline-sensitive-approval-records-sensitive-action-scope).
- `BASELINE-prepare-write-requires-compatible-scope-and-approval`
  See [`prepare_write` compatibility](#scenario-baseline-prepare-write-requires-compatible-scope-and-approval).
- `BASELINE-authorized-attempt-scope-product-file-write-only`
  See [`AuthorizedAttemptScope`](#scenario-baseline-authorized-attempt-scope-product-file-write-only).
- `BASELINE-record-run-consumes-write-authorization-once`
  See [single-use Write Authorization](#scenario-baseline-record-run-consumes-write-authorization-once).
- `BASELINE-stage-artifact-transient-handle-only`
  See [transient staged handle](#scenario-baseline-stage-artifact-transient-handle-only).
- `BASELINE-record-run-artifact-input-validation-order`
  See [artifact input validation order](#scenario-baseline-record-run-artifact-input-validation-order).
- `BASELINE-record-run-promotes-staged-artifact-to-artifact-ref`
  See [staged artifact promotion](#scenario-baseline-record-run-promotes-staged-artifact-to-artifact-ref).
- `BASELINE-record-run-rejects-staged-artifact-surface-instance-mismatch`
  See [staged artifact mismatch](#scenario-baseline-record-run-rejects-staged-artifact-surface-instance-mismatch).
- `BASELINE-record-run-links-existing-artifact-without-registering-bytes`
  See [existing artifact link](#scenario-baseline-record-run-links-existing-artifact-without-registering-bytes).
- `BASELINE-captured-artifact-rejected-in-baseline-scope`
  See [captured artifact rejection](#scenario-baseline-captured-artifact-rejected-in-baseline-scope).
- `BASELINE-close-task-complete-stale-state-version-rejected`
  See [stale close state](#scenario-baseline-close-task-complete-stale-state-version-rejected).
- `BASELINE-close-task-complete-stale-write-authorization-basis-rejected`
  See [stale Write Authorization basis](#scenario-baseline-close-task-complete-stale-write-authorization-basis-rejected).
- `BASELINE-close-task-blocks-current-write-compatibility`
  See [write compatibility blocker](#scenario-baseline-close-task-blocks-current-write-compatibility).
- `BASELINE-close-task-blocks-evidence-insufficient`
  See [evidence blocker](#scenario-baseline-close-task-blocks-evidence-insufficient).
- `BASELINE-close-task-blocks-required-artifact-unavailable`
  See [artifact availability blocker](#scenario-baseline-close-task-blocks-required-artifact-unavailable).
- `BASELINE-close-task-blocks-final-acceptance-missing`
  See [final acceptance blocker](#scenario-baseline-close-task-blocks-final-acceptance-missing).
- `BASELINE-close-task-blocks-visible-unaccepted-residual-risk`
  See [residual risk blocker](#scenario-baseline-close-task-blocks-visible-unaccepted-residual-risk).
- `BASELINE-close-task-check-read-only`
  See [read-only close check](#scenario-baseline-close-task-check-read-only).
- `BASELINE-close-task-state-effecting-dry-run-preview`
  See [state-effecting close dry-run](#scenario-baseline-close-task-state-effecting-dry-run-preview).
- `BASELINE-close-task-supersede-one-state-version`
  See [supersede state version](#scenario-baseline-close-task-supersede-one-state-version).

<a id="scenario-baseline-registered-surface-mismatch-blocks-mutation"></a>
### `BASELINE-registered-surface-mismatch-blocks-mutation`

Focus:
- Local surface mismatch before mutation.

Owner links:
- [Agent Integration](agent-integration.md)
- [API Errors](api/errors.md)
- [Security](security.md)

<a id="scenario-baseline-verified-local-surface-allows-owner-mutation"></a>
### `BASELINE-verified-local-surface-allows-owner-mutation`

Focus:
- Verified local surface permits only owner-scoped mutation checks.

Owner links:
- [Agent Integration](agent-integration.md)
- [Shared envelope and response branch routes](api/methods.md#shared-request-rules)
- [Storage Effects](storage-effects.md)

<a id="scenario-baseline-single-access-class-per-public-request"></a>
### `BASELINE-single-access-class-per-public-request`

Focus:
- One request-level `access_class` per public API request.

Owner links:
- [API Value Sets](api/schema-value-sets.md)
- [Shared envelope and response branch routes](api/methods.md#shared-request-rules)
- [Security](security.md)

<a id="scenario-baseline-detective-display-capability-gated"></a>
### `BASELINE-detective-display-capability-gated`

Focus:
- `detective` wording requires a supported observed scope.

Owner links:
- [Security](security.md)
- [Agent Integration](agent-integration.md)

<a id="scenario-baseline-shaping-readiness-gap-blocks-or-asks"></a>
### `BASELINE-shaping-readiness-gap-blocks-or-asks`

Focus:
- Shaping gaps remain owner-path blockers or judgment candidates, not separate planning artifacts.

Owner links:
- [Core Model](core-model.md)
- [API State Schemas](api/schema-state.md)
- [Status method](api/method-status.md)
- [User-judgment methods](api/method-user-judgment.md)

<a id="scenario-baseline-project-state-version-stale-mutation-rejected"></a>
### `BASELINE-project-state-version-stale-mutation-rejected`

Focus:
- Stale project-wide state version fails before commit.

Owner links:
- [API Errors](api/errors.md)
- [Storage Versioning](storage-versioning.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-baseline-dry-run-pre-commit-failure-rejected"></a>
### `BASELINE-dry-run-pre-commit-failure-rejected`

Focus:
- `dry_run` does not bypass validation, access, capability, or stale-state rejection.

Owner links:
- [API Schema Core](api/schema-core.md)
- [API Errors](api/errors.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-baseline-status-close-blockers-read-only"></a>
### `BASELINE-status-close-blockers-read-only`

Focus:
- Status and close-check blockers can be read without storage mutation.

Owner links:
- [Status method](api/method-status.md)
- [Close-task method](api/method-close-task.md)
- [API State Schemas](api/schema-state.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-baseline-sensitive-approval-records-sensitive-action-scope"></a>
### `BASELINE-sensitive-approval-records-sensitive-action-scope`

Focus:
- Sensitive-action approval is separate from Write Authorization and final acceptance.

Owner links:
- [Core Model](core-model.md)
- [API Judgment Schemas](api/schema-judgment.md)
- [Security](security.md)

<a id="scenario-baseline-prepare-write-requires-compatible-scope-and-approval"></a>
### `BASELINE-prepare-write-requires-compatible-scope-and-approval`

Focus:
- `prepare_write` is a cooperative product-file compatibility path.

Owner links:
- [Prepare-write method](api/method-prepare-write.md)
- [Core Model](core-model.md)
- [Security](security.md)

<a id="scenario-baseline-authorized-attempt-scope-product-file-write-only"></a>
### `BASELINE-authorized-attempt-scope-product-file-write-only`

Focus:
- `AuthorizedAttemptScope` is product-file write scope only.

Owner links:
- [Core Model](core-model.md)
- [Prepare-write method](api/method-prepare-write.md)
- [API Judgment Schemas](api/schema-judgment.md)

<a id="scenario-baseline-record-run-consumes-write-authorization-once"></a>
### `BASELINE-record-run-consumes-write-authorization-once`

Focus:
- Compatible Run recording consumes a matching Write Authorization once.

Owner links:
- [Record-run method](api/method-record-run.md)
- [Storage Effects](storage-effects.md)
- [Storage Versioning](storage-versioning.md)

<a id="scenario-baseline-stage-artifact-transient-handle-only"></a>
### `BASELINE-stage-artifact-transient-handle-only`

Focus:
- Staging creates only a transient staged handle.

Owner links:
- [Stage-artifact method](api/method-stage-artifact.md)
- [API Artifact Schemas](api/schema-artifacts.md)
- [Artifact Storage](storage-artifacts.md)

<a id="scenario-baseline-record-run-artifact-input-validation-order"></a>
### `BASELINE-record-run-artifact-input-validation-order`

Focus:
- Run artifact inputs are validated before promotion or linking.

Owner links:
- [Record-run method](api/method-record-run.md)
- [API Artifact Schemas](api/schema-artifacts.md)
- [Artifact Storage](storage-artifacts.md)

<a id="scenario-baseline-record-run-promotes-staged-artifact-to-artifact-ref"></a>
### `BASELINE-record-run-promotes-staged-artifact-to-artifact-ref`

Focus:
- Compatible Run recording may promote a staged handle to persistent `ArtifactRef`.

Owner links:
- [Artifact Storage](storage-artifacts.md)
- [Record-run method](api/method-record-run.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-baseline-record-run-rejects-staged-artifact-surface-instance-mismatch"></a>
### `BASELINE-record-run-rejects-staged-artifact-surface-instance-mismatch`

Focus:
- Staged-handle provenance mismatch rejects promotion.

Owner links:
- [Artifact Storage](storage-artifacts.md)
- [API Artifact Schemas](api/schema-artifacts.md)
- [API Errors](api/errors.md)

<a id="scenario-baseline-record-run-links-existing-artifact-without-registering-bytes"></a>
### `BASELINE-record-run-links-existing-artifact-without-registering-bytes`

Focus:
- Existing persistent artifacts may be linked without registering new bytes.

Owner links:
- [API Artifact Schemas](api/schema-artifacts.md)
- [Artifact Storage](storage-artifacts.md)
- [Record-run method](api/method-record-run.md)

<a id="scenario-baseline-captured-artifact-rejected-in-baseline-scope"></a>
### `BASELINE-captured-artifact-rejected-in-baseline-scope`

Focus:
- Native/captured artifact sources are not baseline artifact authority.

Owner links:
- [Scope](scope.md)
- [API Artifact Schemas](api/schema-artifacts.md)
- [Scope Reference](scope.md)

<a id="scenario-baseline-close-task-complete-stale-state-version-rejected"></a>
### `BASELINE-close-task-complete-stale-state-version-rejected`

Focus:
- Stale state fails before close-readiness evaluation.

Owner links:
- [Close-task method](api/method-close-task.md)
- [API Errors](api/errors.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-baseline-close-task-complete-stale-write-authorization-basis-rejected"></a>
### `BASELINE-close-task-complete-stale-write-authorization-basis-rejected`

Focus:
- Stale close-relevant Write Authorization basis fails before close commit.

Owner links:
- [Close-task method](api/method-close-task.md)
- [API Errors](api/errors.md)
- [Storage Versioning](storage-versioning.md)

<a id="scenario-baseline-close-task-blocks-current-write-compatibility"></a>
### `BASELINE-close-task-blocks-current-write-compatibility`

Focus:
- Close can block on semantic write compatibility.

Owner links:
- [Core Model](core-model.md)
- [Close-task method](api/method-close-task.md)
- [API State Schemas](api/schema-state.md)

<a id="scenario-baseline-close-task-blocks-evidence-insufficient"></a>
### `BASELINE-close-task-blocks-evidence-insufficient`

Focus:
- Close can block on insufficient required evidence.

Owner links:
- [Core Model](core-model.md)
- [API State Schemas](api/schema-state.md)
- [API Errors](api/errors.md)

<a id="scenario-baseline-close-task-blocks-required-artifact-unavailable"></a>
### `BASELINE-close-task-blocks-required-artifact-unavailable`

Focus:
- Close can block on required artifact availability.

Owner links:
- [API State Schemas](api/schema-state.md)
- [Artifact Storage](storage-artifacts.md)
- [API Errors](api/errors.md)

<a id="scenario-baseline-close-task-blocks-final-acceptance-missing"></a>
### `BASELINE-close-task-blocks-final-acceptance-missing`

Focus:
- Close can block on missing compatible final acceptance.

Owner links:
- [Core Model](core-model.md)
- [API Judgment Schemas](api/schema-judgment.md)
- [Close-task method](api/method-close-task.md)

<a id="scenario-baseline-close-task-blocks-visible-unaccepted-residual-risk"></a>
### `BASELINE-close-task-blocks-visible-unaccepted-residual-risk`

Focus:
- Close can block on visible residual risk without compatible acceptance.

Owner links:
- [Core Model](core-model.md)
- [API Judgment Schemas](api/schema-judgment.md)
- [API State Schemas](api/schema-state.md)

<a id="scenario-baseline-close-task-check-read-only"></a>
### `BASELINE-close-task-check-read-only`

Focus:
- `harness.close_task intent=check` is read-only.

Owner links:
- [Close-task method](api/method-close-task.md)
- [API Schema Core](api/schema-core.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-baseline-close-task-state-effecting-dry-run-preview"></a>
### `BASELINE-close-task-state-effecting-dry-run-preview`

Focus:
- State-effecting close intents use dry-run preview only when valid and previewable.

Owner links:
- [Close-task method](api/method-close-task.md)
- [API Schema Core](api/schema-core.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-baseline-close-task-supersede-one-state-version"></a>
### `BASELINE-close-task-supersede-one-state-version`

Focus:
- Supersede is a terminal non-completion path with one project-wide state mutation when valid.

Owner links:
- [Close-task method](api/method-close-task.md)
- [Core Model](core-model.md)
- [Storage Effects](storage-effects.md)

## Catalog-only boundary

Fixture families outside the baseline scope belong in [Policy and conformance fixture families](scope.md). The out-of-scope index keeps names only as out-of-scope capabilities, and this page does not reproduce the catalog.

Out-of-scope family names are not:

- scenario scripts
- fixture bodies
- active API payload examples
- runner or reporting requirements
- baseline scope
- implementation tasks
- runtime results
- runtime proof

Promotion requirement: an owner must promote a narrow behavior with scope, fallback behavior, exact contracts, and proof-path expectations before executable fixture material is supported.

## Metrics boundary

Metrics are not conformance authority. Local metrics and conformance reporting may be useful for diagnostics, but until an owner promotes them they remain read-only derived displays or out-of-scope capabilities.

Metrics must not:

- create Core state
- satisfy evidence
- pass QA or verification
- authorize writes
- accept final results
- accept residual risk
- close work
- prove implementation routing
- replace runtime conformance

Promotion requirement: if a metric is promoted, its owner must define source records, freshness boundary, display wording, and the non-substitution rule.
