# 런타임 경계

이 문서는 Volicord 구현, Agent Connection, `Product Repository`, `Volicord Runtime Home`, `User Channel`, 외부 MCP 호스트 설정 사이의 구성 요소 경계와 위치 경계를 담당합니다. 이 경계에 적용되는 위치와 연결 권한 가정을 정의하고, 저장소와 보안 세부사항은 각 담당 문서로 안내합니다.

Volicord 구현은 이 저장소가 유지하는 구현 집합입니다. Volicord 전체가 아니며, Core도 아니고, 실행 중인 프로세스 하나도 아니며, Volicord 상태를 위한 로컬 기준 기록도 아닙니다.

## 담당하는 것 / 담당하지 않는 것

| 이 문서가 담당하는 것 | 이 문서가 담당하지 않는 것 |
|---|---|
| 제품/시스템인 Volicord와 저장소가 유지하는 구현 집합인 Volicord 구현의 구분. | 공개 API 동작, 공개 스키마 형태, 메서드별 효과. |
| Volicord 소스 저장소, Volicord 설치, 실행 중인 실행 파일 역할의 구분. | 릴리스 패키징 정책이나 필수 설치 루트 배치. |
| `Product Repository` 정의와 `Product Repository` API 경로 정규화. | 저장소 기록 배치, 잠금, 스키마 초기화, 버전 관리, 아티팩트 생명주기 세부사항. |
| `Volicord Runtime Home` 정의. | API 메서드 동작이나 공개 스키마 형태. |
| Volicord 구현 파일, 제품 파일, 런타임 데이터, 외부 MCP 호스트 설정의 분리. 정확한 Runtime Home/Product Repository 경로 관계 계약을 포함합니다. | 자세한 보안 보장 의미나 보안 비주장. |
| 로컬 파일 접근과 위치가 권한을 만들지 않는다는 규칙. | 상태 보기 권한, 템플릿 본문, 렌더링된 표시의 최신성. |
| 런타임 위치만으로 Volicord 권한, 보안 권한, 격리를 증명할 수 없다는 규칙. | 제품 범위, 닫기 준비 상태, 증거 충분성, 사용자 소유 판단 의미. |

## 구성 요소 모델

Volicord는 제품, 구현, 실행 파일 역할, MCP 호스트 용어, 기준 기록 개념을 구분합니다.

| 용어 | 정의 | 추론하면 안 되는 것 |
|---|---|---|
| Volicord | AI 지원 제품 작업을 위한 로컬 작업 권한 기록. | Core, 소스 저장소, 실행 파일 프로세스 하나로 보면 안 됩니다. |
| Core | Volicord 상태를 위한 로컬 기준 기록. | Volicord 제품/시스템 전체나 어댑터 또는 CLI 실행 파일로 보면 안 됩니다. |
| Volicord 구현 | 이 저장소가 유지하는 구현 집합. 소스 수준에서는 구현 크레이트, `volicord` 관리 CLI, `volicord mcp --stdio` 로컬 MCP 어댑터, 테스트, 문서, 검증 도구, 저장소 설정을 포함합니다. | 모든 가능한 Volicord 제품 노출 경로, Core 자체, `Volicord Runtime Home`, `Product Repository`, 단일 데몬, MCP 서버 항목, 네트워크 서비스로 보면 안 됩니다. |
| Volicord 소스 저장소 | 이 저장소를 체크아웃한 소스 산출물. | 배포된 설치, 실행 중인 프로세스, Runtime Home, Product Repository, MCP 호스트 설정과 같은 것으로 보면 안 됩니다. |
| Volicord 설치 | 배포된 Volicord 실행 파일과 필요한 런타임 리소스의 부분집합. | 모든 설치에 문서, 테스트, 소스 파일, 저장소 메타데이터가 들어 있다고 추론하면 안 됩니다. |
| `volicord` 관리 프로세스 | Volicord 구현 안의 관리 CLI 실행 파일/프로세스. | Volicord나 Volicord 구현 전체와 같은 말로 보면 안 됩니다. |
| `volicord mcp --stdio` MCP 어댑터 프로세스 | Volicord 구현 안의 로컬 stdio MCP 어댑터 실행 파일/프로세스. | Volicord 구현과 별개이거나 그 자체가 Volicord 구현 전체라고 보면 안 됩니다. |
| `Agent Connection` | `connection_internal_id`, 연결 의도, 호스트 범위, `workflow` 또는 `read_only`인 `connection.mode`와 함께 저장되는 로컬 MCP 호스트 연결 단위입니다. | OS 샌드박스, 파일시스템 ACL, 네트워크 정책, 비밀값 격리 장치, 사용자 대상 식별자 요구사항, 사용자 행동 해결 경로로 보면 안 됩니다. |
| `Connection Projects` | 사용자 대상 저장소 루트 선택 뒤 Agent Connection이 다룰 수 있는 `project_internal_id` 값의 명시적 허용 목록입니다. | 기본적으로 등록된 모든 프로젝트를 포함하거나 Product Repository 권한을 증명한다고 보면 안 됩니다. |
| `User Channel` | 판단과 Evidence 관찰을 포함해 권한 효력이 있는 사용자 행동을 기록하는 로컬 사용자 경로입니다. | Agent Connection, MCP 호스트, 생성된 표시, Product Repository 파일로 보면 안 됩니다. |
| MCP 서버 | 외부 MCP 호스트에 노출되는 서버 항목이나 프로세스의 이름으로 쓸 수 있는 일반 MCP 프로토콜 또는 호스트 설정 용어입니다. 호스트가 그 라벨을 사용한다면 `volicord mcp --stdio` 같은 로컬 stdio 어댑터 프로세스를 가리킬 수도 있습니다. | 제품/시스템인 Volicord, Volicord 구현, `volicord`, `volicord mcp --stdio`를 TCP 또는 HTTP 네트워크 서버로 만들지 않으며, Volicord의 제품 라벨도 아닙니다. |

