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
| [Task 통제 수준과 통합 프로필은 서로 다른 축이다](task-control-levels-vs-integration-profiles.md) | Record와 Detective는 호스트 통합 선택으로 남고 Core는 프로젝트 정책의 제약을 받는 별도 통제 수준을 Task마다 기록하는 이유를 설명합니다. |
| [쓰기 티켓 유효성은 관련 작업 상태에 결속한다](state-bound-write-ticket-validity.md) | 티켓 호환성이 관련 없는 프로젝트 전체 변경이나 고정된 기본 수명이 아니라 Task, Change Unit, 범위, baseline, workspace, 승인 근거를 따르는 이유를 설명합니다. |
| [세션 Stop과 Task 완료는 서로 다른 결과다](stop-completion-separation.md) | 호스트 세션은 끝날 수 있지만 닫기 상태가 완료 주장을 계속 억제할 수 있는 이유를 설명합니다. |
| [확인된 효과와 휴리스틱 관찰은 권한 효력이 다르다](observation-confidence-boundary.md) | 결정론적 경로 사실은 행동 전 집행을 뒷받침하고 불확실한 셸 효과는 행동 뒤 보강될 때까지 경고로 남는 이유를 설명합니다. |
| [사용자 판단 권한은 Agent Connection 밖에 남는다](external-user-judgment-authority.md) | 에이전트는 판단을 요청하고 결과를 사용할 수 있지만 별도 User Channel만 사용자의 답변을 기록하는 이유를 설명합니다. |
| [MCP는 기본으로 정적이고 간결한 도구 목록을 사용한다](static-compact-mcp-tool-list.md) | 런타임 스키마가 문서 예시를 제외하고 상태별 동적 도구 목록을 검토하기 전에 반환된 다음 행동 경로를 사용하는 이유를 설명합니다. |
| [최종 출력 권한 고지](final-output-authority-disclosure.md) | 최신 권한 고지가 공유 status/receipt 검증기 하나와 Detective Stop 집행에서 분리된 프로필과 무관한 호스트 UI 경로를 사용하는 이유를 설명합니다. |
| [Credential 전달을 위한 호스트 역량 검증](host-capability-verification.md) | Credential을 포함한 local-web 전달에 listener 준비와 협력적 클라이언트 선언 외에도 정확하고 만료되는 실제 호스트 증거가 필요한 이유를 설명합니다. |
| [관리 호스트 세션·thread 결속과 호출별 turn 검증](managed-host-session-turn-binding.md) | 정확한 호출별 Codex 세션·thread 메타데이터가 stdio 프로세스 하나를 결속할 때까지 관리 시작 출처가 대기 상태로 남는 이유를 설명합니다. |
| [외부 호스트 릴리스 증거 게이트](host-release-evidence-gate.md) | 외부의 정확한 최종 후보 하나에 고정 12개 셀 정규 게이트와 별도 프로세스 재계산 audit을 적용하는 이유를 설명합니다. |
| [호스트 기능 지원 상태 평가](host-feature-support-state-evaluation.md) | 구현, 설정, 정확한 실제 증거, 현재 런타임 준비 상태를 모호한 지원 boolean 대신 typed 평가기 하나로 구분하는 이유를 설명합니다. |
| [오래 유지되는 작업 결과 조회](operation-result-retrieval.md) | 정확한 과거 변경 응답이 변경 불가능한 재실행 행과 접근을 확인하는 제한된 페이지 조회를 재사용하는 이유를 설명합니다. |
| [Evidence capture intent와 producer 최종화](evidence-capture-producer-finalization.md) | Source-owned receipt를 만료되는 intent에 결합하고 `record_run` 안에서만 producer 권한으로 만드는 이유를 설명합니다. |
| [통합 사용자 행동 요청과 해결](unified-user-action-request-resolution.md) | 판단과 사용자 증거 관찰이 하나의 대기 요청, 변경 불가능한 해결, 채널 어댑터 생명주기를 공유하는 이유를 설명합니다. |
| [원자적 변이 커밋 전 계획](plan-and-atomic-commit.md) | 왜 메서드가 Store 커밋 전에 효과를 계획하고, 왜 Store가 원자적 트랜잭션 경계를 소유하는지 확인합니다. |
| [정규 Core UTC 시계](canonical-core-utc-clock.md) | 프로젝트 시각이 `state_version`과 구분되는 감소하지 않는 영속 하한, 준비된 동작 샘플 하나, 정규 Core 커밋 timestamp 하나를 갖는 이유를 설명합니다. |
| [Runtime Home과 Product Repository 분리](runtime-home-and-product-repository.md) | 런타임 상태와 제품 파일이 왜 별도 위치에 남아야 하는지, 구현 코드가 그 분리를 어떻게 반영하는지 확인합니다. |

[구현 아키텍처](../architecture.md)는 워크스페이스 아키텍처 개요, 의존 경계
개요, 오래 유지될 구현 경계, 세부 경로를 볼 때 사용합니다. 정확한 소스 경로
책임과 모듈 배치는 [소스 지도](../source-map.md)를, 반복되는 구현 구조는
[설계 패턴](../design-patterns.md)을, Store 커밋과 아티팩트 경계는
[저장소와 트랜잭션](../storage-and-transactions.md)을, 정확한 Cargo 의존
관계는 `Cargo.toml` 매니페스트를 사용합니다.
