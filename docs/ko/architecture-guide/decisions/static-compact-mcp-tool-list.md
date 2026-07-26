# MCP는 기본으로 정적이고 간결한 도구 목록을 사용한다

## 맥락

MCP `tools/list` 응답은 일반 작업이 시작되기 전에 에이전트 맥락에 들어갑니다.
모든 런타임 도구 스키마에 전체 요청 예시와 긴 분기 설명을 넣으면 에이전트가 다음
행동 하나만 필요할 때도 맥락을 소비합니다. 큰 스키마는 추측성 작업 흐름 호출을
유도할 수도 있습니다.

Task 전환마다 보이는 도구 집합을 바꾸면 일부 메타데이터를 줄일 수 있지만,
클라이언트 역량, 캐시 무효화, 발견, 복구 문제가 생깁니다. 이런 문제는 기본 상호작용
모델이 되기 전에 실제 에이전트 평가 증거가 필요합니다.

## 결정

기존 연결 모드와 저장소 역량 모드마다 정적 도구 집합 하나를 유지하고 런타임 상태
보기를 간결하게 만듭니다.

- 런타임 스키마는 설명용 예시를 제외하고 설명을 결과, 권한 경계, 도구가 적용되는
  시점에 집중합니다.
- 문서와 계약 픽스처는 별도의 문서용 상태 보기를 통해 예시를 유지할 수 있습니다.
- 작업 흐름 응답은 담당 메서드가 있는 실행 가능한 `next_action`을 전달하므로
  에이전트는 도구를 추측해서 호출하지 않고 반환된 상태를 따릅니다.
- 생성 계약 snapshot과 직렬화 크기 테스트가 간결한 런타임 상태 보기를 보호합니다.
- 상태에 따라 바뀌는 동적 도구 목록은 클라이언트 역량과 에이전트 평가 증거가
  필요성을 뒷받침한 뒤에만 확장 후보로 남깁니다.

정확한 공개 도구 집합, 스키마 상태 보기, 바이트 상한, `next_action` 형태, degraded
mode 동작, snapshot 계약은 [MCP 전송](../../reference/mcp-transport.md), 공개 API
스키마 담당 문서, Agent Connection이 계속 정의합니다.

## 결과

- 모든 클라이언트가 세션 중 도구 목록 변경을 지원하지 않아도 안정적인 집합을
  발견할 수 있습니다.
- 예시는 모든 런타임 프롬프트 공간을 차지하지 않고 문서와 테스트에 남습니다.
- 어댑터는 정상 응답과 크기가 제한된 복구 응답 모두에서 다음 행동 경로를 일관되게
  채워야 합니다.
- snapshot 변경은 의미 검토가 필요하며, 바이트 크기를 줄인다는 이유로 검증이나
  권한 필드를 제거할 수 없습니다.
- 에이전트 평가는 동적 목록 확장을 승격하기 전에 맥락 비용과 호출 비용을 비교할
  수 있습니다.

## 비목표

- 공개 메서드를 제거하거나 입력 검증을 느슨하게 하지 않습니다.
- 동적 도구 목록을 영원히 금지하지 않습니다.
- `next_action`을 새로운 권한 출처로 만들지 않습니다. 현재 담당 문서가 정의한
  상태의 상태 보기입니다.
- 바이트 목표는 에이전트가 올바른 도구를 고른다는 증명이 아닙니다.

## 거부한 대안

- 전체 예시를 런타임 스키마에 유지하는 방식은 발견 응답마다 반복되고 문서용 상태
  보기에 속하므로 거부했습니다.
- Task 단계별 도구 목록을 기본으로 사용하는 방식은 상태를 가진 발견 동작을
  추가하기에 현재 클라이언트와 평가 증거가 부족하므로 거부했습니다.
- 검증 세부사항을 제거해서 스키마를 줄이는 방식은 간결한 전송도 공개 계약을
  보존해야 하므로 거부했습니다.

## 관련 구현

- [`crates/volicord-mcp/src/tool_registry.rs`](../../../../crates/volicord-mcp/src/tool_registry.rs):
  도구 정의, 설명, 런타임과 문서용 스키마 상태 보기.
- [`crates/volicord-mcp/src/tool_dispatch.rs`](../../../../crates/volicord-mcp/src/tool_dispatch.rs):
  `tools/list`, 작업 흐름 응답 래핑, 크기가 제한된 복구 상태 보기.
- [`crates/volicord-types/src/schema.rs`](../../../../crates/volicord-types/src/schema.rs):
  공유 다음 행동 응답 형태.
- [`tests/integration/public_contract_snapshots.rs`](../../../../tests/integration/public_contract_snapshots.rs):
  생성 공개 계약 상태 보기와 검토된 snapshot.

## 관련 테스트와 참조 담당 문서

- [`crates/volicord-mcp/src/tests/protocol_projection.rs`](../../../../crates/volicord-mcp/src/tests/protocol_projection.rs)의
  MCP protocol projection 단위 테스트,
  [`crates/volicord-cli/tests/mcp_transport.rs`](../../../../crates/volicord-cli/tests/mcp_transport.rs)의
  프로세스 검증,
  [`tests/integration/mcp_connection.rs`](../../../../tests/integration/mcp_connection.rs)의
  통합 검증.
- [MCP 전송](../../reference/mcp-transport.md),
  [API 메서드](../../reference/api/methods.md),
  [API 상태 스키마](../../reference/api/schema-state.md),
  [Agent Connection](../../reference/agent-connection.md).
