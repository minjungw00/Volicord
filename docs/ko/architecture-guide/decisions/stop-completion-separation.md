# 세션 Stop과 Task 완료는 서로 다른 결과다

## 맥락

호스트 Stop 이벤트는 에이전트 세션이나 turn이 끝난다는 뜻입니다. 현재 `Task`가
완료됐다는 뜻은 아닙니다. 완료되지 않은 모든 Task를 Stop 거부 이유로 취급하면
호스트 생명주기 제어가 닫기 상태와 결합되고, 호스트가 같은 Stop을 반복할 수
있습니다. 다음 행위자가 사용자이거나 나중 세션에서 작업을 이어야 할 때도 정직한
세션 종료를 막습니다.

Volicord는 여전히 모델이 막힌 작업을 완료로 제시하지 못하게 해야 합니다. 이것은
완료 고지의 문제이지 호스트 프로세스를 계속 실행할 이유가 아닙니다.

## 결정

Stop 허용 여부와 완료 주장 허용 여부를 서로 다른 사실로 표현합니다. 관리 Stop
경로는 세션 종료를 허용하고, 가능하면 권한 상태를 새로고침하며, 선택한 Task,
닫기 상태, 다음 행위자, 현재 완료 주장이 허용되는지를 보여 주는 제한된 receipt를
기록합니다.

권한 상태 새로고침이 실패해도 Stop은 끝나며, 결과는 권한 상태를 검증하지 못했다는
사실을 공개합니다. 닫기 차단 사유는 계속 완료 주장을 억제합니다. 범위 밖 또는
권한 없는 쓰기가 결정론적으로 확인되면 별도의 행동 전 집행 경로에서 계속 차단할
수 있습니다.

정확한 Stop 결과, receipt 영속 방식, 차단 사유 상태 보기, 실패 처리 경로, 완료
필드는 [관리 CLI](../../reference/admin-cli.md),
[Core 모델](../../reference/core-model.md), 상태 스키마, Agent Connection, 저장소 참조
담당 문서가 계속 정의합니다.

## 결과

- 사용자 응답 대기, 증거 부족, 다른 닫기 차단 사유가 호스트를 Stop 재시도 반복에
  가두지 않습니다.
- 호스트 어댑터는 생명주기 종료를 확인하면서도 고정 UI와 최종 출력으로 Task가
  미완료임을 계속 공개할 수 있습니다.
- 실제 호스트 테스트는 운영자에게 두 번째 시도를 중단하라고 안내하지 않고 한 번의
  완료된 Stop을 검증합니다.
- Stop receipt는 진단 연속성 기록이며, Task를 끝내는 변경이나 `close_task`의
  대체물이 아닙니다.
- 행동 전 쓰기 집행과 완료 고지는 allow/deny 값 하나를 과하게 사용하지 않고
  발전할 수 있습니다.

## 비목표

- Stop 허용은 Task를 닫거나 취소하거나 대체하지 않습니다.
- 증거, 사용자 판단, 수락, 잔여 위험 요구사항을 면제하지 않습니다.
- 결정론적인 행동 전 쓰기 거부를 약화하지 않습니다.
- receipt는 호스트가 이를 표시했거나 모델이 충실히 보고했다는 증명이 아닙니다.

## 거부한 대안

- 닫기 상태가 깨끗해질 때까지 Stop을 거부하는 방식은 세션 생명주기와 Task
  생명주기의 다음 행위자와 시간 범위가 다르므로 거부했습니다.
- 호스트 프로세스 종료를 암묵적 닫기로 취급하는 방식은 Core 닫기 권한과 사용자
  소유 판단을 우회하므로 거부했습니다.
- 미완료 상태를 기록하거나 공개하지 않고 Stop만 허용하는 방식은 세션 종료 후
  정직하게 이어 가기 어렵게 하므로 거부했습니다.

## 관련 구현

- [`crates/volicord-cli/src/guard_command.rs`](../../../../crates/volicord-cli/src/guard_command.rs)와
  Guard 통합 모듈: 호스트 Stop 처리와 receipt 상태 보기.
- [`crates/volicord-core/src/methods/close_task.rs`](../../../../crates/volicord-core/src/methods/close_task.rs):
  호스트 생명주기와 분리된 권위 있는 닫기 상태.
- [`crates/volicord-store/src/guards.rs`](../../../../crates/volicord-store/src/guards.rs):
  영속 Guard 관찰과 진단 receipt.
- [`crates/volicord-mcp/src/stdio.rs`](../../../../crates/volicord-mcp/src/stdio.rs):
  권한 상태 새로고침과 어댑터에서 보이는 다음 행동 상태 보기.

## 관련 테스트와 참조 담당 문서

- [`crates/volicord-cli/tests/guard_command.rs`](../../../../crates/volicord-cli/tests/guard_command.rs),
  [`crates/volicord-cli/tests/live_host_smoke.rs`](../../../../crates/volicord-cli/tests/live_host_smoke.rs),
  [`tests/conformance/baseline.rs`](../../../../tests/conformance/baseline.rs)의 닫기 검증.
- [관리 CLI](../../reference/admin-cli.md),
  [Core 모델](../../reference/core-model.md),
  [`Task` 닫기](../../reference/api/method-close-task.md),
  [API 상태 스키마](../../reference/api/schema-state.md),
  [Agent Connection](../../reference/agent-connection.md),
  [저장소 기록](../../reference/storage-records.md).