동작을 한 실행 파일 역할이 수행한다면 그 역할의 이름을 씁니다. 의미가 구현 집합 전체에 적용될 때만 단독 Volicord 구현을 사용합니다.

## 파일시스템 위치 모델

Volicord는 구현 파일, 제품 파일, 런타임 데이터, 외부 호스트 설정을 구분합니다. Volicord 구현 집합 전체를 위한 단일 필수 파일시스템 루트는 없습니다.

| 위치 역할 | 정의 | 추론하면 안 되는 것 |
|---|---|---|
| Volicord 소스 저장소 또는 설치 파일 | 소스 체크아웃, 또는 Volicord 구현의 배포된 실행 파일과 필요한 런타임 리소스. | 자동으로 `Volicord Runtime Home`, `Product Repository`, MCP 호스트 설정, Volicord 권한 증거, 본질적인 네트워크 리스너가 된다고 보면 안 됩니다. |
| `Product Repository` | 제품 소스, 제품 문서, 테스트, 설정, 그 밖의 프로젝트 파일을 담는 사용자의 제품 파일 경계. | Volicord 런타임 상태, `Volicord Runtime Home`, Volicord 권한 증거로 보면 안 됩니다. |
| `Volicord Runtime Home` | 저장소/런타임 담당 문서가 정의하는 Volicord 소유 기록, 로컬 런타임 메타데이터, 아티팩트 데이터를 위한 런타임 저장 위치. | `Product Repository`, 기본적인 Volicord 설치 위치, 자동 보안 경계, 기본 격리로 보면 안 됩니다. |
| 외부 MCP 호스트 설정 | `volicord mcp --stdio` 명령, 프로세스 환경, 호스트별 바인딩을 지정할 수 있는 외부 MCP 호스트 소유 설정. | 정의상 Volicord 런타임 상태, `Volicord Runtime Home`, `Product Repository`, Volicord 소스 저장소 또는 설치 파일로 보면 안 됩니다. |

### 런타임과 호스트의 책임

아래 내용은 기준 로컬 Rust 구현의 경계를 요약합니다. 자세한 기록 배치는 [저장소 기록](storage-records.md), 아티팩트 생명주기는 [아티팩트 저장소](storage-artifacts.md), 관리 명령 동작은 [관리 CLI](admin-cli.md), MCP 프로세스 동작은 [MCP 전송](mcp-transport.md)이 담당합니다.

**`Volicord Runtime Home`**

- **포함하는 것:** `registry.sqlite`, 필요할 때 생성되는 비권한 `diagnostics.sqlite`, 프로젝트별 `projects/{project_internal_id}/state.sqlite`, 아티팩트 저장소를 사용할 때의 `projects/{project_internal_id}/artifacts/`입니다. 레지스트리는 Runtime Home 식별 정보와 경로, 설치 프로필, 저장소 루트 기반 프로젝트 등록, 프로젝트 별칭, Agent Connection, Connection Projects 멤버십, 호스트 훅 설치, `managed host configuration state`를 저장합니다. 프로젝트 상태에는 `Task`, Change Unit, 쓰기 티켓, 증거 메타데이터, User Channel 사용자 행동 resolution, 아티팩트, 세션 감시 기록을 저장할 수 있습니다. 별도 진단 데이터베이스는 크기가 제한된 로컬 운영 집계만 저장합니다.
- **사용 경로:** `volicord init`, `project`, `connection`, `inbox`, `changes`, `doctor`, `diagnostics`, 숨겨진 내부 훅 명령이 각 담당 경로에 따라 상태를 초기화하거나 읽거나 갱신합니다. `volicord doctor --privacy-footprint`는 행 본문을 출력하지 않고 저장 범주와 개수를 보고합니다. `volicord diagnostics session`은 일반 설정 점검 뒤 크기 제한 진단 저장소만 읽습니다. `volicord mcp --stdio`, Core, Store는 시작, 프로젝트 처리 경로, Core 상태, 아티팩트, 최선 노력 운영 집계를 위해 Runtime Home 상태를 사용합니다.
- **경계:** Product Repository, 외부 호스트 설정, 설치 디렉터리가 아닙니다. OS 샌드박스, 네트워크 격리, 악성코드·비밀값 검사, 호스트 신뢰, 행위자 귀속, 쓰기 방지, 변조 방지 감사, 전체 파일시스템 감시, 정확성, 테스트 충분성, 검토 완료, 최종 수락, 잔여 위험 수락을 제공하거나 증명하지 않습니다.

**`Product Repository`**

- **포함하는 것:** 사용자 제품 파일과 명시적으로 요청된 통합 파일입니다. 프로젝트 범위 호스트 설정, 탐지형 호스트 훅 정책, 관리 지침이 여기에 해당합니다.
- **사용 경로:** 일반 제품 파일은 사용자나 호스트 도구가 편집합니다. Volicord는 제품 경로를 입력으로 검사할 수 있으며, 담당 문서가 정의한 관리 경로를 통해서만 명시적 통합 파일을 쓸 수 있습니다.
- **경계:** Runtime Home 상태, Core 저장소, 기본 아티팩트 저장소가 아닙니다. 그 내용은 Volicord 권한을 증명하지 않습니다.

