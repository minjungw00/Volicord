# 관찰 신뢰도 경계 설계

## 목적

이 설계는 현재 Guard와 Store 아키텍처에서 structured path fact, uncertain host
observation, reconciliation state, operational diagnostics를 구분하는 방식을 설명합니다.

## 설계

Guard decoding은 typed host-neutral outcome을 만듭니다. 정확하고 호환되는 path fact는
담당 pre-action policy 경로에 들어갈 수 있지만 불확실한 command 또는 repository
observation은 deterministic post-action comparison이 Product Repository path를 확인할
때까지 suspected 상태로 남습니다. Store는 Unrecorded Change state를 shared structured
diagnostics와 별도로 지속 저장합니다.

Diagnostic identity는 closed domain diagnostic kind와 shared typed subject identity가
선택합니다. Occurrence finding은 insert-only이고 current condition은 immutable
`CurrentDiagnosticKey`와 replaceable snapshot을 사용합니다. Renderer는 summary prose를
분류하지 않고 typed field와 action을 선택합니다.

## 불변 조건

- Suspected observation은 묵시적으로 confirmed authority가 되지 않습니다.
- Suppression은 정확한 owner-defined match만 제거합니다.
- 일부만 관찰했거나 unavailable 상태를 complete로 보고하지 않습니다.
- Prompt capture는 observation을 기록하며 사용자 답변을 기록하지 않습니다.
- Diagnostic lifecycle, identity, cause edge, action은 persistence와 rendering 전에 typed
  상태입니다.
- Diagnostic finding은 condition을 설명하며 reconciliation이나 close-readiness state를
  대신하지 않습니다.

## 책임 경계

CLI Guard 모듈은 host input을 decode하고 host output을 투영합니다. Core policy는 write
및 reconciliation 해석을 담당합니다. Store는 Guard event, Unrecorded Change,
structured finding lifecycle, cause graph를 담당합니다. Store는 영속 observation의
status, confidence, Product Repository 경로, typed object, actor, timestamp를
엄격하게 decode한 뒤 reconciliation record를 Core에 전달합니다. `volicord-types`는
dependency-safe diagnostic identity와 report shape를 담당하고 CLI 및 MCP domain
모듈은 자신의 failure를 exhaustive하게 변환합니다.

## 실행 흐름

1. Guard adapter가 host event 하나를 typed neutral outcome으로 decode합니다.
2. 호환되는 structured fact는 해당 policy 경로에 들어가고 incompatible input은 꾸며
   낸 policy result 없이 observation으로 남습니다.
3. Post-action comparison이 suspected 또는 confirmed Unrecorded Change state를
   기록합니다.
4. Reconciliation이 deterministic coverage를 평가하거나 user action을 요청합니다.
5. Domain failure가 typed diagnostic finding으로 투영됩니다.
6. Store가 occurrence를 삽입하거나 current-condition snapshot을 reconcile하고 한도가
   있는 cause graph를 검증합니다.

## 실패 동작

Unavailable persistence, incomplete suppression, invalid host payload, unknown diagnostic
identity, missing cause row, cycle, corrupt stored fact는 명시적인 typed failure로
남습니다. 구현은 missing observation을 빈 successful result로 바꾸거나 사람용 text에서
remediation을 추론하지 않습니다.

## 범위 제외

이 설계는 actor identity, OS sandboxing, complete observability, write prevention,
correctness를 주장하지 않습니다. Public confidence value, reconciliation effect,
diagnostic code, Guard suppression contract도 정의하지 않습니다.

## 구현 경로

- [`crates/volicord-cli/src/guard_command/`](../../../../crates/volicord-cli/src/guard_command/)와
  [`operational_diagnostics/`](../../../../crates/volicord-cli/src/operational_diagnostics/):
  typed Guard adaptation과 CLI diagnostic projection.
- [`crates/volicord-core/src/methods/reconcile_changes.rs`](../../../../crates/volicord-core/src/methods/reconcile_changes.rs):
  현재 reconciliation planning.
- [`crates/volicord-store/src/guards.rs`](../../../../crates/volicord-store/src/guards.rs)와
  [`core_pipeline/reconciliation.rs`](../../../../crates/volicord-store/src/core_pipeline/reconciliation.rs):
  observation 및 reconciliation record.
- [`crates/volicord-store/src/diagnostic_findings/`](../../../../crates/volicord-store/src/diagnostic_findings/)와
  [`crates/volicord-types/src/diagnostics.rs`](../../../../crates/volicord-types/src/diagnostics.rs):
  lifecycle-aware persistence와 shared typed identity.

## 참조 담당 문서

정확한 동작은 [Guard suppression](../../reference/guard-suppression.md),
[변경 조정](../../reference/api/method-reconcile-changes.md),
[저장소 기록](../../reference/storage-records.md),
[실패 모델](../../reference/failure-model.md),
[보안](../../reference/security.md)에 남습니다.
