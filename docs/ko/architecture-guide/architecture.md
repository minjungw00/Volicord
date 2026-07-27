# 구현 아키텍처

이 문서는 로컬 Rust 워크스페이스를 위한 아키텍처 가이드의 상위 개요입니다.
가이드 수준의 운영 경로, 워크스페이스 형태, 의존 방향, 오래 유지될 구현 경계,
집중 세부 담당 문서로 가는 경로를 담당합니다.

이 문서는 소스 지도, 작업 흐름 추적, 테스트 전략, 변경 가이드, 제품 계약이
아닙니다. 학습 경로가 필요하면 [아키텍처 가이드](README.md)에서 시작합니다.
정확한 동작은 집중 [참조 색인](../reference/README.md)을 봅니다. 아래 표에서
구현 질문에 맞는 아키텍처 가이드 문서로 이동할 수 있습니다.

Volicord는 AI 지원 제품 작업을 위한 로컬 작업 권한 기록입니다. Core는 Volicord
상태를 위한 로컬 기준 기록입니다.

이 체크아웃은 이 저장소가 유지하는 Volicord 소스 저장소이자 Rust
워크스페이스입니다. 구현 크레이트, 테스트, 문서, 검증 도구, 저장소 설정을
담습니다. Volicord 설치는 배포된 실행 파일과 필요한 런타임 리소스의 부분집합이므로
이 워크스페이스 개요는 설치 매니페스트가 아닙니다.

직접 열 수 있는 코드와 테스트 경로는 저장소 루트 기준으로 씁니다.

## 운영 경로

아래 그림은 현재 세 진입 경로인 관리형 stdio MCP, 관리 CLI, CLI
받은 편지함 `User Channel`을 구분합니다. 실선은 주된 호출 또는 저장 방향을
나타냅니다. 점선은 검증, 관찰된 입력, 공개 메서드 실행 밖에서 일어나는 작업을
나타냅니다.

```mermaid
flowchart LR
  host["MCP 호스트 / Agent Connection"]
  launcher["volicord _host-launch<br/>(숨은 bootstrap)"]
  mcp["volicord-mcp stdio 어댑터"]
  cli["volicord 관리 CLI"]
  inbox["volicord inbox"]
  syntax["volicord-command-model"]
  presentation["UserAction presentation"]
  core["volicord-core"]
  store["volicord-store<br/>(아티팩트 기능 포함)"]
  runtime["Volicord Runtime Home"]
  product["Product Repository"]

  host --> launcher --> mcp --> core
  launcher -. 엄격한 구성 재검증과 일회성 launch lease .-> store
  mcp -. 시작 및 세션 검증 .-> store
  mcp --> presentation
  cli --> store
  cli --> syntax
  inbox --> core
  inbox --> presentation
  presentation --> syntax
  core --> store
  store --> runtime
  product -. 관찰 입력 및 담당 문서가 정의한 경로 .-> core
  host -. 공개 API 밖의 제품 파일 도구 .-> product
```

숨은 launcher와 `volicord-mcp` 어댑터 라이브러리는 시작 검사, Agent Connection 맥락, session 검증,
요청 시점 project routing을 위해 Store를 직접 사용할 수 있습니다.
이 직접 Store
사용은 공개 Volicord 메서드 의미를 구현하는 다른 경로가 아닙니다. 공개 메서드
실행은 Core를 통과합니다.
Launcher는 원래 프로세스 안에서 현재 관리 구성을 다시 검증하고 수명이 짧은
Registry lease를 발급한 뒤 claim을 메모리에서만 MCP 어댑터로 넘깁니다. 공개
`volicord mcp serve` 진입점은 수동 전송이며 항상 `manual_cli`를 기록합니다.

`Product Repository`는 별도의 제품 파일 경계로 남습니다. 공개 Volicord 메서드는
담당 문서가 정의한 호환성, 관찰, 판단, 증거, 아티팩트 링크를 기록합니다. 제품
파일 쓰기 자체는 공개 메서드 실행 경로 밖에서 Agent Connection, 로컬 도구, 또는
명시적인 관리 통합 경로가 수행합니다.

## 워크스페이스 형태

