# Conformance reference

## Boundary

This reference defines stable conformance scenario semantics and criteria. A
conformance scenario names one observable behavior that can be checked against
facts owned by the API, storage, security, scope, Core, artifact, and Agent
Connection references.

This document owns:

- `scenario_id` naming rules
- scenario-level expected behavior
- assertion authority boundaries
- the relationship between criteria, owner documents, examples, tutorials, and
  method-local API examples

It does not define API branches, request shapes, storage effects, artifact
promotion, security guarantees, close-readiness behavior, method-reference
examples, or implementation structure. Those facts remain with the linked
owners.

For the baseline boundary, see [Scope](scope.md). For public terms, see
[Glossary](glossary.md). Complete structured terminology lives in
[`docs/terminology-map.yaml`](../../terminology-map.yaml).

<a id="surface-stability"></a>
## Surface Stability

Use the [canonical stability
vocabulary](../maintain/documentation-policy.md#surface-stability-labels).

| Surface | Stability | Meaning |
|---|---|---|
| Scenario semantics, `scenario_id` rules, expected behavior, assertion authority, and owner-link requirements | `stable` | Criteria for owner-defined behavior |
| Runner summaries, rendered views, status wording, maintenance labels, and generated reports | `diagnostic` | Not assertion authority unless a focused owner defines the fact |

<a id="conformance-item-summary"></a>
<a id="what-conformance-means"></a>
<a id="scenario-semantics"></a>
## Scenario semantics

A scenario names one baseline behavior criterion or one clearly routed owner
boundary. It contains a `scenario_id`, authority context, one action, expected
behavior, owner links, and an assertion boundary.

A scenario may say what a conforming result preserves, rejects, exposes, or
leaves unchanged. It must not redefine a neighboring API, storage, security,
scope, close-readiness, artifact, or connection contract.

Conformance compares an owner-defined action with owner-defined state and
owner-defined non-effects. Scenario prose, agent summaries, rendered views,
metrics, projections, and maintenance labels are not assertion authority by
themselves.

<a id="scenario-id-rules"></a>
### Scenario ID rules

- Use `BASELINE-*` IDs for baseline behavior.
- Name observable behavior, not a project phase, review stage, queue, runner,
  date, or implementation status.
- Keep an ID stable while its expected behavior stays stable.
- Rename an ID only when its meaning changes. Update same-page anchors and
  links in the same change.

<a id="expected-behavior"></a>
### Expected behavior

Expected behavior is the stable outcome a conforming implementation or check
must satisfy. This page states only the scenario-level outcome. Exact request
fields, response branches, storage effects, error precedence, guarantee levels,
and close-readiness details remain with their owners.

If a summary here conflicts with an owner, the owner wins. Correct this page;
do not implement the conflicting summary.

<a id="criteria-vs-examples-and-tutorials"></a>
## Criteria, examples, and tutorials

Examples and tutorials can help readers recognize a scenario, but they do not
create authority records, API branches, storage effects, security guarantees,
close results, acceptance evidence, or residual-risk acceptance.

Cross-method scenarios may define scenario-level criteria here. They must not
become a shared payload, fixture, or example spine for API method references.
A method example may link to a scenario, but neither side requires the other to
reuse payloads, refs, paths, `state_version`, artifact refs, Run refs, judgment
refs, blocker refs, or response snapshots.

<a id="scenario-criterion-shape"></a>
## Criterion shape

| Part | Required content |
|---|---|
| <a id="criterion-scenario-id"></a>`scenario_id` | Stable behavior identifier that follows the rules above |
| <a id="criterion-authority-context"></a>Authority context | Facts needed before the action, such as `Task`, Change Unit, state version, actor source, owner refs, Core state, storage rows, artifact refs, and capability facts |
| <a id="criterion-action"></a>Action | One public Core, API, or operator request using its owner request schema |
| <a id="criterion-expected-behavior"></a>Expected behavior | Response, state, storage, artifact, blocker, error, guarantee-display, and forbidden-side-effect facts relevant to the criterion |
| <a id="criterion-owner-links"></a>Owner links | Routes to the API, Core, storage, security, Agent Connection, artifact, or policy owner that defines each exact fact |
| Assertion boundary | The owner-defined facts that may be judged and the required non-effects |

A criterion uses public owner schemas. It must not invent criterion-only enum
values, pseudo-fields, prose-only expectations, localized labels as state, or
out-of-scope-only values.

<a id="assertion-authority"></a>
## Assertion authority

Assertion authority is the narrow set of owner-defined facts a criterion may
judge. These include response facts, Core state, storage effects, artifact
facts, public `ErrorCode` values, structured blockers, guarantee-display facts,
and required absence of forbidden side effects.

| Assertion area | Owner |
|---|---|
| API methods and response branches | [API Methods](api/methods.md) and the linked method owners |
| Common response branches and `dry_run` preview shapes | [API Schema Core](api/schema-core.md) |
| State summaries, blockers, evidence, and close-readiness structures | [API State Schemas](api/schema-state.md) |
| `ArtifactRef`, `ArtifactInput`, and `StagedArtifactHandle` shapes | [API Artifact Schemas](api/schema-artifacts.md) |
| API value sets, including `operation_category` | [API Value Sets](api/schema-value-sets.md) |
| Public errors and precedence | [API error codes](api/error-codes.md) and [API error precedence](api/error-precedence.md) |
| Storage effects, no-effect branches, and state-version effects | [Storage Effects](storage-effects.md) |
| Artifact staging, promotion, persistence, and body reads | [Artifact Storage](storage-artifacts.md) |
| Security non-claims and guarantee levels | [Security](security.md) |
| Runtime and repository boundaries | [Runtime Boundaries](runtime-boundaries.md) |

<a id="representative-scenario-index"></a>
## Representative scenarios

The following IDs are compact reference criteria. They are not runtime results,
implementation plans, executable scripts, or required API example payloads.

| `scenario_id` | Expected behavior | Owners |
|---|---|---|
| <a id="scenario-baseline-mcp-supported-profile-capability-conformance"></a>`BASELINE-mcp-supported-profile-capability-conformance` | Every production-supported MCP profile satisfies the same applicable initialize, lifecycle, discovery, call-result, schema, and batching scenarios through its declared semantic capabilities. | [MCP Transport](mcp-transport.md#protocol-revision-negotiation); [Agent Connection](agent-connection.md) |
| <a id="scenario-baseline-mcp-semantic-descriptor-conformance"></a>`BASELINE-mcp-semantic-descriptor-conformance` | Every production tool has one deterministic semantic descriptor; its advertised typed examples validate, decode to the exact request type, and select the declared nested tagged-union variants. | [MCP Transport](mcp-transport.md#generated-semantic-schema-catalog); [Testing Strategy](../architecture-guide/testing-strategy.md) |
| <a id="scenario-baseline-mcp-unsupported-revision-rejected"></a>`BASELINE-mcp-unsupported-revision-rejected` | An unknown or tracked-but-nonproduction MCP revision is rejected without substituting another profile. | [MCP Transport](mcp-transport.md#protocol-revision-negotiation); [Agent Connection](agent-connection.md) |
| <a id="scenario-baseline-mcp-committed-mutation-recovery-preserves-authority"></a>`BASELINE-mcp-committed-mutation-recovery-preserves-authority` | Bounded recovery after a committed MCP mutation preserves current authority semantics and does not retry the mutation. | [MCP Transport](mcp-transport.md#mutation-authority-receipt-projection); [API Schema Core](api/schema-core.md); [Storage Effects](storage-effects.md) |
| <a id="scenario-baseline-agent-connection-mismatch-blocks-mutation"></a>`BASELINE-agent-connection-mismatch-blocks-mutation` | An Agent Connection mismatch rejects the request before mutation. | [Agent Connection](agent-connection.md); [API error codes](api/error-codes.md); [API error routing](api/error-routing.md); [Security](security.md) |
| <a id="scenario-baseline-verified-agent-connection-allows-owner-mutation"></a>`BASELINE-verified-agent-connection-allows-owner-mutation` | A verified Agent Connection permits mutation only within the applicable owner contract. | [Agent Connection](agent-connection.md); [API method routing](api/methods.md#method-owner-routing-table); [Storage Effects](storage-effects.md) |
| <a id="scenario-baseline-single-operation-category-per-public-request"></a>`BASELINE-single-operation-category-per-public-request` | Each public API request has one request-level `operation_category`. | [API Value Sets](api/schema-value-sets.md); [Agent Connection](agent-connection.md); [Security](security.md) |
| <a id="scenario-baseline-shaping-readiness-gap-blocks-or-asks"></a>`BASELINE-shaping-readiness-gap-blocks-or-asks` | Shaping gaps remain owner-defined blockers or judgment candidates, not separate planning artifacts. | [Core Model](core-model.md); [API State Schemas](api/schema-state.md); [Status method](api/method-status.md); [Request judgment](api/method-request-user-action.md); [Record judgment](api/method-resolve-user-action.md) |
| <a id="scenario-baseline-project-state-version-stale-mutation-rejected"></a>`BASELINE-project-state-version-stale-mutation-rejected` | A stale project-wide state version fails before commit. | [State version conflict](api/error-precedence.md#state-conflict-behavior); [Storage Versioning](storage-versioning.md); [Storage Effects](storage-effects.md) |
| <a id="scenario-baseline-dry-run-pre-commit-failure-rejected"></a>`BASELINE-dry-run-pre-commit-failure-rejected` | `dry_run` does not bypass validation, access, capability, or stale-state rejection. | [API Schema Core](api/schema-core.md); [`dry_run` pre-preview failure](api/error-routing.md#rejected-dry-run-pre-preview-failure); [Storage Effects](storage-effects.md) |
| <a id="scenario-baseline-status-close-blockers-read-only"></a>`BASELINE-status-close-blockers-read-only` | Status and close-check blockers can be read without storage mutation. | [Status method](api/method-status.md); [Close-task method](api/method-close-task.md); [API State Schemas](api/schema-state.md); [Storage Effects](storage-effects.md) |
| <a id="scenario-baseline-sensitive-approval-records-sensitive-action-scope"></a>`BASELINE-sensitive-approval-records-sensitive-action-scope` | Sensitive-action approval stays separate from a write ticket and final acceptance. | [Core Model](core-model.md); [API Judgment Schemas](api/schema-judgment.md); [Security](security.md) |
| <a id="scenario-baseline-prepare-write-requires-compatible-scope-and-approval"></a>`BASELINE-prepare-write-requires-compatible-scope-and-approval` | `prepare_write` is a cooperative product-file compatibility path. | [Prepare-write method](api/method-prepare-write.md); [Core Model](core-model.md); [Security](security.md) |
| <a id="scenario-baseline-write-ticket-attempt-scope-bounded-intent"></a>`BASELINE-write-ticket-attempt-scope-bounded-intent` | `WriteTicketAttemptScope` covers one bounded product-file write intent or one exact approval-bound non-product action under effective `sensitive` control. | [Core Model](core-model.md); [Prepare-write method](api/method-prepare-write.md); [API Judgment Schemas](api/schema-judgment.md) |
| <a id="scenario-baseline-record-run-consumes-write-ticket-once"></a>`BASELINE-record-run-consumes-write-ticket-once` | A compatible Run consumes its matching write-ticket row once. | [Record-run method](api/method-record-run.md); [Storage Effects](storage-effects.md); [Storage Versioning](storage-versioning.md) |
| <a id="scenario-baseline-stage-artifact-transient-handle-only"></a>`BASELINE-stage-artifact-transient-handle-only` | Staging creates only a transient staged handle. | [Stage-artifact method](api/method-stage-artifact.md); [API Artifact Schemas](api/schema-artifacts.md); [Artifact Storage](storage-artifacts.md) |
| <a id="scenario-baseline-record-run-artifact-input-validation-order"></a>`BASELINE-record-run-artifact-input-validation-order` | Run artifact inputs are validated before promotion or linking. | [Record-run method](api/method-record-run.md); [API Artifact Schemas](api/schema-artifacts.md); [Artifact Storage](storage-artifacts.md) |
| <a id="scenario-baseline-record-run-promotes-staged-artifact-to-artifact-ref"></a>`BASELINE-record-run-promotes-staged-artifact-to-artifact-ref` | A compatible Run may promote a staged handle to a persistent `ArtifactRef`. | [Artifact Storage](storage-artifacts.md); [Record-run method](api/method-record-run.md); [Storage Effects](storage-effects.md) |
| <a id="scenario-baseline-record-run-rejects-staged-artifact-actor-source-mismatch"></a>`BASELINE-record-run-rejects-staged-artifact-actor-source-mismatch` | A staged-handle provenance mismatch rejects promotion. | [Artifact Storage](storage-artifacts.md); [API Artifact Schemas](api/schema-artifacts.md); [Artifact-input error details](api/error-details.md#artifact-input-error-reason) |
| <a id="scenario-baseline-record-run-links-existing-artifact-without-registering-bytes"></a>`BASELINE-record-run-links-existing-artifact-without-registering-bytes` | An existing persistent artifact may be linked without registering new bytes. | [API Artifact Schemas](api/schema-artifacts.md); [Artifact Storage](storage-artifacts.md); [Record-run method](api/method-record-run.md) |
| <a id="scenario-baseline-captured-artifact-rejected-in-baseline-scope"></a>`BASELINE-captured-artifact-rejected-in-baseline-scope` | Native or captured artifact sources are not baseline artifact authority. | [Scope](scope.md); [API Artifact Schemas](api/schema-artifacts.md) |
| <a id="scenario-baseline-close-task-complete-stale-state-version-rejected"></a>`BASELINE-close-task-complete-stale-state-version-rejected` | Stale state fails before close-readiness evaluation. | [Close-task method](api/method-close-task.md); [State version conflict](api/error-precedence.md#state-conflict-behavior); [Storage Effects](storage-effects.md) |
| <a id="scenario-baseline-close-task-complete-stale-write-ticket-basis-rejected"></a>`BASELINE-close-task-complete-stale-write-ticket-basis-rejected` | A stale close-relevant write-ticket basis fails before close commit. | [Close-task method](api/method-close-task.md); [State version conflict](api/error-precedence.md#state-conflict-behavior); [State conflict details](api/error-details.md#state-conflict-detail-fields); [Storage Versioning](storage-versioning.md) |
| <a id="scenario-baseline-close-task-blocks-current-write-compatibility"></a>`BASELINE-close-task-blocks-current-write-compatibility` | Close can block on semantic write compatibility. | [Core Model](core-model.md); [Close-task method](api/method-close-task.md); [API State Schemas](api/schema-state.md) |
| <a id="scenario-baseline-close-task-blocks-evidence-insufficient"></a>`BASELINE-close-task-blocks-evidence-insufficient` | Close can block on insufficient required evidence. | [Core Model](core-model.md); [API State Schemas](api/schema-state.md); [Close-task method](api/method-close-task.md); [API blocker routing](api/blocker-routing.md) |
| <a id="scenario-baseline-close-task-blocks-required-artifact-unavailable"></a>`BASELINE-close-task-blocks-required-artifact-unavailable` | Close can block when a required artifact is unavailable. | [API State Schemas](api/schema-state.md); [Artifact Storage](storage-artifacts.md); [Close-task method](api/method-close-task.md); [API blocker routing](api/blocker-routing.md) |
| <a id="scenario-baseline-close-task-blocks-final-acceptance-missing"></a>`BASELINE-close-task-blocks-final-acceptance-missing` | Close can block when compatible final acceptance is missing. | [Core Model](core-model.md); [API Judgment Schemas](api/schema-judgment.md); [Close-task method](api/method-close-task.md) |
| <a id="scenario-baseline-close-task-blocks-visible-unaccepted-residual-risk"></a>`BASELINE-close-task-blocks-visible-unaccepted-residual-risk` | Close can block on visible residual risk without compatible acceptance. | [Core Model](core-model.md); [API Judgment Schemas](api/schema-judgment.md); [API State Schemas](api/schema-state.md) |
| <a id="scenario-baseline-check-close-read-only"></a>`BASELINE-check-close-read-only` | `volicord.check_close` is read-only. | [Close-task method](api/method-close-task.md); [API Schema Core](api/schema-core.md); [Storage Effects](storage-effects.md) |
| <a id="scenario-baseline-close-task-state-effecting-dry-run-preview"></a>`BASELINE-close-task-state-effecting-dry-run-preview` | A state-changing close intent uses a dry-run preview only when it is valid and previewable. | [Close-task method](api/method-close-task.md); [API Schema Core](api/schema-core.md); [Storage Effects](storage-effects.md) |
| <a id="scenario-baseline-close-task-supersede-one-state-version"></a>`BASELINE-close-task-supersede-one-state-version` | `supersede` is a terminal non-completion path with one project-wide state mutation when valid. | [Close-task method](api/method-close-task.md); [Core Model](core-model.md); [Storage Effects](storage-effects.md) |

## Catalog boundary

[Scope](scope.md) owns scenario-family names outside the baseline. Such names
are not scenario scripts, supported API payloads, runner requirements,
implementation tasks, runtime results, or runtime proof.

## Metrics boundary

Metrics are not conformance authority. A metric matters to a criterion only
when an owner defines its source records, freshness boundary, display wording,
and non-substitution rule.

A metric cannot create Core state, satisfy evidence, pass QA or verification,
authorize writes, accept results or residual risk, close work, prove
implementation structure, or replace runtime conformance.
