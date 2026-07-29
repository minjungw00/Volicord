# 상태 결속 Write Ticket 유효성 설계

## 목적

이 설계는 구현이 현재 Core와 Store fact를 기준으로 Write Ticket을 구성하고, 평가하고,
재사용하고, 무효화하고, 소비하는 위치를 설명합니다.

## 설계

`prepare_write`는 현재 Task, Change Unit, scope, workspace, approval, workflow-policy,
normalized path fact를 읽습니다. 집중된 `write_ticket/` 담당자는 이 책임을
명시적으로 구분합니다. `read_model.rs`는 typed stored ticket, Task, 정규화된
workflow policy, 현재 UserAction resolution, 증거 fact를 취득합니다.
`current_validity.rs`는 Store가 검증한 각 record를 먼저 terminal stored 평가 또는
active stored candidate로 변환합니다. Invalidated, consumed, revoked record는 이
경계에서 평가를 끝내며 active candidate만 현재 authority와 approval fact를 요청할
수 있습니다. 이후 active candidate를 `ReusableStoredWriteTicket` 또는 invalidated
stored 상태로 변환합니다. `approval.rs`는 정규 Write Ticket 승인 요구사항을
구성하고, 불변 조건을 지키는 현재 민감 승인 집합을 파생하며, Store가 검증한 영속
승인 근거를 typed 사유가 있는 `NotRequired`, `Current`, `Changed`로 평가하는 단일
담당자입니다. `selection.rs`는 완결된 `StoredWriteTicketEvaluation` 값만 선택합니다.
`summary.rs`는 planned 입력과 stored 입력을 구분하므로 발급 예정 ticket이 stored
평가처럼 가장할 수 없습니다. `service.rs`는 현재 fact를 읽기 전에 terminal record와
active record를 나누고, stored 결과 하나를 선택한 뒤 그 결과의 증거만 읽는 영속
흐름을 조율합니다.
`planning.rs`는 집중된 `PrepareWriteInput`을 평가하고 typed 의미 판단 사유,
관련 record identity, 공통 fact, 후보 mutation fact와 정확히 하나의
`PrepareWriteTicketPlan` 분기를 반환합니다. 분기는
`Issue(PlannedWriteTicketDraft)`, `Reuse(ReusableStoredWriteTicket)`,
`NoTicket(WriteDecisionPathFacts)` 가운데 하나입니다. 발급 draft와 재사용 가능한
stored ticket은 각각 불변 `WriteTicketPathScope` 하나를 노출하고, ticket에 붙지
않은 판단 경로는 ticket 없음 분기만 담습니다. Planner는 공개 request envelope,
dry-run intent, 응답 state version, durable ID generator를 받지 않습니다.

공개 메서드는 이 폐쇄형 계획 분기에서 dry run을 투영합니다. 커밋 호출은 분기를
발급됨, 재사용됨, 없음 가운데 하나인 `MaterializedPrepareWriteTicket`으로
바꿉니다. 발급에서는 메서드가 영속 ticket ID, 승인 참조 projection state version,
basis state version을 제공합니다. `approval.rs`가 만든 typed 비어 있지 않은 승인
근거가 state-version이 있는 `UserActionResolutionRef` 값을 구성하면서 검증된
`PlannedWriteTicket` 하나를 구체화합니다. 발급 plan은 중첩 및 최상위 응답 fact와
완전한 typed Store insertion을 제공합니다. 재사용 응답 fact는 재사용 가능한
stored ticket에서 나오며, 없음 분기는 ticket identity나 insertion을 담지 않습니다.
`semantic.rs`는 planned와 stored 형태가 공유하는 불변 ticket 의미만 노출합니다.
Planned lifecycle identity와 stored lifecycle identity는 각자 타입에 남고, 평가된
모든 stored 상태는 필수 `WriteTicketId`를 가집니다.
보호 대상 Record Run에서 planner는 선택한 물리 active record를 active candidate로
먼저 변환합니다. Record Run 평가는 해당 시도의 정확한 typed operation, Task,
Change Unit, Git workspace, observed-change, 현재 policy fact를 검사해 정확한
attempt 호환성 증명을 만들고, 현재 유효성 평가는
`ReusableStoredWriteTicket`임을 증명합니다. `write_ticket/admission.rs`는 서로
일치하는 두 증명을 `AdmissibleStoredWriteTicket`으로 결합하며, mutation
planning과 consumption에는 이 ticket 타입만 전달됩니다.
`core_pipeline/write_tickets.rs`만 물리 ticket 테이블, column, row projection,
정규 decoder, 영속 불변 조건, 엄격한 일반 및 transaction 범위 읽기, 집중된 typed
authority view, grouped mutation 적용을 담당합니다. Decoder는 opaque
`StoredWriteTicket`을 반환합니다. Core와 adapter는 의미 accessor를 볼 수 있지만
비공개 field를 구성하거나 변경하거나 destructuring할 수 없습니다.

