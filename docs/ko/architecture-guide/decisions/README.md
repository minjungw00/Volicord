# 아키텍처 결정

이 디렉터리는 현재 Rust 구현을 위한 작은 오래 유지될 아키텍처 결정
집합을 담습니다. 각 페이지는 안정적인 의도 구조, 결과, 비목표, 관련 소스,
테스트, 참조 담당 문서를 설명합니다.

이 문서들은 공개 API 동작, 스키마, 저장 효과, 보안 보장, 런타임 동작,
Core 권한 의미, 제품 수락, 닫기 준비 상태, 적합성 결과를 정의하지 않습니다.

## 결정 집합

| 결정 | 사용할 때 |
|---|---|
| [Agent Connection과 호스트 라우팅](agent-connection-routing.md) | 왜 coding-agent MCP setup이 고정된 Product Repository 하나가 아니라 Agent Connection과 명시적 Connection Project membership에 묶이는지 확인합니다. |
| [Core와 어댑터 의존 경계](core-adapter-boundary.md) | 왜 Core가 MCP나 CLI 어댑터에 의존하지 않는지, 그리고 어댑터 코드가 Core 호출 전에 무엇을 할 수 있는지 확인합니다. |
| [원자적 변이 커밋 전 계획](plan-and-atomic-commit.md) | 왜 메서드가 Store 커밋 전에 효과를 계획하고, 왜 Store가 원자적 트랜잭션 경계를 소유하는지 확인합니다. |
| [Runtime Home과 Product Repository 분리](runtime-home-and-product-repository.md) | 런타임 상태와 제품 파일이 왜 별도 위치에 남아야 하는지, 구현 코드가 그 분리를 어떻게 반영하는지 확인합니다. |

전체 워크스페이스 지도는 [구현 아키텍처](../architecture.md)를, 반복되는 구현
구조는 [설계 패턴](../design-patterns.md)을, Store 커밋과 아티팩트 경계는
[저장소와 트랜잭션](../storage-and-transactions.md)을 사용합니다.
