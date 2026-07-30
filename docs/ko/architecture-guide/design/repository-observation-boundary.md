# 저장소 관찰 경계 설계

## 목적

이 설계는 호출 범위 Product Repository 스냅샷, 결정적인 delta, Guard aggregate,
예상 쓰기 일치, 미기록 변경 생성을 위한 현재 구현 경계를 설명합니다.

## 설계

`volicord-platform-fs`는 한도 안에서 안정적인 스냅샷을 capture하고 content와 mode를
반영한 순 경로 전이를 계산합니다. Guard adapter는 정확한 typed Codex hook
상관관계 하나를 decode하고 호출 대상이나 검토된 hint를 제공합니다. Store는 정확한
호스트 도구 호출마다 aggregate row 하나를 소유하고 pre-tool 및 post-tool 변경을
원자적으로 적용합니다.

Core는 Store가 검증한 실제 미기록 변경만 사용합니다. 운영상 관찰 불가 진단은
조정 및 닫기 준비 상태와 분리합니다.

## 불변 조건

- 허용된 write-capable 또는 unknown-effect 호출에는 영속한 안정적 pre-tool
  baseline이 있습니다.
- 완전한 관찰 하나는 정확히 일치하는 pre/post hook 쌍 하나를 사용합니다.
- 결정적인 delta는 호환되는 안정적 스냅샷에서만 계산합니다.
- 예상 쓰기는 정확히 자기 관찰과 완전한 delta에만 일치합니다.
- 미기록 변경은 비어 있지 않은 관찰된 불일치 delta만 담습니다.
- Replay는 저장된 terminal 결과를 사용하고 저장소를 다시 scan하지 않습니다.
- 관찰 결과는 actor identity나 단독 인과관계를 주장하지 않습니다.

## 책임 경계

`volicord-host-contract`는 typed hook 상관관계와 정규 Product Repository 효과
catalog를 담당합니다. `volicord-platform-fs`는 스냅샷 및 delta 관찰을 담당합니다.
CLI Guard 모듈은 호스트 adaptation과 policy projection을 담당합니다.
`volicord-store`는 엄격한 aggregate 영속화, digest 검증, 원자적 pre/post 변경,
정확한 예상 쓰기 일치, 미기록 변경 구체화를 담당합니다. Core는 검증된 미기록
변경 사실의 조정 및 닫기 준비 상태 해석을 담당합니다.

## 실행 흐름

1. Pre-tool adaptation이 정확한 hook 호출을 decode하고 안정적인 baseline을
   capture합니다.
2. Store가 pre-tool 이벤트, 호출 관찰, 정확한 예상 쓰기를 원자적으로 기록합니다.
3. Post-tool adaptation이 같은 호출의 안정적인 결과를 capture합니다.
4. Store가 open 관찰을 원자적으로 검증하고 post-tool 이벤트와 delta를 기록하며
   예상 쓰기를 대조하고 불일치 미기록 변경을 만듭니다.
5. Core가 조정과 닫기 준비 상태를 위해 검증된 미해결 변경을 읽습니다.
6. 정확한 replay가 저장된 terminal 관찰 결과를 반환합니다.

## 실패 동작

Write-capable 및 unknown-effect 호출은 baseline을 capture하거나 원자적으로 영속할 수
없으면 거부합니다. No-product-write 호출은 명시적인 관찰 불가 상태와 함께 계속할
수 있습니다. Baseline이 없거나 충돌하거나 손상됐거나 관찰 불가이면 빈 delta나
미기록 변경으로 바꾸지 않습니다. Transaction 실패는 aggregate 전체를 rollback합니다.

## 범위 제외

이 설계는 공개 메서드 동작, 물리 DDL, 닫힌 값 의미, 보안 보장, 호스트 process exit
동작, actor identity, 완전한 monitoring, OS enforcement를 정의하지 않습니다.

## 구현 경로

- [`crates/volicord-platform-fs/src/repository_observation/`](../../../../crates/volicord-platform-fs/src/repository_observation/):
  안정적인 스냅샷과 결정적인 delta
- [`crates/volicord-cli/src/guard_command/`](../../../../crates/volicord-cli/src/guard_command/):
  typed Codex hook adaptation과 Guard 결과 projection
- [`crates/volicord-store/src/guards.rs`](../../../../crates/volicord-store/src/guards.rs):
  정확한 호스트 상관관계, 저장소 관찰 aggregate, 예상 쓰기, 미기록 변경
- [`crates/volicord-core/src/methods/reconcile_changes.rs`](../../../../crates/volicord-core/src/methods/reconcile_changes.rs)와
  [`close_readiness/`](../../../../crates/volicord-core/src/close_readiness/):
  조정 및 닫기 준비 상태 consumer

## 참조 담당 문서

정확한 동작은
[저장소 관찰](../../reference/repository-observation.md),
[변경 조정](../../reference/api/method-reconcile-changes.md),
[저장소 기록](../../reference/storage-records.md),
[저장소 DDL](../../reference/storage-ddl.md),
[저장 효과](../../reference/storage-effects.md),
[보안](../../reference/security.md)에 남습니다.