**Runtime Home 레지스트리의 `managed host configuration state`**

- **포함하는 것:** `connection_internal_id`, 호스트 종류, 연결 의도, 호스트 범위, 선택적 `project_internal_id`, 내부 서버 이름, 설정 대상, 모드, 활성 상태, 관리 지문, 검증 요약 상태, 검증 보고서 JSON, 사용자 동작 JSON, 호스트 훅 설치 상태, 메타데이터입니다.
- **사용 경로:** `volicord init`, `volicord connection` 명령, 숨겨진 내부 훅 흐름이 레지스트리 행, 호스트 훅 설치 행, Connection Projects 멤버십을 만들고, 갱신하고, 조회하고, 검증하고, 제거합니다.
- **경계:** 외부 호스트 설정 객체가 아닙니다. 호스트가 `volicord mcp --stdio`를 신뢰, 승인, 로드, 초기화, 노출했거나 탐지형 호스트 훅을 실행했다는 사실도 증명하지 않습니다.

**외부 MCP 호스트 설정**

- **포함하는 것:** `volicord mcp --stdio` 프로세스를 지정할 수 있는 호스트 소유 또는 사용자 관리 설정입니다. 개인·로컬 오버레이에는 내부 Agent Connection 바인딩, 절대 명령 경로, init이 선택한 절대 `VOLICORD_HOME`이 들어갈 수 있습니다. 저장소에서 보이는 공유 Codex와 Claude Code 항목에는 로컬 ID나 Runtime Home의 리터럴 경로 없이 형식이 지정된 `volicord mcp --stdio --discover-repository --host <host>` 기술 정보가 들어갑니다. 복제본별 로컬 `VOLICORD_HOME`은 정확한 호스트별 이식 가능 형태로 전달합니다. Codex는 `env_vars = ["VOLICORD_HOME"]`, Claude Code는 `"env": {"VOLICORD_HOME": "${VOLICORD_HOME}"}`를 사용합니다.
- **사용 경로:** 외부 호스트가 로드와 신뢰 결정을 담당합니다. [관리 CLI](admin-cli.md)가 동작을 정의한 경우에만 `volicord`가 지원되는 직접 설정을 쓸 수 있습니다.
- **경계:** Runtime Home 레지스트리 상태나 Core 권한이 아니며 Volicord 권한을 증명하지 않습니다. `Product Repository`에 저장되었다면 명시적 통합 파일일 뿐입니다.

**`volicord` 관리 CLI 프로세스**

- **담당하는 것:** Runtime Home 초기화, 프로젝트 등록, Agent Connection과 Connection Projects 관리, 호스트 설정, 상태 조회, 검증, 모드 변경, 담당 문서가 정의한 안전한 제거입니다.
- **시작 주체:** 로컬 운영자 또는 사용자입니다.
- **경계:** 공개 Volicord API 메서드 경로, OS 보안 강제 계층, 호스트 신뢰 결정, 포괄적인 Product Repository 편집 권한이 아닙니다.

**`volicord mcp --stdio` MCP 어댑터 프로세스**

- **담당하는 것:** 명시적 로컬 ID 또는 고유한 로컬 저장소 발견 결과로 Agent Connection 하나에 묶인 로컬 stdio 자식 프로세스입니다. Volicord가 관리하는 시작에서는 init이 만든 설정에 바인딩된 Runtime Home을 사용합니다. 사용자가 관리하는 명시적 ID 시작은 [MCP 전송](mcp-transport.md)이 정한 프로세스 입력에서 Runtime Home을 해석합니다. 연결 상태를 확인하고, `connection.mode`에 맞는 도구를 노출하고, 허용된 프로젝트를 선택하고, 어댑터가 담당하는 호출 사실을 파생하고, 공개 메서드 호출을 Core와 Store로 전달합니다. 저장소 발견은 명시적으로 전달된 비어 있지 않은 절대 경로 `VOLICORD_HOME`을 요구하고 누락, 빈 값, 상대 경로를 플랫폼 기본값으로 대체하기 전에 거부합니다. 그런 다음 현재 Git worktree를 정규화하고 해당 Runtime Home에서 선택한 등록 프로젝트 하나로 프로세스를 좁힙니다.
- **시작 주체:** `stdin`/`stdout`으로 통신하는 MCP 호스트입니다.
- **경계:** 임의 제품 파일 편집 권한이나 사용자 행동 resolution 기록 권한을 부여하지 않습니다. 호스트 신뢰를 강제하거나 샌드박싱을 제공하거나 MCP 네트워크 전송 리스너를 열지 않습니다. 비활성화하지 않으면 로컬 User Channel 동의를 위한 별도 임시 루프백 전용 HTTP 리스너를 열 수 있지만, 이 리스너는 MCP 전송이 아닙니다.

<a id="runtime-location-product-repository"></a>
### `Product Repository`

`Product Repository`는 사용자의 프로젝트 작업 공간이자 제품 파일 경계입니다.

주장할 수 있는 것:
- 제품 파일은 담당 문서가 정한 Volicord 확인이나 사용자 소유 판단의 입력으로 검사될 수 있습니다.
- 호환되는 제품 파일 쓰기는 현재 적용 범위, 현재 적용 Change Unit, 필요한 판단, 쓰기 티켓 호환성의 지배를 받을 수 있습니다.

주장하면 안 되는 것:
- `Product Repository` 내용이 Volicord 상태라는 주장.
- `Product Repository` 내용이 생성된 Volicord 출력이라는 주장.
- `Product Repository` 내용이 Volicord 권한을 증명한다는 주장.
- `Product Repository`가 자동으로 `Volicord Runtime Home`이라는 주장.

