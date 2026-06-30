# Agent Connection 참조

이 문서는 로컬 MCP 호스트 통합을 위한 Agent Connection과 현재 연결 맥락의 경계를
담당합니다. 요청이 Core에 들어가기 전에 Agent Connection, 연결 의도, 연결
프로젝트, 연결 모드, `actor_source`, `operation_category`를 어떻게 해석하는지
정의합니다.

공개 API 스키마, 메서드 동작, 저장 효과, 보안 보장 의미, `volicord mcp --stdio` 와이어
동작, Core 권한 의미는 이 문서가 정의하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- Agent Connection 의미와 Connection Projects 멤버십 규칙
- 연결 의도 의미: `personal`, `shared`, `global`
- MCP 호스트 호출의 현재 연결 맥락 경계
- `actor_source`와 `operation_category` 출처 경계
- 권한을 지니는 판단 해결에서 User Channel과 Agent Connection의 경계
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
- 관리 setup, 연결, 상태, 검증, 모드, 제거, 프로젝트, export 명령:
  [관리 CLI](admin-cli.md)
- 저장소 배치, 아티팩트 생명주기, 스테이징 핸들 검증: [참조 색인](README.md)에서
  고르는 저장소와 아티팩트 담당 문서
- 보안 보장 의미나 접근 경계 표현: [보안](security.md)
- 권한과 파생 표시의 구분 규칙: [상태 보기와 템플릿 표시 경계](projection-and-templates.md)
- 렌더링 본문 문구, 공개 표시 라벨, 템플릿 표현: [템플릿 본문](template-bodies.md)

## Agent Connection

Agent Connection은 `Volicord Runtime Home` 아래에 `connection_internal_id`와 함께
저장되는 로컬 MCP 호스트 연결 단위입니다. 생성된 MCP 시작은 `connection_id` 프로세스
인자 표기를 사용하지만, 일반 텍스트 모드 사용자 흐름은 [관리 CLI](admin-cli.md)가
담당하는 명령을 통해 호스트, 연결 의도, 저장소 루트로 연결을 선택합니다.

하나의 `volicord mcp --stdio` 프로세스는 Agent Connection 하나에 묶입니다. 생성된 호스트
설정에는 호스트가 그 프로세스를 시작할 수 있도록 저장된 `connection_internal_id`에서
파생된 `connection_id` 프로세스 바인딩 값이 들어갈 수 있습니다. 그 값은 사용자 권한
토큰이 아니며 일반 명령 입력으로 필요하지 않습니다.

저장되는 Agent Connection 필드는 아래를 포함합니다.

- `connection_internal_id`
- `host_kind`
- `intent`
- `host_scope`
- 연결 대상이 프로젝트 범위일 때의 `project_internal_id`
- `server_name`
- `config_target`
- `mode`
- `enabled`
- `managed_fingerprint`
- `last_verification_status`
- 생성 및 갱신 시각

내부 호스트 설정 키 `server_name`의 기본값은 `volicord`입니다.

<a id="lifecycle-and-state-boundaries"></a>
## 생명주기와 상태 경계

Agent Connection 생명주기는 여러 상태 영역에 걸쳐 있습니다. 한 명령이 한 상태
영역을 바꾸더라도 다른 영역은 그대로 둘 수 있습니다.

