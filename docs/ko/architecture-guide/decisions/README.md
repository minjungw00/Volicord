# 아키텍처 결정

이 디렉터리는 현재 Rust 구현에서 오래 유지할 소수의 아키텍처 결정을
담습니다. 각 페이지는 의도한 안정적 구조, 결과, 비목표, 관련 소스,
테스트, 참조 담당 문서를 설명합니다.

이 문서들은 공개 API 동작, 스키마, 저장 효과, 보안 보장, 런타임 동작,
Core 권한 의미, 제품 수락, 닫기 준비 상태, 적합성 결과를 정의하지 않습니다.

## 결정 집합

| 결정 | 사용할 때 |
|---|---|
| [Agent Connection과 호스트 처리 경로](agent-connection-routing.md) | 코딩 에이전트의 MCP 설정이 고정된 Product Repository 하나가 아니라 Agent Connection과 명시적인 연결 프로젝트 멤버십에 묶이는 이유를 설명합니다. |
| [Core와 어댑터 의존 경계](core-adapter-boundary.md) | 왜 Core가 MCP나 CLI 어댑터에 의존하지 않는지, 그리고 어댑터 코드가 Core 호출 전에 무엇을 할 수 있는지 확인합니다. |
| [원자적 변이 커밋 전 계획](plan-and-atomic-commit.md) | 왜 메서드가 Store 커밋 전에 효과를 계획하고, 왜 Store가 원자적 트랜잭션 경계를 소유하는지 확인합니다. |
| [Runtime Home과 Product Repository 분리](runtime-home-and-product-repository.md) | 런타임 상태와 제품 파일이 왜 별도 위치에 남아야 하는지, 구현 코드가 그 분리를 어떻게 반영하는지 확인합니다. |

[구현 아키텍처](../architecture.md)는 워크스페이스 아키텍처 개요, 의존 경계
개요, 오래 유지될 구현 경계, 세부 경로를 볼 때 사용합니다. 정확한 소스 경로
책임과 모듈 배치는 [소스 지도](../source-map.md)를, 반복되는 구현 구조는
[설계 패턴](../design-patterns.md)을, Store 커밋과 아티팩트 경계는
[저장소와 트랜잭션](../storage-and-transactions.md)을, 정확한 Cargo 의존
관계는 `Cargo.toml` 매니페스트를 사용합니다.