| 워크스페이스 멤버 | 가이드 수준 역할 |
|---|---|
| `crates/volicord-types` | 공유 요청, 응답, 스키마 형태, 값 집합, 식별자, 정규 해시, 플랫폼, 호스트 구성 타입, 의미 UserAction 요청·불변 resolution·adapter-neutral `UserActionResolutionForm` 타입, 선언 하나에서 함께 만드는 공개 메서드 결과와 필드 전용 구성 타입, 진단 lifecycle 및 `CurrentDiagnosticKey` identity 타입, 선택한 Connection 및 lifecycle-aware lookup report 타입, 공유 tagged integration-verification workflow 모델, 정규 `AgentToolId` catalog와 wire 이름 투영. CLI presentation model이나 rendering helper는 담당하지 않습니다. 공개 Rust 어휘는 각 항목을 담당하는 모듈 경로로 제공하며, 크레이트 루트는 타입 집계 파사드가 아니라 모듈 경로를 제공합니다. |
| `crates/volicord-host-contract` | `CodexMcpTurnMetadata`, `CodexCommandHooks`, `CodexMcpCallableNames`를 통한 의존성이 안전한 semantic Codex 계약, 명시적인 MCP server/raw-tool identity, 충돌 검사를 거친 callable 투영과 catalog 조회, 결정적인 contract identity, 한도 있는 host 값과 error, source별 상관관계 타입. Store, Core, CLI, MCP policy는 소유하지 않습니다. |
| `crates/volicord-store` | 정규 SQLite 저장소, Runtime Home, 부트스트랩, 프로젝트 Store, 일회성 managed MCP launch lease, Agent Connection runtime/project session, lifecycle별 구조화 finding 영속화, 명시적인 진단 조회 및 cause graph 순회 API, 아티팩트 저장소, 검사, 내보내기 스냅샷, 저장소 오류 구현. |
| `crates/volicord-core` | 어댑터와 독립적인 Core 서비스, 호스트 중립 typed invocation authority, 공유 typed 요청 및 결과 구성 파이프라인, 필드 전용 메서드 계획, 정책 점검, 분기 사실이 확정된 뒤의 최종 응답 구성, Store 조율. |
| `crates/volicord-command-model` | `volicord` 바이너리의 완전한 Clap 선언을 담당합니다. Root parser, 공개 및 숨은 하위 명령 tree, 명령과 인수 DTO, 명령 표면의 value enum과 validator, 실제 모델 기반 가시성 분류, 새로운 root `clap::Command`, 명령 경로 순회, 정규 synopsis, parsing 가능한 정규 공개 invocation, 그 선언에서 도출한 typed inbox-resolution invocation builder를 제공합니다. 명령 실행이나 application service는 담당하지 않습니다. |
| `crates/volicord-user-action-presentation` | `CliUserActionInboxResponse`, `CliUserActionInboxItem`, tagged channel/capture-path 상태, CLI JSON Schema, command-model 기반 recovery instruction을 담당하는 typed CLI UserAction presentation입니다. Shared semantic type과 정규 command-model invocation을 사용하며 Core 정책, Store 접근, command 실행, terminal rendering, MCP envelope는 담당하지 않습니다. |
| `crates/volicord-cli` | 설정, 프로젝트 등록, CLI 받은 편지함 명령, Codex Agent Connection 설치·검증·복구·제거, host/MCP/Guard 검증 check, dependency graph 정책, 렌더링, 숨은 동일 프로세스 managed-host launcher, 관리형 stdio MCP 감독 정책·기한·프레이밍·진행 상태·진단을 위한 로컬 `volicord` 프로세스 시작, 관리 명령 디스패치, 재사용 실행 모듈. |
| `crates/volicord-platform-fs` | 프로세스 target 및 플랫폼 관찰, 네이티브 Linux/WSL2 분류, WSL2 배포판 검증 및 파일시스템 관찰, typed 원자적 기존 대상 비대체 일반 파일 공개, 플랫폼 고유 파일시스템 이름 공간 연산, 정규 Runtime Home별 공유·배타 OS 기반 변경 승인과 빌린 변경 permit, 읽기 전용 정규 Git common-directory/worktree snapshot을 위한 내부 안전 파사드. 관리 시작이나 Codex 구성 정책은 담당하지 않습니다. |
| `crates/volicord-platform-process` | 한도가 있는 플랫폼별 자식 프로세스 격리와 비차단 자식 파이프 준비 상태를 위한 내부 안전 파사드. 저수준 Unix 프로세스 그룹, Windows Job Object, 파이프 폴링 primitive를 담당합니다. |
| `crates/volicord-test-process` | 저장소 테스트와 스모크 하네스에서 한도 있는 자식 프로세스 실행을 담당하는 게시 비활성 내부 경계. `volicord-platform-process` primitive를 하나의 기한, 동시 한도 stdio 수집, 프로세스 트리 종료, 직접 자식 회수, 한도 있는 정리로 조합하며 제품 프로세스 정책은 담당하지 않습니다. |
| `crates/volicord-mcp-protocol` | 정확한 MCP 리비전 파싱, 검토된 폐쇄형 프로덕션 레지스트리, 완전한 revision-to-semantic-capability map, 결정론적인 지원 리비전 순회, 지원하지 않는 리비전의 명시적 거절을 담당하는 호스트 독립 내부 크레이트. 추적 중인 사전 릴리스 메타데이터는 프로덕션 레지스트리 밖에 둡니다. |
| `crates/volicord-mcp` | 정규 관리 launch 구성 계약, 메모리 내 launch-lease 소비, 시작 검증, registry가 구동하는 단일 실행 가능 protocol 적합성 harness, Volicord 도구 담당자가 제공하는 정규 도구 모델 사용, capability 기반 `tools/list` 및 `tools/call` projection, 분리된 mutation/UserAction/recovery/authority/telemetry/metric 책임, stdio lifecycle과 프레이밍, Core 호출을 위한 MCP 어댑터 라이브러리. |
| `crates/volicord-test-support` | 재사용 가능한 구현 테스트 fixture만 담당합니다. 폐기 가능한 Runtime Home과 Product Repository 설정, Store 검사, Core 요청 빌더, Agent Connection 설정을 제공하며 제품 동작 assertion이나 계약은 담당하지 않습니다. |
| `tests/conformance` | Core 쪽 API, 공유 픽스처, 버전별 오프라인 MCP 명세 입력을 통한 기준 범위 교차 메서드 시나리오. 고정된 upstream 입력은 런타임 지원을 정의하지 않습니다. |
| `tests/integration` | MCP, Core, Store, Agent Connection session, 작업 범주, 공개 스키마 스냅샷을 가로지르는 테스트. |
| `tests/release-integrity` | 일반 target 다섯 개 범위, 버전 일치, 기준 텍스트 바이트, 패키지 형태, 패키징한 binary identity, checksum 출력, 릴리스 workflow 구조. 운영 런타임 동작을 담당하지 않습니다. |
| `tests/release-smoke` | 게시하지 않는 플랫폼 공통 실제 바이너리 스모크 패키지. 전달받은 정확한 `volicord` 바이너리를 공개 `init` 및 `mcp serve`로 실행하고, 폐기 가능한 런타임 fixture와 안정적인 테스트 소유 Codex 실행 파일을 담당하며, 선호 initialize 리비전과 대표 정규 도구 identity를 검증합니다. 한도가 있는 자식 실행은 `volicord-test-process`에 위임하며 CLI, MCP 어댑터, Core, Store, `xtask`를 링크하지 않습니다. |
| `xtask` | 워크스페이스 아키텍처 검증, 문서 검증, 고정 MCP 명세 manifest 처리, 릴리스 버전 검사를 위한 가벼운 저장소 유지보수 도구. 아키텍처 검증기는 Cargo에서 실제 패키지 manifest, target source root, 의존성 데이터를 읽어 graph를 워크스페이스 메타데이터 담당 원본과 비교합니다. 문서의 명령 예시는 `volicord-command-model`로 parsing하고 런타임 담당 작업 범주 식별자는 `volicord-types`에서 가져오며, 네트워크 작업은 MCP 명세 동기화만 수행합니다. `xtask`는 런타임 어댑터, Core, Store, CLI, platform 크레이트를 링크하지 않으며 Volicord 런타임 아키텍처 밖에 있습니다. |

## 의존 경계

루트 `Cargo.toml`의 `workspace.metadata.architecture`가 워크스페이스 패키지
배정, 의미 기반 책임 그룹, 허용되는 내부 의존 방향, Core 의존 적격성을 위한
기계 판독 담당 원본입니다. 각 그룹은 일반, 개발, 빌드 의존 대상 그룹을 별도로
선언하고 자신을 Core 런타임 의존성, Core 개발 의존성, 또는 Core 의존 계층 밖으로
분류합니다.
`cargo run -p xtask -- architecture-check`는 Cargo 메타데이터에서 실제
워크스페이스 멤버와 의존 간선을 읽고, 선언되지 않은 패키지나 대상 그룹 및
허용되지 않은 간선을 거부합니다. 프로덕션 그룹은 명시적으로 허용된 개발
의존성으로만 테스트 지원 그룹을 사용할 수 있습니다. Core의 일반 및 빌드
의존성은 Core 런타임 그룹만 대상으로 하고, 개발 의존성은 Core 개발 그룹도
대상으로 할 수 있습니다. Core 쪽 그룹은 어댑터 그룹에서 독립적으로
유지됩니다. 이 규칙은 현재 워크스페이스에 직접 적용하며 패키지, 스키마,
프로토콜 버전으로 다른 의존 규칙을 선택하지 않습니다.

오래 유지될 의존 방향은 아래와 같습니다.

