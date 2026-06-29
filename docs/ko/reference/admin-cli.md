# 관리 CLI 참조

이 문서는 로컬 `volicord` 관리/부트스트랩 CLI 계약을 담당합니다. CLI는
`Volicord Runtime Home`을 마련하고, 저장소 루트에서 프로젝트를 등록하며, 사용자가
내부 ID를 다루지 않아도 되도록 Agent Connection을 관리하고, 로컬 `User Channel`
명령 경로를 제공하며, generic MCP 설정을 내보내고, 설정 또는 연결 진단을
보고합니다. 이 명령들은 공개 Volicord API 메서드가 아닙니다.

이 문서는 공개 API 메서드 동작, API 스키마, 저장소 기록 배치, 보안 보장, Core
권한 의미, MCP stdio 전송 동작을 정의하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- `volicord` 명령 이름, 명령줄 인자, 기본값, stdout/stderr 처리, 프로세스 종료 코드
- setup 시점의 Runtime Home, 설치 프로필, 실행 파일 링크, MCP 명령 선택
- 저장소 루트 프로젝트 감지와 관리 프로젝트 명령
- 지원 호스트 통합을 위한 Agent Connection 명령 동작
- generic MCP 설정 내보내기 동작
- 로컬 `User Channel` 명령 이름과 명령 출력
- 진단 상태, 필요한 사용자 동작, dry-run 동작, JSON 출력, 비대화식 동작
- 관리 명령, 로컬 `User Channel` 명령, 공개 Volicord API 메서드 사이의 경계

이 문서는 담당하지 않습니다.

- 공개 Volicord API 메서드: [API 메서드](api/methods.md)
- Agent Connection, Connection Projects, 연결 모드, 연결 의도, 행위자 출처 의미:
  [Agent Connection](agent-connection.md)
- 런타임 데이터 경계 의미와 `Product Repository` 파일 경계 예외:
  [런타임 경계](runtime-boundaries.md)
- MCP 프로세스 시작, stdio 프레이밍, 와이어 동작, 응답 래핑, 종료:
  [MCP 전송](mcp-transport.md)
- 저장소 기록 배치, SQLite DDL, 일반 저장소 마이그레이션 정의, Core 권한 의미,
  보안 보장 의미

## 명령 모델

`volicord`는 로컬 관리/부트스트랩 실행 파일입니다. 장기 실행 서버가 아닙니다.
`volicord user` 명령군은 선택된 Core 메서드 위에 있는 로컬 `User Channel` CLI
어댑터입니다. 이 명령 이름은 공개 Volicord API 메서드가 아니라 관리 CLI 명령으로
남습니다.

지원되는 기준 명령은 아래와 같습니다.

```text
volicord --help
volicord --version
volicord setup [--home PATH] [--link-bin PATH] [--mcp-command PATH] [--json]
volicord doctor [--json]
volicord connect [HOST] [--repo PATH] [--shared|--global] [--read-only] [--dry-run] [--json]
volicord connections [--repo PATH] [--json]
volicord connection status [HOST] [--repo PATH] [--shared|--global] [--json]
volicord connection verify [HOST] [--repo PATH] [--shared|--global] [--json]
volicord connection mode [HOST] workflow|read-only [--repo PATH] [--shared|--global] [--json]
volicord connection remove [HOST] [--repo PATH] [--shared|--global] [--dry-run] [--json]
volicord project use [PATH] [--json]
volicord project current [--json]
volicord project list [--json]
volicord project rename NAME [--repo PATH] [--json]
volicord project forget [PATH|NAME] [--json]
volicord export mcp-config [--output PATH] [--repo PATH] [--read-only] [--json]
volicord user status [--repo PATH] [--task active|ID] [--json]
volicord user judgments [--repo PATH] [--task active|ID] [--json]
volicord user judgment show INDEX_OR_ID [--repo PATH] [--json]
volicord user judgment answer INDEX_OR_ID OPTION_INDEX_OR_ID [--repo PATH] [--note TEXT] [--json]
```

지원되는 `HOST` 값은 `codex`와 `claude-code`입니다. `HOST`를 생략하면 명령은
모호하지 않은 현재 호스트 맥락을 사용할 수 있습니다. 호스트를 모호하지 않게
식별할 수 없으면 명령은 지원되는 호스트 값을 이름 붙인 진단 동작과 함께
실패합니다.

종료 코드와 스트림 동작:

