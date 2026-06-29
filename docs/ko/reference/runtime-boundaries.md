# 런타임 경계 참조

이 문서는 Volicord 구현, Agent Connection, `Product Repository`, `Volicord Runtime Home`, `User Channel`, 외부 MCP 호스트 설정 사이의 구성 요소 경계와 위치 경계를 담당합니다. 이 경계들에 대한 위치와 연결 권한 가정을 정의하고 저장소와 보안 세부사항은 담당 문서로 보냅니다.

Volicord 구현은 이 저장소가 유지하는 구현 집합입니다. Volicord 전체가 아니며, Core도 아니고, 실행 중인 프로세스 하나도 아니며, Volicord 상태를 위한 로컬 기준 기록도 아닙니다.

## 담당하는 것 / 담당하지 않는 것

| 이 문서가 담당하는 것 | 이 문서가 담당하지 않는 것 |
|---|---|
| 제품/시스템인 Volicord와 저장소가 유지하는 구현 집합인 Volicord 구현의 구분. | 공개 API 동작, 공개 스키마 형태, 메서드별 효과. |
| Volicord 소스 저장소, Volicord 설치, 실행 중인 실행 파일 역할의 구분. | 릴리스 패키징 정책이나 필수 설치 루트 배치. |
| `Product Repository` 정의와 `Product Repository` API 경로 정규화. | 저장소 기록 배치, 잠금, 마이그레이션, 버전 관리, 아티팩트 생명주기 세부사항. |
| `Volicord Runtime Home` 정의. | API 메서드 동작이나 공개 스키마 형태. |
| Volicord 구현 파일, 제품 파일, 런타임 데이터, 외부 MCP 호스트 설정의 분리. 정확한 Runtime Home/Product Repository 경로 관계 계약을 포함합니다. | 자세한 보안 보장 의미나 보안 비주장. |
| 로컬 파일 접근과 위치가 권한을 만들지 않는다는 규칙. | 상태 보기 권한, 템플릿 본문, 렌더링된 표시의 최신성. |
| 런타임 위치만으로 Volicord 권한, 보안 권한, 격리를 증명할 수 없다는 규칙. | 제품 범위, 닫기 준비 상태, 증거 충분성, 사용자 소유 판단 의미. |

## 구성 요소와 아티팩트 모델

Volicord는 제품, 구현, 실행 파일 역할, MCP 호스트 용어, 기준 기록 개념을 구분합니다.

| 용어 | 정의 | 추론하면 안 되는 것 |
|---|---|---|
| Volicord | AI 지원 제품 작업을 위한 더 넓은 로컬 작업 권한 제품이자 시스템. | Core, 소스 저장소, 실행 파일 프로세스 하나로 보면 안 됩니다. |
| Core | Volicord 상태를 위한 로컬 기준 기록. | Volicord 제품/시스템 전체나 어댑터 또는 CLI 실행 파일로 보면 안 됩니다. |
| Volicord 구현 | 이 저장소가 유지하는 구현 집합. 소스 수준에서는 구현 크레이트, `volicord` 관리 CLI, `volicord-mcp` 로컬 MCP 어댑터, 테스트, 문서, 검증 도구, 저장소 설정을 포함합니다. | 모든 가능한 Volicord 제품 노출 경로, Core 자체, `Volicord Runtime Home`, `Product Repository`, 단일 데몬, MCP 서버 항목, 네트워크 서비스로 보면 안 됩니다. |
| Volicord 소스 저장소 | 이 저장소를 체크아웃한 소스 아티팩트. | 배포된 설치, 실행 중인 프로세스, Runtime Home, Product Repository, MCP 호스트 설정과 같은 것으로 보면 안 됩니다. |
| Volicord 설치 | 배포된 Volicord 실행 파일과 필요한 런타임 리소스의 부분집합. | 모든 설치에 문서, 테스트, 소스 파일, 저장소 메타데이터가 들어 있다고 추론하면 안 됩니다. |
| `volicord` 관리 프로세스 | Volicord 구현 안의 관리 CLI 실행 파일/프로세스. | Volicord나 Volicord 구현 전체와 같은 말로 보면 안 됩니다. |
| `volicord-mcp` MCP 어댑터 프로세스 | Volicord 구현 안의 로컬 stdio MCP 어댑터 실행 파일/프로세스. | Volicord 구현과 별개이거나 그 자체가 Volicord 구현 전체라고 보면 안 됩니다. |
| `Agent Connection` | 내부 연결 식별 정보, 연결 의도, 호스트 범위, `workflow` 또는 `read_only`인 `connection.mode`와 함께 저장되는 로컬 MCP 호스트 연결 단위입니다. | OS 샌드박스, 파일시스템 ACL, 네트워크 정책, 비밀값 격리 장치, 사용자 대상 식별자 요구사항, 사용자 판단 경로로 보면 안 됩니다. |
| `Connection Projects` | 사용자 대상 저장소 루트 선택 뒤 Agent Connection이 다룰 수 있는 내부 프로젝트 식별 정보의 명시적 허용 목록입니다. | 기본적으로 등록된 모든 프로젝트를 포함하거나 Product Repository 권한을 증명한다고 보면 안 됩니다. |
| `User Channel` | 권한을 지니는 사용자 판단을 기록하는 로컬 사용자 경로입니다. | Agent Connection, MCP 호스트, 생성된 표시, Product Repository 파일로 보면 안 됩니다. |
| MCP 서버 | 외부 MCP 호스트에 노출되는 서버 항목이나 프로세스의 이름으로 쓸 수 있는 일반 MCP 프로토콜 또는 호스트 설정 용어입니다. 호스트가 그 라벨을 사용한다면 `volicord-mcp` 같은 로컬 stdio 어댑터 프로세스를 가리킬 수도 있습니다. | 제품/시스템인 Volicord, Volicord 구현, `volicord`, `volicord-mcp`를 TCP 또는 HTTP 네트워크 서버로 만들지 않으며, Volicord의 제품 라벨도 아닙니다. |

