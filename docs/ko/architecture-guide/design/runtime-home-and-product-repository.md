# Runtime Home과 Product Repository 설계

## 목적

이 설계는 Volicord runtime state, 사용자 product file, 설치된 executable과 host
configuration, 저장소 유지보수 tooling, disposable test state를 구현에서 분리하는
방식을 설명합니다.

## 설계

`volicord-store`는 Registry data, project database, artifact, operational session,
diagnostics를 위한 정규 `Volicord Runtime Home` 하나를 해석하고 검증합니다. Product
Repository registration은 정규 product path와 Git layout identity를 저장하지만 그
repository를 runtime-data directory로 만들지 않습니다.

CLI setup은 mutation admission과 setup transaction state를 유지하면서 owner-defined
integration file을 Product Repository 또는 user configuration에 관리할 수 있습니다.
Public Core 메서드 실행은 owner-defined path와 observation을 기록하지만 product file을
쓰지 않습니다. `xtask`는 Runtime Home state를 사용하지 않고 repository source,
documentation, metadata, Cargo configuration을 읽습니다.

## 불변 조건

- Runtime Home, Product Repository, source checkout, installation location은 서로 다른
  역할입니다.
- Mutation admission 뒤 정규 Runtime Home identity는 고정됩니다.
- Runtime database, log, diagnostic, generated record는 유지 문서에 두지 않습니다.
- Product-file write는 public Core method 경로 밖에 남습니다.
- Test는 disposable Runtime Home과 Product Repository 위치를 사용합니다.
- Repository tooling은 product runtime 밖에 있고 현재 source route와 workspace
  metadata를 직접 읽습니다.

## 책임 경계

Store는 Runtime Home resolution, bootstrap, schema validation, project registration,
database access, artifact path를 담당합니다. Platform filesystem 코드는 안전한 path와
publication primitive를 담당합니다. CLI는 setup 조율과 managed integration file을
담당합니다. Core policy는 owner-defined product path 해석을 담당합니다. `xtask`는
source-repository maintenance check를 담당합니다.

## 실행 흐름

1. Caller가 Runtime Home을 해석하고 canonicalize합니다.
2. Mutation 가능한 setup 또는 일반 writer가 적용되는 filesystem permit을 얻습니다.
3. Store가 Runtime Home을 inspect하거나 publish하고 현재 manifest와 physical schema를
   검증합니다.
4. CLI가 Product Repository와 명시적인 Connection membership을 등록합니다.
5. Core와 adapter가 repository를 runtime storage로 사용하지 않으면서 owner-defined
   작업에 등록된 product 및 Git coordinate를 사용합니다.

## 실패 동작

Path alias mismatch, Runtime Home/Product Repository overlap, corrupt schema, stale
registration, publication ownership loss, setup contention은 dependent mutation 전에 typed
failure로 남습니다. Setup rollback은 publication guard가 계속 소유권을 증명하는
state만 제거하며 관련 없는 file을 재귀적으로 추측하지 않습니다.

## 범위 제외

이 설계는 path normalization contract, security isolation, storage layout, managed-file
content, installation root, artifact lifecycle을 정의하지 않습니다. 위치는 authority나
actor identity를 증명하지 않습니다.

## 구현 경로

- [`crates/volicord-store/src/runtime_home.rs`](../../../../crates/volicord-store/src/runtime_home.rs)와
  [`bootstrap.rs`](../../../../crates/volicord-store/src/bootstrap.rs):
  Runtime Home과 project registration.
- [`crates/volicord-platform-fs/src/lib.rs`](../../../../crates/volicord-platform-fs/src/lib.rs)와
  [`mutation_lease.rs`](../../../../crates/volicord-platform-fs/src/mutation_lease.rs):
  정규 path, publication, mutation admission.
- [`crates/volicord-cli/src/setup_command/`](../../../../crates/volicord-cli/src/setup_command/)와
  [`connection_command/setup_transaction.rs`](../../../../crates/volicord-cli/src/connection_command/setup_transaction.rs):
  setup 및 managed-file transaction.
- [`crates/volicord-core/src/policy/path.rs`](../../../../crates/volicord-core/src/policy/path.rs):
  Core policy path helper.
- [`xtask/src/repository.rs`](../../../../xtask/src/repository.rs):
  source-repository root 및 path 처리.

## 참조 담당 문서

정확한 동작은 [런타임 경계](../../reference/runtime-boundaries.md),
[저장소](../../reference/storage.md),
[아티팩트 저장소](../../reference/storage-artifacts.md),
[관리 CLI](../../reference/admin-cli.md),
[Agent Connection](../../reference/agent-connection.md),
[보안](../../reference/security.md)에 남습니다.