- `volicord-types`는 공유 타입 경계에 있으며 내부 제품 크레이트 의존성이 없습니다.
  Lifecycle별 진단 입력, current key identity와 digest 파생, 공유 read-only finding과
  report projection, 안정적인 네임스페이스 코드 검증, 담당 크레이트의 typed fact에
  한도와 민감정보 제거를 적용하는 projection, 정규 tool identity catalog를 담당합니다.
  사용하는 쪽에서는 크레이트 루트를 공유 타입 이름 공간으로 다루지 않고 `ids`,
  `methods`, `schema`, `tool_names`, `values` 같은 적용되는 담당 모듈을 명시합니다.
  각 도메인 크레이트는 폐쇄형 세부 code 집합과 오류를 finding으로 빠짐없이 변환하는
  책임을 유지합니다.
- `volicord-host-contract`는 저수준 공유 타입과 범용 serialization 및 hashing에만
  의존합니다. Store, CLI, MCP는 명시적인 `codex-command-hooks`,
  `codex-mcp-turn-metadata`, `codex-mcp-callable-names` 계약과 typed 상관관계를 사용합니다. 이 크레이트는 Store,
  Core, CLI, MCP에 의존하지 않습니다.
- `volicord-store`는 공유 타입과 저장된 소유자 경로 검증에 쓰는 읽기 전용 정규 Git
  layout primitive에 의존하고 지속 저장 메커니즘을 담당합니다. Core, CLI, MCP 어댑터
  크레이트에는 의존하지 않습니다.
- `volicord-core`는 Store와 공유 타입에 의존하고 테스트 지원은 개발 의존성으로만
  사용합니다. 공개 invocation 경계는 typed local `UserActionChannelKind` 또는
  불투명한 `ValidatedAgentSession`으로 구성된 `InvocationAuthority`를 받습니다.
  Core는 어댑터가 선택한 actor label, 호스트 명령 문법, 시작 인자, 구성 경로,
  렌더링 지시를 받지 않습니다.
- `volicord-command-model`은 Clap에만 의존합니다. CLI와 문서 validator가 이
  크레이트를 직접 사용하며, Core, Store, MCP, CLI 렌더링, Runtime Home 구현,
  application service에는 의존하지 않습니다.
- `volicord-user-action-presentation`은 shared type과
  `volicord-command-model`에 의존합니다. CLI와 MCP는 이 CLI 지향 presentation을
  사용할 수 있지만 Core와 Store는 이 크레이트에서 독립적입니다.
- `volicord-mcp-protocol`은 내부 제품 크레이트에 의존하지 않습니다. Core, Store,
  CLI, 호스트 통합, Volicord 도구 구현에 의존하지 않으면서 폐쇄형 리비전 프로필
  경계를 담당합니다.
- `volicord-cli`와 `volicord-mcp`는 어댑터 또는 로컬 오케스트레이션 계층입니다.
  CLI는 `volicord-command-model`을 사용하고, 두 어댑터는 각자의 설정, 시작 검증,
  처리 경로, 호출 책임을 위해 Core, Store, 공유 타입에 의존할 수 있습니다.
  `volicord-mcp`는 리비전 프로필 담당 경계를 사용하기 위해
  `volicord-mcp-protocol`에도 의존합니다.
- `volicord-platform-process`는 내부 제품 크레이트에 의존하지 않습니다. MCP 감독
  정책, 기한, 프레이밍, 진행 상태, 진단을 담당하지 않으며 로컬 오케스트레이션
  계층에 안전한 자식 프로세스 격리와 파이프 폴링 primitive를 제공합니다.
- `volicord-test-process`는 `volicord-platform-process`와 범용 테스트
  인프라에만 의존합니다. 저장소 테스트와 스모크 하네스가 OS primitive를 한도
  있는 테스트 자식 실행으로 재사용하게 합니다. 제품 MCP 감독 정책, lifecycle
  기한, 프레이밍, 진행 상태, 진단은 계속 `volicord-cli`가 담당합니다.
- `volicord-platform-fs`는 내부 제품 크레이트에 의존하지 않습니다. 현재 프로세스
  target과 플랫폼, WSL2 배포판 identity와 경로 파일시스템, 플랫폼 고유 이름 공간
  연산, 정규 Runtime Home별 공유·배타 OS 기반 변경 승인, 빌린 변경 permit,
  읽기 전용 Git layout identity primitive의 안전한 관찰을 담당합니다. Store와 로컬
  어댑터는 각자의 계획, 관리 파일 정책, 소유권 및 권한 비교, 복구, 진단 책임을
  유지합니다.
- `tests/release-smoke`는 `volicord-mcp-protocol`, `volicord-types`,
  `volicord-test-process`에 의존합니다. 제품 구현 크레이트나 `xtask`에
  의존하지 않으면서 릴리스 전용 프로세스 한도, 폐기 가능한 fixture 설정, MCP
  transcript assertion, 안정적인 Codex fixture identity, 결과 보고를 담당합니다.
- 그 밖의 테스트 지원 크레이트와 테스트 패키지는 폐기 가능한 fixture와 계층 간
  검증을 위해서만 구현 크레이트를 조합합니다.
- `xtask`는 제품 런타임 밖의 저장소 유지보수 도구로 남습니다. 문서의 공개 CLI
  invocation은 `volicord-command-model`로 parsing하고, 런타임 담당 계약 식별자는
  `volicord-types`에서 가져오며, `volicord-mcp-protocol`을 사용하여 컴파일된
  프로덕션 profile의 일치를 확인합니다. MCP 어댑터, Core, Store, CLI, host
  integration 크레이트를 링크하지 않습니다.
  일반 검사는 오프라인으로 실행되고, 명시적으로 호출한 명세 sync 명령만 네트워크를
  사용합니다.

실제 의존 간선은 Cargo 매니페스트가 담당하고,
`workspace.metadata.architecture`는 어떤 내부 간선을 허용하는지 담당합니다.
정확한 소스 배치는 소스 지도가 담당합니다.

### Store 변경 가시성

Store의 쓰기 가능 데이터베이스 open과 저수준 변경 helper는 crate-private으로
유지합니다. 외부 caller는 하나의 정규 Runtime Home에 대해 보유한 정확한
`RuntimeHomeMutationPermit`을 빌리는, 복제할 수 없는 활성
`RuntimeHomeMutationContext`를 요구하는 공개 API를 통해서만 변경 경로에
진입합니다. Rust 가시성과 위조할 수 없는 수명 관계가 외부 크레이트의 내부 open
호출 및 분리된 변경 context 구성을 막습니다. Compile-fail coverage는 접근할 수
없는 Store 진입점과 permit에 결속된 context 수명을 함께 보호합니다.

## Core UserAction 담당 경계

