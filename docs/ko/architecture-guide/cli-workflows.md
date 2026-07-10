# CLI 작업 흐름

이 가이드는 로컬 `volicord` 관리 작업 흐름의 아키텍처 수준 실행 경계를
담당합니다. CLI 오케스트레이션이 Runtime Home 설정, 설치 프로필 준비, Agent
Connection 기록, 호스트 어댑터, guard integration, 검증, 진단, 렌더링을 어떻게
조합하는지 설명합니다.

이 문서는 명령 문법, 플래그, stdout 또는 stderr 계약, 종료 코드, JSON 출력
스키마, 공개 API 동작, 저장 효과, 보안 보장, Core 권한 의미, 제품 계약을
정의하지 않습니다. 정확한 소스 경로와 모듈 책임은 [소스 지도](source-map.md)를
사용합니다. 정확한 명령 문법, 플래그, 결과 상태, 출력 경계, 숨겨진 hook 명령
계약은 [관리 CLI](../reference/admin-cli.md)를 사용합니다. 정확한 런타임, 연결,
전송, 비보장 표현이 중요하면 [런타임 경계](../reference/runtime-boundaries.md),
[Agent Connection](../reference/agent-connection.md),
[MCP 전송](../reference/mcp-transport.md), [보안](../reference/security.md)을
사용합니다.

구현 소스는 setup helper와 연결 프로비저닝을 분리해 이름 붙입니다. 공개 명령
소유권은 관리 CLI에 남습니다. 이 문서에서 setup workflow는 별도의 공개 명령군이
아니라 설치 프로필 준비와 로컬 CLI 오케스트레이션을 뜻합니다.

## 작업 흐름 담당 지도

