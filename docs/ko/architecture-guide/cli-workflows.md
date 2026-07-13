# CLI 작업 흐름

이 가이드는 로컬 `volicord` 관리 작업 흐름의 아키텍처 수준 실행 경계를
담당합니다. CLI 오케스트레이션이 Runtime Home 설정, 설치 프로필 준비, Agent
Connection 기록, 호스트 어댑터, guard 통합, 검증, 진단, 렌더링을 어떻게
조합하는지 설명합니다. 이 조합에는 최종 출력 권한 고지도 포함됩니다.

이 문서는 명령 문법, 플래그, 표준 출력 또는 표준 오류 계약, 종료 코드, JSON 출력
스키마, 공개 API 동작, 저장 효과, 보안 보장, Core 권한 의미, 제품 계약을
정의하지 않습니다. 정확한 소스 경로와 모듈 책임은 [소스 지도](source-map.md)를
봅니다. 정확한 명령 문법, 플래그, 결과 상태, 출력 경계, 숨겨진 훅 명령
계약은 [관리 CLI](../reference/admin-cli.md)를 봅니다. 정확한 런타임, 연결,
전송, 비보장 표현이 중요하면 [런타임 경계](../reference/runtime-boundaries.md),
[Agent Connection](../reference/agent-connection.md),
[MCP 전송](../reference/mcp-transport.md), [보안](../reference/security.md)을
봅니다.

구현 소스는 설정 도우미와 연결 프로비저닝을 서로 다른 이름으로 구분합니다. 공개
명령은 관리 CLI 문서가 담당합니다. 이 문서의 설정 작업 흐름은 설치 프로필 준비와
로컬 CLI 오케스트레이션을 뜻하며, 별도의 공개 명령군이 아닙니다.

## 작업 흐름 담당 지도

