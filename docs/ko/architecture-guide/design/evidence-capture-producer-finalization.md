# Evidence capture 생산자 확정 설계

## 목적

이 설계는 capture intent, source fulfillment, receipt 검증, evidence fact 획득,
producer finalization, Run commit 사이의 현재 구현 경계를 설명합니다.

## 설계

`volicord.prepare_evidence_capture`는 Core에서 planning되고 한도가 있는 capture
intent를 commit합니다. CLI fulfillment 코드는 담당 source 경로를 실행하거나
상관시키고, Store는 immutable receipt와 content-bound staging data를 씁니다.
`record_run`은 `evidence_facts.rs`를 통해 엄격한 현재 intent와 receipt fact를
읽고 `artifact.rs`에 artifact source 검증을 위임하며, typed fact를 집중 evidence
policy 모듈로 평가한 뒤 Run과 같은 grouped Store mutation에 producer finalization을
포함합니다.

Observation과 producer authority는 구분됩니다. 저장된 source observation은 현재 Core
policy가 전체 binding을 받아들이고 Run commit이 성공하기 전까지 producer가 아닙니다.

## 불변 조건

- Intent는 현재 project, Task, Change Unit, scope, workspace, target, source,
  Connection, digest, 시간 좌표에 결속됩니다.
- Receipt는 immutable하고 한도가 있으며 intent와 source claim 하나에 content-bound
  상태입니다.
- Evidence fact는 한 번 획득하여 typed policy input으로 전달합니다.
- Producer finalization과 Run persistence는 함께 성공하거나 rollback됩니다.
- Replay는 원래 결과를 반환하며 producer를 두 번 확정하지 않습니다.
- Guard prompt capture는 observation으로 남으며 receipt나 user-owned resolution을
  대신하지 않습니다.

## 책임 경계

Core 메서드 코드는 요청을 검증하고 집중 담당자를 조율하며 메서드 응답을 구성합니다.
`evidence_facts.rs`는 재사용 가능한 typed fact 취득을 담당하고, `artifact.rs`는
재사용 가능한 artifact source 검증을 담당하며, Core evidence policy 모듈은 typed
fact에 대한 provenance, binding, target, relevance, close-readiness 평가를 담당합니다.
CLI fulfillment는 command 또는 tool-source collection을 담당합니다. Store는 intent,
receipt, staging, producer, Run persistence를 담당합니다.

## 실행 흐름

1. Core가 evidence-capture intent를 planning하고 commit합니다.
2. 지원되는 local source가 intent를 다시 검증하고 Store를 통해 한도가 있는 receipt를
   기록합니다.
3. `record_run`이 현재 intent, receipt, artifact, target fact를 읽습니다.
4. 집중 Core policy 모듈이 SQL read를 담당하지 않으면서 provenance와 binding을
   평가합니다.
5. 메서드 planner가 Run과 producer mutation을 만듭니다.
6. Store가 immediate transaction 하나에서 producer finalization과 Run 상태를
   적용합니다.

## 실패 동작

입력이 없거나, 만료되었거나, 오래되었거나, 경계를 넘거나, 재사용되었거나,
손상되었거나, digest가 다르거나, 불완전하면 producer authority를 만들기 전에
실패합니다. Transaction이 실패하면 intent와 observation에는 finalized producer가
남지 않습니다. 구현은 잘못된 현재 담당 record를 더 약한 성공 source로 낮추지
않습니다.

## 범위 제외

이 설계는 공개 지원 source, evidence eligibility, receipt field, 정확한 저장 효과,
proof strength를 정의하지 않습니다. Fixture, command output, host observation을 외부
attestation으로 취급하지 않습니다.

## 구현 경로

- [`crates/volicord-core/src/methods/prepare_evidence_capture.rs`](../../../../crates/volicord-core/src/methods/prepare_evidence_capture.rs)와
  [`record_run.rs`](../../../../crates/volicord-core/src/methods/record_run.rs):
  요청별 메서드 조율과 응답 구성.
- [`crates/volicord-core/src/evidence_facts.rs`](../../../../crates/volicord-core/src/evidence_facts.rs)와
  [`artifact.rs`](../../../../crates/volicord-core/src/artifact.rs):
  공유 typed fact 취득과 artifact source 검증.
- [`crates/volicord-core/src/policy/`](../../../../crates/volicord-core/src/policy/):
  집중 evidence provenance, binding, relevance, target, close-readiness policy.
- [`crates/volicord-store/src/evidence_capture.rs`](../../../../crates/volicord-store/src/evidence_capture.rs)와
  [`core_pipeline/evidence.rs`](../../../../crates/volicord-store/src/core_pipeline/evidence.rs):
  intent, receipt, producer, grouped mutation persistence.
- [`crates/volicord-cli/src/evidence_command.rs`](../../../../crates/volicord-cli/src/evidence_command.rs):
  local source fulfillment.

## 참조 담당 문서

정확한 동작은 [Evidence Capture 준비](../../reference/api/method-prepare-evidence-capture.md),
[Run 기록](../../reference/api/method-record-run.md),
[Core 모델](../../reference/core-model.md),
[저장소 기록](../../reference/storage-records.md),
[저장 효과](../../reference/storage-effects.md),
[보안](../../reference/security.md)에 남습니다.
