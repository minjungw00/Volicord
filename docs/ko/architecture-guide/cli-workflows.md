# CLI 작업 흐름

이 가이드는 지원 관리 CLI의 오래 유지될 구현 흐름을 설명합니다. 정확한 명령 동작은
[관리 CLI](../reference/admin-cli.md)가 담당합니다.

## 소유권 지도

| 단계 | 구현 책임 |
|---|---|
| parse와 normalize | CLI command DTO가 알 수 없거나 충돌하는 입력을 거부합니다. |
| context 해결 | Runtime Home, 정규 Product Repository, project, Agent Connection을 선택합니다. |
| plan | 읽기 전용 검사가 정확한 파일과 Store 변경안을 만듭니다. |
| validate | 관리 구성, Connection, session, 저장소, policy를 검사합니다. |
| commit | 담당 문서가 정의한 원자적 filesystem/Store 경계를 사용합니다. |
| render | 구조화된 결과에서 text 또는 JSON 문서 하나를 만듭니다. |

Parsing과 rendering은 다른 Core 또는 Store 권한이 되면 안 됩니다.

## Codex 설정

`init`은 Codex, `record`, 개인/공유 범위만 받습니다. CLI는 현재 정규 입력을 해결하고
Codex adapter에 관리 구성 생성을 요청하며 정확한 관리 변경을 미리 본 뒤 모든 전제
조건이 통과해야 적용합니다. 복구는 같은 흐름을 다시 사용하고 제거는 일치하는 관리
내용만 삭제합니다.

하나의 typed 관리형 MCP 시작 계약이 명령, 인자, 정적 및 전달 환경 binding,
개인/공유 구분, 정규 projection, fingerprint 입력을 담당합니다. 개인 구성은 선택한
절대 Runtime Home을 정적 `VOLICORD_HOME`으로 결속하고, 공유 구성은
`VOLICORD_HOME`만 전달하여 clone 이식성을 유지합니다. Codex 어댑터는 이 계약을
TOML로 직렬화하고 관리 entry를 다시 계약으로 parsing하며 허용된 도구 승인 overlay만
보존합니다. 플랫폼 파일시스템 경계는 Linux 또는 WSL2 분류와 target 및 파일시스템
검증을 별도로 담당합니다. Core는 Store가 소유한 운영 기록에서 만든 현재
`ValidatedAgentSession`만 받습니다.

## 연결 검증

`init`과 선택한 Connection의 `add`, `status`, `verify`, `mode`, `remove` 흐름은 정규
검증 type의 check와 action을 사용해 typed command report 하나를 만듭니다. 선택적인 tagged
result 하나가 다른 상태 트리를 만들지 않고 설정, mode 전환, 제거 사실을 담당합니다.
JSON과 text renderer는 이 보고서를 소비하고 binary 종료 처리는 typed 집계 상태를 읽습니다.
Rendering이 병렬 상태 트리를 다시 만들거나 자체 출력을 parsing하지 않습니다. Connection
list는 명령 보고서 상태에 의존하지 않는 집중 list projection을 유지합니다.

`connection status`는 활성 probe를 실행하거나 파일, 보고서, 관찰, timestamp를 쓰지
않고 현재 파일과 Store 관찰을 읽습니다. `connection verify`는 현재 adapter와 관리 구성을
검사하고 허용된 로컬 probe를 실행한 뒤 실제 managed-host 및 Guard 관찰을 읽고 Store
담당 경로로 보고서를 최대 하나 commit합니다. Executable path와 version은 diagnostic probe
사실입니다. 권위 있는 관리 runtime/project session은 관리 MCP lifecycle 처리에서만 기록하며,
CLI self-test는 `session_source=cli_preflight`를 기록하므로 managed-host 호출을 승인할 수
없습니다.

## 프로젝트와 정책 흐름

Project 명령은 등록된 정규 Git 작업 트리를 해결합니다. Policy apply는 plan, 엄격한
검증, 원자적 commit을 사용합니다. 두 명령 계열 모두 표시 이름에서 권한을 추론하거나
알 수 없는 저장 값을 복구하지 않습니다.

## UserAction 흐름

`inbox`는 strict typed pending request를 읽고 로컬 사용자 소유 form을 표시합니다.
`inbox resolve`는 저장 choice 또는 evidence observation 하나를
`volicord.resolve_user_action`으로 제출합니다. MCP adapter는 요청을 만들거나 재개할
수 있지만 이 해결 경로를 호출할 수 없습니다.

Guard prompt 관찰은 CLI 답이 되지 않습니다. 손상된 저장 요청이나 resolution은 기본
form 대신 영속 데이터 오류로 실패합니다.

## 조정

`changes reconcile`은 공개 Core 메서드로 경로를 잡습니다. Suppression은 명시적인
`Applied` 또는 `Unavailable`이며 rendering은 모든 remaining path와 unavailable reason을
보존해야 합니다.

## 진단과 출력

`doctor`, status, preflight는 읽기 전용 사실을 모아 이름 붙은 다음 동작을 보고합니다.
Connection report 명령에서 `dry_run`은 작업 boolean이며 집계 결과는 3상태로 유지됩니다.
`--json`은 typed 결과를 한 번 직렬화합니다. 사람용 text, log,
diagnostic metadata를 권한 상태로 다시 parse하지 않습니다.

## 경계

- CLI는 Core와 Store에 의존하며 Core는 CLI에 의존하지 않습니다.
- Codex별 구성은 adapter에 남습니다.
- 어떤 명령도 네트워크 전송을 시작하지 않습니다.
- 비대화형 명령은 사용자 판단을 제출하지 않습니다.
- Client와 host version 관찰은 diagnostic입니다. Host version이 바뀌면 운영 관찰을
  갱신하며, 관리 호출 권한은 현재 권위 있는 session 소유권과 정확한 binding을 사용합니다.

## 관련 경로

- [소스 지도](source-map.md)
- [요청 lifecycle](request-lifecycle.md)
- [Agent Connection](../reference/agent-connection.md)
- [MCP 전송](../reference/mcp-transport.md)
- [테스트 전략](testing-strategy.md)