<a id="explicit-integration-files-in-product-repositories"></a>
### `Product Repository`의 명시적 통합 파일

Volicord 런타임 상태, SQLite 데이터베이스, 생성 기록, 런타임 홈, 로그, 상태 보기, QA 결과, 수락 기록, 닫기 준비 상태, 잔여 위험 기록은 `Product Repository`에 쓰면 안 됩니다.

기준 범위에서 허용되는 유일한 예외는 명시적으로 요청된 통합 파일입니다.

- Codex `.codex/config.toml` 또는 Claude Code `.mcp.json` 같은 프로젝트 범위 호스트 설정
- `AGENTS.md` 안의 Volicord 관리 블록
- `.volicord/policy.json`에 있는 의도와 무관한 로컬 정책 오버레이
- Codex `.codex/hooks.json`, 개인 Claude Code `.claude/settings.local.json`, 공유
  Claude Code `.claude/settings.json` 같은 호스트 훅 설정
- Codex `.codex/hooks/` 또는 Claude Code `.claude/hooks/` 아래의 Volicord 관리 호스트 훅
  래퍼 스크립트
- Codex `.codex/rules/*.rules` 또는 `.claude/rules/` 아래의 Claude Code 파일 같은
  Volicord 관리 호스트 규칙 파일
- Git 기반의 모든 init에서 worktree에 실제로 적용되는 Git `info/exclude`의 Volicord
  관리 블록. 이는 추적되지 않는 Git 메타데이터이며 Product Repository 파일이
  아닙니다.

요청된 `guard` 통합 관리 파일을 적용하는 동안 대상 디렉터리에 스테이징, 이전 파일
밀어 두기, 되돌리기를 위한 구현 전용 보조 항목을 사용할 수 있습니다. 이 항목들은
요청된 통합 파일 쓰기 한 번에 속하는 일시적 항목입니다. 추가 기준 통합 파일 종류,
Runtime Home 데이터, 관리 호스트 설정, 오래 유지되는 복구 기록이 아닙니다. 적용이
성공하거나 되돌리기가 검증되면 그 시도가 계속 소유하는 모든 보조 항목을 제거해야
합니다. 동시에 바뀐 바이트를 덮어쓰지 않기 위해 자동 복구를 중단한 경우에는 검사 시
존재한다고 보고한 항목만 남길 수 있습니다. 그 이름과 보존 기간은 안정적인
인터페이스가 아닙니다. 수동으로 삭제하거나 교체하기 전에 보고된 항목을 검사해야
합니다.

Record profile init에서 기본 개인 Codex 연결은 Codex 사용자 설정 대상을 사용하고,
명시적 `--shared`는 프로젝트 범위 `.codex/config.toml` 대상을 추가합니다. 두 의도
모두 저장소 로컬 `.volicord/policy.json`과 `AGENTS.md` 안의 Volicord 관리 안내
블록을 적용합니다. Claude Code 개인 init은 MCP 등록에 로컬 CLI 대상만 사용합니다.
공유 init은 저장소 `.mcp.json` 프로젝트
파일을 주 호스트 대상으로 선택합니다. 개인 Claude Code 탐지 훅은
`.claude/settings.local.json`을 사용하고 공유 탐지 훅은 `.claude/settings.json`을
사용합니다. 두 의도 모두 `.gitignore`를 바꾸지 않고 Git `info/exclude`를 통해
`/.volicord/`와 생성된 훅 래퍼 스크립트를 추적 대상에서 제외합니다. 독립형 개인
init은 개인 훅 설정과 규칙 경로도 보호합니다. `.volicord/policy.json`은
`storage_scope=local_overlay`를 선언하고 선택한 `connection_intent`를 기록하므로 공유
상태 보기로 커밋하면 안 됩니다. 생성된 래퍼 스크립트도 프로세스 바인딩 경로와
식별자를 담으므로 로컬 파일입니다. 모든 관리 생명주기 또는 최종 출력 래퍼는 init이
선택한 절대 `VOLICORD_HOME`을 내보내고 설치 프로필의 절대 `volicord_command`를
실행합니다. 호스트 환경의 기존 Runtime Home 값이나 `PATH`로 해석되는 단순 명령을 신뢰하지
않습니다.

Product Repository 하나에서 이 저장소 로컬 표면은 선택한 내장 호스트 어댑터, 활성 의도,
프로필을 각각 하나씩 나타냅니다. 선택한 호스트, 의도, 프로필 중 하나를 바꾸는 init은
여러 주체가 소유한 파일의 관련 없는 내용을 보존하고, 일치하는 Volicord 소유 이전
호스트·반대 의도·더 이상 적용되지 않는 상태 보기를 폐기하며, 마이그레이션이 성공할
때까지 이전 상태와 요청 상태의 로컬 전용 Git 제외를 합쳐 유지해야 합니다. 안전한
폐기는 계획된 소유 마커, 상태 보기, 관리 지문과 일치할 때만 가능하며 바뀌었거나 비관리
상태인 내용은 충돌입니다.

