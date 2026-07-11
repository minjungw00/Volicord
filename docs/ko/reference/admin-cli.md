# 관리 CLI 참조

이 문서는 로컬 `volicord` 관리 및 초기 설정 CLI를 정의합니다. CLI는
`Volicord Runtime Home`을 준비하고, 저장소 루트에서 프로젝트를 등록하며, Agent
Connection을 관리합니다. 또한 로컬 `User Channel` 명령 경로와 설정·연결 진단을
제공합니다. 숨겨진 훅 명령은 생성된 호스트 통합 래퍼에서만 사용합니다. 이 명령들은
공개 Volicord API 메서드가 아닙니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- `volicord` 명령 이름, 명령줄 인자, 기본값, stdout/stderr 처리, 프로세스 종료 코드
- `init` 중 Runtime Home, 설치 프로필, MCP 명령 선택
- 저장소 루트 프로젝트 감지와 관리 프로젝트 명령
- 지원 호스트 통합을 위한 Agent Connection 명령 동작
- 로컬 `serve` 명령 이름, 명령줄 인자, 기본값, stdout/stderr 처리, 시작 종료 코드
- 생성된 호스트 래퍼를 위한 숨겨진 내부 훅 생명주기 명령 이름, 옵션, 결정, 출력,
  이벤트 기록 동작
- 로컬 `volicord changes` 복구 명령 이름과 출력
- 로컬 `User Channel` 명령 이름과 명령 출력
- 진단 상태, 필요한 사용자 동작, `--dry-run` 미리보기 동작, JSON 출력, 비대화식 동작
- 관리 명령, 로컬 `User Channel` 명령, 공개 Volicord API 메서드 사이의 경계

이 문서는 담당하지 않습니다.

- 공개 Volicord API 메서드: [API 메서드](api/methods.md)
- Agent Connection, Connection Projects, 연결 모드, 연결 의도, 행위자 출처 의미:
  [Agent Connection](agent-connection.md)
- 런타임 데이터 경계 의미와 `Product Repository` 파일 경계 예외:
  [런타임 경계](runtime-boundaries.md)
- MCP 프로세스 시작, stdio와 HTTP 프레이밍, 와이어 동작, 응답 래핑, 종료:
  [MCP 전송](mcp-transport.md)
- 외부 호스트 훅 프로토콜 스키마와 호스트별 응답 의미
- 저장소 기록 배치, SQLite DDL, 기준 저장소 스키마 정의, Core 권한 의미,
  보안 보장 의미

<a id="surface-stability"></a>
## 표면 안정성

