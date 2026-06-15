# Conformance reference

## Boundary

This reference defines stable conformance scenario semantics and reference criteria.

A conformance scenario is a named behavior criterion. It can be evaluated only against facts made authoritative by the API, storage, security, scope, Core, artifact, and surface owner documents.

This document owns:

- `scenario_id` naming rules
- conformance scenario semantics
- expected behavior summaries for the scenario index
- assertion authority boundaries for conformance criteria
- the relationship between conformance criteria, canonical owner documents, examples, and tutorials

This reference does not define neighboring contracts:

- API and storage: API branches, storage effects, access classes, and artifact promotion
- security and close readiness: security guarantees and close-readiness behavior
- implementation: implementation routing

For the canonical baseline scope, see [Scope](scope.md). For compact meanings of curated core terms included in the glossary, see [Glossary](glossary.md). For complete structured terminology metadata, see [`docs/terminology-map.yaml`](../../terminology-map.yaml).

## Conformance item summary

| Item | Boundary | Details |
|---|---|---|
| Scenario semantics | Stable behavior criterion | [Details](#scenario-semantics) |
| Scenario IDs | Stable identifier rules | [Details](#scenario-id-rules) |
| Expected behavior | Owner-routed criteria | [Details](#expected-behavior) |
| Assertion authority | Facts owned by canonical owners | [Details](#assertion-authority) |
| Examples and tutorials | Non-authoritative illustrations | [Details](#criteria-vs-examples-and-tutorials) |

<a id="scenario-semantics"></a>
### Scenario semantics

Definition:
- A conformance scenario names one behavior criterion that belongs to the baseline scope or to a clearly routed owner boundary.

Required parts:
- `scenario_id`
- expected behavior
- owner links
- assertion boundary

Allowed effect:
- A scenario may summarize what a conforming result must preserve, reject, expose, or leave unchanged.

Not allowed:
- A scenario must not redefine the API, storage, security, scope, close-readiness, artifact, or surface contract it cites.

<a id="scenario-id-rules"></a>
### Scenario ID rules

Definition:
- `scenario_id` is the stable identifier for the behavior under review.

Rules:
- Use `BASELINE-*` IDs for baseline-scope behavior.
- Name the observable behavior, not a project phase, review stage, work queue, or implementation status.
- Keep IDs stable when the expected behavior remains stable.
- Rename an ID only when the scenario's meaning changes, and update same-page anchors and internal links in the same batch.

Not allowed:
- Do not use short-lived status labels, date labels, runner names, or maintainer workflow labels as scenario IDs.

<a id="expected-behavior"></a>
### Expected behavior

Definition:
- Expected behavior is the stable criterion a conforming implementation or check must satisfy for the scenario.

Owner relation:
- This page may state the scenario-level outcome.
- Exact request fields, response branches, storage effects, error precedence, guarantee levels, and close-readiness details remain in their canonical owner documents.

Conflict rule:
- If a scenario summary and a canonical owner disagree, the canonical owner wins. Correct this page rather than implementing the conflicting summary.

Not allowed:
- Do not treat scenario prose, summaries, rendered views, metrics, or maintenance-check labels as authority for facts the owner documents do not define.

## What conformance means

Conformance means an implementation or check can compare one owner-defined action with owner-defined authority records and owner-defined non-effects.

Conformance criteria judge only facts made authoritative by an owner document. Scenario prose, agent summaries, rendered views, status wording, maintenance-check labels, or projections become conformance authority only when a specific owner defines that fact as authoritative.

They must not be treated as authority by themselves.

When this page says "must", "required", or "always", it is naming a conformance criterion or an owner-routed requirement. It is not redefining neighboring contracts.

<a id="criteria-vs-examples-and-tutorials"></a>
## Criteria vs examples and tutorials

Conformance criteria are reference criteria. Examples and tutorials may illustrate how a reader might recognize a scenario, but they do not create authority records, API branches, storage effects, security guarantees, close-readiness results, acceptance evidence, or residual-risk acceptance.

Reference scenarios must use stable behavior descriptions. They must not use maintainer workflow labels, broad review stages, or short-lived project status as the behavior being tested.

No example, tutorial, or representative scenario requires API documentation to reuse one product scenario. API examples may use any stable, self-contained product or user scenario that stays consistent with the applicable owner contracts.

## Scenario criterion shape

A conformance scenario criterion uses this compact structure:

| Part | Details |
|---|---|
| `scenario_id` | See [`scenario_id`](#criterion-scenario-id) |
| authority context | See [Authority context](#criterion-authority-context) |
| action | See [Action](#criterion-action) |
| expected behavior | See [Expected behavior](#criterion-expected-behavior) |
| owner links | See [Owner links](#criterion-owner-links) |

<a id="criterion-scenario-id"></a>
### `scenario_id`

Purpose:
- Stable identifier for the behavior under review.

<a id="criterion-authority-context"></a>
### Authority context

Purpose:
- Names the facts needed before the action.

Expected content:
- Task, Change Unit, state version, surface, owner refs, Core state, storage rows, artifact refs, and capability facts.

<a id="criterion-action"></a>
### Action

Purpose:
- Describes one public Core, API, or operator request.

Owner link:
- The request must use the owner request schema.

<a id="criterion-expected-behavior"></a>
### Expected behavior

Purpose:
- Names the stable outcome that a conforming result must satisfy.

Expected content:
- Response facts, owner-state effects, storage or artifact facts, blocker facts, error facts, guarantee-display facts, and required absence of forbidden side effects.

<a id="criterion-owner-links"></a>
### Owner links

Purpose:
- Routes exact values and meaning to their canonical owners.

Owner links:
- API, Core, Storage, Security, Agent Integration, artifact, and policy owners.

A conformance criterion must use public owner schemas. It must not invent criterion-only enum values, pseudo-fields, localized display labels as state, prose-only expectations, or out-of-scope-only values.

<a id="assertion-authority"></a>
## Assertion authority

Assertion authority is the narrow set of facts a conformance criterion may judge. Authority comes from owner-defined facts, not from scenario prose or generated summaries.

Conformance assertions may reference owner-defined response facts, Core state, storage effects, artifact facts, public `ErrorCode` values, structured blockers, guarantee-display facts, and required absence of forbidden side effects.

Exact assertion detail stays with these owners:

| Assertion area | Canonical owner |
|---|---|
| API methods and response branch behavior | [API Methods](api/methods.md) and method owner documents |
| Common response branches and `dry_run` preview shapes | [API Schema Core](api/schema-core.md) |
| State summaries, blockers, evidence, and close-readiness structures | [API State Schemas](api/schema-state.md) |
| `ArtifactRef`, `ArtifactInput`, and `StagedArtifactHandle` shapes | [API Artifact Schemas](api/schema-artifacts.md) |
| API value sets, including `access_class` values | [API Value Sets](api/schema-value-sets.md) |
| Public errors and precedence | [API error codes](api/error-codes.md), [API error precedence](api/error-precedence.md) |
| Storage effects, no-effect branches, and state-version effects | [Storage Effects](storage-effects.md) |
| Artifact staging, promotion, persistence, and body-read lifecycle | [Artifact Storage](storage-artifacts.md) |
| Security non-claims and guarantee levels | [Security](security.md) |
| Runtime location and documentation boundaries | [Runtime Boundaries](runtime-boundaries.md) |

## Representative scenario index

These scenario IDs are compact reference criteria. They are not examples, tutorials, runtime results, an implementation plan, or required API example payloads. Use the owner links above for exact branch, storage, access, artifact, security, and close-readiness contracts.

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
  See [single-use `Write Authorization`](#scenario-baseline-record-run-consumes-write-authorization-once).
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
  See [stale `Write Authorization` basis](#scenario-baseline-close-task-complete-stale-write-authorization-basis-rejected).
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

Expected behavior:
- Local surface mismatch before mutation.

Owner links:
- [Agent Integration](agent-integration.md)
- [API error codes](api/error-codes.md)
- [API error routing](api/error-routing.md)
- [Security](security.md)

<a id="scenario-baseline-verified-local-surface-allows-owner-mutation"></a>
### `BASELINE-verified-local-surface-allows-owner-mutation`

Expected behavior:
- Verified local surface permits only owner-scoped mutation checks.

Owner links:
- [Agent Integration](agent-integration.md)
- [API method owner routing](api/methods.md#method-owner-routing-table)
- [Storage Effects](storage-effects.md)

<a id="scenario-baseline-single-access-class-per-public-request"></a>
### `BASELINE-single-access-class-per-public-request`

Expected behavior:
- One request-level `access_class` per public API request.

Owner links:
- [API Value Sets](api/schema-value-sets.md)
- [Agent Integration](agent-integration.md)
- [Security](security.md)

<a id="scenario-baseline-detective-display-capability-gated"></a>
### `BASELINE-detective-display-capability-gated`

Expected behavior:
- `detective` wording requires a supported observed scope.

Owner links:
- [Security](security.md)
- [Agent Integration](agent-integration.md)

<a id="scenario-baseline-shaping-readiness-gap-blocks-or-asks"></a>
### `BASELINE-shaping-readiness-gap-blocks-or-asks`

Expected behavior:
- Shaping gaps remain contract-defined blockers or judgment candidates, not separate planning artifacts.

Owner links:
- [Core Model](core-model.md)
- [API State Schemas](api/schema-state.md)
- [Status method](api/method-status.md)
- [Request-user-judgment method](api/method-request-user-judgment.md)
- [Record-user-judgment method](api/method-record-user-judgment.md)

<a id="scenario-baseline-project-state-version-stale-mutation-rejected"></a>
### `BASELINE-project-state-version-stale-mutation-rejected`

Expected behavior:
- Stale project-wide state version fails before commit.

Owner links:
- [State version conflict](api/error-precedence.md#state-conflict-behavior)
- [Storage Versioning](storage-versioning.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-baseline-dry-run-pre-commit-failure-rejected"></a>
### `BASELINE-dry-run-pre-commit-failure-rejected`

Expected behavior:
- `dry_run` does not bypass validation, access, capability, or stale-state rejection.

Owner links:
- [API Schema Core](api/schema-core.md)
- [`dry_run=true` pre-preview failure](api/error-routing.md#rejected-dry-run-pre-preview-failure)
- [Storage Effects](storage-effects.md)

<a id="scenario-baseline-status-close-blockers-read-only"></a>
### `BASELINE-status-close-blockers-read-only`

Expected behavior:
- Status and close-check blockers can be read without storage mutation.

Owner links:
- [Status method](api/method-status.md)
- [Close-task method](api/method-close-task.md)
- [API State Schemas](api/schema-state.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-baseline-sensitive-approval-records-sensitive-action-scope"></a>
### `BASELINE-sensitive-approval-records-sensitive-action-scope`

Expected behavior:
- Sensitive-action approval is separate from `Write Authorization` and final acceptance.

Owner links:
- [Core Model](core-model.md)
- [API Judgment Schemas](api/schema-judgment.md)
- [Security](security.md)

<a id="scenario-baseline-prepare-write-requires-compatible-scope-and-approval"></a>
### `BASELINE-prepare-write-requires-compatible-scope-and-approval`

Expected behavior:
- `prepare_write` is a cooperative product-file compatibility path.

Owner links:
- [Prepare-write method](api/method-prepare-write.md)
- [Core Model](core-model.md)
- [Security](security.md)

<a id="scenario-baseline-authorized-attempt-scope-product-file-write-only"></a>
### `BASELINE-authorized-attempt-scope-product-file-write-only`

Expected behavior:
- `AuthorizedAttemptScope` is product-file write scope only.

Owner links:
- [Core Model](core-model.md)
- [Prepare-write method](api/method-prepare-write.md)
- [API Judgment Schemas](api/schema-judgment.md)

<a id="scenario-baseline-record-run-consumes-write-authorization-once"></a>
### `BASELINE-record-run-consumes-write-authorization-once`

Expected behavior:
- Compatible Run recording consumes a matching `Write Authorization` once.

Owner links:
- [Record-run method](api/method-record-run.md)
- [Storage Effects](storage-effects.md)
- [Storage Versioning](storage-versioning.md)

<a id="scenario-baseline-stage-artifact-transient-handle-only"></a>
### `BASELINE-stage-artifact-transient-handle-only`

Expected behavior:
- Staging creates only a transient staged handle.

Owner links:
- [Stage-artifact method](api/method-stage-artifact.md)
- [API Artifact Schemas](api/schema-artifacts.md)
- [Artifact Storage](storage-artifacts.md)

<a id="scenario-baseline-record-run-artifact-input-validation-order"></a>
### `BASELINE-record-run-artifact-input-validation-order`

Expected behavior:
- Run artifact inputs are validated before promotion or linking.

Owner links:
- [Record-run method](api/method-record-run.md)
- [API Artifact Schemas](api/schema-artifacts.md)
- [Artifact Storage](storage-artifacts.md)

<a id="scenario-baseline-record-run-promotes-staged-artifact-to-artifact-ref"></a>
### `BASELINE-record-run-promotes-staged-artifact-to-artifact-ref`

Expected behavior:
- Compatible Run recording may promote a staged handle to persistent `ArtifactRef`.

Owner links:
- [Artifact Storage](storage-artifacts.md)
- [Record-run method](api/method-record-run.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-baseline-record-run-rejects-staged-artifact-surface-instance-mismatch"></a>
### `BASELINE-record-run-rejects-staged-artifact-surface-instance-mismatch`

Expected behavior:
- Staged-handle provenance mismatch rejects promotion.

Owner links:
- [Artifact Storage](storage-artifacts.md)
- [API Artifact Schemas](api/schema-artifacts.md)
- [Artifact-input error details](api/error-details.md#artifact-input-error-reason)

<a id="scenario-baseline-record-run-links-existing-artifact-without-registering-bytes"></a>
### `BASELINE-record-run-links-existing-artifact-without-registering-bytes`

Expected behavior:
- Existing persistent artifacts may be linked without registering new bytes.

Owner links:
- [API Artifact Schemas](api/schema-artifacts.md)
- [Artifact Storage](storage-artifacts.md)
- [Record-run method](api/method-record-run.md)

<a id="scenario-baseline-captured-artifact-rejected-in-baseline-scope"></a>
### `BASELINE-captured-artifact-rejected-in-baseline-scope`

Expected behavior:
- Native/captured artifact sources are not baseline artifact authority.

Owner links:
- [Scope](scope.md)
- [API Artifact Schemas](api/schema-artifacts.md)
- [Scope Reference](scope.md)

<a id="scenario-baseline-close-task-complete-stale-state-version-rejected"></a>
### `BASELINE-close-task-complete-stale-state-version-rejected`

Expected behavior:
- Stale state fails before close-readiness evaluation.

Owner links:
- [Close-task method](api/method-close-task.md)
- [State version conflict](api/error-precedence.md#state-conflict-behavior)
- [Storage Effects](storage-effects.md)

<a id="scenario-baseline-close-task-complete-stale-write-authorization-basis-rejected"></a>
### `BASELINE-close-task-complete-stale-write-authorization-basis-rejected`

Expected behavior:
- Stale close-relevant `Write Authorization` basis fails before close commit.

Owner links:
- [Close-task method](api/method-close-task.md)
- [State version conflict](api/error-precedence.md#state-conflict-behavior)
- [State conflict detail fields](api/error-details.md#state-conflict-detail-fields)
- [Storage Versioning](storage-versioning.md)

<a id="scenario-baseline-close-task-blocks-current-write-compatibility"></a>
### `BASELINE-close-task-blocks-current-write-compatibility`

Expected behavior:
- Close can block on semantic write compatibility.

Owner links:
- [Core Model](core-model.md)
- [Close-task method](api/method-close-task.md)
- [API State Schemas](api/schema-state.md)

<a id="scenario-baseline-close-task-blocks-evidence-insufficient"></a>
### `BASELINE-close-task-blocks-evidence-insufficient`

Expected behavior:
- Close can block on insufficient required evidence.

Owner links:
- [Core Model](core-model.md)
- [API State Schemas](api/schema-state.md)
- [Close-task method](api/method-close-task.md)
- [API blocker routing](api/blocker-routing.md)

<a id="scenario-baseline-close-task-blocks-required-artifact-unavailable"></a>
### `BASELINE-close-task-blocks-required-artifact-unavailable`

Expected behavior:
- Close can block on required artifact availability.

Owner links:
- [API State Schemas](api/schema-state.md)
- [Artifact Storage](storage-artifacts.md)
- [Close-task method](api/method-close-task.md)
- [API blocker routing](api/blocker-routing.md)

<a id="scenario-baseline-close-task-blocks-final-acceptance-missing"></a>
### `BASELINE-close-task-blocks-final-acceptance-missing`

Expected behavior:
- Close can block on missing compatible final acceptance.

Owner links:
- [Core Model](core-model.md)
- [API Judgment Schemas](api/schema-judgment.md)
- [Close-task method](api/method-close-task.md)

<a id="scenario-baseline-close-task-blocks-visible-unaccepted-residual-risk"></a>
### `BASELINE-close-task-blocks-visible-unaccepted-residual-risk`

Expected behavior:
- Close can block on visible residual risk without compatible acceptance.

Owner links:
- [Core Model](core-model.md)
- [API Judgment Schemas](api/schema-judgment.md)
- [API State Schemas](api/schema-state.md)

<a id="scenario-baseline-close-task-check-read-only"></a>
### `BASELINE-close-task-check-read-only`

Expected behavior:
- `harness.close_task intent=check` is read-only.

Owner links:
- [Close-task method](api/method-close-task.md)
- [API Schema Core](api/schema-core.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-baseline-close-task-state-effecting-dry-run-preview"></a>
### `BASELINE-close-task-state-effecting-dry-run-preview`

Expected behavior:
- State-effecting close intents use dry-run preview only when valid and previewable.

Owner links:
- [Close-task method](api/method-close-task.md)
- [API Schema Core](api/schema-core.md)
- [Storage Effects](storage-effects.md)

<a id="scenario-baseline-close-task-supersede-one-state-version"></a>
### `BASELINE-close-task-supersede-one-state-version`

Expected behavior:
- Supersede is a terminal non-completion path with one project-wide state mutation when valid.

Owner links:
- [Close-task method](api/method-close-task.md)
- [Core Model](core-model.md)
- [Storage Effects](storage-effects.md)

## Catalog boundary

Scenario family names outside the baseline scope belong in [Scope](scope.md). The scope owner may keep names only as out-of-scope capabilities, and this page does not reproduce that catalog.

Out-of-scope family names are not:

- scenario scripts
- supported API payload examples
- runner or reporting requirements
- baseline scope
- implementation tasks
- runtime results
- runtime proof

## Metrics boundary

Metrics are not conformance authority. A metric affects a conformance criterion only when a canonical owner defines the source records, freshness boundary, display wording, and non-substitution rule.

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
