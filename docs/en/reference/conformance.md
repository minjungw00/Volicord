# Conformance reference

## 1. Current status

This repository is documentation-only and still in documentation review. It contains no Harness Server runtime, Harness Runtime Home, executable fixture files, conformance runner, generated conformance reports, generated runtime artifacts, or current runtime conformance results.

This document owns documentation-level conformance meaning, candidate future fixture shape, assertion authority boundaries, and a compact scenario index. It does not define API branches, storage effects, access classes, artifact promotion, security guarantees, or close-readiness behavior.

For the canonical current scope, see [Active MVP scope](active-mvp-scope.md). Current phase and handoff status remain owned by [MVP Plan](../build/mvp-plan.md#documentation-acceptance-status).

| Item | Current status | Owner | Executable now? |
|---|---|---|---|
| current documentation criteria | active reference criteria | `docs/en/reference/conformance.md` | no runtime execution |
| planned internal smoke target | planned/documented | [MVP Plan](../build/mvp-plan.md#first-internal-smoke-target) | no |
| future fixture shape | candidate format | this document | no |
| future executable fixtures | not implemented | future runner and owner-promoted fixtures | no |
| later conformance reporting | later candidate | [Later Candidate Index](../later/index.md) | no |

When this page says "must", "required", or "always", it is naming a documentation criterion or a requirement for a future server/runner after implementation exists. It is not a claim that this repository already contains executable checks.

## 2. What conformance means

For a future server, conformance means executable checks can compare one owner-defined action with owner-defined authority records. Documentation checks are separate maintenance aids for links, terminology, owner boundaries, active/later wording, security wording, and bilingual parity.

A future runtime conformance check must judge only facts made authoritative by an owner document. It must not treat generated prose, agent summaries, rendered reports, status wording, documentation-check labels, or projections as authority unless a specific owner promotes that fact.

## 3. What does not exist yet

The following are future implementation work, not current repository contents:

- Harness Server runtime or Harness Runtime Home data
- executable fixture files or a fixture directory
- a conformance runner or `harness conformance run` implementation
- generated conformance reports, generated runtime artifacts, projections, operational files, or runtime state
- current runtime results for active MVP behavior or later candidates
- current runtime proof of preventive blocking, OS permission control, arbitrary-tool sandboxing, tamper-proof storage, security isolation, or profile-gated `preventive` / `isolated` guarantee claims

Examples on this page may guide planning, but they do not create runtime state, acceptance evidence, close readiness, residual-risk acceptance, generated reports, or implementation readiness.

## 4. Fixture shape

Fixture shape is a candidate future format, not current files. After the Harness Server and runner exist, a promoted fixture should be a compact structured record with these parts:

| Part | Purpose |
|---|---|
| `scenario_id` | Stable identifier for the behavior under review. |
| authority context | The Task, Change Unit, state version, surface, owner refs, Core state, storage rows, artifact refs, and capability facts needed before the action. |
| action | One public Core, API, or operator request using the owner request schema. |
| expected assertions | Structured response facts, owner-state effects, storage or artifact facts, blocker facts, error facts, guarantee-display facts, and required absence of forbidden side effects. |
| owner links | The API, Core, Storage, Security, Agent Integration, artifact, and policy owners that define exact values and meaning. |

A future materialized fixture must use public owner schemas. It must not invent fixture-only enum values, pseudo-fields, localized display labels as state, prose-only expectations, or later-candidate-only values.

## 5. Assertion authority

Assertion authority is the narrow set of facts a future fixture may judge after executable fixtures exist. Authority comes from owner-defined facts, not from scenario prose or generated summaries.

Future assertions may reference owner-defined response facts, Core state, storage effects, artifact facts, public `ErrorCode` values, structured blockers, guarantee-display facts, and required absence of forbidden side effects.

Exact assertion detail stays with these owners:

| Assertion area | Canonical owner |
|---|---|
| API methods and response branch behavior | [MVP API](api/mvp-api.md) |
| Common response branches and `dry_run` preview shapes | [API Schema Core](api/schema-core.md) |
| State summaries, blockers, evidence, and close-readiness structures | [API State Schemas](api/schema-state.md) |
| `ArtifactRef`, `ArtifactInput`, and `StagedArtifactHandle` shapes | [API Artifact Schemas](api/schema-artifacts.md) |
| API value sets, including `access_class` values | [API Value Sets](api/schema-value-sets.md) |
| Public errors and precedence | [API Errors](api/errors.md) |
| Storage effects, no-effect branches, and state-version effects | [Storage Effects](storage-effects.md) |
| Artifact staging, promotion, persistence, and body-read lifecycle | [Artifact Storage](storage-artifacts.md) |
| Security non-claims and guarantee levels | [Security](security.md) |
| Runtime location and documentation-only boundaries | [Runtime Boundaries](runtime-boundaries.md) |

## 6. Representative scenario index

These scenario IDs are compact documentation criteria for future fixture planning. They are not fixture bodies, current runtime results, generated runtime objects, or an implementation plan. Use the owner links above for exact branch, storage, access, artifact, security, and close-readiness contracts.

| Scenario ID | Scenario focus | Primary owner route |
|---|---|---|
| `MVP-ACTIVE-registered-surface-mismatch-blocks-mutation` | Local surface mismatch before mutation. | [Agent Integration](agent-integration.md), [API Errors](api/errors.md), [Security](security.md) |
| `MVP-ACTIVE-verified-local-surface-allows-owner-mutation` | Verified local surface permits only owner-scoped mutation checks. | [Agent Integration](agent-integration.md), [MVP API](api/mvp-api.md), [Storage Effects](storage-effects.md) |
| `MVP-ACTIVE-single-access-class-per-public-request` | One request-level `access_class` per public API request. | [API Value Sets](api/schema-value-sets.md), [MVP API](api/mvp-api.md), [Security](security.md) |
| `MVP-ACTIVE-detective-display-capability-gated` | `detective` wording requires a supported observed scope. | [Security](security.md), [Agent Integration](agent-integration.md) |
| `MVP-ACTIVE-shaping-readiness-gap-blocks-or-asks` | Shaping gaps remain owner-path blockers or judgment candidates, not separate planning artifacts. | [Core Model](core-model.md), [API State Schemas](api/schema-state.md), [MVP API](api/mvp-api.md) |
| `MVP-ACTIVE-project-state-version-stale-mutation-rejected` | Stale project-wide state version fails before commit. | [API Errors](api/errors.md), [Storage Versioning](storage-versioning.md), [Storage Effects](storage-effects.md) |
| `MVP-ACTIVE-dry-run-pre-commit-failure-rejected` | `dry_run` does not bypass validation, access, capability, or stale-state rejection. | [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), [Storage Effects](storage-effects.md) |
| `MVP-ACTIVE-status-close-blockers-read-only` | Status and close-check blockers can be read without storage mutation. | [MVP API](api/mvp-api.md), [API State Schemas](api/schema-state.md), [Storage Effects](storage-effects.md) |
| `MVP-ACTIVE-sensitive-approval-records-sensitive-action-scope` | Sensitive-action approval is separate from Write Authorization and final acceptance. | [Core Model](core-model.md), [API Judgment Schemas](api/schema-judgment.md), [Security](security.md) |
| `MVP-ACTIVE-prepare-write-requires-compatible-scope-and-approval` | `prepare_write` is a cooperative product-file compatibility path. | [MVP API](api/mvp-api.md), [Core Model](core-model.md), [Security](security.md) |
| `MVP-ACTIVE-authorized-attempt-scope-product-file-write-only` | `AuthorizedAttemptScope` is product-file write scope only. | [Core Model](core-model.md), [MVP API](api/mvp-api.md), [API Judgment Schemas](api/schema-judgment.md) |
| `MVP-ACTIVE-record-run-consumes-write-authorization-once` | Compatible Run recording consumes a matching Write Authorization once. | [MVP API](api/mvp-api.md), [Storage Effects](storage-effects.md), [Storage Versioning](storage-versioning.md) |
| `MVP-ACTIVE-stage-artifact-temporary-handle-only` | Staging creates only a temporary staged handle. | [MVP API](api/mvp-api.md), [API Artifact Schemas](api/schema-artifacts.md), [Artifact Storage](storage-artifacts.md) |
| `MVP-ACTIVE-record-run-artifact-input-validation-order` | Run artifact inputs are validated before promotion or linking. | [MVP API](api/mvp-api.md), [API Artifact Schemas](api/schema-artifacts.md), [Artifact Storage](storage-artifacts.md) |
| `MVP-ACTIVE-record-run-promotes-staged-artifact-to-artifact-ref` | Compatible Run recording may promote a staged handle to persistent `ArtifactRef`. | [Artifact Storage](storage-artifacts.md), [MVP API](api/mvp-api.md), [Storage Effects](storage-effects.md) |
| `MVP-ACTIVE-record-run-rejects-staged-artifact-surface-instance-mismatch` | Staged-handle provenance mismatch rejects promotion. | [Artifact Storage](storage-artifacts.md), [API Artifact Schemas](api/schema-artifacts.md), [API Errors](api/errors.md) |
| `MVP-ACTIVE-record-run-links-existing-artifact-without-registering-bytes` | Existing persistent artifacts may be linked without registering new bytes. | [API Artifact Schemas](api/schema-artifacts.md), [Artifact Storage](storage-artifacts.md), [MVP API](api/mvp-api.md) |
| `MVP-ACTIVE-captured-artifact-rejected-in-active-mvp` | Native/captured artifact sources are not active MVP artifact authority. | [Active MVP Scope](active-mvp-scope.md), [API Artifact Schemas](api/schema-artifacts.md), [Later Candidate Index](../later/index.md) |
| `MVP-ACTIVE-close-task-complete-stale-state-version-rejected` | Stale state fails before close-readiness evaluation. | [MVP API](api/mvp-api.md), [API Errors](api/errors.md), [Storage Effects](storage-effects.md) |
| `MVP-ACTIVE-close-task-complete-stale-write-authorization-basis-rejected` | Stale close-relevant Write Authorization basis fails before close commit. | [MVP API](api/mvp-api.md), [API Errors](api/errors.md), [Storage Versioning](storage-versioning.md) |
| `MVP-ACTIVE-close-task-blocks-current-write-compatibility` | Close can block on semantic write compatibility. | [Core Model](core-model.md), [MVP API](api/mvp-api.md), [API State Schemas](api/schema-state.md) |
| `MVP-ACTIVE-close-task-blocks-evidence-insufficient` | Close can block on insufficient required evidence. | [Core Model](core-model.md), [API State Schemas](api/schema-state.md), [API Errors](api/errors.md) |
| `MVP-ACTIVE-close-task-blocks-required-artifact-unavailable` | Close can block on required artifact availability. | [API State Schemas](api/schema-state.md), [Artifact Storage](storage-artifacts.md), [API Errors](api/errors.md) |
| `MVP-ACTIVE-close-task-blocks-final-acceptance-missing` | Close can block on missing compatible final acceptance. | [Core Model](core-model.md), [API Judgment Schemas](api/schema-judgment.md), [MVP API](api/mvp-api.md) |
| `MVP-ACTIVE-close-task-blocks-visible-unaccepted-residual-risk` | Close can block on visible residual risk without compatible acceptance. | [Core Model](core-model.md), [API Judgment Schemas](api/schema-judgment.md), [API State Schemas](api/schema-state.md) |
| `MVP-ACTIVE-close-task-check-read-only` | `harness.close_task intent=check` is read-only. | [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [Storage Effects](storage-effects.md) |
| `MVP-ACTIVE-close-task-state-effecting-dry-run-preview` | State-effecting close intents use dry-run preview only when valid and previewable. | [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [Storage Effects](storage-effects.md) |
| `MVP-ACTIVE-close-task-supersede-one-state-version` | Supersede is a terminal non-completion path with one project-wide state mutation when valid. | [MVP API](api/mvp-api.md), [Core Model](core-model.md), [Storage Effects](storage-effects.md) |

## 7. Catalog-only future boundary

Future fixture families belong in [Later Candidate Index: Future Fixture Families](../later/index.md#future-fixture-families). That index keeps names only as later candidates, and this page does not reproduce the catalog.

Future-family names are not scenario scripts, fixture bodies, active API payload examples, runner or reporting requirements, active MVP scope, implementation tasks, current results, or current runtime proof. A future owner must promote a narrow behavior with scope, fallback behavior, exact contracts, and proof-path expectations before executable fixture material exists.

## 8. Metrics boundary

Metrics are not conformance authority in the current documentation set. Future local metrics and later conformance reporting may be useful for diagnostics or planning, but until an owner promotes them they remain read-only derived displays or later candidates.

Metrics must not create Core state, satisfy evidence, pass QA or verification, authorize writes, accept final results, accept residual risk, close work, prove implementation readiness, or replace runtime conformance. If a future metric is promoted, its owner must define source records, freshness boundary, display wording, and the non-substitution rule.
