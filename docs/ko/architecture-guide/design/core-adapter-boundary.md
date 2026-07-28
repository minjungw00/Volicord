# Core와 어댑터 경계 설계

## 목적

이 설계는 관리 command model, CLI와 MCP 어댑터, Core 정책과 typed 결과 구성,
Store, 공유 진단, 저장소 도구 사이의 현재 의존 및 실행 경계를 설명합니다.

## 설계

`volicord-command-model`은 명령 실행 없이 완전한 Clap 선언과 introspection model을
담습니다. Typed invocation builder는 같은 선언에서 명령 경로와 option spelling을
도출합니다. `volicord-user-action-presentation`은 adapter-neutral UserAction fact를
현재 공유 CLI inbox item과 CLI recovery instruction으로 바꿉니다.
`volicord-cli`와 `volicord-mcp`는 각자의 process, setup, transport, routing,
최종 rendering 관심사를 담당하며 typed interface로 `volicord-core`를 호출합니다.

Core는 공통 preflight, 검증된 invocation context, 메서드 planning, `policy/` 아래의
집중 모듈, replay 처리, branch 선택, 최종 공개 결과 구성을 담당합니다.
`InvocationContext`는 typed local `UserActionChannelKind` 또는 불투명한
`ValidatedAgentSession` 중 하나로 구성된 `InvocationAuthority`를 전달하며, actor
identity와 verification basis는 이 authority에서 파생됩니다.
`volicord-types::methods`의 공개 결과 선언은 한 선언에서 fields-only planning type과
완전한 result type을 만들므로 planner가 불완전한 공통 envelope를 구성하지 않습니다.

Store는 엄격한 저장 record decoding, aggregate read, grouped mutation 적용,
transaction mechanism을 담당합니다. 공유 diagnostic identity와 report shape는
`volicord-types`에 있고 domain 변환, 지속 저장, rendering은 각 domain 담당자에게
남습니다. `xtask`는 저장소 검사를 위해 command model과 MCP protocol registry를
비롯해 wire contract descriptor를 사용하지만 runtime 밖에 있습니다.

## 불변 조건

- Core의 일반 및 빌드 의존성은 Core 런타임 의존성으로 분류된 그룹만 대상으로
  합니다. Core 개발 의존성은 Core 개발 의존성으로 분류된 그룹도 대상으로 할 수
  있습니다.
- Core는 관리 명령 문법, 호스트 시작 인자, 호스트별 구성 경로, presentation
  label, rendered recovery instruction, adapter protocol envelope를 담당하지
  않습니다.
- Core 진입점은 자유 형식 actor 또는 host label이 아니라 typed semantic authority를
  받습니다.
- Command-model crate는 Clap에만 의존하며 실행을 담당하지 않습니다.
- 정규 CLI invocation builder는 바이너리가 사용하는 같은 Clap 선언을 inspect하고
  그 선언으로 결과를 검증합니다. Adapter는 명령 grammar를 다시 만들지 않습니다.
- 공유 UserAction presentation은 command model과 shared type에만 의존하며 Core,
  Store, CLI, MCP에는 의존하지 않습니다.
- 메서드 planner는 typed field와 계획된 effect를 반환합니다. 공유 pipeline은 branch
  fact가 정해진 뒤에만 공통 fact를 추가합니다.
- 결과, 거절, 미리보기 메타데이터는 서로 다른 닫힌 base 타입을 사용합니다. 고정된
  discriminant와 effect fact는 공유 타입이 강제하며, 모든 공개 분기와 base는 알 수
  없는 필드를 거절합니다.
- 메서드 선언 하나가 요청 타입, 결과 타입, 정확한 공개 응답 계열, 의미 기반 계약 ID,
  생성 스키마, replay 적격 여부를 담당합니다. Core 분기 선택과 저장 결과 검증은
  병렬 메서드 목록 대신 이 선언을 사용합니다.
- 필수 인프라 의존성이 메서드 결과를 만들 수 없으면 모든 공개 응답 분기 밖의 typed
  neutral Core 운영 오류를 반환합니다.
- `volicord-types`에는 MCP 요청, 결과, 오류, structured content, 도구 envelope,
  JSON-RPC wire type이 없습니다.
- `volicord-mcp-wire`는 일치하는 MCP adapter와 검증 도구 또는 테스트에서만 접근할 수
  있습니다. Core, shared type, Store, UserAction service, CLI, presentation package는
  이 crate에 의존할 수 없습니다.
- Store는 adapter 입력에서 메서드 정책을 파생하지 않습니다.
- Adapter는 projection 전에 선택한 메서드의 정확한 응답 계열로 Core 출력을
  decode합니다. Adapter projection은 이 계열이나 Core 결과를 넓힐 수 없고 typed
  diagnostic identity를 rendered prose로 대신하지 못합니다.
- 공개 오류는 불변 조건을 보존하는 공유 타입으로 구성합니다. Core와 adapter는
  코드에서 도출되는 범주를 사용하며 adapter 로컬 공개 코드/범주 매핑을 두지
  않습니다.
