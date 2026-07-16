# 확인된 효과와 휴리스틱 관찰은 권한 효력이 다르다

## 맥락

Detective 통합은 구조화된 호스트 이벤트, 직접 파일 도구의 대상, 셸 명령, 감시기
snapshot, 저장소 diff를 함께 받습니다. 이 출처들의 정밀도는 같지 않습니다. 명령
이름이나 불완전한 문자열 파싱을 읽기 전용 또는 쓰기 효과의 증명으로 취급하면 실제
쓰기를 놓치거나 정상 작업을 하드 차단할 수 있습니다.

Guard에 내장된 셸 파서는 파이프, 리다이렉션, 서브셸, 인용, 스크립트 내부 효과를
신뢰성 있게 예측할 수 없습니다. 행동 뒤 관찰은 사전에 알 수 없었던 사실을 확인할
수 있습니다.

## 결정

효과 분류와 함께 관찰 신뢰도와 출처 세부사항을 전달합니다. 행동 전 하드 차단은
쓰기 대상이 결정론적으로 식별되고 현재 적용 범위, 쓰기 티켓, 필요한 사용자 권한과
충돌할 때로 제한합니다. 불확실하거나 경로가 없는 가능한 효과는 경고하고 실행 뒤
다시 확인합니다.

행동 뒤 조정은 구조화된 변경 경로를 먼저 사용하고, 제한된 감시기 비교와 저장소
diff를 거친 뒤 휴리스틱 이벤트를 사용합니다. 의심되는 미기록 변경은 확인된 변경과
구분하므로, 보강 증거가 없는 불확실성 자체가 닫기 차단 사유가 되지 않습니다.

정확한 신뢰도와 효과 값, 경로 평가 형태, 이유 코드, 차단 규칙, 진단 출력은 공개
스키마, Guard, Core, 저장소, 보안 참조 담당 문서가 계속 정의합니다.

## 결과

- 구조화되어 있고 범위 밖인 직접 편집은 실행 전에 계속 차단할 수 있습니다.
- 안전하거나 쓰기라고 증명할 수 없는 셸 명령을 결정론적 사실로 잘못 표시하지
  않습니다.
- 행동 뒤 증거는 원래 출처 사실을 다시 쓰지 않고 의심 관찰을 확인 상태로 올리거나
  해소할 수 있습니다.
- 계측은 확인된 집행과 휴리스틱 경고의 품질을 구분할 수 있습니다.
- 명령 분류는 두 번째 셸 구현이 되지 않도록 의도적으로 좁게 유지합니다.

## 비목표

- 모든 파일시스템 효과나 외부 효과를 빠짐없이 관찰한다고 약속하지 않습니다.
- Detective를 OS 샌드박스로 만들거나 파일을 바꾼 행위자를 식별하지 않습니다.
- 모든 명령을 분류하거나 이 ADR에서 정확한 공개 값 집합을 정의하지 않습니다.
- 알 수 없는 효과를 경고하는 것은 그 효과를 승인하는 일이 아닙니다.

## 거부한 대안

- 넓은 실행 파일 이름을 읽기 전용으로 취급하는 방식은 그 프로그램들에 쓰기,
  파괴적 동작, 스크립트 실행 하위 명령이 많으므로 거부했습니다.
- 알 수 없는 모든 명령을 하드 차단하는 방식은 불확실성이 정책 위반의 증명이
  아니고 불필요한 오탐을 만들기 때문에 거부했습니다.
- Guard에서 전체 셸 문법을 파싱하는 방식은 그래도 스크립트 내부와 모든 런타임
  효과를 예측할 수 없으므로 거부했습니다.
- 행동 뒤 관찰을 모두 같은 권한 효력으로 취급하는 방식은 출처 정밀도가 닫기와
  진단에 계속 보여야 하므로 거부했습니다.

## 관련 구현

- [`crates/volicord-cli/src/`](../../../../crates/volicord-cli/src/) 아래 Guard 명령 평가:
  호스트 이벤트 디코딩, 명령 분류, 행동 전후 결정.
- [`crates/volicord-store/src/session_watch.rs`](../../../../crates/volicord-store/src/session_watch.rs):
  제한된 행동 전후 저장소 관찰.
- [`crates/volicord-store/src/guards.rs`](../../../../crates/volicord-store/src/guards.rs):
  영속 관찰과 미기록 변경 상태.
- [`crates/volicord-core/src/methods/reconcile_changes.rs`](../../../../crates/volicord-core/src/methods/reconcile_changes.rs):
  관찰된 변경의 Core 조정.

## 관련 테스트와 참조 담당 문서

- [`crates/volicord-cli/tests/guard_command.rs`](../../../../crates/volicord-cli/tests/guard_command.rs),
  Store 세션 감시 테스트, Core 변경 조정 테스트,
  [`tests/conformance/baseline.rs`](../../../../tests/conformance/baseline.rs).
- [Core 모델](../../reference/core-model.md),
  [관리 CLI](../../reference/admin-cli.md),
  [변경 조정](../../reference/api/method-reconcile-changes.md),
  [API 상태 스키마](../../reference/api/schema-state.md),
  [API 값 집합](../../reference/api/schema-value-sets.md),
  [저장소 기록](../../reference/storage-records.md),
  [보안](../../reference/security.md).