| 작업 흐름 | 이 문서의 아키텍처 수준 담당 | 정확한 담당 경로 |
|---|---|---|
| 설정 작업 흐름 | Runtime Home 해석, 설치 프로필 준비, 명령 탐색, 선택적 대화형 선택, 링크 설치, 셸 시작 파일 갱신, 보고서 렌더링 경계. | [관리 CLI](../reference/admin-cli.md#runtime-home-selection)와 [런타임 경계](../reference/runtime-boundaries.md). |
| 연결 초기화/추가 | 프로젝트와 Agent Connection 등록, 호스트 계획 구성, guard 통합 계획 또는 적용, 검증, 렌더링 경계. | [관리 CLI](../reference/admin-cli.md#volicord-agent-install), [Agent Connection](../reference/agent-connection.md), [MCP 전송](../reference/mcp-transport.md). |
| 연결 상태/검증 | 저장된 연결 정보, 현재 호스트 진단, CLI MCP 사전 점검, 선택적 표준 입출력 핸드셰이크, guard 감사 정보, 최종 출력 고지 역량 진단, 렌더링 경계. | [관리 CLI](../reference/admin-cli.md#agent-connection-result-states), [Agent Connection](../reference/agent-connection.md), [MCP 전송](../reference/mcp-transport.md). |
| Guard 훅 생명주기 | `session-start`, `pre-tool`, `post-tool`, `prompt-capture`, `stop` 단계를 아우르는 숨겨진 내부 훅 명령 오케스트레이션. | [관리 CLI](../reference/admin-cli.md#guard-hook-commands), [Agent Connection](../reference/agent-connection.md), [보안](../reference/security.md). |
| 최종 출력 권한 고지 | 최신 읽기 전용 status 새로 고침, 공유 형식 receipt 검증, 프로필과 무관한 고지 계획, 호스트 고유 고정 UI 렌더링, Stop 집행과 분리된 크기 제한 fallback 경계. | [상태 보기와 템플릿](../reference/projection-and-templates.md), [관리 CLI](../reference/admin-cli.md), [Agent Connection](../reference/agent-connection.md), [보안](../reference/security.md). |
| Doctor 진단 | 설정, 프로필, 연결, 호스트, guard, 개인정보 흔적 정보를 읽기 전용으로 검사한 뒤 진단 결과를 렌더링하는 경계. | [관리 CLI](../reference/admin-cli.md#runtime-home-selection), [런타임 경계](../reference/runtime-boundaries.md), [보안](../reference/security.md). |
| 호스트 통합 | CLI가 조율하는 호스트 어댑터의 계획, 적용, 검증, 제거 책임. | [관리 CLI](../reference/admin-cli.md#external-host-configuration)와 [Agent Connection](../reference/agent-connection.md). |
| Guard 통합 | 프로필과 무관한 최종 출력 처리기 부분 집합과 더 넓은 Detective 생명주기를 위한 생성 파일 계획 및 적용, 초기화·상태 조회·검증·doctor가 사용하는 역량 메타데이터와 사실 기반 감사 도우미. | [관리 CLI](../reference/admin-cli.md#guard-hook-commands)와 [보안](../reference/security.md). |

## 설정 작업 흐름

설정 작업 흐름은 뒤의 연결과 MCP 시작 흐름이 의존하는 로컬 CLI 실행 정보를
준비합니다.

1. 파싱된 CLI 입력, 환경, 플랫폼 기본값에서 선택된 Runtime Home을 해석한 뒤 Runtime
   Home 레지스트리를 초기화하거나 재사용합니다.
2. 설치 프로필이 기록할 실행 중인 `volicord` 명령과 MCP 시작 명령을 발견합니다.
   발견에 실패하면 프로필을 일부만 쓰지 않습니다. 대신 설정 점검 결과와 필요한
   후속 조치를 이름 붙여 보고합니다.
3. 사람용 텍스트 모드에서는 명령 경로가 준비되지 않았을 때 명령 사용 가능 여부를
   대화형으로 물을 수 있습니다. JSON 모드는 비대화식으로 동작합니다.
4. 명령 링크 디렉터리를 선택하면 그 디렉터리를 준비하고 관리 명령 링크를
   설치합니다. 디렉터리가 `PATH`에 있는지 확인하고, 사용자가 선택한 경우 셸 시작
   파일에 관리 블록을 쓸 수 있습니다. 실행 중인 부모 셸은 바뀌지 않으므로 새 셸이나
   호스트 재시작이 필요하다고 보고합니다.
5. 명령 경로, 선택한 바이너리 디렉터리, 기본 연결 모드, 설정 메타데이터, 타임스탬프를
   설치 프로필에 기록합니다.
6. 점검 결과, 수행한 작업, 선택 가능한 작업, 필요한 작업, 프로필 정보를 텍스트 또는
   JSON으로 렌더링합니다. `action_required`는 로컬에서 해야 할 후속 조치를 뜻합니다.
   공개 API 실패나 보안 문제를 뜻하지 않습니다.

설정 작업 흐름은 사용자 소유 판단을 기록하거나, 쓰기 티켓을 발급하거나, 호스트
신뢰를 증명하거나, 공개 명령 문법을 정의하지 않습니다.

## 연결 초기화와 추가

연결 프로비저닝은 로컬 관리 오케스트레이션입니다. 공개 Core 메서드 실행과는
분리됩니다.

계획 단계는 선택한 호스트, 연결 의도, 프로필, 모드, 저장소 루트를 파싱합니다.
Runtime Home과 설치 프로필 정보를 해석하거나 준비합니다. Agent Connection 식별자는
파생하거나 재사용하고, 호스트 설정 계획을 만듭니다. 호스트 계획이 충돌하면
거부합니다. 초기화에서는 선택한 프로필에 맞는 guard 통합 계획도 만듭니다.

Dry-run 프로비저닝은 계획과 렌더링 뒤에 멈춥니다. Runtime Home 상태 생성, 프로젝트나
연결 등록, 호스트 설정 적용, guard 통합 파일 적용, MCP 사전 점검, 도구 탐색을 하지
않습니다. 대신 무엇을 쓰거나 점검할지를 보고합니다.

Dry-run이 아닌 프로비저닝은 Runtime Home 상태를 초기화하거나 재사용하고, 선택된 Product
Repository 프로젝트를 등록하거나 재사용하며, Agent Connection 기록을 만들거나 갱신하고,
선택된 프로젝트 구성원 경계를 적용하며, Connection Projects 구성원 관계를 추가하거나
확인합니다. Init이 다른 호스트나 의도로 전환할 때는 이미 사용할 수 있었던 이전
connection을 계속 사용할 수 있게 두고, 명시적으로 비활성화된 이전 connection은 비활성
상태로 유지하며, 요청한 project membership을 비활성 상태로 유지합니다. 새 요청 Agent
Connection은 비활성 상태로 staging하지만 이미 활성화된 connection은 다른 project를 계속
처리할 수 있습니다. 그다음 CLI는 선택한 호스트 어댑터로 호스트 계획을 적용합니다. 이
과정에서 guard 대상이나 상위 디렉터리가 생기거나 바뀔 수 있습니다. 따라서 초기화는 결과
파일시스템 상태를 기준으로 guard 통합 계획을 다시 만들고 적용합니다. Guard 설치
메타데이터를 만든 뒤 Store의 immediate transaction 도우미로 그 메타데이터 기록, 요청
project membership 추가, 다른 project가 남은 connection의 대체 membership 폐기, 요청
connection 활성화를 함께 처리합니다. 마지막 project인 대체 connection은 도우미가
비활성화하되 membership을 내구성
있는 pending cleanup inventory로 유지합니다. 두 번째 immediate transaction이 이 비활성
inventory를 다시 검증한 뒤 host cleanup 동안 Registry 잠금을 해제합니다. 마지막 immediate
transaction이 Store 소유 marker를 다시 검증하고 보존한 membership을 제거합니다. 일반
일반 등록, 활성화·비활성화, membership 변경, staging 대상 활성화는 marker invariant를
바꾸는 작업을 거절합니다. Mode와 검증 보고서 갱신은 marker를 바꾸지 않습니다. 새 마이그레이션은
이전의 유효한 cleanup marker를 새 replacement에 다시 연결하고 관련 없는 비활성 대안은
보존합니다.
Staging upsert는 기존 활성 bit를 보존하고 transaction 분류는 비활성 staging과 정확한 cleanup
재개를 구분합니다. 오래된 계획 snapshot으로 활성 요청 membership을 제거하지 않습니다.

검증은 호스트와 guard 적용 뒤 실행됩니다. 호스트 어댑터에 관찰 가능한 호스트 정보를
요청하고, 해석된 Runtime Home과 Agent Connection 연결 정보로 CLI MCP 사전 점검을
실행합니다. 호스트 조건과 사전 점검이 허용할 때만 표준 입출력 초기화와 `tools/list`
탐색을 직접 수행합니다. CLI는 마지막 검증 상태를 저장하고, 사용자가 수행할 다음 작업과
함께 연결 결과를 렌더링합니다.

프로비저닝은 Runtime Home 레지스트리 상태, Product Repository 파일, 외부 호스트 설정,
guard 파일, MCP 프로세스 점검을 하나의 트랜잭션으로 처리하지 않습니다. 앞 단계에서
상태가 이미 저장된 뒤 나중 단계가 실패하면 init은 명시적인 부분 적용 결과를 렌더링합니다.
이후 상태 조회, 검증, 프로젝트, doctor, 제거 작업 흐름은 앞 단계의 결과를 확인할 수
있습니다. 좁은 Registry 활성 전환은 transaction으로 처리하지만 더 넓은 작업 흐름은 여러
표면에서 수렴하는 동작으로 남습니다.

## 연결 상태와 검증

연결 상태 조회는 읽기 중심입니다. Agent Connection 하나를 선택하고, 연결된 프로젝트
구성원 관계와 저장된 검증 정보를 읽습니다. 가능하면 관리 호스트 계획을 재구성하고,
어댑터가 보고할 수 있는 현재 호스트 진단과 guard 상태를 모아 최종 출력 고지 역량 및
설정 사실을 포함한 저장되거나 파생된 상태를 렌더링합니다. 호스트를 실행하거나 호스트
설정을 다시 쓰지 않으며, MCP 사전 점검도 새로 실행하지 않습니다.

연결 검증은 능동 진단 작업 흐름입니다. Agent Connection 하나를 선택하고 호스트 계획을
재구성합니다. 호스트 검증과 CLI MCP 사전 점검을 실행하고, 선택적으로 표준 입출력
핸드셰이크와 도구 탐색을 직접 수행합니다. 마지막 검증 보고서도 갱신합니다. 검증 출력은
저장된 연결 정보, 현재 호스트 진단, MCP 명령과 사전 점검 정보, 관리 호스트 생명주기
관찰, 최종 출력 고지 역량 진단, guard 감사 정보를 함께 담을 수 있습니다.

두 작업 흐름은 관찰 가능한 정보와 다음 작업을 보고합니다. 관련 참조 담당 문서가 정확한
의미를 정의하지 않는 한, 외부 호스트가 설정을 불러오거나 신뢰·승인·초기화·노출했다는
사실을 증명하지 않습니다. OS 강제, 사용자 승인, 행위자 신원, 제품 정확성, 테스트 충분성,
닫기 상태도 증명하지 않습니다.

## Guard 훅 생명주기

생성된 호스트 래퍼 파일은 지원되는 생명주기 단계에서 숨겨진 내부 훅 명령을 호출합니다.
CLI 훅 작업 흐름은 Runtime Home과 등록 프로젝트를 해석하고, 호스트 이벤트를 guard 요청
형태로 정규화합니다. 필요하면 세션을 확인하거나 기록합니다. 이벤트가 기록된 역량 및 정책
정보와 일치하면 guard 설치가 활성화되었음을 관찰하고 단계별 처리기로 전달합니다.

각 단계 처리기는 다음 책임을 맡습니다.

- `session-start`는 Agent Session을 기록하거나 재사용하고, 호스트 세션에 주입할 맥락을
  렌더링합니다.
- `pre-tool`은 도구 실행 시도를 분류합니다. 필요한 경우 현재 `Task`와 쓰기 티켓의 호환성을
  확인하고, `expected write` 상관관계 정보를 저장할 수 있습니다.
- `post-tool`은 관찰된 도구 결과를 기록하고 `expected write` 또는 현재 쓰기 티켓 정보와
  연결합니다. 아직 해결되지 않은 Product Repository 변경도 기록할 수 있습니다.
- `prompt-capture`는 프롬프트 캡처를 사용할 수 있을 때 User Channel 사용자 행동 resolution에 필요한
  프롬프트 메타데이터와 엄격한 채팅 명령 처리를 담당합니다.
- `stop`은 공유 형식 status/receipt 검증 경계를 통해 닫기 관련 정보를 확인하고 세션
  완료에 대한 호스트 고유 허용 또는 거부 결과를 렌더링합니다. Stop 집행은 일반 최종
  출력 고지 표면을 담당하지 않습니다.

이벤트 timestamp는 guard 기록과 상관관계를 위한 관찰 metadata로만 남습니다. 현재
Task, 쓰기 티켓, 대기 UserAction, 프롬프트 명령 적격성 조회는 호스트 보고 시각이 아니라
프로젝트/Core 현재 시계를 사용하므로 지연되거나 시계 차이가 있는 이벤트가 현재 권한을
바꾸지 못합니다.

단계 처리 뒤 CLI는 협력형 비보장 안내를 붙이고, 아직 기록하지 않은 guard 이벤트와 해당
단계가 만든 `expected write` 정보를 저장합니다. 결과는 Volicord JSON, 텍스트, 호스트 고유
형식 중 하나로 렌더링합니다.

Guard 훅 결정은 협력형 호스트 결정과 관찰입니다. 공개 Core 메서드, 사용자 소유 판단,
쓰기 티켓, 호스트 신뢰, 셸 승인, OS 샌드박싱, 완전한 쓰기 방지, 행위자 귀속 증명,
정확성 증명, 테스트 충분성 증명, 사람의 검토를 대신하지 않습니다.

## 최종 출력 권한 고지

최종 출력 고지는 Detective Stop 집행 결정이 아니라 프로필과 무관한 호스트 어댑터 작업
흐름입니다. 지원되는 호스트가 최종 출력 이벤트를 보고하면 CLI는 선택된 프로젝트와
Task의 최신 읽기 전용 Core status를 요청합니다. 공유 Core 소유 형식 검증기가 담당
문서가 정의한 관계에 따라 status와 후보 `AuthorityReceipt`를 비교합니다. 이어 CLI가
메모리 내 고지 계획 하나를 만들고 선택된 호스트 어댑터가 고정된 호스트 고유 UI 표면에
표시하게 합니다.

선택된 Task가 있으면 계획은 전체 정규 receipt 또는 크기가 제한된 Task별
`volicord status` fallback을 보존하며 receipt JSON을 자르지 않습니다. Task가 없거나
새로 고침이 실패했거나 status가 잘못되었거나 일치하지 않으면 적용되는 참조 문서가
담당하는 명시적 fallback 또는 진단 경로가 됩니다. Replay 뒤의 이벤트를 포함해 모든
최종 출력 이벤트가 읽기 전용 새로 고침을 다시 수행합니다. 이전 변경 응답, Stop 결과,
모델이 작성한 답변을 현재 권한으로 캐시하지 않습니다.

Record profile과 Detective profile은 이 고지 작업 흐름을 공유합니다. Detective Stop은
별도 allow 또는 deny 결정을 위해 같은 검증 사실을 사용할 수 있지만 Record 고지는
차단하지 않습니다. 범용, 사용자 관리, 미지원, 비활성, 저하된 호스트 경로는 고정 호스트
UI 표면이 있다고 주장하지 않고 지원하는 fallback과 진단 사실만 보고합니다.

렌더러와 생성 설정 픽스처는 어댑터 바이트와 처리 경로를 검증합니다. 실제 호스트가
표면을 불러오고 표시했다는 사실은 확립하지 않습니다. 실제 Codex와 Claude Code 관찰은
선택적 호스트 통합 검증에 속합니다. 구현 근거는
[최종 출력 권한 고지](decisions/final-output-authority-disclosure.md)에 기록하며 정확한
동작은 집중 참조 담당 문서에 남습니다.

## Doctor 진단

Doctor는 읽기 중심 진단 작업 흐름입니다. Runtime Home과 그 접근 가능성, 레지스트리 형태를
검사합니다. 설치 프로필 정보와 저장된 명령 경로를 읽고 `PATH` 사용 가능 여부를 확인합니다.
레지스트리 개수를 보고하고 guard 설치 기록, 생성 파일, 역량 메타데이터를 감사합니다.
사용할 수 있는 세션 감시 관찰 요약을 읽고 개인정보 흔적 보기를 렌더링할 수도 있습니다.

Doctor는 검사 결과를 진단 점검과 권장 작업으로 연결합니다. 프로젝트를 만들거나, 호스트
설정을 설치 또는 제거하거나, Agent Connection 모드를 바꾸지 않습니다. 능동 호스트 검증을
실행하거나, User Channel 판단에 답하거나, guard 파일을 복구하지도 않습니다. 보안, 정확성,
검토, QA, 최종 수락, 잔여 위험 수락, 닫기 상태를 증명하지 않습니다.

## 호스트 통합 경계

호스트 어댑터는 호스트별 계획, 적용, 검증, 제거, 역량 선언, 충돌 탐지를 담당합니다. CLI
작업 흐름은 호스트, 연결 의도, 모드, 프로필, Runtime Home, 프로젝트 맥락, Agent Connection
정보를 선택한 뒤 계획·적용·검증·제거 경계에서 어댑터를 호출합니다.

프로필과 무관한 최종 출력 역량 계약과 검증도 이 경계에 있습니다. Guard 통합이 호스트별
생성 처리기 계획을 적용하지만 전체 Detective 생명주기는 최종 출력 전용 부분 집합과 계속
구분됩니다.

CLI는 호스트 설정을 외부 통합 표면으로 다룹니다. 호스트 설정 쓰기가 성공했다는 사실은
호스트의 신뢰, 승인, 다시 불러오기, 실제 도구 노출, 모델 동작과 다릅니다. 일반 외부 MCP
호스트 설정은 사용자가 관리합니다. CLI는 지원되는 Agent Connection이 생긴 뒤 안내를
보고할 수 있지만, 임의의 외부 호스트 설정을 쓰지 않습니다.

## Guard 통합 경계

Guard 통합은 생성 파일, 정책 JSON, 호스트 이벤트 명령, 역량 메타데이터, 사실 기반 감사
입력을 계획합니다. 지원되는 관리 Record profile과 Detective profile 경로는 최종 출력
처리기 부분 집합을 공유하며, Detective만 나머지 생명주기 처리기와 프롬프트 캡처
관찰을 추가합니다. 적용 단계는 계획된 관리 파일이나 관리 블록만 씁니다. 관리 파일을
적용할 때는 Product Repository의 상위 경로를 고정하고, 커밋 전에 계획된 대상
스냅샷을 비교하며, 같은 디렉터리의 보조 항목에 스테이징합니다. 운영체제 고유 이름
공간 연산이 필요하면 플랫폼 파일시스템 파사드를 사용하고, 연산 뒤 관련 항목을
검증합니다. 정리, 복구 검사, 진단
구성은 CLI 호출자가 담당하며 플랫폼 파사드가 결정하지 않습니다. 감사 단계는 기록된
메타데이터와 생성 파일을 읽고, 상태 조회·검증·doctor에서 쓸 누락, 오래됨, 손상, 안전하지
않음, 관찰되지 않음 상태를 분류합니다.

Guard 통합 정보는 진단과 작업 흐름의 처리 경로를 뒷받침할 수 있습니다. 보안 보장,
호스트 승인, 사용자 승인, 정확성 증명, 완전한 파일시스템 감시, 모델이 Product Repository
안내를 따랐다는 사실을 뜻하지 않습니다.
