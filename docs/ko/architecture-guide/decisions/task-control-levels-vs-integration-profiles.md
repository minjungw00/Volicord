# Task 통제 수준과 통합 프로필은 서로 다른 축이다

## 맥락

Record와 Detective 프로필은 Volicord를 에이전트 호스트에 연결하는 방식을
설명합니다. 호스트 통합, 관찰, 진단 경로를 선택하지만 `Task` 하나의 위험이나
권한 필요 수준을 나타내지는 않습니다.

프로필 값 하나에 두 의미를 함께 넣으면 관련 없는 선택이 결합된 것처럼 보입니다.
Record 설치에서도 민감한 작업을 수행할 수 있고, Detective 설치에서도 읽기 전용
조사를 관찰할 수 있습니다. 또한 가벼운 호스트 설정이 가벼운 작업 경계를 부여한다고
에이전트가 추론하게 만들 수 있습니다.

## 결정

통합 프로필과 Task 통제 수준을 서로 독립적이고 보이는 축으로 유지합니다.

- 통합 프로필은 설치와 Agent Connection의 관심사로 남습니다. 호스트 어댑터와
  관리 설정이 그 구성을 담당합니다.
- Task 통제 수준은 Core가 소유하는 작업 상태의 관심사입니다. Core는 호출자의
  요청, 프로젝트 소유 정책, 현재 적용 범위, 상향이 필요한 사실에서 수준을 정하고
  영속 저장합니다.
- 프로젝트 정책은 요청한 수준을 제한하거나 높일 수 있습니다. Agent Connection은
  요청을 전달할 수 있지만 정책 권한의 출처가 되지는 않습니다.
- 정규화된 쓰기 권한 정책이 바뀌면 저장된 통제 수준과 최종 수락 정책을 높일 필요가
  없어도 활성 Task에 정책 재평가 표시를 만듭니다. 이 경계에서 호환되지 않는 활성
  쓰기 티켓은 무효화합니다.
- 다음 `volicord.prepare_write`는 요청 동작과 경로를 현재 정책으로 해석하고, 충족된
  재평가 표시를 지우며, 요청을 `sensitive`로 높이거나 새로운 민감 동작 승인을 요구할
  수 있습니다.
- 어댑터는 두 사실을 서로 변환하지 않고 상태 보기로 전달합니다.
- 설정이나 나중 요청이 덜 엄격해졌다는 이유만으로 현재 Task의 통제 수준을
  자동으로 낮추지 않습니다.

정확한 공개 타입, 값, 결정 규칙, 영속 방식, 응답 필드는
[Core 모델](../../reference/core-model.md),
[Intake](../../reference/api/method-intake.md), 공개 스키마 담당 문서, 저장소 참조
문서 묶음이 계속 담당합니다.

## 결과

- 설정 문서는 Record와 Detective를 Task 위험 등급처럼 보이게 하지 않고 설명할 수
  있습니다.
- Task 상태 보기는 호스트 프로필과 별개로 통제 수준을 선택한 이유를 보여 줄 수
  있습니다.
- 테스트는 프로필을 위험 사다리로 취급하지 않고 통합 역량과 Task 통제의 조합을
  검증합니다.
- 정규화된 쓰기 권한이 바뀌면 활성 Task를 위한 명시적 재평가 지점을 만들고
  호환되지 않는 활성 티켓을 무효화합니다. 정규화 결과가 같은 권한을 다시 적용하면
  티켓을 유지합니다.
- 이 결속은 기존 쓰기 티켓 유효성 기록을 사용합니다. 저장소 프로필은
  `baseline_sqlite_v7`을 유지하며 이 동작에 저장소 프로필 전환은 없습니다.
- 생성된 호스트 지침이 아니라 Core가 요청 수준과 프로젝트 소유 제약을 대조합니다.

## 비목표

- 이 결정은 Task 통제 값 집합이나 상향 알고리즘을 정의하지 않습니다.
- 통제 수준을 OS 권한, 샌드박스, 에이전트가 지침을 따랐다는 증명으로 만들지
  않습니다.
- Record나 Detective 프로필을 없애거나 이름을 바꾸지 않습니다.
- 에이전트를 프로젝트 작업 정책의 소유자로 만들지 않습니다.
- 호스트 프로필, Guard 결과, 쓰기 뒤의 최종 수락이 필요한 사전 쓰기 승인을 대신하게
  하지 않습니다.

## 거부한 대안

- 결합된 프로필 이름을 더 추가하는 방식은 위험과 호스트 역량 조합마다 설정
  표면이 늘어나므로 거부했습니다.
- 프로젝트 정책 없이 에이전트가 저위험 경로를 고르는 방식은 권한을 요청하는
  주체가 유일한 정책 출처까지 될 수 없으므로 거부했습니다.
- Task 기록 없이 프로젝트 전체 수준 하나만 쓰는 방식은 Task마다 위험이 다르고
  상향 이유를 Task와 함께 확인할 수 있어야 하므로 거부했습니다.

## 관련 구현

- [`crates/volicord-types/src/values.rs`](../../../../crates/volicord-types/src/values.rs)와
  [`crates/volicord-types/src/methods.rs`](../../../../crates/volicord-types/src/methods.rs):
  공유 값 집합과 공개 요청·결과 형태.
- [`crates/volicord-core/src/methods/intake.rs`](../../../../crates/volicord-core/src/methods/intake.rs):
  Task 생성과 Core 소유 통제 수준 선택.
- [`crates/volicord-cli/src/guard_integration/policy.rs`](../../../../crates/volicord-cli/src/guard_integration/policy.rs):
  관리 프로젝트 정책 파싱과 관리 통합.
- [`crates/volicord-core/src/policy/workflow.rs`](../../../../crates/volicord-core/src/policy/workflow.rs):
  Task 작업을 위한 현재 정책 통제 해석.
- [`crates/volicord-store/src/workflow_records.rs`](../../../../crates/volicord-store/src/workflow_records.rs):
  정책 적용 중 정규화된 쓰기 권한 비교, Task 재평가 표시, 활성 쓰기 티켓 무효화.
- [`crates/volicord-store/src/schema/project.sql`](../../../../crates/volicord-store/src/schema/project.sql):
  영속 프로젝트와 Task 상태.

## 관련 테스트와 참조 담당 문서

- [`crates/volicord-core/src/methods/tests/`](../../../../crates/volicord-core/src/methods/tests/)의
  Core 메서드 테스트와
  [`tests/conformance/baseline.rs`](../../../../tests/conformance/baseline.rs)의
  메서드 사이 적합성 검증.
- [Core 모델](../../reference/core-model.md),
  [Intake](../../reference/api/method-intake.md),
  [API 상태 스키마](../../reference/api/schema-state.md),
  [API 값 집합](../../reference/api/schema-value-sets.md),
  [관리 CLI](../../reference/admin-cli.md),
  [저장소 기록](../../reference/storage-records.md).
- [상태 결속 쓰기 티켓 유효성](state-bound-write-ticket-validity.md)은 이 결정과 짝을
  이루는 티켓 유효성 결정을 기록합니다.
