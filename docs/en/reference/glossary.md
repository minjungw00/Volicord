# Glossary

This document owns official terminology for Harness documentation. It defines term-level meaning, Korean terminology choices, and card-level routing for product terms.

It does not define exact schemas, value sets, DDL, storage effects, security mechanisms, API behavior, runtime behavior, or baseline implementation reading paths.

## How to use this glossary

Use the summary table as a compact routing aid. Use the term cards as the editable terminology source for each term.

Each term card uses these ownership fields:

- `Primary owner` is the canonical owner for the term's definition or contract.
- `Related references` are adjacent documents that help interpret the term but do not own it.

Prefer one `Primary owner` per term. When a concept needs a different canonical owner, split it into a more precise glossary term instead of adding another primary owner.

Use this glossary with [docs/terminology-map.yaml](../../terminology-map.yaml), which owns machine-readable bilingual term controls, identifier preservation controls, and Korean mixed-language expressions to avoid.

When a card points to a schema, API, storage, security, projection, or runtime contract, follow the `Primary owner` instead of copying contract detail into the glossary.

## Summary table

| Term | Korean term | Primary owner |
|---|---|---|
| Harness | 하네스 | [Scope](scope.md) |
| Product Repository | Product Repository | [Runtime Boundaries](runtime-boundaries.md) |
| Harness Runtime Home | Harness Runtime Home | [Runtime Boundaries](runtime-boundaries.md) |
| documentation | 문서 | [Authoring Guide](../maintain/authoring-guide.md) |
| baseline scope | 기준 범위 | [Scope](scope.md) |
| supported scope | 지원 범위 | [Scope](scope.md) |
| supported behavior | 지원 동작 | [Scope](scope.md) |
| supported API method | 지원되는 API 메서드 | [API Methods](api/methods.md) |
| supported API value | 지원되는 API 값 | [API Value Sets](api/schema-value-sets.md) |
| out-of-scope capability | 지원 범위 밖 기능 | [Scope](scope.md) |
| evidence collection workflow | 증거 수집 흐름 | [Scope](scope.md) |
| expanded or additional evidence collection workflows | 확장 또는 추가 증거 수집 흐름 | [Scope](scope.md) |
| owner document | 담당 문서 | [Authoring Guide](../maintain/authoring-guide.md) |
| owner contract | 담당 계약 | [Authoring Guide](../maintain/authoring-guide.md) |
| applicable owner path | 적용되는 담당 경로 | [Authoring Guide](../maintain/authoring-guide.md) |
| applicable reference | 적용되는 참조 문서 | [Reference Index](README.md) |
| existing owner | 기존 담당 문서 | [Authoring Guide](../maintain/authoring-guide.md) |
| promotion-time owner update | 승격 시점의 담당 문서 갱신 | [Scope](scope.md) |
| owner placeholder | 담당 문서 자리표시자 | [Authoring Guide](../maintain/authoring-guide.md) |
| `Task` | `Task` | [Core Model](core-model.md) |
| scope | 범위 | [Core Model](core-model.md) |
| active scope | 현재 적용 범위 | [Core Model](core-model.md) |
| active Change Unit | 현재 적용 Change Unit | [Core Model](core-model.md) |
| user-owned judgment | 사용자 소유 판단 | [Core Model](core-model.md) |
| close readiness | 닫기 준비 상태 | [Core Model](core-model.md) |
| close readiness evaluation | 닫기 준비 상태 평가 | [Close-task method](api/method-close-task.md) |
| close task behavior | Task 닫기 동작 | [Close-task method](api/method-close-task.md) |
| close-readiness blocker | 닫기 차단 사유 | [Core Model](core-model.md) |
| `CloseReadinessBlocker` | `CloseReadinessBlocker` | [API State Schemas](api/schema-state.md) |
| blocker category | 차단 사유 범주 | [API Value Sets](api/schema-value-sets.md) |
| complete intent | `complete` | [API Value Sets](api/schema-value-sets.md) |
| full evaluation order | 전체 평가 순서 | [Translation Guide](../maintain/translation-guide.md) |
| artifact | 아티팩트 | [API Artifact Schemas](api/schema-artifacts.md) |
| evidence | 증거 | [Core Model](core-model.md) |
| `ArtifactRef` | `ArtifactRef` | [API Artifact Schemas](api/schema-artifacts.md) |
| `ArtifactInput` | `ArtifactInput` | [API Artifact Schemas](api/schema-artifacts.md) |
| `StagedArtifactHandle` | `StagedArtifactHandle` | [API Artifact Schemas](api/schema-artifacts.md) |
| projection | 상태 보기 | [Projection Authority Reference](projection-and-templates.md) |
| surface | 접점 | [Agent Integration](agent-integration.md) |
| active surface context | 현재 적용 접점 맥락 | [Agent Integration](agent-integration.md) |
| runtime | 런타임 | [Runtime Boundaries](runtime-boundaries.md) |
| `Write Authorization` | 쓰기 권한 부여 | [Core Model](core-model.md) |
| sensitive approval | 민감 동작 승인 | [Core Model](core-model.md) |
| access class | 접근 등급 | [API Value Sets](api/schema-value-sets.md) |
| baseline guarantee | 기준 범위 보장 | [Security](security.md) |
| cooperative guarantee | 협력형 보장 | [Security](security.md) |
| detective guarantee | 탐지형 보장 | [Security](security.md) |
| design-quality owner boundary | 설계 품질 담당 경계 | [Design Quality](design-quality.md) |
| reserved value | 예약된 값 | [Scope](scope.md) |
| profile-gated value | 프로필 조건부 값 | [Scope](scope.md) |
| error routing | 오류 처리 경로 | [API error routing](api/error-routing.md) |
| blocker routing | 차단 사유 처리 경로 | [API blocker routing](api/blocker-routing.md) |
| error/blocker boundary | 오류와 차단 사유의 경계 | [API blocker routing](api/blocker-routing.md) |
| public error as blocker | 공개 오류 코드가 차단 사유로 표현되는 경우 | [API blocker routing](api/blocker-routing.md) |
| `ToolError.details` | `ToolError.details` | [API error details](api/error-details.md) |
| dry-run | dry-run 미리보기 | [API Schema Core](api/schema-core.md) |
| blocked result | 차단 결과 | [API error routing](api/error-routing.md) |
| rejected response | 거부 응답 | [API Schema Core](api/schema-core.md) |
| migration | 마이그레이션 | [Storage Versioning](storage-versioning.md) |
| lifecycle | 생명주기 | [Core Model](core-model.md) |

