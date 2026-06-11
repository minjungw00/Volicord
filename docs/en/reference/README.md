# Reference index

Use this index to answer: "Which document owns this question?" This README routes to canonical owner documents; it does not define API contracts, schemas, storage effects, security guarantees, or active MVP scope.

These documents are source material for a future Harness Server. They do not mean this repository contains runtime implementation, runtime state, generated artifacts, projections, evidence records, QA records, acceptance records, close records, or conformance output.

## Reading rules

- Start with the question you need to answer, then open only the owner rows that apply.
- Keep contract detail in the owner document. If this index starts to need field lists, response branches, DDL, value sets, or guarantee levels, move that detail to the owner and leave a route here.
- For bilingual or terminology-affecting edits, update the paired English/Korean owner documents in the same batch.
- Do not load paired English and Korean docs for the same `doc_id` in one prompt unless the task is translation or semantic-parity review.
- Preserve exact identifiers in backticks and let the owner document decide their meaning.

## Implementer path

Use this order when moving from product boundary to exact contract owners:

| Step | Owner route |
|---|---|
| Active scope | [Active MVP Scope](active-mvp-scope.md) |
| API methods | [MVP API](api/mvp-api.md) |
| Schema owners | [API Schema Core](api/schema-core.md), [API State Schemas](api/schema-state.md), [API Artifact Schemas](api/schema-artifacts.md), [API Judgment Schemas](api/schema-judgment.md), [API Value Sets](api/schema-value-sets.md), [API Errors](api/errors.md) |
| Storage effects | [Storage Effects](storage-effects.md), then [Storage Records](storage-records.md), [Artifact Storage](storage-artifacts.md), or [Storage Versioning](storage-versioning.md) when that narrower owner applies |

This route is for implementers and reviewers who need exact owners. New and working users should begin with [Start](../start.md) and the [User Guide](../use/user-guide.md).

## Current scope

| Question | Owner document(s) |
|---|---|
| Where is the current MVP scope defined? | [Active MVP Scope](active-mvp-scope.md) |
| Where is the current MVP excluded scope defined? | [Active MVP Scope](active-mvp-scope.md) |
| Is a capability active, profile-gated, or later-only? | [Active MVP Scope](active-mvp-scope.md), [API Value Sets](api/schema-value-sets.md), [Later Candidate Index](../later/index.md) |
| Is `isolated` active in the current MVP? | [Security](security.md), [Active MVP Scope](active-mvp-scope.md) |
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
| Which document owns storage effects? | [Storage Effects](storage-effects.md) |
| Which document owns method-to-storage effects? | [Storage Effects](storage-effects.md) |
| Where does a storage effect question go? | [Storage Effects](storage-effects.md) |
| Which document owns security claims and non-claims? | [Security](security.md) |
| Which document owns product terminology? | [Glossary](glossary.md), [docs/terminology-map.yaml](../../terminology-map.yaml) |
| Which document owns read-only projection authority and source-state/freshness boundaries? | [Projection Authority Reference](projection-and-templates.md) |
| Which document owns status card, judgment request, run/evidence summary, close result, and agent context packet bodies? | [Template Bodies](template-bodies.md) |

## API and schema owners

| Question | Owner document(s) |
|---|---|
| What does `harness.prepare_write` return? | [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API State Schemas](api/schema-state.md), [API Judgment Schemas](api/schema-judgment.md), [Core Model](core-model.md) |
| Where is `ToolRejectedResponse` defined? | [API Schema Core](api/schema-core.md), [API Errors](api/errors.md) |
| Is `STATE_VERSION_CONFLICT` a blocker code? | [API Errors](api/errors.md) |
| When can `harness.close_task` with `dry_run=true` return something other than `ToolDryRunResponse`? | [MVP API](api/mvp-api.md) |
| Which document owns active method names, `response_kind`, `effect_kind`, and enum-like API values? | [API Value Sets](api/schema-value-sets.md) |
| Is `complete` an enum value or the word "full" in this context? | [docs/terminology-map.yaml](../../terminology-map.yaml), [Glossary](glossary.md), [API Value Sets](api/schema-value-sets.md) |
| Where are access classes defined? | [API Value Sets](api/schema-value-sets.md) |
| Where are dry-run preview structures such as `DryRunSummary`, `PlannedEffect`, and `PlannedBlocker` defined? | [API Schema Core](api/schema-core.md), [API Value Sets](api/schema-value-sets.md) |
| Which document owns guarantee label values? | [API Value Sets](api/schema-value-sets.md) |
| Where is `isolated` defined as a value? | [API Value Sets](api/schema-value-sets.md); use [Security](security.md) for guarantee semantics |
| Which document owns `StateSummary`, `ShapingReadiness`, `NextActionSummary`, `CloseReadinessBlocker`, and `ValidatorResult` shapes? | [API State Schemas](api/schema-state.md) |
| Which document owns `ArtifactRef`, `ArtifactInput`, and `StagedArtifactHandle` shapes? | [API Artifact Schemas](api/schema-artifacts.md) |
| Which document owns `UserJudgment`, `SensitiveActionScope`, and accepted-risk input shapes? | [API Judgment Schemas](api/schema-judgment.md) |

