# Reference index

Use this index to answer: "Which document owns this question?" This README routes to canonical owner documents; it does not define API contracts, schemas, storage effects, security guarantees, or active MVP scope.

These documents are source material for a future Harness Server. They do not mean this repository contains runtime implementation, runtime state, generated artifacts, projections, evidence records, QA records, acceptance records, close records, or conformance output.

## Reading rules

- Start with the question you need to answer, then open only the owner rows that apply.
- Keep contract detail in the owner document. If this index starts to need field lists, response branches, DDL, value sets, or guarantee levels, move that detail to the owner and leave a route here.
- For bilingual or terminology-affecting edits, update the paired English/Korean owner documents in the same batch.
- Do not load paired English and Korean docs for the same `doc_id` in one prompt unless the task is translation or semantic-parity review.
- Preserve exact identifiers in backticks and let the owner document decide their meaning.

## Current scope

| Question | Owner document(s) |
|---|---|
| What is included in the current MVP? | [Active MVP Scope](active-mvp-scope.md) |
| Is a capability active, profile-gated, or later-only? | [Active MVP Scope](active-mvp-scope.md), [API Value Sets](api/schema-value-sets.md), [Later Candidate Index](../later/index.md) |
| Has this repository started runtime or server implementation? | [MVP Plan](../build/mvp-plan.md), [Active MVP Scope](active-mvp-scope.md) |
| Where is the documentation-only boundary stated? | [Active MVP Scope](active-mvp-scope.md), [Runtime Boundaries](runtime-boundaries.md) |
| Where is implementation-readiness or maintainer handoff status tracked? | [MVP Plan](../build/mvp-plan.md) |

## Find the owner document

| Question | Owner document(s) |
|---|---|
| Which document owns Core authority, Task state, evidence, residual risk, and non-substitution rules? | [Core Model](core-model.md) |
| Which document owns API method behavior? | [MVP API](api/mvp-api.md) |
| Which document owns shared API response branches and envelopes? | [API Schema Core](api/schema-core.md) |
| Which document owns public error codes and error precedence? | [API Errors](api/errors.md) |
| Which document owns storage records or DDL? | [Storage Records](storage-records.md) |
| Which document owns method-to-storage effects? | [Storage Effects](storage-effects.md) |
| Which document owns security claims and non-claims? | [Security](security.md) |
| Which document owns product terminology? | [Glossary](glossary.md), [docs/terminology-map.yaml](../../terminology-map.yaml) |
| Which document owns read-only projection authority and source-state/freshness boundaries? | [Projection Authority Reference](projection-and-templates.md) |
| Which document owns status card, judgment request, run/evidence summary, close result, and agent context packet bodies? | [Template Bodies](template-bodies.md) |

## API and schema owners

| Question | Owner document(s) |
|---|---|
| What does `harness.prepare_write` return? | [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API State Schemas](api/schema-state.md), [API Judgment Schemas](api/schema-judgment.md), [Core Model](core-model.md) |
| Where is `ToolRejectedResponse` defined? | [API Schema Core](api/schema-core.md), [API Errors](api/errors.md) |
| When does `STATE_VERSION_CONFLICT` apply? | [API Errors](api/errors.md), [MVP API](api/mvp-api.md), [Storage Versioning](storage-versioning.md) |
| Which document owns active method names, `response_kind`, `effect_kind`, and enum-like API values? | [API Value Sets](api/schema-value-sets.md) |
| Where are access classes defined? | [API Value Sets](api/schema-value-sets.md), [MVP API](api/mvp-api.md), [Security](security.md) |
| Where are dry-run preview structures such as `DryRunSummary`, `PlannedEffect`, and `PlannedBlocker` defined? | [API Schema Core](api/schema-core.md), [API Value Sets](api/schema-value-sets.md) |
| Which document owns `StateSummary`, `ShapingReadiness`, `NextActionSummary`, `CloseReadinessBlocker`, and `ValidatorResult` shapes? | [API State Schemas](api/schema-state.md) |
| Which document owns `ArtifactRef`, `ArtifactInput`, and `StagedArtifactHandle` shapes? | [API Artifact Schemas](api/schema-artifacts.md) |
| Which document owns `UserJudgment`, `SensitiveActionScope`, and accepted-risk input shapes? | [API Judgment Schemas](api/schema-judgment.md) |

