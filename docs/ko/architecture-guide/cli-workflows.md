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

공유 구성은 `VOLICORD_HOME`을 전달하고 관리 stdio를 시작합니다. 개인 구성은 사용자
소유로 남습니다. 호스트별 파일 문법은 adapter에 남습니다. Core는 Store가 소유한
운영 기록에서 만든 현재 `ValidatedAgentSession`만 받습니다.

## 연결 검증

`init`, `connection status`, `connection verify`는 정규 검증 type의 check와 action을
사용해 typed command report 하나를 만듭니다. JSON과 text renderer는 이 보고서를
소비하고 binary 종료 처리는 typed 집계 상태를 읽습니다. Rendering이 병렬 상태 트리를
다시 만들거나 자체 출력을 parsing하지 않습니다.

`connection status`는 활성 probe를 실행하거나 파일, 보고서, 관찰, timestamp를 쓰지
않고 현재 파일과 Store 관찰을 읽습니다. `connection verify`는 현재 adapter와 관리 구성을
검사하고 허용된 로컬 probe를 실행한 뒤 실제 managed-host 및 Guard 관찰을 읽고 Store
담당 경로로 보고서를 최대 하나 commit합니다. 호스트 실행 파일을 hash하거나 릴리스 인증
카탈로그를 조회하거나 권한 receipt를 발급하거나 host 활동을 꾸며 내거나 managed-host
agent session을 만들지 않습니다. 권위 있는 관리 runtime/project session은 관리 MCP
lifecycle 처리에서만 기록합니다.

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
세 connection report 명령에서 `dry_run`은 작업 boolean이며 집계 결과는 3상태로
유지됩니다. `--json`은 typed 결과를 한 번 직렬화합니다. 사람용 text, log,
diagnostic metadata를 권한 상태로 다시 parse하지 않습니다.

## 경계

- CLI는 Core와 Store에 의존하며 Core는 CLI에 의존하지 않습니다.
- Codex별 구성은 adapter에 남습니다.
- 어떤 명령도 네트워크 전송을 시작하지 않습니다.
- 비대화형 명령은 사용자 판단을 제출하지 않습니다.
- Client와 host version 관찰은 diagnostic일 뿐 권한 credential이 아닙니다. 릴리스
  주장은 별도의 정확한 6셀 증거 흐름에 남습니다.

## 관련 경로

- [소스 지도](source-map.md)
- [요청 lifecycle](request-lifecycle.md)
- [Agent Connection](../reference/agent-connection.md)
- [MCP 전송](../reference/mcp-transport.md)
- [테스트 전략](testing-strategy.md)