## Terms

### Harness

Term:
- Harness

Korean term:
- 하네스

Type:
- product concept

Meaning:
- Harness is the local work-authority server for AI-assisted product work.

Primary owner:
- [Scope](scope.md)

Related references:
- [Runtime Boundaries](runtime-boundaries.md)

Usage note:
- Preserve Harness when naming the product; do not treat this documentation repository as a running server.

### Product Repository

Term:
- Product Repository

Korean term:
- Product Repository; user-facing prose may use 제품 저장소.

Type:
- product label

Meaning:
- `Product Repository` is the user's project workspace, separate from Harness runtime state.

Primary owner:
- [Runtime Boundaries](runtime-boundaries.md)

Related references:
- None.

Usage note:
- Preserve `Product Repository` when naming the boundary.

### Harness Runtime Home

Term:
- Harness Runtime Home

Korean term:
- Harness Runtime Home; user-facing prose may use 런타임 홈.

Type:
- product label

Meaning:
- `Harness Runtime Home` is the operational data space for Harness records and artifacts.

Primary owner:
- [Runtime Boundaries](runtime-boundaries.md)

Related references:
- None.

Usage note:
- Preserve `Harness Runtime Home` when naming the boundary.

### documentation

Term:
- documentation

Korean term:
- 문서

Type:
- documentation term

Meaning:
- Documentation is maintained source material, not runtime implementation, generated runtime output, or acceptance state.

Primary owner:
- [Authoring Guide](../maintain/authoring-guide.md)

Related references:
- [Runtime Boundaries](runtime-boundaries.md)
- [Implementation Guide](../build/implementation-guide.md)

Usage note:
- Keep documentation authority separate from runtime behavior and product implementation output.

### baseline scope

Term:
- baseline scope

Korean term:
- 기준 범위

Type:
- scope term

Meaning:
- Baseline scope is the stable support boundary documented for Harness.

