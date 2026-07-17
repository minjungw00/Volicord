# 정규 Core UTC 시계

## 맥락

Volicord는 Core 계획, 원자적 Core 커밋, 저장소 소유 staging과 receipt writer, 로컬 User
Channel token, bootstrap, 호스트 관찰 전반에서 만료, 최신성, 캡처 시각, lifecycle을
판단합니다. 각 호출 위치가 프로세스 wall clock을 독립적으로 사용하면 프로젝트 시각이
뒤로 갈 수 있습니다. `state_version`을 시각으로 사용하면 권한 상태 순서와 UTC deadline이
뒤섞입니다.

구현은 동작이나 관찰이 일어난 의미 시각과 행이 커밋된 때를 기록하는 Store transaction
metadata도 구분해야 합니다.

## 결정

프로젝트 범위 정규 Core UTC 시계 하나를 사용합니다. Store는 감소하지 않는 영속 하한을
`project_state.updated_at`에 저장하며 새 스키마 필드나 시계 테이블을 추가하지 않습니다.
현재 샘플은 구성된 실시간 시각 후보, 영속 하한, Store handle이 해당 프로젝트에 대해
이미 받아들인 더 늦은 샘플에서 선택합니다. `SystemClock`은 SQLite 현재 UTC를 이 후보로
사용합니다.

Custom 또는 주입 Clock은 실시간 후보를 대신할 수 있지만 Core service 경계는 계속 영속
하한과 같은 handle 샘플을 포함한 최댓값으로 합성합니다. 저장 담당 timestamp를 현재
시각으로 바꾸지 않습니다. 미래 시각 행은 해당 담당자가 그 값을 invalid로 정의한
경우에만 닫힌 상태로 실패합니다. 모든 TTL 파생은 checked 덧셈을 사용하고 커밋 전에
표현 가능한 정규 RFC 3339 UTC 결과를 요구합니다.

메서드 계획까지 진행한 공개 동작은 공통 preflight 뒤 `operation_now`를 정확히 한 번
샘플링합니다. 계획 코드는 모든 현재 시각 확인과 담당 문서가 정의한 의미 있는 동작
timestamp에 이 값을 다시 사용합니다. 커밋 후보 분기는 구성된 Clock을 따릅니다.
Production `SystemClock`에서는 `operation_now`, transaction 안에서 샘플링한 SQLite 현재
UTC, 영속 하한, 같은 handle이 받아들인 더 늦은 샘플의 최댓값을 transaction
`committed_at`으로 선택합니다. 주입 또는 custom Clock에서는 그 Clock의 주입 실시간
후보가 transaction의 SQLite 후보를 대신하며, 최댓값에는 계속 `operation_now`, 영속
하한, 같은 handle 샘플이 포함됩니다. Custom 분기는 SQLite 현재 UTC를 별도 실시간
후보로 추가하지 않습니다.

일반 Core 커밋은 프로젝트 하한, 묶음의 모든 event, 선택적 replay 행, 적용 가능한
`created_at`, `updated_at`, `retired_at`, `promoted_at` 같은 Store 생성 transaction
metadata에 정확히 같은 `committed_at`을 사용합니다. 의미 있는 `requested_at`,
`resolved_at`, `closed_at`, `recorded_at`, `consumed_at`이나 검증된 관찰 사실인
`occurred_at`, `observed_at`, `started_at`은 이 값으로 바꾸지 않습니다.

저장소 소유 artifact staging, evidence-capture receipt 이행, 로컬 User Channel token
발급은 Core 권한 커밋이 되지 않으면서 자신의 생성 시각 이상으로 하한을 원자적으로
갱신합니다. 정확한 replay, 거부, dry run, 읽기 전용 관찰은 더 늦은 하한을 영속화하지
않습니다.

`state_version`은 계속 권한 상태 및 충돌 시계입니다. 정규 UTC 하한은 시간 권한의 하한입니다.
두 값은 서로를 대신하지 않으며 UTC 하한은 모든 상태 전이마다 엄격히 증가할 필요 없이
감소하지 않으면 됩니다.

Bootstrap은 새 프로젝트의 하한을 초기화합니다. 재등록은 기존 하한을 검증하고 보존하며
담당 상태의 형식이 잘못되면 닫힌 상태로 실패합니다. 올바른 미래 시각을 초기화하지
않습니다.

## 결과

- 별도 계획 확인이 서로 다른 시각을 샘플링해 한 동작이 만료 경계를 넘는 일이 없습니다.
- Core 커밋은 감사 가능한 transaction timestamp 하나를 가지면서 동작, 관찰, 커밋 시각의
  의미 차이를 보존합니다.
- 저장소 전용 시간 writer는 권한 상태 전이와 구분되고 event나 state version을 만들지
  않습니다.
- 호스트 시계 오차와 지연 관찰은 현재 Core 권한 경계를 되감거나 전진시키지 못합니다.
- 손상된 영속 하한은 추측해 복구할 값이 아니라 손상된 담당 상태입니다.
- 테스트 시계는 저장된 프로젝트 시각을 우회하는 권한을 얻지 않으면서 사용할 수 있고,
  TTL overflow는 제어된 효과 없음 거부입니다.

## 거부한 대안

- 각 호출 위치에서 프로세스 wall clock을 읽으면 시각이 뒤로 가고 한 동작 안에서 판단이
  달라질 수 있습니다.
- `state_version`은 UTC deadline이나 관찰 시각을 나타낼 수 없고 저장소 전용 시간 효과를
  숨깁니다.
- 새 시계 테이블이나 스키마 버전 필드는 기존 프로젝트 header를 중복합니다.
- timestamp 형태 필드를 모두 커밋 시각으로 다시 쓰면 의미 있는 원천·동작 사실을 잃습니다.
- replay, 거부, dry run, 읽기에서 영속 하한을 전진시키면 효과 없음 경로가 숨은 쓰기가
  됩니다.

## 관련 구현

- [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs):
  준비된 동작 시각 샘플과 커밋 하한 전달.
- [`crates/volicord-store/src/core_pipeline.rs`](../../../../crates/volicord-store/src/core_pipeline.rs)와
  [`core_pipeline/commit.rs`](../../../../crates/volicord-store/src/core_pipeline/commit.rs): 프로젝트 시각
  샘플과 정규 transaction 시각 선택.
- [`crates/volicord-store/src/core_pipeline/mutation_apply.rs`](../../../../crates/volicord-store/src/core_pipeline/mutation_apply.rs):
  transaction metadata 적용.
- [`crates/volicord-store/src/artifacts.rs`](../../../../crates/volicord-store/src/artifacts.rs),
  [`evidence_capture.rs`](../../../../crates/volicord-store/src/evidence_capture.rs): Store 소유
  하한 writer.
- [`crates/volicord-store/src/bootstrap.rs`](../../../../crates/volicord-store/src/bootstrap.rs):
  하한 초기화, 보존, 검증.

## 참조 담당 문서

정확한 계약은 [저장소 버전 관리](../../reference/storage-versioning.md),
[저장소 기록](../../reference/storage-records.md), [저장 효과](../../reference/storage-effects.md),
[Core 모델](../../reference/core-model.md), 적용되는 공개 메서드와 스키마 담당 문서에
남습니다. [원자적 변이 커밋 전 계획](plan-and-atomic-commit.md)과
[통합 사용자 행동 요청과 해결](unified-user-action-request-resolution.md)도 함께 봅니다.
