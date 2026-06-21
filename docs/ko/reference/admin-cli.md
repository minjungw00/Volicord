# 관리 CLI 참조

이 문서는 로컬 `harness` 관리/부트스트랩 CLI 계약을 담당합니다. 이 CLI는 `Harness Runtime Home`을 초기화하고, 로컬 프로젝트와 접점을 등록하며, Agent Integration Profile을 관리하고, 지원되는 코딩 에이전트 호스트의 호스트 설정을 설치하고, 호스트 통합 상태를 검증합니다. 이 명령들은 공개 하네스 API 메서드가 아닙니다.

이 문서는 공개 API 메서드 동작, API 스키마, 접근 등급 값 의미, 저장소 기록 배치, 보안 보장, Core 권한 의미, MCP stdio 전송 동작을 정의하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- `harness` 명령 이름, 명령줄 인자, 기본값, stdout/stderr 처리, 프로세스 종료 코드
- `harness` 관리 명령의 Runtime Home 경로 선택
- 관리용 프로젝트와 접점 등록 기본값
- Agent Integration Profile 명령 동작
- 통합 프로젝트 멤버십 명령 동작
- Codex, Claude Code, generic export를 위한 호스트 설치, 상태, 검증, 제거 명령 동작
- 설정 결과 상태, dry-run 동작, 기계 판독 출력, 비대화식 승인 동작
- 선택적 저장소 지침 적용, 상태, 제거 명령 동작
- `baseline-workflow` 로컬 등록 프로필 확장
- 관리 명령과 공개 하네스 API 메서드 사이의 경계

이 문서는 담당하지 않습니다.