Primary owner:
- [Scope](scope.md)

Related references:
- [API Value Sets](api/schema-value-sets.md)

Usage note:
- Do not describe out-of-scope capabilities or profile-gated values as baseline requirements.

### supported scope

Term:
- supported scope

Korean term:
- 지원 범위; 지원되는 범위 when grammar needs a modifier.

Type:
- scope term

Meaning:
- Supported scope is behavior or capability documented as supported.

Primary owner:
- [Scope](scope.md)

Related references:
- None.

Usage note:
- Do not use supported scope for the currently applied scope inside a `Task` or Change Unit.

### supported behavior

Term:
- supported behavior

Korean term:
- 지원 동작

Type:
- support-boundary term

Meaning:
- Supported behavior is behavior documented as supported by Scope and the affected semantic owner.

Primary owner:
- [Scope](scope.md)

Related references:
- [API Value Sets](api/schema-value-sets.md)

Usage note:
- Do not infer support from value-set presence, examples, route summaries, or owner-routing terminology.

### supported API method

Term:
- supported API method

Korean term:
- 지원되는 API 메서드

Type:
- API term

Meaning:
- A supported API method is a public method documented as supported.

Primary owner:
- [API Methods](api/methods.md)

Related references:
- None.

Usage note:
- Preserve exact method identifiers when naming public API methods.

### supported API value

Term:
- supported API value

Korean term:
- 지원되는 API 값

Type:
- API value term

Meaning:
- A supported API value is a value documented as supported, not merely present as vocabulary.

Primary owner:
- [API Value Sets](api/schema-value-sets.md)

Related references:
- [Scope](scope.md)

Usage note:
- Route exact value-name questions to API Value Sets and support-availability questions to Scope or the semantic owner.

### out-of-scope capability

Term:
- out-of-scope capability

Korean term:
- 지원 범위 밖 기능

Type:
- scope boundary term

Meaning:
- An out-of-scope capability is excluded from baseline behavior until Scope and the affected owners define support.

Primary owner:
- [Scope](scope.md)

Related references:
- None.

Usage note:
- Do not call deferred material a baseline requirement.

### evidence collection workflow

Term:
- evidence collection workflow

Korean term:
- 증거 수집 흐름

Type:
- out-of-scope capability wording

Meaning:
- Evidence collection workflow wording names a capability area that remains terminology or out-of-scope wording unless support is defined.

Primary owner:
- [Scope](scope.md)

Related references:
- [Terminology Map](../../terminology-map.yaml)

Usage note:
- Baseline evidence is recorded evidence and evidence summaries, not a collection workflow feature by name alone.

### expanded or additional evidence collection workflows

Term:
- expanded or additional evidence collection workflows
- expanded evidence collection workflows
- additional evidence collection workflows

Korean term:
- 확장 또는 추가 증거 수집 흐름

Type:
- out-of-scope capability family

Meaning:
- This phrase names an excluded evidence-workflow capability family.

Primary owner:
- [Scope](scope.md)

Related references:
- [Terminology Map](../../terminology-map.yaml)

Usage note:
- Do not define workflow outputs, storage records, or close-readiness behavior from this phrase.

### owner document

Term:
- owner document

Korean term:
- 담당 문서

Type:
- owner-routing term

Meaning:
- An owner document is the canonical document allowed to define a product concept, contract, schema family, route, or terminology rule.

Primary owner:
- [Authoring Guide](../maintain/authoring-guide.md)

Related references:
- [Reference Index](README.md)

Usage note:
- A file path is documentation routing, not a product actor.

### owner contract

Term:
- owner contract

Korean term:
- 담당 계약; 담당 문서가 정의한 계약 when clearer.

Type:
- owner-routing term

Meaning:
- An owner contract names the contract defined by the relevant owner document.

Primary owner:
- [Authoring Guide](../maintain/authoring-guide.md)

Related references:
- [Terminology Map](../../terminology-map.yaml)

Usage note:
- Use it when product behavior depends on an owner-defined contract, not when route metadata itself is the contract.

### applicable owner path

Term:
- applicable owner path

Korean term:
- 적용되는 담당 경로

Type:
- owner-routing term

Meaning:
- An applicable owner path is the owner route that applies to a topic.

