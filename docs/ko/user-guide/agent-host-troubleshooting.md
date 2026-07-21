# 에이전트 호스트 문제 해결

최초 릴리스의 Codex `record` 연결을 제한된 범위에서 복구할 때 이 가이드를
사용합니다. 복구 내내 선택한 Product Repository, 연결 범위, Runtime Home을
명시적으로 유지합니다.

## 변경 전 확인

먼저 읽기 전용 진단을 수집합니다.

```sh
volicord doctor
volicord project current
volicord connection list
volicord connection status codex --repo "<repo>"
```

진단을 없애려고 구성, Runtime Home 데이터, 저장소를 삭제하지 않습니다. 재현 가능한
실패를 전달할 때 JSON 출력을 보존하되 자격 증명이나 비공개 payload는 포함하지 않습니다.

## 명령을 사용할 수 없음

Codex를 시작한 환경의 `PATH`에 정확한 `volicord` 실행 파일이 있는지 확인합니다.
이미 실행 중인 Codex 프로세스는 이전 `PATH`를 유지할 수 있으므로 시작 환경을 고친 뒤
재시작합니다. 그다음 다시 실행합니다.

```sh
volicord doctor
volicord connection verify codex --repo "<repo>"
```

## 저장소 또는 연결이 모호함

의도한 Git 작업 트리에서 명령을 실행하거나 `--repo`를 명시합니다.
`volicord project current`와 `volicord connection list`로 저장 식별자를 찾습니다.
주변 디렉터리를 검색해 저장소를 선택하지 않습니다.

범위 선택자를 일관되게 유지합니다. 공유 연결은 `init`, `status`, `verify`,
`remove`에 `--shared`를 사용하고 개인 연결은 이를 뺍니다.

## `action_required`

`action_required`는 구조화된 다음 단계이며 설명 없는 성공이나 치명적 실패가 아닙니다.
이름 붙은 Codex 신뢰, 다시 불러오기, 구성, 저장 동작만 완료한 뒤 같은 검증 명령을
다시 실행합니다.

```sh
volicord connection verify codex --shared --repo "<repo>"
volicord connection status codex --shared --repo "<repo>"
```

검증 결과를 편집하거나 구성 파일만 보고 준비 상태를 추론하지 않습니다.

## MCP 사전 점검 실패

정확한 저장 연결과 프로젝트 식별자를 사용합니다.

```sh
volicord mcp --check --connection "<connection_id>" --project "<project_id>"
```

실패는 구조, binding, 실행 파일, 저장소, 외부 계약 문제를 식별해야 합니다. 그 문제를
고친 뒤 사전 점검을 다시 실행합니다. 다른 전송을 시작하거나 연결 binding을 우회하지
않습니다.

## MCP 자체 검사 실패

JSON 출력으로 활성 검증을 다시 실행하고 `mcp_server` 검사를 찾습니다.

```sh
volicord connection verify codex --repo "<repo>" --json
```

`details.self_test.diagnostic_code`, `failure_stage`, `finding_id`를 확인합니다.
Matrix 실패도 실패한 revision 또는 host fixture에 같은 세 필드를 표시합니다. 종료
code, timeout, 누락 도구, stderr 발췌와 같은 제한된 Registry 사실을 확인하거나 전달할
때 finding ID를 함께 보존합니다.

`stderr`는 제한된 맥락으로만 취급합니다. 자식 프로세스 문구에서 기계 판독 사유를
추론하거나 자격 증명을 보고서에 복사하지 않습니다. 안정적인 `process.*`, `mcp.*`,
`host.codex.*` code가 후속 산문 parsing 없이 원인을 식별합니다. 정확한 진단 참조 필드와
프로세스 제한은 [관리 CLI](../reference/admin-cli.md), MCP code 의미와 안전한 협상 사실은
[MCP 전송](../reference/mcp-transport.md)이 담당합니다.

`actual_mcp_peer_client_info.version`과 `path_executable_probe.version`이 다르면 먼저
활성 Codex 프로세스와 PATH를 확인합니다. 이 warning은 유용한 evidence지만 그 자체로
치명적 결과는 아닙니다. 보고할 때 한 version을 다른 version으로 바꾸지 않습니다.

## Codex에서 도구가 보이지 않음

Codex가 정확한 프로젝트를 신뢰하고 현재 `.codex/config.toml`을 다시 읽었는지
확인합니다. 관리 명령이 의도한 `volicord` 실행 파일과 Runtime Home을 가리키는지
확인합니다. 그런 다음 Codex 도구 목록에서 읽기 전용 `volicord.status`를 실행하고
관리 검증을 다시 수행합니다.

구성이 있다고 활성 도구 검색이 증명되지는 않습니다. 현재 세션에 도구가 계속 없다면
진단을 보존하고 같은 관리 연결에서 새 Codex 세션을 시작합니다.

## 대기 중인 UserAction

에이전트는 대기 요청을 만들거나 재개할 수 있지만 답할 수 없습니다. 로컬 CLI User
Channel로만 해결합니다.

```sh
volicord inbox --repo "<repo>"
volicord inbox resolve USER_ACTION_REQUEST_ID --choice CHOICE_ID --repo "<repo>"
```

CLI가 저장 요청이나 resolution을 손상으로 거부하면 데이터베이스를 편집하거나 답을
추측해 대신하지 않습니다. 기계 판독 실패를 보존하고 필요한 경우 폐기 가능한 개발
상태를 다시 만듭니다.

## 기록되지 않은 변경

Unrecorded Change는 제한된 관찰이며 actor 귀속이 아닙니다. 반환된 조정 동작을
따릅니다. Guard suppression은 담당 문서가 정의한 일치 경로만 제거할 수 있습니다.
`Unavailable` suppression 결과는 계속 표시해야 하며 비어 있는 성공으로 취급하면 안
됩니다.

## 관리 구성 복구

같은 지원 설정 의도를 다시 실행합니다.

```sh
volicord init --shared --host codex --repo "<repo>" --profile record
volicord connection verify codex --shared --repo "<repo>"
```

변경된 모든 파일을 검토합니다. 복구는 관련 없는 Codex 설정과 저장소 내용을 보존해야
합니다.

## 일부만 제거된 것처럼 보임

정확한 의도를 미리 보고 결과를 확인합니다.

```sh
volicord connection remove codex --shared --repo "<repo>" --dry-run
volicord connection remove codex --shared --repo "<repo>"
volicord connection list --repo "<repo>"
```

제거는 결과가 이름 붙인 Volicord 관리 경로에 대해서만 성공합니다. 명령 계약이 제거
대상으로 정하지 않은 권한 기록이나 관련 없는 구성의 보존은 일부 실패가 아닙니다.

## 보안 경계

Volicord는 협력적 로컬 권한 상태입니다. 쓰기 티켓은 파일시스템 권한이 아니며 연결
검증은 모델 준수 증명이 아니고 닫기 상태는 정확성, 배포, 사람 검토 증명이 아닙니다.
[보안](../reference/security.md)을 봅니다.