Ticket lookup은 structural precondition 뒤에만 실행합니다. Reuse는 관련 없는 global
state counter에 의존하지 않고 stored basis와 current typed fact를 비교합니다. 보호된
Run mutation은 Run 및 관련 effect와 같은 Store commit에서 선택한 ticket을 소비합니다.

## 불변 조건

- Ticket은 현재 work 및 write-authority basis 하나에 결속됩니다.
- Ticket ID, stored status, age만으로 current validity를 성립시키지 않습니다.
- 모든 stored 평가는 null이 아닌 영속 `WriteTicketId`를 가집니다.
- Planned 발급은 stored lifecycle 평가의 variant가 아닙니다.
- Prepare Write ticket 계획과 구체화는 각각 발급·재사용·ticket 없음 가운데 정확히
  하나인 폐쇄형 분기를 선택합니다.
- Planned ticket과 stored ticket은 각각 불변 조건을 지키는
  `WriteTicketPathScope` 하나를 소유하며, 분기 옆에 병렬 ticket 경로 배열을 두지
  않습니다.
- 발급 plan은 응답과 영속화의 단일 원본이고, 재사용 ticket은 해당 응답의 단일
  원본이며, ticket 없음만 ticket에 붙지 않은 판단 경로를 소유합니다.
- Terminal stored 상태는 active currentness 또는 admission 로직에 들어갈 수 없습니다.
- Reuse는 `ReusableStoredWriteTicket`만 받고 보호 대상 mutation은
  `AdmissibleStoredWriteTicket`만 받습니다.
- Structural request와 current Change Unit validation이 lookup보다 먼저입니다.
- Reuse는 path, approval, operation, authority를 넓히지 않습니다.
- `approval.rs`만 현재 민감 승인 identity 집합, Write Ticket 승인 요구사항, 의미
  평가를 구성합니다.
- summary 평가, reuse, Record Run 승인, 닫기 준비 상태, CLI guard context는 이
  평가 또는 여기서 파생된 평가 완료 ticket 상태를 소비하며 프로젝트, `Task`,
  UserAction resolution identity를 독립적으로 비교하지 않습니다. Produced state
  version은 참조 metadata로 남습니다.
- Store는 의미 평가 전에 승인 참조 owner 불일치와 완전한 identity 중복을
  거절합니다.
- 관련 mismatch는 active ticket을 사용할 수 없게 하지만 관련 없는 state change는
  그렇지 않습니다.
- Successful consumption은 보호 대상 committed mutation과 atomic하게 일어납니다.

## 책임 경계