동작을 한 실행 파일 역할이 수행한다면 그 역할의 이름을 씁니다. 의미가 구현 집합 전체에 적용될 때만 단독 Volicord 구현을 사용합니다.

## 파일시스템 위치 모델

Volicord는 구현 파일, 제품 파일, 런타임 데이터, 외부 호스트 설정을 구분합니다. Volicord 구현 집합 전체를 위한 단일 필수 파일시스템 루트는 없습니다.

| 위치 역할 | 정의 | 추론하면 안 되는 것 |
|---|---|---|
| Volicord 소스 저장소 또는 설치 파일 | 소스 체크아웃, 또는 Volicord 구현의 배포된 실행 파일과 필요한 런타임 리소스. | 자동으로 `Volicord Runtime Home`, `Product Repository`, MCP 호스트 설정, Volicord 권한 증거, 본질적인 네트워크 리스너가 된다고 보면 안 됩니다. |
| `Product Repository` | 제품 소스, 제품 문서, 테스트, 설정, 그 밖의 프로젝트 파일을 담는 사용자의 제품 파일 경계. | Volicord 런타임 상태, `Volicord Runtime Home`, Volicord 권한 증거로 보면 안 됩니다. |
| `Volicord Runtime Home` | 저장소/런타임 담당 문서가 정의하는 Volicord 소유 기록, 로컬 런타임 메타데이터, 아티팩트 데이터를 위한 런타임 저장 위치. | `Product Repository`, 기본적인 Volicord 설치 위치, 자동 보안 경계, 기본 격리로 보면 안 됩니다. |
| 외부 MCP 호스트 설정 | `volicord-mcp` 명령, 프로세스 환경, 호스트별 바인딩을 지정할 수 있는 외부 MCP 호스트 소유 설정. | 정의상 Volicord 런타임 상태, `Volicord Runtime Home`, `Product Repository`, Volicord 소스 저장소 또는 설치 파일로 보면 안 됩니다. |

### 런타임과 호스트 책임 표

이 표는 현재 기준 로컬 Rust 구현에서 각 경계가 맡는 역할을 요약합니다. 자세한 기록 배치는 [저장소 기록](storage-records.md)이, 아티팩트 생명주기는 [아티팩트 저장소](storage-artifacts.md)가, 관리 명령 동작은 [관리 CLI](admin-cli.md)가, MCP 프로세스 동작은 [MCP 전송](mcp-transport.md)이 담당합니다.

