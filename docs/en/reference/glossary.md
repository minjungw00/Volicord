# Glossary

This document owns official terminology for Harness documentation.
It defines prose meaning, Korean terminology choices, identifier preservation, expressions to avoid, and owner routing for product terms.

It does not define exact schemas, value sets, DDL, storage effects, security mechanisms, API behavior, runtime behavior, or implementation sequencing.

## How to use this glossary

Use the summary table as a compact routing aid. Use the term cards as the editable terminology source for each term.

Use this glossary with [docs/terminology-map.yaml](../../terminology-map.yaml). The glossary gives reader meaning and owner routing.

The terminology map is the machine-readable control file for bilingual term choices, identifier preservation, and mixed-language Korean expressions to avoid.

Preserve exact identifiers in backticks in both languages.

When a card points to a schema, API, storage, security, projection, or runtime contract, follow the owner document instead of copying contract details into the glossary.

## Summary table

| Term | Korean reference term | Primary owner |
|---|---|---|
| Harness | 하네스 | [Scope](scope.md) |
| Product Repository | Product Repository | [Runtime Boundaries](runtime-boundaries.md) |
| Harness Runtime Home | Harness Runtime Home | [Runtime Boundaries](runtime-boundaries.md) |
| documentation | 문서 | [Authoring Guide](../maintain/authoring-guide.md) |
| active MVP | 현재 MVP | [Scope](scope.md) |
| out-of-scope capability | 이후 후보 | [Scope Reference](scope.md) |
| owner document | 담당 문서 | [Authoring Guide](../maintain/authoring-guide.md) |
| current owner | 현재 담당 문서 | [Authoring Guide](../maintain/authoring-guide.md) |
| promotion-time owner update | 승격 시점의 담당 문서 갱신 | [Scope Reference](scope.md) |
| owner placeholder | 담당 문서 자리표시자 | [Authoring Guide](../maintain/authoring-guide.md) |
| `Task` | `Task` | [Core Model](core-model.md) |
| scope | 범위 | [Core Model](core-model.md) |
| user-owned judgment | 사용자 소유 판단 | [Core Model](core-model.md) |
| close readiness | 닫기 준비 상태 | [Core Model](core-model.md) |
| close readiness evaluation | 닫기 준비 상태 평가 | [Close-task method](api/method-close-task.md) |
| close blocker | 닫기 차단 사유 | [Core Model](core-model.md) |
| `CloseReadinessBlocker` | `CloseReadinessBlocker` | [API State Schemas](api/schema-state.md) |
| complete intent | `complete` | [API Value Sets](api/schema-value-sets.md) |
| full evaluation order | 전체 평가 순서 | [Translation Guide](../maintain/translation-guide.md) |
| artifact | 아티팩트 | [API Artifact Schemas](api/schema-artifacts.md) |
| `ArtifactRef` | `ArtifactRef` | [API Artifact Schemas](api/schema-artifacts.md) |
| `StagedArtifactHandle` | `StagedArtifactHandle` | [API Artifact Schemas](api/schema-artifacts.md) |
| projection | 상태 보기 | [Projection Authority Reference](projection-and-templates.md) |
| surface | 접점 | [Agent Integration](agent-integration.md) |
| runtime | 런타임 | [Runtime Boundaries](runtime-boundaries.md) |
| `Write Authorization` | 쓰기 권한 부여 | [Core Model](core-model.md) |
| sensitive approval | 민감 동작 승인 | [Core Model](core-model.md) |
| access class | 접근 등급 | [API Value Sets](api/schema-value-sets.md) |
| active guarantee | 현재 활성 보장 | [Security](security.md) |
| cooperative guarantee | 협력형 보장 | [Security](security.md) |
| detective guarantee | 탐지형 보장 | [Security](security.md) |
| preventive guarantee | 예방형 보장 | [Security](security.md) |
| `isolated` | `isolated` | [Security](security.md) |
| reserved value | 예약된 값 | [Scope](scope.md) |
| profile-gated value | 프로필 조건부 값 | [Scope](scope.md) |
| dry-run | dry-run 미리보기 | [API Schema Core](api/schema-core.md) |
| blocked result | 차단 결과 | [API Errors](api/errors.md) |
| rejected response | 거부 응답 | [API Schema Core](api/schema-core.md) |
| lifecycle | 생명주기 | [Core Model](core-model.md) |

