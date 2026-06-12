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
| Where is current MVP included scope defined? | `active-mvp-scope.md` |
| Where is current MVP excluded scope defined? | `active-mvp-scope.md` |
| Is a capability active, profile-gated, or later-only? | `active-mvp-scope.md` |
| Is `isolated` active in the current MVP? | `active-mvp-scope.md`, `security.md` |
| Has runtime or server implementation started? | `../build/mvp-plan.md` |
| Where is the documentation-only boundary stated? | `runtime-boundaries.md`, `active-mvp-scope.md` |
| Where is maintainer handoff status tracked? | `../build/mvp-plan.md` |

## Find the owner document

| Question | Owner |
|---|---|
| Which document owns Core authority, Task state, evidence, residual risk, and non-substitution rules? | `core-model.md` |
| Which document owns the active API method list? | `api/mvp-api.md` |
| Which document owns shared API response branches and envelopes? | `api/schema-core.md` |
| Which document owns method response branch schemas? | `api/schema-core.md` |
| Which document owns public error codes and error precedence? | `api/errors.md` |
| Which document owns storage records or DDL? | `storage-records.md` |
| Which document owns storage effects? | `storage-effects.md` |
| Which document owns method-to-storage effects? | `storage-effects.md` |
| Which document owns security claims and non-claims? | `security.md` |
| Which document owns product terminology? | `glossary.md`, `../../terminology-map.yaml` |
| Which document owns read-only projection authority and freshness boundaries? | `projection-and-templates.md` |
| Which document owns rendered template bodies? | `template-bodies.md` |

## API and schema owners