- 공개 하네스 API 메서드: [API 메서드](api/methods.md)
- `access_class` 값의 API 값 의미: [API 값 집합](api/schema-value-sets.md#access-class-values)
- Agent Integration Profile, Host Installation, 확인된 접점 맥락, 행위자 출처, 역량 선언 의미: [에이전트 통합](agent-integration.md)
- 런타임 데이터 경계 의미와 `Product Repository` 파일 경계 예외: [런타임 경계](runtime-boundaries.md)
- MCP 프로세스 시작, stdio 프레이밍, 와이어 동작, 응답 래핑, 사전 점검 내부 동작, 종료: [MCP 전송](mcp-transport.md)
- 저장소 기록 배치, SQLite DDL, 일반 저장소 마이그레이션 정의, Core 권한 의미, 보안 보장 의미

## 명령 모델

`harness`는 로컬 관리/부트스트랩 실행 파일입니다. 장기 실행 서버가 아니며 공개 하네스 API를 직접 노출하지 않습니다.

지원되는 기준 명령은 아래와 같습니다.

```text
harness --help
harness --version
harness init [--runtime-home-id ID]
harness project register --project-id ID --repo-root PATH [--status active]
harness project list
harness surface register --project-id ID --surface-id ID [--surface-instance-id ID] [--kind KIND] [--name NAME] [--interaction-role agent|user_interaction] [--access-class ACCESS_CLASS ...] [--profile baseline-workflow] [--capability-profile JSON]
harness surface list --project-id ID
harness agent install --host codex|claude_code|claude-code|generic --scope user|project|local|export --project-id ID [--repo-root PATH] [--integration-id ID] [--default-project-id ID] [--server-name NAME] [--surface-id ID] [--surface-instance-id ID] [--mcp-command PATH] [--runtime-home PATH] [--export-path PATH|--export-dir PATH] [--guidance none|codex|claude_code|claude-code|both] [--output text|json] [--dry-run] [--allow-repository-write] [--replace-managed]
harness agent project add --integration-id ID --project-id ID [--repo-root PATH] [--default] [--runtime-home PATH] [--output text|json] [--dry-run]
harness agent project remove --integration-id ID --project-id ID [--runtime-home PATH] [--output text|json] [--dry-run]
harness agent project default set --integration-id ID --project-id ID [--runtime-home PATH] [--output text|json] [--dry-run]
harness agent project default clear --integration-id ID [--runtime-home PATH] [--output text|json] [--dry-run]
harness agent status --integration-id ID [--runtime-home PATH] [--output text|json]
harness agent verify --integration-id ID [--installation-id ID] [--runtime-home PATH] [--output text|json]
harness agent uninstall --integration-id ID [--installation-id ID] [--runtime-home PATH] [--output text|json] [--dry-run] [--allow-repository-write] [--remove-managed]
harness agent guidance apply --integration-id ID --project-id ID --host codex|claude_code|claude-code [--runtime-home PATH] [--output text|json] [--dry-run] [--allow-repository-write] [--replace-managed]
harness agent guidance status --integration-id ID --project-id ID [--runtime-home PATH] [--output text|json]
harness agent guidance remove --integration-id ID --project-id ID [--host codex|claude_code|claude-code] [--runtime-home PATH] [--output text|json] [--dry-run] [--allow-repository-write] [--remove-managed]
```

종료 코드와 스트림 동작:

- 성공한 명령은 성공 출력을 stdout에 쓰고 종료 코드 `0`으로 끝납니다.
- `action_required`는 성공한 관리 결과이며 종료 코드 `0`으로 끝납니다.
- `partial_failure`, `failed`, 런타임 오류, 저장소 오류, 사전 점검 실패, 검증 실패, 충돌은 종료 코드 `1`로 끝납니다.
- 사용법 오류는 진단을 stderr에 쓰고 종료 코드 `2`로 끝납니다.
- `harness --version`은 stdout에 `harness <version>`을 쓰며 Runtime Home 해석을 요구하지 않습니다.
- `--output json`은 stdout에 JSON 문서 정확히 하나를 쓰며 사람용 설명을 섞지 않습니다.
- 오류는 기존 CLI 종료 코드 모델에 따라 stderr 진단으로 남습니다.

지원하지 않는 것:

- CLI에는 `serve`, `server`, `connect` 명령이 없습니다.
- 공개 `harness agent` 계약에는 `--yes` 플래그가 없습니다. 포괄적 yes/assume-yes 스위치는 이 계약이 요구하는 명시적 플래그를 대신하면 안 됩니다.
- 관리 명령은 공개 하네스 API 메서드가 아니며 공개 메서드 목록에 추가되면 안 됩니다.

<a id="runtime-home-selection"></a>
## Runtime Home 선택

`harness` 관리 CLI는 아래 Runtime Home 경로 해석 규칙을 사용합니다. `harness-mcp` 프로세스 환경과 현재 MCP Runtime Home 경로 해석은 [MCP 전송](mcp-transport.md#process-environment)이 담당합니다.

해석 순서:

1. 명령이 정의한 경우 명령별 `--runtime-home`
2. `HARNESS_HOME`
3. `HOME`, `USERPROFILE`, `HOMEDRIVE`와 `HOMEPATH` 결합 순서의 첫 번째 비어 있지 않은 홈 소스에 `.harness`를 붙인 경로

규칙:

- `HARNESS_HOME`이 존재하지만 비어 있으면 오류입니다.
- 설정, 설치, 검증, 마이그레이션 계획을 수행하는 명령에서 명령별 `--runtime-home` 값은 절대 경로여야 합니다.
- 상대 경로 `HARNESS_HOME`은 그 경로가 존재하지 않아도 프로세스의 현재 작업 디렉터리를 기준으로 해석합니다.
- `harness init`은 선택된 Runtime Home 레지스트리를 만들거나 검증할 수 있습니다.
- 다른 관리 명령은 선택된 Runtime Home에 요청 작업에 필요한 기록이 있어야 합니다.

## 호스트와 범위 지원

지원되는 호스트와 범위 값:

| `--host` | 지원되는 `--scope` 값 | 기준 대상 |
|---|---|---|
| `codex` | `user`, `project` | 사용자 설정은 Codex 사용자 `config.toml`입니다. 프로젝트 설정은 연결된 `Product Repository` 안의 `.codex/config.toml`입니다. |
| `claude_code` | `local`, `project`, `user` | local과 user 설정은 Claude Code 사용자 소유 설정 대상입니다. 프로젝트 설정은 연결된 `Product Repository` 안의 `.mcp.json`입니다. CLI는 `claude-code`를 별칭으로 받을 수 있지만 저장 기록은 `claude_code`를 사용합니다. |
| `generic` | `export` | 직접 설치를 주장하지 않고 명시적 MCP 설정 객체를 내보냅니다. |

범위 규칙:

- `project`와 `local` 범위는 연결된 `Product Repository` 하나만 허용합니다.
- `user` 범위는 명시적으로 추가된 여러 프로젝트를 허용할 수 있지만, `harness agent install`에는 그래도 하나 이상의 명시적 `--project-id`가 필요합니다.
- `generic export`는 명시적 설정 내보내기만 쓰거나 출력하며, 호스트 로드를 주장하는 Host Installation을 만들지 않습니다.
- 지원하지 않는 호스트/범위 조합은 사용법 오류입니다.

호스트 설정 형태:

- Codex 설치는 `command`, `args = ["--integration", "<integration_id>"]`, 선택적 `env.HARNESS_HOME`을 가진 `[mcp_servers.<server_name>]`과 동등한 MCP 서버 테이블을 씁니다.
- Claude Code 설치는 `command`, `args`, 선택적 `env.HARNESS_HOME`을 가진 `mcpServers.<server_name>` MCP 서버 항목을 씁니다.
- Generic export는 같은 command, args, environment 값을 호스트 중립 JSON 객체로 출력합니다.
- user와 local 범위는 정식으로 확인된 `harness-mcp` 실행 파일 경로나 명시적이고 유효한 절대 경로를 사용할 수 있습니다.
- 프로젝트 범위 공유 설정은 호스트 환경의 `PATH`에서 찾을 수 있는 이식 가능한 `harness-mcp` 명령을 사용해야 합니다. 개인 빌드 경로, 홈 디렉터리 경로, 개인 `HARNESS_HOME`을 넣으면 안 됩니다.
- Generic export는 명시적으로 선택한 절대 명령 경로를 내보낼 수 있지만, 내보낸 설정은 사용자가 관리하는 호스트가 로드하고 검증하기 전까지 계속 `action_required`입니다.
- 새 기준 호스트 설정은 `HARNESS_PROJECT_ID`, `HARNESS_SURFACE_ID`, `HARNESS_SURFACE_INSTANCE_ID`를 요구하면 안 됩니다.

호스트 신뢰 경계:

- 설정 설치와 호스트가 MCP 서버를 로드하고 노출하는 것은 구분됩니다.
- Codex 프로젝트 범위 설정은 로드되기 전에 Codex 프로젝트 신뢰가 필요할 수 있습니다.
- Claude Code 프로젝트 범위 MCP 설정은 로드되기 전에 프로젝트 MCP 승인이 필요할 수 있습니다.
- 하네스는 호스트 신뢰, 프로젝트 신뢰, 프로젝트 MCP 승인, OAuth, 또는 그와 비슷한 사용자 통제 호스트 동작을 우회할 수 있다고 주장하면 안 됩니다.

<a id="agent-setup-result-states"></a>
## 에이전트 설정 결과 상태

에이전트 명령군은 아래 설정 결과 상태를 사용합니다.

| 상태 | 의미 |
|---|---|
| `complete` | 오래 유지되는 통합 상태가 있고, 관리되는 호스트 설정이 존재하며 예상 관리 지문과 일치하고, 호스트별 loadability gate가 충족되고, 필요한 신뢰나 승인 동작이 남아 있지 않고, 통합 사전 점검이 성공하고, MCP 초기화가 성공하고, `tools/list`가 필요한 도구를 노출합니다. |
| `action_required` | 오래 유지되는 통합 상태와 호스트 설정은 있지만 호스트 신뢰, 프로젝트 승인, OAuth, reload, restart, 또는 그와 비슷한 사용자 통제 호스트 동작이 남아 있습니다. |
| `partial_failure` | 일부 오래 유지되는 관리 동작은 성공했지만 이후 설치, 검증, 호스트 대상, 롤백, 정리 단계가 실패했습니다. 결과는 적용된 효과, 롤백된 효과, 잔류 효과를 식별해야 하며 다시 실행할 수 있어야 합니다. |
| `failed` | 요청한 설치나 검증이 사용할 수 있는 오래 유지되는 통합 상태 또는 호스트 설정을 만들지 못했습니다. |

`dry_run`은 출력 상태이며 설정 결과 상태가 아닙니다.

성공한 `harness-mcp --check --integration <integration_id>`만으로는 전체 호스트 통합을 `complete`로 설명하면 안 됩니다. 이는 MCP 프로세스의 시작 검증일 뿐입니다.

호스트별 상태 규칙:

- Codex project 범위는 Codex 프로젝트 신뢰를 확인할 수 없는 동안 `action_required`로 남습니다.
- Claude Code project 범위는 프로젝트 MCP 승인이 대기 중인 동안 `action_required`로 남습니다.
- 거절됨, 없음, 변경됨, 사용할 수 없음, 알 수 없음 호스트 상태는 `complete`가 되면 안 됩니다.
- Generic export는 하네스가 외부 호스트가 내보낸 설정을 로드했다는 사실을 증명할 수 없으므로 `action_required`로 남습니다.

## `harness agent install`

`harness agent install`은 Agent Integration Profile을 만들거나 재사용하고, 요청한 프로젝트를 명시적으로 허용하며, 호스트 설정을 설치하거나 내보내고, 호스트를 확인할 수 있을 때 결과를 검증합니다.

필수 옵션:

- `--host`
- `--scope`
- `--project-id`

선택 동작:

- `--integration-id`는 기존 통합 또는 새 통합에 원하는 식별자를 선택합니다.
- `--default-project-id`는 기본값을 설정하며 허용된 프로젝트를 이름으로 가리켜야 합니다.
- `--server-name`은 호스트 MCP 서버 이름을 선택합니다. 생략하면 CLI는 `integration_id`에서 안정적인 기본값을 파생합니다. 기본값은 `harness-`로 시작하고, ASCII 영문자, 숫자, 하이픈, 밑줄에 맞게 정리되며, 필요하면 해시를 붙여 짧게 만듭니다.
- `--repo-root`는 호스트 대상이 project/local 범위로 그곳에 쓸 때 연결된 `Product Repository`를 검증합니다.
- `--surface-id`와 `--surface-instance-id`는 통합 접점 바인딩을 선택합니다. 생략하면 CLI가 안정적인 불투명 식별자를 생성하고 보고합니다.
- `--mcp-command`는 명시적 명령 경로가 허용되는 범위에서 `harness-mcp` 실행 파일을 선택합니다. user와 local 범위는 이 옵션이 지정되면 존재하는 절대 경로를 요구합니다. project 범위는 `PATH`의 `harness-mcp`를 사용합니다. generic export는 명시적 명령을 지정할 때 절대 경로를 요구합니다.
- `--runtime-home`은 호스트 설정에 `HARNESS_HOME`으로 쓸 Runtime Home 경로를 선택합니다.
- `--guidance none|codex|claude_code|both`는 선택한 프로젝트의 선택적 `Product Repository` 지침을 미리 보여 주고 적용합니다. 생략하거나 `none`이면 지침을 쓰지 않으며, 비대화식 지침 쓰기에는 여전히 `--allow-repository-write`가 필요합니다.

설치 규칙:

- 명령은 Runtime Home의 모든 프로젝트에 접근을 부여하면 안 됩니다.
- 검증이 `complete`가 되려면 각 허용 프로젝트에 대해 통합 접점을 등록, 재사용, 또는 검증해야 합니다.
- 기본 프로젝트는 허용되어 있어야 합니다.
- project/local 범위는 둘 이상의 프로젝트가 허용되면 실패합니다.
- user 범위는 나중에 `harness agent project add`로 프로젝트를 더 추가할 수 있습니다.
- 호스트 설정 쓰기는 관리 소유 마커 또는 동등한 관리 지문을 사용합니다.
- 같은 호스트 대상과 서버 이름에 대한 기존 비관리 설정은 충돌입니다. `--replace-managed`는 소유 마커가 맞는 이전 관리 블록에만 적용됩니다.
- 프로젝트 범위 호스트 설정 쓰기는 비대화식 실행에서 `--allow-repository-write`를 요구합니다.
- `--dry-run`은 [Dry-run과 기계 판독 출력](#dry-run)이 정한 zero-write 계약에 따라 모든 저장소 및 파일 동작을 미리 보여 줍니다.

검증:

- 설치된 설정에서 호스트를 시작할 수 있으면 검증은 MCP 초기화와 `tools/list` 탐색을 시도해야 합니다.
- 설정은 설치되었지만 호스트 신뢰나 승인이 로드를 막으면 결과는 `failed`가 아니라 `action_required`입니다.
- `harness-mcp --check`는 통과했지만 MCP 초기화나 도구 탐색이 성공하지 않았다면 결과는 `complete`가 될 수 없습니다.
- 하네스가 직접 시작한 MCP handshake는 Codex 또는 Claude Code가 서버를 로드, 신뢰, 승인, 노출했다는 사실을 증명하지 않습니다.

## 통합 프로젝트 멤버십 명령

`harness agent project add`는 기존 통합에 허용 프로젝트 하나를 추가합니다.

규칙:

- `--integration-id`와 `--project-id`는 필수입니다.
- 프로젝트는 선택된 Runtime Home에 이미 등록되어 있어야 합니다.
- 프로젝트를 추가해도 inactive, 무효, 실행 부적격 프로젝트가 실행 시점에 사용 가능해지는 것은 아닙니다.
- `--default`는 통합 기본값을 추가된 프로젝트로 설정합니다.
- `project` 또는 `local` 범위 통합에 두 번째 프로젝트를 추가하는 것은 충돌입니다.
- 이 명령은 호스트 설정을 다시 쓰지 않습니다. 접근 철회와 추가는 레지스트리 변경입니다.

`harness agent project remove`는 기존 통합에서 허용 프로젝트 하나를 제거합니다.

규칙:

- 아직 `default_project_id`인 프로젝트를 제거하려면 먼저 기본값을 지우거나 바꿔야 하며, 그렇지 않으면 실패해야 합니다.
- 설치된 통합에서 유일한 프로젝트를 제거하는 것은 명령이 프로젝트가 다시 추가될 때까지 통합을 실행할 수 없다고 보고할 때만 허용됩니다.
- 멤버십 제거는 프로젝트 상태, 접점 기록, Core 기록, 호스트 설정, 지침 파일을 삭제하지 않습니다.

`harness agent project default set`은 기존 통합의 기본 프로젝트를 설정합니다.

규칙:

- `--integration-id`와 `--project-id`는 필수입니다.
- 프로젝트는 해당 통합에 이미 허용되어 있어야 합니다.
- 현재 기본값을 다시 설정하는 작업은 멱등적입니다.
- 이미 허용된 다른 프로젝트로 설정하면 호스트 설정을 다시 쓰지 않고 기본값만 바꿉니다.

`harness agent project default clear`는 기존 통합의 기본 프로젝트를 지웁니다.

규칙:

- `--integration-id`는 필수입니다.
- 이미 기본값이 없을 때 clear를 반복해도 멱등적입니다.
- 현재 기본 프로젝트는 기본값을 바꾸거나 지우기 전까지 제거할 수 없습니다.
- 기본값을 지운 뒤에는 마지막 프로젝트 멤버십도 제거할 수 있습니다.
- 허용 프로젝트가 없는 통합은 저장된 상태로 남을 수 있지만, 프로젝트가 다시 추가되기 전까지 실행할 수 없습니다.

## 상태와 검증 명령

`harness agent status`는 호스트 담당 문서가 가벼운 상태 점검을 정의하지 않는 한 호스트를 시작하지 않고 레지스트리와 호스트 인벤토리 상태를 보고합니다.

최소 보고 항목:

- `integration_id`
- 활성화 상태
- `surface_id`
- `surface_instance_id`
- 가용성과 기본값 상태를 포함한 허용 프로젝트
- Host Installation 기록
- `last_verified_status`
- 지침 상태

`harness agent verify`는 하나의 통합 또는 하나의 설치에 대한 검증 상태를 갱신합니다.

선택 규칙:

- `harness agent verify --installation-id <id>`는 정확히 그 Host Installation 하나를 검증하며, 그 설치가 다른 통합에 속하면 실패합니다.
- `--installation-id`가 없으면 `--integration-id`에 연결된 모든 Host Installation을 선택해 검증합니다.
- 선택된 각 설치는 자기 자신의 `host_kind`, `host_scope`, `config_target`, 저장소 루트, 명령, 인자, 환경, 관리 지문, 호스트별 상태 점검을 사용합니다.
- 한 설치의 결과가 다른 설치의 검증 상태를 덮어쓰면 안 됩니다. 설치별 출력은 `installation_id`와 결과 `last_verified_status`를 식별해야 합니다.

검증해야 하는 항목:

- 통합이 존재하고 활성화되어 있습니다.
- 허용 프로젝트를 읽고 사용 가능 또는 사용 불가로 분류합니다.
- 기본 프로젝트가 있으면 허용되어 있고 사용 가능해야 합니다.
- 직접 설치가 대상을 소유한다면 호스트 설정 대상이 존재하고 관리 지문과 여전히 일치해야 합니다.
- `harness-mcp --check --integration <integration_id>`가 성공합니다.
- MCP 초기화가 성공합니다.
- `tools/list`가 공개 하네스 도구 아홉 개와 `harness.list_projects`를 노출합니다.

검증은 선택된 각 Host Installation의 `last_verified_status`에 `complete`, `action_required`, `partial_failure`, `failed` 중 하나를 기록합니다.

집계 결과 상태:

| 선택된 설치 결과 | 집계 명령 상태 |
|---|---|
| 선택된 모든 설치가 `complete` | `complete` |
| 하나 이상의 선택된 설치가 `action_required`이고 `partial_failure` 또는 `failed`가 없음 | `action_required` |
| 하나 이상의 선택된 설치가 `partial_failure`이고 `failed`가 없음 | `partial_failure` |
| 하나 이상의 선택된 설치가 `failed` | `failed` |

선택된 설치 중 하나라도 `complete`가 아니면 집계 상태는 절대 `complete`가 아닙니다.

## 제거

`harness agent uninstall`은 하네스가 관리하는 호스트 설정을 제거하고, 선택적으로 통합의 레지스트리 인벤토리를 비활성화하거나 제거합니다.

규칙:

- 제거는 적용 전에 관리 파일 편집을 미리 보여 줘야 합니다.
- 일치하는 하네스 소유 마커 또는 관리 지문을 가진 블록, 파일, 항목만 제거해야 합니다.
- `Product Repository`, 프로젝트 상태, Core 기록, Runtime Home, 아티팩트 저장소, 관련 없는 호스트 설정을 삭제하면 안 됩니다.
- 프로젝트 범위 파일 편집은 비대화식 실행에서 `--allow-repository-write`를 요구합니다.
- 관리되는 `Product Repository` 지침을 비대화식으로 제거하려면 `--remove-managed`가 필요합니다.
- 사용자가 호스트 파일을 이미 바꾼 경우 제거는 관련 없는 내용을 제거하지 말고 충돌을 보고해야 합니다.

## 저장소 지침 명령

저장소 지침은 선택 사항입니다. 명시적 사용자 승인 뒤에만 설치되며 강제 메커니즘이 아닙니다.

지원되는 지침 대상:

- Codex: `AGENTS.md`의 하네스 관리 블록.
- Claude Code: `.claude/rules/` 아래의 하네스 관리 Markdown 규칙 파일.

규칙:

- `harness agent guidance apply`는 `--integration-id`, `--project-id`, `--host`, 그리고 비대화식 실행에서 `--allow-repository-write`를 요구합니다.
- 명령은 정확한 파일 경로와 관리 내용을 미리 보여 줘야 합니다.
- 비관리 충돌을 감지해야 하며, `--replace-managed`는 일치하는 이전 관리 내용에만 적용되어야 합니다.
- 관리 지침에는 하네스 관리와 통합을 식별하는 소유 마커가 들어가야 합니다.
- `harness agent guidance status`는 관리 마커 상태를 읽고 지침이 없음, 있음, 변경됨, 충돌 중 어느 상태인지 보고합니다.
- `harness agent guidance remove`는 일치하는 관리 내용만 제거하며 비대화식 실행에서는 `--remove-managed`를 요구합니다.
- 지침은 하네스 MCP 서버 지침과 저장소 지침이 도구 선택을 도울 수 있지만 모델 동작을 보장할 수 없다고 밝혀야 합니다.

정확한 `Product Repository` 쓰기 경계는 [런타임 경계](runtime-boundaries.md#explicit-integration-files-in-product-repositories)가 담당합니다.

<a id="dry-run"></a>
## Dry-run과 기계 판독 출력

`--dry-run`은 영속 변경 없이 계획, 검증, 충돌 감지, 호스트 대상 렌더링, 출력 형태 만들기를 수행합니다.

Dry-run이 하지 않는 것:

- `Harness Runtime Home` 생성
- SQLite 데이터베이스 생성 또는 수정
- SQLite WAL 또는 SHM 파일 생성
- 레지스트리 또는 프로젝트 상태 마이그레이션 적용
- 프로젝트, 접점, 통합, 멤버십, 설치, 검증 상태 행 등록 또는 갱신
- 호스트 설정 파일 생성, 수정, 제거
- 지침 파일을 포함한 `Product Repository` 파일이나 디렉터리 생성, 수정, 제거
- generic export 파일 생성, 수정, 제거
- `harness-mcp --check` 호출
- MCP 초기화 또는 도구 탐색 수행

선택된 Runtime Home이 스키마 버전 1 레지스트리를 가지고 있으면 dry-run은 마이그레이션 없이 이를 검사할 수 있고, apply 중 마이그레이션이 일어날 것이라고 보고할 수 있습니다. 레지스트리를 마이그레이션하거나, 새 레지스트리 테이블을 만들거나, 프로젝트 상태 데이터베이스를 만들거나, 마이그레이션 메타데이터를 쓰면 안 됩니다.

Text 출력은 사람이 읽을 수 있어야 하며 각 리소스 작업을 `created`, `reused`, `updated`, `removed`, `skipped`, `conflict`, `planned` 중 하나로 식별해야 합니다.

<a id="setup-output"></a>
JSON 성공 출력은 아래 최상위 키를 갖습니다.

```text
status
runtime
project
integration
allowed_projects
installations
guidance
host
verification
actions
effects
action_required
warnings
```

필수 JSON 값:

- `status`: `complete`, `action_required`, `partial_failure`, `failed`, 또는 `dry_run`
- `host_kind`: `codex`, `claude_code`, 또는 `generic`
- `host_scope`: `user`, `project`, `local`, 또는 `export`
- `last_verified_status`: `not_verified`, `complete`, `action_required`, `partial_failure`, 또는 `failed`

JSON 출력은 관리 CLI 출력이지 공개 하네스 API 응답 스키마가 아닙니다.

`partial_failure` 출력:

- 사람용 text 출력은 적용된 효과, 롤백된 효과, 잔류 효과를 각각 식별해야 합니다.
- JSON 출력은 같은 사실을 기계 판독 가능한 항목으로 노출해야 합니다.
- 각 효과 항목에는 대상 위치 또는 기록 식별자, 효과 분류, 대상 재실행 또는 검사에 충분한 세부사항이 있어야 합니다.
- 각 잔류 효과에는 롤백을 수행하지 않은 이유 또는 롤백 실패 이유와 권장 운영자 동작이 있어야 합니다.
- `registry changes may remain` 같은 일반 문장은 정확한 잔류 효과 항목과 함께 제공되지 않는 한 충분하지 않습니다.

<a id="noninteractive-approval-behavior"></a>
## 비대화식 승인 동작

비대화식 명령은 명시적 사용자 승인이 없으면 프롬프트를 표시하지 말고 실패해야 합니다.

규칙:

- 프로젝트 범위 호스트 설정이나 저장소 지침을 쓰거나, 교체하거나, 제거하는 모든 명령에는 `--allow-repository-write`가 필요합니다.
- `--replace-managed`는 소유 마커나 관리 지문이 일치하는 하네스 관리 내용에만 적용됩니다.
- `--remove-managed`는 하네스 관리 내용의 안전한 제거에만 적용됩니다.
- 포괄적 셸 승인, 쓰기 승인, 호스트 신뢰 결정, 민감 동작 승인은 `Write Authorization`이 아니며 이 CLI 계약이 요구하는 명시적 관리 플래그를 대신하지 않습니다.
- 호스트 신뢰, 프로젝트 신뢰, 프로젝트 MCP 승인, OAuth, restart, reload 동작은 계속 사용자 통제 호스트 동작이며 CLI가 대신 제공할 수 없습니다.

## 프로젝트 등록

`harness project register --project-id ID --repo-root PATH [--status active]`는 로컬 `Product Repository`를 선택된 Runtime Home에 등록합니다.

규칙:

- `--project-id`는 필수입니다.
- `--repo-root`는 필수입니다.
- `--status`의 기본값은 `active`입니다.
- 기준 등록은 `status=active`를 받습니다.
- `--repo-root`는 프로젝트 등록에 쓰는 로컬 저장소 루트를 식별합니다.
- 선택된 Runtime Home과 `--repo-root`는 등록이 기록되기 전에 [Runtime Home/Product Repository 분리 계약](runtime-boundaries.md#runtime-home-product-repository-separation)을 만족해야 합니다.

`harness project list`는 선택된 Runtime Home의 등록된 프로젝트를 나열합니다.

`harness project list`는 레지스트리 수준 검사입니다. Runtime Home/Product Repository 분리 계약을 위반하는 이력 프로젝트 기록을 진단 목적으로 보여 줄 수 있습니다. 목록에 보인다는 사실만으로 그 기록이 프로젝트 상태 데이터베이스 접근, 접점 관리, Core 실행, 설정 재사용, MCP 시작에 적격해지지는 않습니다.

`Product Repository`와 `Harness Runtime Home`의 구분을 포함한 런타임 위치 경계는 [런타임 경계](runtime-boundaries.md#runtime-home-product-repository-separation)가 담당합니다.

## 접점 등록

`harness surface register`는 등록된 프로젝트에 로컬 접점 인스턴스 하나를 기록합니다.

접점 등록과 목록 조회는 프로젝트 등록이 [런타임 경계](runtime-boundaries.md#runtime-home-product-repository-separation)가 담당하는 Runtime Home/Product Repository 분리 계약에 따라 계속 적격해야 합니다.

기본값:

- `surface_kind` 기본값은 `cli`입니다.
- `interaction_role` 기본값은 `agent`입니다.
- 기본 접근은 `read_status`뿐입니다.
- 생성되는 Runtime Home ID와 생성되는 `surface_instance_id` 값은 구현이 생성하는 불투명 값입니다.

등록 프로필:

- `--profile baseline-workflow`는 명시적으로 선택해야 합니다.
- `baseline-workflow`는 `read_status`, `core_mutation`, `write_authorization`, `artifact_registration`, `run_recording`으로 확장됩니다.
- 명시 접근 등급과 프로필에서 파생된 접근 등급은 결정적이고 중복 제거된 합집합을 이룹니다.
- `baseline-workflow` 프로필은 `artifact_read`를 포함하지 않습니다.

`user_interaction` 제약:

- `user_interaction`에는 `core_mutation`이 필요합니다.
- `user_interaction`은 `read_status`와 `core_mutation`만 가질 수 있습니다.
- 따라서 `baseline-workflow`는 `user_interaction` 접점에 유효하지 않습니다.

MCP 등록 지침:

- 코딩 에이전트 MCP 통합에는 `harness agent install`을 선호합니다. 이 명령은 통합 프로필, 프로젝트 멤버십, 호스트 설치, 프로젝트별 접점 바인딩을 함께 만들거나 검증합니다.
- 낮은 수준의 `harness surface register --kind mcp`는 명시적 관리 복구나 사용자 지정 자동화를 위해 계속 사용할 수 있습니다.

접근 등급 값 이름과 의미는 [API 값 집합](api/schema-value-sets.md#access-class-values)이 담당합니다. 접점 등록 의미와 확인된 맥락 경계는 [에이전트 통합](agent-integration.md)이 담당합니다.

## 접점 목록

`harness surface list --project-id ID`는 선택된 Runtime Home에서 한 프로젝트의 등록된 접점을 나열합니다.

규칙:

- `--project-id`는 필수입니다.
- 목록 출력은 진단용 등록 정보입니다.
- 목록 출력은 권한을 부여하거나, 로컬 도달 가능성을 증명하거나, 담당 결과가 반환한 확인된 접점 맥락을 대신하지 않습니다.

<a id="local-mcp-setup-orchestration"></a>
## 호환성: `harness setup local-mcp`

`harness setup local-mcp`는 레거시 고정 프로젝트 MCP 설정을 위한 기준 범위 밖 호환 명령입니다. 새 설정 예시와 Host Installation 기록은 `harness agent install`을 사용해야 합니다.

<a id="interactive-setup-frontend"></a>

호환성 규칙:

- 이 명령은 계속 관리 오케스트레이션이며 공개 하네스 API 메서드가 아닙니다.
- 이 명령의 대화형 프런트엔드는 같은 기준 범위 밖 레거시 setup 경로를 위한 호환 UI입니다.
- 명시적으로 호환성을 위해 호출된 경우에만 레거시 고정 프로젝트 설정을 생성할 수 있습니다.
- 결과가 호환성 출력임을 식별해야 합니다.
- 직접 Codex 또는 Claude Code 설치의 기준 모델로 사용하면 안 됩니다.

<a id="host-neutral-configuration"></a>
### 호환성 호스트 중립 설정

`harness-agent.mcp.json` 같은 레거시 호스트 중립 설정 조각과 `harness-agent` 같은 서버 이름은 호환성 자료일 뿐입니다. 기준 범위에서 요구되는 이름이 아닙니다.

## 관리 경계

관리 CLI는 로컬 리소스를 초기화하고 등록할 수 있습니다. 그 자체로 공개 하네스 API 메서드를 만들지 않으며 Core 권한, `Write Authorization`, 증거 충분성, 닫기 준비 상태, 사용자 소유 판단, 수락, 잔여 위험 수락, 아티팩트 권한, 보안 보장을 만들지 않습니다.

담당 문서 경로:

- 공개 메서드 목록과 메서드 경로: [API 메서드](api/methods.md).
- 공통 요청/응답 스키마: [API 코어 스키마](api/schema-core.md).
- 접근 등급 값: [API 값 집합](api/schema-value-sets.md#access-class-values).
- Agent Integration Profile, 프로젝트 멤버십, 접점과 행위자 맥락 의미: [에이전트 통합](agent-integration.md).
- MCP 프로세스 동작: [MCP 전송](mcp-transport.md).
- 런타임 위치와 저장소 쓰기 경계: [런타임 경계](runtime-boundaries.md).