## Terms

### Harness

English:
- Harness

Korean:
- Reference: 하네스
- User-facing: 하네스

Preserve:
- Harness when naming the product

Avoid:
- Treating this documentation repository as a working server.

Owner:
- [Scope](scope.md)
- [Runtime Boundaries](runtime-boundaries.md)

Notes:
- Harness is the planned local work-authority server for AI-assisted product work.

### Product Repository

English:
- Product Repository

Korean:
- Reference: Product Repository
- User-facing: 제품 저장소

Preserve:
- `Product Repository` when naming the boundary

Avoid:
- Treating product files as Harness records.

Owner:
- [Runtime Boundaries](runtime-boundaries.md)

Notes:
- The Product Repository is the user's project workspace, not Harness runtime state.

### Harness Runtime Home

English:
- Harness Runtime Home

Korean:
- Reference: Harness Runtime Home
- User-facing: 런타임 홈

Preserve:
- `Harness Runtime Home` when naming the boundary

Avoid:
- Treating this documentation repository or a Product Repository as a Runtime Home.

Owner:
- [Runtime Boundaries](runtime-boundaries.md)

Notes:
- The Harness Runtime Home is the future operational data space for Harness records and artifacts.

### documentation

English:
- documentation

Korean:
- Reference: 문서
- User-facing: 문서

Preserve:
- File paths and owner labels

Avoid:
- implementation-complete
- runtime-ready
- generated operational record

Owner:
- [Authoring Guide](../maintain/authoring-guide.md)
- [Runtime Boundaries](runtime-boundaries.md)
- [Implementation Guide](../build/implementation-guide.md)

Notes:
- Documentation-only work does not authorize runtime implementation or generated runtime records.

### active MVP

English:
- active MVP
- current MVP

Korean:
- Reference: 현재 MVP
- User-facing: 현재 MVP

Preserve:
- Owner titles and exact value strings

Avoid:
- Treating out-of-scope capabilities or profile-gated values as active requirements.

Owner:
- [Scope](scope.md)
- [API Value Sets](api/schema-value-sets.md)

Notes:
- Active MVP is the active product scope boundary for the first planned local work loop.

### out-of-scope capability

English:
- out-of-scope capability

Korean:
- Reference: 이후 후보
- User-facing: 이후 후보

Preserve:
- Exact owner paths when routing promotion requirements

Avoid:
- Calling deferred material an active MVP requirement.

Owner:
- [Scope Reference](scope.md)
- [Scope](scope.md)

Notes:
- A out-of-scope capability is inactive until the relevant owners promote it.

### owner document

English:
- owner document

Korean:
- Reference: 담당 문서
- User-facing: 담당 문서

Preserve:
- File paths
- Anchors
- `doc_id` values

Avoid:
- secondary source of truth
- copied contract owner

Owner:
- [Authoring Guide](../maintain/authoring-guide.md)
- [Reference Index](README.md)

Notes:
- An owner document is the canonical document allowed to define a product concept, contract, schema family, route, or terminology rule.

### current owner

English:
- current owner
- current canonical owner
- current owner document

Korean:
- Reference: 현재 담당 문서
- User-facing: 현재 담당 문서

Preserve:
- File paths
- Anchors
- `doc_id` values

Avoid:
- Naming an owner placeholder as a current canonical owner.

Owner:
- [Authoring Guide](../maintain/authoring-guide.md)
- [Reference Index](README.md)
- [doc-index.yaml](../../doc-index.yaml)

Notes:
- Use this only when the canonical owner exists now and can be linked as the current source of normative meaning.

### promotion-time owner update

English:
- promotion-time owner update

Korean:
- Reference: 승격 시점의 담당 문서 갱신
- User-facing: 승격 시점의 담당 문서 갱신

Preserve:
- File paths
- Anchors

Avoid:
- Naming a missing owner as if it already exists as a current canonical owner.

