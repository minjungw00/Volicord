# 아키텍처 결정

이 문서들은 현재 Rust workspace의 지속 구현 구조를 설명합니다. 공개 동작, schema,
효과, 보안, 값 집합은 집중된 Reference 소유자가 정본입니다.

| 결정 | 구조적 목적 |
|---|---|
| [Agent Connection routing](agent-connection-routing.md) | 하나의 Codex Record 연결과 명시적 project membership을 하나의 관리 stdio 프로세스에 결속. |
| [Core와 어댑터 경계](core-adapter-boundary.md) | Core를 CLI와 MCP 세부사항에서 독립시킴. |
| [상태 결속 Write Ticket 유효성](state-bound-write-ticket-validity.md) | 관련 작업과 권한 좌표가 유효한 동안만 ticket 재사용. |
| [관찰 신뢰도 경계](observation-confidence-boundary.md) | 결정적 경로 사실과 불확실한 관찰 분리. |
| [외부 사용자 판단 권한](external-user-judgment-authority.md) | 사용자 답변을 Agent Connection 밖에 유지. |
| [정적 압축 MCP 도구 목록](static-compact-mcp-tool-list.md) | 공개 도구 registry를 폐쇄적이고 작게 유지. |
| [Host release evidence gate](host-release-evidence-gate.md) | 독립된 네 플랫폼 셀로 정확한 최종 Codex 아티팩트 지원. |
| [영속 operation-result 조회](operation-result-retrieval.md) | 적격 immutable mutation 결과를 bounded lookup으로 복구. |
| [Evidence-capture producer finalization](evidence-capture-producer-finalization.md) | source receipt를 intent에 결속하고 producer authority를 원자적으로 확정. |
| [통합 UserAction 요청과 해결](unified-user-action-request-resolution.md) | agent 요청/재개와 CLI-only immutable resolution 분리. |
| [계획과 원자적 commit](plan-and-atomic-commit.md) | Store transaction 전에 효과 계획. |
| [정규 Core UTC clock](canonical-core-utc-clock.md) | 하나의 비감소 prepared-operation 시간 모델 사용. |
| [Runtime Home과 Product Repository](runtime-home-and-product-repository.md) | runtime 상태와 제품 파일 분리. |

소유자 경로는 [아키텍처](../architecture.md), [소스 맵](../source-map.md),
[Reference Index](../../reference/README.md)를 봅니다.
