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
| Active scope | `active-mvp-scope.md` |
| API method list | `api/mvp-api.md` |
| API method behavior | [API method owners](#api-method-owners) |
| Schema shapes | [API and schema owners](#api-and-schema-owners) |
| Storage effects | `storage-effects.md` |

This route is for implementers and reviewers who need exact owners. New and working users should begin with [Start](../start.md) and the [User Guide](../use/user-guide.md).

## Current scope

| Question | Owner |
|---|---|
| Where is current MVP inclusion defined? | `active-mvp-scope.md` |
| Where is current MVP exclusion defined? | `active-mvp-scope.md` |
| Is a capability active, profile-gated, or later-only? | `active-mvp-scope.md` |
| Is `isolated` active in the current MVP? | `active-mvp-scope.md`, `security.md` |
| Has runtime implementation started? | `../build/mvp-plan.md` |
| Where is the documentation-only boundary defined? | `runtime-boundaries.md`, `active-mvp-scope.md` |
| Where is maintainer handoff status tracked? | `../build/mvp-plan.md` |

## Find the owner document

| Question | Owner |
|---|---|
| Where is Core authority defined? | `core-model.md` |
| Where is the active API method list? | `api/mvp-api.md` |
| Where are shared API request envelopes defined? | `api/schema-core.md` |
| Where are response branches defined? | `api/schema-core.md` |
| Where are public error codes defined? | `api/errors.md` |
| Where are storage records defined? | `storage-records.md` |
| Where are storage effects defined? | `storage-effects.md` |
| Where are method storage effects defined? | `storage-effects.md` |
| Where are security claims defined? | `security.md` |
| Where is product terminology defined? | `glossary.md`, `../../terminology-map.yaml` |
| Where is projection authority defined? | `projection-and-templates.md` |
| Where are template bodies defined? | `template-bodies.md` |

## API and schema owners

| Question | Owner |
|---|---|
| What scenario do API examples use? | `api/mvp-api.md`, `../maintain/authoring-guide.md`, `../maintain/checks.md` |
| Can API examples use documentation maintenance as the scenario? | `../maintain/authoring-guide.md` |
| Where are API example checks defined? | `../maintain/checks.md`, `../maintain/authoring-guide.md` |
| Where are API example consistency checks defined? | `../maintain/checks.md`, `../maintain/authoring-guide.md` |
| Where are API example field-name checks defined? | `../maintain/checks.md`, `../maintain/authoring-guide.md` |
| Where is the active API method list? | `api/mvp-api.md` |
| Where are API method-name values defined? | `api/schema-value-sets.md` |
| Where are method payload fields defined? | [API method owners](#api-method-owners) |
| Where are shared payload schemas defined? | `api/schema-core.md` |
| Where is the `harness.status` `state_version` example rule? | `api/method-status.md`, `../maintain/checks.md` |
| What does `harness.prepare_write` return? | `api/method-prepare-write.md` |
| Where are `harness.prepare_write` response branches defined? | `api/schema-core.md` |
| Where are `harness.prepare_write` state shapes defined? | `api/schema-state.md` |
| Where are `harness.prepare_write` judgment shapes defined? | `api/schema-judgment.md` |
| Where is `harness.prepare_write` sensitive approval defined? | `api/method-prepare-write.md` |
| Where is `ToolRejectedResponse` defined? | `api/schema-core.md` |
| Is `STATE_VERSION_CONFLICT` a blocker code? | `api/errors.md` |
| What can `harness.close_task` return with `dry_run=true`? | `api/method-close-task.md` |
| Where are enum-like API values defined? | `api/schema-value-sets.md` |
| Is `complete` an enum value or prose? | `../../terminology-map.yaml`, `glossary.md` |
| Where are access classes defined? | `api/schema-value-sets.md` |
| Where are `DryRunSummary`, `PlannedEffect`, and `PlannedBlocker` defined? | `api/schema-core.md` |
| Where are guarantee label values defined? | `api/schema-value-sets.md` |
| Where are guarantee semantics defined? | `security.md` |
| Where is `isolated` defined as a value? | `api/schema-value-sets.md` |
| Where is `isolated` guarantee meaning defined? | `security.md` |
| Where are state summary shapes defined? | `api/schema-state.md` |
| Where are artifact reference shapes defined? | `api/schema-artifacts.md` |
| Where are judgment input shapes defined? | `api/schema-judgment.md` |

## API method owners

| Question | Owner |
|---|---|
| Where is `harness.intake` behavior defined? | `api/method-intake.md` |
| Where is `harness.update_scope` behavior defined? | `api/method-update-scope.md` |
| Where is `harness.status` behavior defined? | `api/method-status.md` |
| Where is `harness.prepare_write` behavior defined? | `api/method-prepare-write.md` |
| Where is `harness.stage_artifact` behavior defined? | `api/method-stage-artifact.md` |
| Where is `harness.record_run` behavior defined? | `api/method-record-run.md` |
| Where is `harness.record_run` evidence behavior defined? | `api/method-record-run.md`, `storage-effects.md` |
| Where are `harness.record_run` storage effects defined? | `storage-effects.md` |
| Where is `harness.request_user_judgment` behavior defined? | `api/method-user-judgment.md` |
| Where is `harness.record_user_judgment` behavior defined? | `api/method-user-judgment.md` |
| Where is `harness.close_task` behavior defined? | `api/method-close-task.md` |

## Storage owners

| Question | Owner |
|---|---|
| Where should I start for the storage document family? | `storage.md` |
| Where is Harness Runtime Home separation defined? | `runtime-boundaries.md` |
| Where are local store assumptions defined? | `storage-records.md` |
| Where are storage record values defined? | `storage-records.md` |
| Is `CloseReadinessBlocker` a storage row? | `storage-records.md` |
| Does artifact staging create evidence? | `storage-artifacts.md`, `storage-effects.md` |
| Where are artifact staging and promotion defined? | `storage-artifacts.md` |
| Where are artifact reference schemas defined? | `api/schema-artifacts.md` |
| Where is staged-handle validation defined? | `storage-artifacts.md` |
| Where are state clocks and migrations defined? | `storage-versioning.md` |

## Security and runtime owners

| Question | Owner |
|---|---|
| Does the current MVP provide OS sandboxing? | `security.md` |
| Where are `isolated` guarantee semantics defined? | `security.md` |
| Where are guarantee semantics defined? | `security.md` |
| Where is runtime separation defined? | `runtime-boundaries.md` |
| Where is local connector behavior defined? | `agent-integration.md` |
| Where is verified surface context defined? | `agent-integration.md` |
| Where are verified guarantee boundaries defined? | `security.md` |
| Where are surface recipes defined? | `../use/surface-recipes.md` |
| Where is security error mapping defined? | `api/errors.md` |

## User judgment and close-readiness owners

| Question | Owner |
|---|---|
| Where is user-owned judgment meaning defined? | `core-model.md` |
| Where is user judgment prompt behavior defined? | `api/method-user-judgment.md`, `core-model.md` |
| Where are user judgment schemas defined? | `api/schema-judgment.md` |
| Where is sensitive-action approval meaning defined? | `core-model.md` |
| Where are sensitive-action approval schemas defined? | `api/schema-judgment.md` |
| Where are sensitive-action security semantics defined? | `security.md` |
| Where is close readiness meaning defined? | `core-model.md` |
| Where is `harness.close_task` behavior defined? | `api/method-close-task.md` |
| Where is close-readiness blocker shape defined? | `api/schema-state.md` |
| Where is close error routing defined? | `api/errors.md` |
| Where are acceptance and residual-risk boundaries defined? | `core-model.md` |
| Where are accepted-risk schemas defined? | `api/schema-judgment.md` |
| Where are accepted-risk values defined? | `api/schema-value-sets.md` |
| Where is compact evidence summary meaning defined? | `core-model.md` |
| Where is compact evidence summary shape defined? | `api/schema-state.md` |

## Later and maintenance owners

| Question | Owner |
|---|---|
| Where should later candidates be documented? | `../later/index.md` |
| Where are security later candidates documented? | `../later/security-and-assurance.md` |
| Where are artifact later candidates documented? | `../later/artifacts-and-evidence.md` |
| Where are connector later candidates documented? | `../later/connectors-and-surfaces.md` |
| Where are policy later candidates documented? | `../later/policy-and-conformance.md` |
| Where are workflow later candidates documented? | `../later/workflow-and-collaboration.md` |
| Is a later candidate an active requirement? | `active-mvp-scope.md` |
| What does promotion-time owner update mean? | `glossary.md`, `../later/index.md` |
| What must change before a later candidate becomes active? | `../later/index.md` |
| How should "Full close-readiness evaluation order" be written in Korean? | `glossary.md`, `../maintain/translation-guide.md` |
| How should "close readiness" be written in Korean? | `../../terminology-map.yaml`, `glossary.md`, `../maintain/translation-guide.md` |
| Where are Korean prose and translation guidance controlled? | `../maintain/translation-guide.md`, `../../terminology-map.yaml`, `glossary.md` |
| Where are documentation authoring rules? | `../maintain/authoring-guide.md` |
| Where are large-table rules defined? | `../maintain/authoring-guide.md`, `../maintain/checks.md` |
| When should a long Markdown table be split? | `../maintain/authoring-guide.md`, `../maintain/checks.md` |
| When should a dense reference paragraph be split? | `../maintain/authoring-guide.md`, `../maintain/checks.md` |
| Where are documentation checks? | `../maintain/checks.md` |
| Where is retrieval or route metadata maintained? | `../../doc-index.yaml` |
| Which document should an agent read first? | `../../../AGENTS.md`, `../../doc-index.yaml` |
