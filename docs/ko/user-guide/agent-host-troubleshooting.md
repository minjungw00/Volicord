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
volicord connection status codex --repo "<repo>" --json
```

진단을 없애려고 구성, Runtime Home 데이터, 저장소를 삭제하지 않습니다. 재현 가능한
실패를 전달할 때 JSON 출력을 보존하되 자격 증명이나 비공개 payload는 포함하지 않습니다.

## 안정 Finding Code 사용

`volicord doctor --json`에서는 `findings[].code`와 `findings[].actions[].code`를
확인합니다. Connection status 또는 verification JSON에서는 `root_cause_ids`를 읽고
`findings`에서 같은 ID를 찾은 뒤 최상위 `actions[].code`와
`actions[].root_cause_ids`를 사용합니다. 영속 finding ID, 값이 있으면 runtime-session ID,
namespaced diagnostic code를 함께 보존합니다. 영어 summary, SQLite 메시지, 경로 문구,
stderr 발췌로 실패를 분류하지 않습니다.

Code 계열에 따라 집중 복구 경계를 선택합니다.

| Code 계열 | 복구 경계 |
|---|---|
| `platform.*` | 지원 플랫폼 cell로 옮기거나 필수 플랫폼 관찰을 복구합니다. |
| `runtime_home.*` | Action이 이름 붙인 대로 절대 Runtime Home을 고치고, 누락 Registry를 초기화하고, permission을 복구하거나 경로 경계를 분리합니다. |
| `installation.*` | 실행 가능한 현재 Volicord build를 복구합니다. 있으면 `action.installation.reinstall_current_build`를 사용합니다. |
| `managed_config.*` | 같은 지원 `init` 복구를 실행합니다. Finding은 정적 환경 값이나 argument를 노출하지 않습니다. |
| `store.sqlite.busy`, `store.sqlite.locked` | Database transaction을 잡고 있는 프로세스를 끝내거나 중지한 뒤 재시도합니다. |
| `store.schema.mismatch`, `store.integrity.corruption_failure` | 호환 build와 담당자가 승인한 명시적 복원 또는 재초기화 경로를 사용합니다. Schema table을 직접 편집하지 않습니다. |
| `guard.*` | Guard 설치를 복구하거나 typed action이 이름 붙인 정확한 미관찰 phase를 실행합니다. |
| `trust.repository.not_trusted` | Codex에서 정확한 Product Repository를 승인합니다. |
| `revision.integration.stale` | 이미 적용한 구성 변경 뒤 Codex를 다시 불러옵니다. |
| `revision.observation.mismatch` | 현재 revision에 대해 검증을 다시 실행합니다. |

결정적인 TOML drift, schema mismatch, read-only storage, Runtime Home permission 실패라는
이유만으로 Codex를 재시작하지 않습니다. 먼저 해당 원인을 복구합니다.
`internal.unexpected_failure`는 더 좁은 담당 매핑이 없었다는 뜻이며 산문을 보고 추측할
권한을 주지 않습니다.

## Finding 또는 Runtime Session 하나 조사

Concise, verbose, JSON 출력에 나온 정확한 식별자를 사용합니다.

```sh
volicord diagnostics show "<finding-id>"
volicord diagnostics show "<finding-id>" --json
volicord diagnostics session "<runtime-session-id>"
volicord diagnostics session "<runtime-session-id>" --json
```

이 명령은 한도가 있는 Registry 조회입니다. 사람용 형태와 JSON 형태는 같은 root ID와
typed fact를 담습니다. 식별자가 없으면 `observation_state=absent`인 typed failed report를
반환합니다. 빈 finding이나 빈 session을 관찰했다는 뜻이 아닙니다. SQLite를 scan하거나
다른 식별자를 만들지 말고 선택한 Runtime Home과 정확한 ID를 확인합니다.

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

JSON 출력으로 활성 검증을 다시 실행하고 root ID부터 확인합니다.

```sh
volicord connection verify codex --repo "<repo>" --json
```

`root_cause_ids`와 같은 `findings[].id`를 찾은 뒤 해당 finding의 `code`, typed
`facts.data`, `causes`, correlation, action을 확인합니다. 적용되는 경우 실패한 check에는
`details.self_test.diagnostic_code`, `failure_stage`, `finding_id` 같은 stage별 detail도
남습니다. 종료 code, timeout, 누락 도구, stderr 발췌와 같은 제한된 Registry 사실을
확인하거나 전달할 때 finding ID를 함께 보존합니다.

`mcp.protocol.unsupported_revision`이면 `attempted_client_name`과
`attempted_client_version`, `requested_revision`, `production_supported_revisions`를
비교합니다. 일반 concise 출력에도 이 값과 blocked `required_tools`, `tool_round_trip`
check가 나옵니다. 일반 inspection 단계로 바꾸지 말고
`action.mcp.use_supported_protocol_revision`을 사용합니다. Verbose 출력에서는 requested,
selected, negotiated revision을 구분하고 실제 MCP peer `clientInfo`와 PATH executable
probe도 구분합니다.

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
