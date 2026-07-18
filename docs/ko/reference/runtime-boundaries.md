# 런타임 경계

이 문서는 `Product Repository`, `Volicord Runtime Home`, 설치된 Volicord 실행 파일,
관리 Codex 구성, stdio MCP 자식 프로세스 사이의 파일시스템 위치와 프로세스 경계를
담당합니다.

## 구성 요소 모델

| 구성 요소 | 경계 |
|---|---|
| Product Repository | 사용자 제품 파일과 명시적인 관리 프로젝트 구성을 담는 정규 Git 작업 트리입니다. |
| Volicord Runtime Home | 로컬 registry, 프로젝트 상태, 권위 있는 운영 session, 런타임 소유 아티팩트입니다. |
| Volicord 설치 | 선택한 `volicord` 실행 파일과 build identity입니다. Runtime Home이 아닙니다. |
| 관리 Codex 구성 | 정확한 관리 stdio 프로세스를 시작하는 사용자 또는 프로젝트 소유 구성입니다. Core 권한이 아닙니다. |
| `volicord mcp --stdio` | 현재 Agent Connection 하나에 결속된 로컬 자식 프로세스 하나입니다. 네트워크 서비스가 아닙니다. |

## Product Repository

Product Repository는 정규 현재 Git 작업 트리에서 해결합니다. 최초 릴리스는
[시스템 요구사항](system-requirements.md#wsl2-topology)의 WSL2 ext4 경계를 포함한 담당
문서의 native filesystem topology를 요구합니다. 표시 이름, cwd만, 상위 디렉터리 검색,
복사한 경로 문자열로 저장소 identity를 만들지 않습니다.

WSL2에서는 정규 Linux 형태의 경로만으로 충분하지 않습니다. 등록과 실행은 정확히
고정된 배포판 좌표를 검증하고 저장소 root가 해당 배포판의 ext4 파일 시스템에
있는지 관찰합니다. WSL1, DrvFS, 관찰할 수 없거나 서로 충돌하는 토폴로지 사실은
저장소를 사용하기 전에 fail-closed로 처리합니다.

명시적으로 요청된 관리 파일에는 다음이 포함될 수 있습니다.

- 공유 Codex 항목의 `.codex/config.toml`
- 프로젝트 소유 workflow policy의 `.volicord/policy.json`
- `AGENTS.md`의 Volicord 관리 블록

설정, 복구, 제거는 관련 없는 파일 내용을 보존합니다. Volicord가 경로를 관찰할 수
있다는 이유로 제품 source, build output, test output, 사용자 구성이 Runtime Home
상태가 되지는 않습니다.

<a id="product-repository-api-path-normalization"></a>
## Product Repository API 경로 정규화

공개 제품 경로는 저장소 상대 slash 구분 UTF-8 text를 사용합니다. 절대 경로, drive
또는 UNC prefix, backslash, 빈 component, `.` 또는 `..` component, NUL, 정규
저장소 root를 벗어나는 경로는 유효하지 않습니다. 정규 경로는 `/`로 시작하거나 끝나지
않고 반복 separator를 포함하지 않습니다.

문자열 정규화만으로 파일시스템 containment가 성립하지 않습니다. 경로를 읽거나 쓰는
메서드는 현재 저장소 root를 해결하고 효과 전에 담당 문서의 symlink와 canonicalization
검사를 적용해야 합니다. 저장 기록의 경로는 정규 저장소 상대 identity로 남습니다.

## 관리 Codex 구성

개인 연결은 사용자 소유 관리 구성을 씁니다. 공유 연결은 프로젝트 소유 구성을 쓰고
머신 로컬 Runtime Home 경로를 내장하지 않은 채 `VOLICORD_HOME`을 전달합니다. 생성된
명령, 인자, 관리 시작 marker는 시작 시 등록된 Connection과 선택적 프로젝트를
선택합니다. 이 값은 협력적인 시작 맥락이며 identity credential이 아닙니다.

WSL2에서는 Codex 실행 파일, Volicord 실행 파일, 설정 대상, 생성된 각 관리
아티팩트를 독립적으로 해석하고 같은 배포판 ext4 경계를 확인합니다. 저장소
root가 ext4에 있다는 사실은 다른 mount의 중첩 파일을 승인하지 않습니다.

구성이 있다고 Codex 신뢰, 다시 불러오기, 초기화, 도구 검색, 현재 운영 session,
릴리스 셀 상태가 증명되지는 않습니다. 이 사실들은 서로 분리됩니다.

## Volicord Runtime Home

Runtime Home은 registry 저장소, 프로젝트별 저장소, 권위 있는 운영 session, 런타임
관리 아티팩트 bytes 같은 Volicord 소유 런타임 상태만 담습니다. 명시적 선택 또는
[관리 CLI](admin-cli.md#runtime-home-selection)의 플랫폼 규칙으로 선택합니다.

Runtime Home은 Product Repository 안에 두면 안 됩니다. 제품 파일, 유지 문서, 생성
릴리스 증거, 테스트 결과, 스크린샷, 자격 증명, 대화 기록은 Runtime Home 기록이
아닙니다.

WSL2에서는 초기화 전에 Runtime Home 또는 가장 가까운 기존 상위 경로가 정확한
배포판 ext4 경계에 있는지 검증합니다. 프로젝트 home과 런타임 관리 아티팩트도
같은 경계 안에 있어야 하며 Linux 형태의 `/mnt/*` 또는 ext4가 아닌 위치는
지원하지 않습니다.

## 기준 MCP 프로세스

지원 프로세스는 `volicord mcp --stdio`입니다. stdin에서 JSON-RPC를 읽고 stdout에
응답을 씁니다. TCP, HTTP, Unix domain socket 또는 그 밖의 네트워크 전송 listener를
열지 않습니다. 정확한 시작, binding, protocol 동작은 [MCP 전송](mcp-transport.md)이
담당합니다.

프로세스 하나는 활성 Agent Connection 하나에 결속되며 process 시작 시 Volicord가
생성한 새 Registry runtime-session ID를 받습니다. 프로젝트 집합은 저장된
allowlist 또는 명시적으로 선택한 구성원입니다. 임의의 파일시스템 인접성에서 권한을
검색하지 않습니다.

Registry는 process lifecycle milestone과 프로젝트 간 runtime/host session 예약을
담당합니다. 각 프로젝트 데이터베이스는 프로젝트 Agent Session과 host
session/thread/turn 상관관계를 담당합니다. 분리된 데이터베이스 파일 사이에는 SQLite
foreign key를 집행할 수 없으므로 Store는 프로젝트 쓰기 전에 Registry owner를 검증하고
프로젝트 간 uniqueness에는 Registry 예약을 사용합니다. `diagnostics.sqlite`는 분리된
best-effort carrier이며 운영 권한 출처로 사용하지 않습니다.

## 위치와 권한 경계

- Product Repository 쓰기에는 계속 해당 Core 권한이 필요합니다.
- Runtime Home 쓰기 접근은 Product Repository 쓰기 권한이 아닙니다.
- 관리 구성과 시작 marker는 사용자 판단, 쓰기 티켓, host attestation, client
  identity, human identity가 아닙니다.
- 검증된 운영 session은 로컬에서 관찰한 협력적 session 소유권과 현재 프로젝트 권한만
  증명합니다. Binary, host, client, actor, human identity를 증명하지 않습니다.
- 통합 구성을 제거해도 프로젝트 권한 데이터를 삭제하지 않습니다.
- 내보내기와 릴리스 검증 출력은 유지 문서나 Runtime Home trust input이 아니라 명시적인
  외부 출력 위치에 둡니다.

## 관련 담당 문서

- [Agent Connection](agent-connection.md)
- [관리 CLI](admin-cli.md)
- [MCP 전송](mcp-transport.md)
- [저장소](storage.md)
- [보안](security.md)
