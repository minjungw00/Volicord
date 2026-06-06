# 재작성 수락 리뷰

## 이 리뷰의 성격

이 문서는 현재 하네스 문서 기준선에 대한 maintainer를 위한 문서 재설계 수락 리뷰입니다. Maintainer 인계를 위한 Maintain 문서입니다.

이 리뷰만으로 구현 계획을 수락하지 않습니다. 하네스 서버/런타임 구현, 제품 코드, 생성된 운영 산출물, 생성된 projection, 실행 가능한 fixture, conformance runner, 런타임 상태, evidence record, QA record, Acceptance record, Residual Risk record, close record, 하네스 런타임 홈 내용을 승인하지 않습니다. Runtime conformance가 통과했다고 주장하지 않습니다.

## 권고

권고: 구현 계획 검토에 조건부 준비.

재설계된 문서는 별도의 구현 계획 준비 결정을 위해 maintainer에게 인계할 만큼 일관적입니다. 조건은 maintainer가 [구현 개요: 문서 수락 상태](../build/implementation-overview.md#문서-수락-상태)를 의도적으로 갱신하고, [하네스 서버 구현 준비 조건](../build/implementation-overview.md#하네스-서버-구현-준비-조건)을 수락하거나 재분류하며, [MVP-1 사용자 작업 루프: 서버 코딩 전 필요한 구현 결정](../build/mvp-user-work-loop.md#서버-코딩-전-필요한-구현-결정)의 중앙 결정 기록을 수락하거나 단계 영향과 함께 미루는 것입니다.

이 권고는 지금 서버 코딩을 시작하라는 뜻이 아닙니다. 현재 문서화된 상태는 그대로입니다.

- 문서 검토 상태: 재설계 이후 검토 상태이며 문서 수락 후보입니다.
- 구현 계획 준비 상태: 수락되지 않았습니다.
- 런타임 구현 상태: 시작하지 않았습니다.
- 서버 코딩 전 결정: 코드 작성용으로 수락되지 않았습니다.

## 리뷰 기준

이 리뷰는 active documentation set을 기준으로 합니다. 특히 아래 문서를 봅니다.

- [구현 개요](../build/implementation-overview.md)
- [내부 엔지니어링 점검](../build/engineering-checkpoint.md)
- [MVP-1 사용자 작업 루프](../build/mvp-user-work-loop.md)
- [재작성 계획](rewrite-plan.md)
- [문서 점검표](documentation-checks.md)
- [문서 작성 가이드](authoring-guide.md)
- [번역 가이드](translation-guide.md)

이 문서는 모든 재설계 commit의 역사 diff가 아닙니다. 현재 기준선의 모양을 요약합니다.

## 보존된 핵심 원칙

상태: active baseline에서 보존됨.

문서는 아래 원칙을 일관되게 보존합니다.

- 하네스는 prompt pack이 아닙니다. Scope, 사용자 소유 판단, 증거, 검증 기대, QA 기대, 최종 수락, 닫기 준비 상태, 잔여 위험을 위한 로컬 기준 기록입니다.
- 사용자 소유 판단은 사용자에게 남습니다. Product decision, material technical decision, QA expectation, waiver, final acceptance, residual-risk acceptance를 agent에게 조용히 넘기지 않습니다.
- Evidence, Verification, Manual QA, final acceptance, close readiness, residual risk는 서로 분리됩니다. 서로를 대신하지 않습니다.
- Chat, connector output, Markdown-rendered projection, generated document는 operational truth가 아닙니다.
- Core-owned local state와 artifact reference가 향후 운영 기준입니다.
- Documentation file은 하네스를 이해하고 구현하기 위한 source material입니다. Harness runtime object가 아닙니다.

## 삭제, 축소, 이동된 설계와 문장

상태: owner link가 유지되어 인계 가능.

재설계는 broad workflow, dashboard, reporting, hosted-agent, evaluation-harness, generic MCP-wrapper prose를 제품 중심으로 두지 않습니다. 이런 아이디어는 active MVP framing에서 제거되었거나, non-goal 표현으로 줄었거나, [로드맵](../roadmap.md)과 later-profile 문서로 이동했습니다.

주요 축소와 이동은 다음과 같습니다.

- Broad report/dashboard/export/handoff material은 필요에 따라 [운영 프로필](../later/operations-profile.md), [운영과 Conformance 참조](../reference/operations-and-conformance.md), template owner로 이동했습니다.
- Detached verification hardening, Manual QA matrix, detailed Evidence Manifest behavior, detailed Eval output, risk-review hardening, stewardship validator 같은 full assurance material은 [보증 프로필](../later/assurance-profile.md)이나 관련 Reference owner로 이동했습니다.
- Conformance runner와 executable fixture 표현은 [Conformance Fixtures 참조](../reference/conformance-fixtures.md)와 [향후 Fixtures](../later/future-fixtures.md)에서 future-oriented로 남습니다. 현재 runnable validation처럼 다루지 않습니다.
- Strict schema, DDL, state transition, error semantics, projection rule, template body, storage rule, security guarantee는 Learn, Use, Build, Maintain에서 반복하지 않고 Reference owner로 보냅니다.
- 사용자용 prose는 내부 라벨을 기본 시작점으로 삼지 않도록 줄었습니다. 사용자 문서는 평소 사용자 상황에서 시작해야 합니다.

## 현재 단계 모델

상태: 일관적임.

Active stage model은 다음과 같습니다.

| 라벨 | 현재 의미 |
|---|---|
| Engineering Checkpoint | 첫 내부 local Core authority-loop smoke입니다. Product MVP도 아니고 user-value validation도 아닙니다. |
| Kernel Smoke | Engineering Checkpoint 아래의 좁은 future smoke-check authoring label입니다. Stage가 아니고 현재 executable fixture suite도 아닙니다. |
| MVP-1 User Work Loop | Engineering Checkpoint 이후의 첫 user-value milestone입니다. |
| Assurance Profile | 이후 assurance behavior hardening입니다. |
| Operations Profile | 이후 operations, recovery, export, handoff hardening입니다. |
| Roadmap | Owner docs가 승격하기 전까지 staged delivery 밖의 future candidate입니다. |

## MVP-1 사용자 작업 루프 범위

상태: 계획 검토에 사용할 수 있게 scoped됨.

MVP-1은 첫 사용자 가치 경로입니다. 포함 범위는 ordinary-language start/resume, work-shape classification, scope/non-goals/success criteria, minimal user judgment, separate judgment route display, cooperative `prepare_write`, `record_run`, evidence ref 또는 evidence summary, status와 next-safe-action output, evidence gap, close blocker, residual-risk visibility, compact Core-derived view, 정직한 Core/MCP unavailable behavior입니다.

MVP-1은 full assurance, full operations, broad report, dashboard, hosted UI, broad connector, conformance runner, generated conformance artifact, executable fixture catalog, OS-level sandboxing, arbitrary-tool isolation, permission isolation, tamper-proof local storage, default preventive pre-tool blocking을 명시적으로 제외합니다.

## 내부 엔지니어링 점검 범위

상태: MVP-1보다 좁게 scoped됨.

내부 엔지니어링 점검은 가장 작은 local Core authority loop를 증명합니다.

- local project 하나
- active Task 하나
- active Change Unit 또는 그에 준하는 scope boundary 하나
- `prepare_write` allow/block behavior
- durable single-use Write Authorization 하나
- compatible `record_run` 하나
- artifact/evidence ref 하나
- product file을 변경하지 않고 Core state를 읽는 status output과 `harness.close_task`의 좁은 close-blocker check

여기에는 ordinary-language intake, full judgment presentation, detailed Evidence Manifest behavior, detached verification, Eval, Manual QA, final acceptance, residual-risk acceptance, full close semantics, full projection rendering, dashboard, report, export, recover, conformance runner, broad connector, team workflow, orchestration, metrics, hook, preventive guard expansion, Roadmap automation이 포함되지 않습니다.

## 이후 프로필과 로드맵 경계

상태: early scope와 분리됨.

Assurance Profile은 MVP-1 이후입니다. Owner docs가 exact contract를 정의할 때 verification, Manual QA, detailed evidence, risk review, Eval display, stewardship, TDD trace, feedback-loop, context-hygiene behavior를 단단하게 만들 수 있습니다.

Operations Profile은 MVP-1과 Assurance Profile 이후입니다. Relevant owner가 정의한 뒤 export, recovery, handoff, operator readiness, doctor/readiness surface, artifact integrity operation, projection refresh/reconcile operation, future conformance run entrypoint를 조직합니다.

Roadmap은 candidate material입니다. Dashboard, hosted workflow, team workflow, broader connector, metrics, automation, preventive guard expansion, hook, deployment, canary, rollback, production monitoring, 그 밖의 expansion candidate는 owner docs가 scope, fallback behavior, proof expectation, no projection-as-canonical dependency와 함께 승격하기 전까지 active stage requirement가 아닙니다.

## 남은 열린 구현 결정

상태: 흩어져 있지 않고 중앙화됨.

열린 구현 결정은 [MVP-1 사용자 작업 루프: 아직 열려 있는 구현 결정](../build/mvp-user-work-loop.md#아직-열려-있는-구현-결정)에 중앙화되어 있습니다.

현재 그곳에 기록된 still-open item은 다음과 같습니다.

- 구현 준비 판단: 수락되지 않았습니다.
- Public API coding acceptance: 코드 작성용으로 수락되지 않았습니다.
- Storage/DDL coding acceptance: 코드 작성용으로 수락되지 않았습니다.
- Core transition acceptance: 코드 작성용으로 수락되지 않았습니다.
- Security/local-access acceptance: 코드 작성용으로 수락되지 않았습니다.
- 새 owner conflict: 현재 기록된 항목은 없습니다.

이 항목은 implementation-planning과 coding gate입니다. 문서를 maintainer review에 쓸 수 없다는 뜻은 아니지만, 수락되거나 단계 영향과 함께 명시적으로 미뤄지기 전까지 server coding을 막습니다.

## 문서는 런타임 객체가 아님

상태: 현재 repo guidance로 확인됨.

문서는 documentation file이 source material이라고 반복해서 말합니다. Runtime state, generated projection, evidence record, QA record, Acceptance record, Residual Risk record, close record, operational truth, conformance artifact가 아닙니다. 이 리뷰도 그 경계를 따릅니다.

## 저장소 정체성

상태: 현재 repo guidance로 확인됨.

이 저장소는 현재 문서 전용입니다. 문서 수락과 별도의 구현 계획 준비 결정 이후 하네스 서버 소스 저장소가 될 예정입니다. 사용자의 Product Repository도 아니고 Harness Runtime Home도 아닙니다. 아직 Harness Server/runtime implementation, runtime data, generated projection system, conformance runner, product code, generated operational artifact는 없습니다.

## 사용자 용어 부담 리뷰

상태: 인계 가능. 최종 수락 중 계속 확인 필요.

문서 작성 가이드와 번역 가이드는 사용자용 문서가 `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, `task_events` 같은 라벨이 아니라 평소 사용자 상황에서 시작해야 한다고 요구합니다. Build와 README 경로도 내부 용어를 사용자 prerequisite가 아니라 implementation 또는 reference label로 다룹니다.

이 batch에서 전체 user-language audit는 실행하지 않았습니다. 최종 maintainer는 수락 전 [문서 점검표: 사용자 언어 점검](documentation-checks.md#사용자-언어-점검)을 사용해야 합니다.

## 보안 표현 리뷰

상태: 인계 가능. 구현 증명은 아직 future입니다.

현재 기준선은 MVP-1이 cooperative plus limited detective wording을 사용한다고 말합니다. OS-level permission control, arbitrary-tool sandboxing, tamper-proof local file, default pre-tool blocking, permission isolation, security isolation은 future promoted owner path가 exact covered operation을 증명하기 전까지 주장하지 않습니다.

이 리뷰는 preventive 또는 isolated enforcement를 증명하지 않았습니다. 그런 증명은 future-runtime-only이며 문서 리뷰로 만들 수 없습니다.

## Context, projection, state 분리 리뷰

상태: 인계 가능.

문서는 always-on agent context를 작고 phase-relevant하게 유지합니다. Detailed contract는 전체 Reference set을 prompt에 넣지 않고 owner docs로 보냅니다.

Projection과 template은 derived display로 설명됩니다. Compact MVP-1 view와 status output은 write authorization, evidence satisfaction, acceptance record, risk acceptance, task close, canonical state가 아닙니다. Core-owned local state와 artifact reference가 향후 operational authority로 남습니다.

## Template과 artifact 범위 리뷰

상태: 인계 가능.

현재 기준선은 full template body를 Template Reference owner에 둡니다. Future export/report template은 owner가 승격하기 전까지 MVP-1 밖에 둡니다. 내부 엔지니어링 점검에는 status/blocker output, artifact/evidence ref 하나, 좁은 close-blocker check만 필요합니다. MVP-1은 compact Core-derived view를 사용할 수 있고, later export/report/handoff template은 later-profile material로 남습니다.

Artifact는 owner path로 등록된 reference로 다룹니다. 자유로운 문서 출력물이 authority가 되는 것이 아닙니다.

## 대상 정리 리뷰

상태: 이번 최종 확인에서 대상 점검을 다시 수행했습니다. 전체 문서 수락 점검을 통과했다는 뜻은 아닙니다.

| 영역 | 현재 리뷰 결과 |
|---|---|
| Later-profile Decision Packet template 의미 일치와 authority wording | `docs/en/reference/templates/later-profile/decision-packet.md`, `docs/ko/reference/templates/later-profile/decision-packet.md`, Decision Packet authority wording에 대한 `rg` scan을 수동으로 확인했습니다. 두 문서는 의미가 맞습니다. `DEC`는 특정 `user_judgment`를 위한 optional full-format presentation이고, 일반 MVP-1 경로는 compact 판단 요청으로 남습니다. 아홉 가지 display label이 같고, `decision_packet_id`, `judgment_category`, `judgment_route`, `display_depth` 같은 legacy 이름은 migration 또는 compatibility 문맥으로 제한됩니다. `presentation=short` / `presentation=full`은 렌더링되는 맥락의 양을 바꿀 뿐 authority를 바꾸지 않습니다. 예전 Decision Packet authority-path wording cleanup은 이번 확인 기준으로 해소되었습니다. 확인한 문구 중 Decision Packet을 사용자 소유 판단의 canonical authority path로 다루는 경우는 없었습니다. |
| 한국어 later-profile template localization 점검 | `docs/ko/reference/templates/later-profile/` 아래 19개 파일 전체에서 English cue label 후보, heading, table label, prose label을 scan했고, `README.md`, `decision-packet.md`, `export.md`, `task.md`, heading, table-label hit, 대상 검색 결과를 표본으로 확인했습니다. 이 폴더에는 한국어 rendering-label rule이 있습니다. 확인한 rendered heading, table label, cue label, prose label은 자연스러운 한국어이고, template ID, schema/API name, field name, ref, placeholder, enum value, stable lookup label은 보존합니다. 예전 점검에서 예로 들었던 `display label:`, `active path:`, `write authorization:`, `approval refs`, `Write Authority Summary`, `Close Summary`, `QA waiver user judgment`는 찾지 못했습니다. 이전에 표시됐던 의존성 label은 이제 `차단 원인(blocked_by)`, `해소 대상(unblocks)`, `병렬 가능 항목(parallelizable_with)`처럼 한국어 라벨을 먼저 둡니다. `manifest hash`는 폴더의 localization rule 안에서만 보였고, unresolved rendered label로는 나타나지 않았습니다. `Residual Risk`, `Change Unit`, `Run`, `EVIDENCE-MANIFEST`, `source_state_version`, `run_summary`, `approval_scope` 같은 다른 English hit는 exact/stable label, field name, placeholder, enum value, template ID, 또는 한국어 설명이 붙은 lookup 문맥에 나타납니다. 이번 targeted audit에서 남은 한국어 later-profile localization follow-up은 찾지 못했습니다. 이 점검은 문서 현지화 리뷰이며 runtime conformance나 server-coding status가 아닙니다. |
| Core Model 판단 route와 schema wording | `docs/en/reference/core-model.md`와 `docs/ko/reference/core-model.md`를 수동으로 확인했습니다. 이전 Core Model wording drift는 확인한 source docs에서 해소되었습니다. Route 경계는 의미가 맞습니다. Route verb는 내부 owner-path metadata이고, broad approval은 사용자 표시 모델에 없습니다. Display depth는 presentation metadata이며, 사용자는 같은 아홉 가지 렌더링 판단 라벨을 봅니다. `User Judgment` summary paragraph와 canonical-schema bullet은 같은 `user_judgment`, request/record action, `judgment_kind`, `presentation`, locale에서 파생한 표시 라벨 규칙, compatibility/legacy 용어를 이름 붙입니다. 이번 pass에서 남은 Core Model 영어/한국어 User Judgment wording 문제는 찾지 못했습니다. |
| v01/v02와 legacy fixture identifier | `v0.1`, `v0.2`, `v01`, `v02`, `CORE-v01`, `MVP-v02`, `Core Authority Smoke`, `First User-Value Slice`를 `rg`로 확인했습니다. 남은 match는 translation/glossary 문서의 legacy-label guidance와 이 리뷰 자체의 check 설명입니다. 해당 legacy label을 active stage name, current fixture identifier, executable fixture claim, current conformance result로 쓰는 경우는 찾지 못했습니다. |
| 구현 준비 wording | `docs/en/build/implementation-overview.md`, `docs/ko/build/implementation-overview.md`, 그 문서들이 연결하는 MVP-1 decision log, Maintain guidance, 이 리뷰를 확인했습니다. 문서는 documentation redesign review, 대기 중인 documentation acceptance, 아직 수락되지 않은 implementation-planning readiness, 아직 수락되지 않은 server-coding decision, 시작하지 않은 runtime implementation, future runtime conformance를 구분합니다. |
| Future fixture catalog scope pressure | `docs/en/later/future-fixtures.md`와 `docs/ko/later/future-fixtures.md`를 수동으로 확인했고, MVP requirement, active API/DDL, current conformance, runnable/executable fixture, runner, generated conformance, server implementation, runtime-state wording에 대한 `rg` scan을 실행했습니다. Catalog는 여전히 compact future scenario-family inventory입니다. 각 row가 fixture body, public request schema, storage schema, DDL row, stage exit criterion, generated artifact, runtime result, implementation task, MVP-1 requirement, active API/DDL, executable suite, current conformance가 아니라고 명시합니다. Search hit는 부정 경계 문장이나 future promotion criteria였고, MVP-1로 scope pressure를 만드는 문구는 찾지 못했습니다. |
| Exact contract drift에 대한 Maintain 회귀 점검 coverage | Maintain guidance는 이제 active schema/later-profile separation, prose-only stage gating, undefined active API schema type, API/Core/Storage field coverage, locale label과 schema value 구분, Discovery/`shared_design` output authority, MVP `CloseTaskResponse` scope, structured conformance assertion, Build/Reference contract ownership, maintainer-owned readiness/status value를 future review에서 확인하게 합니다. 이는 documentation-maintenance coverage를 기록할 뿐입니다. Full contract-drift audit가 아니며, 위 권고나 documentation acceptance, implementation-planning readiness, server-coding acceptance, runtime conformance, handoff status를 바꾸지 않습니다. |

## 최종 계약 정리 일관성 기록

이 섹션은 알려진 계약 정리 영역만 짧게 남기는 유지보수 기록입니다. Implementation-readiness approval, final documentation acceptance, runtime conformance, handoff approval, server-coding acceptance가 아닙니다. Readiness, handoff, documentation acceptance, implementation-planning readiness, coding acceptance는 Build owner 문서에서 maintainer가 계속 수동으로 결정합니다.

| 정리 영역 | 최종 일관성 기록 |
|---|---|
| Undefined active schema type reference | 확인한 API/schema owner 경로에서 해소되었습니다. Active reference는 owner 없이 떠 있는 active type이 아니라 `mvp-api.md`, `schema-core.md`, `schema-later.md`, `errors.md`로 해소되어야 합니다. |
| Active/later schema separation | 알려진 정리 범위에서는 해소되었습니다. Active schema block은 active value만 두고, later/profile material은 `schema-later.md`, Later 문서, 또는 명확히 표시된 later/profile owner section으로 보냅니다. |
| API/Core/Storage의 Write Authorization attempt coverage | 확인한 owner 문서에서 해소되었습니다. API, Core, Storage는 같은 `AuthorizedAttemptScope`/attempt boundary 개념을 사용합니다. Blocked 또는 dry-run response는 소비 가능한 authorization row를 만들지 않고, durable status는 `prepare_write.decision`과 분리됩니다. |
| `display_label` canonical conflict | 확인한 judgment owner 문서에서 해소되었습니다. `judgment_kind`가 canonical로 남고, `display_label`이나 localized label은 schema, enum, storage, API, gate value가 아니라 렌더링/호환성 표시 텍스트로 남습니다. |
| Discovery/Shared Design active state boundary | 확인한 owner 문서에서 분명해졌습니다. Discovery shaping과 `shared_design` 계열 output은 active Task, user judgment, Change Unit owner path로 라우팅되지 않는 한 support/display 또는 later/profile material입니다. |
| MVP `CloseTaskResponse` assurance boundary | 확인한 close owner에서 해소되었습니다. Active MVP close response wording은 active close-readiness field와 blocker 안에 남습니다. Later verification, Manual QA, detailed Evidence Manifest, detached verification, Eval, full assurance-profile semantics는 later/profile material로 남습니다. |
| API/Core/Storage transition consistency | 알려진 정리 영역에서 확인했습니다. API transition summary, Core transition wording, Storage row-effect wording은 dry-run, idempotency/replay, state-version, Write Authorization 생성/소비, close/read-only boundary에서 의미가 맞습니다. |
| Minimal structured fixture draft | [Conformance Fixtures 참조](../reference/conformance-fixtures.md)에 active non-executable structured fixture draft와 exact future fixture body shape가 있습니다. 이 draft는 executable fixture, current pass/fail criterion, generated artifact, runtime conformance result가 아닙니다. |
| Link, anchor, Mermaid, bilingual parity | 이 review에서 source-audit 수준으로 확인했습니다. Local link/anchor, active file-map parity, fenced code balance, Mermaid inventory/basic fence form, targeted Korean localization, targeted English/Korean semantic parity를 확인했습니다. Mermaid rendering과 모든 paired file의 full semantic review는 실행하지 않았습니다. |

## 최종 fixture 계약 일관성 기록

이 섹션은 알려진 fixture 계약 일관성 영역에 대한 최종 문서 유지보수 기록입니다. 소스 수준 점검과 아래에 명시한 YAML source parse 검증만 기록합니다. 실행 가능한 conformance fixture나 executable fixture runner support가 존재하거나, conformance runner pass가 발생했거나, generated conformance artifact가 존재하거나, implementation readiness가 수락되었다고 말하지 않습니다. Readiness/status value를 바꾸지 않습니다. Readiness, handoff, documentation acceptance, implementation-planning readiness, coding acceptance는 Build owner 문서에서 maintainer가 계속 결정합니다.

| 점검한 fixture 관련 영역 | 최종 일관성 기록 |
|---|---|
| Public API/schema 기준값 | 다시 확인했습니다. 영어와 한국어 conformance fixture body의 active fixture draft 값은 `conformance-fixtures.md`의 owner guidance와 active public owner인 `schema-core.md`, `mvp-api.md`, `core-model.md`, `storage.md`, `errors.md`를 통해 맞춥니다. 알려진 active draft 영역에 fixture-only dialect는 기록되어 있지 않습니다. |
| `lifecycle_phase` fixture 값 | 확인했습니다. Active fixture draft는 active lifecycle enum value인 `intake`, `shaping`, `ready`, `executing`, `waiting_user`, `blocked`, `completed`, `cancelled`를 사용합니다. `active`, `open`, `terminal` 같은 status word는 lifecycle value가 아닙니다. |
| `RecordRunPayload.kind` fixture 값 | 확인했습니다. Active `RecordRunPayload.kind` branch는 `shaping_update`, `implementation`, `direct`로 남습니다. `verification_input` 같은 later/profile branch는 active MVP fixture body 밖에 둡니다. |
| `CloseTaskRequest.intent` fixture 값 | 확인했습니다. Active close fixture body는 `complete`, `cancel`, `supersede`를 사용합니다. Accepted-risk completion은 새 intent value가 아니라 `intent=complete`, close reason, compatible Core state로 표현합니다. |
| `ArtifactRef.redaction_state` fixture 값 | 확인했습니다. Active artifact assertion은 `none`, `redacted`, `secret_omitted`, `blocked`를 사용합니다. 다른 표시용 단어는 redaction state가 아닙니다. |
| Blocker/error fixture 값 | 확인했습니다. Active blocker와 error assertion은 Core/API owner, active blocker category, API `ErrorCode` precedence를 따릅니다. Prose-only expectation, rendered text, validator finding code는 primary API error가 아닙니다. |
| Storage row fixture 값 | 확인했습니다. Active `expected_storage_rows`는 active storage profile과 owner row shape 안에 남습니다. Later table family는 Storage와 관련 profile owner가 승격하기 전까지 later/profile material입니다. |
| Write Authorization scope assertion | 확인했습니다. Committed allowed Write Authorization fixture는 `request.payload`, `expected_response.write_authorization.attempt_scope`, `expected_storage_rows.write_authorizations.attempt_scope_json`에서 같은 resolved `AuthorizedAttemptScope`를 assert합니다. |
| Judgment display label | 확인했습니다. `MVP-ACTIVE-display-label-not-canonical`의 YAML parse 문제는 backtick으로 시작하는 `display_label` forbidden-side-effect assertion을 따옴표로 감싸 해소되었습니다. Fixture 의미는 그대로입니다. `display_label`과 localized label은 renderer/compatibility display text이며 canonical state, storage identity, validator key, gate key, blocker key, compatibility input, close aggregation key가 아닙니다. |
| Structured fixture body validity | 문서 source로 parse 검증했습니다. `docs/en/reference/conformance-fixtures.md`의 active full structured fixture draft YAML block 16개와 `docs/ko/reference/conformance-fixtures.md`의 대응 block 16개가 PyYAML로 load되었습니다. 영어와 한국어 scenario ID와 순서가 일치했고, 확인한 모든 block은 expected top-level fixture body shape를 사용했습니다. 이는 runner 실행, executable fixture runner support, conformance runner pass, readiness/status 변경이 아닙니다. |
| 영어/한국어 conformance fixture body | 함께 확인했습니다. 영어와 한국어 active conformance fixture body는 scenario coverage, body shape, owner-doc routing, `MVP-ACTIVE-display-label-not-canonical`의 non-canonical display-label 의미를 함께 확인했습니다. |
| Later/profile fixture separation | 확인했습니다. Later/profile fixture material은 active MVP fixture body와 분리되어 있습니다. Future catalog row는 inventory일 뿐 MVP-1 requirement, active API/DDL, executable suite, current conformance result가 아닙니다. |
| 한국어 conformance prose | 확인했습니다. 한국어 conformance 문장과 이 대응 리뷰 문구를 자연스러운 한국어 기술 문장으로 검토했습니다. Exact identifier와 file name은 보존했습니다. |
| Runner/readiness boundary | 확인했습니다. 이 리뷰는 executable fixture runner support를 주장하지 않고, conformance runner pass를 주장하지 않으며, readiness/status value를 바꾸지 않습니다. Readiness, handoff, implementation-planning readiness, coding acceptance는 maintainer 결정으로 남습니다. |
| Link, anchor, Mermaid block | 소스 감사 수준으로 다시 확인했습니다. Local link/anchor, fenced code balance, Mermaid inventory/basic fence form, bilingual file-map parity는 review check일 뿐입니다. Mermaid parser/rendering과 full bilingual semantic review는 실행하지 않았습니다. |

## Link, diagram, bilingual 리뷰 상태

상태: 스크립트로 확인 가능한 점검과 대상 표본 확인을 수행했습니다. Runtime validation, runtime conformance, server-coding acceptance, implementation-planning readiness, full manual documentation acceptance pass로 취급하면 안 됩니다.

이번 리뷰 중 실제로 실행한 점검:

- 최상위 repository Markdown entrypoint와 `docs/**/*.md`에 대한 local relative link/anchor checker를 실행했습니다. 명시적 `<a id="...">` / `<a name="...">` anchor도 포함했습니다. 131개 Markdown 파일, fenced code block 밖의 inline/reference-style local Markdown link 2,284개, fragment/anchor link 867개를 확인했습니다. 이 checker는 raw autolink나 fenced example 안의 link는 세지 않았습니다. Unresolved relative link나 anchor는 보고되지 않았습니다.
- `docs/en`과 `docs/ko`의 active file-map을 확인했습니다. 양쪽 모두 Markdown 파일 64개였고 active file-map 차이는 보고되지 않았습니다.
- 같은 131개 Markdown 파일에 대해 fenced code block balance check를 실행했습니다. Fenced code block 462개를 확인했고 닫히지 않은 fenced code block은 보고되지 않았습니다.
- `v01`, `v02`, `CORE-v01`, `MVP-v02`, `v0.1`, `v0.2`, `Core Authority Smoke`, `First User-Value Slice`에 대한 legacy stage/fixture term check를 `rg`로 실행했습니다. 현재 문맥의 오용은 찾지 못했습니다. 남은 match는 legacy-label guidance 또는 이 리뷰 자체의 check text입니다.
- Decision Packet authority-path wording check를 `rg`와 Core Model 및 later-profile Decision Packet template 수동 확인으로 실행했습니다. 확인한 문구 중 Decision Packet을 사용자 소유 판단의 authority path로 설명하는 경우는 없었습니다. Decision Packet은 `user_judgment`를 위한 full-format/later-profile 또는 legacy presentation label로 남습니다.
- Security Reference와 Implementation Overview에 대해 security wording spot check를 실행했습니다. Cooperative/detective/preventive/isolated wording과 local-access non-claim을 확인했습니다. 확인한 문구는 early stage에 OS permission, arbitrary-tool sandboxing, tamper-proof storage, default pre-tool blocking, permission isolation, security isolation을 주장하지 않습니다.
- Learn과 Use 문서에 대해 user-language/internal-term `rg` scan을 실행했고, Learn Overview, Learn concepts page, User Guide opening과 advanced-terms section, Agent Guide guidance, 영어/한국어 Judgment Request Cookbook opening을 표본으로 확인했습니다. 내부 label은 lookup, advanced, reference-link, stable-anchor 또는 "이 startup language를 요구하지 말라"는 문맥에 나타납니다. 확인한 opening에서는 required user startup language로 쓰이지 않았습니다.
- 한국어 later-profile template localization 점검을 실행했습니다. Targeted exact scan과 broader rendered-label, heading, table-label, cue-label review 결과, 확인한 범위에서는 이전 cleanup이 완료된 것으로 보입니다. 예전 unresolved example은 없었고, 의존성 label은 한국어 라벨 뒤에 exact field name을 괄호로 둡니다. 남은 English hit는 exact/stable identifier, field name, placeholder, enum value, template ID, 또는 한국어 설명이 붙은 lookup label입니다.
- Core Model 영어/한국어 judgment schema wording 표본 확인을 실행했습니다. 이전 wording drift는 해소되었습니다. 확인한 summary paragraph와 canonical-schema bullet은 `user_judgment`, `harness.request_user_judgment`, `harness.record_user_judgment`, `judgment_kind`, `presentation`, locale에서 파생한 표시 라벨, compatibility/legacy terms에서 의미가 맞습니다.
- Mermaid 목록과 기본 fence 확인을 실행했습니다. Paired Build/Reference 문서에서 실제 Mermaid fence 24개를 찾았고, 실제 fence는 모두 `flowchart LR`, `flowchart TD`, 또는 `flowchart TB`로 시작합니다. `command -v mmdc`가 `PATH`에서 renderer를 찾지 못했고, local `npm list mermaid @mermaid-js/mermaid-cli --depth=0`는 empty였으며, global npm package list에는 `corepack`과 `npm`만 있었기 때문에 Mermaid rendering이나 parser validation은 실행하지 않았습니다.
- `TODO_DECISION`, `TODO_IMPLEMENT`, `TODO_REWRITE`에 대한 `rg` spot check를 실행했습니다. Match는 Maintain guidance와 이 리뷰 자체의 check description에 한정됐고, 흩어진 implementation-decision TODO는 찾지 못했습니다.
- Future fixture catalog scope-pressure spot check를 실행했습니다. 영어와 한국어 Future Fixtures catalog와 대상 검색 결과를 확인했고, catalog를 MVP-1 요구사항, executable fixture suite, active API/DDL, current conformance result, implementation checklist로 바꾸는 wording은 찾지 못했습니다.

실행하지 않은 점검:

- Mermaid parser 또는 renderer. 이번 리뷰 중 `PATH`에서 `mmdc`를 찾지 못했으므로 Mermaid syntax/rendering은 검증하지 않았습니다.
- 모든 paired file의 full bilingual semantic review.
- 모든 Learn과 Use 문장의 full user-language audit.
- 전체 문서의 full owner-boundary duplicate-contract audit.
- Targeted later-profile localization audit를 넘어서는 전체 line-by-line 한국어 prose polish.
- Runtime conformance, executable fixture execution, conformance-runner check, generated projection check, generated operational artifact check, runtime state check. 이 documentation-only repository phase에는 그런 대상이 존재하지 않습니다.

## 남은 blocker와 위험

현재 MVP-1 decision log에는 새 owner conflict가 기록되어 있지 않습니다.

### 구현 계획 또는 코딩 전 실제 blocker

아래 항목은 maintainer가 명시적으로 수락하거나, 재분류하거나, stage impact를 적어 미루기 전까지 구현 계획 준비 상태 수락이나 server coding을 막습니다.

- Maintainer documentation acceptance가 아직 대기 중입니다.
- Implementation-planning readiness가 수락되지 않았습니다.
- API, Storage/DDL, Core transition, Security/local-access coding acceptance가 수락되지 않았습니다.
- 관련 readiness와 coding decision을 maintainer가 명시적으로 수락하기 전까지 server coding은 계속 승인되지 않습니다.

이 항목은 documentation-acceptance, implementation-planning, coding gate입니다. Runtime state를 만들지 않으며, generated runtime artifact나 conformance report를 만들어 해소하는 대상이 아닙니다.

### 리뷰 한계 또는 수락 위험

아래 항목은 maintainer documentation acceptance 때 고려해야 합니다. Runtime conformance result가 아니며, 그 자체로 runtime state를 만들거나 막지 않습니다.

- Mermaid parser/renderer validation은 수행하지 않았습니다. Inventory와 basic source audit만 수행했고 source-audit 수준에서는 통과했습니다. 이 리뷰는 실제 Mermaid fence 24개를 찾았고 모두 `flowchart LR`, `flowchart TD`, `flowchart TB`로 시작한다고 확인했지만, actual syntax rendering은 검증하지 않았습니다.
- 모든 English/Korean paired file의 full semantic review는 수행하지 않았습니다.
- 모든 Learn/Use 문장의 full manual user-language audit는 수행하지 않았습니다.
- 전체 문서의 full owner-boundary duplicate-contract audit는 수행하지 않았습니다.
- Targeted later-profile localization audit를 넘어서는 전체 line-by-line Korean prose polish는 수행하지 않았습니다.

### Maintainer가 선택할 수 있는 hard gate

- Maintainer가 actual Mermaid rendering을 hard documentation-acceptance gate로 요구한다면, Mermaid renderer/parser가 있는 환경에서 나중에 실행합니다.
- Maintainer가 이 documentation phase에서 source-audit-only Mermaid validation을 수락한다면, 그 결정을 documentation acceptance process에 명시적으로 기록합니다.
- Maintainer가 그 workflow를 명시적으로 요청하지 않는 한, 이 리뷰를 충족하려고 dependency를 설치하거나 package metadata를 바꾸거나 generated SVG/PNG/PDF render artifact를 commit하지 않습니다.

## 최종 인계 문장

재설계된 문서는 maintainer implementation-planning review에 조건부 준비 상태입니다. 아직 accepted implementation-ready material이 아니며 server coding을 승인하지 않습니다. Maintainer는 다음 explicit readiness decision을 위해 이 리뷰를 [구현 개요](../build/implementation-overview.md), [MVP-1 사용자 작업 루프](../build/mvp-user-work-loop.md), [문서 점검표](documentation-checks.md)와 함께 사용해야 합니다.