| 영역 | 저장 또는 담당 위치 | 바꾸는 명령 | 경계 |
|---|---|---|---|
| 설치 프로필 | 선택된 Runtime Home 식별 정보와 MCP 명령 위치를 포함하는 Runtime Home registry 설치 기록입니다. | `volicord setup`. | Setup은 필요한 로컬 설정입니다. 호스트 신뢰 결정, 사용자 판단, 공개 API 메서드가 아닙니다. |
| Agent Connection 레지스트리 상태 | `Volicord Runtime Home` 아래의 `agent_connections` 기록입니다. `connection_internal_id`, 호스트 종류, 연결 의도, `server_name`, `config_target`, `connection.mode`, 활성 상태, 관리 지문, `last_verification_status`를 포함합니다. | `volicord connect`가 기록을 만들거나 갱신하고, `volicord connection mode`가 모드를 바꾸며, `volicord connection verify`가 검증 상태를 갱신하고, `volicord connection remove`는 멤버십 제거 뒤 기록을 제거할 수 있습니다. | 레지스트리 상태는 관리 상태입니다. 호스트 설정 파일이 아니며, 외부 호스트가 MCP 서버를 로드, 신뢰, 승인, 노출했다는 증거가 아닙니다. |
| Connection Projects 멤버십 | 같은 Runtime Home 아래의 `connection_projects` 기록입니다. | `volicord connect`, `volicord project use`, 연결 제거 흐름이 선택된 저장소 루트에 따라 멤버십을 추가, 검증, 제거할 수 있습니다. | 멤버십은 Agent Connection의 프로젝트 허용 목록을 제어합니다. 모든 Runtime Home 프로젝트를 등록하지 않으며 프로젝트 등록, 프로젝트 상태, Core 기록을 삭제하지 않습니다. |
| 호스트 설정 | `config_target`이 가리키는 MCP 호스트 설정 위치 또는 사용자 관리 generic export입니다. | `volicord connect`가 관리 호스트 설정을 설치하거나 갱신합니다. `volicord connection remove`는 안전 점검이 허용할 때 일치하는 관리 내용만 제거합니다. `volicord export mcp-config`는 호스트 중립 설정을 렌더링합니다. | 호스트 설정은 `volicord mcp --stdio`를 시작하지만 외부 호스트 통합 표면으로 남습니다. 레지스트리 상태와 동일하지 않습니다. |
| 검증 상태 | Agent Connection 레지스트리 기록의 `last_verification_status`와 [관리 CLI](admin-cli.md#agent-connection-result-states)가 담당하는 명령 출력입니다. | `volicord connect`와 `volicord connection verify`가 관찰 가능한 setup, 호스트, MCP 시작, MCP 초기화, `tools/list` 점검을 가능한 곳에서 실행합니다. | 검증은 Volicord 쪽 상태와 호스트/MCP 준비 상태를 모두 살펴볼 수 있습니다. MCP 시작 검증만으로는 `complete` Agent Connection이 아닙니다. |
| 호출 가능 여부 | MCP 어댑터가 시작 시점과 공개 도구 호출마다 파생하는 현재 연결 맥락입니다. | `enabled`, 연결 프로젝트 가용성, `connection.mode`, 메서드의 `operation_category`가 영향을 줍니다. | 레지스트리나 프로젝트 상태가 바뀌면 호스트 설정을 다시 쓰지 않아도 호출이 불가능해질 수 있습니다. |
| 제거 | 관리 호스트 내용, `connection_projects`, 그리고 경우에 따라 `agent_connections`입니다. | `volicord connection remove`. | 제거는 `Product Repository`, 프로젝트 등록, 프로젝트 상태, Core 기록, Runtime Home 자체, 아티팩트 저장소, 관련 없는 호스트 설정을 삭제하면 안 됩니다. |

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
  호스트 설정, MCP 서버 지침은 OS 권한, 호스트 신뢰, 비밀 격리, 파일시스템 ACL,
  네트워크 정책, 사용자 권한이 아닙니다.

저장 기록 계열과 DDL은 [저장소 기록](storage-records.md)과 [저장소 DDL](storage-ddl.md)이
담당합니다. 관리 생성, 갱신, 검증, 모드, export, 제거 명령은
[관리 CLI](admin-cli.md)가 담당합니다.

## 연결 의도

연결 의도는 호스트 설정이 어디에서 쓰이도록 의도되었는지 설명합니다. 보안 수준도,
권한 부여도 아닙니다.

| 의도 | 의미 | 추론하면 안 되는 것 |
|---|---|---|
| `personal` | 현재 사용자의 일반 로컬 흐름을 위한 사용자 소유 호스트 설정입니다. | 호스트 신뢰, 사용자 신원, 모든 로컬 프로젝트 접근을 증명하지 않습니다. |
| `shared` | 선택된 `Product Repository` 안의 명시적 통합 파일로만 저장되는 프로젝트 소유 또는 프로젝트 공유 호스트 설정입니다. | Volicord 런타임 상태가 아니며 임의 제품 파일 편집을 승인하지 않습니다. |
| `global` | 지원 호스트의 사용자 전역 호스트 설정입니다. 프로젝트 접근은 계속 저장소 루트 등록과 Connection Projects로 제한됩니다. | 모든 저장소를 연결하지 않으며 프로젝트나 호스트 신뢰를 우회하지 않습니다. |

기준 범위에서 직접 관리하는 호스트 종류는 `codex`와 `claude_code`입니다. 호스트 중립
MCP 설정 내보내기는 별도 export 흐름입니다. 내보낸 설정은 `volicord mcp --stdio`를 시작하는 데
필요한 내부 registry 상태를 사용할 수 있지만, export는 직접 호스트 설치를 위한 일반
연결 의도가 아닙니다.

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
- inactive이거나 그 밖의 이유로 실행 부적격인 유효한 프로젝트는 멤버십이 있어도 실행
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

## 호스트 설정 인벤토리

저장된 Agent Connection은 Volicord가 관리하는 호스트 설정과 검증 상태를 위한 관리
인벤토리입니다. 호스트 설정 파일은 외부 호스트의 운영상 원천으로 남습니다. 레지스트리
기록은 관리 인벤토리와 마지막으로 알려진 검증 상태일 뿐이며 호스트 설정을 대신하지
않습니다.

규칙:

- registry는 `host_kind`, `connection_intent`, 내부 서버 이름, 설정 대상, 모드, 활성
  상태, 관리 지문, 마지막 검증 상태를 저장합니다.
- 호스트 신뢰, 프로젝트 신뢰, 프로젝트 MCP 승인, OAuth, 또는 그와 비슷한 호스트 통제
  승인은 Volicord가 우회할 수 없습니다.
- 호스트 설정 쓰기는 파일 작업으로 성공했더라도 호스트가 아직 서버를 신뢰, 승인, 로드,
  초기화, 노출하지 않았다면 결과 상태가 `action_required`로 남을 수 있습니다.
- `last_verification_status=complete`는 [관리 CLI](admin-cli.md#agent-connection-result-states)가
  담당하는 운영 게이트를 만족한 관리 검증 결과에 대해서만 저장할 수 있습니다. Volicord가
  직접 시작한 MCP handshake만으로는 충분하지 않습니다.
- `last_verification_status=action_required`는 Volicord가 설정을 관리하거나 내보낼 수 있지만
  호스트가 소유한 신뢰, 승인, OAuth, reload, restart, 명령 링크 복구, setup 복구가
  남아 있을 때의 예상 상태입니다.
- 거절됨, 없음, 변경됨, 사용할 수 없음, 알 수 없음 호스트 상태는 `complete` Agent
  Connection 상태가 아닙니다.
- Product Repository 지침, 생성된 호스트 지침, MCP 서버 지침은 도구 선택을 개선할 수
  있지만 강제 메커니즘이 아니며 모델이 항상 Volicord 도구를 선택한다고 보장할 수
  없습니다.

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
   setup 또는 연결 명령을 이름 붙인 실행 가능한 텍스트를 반환합니다.

명시적 선택이 필요할 때 MCP에 보이는 선택자는 호출자 소유 Core 래퍼 필드가 아니라
`volicord.list_projects`가 반환한 `project_selector` 값입니다.

어댑터는 폴더 이름, 임의의 프로세스 현재 작업 디렉터리 값, 호스트 라벨, 저장소가 반환한
첫 행에서 프로젝트를 추측하면 안 됩니다. 호스트 roots는 호스트가 제공한 저장소 루트
근거로만 사용할 수 있습니다. 등록, Connection Projects, 경로 분리 점검을 우회하지
않습니다.

공개 도구 호출이 Core에 들어가기 전에 MCP 어댑터는 아래를 검증해야 합니다.

- Agent Connection이 존재하고 활성화되어 있습니다.
- 선택된 프로젝트가 그 Agent Connection에 명시적으로 연결되어 있습니다.
- 선택된 프로젝트가 active이고 실행 가능합니다.
- 연결 모드가 메서드의 `operation_category`를 허용합니다.

연결 모드와 동작 범주:

| Agent Connection 모드 | MCP를 통해 허용되는 동작 범주 | MCP에 보이는 공개 메서드 도구 |
|---|---|---|
| `workflow` | `read`, `agent_workflow` | `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.prepare_write`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_judgment`, `volicord.check_close`, `volicord.close_task` |
| `read_only` | `read` | `volicord.status`, `volicord.check_close` |

어댑터 소유 `volicord.list_projects` 유틸리티는 `workflow`와 `read_only` 모드 모두에
보입니다. `volicord.check_close`는 읽기 전용 MCP 닫기 준비 상태 도구입니다.
`volicord.close_task`는 워크플로 전용 MCP 변경 도구이며 `read_only` 도구 탐색에 나타나면
안 됩니다.

`volicord.record_user_judgment`는 `operation_category=user_only`입니다. User Channel
경로를 위한 공개 Core API 메서드이지만 Agent Connection에는 노출되지 않습니다. 권한을
지니는 답변을 기록하는 지원 로컬 사용자 경로는
[관리 CLI](admin-cli.md#user-channel-commands)가 담당하는 `volicord user` 명령군입니다.

내부 행위자 형태이며 공개 API 스키마가 아닙니다.

```yaml
InvocationContext:
  actor_source: local_user | system | agent_connection:<connection_id>
  operation_category: read | agent_workflow | user_only | admin_local
  verification_basis: string
  assurance_level: string
```

기준 `assurance_level`은 협력적 로컬 출처를 뜻하며 암호학적 인간 신원 증명이 아닙니다.
권한을 지니는 사용자 판단 해결에는 `actor_source=local_user`, `operation_category=user_only`,
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

## User Channel과 Agent Connection

Agent Connection은 에이전트 대상 연결입니다. 모델이 사용자의 말을 전달하고 있더라도
`User Channel`이 아닙니다.

조건:

- 사람이 대기 중인 판단을 확인하고 Core 생성 선택지를 골라 기록하는 지원 로컬 CLI
  경로는 [관리 CLI](admin-cli.md#user-channel-commands)가 담당하는 `volicord user`
  명령군입니다.
- 초기화된 MCP 클라이언트가 `capabilities.elicitation`을 선언하면
  `volicord mcp --stdio`는 `volicord.request_user_judgment`가 만든 대기 판단에 대해 서버
  시작 elicitation을 User Channel 경로로 사용할 수 있습니다. 전송 동작은
  [MCP 전송](mcp-transport.md#user-judgment-elicitation)이 담당합니다.
- MCP elicitation을 사용할 수 없으면 MCP 대체 안내 텍스트는 그 로컬 경로가 설정되어 있을
  때 prompt-submit hook 경로와 호환되는 채팅 prompt-capture 명령으로 사람 사용자를 안내할
  수 있습니다.
- 권한을 지니는 사용자 판단 해결에는 `actor_source=local_user`,
  `operation_category=user_only`, 호환 User Channel 출처가 필요합니다.
- `actor_source=agent_connection:<connection_id>`는 사용자의 텍스트를 전달해도
  `local_user` 출처가 될 수 없습니다.

에이전트가 할 수 있는 것:

- 메서드 담당 문서가 그 경로를 지원할 때 빠진 사용자 소유 판단을 요청할 수 있습니다.
- 담당 결과가 반환한 대기 판단 상태와 Core 생성 선택지를 표시할 수 있습니다.
- 사람 사용자를 지원되는 `User Channel`로 안내할 수 있습니다.
- 기준 local web `User Channel`은 구현되어 있지 않습니다. 실험적 HTTP serve 전송은 MCP
  전송 경계이지 브라우저 판단 UI가 아닙니다.

에이전트가 하면 안 되는 것:

- Agent Connection에서 권한을 지니는 사용자 결정을 기록하면 안 됩니다.
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
  담당하는 경계 안에서만 `Product Repository` 안의 Volicord 관리 블록이나 호스트별
  규칙 파일을 추가할 수 있습니다.
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
