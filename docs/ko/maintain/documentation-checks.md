# 문서 점검표

최종 문서 수락 전이나 큰 리뷰 인계 전에 이 점검표를 사용합니다. Markdown 문서만 보는 실무용 docs-maintenance 점검표입니다. 즉 읽기 전용 문서 품질 점검 profile입니다.

이 점검표는 runtime conformance suite가 아닙니다. Fixture를 실행하지 않습니다. Runtime state를 seed하지 않습니다. Runtime state/events/artifacts/projections/errors를 비교하지 않습니다. `task_events`를 append하지 않습니다. artifact를 만들지 않고, projection을 refresh하지 않으며, generated operational artifact나 conformance report를 만들지 않습니다. QA 또는 acceptance state를 만들지 않습니다. 증거, QA, Acceptance, Residual Risk, close를 기록하지 않습니다. close readiness에 영향을 주지 않고 implementation readiness도 증명하지 않습니다.

docs-maintenance의 `PASS`, `WARN`, `FAIL` 라벨은 manual review가 다음에 볼 것과 고칠 것을 정하는 데 도움이 될 수 있습니다. 하지만 manual acceptance, final acceptance, close readiness, implementation readiness, runtime fixture result가 아닙니다.

runtime conformance는 별도입니다. 구현된 Core/API/storage/surface behavior에만 적용되며, documentation prose가 아니라 실행 가능한 fixture와 state assertion으로 판단합니다. Runtime implementation과 materialized fixture suite가 생기기 전에는 runtime conformance result를 암시하면 안 됩니다.

## 점검 유형

결과를 보고할 때 아래 라벨을 사용합니다.

| 점검 유형 | 의미 |
|---|---|
| `manual` | 리뷰어 판단이 필요합니다. 검색 도구로 후보를 모을 수 있지만 script-only pass로 충분하지 않습니다. |
| `scriptable` | 로컬 문서 script나 parser가 해당 조건을 직접 확인할 수 있습니다. 문서화된 예외는 리뷰어가 확인합니다. |
| `future-runtime-only` | 향후 runtime implementation과 그 증명 경로가 있어야 확인할 수 있습니다. 현재 문서 리뷰에서는 docs가 과장하지 않는지만 확인합니다. |

## 결과 의미

아래 의미는 최종 사전 구현 일관성 점검표를 포함해 모든 항목에 적용합니다.

| 결과 | 의미 |
|---|---|
| `PASS` | 해당 항목의 문서가 내부적으로 일관되고 담당 문서 링크가 예상 출처를 가리킨다는 뜻입니다. `PASS`는 문서 수락, final acceptance, implementation readiness, development readiness, runtime conformance, server coding 시작 허가를 뜻하지 않습니다. |
| `WARN` | 수동 확인이 필요하지만 계약 모순은 아직 확인되지 않은 상태입니다. 모호한 표현, 낡아 보이는 route, 분류가 필요한 owner 문구, 검토가 필요한 한국어/영어 표현에 씁니다. |
| `FAIL` | 해당 항목에서 문서가 서로 모순되거나, owner 충돌이 중복되거나, active/later 경계가 깨진 상태입니다. Docs-maintenance 결과로만 보고하고 owner나 stage로 라우팅합니다. Runtime state, acceptance status, readiness status를 만들지 않습니다. |

이 점검표는 문서 수락을 결정하지 않습니다. 문서 수락과 구현 계획 준비 상태는 Build 인계 담당 문서에서 유지보수자가 직접 결정합니다.

## 최종 사전 구현 일관성 점검표

유지보수자가 문서 수락이나 구현 계획 준비 상태를 검토하기 전에 마지막으로 훑는 수동 점검표입니다. 수락 관문 결과가 아니라 수동 보조 자료입니다. 각 행은 active 영어/한국어 문서, owner 문서, 진입점 요약을 함께 봅니다. `PASS`는 해당 항목의 문서가 내부적으로 일관된다는 뜻일 뿐입니다. 문서가 수락되었거나, implementation-ready, development-ready, runtime으로 증명된 상태라는 뜻이 아닙니다.

