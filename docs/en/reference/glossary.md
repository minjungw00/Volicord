# Glossary

This glossary is a compact human-readable guide to selected core Harness terms. Use it to understand major concepts and find the focused primary owner for each included term. It is a curated reader-facing subset.

Complete structured terminology metadata lives in [`docs/terminology-map.yaml`](../../terminology-map.yaml), including term records, bilingual wording controls, identifier-preservation rules, primary owner paths, and related-reference metadata. Each glossary row follows the matching terminology-map entry for its focused primary owner; adjacent references stay in the map's `related_references`. For owner lookup by topic, use the [Reference Index](README.md). For exact machine-readable routing by `doc_id`, use [`docs/doc-index.yaml`](../../doc-index.yaml).

Contract detail stays in the focused owner documents, not in this table.

## Terms

| Term | Korean term | Short meaning | Primary owner |
|---|---|---|---|
| Harness | 하네스 | The local work-authority server for AI-assisted product work. | [Scope](scope.md) |
| `Product Repository` | 제품 저장소 | The user's project workspace, separate from Harness runtime state. | [Runtime Boundaries](runtime-boundaries.md) |
| `Harness Runtime Home` | 런타임 홈 | The operational data space for Harness records and artifacts. | [Runtime Boundaries](runtime-boundaries.md) |
| runtime | 런타임 | The operational Harness execution and data context. | [Runtime Boundaries](runtime-boundaries.md) |
| baseline scope | 기준 범위 | The stable support boundary documented for Harness. | [Scope](scope.md) |
| out-of-scope capability | 지원 범위 밖 기능 | A deferred capability outside the baseline support boundary. | [Scope](scope.md) |
| owner document | 담당 문서 | The canonical document that defines a term, product concept, or contract. | [Authoring Guide](../maintain/authoring-guide.md) |
| applicable owner path | 적용되는 담당 경로 | The documentation route to the focused owner for a question or concept. | [Authoring Guide](../maintain/authoring-guide.md) |
| `Task` | `Task` | The Harness entity that gathers scope, authority context, judgments, evidence, and close-readiness state. | [Core Model](core-model.md) |
| scope | 범위 | The work or authority boundary attached to a `Task` or Change Unit context. | [Core Model](core-model.md) |
| active scope | 현재 적용 범위 | The scope currently applied inside a `Task` or Change Unit context. | [Core Model](core-model.md) |
| active Change Unit | 현재 적용 Change Unit | The currently applied Change Unit in the authority model. | [Core Model](core-model.md) |
| user-owned judgment | 사용자 소유 판단 | A user-owned decision or assessment recorded without becoming Core-owned fact. | [Core Model](core-model.md) |
| evidence | 증거 | Recorded support for a specific claim at a specific scope. | [Core Model](core-model.md) |
| artifact | 아티팩트 | Work material referenced or staged through Harness artifact concepts. | [API Artifact Schemas](api/schema-artifacts.md) |
| `Write Authorization` | 쓰기 권한 부여 | The Harness authorization concept for one compatible product-file write attempt. | [Core Model](core-model.md) |
| sensitive-action approval | 민감 동작 승인 | User approval for a named sensitive step, separate from `Write Authorization`. | [Core Model](core-model.md) |
| close readiness | 닫기 준비 상태 | The Core concept for whether a `Task` is ready to close from its current state. | [Core Model](core-model.md) |
| close-readiness blocker | 닫기 차단 사유 | A reason surfaced when close readiness cannot proceed. | [API blocker routing](api/blocker-routing.md) |
| `Projection` | 상태 보기 | The read-only state-view concept and exact product label. | [Projection Authority Reference](projection-and-templates.md) |
| surface | 접점 | An integration or interaction boundary where context appears. | [Agent Integration](agent-integration.md) |
| access class | 접근 등급 | A value category for verified surface and access context. | [API Value Sets](api/schema-value-sets.md) |
| baseline guarantee | 기준 범위 보장 | Security wording for a guarantee supported in the baseline scope. | [Security](security.md) |
| `ErrorCode` | 공개 오류 코드 | The public API error-code identifier. | [API error codes](api/error-codes.md) |
