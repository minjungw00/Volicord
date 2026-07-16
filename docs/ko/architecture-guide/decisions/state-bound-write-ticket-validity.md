# 쓰기 티켓 유효성은 관련 작업 상태에 결속한다

## 맥락

쓰기 티켓은 제안된 Product Repository 변경 하나를 그 근거가 된 Task, Change Unit,
범위, baseline, workspace, 승인에 연결합니다. 유효성을 프로젝트 전체 상태 카운터나
고정된 짧은 수명에 주로 묶으면 관련 없는 활동이 이 연결을 취소합니다. 읽기 전용
상태 조회, 증거 기록, 관련 없는 사용자 행동 때문에 제안된 쓰기의 근거가 바뀌지
않았는데도 티켓을 다시 받아야 할 수 있습니다.

이런 반복은 권한 경계를 개선하지 않으면서 에이전트 호출만 늘립니다. 또한 실제로
유용한 질문, 즉 관련 사실 중 무엇이 바뀌었는지를 가립니다.

## 결정

쓰기 티켓 유효성을 관련 작업 상태의 명시적인 snapshot에 결속합니다. 영속 근거는
Task, 현재 Change Unit, 범위 개정, baseline, 필요한 경우 workspace 맥락, 권한이
의존하는 승인 기록을 식별합니다.

Core는 티켓을 사용하기 전에 이 근거를 현재 관련 상태와 비교합니다. 호환되고 아직
소비되지 않은 티켓은 그 범위에 포함되는 쓰기 의도에 재사용할 수 있습니다. 기록된
제품 파일 변경이 티켓을 사용하면 소비합니다. 시간 경과만으로 기본 권한 경계를
만들지 않습니다. 프로젝트 소유 정책은 승인 만료와 혼동하지 않는 유휴 제한을
별도로 추가할 수 있습니다.

정확한 근거 필드, 호환 규칙, 무효화 이유, 효과 값, 오류 우선순위, 저장소 제약은
[Core 모델](../../reference/core-model.md),
[쓰기 준비](../../reference/api/method-prepare-write.md),
[실행 기록](../../reference/api/method-record-run.md), 저장소 참조 문서 묶음이 계속
담당합니다.

## 결과

- 읽기 전용 활동과 관련 없는 권한 활동이 호환되는 티켓을 무효화할 필요가
  없어집니다.
- 거부 결과는 일반적인 오래된 카운터만 보여 주지 않고 바뀐 관련 근거를 이름 붙일
  수 있습니다.
- 티켓 재사용은 추측성 쓰기 준비 호출을 줄이면서 제품 변경 Run 하나와의 단일 소비
  관계를 보존합니다.
- 영속 계층에는 호환성을 검증하고 무효화 이유를 설명할 수 있는 구조화된 근거가
  필요합니다.
- 저장소 프로필 전환은 빠진 근거를 재구성한 척하지 않고 이전의 활성 티켓을
  취소하거나 오래된 상태로 표시합니다.

## 비목표

- 쓰기 티켓은 여전히 파일시스템 권한, 잠금, 사용자 수락, 실제 쓰기의 증명이
  아닙니다.
- Task, Change Unit, workspace, 승인 근거를 넘어 티켓을 이전할 수 있게 하지
  않습니다.
- 공개 변경 메서드의 낙관적 동시성 제어를 없애지 않습니다.
- 이 ADR에서 공개 유효성이나 오류 스키마를 정의하지 않습니다.

## 거부한 대안

- 프로젝트 전체 상태 일치를 주 규칙으로 유지하는 방식은 관련 없는 변경이 잘못된
  무효화를 만들기 때문에 거부했습니다.
- 고정된 짧은 수명을 기본으로 유지하는 방식은 시간 경과가 바뀐 권한 사실을
  식별하지 못하므로 거부했습니다.
- 티켓을 무기한 재사용하는 방식은 기록된 쓰기에 영속 소비 관계 하나가 필요하고,
  다음 쓰기 단계에는 새로운 호환성 확인이 필요하므로 거부했습니다.

## 관련 구현

- [`crates/volicord-types/src/schema.rs`](../../../../crates/volicord-types/src/schema.rs):
  공유 쓰기 티켓 결과와 상태 보기.
- [`crates/volicord-core/src/methods/prepare_write.rs`](../../../../crates/volicord-core/src/methods/prepare_write.rs)와
  [`crates/volicord-core/src/methods/record_run.rs`](../../../../crates/volicord-core/src/methods/record_run.rs):
  근거 비교, 재사용, 발급, 소비 계획.
- [`crates/volicord-store/src/schema/project.sql`](../../../../crates/volicord-store/src/schema/project.sql)과
  Store 쓰기 티켓 접근자: 영속 유효성 근거와 소비 제약.

## 관련 테스트와 참조 담당 문서

- [`crates/volicord-core/src/methods/tests/prepare_write.rs`](../../../../crates/volicord-core/src/methods/tests/prepare_write.rs),
  [`crates/volicord-core/src/methods/tests/record_run.rs`](../../../../crates/volicord-core/src/methods/tests/record_run.rs),
  [`crates/volicord-store/tests/storage_ddl_contract.rs`](../../../../crates/volicord-store/tests/storage_ddl_contract.rs),
  [`tests/conformance/baseline.rs`](../../../../tests/conformance/baseline.rs)의 쓰기 티켓
  생명주기 검증.
- [Core 모델](../../reference/core-model.md),
  [쓰기 준비](../../reference/api/method-prepare-write.md),
  [실행 기록](../../reference/api/method-record-run.md),
  [저장소 기록](../../reference/storage-records.md),
  [저장 효과](../../reference/storage-effects.md),
  [저장소 DDL](../../reference/storage-ddl.md),
  [저장소 버전 관리](../../reference/storage-versioning.md).
