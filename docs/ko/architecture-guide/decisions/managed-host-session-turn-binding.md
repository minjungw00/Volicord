# 관리 호스트 세션·thread 결속과 호출별 turn 검증

## 맥락

생성된 Codex MCP 기술 정보는 Volicord가 시작 형태를 소유한다는 사실을 보여 줄 수 있지만,
도구를 호출한 native Codex 세션이나 thread를 식별하지 못합니다. 호스트 환경 변수는 MCP와
훅 자식에 일관되게 전달되지 않으며 프로세스 조상 관계, PID, 이벤트 순서, 시간적 근접성도
서로 다른 표면의 정확한 상관관계를 확립하지 못합니다. 프로세스 시작 시점에 결속하면
identity를 만들어 내거나 영속 관찰을 잘못된 호스트 세션과 연결하게 됩니다.

Codex `0.144.4`는 각 MCP 도구 호출에 권위 있는 협력적 상관관계 메타데이터를 제공합니다.
바깥 `_meta.threadId`와 안쪽 `_meta["x-codex-turn-metadata"]`의 `session_id`,
`thread_id`, `turn_id`입니다. MCP 초기화 identity는 정확히
`clientInfo.name=codex-mcp-client`, `clientInfo.version=0.144.4`입니다.

## 결정

Volicord는 기술 정보나 관리 marker 검증을 시작 출처로만 취급합니다. 관리 Codex stdio
프로세스는 결속 대기 상태로 시작합니다. 대기 상태에서는 진단 세션, 세션 감시 기준선,
관리 생명주기 행, Core 효과, token, local-web 자격을 만들지 않습니다.

정확한 초기화 identity와 ready 전환 뒤 처음으로 알려진 도구를 구조적으로 올바르게
호출하면 프로세스를 결속할 수 있습니다. 어댑터는 먼저 JSON-RPC 형태, 도구 이름,
`arguments`를 검증한 뒤 다음을 요구합니다.

- 문자열 `_meta.threadId`
- 문자열 `session_id`, `thread_id`, `turn_id`가 있는 객체
  `_meta["x-codex-turn-metadata"]`
- 바깥 `threadId`와 안쪽 `thread_id`의 정확한 같음
- 각 native 값이 UTF-8 1바이트 이상 256바이트 이하이고
  `[A-Za-z0-9._:-]+`와 일치함

어댑터는 `session_id`를 domain 분리된 `volicord-managed-host-session-v1` 함수로
매핑하고 `thread_id`에는 별도의 domain 분리 메모리 내 다이제스트를 파생합니다. 원본
세션·thread·turn 값은 검증과 해시 뒤 폐기합니다. 매핑된 세션과 thread 다이제스트는
해당 stdio 프로세스 수명 동안 바뀌지 않습니다. 이후 호출은 둘 다 일치해야 하며 새
turn에서는 다른 유효한 `turn_id`를 사용할 수 있습니다.

처음 성공한 결속은 한정된 세션 감시 범위를 시작하고 그때까지 프로세스에서 관찰한
생명주기 사실을 구체화합니다. 관찰 범위는 명시적으로 부분 범위이며 결속 시점에
시작합니다. 구체화한 시작, 초기화, 도구 목록 사실은 기준선 이전의 Product Repository
변경을 관찰했다는 주장이 아닙니다. 값이 없거나 형식이 잘못되었거나 세션 또는 thread가
일치하지 않으면 영속 효과, Core 효과, 도구 호출 효과, token, local-web 효과 없이
JSON-RPC `-32602`를 반환합니다. 다시 결속하는 경로는 없습니다.

로컬 `diagnostics.sqlite` 영속화는 최선형이며 권한 효력이 없습니다. 손상, 쓰기 거부,
이미 존재하는 진단 좌표 충돌은 그 밖에는 유효한 결속을 거부하거나 MCP, guard, Core
결과를 바꿀 수 없습니다. 어댑터는 가능하면 해당 진단 실패를 건너뛰거나 치명적이지 않게
보고합니다. 권한 효력이 있는 소유권 충돌은 프로젝트 Agent Session과 등록 연결 상태에서
판단합니다. 잘못되거나 일치하지 않는 요청 메타데이터는 계속 위의 효과 없는 거부를
따릅니다.