Core 메서드는 요청별 조율과 응답 구성을 담당합니다. Prepare Write에서는 dry-run
분기 선택, 영속 ID 할당, state-version이 있는 reference, 보장 표시, typed
planning·Store·UserAction 실패를 공개 `PlanError` 분기로 바꾸는 작업도 여기에
포함됩니다. 집중된 Write Ticket 읽기 경계는 typed fact 취득만 담당합니다. 승인
구성과 평가, stored 전용 선택, 현재 유효성은 집중된 순수 의미 policy입니다.
Planned summary projection은 `PlannedWriteTicket`을 받고 stored summary projection은
`StoredWriteTicketEvaluation`과 제공된 증거를 받습니다. 어느 쪽도 Store를 읽거나
workflow, UserAction, 승인 policy를 다시 계산할 수 없습니다. 좁은 service는
terminal 사전 평가, active에 한정한 현재 fact 읽기, 선택, 증거 읽기, stored
projection을 조율합니다. Record Run planner는 typed reusable ticket과 일치하는
정확한 attempt 호환성 증명을 admission에 전달하고 반환된 admissible ticket만
유지합니다. 저장된 path JSON을 해석하거나 ticket 승인 policy를 독립적으로
구성하지 않습니다. Store는 물리
ticket row를 비공개로 유지하고 status, validity basis, attempt scope, Product
Repository 경로 모음, timestamp, 중복 owner coordinate를 엄격하게 decode한 뒤
`StoredWriteTicket`을 반환합니다. 물리 field 사이 관계는 폐쇄형 Write Ticket
aggregate invariant로 검증하며, 여기에는 approval reference owner 일치와 완전한
resolution identity 고유성이 포함됩니다. Core는 ID 없는 draft를 구성할 때 의미 planning
invariant를 검증하고, 메서드가 `PlannedWriteTicket`을 구체화할 때 identity와
state-version 의존 invariant를 검증합니다. 이 검증은 Store가 담당하는 영속 물리
검증과 구분됩니다. `WriteTicketPathScope`는 두 형태를 노출하기 전에 typed 경로
고유성과 허용·거부 분리 조건을 검증하고, Core와 Store는 lifecycle별 필드 간 검사를
유지합니다. Planned 발급과 각 stored lifecycle 상태는 불변 의미 fact만 공유합니다.
Stored 평가, 선택, currentness, reuse, admission, consumption, summary
projection은 실제 영속 identity를 유지합니다. Store는 ticket query,
invalidation persistence, consumption mutation도 담당합니다. Workflow policy
영속화는 검증된 record에서 만든 집중된 typed authority view만 받으며 ticket row를
query하거나 decode하지 않습니다. Guard는 observation을 제공하지만 ticket basis를
넓히지 않습니다.

## 실행 흐름

1. 공통 preflight가 actor, operation category, project, request shape를 검증합니다.
2. `prepare_write`가 현재 work, policy, workspace, path, approval fact를 읽습니다.
3. Core policy가 normalized current write-authority basis를 계산합니다.
4. Write Ticket 읽기 경계가 모든 stored record를 사전 평가합니다. Terminal record는
   즉시 완결된 typed 결과가 되고 active record는
   `ActiveStoredWriteTicketCandidate`가 됩니다.
5. Active candidate가 있을 때만 service가 현재 Task, workflow policy, UserAction
   fact를 읽습니다. `approval.rs`가 각 active candidate의 의미 평가를 만들고 현재
   유효성 policy가 reusable 또는 invalidated active 결과를 만듭니다.
6. 순수 선택은 완성된 stored 평가만 대상으로 합니다. Service는 선택 뒤 해당
   ticket의 증거를 읽고 stored summary를 투영합니다. 닫기 준비 상태와 CLI guard는
   같은 평가 완료 stored 상태를 받으며 승인 현재성을 다시 계산하지 않습니다.
7. Write Ticket planning은 dry-run intent를 보거나 공개 ref를 구성하지 않고 공통
   fact, mutation fact, 발급·재사용·ticket 없음 가운데 하나인 폐쇄형 분기를
   반환합니다.
8. `dry-run`이면 `prepare_write`가 해당 분기에서 미리보기를 투영하고 끝냅니다.
   커밋에서는 분기를 발급됨·재사용됨·없음으로 보존합니다. 발급됨은 영속 ID를
   할당하고 `PlannedWriteTicket` 하나를 구체화하며, 그 값에서 중첩 결과, 최상위
   identity와 경로, planned summary, `WriteTicketInsert`를 파생합니다. 재사용됨은
   `ReusableStoredWriteTicket`에서 결과와 stored summary를 파생하고, 없음은 판단
   경로만 파생합니다.