Codex `.codex/hooks.json`은 Volicord 통합이 파일 전체를 소유하며, 다른 기존 JSON은
충돌입니다. Claude Code `.claude/settings.local.json`은 관리 상태 보기로 관련 없는
설정을 보존하지만 호스트가 파일 전체를 로컬로 취급하므로 개인 init은 경로 전체를
제외합니다. 공유 훅 설정과 규칙 파일은 저장소에 보이는 상태로 남습니다. 이 공유
표면을 커밋할지는 Product Repository 정책 결정입니다. 공유 `.codex/config.toml` 또는
`.mcp.json`의 Volicord MCP 항목은 호스트별 저장소 발견 형태와 정확히 일치할 때만
복제본에서 그대로 쓸 수 있습니다. 명령은 `volicord`, 인자는
`mcp --stdio --discover-repository --host codex|claude-code`이고, 이식 가능한 Runtime Home
전달 설정은 하나입니다. Codex는 `env_vars = ["VOLICORD_HOME"]`, Claude Code는
`"env": {"VOLICORD_HOME": "${VOLICORD_HOME}"}`를 사용합니다. 어느 형태도 Runtime Home
경로를 내장하지 않습니다. Connection/프로젝트 ID, 절대 명령, Runtime Home의 리터럴 경로,
비밀값 형태를 포함한 그 밖의 환경 키는 로컬 오버레이에만 속하며 이 공유 항목에서는
유효하지 않습니다. 각 복제본은 선택한 로컬 Runtime Home에서 고유한 활성 공유 연결로
별도로 등록되어야 하고 시작 호스트는 같은 비어 있지 않은 절대 경로
`VOLICORD_HOME`을 제공해야 합니다.

연결된 worktree에서 공통 `info/exclude`는 모든 형제가 안전하게 읽을 수 있는 의도와
무관한 정책 및 래퍼 경로만 담습니다. 개인 Detective init은 추가 개인 전용 경로를
공통 메타데이터에서 격리할 수 없으므로 파일을 적용하기 전에 거부됩니다. 이런 통합
표면은 Runtime Home 데이터가 아닙니다.

규칙:

