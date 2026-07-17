# Evidence-capture intent와 producer finalization

## 맥락

evidence provenance는 수집 전에 결속해야 하지만, Run은 완전한 owner-defined 결과가
commit될 때만 권한을 가집니다. 관찰 레코드만으로 producer authority가 되면 안 됩니다.

## 결정

`volicord.prepare_evidence_capture`는 메서드 소유 task, change unit, source,
selector, expiry, policy 좌표를 담은 bounded intent를 만듭니다. 지원되는 source는 그
intent에 digest-bound receipt를 기록합니다.

`volicord.record_run`은 현재 intent, receipt, task, change unit, source, expiry,
evidence body를 다시 검증합니다. Producer finalization과 Run commit은 하나의 Store
transaction에서 일어납니다. 실패하거나 거부된 commit은 intent와 관찰에 producer
authority를 부여하지 않습니다.

Guard prompt capture는 관찰일 뿐이며 UserAction을 해결하거나 evidence receipt를
대신할 수 없습니다.

## 결과

- receipt는 project, intent, source, 작업 상태 사이에서 이동할 수 없습니다.
- replay는 원래 immutable 결과를 반환하며 두 번 확정하지 않습니다.
- fixture는 파싱과 상태 전이만 증명하며 외부 아티팩트 지원을 증명하지 않습니다.
- 정확한 schema와 effect는 API와 Storage 소유자에 남습니다.

[Prepare Evidence Capture](../../reference/api/method-prepare-evidence-capture.md),
[Record Run](../../reference/api/method-record-run.md),
[Storage Effects](../../reference/storage-effects.md)를 봅니다.
