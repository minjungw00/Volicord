# 사용자 가이드

Volicord로 무언가를 해야 할 때 사용자 가이드에서 시작합니다. 이 문서는 설치,
첫 설정, 사용자 작업 흐름, 판단 예시, 에이전트 작업 흐름, 에이전트 호스트 설정,
다중 저장소 운영, 상태와 닫기 해석, 문제 해결로 안내합니다.

이 문서들은 절차와 공개 작업 흐름 용어를 설명합니다. 정확한 CLI, MCP, API,
저장소, 보안, 권한 동작은 연결된 참조 담당 문서를 따릅니다.

## 먼저 볼 곳

- 제품 방향 잡기: [사용자 가이드 개요](overview.md).
- 실행 파일 설치 또는 선택: [설치](installation.md).
- 내장 관리 호스트의 첫 설정 실행하기: [빠른 시작](quickstart.md).
- 에이전트와 보이는 판단을 다루기: [사용자 작업 흐름](user-workflow.md).
- 판단 문구와 경계 고르기: [판단 예시](judgment-examples.md).
- 에이전트로 작업하기: [에이전트 작업 흐름](agent-workflow.md).
- 호스트 통합 설정, 검증, 제거: [에이전트 호스트 설정](agent-host-setup.md).
- 여러 명시적으로 허용된 저장소 처리하기: [다중 저장소 에이전트 설정](multi-repository-agent-setup.md).
- 실패하거나 멈춘 설정 복구: [에이전트 호스트 문제 해결](agent-host-troubleshooting.md).

## 계층 경계

정확한 제품 계약은 [참조 색인](../reference/README.md), 명령 동작은
[관리 CLI](../reference/admin-cli.md), 관리 stdio 전송 동작은
[MCP 전송](../reference/mcp-transport.md), 보장과 비보장은
[보안](../reference/security.md)을 사용합니다. 구현 구조나 근거가 필요할 때만
[아키텍처 가이드](../architecture-guide/README.md)를 사용합니다.