- 관리 명령은 쓰기를 적용하기 전에 정확한 대상 경로와 내용을 미리 보여 줘야 합니다.
- 프로젝트 범위 호스트 MCP 설정은 명시적 `shared` 연결 의도 명령 경로를 사용해야
  합니다. 다른 init 소유 저장소 통합 파일은
  [관리 CLI](admin-cli.md#noninteractive-approval-behavior)가 정의한 명시적 init
  명령과 충돌 동작을 사용합니다.
- 쓰기는 Volicord 소유 마커 또는 관리 지문을 사용해야 합니다.
- 기존 비관리 내용은 덮어쓰지 말고 충돌로 보고해야 합니다.
- 교체는 일치하는 Volicord 관리 내용에만 적용할 수 있습니다.
- `guard` 통합의 조건부 생성이나 교체는 오래된 계획을 권한으로 취급하지 말고 바뀐
  대상이나 부모 경로를 거부해야 합니다. 정확한 커밋, 되돌리기, 잔류 항목 보고,
  메타데이터 동작은 [관리 CLI](admin-cli.md#agent-host-setup-and-init)가 담당합니다.
- 안전한 제거는 일치하는 Volicord 관리 내용만 제거할 수 있으며 관련 없는 프로젝트 파일을 그대로 둬야 합니다.
- 이 파일들은 호스트 설정 또는 지침입니다. Volicord 런타임 상태, Core 권한, 증거, 수락, 닫기 준비 상태, 잔여 위험 수락, 보안 보장이 아닙니다.
- 현재 작업 디렉터리와 무관한 호스트 훅 명령과 관리 래퍼 경로 검증은 호스트 설정 상태 점검입니다.
  이 파일들을 Volicord 런타임 상태로 만들지 않으며 OS 샌드박싱, 명령 차단, 네트워크
  차단, 비밀값 차단, 전역 파일시스템 가로채기를 제공하지 않습니다.

`detective`의 감시 범위는 한정된 관찰입니다. 호스트 훅과 세션 감시기는 감시가
시작된 뒤의 미기록 Product Repository 변경을 드러낼 수 있습니다. 상태 조회 또는 닫기
준비 상태 결과는 현재 프로필, 호스트 훅 상태, 세션 감시기 상태, 감시 시작 시각,
스냅샷 상태 시각, 해결되지 않은 미기록 변경 수, 감시 범위의 비보장을 `CoverageSummary`
필드로 보고할 수 있습니다. 이 요약은 전체 파일시스템 감시, 행위자 신원 증명,
쓰기 방지, 변조 불가능 감사, OS 강제, 보안 격리를 제공하지 않습니다.

세션 감시기는 한정된 `Product Repository` 스냅샷 검사기이지 전체 감시
서비스가 아닙니다. 기본적으로 `.git/`, `.hg/`, `.svn/`, `.jj/`, `.volicord/`,
`target/`, `node_modules/`, `dist/`, `build/`, `coverage/`, `vendor/`를 건너뛰며,
Runtime Home/Product Repository 경로 분리 규칙은 선택된 `Volicord Runtime Home`을
스캔되는 저장소 밖에 둡니다. 기본적으로 심볼릭 링크를 따라가지 않습니다. 사용자 대상
`status`, `guard`, `doctor`, 감시 범위 요약은 `files_scanned`, `files_skipped`,
`unreadable_paths_count`, 파일 수 제한, 파일 크기 제한, 읽을 수 없는 경로, 정책상
건너뛴 경로, 건너뛴 심볼릭 링크에 대한 저하 이유별 개수와 건너뛴 경로 샘플을
보고할 수 있습니다.

<a id="product-repository-api-path-normalization"></a>
### `Product Repository` API 경로 정규화

이 규칙은 API, 스키마, 메서드 담당 문서가 어떤 필드를 `Product Repository` 제품 경로로 식별할 때 적용됩니다.

규칙:
- API 제품 경로는 `Product Repository` 안의 저장소 상대 경로입니다.
- 절대 경로는 `Product Repository` API 경로로 무효입니다.
- 경로 정규화는 `.` 세그먼트와 저장소 밖으로 나가지 않는 `..` 세그먼트를 정리합니다. `..` 때문에 저장소 밖으로 벗어나는 경로는 무효입니다.
- `Product Repository` 밖으로 해결되는 심볼릭 링크는 `Product Repository` 경로 필드에서 무효입니다.
- 내부 경로 비교는 정규화된 저장소 상대 경로를 사용합니다.
- API 응답은 정규화된 상대 경로만 기록합니다.

의미하지 않는 것:
- 이 경로 규칙은 OS 샌드박싱, 명령 차단, 네트워크 차단, 비밀값 차단, 또는 기준 범위의 `detective` 강제를 제공하지 않습니다.
- 쓰기 티켓 호환성은 Core 담당 메서드 경로로 기록되는 제안된 제품 파일 변경에만 적용됩니다. 전역 파일시스템 가로채기, 셸 권한, 명령 승인, 쓰기가 실제로 일어났다는 증명이 아닙니다.
- 메서드별 호환성 결정은 API 메서드 담당 문서에 둡니다.

<a id="runtime-location-source-installation-processes"></a>
### Volicord 구현 소스, 설치, 프로세스

Volicord 구현은 이 저장소가 유지하는 구현 집합을 뜻합니다. 코드, 문서, 테스트, 검증 도구, 저장소 설정을 담은 체크아웃에는 Volicord 소스 저장소를 씁니다. 배포된 실행 파일과 필요한 런타임 리소스에는 Volicord 설치를 씁니다.

주장할 수 있는 것:
- `volicord`는 Volicord 구현 안의 관리 CLI/프로세스입니다.
- `volicord mcp --stdio`는 Volicord 구현 안의 로컬 stdio MCP 어댑터 프로세스입니다.
- Volicord 설치는 소스 저장소, `Volicord Runtime Home`, `Product Repository`, MCP 호스트 설정과 다른 위치일 수 있습니다.
- Volicord 설치가 모든 소스 저장소 파일을 포함할 필요는 없습니다.
- 기준 로컬 Rust 구현에서는 MCP 호스트가 `volicord mcp --stdio`를 자식 프로세스로 시작하고 stdio로 통신합니다.

주장하면 안 되는 것:
- Volicord 구현이 Volicord 제품/시스템 전체라는 주장.
- Volicord 구현이 Core 또는 Volicord 상태를 위한 로컬 기준 기록이라는 주장.
- Volicord 구현이 오직 `volicord`, 오직 `volicord mcp --stdio`, 하나의 장기 실행 데몬, 또는 하나의 네트워크 서비스라는 주장.
- `volicord mcp --stdio`가 Volicord 구현 안의 실행 파일 역할이 아니라 Volicord 구현과 별개라는 주장.
- 어떤 디렉터리에서 Volicord를 설치하거나 실행하면 그 디렉터리가 `Volicord Runtime Home`이 된다는 주장.
- 설치 위치가 그곳에 런타임 데이터가 있음을 증명한다는 주장.
- 설치 경로가 Volicord 권한, 보안 권한, 제품 파일 쓰기 권한을 부여한다는 주장.
- Volicord 구현이라는 용어 자체가 TCP, HTTP, 소켓, 또는 그 밖의 네트워크 리스너를 뜻한다는 주장.

### 기준 로컬 MCP 프로세스

현재 로컬 Rust MCP 어댑터는 Volicord 구현 안의 실행 파일 역할인 `volicord mcp --stdio` stdio 프로세스입니다. MCP 호스트는 프로토콜이나 호스트 설정 맥락에서 설정된 항목을 MCP 서버라고 부를 수 있습니다. 그 라벨은 Volicord를 서버 제품으로 만들거나 Volicord 구현을 네트워크 서버로 만들지 않습니다. MCP 호스트는 `volicord mcp --stdio`를 자식 프로세스로 시작하고, 프로세스 환경으로 설정을 전달하며, `stdin`/`stdout`을 통해 줄 단위 JSON-RPC를 주고받습니다. MCP 전송 자체는 TCP, HTTP, Unix 도메인 소켓, 또는 그 밖의 네트워크 전송 리스너를 열지 않습니다. 다만 `VOLICORD_LOCAL_WEB_CONSENT`로 비활성화하지 않으면 같은 프로세스가 로컬 User Channel 동의를 위한 임시 루프백 전용 HTTP 리스너를 열려고 시도합니다. 이 선택적 리스너를 시작하지 못해도 stdio 시작은 계속됩니다. 동의 리스너는 MCP 전송이 아니며, 정확한 동작은 [MCP 전송](mcp-transport.md#local-web-consent-fallback)이 담당합니다.

Volicord 관리 시작에서는 프로세스 환경이 MCP 자식 프로세스를 init이 선택한 Runtime
Home에 묶어야 합니다. 개인·로컬 설정은 선택한 절대 경로를 직접 제공합니다. 복제본에서
그대로 쓰는 공유 설정은 경로를 내장하지 않고 시작 호스트의 `VOLICORD_HOME`을 전달하며,
저장소 발견 시작은 그 값이 존재하고 비어 있지 않을 것을 요구합니다. 플랫폼 기본 Runtime
Home으로 대체하지 않으며 상대 경로도 거부합니다.

별도 `volicord serve --transport local-http` 모드는 로컬/Docker 사용을 위한 Local HTTP
transport입니다. 기준 stdio 프로세스가 아니며 공개 네트워크 API, SaaS 엔드포인트, 다중
사용자 서버, 보안 경계로 취급하면 안 됩니다. 정확한 리스너, 인증, Origin, HTTP 메시지 교환
동작은 [MCP 전송](mcp-transport.md)이 담당합니다.

정확한 실행 파일 동작, 환경 변수, 프레이밍, 시작 검증 또는 사전 점검 동작, 응답 래핑, 종료, 재연결 규칙은 [MCP 전송](mcp-transport.md)이 담당합니다. 이 런타임 경계 담당 문서는 프로세스, 위치, 금지되는 추론의 경계만 구분합니다.

### Agent Connection과 Connection Projects

Agent Connection은 `volicord mcp --stdio`를 위한 로컬 MCP 호스트 연결 단위입니다. 연결은 `connection_internal_id`, `personal`, `shared`, `global` 중 하나의 연결 의도, 호스트 범위, `connection.mode=workflow` 또는 `connection.mode=read_only`를 가지며, Connection Projects 허용 목록에 명시된 `project_internal_id` 값만 다룰 수 있습니다. 사용자 대상 관리 명령은 내부 식별 정보를 요구하지 않고 호스트, 의도, 저장소 루트로 연결을 선택합니다. MCP에 보이는 프로젝트 선택은 Volicord가 반환한 `project_selector`를 사용합니다.

Agent Connection은 지원되는 API 경로를 통해 사용자 행동을 요청할 수 있지만, 권한 효력이 있는 사용자 행동 resolution을 기록할 수 없습니다. 그런 resolution은 `User Channel`을 통해 `actor_source=local_user`로 기록됩니다.

추론하면 안 되는 것:
- 복사된 `connection_id` 프로세스 바인딩 값이 권한, 사용자 신원, OS 권한, 호스트 신뢰, 역량을 증명한다는 주장.
- `connection.mode=workflow`가 파일시스템, 셸, 네트워크, 비밀값, 배포, Product Repository 쓰기 권한을 부여한다는 주장.
- Connection Projects 허용 목록이 등록된 모든 프로젝트를 허용 프로젝트로 만든다는 주장.
- Agent Connection이 사용자를 대신해 최종 수락, 잔여 위험 수락, 민감 동작 승인, 취소, 범위 결정을 기록할 수 있다는 주장.

### 외부 MCP 호스트 설정

MCP 호스트 설정은 외부 MCP 호스트가 소유합니다. [관리 CLI](admin-cli.md)가 그 동작을 정의할 때 Volicord 관리 명령은 허용되는 관리 호스트 대상의 설정을 직접 설치할 수 있습니다. 사용자 관리 외부 호스트 설정은 호스트가 소유하는 영역으로 남으며, 이 문서는 위치 경계만 담당합니다.

주장할 수 있는 것:
- 호스트 설정은 `volicord mcp --stdio` 실행 파일, 내부 Agent Connection 바인딩, 그 호스트에 필요한 환경 값을 지정할 수 있습니다.
- 호스트 설정은 소스 저장소, 설치 파일, `Volicord Runtime Home`, `Product Repository` 밖에 있을 수 있습니다.
- 공유 저장소 호스트 설정은 로컬 경로를 내장하지 않고 `VOLICORD_HOME`을 전달할 수
  있습니다. 생성된 로컬 훅 래퍼는 명시적 로컬 통합 파일로 남으면서 선택한 절대 경로와
  절대 `volicord_command`를 고정할 수 있습니다.

주장하면 안 되는 것:
- MCP 호스트 설정이 정의상 Volicord 런타임 상태라는 주장.
- MCP 호스트 설정이 로컬 기준 기록, Product Repository 파일, Volicord 권한 증거라는 주장.
- 호스트 설정 디렉터리가 자동으로 `Volicord Runtime Home`이라는 주장.
- 호스트 설정 쓰기가 호스트가 MCP 서버를 신뢰, 승인, 로드, 초기화, 노출했다는 뜻이라는 주장.

<a id="runtime-location-runtime-home"></a>
### `Volicord Runtime Home`

`Volicord Runtime Home`은 Volicord 런타임 데이터를 위한 런타임 저장 위치입니다.

주장할 수 있는 것:
- 저장소/런타임 담당 문서는 어떤 운영 데이터가 `Volicord Runtime Home`에 속하는지 정의합니다.
- 저장소/런타임 담당 문서는 그 데이터의 검증, 저장 효과, 기록 배치, 아티팩트 저장, 버전 관리, 복구 동작을 정의합니다.
- `diagnostics.sqlite`는 Runtime Home 루트에 있을 수 있지만 레지스트리 및 모든 프로젝트 권한 데이터베이스와 분리됩니다. 그 위치는 진단 관찰에 Core, 증거, 닫기 준비 상태, User Channel, 보안 권한을 부여하지 않습니다.

주장하면 안 되는 것:
- `Volicord Runtime Home`이 `Product Repository`라는 주장.
- `Volicord Runtime Home`이 기본적으로 Volicord 설치 위치라는 주장.
- `Volicord Runtime Home`이 자동으로 보안 경계라는 주장.
- `Volicord Runtime Home`이 기본적으로 격리를 제공한다는 주장.

<a id="runtime-home-product-repository-separation"></a>
### Runtime Home/Product Repository 경로 분리

유효한 등록 프로젝트는 해결된 파일시스템 경로가 서로 별개이고 조상-자손 관계가 아닌 `Volicord Runtime Home`과 `Product Repository`를 사용해야 합니다.

금지되는 관계:

| 관계 | 계약 |
|---|---|
| 같은 해결 경로 | `Volicord Runtime Home`과 `Product Repository`가 같은 경로로 해결되면 안 됩니다. |
| `Volicord Runtime Home` 안의 `Product Repository` | `Product Repository`는 `Volicord Runtime Home` 안에 위치하면 안 됩니다. |
| `Product Repository` 안의 `Volicord Runtime Home` | `Volicord Runtime Home`은 `Product Repository` 안에 위치하면 안 됩니다. |

허용되는 관계:
- 조상-자손 관계가 없는 서로 다른 해결 경로는 허용됩니다.
- 이 규칙은 Volicord 소스 저장소를 의도적으로 `Product Repository`로 선택하는 것을 금지하지 않습니다. 단, 그 소스 저장소는 `Volicord Runtime Home`과 분리되어 있어야 합니다.

이 분리 계약은 적격성 규칙입니다. 새 프로젝트 등록, 프로필 재사용, 프로젝트 상태 관리 접근, Core 실행 진입, MCP 프로젝트 세션 시작은 선택된 `Volicord Runtime Home`과 등록된 `Product Repository`가 이 계약을 만족해야 합니다.

네이티브 Windows에서 Runtime Home/Product Repository 경계 검증은 로컬 드라이브 문자 경로를
허용하고, UNC 경로, `\\wsl$\...` 같은 WSL UNC 경로, `/mnt/c/...` 같은 WSL 마운트 형식
경로를 거부합니다. Windows 경계 비교는 경로 정규화 뒤 경로 구성 요소 단위로 대소문자를
구분하지 않습니다. 심볼릭 링크 또는 접합점 별칭이 같은 경로나 조상-자손 관계로
해석된다는 사실을 호스트 파일시스템이 Volicord에 노출하면 그 별칭은 유효하지
않습니다.

검사 계층은 이 계약을 위반하는 원시 저장 프로젝트 행을 진단 목적으로 계속 보여 줄 수 있습니다. 운영 프로젝트 조회, 프로젝트 목록 조회, 프로필 재사용, 프로젝트 상태 관리 접근, Agent Connection 관리, Connection Projects 접근, Core 실행 진입, MCP 프로젝트 가용성은 그런 행을 정상 프로젝트 기록이나 프로젝트 항목으로 반환하지 말고 거절해야 합니다. 시스템은 검사가 그 행을 보고할 수 있다는 이유만으로 경로를 자동 이동하거나, 레지스트리 행을 복구하거나, 그 기록을 삭제하지 않습니다.

## 로컬 권한 경계

파일이나 디렉터리에 대한 로컬 파일 접근은 Volicord 권한과 같지 않습니다.

주장할 수 있는 것:
- 로컬 행위자는 호스트 환경에 따라 제품 파일, 설치 파일, MCP 호스트 설정, 런타임 데이터 위치에 대한 파일시스템 접근을 가질 수 있습니다.
- Volicord 권한은 문서화된 API, 저장소, 런타임, 보안, 사용자 판단 계약에 달려 있습니다.

주장하면 안 되는 것:
- 로컬 경로, 디렉터리 이름, 복사된 식별자, 렌더링된 표시, 대화 메시지, 커넥터 설명, 에이전트 기억이 Volicord 권한을 증명한다는 주장.
- 문서화된 Volicord 계약 밖의 직접 로컬 수정이 유효한 Volicord 기록, 증거, 수락, 잔여 위험 수락, 쓰기 티켓, 아티팩트 권한을 만든다는 주장.
- 런타임 데이터 위치만으로 보안 보장 수준이 달라진다는 주장.

## 런타임 위치, 저장소, 보안 담당 문서

런타임 위치는 경계 설명이지 저장소 배치나 보안 메커니즘이 아닙니다.

저장소 담당 문서가 정의하는 것:
- 어떤 Volicord 기록, 메타데이터, 아티팩트 데이터, 운영 진단이 `Volicord Runtime Home`에 속하는지
- 그 기록이 어떤 형태를 갖고, 어떻게 초기화하고 버전을 관리하고 검증하고 갱신하는지
- 어떤 메서드 분기가 저장 효과를 만드는지

보안 담당 문서가 정의하는 것:
- 보장 수준과 비주장
- 로컬 연결 가정과 접근 경계 표현
- 어떤 주장에 `cooperative` 또는 연결 관찰 기반 `detective` 표현을 쓸 수 있는지
- `Volicord Runtime Home`이 자동으로 보안 경계가 아니라는 비주장

이 문서는 위치와 금지되는 추론만 구분합니다.

## 관련 담당 문서

- [저장소 기록](storage-records.md), [저장 효과](storage-effects.md), [아티팩트 저장소](storage-artifacts.md), [저장소 버전 관리](storage-versioning.md): 저장소 기록 배치, 효과, 아티팩트, 스키마 초기화, 버전 관리, 런타임 데이터 세부사항.
- [API 메서드](api/methods.md)와 메서드 담당 문서: 메서드 처리 경로와 메서드 동작.
- [Core 모델](core-model.md): Core 권한, User Channel 사용자 행동 경계, `actor_source`, 쓰기 티켓, 수락, 잔여 위험.
- [보안](security.md): 보안 주장과 비주장, 신뢰 경계, 보장 수준, `operation_category`, Agent Connection 권한 추론 금지.
- [상태 보기 권한 참조](projection-and-templates.md): 상태 보기 권한과 최신성 경계.
- [템플릿 본문](template-bodies.md): 렌더링된 템플릿 본문 계약.
