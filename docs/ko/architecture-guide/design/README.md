# 아키텍처 설계

이 문서 계열은 현재 Volicord Rust 워크스페이스의 구현 설계를 설명합니다.
각 문서는 현재 책임, 실행 구조, 불변 조건, 실패 동작, 소스 경로를 중심으로
구성합니다. 정확한 공개 동작, 스키마, 저장 효과, 보안 보장, 값의 의미는 계속
집중 참조 담당 문서가 담당합니다.

## 설계 참조 지도

| 설계 참조 | 현재 구현 관심사 |
|---|---|
| [Agent Connection 라우팅](agent-connection-routing.md) | 관리 stdio 프로세스를 현재 Connection 하나와 명시적으로 승인된 Product Repository에 결속합니다. |
| [Core와 어댑터 경계](core-adapter-boundary.md) | 문법, 어댑터, Core 정책, Store, 진단, 저장소 도구를 서로 다른 의존 계층에 둡니다. |
| [상태 결속 Write Ticket 유효성](state-bound-write-ticket-validity.md) | 현재 권한 좌표에서 티켓 재사용을 평가하고 보호 대상 변경과 함께 티켓을 소비합니다. |
| [관찰 신뢰도 경계](observation-confidence-boundary.md) | 구조화된 경로 사실, 불확실한 관찰, 조정, typed 진단을 구분합니다. |
| [외부 사용자 판단 권한](external-user-judgment-authority.md) | 사용자 소유 해결을 Agent Connection 밖의 로컬 User Channel에 둡니다. |
| [정적 압축 MCP 도구 목록](static-compact-mcp-tool-list.md) | 압축된 런타임 스키마와 함께 하나의 폐쇄형 capability 인식 도구 catalog를 투영합니다. |
| [작업 결과 조회](operation-result-retrieval.md) | 효과를 다시 실행하지 않고 불변 replay 응답에서 한도가 있는 정확한 페이지를 읽습니다. |
| [Evidence capture 생산자 확정](evidence-capture-producer-finalization.md) | source receipt를 현재 capture intent에 결속하고 Run commit과 함께 producer record를 확정합니다. |
| [통합 UserAction 요청과 해결](unified-user-action-request-resolution.md) | agent-safe 요청 및 재개 projection과 로컬 사용자 해결을 분리합니다. |
| [계획과 원자적 커밋](plan-and-atomic-commit.md) | 계획한 필드에서 typed 메서드 결과를 구성하고 하나의 commit 경계에서 그룹화된 Store mutation을 적용합니다. |
| [정규 Core UTC 시계](canonical-core-utc-clock.md) | 준비된 동작 시각과 프로젝트 범위 비감소 시간 하한을 조율합니다. |
| [Runtime Home과 Product Repository](runtime-home-and-product-repository.md) | 런타임 기록, 제품 파일, 설치 파일, 저장소 도구를 각각의 담당 위치에 둡니다. |

## 읽기 경로

워크스페이스 형태는 [구현 아키텍처](../architecture.md), 정확한 모듈 책임은
[소스 지도](../source-map.md), 대표 Core 경로는
[요청 생명주기](../request-lifecycle.md), 지속 저장 조율은
[저장소와 트랜잭션](../storage-and-transactions.md)에서 시작합니다. 정확한
제품 동작이 중요할 때는 [참조 색인](../../reference/README.md)을 사용합니다.