| 경계 또는 표면 | 거기에 속하는 것 | 주요 프로세스 경로 | 추론하면 안 되는 것 |
|---|---|---|---|
| `Volicord Runtime Home` | `registry.sqlite`, 프로젝트별 `projects/{project_internal_id}/state.sqlite`, 아티팩트 저장소를 사용할 때의 `projects/{project_internal_id}/artifacts/` 같은 프로젝트 아티팩트 저장소. registry는 Runtime Home 식별 정보와 경로, 설치 프로필 기록, 저장소 루트 기반 프로젝트 등록, 프로젝트 alias, Agent Connection 기록, Connection Projects 멤버십, 호스트 범위, 설정 대상, 연결 의도, 관리 fingerprint, 검증 요약 상태, 검증 보고서 JSON, 사용자 동작 JSON 같은 `managed host configuration state` 인벤토리를 저장합니다. | `volicord setup`, 프로젝트, 연결, export, 사용자 명령은 담당 경로에 따라 registry 상태를 초기화하거나 읽거나 갱신합니다. `volicord-mcp`, Core, Store는 시작, 프로젝트 라우팅, Core 상태, 아티팩트를 위해 Runtime Home 상태를 읽거나 사용합니다. | `Product Repository`, 외부 호스트 설정, 설치 디렉터리, OS 샌드박스, 네트워크 격리 계층, 악성코드 검사기, 비밀값 검사기, 호스트 신뢰 증거가 아닙니다. |
| `Product Repository` | 사용자 제품 파일과 프로젝트 범위 호스트 설정 또는 관리 지침처럼 명시적으로 요청된 통합 파일만 여기에 속합니다. | 일반 제품 파일 편집은 사용자 또는 호스트 도구가 소유합니다. Volicord는 제품 경로를 입력으로 검사할 수 있고, 담당 문서가 정의한 관리 경로를 통해서만 명시적 통합 파일을 쓸 수 있습니다. | Runtime Home 상태, Core 저장소, 기본 아티팩트 저장소, Volicord 권한 증거가 아닙니다. |
| Runtime Home registry 안의 `managed host configuration state` | 호스트 대상에 대한 Volicord registry 인벤토리입니다. 내부 연결 식별 정보, 호스트 종류, 연결 의도, 호스트 범위, 선택적 내부 프로젝트 식별 정보, 내부 서버 이름, 설정 대상, 모드, 활성 상태, 관리 fingerprint, 검증 요약 상태, 검증 보고서 JSON, 사용자 동작 JSON, 메타데이터를 포함합니다. | `volicord connect`, `volicord connections`, `volicord connection status`, `volicord connection verify`, `volicord connection mode`, `volicord connection remove`, export 흐름은 registry 행과 Connection Projects 멤버십을 만들고, 갱신하고, 목록 조회하고, 검증하고, 제거합니다. | 외부 호스트 설정 객체 자체가 아니며, 호스트가 `volicord-mcp`를 신뢰, 승인, 로드, 초기화, 노출했다는 증거가 아닙니다. |
| 외부 MCP 호스트 설정 | 내부 Agent Connection 바인딩이 있는 `volicord-mcp`와 `VOLICORD_HOME` 같은 환경 값을 지정할 수 있는 호스트 소유 설정 또는 사용자 관리 내보내기입니다. | [관리 CLI](admin-cli.md)가 그 동작을 정의할 때 `volicord`는 지원되는 직접 설정을 쓰거나 내보내기를 렌더링할 수 있습니다. 외부 호스트는 로딩과 신뢰 결정을 소유합니다. | Runtime Home registry 상태, Core 권한, Volicord 권한 증거가 아닙니다. `Product Repository`에 있다면 명시적 통합 파일일 뿐입니다. |
| `volicord` 관리 CLI 프로세스 | Runtime Home 초기화, 저장소 루트에서의 프로젝트 등록, Agent Connection과 Connection Projects 관리, 호스트 설정 적용 또는 내보내기, 상태 조회, 검증, 모드 변경, 정의된 안전 제거 같은 로컬 설정과 registry/호스트 통합 관리. | 로컬 운영자 또는 사용자가 이 프로세스를 실행합니다. | 공개 Volicord API 메서드 경로, OS 보안 강제 계층, 호스트 신뢰 결정, 포괄적 Product Repository 편집 권한이 아닙니다. |
| `volicord-mcp` MCP 어댑터 프로세스 | 하나의 Agent Connection에 묶인 로컬 stdio 자식 프로세스입니다. Runtime Home을 해석하고, 연결 상태를 검증하고, `connection.mode`에 따라 도구를 노출하고, 담당자가 정의한 저장소 루트 규칙으로 허용된 프로젝트를 선택하고, 어댑터 소유 호출 사실을 파생하며, 공개 메서드 호출을 Core와 Store로 라우팅합니다. | MCP 호스트가 프로세스를 시작하고 stdin/stdout으로 통신합니다. | 그 자체로 임의 제품 파일 편집 권한을 부여하거나, 권한을 지니는 사용자 판단을 기록하거나, 호스트 신뢰를 강제하거나, 샌드박싱을 제공하거나, 네트워크 리스너를 열지 않습니다. |

