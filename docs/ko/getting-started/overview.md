# 시작 개요

이 문서는 Volicord의 첫 읽기 개요입니다. 제품의 핵심 생각을 일반 언어로 설명하고 정확한 계약 질문은 Reference 담당 문서로 보냅니다.

<a id="what-volicord-is"></a>
## Volicord란

Volicord는 AI 보조 제품 작업을 위한 로컬 work-authority 제품/시스템입니다. 사용자, AI 호스트, 에이전트가 함께 일할 때 사용자의 권한 근거를 작업 흐름 안에서 보이게 유지하는 로컬 권한 제어 평면입니다.

Core는 Volicord 상태의 로컬 권한 기록입니다. Volicord는 그 기록을 둘러싼 더 넓은 제품/시스템이며, 로컬 런타임 구성요소, Agent Connection, 지원되는 호스트 설정, 문서 경로를 포함합니다.

Volicord는 OS 보안 제품이 아닙니다. OS 샌드박싱, 파일시스템 ACL, 네트워크 정책, 비밀 격리를 제공하지 않습니다.

## 일반적인 문제

사용자는 에이전트에게 제품 동작 변경, 실패 조사, 릴리스 노트 준비를 요청할 수 있습니다. 에이전트는 파일을 살피고, 계획을 제안하고, 코드를 쓰고, 테스트를 실행하고, 결과를 요약할 수 있습니다. 그 속도는 유용하지만 다음 대체를 숨길 수 있습니다.

- 작은 요청이 더 넓은 제품 변경이 됩니다.
- 제품 결정이 구현 안에 묻힙니다.
- 한 주장에 대한 증거가 모든 것에 대한 증거처럼 들립니다.
- 통과한 테스트가 최종 수락처럼 취급됩니다.
- 사용자의 가벼운 승인이 모든 미해결 판단을 해결한 것처럼 취급됩니다.

Volicord는 이런 대체를 보이게 하려고 존재합니다. 범위, 사용자 소유 판단, 증거, 검증 기준, 수락, 잔여 위험, 닫기 준비 상태를 구분해 둘 로컬 장소를 에이전트와 사용자에게 제공합니다.

## 로컬 구성요소

아래 이름들은 서로 관련되지만 서로 바꿔 쓸 수 없습니다.