- 성공한 명령은 성공 출력을 stdout에 쓰고 종료 코드 `0`으로 끝납니다.
- `action_required`는 성공한 관리 결과이며 종료 코드 `0`으로 끝납니다.
- `failed`, 런타임 오류, 저장소 오류, 검증 실패, 충돌은 종료 코드 `1`로
  끝납니다.
- 사용법 오류는 진단을 stderr에 쓰고 종료 코드 `2`로 끝납니다.
- `volicord --version`은 stdout에 `volicord <version>`을 쓰며 Runtime Home 해석을
  요구하지 않습니다.
- `--json`은 stdout에 JSON 문서 정확히 하나를 쓰며 사람용 설명을 섞지 않습니다.
- 오류는 CLI 종료 코드 모델에 따라 stderr 진단으로 남습니다.

지원하지 않는 것:

- CLI에는 `serve`, `server`, daemon 명령이 없습니다.
- 관리 명령은 공개 Volicord API 메서드가 아니며 공개 메서드 목록에 추가되면
  안 됩니다.
- 텍스트 모드 사용자 흐름은 내부 프로젝트 ID, Agent Connection ID, 호스트 설정
  키, 프로토콜 래퍼, 저장된 registry 필드를 사용자가 입력하도록 요구하면 안 됩니다.

<a id="runtime-home-selection"></a>
## setup과 Runtime Home

`volicord setup`은 로컬 설치 프로필을 마련합니다. 선택된 Runtime Home을 만들거나
검증하고, 이후 관리 명령, Agent Connection, export, MCP 프로세스 흐름이 사용할
명령 경로를 저장합니다. Setup은 Runtime Home 경로나 MCP 명령 위치를 직접 선택하는
유일한 기준 명령입니다.

인자:

| 인자 | 의미 |
|---|---|
| `--home PATH` | `Volicord Runtime Home`을 선택합니다. 생략하면 플랫폼 기본 로컬 런타임 위치를 사용합니다. 선택한 경로는 프로젝트 상태를 사용하기 전에 Runtime Home/Product Repository 분리 계약을 만족해야 합니다. |
| `--link-bin PATH` | 가능할 때 `volicord`와 `volicord-mcp` 둘 다에 대한 사용자 선택 명령 링크를 설치하거나 갱신합니다. 명령은 각 대상 경로를 보고하고 안전하지 않은 교체를 거절합니다. |
| `--mcp-command PATH` | 관리 호스트 설정과 generic export가 `volicord-mcp`를 시작할 때 사용할 명령을 저장합니다. 찾기 순서는 명시적 `--mcp-command PATH`가 제공된 경우 그 값, 실행 중인 `volicord` 실행 파일 옆의 `volicord-mcp`, `PATH`의 명령 순서입니다. |
| `--json` | 기계 판독 출력을 선택합니다. |

Setup 효과:

- Runtime Home registry를 만들거나 검증합니다.
- Runtime Home 식별 정보와 설치 프로필 메타데이터를 기록합니다.
- 이후 `connect`, `doctor`, export, MCP 시작 흐름이 사용할 `volicord`와
  `volicord-mcp` 명령 위치를 기록합니다.
- 두 실행 파일 역할 모두에 대해 `--link-bin`으로 지정한 명령 링크를 갱신할 수 있습니다.
- 링크 디렉터리가 현재 프로세스의 `PATH`에 보이지 않으면 `PATH` 동작을 보고합니다.
  부모 셸 환경을 영구적으로 수정할 수는 없습니다.
- 별도의 프로젝트 또는 연결 명령이 저장소를 선택하기 전에는 프로젝트를 등록하지 않습니다.
- 공개 Volicord API 메서드를 만들거나 사용자 소유 판단을 기록하지 않습니다.

`volicord doctor`는 setup 프로필을 위한 읽기 중심 진단 명령입니다. Runtime Home
접근, registry 스키마, 설치 프로필 존재 여부, 저장된 명령 준비 상태, 링크
메타데이터가 있을 때의 명령 링크 또는 shim 준비 상태를 확인합니다. 지원 호스트
감지는 연결 검증이 보고할 문제로 표시합니다. 프로젝트를 만들거나, 호스트 설정을
설치하거나, 연결 모드를 바꾸거나, 사용자 판단에 답하지 않습니다.

## 프로젝트 명령

