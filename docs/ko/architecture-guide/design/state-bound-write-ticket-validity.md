# 상태 결속 Write Ticket 유효성 설계

## 목적

이 설계는 구현이 현재 Core와 Store fact를 기준으로 Write Ticket을 구성하고, 평가하고,
재사용하고, 무효화하고, 소비하는 위치를 설명합니다.

## 설계

`prepare_write`는 현재 Task, Change Unit, scope, workspace, approval, workflow-policy,
normalized path fact를 읽습니다. `policy/write_ticket.rs`의 집중 Core policy가 ticket
basis를 구성하고 평가합니다. `core_pipeline/write_tickets.rs`는 strict ticket read와
grouped mutation 적용을 담당합니다.

Ticket lookup은 structural precondition 뒤에만 실행합니다. Reuse는 관련 없는 global
state counter에 의존하지 않고 stored basis와 current typed fact를 비교합니다. 보호된
Run mutation은 Run 및 관련 effect와 같은 Store commit에서 선택한 ticket을 소비합니다.

## 불변 조건

- Ticket은 현재 work 및 write-authority basis 하나에 결속됩니다.
- Ticket ID, stored status, age만으로 current validity를 성립시키지 않습니다.
- Structural request와 current Change Unit validation이 lookup보다 먼저입니다.
- Reuse는 path, approval, operation, authority를 넓히지 않습니다.
- 관련 mismatch는 active ticket을 사용할 수 없게 하지만 관련 없는 state change는
  그렇지 않습니다.
- Successful consumption은 보호 대상 committed mutation과 atomic하게 일어납니다.

## 책임 경계

Core 메서드는 request-specific planning을 담당합니다. Core workflow, path, access,
write-ticket policy 모듈은 typed fact에 대한 current authority evaluation을 담당합니다.
Store는 물리 ticket row를 비공개로 유지하고 status, validity basis, attempt scope,
Product Repository 경로 모음, timestamp, 중복 owner coordinate를 엄격하게 decode한
뒤 typed record를 반환합니다. Store는 ticket query, invalidation persistence,
consumption mutation도 담당합니다. Guard는 observation을 제공하지만 ticket basis를
넓히지 않습니다.

## 실행 흐름

1. 공통 preflight가 actor, operation category, project, request shape를 검증합니다.
2. `prepare_write`가 현재 work, policy, workspace, path, approval fact를 읽습니다.
3. Core policy가 normalized current write-authority basis를 계산합니다.
4. Store가 compatible ticket candidate를 반환하고 Core가 reuse 또는 new issuance를
   선택합니다.
5. 이후 보호 대상 operation이 current compatibility evaluation을 반복합니다.
6. Store가 보호 대상 mutation과 함께 ticket consumption을 commit합니다.

## 실패 동작

Current work 부재, stale 또는 corrupt policy, path normalization failure, approval
mismatch, workspace mismatch, explicit revocation, incompatible basis, ambiguous ticket
selection은 partial protected effect 없이 reuse 또는 consumption을 막습니다. Exact
replay는 ticket을 다시 소비하지 않습니다.

## 범위 제외

이 설계는 Write Ticket 제품 의미, 공개 request field, timeout policy, invalidation
value, storage effect, user approval, OS write enforcement를 정의하지 않습니다. Ticket은
actor identity나 transferable capability가 아닙니다.

## 구현 경로

- [`crates/volicord-core/src/methods/prepare_write.rs`](../../../../crates/volicord-core/src/methods/prepare_write.rs)와
  [`record_run.rs`](../../../../crates/volicord-core/src/methods/record_run.rs):
  issue/reuse와 protected consumption planning.
- [`crates/volicord-core/src/policy/write_ticket.rs`](../../../../crates/volicord-core/src/policy/write_ticket.rs)와
  [`workflow.rs`](../../../../crates/volicord-core/src/policy/workflow.rs):
  typed current-basis 평가.
- [`crates/volicord-types/src/product_path.rs`](../../../../crates/volicord-types/src/product_path.rs):
  공유 typed 제품 경로 정규화와 포함 관계.
- [`crates/volicord-store/src/core_pipeline/write_tickets.rs`](../../../../crates/volicord-store/src/core_pipeline/write_tickets.rs):
  strict record, query, grouped mutation 적용.

## 참조 담당 문서

정확한 동작은 [Core 모델](../../reference/core-model.md),
[쓰기 준비](../../reference/api/method-prepare-write.md),
[Run 기록](../../reference/api/method-record-run.md),
[저장소 기록](../../reference/storage-records.md),
[저장 효과](../../reference/storage-effects.md),
[저장소 버전 관리](../../reference/storage-versioning.md),
[보안](../../reference/security.md)에 남습니다.
