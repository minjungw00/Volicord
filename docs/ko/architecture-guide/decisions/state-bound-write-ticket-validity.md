# 쓰기 티켓 유효성은 관련 작업 상태에 결속한다

## 맥락

쓰기 티켓은 제안된 Product Repository 변경 하나를 그 근거가 된 Task, Change Unit,
범위, baseline, workspace, 승인에 연결합니다. 이 연결은 그 쓰기를 허가한 프로젝트
소유 작업 흐름 정책에도 의존합니다. Light 작업에 해당하는 경로,
적용되는 통제 수준, 사전 쓰기 승인 필요 여부, 티켓 사용 가능 시간이 정책 때문에
바뀌면 이전 티켓이 새 권한 경계를 넘어 기존 허가를 이어 가면 안 됩니다.

유효성을 프로젝트 전체 상태 카운터나
고정된 짧은 수명에 주로 묶으면 관련 없는 활동이 이 연결을 취소합니다. 읽기 전용
상태 조회, 증거 기록, 관련 없는 사용자 행동 때문에 제안된 쓰기의 근거가 바뀌지
않았는데도 티켓을 다시 받아야 할 수 있습니다.

이런 반복은 권한 경계를 개선하지 않으면서 에이전트 호출만 늘립니다. 또한 실제로
유용한 질문, 즉 관련 사실 중 무엇이 바뀌었는지를 가립니다.

## 결정

쓰기 티켓 유효성을 관련 작업 상태의 명시적인 snapshot에 결속합니다. 영속 근거는
Task, 현재 Change Unit, 범위 개정, baseline, 필요한 경우 workspace 맥락, 권한이
의존하는 승인 기록을 식별합니다. 또한 현재 권위 프로젝트 정책에서 파생한 정규화된
`write_authority_fingerprint`를 저장합니다.

이 fingerprint는 전체 canonical 정책보다 의도적으로 좁습니다. Canonical 근거에는
`volicord-write-authority-v1` 스키마 식별자, direct와 work 통제 기본값, Light 활성화
여부, 경로 수 제한, 허용·거부 경로 패턴, Light 최종 수락 정책, 쓰기 티켓 유휴 제한
시간이 들어갑니다. 패턴 배열은
canonicalization 전에 정렬하고 중복을 제거하며 canonical JSON을 소문자
`sha256:<hex>` digest로 저장합니다. Core가 쓰기 권한을 결정할 때
사용하지 않는 Detective 동작과 저장소, connection, MCP, host hook, 그 밖의 통합
바인딩은 포함하지 않습니다.

Core는 티켓을 사용하기 전에 이 근거를 현재 관련 상태와 비교합니다. 호환되고 아직
소비되지 않은 티켓은 그 범위에 포함되는 쓰기 의도에 재사용할 수 있습니다. 기록된
제품 파일 변경이 티켓을 사용하면 소비합니다. 시간 경과만으로 기본 권한 경계를
만들지 않습니다. 프로젝트 소유 정책은 승인 만료와 혼동하지 않는 유휴 제한을
별도로 추가할 수 있습니다.

정규화된 쓰기 권한이 이전과 다른 정책을 적용하면 같은 트랜잭션에서 활성 Task에
재평가 표시를 만들고 호환되지 않는 활성 티켓을 `explicit_revoke`로 무효화합니다.
이 처리는 강화와 완화를 모두 보수적으로 다룹니다. Fingerprint가 바뀐 경계를 넘어
호환성을 추론하지 않습니다. 저장된 통제 수준이나 수락 정책을 높일 필요가 없어도
활성 Task 표시는 만듭니다. 따라서 다음 `volicord.prepare_write`는 현재 정책으로
요청 동작과 경로를 다시 평가합니다. 전체 정책 문서가 바뀌거나 다시 적용되었더라도
정규화된 쓰기 권한이 같으면 그 이유만으로 티켓을 무효화하지 않습니다.

협력형 Guard 경로는 정책 결속이 없거나 일치하지 않는 티켓을 활성 후보에서
제외합니다. Core는 `volicord.record_run`에서 현재 정책 결속을 독립적으로 다시
확인하고, Store는 소비 트랜잭션 안에서 한 번 더 확인합니다. 따라서 새 쓰기 준비
결정은 통제를 `sensitive`로 높이고 새로운 민감 동작 승인을 요구할 수 있습니다.
쓰기 뒤의 최종 수락은 별도의 사용자 판단이므로 필요한 사전 쓰기 승인을 대신할 수
없습니다.

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
- 현재 정책 결속이 없는 이전 활성 티켓은 닫힌 방식으로 실패하며 다시 발급해야
  합니다. 이미 소비된 과거 티켓은 소급해서 다시 쓰지 않고 계속 조회할 수 있습니다.
- 결속은 기존 `validity_basis_json` 기록에 들어갑니다. 저장소 프로필은
  `baseline_sqlite_v7`을 유지하며 오프라인 복사 저장소 업그레이드가 필요하지
  않습니다.

## 비목표

- 쓰기 티켓은 여전히 파일시스템 권한, 잠금, 사용자 수락, OS 샌드박스, 변조 방지
  감사 로그, 실제 쓰기의 발생이나 정확성에 대한 증명이 아닙니다.
- Task, Change Unit, workspace, 승인 근거를 넘어 티켓을 이전할 수 있게 하지
  않으며 정책 권한 경계도 넘을 수 없습니다.
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
- [`crates/volicord-store/src/workflow_records.rs`](../../../../crates/volicord-store/src/workflow_records.rs):
  정규화된 쓰기 권한 파생, 정책 적용 재평가, 활성 티켓 원자적 무효화.
- [`crates/volicord-cli/src/guard_command/context.rs`](../../../../crates/volicord-cli/src/guard_command/context.rs)와
  [`crates/volicord-cli/src/guard_command/write_ticket.rs`](../../../../crates/volicord-cli/src/guard_command/write_ticket.rs):
  협력형 Guard 후보 선택과 오래된 정책 진단.
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
