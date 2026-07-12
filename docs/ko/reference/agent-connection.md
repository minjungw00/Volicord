# Agent Connection 참조

이 문서는 로컬 MCP 호스트 통합에서 Agent Connection과 현재 연결 맥락의 경계를
정의합니다. 요청이 Core에 들어가기 전에 Agent Connection, 연결 의도, 연결 프로젝트,
연결 모드, `actor_source`, `operation_category`를 어떻게 해석하는지 설명합니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- Agent Connection 의미와 Connection Projects 멤버십 규칙
- 연결 의도 의미: `personal`, `shared`, `global`
- MCP 호스트 호출의 현재 연결 맥락 경계
- `actor_source`와 `operation_category` 출처 경계
- 권한 효력이 있는 판단 해결에서 User Channel과 Agent Connection의 경계
- Agent Connection 계층의 저장소 루트 프로젝트 선택과 프로젝트 가용성 경계
- 담당 결과와 Agent Connection 사이의 에이전트 맥락 전달 규칙
- 선택된 Agent Connection이나 현재 연결 맥락을 사용할 수 없거나, 맞지 않거나,
  오래되었거나, 충분하지 않을 때의 대체 표시

이 문서는 담당하지 않습니다.

- API 요청 래퍼, 응답 분기, 스키마 형태, 동작 범주 값 이름:
  [API 코어 스키마](api/schema-core.md), [API 메서드](api/methods.md), 메서드 담당
  문서, [API 값 집합](api/schema-value-sets.md)
- `volicord mcp --stdio` 시작, 프로세스 환경, stdio 프레이밍, 시작 검증, 응답
  래핑, 종료: [MCP 전송](mcp-transport.md)
- 초기 설정, 연결, 상태, 검증, 모드, 제거, 프로젝트, 권한 번들 내보내기 관리 명령:
  [관리 CLI](admin-cli.md)
- 저장소 배치, 아티팩트 생명주기, 스테이징 핸들 검증: [참조 색인](README.md)에서
  고르는 저장소와 아티팩트 담당 문서
- 보안 보장 의미나 접근 경계 표현: [보안](security.md)
- 권한과 파생 표시의 구분 규칙: [상태 보기와 템플릿 표시 경계](projection-and-templates.md)
- 렌더링 본문 문구, 공개 표시 라벨, 템플릿 표현: [템플릿 본문](template-bodies.md)

<a id="surface-stability"></a>
## 표면 안정성