Primary owner:
- [Authoring Guide](../maintain/authoring-guide.md)

Related references:
- [Reference Index](README.md)
- [doc-index.yaml](../../doc-index.yaml)

Usage note:
- Use this only for documentation routing; do not use `active` for owner routes.

### applicable reference

Term:
- applicable reference

Korean term:
- 적용되는 참조 문서

Type:
- reference-routing term

Meaning:
- Applicable reference names the reference document that defines the relevant contract.

Primary owner:
- [Reference Index](README.md)

Related references:
- [Authoring Guide](../maintain/authoring-guide.md)
- [Terminology Map](../../terminology-map.yaml)

Usage note:
- Treat it as documentation routing shorthand, not runtime state or a storage condition.

### existing owner

Term:
- existing owner
- existing canonical owner
- existing owner document

Korean term:
- 기존 담당 문서

Type:
- owner-routing term

Meaning:
- An existing owner is a canonical owner document that already exists and can be linked as the source of normative meaning.

Primary owner:
- [Authoring Guide](../maintain/authoring-guide.md)

Related references:
- [Reference Index](README.md)
- [doc-index.yaml](../../doc-index.yaml)

Usage note:
- Do not name an owner placeholder as an existing canonical owner.

### promotion-time owner update

Term:
- promotion-time owner update

Korean term:
- 승격 시점의 담당 문서 갱신

Type:
- scope-promotion term

Meaning:
- Promotion-time owner update names the owner changes needed when an out-of-scope capability is promoted into support.

Primary owner:
- [Scope](scope.md)

Related references:
- [Authoring Guide](../maintain/authoring-guide.md)

Usage note:
- Promotion may require creating or designating an owner before updating scope, API, storage, templates, checks, and paired-language docs.

### owner placeholder

Term:
- owner placeholder

Korean term:
- 담당 문서 자리표시자

Type:
- owner-gap term

Meaning:
- An owner placeholder signals that a capability may need an owner created or designated before promotion.

Primary owner:
- [Authoring Guide](../maintain/authoring-guide.md)

Related references:
- [Scope](scope.md)

Usage note:
- Do not route readers to a placeholder as if it were an existing canonical owner.

### `Task`

Term:
- `Task`

Korean term:
- `Task`; user-facing prose may use 작업 when exact entity identity is not needed.

Type:
- Core entity

Meaning:
- `Task` is the user-value unit being shaped, executed, blocked, or closed.

Primary owner:
- [Core Model](core-model.md)

Related references:
- [API State Schemas](api/schema-state.md)
- [API Value Sets](api/schema-value-sets.md)

Usage note:
- Preserve identifiers such as `Task`, `task_id`, and `active_task_id`.

### scope

Term:
- scope

Korean term:
- 범위

Type:
- Core authority term

Meaning:
- Scope is the accepted boundary for what the current `Task` or Change Unit covers and excludes.

Primary owner:
- [Core Model](core-model.md)

Related references:
- [Update-scope method](api/method-update-scope.md)
- [API Judgment Schemas](api/schema-judgment.md)

Usage note:
- Preserve exact identifiers such as `scope`, `scope_decision`, `AuthorizedAttemptScope`, and `SensitiveActionScope`.

### active scope

Term:
- active scope
- currently applied scope

Korean term:
- 현재 적용 범위

Type:
- Core authority term

Meaning:
- Active scope is the scope currently applied inside a `Task` or Change Unit context.

Primary owner:
- [Core Model](core-model.md)

Related references:
- [Update-scope method](api/method-update-scope.md)

Usage note:
- Do not use active scope to mean baseline scope, supported scope, or a documentation contract.

### active Change Unit

Term:
- active Change Unit

Korean term:
- 현재 적용 Change Unit

Type:
- Core authority term

Meaning:
- An active Change Unit is the currently applied Change Unit in the authority model.

Primary owner:
- [Core Model](core-model.md)

Related references:
- [Update-scope method](api/method-update-scope.md)

Usage note:
- Preserve Change Unit as the product term in Korean prose.

### user-owned judgment

Term:
- user-owned judgment

Korean term:
- 사용자 소유 판단; user-facing prose may use 사용자 판단.

Type:
- Core authority term

Meaning:
- User-owned judgment is a decision Harness must ask for or preserve instead of inferring.

