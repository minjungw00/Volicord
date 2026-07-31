# CLI 작업 흐름

이 가이드는 지원 관리 CLI의 오래 유지될 구현 흐름을 설명합니다. 정확한 명령 동작은
[관리 CLI](../reference/admin-cli.md)가 담당합니다.

## 소유권 지도

| 단계 | 구현 책임 |
|---|---|
| 선언과 introspection | `volicord-command-model`이 완전한 Clap tree, 명령 DTO, value enum, 문법 validator, 공개/숨김 분류, 정규 synopsis, 명령 경로 순회, 그 tree에서 도출하는 typed 정규 invocation builder를 담당합니다. |
| parse와 normalize | command model이 알 수 없거나 누락되거나 충돌하는 입력을 거부하고 `volicord-cli`에 명령 DTO를 제공합니다. |
| context 해결 | Runtime Home, 정규 Product Repository, project, Agent Connection을 선택합니다. |
| plan | 읽기 전용 검사가 정확한 파일과 Store 변경안을 만듭니다. |
| validate | 관리 구성, Connection, session, 저장소, policy를 검사합니다. |
| commit | 담당 문서가 정의한 원자적 filesystem/Store 경계를 사용합니다. |
| project와 render | `volicord-user-action-presentation`이 typed `Cli*` UserAction inbox model, JSON Schema, command-model 기반 resolution path를 담당합니다. `volicord-cli`는 이 model에서 terminal text를 직접 렌더링하거나 typed JSON 문서 하나를 직렬화합니다. |

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
list는 명령 보고서 상태에 의존하지 않는 집중 list projection을 유지하지만 각
membership summary는 선택한 status와 같은 현재 평가기에서 만듭니다.

`connection status`는 활성 probe를 실행하거나 파일, 보고서, 관찰, timestamp를 쓰지
않고 현재 파일과 Store 관찰을 읽습니다. 현재 평가 서비스 하나가 정확한 Runtime Home,
Agent Connection, 선택한 Connection Project membership, 현재 integration revision,
호출자가 제공한 평가 timestamp를 담당합니다. 이 서비스는 현재 관리 구성, runtime
session, 선택한 프로젝트의 Guard, policy, trust, repository fact를 적격한 영속 활성 검증
근거와 함께 조립합니다. 영속 보고서는 근거 입력이며 현재 집계 상태 cache가 아닙니다.
등록, 근거, 구성, membership, 프로젝트 Store, Guard, revision 획득 실패는 명령 handler가
소비하는 폐쇄형 typed unavailable 결과로 유지합니다.

`connection list`는 Connection마다 요청 범위 평가 context 하나를 만들고 가능한
Connection 범위 입력을 재사용하며, filter된 membership을 invocation timestamp 하나로
독립적으로 평가합니다. Repository filter는 현재 평가보다 먼저 실행합니다. 사용할 수
없는 membership 하나는 성공한 membership 옆에 표시하며 Runtime Home Registry 전체
열거가 실패하면 명령을 종료합니다. 어떤 context도 invocation 뒤에 남지 않습니다.

`connection verify`는 현재 adapter와 관리 구성을 검사하고 허용된 로컬 probe를 실행한 뒤
실제 managed-host 및 Guard 관찰을 읽고 Store 담당 경로로 보고서를 최대 하나
commit합니다. Executable path와 version은 diagnostic probe 사실입니다. 권위 있는 관리
runtime/project session은 관리 MCP lifecycle 처리에서만 기록하며, CLI self-test는
`session_source=cli_preflight`를 기록하므로 managed-host 호출을 승인할 수 없습니다.
명령 handler는 좌표와 출력 mode를 선택하고 typed 평가 결과를 소비한 뒤 check나 activation
상태를 다시 만들지 않고 최종 표시를 수행합니다.

## 프로젝트와 정책 흐름