Owner:
- [Authoring Guide](../maintain/authoring-guide.md)
- [Scope Reference](scope.md)
- [Scope](scope.md)

Notes:
- Promotion may require creating or designating an owner, then updating active scope, schemas, API behavior, storage, templates, checks, and paired-language docs as applicable.

### owner placeholder

English:
- owner placeholder

Korean:
- Reference: 담당 문서 자리표시자
- User-facing: 담당 문서 자리표시자

Preserve:
- Exact owner-gap wording when routing out-of-scope capabilities

Avoid:
- Routing readers to the placeholder as if it were a current canonical owner.

Owner:
- [Authoring Guide](../maintain/authoring-guide.md)
- [Scope Reference](scope.md)

Notes:
- Use this phrase only to signal that an out-of-scope capability may need an owner created or designated before activation.
- An owner placeholder is not a current owner document.

### `Task`

English:
- `Task`

Korean:
- Reference: `Task`
- User-facing: 작업, when exact entity identity is not needed

Preserve:
- `Task`
- `task_id`
- `active_task_id`

Avoid:
- Translating the identifier.
- Using "task" for unrelated chores when the Harness entity matters.

Owner:
- [Core Model](core-model.md)
- [API State Schemas](api/schema-state.md)
- [API Value Sets](api/schema-value-sets.md)

Notes:
- `Task` is the user-value unit being shaped, executed, blocked, or closed.

### scope

English:
- scope

Korean:
- Reference: 범위
- User-facing: 범위

Preserve:
- `scope`
- `scope_decision`
- `AuthorizedAttemptScope`
- `SensitiveActionScope`

Avoid:
- 스코프
- silent scope expansion
- broad approval

Owner:
- [Core Model](core-model.md)
- [Update-scope method](api/method-update-scope.md)
- [API Judgment Schemas](api/schema-judgment.md)

Notes:
- Scope is the accepted boundary for what the current `Task` or Change Unit covers and excludes.

### user-owned judgment

English:
- user-owned judgment

Korean:
- Reference: 사용자 소유 판단
- User-facing: 사용자 판단

Preserve:
- `user_judgment`
- `UserJudgment`
- `judgment_kind`

Avoid:
- Treating broad approval as acceptance, risk acceptance, scope change, sensitive-action approval, or Write Authorization.

Owner:
- [Core Model](core-model.md)
- [API Judgment Schemas](api/schema-judgment.md)

Notes:
- Harness must ask for or preserve user-owned judgment instead of inferring it.

### close readiness

English:
- close readiness

Korean:
- Reference: 닫기 준비 상태
- User-facing: 닫기 가능 여부

Preserve:
- `CloseReadinessBlocker`

Avoid:
- close 가능성 평가
- 닫기 가능성 평가

Owner:
- [Core Model](core-model.md)
- [Close-task method](api/method-close-task.md)
- [API Errors](api/errors.md)

Notes:
- This is the evaluation concept, not the blocker schema.

### close readiness evaluation

English:
- close readiness evaluation

Korean:
- Reference: 닫기 준비 상태 평가
- User-facing: 닫기 준비 상태 평가

Preserve:
- `harness.close_task`
- `CloseTaskResult`
- `CloseReadinessBlocker`

Avoid:
- close 가능성 평가
- 닫기 가능성 평가

Owner:
- [Core Model](core-model.md)
- [Close-task method](api/method-close-task.md)
- [API Errors](api/errors.md)

Notes:
- This is the owner-path check that derives close readiness and remaining close blockers.

### close blocker

English:
- close blocker

Korean:
- Reference: 닫기 차단 사유
- User-facing: 닫기 차단 사유

Preserve:
- `close_blockers`
- `CloseReadinessBlocker`

Avoid:
- close blocker를 확인한다
- blocker reason

Owner:
- [Core Model](core-model.md)
- [API State Schemas](api/schema-state.md)
- [API Errors](api/errors.md)

Notes:
- Use this for a close-relevant reason that prevents honest close readiness until the owner path addresses it.

### `CloseReadinessBlocker`

English:
- `CloseReadinessBlocker`

Korean:
- Reference: `CloseReadinessBlocker`
- User-facing: 닫기 차단 사유, when not naming the schema