- 저장소 검증은 runtime dependency나 compatibility 경로가 되지 않습니다.

## 책임 경계

Adapter는 host integration을 선택하고 host별 값을 검증하며 신뢰할 수 있는 local
semantic context를 파생하고 transport data를 변환합니다. Core는 권한을 고려한
planning과 policy 평가를 담당합니다. Store는 persistence와 atomicity를 담당합니다.
`volicord-types`는 dependency-safe shared shape를 담당합니다.
`volicord-host-contract`는 semantic host contract를 담당합니다.
`volicord-mcp-protocol`은 protocol profile과 semantic capability를 담당합니다.
`volicord-mcp-wire`는 정확한 MCP field, error identity, structured content,
JSON-RPC 및 tool envelope, 생성 MCP schema를 담당합니다.
`volicord-user-action-presentation`은 공유 CLI 지향 UserAction
presentation을 담당합니다. CLI는 terminal rendering을 담당하고 MCP는 자체
protocol result projection과 neutral Core availability failure의 경계 변환을
담당합니다. `xtask`는 현재 저장소 검증과 생성 작업 흐름을 담당합니다.

## 실행 흐름

1. Command 또는 transport 문법을 담당 경계에서 parsing합니다.
2. Adapter가 local Runtime Home, Connection, project, operation context를 해석하고
   typed local-user 또는 Agent Connection authority를 구성합니다.
3. Core가 공통 preflight와 집중 policy 담당자를 사용하는 메서드별 planning을
   수행합니다.
4. 선택한 branch는 read-only, no-effect, dry-run, staging, committed mutation 중
   하나의 typed 상태를 유지합니다.
5. Store가 계획된 effect를 읽거나 적용합니다. 운영 불가는 typed operation,
   resource, retryability fact와 함께 neutral Core 오류 경로로 반환됩니다.
6. Core가 공개 메서드 결과를 구성하거나 내부 adapter read에 adapter-neutral
   semantic fact를 반환합니다.
7. 공유 presentation이 typed command-model invocation을 통해 이 fact에서 CLI inbox
   item 또는 recovery instruction을 도출합니다.
8. CLI가 local output을 rendering하고 MCP가 권한을 더하지 않은 채 자체 protocol
   projection을 구성합니다.

## 실패 동작

문법 실패는 command 또는 protocol 경계에 남습니다. 공개 거부는 구조화된 Core
response로 남습니다. 필수 Store 또는 인프라 실패 때문에 어떤 메서드 결과도 만들 수
없으면 typed Core 운영 오류를 반환하며 공개 거부나 성공 응답을 만들지 않습니다.
CLI는 이 neutral 실패를 runtime 진단 계약으로 변환합니다. MCP는 capability가 선택한
protocol carrier와 MCP 소유 wire identity로 변환합니다. 영속 담당 데이터 손상과
예상하지 못한 구현 실패는 typed Store/Core/adapter 경로를 유지합니다. 저장소 검사
실패는 `xtask`가 보고하며 runtime fallback 동작을 시작하지 않습니다.

## 범위 제외

이 설계는 API 동작, CLI 명령 의미, 스키마 의미, 저장 효과, diagnostic code 의미를
정의하지 않습니다. Adapter 내부를 Core를 통해 노출하거나 version에 따라 다른 모듈
경로를 사용하지 않습니다.

## 구현 경로

- [`crates/volicord-command-model/src/lib.rs`](../../../../crates/volicord-command-model/src/lib.rs):
  명령 문법, 가시성, traversal, synopsis, 정규 invocation, typed inbox-resolution
  invocation 구성.
- [`crates/volicord-user-action-presentation/src/lib.rs`](../../../../crates/volicord-user-action-presentation/src/lib.rs):
  adapter-neutral fact에서 공유 CLI inbox, availability, recovery instruction을
  만드는 presentation.
- [`Cargo.toml`](../../../../Cargo.toml),
  [`xtask/src/architecture.rs`](../../../../xtask/src/architecture.rs):
  Core 의존 적격성과 구조적 패키지 graph 집행.
- [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs),
  [`methods/`](../../../../crates/volicord-core/src/methods/),
  [`policy/`](../../../../crates/volicord-core/src/policy/): typed Core 조율,
  메서드 planning, 집중 policy.
- [`crates/volicord-types/src/methods.rs`](../../../../crates/volicord-types/src/methods.rs):
  fields-only 및 완전한 adapter-neutral 공개 result 선언.
- [`crates/volicord-mcp-protocol/src/lib.rs`](../../../../crates/volicord-mcp-protocol/src/lib.rs):
  정확한 profile 선택과 semantic capability.
- [`crates/volicord-mcp-wire/src/`](../../../../crates/volicord-mcp-wire/src/):
  MCP wire 값, envelope, 직렬화, schema, contract descriptor.
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
