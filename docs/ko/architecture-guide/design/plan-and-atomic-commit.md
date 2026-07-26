# 계획과 원자적 커밋 설계

## 목적

이 설계는 메서드 모듈이 Store mutation 전에 typed result field와 effect를 planning하고
공유 Core pipeline 및 aggregate 담당 Store 모듈이 하나의 atomic commit을 완성하는
방식을 설명합니다.

## 설계

각 Core 메서드 planner는 최종 공통 response envelope를 구성하지 않고 메서드 담당
field와 계획된 effect를 반환합니다. `volicord-types::methods`의 result 선언은
fields-only type과 완전한 public result type을 함께 제공합니다.
`OwnerPipelineBranch<F>`는 read-only, no-effect, dry-run, staging, rejection, committed
경로에서 그 type을 유지합니다.

Committed branch에서 Core는 grouped `CoreStorageMutation` 값, 계획된 event, replay
coordinate, typed response builder로 `CommitMutationInput`을 만듭니다.
`CoreProjectStore::commit_mutation`은 immediate transaction 하나를 엽니다. 얇은
dispatcher가 planner 순서를 보존하고 각 mutation을 validation, SQL, typed application
fact 담당 aggregate 모듈에 위임합니다. Coordinator는 state를 한 번 전진시키고 event를
추가하며 replay output을 저장한 뒤 commit하거나 rollback합니다.

## 불변 조건

- 메서드 정책과 planning은 Core에 있고 SQL mechanism은 Store에 있습니다.
- Result 선언 하나가 fields-only planning과 complete result composition을 함께
  담당합니다.
- 공통 branch fact는 execution branch가 정해진 뒤에만 추가합니다.
- Grouped mutation은 planner 순서와 aggregate 소유권을 보존합니다.
- 일반 committed operation은 project state를 최대 한 번 전진시키며 event, replay
  response, aggregate effect를 함께 commit합니다.
- Transient artifact staging은 일반 Core mutation commit 밖에 남습니다.

## 책임 경계

메서드 모듈은 request-specific validation과 계획된 result field를 담당합니다. 집중
Core policy 모듈은 재사용 가능한 authority evaluation을 담당합니다. Pipeline은 branch
조율과 final response composition을 담당합니다. Store aggregate 모듈은 자신의 record에
대한 strict read/write logic을 담당하며 commit coordinator는 aggregate 사이의
transaction 조율만 담당합니다.

## 실행 흐름

1. 공통 preflight가 verified invocation과 method policy를 파생합니다.
2. Method planner가 typed fact를 읽고 집중 policy를 평가하여 method result field와
   계획된 effect를 만듭니다.
3. Pipeline이 typed branch를 선택합니다.
4. Committed branch가 Store commit input과 response builder를 구성합니다.
5. Store가 immediate transaction 안에서 replay와 freshness를 확인합니다.
6. Aggregate 모듈이 자신의 grouped mutation을 검증하고 적용합니다.
7. Store가 state를 전진시키고 event를 추가하며 complete typed result를 직렬화해
   replay를 저장하고 commit합니다.

## 실패 동작

Planning failure는 Store mutation을 만들지 않습니다. Aggregate validation 또는 SQL
failure는 transaction의 모든 mutation, event, state update, replay row를 rollback합니다.
Replay와 stale-state check는 새 effect보다 먼저 실행됩니다. Result composition
failure는 stored response 없는 effect를 남기지 못합니다.

## 범위 제외

이 설계는 공개 메서드 result, storage effect, DDL shape, event meaning, state-version
contract를 정의하지 않습니다. Dry-run, no-effect, staging branch를 committed Core
mutation과 같게 만들지 않습니다.

## 구현 경로

- [`crates/volicord-types/src/methods.rs`](../../../../crates/volicord-types/src/methods.rs):
  result 선언과 fields-only composition type.
- [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs):
  `OwnerPipelineBranch`, preflight, branch execution, final composition.
- [`crates/volicord-core/src/methods/`](../../../../crates/volicord-core/src/methods/):
  method-specific planner.
- [`crates/volicord-store/src/core_pipeline/commit.rs`](../../../../crates/volicord-store/src/core_pipeline/commit.rs),
  [`mutations.rs`](../../../../crates/volicord-store/src/core_pipeline/mutations.rs), 인접한
  aggregate 모듈: transaction 조율과 담당 mutation 적용.
- [`crates/volicord-store/src/artifacts.rs`](../../../../crates/volicord-store/src/artifacts.rs):
  분리된 staging 경로.

## 참조 담당 문서

정확한 동작은 [API 메서드](../../reference/api/methods.md),
[Core 모델](../../reference/core-model.md),
[저장소](../../reference/storage.md),
[저장 효과](../../reference/storage-effects.md),
[저장소 버전 관리](../../reference/storage-versioning.md)에 남습니다.