Preserve:
- `CloseReadinessBlocker`
- `CloseReadinessBlocker.code`

Avoid:
- Translating the identifier.
- Using it as a prepare-write reason.
- Using it as the whole close-readiness concept.

Owner:
- [API State Schemas](api/schema-state.md)
- [API Value Sets](api/schema-value-sets.md)
- [API Errors](api/errors.md)

Notes:
- `CloseReadinessBlocker` is the API schema identifier for close-readiness blocking data.

### complete intent

English:
- complete intent
- `complete` when naming the intent value

Korean:
- Reference: `complete`
- User-facing: `complete`

Preserve:
- `complete`
- `intent=complete`

Avoid:
- Preserving `complete` in Korean prose when the meaning is full, entire, or complete evaluation.
- complete 평가
- complete 닫기 준비 상태

Owner:
- [Terminology Map](../../terminology-map.yaml)
- [Close-task method](api/method-close-task.md)
- [API Value Sets](api/schema-value-sets.md)

Notes:
- The prose concept is complete intent; only the value string is `complete`.
- Preserve `complete` only when it is an enum value or explicit identifier.
- For `complete` enum-versus-full questions, use the Terminology Map and this glossary first. Open API Value Sets only for exact value-name contracts.

### full evaluation order

English:
- full evaluation order
- entire evaluation order

Korean:
- Reference: 전체 평가 순서; in close-readiness context, 전체 닫기 준비 상태 평가 순서
- User-facing: 전체 평가 순서; in close-readiness context, 전체 닫기 준비 상태 평가 순서

Preserve:
- None specific

Avoid:
- `complete` 평가 순서
- complete 평가 순서
- `complete` 닫기 준비 상태 순서
- complete 닫기 준비 상태 순서

Owner:
- [Translation Guide](../maintain/translation-guide.md)
- [Terminology Map](../../terminology-map.yaml)

Notes:
- English prose should prefer "full" or "entire" when "complete" could be confused with the `complete` enum.
- Use 전체 닫기 준비 상태 평가 순서 for the full close-readiness evaluation order in Korean.

### artifact

English:
- artifact

Korean:
- Reference: 아티팩트
- User-facing: 아티팩트

Preserve:
- `ArtifactRef`
- `ArtifactInput`
- `StagedArtifactHandle`
- `artifact_id`

Avoid:
- artifact 저장
- artifact bytes
- raw path as authority

Owner:
- [API Artifact Schemas](api/schema-artifacts.md)
- [Artifact Storage](storage-artifacts.md)

Notes:
- Artifact storage behavior belongs to artifact contracts, not to the general term.

### `ArtifactRef`

English:
- `ArtifactRef`

Korean:
- Reference: `ArtifactRef`
- User-facing: 아티팩트 참조, when not naming the schema

Preserve:
- `ArtifactRef`
- `existing_artifact_ref`

Avoid:
- Translating the identifier.
- Treating a displayed ref as proof of readable bytes or evidence sufficiency.

Owner:
- [API Artifact Schemas](api/schema-artifacts.md)
- [Artifact Storage](storage-artifacts.md)

Notes:
- `ArtifactRef` is a public pointer to a registered persistent artifact.

### `StagedArtifactHandle`

English:
- `StagedArtifactHandle`

Korean:
- Reference: `StagedArtifactHandle`
- User-facing: 스테이징된 아티팩트 핸들

Preserve:
- `StagedArtifactHandle`
- `staged_artifact_handle`

Avoid:
- staged handle
- bearer token
- persistent artifact

Owner:
- [API Artifact Schemas](api/schema-artifacts.md)
- [Artifact Storage](storage-artifacts.md)

Notes:
- `StagedArtifactHandle` is temporary and is not persistent artifact authority by itself.

### projection

English:
- projection

Korean:
- Reference: 상태 보기
- User-facing: 상태 보기

Preserve:
- `Projection`
- `ProjectionKind`

Avoid:
- Treating rendered display as Core state, evidence, acceptance, or authority.

Owner:
- [Projection Authority Reference](projection-and-templates.md)
- [Template Bodies](template-bodies.md)