## Storage owners

| Question | Owner document(s) |
|---|---|
| Where should I start for the storage document family? | [Storage](storage.md), then the specific storage owner below |
| Which document owns Runtime Home layout, local store assumptions, and table overview? | [Storage Records](storage-records.md), [Runtime Boundaries](runtime-boundaries.md) |
| Is `CloseReadinessBlocker` a storage row? | [API State Schemas](api/schema-state.md), [Storage Effects](storage-effects.md), [Storage Records](storage-records.md) |
| Does artifact staging create evidence? | [MVP API](api/mvp-api.md), [Storage Effects](storage-effects.md), [Artifact Storage](storage-artifacts.md), [Core Model](core-model.md) |
| Which document owns artifact promotion? | [Artifact Storage](storage-artifacts.md), [MVP API](api/mvp-api.md), [Storage Effects](storage-effects.md) |
| Which document owns staged-handle validation and artifact body-read eligibility? | [Artifact Storage](storage-artifacts.md), [API Artifact Schemas](api/schema-artifacts.md) |
| Which document owns idempotency, state clocks, locks, and migrations? | [Storage Versioning](storage-versioning.md), [API Errors](api/errors.md) |

## Security and runtime owners

| Question | Owner document(s) |
|---|---|
| Does the active MVP provide OS sandboxing? | [Security](security.md), [Runtime Boundaries](runtime-boundaries.md), [Active MVP Scope](active-mvp-scope.md) |
| Which document owns cooperative, detective, preventive, and isolated guarantee wording? | [Security](security.md), [docs/terminology-map.yaml](../../terminology-map.yaml) |
| Which document owns Product Repository, Harness Server, and Harness Runtime Home separation? | [Runtime Boundaries](runtime-boundaries.md) |
| Which document owns local connector behavior, capability context, and verified surface boundaries? | [Agent Integration](agent-integration.md), [MVP API](api/mvp-api.md), [Security](security.md) |
| Which document owns CLI, IDE/editor, chat, and local MCP usage recipes? | [Surface Recipes](../use/surface-recipes.md) |
| Which document owns public security-related error mapping? | [API Errors](api/errors.md), [Security](security.md) |

## User judgment and close-readiness owners

| Question | Owner document(s) |
|---|---|
| Which document owns user-owned judgment and non-substitution rules? | [Core Model](core-model.md), [API Judgment Schemas](api/schema-judgment.md) |
| Which document owns sensitive-action approval boundaries? | [Core Model](core-model.md), [API Judgment Schemas](api/schema-judgment.md), [Security](security.md) |
| Which document owns close readiness and close honesty? | [Core Model](core-model.md), [MVP API](api/mvp-api.md), [API Errors](api/errors.md) |
| Which document owns close-readiness blocker shape and close error routing? | [API State Schemas](api/schema-state.md), [API Errors](api/errors.md) |
| Which document owns final acceptance and residual-risk acceptance boundaries? | [Core Model](core-model.md), [API Judgment Schemas](api/schema-judgment.md), [API Value Sets](api/schema-value-sets.md) |
| Which document owns compact evidence summary meaning? | [Core Model](core-model.md), [API State Schemas](api/schema-state.md), [MVP API](api/mvp-api.md) |

## Later and maintenance owners

| Question | Owner document(s) |
|---|---|
| Where should later candidates be added? | [Later Candidate Index](../later/index.md) |
| What else must change before a later candidate becomes active? | [Later Candidate Index](../later/index.md), [Active MVP Scope](active-mvp-scope.md) |
| Where is Korean terminology controlled? | [docs/terminology-map.yaml](../../terminology-map.yaml), [Translation Guide](../maintain/translation-guide.md), [Glossary](glossary.md) |
| Where are documentation authoring rules? | [Authoring Guide](../maintain/authoring-guide.md) |
| Where are documentation checks? | [Checks](../maintain/checks.md) |
| Where is retrieval or route metadata maintained? | [docs/doc-index.yaml](../../doc-index.yaml) |
