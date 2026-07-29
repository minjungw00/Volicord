# 상태 결속 Write Ticket 유효성 설계

## 목적

이 설계는 구현이 현재 Core와 Store fact를 기준으로 Write Ticket을 구성하고, 평가하고,
재사용하고, 무효화하고, 소비하는 위치를 설명합니다.

## 설계

`prepare_write`는 현재 Task, Change Unit, scope, workspace, approval, workflow-policy,
normalized path fact를 읽습니다. 집중된 `write_ticket/` 담당자는 이 책임을
명시적으로 구분합니다. `read_model.rs`는 typed ticket, Task, 정규화된 workflow
policy, 현재 UserAction resolution, 증거 fact를 취득합니다. `selection.rs`는
typed candidate 중 하나를 선택합니다. `approval.rs`는 정규 Write Ticket 승인
요구사항을 구성하고, 불변 조건을 지키는 현재 민감 승인 집합을 파생하며, Store가
검증한 영속 승인 근거를 typed 사유가 있는 `NotRequired`, `Current`, `Changed`로
평가하는 단일 담당자입니다. `current_validity.rs`는 Store 접근 없이 제공받은
평가와 현재 권한 fact를 effective status와 invalidation으로 변환합니다.
`summary.rs`는 candidate를 선택하거나 policy를 다시 평가하지 않고 이미 평가된
ticket과 제공된 증거를 adapter-neutral summary로 변환합니다. `service.rs`는 이
완전한 영속 평가 use case만 좁게 조율합니다.
`planning.rs`는 집중된 `PrepareWriteInput`을 평가하고 typed 의미 판단 사유,
관련 record identity, 후보 mutation, 별도의 비영속 `PlannedWriteTicketDraft`를
반환합니다. 공개 request envelope, dry-run intent, 응답 state version, durable ID
generator는 받지 않습니다. 커밋되는 새 발급에서는 공개 메서드가 영속 ticket ID,
승인 참조 projection state version, basis state version을 제공합니다.
`approval.rs`가 만든 typed 비어 있지 않은 승인 근거가 state-version이 있는
`UserActionResolutionRef` 값을 구성하면서 검증된 `PlannedWriteTicket` 하나를
구체화합니다. 이 값이 응답 projection과 완전한 typed Store insertion을 함께
제공합니다. Dry run은 구체화 전에 메서드 경계에서 끝납니다.
`semantic.rs`는 planned와 stored 형태가 공유하는 불변 ticket 의미를 노출하지만,
`WriteTicketEvaluationIdentity`는 발급 예정 identity와 영속 identity를 명시적으로
구분합니다.
보호 대상 Record Run에서는 `write_ticket/admission.rs`가 실제 평가하는 정확한
typed operation, Task, Change Unit, Git workspace, 관찰 시각, observed-change,
현재 policy fact를 받아 승인된 attempt scope 또는 typed 의미 승인 오류를
반환합니다.
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
구성과 평가, 선택, 현재 유효성은 집중된 순수 의미 policy입니다. Summary
projection은 평가된 typed 상태, state-version 및 display fact, 증거 fact만 받으며
Store를 읽거나 workflow, UserAction, 승인 policy를 다시 계산할 수 없습니다. 좁은
service는 이 담당자들을 조율하고 평가 완료 ticket 상태를 소비자에게 제공합니다.
Record Run planner는 의미 fact를 제공하며 저장된 path JSON을 해석하거나 ticket
승인 정책을 독립적으로 구성하지 않습니다. Store는 물리
ticket row를 비공개로 유지하고 status, validity basis, attempt scope, Product
Repository 경로 모음, timestamp, 중복 owner coordinate를 엄격하게 decode한 뒤
`StoredWriteTicket`을 반환합니다. 물리 field 사이 관계는 폐쇄형 Write Ticket
aggregate invariant로 검증하며, 여기에는 approval reference owner 일치와 완전한
resolution identity 고유성이 포함됩니다. Core는 ID 없는 draft를 구성할 때 의미 planning
invariant를 검증하고, 메서드가 `PlannedWriteTicket`을 구체화할 때 identity와
state-version 의존 invariant를 검증합니다. 이 검증은 Store가 담당하는 영속 물리
검증과 구분됩니다. Planned 발급, stored 상태, projected post-consumption 상태는 의미
view만 공유하며 실제 identity는 그대로 구분합니다. Store는 ticket query,
invalidation persistence, consumption mutation도 담당합니다. Workflow policy
영속화는 검증된 record에서 만든 집중된 typed authority view만 받으며 ticket row를
query하거나 decode하지 않습니다. Guard는 observation을 제공하지만 ticket basis를
넓히지 않습니다.