Notes:
- Projection is read-only derived display or support context from owner records.

### surface

English:
- surface

Korean:
- Reference: 접점
- User-facing: 접점

Preserve:
- `surface_id`
- `surface_instance_id`
- `VerifiedSurfaceContext`

Avoid:
- surface 정보
- surface authority
- Treating `surface_id` as authority proof.

Owner:
- [Agent Integration](agent-integration.md)
- [Security](security.md)

Notes:
- A surface is a user, agent, tool, connector, or local context where Harness is used or observed.

### runtime

English:
- runtime

Korean:
- Reference: 런타임
- User-facing: 런타임

Preserve:
- `Harness Runtime Home`

Avoid:
- Treating Markdown source docs as runtime state.
- Treating Markdown source docs as generated runtime output.

Owner:
- [Runtime Boundaries](runtime-boundaries.md)
- [Security](security.md)

Notes:
- Runtime means future executing Harness server/runtime behavior and runtime data space.

### `Write Authorization`

English:
- `Write Authorization`

Korean:
- Reference: 쓰기 권한 부여
- User-facing: 쓰기 권한 부여

Preserve:
- `Write Authorization`
- `AuthorizedAttemptScope`
- `WriteAuthorization.basis_state_version`

Avoid:
- write permission
- command approval
- sensitive approval substitute

Owner:
- [Core Model](core-model.md)
- [Security](security.md)
- [Prepare-write method](api/method-prepare-write.md)

Notes:
- `Write Authorization` is the named Core authorization for one compatible product-file write attempt.
- It is not OS permission or sensitive-action approval.

### sensitive approval

English:
- sensitive approval
- sensitive-action approval

Korean:
- Reference: 민감 동작 승인
- User-facing: 민감 동작 승인

Preserve:
- `sensitive_approval`
- `SensitiveActionScope`

Avoid:
- Treating it as Write Authorization.
- Treating it as final acceptance.
- Treating it as broad approval.

Owner:
- [Core Model](core-model.md)
- [API Judgment Schemas](api/schema-judgment.md)
- [Security](security.md)

Notes:
- Prefer "sensitive-action approval" in English prose.

### access class

English:
- access class

Korean:
- Reference: 접근 등급
- User-facing: 접근 등급

Preserve:
- `access_class`
- `VerifiedSurfaceContext.access_class`

Avoid:
- Treating an access class as OS permission.
- Treating an access class as broad authority.