Primary owner:
- [Core Model](core-model.md)

Related references:
- [API Judgment Schemas](api/schema-judgment.md)

Usage note:
- Do not treat broad approval as acceptance, risk acceptance, scope change, sensitive-action approval, or `Write Authorization`.

### close readiness

Term:
- close readiness

Korean term:
- 닫기 준비 상태; user-facing prose may use 닫기 가능 여부.

Type:
- Core close-readiness concept

Meaning:
- Close readiness is the Core concept for whether a task can be honestly closed.

Primary owner:
- [Core Model](core-model.md)

Related references:
- [Close-task method](api/method-close-task.md)
- [API blocker routing](api/blocker-routing.md)

Usage note:
- This is the evaluation concept, not the `CloseReadinessBlocker` schema.

### close readiness evaluation

Term:
- close readiness evaluation

Korean term:
- 닫기 준비 상태 평가

Type:
- close-task method term

Meaning:
- Close readiness evaluation is the method-specific evaluation that derives close readiness and remaining blockers.

Primary owner:
- [Close-task method](api/method-close-task.md)

Related references:
- [Core Model](core-model.md)
- [API blocker routing](api/blocker-routing.md)

Usage note:
- Preserve `harness.close_task`, `CloseTaskResult`, and `CloseReadinessBlocker` when naming exact API elements.

### close task behavior

Term:
- close task behavior
- `harness.close_task` behavior
- close-task method behavior

Korean term:
- Task 닫기 동작

Type:
- API method behavior term

Meaning:
- Close task behavior is method-specific request validation, evaluation order, result branching, dry-run behavior, and blocker production.

Primary owner:
- [Close-task method](api/method-close-task.md)

Related references:
- [Core Model](core-model.md)
- [API blocker routing](api/blocker-routing.md)

Usage note:
- Do not use close task behavior as the owner for Core close-readiness meaning or blocker/API response routing.

### close-readiness blocker

Term:
- close-readiness blocker
- close blocker

Korean term:
- 닫기 차단 사유

Type:
- Core close-readiness concept

Meaning:
- A close-readiness blocker is a close-relevant reason that prevents honest close readiness until the responsible owner-defined condition is resolved.

Primary owner:
- [Core Model](core-model.md)

Related references:
- [API State Schemas](api/schema-state.md)
- [API blocker routing](api/blocker-routing.md)

Usage note:
- Use the Korean prose term 닫기 차단 사유 and preserve `CloseReadinessBlocker` only when naming the schema.

### `CloseReadinessBlocker`

Term:
- `CloseReadinessBlocker`

Korean term:
- `CloseReadinessBlocker`; user-facing prose should use 닫기 차단 사유 when not naming the schema.

Type:
- API schema

Meaning:
- `CloseReadinessBlocker` is the API schema identifier for close-readiness blocking data.

Primary owner:
- [API State Schemas](api/schema-state.md)

Related references:
- [API Value Sets](api/schema-value-sets.md)
- [API blocker routing](api/blocker-routing.md)

Usage note:
- Do not use the schema name as the whole close-readiness concept.

### blocker category

Term:
- blocker category

Korean term:
- 차단 사유 범주

Type:
- API value concept

Meaning:
- Blocker category is the prose concept for classifying close-readiness blockers by responsible concern.

Primary owner:
- [API Value Sets](api/schema-value-sets.md)

Related references:
- [API blocker routing](api/blocker-routing.md)

Usage note:
- Preserve `CloseReadinessBlocker.category` when naming the exact field.

### complete intent

Term:
- complete intent
- `complete` when naming the intent value

Korean term:
- `complete`

Type:
- API value term

Meaning:
- Complete intent is the prose concept behind the `complete` intent value.

Primary owner:
- [API Value Sets](api/schema-value-sets.md)

Related references:
- [Close-task method](api/method-close-task.md)
- [Terminology Map](../../terminology-map.yaml)

Usage note:
- Preserve `complete` only when it is an enum value or explicit identifier; use full or entire for ordinary prose meaning.

### full evaluation order

Term:
- full evaluation order
- entire evaluation order

Korean term:
- 전체 평가 순서; in close-readiness context, 전체 닫기 준비 상태 평가 순서.

Type:
- translation term