## 실행 흐름

1. 공통 preflight가 actor, operation category, project, request shape를 검증합니다.
2. `prepare_write`가 현재 work, policy, workspace, path, approval fact를 읽습니다.
3. Core policy가 normalized current write-authority basis를 계산합니다.
4. Write Ticket 읽기 경계가 typed candidate와 각 active candidate에 필요한 현재
   fact를 읽습니다.
5. `approval.rs`가 각 candidate의 현재 typed 승인 집합과 요구사항을 구성하고 단일
   의미 평가를 만듭니다. 현재 유효성 policy가 이를 소비한 뒤 순수 선택 policy가
   현재 우선순위와 동률 해소 규칙을 적용합니다.
6. 상태 summary projection, 닫기 준비 상태, CLI guard context는 이 평가에서 파생된
   평가 완료 ticket 상태를 받습니다. Summary는 선택된 ticket과 제공된 증거 fact를
   Store 또는 policy 접근 없이 변환하며, 닫기 준비 상태와 CLI guard는 승인
   현재성을 다시 계산하지 않습니다.
7. Write Ticket planning은 dry-run intent를 보거나 공개 ref를 구성하지 않고
   reuse 또는 ID 없는 새 발급 draft, typed 판단 사유, 관련 의미 identity, 후보
   mutation을 반환합니다.
8. `dry-run`이면 `prepare_write`가 미리보기를 투영하고 끝냅니다. 커밋되는 새
   발급이면 영속 ID를 할당하고 `PlannedWriteTicket` 하나를 구체화하며, 그 값에서
   응답 projection과 `WriteTicketInsert`를 파생합니다. Reuse는
   `StoredWriteTicket`을 읽습니다.
9. Record Run에서는 `write_ticket/admission.rs`가 같은 승인 평가를 소비한 뒤
   operation별 호환성 검사를 적용하고 승인된 scope를 반환합니다.
10. Store가 보호 대상 mutation과 함께 ticket consumption을 commit합니다.

## 실패 동작

Current work 부재, stale 또는 corrupt policy, path normalization failure, typed
approval change, workspace mismatch, explicit revocation, incompatible basis, ambiguous
ticket selection은 partial protected effect 없이 reuse 또는 consumption을
막습니다. 평가는 새로 필요해진 승인, 현재 resolution 부재, 승인 범위 변경, 더
이상 현재 상태가 아닌 근거 resolution을 구분합니다. Exact replay는 ticket을 다시
소비하지 않습니다.
형식이 잘못된 물리 field와 영속 필드 간 불일치는 Store corruption입니다. 내부적으로
유효한 typed ticket을 기준으로 판단한 expiry, path, operation, current-policy
mismatch는 의미 policy 결과이며 영속 corruption이 아닙니다.

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
  typed fact 취득, 정규 승인 요구사항과 현재 집합 구성, typed 승인 평가, 순수
  candidate 선택 및 현재 유효성 평가, 의미 발급 draft 계획, 검증된
  `PlannedWriteTicket` 구체화, 순수 summary projection, 좁은 영속 summary 조율,
  보호 대상 Record Run 승인.
- [`crates/volicord-core/src/write_ticket/tests/read_model_service.rs`](../../../../crates/volicord-core/src/write_ticket/tests/read_model_service.rs):
  집중된 Store 기반 fact 취득 및 영속 summary service coverage.
- [`crates/volicord-core/src/write_ticket/tests/record_run_admission.rs`](../../../../crates/volicord-core/src/write_ticket/tests/record_run_admission.rs):
  집중된 Record Run ticket 승인 및 무효과 거절 coverage.
- [`crates/volicord-types/src/product_path.rs`](../../../../crates/volicord-types/src/product_path.rs):
  공유 typed 제품 경로 정규화와 포함 관계.
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
