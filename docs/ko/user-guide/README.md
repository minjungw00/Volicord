# 사용자 가이드

| 메타데이터 | 값 |
|---|---|
| 목적 | Volicord 설치, 에이전트 호스트 연결, 사용자 판단 처리, 상태와 닫기 차단 사유 해석, 설정 실패 복구를 위한 사용자 절차로 안내합니다. |
| 독자 | 새 사용자, 운영자, 에이전트, 에이전트 통합 담당자입니다. |
| 담당 범위 | 사용자 가이드 계층 탐색입니다. 정확한 CLI, MCP, API, 저장소, 보안, 권한 계약은 참조 문서에 남습니다. |

Volicord로 무언가를 해야 할 때 이 계층을 사용합니다. 이 문서들은 절차와 공개
작업 흐름 용어를 설명하지만, 집중 참조 계약을 대신하지 않습니다.

## 먼저 볼 곳

- 제품 방향 잡기: [사용자 가이드 개요](overview.md).
- 실행 파일 설치 또는 선택: [설치](installation.md).
- 지원되는 호스트 하나를 처음 연결하기: [빠른 시작](quickstart.md).
- 에이전트와 보이는 판단을 다루기: [사용자 작업 흐름](user-workflow.md).
- 판단 문구와 경계 고르기: [판단 예시](judgment-examples.md).
- 에이전트로 작업하기: [에이전트 작업 흐름](agent-workflow.md).
- 호스트 통합 설정, 검증, 제거: [에이전트 호스트 설정](agent-host-setup.md).
- 여러 명시적으로 허용된 저장소 처리하기: [다중 저장소 에이전트 설정](multi-repository-agent-setup.md).
- 실패하거나 멈춘 설정 복구: [에이전트 호스트 문제 해결](agent-host-troubleshooting.md).

## 계층 경계

정확한 제품 계약은 [참조 색인](../reference/README.md), 명령 동작은
[관리 CLI](../reference/admin-cli.md), stdio와 Local HTTP 전송 동작은
[MCP 전송](../reference/mcp-transport.md), 보장과 비보장은
[보안](../reference/security.md)을 사용합니다. 구현 구조나 근거가 필요할 때만
[아키텍처 가이드](../architecture-guide/README.md)를 사용합니다.
