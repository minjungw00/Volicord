# 재작성 수락 리뷰

## 이 리뷰의 성격

이 문서는 현재 하네스 문서 기준선에 대한 maintainer를 위한 문서 재설계 수락 리뷰입니다. Maintainer 인계를 위한 Maintain 문서입니다.

이 리뷰만으로 구현 계획을 수락하지 않습니다. 하네스 서버/런타임 구현, 제품 코드, 생성된 운영 산출물, 생성된 projection, 실행 가능한 fixture, conformance runner, 런타임 상태, evidence record, QA record, Acceptance record, Residual Risk record, close record, 하네스 런타임 홈 내용을 허가하지 않습니다. Runtime conformance가 통과했다고 주장하지 않습니다.

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

- 하네스는 prompt pack이 아닙니다. Scope, 사용자 소유 판단, 근거, 검증 기대, QA 기대, 작업 수락, 닫기 준비 상태, 잔여 위험을 위한 로컬 기준 기록입니다.
- 사용자 소유 판단은 사용자에게 남습니다. Product/UX judgment, material technical judgment, QA expectation, waiver, work acceptance, residual-risk acceptance를 agent에게 조용히 넘기지 않습니다.
- Evidence, Verification, Manual QA, work acceptance, close readiness, residual risk는 서로 분리됩니다. 서로를 대신하지 않습니다.
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
- scope boundary 하나
- `prepare_write` allow/block behavior
- durable single-use Write Authorization 하나
- compatible `record_run` 하나
- artifact/evidence ref 하나
- Core state를 변경하지 않고 읽는 status/blocker output

여기에는 ordinary-language intake, full judgment presentation, detailed Evidence Manifest behavior, detached verification, Eval, Manual QA, work acceptance, residual-risk acceptance, full close semantics, full projection rendering, dashboard, report, export, recover, conformance runner, broad connector, team workflow, orchestration, metrics, hook, preventive guard expansion, Roadmap automation이 포함되지 않습니다.

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

현재 기준선은 full template body를 Template Reference owner에 둡니다. Future export/report template은 owner가 승격하기 전까지 MVP-1 밖에 둡니다. 내부 엔지니어링 점검에는 status/blocker output과 artifact/evidence ref 하나만 필요합니다. MVP-1은 compact Core-derived view를 사용할 수 있고, later export/report/handoff template은 later-profile material로 남습니다.

Artifact는 owner path로 등록된 reference로 다룹니다. 자유로운 문서 출력물이 authority가 되는 것이 아닙니다.

## 대상 정리 리뷰

상태: 최신 정리 항목에 대한 대상 점검을 갱신했습니다. 전체 검증 절차를 통과했다는 뜻은 아닙니다.

| 영역 | 현재 리뷰 결과 |
|---|---|
| Later-profile Decision Packet template 의미 일치 | `docs/en/reference/templates/later-profile/decision-packet.md`와 `docs/ko/reference/templates/later-profile/decision-packet.md`를 수동으로 확인했습니다. 두 문서는 의미가 맞습니다. `DEC`는 특정 `user_judgment`를 위한 optional full-format presentation이고, 일반 MVP-1 경로는 compact 판단 요청으로 남습니다. 다섯 가지 display label이 같고, `decision_packet_id`, `judgment_category`, `judgment_route`, `display_depth` 같은 legacy 이름은 migration 또는 compatibility 문맥으로 제한됩니다. `presentation=short` / `presentation=full`은 렌더링되는 맥락의 양을 바꿀 뿐 authority를 바꾸지 않습니다. |
| Core Model 판단 route와 display-depth 의미 | `docs/en/reference/core-model.md`와 `docs/ko/reference/core-model.md`를 수동으로 확인했습니다. Route 경계는 의미가 맞습니다. Route verb는 내부 owner-path metadata이고, broad approval은 사용자 표시 모델에 없습니다. Display depth는 presentation metadata이며, 사용자는 같은 다섯 가지 display type을 봅니다. 작은 follow-up은 남습니다. 한국어의 canonical-schema bullet list가 영어 bullet list보다 `user_judgment`, `harness.request_user_judgment`, `presentation`, `display_label`을 더 명시적으로 설명합니다. 알려진 모순은 아니지만, maintainer가 더 쉬운 의미 일치 검토를 원하면 wording을 맞출 수 있습니다. |
| v01/v02와 legacy fixture identifier | `v0.1`, `v0.2`, `v01`, `v02`, 오래된 scenario-prefix pattern을 `rg`로 확인했습니다. Active v01/v02 fixture identifier는 찾지 못했습니다. 역사적 `v0.x` stage label은 translation/glossary 문서의 legacy-label guidance에만 남습니다. 현재 behavior-example ID는 `ENG-CHECK-*`, `MVP1-*`, `CLARIFY-*`를 사용합니다. 예시용 `CORE-active-status-no-task`는 suite metadata 예시에만 있으며 현재 실행 가능한 fixture가 아닙니다. |
| 구현 준비 wording | README, Build handoff, MVP-1 decision log, Maintain guidance, 이 리뷰를 확인했습니다. 문서는 documentation redesign review, 대기 중인 documentation acceptance, 아직 수락되지 않은 implementation-planning readiness, 아직 수락되지 않은 server-coding decision, 시작하지 않은 runtime implementation을 구분합니다. |
| Future fixture catalog scope pressure | [Conformance Fixtures 참조](../reference/conformance-fixtures.md)와 [향후 Fixtures](../later/future-fixtures.md)를 확인했습니다. Future catalog는 compact scenario-family inventory입니다. 예전의 긴 pseudo-fixture payload와 fixture skeleton은 catalog에서 제거되었고, catalog row는 내부 엔지니어링 점검, MVP-1, 현재 conformance, implementation task가 아닙니다. |