`crates/volicord-core/src/user_action/`은 메서드별 조율을 제외한 공유 UserAction
도메인 및 애플리케이션 동작을 담당합니다. `model.rs`는 의미 의도, 검증된 구성 값,
adapter-neutral fact type을 담습니다. `validation.rs`, `body.rs`, `identity.rs`는
각각 순수 의미 검증, 정규 typed `UserActionRequestBody` 및 `UserActionBasis` 구성,
안정적인 source identity와 request ID 할당을 담당합니다. `service.rs`는 현재 Store
fact를 취득하고 쓰기 없이 이 순수 단계를 조율합니다. `materialization.rs`는 연산
identity를 더해 공개 request 또는 불변 resolution을 만들고, `persistence.rs`는
유효 Store record와 typed `CoreStorageMutation`으로 정확히 매핑합니다. 직렬화는
이 구체화 및 영속화 경계에서만 수행합니다.

`authority.rs`는 저장된 권한을 엄격하게 디코딩하고 정규화하며, `lifecycle.rs`는
투영된 Task lifecycle을 해석합니다. `resolution.rs`는 typed resolution 검증과
구성을 담당하고, `continuity.rs`는 수락된 resolution에서 파생되는 continuity
record를 계획합니다. `projection.rs`, `reader.rs`, `summary.rs`는 각각 neutral
pending/current fact 투영, Store 기반 읽기와 원래 result replay, adapter-safe
요약을 담당합니다. 이 fact에는 의미 좌표, status, availability, safe resolution
fact가 있으며 command string, label, CLI capture metadata, rendered instruction,
MCP envelope는 없습니다.

`crates/volicord-core/src/methods/user_action.rs`는 직접 request 및 resolution
메서드 조율만 유지하며 공유 typed 결과를 각 연산 결과에 반영하는 방식을
결정합니다. Caller는 의미 의도를 `user_action::service`에 제공하며 정규 저장
action JSON을 직접 구성하지 않습니다.

`volicord-types`는 저장된 의미 request body에서 adapter-neutral
`UserActionResolutionForm`을 도출합니다. `volicord-user-action-presentation`은 이
fact를 typed `CliUserActionInboxResponse`, 폐쇄형 channel/capture-path 상태, JSON
Schema로 투영합니다. Command text는 실제 Clap 선언에서 도출한 typed
`volicord-command-model` invocation에서 얻습니다. CLI는 이 typed model을 직접
렌더링합니다. MCP는 자체 safe compound protocol projection과 neutral fact-read
failure를 MCP error로 바꾸는 작업만 담당합니다. `methods/reconcile_changes.rs`와 다른 Core
consumer는 공유 책임 담당 모듈을 직접 호출하고 typed UserAction 결과를 각자
plan과 응답에 반영하는 방식을 결정합니다. 프로덕션 메서드 모듈은 형제 메서드
구현을 재사용 라이브러리로 사용하지 않습니다.

정확한 공개 동작과 스키마 의미는
[사용자 행동 요청](../reference/api/method-request-user-action.md),
[사용자 행동 해결](../reference/api/method-resolve-user-action.md),
[변경 조정](../reference/api/method-reconcile-changes.md),
[사용자 행동 스키마](../reference/api/schema-user-action.md)에 남습니다. Store
기록과 효과는 집중 [저장소 담당 문서](../reference/storage.md)에 남습니다.

## Core 증거 및 닫기 준비 상태 경계

Core는 증거 사실 취득과 증거 정책 평가를 분리합니다.
`crates/volicord-core/src/methods/evidence_facts.rs`는 공유 Store 조회, 담당
레코드의 엄격한 디코딩, 저장된 증거와 투영된 증거를 위한 typed 정책 입력 구성을
담당합니다. `crates/volicord-core/src/policy/` 아래의 책임별 모듈은 출처,
관련성과 뒷받침 여부, 대상과 `CurrentCloseBasis` 일치, 생산자와 아티팩트 결속,
닫기 준비 상태의 증거 해석을 평가합니다. 필요한 입력이 이미 있으면 이 평가는
순수 함수로 수행됩니다.

닫기 준비 상태는
`crates/volicord-core/src/methods/close_readiness/`의 책임별 패키지가
담당합니다. `mod.rs`는 메서드 plan이 사용하는 좁은 typed 표면을 노출합니다.
`facts.rs`는 수락 기준, workflow policy, 현재 `CoreProjectStore`를 통한
미조정 변경 조회를 포함해 현재 사실 집합을 한 번 취득하고, 준비 상태를
판단하지 않은 채 메서드 projection을 조립합니다. `change_control.rs`, `evidence.rs`,
`acceptance.rs`는 각각 변경 및 쓰기, 증거 및 아티팩트, 사용자 권한 조건을
평가합니다. 증거 평가는 `crates/volicord-core/src/policy/` 아래의 집중된 순수
정책 모듈을 호출하며 출처, 관련성, 대상, 결속, 증거 게이트 정책을 다시
구현하지 않습니다.

`policy.rs`는 취득한 사실에서 control을 해석하고 이 typed 평가를 닫기 계약
순서대로 순수하게 결합하여 닫기 상태를 선택합니다. Store handle은 받지
않습니다. `blockers.rs`는 공개 typed 차단 사유 projection을 구성하고
정규화합니다. `guidance.rs`는 typed 담당 메서드와 연산 범주를 포함한 의미 기반
후속 행위 선택을 담당하며 CLI 명령, 캡처 경로, Markdown 지침, adapter rendering,
credential을 구성하지 않습니다. `summary.rs`는 전체 닫기 평가와 더 작은
메서드 중립 준비 상태 projection을 담당합니다. `service.rs`는 이 좁은 API를
통해 사실 취득, 책임별 평가, 순수 정책 결합을 조율합니다. 다른 Core 도메인과
공유하는 현재 근거 및
사용자 권한 호환성 순수 함수는
`crates/volicord-core/src/policy/close_readiness.rs`에 남습니다.

`close_task.rs`는 요청별 닫기 연산 조율을 담당합니다. 공개 요청을 검증하고
닫기 준비 상태 서비스를 호출하며, 종료 변경을 계획하고 typed 닫기 결과를
구성합니다. `status`는 닫기 준비 상태 요약을 사용합니다. `prepare_write`는
패키지의 차단 사유 projection 표면을 사용합니다. `reconcile_changes`,
`intake`, `update_scope`, `record_run`, `user_action`은 각자 투영한 typed 사실을
닫기 준비 상태 서비스에 전달합니다. 어떤 메서드 모듈도 형제 메서드에 재사용
가능한 닫기 준비 상태 정책이나 adapter 표현을 제공하지 않습니다.

프로덕션 메서드 모듈은 적용되는 Core pipeline, 서비스, 정책, Store,
`volicord-types` 담당 모듈에서 의존성을 명시합니다. 상위 methods 모듈은 실제로
공유되는 조율 helper만 담당하며 하위 모듈의 import prelude가 아닙니다. 정확한
증거와 닫기 준비 상태의 의미는
[Core 모델](../reference/core-model.md), 적용되는
[API 메서드 담당 문서](../reference/api/methods.md),
[상태 스키마](../reference/api/schema-state.md),
[아티팩트 스키마](../reference/api/schema-artifacts.md), 집중
[저장소 담당 문서](../reference/storage.md)에 남습니다.