| 이름 | 첫 읽기 의미 | 정확한 담당 문서 |
|---|---|---|
| Volicord | AI 보조 제품 작업을 위한 로컬 work-authority 제품/시스템과 권한 제어 평면. | [Volicord란](#what-volicord-is) |
| Core | Volicord 상태의 로컬 권한 기록. | [Core 모델](../reference/core-model.md) |
| Volicord 구현 | Core, 저장소, 타입, `volicord` 실행 파일, MCP 어댑터 코드, 테스트, 문서, 검증 도구를 포함하는 이 저장소의 구현 집합. | [런타임 경계](../reference/runtime-boundaries.md) |
| `volicord` | 로컬 관리 CLI 명령, 로컬 User Channel, 생성된 MCP 호스트 설정이 사용하는 `mcp` 하위 명령을 제공하는 설치 실행 파일. | [관리 CLI](../reference/admin-cli.md) |
| `volicord mcp --stdio` | 생성된 호스트 설정이 선택된 Agent Connection을 위해 자식 프로세스로 시작하는 stdio MCP 프로세스 모드. | [MCP 전송](../reference/mcp-transport.md) |
| `Volicord Runtime Home` | 저장소/런타임 담당 문서가 정의하는 Volicord 운영 데이터의 로컬 런타임 데이터 공간. | [런타임 경계](../reference/runtime-boundaries.md) |
| `Product Repository` | 사용자의 프로젝트 작업공간과 제품 파일. 명시적으로 선택된 프로젝트 범위 호스트 설정을 포함할 수 있지만 Core 권한도 런타임 홈도 아닙니다. | [런타임 경계](../reference/runtime-boundaries.md) |
| Agent Connection | 로컬 MCP 호스트 connection 단위. 하나의 호스트 설정 대상, 관리되는 연결 식별 정보, 모드, 명시적으로 연결된 Project를 묶습니다. | [Agent Connection Reference](../reference/agent-connection.md) |
| User Channel | 권한을 지니는 사용자 판단을 위한 로컬 사용자 경로. Agent Connection은 `user_only` 판단을 기록하지 않습니다. | [관리 CLI](../reference/admin-cli.md#user-channel-commands) |

현재 기준 에이전트 호스트 모델은 connection 기반입니다. 하나의
`volicord mcp --stdio` 프로세스는 내부 연결 식별 정보로 하나의 Agent Connection에
묶이고, connection은 명시적으로 연결된 Project에만 접근할 수 있습니다. 정확한 프로젝트
선택과 MCP 도구 인자 동작은 [Agent Connection Reference](../reference/agent-connection.md)와
[MCP 전송](../reference/mcp-transport.md)이 담당합니다.

## 설정이 하는 일

일반적인 `volicord init --host HOST --repo PATH` 경로의 에이전트 설정은 다음을 할 수
있습니다.

- Runtime Home 기록 생성 또는 재사용
- 설치 프로필 생성 또는 재사용
- `Product Repository` 등록 또는 재사용
- Agent Connection과 Connection Projects 멤버십 생성 또는 재사용
- `volicord mcp --stdio`를 시작하는 프로젝트 범위 Codex 또는 Claude Code MCP 설정 설치
- guarded 로컬 사용을 위한 Volicord 관리 지침과 guard 통합 파일 설치
- guard 설치 상태 기록
- 설정 검증을 실행하고 `complete`, `action_required`, `failed` 보고

`volicord setup`은 설치 프로필 준비와 복구 경로로 남습니다. `volicord connect`는
personal, shared, global, read-only, generic export 흐름을 위한 낮은 수준의 연결
관리 명령으로 남습니다.

에이전트 설정은 다음을 하면 안 됩니다.

- Runtime Home의 모든 Project 접근 부여
- Volicord 런타임 데이터베이스나 런타임 기록을 `Product Repository`에 저장
- Codex 프로젝트 trust, Claude Code 프로젝트 MCP 승인, OAuth, reload, restart, 그 밖의 호스트 소유 동작 우회
- 모델이 Volicord 도구를 자동으로 선택한다고 약속

## 첫 읽기 권한 개념

Volicord 문서는 첫 읽기 수준에서 아래 권한 개념을 구분하고 정확한 의미를 [Core 모델](../reference/core-model.md)로 보냅니다.

- 사용자 소유 판단은 사용자 소유로 남습니다. 에이전트는 선택지를 설명할 수 있지만 판단을 만들어 내면 안 됩니다.
- User Channel은 `actor_source=local_user`, `operation_category=user_only`로 사용자 판단을 기록합니다.
- Agent Connection 호출은 Agent Connection 출처와 connection mode가 허용한 operation category를 사용합니다.
- 증거는 특정 기록된 주장을 뒷받침합니다. 최종 수락이나 잔여 위험 수락이 아닙니다.
- 검증 기준은 무엇을 확인해야 하는지 안내합니다. 그 자체가 증거나 수락은 아닙니다.
- `Write Check`은 제품 파일 쓰기 시도 하나에 대한 Core 상태 호환성입니다. 일반 쓰기 승인, 민감 동작 승인, 최종 수락, 잔여 위험 수락과 구분되며 OS 권한이 아닙니다.
- 닫기 준비 상태는 Core 권한 개념이지 제품 정확성 증명이 아닙니다.

## Connection Mode

Agent Connection은 읽기 중심 모드나 workflow 가능 모드로 둘 수 있습니다. 호스트가
상태를 조회하고, 프로젝트를 찾고, workflow 변경 도구 없이 닫기 준비 상태를 확인해야
하면 읽기 중심 모드를 사용합니다. 일반 에이전트 workflow 작업에는 workflow 모드를
사용합니다. 정확한 CLI 선택 동작은 [관리
CLI](../reference/admin-cli.md#connection-intents-and-hosts)가 담당하고, MCP에 보이는
정확한 도구 노출은 [MCP
전송](../reference/mcp-transport.md#tool-discovery-and-toolscall-response-wrapping)이
담당합니다.

## Volicord가 아닌 것

첫 읽기 제품 정체성에는 이 개요를 사용합니다. 정확한 지원 기준과 범위 밖 경계는 [범위](../reference/scope.md#product-role-exclusions)를 봅니다.

Volicord는 다듬어진 채팅 답변, 생성된 요약, 읽기 쉬운 상태 카드, 복사된 식별자, 선택적 저장소 안내, `Projection`을 권한 기록으로 바꾸지 않습니다. 정확한 표시 경계는 [Projection과 템플릿](../reference/projection-and-templates.md), 런타임과 위치 경계는 [런타임 경계](../reference/runtime-boundaries.md), 보안 문구는 [보안](../reference/security.md)이 담당합니다.

## 다음 독자 경로

| 독자 | 다음 경로 |
|---|---|
| 새 제품 독자 | [사용자 가이드](../guides/user-workflow.md) |
| 환경 확인 | [시스템 요구사항](../reference/system-requirements.md) |
| 첫 설정 | [설치](installation.md) -> [빠른 시작](quickstart.md) |
| 에이전트 호스트 운영자 | [빠른 시작](quickstart.md) -> [에이전트 호스트 설정](../guides/agent-host-setup.md) -> [에이전트 호스트 문제 해결](../guides/agent-host-troubleshooting.md) |
| 여러 저장소 운영자 | [여러 저장소 에이전트 설정](../guides/multi-repository-agent-setup.md) |
| 에이전트 작성자 | [에이전트 가이드](../guides/agent-workflow.md) -> [Agent Connection Reference](../reference/agent-connection.md) |
| 소스 코드 학습자 | [구현 가이드](../development/change-guide.md) -> [아키텍처](../development/architecture.md) |
| Reference 독자 | [Reference Index](../reference/README.md), [관리 CLI](../reference/admin-cli.md), [API 메서드](../reference/api/methods.md) |

새 독자가 Volicord를 이해하는 데 API 스키마나 담당자 메타데이터가 필요해서는 안 됩니다. 정확한 계약 담당 문서가 필요할 때 [Reference Index](../reference/README.md)를 사용합니다.