## Link, diagram, bilingual 리뷰 상태

상태: scriptable link/anchor 점검과 대상 spot check를 수행했습니다. Runtime validation이나 full manual documentation acceptance pass로 취급하면 안 됩니다.

이번 리뷰 중 실제로 실행한 점검:

- `AGENTS.md`와 `docs` 아래 Markdown에 대한 local relative link/anchor checker: 130개 Markdown file을 확인했고 unresolved relative link나 anchor는 보고되지 않았습니다.
- `rg --files`와 `comm`을 사용한 영어/한국어 active file-map spot check: 차이가 보고되지 않았습니다.
- `rg -n '```mermaid' docs/en docs/ko`를 사용한 Mermaid inventory: paired Reference와 Build 문서에서 Mermaid block을 찾았습니다. Syntax rendering은 실행하지 않았습니다.
- `TODO_DECISION`, `TODO_IMPLEMENT`, `TODO_REWRITE`에 대한 `rg` spot check: Maintain guidance reference 밖에 흩어진 implementation-decision TODO는 찾지 못했습니다.
- Learn과 Use 문서에 대한 user-language/internal-term `rg` scan: glossary, cookbook, agent-guide의 의도된 사용을 확인했습니다. Full manual user-language audit는 실행하지 않았습니다.

실행하지 않은 점검:

- Mermaid parser 또는 renderer. 이번 리뷰 중 `PATH`에서 `mmdc`를 찾지 못했습니다.
- 모든 paired file의 full bilingual semantic review.
- 모든 Learn과 Use 문서의 full user-language audit.
- 전체 문서의 full owner-boundary duplicate-contract audit.

## 남은 blocker와 위험

현재 MVP-1 decision log에는 새 owner conflict가 기록되어 있지 않습니다.

Implementation planning 또는 coding 전 알려진 blocker는 다음과 같습니다.

- Maintainer documentation acceptance가 아직 대기 중입니다.
- Implementation-planning readiness가 수락되지 않았습니다.
- API, Storage/DDL, Core transition, Security/local-access coding acceptance가 수락되지 않았습니다.
- Mermaid syntax rendering, 전체 paired-file semantic review, full manual user-language audit, full owner-boundary duplicate-contract audit는 이 리뷰 batch에서 실행하지 않았습니다.
- 작은 문서 follow-up: maintainer가 더 쉬운 줄 단위 의미 일치 검토를 원하면 `presentation`과 `display_label`을 다루는 Core Model 영어/한국어 canonical-schema bullet wording을 맞출 수 있습니다.

이 blocker는 maintainer acceptance review에서 처리해야 합니다. Runtime artifact나 conformance report를 만들어 처리하지 않습니다.

## 최종 인계 문장

재설계된 문서는 maintainer implementation-planning review에 조건부 준비 상태입니다. 아직 accepted implementation-ready material이 아니며 server coding을 허가하지 않습니다. Maintainer는 다음 explicit readiness decision을 위해 이 리뷰를 [구현 개요](../build/implementation-overview.md), [MVP-1 사용자 작업 루프](../build/mvp-user-work-loop.md), [문서 점검표](documentation-checks.md)와 함께 사용해야 합니다.
