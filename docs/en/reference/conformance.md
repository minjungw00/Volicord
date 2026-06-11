# Conformance reference

## Current status

This repository is documentation-only and still in documentation review. It contains no Harness Server runtime, Harness Runtime Home, executable fixture files, conformance runner, generated conformance reports, generated runtime artifacts, or current runtime conformance results.

This document owns documentation-level conformance meaning, candidate future fixture shape, assertion authority boundaries, and a compact scenario index. It does not define API branches, storage effects, access classes, artifact promotion, security guarantees, or close-readiness behavior.

For the canonical current scope, see [Active MVP scope](active-mvp-scope.md). Current phase and handoff status remain owned by [MVP Plan repository status](../build/mvp-plan.md#documentation-acceptance-status).

## Conformance item summary

| Item | Current status | Details |
|---|---|---|
| current documentation criteria | active reference criteria | See [Current documentation criteria](#current-documentation-criteria) |
| internal smoke target | planned/documented | See [Internal smoke target](#internal-smoke-target) |
| future fixture shape | candidate future format | See [Future fixture shape](#future-fixture-shape) |
| future executable fixtures | not implemented | See [Future executable fixtures](#future-executable-fixtures) |
| runtime conformance report | later candidate; not implemented | See [Runtime conformance report](#runtime-conformance-report) |

<a id="current-documentation-criteria"></a>
### Current documentation criteria

Current status:
- Active reference criteria for documentation review and future planning.

Executable now:
- No runtime execution. These criteria do not run a Harness Server or create runtime records.

Owner:
- `docs/en/reference/conformance.md`

Not allowed:
- Do not treat documentation criteria as runtime conformance results, acceptance evidence, or implementation readiness.

<a id="internal-smoke-target"></a>
### Internal smoke target

Current status:
- Planned or documented as an implementation-planning target.

Executable now:
- No, unless a current implementation file explicitly provides it. This repository does not contain one.

Owner:
- [MVP Plan first internal smoke target](../build/mvp-plan.md#first-internal-smoke-target)

Not allowed:
- Do not describe this as an implemented conformance suite, a fixture specification, or proof that Harness is implemented.

<a id="future-fixture-shape"></a>
### Future fixture shape

Current status:
- Candidate future format documented by this reference.

Executable now:
- No. This repository contains no executable fixture files, fixture directory, or fixture runner.

Owner:
- `docs/en/reference/conformance.md`

Not allowed:
- Do not describe the candidate shape as current fixture files, current runner input, or an implemented conformance suite.

<a id="future-executable-fixtures"></a>
### Future executable fixtures

Current status:
- Not implemented.

Executable now:
- No. Executable fixture material requires a future runner and owner-promoted fixtures.

Owner:
- Future runner owner and the owners that promote fixtures. This repository has no current fixture runner or executable fixture owner.

Not allowed:
- Do not add fixture bodies, runner output, generated runtime objects, or current runtime results to this documentation repository.

<a id="runtime-conformance-report"></a>
### Runtime conformance report

Current status:
- Later candidate and not implemented.

Executable now:
- No. This repository contains no conformance runner, generated conformance reports, or runtime conformance results.

Owner:
- [Later Candidate Index](../later/index.md)
- [Later policy and conformance: future conformance run entrypoint](../later/policy-and-conformance.md#future-conformance-run-entrypoint)

Not allowed:
- Do not present metrics, generated prose, rendered reports, or documentation-check labels as conformance authority or current runtime proof.

When this page says "must", "required", or "always", it is naming a documentation criterion or a requirement for a future server/runner after implementation exists. It is not a claim that this repository already contains executable checks.

## What conformance means

For a future server, conformance means executable checks can compare one owner-defined action with owner-defined authority records. Documentation checks are separate maintenance aids for links, terminology, owner boundaries, active/later wording, security wording, and bilingual parity.

A future runtime conformance check must judge only facts made authoritative by an owner document. It must not treat generated prose, agent summaries, rendered reports, status wording, documentation-check labels, or projections as authority unless a specific owner promotes that fact.

## What does not exist yet

The following are future implementation work, not current repository contents:

- Harness Server runtime or Harness Runtime Home data
- executable fixture files or a fixture directory
- a conformance runner or `harness conformance run` implementation
- generated conformance reports, generated runtime artifacts, projections, operational files, or runtime state
- current runtime results for active MVP behavior or later candidates
- current runtime proof of preventive blocking, OS permission control, arbitrary-tool sandboxing, tamper-proof storage, security isolation, or profile-gated `preventive` / `isolated` guarantee claims

Examples on this page may guide planning, but they do not create runtime state, acceptance evidence, close readiness, residual-risk acceptance, generated reports, or implementation readiness.

## Fixture shape

Fixture shape is a candidate future format, not current files. After the Harness Server and runner exist, a promoted fixture should be a compact structured record with these parts:

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
- Names the structured facts a future fixture may compare.

Expected content:
- Response facts, owner-state effects, storage or artifact facts, blocker facts, error facts, guarantee-display facts, and required absence of forbidden side effects.

<a id="fixture-owner-links"></a>
### Owner links

Purpose:
- Routes exact values and meaning to their canonical owners.

Owner links:
- API, Core, Storage, Security, Agent Integration, artifact, and policy owners.

A future materialized fixture must use public owner schemas. It must not invent fixture-only enum values, pseudo-fields, localized display labels as state, prose-only expectations, or later-candidate-only values.

## Assertion authority

Assertion authority is the narrow set of facts a future fixture may judge after executable fixtures exist. Authority comes from owner-defined facts, not from scenario prose or generated summaries.

Future assertions may reference owner-defined response facts, Core state, storage effects, artifact facts, public `ErrorCode` values, structured blockers, guarantee-display facts, and required absence of forbidden side effects.

Exact assertion detail stays with these owners:

| Assertion area | Canonical owner |
|---|---|
| API methods and response branch behavior | [MVP API router](api/mvp-api.md) and method owner documents |
| Common response branches and `dry_run` preview shapes | [API Schema Core](api/schema-core.md) |
| State summaries, blockers, evidence, and close-readiness structures | [API State Schemas](api/schema-state.md) |
| `ArtifactRef`, `ArtifactInput`, and `StagedArtifactHandle` shapes | [API Artifact Schemas](api/schema-artifacts.md) |
| API value sets, including `access_class` values | [API Value Sets](api/schema-value-sets.md) |
| Public errors and precedence | [API Errors](api/errors.md) |
| Storage effects, no-effect branches, and state-version effects | [Storage Effects](storage-effects.md) |
| Artifact staging, promotion, persistence, and body-read lifecycle | [Artifact Storage](storage-artifacts.md) |
| Security non-claims and guarantee levels | [Security](security.md) |
| Runtime location and documentation-only boundaries | [Runtime Boundaries](runtime-boundaries.md) |

## Representative scenario index

These scenario IDs are compact documentation criteria for future fixture planning. They are not fixture bodies, current runtime results, generated runtime objects, or an implementation plan. Use the owner links above for exact branch, storage, access, artifact, security, and close-readiness contracts.

| Scenario ID | Details |
|---|---|
| `MVP-ACTIVE-registered-surface-mismatch-blocks-mutation` | See [registered surface mismatch](#scenario-mvp-active-registered-surface-mismatch-blocks-mutation) |
| `MVP-ACTIVE-verified-local-surface-allows-owner-mutation` | See [verified local surface](#scenario-mvp-active-verified-local-surface-allows-owner-mutation) |
| `MVP-ACTIVE-single-access-class-per-public-request` | See [single access class](#scenario-mvp-active-single-access-class-per-public-request) |
| `MVP-ACTIVE-detective-display-capability-gated` | See [`detective` display](#scenario-mvp-active-detective-display-capability-gated) |
| `MVP-ACTIVE-shaping-readiness-gap-blocks-or-asks` | See [shaping readiness gap](#scenario-mvp-active-shaping-readiness-gap-blocks-or-asks) |
| `MVP-ACTIVE-project-state-version-stale-mutation-rejected` | See [stale mutation](#scenario-mvp-active-project-state-version-stale-mutation-rejected) |
| `MVP-ACTIVE-dry-run-pre-commit-failure-rejected` | See [`dry_run` pre-commit failure](#scenario-mvp-active-dry-run-pre-commit-failure-rejected) |
| `MVP-ACTIVE-status-close-blockers-read-only` | See [read-only close blockers](#scenario-mvp-active-status-close-blockers-read-only) |
| `MVP-ACTIVE-sensitive-approval-records-sensitive-action-scope` | See [sensitive approval scope](#scenario-mvp-active-sensitive-approval-records-sensitive-action-scope) |
| `MVP-ACTIVE-prepare-write-requires-compatible-scope-and-approval` | See [`prepare_write` compatibility](#scenario-mvp-active-prepare-write-requires-compatible-scope-and-approval) |
| `MVP-ACTIVE-authorized-attempt-scope-product-file-write-only` | See [`AuthorizedAttemptScope`](#scenario-mvp-active-authorized-attempt-scope-product-file-write-only) |
| `MVP-ACTIVE-record-run-consumes-write-authorization-once` | See [single-use Write Authorization](#scenario-mvp-active-record-run-consumes-write-authorization-once) |
| `MVP-ACTIVE-stage-artifact-temporary-handle-only` | See [temporary staged handle](#scenario-mvp-active-stage-artifact-temporary-handle-only) |
| `MVP-ACTIVE-record-run-artifact-input-validation-order` | See [artifact input validation order](#scenario-mvp-active-record-run-artifact-input-validation-order) |
| `MVP-ACTIVE-record-run-promotes-staged-artifact-to-artifact-ref` | See [staged artifact promotion](#scenario-mvp-active-record-run-promotes-staged-artifact-to-artifact-ref) |
| `MVP-ACTIVE-record-run-rejects-staged-artifact-surface-instance-mismatch` | See [staged artifact mismatch](#scenario-mvp-active-record-run-rejects-staged-artifact-surface-instance-mismatch) |
| `MVP-ACTIVE-record-run-links-existing-artifact-without-registering-bytes` | See [existing artifact link](#scenario-mvp-active-record-run-links-existing-artifact-without-registering-bytes) |
| `MVP-ACTIVE-captured-artifact-rejected-in-active-mvp` | See [captured artifact rejection](#scenario-mvp-active-captured-artifact-rejected-in-active-mvp) |
| `MVP-ACTIVE-close-task-complete-stale-state-version-rejected` | See [stale close state](#scenario-mvp-active-close-task-complete-stale-state-version-rejected) |
| `MVP-ACTIVE-close-task-complete-stale-write-authorization-basis-rejected` | See [stale Write Authorization basis](#scenario-mvp-active-close-task-complete-stale-write-authorization-basis-rejected) |
| `MVP-ACTIVE-close-task-blocks-current-write-compatibility` | See [write compatibility blocker](#scenario-mvp-active-close-task-blocks-current-write-compatibility) |
| `MVP-ACTIVE-close-task-blocks-evidence-insufficient` | See [evidence blocker](#scenario-mvp-active-close-task-blocks-evidence-insufficient) |
| `MVP-ACTIVE-close-task-blocks-required-artifact-unavailable` | See [artifact availability blocker](#scenario-mvp-active-close-task-blocks-required-artifact-unavailable) |
| `MVP-ACTIVE-close-task-blocks-final-acceptance-missing` | See [final acceptance blocker](#scenario-mvp-active-close-task-blocks-final-acceptance-missing) |
| `MVP-ACTIVE-close-task-blocks-visible-unaccepted-residual-risk` | See [residual risk blocker](#scenario-mvp-active-close-task-blocks-visible-unaccepted-residual-risk) |
| `MVP-ACTIVE-close-task-check-read-only` | See [read-only close check](#scenario-mvp-active-close-task-check-read-only) |
| `MVP-ACTIVE-close-task-state-effecting-dry-run-preview` | See [state-effecting close dry-run](#scenario-mvp-active-close-task-state-effecting-dry-run-preview) |
| `MVP-ACTIVE-close-task-supersede-one-state-version` | See [supersede state version](#scenario-mvp-active-close-task-supersede-one-state-version) |

<a id="scenario-mvp-active-registered-surface-mismatch-blocks-mutation"></a>
### `MVP-ACTIVE-registered-surface-mismatch-blocks-mutation`

Focus:
- Local surface mismatch before mutation.

Owner links:
- [Agent Integration](agent-integration.md)
- [API Errors](api/errors.md)
- [Security](security.md)

<a id="scenario-mvp-active-verified-local-surface-allows-owner-mutation"></a>
### `MVP-ACTIVE-verified-local-surface-allows-owner-mutation`

Focus:
- Verified local surface permits only owner-scoped mutation checks.

Owner links:
- [Agent Integration](agent-integration.md)
- [Shared request rules](api/mvp-api.md#shared-request-rules)
- [Storage Effects](storage-effects.md)

<a id="scenario-mvp-active-single-access-class-per-public-request"></a>
### `MVP-ACTIVE-single-access-class-per-public-request`

Focus:
- One request-level `access_class` per public API request.

Owner links:
- [API Value Sets](api/schema-value-sets.md)
- [Shared request rules](api/mvp-api.md#shared-request-rules)
- [Security](security.md)

<a id="scenario-mvp-active-detective-display-capability-gated"></a>
### `MVP-ACTIVE-detective-display-capability-gated`

Focus:
- `detective` wording requires a supported observed scope.

Owner links:
- [Security](security.md)
- [Agent Integration](agent-integration.md)

<a id="scenario-mvp-active-shaping-readiness-gap-blocks-or-asks"></a>
### `MVP-ACTIVE-shaping-readiness-gap-blocks-or-asks`

Focus:
- Shaping gaps remain owner-path blockers or judgment candidates, not separate planning artifacts.

Owner links:
- [Core Model](core-model.md)
- [API State Schemas](api/schema-state.md)
- [Status method](api/method-status.md)
- [User-judgment methods](api/method-user-judgment.md)

<a id="scenario-mvp-active-project-state-version-stale-mutation-rejected"></a>
### `MVP-ACTIVE-project-state-version-stale-mutation-rejected`

Focus:
- Stale project-wide state version fails before commit.

Owner links:
- [API Errors](api/errors.md)
- [Storage Versioning](storage-versioning.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-mvp-active-dry-run-pre-commit-failure-rejected"></a>
### `MVP-ACTIVE-dry-run-pre-commit-failure-rejected`

Focus:
- `dry_run` does not bypass validation, access, capability, or stale-state rejection.

Owner links:
- [API Schema Core](api/schema-core.md)
- [API Errors](api/errors.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-mvp-active-status-close-blockers-read-only"></a>
### `MVP-ACTIVE-status-close-blockers-read-only`

Focus:
- Status and close-check blockers can be read without storage mutation.

Owner links:
- [Status method](api/method-status.md)
- [Close-task method](api/method-close-task.md)
- [API State Schemas](api/schema-state.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-mvp-active-sensitive-approval-records-sensitive-action-scope"></a>
### `MVP-ACTIVE-sensitive-approval-records-sensitive-action-scope`

Focus:
- Sensitive-action approval is separate from Write Authorization and final acceptance.

Owner links:
- [Core Model](core-model.md)
- [API Judgment Schemas](api/schema-judgment.md)
- [Security](security.md)

<a id="scenario-mvp-active-prepare-write-requires-compatible-scope-and-approval"></a>
### `MVP-ACTIVE-prepare-write-requires-compatible-scope-and-approval`

Focus:
- `prepare_write` is a cooperative product-file compatibility path.

Owner links:
- [Prepare-write method](api/method-prepare-write.md)
- [Core Model](core-model.md)
- [Security](security.md)

<a id="scenario-mvp-active-authorized-attempt-scope-product-file-write-only"></a>
### `MVP-ACTIVE-authorized-attempt-scope-product-file-write-only`

Focus:
- `AuthorizedAttemptScope` is product-file write scope only.

Owner links:
- [Core Model](core-model.md)
- [Prepare-write method](api/method-prepare-write.md)
- [API Judgment Schemas](api/schema-judgment.md)

<a id="scenario-mvp-active-record-run-consumes-write-authorization-once"></a>
### `MVP-ACTIVE-record-run-consumes-write-authorization-once`

Focus:
- Compatible Run recording consumes a matching Write Authorization once.

Owner links:
- [Record-run method](api/method-record-run.md)
- [Storage Effects](storage-effects.md)
- [Storage Versioning](storage-versioning.md)

<a id="scenario-mvp-active-stage-artifact-temporary-handle-only"></a>
### `MVP-ACTIVE-stage-artifact-temporary-handle-only`

Focus:
- Staging creates only a temporary staged handle.

Owner links:
- [Stage-artifact method](api/method-stage-artifact.md)
- [API Artifact Schemas](api/schema-artifacts.md)
- [Artifact Storage](storage-artifacts.md)

<a id="scenario-mvp-active-record-run-artifact-input-validation-order"></a>
### `MVP-ACTIVE-record-run-artifact-input-validation-order`

Focus:
- Run artifact inputs are validated before promotion or linking.

Owner links:
- [Record-run method](api/method-record-run.md)
- [API Artifact Schemas](api/schema-artifacts.md)
- [Artifact Storage](storage-artifacts.md)

<a id="scenario-mvp-active-record-run-promotes-staged-artifact-to-artifact-ref"></a>
### `MVP-ACTIVE-record-run-promotes-staged-artifact-to-artifact-ref`

Focus:
- Compatible Run recording may promote a staged handle to persistent `ArtifactRef`.

Owner links:
- [Artifact Storage](storage-artifacts.md)
- [Record-run method](api/method-record-run.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-mvp-active-record-run-rejects-staged-artifact-surface-instance-mismatch"></a>
### `MVP-ACTIVE-record-run-rejects-staged-artifact-surface-instance-mismatch`

Focus:
- Staged-handle provenance mismatch rejects promotion.

Owner links:
- [Artifact Storage](storage-artifacts.md)
- [API Artifact Schemas](api/schema-artifacts.md)
- [API Errors](api/errors.md)

<a id="scenario-mvp-active-record-run-links-existing-artifact-without-registering-bytes"></a>
### `MVP-ACTIVE-record-run-links-existing-artifact-without-registering-bytes`

Focus:
- Existing persistent artifacts may be linked without registering new bytes.

Owner links:
- [API Artifact Schemas](api/schema-artifacts.md)
- [Artifact Storage](storage-artifacts.md)
- [Record-run method](api/method-record-run.md)

<a id="scenario-mvp-active-captured-artifact-rejected-in-active-mvp"></a>
### `MVP-ACTIVE-captured-artifact-rejected-in-active-mvp`

Focus:
- Native/captured artifact sources are not active MVP artifact authority.

Owner links:
- [Active MVP Scope](active-mvp-scope.md)
- [API Artifact Schemas](api/schema-artifacts.md)
- [Later Candidate Index](../later/index.md)

<a id="scenario-mvp-active-close-task-complete-stale-state-version-rejected"></a>
### `MVP-ACTIVE-close-task-complete-stale-state-version-rejected`

Focus:
- Stale state fails before close-readiness evaluation.

Owner links:
- [Close-task method](api/method-close-task.md)
- [API Errors](api/errors.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-mvp-active-close-task-complete-stale-write-authorization-basis-rejected"></a>
### `MVP-ACTIVE-close-task-complete-stale-write-authorization-basis-rejected`

Focus:
- Stale close-relevant Write Authorization basis fails before close commit.

Owner links:
- [Close-task method](api/method-close-task.md)
- [API Errors](api/errors.md)
- [Storage Versioning](storage-versioning.md)

<a id="scenario-mvp-active-close-task-blocks-current-write-compatibility"></a>
### `MVP-ACTIVE-close-task-blocks-current-write-compatibility`

Focus:
- Close can block on semantic write compatibility.

Owner links:
- [Core Model](core-model.md)
- [Close-task method](api/method-close-task.md)
- [API State Schemas](api/schema-state.md)

<a id="scenario-mvp-active-close-task-blocks-evidence-insufficient"></a>
### `MVP-ACTIVE-close-task-blocks-evidence-insufficient`

Focus:
- Close can block on insufficient required evidence.

Owner links:
- [Core Model](core-model.md)
- [API State Schemas](api/schema-state.md)
- [API Errors](api/errors.md)

<a id="scenario-mvp-active-close-task-blocks-required-artifact-unavailable"></a>
### `MVP-ACTIVE-close-task-blocks-required-artifact-unavailable`

Focus:
- Close can block on required artifact availability.

Owner links:
- [API State Schemas](api/schema-state.md)
- [Artifact Storage](storage-artifacts.md)
- [API Errors](api/errors.md)

<a id="scenario-mvp-active-close-task-blocks-final-acceptance-missing"></a>
### `MVP-ACTIVE-close-task-blocks-final-acceptance-missing`

Focus:
- Close can block on missing compatible final acceptance.

Owner links:
- [Core Model](core-model.md)
- [API Judgment Schemas](api/schema-judgment.md)
- [Close-task method](api/method-close-task.md)

<a id="scenario-mvp-active-close-task-blocks-visible-unaccepted-residual-risk"></a>
### `MVP-ACTIVE-close-task-blocks-visible-unaccepted-residual-risk`

Focus:
- Close can block on visible residual risk without compatible acceptance.

Owner links:
- [Core Model](core-model.md)
- [API Judgment Schemas](api/schema-judgment.md)
- [API State Schemas](api/schema-state.md)

<a id="scenario-mvp-active-close-task-check-read-only"></a>
### `MVP-ACTIVE-close-task-check-read-only`

Focus:
- `harness.close_task intent=check` is read-only.

Owner links:
- [Close-task method](api/method-close-task.md)
- [API Schema Core](api/schema-core.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-mvp-active-close-task-state-effecting-dry-run-preview"></a>
### `MVP-ACTIVE-close-task-state-effecting-dry-run-preview`

Focus:
- State-effecting close intents use dry-run preview only when valid and previewable.

Owner links:
- [Close-task method](api/method-close-task.md)
- [API Schema Core](api/schema-core.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-mvp-active-close-task-supersede-one-state-version"></a>
### `MVP-ACTIVE-close-task-supersede-one-state-version`

Focus:
- Supersede is a terminal non-completion path with one project-wide state mutation when valid.

Owner links:
- [Close-task method](api/method-close-task.md)
- [Core Model](core-model.md)
- [Storage Effects](storage-effects.md)

## Catalog-only future boundary

Future fixture families belong in [Later policy and conformance: future fixture families](../later/policy-and-conformance.md#future-fixture-families). The later-candidate index keeps names only as later candidates, and this page does not reproduce the catalog.

Future-family names are not scenario scripts, fixture bodies, active API payload examples, runner or reporting requirements, active MVP scope, implementation tasks, current results, or current runtime proof. A future owner must promote a narrow behavior with scope, fallback behavior, exact contracts, and proof-path expectations before executable fixture material exists.

## Metrics boundary

Metrics are not conformance authority in the current documentation set. Future local metrics and later conformance reporting may be useful for diagnostics or planning, but until an owner promotes them they remain read-only derived displays or later candidates.

Metrics must not create Core state, satisfy evidence, pass QA or verification, authorize writes, accept final results, accept residual risk, close work, prove implementation readiness, or replace runtime conformance. If a future metric is promoted, its owner must define source records, freshness boundary, display wording, and the non-substitution rule.
