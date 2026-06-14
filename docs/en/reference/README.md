# Reference index

Use this index to choose the owner document for a Harness reference question. This README is route-only: it points to owners and does not define term meanings, terminology metadata, API values, enum values, reserved/profile-gated values, status values, schema fields, API contracts, schemas, storage effects, security guarantees, or scope.

Key routes:

- [Glossary](glossary.md): compact human-readable guide for selected core terms.
- [`docs/terminology-map.yaml`](../../terminology-map.yaml): complete structured terminology metadata, bilingual wording controls, and identifier controls.
- [Translation Guide](../maintain/translation-guide.md): English/Korean wording and Korean style guidance.
- [API Value Sets](api/schema-value-sets.md): API values, enum values, reserved/profile-gated values, and status values.
- API schema rows below: schema fields; method-specific payload fields route through [API Methods](api/methods.md).

## Product and system owners

| Topic | Owner |
|---|---|
| Scope questions | [`scope.md`](scope.md) |
| Core authority, product concepts, user-owned judgment, and close-readiness authority concepts | [`core-model.md`](core-model.md) |
| Runtime and repository boundaries | [`runtime-boundaries.md`](runtime-boundaries.md) |
| Security wording and guarantee semantics | [`security.md`](security.md) |
| Implementation entry route | [`../build/implementation-guide.md`](../build/implementation-guide.md) |

## API and schema owners

| Topic | Owner |
|---|---|
| Public API method list and method routing | [`api/methods.md`](api/methods.md) |
| Shared request envelopes and response branches | [`api/schema-core.md`](api/schema-core.md) |
| State schemas and `CloseReadinessBlocker` shape | [`api/schema-state.md`](api/schema-state.md) |
| Artifact reference shapes | [`api/schema-artifacts.md`](api/schema-artifacts.md) |
| User judgment and sensitive-action schemas | [`api/schema-judgment.md`](api/schema-judgment.md) |
| API values, enum values, reserved/profile-gated values, status values, and blocker category values | [`api/schema-value-sets.md`](api/schema-value-sets.md) |
| API error family index | [`api/errors.md`](api/errors.md) |
| Public `ErrorCode` identifiers and meanings | [`api/error-codes.md`](api/error-codes.md) |
| API error selection precedence | [`api/error-precedence.md`](api/error-precedence.md) |
| API response branch routing | [`api/error-routing.md`](api/error-routing.md) |
| Close-readiness blocker/API response boundary routing | [`api/blocker-routing.md`](api/blocker-routing.md) |
| Machine-readable `ToolError.details` and helper values | [`api/error-details.md`](api/error-details.md) |

## Storage owners

| Topic | Owner |
|---|---|
| Storage family route | [`storage.md`](storage.md) |
| Storage records | [`storage-records.md`](storage-records.md) |
| Storage effects | [`storage-effects.md`](storage-effects.md) |
| Artifact storage | [`storage-artifacts.md`](storage-artifacts.md) |
| State clocks and versioning | [`storage-versioning.md`](storage-versioning.md) |
| Runtime home separation | [`runtime-boundaries.md`](runtime-boundaries.md) |

## Surface, projection, and quality owners

| Topic | Owner |
|---|---|
| Agent integration and current surface context | [`agent-integration.md`](agent-integration.md) |
| Surface usage recipes | [`../use/surface-recipes.md`](../use/surface-recipes.md) |
| Authority vs projected/status/template views | [`projection-and-templates.md`](projection-and-templates.md) |
| Display-facing template bodies and labels | [`template-bodies.md`](template-bodies.md) |
| Conformance reference | [`conformance.md`](conformance.md) |
| Design-quality finding semantics and owner-boundary rules | [`design-quality.md`](design-quality.md) |

## User judgment and close-readiness owners

| Topic | Owner |
|---|---|
| User-owned judgment meaning | [`core-model.md`](core-model.md) |
| User judgment methods | [`api/method-user-judgment.md`](api/method-user-judgment.md) |
| User judgment schemas | [`api/schema-judgment.md`](api/schema-judgment.md) |
| Close-readiness authority concepts | [`core-model.md`](core-model.md) |
| `harness.close_task` method behavior | [`api/method-close-task.md`](api/method-close-task.md) |
| `CloseReadinessBlocker` shape | [`api/schema-state.md`](api/schema-state.md) |
| Blocker category values | [`api/schema-value-sets.md`](api/schema-value-sets.md) |
| Close-readiness blocker/API response boundary routing | [`api/blocker-routing.md`](api/blocker-routing.md) |

## Maintenance and metadata

| Need | Route |
|---|---|
| Repository editing rules | [`../../../AGENTS.md`](../../../AGENTS.md) |
| Machine-readable owner routing | [`../../doc-index.yaml`](../../doc-index.yaml) |
| Documentation authoring rules | [`../maintain/authoring-guide.md`](../maintain/authoring-guide.md) |
| Documentation checks index | [`../maintain/checks.md`](../maintain/checks.md) |
| English/Korean wording and Korean style guidance | [`../maintain/translation-guide.md`](../maintain/translation-guide.md) |
| Compact human-readable guide for selected core terms | [`glossary.md`](glossary.md) |
| Complete structured terminology metadata | [`../../terminology-map.yaml`](../../terminology-map.yaml) |