<a id="runtime-location-product-repository"></a>
### `Product Repository`

`Product Repository`는 사용자의 프로젝트 작업 공간이자 제품 파일 경계입니다.

주장할 수 있는 것:
- 제품 파일은 담당 문서가 정한 Volicord 확인이나 사용자 소유 판단의 입력으로 검사될 수 있습니다.
- 호환되는 제품 파일 쓰기는 현재 적용 범위, 현재 적용 Change Unit, 필요한 판단, `Write Check` 호환성의 지배를 받을 수 있습니다.

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
- `.claude/rules/` 아래의 Volicord 관리 Claude Code 규칙 파일

규칙:

- 관리 명령은 쓰기를 적용하기 전에 정확한 대상 경로와 내용을 미리 보여 줘야 합니다.
- 비대화식 실행은 [관리 CLI](admin-cli.md#noninteractive-approval-behavior)가 정의한 명시적 shared 의도 명령 경로와 충돌 동작을 사용해야 합니다.
- 쓰기는 Volicord 소유 마커 또는 관리 지문을 사용해야 합니다.
- 기존 비관리 내용은 덮어쓰지 말고 충돌로 보고해야 합니다.
- 교체는 일치하는 Volicord 관리 내용에만 적용할 수 있습니다.
- 안전한 제거는 일치하는 Volicord 관리 내용만 제거할 수 있으며 관련 없는 프로젝트 파일을 그대로 둬야 합니다.
- 이 파일들은 호스트 설정 또는 지침입니다. Volicord 런타임 상태, Core 권한, 증거, 수락, 닫기 준비 상태, 잔여 위험 수락, 보안 보장이 아닙니다.

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
- `Write Check` 호환성은 Core 담당 메서드 경로로 기록되는 제안된 제품 파일 변경에만 적용됩니다. 전역 파일시스템 가로채기, 셸 권한, 명령 승인, 쓰기가 실제로 일어났다는 증명이 아닙니다.
- 메서드별 호환성 결정은 API 메서드 담당 문서에 둡니다.

<a id="runtime-location-source-installation-processes"></a>
### Volicord 구현 소스, 설치, 프로세스

Volicord 구현은 이 저장소가 유지하는 구현 집합을 뜻합니다. 코드, 문서, 테스트, 검증 도구, 저장소 설정을 담은 체크아웃에는 Volicord 소스 저장소를 씁니다. 배포된 실행 파일과 필요한 런타임 리소스에는 Volicord 설치를 씁니다.

주장할 수 있는 것:
- `volicord`는 Volicord 구현 안의 관리 CLI/프로세스입니다.
- `volicord-mcp`는 Volicord 구현 안의 로컬 stdio MCP 어댑터 프로세스입니다.
- Volicord 설치는 소스 저장소, `Volicord Runtime Home`, `Product Repository`, MCP 호스트 설정과 다른 위치일 수 있습니다.
- Volicord 설치가 모든 소스 저장소 파일을 포함할 필요는 없습니다.
- 기준 로컬 Rust 구현에서는 MCP 호스트가 `volicord-mcp`를 자식 프로세스로 시작하고 stdio로 통신합니다.

주장하면 안 되는 것:
- Volicord 구현이 Volicord 제품/시스템 전체라는 주장.
- Volicord 구현이 Core 또는 Volicord 상태를 위한 로컬 기준 기록이라는 주장.
- Volicord 구현이 오직 `volicord`, 오직 `volicord-mcp`, 하나의 장기 실행 데몬, 또는 하나의 네트워크 서비스라는 주장.
- `volicord-mcp`가 Volicord 구현 안의 실행 파일 역할이 아니라 Volicord 구현과 별개라는 주장.
- 어떤 디렉터리에서 Volicord를 설치하거나 실행하면 그 디렉터리가 `Volicord Runtime Home`이 된다는 주장.
- 설치 위치가 그곳에 런타임 데이터가 있음을 증명한다는 주장.
- 설치 경로가 Volicord 권한, 보안 권한, 제품 파일 쓰기 권한을 부여한다는 주장.
- Volicord 구현이라는 용어 자체가 TCP, HTTP, 소켓, 또는 그 밖의 네트워크 리스너를 뜻한다는 주장.

### 기준 로컬 MCP 프로세스

현재 로컬 Rust MCP 어댑터는 Volicord 구현 안의 실행 파일 역할인 `volicord-mcp` stdio 프로세스입니다. MCP 호스트는 프로토콜이나 호스트 설정 맥락에서 설정된 항목을 MCP 서버라고 부를 수 있습니다. 그 라벨은 Volicord를 서버 제품으로 만들거나 Volicord 구현을 네트워크 서버로 만들지 않습니다. MCP 호스트는 `volicord-mcp`를 자식 프로세스로 시작하고, 프로세스 환경으로 설정을 전달하며, stdin/stdout을 통해 줄 단위 JSON-RPC를 주고받습니다. 기준 프로세스는 TCP, HTTP, Unix-domain socket, 또는 그 밖의 네트워크 리스너를 열지 않습니다.

정확한 실행 파일 동작, 환경 변수, 프레이밍, 시작 검증 또는 사전 점검 동작, 응답 래핑, 종료, 재연결 규칙은 [MCP 전송](mcp-transport.md)이 담당합니다. 이 런타임 경계 담당 문서는 프로세스, 위치, 금지되는 추론의 경계만 구분합니다.

### Agent Connections와 Connection Projects

Agent Connection은 `volicord-mcp`를 위한 로컬 MCP 호스트 연결 단위입니다. 연결은 내부 연결 식별 정보, `personal`, `shared`, `global` 중 하나의 연결 의도, 호스트 범위, `connection.mode=workflow` 또는 `connection.mode=read_only`를 가지며, Connection Projects 허용 목록에 명시된 내부 프로젝트 식별 정보만 다룰 수 있습니다. 사용자 대상 관리 명령은 내부 식별 정보를 요구하지 않고 호스트, 의도, 저장소 루트 또는 Volicord가 반환한 프로젝트 선택자로 연결을 선택합니다.

Agent Connection은 지원되는 API 경로를 통해 사용자 판단을 요청할 수 있지만, 권한을 지니는 사용자 판단을 기록할 수 없습니다. 그런 판단은 `User Channel`을 통해 `actor_source=local_user`로 기록됩니다.

추론하면 안 되는 것:
- 복사된 내부 `connection_id`가 권한, 사용자 신원, OS 권한, 호스트 신뢰, 역량을 증명한다는 주장.
- `connection.mode=workflow`가 파일시스템, 셸, 네트워크, 비밀값, 배포, Product Repository 쓰기 권한을 부여한다는 주장.
- Connection Projects 허용 목록이 등록된 모든 프로젝트를 허용 프로젝트로 만든다는 주장.
- Agent Connection이 사용자를 대신해 최종 수락, 잔여 위험 수락, 민감 동작 승인, 취소, 범위 결정을 기록할 수 있다는 주장.

### 외부 MCP 호스트 설정

MCP 호스트 설정은 외부 MCP 호스트가 소유합니다. [관리 CLI](admin-cli.md)가 그 동작을 정의할 때 Volicord 관리 명령은 지원되는 호스트 설정을 직접 설치하거나 명시적 내보낸 설정을 렌더링할 수 있지만, 이 문서는 위치 경계만 담당합니다.

주장할 수 있는 것:
- 호스트 설정은 `volicord-mcp` 실행 파일, 내부 Agent Connection 바인딩, 그 호스트에 필요한 환경 값을 지정할 수 있습니다.
- 호스트 설정은 소스 저장소, 설치 파일, `Volicord Runtime Home`, `Product Repository` 밖에 있을 수 있습니다.

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

이 분리 계약은 적격성 규칙입니다. 새 프로젝트 등록, 설정 재사용, 프로젝트 상태 관리 접근, Core 실행 진입, MCP 프로젝트 세션 시작은 선택된 `Volicord Runtime Home`과 등록된 `Product Repository`가 이 계약을 만족해야 합니다.

검사 계층은 이 계약을 위반하는 원시 저장 프로젝트 행을 진단 목적으로 계속 보여 줄 수 있습니다. 운영 프로젝트 조회, 프로젝트 목록 조회, 설정 재사용, 프로젝트 상태 관리 접근, Agent Connection 관리, Connection Projects 접근, Core 실행 진입, MCP 프로젝트 가용성은 그런 행을 정상 프로젝트 기록이나 프로젝트 항목으로 반환하지 말고 거절해야 합니다. 시스템은 검사가 그 행을 보고할 수 있다는 이유만으로 경로를 자동 이동하거나, registry 행을 복구하거나, 그 기록을 삭제하지 않습니다.

## 로컬 권한 경계

파일이나 디렉터리에 대한 로컬 파일 접근은 Volicord 권한과 같지 않습니다.

주장할 수 있는 것:
- 로컬 행위자는 호스트 환경에 따라 제품 파일, 설치 파일, MCP 호스트 설정, 런타임 데이터 위치에 대한 파일시스템 접근을 가질 수 있습니다.
- Volicord 권한은 문서화된 API, 저장소, 런타임, 보안, 사용자 판단 계약에 달려 있습니다.

주장하면 안 되는 것:
- 로컬 경로, 디렉터리 이름, 복사된 식별자, 렌더링된 표시, 대화 메시지, 커넥터 설명, 에이전트 기억이 Volicord 권한을 증명한다는 주장.
- 문서화된 Volicord 계약 밖의 직접 로컬 수정이 유효한 Volicord 기록, 증거, 수락, 잔여 위험 수락, `Write Check`, 아티팩트 권한을 만든다는 주장.
- 런타임 데이터 위치만으로 보안 보장 수준이 달라진다는 주장.

## 런타임 위치, 저장소, 보안 담당 문서

런타임 위치는 경계 설명이지 저장소 배치나 보안 메커니즘이 아닙니다.

저장소 담당 문서가 정의하는 것:
- 어떤 Volicord 기록, 메타데이터, 아티팩트 데이터, 운영 진단이 `Volicord Runtime Home`에 속하는지
- 그 기록이 어떤 형태를 갖고, 어떻게 버전 관리, 검증, 마이그레이션, 갱신되는지
- 어떤 메서드 분기가 저장 효과를 만드는지

보안 담당 문서가 정의하는 것:
- 보장 수준과 비주장
- 로컬 연결 가정과 접근 경계 표현
- 어떤 주장에 `cooperative`나 연결 관찰 기반 `detective` 표현을 쓸 수 있는지
- `Volicord Runtime Home`이 자동으로 보안 경계가 아니라는 비주장

이 문서는 위치와 금지되는 추론만 구분합니다.

## 추론하면 안 되는 것

아래에서 Volicord 권한, 보안 권한, 런타임 상태, 격리를 추론하지 않습니다.

- `Product Repository` 텍스트나 프로젝트 파일.
- Volicord가 설치되거나 시작된 디렉터리.
- 외부 MCP 호스트 설정.
- `Volicord Runtime Home`으로 선택된 디렉터리.
- 복사된 내부 `connection_id`.
- 표시된 `ArtifactRef`.
- 렌더링된 `Projection`, 상태 카드, 템플릿 출력.
- 커넥터 설명, 대화 텍스트, 에이전트 기억.

아래도 추론하지 않습니다.

- `Product Repository`가 `Volicord Runtime Home`이라는 것.
- 설치 위치와 런타임 데이터 위치가 같다는 것.
- MCP 호스트 설정이 Volicord 런타임 상태나 Volicord 권한이라는 것.
- `Volicord Runtime Home`이 보안 경계라는 것.
- 제품 파일이 Volicord 기록이라는 것.
- 생성된 표시가 원천 기록 권한을 대신한다는 것.

## 관련 담당 문서

- [보안](security.md): 보안 주장, 비주장, 신뢰 경계, 보장 수준.
- [저장소 기록](storage-records.md), [저장 효과](storage-effects.md), [아티팩트 저장소](storage-artifacts.md), [저장소 버전 관리](storage-versioning.md): 저장소 기록 배치, 효과, 아티팩트, 마이그레이션, 버전 관리, 런타임 데이터 세부사항.
- [API 메서드](api/methods.md)와 메서드 담당 문서: 메서드 경로와 메서드 동작.
- [Core 모델](core-model.md): Core 권한, User Channel 판단 경계, `actor_source`, `Write Check`, 수락, 잔여 위험.
- [보안](security.md): `operation_category`, 보안 비보장, Agent Connection 권한 추론 금지.
- [상태 보기 권한 참조](projection-and-templates.md): 상태 보기 권한과 최신성 경계.
- [템플릿 본문](template-bodies.md): 렌더링된 템플릿 본문 계약.