Meaning:
- Full evaluation order names an entire evaluation sequence without invoking the `complete` enum value.

Primary owner:
- [Translation Guide](../maintain/translation-guide.md)

Related references:
- [Terminology Map](../../terminology-map.yaml)

Usage note:
- Prefer full or entire in English when complete could be confused with `intent=complete`.

### artifact

Term:
- artifact

Korean term:
- 아티팩트

Type:
- artifact term

Meaning:
- An artifact is product work material represented through artifact schemas or artifact storage.

Primary owner:
- [API Artifact Schemas](api/schema-artifacts.md)

Related references:
- [Artifact Storage](storage-artifacts.md)

Usage note:
- Artifact availability alone is not evidence sufficiency.

### evidence

Term:
- evidence

Korean term:
- 증거

Type:
- Core evidence concept

Meaning:
- Evidence supports recorded claims at recorded scope.

Primary owner:
- [Core Model](core-model.md)

Related references:
- [API State Schemas](api/schema-state.md)
- [Record-run method](api/method-record-run.md)
- [Storage Records](storage-records.md)

Usage note:
- Evidence is not final acceptance, residual-risk acceptance, broad verification, or artifact availability by itself.

### `ArtifactRef`

Term:
- `ArtifactRef`

Korean term:
- `ArtifactRef`; user-facing prose may use 아티팩트 참조 when not naming the schema.

Type:
- API schema

Meaning:
- `ArtifactRef` is a public pointer to a registered persistent artifact.

Primary owner:
- [API Artifact Schemas](api/schema-artifacts.md)

Related references:
- [Artifact Storage](storage-artifacts.md)

Usage note:
- A displayed ref is not proof of readable bytes or evidence sufficiency.

### `ArtifactInput`

Term:
- `ArtifactInput`

Korean term:
- `ArtifactInput`; user-facing prose may use provided artifact when not naming the schema.

Type:
- API schema

Meaning:
- `ArtifactInput` is the schema identifier for artifact data supplied to an artifact-owning method.

Primary owner:
- [API Artifact Schemas](api/schema-artifacts.md)

Related references:
- None.

Usage note:
- Artifact input is not persistent artifact authority by itself.

### `StagedArtifactHandle`

Term:
- `StagedArtifactHandle`

Korean term:
- `StagedArtifactHandle`; user-facing prose may use 스테이징된 아티팩트 핸들.

Type:
- API schema

Meaning:
- `StagedArtifactHandle` is the schema identifier for a transient staged artifact handle.

Primary owner:
- [API Artifact Schemas](api/schema-artifacts.md)

Related references:
- [Artifact Storage](storage-artifacts.md)

Usage note:
- A staged handle is transient and is not persistent artifact authority by itself.

### projection

Term:
- projection

Korean term:
- 상태 보기

Type:
- projection term

Meaning:
- A projection is read-only derived display or support context from owner records.

Primary owner:
- [Projection Authority Reference](projection-and-templates.md)

Related references:
- [Template Bodies](template-bodies.md)

Usage note:
- Do not treat rendered display as Core state, evidence, acceptance, or authority.

### surface

Term:
- surface

Korean term:
- 접점

Type:
- integration term

Meaning:
- A surface is a user, agent, tool, connector, or local context where Harness is used or observed.

Primary owner:
- [Agent Integration](agent-integration.md)

Related references:
- [Security](security.md)

Usage note:
- `surface_id` is not authority proof.

### active surface context

Term:
- active surface context

Korean term:
- 현재 적용 접점 맥락

Type:
- integration term

Meaning:
- Active surface context is the current surface context for a request or interaction.

Primary owner:
- [Agent Integration](agent-integration.md)

Related references:
- [Security](security.md)

Usage note:
- Do not treat active surface context as proof of authority, access, binding, or capability by itself.

### runtime

Term:
- runtime

Korean term:
- 런타임

Type:
- runtime term

Meaning:
- Runtime means executing Harness server/runtime behavior and runtime data space.

Primary owner:
- [Runtime Boundaries](runtime-boundaries.md)

Related references:
- [Security](security.md)

Usage note:
- Markdown source docs are not runtime state or generated runtime output.

### `Write Authorization`

Term:
- `Write Authorization`

