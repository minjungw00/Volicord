# Core와 어댑터 경계 설계

## 목적

이 설계는 관리 command model, CLI와 MCP 어댑터, Core 정책과 typed 결과 구성,
Store, 공유 진단, 저장소 도구 사이의 현재 의존 및 실행 경계를 설명합니다.

## 설계

`volicord-command-model`은 명령 실행 없이 완전한 Clap 선언과 introspection model을
담습니다. `volicord-cli`와 `volicord-mcp`는 각자의 process, setup, transport,
routing, rendering 관심사를 담당하며 typed interface로 `volicord-core`를 호출합니다.

Core는 공통 preflight, 검증된 invocation context, 메서드 planning, `policy/` 아래의
집중 모듈, replay 처리, branch 선택, 최종 공개 결과 구성을 담당합니다.
`volicord-types::methods`의 공개 결과 선언은 한 선언에서 fields-only planning type과
완전한 result type을 만들므로 planner가 불완전한 공통 envelope를 구성하지 않습니다.

Store는 엄격한 저장 record decoding, aggregate read, grouped mutation 적용,
transaction mechanism을 담당합니다. 공유 diagnostic identity와 report shape는
`volicord-types`에 있고 domain 변환, 지속 저장, rendering은 각 domain 담당자에게
남습니다. `xtask`는 저장소 검사를 위해 command model과 MCP protocol registry를
사용하지만 runtime 밖에 있습니다.

## 불변 조건

- Core 쪽 crate는 CLI나 MCP adapter crate에 의존하지 않습니다.
- Command-model crate는 Clap에만 의존하며 실행을 담당하지 않습니다.
- 메서드 planner는 typed field와 계획된 effect를 반환합니다. 공유 pipeline은 branch
  fact가 정해진 뒤에만 공통 fact를 추가합니다.
- Store는 adapter 입력에서 메서드 정책을 파생하지 않습니다.
- Adapter projection은 Core 결과를 넓히거나 typed diagnostic identity를 rendered
  prose로 대신하지 못합니다.
- 저장소 검증은 runtime dependency나 compatibility 경로가 되지 않습니다.

## 책임 경계

Adapter는 신뢰할 수 있는 local context를 파생하고 transport data를 변환합니다.
Core는 권한을 고려한 planning과 policy 평가를 담당합니다. Store는 persistence와
atomicity를 담당합니다. `volicord-types`는 dependency-safe shared shape를 담당합니다.
`volicord-host-contract`와 `volicord-mcp-protocol`은 중립적인 external-wire profile을
담당합니다. `xtask`는 현재 저장소 검증과 생성 작업 흐름을 담당합니다.

## 실행 흐름

1. Command 또는 transport 문법을 담당 경계에서 parsing합니다.
2. Adapter가 local Runtime Home, Connection, project, actor, operation context를
   해석합니다.
3. Core가 공통 preflight와 집중 policy 담당자를 사용하는 메서드별 planning을
   수행합니다.
4. 선택한 branch는 read-only, no-effect, dry-run, staging, committed mutation 중
   하나의 typed 상태를 유지합니다.
5. Store가 계획된 effect를 읽거나 적용합니다.
6. Core가 완전한 typed 결과를 구성하고 adapter가 권한을 더하지 않은 채 CLI 또는
   MCP 형태로 투영합니다.

## 실패 동작

문법 실패는 command 또는 protocol 경계에 남습니다. 공개 거부는 구조화된 Core
response로 남습니다. 지속되는 담당 데이터 실패와 예상하지 못한 구현 실패는 typed
Store/Core/adapter 경로를 유지합니다. 저장소 검사 실패는 `xtask`가 보고하며 runtime
fallback 동작을 시작하지 않습니다.

## 범위 제외

이 설계는 API 동작, CLI 명령 의미, 스키마 의미, 저장 효과, diagnostic code 의미를
정의하지 않습니다. Adapter 내부를 Core를 통해 노출하거나 version에 따라 다른 모듈
경로를 사용하지 않습니다.

## 구현 경로

- [`crates/volicord-command-model/src/lib.rs`](../../../../crates/volicord-command-model/src/lib.rs):
  명령 문법, 가시성, traversal, synopsis, 정규 invocation.
- [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs),
  [`methods/`](../../../../crates/volicord-core/src/methods/),
  [`policy/`](../../../../crates/volicord-core/src/policy/): typed Core 조율,
  메서드 planning, 집중 policy.
- [`crates/volicord-types/src/methods.rs`](../../../../crates/volicord-types/src/methods.rs):
  fields-only 및 완전한 공개 result 선언.
- [`crates/volicord-store/src/core_pipeline/`](../../../../crates/volicord-store/src/core_pipeline/):
  project Store facade, aggregate 소유권, commit 조율.
- [`crates/volicord-mcp/src/`](../../../../crates/volicord-mcp/src/),
  [`crates/volicord-cli/src/`](../../../../crates/volicord-cli/src/),
  [`xtask/src/`](../../../../xtask/src/): adapter 및 저장소 도구 책임.

## 참조 담당 문서

정확한 동작은 [API 메서드](../../reference/api/methods.md),
[Core 모델](../../reference/core-model.md),
[MCP 전송](../../reference/mcp-transport.md),
[관리 CLI](../../reference/admin-cli.md),
[저장소](../../reference/storage.md),
[실패 모델](../../reference/failure-model.md)에 남습니다.