## Storage owners

| Question | Owner document(s) |
|---|---|
| Where should I start for the storage document family? | [Storage](storage.md), then the specific storage owner below |
| Which document owns Runtime Home layout, local store assumptions, and table overview? | [Storage Records](storage-records.md), [Runtime Boundaries](runtime-boundaries.md) |
| Is `CloseReadinessBlocker` a storage row? | [Storage Records](storage-records.md) |
| Does artifact staging create evidence? | [Artifact Storage](storage-artifacts.md), [Storage Effects](storage-effects.md) |
| Which document owns artifact promotion? | [Artifact Storage](storage-artifacts.md) |
| Which document owns staged-handle validation and artifact body-read eligibility? | [Artifact Storage](storage-artifacts.md), [API Artifact Schemas](api/schema-artifacts.md) |
| Which document owns idempotency, state clocks, locks, and migrations? | [Storage Versioning](storage-versioning.md), [API Errors](api/errors.md) |

## Security and runtime owners

| Question | Owner document(s) |
|---|---|
| Does the current MVP provide OS sandboxing? | [Security](security.md) |
| Which document owns `isolated` guarantee semantics? | [Security](security.md) |
| Which document owns guarantee semantics? | [Security](security.md) |
| Where are guarantee semantics defined? | [Security](security.md) |
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
| Where should later candidates be documented? | [Later Candidate Index](../later/index.md) |
| Where are later security and assurance candidates documented? | [Security and Assurance Later Candidates](../later/security-and-assurance.md) |
| Where are later artifact and evidence candidates documented? | [Artifacts and Evidence Later Candidates](../later/artifacts-and-evidence.md) |
| Where are artifact later candidates documented? | [Artifacts and Evidence Later Candidates](../later/artifacts-and-evidence.md) |
| Where are later connector and surface candidates documented? | [Connectors and Surfaces Later Candidates](../later/connectors-and-surfaces.md) |
| Where are later policy and conformance candidates documented? | [Policy and Conformance Later Candidates](../later/policy-and-conformance.md) |
| Where are later workflow and collaboration candidates documented? | [Workflow and Collaboration Later Candidates](../later/workflow-and-collaboration.md) |
| Is a later candidate an active requirement? | [Later Candidate Index](../later/index.md), [Active MVP Scope](active-mvp-scope.md) |
| What does promotion-time owner update mean? | [Glossary](glossary.md), [Later Candidate Index](../later/index.md) |
| What else must change before a later candidate becomes active? | [Later Candidate Index](../later/index.md), [Active MVP Scope](active-mvp-scope.md) |
| How should "Complete close-readiness order" be written in Korean? | [Glossary](glossary.md), [Translation Guide](../maintain/translation-guide.md), [API Value Sets](api/schema-value-sets.md) |
| How should "close readiness" be written in Korean? | [docs/terminology-map.yaml](../../terminology-map.yaml), [Glossary](glossary.md), [Translation Guide](../maintain/translation-guide.md) |
| How should Korean close readiness terminology be written? | [docs/terminology-map.yaml](../../terminology-map.yaml), [Glossary](glossary.md), [Translation Guide](../maintain/translation-guide.md) |
| Where is close readiness Korean terminology controlled? | [docs/terminology-map.yaml](../../terminology-map.yaml), [Glossary](glossary.md), [Translation Guide](../maintain/translation-guide.md) |
| Where is Korean terminology controlled? | [docs/terminology-map.yaml](../../terminology-map.yaml), [Translation Guide](../maintain/translation-guide.md), [Glossary](glossary.md) |
| Where are documentation authoring rules? | [Authoring Guide](../maintain/authoring-guide.md) |
| Where is the large-table authoring rule defined? | [Authoring Guide](../maintain/authoring-guide.md), [Checks](../maintain/checks.md) |
| Where are documentation checks? | [Checks](../maintain/checks.md) |
| Where is retrieval or route metadata maintained? | [docs/doc-index.yaml](../../doc-index.yaml) |
| Which document should an agent read first? | [AGENTS.md](../../../AGENTS.md), then [docs/doc-index.yaml](../../doc-index.yaml) |
