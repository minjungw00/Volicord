# Glossary

This glossary is the compact human-readable term meaning guide for Harness documentation. Use it to understand a term and find the focused owner for that term.

It is not the full reference owner index. For owner lookup by topic, use the [Reference Index](README.md). For exact machine-readable routing by `doc_id`, use [`docs/doc-index.yaml`](../../doc-index.yaml).

Structured terminology metadata, identifier-preservation controls, and Korean mixed-language controls live in [docs/terminology-map.yaml](../../terminology-map.yaml). Exact API behavior, schemas, storage effects, security guarantees, close-readiness behavior, and error routing live in the linked owner documents.

## Terms

| Term | Korean term | Short meaning | Primary owner |
|---|---|---|---|
| Harness | 하네스 | The local work-authority server for AI-assisted product work. | [Scope](scope.md) |
| `Product Repository` | `Product Repository`; 제품 저장소 | The user's project workspace, separate from Harness runtime state. | [Runtime Boundaries](runtime-boundaries.md) |
| `Harness Runtime Home` | `Harness Runtime Home`; 런타임 홈 | The operational data space for Harness records and artifacts. | [Runtime Boundaries](runtime-boundaries.md) |
| documentation | 문서 | Maintained source material, separate from runtime output, implementation, and acceptance state. | [Authoring Guide](../maintain/authoring-guide.md) |
| semantic skeleton | 의미 골격 | The planned meaning-unit structure for an important Reference section. | [Authoring Guide](../maintain/authoring-guide.md) |
| baseline scope | 기준 범위 | The stable support boundary documented for Harness. | [Scope](scope.md) |
| supported scope | 지원 범위 | A capability or behavior documented as supported. | [Scope](scope.md) |
| supported behavior | 지원 동작 | Behavior documented as supported by Scope and the affected owner. | [Scope](scope.md) |
| supported API method | 지원되는 API 메서드 | A public API method documented as supported. | [API Methods](api/methods.md) |
| supported API value | 지원되는 API 값 | An API value documented as supported, not only reserved or named. | [API Value Sets](api/schema-value-sets.md) |
| out-of-scope capability | 지원 범위 밖 기능 | A deferred capability outside the baseline support boundary. | [Scope](scope.md) |
| evidence collection workflow | 증거 수집 흐름 | Evidence-workflow wording whose support status belongs to Scope. | [Scope](scope.md) |
| expanded or additional evidence collection workflows | 확장 또는 추가 증거 수집 흐름 | An out-of-scope evidence-workflow family. | [Scope](scope.md) |
| owner document | 담당 문서 | The canonical document that defines a term, product concept, or contract. | [Authoring Guide](../maintain/authoring-guide.md) |
| owner contract | 담당 계약 | The contract defined by the relevant owner document. | [Authoring Guide](../maintain/authoring-guide.md) |
| applicable owner path | 적용되는 담당 경로 | The documentation route to the focused owner for a question or concept. | [Authoring Guide](../maintain/authoring-guide.md) |
| applicable reference | 적용되는 참조 문서 | The reference page that defines the relevant contract. | [Reference Index](README.md) |
| existing owner | 기존 담당 문서 | A canonical owner that already exists and can carry normative meaning. | [Authoring Guide](../maintain/authoring-guide.md) |
| promotion-time owner update | 승격 시점의 담당 문서 갱신 | Owner changes needed when an out-of-scope capability is promoted into support. | [Scope](scope.md) |
| owner placeholder | 담당 문서 자리표시자 | A marker that an out-of-scope capability still needs an owner. | [Authoring Guide](../maintain/authoring-guide.md) |
| `Task` | `Task` | The Harness entity that gathers scope, authority context, judgments, evidence, and close-readiness state. | [Core Model](core-model.md) |
| scope | 범위 | The work or authority boundary attached to a `Task` or Change Unit context. | [Core Model](core-model.md) |
| active scope | 현재 적용 범위 | The scope currently applied inside a `Task` or Change Unit context. | [Core Model](core-model.md) |
| active Change Unit | 현재 적용 Change Unit | The Change Unit currently applied in the authority model. | [Core Model](core-model.md) |
| user-owned judgment | 사용자 소유 판단 | A user-owned decision or assessment recorded without becoming Core-owned fact. | [Core Model](core-model.md) |
| `UserJudgment` | `UserJudgment` | The API schema identifier for user-owned judgment data. | [API Judgment Schemas](api/schema-judgment.md) |
| close readiness | 닫기 준비 상태 | The Core concept for whether a `Task` is ready to close from its current state. | [Core Model](core-model.md) |
| close readiness evaluation | 닫기 준비 상태 평가 | The close-task evaluation term. | [Close-task method](api/method-close-task.md) |
| close task | `Task` 닫기 | The user or API action that attempts to close a `Task`. | [Close-task method](api/method-close-task.md) |
| close task behavior | `Task` 닫기 동작 | The close-task API behavior area. | [Close-task method](api/method-close-task.md) |
| `harness.close_task` | `harness.close_task` | The public API method identifier for close task. | [Close-task method](api/method-close-task.md) |
| close-readiness blocker | 닫기 차단 사유 | A reason surfaced when close readiness cannot proceed. | [API blocker routing](api/blocker-routing.md) |
| `CloseReadinessBlocker` | `CloseReadinessBlocker` | The schema identifier for close-readiness blocker data. | [API State Schemas](api/schema-state.md) |
| blocker category | 차단 사유 범주 | The category concept for close-readiness blockers. | [API Value Sets](api/schema-value-sets.md) |
| blocker | 차단 사유 | A general prose term for a blocking reason. | [Terminology Map](../../terminology-map.yaml) |
| complete intent | `complete` | The `complete` API value, distinct from ordinary "full" or "entire". | [API Value Sets](api/schema-value-sets.md) |
| full evaluation order | 전체 평가 순서 | Ordinary prose for the full or entire evaluation order, distinct from the `complete` API value. | [Terminology Map](../../terminology-map.yaml) |
| artifact | 아티팩트 | Work material referenced or staged through Harness artifact concepts. | [API Artifact Schemas](api/schema-artifacts.md) |
| evidence | 증거 | Recorded support for claims, verification results, or user judgment context. | [Core Model](core-model.md) |
| `ArtifactRef` | `ArtifactRef` | The schema identifier for a persisted artifact reference. | [API Artifact Schemas](api/schema-artifacts.md) |
| `ArtifactInput` | `ArtifactInput` | The schema identifier for artifact input data. | [API Artifact Schemas](api/schema-artifacts.md) |
| `StagedArtifactHandle` | `StagedArtifactHandle` | The identifier for a staged artifact handle. | [API Artifact Schemas](api/schema-artifacts.md) |
| projection | 상태 보기 | A read-only state view. | [Projection Authority Reference](projection-and-templates.md) |
| `Projection` | `Projection` | The exact product label for the read-only state-view concept. | [Projection Authority Reference](projection-and-templates.md) |
| surface | 접점 | An integration or interaction boundary where context appears. | [Agent Integration](agent-integration.md) |
| `surface_id` | `surface_id` | The exact identifier for a surface. | [Agent Integration](agent-integration.md) |
| active surface context | 현재 적용 접점 맥락 | The current surface context for a request or interaction. | [Agent Integration](agent-integration.md) |
| `state_version` | `state_version` | The state-clock identifier for stored project state. | [Storage Versioning](storage-versioning.md) |
| runtime | 런타임 | The operational Harness execution and data context. | [Runtime Boundaries](runtime-boundaries.md) |
| `Write Authorization` | 쓰기 권한 부여 | The exact product label for the Harness write-authorization concept. | [Core Model](core-model.md) |
| sensitive approval | 민감 동작 승인 | User approval for a sensitive action, separate from `Write Authorization`. | [Core Model](core-model.md) |
| access class | 접근 등급 | A value category for access context. | [API Value Sets](api/schema-value-sets.md) |
| baseline guarantee | 기준 범위 보장 | Security terminology for a baseline-scope guarantee. | [Security](security.md) |
| cooperative guarantee | 협력형 보장 | Security terminology for a cooperative guarantee type. | [Security](security.md) |
| detective guarantee | 탐지형 보장 | Security terminology for a detective guarantee type. | [Security](security.md) |
| design-quality owner boundary | 설계 품질 담당 경계 | The boundary that routes design-quality observations to the relevant owner. | [Design Quality](design-quality.md) |
| reserved value | 예약된 값 | A value reserved as vocabulary or surface area without baseline behavior by itself. | [Scope](scope.md) |
| profile-gated value | 프로필 조건부 값 | A value available only when the documented profile or gate supports it. | [Scope](scope.md) |
| `ErrorCode` | `ErrorCode` | The public API error-code identifier. | [API error codes](api/error-codes.md) |
| error code meanings | 공개 오류 코드 의미 | The meaning area for public API error codes. | [API error codes](api/error-codes.md) |
| error precedence | 오류 우선순위 | The API error selection and ordering area. | [API error precedence](api/error-precedence.md) |
| error routing | 오류 처리 경로 | The API response-branch routing area. | [API error routing](api/error-routing.md) |
| blocker routing | 차단 사유 처리 경로 | The boundary between close-readiness blockers and API response branches. | [API blocker routing](api/blocker-routing.md) |
| error/blocker boundary | 오류와 차단 사유의 경계 | The distinction between public API errors and close-readiness blocker data. | [API blocker routing](api/blocker-routing.md) |
| public error as blocker | 공개 오류 코드가 차단 사유로 표현되는 경우 | Boundary wording for public error-code wording in blocker data. | [API blocker routing](api/blocker-routing.md) |
| `ToolError.details` | `ToolError.details` | The machine-readable error details field. | [API error details](api/error-details.md) |
| error detail helper values | 오류 세부사항 보조 값 | Helper values under machine-readable error details. | [API error details](api/error-details.md) |
| dry-run | dry-run 미리보기 | API preview mode using `dry_run`. | [API Core Schemas](api/schema-core.md) |
| dry-run preview routing | dry-run 미리보기 처리 경로 | The routing term for `dry_run` preview responses. | [API error routing](api/error-routing.md) |
| blocked result | 차단 결과 | An API result branch that reports a block. | [API error routing](api/error-routing.md) |
| rejected response | 거부 응답 | An API response for a request rejected before an operation proceeds. | [API error routing](api/error-routing.md) |
| migration | 마이그레이션 | A technical migration of schema, storage, data, or documentation. | [Storage Versioning](storage-versioning.md) |
| lifecycle | 생명주기 | The stages of an entity or artifact over time. | [Core Model](core-model.md) |
