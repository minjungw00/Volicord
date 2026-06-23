# Glossary

This glossary is a compact human-readable guide to selected core Volicord terms. Use it to understand major concepts and find the focused primary owner for each included term.

Complete structured terminology metadata lives in [`docs/terminology-map.yaml`](../../terminology-map.yaml). This glossary is only a curated reader-facing subset.

It repeats selected:

- term labels
- Korean terms
- compact meanings
- focused primary owners

Preferred expressions, avoid expressions, identifier-preservation controls, and adjacent references stay in the terminology map.

For owner lookup by topic, use the [Reference Index](README.md). For exact machine-readable routing by `doc_id`, use [`docs/doc-index.yaml`](../../doc-index.yaml).

Contract detail stays in the focused owner documents. Translation and style rules stay in the [Translation Policy](../maintain/translation-policy.md).

## Terms

| Term | Korean term | Short meaning | Primary owner |
|---|---|---|---|
| Volicord | Volicord | The local work-authority product/system for AI-assisted product work. | [Scope](scope.md) |
| Core | Core | The local authority record for Volicord state. | [Core Model](core-model.md) |
| Volicord implementation | Volicord implementation | The server implementation set maintained by this repository, including source-level crates, executable roles, tests, documentation, validation tooling, and repository configuration. Not a synonym for Volicord as a whole. | [Runtime Boundaries](runtime-boundaries.md) |
| `Product Repository` | 제품 저장소 | The user's project workspace and product files, separate from Volicord runtime state. | [Runtime Boundaries](runtime-boundaries.md) |
| `Volicord Runtime Home` | 런타임 홈 | The local runtime data space for Volicord operational data, as storage/runtime owners define it. | [Runtime Boundaries](runtime-boundaries.md) |
| runtime | 런타임 | The operational Volicord execution and data context. | [Runtime Boundaries](runtime-boundaries.md) |
| baseline scope | 기준 범위 | The stable support boundary documented for Volicord. | [Scope](scope.md) |
| out-of-scope capability | 지원 범위 밖 기능 | A deferred capability outside the baseline support boundary. | [Scope](scope.md) |
| owner document | 담당 문서 | The canonical document that defines a term, product concept, or contract. | [Documentation Policy](../maintain/documentation-policy.md) |
| applicable owner path | 적용되는 담당 경로 | The documentation route to the focused owner for a question or concept. | [Documentation Policy](../maintain/documentation-policy.md) |
| `Task` | `Task` | The Volicord entity that gathers scope, authority context, judgments, evidence, and close-readiness state. | [Core Model](core-model.md) |
| scope | 범위 | The work or authority boundary attached to a `Task` or Change Unit context. | [Core Model](core-model.md) |
| current scope | 현재 적용 범위 | The scope currently applied inside a `Task` or Change Unit context. | [Core Model](core-model.md) |
| current Change Unit | 현재 적용 Change Unit | The Change Unit currently applied in the authority model. | [Core Model](core-model.md) |
| user-owned judgment | 사용자 소유 판단 | A user decision or assessment recorded without becoming Core-owned fact. | [Core Model](core-model.md) |
| evidence | 증거 | Recorded support for a specific claim at a specific scope. | [Core Model](core-model.md) |
| verification criteria | 검증 기준 | User-visible criteria for checking work. | [Core Model](core-model.md) |
| artifact | 아티팩트 | Work material referenced or staged through Volicord artifact concepts. | [API Artifact Schemas](api/schema-artifacts.md) |
| `Write Authorization` | 쓰기 권한 부여 | The exact Volicord product label for Core authority around one compatible product-file write attempt. | [Core Model](core-model.md) |
| write approval | 쓰기 승인 | Ordinary user approval, or prose about approving a write. Separate from `Write Authorization`. | [Core Model](core-model.md) |
| sensitive-action approval | 민감 동작 승인 | User approval for a named sensitive step, separate from `Write Authorization` and final acceptance. | [Core Model](core-model.md) |
| final acceptance | 최종 수락 | A user-owned judgment about whether the visible close basis is acceptable. | [Core Model](core-model.md) |
| residual-risk acceptance | 잔여 위험 수락 | A user-owned judgment about a named visible residual risk. | [Core Model](core-model.md) |
| close readiness | 닫기 준비 상태 | The Core concept for whether a `Task` is ready to close from its current state. | [Core Model](core-model.md) |
| close-readiness blocker | 닫기 차단 사유 | A close-relevant reason surfaced when close readiness cannot proceed. | [API blocker routing](api/blocker-routing.md) |
| `Projection` | 상태 보기 | The exact product label for a read-only state view. Projection output is display, not Core authority. | [Projection Authority Reference](projection-and-templates.md) |
| `Agent Integration Profile` | 에이전트 통합 프로필 | The durable registry identity for one coding-agent integration and its bound surface context. | [Agent Integration](agent-integration.md) |
| integration project membership | 통합 프로젝트 멤버십 | The explicit allowlist relation between an Agent Integration Profile and registered projects. | [Agent Integration](agent-integration.md) |
| `Host Installation` | 호스트 설치 | Managed host setup inventory for a coding-agent integration. Not proof that the external host trusted or loaded the server. | [Agent Integration](agent-integration.md) |
| `volicord.list_projects` | `volicord.list_projects` | MCP adapter utility for listing projects allowed for the bound integration. Not a public Core API method. | [MCP Transport](mcp-transport.md) |
| surface | 접점 | An integration or interaction boundary where context appears. | [Agent Integration](agent-integration.md) |
| access class | 접근 등급 | A value category for verified surface and access context. | [API Value Sets](api/schema-value-sets.md) |
| baseline guarantee | 기준 범위 보장 | Security wording for a guarantee supported in the baseline scope. | [Security](security.md) |
| `ErrorCode` | 공개 오류 코드 | The public API error-code identifier. | [API error codes](api/error-codes.md) |