이 메타데이터는 협력적 로컬 상관관계 입력이며 사용자 신원, 권한, host attestation,
같은 로컬 principal에 대한 위조 방지, 호스트 격리 증명이 아닙니다. 정확한 wire 동작은
[MCP 전송](../../reference/mcp-transport.md)이, 불투명 매핑과 릴리스 증거 규칙은
[호스트 릴리스 증거](../../reference/host-release-evidence.md)가 담당합니다.

## 결과

- 저장소 기술 정보가 유효해도 프로세스는 아직 결속되지 않은 상태일 수 있습니다.
- 관리 생명주기와 Strong Evidence는 원본 호스트 ID를 보존하지 않고 MCP와 훅 경로에서
  정확한 불투명 root 세션 좌표 하나를 사용합니다.
- 서로 다른 Codex thread가 같은 root 세션에 매핑될 수 있지만 각 stdio 프로세스는
  자신의 정확한 thread 결속을 유지합니다.
- 세션과 thread가 그대로 일치하면 이후 turn에서 다시 호출할 수 있습니다.
- 시작 관찰 범위는 세션 감시를 과거 시점으로 소급하지 않고 결속 전 공백을 정직하게
  보고합니다.
- 진단 저장소 가용성이나 좌표 충돌은 두 번째 관리 세션 권한 원천이 될 수 없습니다.

## 호환성과 마이그레이션

이 결정은 검토된 `0.144.4` 클라이언트의 관리 Codex transport 경로를 더 엄격하게 합니다.
필수 메타데이터가 없는 관리 호출에 더 이상 합성 관리 세션을 제공하지 않습니다. 환경
fallback, 시간 rendezvous, 레거시 alias, 이전 관찰 migration은 없습니다. 호환 관찰은
새로 결속된 호스트 세션을 통해 다시 만듭니다.

공개 Core API 메서드, 공개 MCP 도구 인자, 공개 도구 스키마 필드, SQLite DDL, 저장소
프로필 버전을 추가하지 않습니다. 요청측 `_meta`는 숨은 transport 메타데이터로 남습니다.

## 거부한 대안

- 기술 정보나 관리 marker를 세션 identity로 취급하는 방안은 시작 출처만 증명하므로
  거부했습니다.
- `CODEX_THREAD_ID` 또는 다른 호스트 환경 변수를 사용하는 방안은 호출별 MCP와 훅의
  권위 있는 공통 채널이 아니므로 거부했습니다.
- PID, 부모 프로세스, 시간 창, 도착 순서, 가장 가까운 세션으로 최신 훅과 MCP 이벤트를
  짝짓는 방안은 동시 세션에서 결과가 모호하므로 거부했습니다.
- 원본 세션, thread, turn, 이벤트, invocation 식별자를 영속 저장하는 방안은 상관관계에
  domain 분리된 불투명 값만 필요하므로 거부했습니다.
- 이후 호출이 프로세스를 다시 결속하게 하는 방안은 서로 다른 native 세션이나 thread의
  관찰을 섞으므로 거부했습니다.

## 관련 구현 영역과 담당 문서

- [`crates/volicord-mcp`](../../../../crates/volicord-mcp): 대기/결속 stdio 상태,
  initialize 보존, 요청 메타데이터 검증, 지연된 생명주기 구체화.
- 공유 관리 호스트 세션 타입: 불투명 세션과 thread 다이제스트 매핑.
- [Agent Connection](../../reference/agent-connection.md)
- [MCP 전송](../../reference/mcp-transport.md)
- [호스트 릴리스 증거](../../reference/host-release-evidence.md)
- [보안](../../reference/security.md)