## 정규 릴리스 경계

경계 어댑터는 담당 문서가 정의한 현재 입력을 하나의 정규 내부 모델로 디코딩합니다. Codex
wire 경계는 버전이 지정된 `volicord-host-contract` profile을 명시적으로 선택하며 field
형태에서 profile을 추론하거나 MCP 상관관계를 hook 상관관계로 재사용하지 않습니다.
Core와 Store는 호스트 설정 문법, 셸 문법, 생성 wrapper, 플랫폼 명령 문자열에 따라
분기하지 않습니다. Store는 매니페스트와 정규 SQL digest가 현재 릴리스 계약과
일치하는 데이터베이스만 엽니다. Codex 어댑터는 Codex 구성의 parsing과 직렬화,
관리 entry 검증을 담당합니다. 플랫폼 파일시스템 경계는 프로세스 target과 환경 관찰,
target 및 파일시스템 검증을 별도로 담당합니다. MCP는 Store가 소유한 현재
runtime/project session을 검증하고 typed `ValidatedAgentSession`을 Core에 제공합니다.

실패, 저장소, Agent Connection 계약은
[실패 모델](../reference/failure-model.md),
[저장소 버전 관리](../reference/storage-versioning.md),
[Agent Connection](../reference/agent-connection.md)이 담당합니다.

### Activation-state 소유권

Activation은 기존 경계를 가로지르는 typed projection 하나입니다.

```text
host/config 조사 + Store session/event 근거
  -> volicord-cli 집중 check와 계층형 activation plan 하나
  -> volicord-types ConnectionVerificationReport + IntegrationActivationPlan
  -> concise / verbose / JSON projection
```

`volicord-types`는 `HookActivationState`, `IntegrationActivationState`, 집중 check
dependency, 안정적인 `ActivationStep` metadata, 결정적인 prerequisite 순서, 정규 typed
tool reference를 포함하는 폐쇄형 `IntegrationVerificationWorkflowState`를 담당합니다.
보고서가 유일한 activation plan 소유자이며 host plan과 effect에는 병렬 action 목록이
없습니다. `volicord-cli`는 현재 managed
configuration, host reload, hook source, session, capability, Guard, 별도 project-trust
근거를 수집합니다. `volicord-store`는 Guard definition 경계를 보존하며 불변 semantic
verification 좌표에서 공유 workflow 상태를 만드는 유일한 domain projector입니다.
`volicord-host-contract`는 semantic synchronous/deferred observation policy를 담당하고
현재 Codex 계약은 version 분기 없이 synchronous status read 한 번을 선택합니다.
Begin, probe, get, `volicord-mcp`, CLI check, 생성 host guidance는 모두 그 projection을
사용합니다. 정확한 replay는 같은 verification ID와 상태를 유지하고 `complete`와 typed
`repair_required`는 terminal로 남습니다. Adapter와 renderer는 별도의 상태를 파생하거나
summary 산문을 분류하지 않습니다. 사용자 수준
`request_integration_verification` step은 list, begin, workflow가 지시한 probe,
workflow가 지시한 status 호출을 중첩하며 Guard probe는 최상위 사용자 action이
아닙니다.

CLI는 Guard를 독립된 check/evidence 경로 둘로 projection합니다.
`AmbientGuardCoverageEvidence`는 현재 definition과 configured-phase coverage를
담당합니다. `CorrelatedGuardAttemptEvidence`와 `CorrelatedGuardProof`는 최신 attempt와
최신 완료 proof를 담당합니다. Report context는 Guard와 managed-MCP runtime session을
함께 수집하고 폐쇄형 evidence role 네 개를 정규화하며 관련 verification ID를
보존합니다. Diagnostic은 typed repair reason과 acquisition stage로 선택하고 renderer
문구나 숫자형 Codex version으로 선택하지 않습니다.

Store 내부의 integration-verification facade는 생성·재개, probe acknowledgement,
event correlation, bounded observation, typed repair/retry projection, coordinate 검증,
SQL row 변환을
lifecycle별 모듈에 위임합니다. 각 변경 진입점은 자신의 즉시 Registry
transaction을 소유하며 row 및 coordinate helper는 transaction을 열거나
데이터베이스 표현을 Store 밖으로 노출하지 않습니다.

Host가 제공한 disabled, policy-managed, invocation-bypass 근거는 typed host evidence
경계를 통해서만 받습니다. 이 근거가 없으면 Volicord는 현재 definition에 맞는 호환 event로
hook 효과를 성립시키거나 `unknown`을 보고할 수 있을 뿐 trust 상태를 만들 수 없습니다.
Core 권한 부여는 계속 분리되어 각 managed MCP 호출을 검증합니다.

## 오래 유지될 구현 경계

