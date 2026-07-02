# Volicord 문서

Volicord(볼리코드) 한국어 문서의 진입점입니다. 하려는 일에 맞는 경로에서 시작합니다.

## 먼저 볼 곳

- 전체 제품 및 첫 설정 개요: [루트 README](../../README.ko.md)
- 제품 이해하기: [사용자 가이드 개요](user-guide/overview.md)
- 이 환경에서 Volicord 실행 파일을 실행할 수 있는지 확인하기: [시스템 요구사항](reference/system-requirements.md)
- 실행 파일 설치와 검증: [설치](user-guide/installation.md)
- Codex 또는 Claude Code 설정 경로 고르기: [빠른 시작](user-guide/quickstart.md)
- 전체 호스트 운영 또는 멈춘 설정 복구: [에이전트 호스트 설정](user-guide/agent-host-setup.md), 이후 [에이전트 호스트 문제 해결](user-guide/agent-host-troubleshooting.md)
- 여러 명시적으로 허용된 저장소 처리하기: [다중 저장소 에이전트 설정](user-guide/multi-repository-agent-setup.md)
- 정확한 CLI 또는 API 계약 찾기: [관리 CLI](reference/admin-cli.md), [API 메서드](reference/api/methods.md), [참조 색인](reference/README.md)

## 네 계층

- 사용자 가이드: [사용자 가이드 색인](user-guide/README.md)은 설치, 첫 설정, 작업 흐름 실무, 사용자 판단, 에이전트 호스트 운영, Docker, 상태, 닫기 해석, 문제 해결을 다룹니다.
- 아키텍처 가이드: [아키텍처 가이드](architecture-guide/README.md)는 구현 구조, 모듈 책임, 요청 흐름, 저장소와 트랜잭션 아키텍처, 설계 근거, 테스트 전략을 다룹니다.
- 참조: [참조 색인](reference/README.md)은 API 메서드, 스키마, CLI 동작, MCP 전송, 저장소, 런타임 경계, 보안 표현, 용어, 적합성, 설계 품질 의미 같은 안정적인 계약을 다룹니다.
- Maintain: [문서 정책](maintain/documentation-policy.md), [문서 헌장](maintain/document-charters.md), [번역 정책](maintain/translation-policy.md), [브랜드 지침](maintain/brand-guidelines.md), [검증](maintain/validation.md)은 오래 유지될 유지보수 규칙을 다룹니다.

## 독자별 경로

- 제품 사용자: [사용자 가이드 개요](user-guide/overview.md)를 읽은 뒤 [사용자 가이드](user-guide/user-workflow.md), [판단 예시](user-guide/judgment-examples.md), [에이전트 호스트 설정](user-guide/agent-host-setup.md)으로 이동합니다.
- 에이전트 호스트 운영자: [시스템 요구사항](reference/system-requirements.md), [설치](user-guide/installation.md), [빠른 시작](user-guide/quickstart.md), [에이전트 호스트 설정](user-guide/agent-host-setup.md)을 보고, 설정이 실패하거나 멈추면 [에이전트 호스트 문제 해결](user-guide/agent-host-troubleshooting.md)을 봅니다.
- 다중 저장소 운영자: [다중 저장소 에이전트 설정](user-guide/multi-repository-agent-setup.md)을 봅니다.
- 에이전트: [에이전트 가이드](user-guide/agent-workflow.md)를 사용하고, 저장소 작업 규칙은 [AGENTS.md](../../AGENTS.md)에서 확인합니다.
- 소스 코드 학습자: [아키텍처 가이드](architecture-guide/README.md)에서 시작한 뒤 [코드베이스 둘러보기](architecture-guide/codebase-tour.md), [요청 생명주기](architecture-guide/request-lifecycle.md), [아키텍처](architecture-guide/architecture.md)를 봅니다.
- 참조 독자: [참조 색인](reference/README.md), [관리 CLI](reference/admin-cli.md), [API 메서드](reference/api/methods.md)에서 담당 문서를 찾습니다.
- 문서 유지보수자: [제품 및 유지보수 헌장](maintain/product-maintenance-charter.md), [문서 정책](maintain/documentation-policy.md), [문서 헌장](maintain/document-charters.md), [그림 정책](maintain/diagram-policy.md), [번역 정책](maintain/translation-policy.md), [브랜드 지침](maintain/brand-guidelines.md), [검증](maintain/validation.md), [doc-index.yaml](../doc-index.yaml), [용어 지도](../terminology-map.yaml)를 사용합니다.

독자용 문서는 작업을 설명하고 순서화합니다. 정확한 제품 계약은 참조 문서에 있습니다. 사람이 읽는 참조 경로는 [참조 색인](reference/README.md)을 사용합니다. 유지보수 중 기계가 읽는 정확한 담당 경로가 필요할 때는 [doc-index.yaml](../doc-index.yaml)을 사용합니다.