Owner:
- [API Value Sets](api/schema-value-sets.md)
- [Shared envelope and response branch routes](api/methods.md#shared-request-rules)
- [Security](security.md)

Notes:
- Access class is a classification used by API and security owners to describe protected access expectations.

### active guarantee

English:
- active guarantee

Korean:
- Reference: 현재 활성 보장
- User-facing: 현재 활성 보장

Preserve:
- Exact guarantee label values

Avoid:
- Treating reserved or profile-gated labels as active guarantees.

Owner:
- [Security](security.md)
- [Scope](scope.md)
- [API Value Sets](api/schema-value-sets.md)

Notes:
- A guarantee is active only when active scope and Security both document it as current behavior.

### cooperative guarantee

English:
- cooperative guarantee

Korean:
- Reference: 협력형 보장
- User-facing: 협력형 보장

Preserve:
- `cooperative`

Avoid:
- Strengthening cooperative wording into detective, preventive, isolated, sandboxed, or enforced wording.

Owner:
- [Security](security.md)

Notes:
- Cooperative guarantee wording depends on the surface following the documented procedure.

### detective guarantee

English:
- detective guarantee

Korean:
- Reference: 탐지형 보장
- User-facing: 탐지형 보장

Preserve:
- `detective`

Avoid:
- Claiming full monitoring.
- Claiming prevention.

Owner:
- [Security](security.md)
- [Agent Integration](agent-integration.md)

Notes:
- Use detective guarantee only when the documented observable scope and capability check support it.

### preventive guarantee

English:
- preventive guarantee

Korean:
- Reference: 예방형 보장
- User-facing: 예방형 보장

Preserve:
- `preventive`

Avoid:
- Claiming current-MVP sandboxing without an active owner.
- Claiming permission control without an active owner.

Owner:
- [Security](security.md)
- [Scope Reference](scope.md)

Notes:
- Use preventive guarantee only when the exact preventive mechanism and proof path are documented.

### `isolated`

English:
- `isolated`

Korean:
- Reference: `isolated`
- User-facing: `isolated`

Preserve:
- `isolated`

Avoid:
- 격리 보장이 제공됩니다
- 현재 격리됩니다
- 현재 MVP가 isolated 보장을 제공합니다

Owner:
- [Security](security.md) for semantics and non-claims
- [Scope](scope.md) for current-MVP availability
- [API Value Sets](api/schema-value-sets.md) for the value entry

Notes:
- `isolated` is a reserved or profile-gated guarantee label, not an active current-MVP guarantee.
- Presence in a value set does not activate behavior.

### reserved value

English:
- reserved value

Korean:
- Reference: 예약된 값
- User-facing: 예약된 값

Preserve:
- Exact value strings

Avoid:
- default
- required
- supported
- enforced
- accepted
- verified
- close-ready
- active guarantee

Owner:
- [Scope](scope.md)
- [API Value Sets](api/schema-value-sets.md)

Notes:
- A reserved value may exist as vocabulary or future surface area without activating behavior.
- Presence in a value set does not activate behavior.

### profile-gated value

English:
- profile-gated value

Korean:
- Reference: 프로필 조건부 값
- User-facing: 프로필 조건부 값

Preserve:
- Exact value strings

Avoid:
- Treating a profile-gated value as current MVP behavior because it appears in a value set.

Owner:
- [Scope](scope.md)
- [API Value Sets](api/schema-value-sets.md)

Notes:
- A profile-gated value is available only when the relevant profile and owner behavior are active.
- Presence in a value set does not activate behavior.

### dry-run

English:
- dry-run

Korean:
- Reference: dry-run 미리보기
- User-facing: 미리보기

Preserve:
- `dry_run`
- `ToolDryRunResponse`
- `DryRunSummary`
- `PlannedBlocker`

Avoid:
- Treating dry-run output as committed state.
- Treating dry-run output as stored blocker state.
- Treating `PlannedBlocker` as `CloseReadinessBlocker`.

Owner:
- [API Schema Core](api/schema-core.md)
- [API Methods](api/methods.md)
- [API Errors](api/errors.md)
- [Storage Effects](storage-effects.md)

Notes:
- Dry-run is a valid preview path for selected operations and does not commit writes or create owner records.

### blocked result

English:
- blocked result

Korean:
- Reference: 차단 결과
- User-facing: 차단 결과

Preserve:
- `CloseTaskResult(close_state=blocked)`
- `decision=blocked`
- `WriteDecisionReason`
- `CloseReadinessBlocker`

Avoid:
- rejected response
- public error
- `STATE_VERSION_CONFLICT` as blocker code

Owner:
- [API Errors](api/errors.md)
- [Prepare-write method](api/method-prepare-write.md)
- [Close-task method](api/method-close-task.md)
- [Storage Effects](storage-effects.md)

Notes:
- A blocked result is method-specific and is not a public transport or schema rejection.

### rejected response

English:
- rejected response

Korean:
- Reference: 거부 응답
- User-facing: 거부 응답

Preserve:
- `ToolRejectedResponse`
- `ToolError`
- `ErrorCode`

Avoid:
- blocked result
- close blocker
- committed outcome

Owner:
- [API Schema Core](api/schema-core.md)
- [API Errors](api/errors.md)
- [Storage Effects](storage-effects.md)

Notes:
- A rejected response means the method failed before proceeding to the committed operation.

### lifecycle

English:
- lifecycle

Korean:
- Reference: 생명주기
- User-facing: 생명주기

Preserve:
- `Task.lifecycle_phase`
- `artifact_staging.status`

Avoid:
- lifecycle 의미

Owner:
- [Core Model](core-model.md)
- [API Value Sets](api/schema-value-sets.md)
- [Artifact Storage](storage-artifacts.md)

Notes:
- Use lifecycle for the allowed phase progression of a concept such as a `Task` or artifact handle.
