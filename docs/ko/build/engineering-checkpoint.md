# Build: 내부 엔지니어링 점검

## 이 문서가 도와주는 일

이 문서는 첫 내부 하네스 서버 구현 조각인 내부 엔지니어링 점검을 계획할 때 사용합니다. 범위는 로컬 Core 권한 루프를 확인하는 smoke입니다. 제품 MVP도 아니고 MVP-1 사용자 작업 루프도 아니며, 오늘 런타임이 존재한다는 증거도 아닙니다.

이 문서는 계획 문서입니다. Runtime/server 구현은 [구현 개요](implementation-overview.md#문서-수락-상태)에서 문서 수락과 별도의 구현 계획 준비 결정이 수락된 뒤에만 시작할 수 있습니다.

## 이런 때 읽기

- 향후 실행 가능한 가장 작은 조각이 필요할 때.
- 첫 batch가 사용자 가치 MVP 범위로 커지지 않았는지 확인할 때.
- API, DDL, fixture 정의를 복사하지 않고 checkpoint 담당 문서만 찾고 싶을 때.

## 핵심 생각

내부 엔지니어링 점검은 향후 하네스가 Core를 통해 로컬 기준 기록 하나를 유지할 수 있음을 증명하도록 설계된 조각입니다.

1. Status가 active Task 없음 상태를 state 변경 없이 보고할 수 있다.
2. Owner-valid setup/intake path가 정확히 active Task 하나를 만들 수 있다.
3. `surface_id=reference-local-mcp`인 reference `capability_profile` 하나가 등록되어 있다.
4. 호환되는 제품 쓰기 확인이 성공하기 전에 활성 Change Unit 또는 owner-approved scope boundary 하나가 필요하다.
5. `harness.prepare_write`는 missing scope나 out-of-scope work에 structured blocker를 반환하고, compatible non-dry-run decision에 대해서만 durable active Write Authorization 하나를 만들며, dry-run에는 authorization을 만들지 않는다. Replay에서는 authorization을 중복 생성하지 않고, 같은 idempotency key와 다른 hash의 conflict는 side effect 없이 보고한다.
6. `harness.record_run`이 compatible Run 하나를 기록하고 authorization을 한 번 소비한다.
7. Consumed, missing, stale authorization이나 authorized scope 밖에서 관찰된 attempt는 completion evidence를 만들지 않고 `record_run`을 막거나 owner violation/audit path로 라우팅한다.
8. Artifact/evidence ref가 owner path를 통해 hash와 redaction metadata를 기록하고, raw secret artifact storage는 막거나 safe omission/block metadata로만 표현한다.
9. Evidence summary가 registered ref에서 partial 또는 sufficient state를 보여줄 수 있다.
10. 상태/차단 사유 출력이 Core state를 변경하지 않고 읽으며 reference surface guarantee limit을 보여준다.
11. 좁은 `harness.close_task` blocker check가 missing evidence 또는 unresolved user judgment 때문에 close가 막혔음을 보여주고, full close semantics 없이 acceptance 전 residual risk를 보여줄 수 있다.

여기까지입니다. 이 checkpoint는 사용자에게 보이는 가치를 더하기 전에 권한 루프가 살아 있는지 확인하기 위해 존재합니다.

## 제품 MVP가 아님

내부 엔지니어링 점검에는 아래 항목이 명시적으로 포함되지 않습니다.

- 평소 언어 intake나 전체 요구사항 구체화
- 전체 사용자 판단 표시
- detailed Evidence Manifest behavior
- detached verification, Eval, Manual QA, 최종 수락, 잔여 위험 수락, full close semantics
- projection renderer, detailed template, dashboard, hosted UI, report, export, recover
- conformance runner나 실행 가능한 fixture catalog
- broad connector ecosystem, hosted connector registry, cross-surface orchestration, team workflow, metrics, hook expansion, preventive guard expansion, Roadmap automation

제안된 첫 조각이 통과하려면 이런 capability가 필요하다면 더 이상 내부 엔지니어링 점검이 아닙니다.

## Build 순서

Readiness가 수락된 뒤 구현 계획 순서로 사용합니다. 여기서는 command name이나 schema detail이 아니라 capability를 이름 붙입니다.

| 단계 | 구현자 목표 | 완료 상태 | 담당 문서 |
|---|---|---|---|
| 1. Runtime home, project registration, reference surface profile | 미래 하네스 런타임 홈을 통해 local product repository 하나를 resolve하고 reference `capability_profile`을 등록합니다. | Status가 unregistered, registered-idle, active-work 상태를 구분하고, reference profile이 pre-tool blocking 또는 isolation claim 없이 cooperative/detective임을 표시합니다. | [런타임 아키텍처 참조](../reference/runtime-architecture.md), [Storage](../reference/storage.md), [보안 참조](../reference/security.md), [Agent 통합 참조](../reference/agent-integration.md#capability-profiles). |
| 2. Task record 하나 | Owner-valid path로 active Task 하나를 만들거나 seed합니다. | Status가 active Task와 state version을 보여 주고, 필요한 곳에서 stale state-changing call을 거절합니다. | [Core Model 참조](../reference/core-model.md), [API Errors](../reference/api/errors.md). |
| 3. 활성 Change Unit/scope boundary 하나 | 의도한 제품 쓰기 하나를 제한할 수 있는 가장 작은 활성 Change Unit 또는 owner-approved scope boundary를 붙입니다. | Compatible scope가 없으면 product write가 Write Authorization을 받을 수 없습니다. | [Core Model 참조](../reference/core-model.md). |
| 4. `prepare_write` decision | 의도한 쓰기를 owner가 정의한 쓰기 전 범위 확인으로 보냅니다. | Missing scope나 out-of-scope work는 구조화된 하네스 차단 사유 또는 non-`allowed` response를 반환하고, compatible work는 정직한 guarantee display와 함께 Write Authorization ref를 돌려줍니다. 이는 OS 권한이나 물리적 도구 실행 전 차단이 아닙니다. | [Core Model 참조](../reference/core-model.md#prepare_write), [`harness.prepare_write`](../reference/api/mvp-api.md#harnessprepare_write), [API Errors](../reference/api/errors.md). |
| 5. `record_run` | Compatible Run 하나를 기록하고 authorization을 소비합니다. | Compatible Run은 한 번 성공하고, 소비된 authorization 재사용은 실패합니다. | [Core Model 참조](../reference/core-model.md#record_run), [`harness.record_run`](../reference/api/mvp-api.md#harnessrecord_run). |
| 6. Artifact/evidence ref | Durable artifact 또는 evidence ref 하나를 owner path로 등록합니다. | Run 또는 minimal owner relation이 등록된 ref를 cite할 수 있습니다. Owner path가 요구하는 경우 hash, size, content type, redaction, owner, availability metadata도 포함합니다. | [API Schema Core](../reference/api/schema-core.md#artifactref), [Storage](../reference/storage.md). |
| 7. Status와 blocker | 현재 상태와 blocker를 mutation 없이 노출합니다. | 반복 read가 state를 바꾸지 않고, blocker가 향후 smoke check에서 비교할 만큼 구조화되어 있습니다. | [`harness.status`](../reference/api/mvp-api.md#harnessstatus), [Core Model 참조](../reference/core-model.md), [API Schema Core](../reference/api/schema-core.md). |
| 8. 좁은 close blocker check | 이 권한 루프에서 missing evidence, unresolved user judgment, visible residual risk 때문에 close가 막히는지 확인합니다. | 막힌 close는 최종 수락, 잔여 위험 수락, full assurance close semantics, generated report를 만들지 않고 structured blocker를 반환합니다. | [Core Model 참조](../reference/core-model.md#close_task), [`harness.close_task`](../reference/api/mvp-api.md#harnessclose_task), [API Errors](../reference/api/errors.md). |

API 단계 구분은 [Stage Profile Manifest](../reference/api/schema-core.md#stage-profile-manifest)를 사용합니다. Storage 계획은 [Storage](../reference/storage.md)를 사용하고, 이 checkpoint에 필요한 owner-approved minimal subset만 적용합니다.

## 문서 수준 계획 리뷰 점검

향후 내부 엔지니어링 점검 계획이 maintainer planning review에 들어가려면 아래를 만족해야 합니다.

- Local, single-project, one Task 권한 루프에 집중한다.
- Registered reference `capability_profile` 하나를 사용하며 connector platform이나 registry를 요구하지 않는다.
- [문서 수락 상태](implementation-overview.md#문서-수락-상태)가 구현 계획 준비 상태를 수락하기 전까지 계획 전용으로 남는다.
- `prepare_write`, Write Authorization, `record_run`, artifact/evidence ref, structured status/blocker output, 좁은 close-blocker check를 지나는 scoped Harness authority path 하나를 보여준다.
- Active path에서 support가 필요한 경우 missing scope, out-of-scope intended work, 같은 idempotency key와 다른 hash의 conflict, product-write Run의 missing Write Authorization, consumed Write Authorization 재사용, authorized scope 밖에서 관찰된 attempt, raw secret artifact input, missing artifact/evidence support에 대해 structured blocker, non-`allowed` response, API-owned error를 반환한다.
- Status text, generated prose, projection-like output은 모두 Core record에서 파생된 read로 취급하며 fixture proof로 보지 않는다.
- Future smoke check는 [Conformance Fixtures 참조](../reference/conformance-fixtures.md)가 담당하는 structured draft field를 사용한다. Field는 `expected_response`, `expected_state_changes`, `expected_storage_rows`, `expected_events`, `expected_artifacts`, `expected_blockers`, `expected_errors`, `forbidden_side_effects`입니다.
- 통과 조건에 full projection rendering, multiple projection kind, detailed template, operations, conformance runner, broad connector ecosystem, hosted connector registry, cross-surface orchestration, later-profile storage를 요구하지 않는다.
- Strict fixture format과 assertion은 여기서 정의하지 않고 [Conformance Fixtures 참조](../reference/conformance-fixtures.md)로 연결한다.

이 항목은 문서 계획 점검일 뿐입니다. acceptance state, manual acceptance, close readiness, runtime conformance result, generated artifact, projection refresh, implementation readiness를 만들지 않습니다.

## 향후 smoke check

Kernel Smoke는 내부 엔지니어링 점검 check를 위한 좁은 향후 작성 라벨일 뿐입니다. Stage name도 아니고 full suite도 아니며 현재 실행 가능한 fixture set도 아닙니다.

런타임 구현이 생긴 뒤 future smoke check는 owner record, state transition, storage row, stable event가 있을 때의 `task_events`, artifact/evidence ref, structured blocker, primary error, guarantee display fact, forbidden side effect를 확인해야 합니다. Rendered prose, generated Markdown, polished template matching만으로 success를 증명하면 안 됩니다.

향후 작성 순서는 [Conformance Fixtures 참조: Kernel Smoke Authoring Queue](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue)를 사용하고, structured non-executable fixture draft shape은 [Conformance Fixture Format](../reference/conformance-fixtures.md#conformance-fixture-format)을 사용합니다.

## 이것이 증명하는 것

내부 엔지니어링 점검은 아래를 증명합니다.

- Core가 local state transition path 하나를 소유할 수 있다.
- 하네스와 호환되는 Write Authorization을 만들기 전에 scope가 필요하다.
- Write Authorization은 durable하고 single-use다.
- `record_run`은 Write Authorization을 소비하고 observed work를 기록한다.
- Registered artifact/evidence ref 하나가 recorded Run을 support할 수 있다.
- Status/blocker read는 missing authority를 설명하지만 authority 자체가 되지 않는다.
- Close-blocker check는 full close semantics나 generated close report 없이 필요한 지원 기록이 없음을 보고할 수 있다.

## MVP-1에 남는 것

MVP-1 사용자 작업 루프는 이 checkpoint 이후에 시작합니다. 거기에는 평소 말로 시작/이어가기, work-shape classification, scope/non-goals/success criteria, minimal user judgment, evidence summary, 사용자에게 보이는 close result/blocker display, 다음 안전한 행동, 잔여 위험 표시, final-acceptance blocker, compatible residual-risk acceptance judgment가 있을 때의 accepted-risk close, localized label이 canonical state가 아님을 증명하는 display-label check가 추가됩니다.

[MVP-1 사용자 작업 루프](mvp-user-work-loop.md)를 사용합니다.