9. Record Run에서는 현재 유효성 평가가 `ReusableStoredWriteTicket`임을 증명하는
   동안 평가가 정확한 attempt 호환성 증명을 만듭니다.
   `write_ticket/admission.rs`는 서로 일치하는 두 증명을 결합해
   `AdmissibleStoredWriteTicket`을 반환합니다.
10. Store가 이 admissible ticket의 consumption을 보호 대상 mutation과 함께
    commit합니다.

## 실패 동작

Current work 부재, stale 또는 corrupt policy, path normalization failure, typed
approval change, workspace mismatch, explicit revocation, incompatible basis, ambiguous
ticket selection은 partial protected effect 없이 reuse 또는 consumption을
막습니다. 평가는 새로 필요해진 승인, 현재 resolution 부재, 승인 범위 변경, 더
이상 현재 상태가 아닌 근거 resolution을 구분합니다. Exact replay는 ticket을 다시
소비하지 않습니다.
형식이 잘못된 물리 field와 영속 필드 간 불일치는 Store corruption입니다. 확인된
Core 상태 변환 실패는 panic 기반 narrowing 경로가 아니라 invariant failure입니다.
내부적으로 유효한 typed ticket을 기준으로 판단한 expiry, path, operation,
current-policy mismatch는 의미 policy 결과이며 영속 corruption이 아닙니다.

## 범위 제외

이 설계는 Write Ticket 제품 의미, 공개 request field, timeout policy, invalidation
value, storage effect, user approval, OS write enforcement를 정의하지 않습니다. Ticket은
actor identity나 transferable capability가 아닙니다.

## 구현 경로

- [`crates/volicord-core/src/methods/prepare_write.rs`](../../../../crates/volicord-core/src/methods/prepare_write.rs)와
  [`record_run.rs`](../../../../crates/volicord-core/src/methods/record_run.rs):
  요청별 issue/reuse와 protected consumption 조율.
- [`crates/volicord-core/src/write_ticket/`](../../../../crates/volicord-core/src/write_ticket/)와
  [`workflow.rs`](../../../../crates/volicord-core/src/policy/workflow.rs):
  typed fact 취득, 정규 승인 요구사항과 현재 집합 구성, typed 승인 평가, terminal
  사전 평가, active에 한정한 현재 유효성 평가, stored 전용 선택, 의미 발급 draft
  계획, 검증된 `PlannedWriteTicket` 구체화, planned와 stored summary의 분리된
  projection, 좁은 영속 summary 조율, reusable에서 admissible로 이어지는 보호 대상
  Record Run 승인.
- [`crates/volicord-core/src/write_ticket/tests/read_model_service.rs`](../../../../crates/volicord-core/src/write_ticket/tests/read_model_service.rs):
  집중된 Store 기반 fact 취득 및 영속 summary service coverage.
- [`crates/volicord-core/src/write_ticket/tests/record_run_admission.rs`](../../../../crates/volicord-core/src/write_ticket/tests/record_run_admission.rs):
  집중된 Record Run ticket 승인 및 무효과 거절 coverage.
- [`crates/volicord-types/src/product_path.rs`](../../../../crates/volicord-types/src/product_path.rs):
  공유 typed 제품 경로 정규화와 포함 관계, 불변 `WriteTicketPathScope`의 고유성과
  분리 조건.
- [`crates/volicord-store/src/core_pipeline/write_tickets.rs`](../../../../crates/volicord-store/src/core_pipeline/write_tickets.rs):
  물리 소유권, opaque `StoredWriteTicket`으로의 정규 decoding, typed insertion
  직렬화, authority view, query, grouped mutation 적용.
- [`crates/volicord-store/src/workflow_records.rs`](../../../../crates/volicord-store/src/workflow_records.rs):
  workflow policy 영속화와 typed Write Ticket authority view를 이용한 의미 평가.

## 참조 담당 문서

정확한 동작은 [Core 모델](../../reference/core-model.md),
[쓰기 준비](../../reference/api/method-prepare-write.md),
[Run 기록](../../reference/api/method-record-run.md),
[저장소 기록](../../reference/storage-records.md),
[저장 효과](../../reference/storage-effects.md),
[저장소 버전 관리](../../reference/storage-versioning.md),
[보안](../../reference/security.md)에 남습니다.
