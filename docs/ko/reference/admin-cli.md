# 관리 CLI 참조

이 문서는 지원되는 로컬 관리 명령 표면을 담당합니다. 공개 Core API 메서드 동작은
[API 메서드](api/methods.md), 관리 stdio 동작은 [MCP 전송](mcp-transport.md)이
담당합니다.

<a id="surface-stability"></a>
## 표면 안정성

| 표면 | 안정성 |
|---|---|
| 여기에 나열한 명령과 선택자 | `stable` |
| stable 명령으로 나열하지 않은 pre-1.0 추가 표면 | `beta` |
| 사람용 형식과 진단 문구 | `diagnostic` |
| 생성 Codex 구성 세부사항 | `internal` |

라벨은 [문서 정책](../maintain/documentation-policy.md#surface-stability-labels)을 사용합니다.

## 명령 모델

`volicord`는 로컬 관리/bootstrap 실행 파일이며 장기 실행 네트워크 서비스가 아닙니다.
최초 릴리스는 `host=codex`, `profile=record`, `personal` 또는 `shared` 연결 범위만
받습니다.

지원 명령 그룹은 다음과 같습니다.

```text
volicord init
volicord status
volicord doctor
volicord diagnostics
volicord policy
volicord connection
volicord project
volicord mcp
volicord export authority-bundle
volicord changes reconcile
volicord inbox
```

알 수 없는 명령, 제거된 선택자, 추가 positional, 충돌하는 option은 사용법 오류입니다.
관리 명령 이름은 공개 API 메서드 이름이 아닙니다.

<a id="doctor-diagnostic-states"></a>
## Doctor 진단 상태

`volicord doctor --json`은 missing, invalid, unavailable, corrupt, stale 관찰을
서로 구분합니다. `states.installation_profile` 값은 `present`, `missing`, `invalid`,
`unavailable`, `corrupt`, `unsupported_contract`, `not_checked`, `unknown` 중 하나이며
이 조건들을 하나의 값으로 합치지 않습니다. `project_policy_authority` finding은
`authority_missing`,
`authority_corrupt`, `authority_unavailable`, `authority_unsupported_contract`,
`managed_file_missing`, `managed_file_invalid`, `managed_file_unavailable`,
`managed_file_stale` 중 하나를 사용합니다. `managed_file_stale`은 두 복사본이 각각
유효하지만 기준 fingerprint가 서로 다르다는 뜻입니다. 복구 동작을 제안할 수는 있지만
doctor가 기본 policy를 대신 넣거나 어느 authority 복사본도 다시 쓰지 않습니다.
제한된 project-policy 감사는 `scan_state: complete` 또는
`scan_state: bounded_incomplete`를 보고합니다. 점검한 page에 finding이 없어도 제한 때문에
감사를 완료하지 못했으면 warning이며 통과로 보고할 수 없습니다.

<a id="runtime-home-selection"></a>
## Runtime Home 선택

명령이 `--home PATH`를 받으면 정확한 Runtime Home을 선택합니다. 그 밖에는
`VOLICORD_HOME`과 플랫폼 기본값 순서로 선택합니다. 비어 있거나 상대 경로이거나 잘못
됐거나 충돌하는 값은 저장소 접근 전에 실패합니다. Product Repository를 Runtime Home으로
사용하지 않습니다.

<a id="volicord-agent-install"></a>
<a id="agent-host-setup-and-init"></a>
## Codex 설정

```sh
volicord init --host codex --repo "<repo>" --profile record
volicord init --shared --host codex --repo "<repo>" --profile record
```

첫 명령은 개인 연결, `--shared`는 프로젝트 소유 공유 연결을 선택합니다. `init`은
정확한 관리 binding을 계획, 검증, 적용하고 남은 Codex 신뢰, 다시 불러오기, 검증 동작을
보고합니다. `--dry-run`은 파일시스템과 저장소를 변경하지 않습니다.

설정은 관련 없는 Codex와 저장소 내용을 보존합니다. 복구는 현재 정규 입력으로 같은
의도를 다시 실행합니다. 제거는 일치하는 Volicord 관리 내용만 삭제합니다.

<a id="project-commands"></a>
## 프로젝트 명령

```text
volicord project use [PATH]
volicord project current
volicord project list
volicord project rename NAME [--repo PATH]
volicord project forget [PATH|NAME]
```

프로젝트 선택은 등록된 정규 Git 작업 트리를 해결합니다. 모호한 선택은 실패하며 cwd와
표시 이름으로 identity를 조용히 만들지 않습니다.

<a id="project-workflow-policy-commands"></a>
## 정책 명령

```text
volicord policy show --repo PATH
volicord policy validate --file PATH
volicord policy apply --repo PATH --file PATH
```

검증은 효과가 없습니다. Apply는 담당 문서의 plan과 원자적 commit 경계를 사용하며 알 수
없는 필드와 잘못된 값은 commit 전에 실패합니다.

## Agent Connection 명령

```text
volicord connection add [codex] [--repo PATH] [--shared] [--read-only] [--dry-run]
volicord connection list [--repo PATH]
volicord connection status [codex] [--repo PATH] [--shared]
volicord connection verify [codex] [--repo PATH] [--shared]
volicord connection mode [codex] workflow|read-only [--repo PATH] [--shared]
volicord connection remove [codex] [--repo PATH] [--shared] [--dry-run]
```

호스트를 생략하면 현재 맥락이 모호하지 않을 때만 사용합니다. 명시적으로 받는 유일한 값은
`codex`입니다.

<a id="agent-connection-result-states"></a>
### 연결 결과 상태

| 상태 | 의미 |
|---|---|
| `complete` | 선택한 동작이 끝나고 담당 문서가 요구하는 모든 현재 검사를 통과했습니다. |
| `action_required` | 오래 유지되는 설정이 있을 수 있지만 이름 붙은 사용자 또는 Codex 동작이 남았습니다. |
| `failed` | 동작이 실패했고 기계 판독 원인을 보고합니다. |

`complete`는 릴리스 셀 통과, Core 호출 권한, host attestation, 활성 Codex 세션의
도구 노출 증명이 아닙니다.

`volicord init`, `volicord connection status`,
`volicord connection verify`는 `ConnectionCommandReport` 하나를 직렬화합니다.

```yaml
ConnectionCommandReport:
  operation: init | status | verify
  dry_run: bool
  status: complete | action_required | failed
  setup_applied: bool                    # init에만 사용
  runtime_home: string
  connection:
    id: string
    host: codex
    scope: user | project
    profile: record
    mode: read_only | workflow
    repository: string
    config_target: string
  checks: ConnectionCheck[]
  actions: ConnectionAction[]
  planned_changes: PlannedConnectionChange[] # dry-run에만 사용
  limits: string[]
```

이 보고서에는 집계 상태 하나와 check/action 트리 하나만 있습니다. `states`, 중첩 검증
보고서나 상태, Guard 상태 트리, host gate나 승인 필드, summary card, primary action,
두 번째 disclosure 트리를 추가하지 않습니다. JSON은 적용되지 않는 선택 필드를 null
placeholder로 채우지 않고 생략합니다. `limits`에는 협력적 보장 한계를 한 번만 둡니다.

`setup_applied`는 설정 변경과 운영 검증을 구분합니다. `init` 적용이 성공하면 뒤의 로컬
또는 운영 check 때문에 `status=failed`가 되더라도 `setup_applied=true`입니다. Dry run은
`setup_applied=false`와 `planned_changes`를 보고하며 `status=dry_run`을 직렬화하지
않습니다. 계획 변경이나 host action이 남으면 `action_required`, 둘 다 없으면
`complete`입니다.

`checks`와 `actions`는 정규
[`ConnectionVerificationReport`](agent-connection.md#connection-verification-report)의
구성원 type과 순서를 사용합니다. JSON과 사람용 출력은 같은 typed command report를
표시합니다. 사람용 출력은 check를 묶어 보여 줄 수 있지만 상태나 action을 다시 계산하지
않습니다.

`volicord connection status`는 읽기 전용입니다. 현재 관리 구성, 신뢰, Guard audit,
통합 revision, managed-host session 관찰을 마지막 활성 executable/MCP server probe와
함께 projection합니다. Process를 시작하지 않으며 파일, timestamp, 보고서, action,
관찰, 데이터베이스 row를 바꾸지 않습니다.

`volicord connection verify`는 `codex`를 활성 탐색하고 version 명령을 실행한 뒤
`volicord mcp --check`와 CLI 전용 MCP self-test를 실행합니다. Self-test는
`initialize`, `tools/list`, 필수 도구 검증, 안전한 읽기 전용
`volicord.list_projects` 호출을 수행합니다. 그런 다음 현재 managed-host 관찰을 읽고 정규
보고서 하나만 영속합니다. CLI self-test는 managed-host session이 아닙니다.

`volicord init`과 `volicord connection add`는 뒤의 운영 check가 실패하더라도 이미 쓴
유효한 설정을 유지합니다. Codex를 사용할 수 없거나 self-test가 실패했다는 이유로 관리
구성을 rollback하지 않습니다. Managed-host 관찰이 아직 없는 새 유효 설정은
`action_required`이며, 관찰을 얻는 데 필요한 typed reload/first-use action을 담습니다.
Codex version이나 executable digest를 eligibility allowlist로 사용하지 않습니다.

Action은 pending 및 failed check에서 직접 만드는 정렬되고 중복 제거된 목록입니다. 다시
불러오기와 최초 사용 지시는 실제 Codex 활동을 관찰해야 한다고 명시합니다.
`guard_files` check가 통과했다면 Guard 파일 재설치를 지시하지 않습니다.

<a id="external-host-configuration"></a>
## 관리 Codex 구성

개인 연결은 사용자 소유 관리 Codex 구성만 씁니다. 공유 연결은 지원되는 프로젝트 소유
Codex 항목을 쓰고 머신 로컬 경로를 내장하지 않은 채 `VOLICORD_HOME`을 전달합니다.
정확한 관리 entry marker, drift, 복구, launch 맥락, uninstall 경계는
[Agent Connection](agent-connection.md)이 담당합니다. 구성 marker는 협력적 launch
경로를 선택할 뿐 credential이나 identity 증거가 아닙니다.

## MCP 명령

```text
volicord mcp --stdio --connection <connection_id> [--project <project_id>]
volicord mcp --stdio --discover-repository --host codex
volicord mcp --check --connection <connection_id> [--project <project_id>]
```

이 명령은 관리 stdio만 노출합니다. 정확한 framing, lifecycle, 도구 목록, 응답
projection은 [MCP 전송](mcp-transport.md)이 담당합니다.

<a id="diagnostics"></a>
## Diagnostics

```text
volicord diagnostics session [--session SESSION_ID] [--json]
volicord diagnostics workflow-metrics --repo PATH --json
```

Diagnostics 출력은 제한된 비권한 operability 데이터입니다. JSON 보고서는 현재
diagnostics SQL에서 파생한 정확한 `canonical_schema_digest`와
`contract_id=volicord.sqlite.diagnostics`로 로컬 저장소를 식별합니다. 숫자 schema
version을 노출하거나 그 값으로 dispatch하지 않습니다. Diagnostics 읽기는 저장소를
만들거나 프로젝트 권한 상태를 열거나 state version을 전진시키거나 evidence 또는
assurance, 닫기 준비 상태를 바꾸거나 UserAction을 해결하지 않습니다.

<a id="authority-bundle-export"></a>
## 권한 번들 내보내기

```text
volicord export authority-bundle --output "<path>" --repo "<repo>"
```

내보내기는 담당 문서가 정의한 권한 번들을 새 경로나 명시적으로 허용된 출력 경로에
씁니다. 프로젝트 권한 상태를 바꾸거나 릴리스 증거를 만들지 않습니다.

## 변경 조정

```text
volicord changes reconcile --repo "<repo>"
```

조정은 공개 `volicord.reconcile_changes` 동작을 로컬 관리 흐름에 투영합니다. Guard
suppression 실패는 명시적으로 남으며 `Unavailable` 결과를 비어 있는 성공으로 표시하지
않습니다.

<a id="user-channel-commands"></a>
## User Channel 명령

```sh
volicord inbox --repo "<repo>"
volicord inbox resolve USER_ACTION_REQUEST_ID --choice CHOICE_ID --repo "<repo>"
```

`inbox`는 최초 릴리스의 유일한 UserAction 해결 채널입니다. 로컬 사용자 소유 표면에
저장 typed form을 표시하고 명시적인 choice 또는 evidence observation 하나를
`volicord.resolve_user_action`에 제출합니다. MCP 에이전트는 대기 요청을 만들거나
재개할 수 있지만 이 해결 경로를 실행할 수 없습니다.

저장 요청과 resolution은 엄격한 typed record입니다. 손상되거나 알 수 없거나 섞이거나
잘못된 저장 값은 영속 데이터 실패 분류로 실패하며 CLI가 기본값을 넣거나 복구하거나
답을 추측하지 않습니다.

## 출력과 종료 상태

`--json`은 stdout에 JSON 문서 하나만 씁니다. 기본 산문은 사람용이며 자동화에서
파싱하면 안 됩니다. `complete`, `action_required`, 유효한 모든 dry run은 `0`으로
종료합니다. Typed `failed` 운영 보고서는 `1`, 사용법 오류는 `2`로 종료합니다. 실패한
JSON 운영 보고서는 stdout 문서 하나만 쓰고 stderr는 비워 둡니다. 실패한 사람용 운영
보고서는 stdout에 표시합니다. 예상하지 못한 런타임 또는 직렬화 오류는 stderr를 사용하고
`1`로 종료합니다. 종료 상태는 표시 문자열이나 다시 parsing한 JSON이 아니라 typed report
상태로 선택합니다.

<a id="noninteractive-approval-behavior"></a>
## 비대화형 동작

비대화형 실행은 프로젝트 신뢰를 수락하거나 UserAction을 해결하거나 민감 동작을
승인하거나 호스트가 표시한 질문에 답하지 않습니다. 구조화된 다음 동작을 반환하고 판단을
사용자에게 남깁니다.

## 관련 담당 문서

- [Agent Connection](agent-connection.md)
- [MCP 전송](mcp-transport.md)
- [런타임 경계](runtime-boundaries.md)
- [API UserAction 스키마](api/schema-user-action.md)
- [저장 효과](storage-effects.md)
