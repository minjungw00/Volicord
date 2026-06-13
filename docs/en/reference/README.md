# Reference index

Use this index to choose the owner document for a Harness reference question. This README is a route: it points to owners and does not define API contracts, schemas, storage effects, security guarantees, or scope.

For machine-readable routing by `doc_id`, use [`docs/doc-index.yaml`](../../doc-index.yaml).

## Product and system owners

| Topic | Owner |
|---|---|
| Scope questions | [`scope.md`](scope.md) |
| Core authority, product concepts, user-owned judgment, and close readiness | [`core-model.md`](core-model.md) |
| Runtime and repository boundaries | [`runtime-boundaries.md`](runtime-boundaries.md) |
| Security wording and guarantee semantics | [`security.md`](security.md) |
| Product terminology | [`glossary.md`](glossary.md), [`docs/terminology-map.yaml`](../../terminology-map.yaml) |
| Implementation entry route | [`../build/implementation-guide.md`](../build/implementation-guide.md) |

## API and schema owners

| Topic | Owner |
|---|---|
| Public API method list and method routing | [`api/methods.md`](api/methods.md) |
| Shared request envelopes and response branches | [`api/schema-core.md`](api/schema-core.md) |
| State and close-readiness state shapes | [`api/schema-state.md`](api/schema-state.md) |
| Artifact reference shapes | [`api/schema-artifacts.md`](api/schema-artifacts.md) |
| User judgment and sensitive-action schemas | [`api/schema-judgment.md`](api/schema-judgment.md) |
| API value sets | [`api/schema-value-sets.md`](api/schema-value-sets.md) |
| API error family route | [`api/errors.md`](api/errors.md) |
| Public `ErrorCode` identifiers and meanings | [`api/error-codes.md`](api/error-codes.md) |
| API error precedence and state conflict behavior | [`api/error-precedence.md`](api/error-precedence.md) |
| API error versus blocker routing | [`api/error-routing.md`](api/error-routing.md) |
| Machine-readable `ToolError.details` | [`api/error-details.md`](api/error-details.md) |

## Storage owners

| Topic | Owner |
|---|---|
| Storage family route | [`storage.md`](storage.md) |
| Storage records | [`storage-records.md`](storage-records.md) |
| Storage effects | [`storage-effects.md`](storage-effects.md) |
| Artifact storage | [`storage-artifacts.md`](storage-artifacts.md) |
| State clocks, versioning, and migrations | [`storage-versioning.md`](storage-versioning.md) |
| Runtime home separation | [`runtime-boundaries.md`](runtime-boundaries.md) |

## Surface, projection, and quality owners

| Topic | Owner |
|---|---|
| Agent integration and current surface context | [`agent-integration.md`](agent-integration.md) |
| Surface usage recipes | [`../use/surface-recipes.md`](../use/surface-recipes.md) |
| Authority vs projected/status/template views | [`projection-and-templates.md`](projection-and-templates.md) |
| Template bodies, labels, and display wording | [`template-bodies.md`](template-bodies.md) |
| Conformance reference | [`conformance.md`](conformance.md) |
| Design quality reference | [`design-quality.md`](design-quality.md) |

## User judgment and close-readiness owners

| Topic | Owner |
|---|---|
| User-owned judgment meaning | [`core-model.md`](core-model.md) |
| User judgment methods | [`api/method-user-judgment.md`](api/method-user-judgment.md) |
| User judgment schemas | [`api/schema-judgment.md`](api/schema-judgment.md) |
| Close-readiness meaning | [`core-model.md`](core-model.md) |
| Close task method | [`api/method-close-task.md`](api/method-close-task.md) |
| Close-readiness state shapes | [`api/schema-state.md`](api/schema-state.md) |
| Close error routing | [`api/error-routing.md`](api/error-routing.md) |

## Maintenance and metadata

| Need | Route |
|---|---|
| Repository editing rules | [`../../../AGENTS.md`](../../../AGENTS.md) |
| Machine-readable owner routing | [`../../doc-index.yaml`](../../doc-index.yaml) |
| Documentation authoring rules | [`../maintain/authoring-guide.md`](../maintain/authoring-guide.md) |
| Documentation checks index | [`../maintain/checks.md`](../maintain/checks.md) |
| Translation guidance | [`../maintain/translation-guide.md`](../maintain/translation-guide.md) |
| Bilingual terminology controls | [`../../terminology-map.yaml`](../../terminology-map.yaml) |
