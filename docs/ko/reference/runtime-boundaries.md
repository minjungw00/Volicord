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
- Guard hook 구성, dispatch/phase wrapper, rule instruction
- `AGENTS.md`의 Volicord 관리 블록
- 선택적인 `.git/info/exclude` 관리 블록

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

구성이 있다고 Codex 신뢰, 다시 불러오기, 초기화, 도구 검색, 안전한 도구 동작,
Guard 관찰, 현재 운영 session이 증명되지는 않습니다. 이 사실들은 서로 분리됩니다.

Connection mode 전환은 관리 Codex configuration이나 Product Repository file을 다시 쓰지
않습니다. 일관된 revision 전환은 Connection mode와 generation, verification report,
소유한 모든 엄격한 Guard manifest의 integration revision이라는 Registry 상태에만
한정됩니다. 실제 전환 뒤 CLI는 새 managed host가 현재 runtime evidence를 만들 수 있도록
reload action 하나를 내보내며, 같은 mode의 no-op에서는 내보내지 않습니다.

Runtime Home의 Guard manifest는 위 파일 가운데 정확한 Guard-managed subset과 typed
runtime command에 대한 소유 inventory입니다. Managed script entry는 모든 플랫폼에서 executable
동작을 요구하지만 파일시스템 조사와 permission 복구는 플랫폼별로 수행합니다. Manifest는
관련 없는 repository content의 소유권을 주장하지 않으며 host capability 또는 runtime
인증도 아닙니다.

운영 연결 검증은 `PATH`에서 실제 `codex` 명령을 찾고 플랫폼 topology 규칙에 따라 관찰한
실행 파일 경로를 canonicalize한 뒤 version 명령을 실행합니다. Path와 version
diagnostic만 기록합니다. Package-native artifact를 해석하거나 executable bytes를
hash하거나 플랫폼 실행 파일 identity를 도출하거나 version이 정확한 호스트 allowlist에 있어야 한다고
요구하지 않습니다.

## Volicord Runtime Home

Runtime Home은 registry 저장소, 프로젝트별 저장소, 권위 있는 운영 session, 런타임
관리 아티팩트 bytes 같은 Volicord 소유 런타임 상태만 담습니다. 명시적 선택 또는
[관리 CLI](admin-cli.md#runtime-home-selection)의 플랫폼 규칙으로 선택합니다.

Runtime Home은 Product Repository 안에 두면 안 됩니다. 제품 파일, 유지 문서, 릴리스
작업 출력, 테스트 결과, 스크린샷, 자격 증명, 대화 기록은 Runtime Home 기록이
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
session/thread/turn 상관관계를 담당합니다. MCP는 실제 프로젝트를 선택할 때까지 이 native
좌표를 유지하고, 그 뒤 Store가 Connection, 현재 프로젝트 통합 revision, native session으로
프로젝트 session 좌표를 도출합니다. 분리된 데이터베이스 파일 사이에는 SQLite foreign key를
집행할 수 없으므로 유효한 Guard 관찰은 먼저 unbound 프로젝트 session을 만들 수 있습니다.
동일한 host identity의 첫 실제 managed MCP 도구 호출이 Registry owner를 검증하고 정확한
프로젝트 revision과 함께 프로젝트 간 uniqueness를 예약한 뒤 그 runtime을 프로젝트 row에
붙입니다. 정확한 replay는 중단된 attach를 복구합니다. Process row는 lease나 liveness
signal이 아니므로 crash 뒤 열린 것처럼 남은 row와 concurrent process가 Guard 상관관계를
선택하거나 막지 않습니다. `diagnostics.sqlite`는 분리된 best-effort carrier이며 운영 권한
출처로 사용하지 않습니다.

## 위치와 권한 경계

- Product Repository 쓰기에는 계속 해당 Core 권한이 필요합니다.
- Runtime Home 쓰기 접근은 Product Repository 쓰기 권한이 아닙니다.
- 관리 구성과 시작 marker는 사용자 판단, 쓰기 티켓, host attestation, client
  identity, human identity가 아닙니다.
- 검증된 운영 session은 로컬에서 관찰한 협력적 session 소유권과 현재 프로젝트 권한만
  증명합니다. Binary, host, client, actor, human identity를 증명하지 않습니다.
- 내부 runtime/project session ID는 비공개 로컬 상관관계 좌표이며 host-native identity,
  actor identity, credential이 아닙니다.
- 변경 불가능한 Connection 통합 instance ID와 integration generation은 Runtime Home
  lifecycle 좌표입니다. Host나 actor identity, release certification, security
  credential, 호출자가 고르는 값이 아닙니다.
- 명시적 제거와 Connection migration은 저장소 담당자가 정한 순서로 선택한
  connection/project 소유 Registry 통합 행만 폐기합니다. 마지막 프로젝트의 pending
  migration은 host 정리가 성공할 때까지 이 완전한 Registry inventory를 유지합니다. 어느
  경로도 프로젝트 등록이나 프로젝트 로컬 Agent Session, Guard 및 workflow 이력,
  evidence와 그 밖의 권한 데이터를 삭제하지 않으며, 유지된 이력은 현재 Registry
  소유권이 없으면 현재 호출에 권한을 부여할 수 없습니다.
- 내보내기와 릴리스 검증 출력은 유지 문서나 Runtime Home trust input이 아니라 명시적인
  외부 출력 위치에 둡니다.

## 관련 담당 문서

- [Agent Connection](agent-connection.md)
- [관리 CLI](admin-cli.md)
- [MCP 전송](mcp-transport.md)
- [저장소](storage.md)
- [보안](security.md)