프로젝트 명령은 저장소 루트를 사용자 대상 프로젝트 식별자로 사용합니다. 내부
`project_id`는 저장소와 출처 데이터이며 텍스트 모드 명령은 이를 요구하지 않습니다.

저장소 루트 감지:

- `--repo PATH`와 `PATH` 인자는 프로젝트 조회 전에 해석됩니다.
- 경로가 제공되지 않으면 명령은 프로세스의 현재 작업 디렉터리를 사용합니다.
- 감지된 저장소 루트는 선택한 경로를 포함하는 가장 가까운 지원 저장소 루트입니다.
  루트를 감지할 수 없으면 프로젝트가 필요한 명령은 `volicord project use PATH`를
  이름 붙인 진단 동작과 함께 실패합니다.
- Runtime Home과 `Product Repository` 경로는 [Runtime Home/Product Repository
  분리 계약](runtime-boundaries.md#runtime-home-product-repository-separation)을
  만족해야 합니다.

`volicord project use [PATH]`는 감지된 저장소 루트를 등록하거나 재사용합니다.
등록은 내부 프로젝트 ID, 사용자 대상 프로젝트 이름, Runtime Home 아래의 프로젝트
홈, 필요한 프로젝트별 상태를 만듭니다. 기본 프로젝트 이름은 저장소 디렉터리에서
파생하고 Runtime Home 안에서 필요하면 고유하게 만듭니다.

`volicord project current`는 현재 작업 디렉터리에서 감지된 프로젝트를 보고합니다.
프로젝트 등록을 만들지 않습니다.

`volicord project list`는 등록 프로젝트를 사용자 대상 이름, 저장소 루트, 상태,
진단 가용성과 함께 나열합니다.

`volicord project rename NAME [--repo PATH]`는 선택된 저장소의 사용자 대상 프로젝트
이름을 바꿉니다. 내부 프로젝트 ID, 저장소 루트, 프로젝트 홈, Core 상태는 바꾸지
않습니다.

`volicord project forget [PATH|NAME]`은 active Agent Connection 멤버십이나 담당
문서가 계속 주소 지정 가능해야 한다고 요구하는 프로젝트 상태를 고아로 만들지 않을
때만 선택된 프로젝트 등록을 제거합니다. 프로젝트를 잊는 동작은 `Product Repository`,
관련 없는 Runtime Home 데이터, 호스트 설정, 남아 있는 다른 등록이 소유하는
아티팩트 저장소, 보존되어야 하는 Core 권한 행을 삭제하면 안 됩니다.

## 연결 의도와 호스트

Agent Connection 설정은 낮은 수준의 호스트 설정 범위 이름 대신 연결 의도를
사용합니다.

| 의도 | 선택 방법 | 의미 |
|---|---|---|
| `personal` | 기본값 | 현재 사용자의 일반 로컬 흐름을 위한 사용자 소유 호스트 설정입니다. |
| `shared` | `--shared` | 선택된 `Product Repository` 안의 명시적 통합 파일로 저장되는 프로젝트 소유 또는 프로젝트 공유 호스트 설정입니다. |
| `global` | `--global` | 선택된 호스트의 사용자 전역 호스트 설정입니다. 프로젝트 접근은 계속 등록된 저장소 루트와 Connection Projects로 제한됩니다. |

`--shared`와 `--global`은 함께 사용할 수 없습니다. 둘 다 없으면 의도는
`personal`입니다.

연결 모드:

- `workflow`가 기본 모드입니다.
- `read-only`는 명시적으로 선택하며 Agent Connection을 통해 읽기와 프로젝트 탐색
  동작만 노출합니다.
- `volicord connection mode ... workflow|read-only`는 사용자가 호스트 설정을 직접
  편집하지 않아도 선택된 연결의 저장 모드를 바꿉니다.

내부 호스트 설정 키 `server_name`의 기본값은 `volicord`입니다. 일반 CLI 흐름은
서버 이름 옵션을 노출하지 않습니다. 생성된 호스트 설정에는 호스트가
`volicord-mcp`를 시작할 수 있도록 내부 연결 ID, 서버 이름, 명령 인자가 들어갈 수
있습니다. 이 값들은 사용자 권한 토큰이 아니며 텍스트 모드 명령 입력으로 요구되지
않습니다.

일반 `volicord connect` 명령은 MCP 명령 경로나 Runtime Home 경로를 다시 묻지 않고
해석된 Runtime Home에 저장된 프로필을 사용합니다. 개인, 로컬, 사용자 전체 호스트
설정은 그 Runtime Home을 `VOLICORD_HOME`으로 담을 수 있습니다. shared 프로젝트
호스트 설정은 개인 Runtime Home 경로를 포함하면 안 되며, 미래의 호스트 환경이
`PATH`로 해석해야 하는 명령 이름 `volicord-mcp`를 사용합니다.

<a id="volicord-agent-install"></a>
## Agent Connection 명령

연결 선택은 호스트, 의도, 저장소 루트를 사용합니다. 명령은 내부 연결 식별자를
파생하거나 조회합니다.

| 명령 | Runtime Home registry 효과 | 호스트 설정 효과 | 검증 효과 |
|---|---|---|---|
| `volicord connect` | 선택된 저장소 프로젝트를 등록하거나 재사용하고, 일치하는 Agent Connection을 만들거나 갱신하며, 연결 의도와 모드를 기록하고, 프로젝트가 Connection Projects에 들어 있음을 보장합니다. | 선택된 의도에 따라 `codex` 또는 `claude-code`용 관리 호스트 설정을 설치하거나 갱신합니다. | 관찰 가능한 곳에서 setup, 호스트 설정, MCP 시작, 초기화, `tools/list` 점검을 실행합니다. |
| `volicord connections` | 일치하는 Agent Connection과 연결 프로젝트를 읽습니다. | 호스트를 시작하지 않고 호스트 설정을 다시 쓰지 않습니다. | 저장된 검증 상태와 진단 검증 상태를 호스트 점검 없이 보고합니다. |
| `volicord connection status` | 선택된 Agent Connection 하나를 읽습니다. | 호스트를 시작하지 않고 호스트 설정을 다시 쓰지 않습니다. | 저장된 전체 검증 상태와 필요한 사용자 동작을 보고합니다. |
| `volicord connection verify` | 선택된 Agent Connection을 읽고 마지막으로 알려진 검증 상태를 갱신합니다. | 호스트 통합이 관찰 가능한 대상을 소유하면 관리 대상을 검사합니다. | 관찰 가능한 점검을 실행하고 결과 검증 상태를 저장합니다. |
| `volicord connection mode` | 선택된 연결 모드를 갱신합니다. | 모드를 반영하려면 호스트 항목을 다시 생성해야 하는 경우가 아니면 호스트 설정을 다시 쓰지 않습니다. | 모드 변경 뒤 진단을 보고합니다. |
| `volicord connection remove` | 선택된 Connection Projects 멤버십을 제거하고 소유 멤버십이 남지 않으면 Agent Connection을 제거합니다. | 소유권과 안전 점검이 허용할 때 일치하는 관리 호스트 설정만 제거합니다. | 프로젝트, Core 상태, Runtime Home, 아티팩트 저장소, 관련 없는 호스트 설정을 삭제하지 않습니다. |

규칙:

- `volicord connect`는 기본적으로 Runtime Home의 모든 프로젝트를 연결하면 안 됩니다.
- 선택 프로젝트는 항상 저장소 루트에서 해석되며 명령이 지속 프로젝트 등록을 필요로
  하면 자동 등록됩니다.
- shared 의도는 [런타임 경계](runtime-boundaries.md#explicit-integration-files-in-product-repositories)가
  허용하는 명시적 통합 파일만 쓸 수 있습니다.
- 같은 생성 호스트 대상의 기존 비관리 호스트 설정은 충돌입니다. 일치하는
  Volicord 관리 내용은 소유 명령으로만 갱신하거나 제거할 수 있습니다.
- 호스트 신뢰, 프로젝트 신뢰, 프로젝트 MCP 승인, OAuth, restart, reload, 그 밖의
  호스트 통제 동작은 계속 사용자 통제 호스트 동작입니다.

<a id="agent-connection-result-states"></a>
<a id="agent-setup-result-states"></a>
## 연결 결과 상태

Agent Connection 명령은 아래 결과 상태를 사용합니다.

| 상태 | 의미 |
|---|---|
| `not_verified` | 선택된 Agent Connection에 현재 기록된 검증 결과가 없습니다. 호스트가 실패했다는 증거가 아닙니다. |
| `complete` | 오래 유지되는 Agent Connection 상태가 있고, 관리 호스트 설정이 존재하며 예상 관리 지문과 일치하고, 필요한 호스트 로드 가능성 및 신뢰 게이트가 충족되고, MCP 시작이 성공하고, MCP 초기화가 성공하며, `tools/list`가 모드에 필요한 도구를 노출합니다. |
| `action_required` | 오래 유지되는 Agent Connection 상태와 호스트 설정은 있지만 호스트 신뢰, 프로젝트 승인, OAuth, reload, restart, 명령 링크 복구, setup 복구, 또는 그와 비슷한 사용자 통제 동작이 남아 있습니다. |
| `failed` | 요청한 명령이나 검증이 사용할 수 있는 오래 유지되는 Agent Connection 상태, 사용할 수 있는 호스트 설정, 또는 필요한 로컬 전제 조건을 만들지 못했습니다. |
| `dry_run` | 명령이 영속 변경 없이 계획된 동작을 보고했습니다. |

검증 출력은 점검과 사용자 동작을 일급 진단으로 만들어야 합니다. Text 출력은 전체
상태, 시도되었거나 차단된 각 점검, 필요한 경우 다음 사용자 동작을 보여 줘야 합니다.
JSON 출력은 진단 소비자를 위해 최상위 `status`, `checks`, `actions` 필드를 포함해야
합니다.

성공한 `volicord-mcp` 시작 점검만으로는 Agent Connection을 `complete`로 설명하면
안 됩니다. 이는 MCP 프로세스의 시작 검증일 뿐입니다.

## generic MCP 설정 내보내기

`volicord export mcp-config [--output PATH] [--repo PATH] [--read-only] [--json]`는
호스트 중립 MCP 설정을 내보냅니다. 이는 별도 export 흐름이며 일반 호스트 연결
의도가 아닙니다.

규칙:

- 명령은 선택된 저장소 프로젝트를 루트 기준으로 해석하거나 등록합니다.
- setup이 유효하지 않으면 `action_required` setup 진단을 보고하고, 유효하면 setup
  프로필의 저장된 MCP 명령을 사용합니다.
- 내보낸 명령이 묶인 `volicord-mcp` 프로세스를 시작하는 데 필요한 내부 registry
  상태를 만들거나 갱신할 수 있습니다.
- `--read-only`를 생략하면 workflow 모드를 사용합니다.
- `--output`을 생략하면 설정을 stdout에 씁니다. `--output`이 있으면 정확한 출력
  파일을 이름 붙입니다.
- 내보낸 설정은 export 뒤에도 사용자 관리 설정으로 남습니다. Volicord는 임의 외부
  호스트가 이를 로드, 신뢰, 승인, 초기화, 노출했다고 주장하면 안 됩니다.

## User Channel 명령

<a id="user-channel-commands"></a>
<a id="user-interaction-commands"></a>

`volicord user` 명령은 사람이 로컬 CLI에서 `User Channel`을 통해 작업 상태를
확인하고 대기 중인 사용자 판단에 답할 수 있는 경로를 제공합니다. 이 명령은 Agent
Connection을 만들거나, MCP 호스트 설정을 설치하거나, Agent Connection이 사용자처럼
동작할 수 있게 하지 않습니다.

프로젝트 선택은 `--repo PATH` 또는 현재 작업 디렉터리의 저장소 루트를 사용합니다.
작업 선택은 기본적으로 active 작업을 사용합니다. `--task active`는 이를 명시하고,
`--task ID`는 이름 붙은 작업을 선택합니다.

명령:

- `volicord user status`는 `actor_source=local_user`, `operation_category=read`,
  User Channel 출처로 `volicord.status`를 통해 사용자 중심 작업 상태를 보여 줍니다.
- `volicord user judgments`는 선택된 작업의 대기 판단을 나열하고 현재 출력에 안정적인
  표시 인덱스를 제공합니다.
- `volicord user judgment show INDEX_OR_ID`는 대기 중이거나 과거의 판단 하나, 맥락
  요약, Core 생성 선택지를 표시합니다.
- `volicord user judgment answer INDEX_OR_ID OPTION_INDEX_OR_ID`는 `actor_source=local_user`,
  `operation_category=user_only`, 호환 User Channel 출처, 선택된 선택지의 저장된 기계
  동작과 결과로 `volicord.record_user_judgment`를 통해 Core 생성 선택지 하나를
  기록합니다. `--note`는 메모로만 저장됩니다.

판단 하나를 기록하는 것은 그 판단만 기록합니다. 최종 수락과 잔여 위험 수락은 별개의
판단 종류와 동작으로 남아야 하며, 이 명령이 둘을 하나로 합치면 안 됩니다.

상태, 판단 목록, show 출력은 사용자의 다음 행동을 위해 선택된 담당 상태를 보여
줍니다. 이 출력은 증거, 최종 수락, 잔여 위험 수락, 닫기 준비 상태를 만들지 않습니다.
`volicord user judgment answer`만 대기 중인 해당 판단을 변경하며, 그것도 선택된 Core
생성 선택지를 통해서만 변경합니다.

<a id="dry-run"></a>
## Dry-run과 JSON 출력

`--dry-run`은 영속 변경 없이 계획, 검증, 충돌 감지, 호스트 대상 렌더링, 출력 형태
만들기를 수행합니다.

Dry-run이 하지 않는 것:

- `Volicord Runtime Home` 생성
- SQLite 데이터베이스 생성 또는 수정
- SQLite WAL 또는 SHM 파일 생성
- registry 또는 프로젝트 상태 마이그레이션 적용
- 프로젝트, Agent Connection, Connection Projects, setup 프로필 행, 검증 상태 행
  등록 또는 갱신
- 호스트 설정 파일 생성, 수정, 제거
- `Product Repository` 파일이나 디렉터리 생성, 수정, 제거
- generic export 파일 생성, 수정, 제거
- MCP 시작 점검, MCP 초기화, 도구 탐색 호출

Text 출력은 사람이 읽을 수 있어야 하며 각 리소스 작업을 `created`, `reused`,
`updated`, `removed`, `skipped`, `conflict`, `planned` 중 하나로 식별해야 합니다.

<a id="setup-output"></a>
JSON 출력은 관리 CLI 출력이지 공개 Volicord API 응답 스키마가 아닙니다. setup, 연결,
export, 프로젝트, User Channel 상태를 보고하는 명령은 비대화식 운영자가 성공한
setup과 필요한 사용자 동작을 구분할 수 있을 만큼 구조화된 상태를 포함해야 합니다.

필수 진단 JSON 값:

- `status`: `complete`, `action_required`, `failed`, `not_verified`, 또는 `dry_run`
- `checks[]`: 안정적인 점검 ID, 상태, 요약, 선택 세부사항이 있는 순서 있는 진단 점검
- `actions[]`: 필요하거나 제안되는 사용자 동작. 사용할 수 있을 때 안정적인 동작 ID와
  사람이 읽을 수 있는 명령 또는 안내를 포함합니다.

<a id="noninteractive-approval-behavior"></a>
## 비대화식 동작

비대화식 명령은 필요한 사용자 입력이나 호스트 통제 동작이 없으면 프롬프트를 표시하지
말고 실패해야 합니다.

규칙:

- shared 의도 `Product Repository` 쓰기는 명시적 `--shared` 명령 경로로 승인되며, 그
  명령이 미리 보여 주는 관리 통합 파일로 제한됩니다.
- 기존 비관리 내용은 충돌입니다. CLI는 관련 없는 호스트 설정이나 제품 파일을 조용히
  교체하면 안 됩니다.
- 포괄적 셸 승인, 쓰기 승인, 호스트 신뢰 결정, 민감 동작 승인, `Write Check`는 이
  관리 계약이 요구하는 명시적 CLI 명령 경로를 대신하지 않습니다.
- 호스트 신뢰, 프로젝트 신뢰, 프로젝트 MCP 승인, OAuth, restart, reload 동작은 계속
  사용자 통제 호스트 동작이며 CLI가 대신 제공할 수 없습니다.

## 관리 경계

관리 CLI는 로컬 리소스를 초기화, 등록, 연결, 내보내기, 진단할 수 있습니다. 그 자체로
공개 Volicord API 메서드를 만들지 않으며 Core 권한, Write Check 호환성, 증거 충분성,
닫기 준비 상태, 사용자 소유 판단, 수락, 잔여 위험 수락, 아티팩트 권한, 보안 보장을
만들지 않습니다.

담당 문서 경로:

- 공개 메서드 목록과 메서드 경로: [API 메서드](api/methods.md).
- 공통 요청/응답 스키마: [API 코어 스키마](api/schema-core.md).
- Agent Connection, Connection Projects, 행위자 맥락 의미:
  [Agent Connection](agent-connection.md).
- MCP 프로세스 동작: [MCP 전송](mcp-transport.md).
- 런타임 위치와 저장소 쓰기 경계: [런타임 경계](runtime-boundaries.md).