Korean term:
- 쓰기 권한 부여

Type:
- Core authorization term

Meaning:
- `Write Authorization` is the named Core authorization for one compatible product-file write attempt.

Primary owner:
- [Core Model](core-model.md)

Related references:
- [Security](security.md)
- [Prepare-write method](api/method-prepare-write.md)

Usage note:
- It is not OS permission, command approval, or sensitive-action approval.

### sensitive approval

Term:
- sensitive approval
- sensitive-action approval

Korean term:
- 민감 동작 승인

Type:
- approval term

Meaning:
- Sensitive-action approval is user permission for a sensitive action boundary.

Primary owner:
- [Core Model](core-model.md)

Related references:
- [API Judgment Schemas](api/schema-judgment.md)
- [Security](security.md)

Usage note:
- Prefer sensitive-action approval in English prose; do not treat it as `Write Authorization` or final acceptance.

### access class

Term:
- access class

Korean term:
- 접근 등급

Type:
- access term

Meaning:
- Access class is a classification used to describe protected access expectations.

Primary owner:
- [API Value Sets](api/schema-value-sets.md)

Related references:
- [Agent Integration](agent-integration.md)
- [Security](security.md)

Usage note:
- Do not treat access class as OS permission or broad authority.

### baseline guarantee

Term:
- baseline guarantee

Korean term:
- 기준 범위 보장

Type:
- security term

Meaning:
- A guarantee is a baseline guarantee only when Scope and Security document it as supported in the baseline scope.

Primary owner:
- [Security](security.md)

Related references:
- [Scope](scope.md)
- [API Value Sets](api/schema-value-sets.md)

Usage note:
- Do not treat reserved or profile-gated labels as baseline guarantees.

### cooperative guarantee

Term:
- cooperative guarantee

Korean term:
- 협력형 보장

Type:
- security term

Meaning:
- A cooperative guarantee depends on the surface following the documented procedure.

Primary owner:
- [Security](security.md)

Related references:
- None.

Usage note:
- Do not strengthen cooperative wording into detective, sandboxed, enforced, or stronger-isolation wording.

### detective guarantee

Term:
- detective guarantee

Korean term:
- 탐지형 보장

Type:
- security term

Meaning:
- A detective guarantee depends on documented observable scope and capability checks.

Primary owner:
- [Security](security.md)

Related references:
- [Agent Integration](agent-integration.md)

Usage note:
- Do not claim full monitoring or prevention from detective wording.

### design-quality owner boundary

Term:
- design-quality owner boundary
- design-quality routing boundary
- design-quality boundary

Korean term:
- 설계 품질 담당 경계

Type:
- design-quality term

Meaning:
- Design-quality owner boundary routes design-quality observations to the relevant owner documents or owner contracts.

Primary owner:
- [Design Quality](design-quality.md)

Related references:
- None.

Usage note:
- Design-quality wording is not independent QA, acceptance, residual-risk, evidence, or close authority.

### reserved value

Term:
- reserved value

Korean term:
- 예약된 값

Type:
- value-status term

Meaning:
- A reserved value may exist as vocabulary or reserved surface area without making behavior supported.

Primary owner:
- [Scope](scope.md)

Related references:
- [API Value Sets](api/schema-value-sets.md)

Usage note:
- Value-set presence does not make behavior supported.

### profile-gated value

Term:
- profile-gated value

Korean term:
- 프로필 조건부 값

Type:
- value-status term

Meaning:
- A profile-gated value is available only when the relevant profile and owner behavior define it as supported.

Primary owner:
- [Scope](scope.md)

Related references:
- [API Value Sets](api/schema-value-sets.md)

Usage note:
- Do not treat a profile-gated value as baseline behavior because it appears in a value set.

### error routing

Term:
- error routing
- API response branch routing
- API error routing, when naming the owner document

Korean term:
- 오류 처리 경로

Type:
- API error-routing term

Meaning:
- Error routing covers API response branch routing for rejected responses, blocked results, and `dry_run` previews.

Primary owner:
- [API error routing](api/error-routing.md)

Related references:
- None.

Usage note:
- Do not use error routing for public `ErrorCode` meaning, error precedence, `ToolError.details`, or close-readiness blocker routing.

### blocker routing