| Question | Owner |
|---|---|
| What scenario do API examples use? | `api/mvp-api.md`, `../maintain/authoring-guide.md`, `../maintain/checks.md` |
| Can API examples use documentation maintenance as the scenario? | `../maintain/authoring-guide.md` |
| Where are API example checks defined? | `../maintain/checks.md` |
| Where are API example consistency questions routed? | `../maintain/authoring-guide.md`, `../maintain/checks.md`, then the affected method or schema owner |
| Where are field-name consistency questions routed? | `../maintain/authoring-guide.md`, `../maintain/checks.md`, then the affected method, schema, or storage owner |
| Where is the active API method list? | `api/mvp-api.md` |
| Where are exact API method-name values defined? | `api/schema-value-sets.md` |
| Where do method payload field questions go? | the affected [API method owner](#api-method-owners), or the schema owner for shared envelope and nested fields |
| Where is the `harness.status` example `state_version` rule? | `api/method-status.md`, `../maintain/checks.md` |
| What does `harness.prepare_write` return? | `api/method-prepare-write.md` |
| Which schema owns `harness.prepare_write` response branch schemas? | `api/schema-core.md` |
| Which schema owns `harness.prepare_write` state shapes? | `api/schema-state.md` |
| Which schema owns `harness.prepare_write` judgment shapes? | `api/schema-judgment.md` |
| Where is `harness.prepare_write` sensitive approval routed? | `api/method-prepare-write.md` |
| Where is `ToolRejectedResponse` defined? | `api/schema-core.md` |
| Is `STATE_VERSION_CONFLICT` a blocker code? | `api/errors.md` |
| When can `harness.close_task` with `dry_run=true` return something other than `ToolDryRunResponse`? | `api/method-close-task.md` |
| Which document owns `response_kind`, `effect_kind`, and enum-like API values? | `api/schema-value-sets.md` |
| Is `complete` an enum value or the word "full" here? | `../../terminology-map.yaml`, `glossary.md` |
| Where are access classes defined? | `api/schema-value-sets.md` |
| Where are `DryRunSummary`, `PlannedEffect`, and `PlannedBlocker` defined? | `api/schema-core.md` |
| Which document owns guarantee label values? | `api/schema-value-sets.md` |
| Which document owns guarantee semantics? | `security.md` |
| Where is `isolated` defined as a value? | `api/schema-value-sets.md` |
| Where is `isolated` guarantee meaning defined? | `security.md` |
| Which document owns state summary shapes? | `api/schema-state.md` |
| Which document owns artifact reference shapes? | `api/schema-artifacts.md` |
| Which document owns judgment and accepted-risk input shapes? | `api/schema-judgment.md` |

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
| Which document owns Harness Runtime Home separation? | `runtime-boundaries.md` |
| Which document owns local store assumptions and table overview? | `storage-records.md` |
| Where are storage record values defined? | `storage-records.md` |
| Is `CloseReadinessBlocker` a storage row? | `storage-records.md` |
| Does artifact staging create evidence? | `storage-artifacts.md`, `storage-effects.md` |
| Which document owns artifact staging and promotion? | `storage-artifacts.md` |
| Which document owns artifact reference schemas? | `api/schema-artifacts.md` |
| Which document owns staged-handle validation and body-read eligibility? | `storage-artifacts.md` |
| Which document owns idempotency, state clocks, locks, and migrations? | `storage-versioning.md` |

## Security and runtime owners

| Question | Owner |
|---|---|
| Does the current MVP provide OS sandboxing? | `security.md` |
| Which document owns `isolated` guarantee semantics? | `security.md` |
| Which document owns guarantee semantics? | `security.md` |
| Which document owns Product Repository, Harness Server, and Harness Runtime Home separation? | `runtime-boundaries.md` |
| Which document owns local connector behavior and capability context? | `agent-integration.md` |
| Which document owns verified surface context? | `agent-integration.md` |
| Which document owns verified guarantee boundaries? | `security.md` |
| Which document owns CLI, IDE/editor, chat, and local MCP recipes? | `../use/surface-recipes.md` |
| Which document owns public security-related error mapping? | `api/errors.md` |

## User judgment and close-readiness owners

| Question | Owner |
|---|---|
| Which document owns user-owned judgment meaning? | `core-model.md` |
| Which document owns user judgment prompt behavior? | `api/method-user-judgment.md`, `core-model.md` |
| Which document owns user judgment schemas? | `api/schema-judgment.md` |
| Which document owns sensitive-action approval meaning? | `core-model.md` |
| Which document owns sensitive-action approval schemas? | `api/schema-judgment.md` |
| Which document owns sensitive-action security semantics? | `security.md` |
| Which document owns close readiness and close honesty meaning? | `core-model.md` |
| Which document owns `harness.close_task` behavior? | `api/method-close-task.md` |
| Which document owns close-readiness blocker shape? | `api/schema-state.md` |
| Which document owns close error routing? | `api/errors.md` |
| Which document owns final acceptance and residual-risk boundaries? | `core-model.md` |
| Which document owns accepted-risk schemas? | `api/schema-judgment.md` |
| Which document owns accepted-risk values? | `api/schema-value-sets.md` |
| Which document owns compact evidence summary meaning? | `core-model.md` |
| Which document owns compact evidence summary shape? | `api/schema-state.md` |

## Later and maintenance owners

| Question | Owner |
|---|---|
| Where should later candidates be documented? | `../later/index.md` |
| Where are later security and assurance candidates documented? | `../later/security-and-assurance.md` |
| Where are later artifact and evidence candidates documented? | `../later/artifacts-and-evidence.md` |
| Where are later connector and surface candidates documented? | `../later/connectors-and-surfaces.md` |
| Where are later policy and conformance candidates documented? | `../later/policy-and-conformance.md` |
| Where are later workflow and collaboration candidates documented? | `../later/workflow-and-collaboration.md` |
| Is a later candidate an active requirement? | `active-mvp-scope.md` |
| What does promotion-time owner update mean? | `glossary.md` |
| What must change before a later candidate becomes active? | `../later/index.md` |
| How should "Full close-readiness evaluation order" be written in Korean? | `../maintain/translation-guide.md` |
| How should "close readiness" be written in Korean? | `../../terminology-map.yaml` |
| Where is Korean terminology controlled? | `../../terminology-map.yaml` |
| Where are documentation authoring rules? | `../maintain/authoring-guide.md` |
| Where is the large-table authoring rule defined? | `../maintain/authoring-guide.md`, `../maintain/checks.md` |
| When should a long Markdown table be split? | `../maintain/authoring-guide.md`, `../maintain/checks.md` |
| When should a dense reference paragraph be split? | `../maintain/authoring-guide.md`, `../maintain/checks.md` |
| Where are documentation checks? | `../maintain/checks.md` |
| Where is retrieval or route metadata maintained? | `../../doc-index.yaml` |
| Which document should an agent read first? | `../../../AGENTS.md`, `../../doc-index.yaml` |