| 경계 | 개요 책임 | 세부 사항과 계약 경로 |
|---|---|---|
| 공유 진단 구조 | `volicord-types`는 lifecycle별 finding 입력, current key identity와 digest 파생, 의존성 안전한 read-only finding, cause 및 action 표현, 선택한 Connection 보고서, 분리된 불변 MCP preflight 및 활성 검증 증거 type, 별도의 lifecycle-aware 정확한 lookup envelope를 담당합니다. 각 도메인 담당자는 폐쇄형 typed 오류와 사실을 이 구조로 변환하며, 지속 저장, 검증, 렌더링은 기존 담당 경계에 남습니다. | [소스 지도](source-map.md), [테스트 전략](testing-strategy.md), [실패 모델](../reference/failure-model.md), [보안](../reference/security.md). |
| 관리 명령 모델 | `volicord-command-model`은 완전한 Clap 선언, 문법 DTO와 validator, 실제 명령 tree 기반 공개/숨김 분류, root command 구성, 명령 경로 순회, 정규 synopsis, parsing 가능한 정규 공개 invocation, 같은 tree에서 경로와 option spelling을 도출하는 typed invocation builder를 담당합니다. 프로세스를 시작하거나 명령을 디스패치하거나 Core 또는 Store를 호출하거나 출력을 렌더링하거나 Runtime Home을 해석하거나 application service를 실행하지 않습니다. | [CLI 작업 흐름](cli-workflows.md), [소스 지도](source-map.md), [관리 CLI](../reference/admin-cli.md). |
| CLI 운영 진단 | `volicord-cli`는 불변 운영 definition, 폐쇄형 typed subject와 facts, typed action 선택, 담당자 범위 current-condition 영속화를 `operational_diagnostics`에 둡니다. 별도의 Connection 검증 패키지는 host, MCP, Guard check, dependency graph 평가, 보고서 입력, 불변 preflight 생성, 활성 쓰기 및 conformance 증거를 조율하고 typed 관찰을 finding으로 투영합니다. Store는 lifecycle 및 조회 구현 담당을 유지합니다. | [소스 지도](source-map.md), [실패 모델](../reference/failure-model.md), [관리 CLI](../reference/admin-cli.md), [Agent Connection](../reference/agent-connection.md). |
| 진단 영속화와 조회 | `volicord-store`는 caller data보다 먼저 완전한 staged 초기화와 원자적 공개로 비권한 진단 carrier를 만들고, 추가 전용 occurrence, 교체 가능한 current snapshot, cause graph 검증 및 순회, lifecycle-aware 정확한 lookup 및 graph API, 현재 보고서 projection, 내부 row 인코딩을 분리합니다. 정확한 read는 occurrence/current lifecycle, active/resolved 상태, 해소 시각을 유지하고, 보고 가능한 read는 적격 occurrence와 active current finding만 projection합니다. | [소스 지도](source-map.md), [저장소](../reference/storage.md), [저장소 레코드](../reference/storage-records.md), [실패 모델](../reference/failure-model.md). |
| Core와 어댑터 | Core는 어댑터와 독립적인 공개 메서드 처리와 adapter-neutral semantic fact를 담당합니다. 공유 UserAction presentation은 이 fact의 CLI 지향 projection을 담당하고 CLI는 terminal rendering, MCP는 protocol envelope와 adapter failure mapping을 담당합니다. Core는 adapter 또는 presentation 계층에 의존하지 않습니다. 읽기 전용 caller는 경로에서 `CoreService`를 구성하고, 승인된 caller는 두 번째 Runtime Home 좌표 없이 `RuntimeHomeMutationContext`에서만 구성합니다. | [요청 생명주기](request-lifecycle.md), [구현 설계 패턴](design-patterns.md), [Core와 어댑터 의존 경계](design/core-adapter-boundary.md), [API 메서드](../reference/api/methods.md), [MCP 전송](../reference/mcp-transport.md), [관리 CLI](../reference/admin-cli.md). |
| Codex host-wire 계약 | `volicord-host-contract`는 `CodexMcpTurnMetadata`/`codex-mcp-turn-metadata`, `CodexCommandHooks`/`codex-command-hooks`, `CodexMcpCallableNames`/`codex-mcp-callable-names`, 결정적인 profile digest, 한도 있는 값과 failure, 서로 바꿔 쓸 수 없는 상관관계 타입, 명시적 `McpServerKey`와 완전한 `McpRawToolName`에서 `HostCallableIdentity`로 가는 투영을 담당합니다. 또한 검토된 native host tool과 server-qualified MCP routing을 통합하는 typed hook-routing strategy를 소유하며, 등록된 namespace를 표현할 수 있으면 이를 사용하고 아니면 catalog에서 파생한 exact callable을 사용합니다. 생성 구성은 이 strategy를 투영하고 엄격한 audit은 이를 다시 구성합니다. 정규 `AgentToolId` catalog는 모든 tool에 probe target, workflow control, unrelated known tool 중 정확히 하나의 integration-verification role을 부여하고, `McpToolCatalog`는 정규화 충돌뿐 아니라 모순되는 role metadata도 거부합니다. Routing이 event를 전달하면 Store는 probe 좌표보다 먼저 callable과 role을 해석합니다. Workflow control과 그 밖의 known tool은 한도 있는 nonterminal trace로만 남고, probe target만 session, turn, verification ID, tool-use 검사를 계속합니다. Codex package version에서 host 동작을 선택하지 않습니다. | [소스 맵](source-map.md), [MCP 전송](../reference/mcp-transport.md), [Agent Connection](../reference/agent-connection.md), [저장소 레코드](../reference/storage-records.md), [실패 모델](../reference/failure-model.md). |
| Runtime Home과 Product Repository | `Volicord Runtime Home`은 저장소/런타임 담당 문서가 정의하는 Volicord 런타임 기록과 아티팩트 데이터를 담습니다. `Product Repository`는 사용자 제품 파일과 담당 문서가 허용하는 명시적 통합 파일을 담습니다. | [저장소와 트랜잭션](storage-and-transactions.md), [Runtime Home과 Product Repository 분리](design/runtime-home-and-product-repository.md), [런타임 경계](../reference/runtime-boundaries.md), [보안](../reference/security.md). |
| Runtime Home bootstrap | `volicord-store`는 기존 Registry를 읽기 전용으로 열어 정확한 현재 manifest와 물리 schema를 검사합니다. 최종 경로가 없으면 같은 상위 directory의 staging에 불투명한 publication ID, singleton, 최초 installation profile을 함께 준비합니다. 기존 대상을 교체하지 않는 원자적 rename에 성공하면 상위 directory 동기화와 read-back 검증 전에 invocation별 publication guard를 만듭니다. `AlreadyExists`는 읽기 전용으로 정확히 검증한 현재 승자만 반환합니다. | [런타임 경계](../reference/runtime-boundaries.md), [저장소 버전 관리](../reference/storage-versioning.md), [저장소 레코드](../reference/storage-records.md), [저장소 DDL](../reference/storage-ddl.md). |
| Runtime Home 변경 승인 | `volicord-platform-fs`는 정규 Runtime Home마다 OS 기반 공유·배타 coordination 영역 하나를 담당합니다. `SharedWriter`는 일반 writer의 동시 진입을 허용하고 `ExclusiveSetup`은 모든 공유 또는 배타 holder와 충돌합니다. 빌린 permit은 OS handle을 노출하지 않고 정확한 정규 target과 보유 mode를 `volicord-store`에 전달하며, Store는 쓰기 가능 database open과 변경 API에 필요한 복제 불가능한 `RuntimeHomeMutationContext`를 만듭니다. Setup은 하나의 배타 permit에서 이 context를 만들고 CLI, MCP, Guard, Core, operational-session, diagnostic, artifact 연산은 공유 permit에서 만든 context를 전체 효과가 끝날 때까지 유지합니다. 그 `CanonicalRuntimeHomePath`는 변경 의존 planning, 읽기, service, handle, 진단, 효과, 응답에서 승인 뒤 유일한 identity이며 alias는 두 번째 권한 좌표를 만들 수 없습니다. Coordination 파일은 Runtime Home 밖에 있으며 lock은 actor identity, credential, security boundary가 아니라 coordination 수단입니다. | [소스 지도](source-map.md), [런타임 경계](../reference/runtime-boundaries.md). |
| Init setup transaction | `volicord-cli`는 검사와 planning 전에 정규 Runtime Home의 `ExclusiveSetup` 변경 승인을 획득한 뒤 Runtime Home, Store, Codex 구성, repository 관리 파일, activation을 아우르는 읽기 전용 typed plan 하나를 만듭니다. 복제할 수 없는 같은 lease를 준비, 공개, Store와 파일 commit, 보고, 정리, 보존 또는 전체 rollback이 끝날 때까지 유지합니다. Coordination 파일은 rollback 대상 Runtime Home 밖에 있고 OS handle을 닫으면 소유권이 해제됩니다. Prepare 단계는 snapshot을 검증하고 같은 directory에 파일을 staging하며 `volicord-store`는 복구 entry를 준비하고 checkpoint합니다. 지원되는 동시 setup은 planning 전에 busy로 보고합니다. Lease 보유 중 예상하지 않은 `AlreadyExists`가 발생하면 오래된 plan을 중단하고 Store나 관리 파일을 변경하지 않습니다. 실패하면 한도가 있는 역순 rollback을 수행하며, Runtime Home 제거에는 소유 guard가 정확한 publication ID, Runtime Home identity, manifest, 경로, schema, installation identity, managed-host 소비 부재를 다시 검증해야 합니다. Platform 제거는 재귀 효과, 정확한 경로 관찰, 실패 단계, 상위 entry 내구성을 각각 보고합니다. 확인된 제거와 불완전한 제거는 모두 guard를 terminal로 만들며, 확인 실패는 주 오류와 완전한 rollback 결과를 함께 유지합니다. 이는 여러 파일시스템 전체의 전역 원자성이 아닙니다. | [관리 CLI](../reference/admin-cli.md), [런타임 경계](../reference/runtime-boundaries.md), [Agent Connection](../reference/agent-connection.md), [실패 모델](../reference/failure-model.md). |
| 프로젝트 Store facade와 aggregate 소유권 | `CoreProjectStore`는 프로젝트 데이터베이스 facade로 유지됩니다. Handle과 open 진입점은 인프라 담당 모듈에 있고, aggregate 모듈은 자신의 grouped mutation 입력, 저장 검증과 SQL 적용, 필요한 typed 적용 사실, column projection, 읽기 SQL, 엄격한 decoding, inherent facade 메서드, 집중 테스트를 담당합니다. Task와 수락, Change Unit, Write Ticket, Run, 증거, artifact, User Action, continuity 책임은 `core_pipeline/` 아래에 있고 workflow policy record와 mutation group은 `workflow_records.rs`에 남습니다. 프로젝트 상태, replay, reconciliation 관찰, blocker, event, Agent Session, clock 데이터는 집중된 인프라 모듈을 유지합니다. | [저장소와 트랜잭션](storage-and-transactions.md), [소스 지도](source-map.md), [저장소](../reference/storage.md), [저장소 레코드](../reference/storage-records.md). |
| Store 커밋 경계 | Core 메서드 계획 코드는 읽기 전용, 효과 없음, dry-run, 스테이징, 커밋 분기를 고릅니다. 얇은 정적 mutation dispatcher가 계획 순서를 보존하고 aggregate 담당 모듈에 위임합니다. Commit coordinator는 즉시 transaction, replay 및 최신성 gate, 정규 state-version 전진 한 번, event와 replay 영속화, 응답 구성, commit 또는 rollback만 담당합니다. Artifact staging은 정상 Core mutation commit과 분리됩니다. Core 권한 의미는 Core 담당 문서에, 정확한 저장소 기록과 효과는 저장소 담당 문서에 남습니다. | [저장소와 트랜잭션](storage-and-transactions.md), [요청 생명주기](request-lifecycle.md), [Core 모델](../reference/core-model.md), [저장소](../reference/storage.md), [저장 효과](../reference/storage-effects.md). |
| MCP 프로토콜 프로필 및 적합성 경계 | `volicord-mcp-protocol`은 폐쇄형 리비전 타입 집합, 검토된 프로덕션 프로필, 메시지·도구·스키마 기능 선언, 명시적 순회 순서, 서버 선호 리비전을 담당합니다. 일반 실행 적합성 테스트와 CLI 서버 probe는 이 프로덕션 registry를 직접 순회합니다. CLI conformance는 새로운 일회용 Runtime Home과 Product Repository 상태만 사용하며 선택한 실제 database에는 명시적인 rollback-only 쓰기 가능성 probe만 수행합니다. `xtask`는 이와 독립적으로 릴리스된 manifest 프로덕션 지원과 같은 컴파일된 registry의 정확한 일치를 강제합니다. Host 호환성 fixture는 독립적으로 고정하며 서버 선호값이나 revision 적합성을 담당하지 않습니다. | [소스 지도](source-map.md), [테스트 전략](testing-strategy.md), [MCP 전송](../reference/mcp-transport.md). |
| MCP 어댑터 경계 | `volicord mcp serve`가 공개 수동 전송 진입 경로이며 숨은 launcher의 메모리 내 lease claim만 `managed_host` runtime을 만들 수 있습니다. `stdio` facade는 한도가 있는 transport, JSON-RPC envelope, lifecycle, Runtime Home/repository/Connection binding, tool dispatch, telemetry를 각각 담당하는 모듈에 위임합니다. Lifecycle은 폐쇄형 `SessionState` variant를 사용하므로 initialize 전에는 initialization 선택 정보가, initialized 전환 전에는 ready 전용 session 정보가, `Closed` 밖에서는 종료 정보가 존재할 수 없습니다. Framing은 결과 projection을 알지 못하고, JSON-RPC parsing은 Core 연산을 수행하지 않으며, lifecycle은 message를 승인하고, binding은 routing identity를 선택하며, tool dispatch는 framing을 담당하지 않은 채 호출을 디코딩하고 adapter를 호출합니다. `volicord-types`는 Core 소유 도구에 `MethodName`을 재사용하고 운영 verification role을 컴파일 시점에 결합하는 폐쇄형 `AgentToolId` identity catalog를 제공합니다. `volicord-mcp`는 정규 registry를 이 identity로 식별하고 revision별 도구 소유권을 나누지 않은 채 선택한 profile을 통해 wire 이름, 정의, 결과를 투영합니다. 어댑터는 연산 전 Runtime Home과 Agent Connection routing context를 해석한 뒤 각 활성 mutation context가 그 routing identity와 일치하는지 요구합니다. Project routing, 진단, Store 접근, Core 구성에는 승인된 identity를 사용하고 adapter 관리 local invocation fact를 파생하며 Core JSON을 MCP 콘텐츠로 감쌉니다. | [요청 생명주기](request-lifecycle.md), [소스 지도](source-map.md), [MCP 전송](../reference/mcp-transport.md), [Agent Connection](../reference/agent-connection.md). |
| 관리 CLI와 Codex 어댑터 | `volicord-cli`는 parsing된 command-model DTO에 대한 프로세스 시작과 디스패치, Codex 설정 탐색, 관리 entry 설치 및 검증, dependency-aware 검증 정책, 결정론적 진단 root 선택, 선택한 Connection 보고서의 concise/verbose/JSON 표시, lifecycle-aware finding 및 runtime-session lookup 출력, 복구, 제거, 숨은 동일 프로세스 host launcher를 담당합니다. Launcher는 일회성 Store lease를 발급하기 전에 현재 entry를 정확히 다시 검증하며 lease 자료를 구성, 인자, 환경, 출력에 두지 않습니다. Lookup 성공 여부는 저장된 finding severity와 독립적입니다. Codex 어댑터는 허용된 도구 승인 overlay만 보존하면서 정규 관리 시작 계약을 Codex TOML로 변환하고 다시 읽습니다. Linux나 WSL2를 분류하지 않습니다. | [CLI 작업 흐름](cli-workflows.md), [소스 지도](source-map.md), [관리 CLI](../reference/admin-cli.md), [Agent Connection](../reference/agent-connection.md), [보안](../reference/security.md). |
| 릴리스 무결성 | 일반 점검은 모든 게시 Volicord target, 패키지와 checksum 연속성, workflow 의미를 다룹니다. 재사용 workflow action 하나가 일반 CI의 로컬 debug build 뒤에 전용 `tests/release-smoke` 패키지를 정확히 한 번 호출하고, 네이티브 릴리스 target마다 artifact staging 전에 정확히 한 번 호출합니다. 이 패키지는 안정적인 테스트 소유 Codex fixture와 전달받은 정확한 바이너리를 사용해 공개 수동 stdio를 실행하며 선택적 실제 Codex 관찰 및 managed-host 증거와 구분됩니다. | [테스트 전략](testing-strategy.md), [검증](../maintain/validation.md). |
| 플랫폼 파일시스템 파사드 | `volicord-platform-fs`는 프로세스 target과 kernel을 관찰하고 네이티브 Linux와 WSL2를 구분하며 `/etc/os-release`를 통해 WSL2 배포판을 검증하고 target 경로 제한 집행에 필요한 파일시스템 관찰을 제공합니다. 또한 효과를 인식하는 기존 대상 비대체 일반 파일 공개와 상위 entry 내구성, 플랫폼 고유 이름 공간 primitive, 정규 Runtime Home별 공유·배타 변경 승인과 빌린 permit, 정규 읽기 전용 Git common-directory/worktree 탐색을 격리합니다. 어떤 파일을 관리할지, 교체나 쓰기를 승인할지, 복구가 무엇을 뜻할지는 결정하지 않습니다. | [소스 지도](source-map.md), [CLI 작업 흐름](cli-workflows.md), [관리 CLI](../reference/admin-cli.md), [런타임 경계](../reference/runtime-boundaries.md), [시스템 요구사항](../reference/system-requirements.md). |
| 플랫폼 프로세스 파사드 | `volicord-platform-process`는 한도가 있는 자식 프로세스 격리와 자식 파이프 준비 상태를 위한 안전한 API를 노출합니다. 저수준 프로세스 그룹, Windows Job Object, 비차단 파이프 설정, 파이프 폴링을 담당합니다. `volicord-cli`는 MCP 감독 정책, 생명주기 기한, 프로토콜 프레이밍, 교환 진행 상태, 진단 책임을 유지합니다. | [소스 지도](source-map.md), [CLI 작업 흐름](cli-workflows.md), [관리 CLI](../reference/admin-cli.md), [Agent Connection](../reference/agent-connection.md). |
| 테스트 프로세스 경계 | `volicord-test-process`는 저장소 테스트와 스모크 하네스가 재사용하는 한도 있는 자식 프로세스 실행을 담당합니다. 프로세스를 시작하기 전에 플랫폼 격리를 만들고, 하나의 lifecycle 기한 안에서 한도 있는 stdio를 함께 처리하며, 시간 초과나 실패 시 프로세스 트리를 종료하고, 직접 자식을 회수하고, 마지막 파이프 정리 시간을 제한합니다. Volicord 제품 API를 노출하지 않으며 제품 프로세스 정책은 `volicord-cli`에 남습니다. | [소스 지도](source-map.md), [테스트 전략](testing-strategy.md). |
| 테스트와 검증 | 구현 테스트는 담당 문서가 정의한 사실을 적절한 계층에서 검증합니다. MCP 모듈 테스트는 lifecycle, batching, protocol projection, tool call, managed-host observation, diagnostics, conformance 계약별로 나누며 공유 설정은 그 assertion과 분리합니다. MCP 프로덕션 지원에는 고정 manifest의 릴리스 항목과 프로덕션 profile이 필요하며 가벼운 checker가 정확한 집합 일치를 강제합니다. 독립적인 registry 기반 적합성 테스트는 모든 프로덕션 profile의 실제 wire 동작을 실행합니다. 추적 중인 pre-release schema는 프로덕션 순회 밖에 있고 저장소 로컬 적합성 범위는 외부 인증이 아닙니다. 테스트, 픽스처, 생성 스냅샷, 문서 점검은 제품 계약 담당 문서가 되지 않습니다. | [테스트 전략](testing-strategy.md), [검증](../maintain/validation.md). |