| 점검 | 점검 유형 | 담당 경로 | `PASS` | `WARN` | `FAIL` |
|---|---|---|---|---|---|
| Write Authorization status set이 모든 곳에서 같습니다. | `manual`, 후보 검색은 `scriptable`. | [Core Model 참조](../reference/core-model.md#write-authorization), [MVP API](../reference/api/mvp-api.md#harnessprepare_write), [API Schema Core](../reference/api/schema-core.md#evidence-and-pre-write-scope-schemas), [Storage](../reference/storage.md#storage-validation과-enum-hardening). | 모든 active 언급이 owner status set을 사용하고, `prepare_write.decision` 값과 `write_authorizations.status`를 섞지 않습니다. | 요약이 허용/차단 같은 느슨한 표현을 쓰지만 owner로 연결하고 새 lifecycle value를 만들지 않습니다. | 문서가 durable authorization status를 추가, 삭제, 이름 변경하거나 `blocked`를 저장된 authorization status처럼 쓰거나 Core/API/Storage를 서로 다르게 만듭니다. |
| 차단된 write는 authorization row를 만들지 않습니다. | `manual`, 후보 검색은 `scriptable`. | [Core Model 참조: `prepare_write`](../reference/core-model.md#prepare_write), [MVP API: `harness.prepare_write`](../reference/api/mvp-api.md#harnessprepare_write), [Storage](../reference/storage.md#storage-validation과-enum-hardening). | `blocked`, `approval_required`, `decision_required`, `state_conflict`, dry-run response는 response/blocker/error 상태로만 남습니다. Non-dry-run `decision=allowed` 경로만 durable Write Authorization row를 만듭니다. | 어떤 페이지가 write가 차단되었거나 hold되었다고 말하지만, 주변 문장과 owner link가 row 경계를 분명히 합니다. | Active 문서가 blocked 또는 dry-run write가 consumable authorization row, replay row, evidence record, close state, write authority를 만든다고 말합니다. |
| Active MVP method set은 고정되어 있습니다. | `manual`, 후보 검색은 `scriptable`. | [MVP API: MVP-1 method set](../reference/api/mvp-api.md#mvp-1-method-set), [MVP-1 사용자 작업 루프](../build/mvp-user-work-loop.md#핵심-생각), [API Schema Core](../reference/api/schema-core.md#stage-specific-active-value-sets). | Build, API, Reference, surface 문서가 하나의 고정된 owner method set을 유지하고 summary에서 active MVP method를 추가하거나 빼지 않습니다. | Tutorial이나 route가 local example을 위해 일부 method만 이름 붙이지만 owner method set으로 분명히 연결합니다. | 문서가 later/compatibility method를 active MVP처럼 쓰거나, owner summary에서 active method를 빼거나, 다른 내용의 두 번째 active method list를 만듭니다. |
| `harness.next`는 active MVP가 아니며 next action은 `harness.status.next_actions`를 사용합니다. | `manual`, 후보 검색은 `scriptable`. | [MVP API](../reference/api/mvp-api.md#mvp-1-method-set), [Schema Later: `harness.next`](../reference/api/schema-later.md#harnessnext), [MVP-1 사용자 작업 루프](../build/mvp-user-work-loop.md#핵심-생각). | Active MVP 문서는 다음 안전한 행동을 `harness.status.next_actions`로 라우팅하고, `harness.next`는 later/compatibility로 남깁니다. | 페이지가 "next"를 일반 단어로 쓰지만 `harness.next`를 active로 이름 붙이지 않습니다. | Active MVP나 Build 문구가 별도 `harness.next` method를 요구하거나 active status 경로와 같은 것으로 다룹니다. |
| Active storage slice는 구현 가능한 범위입니다. | `manual`. | [Storage](../reference/storage.md#활성-첫-구현-저장-범위), [구현 개요](../build/implementation-overview.md#하네스-서버-구현-준비-조건), [MVP-1 사용자 작업 루프](../build/mvp-user-work-loop.md#mvp-1에-필요한-storage-문서), [런타임 아키텍처 참조](../reference/runtime-architecture.md). | MVP storage는 owner가 승인한 active record로 제한되고, later-profile table, projection job, rich report, generated document를 authority로 요구하지 않습니다. | 어떤 row나 summary가 active/later 경계를 분명히 하기 전에 유지보수자 분류가 필요하지만 Storage owner와 아직 충돌하지 않습니다. | MVP 종료나 내부 엔지니어링 점검 문구가 later-profile storage, projection cache, rich Approval/residual-risk table, full Evidence Manifest storage, generated Markdown source of truth를 요구합니다. |
| Active schema block은 later/profile enum value를 제외합니다. | `manual`, 후보 검색은 `scriptable`. | [API Schema Core](../reference/api/schema-core.md), [API Schema Later](../reference/api/schema-later.md), [MVP API](../reference/api/mvp-api.md), [구현 개요](../build/implementation-overview.md). | Active schema block에는 owning tool, record, profile에 active인 value만 있습니다. Later/profile value는 `schema-later.md`, Later 문서, 명확히 표시된 later/profile owner section에 있습니다. | 주변 note가 review classification을 요구하지만 active schema block 자체에는 inactive value가 없습니다. | Active schema block이 later/profile enum value, compatibility-only value, locale label, Roadmap value를 나열합니다. |
| Prose-only stage gating은 schema 분리를 대신하지 않습니다. | `manual`. | [API Schema Core](../reference/api/schema-core.md), [API Schema Later](../reference/api/schema-later.md), [Storage](../reference/storage.md), [MVP-1 사용자 작업 루프](../build/mvp-user-work-loop.md). | Stage prose는 value, field, table, tool이 언제 쓰이는지 설명하고, active schema, API doc, DDL은 inactive material을 제외하거나 later/profile owner로 보냅니다. | Stage note가 짧지만 owner link가 inactive material을 active contract block 밖에 둡니다. | 문서가 inactive value를 active schema/API/DDL block 안에 두고 주변 prose만으로 아직 active가 아니라고 설명합니다. |
| Active API 문서는 참조한 schema type을 모두 해소합니다. | `manual`, 후보 검색은 `scriptable`. | [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), [API Schema Later](../reference/api/schema-later.md), [API Errors](../reference/api/errors.md). | 모든 active method, request, response, shared ref, envelope member, error shape, `CloseTaskResponse` field가 active schema owner로 해소되거나 later/profile로 명확히 표시됩니다. | Summary에 type name이 나오지만 owner로 연결하고 두 번째 shape을 만들지 않습니다. | Active API 문서가 undefined schema type, shared ref, response member, error shape, later/profile type을 owner route 없이 이름 붙입니다. |
| Shared concept는 API/Core/Storage에서 같은 field coverage를 갖습니다. | `manual`, 후보 검색은 `scriptable`. | [Core Model 참조](../reference/core-model.md), [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), [Storage](../reference/storage.md). | Write Authorization 같은 개념은 API, Core, Storage owner 사이에서 field, status/value set, row boundary, response semantics, storage implication이 맞습니다. | 한 owner가 의도적으로 detail을 줄였지만 exact owner로 연결하고 value나 field를 더하거나 빼지 않습니다. | API, Core, Storage 중 하나가 shared concept의 field나 value를 schema/design decision으로 라우팅하지 않고 추가, 누락, 이름 변경, 의미 변경합니다. |
| Locale-specific display label은 schema identifier가 아닙니다. | `manual`, exact-label 검색은 `scriptable`. | [API Schema Core](../reference/api/schema-core.md), [MVP API](../reference/api/mvp-api.md), [용어집 참조](../reference/glossary.md), [영어 번역 가이드](../../en/maintain/translation-guide.md), [한국어 번역 가이드](translation-guide.md). | `display_label`과 `제품 판단`, `기술 판단`, `범위 판단` 같은 label은 `judgment_kind` 같은 stable identifier에서 파생한 localized rendering text로 남습니다. | Display example이 localized label을 쓰지만 주변 문장이 rendered text라고 말합니다. | 문서가 localized label이나 `display_label` string을 canonical schema value, enum value, API field, storage value, owner contract name처럼 다룹니다. |
| Discovery와 `shared_design` output 권한이 표시됩니다. | `manual`. | [Core Model 참조](../reference/core-model.md), [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), [Projection과 Template 참조](../reference/projection-and-templates.md). | Discovery 또는 `shared_design` output은 canonical active state, projection/derived display, support text, later/profile material 중 무엇인지 말합니다. | 짧은 example은 reviewer classification이 필요하지만 support text나 projection output을 Core-owned state처럼 다루지 않습니다. | 문서가 Discovery/shared-design support text, generated prose, projection output을 canonical active state처럼 제시하거나 contract context에서 authority class를 모호하게 둡니다. |
| MVP `CloseTaskResponse`는 later assurance 의미를 노출하지 않습니다. | `manual`, 후보 검색은 `scriptable`. | [Core Model 참조](../reference/core-model.md), [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), [보증 프로필](../later/assurance-profile.md). | Active MVP close response는 active close-readiness field와 blocker만 노출합니다. Later verification, Manual QA, detailed Evidence Manifest, detached verification, Eval, full assurance-profile semantics는 later/profile로 남습니다. | Close summary가 대비를 위해 later concept를 언급하지만 active로 만들지 않고 owner로 연결합니다. | Active `CloseTaskResponse` 문서나 예시가 later assurance field를 포함하거나 later verification, Manual QA, full Evidence Manifest behavior를 active MVP close behavior라고 말합니다. |
| `dry_run`, idempotency, `state_version` 규칙이 API, Storage, Core에서 맞습니다. | `manual`, 후보 검색은 `scriptable`. | [MVP API: 공통 request 규칙](../reference/api/mvp-api.md#공통-request-규칙), [API Errors: idempotency](../reference/api/errors.md#idempotency), [API Errors: state conflict behavior](../reference/api/errors.md#state-conflict-behavior), [API Schema Core](../reference/api/schema-core.md#tool-envelope), [런타임 아키텍처 참조](../reference/runtime-architecture.md#state-transaction-flow), [Storage](../reference/storage.md#event와-idempotency-의미). | Dry run은 authoritative하지 않고, committed idempotent replay는 중복 side effect 없이 원래 response를 반환하며, state-version/freshness 표현은 owner clock을 따릅니다. | 요약에서 일부 detail이 빠졌지만 owner로 연결하고 충돌하는 규칙을 주장하지 않습니다. | 문서가 dry-run이 idempotency key를 예약하거나 current record를 만든다고 하거나, replay가 side effect를 다시 계산한다고 하거나, stale `expected_state_version`을 current처럼 받아들일 수 있다고 하거나, `basis_state_version`과 response `state_version`을 같은 값처럼 다룹니다. |
| Evidence sufficiency와 close blocker가 분명합니다. | `manual`. | [Core Model 참조: close](../reference/core-model.md#close_task), [MVP API: `harness.close_task`](../reference/api/mvp-api.md#harnessclose_task), [API Schema Core: evidence and pre-write scope schemas](../reference/api/schema-core.md#evidence-and-pre-write-scope-schemas), [설계 품질 정책](../reference/design-quality-policies.md#활성-mvp-차단-집합). | Evidence sufficiency, missing evidence, unresolved judgment, QA/verification status, final acceptance, residual-risk visibility, close blocker가 서로 분리되어 보입니다. | 독자용 summary가 짧아서 owner path를 확인해야 하지만 의미가 충돌하지 않습니다. | Test, screenshot, generic summary, final acceptance, QA waiver, projection prose, status text가 evidence sufficiency나 close readiness를 자동으로 만족한다고 설명합니다. |
| Broad approval은 product judgment를 대신하지 않습니다. | `manual`. | [에이전트 가이드: 판단 요청은 좁고 분명하게](../use/agent-guide.md#5-판단-요청은-좁고-분명하게), [Decision Packet Cookbook](../use/decision-packet-cookbook.md), [API Schema Core: `UserJudgment`](../reference/api/schema-core.md#userjudgment), [용어집 참조: Approval](../reference/glossary.md#approval). | "go ahead"나 "looks good" 같은 넓은 말이 product, technical, scope, QA waiver, final-acceptance, residual-risk judgment로 조용히 바뀌지 않습니다. | 예시의 말투는 넓지만 어떤 한 판단을 해결하는지 바로 좁힙니다. | 문서가 generic approval을 product judgment, technical judgment, scope expansion, QA waiver, final acceptance, residual-risk acceptance, cancellation로 다룹니다. |
| Sensitive approval은 product decision을 대신하지 않습니다. | `manual`. | [API Schema Core: `UserJudgment`](../reference/api/schema-core.md#userjudgment), [Core Model 참조: Approval Gate](../reference/core-model.md#approval-gate), [용어집 참조: Approval](../reference/glossary.md#approval). | `sensitive_approval`은 이름 붙은 민감 동작에 대한 permission으로 남습니다. Product behavior, architecture, UX, scope, correctness, final acceptance, risk acceptance를 결정하지 않습니다. | Sensitive-action 예시가 product choice도 다루지만 product choice를 별도로 묻습니다. | Dependency install, secret access, deployment, destructive write 같은 sensitive-action approval이 제품/기술 방향을 그 자체로 결정한다고 설명합니다. |
| Final acceptance는 evidence나 residual-risk visibility를 대신하지 않습니다. | `manual`. | [Core Model 참조: Acceptance Gate](../reference/core-model.md#acceptance-gate), [Core Model 참조: 증거, 검증, 수동 QA, 최종 수락, 위험](../reference/core-model.md#증거-검증-수동-qa-최종-수락-위험), [API Schema Core: current-position display schemas](../reference/api/schema-core.md#current-position-display-schemas), [용어집 참조: Acceptance](../reference/glossary.md#acceptance). | Final acceptance는 close basis가 보인 뒤 사용자가 결과를 판단하는 것입니다. Evidence를 만들거나 evidence gap을 숨기거나 알려진 residual risk를 지우거나 residual-risk path가 묻지 않은 위험을 수락하지 않습니다. | 페이지가 쉬운 말로 "결과를 수락한다"고 쓰지만 근처 문장이 evidence와 risk를 분리합니다. | Final acceptance가 blocker가 남아 있는데 sufficient evidence, verification, Manual QA, residual-risk acceptance, close readiness처럼 설명됩니다. |
| Design-quality policy가 끝없는 planning loop를 만들 수 없습니다. | `manual`. | [설계 품질 정책: 활성 MVP 차단 집합](../reference/design-quality-policies.md#활성-mvp-차단-집합), [영향 분류와 허용 라우트](../reference/design-quality-policies.md#영향-분류와-허용-라우트), [Core Model 참조: Design Gate](../reference/core-model.md#design-gate). | Active MVP design-quality finding은 작은 Core-backed blocking set이나 하나의 bounded user judgment, evidence request, residual-risk marker, advisory next action, no action으로 라우팅됩니다. Broad catalog가 ordinary work를 무기한 planning에 묶지 않습니다. | Policy page의 종료 표현이 더 분명해야 하지만 finding은 owner impact class로 라우팅됩니다. | 문서가 owner-promoted activation rule이나 bounded route 없이 broad design-quality review, stewardship, TDD, Manual QA, context-hygiene catalog 완료를 ordinary write/close 전 기본 요구사항으로 만듭니다. |
| Cooperative/detective guarantee가 preventive/isolated로 과장되지 않습니다. | 문서 표현은 `manual`; 실제 enforcement 증명은 `future-runtime-only`. | [보안 참조: 정직한 guarantee display](../reference/security.md#정직한-guarantee-display), [런타임 아키텍처 참조: 보장 수준 동작 지도](../reference/runtime-architecture.md#보장-수준-동작-지도), [Agent 통합 참조: Guarantee Levels](../reference/agent-integration.md#guarantee-levels). | Cooperative와 detective 표현은 실제 수준에 맞습니다. Preventive나 isolated claim은 정확한 mechanism, covered operation, owner, proof status를 이름 붙입니다. | Guard, freeze, careful mode 같은 친근한 표현은 검토가 필요하지만 OS blocking이나 isolation을 주장하지 않습니다. | Cooperative/detective path가 증명된 owner path 없이 OS permission, arbitrary-tool sandboxing, tamper-proof storage, universal pre-tool blocking, security isolation처럼 설명됩니다. |
| Reference surface capability profile과 표시 guarantee가 맞습니다. | `manual`, 후보 검색은 `scriptable`. | [Agent 통합 참조: Capability Profiles](../reference/agent-integration.md#capability-profiles), [Surface Cookbook: Reference Local Surface](../reference/surface-cookbook.md#reference-local-surface), [보안 참조](../reference/security.md#정직한-guarantee-display). | Reference `capability_profile` field가 사용자에게 표시하는 guarantee를 뒷받침합니다. Unsupported capability는 authority를 넓히지 않고 claim을 낮추거나 막습니다. | Surface 예시가 field 하나를 생략하지만 owner link를 보존하고 더 강한 guarantee를 주장하지 않습니다. | Surface name, connector label, capability label이 write authority를 주거나 unsupported capability를 숨기거나 profile이 증명하지 못하는 preventive/isolated guarantee를 표시합니다. |
| Artifact redaction, hash, path validation 규칙이 일관됩니다. | `manual`, 후보 검색은 `scriptable`. | [API Schema Core: `ArtifactRef`](../reference/api/schema-core.md#artifactref), [Storage: Artifact와 Evidence 경계](../reference/storage.md#artifact와-evidence-경계), [운영과 Conformance: artifacts check](../reference/operations-and-conformance.md#artifacts-check). | Artifact ref, storage row, operations check, evidence summary가 owner relation, integrity metadata, hash mismatch, redaction/omission/block behavior, staged-path validation에 대해 같은 의미를 유지합니다. | Display summary가 owner rule보다 짧지만 unsafe raw content, arbitrary path, missing integrity fact를 허용하지 않습니다. | 문서가 arbitrary absolute path나 parent traversal을 committed evidence로 받아들이거나, `hash_mismatch`를 cosmetic issue로 보거나, redaction이 omission/blocking을 요구하는 곳에 raw secret/PII를 저장하거나, omitted/blocked bytes를 Harness에서 복구할 수 있다고 말합니다. |
| Conformance fixture는 rendered prose만이 아니라 Core state를 assert합니다. | `manual`; 향후 실행은 `future-runtime-only`. | [Conformance Fixtures 참조](../reference/conformance-fixtures.md), [운영과 Conformance 참조](../reference/operations-and-conformance.md), [향후 Fixtures](../later/future-fixtures.md). | Fixture 문서는 future-oriented로 남고, 향후 assertion이 Core state, storage row, 안정화된 event, error, artifact ref, blocker, guarantee fact를 검사한다고 말합니다. Rendered Markdown/prose만으로는 충분하지 않습니다. | Behavior example이 짧아 리뷰어 분류가 필요하지만 current runnable suite라고 부르지 않습니다. | 문서가 rendered Markdown, generated prose, documentation check, 현재 example을 runtime conformance result나 충분한 fixture pass/fail evidence처럼 다룹니다. |
| Conformance example은 prose-only expectation이 아니라 structured assertion을 사용합니다. | `manual`; 향후 실행은 `future-runtime-only`. | [Conformance Fixtures 참조](../reference/conformance-fixtures.md), [운영과 Conformance 참조](../reference/operations-and-conformance.md), [향후 Fixtures](../later/future-fixtures.md). | Example은 Core state, storage row, 가능한 경우 stable event, error, artifact ref, blocker, guarantee fact에 대한 structured assertion을 이름 붙입니다. | Compact example은 reviewer classification이 필요하지만 prose-only pass/fail을 암시하지 않습니다. | Conformance example이 structured state/storage/event/error assertion 없이 prose, rendered Markdown, generated summary, documentation-check result만으로 expectation이 충족된다고 말합니다. |
| 활성 structured conformance fixture 초안이 owner contract와 일치하고 YAML로도 유효합니다. | `manual`, exact identifier, structured body, YAML parser 후보 검색은 `scriptable`; 향후 실행은 `future-runtime-only`. | [Conformance Fixtures 참조](../reference/conformance-fixtures.md), [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), [API Errors](../reference/api/errors.md), [Core Model 참조](../reference/core-model.md), [Storage](../reference/storage.md). | Active draft의 request payload는 public API schema와 맞고, active value는 `schema-core.md`에서 오며, fixture shorthand와 later/profile-only value가 없습니다. 영어와 한국어의 full structured fixture draft code block은 YAML parser가 읽을 수 있어야 합니다. Full body와 partial illustrative snippet은 구분되고, assertion은 rendered prose가 아니라 structured outcome을 대상으로 합니다. | Draft가 작거나 작성자 note가 있어도 모든 value가 active owner로 해소되고, full YAML body는 YAML parser가 읽을 수 있으며, partial snippet은 runner-loadable이 아니라 partial이라고 분명히 표시됩니다. | Draft가 fixture-only payload branch, active body 안의 shorthand, active schema에 없는 enum, state로 쓰는 `display_label`, later/profile-only value, owner와 충돌하는 expected rows/state/blockers/errors, YAML parser가 읽지 못하는 full structured fixture body, 또는 full runner-loadable fixture처럼 설명된 partial snippet을 사용합니다. |
| Docs-maintenance는 runtime readiness를 주장하지 않습니다. | `manual`, 후보 검색은 `scriptable`. | 이 문서, [문서 작성 가이드](authoring-guide.md), [구현 개요](../build/implementation-overview.md#문서-수락-상태). | Documentation check는 읽기 전용 Markdown review aid로 남습니다. `PASS`/`WARN`/`FAIL`은 문서 수락, runtime behavior 증명, handoff status 변경, implementation authorization을 하지 않습니다. | Route가 "handoff 전 점검"을 말하지만 maintainer decision boundary도 함께 말합니다. | Checklist, status table, README, Build page가 docs-maintenance result 때문에 project가 accepted, implementation-ready, development-ready, close-ready, runtime-conformant가 된다고 말합니다. |
| Build 문서는 Reference-owned exact contract를 다시 정의하지 않습니다. | `manual`, 후보 검색은 `scriptable`. | [문서 작성 가이드: Reference 계약 owner 지도](authoring-guide.md#reference-계약-owner-지도), [구현 개요](../build/implementation-overview.md), [MVP-1 사용자 작업 루프](../build/mvp-user-work-loop.md). | Build 문서는 sequencing, scope, exit criteria, local consequence를 설명하고, exact schema, DDL, API, storage, transition, fixture definition은 Reference owner로 연결합니다. | Build summary가 짧은 reminder를 포함하지만 owner로 연결하고 두 번째 contract를 만들지 않습니다. | Build 문서가 Reference가 소유한 exact schema, DDL, enum, table/column, API response, transition, fixture body, storage rule을 정의하거나 바꿉니다. |
| Readiness와 status value는 maintainer decision입니다. | `manual`, 후보 검색은 `scriptable`. | [구현 개요](../build/implementation-overview.md), [MVP-1 사용자 작업 루프](../build/mvp-user-work-loop.md), 이 문서, [문서 작성 가이드](authoring-guide.md). | Cleanup work는 maintainer가 owner section을 명시적으로 바꾸지 않는 한 documentation acceptance, implementation-planning readiness, server-coding acceptance, coding decision, handoff, final documentation acceptance status value를 보존합니다. | Cleanup이 근처 문장을 고치지만 status value와 maintainer decision boundary는 유지합니다. | Cleanup batch가 readiness, handoff, implementation acceptance, coding acceptance, final documentation acceptance value를 우발적으로 또는 maintainer-owned decision path 밖에서 바꿉니다. |
| 한국어/영어 계약 용어가 의미상 일치합니다. | `manual`, exact identifier 검색은 `scriptable`. | [영어 번역 가이드](../../en/maintain/translation-guide.md), [한국어 번역 가이드](translation-guide.md), [용어집 참조](../reference/glossary.md). | 영어/한국어 대응 문서가 의미, owner link, active/later boundary, exact identifier, API/schema name, enum value, error code, validator ID를 보존하고 한국어 prose가 자연스럽습니다. | 한국어 문체나 표현은 손볼 필요가 있지만 계약 의미와 identifier는 유지됩니다. | 영어/한국어 pair가 계약 의미를 바꾸거나, exact identifier를 번역하거나, active material을 later로 또는 later material을 active로 옮기거나, 다른 owner route를 사용합니다. |

## 점검표

### 링크 점검

- 점검 유형: `scriptable`.
- 볼 것: Active docs의 relative Markdown link, README route, paired-language link, owner-section link, heading anchor.
- 자주 실패하는 예: 이동된 file을 가리킵니다. 예전 heading anchor가 남아 있습니다. 영어 문서가 실수로 한국어 전용 anchor를 가리킵니다. README route가 삭제되었거나 inactive인 page를 가리킵니다.
- 통과 의미: 모든 relative link와 anchor가 active document나 명시된 예외로 연결됩니다. Owner link는 현재 owner document나 owner section으로 갑니다.

### 용어 점검

- 점검 유형: `manual`.
- 볼 것: Start와 Use 문서의 예시, 제목, 요약, 상태 설명에서 internal label이 기본 사용자 언어처럼 쓰이는지 봅니다.
- 자주 실패하는 예: 사용자용 문서가 평소 사용자 상황을 설명하기 전에 `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, `task_events`로 시작합니다. 사용자가 내부 라벨을 말해야 도움을 받을 수 있는 것처럼 보입니다.
- 통과 의미: 사용자용 prose는 평소 말에서 시작합니다. 내부 라벨은 보이는 경계, 차단 사유, record, API, template, Reference link를 설명할 때만 씁니다.

### 단계 점검

- 점검 유형: `manual`.
- 볼 것: Build, Reference, Use, Roadmap 문서의 MVP-1, Engineering Checkpoint, Kernel Smoke, Assurance Profile, Operations Profile, Later, Roadmap 표현.
- 자주 실패하는 예: Roadmap candidate가 MVP-1 requirement처럼 쓰입니다. `Kernel Smoke`가 stage처럼 쓰입니다. Later-profile export, reporting, operations, conformance-runner 내용이 smallest runnable slice의 필수 조건이 됩니다.
- 통과 의미: Engineering Checkpoint는 internal authority-loop smoke로 남습니다. MVP-1 User Work Loop는 첫 사용자 가치 milestone으로 남습니다. Later-profile과 Roadmap material은 owner가 scope, fallback behavior, proof expectation과 함께 승격하기 전까지 future scope입니다.

### 상태 점검

- 점검 유형: `manual`.
- 볼 것: Entrypoint, handoff section, Build 문서, Maintain 문서, 현재 구현 상태를 암시할 수 있는 문장.
- 자주 실패하는 예: 이 repo에 Harness Server가 이미 있다고 말합니다. Documentation acceptance가 server-coding authorization처럼 쓰입니다. Reference design prose가 future 또는 design 경계 없이 구현된 runtime behavior처럼 보입니다.
- 통과 의미: Docs는 현재 repo가 documentation-only이고 post-redesign review 상태라고 말합니다. Maintainer handoff owner가 명시하지 않는 한 implementation-ready가 아닙니다. 의도한 future behavior와 implemented behavior가 구분됩니다.

### 보안 표현 점검

- 점검 유형: 문서 표현은 `manual`. 실제 preventive 또는 isolated enforcement 증명은 `future-runtime-only`.
- 볼 것: Cooperative, detective, preventive, isolated, guard, freeze, careful-mode, sandbox, permission, blocking, tamper-proof, isolation 표현.
- 자주 실패하는 예: Write Authorization을 OS permission, sandboxing, tamper-proof enforcement, preventive blocking, isolation처럼 설명합니다. 증명된 blocking path 없이 connector가 arbitrary tool call을 막는다고 말합니다. Security boundary가 owner 문서보다 넓은 OS isolation을 암시합니다.
- 통과 의미: 모든 claim이 문서화된 guarantee level과 맞습니다. Cooperative나 detective surface가 preventive control을 주장하지 않습니다. Preventive나 isolated claim은 covered operation, mechanism, owner document, proof status를 이름 붙이거나 future-oriented로 둡니다.

### 사용자 언어 점검

- 점검 유형: `manual`.
- 볼 것: 사용자용 문서의 opening, 예시, 사용자가 말할 수 있는 요청, 상태 설명, 판단 질문, close 설명, recovery text.
- 자주 실패하는 예: Use 문서가 사용자가 할 수 있는 요청보다 record taxonomy로 시작합니다. 판단 질문이 선택과 결과보다 `Decision Packet`을 먼저 보여줍니다. Status view 설명이 보이는 요약보다 `ProjectionKind`를 먼저 말합니다.
- 통과 의미: 사용자 문서는 평소 작업, 질문, 보이는 차단 사유, 필요한 판단, 있는 증거, close 결과에서 시작합니다. 내부 라벨은 그 라벨이 해결하는 문제가 먼저 보인 뒤 소개합니다.

### Mermaid 점검

- 점검 유형: Mermaid parser가 있으면 문법은 `scriptable`. 유용성은 `manual`.
- 볼 것: Mermaid fenced code block, 주변 설명, diagram label, owner prose와의 일치.
- 자주 실패하는 예: Mermaid syntax가 render되지 않습니다. Diagram이 장식일 뿐 관계, 순서, 경계, lifecycle을 더 분명하게 하지 않습니다. Diagram이 주변 prose나 owner contract와 다릅니다.
- 통과 의미: Diagram은 문법상 합리적이고 예상 docs toolchain에서 render될 수 있습니다. 독자 부담을 줄일 만큼 유용합니다. 주변 prose가 무엇을 봐야 하는지 알려줍니다.

### 이중 언어 점검

- 점검 유형: `manual`.
- 볼 것: 영어/한국어 active file map, paired file purpose, section coverage, owner link, stable identifier, exact code-like string, 한국어 prose 품질.
- 자주 실패하는 예: 영어에 추가된 section이 한국어에서 빠졌습니다. Path, enum value, error code, validator ID, API name이 번역되거나 바뀝니다. 한국어 문장이 영어 기술 명사에 조사만 붙인 형태가 됩니다. Paired link가 다른 owner를 가리킵니다.
- 통과 의미: 영어와 한국어 문서가 같은 의미, coverage, owner routing, exact identifier를 보존합니다. 한국어 제목과 문단은 자연스럽고 의미가 같다면 영어와 달라도 됩니다.

### Owner 점검

- 점검 유형: `manual`.
- 볼 것: Strict contract, schema, DDL, enum value, state transition, gate rule, algorithm, fixture body shape, template body, storage rule, security guarantee, official definition.
- 자주 실패하는 예: Use 문서가 full gate matrix를 반복합니다. Build 문서가 Reference owner의 enum table을 정의합니다. Maintain 문서가 projection freshness를 두 번째 규범 정의로 설명합니다. Glossary owner 밖에서 glossary definition을 복사해 바꿉니다.
- 통과 의미: Strict contract 하나는 owner 문서 하나에서 정의합니다. Owner가 아닌 문서는 짧은 독자용 요약, local consequence, owner link를 둡니다.

### Exact contract drift 점검

- 점검 유형: `manual`, identifier 후보 검색은 `scriptable`.
- 볼 것: Active schema block, API request/response 문서, Core/API/Storage shared concept, localized display label, Discovery와 `shared_design` output, `CloseTaskResponse`, conformance example, Build stage summary, readiness/status value.
- 자주 실패하는 예: Active schema block에 later/profile enum value가 있습니다. Prose-only stage note가 schema separation을 대신합니다. Active API 문서가 undefined schema type을 참조합니다. Write Authorization field가 API, Core, Storage에서 다릅니다. `display_label`이나 `제품 판단`, `기술 판단`, `범위 판단` 같은 한국어 label을 schema value처럼 씁니다. Discovery output이 canonical state, projection, support text, later/profile 중 무엇인지 말하지 않습니다. MVP `CloseTaskResponse`가 later verification, Manual QA, full Evidence Manifest semantics를 노출합니다. Conformance example이 prose-only expectation을 씁니다. Build 문서가 Reference-owned schema나 DDL을 다시 정의합니다. Cleanup work가 readiness/status value를 바꿉니다.
- 통과 의미: Active contract block에는 active contract material만 있습니다. 이름 붙인 active type은 owner로 해소됩니다. Shared concept은 owner 사이에서 맞습니다. Localized label은 display text로 남습니다. Output authority class가 명확합니다. MVP close response는 active scope 안에 남습니다. Conformance example은 structured assertion으로 연결됩니다. Build 문서는 exact contract를 Reference로 연결합니다. Maintainer가 명시적으로 바꾸지 않은 status value는 그대로입니다.

### Conformance fixture 계약 일치 점검

- 점검 유형: `manual`, exact identifier와 structured body 후보 검색은 `scriptable`입니다. 향후 fixture 실행은 `future-runtime-only`입니다.
- 볼 것: [`conformance-fixtures.md`](../reference/conformance-fixtures.md)의 활성 structured fixture 초안과 active fixture example, 그리고 active fixture body를 요약한 Maintain, Build, Reference, Later 문구.
- 담당 경로: [`mvp-api.md`](../reference/api/mvp-api.md), [`schema-core.md`](../reference/api/schema-core.md), [`errors.md`](../reference/api/errors.md), [`core-model.md`](../reference/core-model.md), [`storage.md`](../reference/storage.md), later/profile 경계는 [`schema-later.md`](../reference/api/schema-later.md)와 [`future-fixtures.md`](../later/future-fixtures.md)를 봅니다.
- 점검 목록:
  - [ ] Active fixture `request.payload` body가 `mvp-api.md`의 public request schema와 맞습니다. Fixture-only request field를 추가하거나 required public field를 빠뜨리지 않습니다.
  - [ ] Active fixture enum value가 `schema-core.md`의 active value set으로 해소됩니다. Active schema owner에 없는 enum value는 docs-maintenance `FAIL`입니다.
  - [ ] Fixture shorthand가 active fixture body 안에 없습니다. Shorthand는 active body 밖의 later/profile planning material로만 나타날 수 있습니다.
  - [ ] 모든 full structured fixture draft code block은 fenced body만 꺼낸 뒤 일반 YAML parser가 읽을 수 있어야 합니다. Fixture-specific preprocessing이나 별도 fixture dialect에 기대지 않습니다.
  - [ ] Markdown inline code나 backtick이 따옴표 없는 scalar의 첫 문자로 오면 안 됩니다. Backtick으로 시작하는 identifier 설명은 따옴표로 감싸거나 block scalar로 씁니다.
  - [ ] Colon, backtick, bracket, brace, hash, ampersand, asterisk, question mark, pipe, greater-than, at sign, leading hyphen처럼 YAML에서 민감한 문장 부호를 포함한 prose scalar는 따옴표로 감싸거나 block scalar로 씁니다.
  - [ ] Full fixture body와 partial illustrative snippet을 분명히 구분합니다. Partial YAML snippet은 complete fixture body가 아니라면 runner-loadable이라고 설명하지 않습니다.
  - [ ] Active fixture body는 일반 YAML과 owner가 정의한 schema rule만으로 향후 runner가 읽을 수 있어야 하며, 별도 fixture dialect가 필요하면 안 됩니다.
  - [ ] 영어와 한국어의 full structured fixture body는 모두 YAML parse validation 대상입니다.
  - [ ] YAML parse validation은 source-level docs-maintenance check일 뿐입니다. Executable runner가 존재한다거나, runner가 실행되었다거나, runtime conformance가 통과했다는 뜻이 아닙니다.
  - [ ] `expected_storage_rows`가 `storage.md`의 active table, column, JSON `TEXT` shape, row effect, owner-bound value set과 맞습니다.
  - [ ] `expected_state_changes`가 `core-model.md`의 active Core-owned state field, lifecycle과 transition rule, gate effect, close semantics와 맞습니다.
  - [ ] `expected_blockers`와 `expected_errors`가 `mvp-api.md`, `schema-core.md`, `errors.md`, 관련 Core/storage owner path의 active blocker/error taxonomy와 맞습니다.
  - [ ] Later/profile-only value, method, branch, table family, status value, ref, error, fixture profile이 active fixture body 안에 있으면 docs-maintenance `FAIL`입니다.
  - [ ] Fixture는 Core state, storage row, 승격된 stable event, artifact ref, blocker field, error code, `forbidden_side_effects`를 assert합니다. Rendered Markdown, template prose, status prose, generated summary를 conformance truth로 assert하지 않습니다.
  - [ ] `lifecycle_phase` value가 active lifecycle enum과 맞습니다.
  - [ ] `RecordRunPayload.kind` value가 active record-run branch value와 맞습니다.
  - [ ] `CloseTaskRequest.intent` value가 active close intent value와 맞습니다.
  - [ ] `ArtifactRef.redaction_state` value가 active redaction value와 맞습니다.
  - [ ] `display_label`과 localized label은 rendered display text로 남습니다. Canonical fixture state, state-compatibility input, storage identity, blocker key, gate key, validator key, close aggregation key가 아닙니다.
  - [ ] 한국어 fixture 문서는 `conformance-fixtures.md`, `schema-core.md`, `mvp-api.md`, `storage.md`, `core-model.md`, `errors.md`, `lifecycle_phase`, `RecordRunPayload.kind`, `CloseTaskRequest.intent`, `ArtifactRef.redaction_state`, `display_label` 같은 exact identifier를 보존하면서 자연스러운 한국어 기술 문장으로 씁니다.
  - [ ] 이 점검을 `PASS`, `WARN`, `FAIL`로 보고해도 readiness, handoff, implementation acceptance, coding acceptance, documentation acceptance, close readiness, runtime conformance status는 바뀌지 않습니다.
- 통과 의미: 모든 active fixture draft가 active public API, schema, Core, Storage, blocker, error owner로 추적되고, 모든 full structured fixture draft를 YAML parser가 읽을 수 있습니다. Fixture-only dialect, partial snippet 혼동, later/profile leakage, display-label state, prose-only assertion path가 없습니다.

### 정비 대상 owner 지도 점검

- 점검 유형: `manual`.
- 볼 것: [문서 작성 가이드: 사전 구현 문서 정비 대상 owner 지도](authoring-guide.md#사전-구현-문서-정비-대상-owner-지도)의 알려진 사전 구현 정비 축을 봅니다. Owner contract, API/schema, Storage/DDL, Core transition, stage/profile, evidence/close, security/local-access, conformance proof, user-output/context, design-quality drift가 포함됩니다.
- 자주 실패하는 예: Later-profile API branch가 MVP requirement처럼 쓰입니다. Status card가 gate authority처럼 다뤄집니다. Design-quality validator가 owner activation rule 밖에서 blocker가 됩니다. Documentation check가 runtime conformance처럼 설명됩니다. 보안 문구가 증명된 owner path 없이 pre-tool blocking을 주장합니다.
- 통과 의미: 관찰된 정비 축이 기준 owner 문서군으로 라우팅됩니다. Owner가 아닌 문서는 짧은 local summary와 owner link만 둡니다. 표의 `FAIL` 증상은 docs-maintenance 실패로만 보고합니다. 이 점검은 문서 수락, manual acceptance, runtime conformance, implementation readiness를 결정하지 않습니다.

### Projection/상태 점검

- 점검 유형: `manual`.
- 볼 것: Projection, rendered template, Markdown status view, generated document, state, artifact, evidence, QA, Acceptance, close readiness, operational truth 관련 표현.
- 자주 실패하는 예: Markdown으로 렌더링된 view를 canonical state라고 부릅니다. 문서 파일을 runtime object, generated projection, evidence record, QA record, Acceptance record, Residual Risk record, close record, operational artifact처럼 다룹니다. Projection을 gate authority처럼 설명합니다.
- 통과 의미: Rendered view와 generated document는 derived display로 설명합니다. 향후 operational authority는 관련 Reference owner가 정의한 Core-owned local state와 artifact reference에 남습니다.

### Template scope 점검

- 점검 유형: `manual`.
- 볼 것: Template reference, projection/template page, Use 문서, Build 문서, Later 문서, future template이나 rendered output을 언급하는 Roadmap item.
- 자주 실패하는 예: Later-profile export template이 MVP-1 close에 필요하다고 합니다. Future template body가 current MVP requirement처럼 쓰입니다. 사용자용 문서가 Template Reference owner로 link하지 않고 full template body를 중복합니다.
- 통과 의미: Future template은 owner가 승격하기 전까지 future 또는 later-profile material로 남습니다. Active MVP requirement는 active stage에 필요한 template과 rendered view만 이름 붙입니다. Full template body는 Template Reference owner에 남습니다.
