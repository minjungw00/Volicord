# 원자적 변이 커밋 전 계획

## 맥락

공개 메서드에는 메서드별 검증과 계획이 필요하지만, 정상 커밋 효과는 상태
버전 변경, 이벤트, 재실행 기록, 응답 JSON 저장과 함께 원자적으로
적용되어야 합니다. Store가 메서드 정책을 소유하면 안 되고, 메서드 계획
코드가 SQL 트랜잭션 메커니즘을 소유해도 안 됩니다.

## 결정

메서드 모듈은 효과 실행 전에 계획합니다. 계획 함수는
`OwnerPipelineBranch`를 선택하고, 커밋 분기는 결과 필드, 이벤트 페이로드,
`CoreStorageMutation` 값을 제공합니다. 공유 Core 파이프라인은
`CommitMutationInput`을 만들고, `CoreProjectStore::commit_mutation`은 정상
커밋 변이를 하나의 즉시 Store 트랜잭션 안에서 적용합니다.

준비된 동작은 `operation_now` 시계 하한 하나를 제공합니다. Store는 이 하한보다 이르지
않은 정규 transaction `committed_at` 하나를 선택하고 영속 프로젝트 시각 하한, event와
선택적 replay 생성, Store 생성 transaction metadata에 일관되게 사용합니다. 의미 있는
동작·관찰 시각은 계획 코드 또는 담당자가 제공한 사실로 남습니다. 근거는
[정규 Core UTC 시계](canonical-core-utc-clock.md)에 기록합니다.

일시적 아티팩트 스테이징은 별도의 Store 소유 효과 경로이며 정상 Core
변이 커밋을 사용하지 않습니다.

## 결과

- `dry-run`, 읽기 전용, 효과 없음, 일시적 스테이징, 커밋 변이 경로가 코드와
  테스트에서 구분됩니다.
- Store는 재실행, 오래된 상태, 이벤트 추가, 응답 저장, 롤백 동작을 하나의
  커밋 경계에서 집행할 수 있습니다.
- Store는 재실행, 최신성, 영속 프로젝트 시각 하한을 점검하는 직렬화 경계 안에서
  커밋 timestamp를 선택할 수 있습니다.
- 메서드 코드는 원시 저장소 메커니즘을 품지 않고 의도 효과를 표현할 수
  있습니다.
- 책임별 aggregate enum이 mutation 입력, 저장 검증, SQL 적용, 필요한 typed
  사실을 자신의 record와 집중 테스트 곁에 둡니다. 얇은 정적 dispatcher가 계획
  순서를 보존합니다.
- 커밋된 메서드 효과 변경은 보통 메서드 계획 코드, Store aggregate mutation 담당,
  집중
  테스트, 적용되는 참조 담당 문서를 함께 건드립니다.

## 비목표

- 이 결정은 어떤 공개 메서드의 정확한 저장 효과도 정의하지 않습니다.
- DDL, 저장소 기록, 스키마 필드 의미를 다시 쓰지 않습니다.
- `dry-run` 또는 효과 없음 분기를 제품 수락으로 만들지 않습니다.
- 아티팩트 스테이징이 정상 Core 변이 커밋이 되어야 한다고 요구하지
  않습니다.

## 관련 구현

- [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs):
  `OwnerPipelineBranch`, `CoreService::execute_prepared_request`, Core 커밋 조율.
- [`crates/volicord-core/src/methods/`](../../../../crates/volicord-core/src/methods/):
  `plan_intake`, `plan_prepare_write` 같은 메서드별 계획 함수.
- [`crates/volicord-store/src/core_pipeline/`](../../../../crates/volicord-store/src/core_pipeline/):
  `CoreStorageMutation`, `CommitMutationInput`,
  `CoreProjectStore::commit_mutation`, `MutationCommitOutcome`,
  `mutations.rs`의 정적 grouped dispatcher, aggregate mutation 담당 모듈,
  `commit.rs`의 aggregate 간 coordinator.
- [`crates/volicord-store/src/artifacts.rs`](../../../../crates/volicord-store/src/artifacts.rs):
  `CoreProjectStore::create_artifact_staging`.

## 관련 테스트와 참조 담당 문서

- [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs)의
  `committed_mutation_increments_state_version_once`,
  `idempotency_replay_returns_stored_response`,
  `stale_expected_state_version_is_rejected_without_effect`.
- [`crates/volicord-store/src/core_pipeline/`](../../../../crates/volicord-store/src/core_pipeline/)의
  `transaction_replay_returns_stored_response_before_stale_expected_state`,
  `transaction_replay_hash_conflict_rejects_without_effect`,
  `ordered_multi_aggregate_commit_is_versioned_replayable_and_durable`,
  `intermediate_aggregate_failure_rolls_back_every_commit_effect`.
- [`crates/volicord-core/src/methods/tests/stage_artifact.rs`](../../../../crates/volicord-core/src/methods/tests/stage_artifact.rs)의
  `stage_artifact_creates_transient_handle_without_core_commit`.
- [저장 효과](../../reference/storage-effects.md),
  [저장소 버전 관리](../../reference/storage-versioning.md),
  [API 메서드](../../reference/api/methods.md)에서 연결된 공개 메서드 담당 문서.
- [정규 Core UTC 시계](canonical-core-utc-clock.md).
