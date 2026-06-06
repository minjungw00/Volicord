# Reference 색인

Reference는 정확한 owner 계약이 필요할 때 사용합니다. Reference는 향후 하네스 서버 계획을 위한 계약 lookup을 담당합니다. 처음 읽는 튜토리얼도, 단계별 구현 계획도 아닙니다.

이 문서들은 향후 하네스 서버 계약을 검토하기 위한 문서입니다. 지금 이 저장소에 서버/런타임, 하네스 런타임 홈, conformance runner, 생성된 읽기용 보기 시스템, 런타임 데이터, 구현이 있다는 뜻이 아닙니다.

Reference 전체를 기본으로 읽지 않습니다. 지금 필요한 질문의 담당 문서를 고른 뒤, 그 담당 문서가 더 엄격한 세부사항을 위임할 때만 링크를 따라갑니다.

## 정확한 계약 담당 문서

Exact field, enum value, lifecycle state, DDL, request/response shape, security guarantee, projection/template body, fixture assertion, validator ID, official terminology는 owner 문서에서만 정의합니다. 다른 문서는 독자에게 보이는 결과를 짧게 요약하고 owner로 연결합니다.

| 계약 영역 | 담당 문서 |
|---|---|
| Core 권한, entity, gate, 상태 전이, `prepare_write`, Write Authorization, `record_run`, `close_task`, blocker, waiver, 대체 불가능한 경계 | [Core Model 참조](core-model.md) |
| Public MCP/API method와 active/later method ownership | Active MVP-1은 [MVP API](api/mvp-api.md), later/profile-gated method는 [API Schema Later](../later/index.md#later-schema-candidates) |
| API schema, envelope, shared ref, `ArtifactRef`, `ValidatorResult`, staged value set, read-only resource, public error, idempotency, replay, state conflict | [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), 그리고 위 method owner |
| Storage layout, SQLite DDL profile, persisted table, storage-owned JSON `TEXT`, lock, migration, artifact, baseline, projection-job storage, validator-run storage | [Storage](storage.md) |
| Security asset, local access posture, trust boundary, threat/control category, guarantee-level 의미, 정직한 cooperative/detective/preventive/isolated 표현 | [보안 참조](security.md) |
| 에이전트 통합, connector capability profile, 대체 동작, context push/pull, generated manifest, Role Lens behavior, 짧은 surface recipe | [Agent 통합 참조](agent-integration.md) |
| Projection rule, 읽기용 보기, 권한 경계, freshness/failure behavior, managed block, 사람이 편집할 수 있는 section, active template body, template class, card body, template display shape, artifact-ref rendering | [Projection과 Template 참조](projection-and-templates.md) |
| Conformance model, MVP behavior example, future fixture body shape, future runner/assertion semantics, fixture profile, suite metadata boundary, current-phase fixture status, Kernel Smoke authoring queue | [Conformance Fixtures 참조](conformance-fixtures.md) |
| 활성 MVP 경로 밖의 향후 scenario family 목록, 승격 조건, suite-family label, catalog-only future candidate | [향후 Fixtures](../later/index.md#future-fixture-families) |
| Operations, diagnostic, staged operator surface, recovery/export/reconcile operation, artifact check, future conformance run entrypoint, docs-maintenance reporting profile | [운영과 Conformance 참조](operations-and-conformance.md) |
| 런타임 경계 공간, Product Repository / Harness Server / Runtime Home 분리, Core-only canonical mutation authority, projection/status card의 파생 표시 경계, artifact storage boundary, recovery boundary, 현재 non-isolation claim | [런타임 경계 참조](runtime-boundaries.md) |
| Design-quality policy, policy-to-validator mapping, stable validator ID, severity composition, waiver semantics, evidence expectation, design-quality close impact | [설계 품질 정책](design-quality-policies.md) |
| 용어, capitalization, official term wording, record-name orientation, owner routing | [용어집 참조](glossary.md) |

문서 작성, 번역, 검토, link hygiene, owner-boundary drift, docs-maintenance 점검은 Maintain이 담당합니다. [문서 작성 가이드](../maintain/authoring-guide.md), [번역 가이드](../maintain/translation-guide.md), [문서 점검표](../maintain/documentation-checks.md)를 사용합니다.

## 독자별 바로가기

- 향후 서버 구현자: [MVP 계획](../build/mvp-plan.md)에서 시작합니다. 정확한 owner 계약이 필요할 때만 이 색인으로 돌아옵니다.
- 첫 내부 증명: [MVP 계획: 첫 내부 smoke 목표](../build/mvp-plan.md#first-internal-smoke-target)를 보고, 필요에 따라 [Core Model 참조](core-model.md), [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), [Storage](storage.md), [보안 참조](security.md)를 사용합니다.
- 사용자 또는 에이전트 동작 문구: [사용자 가이드](../use/user-guide.md)나 [에이전트 가이드](../use/agent-guide.md)에서 시작합니다. 보이는 차단 사유, 판단, 쓰기 전 확인, 증거 공백, 닫기 결과, connector behavior의 정확한 증거가 필요할 때만 Reference를 사용합니다.
- API 질문: active method는 [MVP API](api/mvp-api.md), shared shape는 [API Schema Core](api/schema-core.md), public error는 [API Errors](api/errors.md), later/profile-gated material은 [API Schema Later](../later/index.md#later-schema-candidates)에서 시작합니다.
- Storage 또는 DDL 질문: [Storage](storage.md)에서 시작합니다.
- 보안 보장 질문: [보안 참조](security.md)에서 시작한 뒤, 대상 operation의 정확한 API, storage, Core, connector, operations, conformance owner를 함께 봅니다.
- Projection 또는 template 질문: 파생 표시 규칙, 활성 현재 MVP 템플릿 본문, card shape, 최신성, 권한 경계는 [Projection과 Template 참조](projection-and-templates.md)를 봅니다.
- 향후 보증, 운영, fixture catalog material: [보증 프로필](../later/index.md#assurance-candidates), [운영 프로필](../later/index.md#operations-candidates), [향후 Fixtures](../later/index.md#future-fixture-families)를 사용합니다. 담당 문서가 승격하기 전까지 이 경로는 MVP 구현 경로가 아닙니다.

## Owner가 아닌 문서 규칙

Build, Use, Start, Maintain, README 문서가 strict contract를 필요로 하면 독자에게 보이는 결과를 말하고 이 색인이나 담당 문서로 연결합니다. Full schema, DDL block, transition table, fixture mini-language, template body, enum table, validator table, projection table, threat catalog, glossary definition을 붙여 넣지 않습니다.
