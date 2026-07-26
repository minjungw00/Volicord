# 정규 Core UTC 시계 설계

## 목적

이 설계는 Core와 Store가 준비된 동작과 지속되는 시간 조율에 프로젝트 범위의 비감소
UTC 시간 경계 하나를 사용하면서 권한 상태 순서를 별도로 유지하는 방식을 설명합니다.

## 설계

`CoreService`는 공통 preflight 뒤 `operation_now`를 한 번 샘플링합니다. Store handle은
구성된 실시간 시각 후보, 지속되는 프로젝트 하한, 같은 handle이 받아들인 더 늦은
샘플을 합성합니다. 일반 Core commit은 직렬화된 transaction 안에서 `committed_at`
하나를 선택하여 프로젝트 하한, 권한 event, replay metadata, Store가 만든 mutation
metadata에 사용합니다.

Production `SystemClock`은 SQLite UTC를 실시간 후보로 사용합니다. 주입한 시계는
결정적인 테스트를 위해 이 후보를 대신하지만 지속되는 하한을 우회하지 않습니다.
Store가 담당하는 artifact, receipt, User Channel writer는 Core 권한 commit이 되지
않으면서 각자의 원자적 경로로 하한을 전진시킵니다.

## 불변 조건

- 준비된 동작 하나는 `operation_now` 값 하나를 다시 사용합니다.
- 프로젝트 시간 하한은 뒤로 이동하지 않습니다.
- `state_version`은 권한 상태 전이를 정렬하고 UTC 하한은 시간 권한을 정렬합니다.
  어느 쪽도 다른 쪽을 대신하지 않습니다.
- 의미 있는 source 및 observation timestamp는 transaction metadata와 구분됩니다.
- Deadline 파생은 checked 연산과 표현 가능한 정규 UTC timestamp를 사용합니다.
- 읽기 전용, 거부, dry-run, 정확한 replay 경로는 숨은 시간 하한 쓰기를 만들지
  않습니다.

## 책임 경계

Core는 준비된 동작 시각을 샘플링하고 planning 및 commit 조율로 시간 하한을
전달합니다. Store는 지속되는 하한 검증, transaction 시각 선택, 원자적 하한 갱신을
담당합니다. 메서드와 정책 모듈은 준비된 시각을 사용하며 어댑터와 관찰된 host
timestamp는 이를 대신하지 않습니다.

## 실행 흐름

1. 공통 preflight가 현재 프로젝트 Store를 열고 검증합니다.
2. `CoreService`가 정규 동작 시각을 한 번 샘플링합니다.
3. 메서드 planning이 현재 시각 검사와 담당 의미의 동작 timestamp에 그 값을 다시
   사용합니다.
4. Commit coordinator가 immediate transaction 안에서 `committed_at`을 선택합니다.
5. 담당 문서가 요구하는 곳에서 grouped Store mutation, event 및 replay row,
   지속되는 하한이 조율된 transaction 시각을 사용합니다.

## 실패 동작

지속되는 하한의 형식이 잘못되었거나 timestamp를 표현할 수 없거나 deadline overflow가
발생하면 일부 mutation 없이 실패합니다. 미래 값을 가진 담당 데이터는 적용되는 집중
담당 규칙으로만 실패합니다. Store는 이를 추측으로 복구하거나 project 등록 중 올바른
하한을 초기화하지 않습니다.

## 범위 제외

이 설계는 공개 timestamp 필드, 만료 규칙, 저장 효과, 스키마 의미를 다시 정의하지
않습니다. Host time이나 `state_version`을 UTC 시계로 만들지 않으며 읽기나 상태 변경
때마다 하한이 전진해야 한다고 요구하지 않습니다.

## 구현 경로

- [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs):
  `Clock`, `SystemClock`, 준비된 동작 시각 샘플, commit 하한 전달.
- [`crates/volicord-store/src/core_pipeline/clock.rs`](../../../../crates/volicord-store/src/core_pipeline/clock.rs)와
  [`commit.rs`](../../../../crates/volicord-store/src/core_pipeline/commit.rs):
  프로젝트 시각 샘플과 transaction 시각 선택.
- [`crates/volicord-store/src/artifacts.rs`](../../../../crates/volicord-store/src/artifacts.rs),
  [`evidence_capture.rs`](../../../../crates/volicord-store/src/evidence_capture.rs),
  [`bootstrap.rs`](../../../../crates/volicord-store/src/bootstrap.rs):
  Store 소유 시간 writer와 하한 초기화.

## 참조 담당 문서

정확한 시간 계약은 [저장소 버전 관리](../../reference/storage-versioning.md),
[저장소 기록](../../reference/storage-records.md),
[저장 효과](../../reference/storage-effects.md),
[Core 모델](../../reference/core-model.md), 적용되는 공개 메서드 및 스키마 담당
문서에 남습니다.
