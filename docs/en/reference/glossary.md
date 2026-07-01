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
| Volicord | Volicord | The local work-authority product/system and authority control plane for AI-assisted product work. | [Getting Started Overview](../getting-started/overview.md) |
| Core | Core | The local authority record for Volicord state. | [Core Model](core-model.md) |
| Volicord implementation | Volicord implementation | The implementation set maintained by this repository, not a synonym for Volicord as a whole. Runtime and location boundary details belong to Runtime Boundaries. | [Runtime Boundaries](runtime-boundaries.md) |
| `Product Repository` | 제품 저장소 | The user's project workspace and product files, separate from Volicord runtime state. | [Runtime Boundaries](runtime-boundaries.md) |
| `Volicord Runtime Home` | 런타임 홈 | The local runtime data space for Volicord operational data, as storage/runtime owners define it. | [Runtime Boundaries](runtime-boundaries.md) |
| `installation_profile` | 설치 프로필 저장 기록 | A Runtime Home registry storage record for selected command paths, default connection mode, metadata, and timestamps; not host trust, user authority, or public API state. | [Storage DDL](storage-ddl.md) |
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
| `Write Check` | 쓰기 확인 | Durable Core-state compatibility record for one proposed product-file change. | [Core Model](core-model.md) |
| write approval | 쓰기 승인 | Ordinary user approval, or prose about approving a write. Separate from `Write Check`. | [Core Model](core-model.md) |
| sensitive-action approval | 민감 동작 승인 | User approval for a named sensitive step, separate from `Write Check` and final acceptance. | [Core Model](core-model.md) |
| final acceptance | 최종 수락 | A user-owned judgment about whether the visible close basis is acceptable. | [Core Model](core-model.md) |
| residual-risk acceptance | 잔여 위험 수락 | A user-owned judgment about a named visible residual risk. | [Core Model](core-model.md) |
| close readiness | 닫기 준비 상태 | The Core concept for whether a `Task` is ready to close from its current state. | [Core Model](core-model.md) |
| close-readiness blocker | 닫기 차단 사유 | A close-relevant reason shown when close readiness cannot proceed. | [API blocker routing](api/blocker-routing.md) |
| `Projection` | 상태 보기 | The exact product label for a read-only state view. Projection output is display, not Core authority. | [Projection Authority Reference](projection-and-templates.md) |
| `Agent Connection` | 에이전트 연결 | The local MCP host connection unit stored with `connection_internal_id`; MCP startup uses `connection_id` as the process-binding argument spelling. | [Agent Connection Reference](agent-connection.md) |
| `connection_internal_id` | 연결 내부 식별자 | The storage primary key for Agent Connection records and Connection Projects membership; not an ordinary user-facing selector. | [Storage DDL](storage-ddl.md) |
| `connection_id` | 연결 프로세스 바인딩 | The MCP process-binding and startup diagnostic spelling for a stored Agent Connection; not a storage primary key or authority token. | [MCP Transport](mcp-transport.md) |
| `project_internal_id` | 프로젝트 내부 식별자 | The storage primary key for registered project records and Connection Projects membership; user-facing flows use repository roots, names, aliases, or public selectors. | [Storage DDL](storage-ddl.md) |
| `project_id` | 프로젝트 진단 필드 | A diagnostic or owner-defined schema field spelling in specific contexts; not the public MCP project selector. | [MCP Transport](mcp-transport.md) |
| `project_selector` | 프로젝트 공개 선택자 | The public MCP project selector returned by `volicord.list_projects` for multi-project selection; not Runtime Home registry identity. | [MCP Transport](mcp-transport.md) |
| connection intent | 연결 의도 | The Agent Connection placement intent: `personal`, `shared`, or `global`. | [Agent Connection Reference](agent-connection.md) |
| `connection.mode` | 연결 모드 | The Agent Connection mode, either `workflow` or `read_only`. | [Agent Connection Reference](agent-connection.md) |
| `Connection Projects` | 연결 프로젝트 | The explicit `project_internal_id` allowlist for an Agent Connection. | [Agent Connection Reference](agent-connection.md) |
| `User Channel` | 사용자 채널 | The local user path for recording authority-bearing user judgments. | [Core Model](core-model.md) |
| `actor_source` | 행위자 출처 | Durable provenance such as `agent_connection:<connection_id>`, `local_user`, or `system`; not a registered connection or user identity proof. | [Core Model](core-model.md) |
| `operation_category` | 작업 범주 | Internal API operation classification: `read`, `agent_workflow`, `user_only`, or `admin_local`. | [Security](security.md) |
| `managed host configuration state` | 관리 호스트 설정 상태 | Managed host setup inventory for an Agent Connection. Not proof that the external host trusted or loaded the server. | [Agent Connection Reference](agent-connection.md) |
| baseline guarantee | 기준 범위 보장 | Security wording for a guarantee supported in the baseline scope. | [Security](security.md) |
| `ErrorCode` | 공개 오류 코드 | The public API error-code identifier. | [API error codes](api/error-codes.md) |
