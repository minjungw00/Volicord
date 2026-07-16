# 사용자 판단 권한은 Agent Connection 밖에 남는다

## 맥락

에이전트는 제품 방향, 범위, 민감 동작, 수락, 잔여 위험에 사용자 판단이
필요하다는 사실을 식별할 수 있습니다. 선택지와 추천도 제시할 수 있습니다. 그러나
자신의 산문, 추론한 의도, 도구 호출을 사용자의 권한 효력이 있는 답변으로 바꿀 수는
없습니다.

호출 수가 적은 작업 흐름일수록 이 경계가 중요합니다. 중간 호출을 줄이더라도 같은
에이전트가 판단을 요청하고 해결하거나, 넓은 대화 응답을 모든 대기 승인으로 다시
해석하게 해서는 안 됩니다.

## 결정

사용자 소유 판단의 해결은 Agent Connection 밖의 User Channel에 유지합니다.
에이전트 쪽 경로는 초점이 맞춰진 요청을 만들고 나중에 결과 상태를 사용할 수 있지만,
사용자의 선택, 최종 수락, 잔여 위험 수락을 대신 제출할 수 없습니다.

프로젝트 소유 정책은 담당 문서가 정의한 모든 조건을 만족한 특정 낮은 통제 작업에
최종 수락이 필요하지 않다고 정할 수 있습니다. 이것은 정책과 Core 닫기 결정이며,
추론한 사용자 판단이나 에이전트가 만든 면제가 아닙니다.

정확한 판단 종류, 요청과 해결 스키마, 호환되는 출처, 닫기 효과, User Channel
방법은 [Core 모델](../../reference/core-model.md), 사용자 행동 메서드와 스키마 담당
문서, Agent Connection, 관리 CLI, 보안 문서가 계속 정의합니다.

## 결과

- 사용자 권한 출처를 약화하지 않고 에이전트 프롬프트와 생성 지침을 단순화할 수
  있습니다.
- 대기 판단은 세션 경계를 넘어 보존되고 지원되는 로컬 사용자 경로로 해결할 수
  있습니다.
- 모델이 작성한 요약이나 넓은 승인 문구가 서로 다른 여러 결정을 조용히 충족할 수
  없습니다.
- 정책이 허용하는 자동 닫기는 만들어 낸 사용자 수락이 아니라 정책 평가로 설명할
  수 있습니다.
- 테스트는 에이전트와 로컬 사용자 출처 경로를 계속 분리합니다.

## 비목표

- 사용자 신원, 인증, 부인 방지를 정의하지 않습니다.
- 모든 호스트에 UI 하나만 요구하지 않습니다.
- 에이전트가 좁은 추천을 제시하거나 판단 대기 중 독립적인 안전 작업을 계속하는
  일을 막지 않습니다.
- 모든 대화 메시지를 User Channel 해결로 만들지 않습니다.

## 거부한 대안

- Agent Connection이 자신이 만든 요청을 해결하게 하는 방식은 요청 출처가 사용자
  권한이 아니므로 거부했습니다.
- 대화 문구에서 수락을 추론하는 방식은 문구 하나가 제품 방향, 범위, 최종 수락,
  잔여 위험 사이에서 모호할 수 있으므로 거부했습니다.
- 정책 기반의 낮은 통제 닫기를 암묵적 사용자 수락으로 취급하는 방식은 정책과
  판단이 서로 다른 권한 출처이므로 거부했습니다.

## 관련 구현

- [`crates/volicord-core/src/methods/user_action.rs`](../../../../crates/volicord-core/src/methods/user_action.rs):
  서로 다른 호출 범주를 사용하는 요청과 해결 계획.
- [`crates/volicord-cli/src/user_command.rs`](../../../../crates/volicord-cli/src/user_command.rs):
  로컬 User Channel 명령 조율.
- [`crates/volicord-mcp/src/adapter.rs`](../../../../crates/volicord-mcp/src/adapter.rs):
  사용자 전용 해결 권한 없이 에이전트 쪽 요청을 디스패치.
- [`crates/volicord-store/src/`](../../../../crates/volicord-store/src/) 아래 Store
  사용자 행동 기록과 제약.

## 관련 테스트와 참조 담당 문서

- Core 사용자 행동과 닫기 테스트, CLI User Channel 테스트, MCP 거부 테스트,
  [`tests/conformance/baseline.rs`](../../../../tests/conformance/baseline.rs).
- [통합 사용자 행동 요청과 해결](unified-user-action-request-resolution.md).
- [Core 모델](../../reference/core-model.md),
  [사용자 행동 요청](../../reference/api/method-request-user-action.md),
  [사용자 행동 해결](../../reference/api/method-resolve-user-action.md),
  [사용자 행동 스키마](../../reference/api/schema-user-action.md),
  [판단 스키마](../../reference/api/schema-judgment.md),
  [Agent Connection](../../reference/agent-connection.md),
  [관리 CLI](../../reference/admin-cli.md),
  [보안](../../reference/security.md).