## 세부 경로

| 필요 | 경로 |
|---|---|
| 정확한 소스 경로, 모듈 책임, CLI 하위 모듈 경계, 어댑터 모듈, 테스트 지원 경로 | [소스 지도](source-map.md) |
| 크레이트, 진입 심볼, 구현 흐름을 처음 읽는 순서 | [코드베이스 둘러보기](codebase-tour.md) |
| 설정, 연결 프로비저닝, 상태 조회, 검증, doctor, guard, 호스트 및 guard 통합의 실행 흐름 경계 | [CLI 작업 흐름](cli-workflows.md) |
| 대표 MCP/Core 요청 흐름, 분기 차이, 메서드 추적, Store 상호작용, 응답 래핑 | [요청 생명주기](request-lifecycle.md) |
| Store 트랜잭션, 효과 경로, 재실행, 아티팩트 스테이징, 커밋 경계, 실패 경계 | [저장소와 트랜잭션](storage-and-transactions.md) |
| 테스트 계층 선택, 픽스처, 생성 출력 변경 점검, 오래 유지될 테스트, 검증 책임 | [테스트 전략](testing-strategy.md) |
| 변경 분류, 담당 경로 지정, 소스 경로 지정, 검증 명령 선택 | [구현 가이드](change-guide.md) |
| 현재 구현 설계, 불변 조건, 책임과 실패 경계, 범위 제외, 구현 경로, 참조 담당 문서 | [아키텍처 설계](design/README.md) |

## 설계 참조 경로

집중 구현 관심사에는 현재 설계 참조가 있습니다.

| 경계 | 설계 참조 |
|---|---|
| Agent Connection, 호스트 처리 경로, 명시적 Connection Project 멤버십 | [Agent Connection과 호스트 라우팅](design/agent-connection-routing.md) |
| Core가 MCP와 CLI 어댑터에서 독립적임 | [Core와 어댑터 의존 경계](design/core-adapter-boundary.md) |
| 정상 커밋된 Store 변이 전 메서드 계획 | [원자적 변이 커밋 전 계획](design/plan-and-atomic-commit.md) |
| 런타임 데이터와 제품 파일 분리 | [Runtime Home과 Product Repository 분리](design/runtime-home-and-product-repository.md) |