라벨은 [문서 정책](../maintain/documentation-policy.md#surface-stability-labels)의
기준 어휘를 따릅니다.

| 표면 | 안정성 | 비고 |
|---|---|---|
| 지원되는 관리 명령 이름, 옵션, stdout/stderr 처리, 프로세스 종료 코드, `--dry-run` 미리보기 동작, 로컬 User Channel 명령 이름 | `stable` | 로컬 CLI 계약이며 공개 Volicord API 메서드가 아닙니다. |
| `detective` 프로필 설정, 호스트 훅 관찰, 세션 감시기 관찰, 로컬 consent URL 사용 가능 상태, 호스트별 통합 기능 보고 | `beta` | 기능 조건과 담당 문서가 정한 비보장 안에서 지원됩니다. |
| 숨겨진 훅 생명주기 명령군, 생성 래퍼 세부사항, 관찰 훅 통합의 조건부 커밋과 복구용 보조 항목 이름, 내부 식별 정보, 호스트 설정 키, 프로세스 바인딩 값 | `internal` | 생성된 호스트 통합을 위한 세부사항입니다. 일반 사용자 입력이나 안정적인 복구 파일 이름이 아닙니다. |
| 사람이 읽는 초기 설정 요약, 상태 요약, 진단 보고서, 연결 검증 보고서, 간결한 요약 카드, 다음 행동 문구, 진단 고지 | `diagnostic` | 이 문서가 명시한 JSON 필드와 안정적인 ID만 계약입니다. 텍스트 서식은 공개 API 스키마가 아닙니다. |

## 명령 모델

`volicord`는 로컬 관리/부트스트랩 실행 파일입니다. 일반 목적의 장기 실행 서버가
아닙니다. 명시적 `volicord serve` 명령은 [MCP 전송](mcp-transport.md)이 설명하는 로컬
MCP 전송 프로세스로 제한됩니다. `volicord inbox` 명령군은 사용자에게 보이는
`Judgment Inbox`이자 선택된 Core 메서드 위에 있는 로컬 `User Channel` CLI
어댑터입니다. 이 명령 이름은 공개 Volicord API 메서드가 아니라 관리 CLI 명령으로
남습니다.

지원되는 기준 명령은 아래와 같습니다.

```text
volicord --help
volicord --version
volicord init --host codex|claude-code --repo PATH [--shared] [--profile record|detective] [--home PATH] [--mcp-command PATH] [--dry-run] [--json]
volicord status [--repo PATH] [--task active|ID] [--json]
volicord doctor [--json] [--privacy-footprint]
volicord connection add [HOST] [--repo PATH] [--shared|--global] [--read-only] [--dry-run] [--json]
volicord connection list [--repo PATH] [--json]
volicord connection status [HOST] [--repo PATH] [--shared|--global] [--json]
volicord connection verify [HOST] [--repo PATH] [--shared|--global] [--json]
volicord connection mode [HOST] workflow|read-only [--repo PATH] [--shared|--global] [--json]
volicord connection remove [HOST] [--repo PATH] [--shared|--global] [--dry-run] [--json]
volicord project use [PATH] [--json]
volicord project current [--json]
volicord project list [--json]
volicord project rename NAME [--repo PATH] [--json]
volicord project forget [PATH|NAME] [--json]
volicord mcp --stdio --connection <connection_id> [--project <project_id>]
volicord mcp --check --connection <connection_id>
volicord mcp --check --connection <connection_id> --project <project_id>
volicord serve --transport local-http [--listen 127.0.0.1:8765 | --container-listen 0.0.0.0:8765] [--home PATH] [--connection <connection_id>] [--project PATH]... [--token-file PATH | --token TOKEN | --generate-token] [--allow-origin ORIGIN]
volicord export authority-bundle --output PATH [--repo PATH] [--json]
volicord changes reconcile [--repo PATH] [--task active|ID] [--dry-run] [--json]
volicord inbox [--repo PATH] [--task active|ID] [--json]
volicord inbox answer <judgment-id> --choice <choice> [--repo PATH] [--note TEXT] [--json]
volicord inbox open <judgment-id> [--repo PATH] [--json]
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
- `--json`은 stdout에 JSON 문서 정확히 하나를 쓰며 사람용 설명을 섞지 않습니다. JSON
  출력은 결과 상태, 진단, `summary_card`, `checks`, `actions`, 안정 필드를 위한 자동화
  표면입니다. 자동화는 기본 사람용 텍스트 출력을 파싱하면 안 됩니다.
- 숨겨진 내부 훅 명령은 기본적으로 `--output volicord-json`을 사용합니다.
  `volicord-json` 모드에서는 Volicord 래퍼 JSON을 쓰고, `deny`는 종료 코드
  `1`로 끝나며, `allow`, `warn`, `inject_context`는 종료 코드 `0`으로 끝납니다.
  `--output text`는 같은 종료 동작으로 사람이 읽기 쉬운 짧은 한 줄을 씁니다.
- `--host-output codex|claude-code`를 받은 숨겨진 내부 훅 명령은 Volicord 래퍼 JSON
  대신 호스트 고유 훅 출력을 씁니다. 정책 결정은 해당 호스트의 stdout, stderr, 종료
  코드 규칙을 사용합니다. 생성되는 Codex와 Claude Code 훅 래퍼 스크립트는 이 모드를
  사용하며, Claude Code 정책 차단은 종료 코드 `1`로 표현하지
  않습니다.
- 오류는 CLI 종료 코드 모델에 따라 stderr 진단으로 남습니다.
- `volicord serve --transport local-http`는 명시적 장기 실행 MCP 전송 프로세스입니다.
  네이티브 실행은 루프백 주소만 허용하고, `--container-listen`은 Docker 호스트 루프백
  노출에만 사용합니다. 베어러 인증, Origin 점검, HTTP 와이어 동작은
  [MCP 전송](mcp-transport.md)이 담당합니다. 이 명령은 공개 네트워크 API, SaaS
  엔드포인트, 다중 사용자 서버, 보안 경계가 아닙니다.

지원하지 않는 것:

- CLI에는 일반 목적의 `server` 또는 daemon 명령이 없습니다.
- `volicord serve`는 공개 Volicord API 서비스, SaaS 엔드포인트, 다중 사용자 서버,
  보안 경계, 인증 없는 네트워크 서비스로 취급하면 안 됩니다. `--container-listen`은
  Docker 호스트 루프백 노출만을 위한 옵션이며, 공개 호스트 인터페이스나 원격 제공
  옵션이 아닙니다.
- 관리 명령은 공개 Volicord API 메서드가 아니며 공개 메서드 목록에 추가되면
  안 됩니다.
- 숨겨진 훅 명령은 협력적 탐지 명령이며 OS 수준 샌드박싱이나 보안 집행 증명이
  아닙니다. 일반 최상위 도움말에는 표시되지 않습니다.
- 텍스트 모드 사용자 흐름은 `project_internal_id`, `connection_internal_id`,
  호스트 설정 키, 프로토콜 래퍼, 저장된 레지스트리 필드를 사용자가 입력하도록 요구하면
  안 됩니다.

### 출력 규칙

- `volicord init`은 원시 상태 목록 대신 간결한 온보딩 요약을 표시할 수 있습니다. 이
  요약에는 호스트, 프로필, 연결 의도, 호스트 범위, 저장소, 저장소 파일 변경,
  Runtime Home, `Next:` 체크리스트, 한계, JSON 진단 명령이 들어갑니다.
  `Next:` 아래의 명령은 들여쓴 별도 줄에 표시하며 산문 구두점을 붙이지 않습니다.
- 상태형 명령은 세부 진단보다 먼저 간결한 요약 카드나 짧은 섹션을 보여 줍니다. 여기에는
  `volicord status`, `volicord doctor`, 연결 상태와 검증, 변경 조정, 받은편지함이
  포함됩니다. 명령이 공통 요약 카드 텍스트 렌더러를 사용할 때 라벨은
  `Task lifecycle`, `Volicord record effect for this command`, `Profile`, `Write Ticket`,
  `Evidence`, `Pending user judgments`, `Unrecorded Product Repository changes`,
  `Close readiness`, `Transport`, `Primary next action`입니다.
- `Volicord record effect for this command`는 현재 명령이 Core 권한 상태를 변경했는지만
  나타냅니다. 사람용 텍스트는 JSON 값 `read_only`를 `none`으로 표시하며, 이는
  `this command made no Core authority-state mutation`을 뜻합니다. 커밋된 기록 효과는
  `recorded`로 표시합니다. 같은 줄의 괄호는 이 값이 Product Repository 파일 쓰기나
  Runtime Home 쓰기 능력을 나타내지 않는다고 명시합니다. 마찬가지로 `not_selected`는
  `not shown in this view`로, 대기 판단 수가 0이면 `pending (0)`으로 표시합니다.
- `Primary next action`은 알 수 있는 경우 간결한 카드가 제시하는 즉시 수행할 행동을
  표시하며, 필요하면 후속 검증 명령을 포함합니다. 다음 행동을 알 수 없을 때만 `none`을
  사용합니다. 상태와 변경 조정 텍스트는 카드의 행동이 유일한 행동처럼 보이지 않도록
  최상위 `close_blockers`와 `next_actions` 전체 개수도 표시합니다. 공개된 주 행동과 추가
  행동의 표시 역할은 `presentation_role`이 정의하며 배열 순서는 정의하지 않습니다. 어느
  역할도 권한을 부여하지 않습니다. 안정적인 차단 사유 코드와 행동 코드는 사람용 문장과
  구분합니다.
- 텍스트 요약은 표시된 행동에 필요한 경우가 아니면 내부 ID를 숨깁니다. JSON은
  `summary_card`와 최상위 응답의 기존 필드 이름과 안정 값을 그대로 유지합니다. 자동화는
  텍스트 서식을 파싱하면 안 됩니다.
- 연결 상태와 검증의 텍스트 출력은 제목과 `Status`, `Profile`, `Repository` 또는
  `Repositories`, `Checks`, `Next`, `Limits`, `Diagnostics` 섹션으로 시작합니다. 연결
  추가는 호스트 설정이나 저장소 파일 변경도 요약할 수 있습니다. 자세한 훅 상태, CLI MCP
  사전 점검과 핸드셰이크, 호스트 관찰은 JSON에 둡니다. Codex 텍스트는 프로젝트 신뢰,
  관리 시작, 관리 `tools/list`, 관리 도구 호출, 활성 도구 노출, 호스트 MCP 명령을 별도
  점검으로 보여 줄 수 있습니다.
- 대기 판단이 있으면 상태와 받은편지함 텍스트는 `Available answer paths:`에 호스트
  프롬프트 입력, 채팅 명령 캡처, 로컬 consent URL, CLI 받은편지함을 표시할 수 있습니다.
  이 줄은 사용자를 답변 경로로 안내할 뿐 판단을 기록하거나 Agent Connection이 사용자처럼
  동작하게 하지 않습니다. JSON은 같은 사실을 `user_channel_availability` 또는
  `answer_path_availability`에 담습니다.
- 다른 진단 보기는 `action_required` 또는 저하 상태에 간결한 `Result`, `Why`, `Next`,
  `Does not prove` 줄을 사용할 수 있습니다. 연결 상태와 검증은 섹션 형태를 사용합니다.
  어느 형태에서도 `action_required`는 치명적인 CLI 오류가 아닙니다. `Next`에는 호스트
  설정 다시 불러오기나 재시작, 호스트·프로젝트 승인, 관리 설정 복구, 이후 표시된 검증
  명령 실행처럼 구체적인 행동을 적습니다.

<a id="runtime-home-selection"></a>
## Runtime Home 선택

`volicord init`은 로컬 설치 프로필을 만들거나 재사용하는 공개 첫 실행 경로입니다.
선택된 Runtime Home을 만들거나 검증하고, 이후 관리 명령, Agent Connection, MCP
프로세스 흐름이 사용할 명령 경로를 저장하며, 선택된 저장소를 등록하고 선택된 호스트
연결을 설치합니다. Init은 저장소 설정과 호스트 연결을 수행하면서 Runtime Home 경로나
MCP 시작 명령도 선택할 수 있습니다. 부모 셸의 현재 환경을 바꿀 수는 없습니다.

최상위 설정 상태는 설치 프로필 준비 또는 호스트 연결에 이름 붙은 사용자 동작이 아직
필요한지를 답합니다. Runtime Home과 설치 프로필을 저장한 뒤에도 선택된 명령을 이후
셸이나 에이전트 호스트가 `PATH`로 찾을 준비가 되어 있지 않으면 init은
`action_required`를 보고할 수 있습니다. JSON 출력은 명령 가용성 세부사항과 필요한
동작을 명시적으로 보여 줘야 합니다. 기본 텍스트 출력은 세부 진단 목록 대신 간결한
온보딩 `Next:` 체크리스트로 그 동작을 보여 줄 수 있습니다.

인자:

| 인자 | 의미 |
|---|---|
| `--home PATH` | `Volicord Runtime Home`을 선택합니다. 생략하면 플랫폼 기본 로컬 런타임 위치를 사용합니다. 선택한 경로는 프로젝트 상태를 사용하기 전에 Runtime Home/Product Repository 분리 계약을 만족해야 합니다. |
| `--mcp-command PATH` | init이 설치 프로필을 만들거나 갱신할 때 관리 호스트 설정이 `mcp --stdio --connection <connection_id> [--project <project_id>]` 인자를 붙여 사용할 정확한 `volicord` 명령을 저장합니다. 생략하면 init이 선택한 실행 중인 `volicord` 실행 파일을 사용합니다. |
| `--json` | 기계 판독 비대화식 출력을 선택합니다. JSON 모드에서는 init이 프롬프트를 표시하지 않습니다. |

Runtime Home과 설치 프로필 선택에 관련된 init 효과:

- Runtime Home 레지스트리를 만들거나 검증합니다.
- Runtime Home 식별 정보와 설치 프로필 메타데이터를 기록합니다.
- 이후 `init`, `connection`, `doctor`, MCP 시작 흐름이 사용할 `volicord` 명령 위치와 MCP
  시작 명령을 기록합니다.
- 선택된 명령 경로가 현재 프로세스의 `PATH`로 해석되는지 검사합니다.
- 선택된 명령이 현재 프로세스의 `PATH`에 보이지 않으면 `PATH` 동작을 보고합니다.
  기존 셸과 에이전트 호스트 프로세스에는 재시작이나 설정 다시 불러오기가 필요할 수 있습니다.
- 첫 실행 호스트 설정 경로의 일부로 선택된 저장소를 등록하거나 재사용합니다.
- 공개 Volicord API 메서드를 만들거나 사용자 소유 판단을 기록하지 않습니다.

`volicord doctor`는 설치 프로필을 읽기 중심으로 진단합니다. 최상위 상태는 현재 프로필을
사용할 수 있는지를 답합니다.

다음을 점검합니다.

- Runtime Home 접근, 레지스트리 스키마, 설치 프로필 존재 여부
- 저장된 명령의 준비 상태와 `PATH`를 통한 사용 가능 여부
- 링크 메타데이터가 있을 때의 명령 링크 또는 호환 실행 파일 준비 상태
- 연결 검증에서 다뤄야 할 지원 호스트 감지 상태
- 탐지 프로필 기록이 있을 때 훅 파일 설치, 설정 상태, 런타임 훅 관찰, 종합 탐지 상태,
  호스트 설정 다시 불러오기 필요 여부

저장된 명령 경로가 실행 가능하면 doctor는 `complete`와 함께 명령 가용성 경고와
`actions_recommended`를 보고할 수 있습니다. `PATH` 또는 명령 링크 권장 동작은 기존
에이전트 호스트에 재시작이나 설정 다시 불러오기가 필요한 때를 알려야 합니다. 세션
감시기 관찰이나 로컬 consent URL 같은 런타임 전용 기능은 보고 프로세스가 해당 상태를
실제로 소유할 때만 사용 가능으로 보고합니다.

이 점검은 OS 강제, 샌드박싱, 쓰기 방지, 제품 정확성, 닫기 상태를 증명하지 않습니다.
Doctor는 프로젝트를 만들거나, 호스트 설정을 설치하거나, 연결 모드를 바꾸거나, 사용자
판단에 답하지 않습니다. 사람용 텍스트는 프로필과 관찰 한계를 요약할 수 있습니다. 정확한
`selected_profile`, `observation_summary`, `control_surface` 필드는 JSON에 둡니다.

텍스트와 JSON 출력은 진단 고지와 간결한 `summary_card`를 포함합니다. JSON은
`disclosure.guarantee_class=detective_observation`을 사용하고,
`NotOsSandbox`, `NotNetworkIsolation`, `NotFullWritePrevention`,
`NotActorAttributionProof`, `NotCorrectnessProof`,
`NotTestSufficiencyProof`, `NotHumanReviewReplacement` 같은
`non_guarantees` 값을 담습니다.

<a id="project-commands"></a>
## 프로젝트 명령

프로젝트 명령은 저장소 루트를 사용자 대상 프로젝트 식별자로 사용합니다. 내부 프로젝트
식별 정보는 저장소와 출처 데이터이며 텍스트 모드 명령은 이를 요구하지 않습니다.

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
등록은 `project_internal_id`, 사용자 대상 프로젝트 이름, Runtime Home 아래의
프로젝트 홈, 필요한 프로젝트별 상태를 만듭니다. 기본 프로젝트 이름은 저장소 디렉터리에서
파생하고 Runtime Home 안에서 필요하면 고유하게 만듭니다.

`volicord project current`는 현재 작업 디렉터리에서 감지된 프로젝트를 보고합니다.
프로젝트 등록을 만들지 않습니다.

`volicord project list`는 등록 프로젝트를 사용자 대상 이름, 저장소 루트, 상태와
함께 나열합니다.

`volicord project rename NAME [--repo PATH]`는 선택된 저장소의 사용자 대상 프로젝트
이름을 바꿉니다. `project_internal_id`, 저장소 루트, 프로젝트 홈, Core 상태는
바꾸지 않습니다.

`volicord project forget [PATH|NAME]`은 활성 Agent Connection 멤버십이나 담당
문서가 계속 주소 지정 가능해야 한다고 요구하는 프로젝트 상태를 고아로 만들지 않을
때만 선택된 프로젝트 등록을 제거합니다. 프로젝트를 잊는 동작은 `Product Repository`,
관련 없는 Runtime Home 데이터, 호스트 설정, 남아 있는 다른 등록이 소유하는
아티팩트 저장소, 보존되어야 하는 Core 권한 행을 삭제하면 안 됩니다.

<a id="connection-intents-and-hosts"></a>
## 연결 의도와 호스트

Agent Connection 설정은 낮은 수준의 호스트 설정 범위 이름 대신 연결 의도를
사용합니다.

| 의도 | 선택 방법 | 의미 |
|---|---|---|
| `personal` | 기본값 | 현재 사용자의 일반 로컬 흐름을 위한 사용자 소유 호스트 설정입니다. |
| `shared` | `--shared` | 선택된 `Product Repository` 안의 명시적 통합 파일로 저장되는 프로젝트 소유 또는 프로젝트 공유 호스트 설정입니다. |
| `global` | `--global` | 선택된 호스트의 사용자 전역 호스트 설정입니다. 프로젝트 접근은 계속 등록된 저장소 루트와 Connection Projects로 제한됩니다. |

`volicord init`은 기본적으로 `personal`을 사용하고, 명시적 공유 연결에는
`--shared`를 받습니다. `--global`은 받지 않습니다. 다른 연결 명령은 각 명령
구문에 표시된 의도 옵션을 받습니다. 두 옵션을 모두 지원하는 명령에서
`--shared`와 `--global`은 함께 사용할 수 없습니다. 의도 플래그가 없으면
의도는 `personal`입니다.

연결 모드:

- `workflow`가 기본 모드입니다.
- `read-only`는 명시적으로 선택하며 Agent Connection을 통해 읽기와 프로젝트 탐색
  동작만 노출합니다.
- `volicord connection mode ... workflow|read-only`는 사용자가 호스트 설정을 직접
  편집하지 않아도 선택된 연결의 저장 모드를 바꿉니다.

내부 호스트 설정 키 `server_name`의 기본값은 `volicord`입니다. 일반 CLI 흐름은
서버 이름 옵션을 노출하지 않습니다. 생성된 호스트 설정에는 호스트가
`volicord mcp --stdio`를 시작할 수 있도록 저장된 내부 식별 정보에서 파생된
`connection_id`와, 안전하게 프로젝트에 묶인 경우의 `project_id` 프로세스 바인딩 값,
서버 이름, 명령 인자가 들어갈 수 있습니다. 이 값들은 저장된 프로세스 바인딩
세부사항이며 사용자 권한 토큰이 아닙니다. 텍스트 모드 명령 입력은 선택된 호스트,
의도, 저장소 루트를 사용합니다.

일반 `volicord connection add` 명령은 MCP 명령 경로나 Runtime Home 경로를 다시 묻지 않고
해석된 Runtime Home에 저장된 프로필을 사용합니다. 개인, 로컬, 사용자 전체 호스트
설정은 그 Runtime Home을 `VOLICORD_HOME`으로 담을 수 있습니다. `shared` 프로젝트
호스트 설정은 개인 Runtime Home 경로를 포함하면 안 되며, 생성된 항목이 선택된 프로젝트
하나를 위한 것일 때 명령 이름 `volicord`와 프로젝트에 묶인
`mcp --stdio --connection <connection_id> --project <project_id>` 인자를 사용합니다.
여러 연결 프로젝트를 의도적으로 다루는 항목에만 연결 전용 생성 인자를 남깁니다.
호스트 환경은 명령을 `PATH`로 해석해야 합니다.

<a id="agent-host-setup-and-init"></a>
### 호스트 설정 프로필

`volicord init --host codex --repo PATH --profile record`와
`volicord init --host claude-code --repo PATH --profile record`는 호스트 훅과 세션 감시기
관찰을 설치하지 않는 대화 중심 사용을 위한 첫 실행 저장소 설정 및 호스트 연결
예시입니다. 의도 플래그가 없으면 init은 `personal` 연결을 만듭니다. Codex는 사용자
설정 대상을 사용하고 Claude Code는 저장소 로컬 CLI 범위를 사용합니다. `--shared`를
추가하면 `.codex/config.toml` 또는 `.mcp.json`의 프로젝트 범위 호스트 배치를
선택합니다. 이 생성 항목은 `PATH`를 통해
`volicord mcp --stdio --connection <connection_id> --project <project_id>`를 시작하고
개인 Runtime Home 경로를 포함하지 않습니다.

연결 의도는 관리 Agent Connection 대상을 선택합니다. 이와 별도로 init은
`personal`과 `shared` 모두에서 현재 저장소 통합 파일 구성을 유지합니다.
`AGENTS.md`의 관리 블록과 `.volicord/policy.json`은 저장소 로컬 파일이고,
공유 Claude Code init은 저장소의 `.mcp.json` 상태 보기도 함께 관리합니다. 개인
Claude Code init은 MCP 등록에 호스트의 로컬 CLI 대상만 사용합니다. Detective
profile은 지원되는 저장소 로컬 훅, 래퍼, 규칙 파일을 추가합니다. 개인 Claude Code
탐지 설정은 `.claude/settings.local.json`을 사용하고, 공유 탐지 설정은
`.claude/settings.json`을 사용합니다. 이런 통합 파일은 저장된 연결 의도나 주 호스트
범위를 바꾸지 않습니다.

`personal` init에서는 선택한 작업 트리에 실제로 적용되는 Git `info/exclude`에도
Volicord 관리 블록을 둡니다. 일반 `.git` 디렉터리, `.git` gitdir 파일, 연결된
worktree의 `commondir`를 해석해 실제 공통 Git 디렉터리를 찾습니다. 관리 블록은
`/.volicord/`, Volicord 전용 훅 래퍼와 규칙 경로, 개인 훅 설정 파일인
`/.codex/hooks.json`과 `/.claude/settings.local.json`만 제외합니다.
`.codex/`나 `.claude/` 전체를 제외하지 않으며, `AGENTS.md`, `.mcp.json`,
`.claude/settings.json`처럼 여러 주체가 함께 관리할 수 있는 상태 보기 파일도
제외하지 않습니다. init은 추적되는 `.gitignore`를 쓰거나 바꾸지 않습니다.
`shared` init은 개인 로컬 제외 블록을 추가하지 않으며, 이전 개인 init이 남긴
블록을 제거하지도 않습니다.

`--profile`은 공개 통합 프로필을 선택합니다.

- `record`가 기본값입니다. MCP 설정, 관리되는 `AGENTS.md` 안내 블록, 정책 메타데이터를
  써서 호스트 생명주기 훅이나 세션 감시기 없이 MCP를 통한 협력적 Volicord 작업 흐름
  기록을 지원합니다.
- `detective`는 MCP 설정, 관리되는 `AGENTS.md` 안내 블록, `.volicord/policy.json` 훅 명령
  정책, 지원되는 프로젝트 로컬 호스트 훅 및 규칙 파일을 쓰고 호스트 훅과 세션 감시기
  관찰 상태를 기록합니다.

탐지 프로필을 인식하는 설정, 상태, 검증, doctor 출력은 프로필과 관찰 상태를
보고합니다. 사람용 텍스트는 `Profile`, `Checks`, `Limits`, `Diagnostics` 같은 명령별
섹션으로 이를 요약할 수 있으며, 원시 진단 목록이 아닙니다. JSON 진단은
`selected_profile`, `observation_summary`, `control_surface` 필드를 정확히 담고,
`host_hooks_active`, `session_watcher_active`,
`cooperative_pre_tool_warning_available`,
`cooperative_pre_tool_denial_available`, `unrecorded_changes_detectable`,
`actor_identity_provable`, `os_enforced`를 포함합니다. 현재 Volicord 출력은
`os_enforced=false`와 `actor_identity_provable=false`를 보고해야 합니다.

`detective` 초기화에는 선택한 호스트 어댑터가 모든 필수 생명주기 훅인
`session-start`, `pre-tool`, `post-tool`, `prompt-capture`, `stop` 지원을 선언하고
검증할 수 있어야 합니다. 또한 선택한 Product Repository에 대해 세션 감시기 스냅샷
지원이 필요합니다. `AGENTS.md`와 `.volicord/policy.json`은 호스트 훅 설정이 아닙니다.
어댑터가 모든 필수 단계에 대해 신뢰할 수 있는 프로젝트 로컬 훅
스키마나 경로를 알지 못하면 init은 `DETECTIVE_HOOKS_UNSUPPORTED`로 실패합니다. 세션
감시기가 선택한 저장소의 스냅샷을 만들 수 없으면 init은
`DETECTIVE_WATCHER_UNSUPPORTED`로 실패합니다. 복구 방법은 기록 전용 설정에는
`--profile record`를 사용하거나, `detective`를 다시 실행하기 전에 지원되는 호스트, 플랫폼,
저장소 설정을 준비하는 것입니다. `record`는 훅 설치나 세션 감시기 설정을
요구하지 않습니다.

네이티브 Windows에서는 init이 탐지용 호스트 훅 파일을 계획하거나 쓰기 전에
`--profile detective`를 `DETECTIVE_WINDOWS_UNSUPPORTED`로 거부합니다. 네이티브 Windows는
`--profile record`를 지원합니다. `detective`는 선택한 호스트 훅과 감시기 계약이 지원되고
테스트된 WSL2, Linux, macOS에서만 사용합니다.

Codex `detective` 초기화에서는 선택된 Product Repository가 Git 작업 트리 루트여야
합니다. 그래야 하위 디렉터리 호스트 세션에서도 현재 작업 디렉터리와 무관하게 래퍼
경로를 해석할 수 있습니다. 이 전제조건을 만족하지 않으면 init은 단순 상대 훅 경로를
생성하지 않고 `DETECTIVE_HOOK_ROOT_UNSUPPORTED`로 실패합니다. Claude Code `detective` 초기화는
[탐지용 호스트 훅 생명주기 명령](#guard-hook-commands)에서 설명한 호스트 프로젝트 디렉터리 자리표시자를
사용합니다.

`detective`에서 init은 생성된 탐지용 호스트 훅을 불러오려면 호스트 재시작이나 설정 다시
불러오기가 필요할 때 `reload_required`를 기록합니다. 파일은 설치되었지만 일치하는
호스트 훅 이벤트가 아직 관찰되지 않았으면 `configured`를 기록합니다. 파일을 썼다는
이유만으로 탐지 프로필 설치 기록을 `active`로 표시하지 않습니다.

`--home PATH`는 이 초기화에 사용할 Runtime Home을 선택합니다. `--mcp-command PATH`는
init이 설치 프로필을 만들거나 갱신해야 할 때 정확한 명령 경로를 설치 프로필에
저장합니다. 개인 호스트 설정은 호스트 어댑터가 요구하는 대로 저장된 프로필 경로와
Runtime Home을 사용합니다. `--shared`가 선택한 프로젝트 범위 호스트 MCP 설정은
그래도 `PATH`의 `volicord`를 사용합니다.

미리보기가 아닌 `volicord init`은 다음을 수행합니다.

- Runtime Home이 없으면 초기화합니다.
- 필요하면 설치 프로필을 만들거나 갱신합니다.
- 선택한 `Product Repository`를 등록하거나 재사용합니다.
- 일치하는 Agent Connection과 Connection Projects 멤버십을 만들거나 갱신합니다.
- 의도에 따라 주 관리 호스트 연결 대상을 설치합니다. `personal`은 Codex 사용자
  대상 또는 Claude Code 로컬 CLI 대상을 사용하고, 명시적 `--shared`는 프로젝트 범위
  Codex `.codex/config.toml` 또는 Claude Code `.mcp.json`을 사용합니다.
- 명시적 공유 연결에는
  `volicord mcp --stdio --connection <connection_id> --project <project_id>`를 쓰며,
  Codex에는 관리 시작 출처 환경 변수 마커도 넣습니다.
- `AGENTS.md` 안의 Volicord 관리 블록만 쓰거나 갱신합니다.
- 숨겨진 내부 훅 명령군을 호출하는 탐지용 호스트 훅 명령을 담은
  `.volicord/policy.json`을 씁니다.
- `personal`에서는 실제 Git `info/exclude`의 Volicord 관리 로컬 경로 블록을 쓰거나
  갱신합니다. 명시적 `--shared`는 이 블록을 추가하지 않습니다.
- 필수 탐지 생명주기 단계를 위한 Volicord 관리 훅 래퍼 스크립트를
  `.codex/hooks/` 또는 `.claude/hooks/` 아래에 씁니다.
- `.codex/hooks.json` 또는 `.claude/settings.json` 같은 지원 호스트 훅 파일을
  쓰며, 이 파일은 해당 래퍼 스크립트를 호출합니다.
- `.codex/rules/*.rules` 또는 `.claude/rules/volicord.md` 같은 지원 호스트 규칙 파일을
  씁니다.
- Runtime Home 레지스트리에 탐지 프로필 훅 관찰 상태를 기록합니다.
- 필수 호스트 훅 설정이나 세션 감시기 지원이 없으면 `detective` 초기화를
  거부합니다.
- Windows 호스트 훅 래퍼와 감시기 동작이 구현되고 테스트되지 않았으므로 네이티브
  Windows에서 `detective` 초기화를 거부합니다.
- 호스트가 새 MCP 또는 탐지용 호스트 훅 설정을 불러와야 할 때 필요한 재시작, 설정 다시
  불러오기, 신뢰, 승인 동작을 보고합니다.

init 재실행은 일치하는 Volicord 관리 내용에 대해 멱등입니다. 관리 블록, 정책 파일,
호스트 MCP 항목, 탐지 프로필 설치 기록을 중복 없이 갱신합니다. 기존 대상에 Volicord가
소유 마커나 관리 지문을 요구하는 위치의 비관리 내용이 있으면 init은 이를 덮어쓰지
않고 충돌로 보고해야 합니다. 후속 상태 조회와 검증 명령, dry-run 진단, 복구 명령은
선택한 의도를 보존하며 공유 init 결과일 때만 `--shared`를 포함합니다.

Unix에서 새 `.volicord/policy.json`은 사용자 전용 모드 `0600`으로 만듭니다. 기존
일반 파일 정책에 Volicord 소유권 메타데이터가 있으면 같은 init을 다시 실행할 때
동일한 조건부 관리 파일 경로를 거쳐 그룹 또는 기타 사용자 권한 비트를 복구합니다.
비관리 정책 파일은 계속 충돌입니다. 정책 JSON을 직렬화하기 전에 MCP 환경 맵은
`VOLICORD_HOME`, `VOLICORD_MCP_LAUNCH`, `VOLICORD_MCP_HOST`,
`VOLICORD_MCP_CONNECTION_ID`, `VOLICORD_MCP_PROJECT_ID`만 허용합니다. 비밀값을
나타내는 형태의 키와 그 밖의 환경 키는 값을 진단에 포함하지 않고 거부합니다. 이
허용 목록은 일반적인 비밀값 내용 검사기가 아닙니다.

init에서 계획된 관찰 훅 통합 관리 파일은 같은 디렉터리의 조건부 커밋으로 적용합니다.
이 규칙은 관리 지침, 정책, 훅, 래퍼, 규칙, Git 제외 파일에 적용됩니다. 프로젝트
범위 MCP 설정을 호스트 어댑터가 적용하는 경계는 별도로 유지됩니다. 관찰 훅 통합 계획은 대상이
없다는 사실 또는 안정적인 일반 파일 스냅샷을 기록합니다. 적용 단계는 심볼릭 링크를
따라가지 않고 `Product Repository`와 대상의 각 부모 디렉터리를 고정하며, 바뀌었거나
일반 파일이 아닌 대상을 거부하고, 대상과 같은 디렉터리에 스테이징 파일을 씁니다.
그다음 기존 대상을 덮어쓰지 않는 생성 연산이나 운영체제 고유의 교체 또는 맞바꾸기
연산을 사용합니다. 생성은 동시에 만들어진 대상을 교체하면 안 됩니다. 갱신은 설치된
대상이 스테이징 파일과 일치하고, 밀려난 항목이 계획된 이전 파일과 일치하며, 그 항목이
제거되었음을 확인한 뒤에만 성공합니다.

동시 변경이나 운영체제 고유 연산의 부분 실패 상태 때문에 이 결과를 검증할 수 없으면
CLI는 모든 관련 항목이 검사한 상태와 계속 일치할 때만 되돌리기를 시도합니다. 검증된
되돌리기는 해당 시도가 소유한 같은 디렉터리의 보조 항목을 제거하고 실패를 보고합니다.
동시 작성자의 바이트를 손상시킬 위험 없이 자동 복구를 계속할 수 없다면 CLI는 실패를
보고하고, 검사 시 실제로 존재한 복구 항목만 이름 붙이며, 자동 삭제나 교체를 중단합니다.
내부 보조 항목 이름에는 안정적인 명명 또는 보존 계약이 없습니다. 여기서 원자성은 지원
플랫폼의 같은 디렉터리 이름 공간 전환만 뜻합니다. 여러 파일이나 Runtime Home 상태를
가로지르는 프로비저닝 트랜잭션이 아니며 전원 장애 내구성 보장도 아닙니다.

Git 제외 계획은 심볼릭 링크인 `.git` 마커, 잘못되었거나 지나치게 큰 gitdir 또는
commondir 제어 파일, 디렉터리가 아닌 Git 대상, 심볼릭 링크나 비정규 구성 요소를
통해 해석되는 Git 디렉터리 경로를 거부합니다. 연결된 worktree에서는 Product
Repository가 아니라 해석된 공통 Git 디렉터리를 고정한 뒤 씁니다. 부모 경로에서
심볼릭 링크를 따라가지 않는 규칙, 오래된 계획 거부, 조건부 교체, 복구 규칙은 이
경로에도 동일하게 적용됩니다.

조건부 쓰기 점검은 일반적인 동시 작성자가 관리 대상이나 부모 경로를 바꾸는 경우를
다룹니다. 구현 전용 보조 항목 이름은 활성 CLI 시도에 예약됩니다. 같은 권한을 가진 로컬
프로세스가 예측할 수 없는 이 이름을 찾아 의도적으로 삭제하거나 교체하는 행위는 이
협력적 쓰기 보장 밖입니다. CLI는 관찰할 수 있는 상태를 모두 다시 검증하지만, 이 이름은
해당 디렉터리에 이미 쓰기·삭제 권한이 있는 다른 프로세스를 막는 OS 샌드박스나 격리
경계가 아닙니다.

기존 관찰 훅 통합 관리 파일을 갱신할 때의 메타데이터 처리는 플랫폼마다 다릅니다.

- Linux와 macOS에서 같은 디렉터리의 스테이징 파일은 내용을 쓰는 동안 모드 `0600`을
  유지합니다. CLI는 커밋 전에 이전 파일의 POSIX 모드, 사용자 ID, 그룹 ID, 선택한
  플랫폼 인터페이스가 노출하는 모든 확장 속성을 다시 적용하고 검증합니다. 다만
  Volicord 소유 정책 파일은 위에서 설명한 모드 `0600`을 적용하면서 사용자 ID,
  그룹 ID, 지원되는 확장 속성을 유지합니다. 그 집합을
  읽거나 재현하거나 검증할 수 없으면 성공을 보고하기 전에 갱신을 거부합니다.
  운영체제가 ACL을 이런 확장 속성으로 표현할 때만 해당 ACL이 포함되며, 인터페이스가
  노출하지 않는 별도 메타데이터 메커니즘까지 보장하지 않습니다.
- 네이티브 Windows에서 CLI는 계획된 이전 파일에 대한 새 쓰기 공유를 차단하고,
  새 항목 전용 생성으로 백업 이름을 예약하고, 이전 파일을 가리키는 두 번째 하드 링크를
  보존한 뒤 `ReplaceFileW`의 기본 속성과 ACL 병합 동작을 사용합니다. 모든 운영체제 고유
  반환 뒤 대상, 교체 파일, 백업, 보존한 이전 파일을 다시 검사합니다. 이 운영체제 고유
  병합은 플랫폼 간 메타데이터 동등성 보장이 아닙니다.
- 새 관리 파일은 선택된 디렉터리에서 새 파일이 일반적으로 받는 메타데이터를
  사용합니다. 플랫폼 전체에서 소유자, ACL, 확장 속성, 타임스탬프, 대체 데이터
  스트림, 라벨, 그 밖의 메타데이터 전체가 동등하다는 의미는 아닙니다.

<a id="volicord-agent-install"></a>
## Agent Connection 명령

연결 선택은 호스트, 의도, 저장소 루트를 사용합니다. 의도 플래그가 없고 저장소가
선택되어 있으면 status, verify, mode, remove는 그 호스트와 저장소에 대해 의도를
가로질러 하나만 일치하는 연결을 선택합니다. 둘 이상의 연결이 일치하면 명령은
모호한 선택자를 보고하고 호출자는 일치하는 의도 플래그를 추가해야 합니다. 명령은
내부 연결 식별자를 파생하거나 조회합니다.

- `volicord init`
  - 레지스트리: Runtime Home과 설치 프로필을 준비하고, 저장소를 등록하며, 기본 개인
    연결 또는 명시적 공유 연결과 멤버십을 만들거나 갱신하고, 탐지 프로필 관찰 상태를
    기록합니다.
  - 호스트: 의도에 맞는 관리 MCP 대상과 `codex` 또는 `claude-code`용 저장소 로컬
    지침, 정책, 프로필별 통합 파일을 설치합니다.
  - 검증: 관찰 가능한 호스트 설정, MCP 시작, 초기화, `tools/list`를 점검하고 필요한
    호스트 통제 동작을 보고합니다.
- `volicord connection add`
  - 레지스트리: 저장소를 등록하고, 일치하는 연결을 만들거나 갱신하며, 의도와 모드를
    기록하고 프로젝트 멤버십을 보장합니다.
  - 호스트: 선택한 호스트와 의도에 맞는 관리 설정을 설치합니다.
  - 검증: 관찰 가능한 호스트 설정과 MCP 점검을 실행합니다.
- `volicord connection list`는 일치하는 연결과 프로젝트, 저장된 진단 검증 상태를 읽습니다.
  호스트를 시작하거나 설정을 다시 쓰거나 호스트 점검을 새로 실행하지 않습니다.
- `volicord connection status`는 연결 하나를 읽고 저장된 검증 상태와 필요한 동작을
  보고합니다. 호스트를 시작하거나 설정을 다시 쓰지 않습니다.
- `volicord connection verify`는 관리 대상을 관찰할 수 있을 때 검사하고, 점검을
  실행한 뒤 결과 검증 상태를 저장합니다.
- `volicord connection mode`는 저장된 연결 모드를 바꿉니다. 호스트 항목을 다시 생성해야 할
  때만 설정을 다시 쓰고 진단을 보고합니다.
- `volicord connection remove`는 선택된 멤버십을 제거하고, 소유 멤버십이 남지 않으면
  연결도 제거합니다. 일치하는 관리 호스트 설정만 제거하며 프로젝트, Core 상태,
  Runtime Home, 아티팩트 저장소, 관련 없는 호스트 설정은 삭제하지 않습니다.

규칙:

- `volicord connection add`는 기본적으로 Runtime Home의 모든 프로젝트를 연결하면 안 됩니다.
- 선택 프로젝트는 항상 저장소 루트에서 해석되며 명령이 지속 프로젝트 등록을 필요로
  하면 자동 등록됩니다.
- shared 의도는 [런타임 경계](runtime-boundaries.md#explicit-integration-files-in-product-repositories)가
  허용하는 명시적 통합 파일만 쓸 수 있습니다.
- 같은 생성 호스트 대상의 기존 비관리 호스트 설정은 충돌입니다. 일치하는
  Volicord 관리 내용은 소유 명령으로만 갱신하거나 제거할 수 있습니다.
- 호스트 신뢰, 프로젝트 신뢰, 프로젝트 MCP 승인, OAuth, 재시작, 설정 다시 불러오기, 그 밖의
  호스트 통제 동작은 계속 사용자 통제 호스트 동작입니다.

<a id="agent-connection-result-states"></a>
<a id="agent-setup-result-states"></a>
## 연결 결과 상태

Agent Connection 명령은 아래 결과 상태를 사용합니다.

| 상태 | 의미 |
|---|---|
| `not_verified` | 선택된 Agent Connection에 현재 기록된 검증 결과가 없습니다. 호스트가 실패했다는 증거가 아닙니다. |
| `complete` | 오래 유지되는 Agent Connection 상태가 있고, 관리 호스트 설정이 존재하며 예상 관리 지문과 일치하고, 필요한 호스트 로드 가능성 및 신뢰 게이트가 충족되고, CLI MCP 시작과 초기화가 실패하지 않으며, 관리 호스트 도구 호출 증거 또는 명시적으로 신뢰할 수 있는 다른 활성 도구 노출 출처가 활성 Codex 도구 노출을 확인합니다. |
| `action_required` | 오래 유지되는 Agent Connection 상태와 호스트 설정은 있지만 호스트 신뢰, 프로젝트 승인, OAuth, 설정 다시 불러오기, 재시작, 명령 링크 복구, 설치 프로필 복구, 또는 그와 비슷한 사용자 통제 동작이 남아 있습니다. |
| `failed` | 요청한 명령이나 검증이 사용할 수 있는 오래 유지되는 Agent Connection 상태, 사용할 수 있는 호스트 설정, 또는 필요한 로컬 전제 조건을 만들지 못했습니다. |
| `dry_run` | 명령이 영속 변경 없이 계획된 동작을 보고했습니다. |

Codex 연결 검증은 아래 진단 개념을 분리해 유지합니다.

| 진단 개념 | 텍스트 출력 | JSON 진단 | 의미 |
|---|---|---|---|
| MCP 설정 일치 | `MCP configuration` 또는 `Current MCP configuration` | `managed_config`를 포함한 호스트 점검 세부사항과 관리 설정 필드 | 항목이 기대하는 명령, 인자, 관리 시작 마커와 일치합니다. 허용된 도구 승인 추가 설정은 이 일치를 바꾸지 않습니다. 관리 마커가 없으면 `managed_config=unmanaged`를 보고할 수 있습니다. 명령, 인자, 마커가 다르면 계속 불일치입니다. |
| Codex 도구 승인 정책 | 있을 때 `Codex tool approval policy` | 있을 때 `verification.host.host_policy_overlay`와 `id=codex_tool_approval_policy`인 `checks[]` 항목 | Codex 소유 `tools.<known Volicord tool>.approval_mode` 하위 테이블은 `kind=codex_tool_approval`로 나타납니다. 진단은 `entries[].tool`과 `entries[].approval_mode`를 포함합니다. 호스트 신뢰, 활성 도구 노출, 실행 중인 세션의 승인을 증명하지 않습니다. |
| CLI MCP 사전 점검과 핸드셰이크 | `CLI MCP preflight`, `CLI MCP handshake`, `Last CLI MCP preflight`, `Last CLI MCP handshake` | `id=cli_mcp_preflight`와 `id=cli_mcp_handshake`인 `checks[]` 항목, 검증 보고서 필드 | CLI 검증 경로가 Volicord MCP 서버를 직접 시작하고 통신했음을 나타냅니다. CLI가 관찰할 수 있는 MCP 프로세스 검증이며 활성 Codex 도구 노출을 뜻하지 않습니다. |
| CLI MCP 저장 기능 | `CLI MCP storage read`, `CLI MCP storage write`, `CLI MCP effective tools` | 가능할 때 `id=cli_mcp_storage_read`, `id=cli_mcp_storage_write`, `id=cli_mcp_effective_tools`인 `checks[]` 항목 | CLI MCP 검증 프로세스에서 관찰한 저장 기능입니다. 관리 Codex 호스트에서 관찰한 저장 기능과 별개입니다. |
| Codex 프로젝트 신뢰 | `Codex project trust` | 가능할 때 `verification.project_trust`와 `id=codex_project_trust`인 `checks[]` 항목 | Codex 사용자 설정이 프로젝트를 `trusted`, `untrusted`, `unknown`, 또는 그 밖의 신뢰 미확인 상태로 표시합니다. |
| 관리 Codex 시작 | `Managed Codex MCP startup` | 가능할 때 `verification.host_runtime.managed_host_startup`과 `id=managed_host_startup`인 `checks[]` 항목 | Volicord가 선택된 연결에 대해 관리 Codex 호스트 프로세스가 Volicord MCP 서버를 시작했는지 관찰한 상태입니다. |
| 관리 Codex 도구 목록 | `Managed Codex tools/list` | 가능할 때 `verification.host_runtime.managed_host_tools_list`와 `id=managed_host_tools_list`인 `checks[]` 항목 | 관리 Codex 호스트의 `tools/list` 생명주기 이벤트를 관찰했는지 나타냅니다. 이것만으로는 활성 도구 노출을 확인하지 않습니다. |
| 관리 Codex 도구 호출 | `Managed Codex tool call` | 가능할 때 `verification.host_runtime.managed_host_tool_call`과 `id=managed_host_tool_call`인 `checks[]` 항목 | 선택된 연결에 대해 관리 Codex 호스트가 Volicord 도구를 호출했는지 나타냅니다. 이것이 현재 활성 Codex 도구 노출의 완료 증거입니다. |
| 활성 세션 도구 노출 | `Active Codex tool exposure`와 확인이 필요할 때의 `Next` 문구 | `verification.active_tool_exposure`, `verification.host_runtime.active_tool_exposure`, `primary_next_action`, `actions[]`, `connection.user_actions[]` | 활성 Codex 도구 노출이 확인됨, 미확인, 알 수 없음 중 어느 상태인지 나타냅니다. 수동 점검, 권한 상승 점검, CLI 사전 점검, 직접 핸드셰이크, 출처 없는 이전 관찰은 이를 확인하지 않습니다. |
| 관리 호스트 저장 기능 | `Managed host storage read`, `Managed host storage write`, `Managed host effective tools` | 가능할 때 `verification.host_runtime.managed_host_storage`와 `id=managed_host_storage_read`, `id=managed_host_storage_write`, `id=managed_host_effective_tools`인 `checks[]` 항목 | 관리 Codex 호스트 생명주기에서 관찰한 저장 기능입니다. CLI MCP 저장 기능과 별개입니다. |
| 호스트 MCP 명령 실행 가능성 | `Host MCP command` | 가능할 때 `verification.host_mcp_command`와 `id=host_mcp_command`인 `checks[]` 항목 | 설정된 MCP 명령이 `absolute`, `PATH-resolved`, `remote/executor-backed`, `unknown`, `malformed` 중 어느 시작 방식인지와 `host_path_unconfirmed` 같은 위험 세부사항을 나타냅니다. 실제 시작 실패가 확인되지 않았다면 PATH 위험은 경고입니다. |
| Codex 도구 스냅샷 또는 목록 문제 | `Next` 문구가 Codex MCP 시작 또는 도구 목록 로그 확인을 안내할 수 있습니다. | Codex 호스트 로그이며 Volicord 소유 JSON 필드가 아닙니다. | Codex가 MCP 서버의 존재를 알거나 `startup_complete`를 기록해도 활성 세션에는 캐시된 도구 스냅샷이나 나열된 `volicord.*` 도구가 없을 수 있습니다. |

허용되는 Codex 도구 승인 정책 추가 설정 형태는 아래와 같습니다.

```toml
[mcp_servers.volicord.tools."volicord.intake"]
approval_mode = "approve"
```

`volicord` 서버 항목의 명령, 인자, Volicord 관리 환경 변수 마커가 계속 일치한다면 이
추가 설정만으로 `managed_config`가 `changed`가 되거나 `mcp_config_changed` 다음 동작이
나오면 안 됩니다. Volicord 관리 마커가 없는 `volicord` 서버 항목은 비관리로 보고될 수
있으며, 명령, 인자, 관리 마커의 차이는 계속 설정 불일치입니다.

Claude Code 연결 검증은 런타임을 향한 Claude Code 어댑터를 사용합니다. `shared` 프로젝트
설정에서는 프로젝트 `.mcp.json`의 `mcpServers.<server_name>` 항목이 관리 식별 정보이며,
기대하는 명령, 인자, 환경, 관리 지문과 일치해야 합니다. `personal`과 `global` 설정에서는
Volicord가 Claude Code CLI 대상을 사용하고 `claude mcp get
<server_name>` 출력을 기대하는 관리 항목과 비교합니다.

Claude Code 검증은 아래 호스트 상태를 보고할 수 있습니다.

| 호스트 상태 | 의미 |
|---|---|
| 연결됨·일치함 | `claude mcp get <server_name>`이 명령, 인자, 환경, 범위가 Volicord 관리 설정과 일치하는 연결된 서버를 보고합니다. |
| 승인 대기 | Claude Code가 MCP 서버를 프로젝트 승인 대기 상태로 보고합니다. 사용자가 Claude Code에서 승인할 때까지 결과는 `action_required`로 남습니다. |
| 거절됨 | Claude Code가 MCP 서버가 거절되었다고 보고합니다. |
| 없음 | Claude Code가 기대한 이름의 MCP 서버를 보고하지 않거나 프로젝트 `.mcp.json` 항목이 없습니다. |
| 변경됨·비관리 | 기대한 이름의 서버는 있지만 명령, 인자, 환경, 범위, 지문, 소유권이 Volicord 관리 항목과 맞지 않습니다. |
| 사용 불가·알 수 없음 | `claude` 실행 파일을 사용할 수 없거나, 명령이 실패했거나, 출력 형태를 안전하게 해석할 수 없습니다. |

이 Claude Code 검증은 관리 설정과 Claude Code가 `claude mcp get` 또는 프로젝트
설정 파일을 통해 노출하는 호스트 상태만 증명합니다. 그 자체만으로 활성 Claude Code
세션의 도구 노출, 관리 생명주기 시작, 관리 `tools/list`, 관리 도구 호출 증거, 실행 중인
호스트 세션의 저장소 기능, 이후 도구 선택, 보고된 호스트 조건을 넘어서는
사용자 승인을 증명하지 않습니다.

`verification.host_runtime`은 관리되는 Codex 생명주기 단계 필드
`managed_host_startup`, `managed_host_tools_list`,
`managed_host_tool_call`을 각각 `observed`, `not_observed`, `unknown`으로
보고합니다. 생명주기 증거가 해당 데이터를 담고 있을 때는
`active_tool_exposure`와 관리 호스트 저장 진단도 보고할 수 있습니다.
선택된 연결과 프로젝트에 대해 메타데이터가 `host_kind=codex`이고
`launch_origin=managed_host`인 생명주기 이벤트만 이 필드에 계산됩니다. CLI 사전 점검,
직접 핸드셰이크 또는 점검 시작, 수동 시작, 출처 없는 이전 관찰은 이 필드를 충족하지
않습니다. 관리 `tools/list` 이벤트만 있고 관리
도구 호출이 없으면 활성 도구 노출은 미확인으로 남습니다.

### 검증 출력

- 연결 상태와 검증 텍스트는 간결한 `Status`, `Checks`, `Next`, `Diagnostics` 섹션을
  사용합니다. 전체 상태, 시도했거나 차단된 모든 점검, 필요한 다음 행동을 보여 줍니다.
- `action_required` 또는 저하된 탐지 프로필 진단에서 `Next`는 호스트 설정 다시 불러오기,
  재시작, 승인, 설정 복구, 후속 `volicord connection verify ...` 명령 중 필요한 행동을
  구체적으로 표시합니다. `action_required`는 치명적 오류가 아니라 성공한 관리 결과입니다.
- JSON은 최상위 `status`, `checks`, `actions`, `summary_card`를 포함합니다. Codex 신뢰,
  런타임 관찰, 명령 시작, MCP 핸드셰이크 상태는 서로 합치지 않습니다.
- 탐지 프로필 진단은 파일 설치, 설정 상태, 런타임 관찰 상태, 종합 탐지 상태, 설정 다시
  불러오기 필요 여부, 프롬프트 캡처 사용 가능 여부, 마지막 훅 이벤트를 분리합니다. 정확한
  JSON 필드는 [미리보기와 JSON 출력](#setup-output)에 나열합니다.
- 일치하는 이벤트가 있기 전에는 설치되거나 설정된 파일을 활성 훅 관찰로 보고하지
  않습니다. 부분 세션 감시 범위를 전체 미기록 변경 탐지로 보고하지 않습니다.
- JSON은 `disclosure.guarantee_class=detective_observation`과 함께 OS 샌드박싱, 네트워크
  격리, 악성 코드 방어, 전체 쓰기 방지, 행위자 귀속, 정확성, 테스트 충분성, 사람 검토
  대체에 대한 안정적인 `non_guarantees` 값을 담습니다.

성공한 `volicord mcp --check` 시작 점검, CLI MCP 사전 점검, 직접 MCP 핸드셰이크만으로는
Agent Connection을 `complete`로 설명하면 안 됩니다. 이는 CLI가 관찰할 수 있는 환경에서
MCP 프로세스가 시작되는지 검증하는 것일 뿐입니다. 그 자체만으로 Codex, Claude Code 또는
다른 외부 호스트가 프로젝트 설정을 로드, 신뢰, 승인, 초기화, 노출했다는 증명은 아닙니다.
Codex에서는 활성 세션이 도구 스냅샷을 캐시했거나 `volicord.*` 도구를 나열했다는
증명도 아닙니다.

<a id="authority-bundle-export"></a>
## 권한 번들 내보내기

`volicord export authority-bundle --output PATH [--repo PATH] [--json]`는 이미
등록된 하나의 `Product Repository`에 대한 로컬 Volicord 기록의 무결성 라벨이 붙은
복사본을 내보냅니다. `--repo`를 생략하면 현재 디렉터리에서 Git 저장소 root를
해석합니다. `--output`은 아직 없거나 이미 비어 있는 디렉터리를 가리켜야 합니다.

명령은 아래 파일을 씁니다.

- `manifest.json`: 선택된 Runtime Home, 등록 프로젝트, 내보낸 기록 수, 아티팩트
  복사 상태, 파일, 체크섬 경로, 비보장을 설명합니다.
- `records.jsonl`: 프로젝트 `state.sqlite` 저장소 행을 `database`, `table`, `row`
  필드가 있는 JSON Lines로 담습니다.
- `artifacts/`: 현재 로컬 아티팩트 저장소가 해당 바이트를 제공할 수 있을 때 영속
  아티팩트 본문 파일을 복사해 둡니다. 본문이 복사되지 않은 경우에도 아티팩트 행은
  `records.jsonl`과 `manifest.json`에 남습니다.
- `checksums.sha256`: `manifest.json`, `records.jsonl`, `README.txt`, 복사된
  아티팩트 본문 파일에 대한 SHA-256 체크섬을 담습니다.
- `README.txt`: 번들 내용과 보장 한계를 설명합니다.

규칙:

- 권한 번들 내보내기는 선택된 Runtime Home과 프로젝트 상태를 읽기만 하며 Runtime
  Home 기록을 만들거나, 등록하거나, migration하거나, 복구하거나, 갱신하지 않습니다.
- 체크섬 파일은 내보낸 복사본에 라벨을 붙입니다. Runtime Home이 내보내기 전에
  한 번도 수정되지 않았다는 증명이 아닙니다.
- 이 번들은 변조 방지 저장소, 암호학적 서명, 외부 감사 로그, 정확성 증명,
  테스트 충분성 증명, 검토 완료 증명, 배포 증명, 최종 수락, 잔여 위험 수락이
  아닙니다.
- JSON 출력은 출력 경로, 번들 파일 경로, 기록 수, 아티팩트 수, 복사된 아티팩트 수,
  체크섬 항목 수를 보고합니다.

<a id="external-host-configuration"></a>
## 호스트 MCP 설정

공개 `volicord export` 표면은 `volicord export authority-bundle`입니다. Volicord는
일반 외부 MCP 호스트 설정을 렌더링하는 공개 명령을 제공하지 않습니다. 지원 호스트
설정은 `volicord init`과 `volicord connection add`를 통해 수행됩니다. 이 명령들은
선택된 호스트 어댑터가 관리 대상을 소유할 때 지원 호스트 설정을 직접 씁니다.
호스트 중립 또는 그 밖의 미지원 외부 호스트는 사용자 관리 설정 표면으로 남습니다.

규칙:

- 지원되는 관리 호스트 설정은 Agent Connection에 묶이며, 묶인
  `volicord mcp --stdio` 프로세스를 시작합니다.
- 사용자 관리 외부 호스트 설정은 지원되는 Agent Connection이 존재한 뒤 설치된
  `volicord` 실행 파일과 `mcp --stdio --connection <connection_id>
  [--project <project_id>]` 인자를 이름 붙일 수 있습니다.
- Volicord는 임의 외부 호스트가 사용자 관리 설정을 로드, 신뢰, 승인, 초기화, 노출했다고
  주장하면 안 됩니다.

<a id="guard-hook-commands"></a>
## 내부 탐지 훅 생명주기 명령

숨겨진 내부 훅 명령군은 에이전트 생명주기 이벤트 때 명령을 실행하는 생성 호스트 래퍼의
로컬 진입점입니다. 일반 최상위 도움말에는 표시되지 않고 일반 사용자 대상 명령군도
아닙니다. 훅 명령은 등록된 프로젝트 상태를 검사하고 호스트 관찰 이벤트를 기록하며 기계
판독 가능한 로컬 결정을 반환합니다. Core 메서드, 사용자
소유 판단, 쓰기 티켓, 닫기 준비 상태 점검, 호스트 신뢰, 셸 승인, OS 수준 sandboxing을
대체하지 않습니다.

각 호스트 훅 명령은 기본적으로 stdin에서 JSON 훅 이벤트 하나를 읽습니다. `--file PATH`는
테스트나 이벤트를 파일에 준비하는 호스트 통합을 위해 그 파일에서 JSON 이벤트를
읽습니다. 기본 `--output volicord-json` 출력은 `decision`, `allowed`,
`guard_event_id`, 선택적 `session_id`, 명령별 `result`를 포함합니다.
`--output text`는 사람이 읽기 쉬운 짧은 한 줄 출력을 선택합니다. 지원되는 결정은
`allow`, `deny`, `warn`, `inject_context`입니다.

`--host-output codex|claude-code`는 설치된 호스트 훅에 맞는 호스트 고유 출력 방식을
선택합니다. 이 모드에서 stdout은 호스트가 인식하는 응답 JSON이나 맥락만 포함하거나,
호스트가 출력을 기대하지 않으면 비어 있습니다. stdout에는 Volicord 래퍼 JSON을 쓰지
않습니다. 저장되는 호스트 훅 이벤트에는 Volicord가 사용하는 내부 결정과 결과 세부
정보가 계속 남습니다.
Volicord 래퍼 JSON은 `disclosure.guarantee_class=cooperative_host_decision`을
포함합니다. 호스트 고유 출력은 맥락이나 거절 이유에 짧은 협력형 결정 고지를 포함해야
합니다. 호스트 훅 결정은 OS 수준 집행, 네트워크 격리, 악성 코드 방어,
행위자 귀속 증명, 전체 쓰기 방지, 정확성 증명, 테스트 충분성 증명, 인간 검토 대체가
아닙니다.

프로젝트 선택은 `--repo PATH`, 이벤트의 프로젝트나 저장소 필드, 또는 현재 작업
디렉터리를 사용합니다. 훅 이벤트에 `connection_id`가 없으면 `--connection ID`로
Agent Connection 식별 정보를 제공합니다. `--session ID`, `--guard-installation ID`,
`--host HOST`, `--integration-profile record|detective`로 기록할 세션, 설치, 호스트 종류,
통합 프로필을 고정할 수 있습니다. 호스트 종류는 `codex`, `claude_code`, `generic` 같은
저장소 값을 사용합니다. 공개 통합 프로필은 `record`와 `detective`입니다.
`--policy-hash HASH`는 생성된 훅 래퍼 스크립트가 기대하는
`.volicord/policy.json` 해시를 고정합니다. 해시가 맞지 않으면 그 훅 이벤트는 호스트 훅
설치를 활성화할 수 없지만, 테스트나 디버깅에 쓰는 내부 훅 명령은 이 옵션을 생략할
수 있습니다.

생성된 Codex 훅 설정은 현재 작업 디렉터리와 무관하고 하위 디렉터리에서도 안전해야
합니다. 단순 `.codex/hooks/...` 경로를 호출하지 않습니다. 각 훅 항목은 아래 형태의
POSIX `sh`
명령을 실행합니다.

```sh
root=$(git rev-parse --show-toplevel) || exit $?
exec "$root/.codex/hooks/volicord-dispatch.sh" PHASE
```

생성된 `.codex/hooks/volicord-dispatch.sh` 스크립트는 Volicord 관리 파일입니다. 이
스크립트는 런타임에 Git 작업 트리 루트를 다시 해석하고 절대 경로를 요구합니다. 선택된
단계 래퍼가 존재하고 실행 가능한지 확인한 뒤 해당 루트 아래의 래퍼를 실행합니다. Git
루트를 해석할 수 없으면 호스트 세션의 현재 작업 디렉터리로 대체하지 않고 실패합니다.

생성된 Claude Code 훅 설정도 현재 작업 디렉터리와 무관하고 하위 디렉터리에서 안전해야 합니다.
`${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-pre-tool.sh` 같은
`${CLAUDE_PROJECT_DIR}` 기준 exec-form 명령과 빈 args를 사용합니다.

`.codex/hooks/`와 `.claude/hooks/` 아래의 생성 래퍼 스크립트는 stdin을 변경하지 않고
숨겨진 내부 훅 명령군으로 전달합니다. stdout, stderr, 호스트 훅 종료 코드를 보존하며,
기대하는 호스트 종류, 호스트 고유 출력 방식, 저장소 선택자, Agent Connection, 호스트 훅
설치, 정책 해시를 전달합니다. 사용자는 생성된 훅 명령을 단순 `.codex/hooks/...` 또는
`.claude/hooks/...` 상대 경로로 바꾸면 안 됩니다.

탐지 프로필을 인식하는 상태, 검증, doctor 진단은 `hook_path_safety`,
`hook_commands_cwd_independent`, `hook_commands_subdirectory_safe`,
`generated_config_verified`를 보고합니다. 훅 경로 안전성은
`relative_path_unsafe`, `wrapper_missing`, `wrapper_not_executable`,
`absolute_path_stale`, `placeholder_unsupported`, `host_output_mismatch`,
`policy_hash_mismatch` 같은 값을 보고할 수 있으며, 전체 값 집합은
[API 값 집합](api/schema-value-sets.md#state-and-blocker-values)이 담당합니다. `ok`가 아닌
훅 경로 안전성 값은 해당 보기에서 탐지용 호스트 훅을 비활성 상태로 유지합니다. 복구 동작은
`volicord init --host HOST --repo PATH --profile detective`로 안전한 관리 명령을 다시 생성한
뒤, 계속 보고되는 호스트 신뢰, 승인, 설정 다시 불러오기, 재시작 동작을 완료하는 것입니다.

`detective` 호스트 훅 명령이 기록된 프로젝트, Agent Connection, 호스트 훅 설치,
호스트 종류, 통합 프로필, 정책 해시, 알려진 훅 단계와 일치하는 유효한 이벤트를 받으면
Volicord는 관찰 메타데이터를 기록합니다. 필요한 훅 설정이 완전하고 설치가 `degraded`,
`stale`, `broken` 상태가 아닐 때만 그 관찰이 탐지 프로필 설치 기록을 `active`로 승격할
수 있습니다. 프로젝트, 연결, 호스트 종류, 통합 프로필, 정책 해시, 훅 단계 데이터가 맞지
않으면 설치를 활성화하지 않습니다. `active`는 Volicord가 현재 사용할 수 있는 탐지
설정에 대해 일치하는 훅 이벤트를 관찰했다는 뜻입니다. OS 수준 집행, 샌드박싱, 행위자
증명, 쓰기 방지를 주장하지 않습니다.

입력 이벤트 계약은 호스트 중립입니다. 호스트 훅 파서는 호스트 종류, 세션, 도구 이름,
명령, 프롬프트, 결과, 변경 경로의 일반적인 필드 위치를 관대하게 읽습니다. 알 수 없는
필드는 저장되는 호스트 훅 이벤트의 가려진 대상 정보에 보존합니다. 프롬프트 형태 필드는
기본적으로 해시하거나 생략합니다. 프롬프트 캡처 기록은 프롬프트 해시를 저장하고 본문은
생략합니다.

생명주기 동작:

- `session-start`는 Agent Session을 기록하거나 재사용하고, 호스트 세션 주입용으로
  간결한 프로젝트, 현재 작업, 쓰기 티켓, 대기 판단, 차단 사유, 미해결 변경
  맥락과 함께 `inject_context`를 반환합니다.
- `pre-tool`은 읽기 전용, 명확한 변경, 불확실한 도구 시도를 분류합니다. 읽기와 상태
  명령은 차단 사유를 만들지 않고 허용됩니다. 제품 파일 쓰기 시도는 현재 작업이 없거나,
  현재 활성 쓰기 티켓 행이 없거나, 시도 대상이 선택된 `Product Repository` 밖에 있거나,
  관찰된 경로가 활성 쓰기 티켓 범위 밖에 있거나, 활성 티켓 대조가 모호하거나, 정책이
  명확한 변경 셸 명령을 차단할 때 `deny` 또는 `warn`을
  반환할 수 있습니다. 이러한 결정은 협력형 호스트 결정이며 OS 수준 집행이 아닙니다.
  불확실한 셸 명령은 탐지용 호스트 훅 정책이 `deny`를 요구하지 않으면 기본적으로
  `warn`입니다. `pre-tool`이 구체적인 저장소 내부 경로 집합, 현재 작업, 정확히 하나의
  현재 활성 일치 쓰기 티켓, 호환되는 프로젝트 범위를 가진 명확한 제품 파일 쓰기를
  허용하면 예상 쓰기 상관 행을 기록합니다. 이 행은 프로젝트, 연결, 세션, 선택적 호스트
  호출 식별 정보, 도구 종류, 정확한 경로 정책, Task/Change Unit/쓰기 티켓
  근거, 타임스탬프 메타데이터를 담습니다. 읽기 전용, 불확실한, 모호한, 티켓 범위 밖
  명령은 예상 쓰기 행을 만들지 않습니다.
- `post-tool`은 관찰된 도구 결과를 기록합니다. 이벤트가 변경된 `Product Repository`
  경로를 제공하면 먼저 같은 프로젝트, 연결, 세션, 제한된 시간 창, 정확한 경로 정책의
  이전 예상 쓰기 행과 맞춰 봅니다. 호스트가 호출 식별 정보를 제공하면 그 식별 정보를
  사용합니다. 예상 쓰기 행이 일치하지 않으면 `post-tool`은 변경 경로를 정확히 하나의
  현재 활성 일치 쓰기 티켓에 연결할 수 있습니다. 일치한 범위 안 쓰기는 미해결 미기록
  변경 행을 만들지 않습니다. 일치하지 않았거나, 티켓 범위 밖이거나, 모호한 Product
  Repository 변경은 미해결 미기록 변경 행을 기록하고 `warn`을 반환합니다. `post-tool`
  관찰과 대조는 호스트 관찰 기록이지 제품 정확성, 행위자 신원, 쓰기 방지 증명이 아닙니다.
  변경을 찾기 위해 신뢰할 수 없는
  명령을 실행하지 않습니다.
- `prompt-capture`는 현재 호스트, 프로젝트, 연결의 프롬프트 캡처 사용 가능 상태가
  `configured`, `observed`, `active`일 때만 프롬프트 캡처 메타데이터를 기록하고 엄격한
  채팅 판단 명령을 인식합니다. 프롬프트에는
  `Volicord: answer J-3 1 #AB7K`, `Volicord: answer J-3 reject #AB7K`,
  `Volicord: answer J-3 defer #AB7K`, `Volicord: note J-3 "text" #AB7K` 같은 명시적
  줄이 있어야 합니다. 지원되지 않거나, 설정되지 않았거나, 다시 읽어야 하거나,
  저하된 프롬프트 캡처는 `prompt_capture_unsupported`,
  `prompt_capture_not_configured`, `prompt_capture_reload_required` 같은 구조화된
  비기록 출력을 하나의 다음 행동과 함께 반환합니다. 명령이 아닌 프롬프트는 프롬프트
  캡처를 사용할 수 있을 때만 정상적으로 진행됩니다. 형식이 잘못되었거나,
  모호하거나, 알 수 없거나, 코드가 없거나, 코드가 틀렸거나, 오래되었거나, 이미
  답했거나, 프로젝트나 연결이 맞지 않는 판단 명령은 판단을 기록하지 않고 `deny`를
  반환합니다. 유효한 명령은 로컬 `User Channel`을 통해 지정된 대기 판단을
  `actor_source=local_user`와
  `resolved_verification_basis=user_prompt_submit_hook`으로
  기록하고, 프롬프트 캡처 저장소에는 전체 프롬프트 본문을 생략하며, 그 명령을 일반
  에이전트 지시로 다루지 않고 모델에 보이는 기록 완료 맥락을 반환합니다.
- `stop`은 현재 작업을 완료로 다뤄도 되는지 점검합니다. 현재 작업에 대해 `allow`를
  반환하기 전에 닫기 데이터를 포함한 최신 Core 상태 응답을 가져옵니다.
  `base.response_kind=result`인 응답만 이 결정에 사용할 수 있는 최신 상태 근거입니다.
  응답 종류가 `result`가 아니거나 필요한 상태 필드가 누락되었거나 형식이 잘못되었으면
  이유 코드 `authoritative_refresh_failed`와 함께 `deny`를 반환합니다. 이 거절에서
  `result.close_status.authoritative_refresh`는 인식된 `response_kind` 값만 담고, 값이
  누락되었거나 형식이 잘못되었으면 `null`을 담으며, 유효한 공개 `ErrorCode` 값으로만
  구성된 `error_codes` 목록을 함께 담습니다. Core 오류 메시지, 오류 세부사항, 요청 본문,
  응답 본문을 복사하면 안 됩니다. Core 호출 자체가 실패하면 기존 내부 훅 명령 오류
  경로를 유지합니다. 이 갱신은 읽기 전용이며, 갱신 실패 거절은 일반 훅 관찰 기록 외에
  Core 상태 효과를 만들지 않습니다. 유효한 결과에서 닫기 차단 사유가 남아 있거나,
  사용자 소유 판단이 대기 중이거나, 미해결 미기록 변경이 남아 있으면 `deny`를 반환하고,
  그렇지 않으면 `allow`를 반환합니다.

## 변경 조정 명령

`volicord changes reconcile [--repo PATH] [--task active|ID] [--dry-run]
[--json]`은 미해결 미기록 Product Repository 변경을 위한 로컬 복구 명령입니다.

선택과 호출 규칙:

- `--repo PATH`가 프로젝트를 선택합니다. 생략하면 현재 디렉터리를 사용합니다.
- 기본값은 현재 작업입니다. `--task`로 다른 작업을 선택할 수 있습니다.
- 명령은 `actor_source=local_user`, `operation_category=local_recovery`로
  `volicord.reconcile_changes`를 호출합니다.
- 출력에는 간결한 요약 카드와 해결된 변경, 대기 판단, 미해결 변경, 최상위 닫기 차단
  사유 전체, 최상위 다음 행동 전체의 개수가 들어갑니다. 개수와 배열 순서는 표시 역할을
  부여하지 않으며 구조화된 `presentation_role` 필드가 그 역할을 나타냅니다. 거절된 Core
  응답은 거절된 CLI 결과로 유지합니다.

`--dry-run`을 사용하면 텍스트와 JSON은 계획된 자동 해결, 사용자 판단이 필요한 변경,
만들어질 판단 요청, 예상 닫기 차단 사유, 다음 행동, 미리보기 고지를 보여 줍니다.
미리보기는 행위자 신원, 의도, 정확성을 증명하지 않습니다. 또한
`project_state.state_version`을 증가시키거나, 변경·재실행 기록을 쓰거나, 닫기 차단 사유를
해결하거나, 사용자 판단을 만들거나, 아티팩트를 스테이징·연결하지 않습니다.

`--dry-run`이 없으면 명령은 결정적인 변경을 해결하거나 대기 중인 사용자 소유 판단을
만들 수 있습니다. 판단에 답하거나, 사용자를 대신해 변경을 수락하거나, 신원·의도·정확성,
검토·테스트 충분성을 증명하거나, 닫기 준비를 완료하지 않습니다. 대기 판단이 만들어지면
사용자는 `Judgment Inbox`에서 답한 뒤 명령을 다시 실행합니다.

## User Channel 명령

<a id="user-channel-commands"></a>
<a id="user-interaction-commands"></a>

`volicord inbox` 명령은 사람이 로컬 CLI에서 `User Channel`을 통해 대기 중인 사용자
판단을 나열하고 답할 수 있는 경로를 제공합니다. 이 명령은 Agent
Connection을 만들거나, MCP 호스트 설정을 설치하거나, Agent Connection이 사용자처럼
동작할 수 있게 하지 않습니다.

초기화된 MCP 클라이언트가 호스트 프롬프트 지원을 선언하면 호스트 프롬프트 입력은
`volicord.request_user_judgment`로 만들어진 대기 판단의 선호 User Channel 입력 방법입니다.
호스트 프롬프트 입력을 사용할 수 없고 채팅 명령 캡처가 `configured`, `observed`,
`active`이면 대체 안내가 현재 검증 코드가 포함된
`Volicord: answer J-3 1 #AB7K` 같은 정확한 채팅 명령을 보여 줄 수 있습니다.
호스트 프롬프트 입력과 채팅 명령 캡처를 모두 사용할 수 없고 어댑터가 로컬 consent URL을
안전하게 노출할 수 있으면, 대체 안내가 짧게 만료되는 일회성 토큰을 쓰는 루프백 consent
URL을 보여 줄 수 있습니다. 터미널의 `volicord inbox` 명령은 호스트 프롬프트 입력, 채팅
명령 캡처, 로컬 consent URL을 사용할 수 없거나, 비활성화되었거나, 저하되었거나, 작업
흐름에 부적합할 때 쓰는 CLI 받은편지함과 수동 점검 경로로 남습니다.

프로젝트 선택은 `--repo PATH` 또는 현재 작업 디렉터리의 저장소 루트를 사용합니다.
작업 선택은 기본적으로 현재 작업을 사용합니다. `--task active`는 이를 명시하고,
`--task ID`는 이름 붙은 작업을 선택합니다.

일반 텍스트 모드 판단 흐름은 `volicord inbox`가 출력하는 안정적인 판단 식별자와
선택지 식별자를 중심으로 합니다. 저장된 판단 참조와 추가 캡처 경로 세부사항은
JSON 출력에서 확인할 수 있습니다.

명령:

- `volicord inbox`는 선택된 작업의 대기 `JudgmentInboxItem` 항목을 나열합니다.
  항목에는 판단 ID, 질문, 선택지 또는 답변 제약, 필수·선택 상태, 선호 캡처 경로,
  사용할 수 있을 때의 로컬 consent URL 또는 CLI 답변 명령 같은 대체 경로가 들어갑니다.
- `volicord inbox answer <judgment-id> --choice <choice>`는 `actor_source=local_user`,
  `operation_category=user_only`, 호환 User Channel 출처, 선택된 선택지의 저장된 기계
  동작과 결과로 `volicord.record_user_judgment`를 통해 Core 생성 선택지 하나를
  기록합니다. `--note`는 메모로만 저장됩니다.
- `volicord inbox open <judgment-id>`는 선택한 판단이 여전히 대기 중인지 검증한 뒤,
  이 CLI 프로세스가 로컬 consent URL을 소유하지 않으므로 `action_required`를
  보고합니다. 사용할 수 있다면 MCP Judgment Inbox 항목에 이미 표시된 URL을
  사용하도록 안내하고, 그렇지 않으면 CLI 답변 명령을 안내합니다.

판단 하나를 기록하는 것은 그 판단만 기록합니다. 최종 수락과 잔여 위험 수락은 별개의
판단 종류와 동작으로 남아야 하며, 이 명령이 둘을 하나로 합치면 안 됩니다.

상태와 받은편지함 목록 출력은 사용자의 다음 행동을 위해 선택된 담당 상태를 보여 주며,
보기를 계산할 수 있을 때 간결한 `summary_card`를 포함합니다. 대기 판단이 있으면 텍스트
출력은 사용할 수 없는 호스트 프롬프트 입력이 채팅 캡처, 로컬 consent URL, CLI
받은편지함 같은 다른 답변 경로를 숨기지 않도록 사용 가능한 답변 경로도 요약합니다. 이
출력은 증거, 최종 수락, 잔여 위험 수락, 닫기 준비 상태를 만들지 않습니다.
`volicord inbox answer`만 대기 중인 해당 판단을 변경하며, 그것도 선택된 Core 생성
선택지를 통해서만 변경합니다.

<a id="dry-run"></a>
## 미리보기와 JSON 출력

`--dry-run`은 영속 변경 없이 계획, 검증, 충돌 감지, 호스트 대상 렌더링, 출력 형태
만들기를 수행합니다.

미리보기가 하지 않는 것:

- `Volicord Runtime Home` 생성
- SQLite 데이터베이스 생성 또는 수정
- SQLite WAL 또는 SHM 파일 생성
- 레지스트리 또는 프로젝트 상태 스키마 초기화나 검증
- 프로젝트, Agent Connection, Connection Projects, 설치 프로필 행, 검증 상태 행
  또는 호스트 훅 설치 행 등록 또는 갱신
- 호스트 설정 파일 생성, 수정, 제거
- `Product Repository` 파일이나 디렉터리 생성, 수정, 제거
- MCP 시작 점검, MCP 초기화, 도구 탐색 호출

텍스트 출력은 사람이 읽을 수 있어야 하며 각 리소스 작업을 `created`, `reused`,
`updated`, `removed`, `skipped`, `conflict`, `planned` 중 하나로 식별해야 합니다.

<a id="setup-output"></a>
JSON 출력은 관리 CLI 출력이지 공개 Volicord API 응답 스키마가 아닙니다. 설정, 연결,
프로젝트, User Channel 상태를 보고하는 명령은 비대화식 운영자가 성공한
완료된 설정과 필요한 사용자 동작을 구분할 수 있을 만큼 구조화된 상태를 포함해야 합니다.

필수 진단 JSON 값:

- `status`: `complete`, `action_required`, `failed`, `not_verified`, 또는 `dry_run`
- `checks[]`: 안정적인 점검 ID, 상태, 요약, 선택 세부사항이 있는 순서 있는 진단 점검
- `actions[]`: 필요하거나 제안되는 사용자 동작. 사용할 수 있을 때 안정적인 동작 ID와
  사람이 읽을 수 있는 명령 또는 안내를 포함합니다.
- init JSON은 선택된 `connection.connection_intent`와
  `connection.host_scope`를 보고합니다. dry-run과 적용 출력은 같은 해석 값을
  사용합니다.
- `summary_card`: 요약 카드를 계산하는 상태형 진단 또는 User Channel 출력을 위한
  안정적인 간결 요약 데이터
- 연결 상태와 검증 JSON은 CLI MCP 사전 점검과 핸드셰이크, `project_trust`,
  `host_runtime`, `active_tool_exposure`, `host_mcp_command`, CLI MCP 저장
  기능, 관리 호스트 저장 기능에 대한 별도 Codex 진단을 노출할 수 있습니다.
  대응 `checks[]` 항목에는 가능할 때 `cli_mcp_preflight`, `cli_mcp_handshake`,
  `codex_project_trust`, `managed_host_startup`, `managed_host_tools_list`,
  `managed_host_tool_call`, `active_tool_exposure`, `host_mcp_command`,
  `cli_mcp_storage_read`, `cli_mcp_storage_write`, `cli_mcp_effective_tools`,
  `managed_host_storage_read`, `managed_host_storage_write`,
  `managed_host_effective_tools`가 포함됩니다. 이 진단은 신뢰 상태, CLI MCP 시작,
  관리 호스트 런타임 관찰, 활성 도구 노출, 명령 시작 위험, 저장 기능을 CLI MCP
  핸드셰이크 성공과 구분합니다.
- 탐지 프로필을 인식하는 설정, doctor, 연결 상태, 연결 검증 JSON은 탐지 진단을 보고하는
  곳에서 `selected_profile`, `control_surface`,
  `cooperative_pre_tool_warning_available`,
  `cooperative_pre_tool_denial_available`, `post_tool_correlation_available`,
  `bash_shell_mutation_coverage`, `hook_path_safety`,
  `hook_commands_cwd_independent`, `hook_commands_subdirectory_safe`,
  `prompt_capture_available`, `local_web_consent_available`를 노출해야 합니다.
  `control_surface.os_enforced`는 Volicord가 OS 수준 집행을 구현하지 않는 한 `false`여야
  합니다. `guard_health` JSON은 더 엄격한 호스트 훅 전제조건을 보여 주기 위해
  `generated_config_verified`, `native_host_output_adapter_verified`,
  `direct_file_write_matcher_coverage`도 노출할 수 있습니다. 감시기 진단을 보고하는 경우
  JSON은 `watcher_status`, `watcher_baseline_created_at`, `watcher_coverage_start_at`,
  `watcher_coverage_basis`, `watcher_partial_coverage_warning`, `watcher_scan_summary`도
  노출해야 합니다. `watcher_scan_summary`는 `files_scanned`, `files_skipped`,
  `unreadable_paths_count`, `degraded_reasons`, `degraded_reason_counts`,
  `skipped_paths_sample`, `skipped_paths_truncated`, `default_excluded_paths`,
  `max_file_size_bytes`, `max_file_count`, `follows_symlinks=false`,
  `not_full_filesystem_monitoring=true`를 보고합니다.

`volicord doctor --privacy-footprint`는 선택된 `Volicord Runtime Home`에 대한 읽기 전용
진단 보고서입니다. 텍스트와 JSON 출력은 Runtime Home에 저장되는 데이터 범주와 개수를
요약하고, 행위자 귀속, 쓰기 방지, 변조 불가능 감사, 전체 파일시스템 감시, OS 강제,
정확성, 테스트 충분성, 검토 완료, 최종 수락, 잔여 위험 수락을 증명하지 않는다고
나열합니다. 이 명령은 저장된 행 본문, Product Repository 파일 내용, 프롬프트 텍스트를
출력하면 안 됩니다.

설정과 doctor JSON은 진단 소비자가 설정 동작 상태와 설치 프로필 상태를 구분할 수
있도록 `status_meaning`을 포함해야 합니다. doctor JSON은 최상위 상태가
`complete`로 남는 경고 전용 후속 동작을 `actions_recommended[]`에, 차단하는 로컬
복구 동작을 `actions_required[]`에 구분해야 합니다.

<a id="noninteractive-approval-behavior"></a>
## 비대화식 동작

비대화식 명령은 누락된 사용자 입력이나 호스트 통제 동작을 위해 프롬프트를 표시하면
안 됩니다. 누락 상태는 일반 결과 모델로 보고해야 합니다. 복구 가능한 사용자 또는
호스트 동작은 `action_required`, 사용법 오류는 종료 코드 `2`, 충돌이나 런타임
실패는 종료 코드 `1`로 보고합니다.

규칙:

- 프로젝트 범위 호스트 MCP 설정에는 명시적 `--shared` 명령 경로가 필요합니다.
  이와 별개인 init의 저장소 로컬 지침, 정책, 프로필별 통합 파일은 지원되는 두 init
  의도 모두에서 명시적 init 명령으로 승인되며, 그 명령이 미리 보여 준 관리 파일로
  제한됩니다.
- 기존 비관리 내용은 충돌입니다. CLI는 관련 없는 호스트 설정이나 제품 파일을 조용히
  교체하면 안 됩니다.
- 포괄적 셸 승인, 쓰기 승인, 호스트 신뢰 결정, 민감 동작 승인, 쓰기 티켓은 이
  관리 계약이 요구하는 명시적 CLI 명령 경로를 대신하지 않습니다.
- 호스트 신뢰, 프로젝트 신뢰, 프로젝트 MCP 승인, OAuth, 재시작, 설정 다시 불러오기 동작은 계속
  사용자 통제 호스트 동작이며 CLI가 대신 제공할 수 없습니다.

## 관리 경계

관리 CLI는 로컬 리소스를 초기화, 등록, 연결, 진단할 수 있습니다. 그 자체로
공개 Volicord API 메서드를 만들지 않으며 Core 권한, 쓰기 티켓 호환성, 증거 충분성,
닫기 준비 상태, 사용자 소유 판단, 수락, 잔여 위험 수락, 아티팩트 권한, 보안 보장을
만들지 않습니다.

담당 문서 경로:

- 공개 메서드 목록과 메서드 경로: [API 메서드](api/methods.md).
- 공통 요청/응답 스키마: [API 코어 스키마](api/schema-core.md).
- Agent Connection, Connection Projects, 행위자 맥락 의미:
  [Agent Connection](agent-connection.md).
- MCP 프로세스 동작: [MCP 전송](mcp-transport.md).
- 런타임 위치와 저장소 쓰기 경계: [런타임 경계](runtime-boundaries.md).