Project 명령은 등록된 정규 Git 작업 트리를 해결합니다. Policy apply는 plan, 엄격한
검증, 원자적 commit을 사용합니다. 두 명령 계열 모두 표시 이름에서 권한을 추론하거나
알 수 없는 저장 값을 복구하지 않습니다.

CLI crate는 headline, section, field, 중첩 record, bullet, 반복 collection item,
action hint, yes/no 및 none/count 값, compact 또는 verbose detail을 위한 의미 중립
사람용 presentation 어휘를 담당합니다. 이 어휘는 간격, 들여쓰기, control 문자를
안전하게 표시하는 text, 마지막 newline 하나를 담당하지만 프로젝트, 정책, Core,
Store, MCP, Guard, host 또는 제품 의미를 소유하지 않습니다. 각 명령별 투영이 제공할
fact, label, 빈 상태 문장, count, action을 선택합니다.

`project current`와 `project list`는 Store가 검증한 typed 프로젝트 record를 이
사람용 primitive로 직접 투영합니다. 목록은 정규 Store 순서를 보존하고 반복 record를
사용하므로 긴 값을 전용 field 줄에 완전하게 표시합니다. 두 명령의 `--json` 경로는
완전한 typed 프로젝트 record를 별도로 직렬화하며 사람용 출력을 JSON을 거쳐 변환하지
않습니다. 명령은 의미 있는 verbose 투영이 있을 때만 verbose 출력을 제공합니다.

## UserAction 흐름

`inbox`는 Core에 adapter-neutral pending fact를 요청합니다. `volicord-types`가 의미
`UserActionResolutionForm`을 도출하고 공유 UserAction presentation이 이를
`CliUserActionInboxResponse`, typed channel availability, tagged request-specific
capture path로 투영합니다. Available path의 command는 이를 parsing하는 같은 Clap
선언에서 경로와 option spelling을 얻는 typed command-model invocation입니다.
`inbox resolve`는 저장 choice 또는 evidence observation 하나를
`volicord.resolve_user_action`으로 제출합니다. Text rendering은 typed CLI model을
직접 사용하고 `--json`은 그 model을 한 번 직렬화합니다. MCP adapter는 요청을
만들거나 재개할 수 있지만 이 해결 경로를 호출하거나 CLI inbox presentation을
사용할 수 없습니다.

Guard prompt 관찰은 CLI 답이 되지 않습니다. 손상된 저장 요청이나 resolution은 기본
form 대신 영속 데이터 오류로 실패합니다.

## 조정

`changes reconcile`은 공개 Core 메서드로 경로를 잡고 미해결 finding, 해결 결과,
사용자 행동 경로, 닫기 상태, 다음 행동을 표시합니다. 저장소 관찰 불가 상태는 별도
Guard 진단으로 남으며 path finding으로 합성하지 않습니다. 정확한 관찰과 해결
동작은 [저장소 관찰](../reference/repository-observation.md)과
[`volicord.reconcile_changes`](../reference/api/method-reconcile-changes.md)가
담당합니다.

## 진단과 출력

`doctor`, status, preflight는 읽기 전용 사실을 모아 이름 붙은 다음 동작을 보고합니다.
Connection report 명령에서 `dry_run`은 작업 boolean이며 집계 결과는 3상태로 유지됩니다.
`--json`은 typed 결과를 한 번 직렬화합니다. 사람용 text, log,
diagnostic metadata를 권한 상태로 다시 parse하지 않습니다.

## 경계

- `volicord-command-model`은 Clap에만 의존합니다. Core, Store, MCP, CLI
  렌더링, Runtime Home 구현, application service에 의존하지 않습니다.
- `volicord-user-action-presentation`은 command model과 shared type에 의존합니다.
  Core 정책, Store read, command 실행, terminal rendering, MCP envelope는 담당하지
  않습니다.
- `volicord-cli`는 command model, 공유 UserAction presentation, Core, Store에
  의존하며 이 크레이트들은 `volicord-cli`에 의존하지 않습니다.
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