라벨은 [문서 정책](../maintain/documentation-policy.md#surface-stability-labels)의
기준 어휘를 따릅니다.

| 표면 | 안정성 | 비고 |
|---|---|---|
| Agent Connection 의미, 연결 의도, Connection Projects 멤버십, 연결 모드, 현재 연결 맥락 경계 | `stable` | 로컬 통합 계약입니다. OS 권한이나 사용자 권한이 아닙니다. |
| 관리 호스트 생명주기와 검증 관찰 | `beta` | 지원되지만 호스트와 사용 가능한 기능에 따라 달라집니다. |
| 저장된 식별 정보, 프로세스 바인딩 값, 호스트 설정 키, 파생 호출 메타데이터 | `internal` | 공개 MCP 입력에서 호출자 소유 권한처럼 노출하면 안 됩니다. |
| 사람이 읽는 상태, 검증, 대체 안내, 지침 문구 | `diagnostic` | 집중 담당 문서가 명시한 필드만 안정적인 계약입니다. |

## Agent Connection

Agent Connection은 `Volicord Runtime Home` 아래에 `connection_internal_id`와 함께
저장되는 로컬 MCP 호스트 연결 단위입니다. 생성된 MCP 시작은 `connection_id` 프로세스
인자 표기를 사용하지만, 일반 텍스트 모드 사용자 흐름은 [관리 CLI](admin-cli.md)가
담당하는 명령을 통해 호스트, 연결 의도, 저장소 루트로 연결을 선택합니다.

하나의 `volicord mcp --stdio` 프로세스는 Agent Connection 하나에 묶입니다. 생성된 호스트
설정에는 호스트가 그 프로세스를 시작할 수 있도록 저장된 `connection_internal_id`에서
파생된 `connection_id` 프로세스 바인딩 값이 들어갈 수 있습니다. 그 값은 사용자 권한
토큰이 아니며 일반 명령 입력으로 필요하지 않습니다.

레지스트리는 연결의 내부 식별 정보, 호스트와 의도, 설정 대상, 모드, 활성 상태,
관리 지문, 검증 상태, 관련 메타데이터를 저장합니다. 정확한 기록 필드는
[저장소 기록](storage-records.md)과 [저장소 DDL](storage-ddl.md)이 담당합니다. 내부
호스트 설정 키 `server_name`의 기본값은 `volicord`입니다.

<a id="lifecycle-and-state-boundaries"></a>
## 생명주기와 상태 경계

Agent Connection 생명주기는 여러 상태 영역에 걸쳐 있습니다. 한 명령이 한 상태
영역을 바꾸더라도 다른 영역은 그대로 둘 수 있습니다.

- 설치 프로필: Runtime Home 레지스트리 설치 기록은 선택된 Runtime Home 식별 정보와 MCP
  명령 위치를 저장합니다. `volicord init`이 이 필수 로컬 설정을 만들거나 재사용합니다.
  호스트 신뢰, 사용자 판단, 공개 API 메서드가 아닙니다.
- Agent Connection 레지스트리 상태: `agent_connections`는 관리 상태를 저장합니다. Init과
  연결 명령이 생성, 갱신, 검증, 모드 변경, 제거를 수행합니다. 레지스트리 상태는 호스트
  설정이 아니며 외부 호스트가 MCP 서버를 불러오고, 신뢰하고, 승인하고, 노출했다는
  증거도 아닙니다.
- Connection Projects 멤버십: `connection_projects`는 명시적 프로젝트 허용 목록을
  저장합니다. Init과 연결 추가가 멤버십을 추가하거나 검증하고, 제거가 이를 삭제할 수
  있습니다. `volicord project use`는 프로젝트를 등록하지만 멤버십을 추가하지 않습니다.
  멤버십 변경은 프로젝트나 Core 상태를 삭제하지 않습니다.
- 호스트 설정: `config_target` 또는 사용자가 관리하는 일반 대상이 외부 호스트 표면을
  가리킵니다. Init과 연결 추가는 관리 내용을 설치하고, 제거는 안전하게 일치하는 관리
  내용만 삭제합니다. 이 설정은 `volicord mcp --stdio`를 시작하지만 레지스트리 상태는
  아닙니다.
- 검증 상태: `last_verification_status`와 [관리 CLI](admin-cli.md#agent-connection-result-states)가
  담당하는 출력은 최근 점검을 기록합니다. 호스트 설정, 훅 안전성, MCP 시작, 초기화,
  `tools/list` 점검은 관리 생명주기 관찰과 활성 세션 도구 노출과 구분합니다.
- 호출 가능 여부: MCP 어댑터가 시작 시점과 공개 도구 호출마다 파생합니다. `enabled`,
  프로젝트 가용성, `connection.mode`, `operation_category`가 영향을 줍니다. 레지스트리나
  프로젝트 상태가 바뀌면 호스트 설정을 다시 쓰지 않아도 호출할 수 없게 될 수 있습니다.
- 제거: `volicord connection remove`는 관리 호스트 내용, 멤버십, 경우에 따라 Agent
  Connection을 제거합니다. Product Repository, 프로젝트 등록·상태, Core 기록, Runtime
  Home, 아티팩트 저장소, 관련 없는 호스트 설정은 삭제하면 안 됩니다.

Volicord 관리 호스트 설정은 Volicord가 특정 생성 호스트 설정 내용을 소유하고 지문으로
확인한다는 뜻입니다. 이는 호스트 계약에 기록된 내부 호스트 훅 배포 상태와 같지 않습니다.
그 상태는 훅 관련 구현 기록을 위한 검증된 출처를 설명하며, 공개 통합 모드나 보안 경계가
아닙니다.

Agent Connection 검증은 아래 계층을 분리해 유지합니다.

- 호스트 관리 설정 식별 정보: 선택된 연결에 필요한 관리 서버 이름, 명령, 인자, 환경,
  범위, 지문
- 호스트 신뢰·승인·대기 상태: 신뢰, 프로젝트 MCP 승인, OAuth, 승인 대기, 거절 같은
  호스트 소유 조건
- 호스트 정책 추가 설정: 관리 식별 정보가 계속 일치할 때 Volicord 설정 불일치로
  취급하지 않는 호스트 소유 승인 또는 권한 설정
- CLI MCP 사전 점검과 핸드셰이크: Volicord MCP 서버에 대한 터미널 쪽 시작과 프로토콜 점검
- 관리 호스트 시작: 선택된 연결에 대해 관리 호스트 프로세스가 Volicord MCP 서버를
  시작했다는 생명주기 증거
- 관리 호스트 `tools/list`: 관리 호스트 프로세스가 MCP 도구 탐색에 도달했다는 생명주기
  증거
- 관리 호스트 도구 호출: 관리 호스트 프로세스가 Volicord 도구를 호출했다는 생명주기 증거
- 활성 도구 노출: 활성 호스트 세션이 현재 호스트 도구 목록, 도구 검색, 또는 명시적으로
  신뢰할 수 있는 다른 출처에서 Volicord 도구를 볼 수 있다는 증거
- 저장소 기능: 선택된 프로세스 바인딩이 레지스트리와 프로젝트 상태를 읽을 수 있는지,
  워크플로 도구에서 프로젝트 상태를 쓸 수 있는지

CLI 쪽 MCP 사전 점검, `volicord mcp --check`, 직접 MCP 핸드셰이크는 프로세스 시작과
프로토콜 진단입니다. 그 자체만으로 Codex, Claude Code 또는 다른 외부 호스트가 프로젝트
설정을 로드, 신뢰, 승인, 초기화, 노출했다는 증명이 아닙니다.

Codex 프로젝트 범위 MCP 설정에서 Volicord가 관리하는 식별 정보는 `volicord` 서버
이름과 정확히 다음 이식 가능한 프로세스 기술 정보입니다. 명령은
`command="volicord"`, 인자는 `mcp --stdio --discover-repository --host codex`, 환경
맵은 없음입니다. 저장소에서 보이는 이 관리 항목에는 Connection ID, 프로젝트 ID, 절대
명령 경로, Runtime Home 경로, 어떤 환경 키도 들어갈 수 없습니다. Codex 사용자 범위
설정은 계속 로컬 바인딩이므로 선택된 연결·프로젝트 ID와
`VOLICORD_MCP_LAUNCH=managed_host`, `VOLICORD_MCP_HOST=codex`,
`VOLICORD_MCP_CONNECTION_ID=<connection_id>`, 프로젝트 바인딩이 있을 때의
`VOLICORD_MCP_PROJECT_ID=<project_id>` 같은 관리 시작 환경 변수 마커를 담을 수 있습니다.
그 서버 항목 아래의 Codex 소유 도구 승인 하위 테이블은 호스트 정책 추가 설정이며,
Volicord 관리 식별 정보가 아닙니다. 허용된 `tools.<tool>.approval_mode` 설정을 보존해도
호스트 신뢰, 활성 도구 노출, 실행 중인 세션의 승인, 정확성, 테스트 충분성, 사람 검토
완료, 샌드박싱, 행위자 신원을 증명하지 않습니다. 프로젝트 기술 정보가 정확한 형태와
다르거나 로컬 바인딩의 명령·인자·관리 마커가 달라지면 설정 불일치입니다.

규칙:

- Agent Connection은 에이전트 대상이며 로컬 `User Channel`로 동작할 수 없습니다.
- 연결은 호스트 설정 텍스트를 권한으로 취급하지 않고도 켜거나, 끄거나, 제거하거나,
  모드를 바꿀 수 있습니다.
- 연결 등록은 `Volicord Runtime Home`의 모든 프로젝트를 자동으로 부여하지 않습니다.
- 연결은 Connection Projects 기록에 명시적으로 들어 있는 프로젝트나 담당자가 정의한
  저장소 루트 등록 경로로 선택된 프로젝트만 다룰 수 있습니다.
- `connection.mode=workflow`는 기본 Agent Connection 모드입니다. 읽기와 프로젝트
  탐색 동작에 더해 에이전트 워크플로 동작을 노출합니다. 사용자 전용 판단 기록은
  노출하지 않습니다.
- `connection.mode=read_only`는 읽기와 프로젝트 탐색 동작을 노출합니다. 워크플로 쓰기
  역량이 아닙니다.
- `connection_internal_id`, `connection_id` 프로세스 바인딩, 연결 모드, 연결 의도,
  호스트 설정, MCP 서버 지침은 OS 권한, 호스트 신뢰, 비밀값 격리, 파일시스템 ACL,
  네트워크 정책, 사용자 권한이 아닙니다.

저장 기록 계열과 DDL은 [저장소 기록](storage-records.md)과 [저장소 DDL](storage-ddl.md)이
담당합니다. 관리 생성, 갱신, 검증, 모드, 제거 명령은 [관리 CLI](admin-cli.md)가
담당합니다.

## 연결 의도

연결 의도는 호스트 설정이 어디에서 쓰이도록 의도되었는지 설명합니다. 보안 수준도,
권한 부여도 아닙니다.

| 의도 | 의미 | 추론하면 안 되는 것 |
|---|---|---|
| `personal` | 현재 사용자의 일반 로컬 흐름을 위한 사용자 소유 호스트 설정입니다. | 호스트 신뢰, 사용자 신원, 모든 로컬 프로젝트 접근을 증명하지 않습니다. |
| `shared` | 선택된 `Product Repository` 안의 명시적 통합 파일로 저장되는 프로젝트 소유 또는 프로젝트 공유 주 호스트 설정입니다. | Volicord 런타임 상태가 아니며 임의 제품 파일 편집을 승인하지 않습니다. |
| `global` | 지원 호스트의 사용자 전역 호스트 설정입니다. 프로젝트 접근은 계속 저장소 루트 등록과 Connection Projects로 제한됩니다. | 모든 저장소를 연결하지 않으며 프로젝트나 호스트 신뢰를 우회하지 않습니다. |

`volicord init`에서는 `personal`이 기본값이고 `--shared`가 `shared`를 명시적으로
선택합니다. Init은 `global` 연결을 만들지 않습니다. 연결 의도는 주 관리 호스트 대상을
분류합니다. Init이 적용하는 저장소 로컬 지침, 로컬 정책, 프로필별 훅 통합 파일은
별도의 관리 통합 표면이며 저장된 연결 의도나 호스트 범위를 바꾸지 않습니다. 특히
`.volicord/policy.json`은 의도와 무관한 `local_overlay`이고, 생성된 훅 래퍼는
`shared`에서도 로컬 파일입니다.

Product Repository 하나에서 `volicord init`은 선택한 지원 호스트 하나와 활성 저장소
로컬 `personal` 또는 `shared` 통합 하나만 유지합니다. 다른 지원 호스트나 반대 의도를
선택하면 관리 호스트와 훅 상태 보기를 마이그레이션하고 이전 Connection Project를 활성
사용에서 폐기합니다. 단일 로컬 정책에 여러 호스트 통합이나 서로 다른 로컬 통합 의도를
암묵적으로 동시에 활성화하지 않습니다.

`shared` 주 호스트 파일에는 형식이 지정된 저장소 발견 기술 정보만 들어갑니다. Codex는
`volicord mcp --stdio --discover-repository --host codex`, Claude Code는 같은 명령의
`--host claude-code` 형태를 사용합니다. Connection ID, 프로젝트 ID, 절대 실행 파일,
Runtime Home 경로, 어떤 환경 항목도 넣으면 안 됩니다. 따라서 기술 정보 바이트는 다른
복제본에서도 그대로 사용할 수 있지만 Agent Connection과 프로젝트 식별 정보는 각
Runtime Home에 로컬로 남습니다.

저장소 발견 시작 시 MCP 어댑터는 일반 프로세스 환경과 기본값 규칙으로 로컬 Runtime
Home을 선택하고, 호스트 프로세스의 현재 디렉터리에서 정규화된 Git worktree 루트를
찾고, 그 정확한 저장소 루트 등록을 조회합니다. 이어서 그 프로젝트를 Connection
Projects에 포함하고 기술 정보의 호스트와 일치하는 활성 `shared` 프로젝트 범위 Agent
Connection이 정확히 하나인지 요구한 뒤 세션을 해당 프로젝트 하나로 좁힙니다. 등록되지
않은 복제본, 일치하는 연결 없음, 일치하는 연결이 둘 이상인 경우에는 init, verify, list,
중복 제거 동작을 이름 붙인 진단과 함께 닫힌 방식으로 실패합니다. 저장소 메타데이터에서
내부 ID를 가져오거나 파생하지 않습니다.

로컬 정책과 호스트 오버레이에는 Connection/프로젝트 ID, 절대 명령, Runtime Home 선택,
허용 목록에 든 로컬 환경 값을 유지할 수 있지만 공유 MCP 기술 정보로 취급하면 안 됩니다.
이전에 생성된 명시적 로컬 바인딩 형태의 공유 항목은 저장된 관리 지문이 안전한
마이그레이션을 승인할 때만 인식합니다. Init을 다시 실행하면 관련 없는 호스트 내용을
보존하면서 이식 가능한 기술 정보로 한 번 교체하고, 수렴한 뒤에는 무동작이 됩니다. 새
공유 상태 보기는 이전 바인딩 형태를 만들지 않습니다.

기준 범위에서 직접 관리하는 호스트 종류는 `codex`와 `claude_code`입니다. 호스트 중립
MCP 설정은 사용자 관리입니다. 사용자 관리 설정은 지원되는 Agent Connection이 이미
있을 때만 `volicord mcp --stdio`를 시작하는 데 필요한 내부 레지스트리 상태를 사용할 수
있지만, 직접 호스트 설치를 위한 일반 연결 의도는 아닙니다.

## Connection Projects

Connection Projects는 Agent Connection과 등록 프로젝트 사이의 명시적 레지스트리
관계입니다. 사용자 대상 명령은 저장소 루트나 프로젝트 이름으로 프로젝트를 선택하지만,
레지스트리 저장소는 참조 무결성과 출처를 위해 `project_internal_id` 값을 계속
보관합니다.

멤버십 필드:

- `connection_internal_id`
- `project_internal_id`
- 생성 시각
- `connection_internal_id`와 `project_internal_id`의 복합 기본 키

규칙:

- 프로젝트 멤버십은 프로젝트 상태, 경로 분리, 저장소 실행 가능성, Agent Connection
  모드, 메서드 담당 호출 요구사항을 우회하지 않습니다.
- 유효하지 않은 현재 프로젝트 등록은 연결 프로젝트 기록으로 반환하지 말고 Connection
  Projects 목록 조회와 접근 해석에서 거절해야 합니다.
- `inactive`이거나 그 밖의 이유로 실행 부적격인 유효한 프로젝트는 멤버십이 있어도 실행
  시점에 계속 사용할 수 없습니다.
- Connection Project 제거 또는 Agent Connection 비활성화는 호스트 설정을 다시 쓰지
  않아도 효력을 가져야 합니다.
- 연결 프로젝트가 없는 Agent Connection은 저장된 상태로 남을 수 있으며, 호스트 설정도
  디스크에 남을 수 있습니다. 이 저장 상태는 새 `volicord mcp --stdio` 프로세스가 성공적으로
  시작될 수 있다는 뜻이 아닙니다.
- 새 MCP stdio 시작과 시작 점검은 Agent Connection에 연결 프로젝트가 하나도 없으면
  실패합니다.
- 하나 이상의 프로젝트가 연결되어 있을 때 이미 시작된 `volicord mcp --stdio` 프로세스는 호스트
  설정을 다시 쓰지 않아도 이후 멤버십 변경을 관찰할 수 있습니다. 마지막 멤버십이 제거된
  뒤 프로젝트 탐색은 사용 가능한 프로젝트가 없다고 보고할 수 있으며, 프로젝트 라우팅이
  필요한 공개 도구는 정상 진행할 수 없습니다.
- 프로젝트가 연결되고 시작 또는 호출별 프로젝트 점검이 필요한 프로젝트 상태를 검증할 수
  있어야 Agent Connection을 다시 실행할 수 있습니다.

## 호스트 설정 관리 현황

저장된 Agent Connection은 Volicord가 관리하는 호스트 설정과 검증 상태를 추적합니다.
호스트 설정 파일은 외부 호스트가 실제로 사용하는 원천입니다. 레지스트리 기록은 관리
현황과 마지막으로 알려진 검증 상태일 뿐이며 호스트 설정을 대신하지 않습니다.

규칙:

- 레지스트리는 `host_kind`, `connection_intent`, 내부 서버 이름, 설정 대상, 모드, 활성
  상태, 관리 지문, 마지막 검증 상태를 저장합니다.
- 호스트 신뢰, 프로젝트 신뢰, 프로젝트 MCP 승인, OAuth, 또는 그와 비슷한 호스트 통제
  승인은 Volicord가 우회할 수 없습니다.
- 호스트 설정 쓰기는 파일 작업으로 성공했더라도 호스트가 아직 서버를 신뢰, 승인, 로드,
  초기화, 노출하지 않았다면 결과 상태가 `action_required`로 남을 수 있습니다.
- Codex 프로젝트 범위 설정에서는 프로젝트 신뢰, 호스트 런타임 관찰, 활성 세션의
  Volicord 도구 노출, 호스트 MCP 명령 실행 가능성이 별도 진단으로 남습니다. Codex
  프로젝트가 `trusted`여도 Volicord가 아직 Codex 호스트 프로세스가 MCP 서버를 시작한 것을
  관찰하지 못했을 수 있으며, `volicord`처럼 `PATH`로 찾는 명령은 MCP 서버를 시작하는
  환경에서 실행 가능해야 합니다.
- Codex가 MCP 서버 항목을 알거나 시작 완료를 기록하더라도 활성 세션에는 캐시된 도구
  스냅샷이나 나열된 `volicord.*` 도구가 없을 수 있습니다. CLI 쪽 MCP 사전 점검, 직접
  핸드셰이크, 수동 또는 권한 상승 점검, 관리 시작 관찰, 관리 `tools/list` 관찰은 관리
  도구 호출 증거 또는 명시적으로 신뢰할 수 있는 다른 활성 도구 노출 출처를 대신하지
  않습니다.
- Claude Code 관리 검증은 프로젝트 `.mcp.json` 항목과 `claude mcp get` 출력에서 명령,
  인자, 환경, 범위가 일치하는지 점검합니다. `connected`, `pending approval`, `rejected`,
  `missing`, `changed`, `unavailable`, `unknown` 호스트 상태를 보고할 수 있습니다. 현재
  Claude Code 검증만으로는 활성 Claude Code 세션의 도구 노출, 관리 생명주기 시작, 관리
  `tools/list`, 관리 도구 호출 증거, 실행 중인 Claude Code 세션의 저장소 기능을 증명하지
  못합니다.
- MCP 설정 변경 뒤 호스트 프로세스를 완전히 재시작하거나, 설정을 다시 불러오거나, 세션을
  재개하거나 새로 시작해야 할 수 있습니다. 호스트를 시작한 터미널은 나중에 호스트 안에서
  연 터미널과 다른 `PATH`나 설정 스냅샷을 가질 수 있습니다.
- 사람이 읽는 상태와 검증 출력은 대화형 사용자를 위한 진단 요약입니다.
  `volicord connection status`와 `volicord connection verify`에서는 먼저 `Status`,
  `Checks`, `Next`, `Diagnostics`를 읽습니다. 자동화와 전체 진단 점검은
  [관리 CLI](admin-cli.md#setup-output)가 담당하는 `--json` 출력을 사용합니다.
- `last_verification_status=complete`는 [관리 CLI](admin-cli.md#agent-connection-result-states)가
  담당하는 운영 게이트를 만족한 관리 검증 결과에 대해서만 저장할 수 있습니다. Volicord가
  직접 시작한 MCP handshake만으로는 충분하지 않습니다.
- `last_verification_status=action_required`는 Volicord가 지원 호스트 설정을 관리할 수 있지만
  호스트가 소유한 신뢰, 승인, OAuth, 설정 다시 불러오기, 재시작, 명령 링크 복구,
  설치 프로필 복구가 남아 있을 때의 예상 상태입니다.
- 거절됨, 없음, 변경됨, 사용할 수 없음, 알 수 없음 호스트 상태는 `complete` Agent
  Connection 상태가 아닙니다.
- Volicord 관리 `AGENTS.md` 블록을 포함한 Product Repository 지침, 생성된 호스트 지침,
  호스트 규칙 파일, MCP 서버 지침은 도구 선택을 개선할 수 있지만 강제 메커니즘이 아니며
  모델이 항상 Volicord 도구를 선택한다고 보장할 수 없습니다.

<a id="current-connection-context"></a>
## 현재 연결 맥락

현재 연결 맥락은 MCP 도구 호출 하나에 대해 파생되는 로컬 호출 맥락입니다. 로컬
어댑터가 묶인 Agent Connection, 선택된 프로젝트, 호출된 메서드, 어댑터 소유 호출
사실에서 파생합니다. 이는 공개 요청 페이로드가 아닙니다.

MCP 세션은 어댑터 시작 시 저장된 `connection_internal_id`를 가리키는 정확히 하나의
`connection_id` 프로세스 바인딩 값에 묶입니다. 프로젝트 선택은 Agent Connection에
등록된 저장소 루트와 사용할 수 있는 경우 호스트가 제공한 프로젝트 맥락에서 해석됩니다.
공개 MCP 도구 입력 스키마는 내부 요청 래퍼, 프로토콜 메타데이터, `connection_id`,
`project_id`, `actor_source`, `operation_category`, 검증 근거 필드를 호출자 소유 입력으로
노출하면 안 됩니다.

공개 MCP 메서드 호출의 프로젝트 선택은 결정적입니다.

1. 선택된 Agent Connection에 사용 가능한 프로젝트가 정확히 하나이면 이미 묶인 그
   프로젝트를 사용합니다.
2. 연결이 호스트 제공 저장소 루트를 볼 수 있으면 그 루트를 연결된 등록 프로젝트 하나와
   대조합니다.
3. 그 밖의 경우 호출을 모호하거나 사용할 수 없는 상태로 거절하고 상태를 고칠 저장소 루트
   설정 또는 연결 명령을 이름 붙인 실행 가능한 텍스트를 반환합니다.

명시적 선택이 필요할 때 MCP에 보이는 선택자는 호출자 소유 Core 래퍼 필드가 아니라
`volicord.list_projects`가 반환한 `project_selector` 값입니다.

어댑터는 폴더 이름, 임의의 프로세스 현재 작업 디렉터리 값, 호스트 라벨, 저장소가 반환한
첫 행에서 프로젝트를 추측하면 안 됩니다. 호스트 roots는 호스트가 제공한 저장소 루트
근거로만 사용할 수 있습니다. 등록, Connection Projects, 경로 분리 점검을 우회하지
않습니다.

공개 도구 호출이 Core에 들어가기 전에 MCP 어댑터는 아래를 검증해야 합니다.

- Agent Connection이 존재하고 활성화되어 있습니다.
- 선택된 프로젝트가 그 Agent Connection에 명시적으로 연결되어 있습니다.
- 선택된 프로젝트가 `active`이고 실행 가능합니다.
- 연결 모드가 메서드의 `operation_category`를 허용합니다.

연결 모드와 동작 범주:

| Agent Connection 모드 | MCP를 통해 허용되는 동작 범주 | MCP에 보이는 공개 메서드 도구 |
|---|---|---|
| `workflow` | `read`, `agent_workflow` | `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.prepare_write`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_judgment`, `volicord.reconcile_changes`, `volicord.check_close`, `volicord.close_task` |
| `read_only` | `read` | `volicord.status`, `volicord.check_close` |

어댑터 소유 `volicord.list_projects` 유틸리티는 `workflow`와 `read_only` 모드 모두에
보입니다. `volicord.check_close`는 일급 Core 읽기 메서드에 매핑되는 읽기 전용 MCP
닫기 준비 상태 도구입니다. `volicord.close_task`는 워크플로 전용 MCP 변경 도구이며
`read_only` 도구 탐색에 나타나면 안 됩니다.

위 표는 모드 기준 허용 목록입니다. 실제 MCP `tools/list` 출력은 선택된 프로젝트 저장소를
읽고 쓸 수 있는지에도 제약됩니다. 전송 수준 도구 탐색과 읽기 전용 저장소
축소 노출 규칙은 [MCP 전송](mcp-transport.md#tool-discovery-and-toolscall-response-wrapping)이
담당합니다.

읽기 전용 연결 점검은 관리 검증과 활성 MCP 읽기 호출을 함께 사용합니다. 터미널에서는
`volicord connection verify`를 실행하고, 활성 호스트 세션에서는
`volicord.list_projects`와 `volicord.status`를 호출합니다. 이 경로는 설정, 프로젝트
탐색, 활성 읽기 도구 노출, 읽을 수 있는 프로젝트 상태를 검증합니다. `Task` 생성을
요구하면 안 됩니다.

워크플로 쓰기 경로 간단 점검은 Agent Connection 워크플로 도구를 사용하며 Volicord
상태를 만들 수 있습니다. 최소 경로는 `volicord.intake`, `volicord.update_scope`,
`volicord.record_run`, 닫기에 최종 수락이 필요할 때 `volicord.request_user_judgment`,
그리고 `volicord.check_close`를 포함할 수 있습니다. 만들어진 `Task`는 사용자가 지원되는
`User Channel`을 통해 필요한 최종 판단을 기록할 때까지 `missing_final_acceptance`로 막힌
상태에 남을 수 있습니다.

[테스트 전략](../architecture-guide/testing-strategy.md)의 명시적 실제 판단 테스트 하네스는
설치된 호스트로 더 작은 연결 왕복을 실행합니다. 표식 `Task` 생성, 제품 결정 판단 생성,
호스트 고유 MCP User Channel을 통한 사람의 답변, 그 결과인 Task 상태 새로 고침을
확인합니다. 저장된 해결 근거가 `mcp_elicitation_user_channel`이어야 합니다. 대기 CLI
inbox 대체 경로는 실행 가능한 복구이지만 성공한 고유 왕복으로 세지 않습니다. 이
테스트 하네스는 기본적으로 무시되며 이식 가능한 호스트 적합성이나 보안 테스트가 아닙니다.

`volicord.record_user_judgment`는 `operation_category=user_only`입니다. User Channel
경로를 위한 공개 Core API 메서드이지만 Agent Connection에는 노출되지 않습니다. 권한을
지니는 답변을 기록하는 지원 로컬 사용자 경로는
[관리 CLI](admin-cli.md#user-channel-commands)가 담당하는 `volicord inbox` 명령군입니다.

내부 행위자 형태이며 공개 API 스키마가 아닙니다.

```yaml
InvocationContext:
  actor_source: local_user | system | agent_connection:<connection_id>
  operation_category: read | agent_workflow | user_only | admin_local | local_recovery
  verification_basis: string
  assurance_level: string
```

기준 `assurance_level`은 협력적 로컬 출처를 뜻하며 암호학적 인간 신원 증명이 아닙니다.
권한 효력이 있는 사용자 판단을 해결하려면 `actor_source=local_user`, `operation_category=user_only`,
호환 User Channel 출처, 메서드가 정의한 호환성이 필요합니다. Agent Connection은 복사된
사용자 텍스트나 생성 지침을 제출해 사용자 권한을 얻을 수 없습니다.

조건:

- 공개 API 요청 하나에는 파생된 `InvocationContext`가 정확히 하나 있습니다.
- 내부 프로젝트 선택은 Agent Connection의 연결 프로젝트로 제한됩니다. 호출자 권한이
  아니며 목록에 없거나 inactive이거나 무효인 프로젝트 접근을 부여할 수 없습니다.
- MCP에 보이는 공개 도구 스키마는 `actor_source`, `operation_category`, `connection_id`,
  `project_id`, 요청 메타데이터, 프로토콜 래퍼 필드를 노출하지 않습니다. 원시 MCP 인자에
  이 필드가 들어 있으면 어댑터는 Core 실행 전에 호출을 거절합니다.
- `ArtifactInput`이나 `StagedArtifactHandle` 같은 중첩 페이로드는 두 번째 호출 맥락을
  추가하지 않습니다.
- 해결된 권한 판단의 권한 출처 필드는 호출자 텍스트, 라벨, 답변 본문, 복사된 참조,
  생성된 Markdown, Product Repository 지침이 아니라 파생된 `InvocationContext`에서
  옵니다.
- 보호된 읽기, 상태 변경, 아티팩트 동작은 메서드 담당 문서가 파생된 호출 맥락을
  받아들일 때만 그 호출에 의존할 수 있습니다.

에이전트가 할 수 있는 것:

- 담당 결과 맥락을 표시하거나 전달할 때 파생된 호출 맥락을 보존할 수 있습니다.
- 맥락이 없거나 호환되지 않으면 사용 불가, 불일치, 오래됨, 충분하지 않은 Agent
  Connection 상태로 표시할 수 있습니다.

에이전트가 하면 안 되는 것:

- `InvocationContext`를 요청 페이로드로 제출하면 안 됩니다.
- `verified=true`를 스스로 주장하면 안 됩니다.
- Agent Connection에서 `actor_source=local_user`나 `operation_category=user_only`를 제출해
  사용자 권한을 만족시키면 안 됩니다.
- 임의의 검증 근거 문구를 공개 요청 권한으로 제출하면 안 됩니다.
- 스테이징된 아티팩트 출처를 꾸며 내면 안 됩니다.
- 복사된 식별자, 생성된 Markdown, 대화 텍스트, 상태 보기 텍스트, 에이전트 기억을 현재
  연결 맥락의 대체물로 쓰면 안 됩니다.

담당 문서 링크:

- 정확한 요청 래퍼와 응답 형태는 [API 코어 스키마](api/schema-core.md),
  [API 메서드](api/methods.md), 메서드 담당 문서가 담당합니다.
- `operation_category` 값 이름은 [API 값 집합](api/schema-value-sets.md)이 담당합니다.
- `volicord mcp --stdio` 시작, 연결 바인딩, 환경 변수, stdio 프레이밍, 시작 검증, 응답 래핑,
  종료는 [MCP 전송](mcp-transport.md)이 담당합니다.

<a id="user-channel-and-agent-connections"></a>
## User Channel과 Agent Connection

Agent Connection은 에이전트 대상 연결입니다. 모델이 사용자의 말을 전달하고 있더라도
`User Channel`이 아닙니다.

조건:

- 사람이 대기 중인 판단을 확인하고 Core 생성 선택지를 골라 기록하는 지원 로컬 CLI
  경로는 [관리 CLI](admin-cli.md#user-channel-commands)가 담당하는 `volicord inbox`
  명령군입니다.
- `volicord.record_user_observation`은 `user_only`이며 Agent Connection에 노출되지
  않습니다. `volicord inbox observe`가 대상 결합 User Channel Evidence를
  기록합니다. Agent Connection은 일반 `record_run` 주장, staged artifact, tool
  metadata, raw guard payload를 사용자 소유 producer/relevance 레코드 대신 사용할
  수 없습니다.
- 초기화된 MCP 클라이언트가 `capabilities.elicitation`을 선언하면
  `volicord mcp --stdio`는 `volicord.request_user_judgment`가 만든 대기 판단에 대해 서버
  시작 사용자 입력 요청을 User Channel 경로로 사용할 수 있습니다. 전송 동작은
  [MCP 전송](mcp-transport.md#user-judgment-elicitation)이 담당합니다.
- 호스트 프롬프트 입력을 사용할 수 없으면 MCP 대체 안내 텍스트는 명령 캡처가
  `configured`, `observed`, `active`일 때 프롬프트 제출 훅 경로와 호환되는 채팅
  명령으로 사람 사용자를 안내할 수 있습니다.
- 호스트 프롬프트 입력과 채팅 명령 캡처를 사용할 수 없으면 MCP 대체 안내 텍스트는
  사람 사용자를 [MCP 전송](mcp-transport.md#user-judgment-elicitation)이 담당하는
  루프백 로컬 consent URL로 안내할 수 있습니다. 이 웹 답변은 여전히 `local_user` User
  Channel 경로이지 Agent Connection 답변이 아닙니다. 동의 페이지는 사용자에게 대기
  판단과 비보장을 식별해 보여 주며, Agent Connection이 판단에 답할 권한을 만들지
  않습니다.
- 대체 안내 텍스트는 호스트 프롬프트 입력, 채팅 명령 캡처, 로컬 consent URL을 모두
  사용할 수 없을 때만 사용자를 `volicord inbox` CLI inbox 경로로 안내합니다.
- 상태 보기와 판단 받은편지함은 호스트 프롬프트 입력, 채팅 명령 캡처, 로컬 consent URL,
  CLI 받은편지함의 User Channel 사용 가능 상태를 함께 보여 줄 수 있습니다. 호스트
  프롬프트 입력을 사용할 수 없다는 사실이 다른 사용 가능한 답변 경로를 숨기면 안 되며,
  적용 가능한 경우 CLI 받은편지함은 계속 보입니다. 이 상태 보기는 사용자가 어디에서 답할 수
  있는지 알려 줄 뿐이며 Agent Connection이 판단을 기록할 수 있게 하지 않습니다.
- 권한 효력이 있는 사용자 판단을 해결하려면 `actor_source=local_user`,
  `operation_category=user_only`, 호환 User Channel 출처가 필요합니다.
- `actor_source=agent_connection:<connection_id>`는 사용자의 텍스트를 전달해도
  `local_user` 출처가 될 수 없습니다.

에이전트가 할 수 있는 것:

- 메서드 담당 문서가 그 경로를 지원할 때 빠진 사용자 소유 판단을 요청할 수 있습니다.
- 담당 결과가 반환한 대기 판단 상태와 Core 생성 선택지를 표시할 수 있습니다.
- 사람 사용자를 지원되는 `User Channel`로 안내할 수 있습니다.

에이전트가 하면 안 되는 것:

- Agent Connection에서 권한 효력이 있는 사용자 결정을 기록하면 안 됩니다.
- Agent Connection 도구 인자를 MCP elicitation 응답으로 취급하면 안 됩니다.
- 자연어 승인, 채팅 답변, 생성된 Markdown 상태, 렌더링된 상태 보기를 User Channel
  출처로 취급하면 안 됩니다.
- 선택지 하나를 최종 수락, 잔여 위험 수락, 민감 동작 승인, 범위 수락, 또는 다른 판단
  종류로 넓히면 안 됩니다.
- 표시된 판단 문구에서 증거 충분성, 수락, 잔여 위험 수락, 닫기 준비 상태, 보안 권한을
  만들면 안 됩니다.

담당 문서 링크:

- [Core 모델](core-model.md)은 사용자 소유 판단, 최종 수락, 잔여 위험 수락, 증거,
  닫기 준비 상태의 권한 의미를 담당합니다.
- [사용자 판단 기록 메서드](api/method-record-user-judgment.md)는 대기 판단 하나를
  해결하는 공개 메서드 동작을 담당합니다.
- [상태 보기와 템플릿 표시 경계](projection-and-templates.md)는 생성 표시와 상태 보기
  권한 경계를 담당합니다.

## 에이전트 동작 지침

에이전트 동작 지침은 두 계층으로 나뉩니다.

- MCP 서버 지침은 MCP 초기화 중 서버가 항상 제공합니다.
- 선택적 `Product Repository` 지침은 관리 명령이 지원하고 사용자가 명시적으로 승인한
  경우에만 설치됩니다.

규칙:

- MCP 서버 지침은 Volicord 도구 전체에 적용되는 도구 간 흐름, 프로젝트 선택 규칙,
  제한을 설명할 수 있습니다.
- 선택적 저장소 지침은 [런타임 경계](runtime-boundaries.md#explicit-integration-files-in-product-repositories)가
  담당하는 경계 안에서만 `Product Repository` 안의 Volicord 관리 `AGENTS.md` 블록이나
  호스트별 규칙 파일을 추가할 수 있습니다.
- 지침은 도구 선택을 개선할 수 있지만 권한, 접근 통제, 사용자 판단, 보안 강제, 모델이
  Volicord 도구를 선택한다는 증거가 아닙니다.

## 에이전트 맥락 전달

에이전트 맥락 전달은 다음 행동에 필요한 담당 맥락만 에이전트에 제공하되, 그 패킷을
권한 기록으로 만들지 않는 규칙입니다.

조건:

- 에이전트 맥락에는 다음 행동에 필요한 담당 결과와 그 행동에 영향을 주는 현재 연결
  맥락의 한계만 담아야 합니다.
- 맥락 패킷은 지원 맥락일 뿐 Core 상태, 저장소 상태, 증거, 수락, 잔여 위험 수락,
  닫기 출력이 아닙니다.

에이전트가 할 수 있는 것:

- 현재 `Task` 요약, 현재 적용 범위, `state_version`, 대기 중인 사용자 소유 판단, 차단
  사유, 다음 안전한 행동, 증거와 아티팩트 요약, 닫기 준비 상태와 잔여 위험 요약,
  담당 문서가 뒷받침하는 보장 표시, 출처 또는 제한 메모를 담은 압축 맥락을 전달할 수
  있습니다.
- 다음 행동에 필요할 때만 정확한 담당 문서 섹션을 가져올 수 있습니다.
- 한영 문서 유지보수에서 의미 일치 검토가 필요할 때만 같은 `doc_id`의 두 언어 문서를
  함께 가져올 수 있습니다.

에이전트가 하면 안 되는 것:

- 전체 스키마, DDL, 과거 로그, 아티팩트 본문, 관련 없는 계약 자료, 지원 범위 밖 기능
  목록, 정확한 템플릿 본문, 같은 `doc_id`의 두 언어 문서를 기본으로 주입하면 안 됩니다.
- 오래되었거나 복사된 맥락 패킷을 담당 결과나 기반 기록보다 최신 권한처럼 취급하면 안
  됩니다.

담당 문서 링크:

- [템플릿 본문](template-bodies.md)은 에이전트 맥락 패킷 문구를 담당합니다.
- [참조 색인](README.md)은 정확한 담당 문서 섹션 경로를 안내합니다.
- [번역 정책](../maintain/translation-policy.md)은 한영 의미 일치 검토 지침을 담당합니다.

## 대체 경계

현재 연결 맥락이나 필요한 연결 모드를 사용할 수 없거나, 맞지 않거나, 오래되었거나,
충분하지 않을 때 대체 표시를 사용합니다.

에이전트가 할 수 있는 것:

- 적절한 연결 모드나 다른 연결 프로젝트로 옮길 수 있습니다.
- 동작을 좁힐 수 있습니다.
- 빠진 사용자 소유 판단을 요청할 수 있습니다.
- 사용자가 그 방식을 명시적으로 선택한 경우에만 Volicord 밖에서 계속할 수 있습니다.

에이전트가 해야 하는 것:

- 제한을 지원 문구나 표시 문구에 드러내야 합니다.
- 기계 판독용 실패 의미는 [API 오류 코드](api/error-codes.md)와
  [API 오류 세부사항](api/error-details.md)으로 보내야 합니다.
- 사용자에게 보이는 문구는 [템플릿 본문](template-bodies.md)으로 보내야 합니다.

에이전트가 하면 안 되는 것:

- 권한을 지어내면 안 됩니다.
- 사용 불가, 불일치, 오래됨, 충분하지 않은 맥락 상태를 일반 성공 문구 속에 숨기면
  안 됩니다.
- 사용자의 명시적 선택 없이 Volicord 밖에서 계속하면 안 됩니다.