Term:
- blocker routing
- close-readiness blocker routing
- API blocker routing, when naming the owner document

Korean term:
- 차단 사유 처리 경로

Type:
- API blocker-routing term

Meaning:
- Blocker routing covers the boundary between close-readiness blockers and API response branches.

Primary owner:
- [API blocker routing](api/blocker-routing.md)

Related references:
- [Close-task method](api/method-close-task.md)

Usage note:
- Method-specific `harness.close_task` behavior belongs to the close-task method owner.

### error/blocker boundary

Term:
- error/blocker boundary
- API error versus close-readiness blocker boundary

Korean term:
- 오류와 차단 사유의 경계

Type:
- API blocker-routing term

Meaning:
- The error/blocker boundary separates API errors returned before a valid evaluation from close-readiness blocker data returned after a valid evaluation.

Primary owner:
- [API blocker routing](api/blocker-routing.md)

Related references:
- [API error codes](api/error-codes.md)

Usage note:
- Do not treat public error codes and blockers as the same code space.

### public error as blocker

Term:
- public error as blocker
- public `ErrorCode` as blocker

Korean term:
- 공개 오류 코드가 차단 사유로 표현되는 경우

Type:
- API blocker-routing term

Meaning:
- Public error as blocker names the narrow case where a public error code may appear as blocker data.

Primary owner:
- [API blocker routing](api/blocker-routing.md)

Related references:
- [API error codes](api/error-codes.md)

Usage note:
- Do not automatically copy public `ErrorCode` values into `CloseReadinessBlocker.code`.

### `ToolError.details`

Term:
- `ToolError.details`

Korean term:
- `ToolError.details`; user-facing prose may use 오류 세부사항 when not naming the exact API identifier.

Type:
- API detail identifier

Meaning:
- `ToolError.details` is the exact API detail identifier for machine-readable error details.

Primary owner:
- [API error details](api/error-details.md)

Related references:
- None.

Usage note:
- Do not treat detail helper values as top-level public `ErrorCode` values.

### dry-run

Term:
- dry-run

Korean term:
- dry-run 미리보기; user-facing prose may use 미리보기.

Type:
- API preview term

Meaning:
- Dry-run is a valid preview path for selected operations.

Primary owner:
- [API Schema Core](api/schema-core.md)

Related references:
- [API Methods](api/methods.md)
- [API error routing](api/error-routing.md)
- [Storage Effects](storage-effects.md)

Usage note:
- Dry-run output does not commit writes, create owner records, or store blocker state.

### blocked result

Term:
- blocked result

Korean term:
- 차단 결과

Type:
- API result term

Meaning:
- A blocked result is a method-specific result that reports a valid operation could not proceed.

Primary owner:
- [API error routing](api/error-routing.md)

Related references:
- [Prepare-write method](api/method-prepare-write.md)
- [Close-task method](api/method-close-task.md)
- [Storage Effects](storage-effects.md)

Usage note:
- A blocked result is not a public transport error or schema rejection.

### rejected response

Term:
- rejected response

Korean term:
- 거부 응답

Type:
- API response branch

Meaning:
- A rejected response means the method failed before proceeding to the committed operation.

Primary owner:
- [API Schema Core](api/schema-core.md)

Related references:
- [API error routing](api/error-routing.md)
- [Storage Effects](storage-effects.md)

Usage note:
- Do not treat a rejected response as a blocked result, close blocker, or committed outcome.

### migration

Term:
- migration

Korean term:
- 마이그레이션

Type:
- storage term

Meaning:
- Migration is a technical schema, storage, data, or documentation migration concept.

Primary owner:
- [Storage Versioning](storage-versioning.md)

Related references:
- [Storage Overview](storage.md)

Usage note:
- Do not translate technical migration as previous choice or prior decision.

### lifecycle

Term:
- lifecycle

Korean term:
- 생명주기

Type:
- lifecycle term

Meaning:
- Lifecycle is the allowed phase progression of a concept such as a `Task` or artifact handle.

Primary owner:
- [Core Model](core-model.md)

Related references:
- [API Value Sets](api/schema-value-sets.md)
- [Artifact Storage](storage-artifacts.md)

Usage note:
- Preserve exact identifiers such as `Task.lifecycle_phase` and `artifact_staging.status`.