| 작업 흐름 | 이 문서의 아키텍처 수준 담당 | 정확한 담당 경로 |
|---|---|---|
| Setup workflow | Runtime Home 해석, 설치 프로필 준비, 명령 발견, 선택적 interactive 선택, link 설치, shell startup 파일 갱신, report 렌더링 경계. | [관리 CLI](../reference/admin-cli.md#runtime-home-selection)와 [런타임 경계](../reference/runtime-boundaries.md). |
| Connection init/add | 프로젝트 등록, Agent Connection 등록, host plan 구성, guard integration 계획 또는 적용, 검증, 렌더링 경계. | [관리 CLI](../reference/admin-cli.md#volicord-agent-install), [Agent Connection](../reference/agent-connection.md), [MCP 전송](../reference/mcp-transport.md). |
| Connection status/verify | 저장된 연결 사실, 현재 host 진단, CLI MCP preflight, 선택적 stdio handshake, guard audit 사실, 렌더링 경계. | [관리 CLI](../reference/admin-cli.md#agent-connection-result-states), [Agent Connection](../reference/agent-connection.md), [MCP 전송](../reference/mcp-transport.md). |
| Guard hook lifecycle | `session-start`, `pre-tool`, `post-tool`, `prompt-capture`, `stop` phase를 가로지르는 숨겨진 내부 hook 명령 오케스트레이션. | [관리 CLI](../reference/admin-cli.md#guard-hook-commands), [Agent Connection](../reference/agent-connection.md), [보안](../reference/security.md). |
| Doctor diagnostics | setup, profile, connection, host, guard, privacy-footprint 사실을 읽기 전용으로 검사한 뒤 진단으로 렌더링하는 경계. | [관리 CLI](../reference/admin-cli.md#runtime-home-selection), [런타임 경계](../reference/runtime-boundaries.md), [보안](../reference/security.md). |
| Host integration | CLI가 오케스트레이션하는 호스트 어댑터의 plan, apply, verify, remove 책임. | [관리 CLI](../reference/admin-cli.md#external-host-configuration)와 [Agent Connection](../reference/agent-connection.md). |
| Guard integration | init, status, verification, doctor가 사용하는 생성 파일 계획, 적용, capability metadata, 사실 기반 audit helper. | [관리 CLI](../reference/admin-cli.md#guard-hook-commands)와 [보안](../reference/security.md). |

## Setup workflow

Setup workflow는 뒤의 연결과 MCP 시작 흐름이 의존하는 로컬 CLI 실행 사실을
준비합니다.

1. 파싱된 CLI 입력, 환경, 플랫폼 기본값에서 선택된 Runtime Home을 해석한 뒤 Runtime
   Home registry를 초기화하거나 재사용합니다.
2. 설치 프로필이 기록할 실행 중인 `volicord` 명령과 MCP 시작 명령을 발견합니다.
   발견 실패는 profile을 부분적으로 쓰지 않고 setup check와 이름 붙은 required action을
   만듭니다.
3. 사람용 text 모드에서는 명령 경로가 준비되지 않았을 때 interactive command-availability
   질문을 할 수 있습니다. JSON 모드는 비대화식으로 유지됩니다.
4. command-link 디렉터리가 선택되면 workflow는 그 디렉터리를 준비하고, 관리 command
   link를 설치하며, 디렉터리가 `PATH`에 있는지 점검하고, 선택한 interactive 선택이
   요청한 경우 관리 shell startup block을 쓸 수 있습니다. Shell startup 파일 갱신 뒤에도
   실행 중인 부모 shell은 변하지 않으므로 새 shell이나 host restart 필요를 보고합니다.
5. Workflow는 명령 경로, 선택한 binary directory, 기본 연결 모드, setup metadata,
   timestamp를 담은 설치 프로필을 씁니다.
6. 렌더링은 check, performed action, optional action, required action, profile fact를
   text 또는 JSON 출력으로 바꿉니다. `action_required`는 이름 붙은 로컬 후속 행동을
   뜻하며 공개 API 실패나 보안 finding이 아닙니다.

Setup workflow는 사용자 소유 판단을 기록하거나, 쓰기 티켓을 발급하거나, 호스트
trust를 증명하거나, 공개 명령 문법을 정의하지 않습니다.

## Connection init과 add

연결 프로비저닝은 로컬 관리 오케스트레이션입니다. 공개 Core 메서드 실행과는
분리됩니다.

계획 단계는 선택된 host, connection intent, profile, mode, repository root를 파싱합니다.
Runtime Home과 installation profile fact를 해석하거나 준비하고, Agent Connection 식별자를
파생하거나 재사용하며, host configuration plan을 만들고, host plan conflict를 거부합니다.
Init에서는 선택된 profile에 맞는 guard integration plan도 만듭니다.

Dry-run 프로비저닝은 계획과 렌더링 뒤에 멈춥니다. Runtime Home 상태 생성, 프로젝트나
연결 등록, 호스트 설정 적용, guard integration 파일 적용, MCP preflight 실행, tool
discovery 없이 무엇을 쓰거나 점검할지를 보고합니다.

Dry-run이 아닌 프로비저닝은 Runtime Home 상태를 초기화하거나 재사용하고, 선택된 Product
Repository 프로젝트를 등록하거나 재사용하며, Agent Connection 기록을 만들거나 갱신하고,
선택된 프로젝트 membership 경계를 적용하며, Connection Projects membership을 추가하거나
확인합니다. 그다음 CLI는 선택된 호스트 어댑터를 통해 host plan을 적용합니다. Init은
이 적용이 guard 대상과 부모 디렉터리를 만들거나 바꿀 수 있으므로 결과 파일시스템 상태를
기준으로 guard 통합 계획을 다시 만든 뒤 적용합니다. 그다음 프로젝트와 Agent
Connection 사실이 존재하는 상태에서 guard 설치 메타데이터를 기록합니다.

검증은 host와 guard 적용 뒤 실행됩니다. 호스트 어댑터에서 관찰 가능한 host fact를
요청하고, 해석된 Runtime Home과 Agent Connection 바인딩으로 CLI MCP preflight를
실행하며, host gate와 preflight가 허용할 때만 직접 stdio 초기화와 `tools/list` discovery를
수행합니다. CLI는 그 결과의 last-known verification status를 저장하고 사용자 통제 next
action과 함께 connection result를 렌더링합니다.

프로비저닝은 Runtime Home registry 상태, Product Repository 파일, 외부 host 설정, guard
파일, MCP process check를 가로지르는 단일 transaction이 아닙니다. 앞선 durable effect가
적용된 뒤 뒤쪽 경계가 실패를 보고하면, 이후 status, verify, project, remove workflow가
그 앞선 effect를 관찰할 수 있습니다.

## Connection status와 verify

Connection status는 읽기 중심입니다. Agent Connection 하나를 선택하고, 연결된 project
membership과 저장된 verification fact를 읽으며, 가능할 때 managed host plan을 재구성하고,
어댑터가 보고할 수 있는 current host diagnostic을 붙이고, guard state를 모아 저장되거나
파생된 status를 렌더링합니다. Host를 실행하지 않고, host configuration을 다시 쓰지 않으며,
MCP preflight를 새로 고치지 않습니다.

Connection verify는 능동 진단 workflow입니다. Agent Connection 하나를 선택하고, host plan을
재구성하며, host verification을 실행하고, CLI MCP preflight를 실행하고, 선택적으로 직접
stdio handshake와 tool discovery를 수행하며, connection의 last-known verification report를
갱신합니다. Verification output은 stored connection fact, current host diagnostic, MCP
command와 preflight fact, managed host lifecycle observation, guard audit fact를 함께 담을
수 있습니다.

이 workflow들은 관찰 가능한 사실과 next action을 보고합니다. 관련 참조 담당 문서가 그
정확한 의미를 정의하지 않는 한 외부 host가 configuration을 load, trust, approve,
initialize, expose했다는 사실을 증명하지 않습니다. 또한 OS enforcement, 사용자 승인, 행위자
identity, 제품 정확성, 테스트 충분성, 닫기 상태를 증명하지 않습니다.

## Guard hook lifecycle

생성된 host wrapper 파일은 지원되는 lifecycle phase에서 숨겨진 내부 hook namespace를
호출합니다. CLI hook workflow는 Runtime Home과 등록 프로젝트를 해석하고, host event를 guard
envelope로 정규화하며, 필요할 때 session을 보장하거나 기록하고, event가 기록된 capability와
policy fact에 맞으면 guard installation activation을 관찰한 뒤 phase handler로 dispatch합니다.

Phase handler에는 서로 다른 아키텍처 책임이 있습니다.

- `session-start`는 Agent Session을 기록하거나 재사용하고 host session injection을 위한
  context를 렌더링합니다.
- `pre-tool`은 tool attempt를 분류하고, 적용되는 경우 현재 task와 write-ticket compatibility를
  점검하며, expected-write correlation fact를 저장할 수 있습니다.
- `post-tool`은 관찰된 tool result를 기록하고 expected write 또는 현재 write-ticket fact와
  상관시키며, 해결되지 않은 관찰된 Product Repository 변경을 기록할 수 있습니다.
- `prompt-capture`는 prompt capture가 사용 가능할 때 User Channel 판단 답변을 위한 prompt
  metadata와 엄격한 chat command 처리를 담당합니다.
- `stop`은 close 관련 fact를 점검하고 session completion을 위한 host-native allow 또는 deny
  result를 렌더링합니다.

Phase 처리 뒤 CLI는 cooperative disclosure를 붙이고, 아직 기록되지 않은 guard event를
저장하며, phase가 만든 expected-write fact를 저장하고, Volicord JSON, text, host-native 출력
중 하나로 렌더링합니다.

Guard hook decision은 협력형 host decision과 observation입니다. 공개 Core 메서드, 그
자체로서의 사용자 소유 판단, 쓰기 티켓, host trust, shell approval, OS sandboxing, 전체
쓰기 방지, 행위자 귀속 증명, 정확성 증명, 테스트 충분성 증명, 인간 검토 대체가
아닙니다.

## Doctor diagnostics

Doctor는 읽기 중심 진단 workflow입니다. Runtime Home을 해석하고, Runtime Home 접근과
registry 형태를 검사하며, installation profile fact를 읽고, 저장된 command path와 `PATH`
availability를 점검하고, registry count를 보고하며, guard installation record를 검사하고,
생성 guard file과 capability metadata를 audit하고, 사용할 수 있는 session-watch observation
summary를 읽으며, privacy-footprint view를 렌더링할 수 있습니다.

Doctor는 사실 기반 inspection result를 diagnostic check와 suggested action으로 매핑합니다.
Project를 만들거나, host configuration을 설치하거나 제거하거나, Agent Connection mode를
바꾸거나, active host verification을 실행하거나, User Channel judgment에 답하거나, guard file을
복구하거나, 보안, 정확성, review, QA, 최종 수락, 잔여 위험 수락, 닫기 상태를 증명하지
않습니다.

## Host integration 경계

호스트 어댑터는 host-specific planning, application, verification, removal, capability
declaration, conflict detection을 담당합니다. CLI workflow는 host, intent, mode, profile,
Runtime Home, project context, Agent Connection fact를 선택한 뒤 plan, apply, verify, remove
경계에서 adapter를 호출합니다.

CLI는 host configuration을 외부 integration surface로 다룹니다. Host configuration write가
성공했다는 사실은 host trust, host approval, host reload, active tool exposure, model behavior와
구분됩니다. Generic external MCP host configuration은 사용자 관리로 남습니다. CLI는 지원되는
Agent Connection이 존재한 뒤 guidance를 보고할 수 있지만 임의 external host configuration을
쓰지 않습니다.

## Guard integration 경계

Guard integration은 detective-aware workflow를 위한 generated file, policy JSON, host hook
command, capability metadata, prompt-capture availability, factual audit input을 계획합니다.
Application은 계획된 managed file 또는 managed block만 씁니다. 관리 파일 적용은 Product
Repository 부모 경로를 고정하고, 커밋 전에 계획된 대상 스냅샷을 비교하며, 같은
디렉터리의 보조 항목에 스테이징합니다. 운영체제 고유 이름 공간 연산이 필요하면 플랫폼
파일시스템 파사드를 사용하고, 연산 뒤 관련 항목을 검증합니다. 정리, 복구 검사, 진단
구성은 CLI 호출자가 담당하며 플랫폼 파사드가 결정하지 않습니다. Audit은 기록된
메타데이터와 생성 파일을 읽어 status, verification, doctor를 위한 missing, stale,
broken, unsafe, unobserved 사실을 분류합니다.

Guard integration fact는 진단과 workflow routing을 뒷받침할 수 있습니다. 보안 보장, host
approval, 사용자 승인, 정확성 증명, 완전한 filesystem monitoring, 모델이 Product Repository
guidance를 따랐다는 사실을 뜻하지 않습니다.
