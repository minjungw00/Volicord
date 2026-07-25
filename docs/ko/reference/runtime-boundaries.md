# 런타임 경계

이 문서는 `Product Repository`, `Volicord Runtime Home`, 설치된 Volicord 실행 파일,
관리 Codex 구성, stdio MCP 자식 프로세스 사이의 파일시스템 위치와 프로세스 경계를
담당합니다.

## 구성 요소 모델

| 구성 요소 | 경계 |
|---|---|
| Product Repository | 사용자 제품 파일과 명시적인 관리 프로젝트 구성을 담는 정규 Git 작업 트리입니다. |
| Volicord Runtime Home | 로컬 registry, 프로젝트 상태, 권위 있는 운영 session, 구조화된 diagnostic finding, 런타임 소유 아티팩트입니다. |
| Volicord 설치 | 선택한 `volicord` 실행 파일과 build identity입니다. Runtime Home이 아닙니다. |
| 관리 Codex 구성 | 정확한 숨겨진 host launcher를 시작하는 사용자 또는 프로젝트 소유 구성입니다. 재사용 가능한 launch 권한을 담지 않으며 Core 권한이 아닙니다. |
| 숨겨진 host launcher와 stdio adapter | 현재 관리 구성을 검증하고 one-time launch lease를 소비해 관리 stdio를 현재 Agent Connection 하나에 결속하는 로컬 프로세스 하나입니다. 네트워크 서비스가 아닙니다. |

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

개인 연결은 사용자 소유 관리 구성에 선택한 정규 절대 Runtime Home을 정적
`VOLICORD_HOME`으로 쓰며 전달 환경 이름을 두지 않습니다. 공유 연결은 프로젝트 소유
구성에서 `VOLICORD_HOME`만 전달하고 머신 로컬 Runtime Home 경로나 lifecycle 좌표를
내장하지 않습니다. 두 형태 모두 [Agent Connection](agent-connection.md#managed-mcp-launch-contract)이
정의한 하나의 정규 관리 시작 계약에서 projection합니다. 개인 hidden-launcher 명령,
인자, Runtime Home binding은 프로젝트 선택자 없이 시작 시 등록된 Connection을 선택하며,
현재 프로젝트 연결 관계는 Store가 소유하는 Connection Project membership에서 가져옵니다.
공유 시작은 저장소 검색으로 Connection과 프로젝트를 해석합니다. 정적 구성에는 lease,
nonce, 재사용 가능한 secret, raw handle을 넣지 않습니다. 이 값은
협력적인 시작 맥락이며 identity credential이 아닙니다.

숨겨진 launcher는 수명이 짧은 Registry lease를 만들기 전에 그 정확한 현재 entry,
Connection revision, fingerprint를 다시 읽고 엄격하게 검증합니다. Lease는 MCP
bootstrap으로 이어지는 메모리 안의 전환으로만 전달합니다. Store는 lease를 정확히 한 번
소비하면서 `managed_host` runtime을 원자적으로 만듭니다. 이는 evidence-integrity
경계이며 OS actor identity나 adversarial security 보장이 아닙니다.

저장된 managed fingerprint는 setup, repair, staged activation 또는 다른 명시적
configuration 담당 경로가 마지막으로 적용에 성공했거나 채택한 Volicord 관리 host
configuration을 식별합니다. 이 변경 경로는 host 적용이 성공한 뒤에만 fingerprint를
기록합니다. 다른 적용 fingerprint는 Connection integration revision을 바꾸고 이전
verification report를 비웁니다. 운영 검증은 현재 file과 새로 생성한 Host Plan을 관찰하지만
그 plan의 fingerprint를 적용하거나 채택하지 않습니다. 보고서 전용 쓰기는 probe 전에 관찰한
정확한 revision으로 보호합니다.

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
관련 없는 repository content의 소유권을 주장하지 않습니다. Audit과 관찰에 사용하는 현재
policy hash, integration revision, typed runtime command, 전체 managed-file 기대값, 필수 hook
phase를 담당합니다.

운영 연결 검증은 `PATH`에서 실제 `codex` 명령을 찾고 플랫폼 topology 규칙에 따라 관찰한
실행 파일 경로를 canonicalize한 뒤 version 명령을 실행합니다. Path와 version
diagnostic을 기록합니다. 관찰한 version이 바뀌면 현재 host 동작은 운영 관찰을 갱신할 때까지
pending이 됩니다.

## Volicord Runtime Home

Runtime Home은 registry 저장소, 프로젝트별 저장소, 권위 있는 운영 session, 구조화된
diagnostic finding, 런타임 관리 아티팩트 bytes 같은 Volicord 소유 런타임 상태만 담습니다.
명시적 선택 또는 [관리 CLI](admin-cli.md#runtime-home-selection)의 플랫폼 규칙으로
선택합니다.

Runtime Home은 Product Repository 안에 두면 안 됩니다. 제품 파일, 유지 문서, 릴리스
작업 출력, 테스트 결과, 스크린샷, 자격 증명, 대화 기록은 Runtime Home 기록이
아닙니다.

WSL2에서는 초기화 전에 Runtime Home 또는 가장 가까운 기존 상위 경로가 정확한
배포판 ext4 경계에 있는지 검증합니다. 프로젝트 home과 런타임 관리 아티팩트도
같은 경계 안에 있어야 하며 Linux 형태의 `/mnt/*` 또는 ext4가 아닌 위치는
지원하지 않습니다.

Bootstrap 검사는 선택한 최종 경로를 `Absent`, `Ready`, `Incompatible`, `Corrupt`로
분류합니다. 기존 Runtime Home은 읽기 전용으로 열며 정규 `StorageManifest`, 전체 물리
schema, singleton identity, 최종 경로가 정확히 일치할 때만 `Ready`가 됩니다. 검사는
파일을 만들거나 쓰지 않으며 호환되지 않거나 손상된 home의 bytes와 timestamp를 바꾸지
않고 보존합니다. Schema 불일치는 예상·관찰 manifest digest와 relation 범주를 한도
안에서 보고합니다. 복구할 때는 기존 home을 보존하고 명시적 `--home`으로 새 위치를
선택하거나, 담당자가 정의한 importer가 있는 경우에만 그것을 사용합니다. 누락 relation을
만들거나 schema를 patch하거나 숫자로 format을 선택해 제자리에서 복구하지 않습니다.

최종 경로가 `Absent`이면 초기화는 같은 상위 directory 아래에 고유 staging directory를
만듭니다. 그 안에서 Registry, Runtime Home singleton, 최초 installation profile을 만들고
singleton에는 해당 준비에서 생성한 불투명한 UUID 기반 publication ID 하나와 현재 정규
manifest를 기록합니다. Staging을 검증하고 동기화한 뒤 기존 최종 경로를 교체하지 않는
원자적 rename으로 공개합니다. Rename이 성공하면 상위 directory 동기화, read-back,
manifest 확인이 실패할 수 있는 작업보다 먼저 복제할 수 없는 invocation별 publication
guard를 즉시 만듭니다. `AlreadyExists`이면 이 invocation의 staging을 정리하고 rollback
권한 없이 읽기 전용으로 정확히 검증한 `Ready` 승자만 반환합니다.

### Init setup transaction

Runtime Home bootstrap은 더 큰 `volicord init` setup transaction에서 준비되는 구성원
하나입니다. 읽기 전용 planning은 기존 Codex 구성과 소유한 모든 Product Repository
파일도 snapshot하고, 정확한 target bytes를 계산하며, 상위 경로와 conflict를 검증하고,
mutation을 결정적인 순서로 정렬합니다. Prepare 단계는 최종 target을 commit하기 전에
같은 directory의 staging 파일과 Store 복구 entry를 만듭니다.

Commit 단계는 Runtime Home을 공개하거나 검증하고 checkpoint한 Store mutation을 적용한
뒤 Product Repository 파일을 원자 교체하고 Codex 구성을 마지막에 원자 교체한 다음
integration revision을 기록합니다. Setup은 준비됨, 소유한 공개, 소유한 확인 완료, 동시
승자 관찰, 소유한 보존, 소유한 제거 미완료, 소유한 rollback 완료 상태를 명시적으로
유지합니다. 오래된 입력은
concurrent-modification 실패이며 더 새로운 외부 bytes를 보존합니다. Runtime Home
rollback은 플랫폼 소유 제거 직전에 소유 guard가 최종 home을 다시 열어 publication ID,
Runtime Home identity, 정규 manifest digest, 정확한 경로와 schema, 준비한 installation
identity, managed-host 소비 부재를 다시 검증해야 합니다. 재귀 제거 효과, 정확한 경로의
관찰 결과, 상위 entry의 내구성은 서로 분리된 typed fact입니다. 제거가 확인되면 상위
directory를 동기화하기 전에 guard가 terminal 상태가 되므로 동기화 실패는 보존된
publication이 아니라 내구성을 확인하지 못한 부재 publication으로 보고합니다. 제거 오류는
제거가 전혀 없었는지, 일부 제거 가능성이 있는지, 완전한 제거가 관찰되었는지를 기록하며
효과가 불완전하면 terminal 상태가 되어 재시도가 나중의 대체 경로를 삭제하지 못합니다.
정확한 경로의 부재는 관찰 사실이며 이후 재생성을 막는다는 증명이 아닙니다. 소유권
불일치, managed-host 소비, setup 정책은 제거를 막고 관찰자는 동시 승자를 제거하거나
변경하지 않습니다. Runtime Home, Codex 구성, Product Repository가 서로 다른 파일시스템에 있을 수
있으므로 보장은 전역 파일시스템 transaction 하나가 아니라 완전한 준비, 파일별 원자 교체,
한도가 있는 rollback입니다.

Registry는 구조화된 diagnostic finding과 cause edge의 영속 Runtime Home carrier입니다.
Finding은 해당 Connection, project, runtime session, integration revision과 상관관계를
가질 수 있고 MCP runtime session은 terminal finding을 가리킬 수 있습니다. 이 기록은
diagnostic evidence이며 권한을 부여하지 않습니다.

Registry를 열기 전에 발생한 실패는 정확히
`VOLICORD_DIAGNOSTIC_V1 <bounded-json>` 형태인 한도가 있는 stderr 단일 행 fallback 하나를
내보낼 수 있습니다. `bounded-json`은 별도 오류 형태가 아니라 현재 공유
`DiagnosticFinding` 표현입니다. Parser는 정확한 prefix, 한 행, 공유 finding의 한도와
완전한 공유 typed model을 요구합니다. 이 fallback에 환경 dump, 제한 없는 process output,
raw request body 또는 projection하지 않은 다른 입력을 담으면 안 됩니다. 성공한 Registry
쓰기를 대신하지 않으며 Store는 이 envelope을 렌더링하지 않습니다.

### 플랫폼, Runtime Home, 설치 finding

운영 분류는 닫힌 담당 enum을 사용합니다. 렌더링한 오류 문구에서 code나 권장 동작을
파생하지 않습니다. 플랫폼 관찰은 다음 안정 code를 냅니다.

| Code | 조건 |
|---|---|
| `platform.operating_system.unsupported` | 운영체제에 지원되는 릴리스 cell이 없습니다. |
| `platform.target.unsupported` | WSL2와 호환되지 않는 target을 포함해 실행 파일 target을 지원하지 않습니다. |
| `platform.wsl1.unsupported` | 프로세스가 WSL1에서 실행 중입니다. |
| `platform.wsl2.distribution_identity_unavailable` | WSL2 배포판 identity를 관찰할 수 없습니다. |
| `platform.wsl2.distribution_unsupported` | 관찰한 WSL2 배포판이 고정된 cell 밖에 있습니다. |
| `platform.filesystem.unsupported` | 선택한 경로가 지원되는 파일시스템 경계 밖에 있습니다. |
| `platform.filesystem.observation_failed` | 파일시스템 identity를 관찰할 수 없습니다. |
| `platform.observation.failed` | 그 밖의 필수 플랫폼 관찰이 실패했습니다. |

Runtime Home finding은 `runtime_home.path.missing`,
`runtime_home.path.empty_or_relative`, `runtime_home.path.invalid`,
`runtime_home.registry.missing`, `runtime_home.permission.denied`,
`runtime_home.filesystem.unsupported`,
`runtime_home.boundary.owner_mismatch`를 사용합니다. 명시적인 `VOLICORD_HOME`은 비어
있지 않은 절대 경로여야 합니다. 명시적인 상대 CLI `--home`은 계속 CLI 경로 입력으로
취급하며, 이 환경 값 규칙을 적용하기 전에 절대 경로로 해석합니다.

설치 finding은 `installation.executable.missing`,
`installation.executable.not_runnable`,
`installation.build_identity.unavailable`,
`installation.managed_config.inconsistent`를 사용합니다. Finding에는 한도가 있는 범주형
사실만 남기며 환경 dump, 전체 환경 값, 파일시스템 내용, 제한 없는 경로 탐색 결과를
남기지 않습니다.

`volicord mcp preflight`는 선택한 Runtime Home의 읽기 경계 안에 머뭅니다. 파일을
만들거나 write transaction을 열거나 runtime session을 시작하지 않고 정규 관리 구성,
Registry, project 상태, protocol profile, 도구 schema, host contract를 읽습니다. 활성
연결 검증은 별도 경계이므로 preflight 쓰기 가능성은 `not_checked`로 남습니다. 활성 검증은
선택한 store에 rollback 전용 쓰기 가능성 probe를 수행할 수 있으며 이 최소 transaction은
항상 rollback합니다. 모든 protocol revision 및 host 호환성
conformance process는 명령이 소유한 임시 directory 아래의 새로운 일회용 Runtime Home과
Product Repository를 사용합니다. 실행 뒤 전체 fixture를 제거하며, conformance는 선택한
실제 Runtime Home에 runtime session, finding, project record를 만들지 않습니다. 보고서
영속과 diagnostic reconcile은 각각 담당자가 정한 선택 Runtime Home 효과를 유지합니다.

## 기준 MCP 프로세스

관리 구성은 숨겨진 host launcher를 시작하며, 이 launcher는 lease를 소비한 뒤 같은
프로세스에서 stdio adapter로 들어갑니다. 공개 수동 프로세스는
`volicord mcp serve`입니다. Adapter는 stdin에서 JSON-RPC를 읽고 stdout에
응답을 씁니다. TCP, HTTP, Unix domain socket 또는 그 밖의 네트워크 전송 listener를
열지 않습니다. 정확한 시작, binding, protocol 동작은 [MCP 전송](mcp-transport.md)이
담당합니다.

프로세스 하나는 활성 Agent Connection 하나에 결속되며 process 시작 시 Volicord가
생성한 새 Registry runtime-session ID를 받습니다. 프로젝트 집합은 저장된
allowlist 또는 명시적으로 선택한 구성원입니다. 임의의 파일시스템 인접성에서 권한을
검색하지 않습니다.

Registry는 process lifecycle milestone, 구조화된 runtime diagnostic finding과 cause edge,
terminal-finding 연결, 프로젝트 간 runtime/host session 예약을 담당합니다. 각 프로젝트
데이터베이스는 정규화한 host session, turn, hook tool invocation, Guard observation, MCP
전용 session anchor를 담당합니다. 명시적으로 선택한 `CodexMcpTurnMetadata` marker와
`codex-mcp-turn-metadata` profile은 session/thread/turn 상관관계를 제공합니다. 별도로
선택한 `CodexCommandHooks` marker와 `codex-command-hooks` profile은 prompt의 session/turn 또는
tool의 session/turn/tool-use/tool-name 상관관계를 제공하며 command hook에는 thread
좌표가 없습니다.

Runtime source도 명시적입니다. `managed_host`는 one-time lease를 원자적으로 소비하는
경우에만 만들고, `manual_cli`는 공개 stdio 또는 일회용 CLI conformance 경로를
식별하며, `cli_preflight`와 `integration_probe`는 비관리 diagnostic 분류입니다. 현재
공개 preflight는 순수 읽기로 남고 runtime을 만들지 않습니다. 세 비관리 source 중 어느
것도 managed-host 권한 또는 activation evidence를 제공할 수 없습니다.

MCP는 실제 프로젝트를 선택할 때까지 typed native 좌표를 유지하고, 그 뒤 Store가
Connection, 현재 프로젝트 통합 revision, native session으로 로컬 프로젝트 session 좌표를
도출합니다. Guard 관찰은 공유하는 정규화 host row를 만들 수 있지만 MCP 전용 thread나
runtime anchor를 만들 수 없습니다. 첫 실제 managed MCP 도구 호출은 현재 managed runtime을
변경 없이 검증하고 정확한 anchor를 만들거나 검증합니다. 프로젝트 소유권 검증이 commit된
뒤에만 Registry가 현재 소유자 사실을 다시 검증하고 정확한 프로젝트 revision과 함께
프로젝트 간 uniqueness를 예약합니다. 마지막 프로젝트 transaction에서 runtime을 붙입니다.
프로젝트 소유권 충돌은 Registry 예약을 남기지 않습니다. Unbound MCP anchor와 프로젝트
attach가 없는 Registry 예약은 각각 권한 효력이 없습니다. 소유자 상태가 바뀌지 않은 정확한
replay는 중단된 마지막 attach를 복구합니다. Process row는 lease나 liveness signal이 아니므로
crash 뒤 열린 것처럼 남은 row와 concurrent process가 Guard 상관관계를 선택하거나 막지
않습니다. `diagnostics.sqlite`는 분리된 best-effort carrier이며 운영 권한 출처로 사용하지
않습니다.

## 위치와 권한 경계

- Product Repository 쓰기에는 계속 해당 Core 권한이 필요합니다.
- Runtime Home 쓰기 접근은 Product Repository 쓰기 권한이 아닙니다.
- 관리 구성은 협력적 routing 맥락을 선택하고 one-time lease는 bootstrap을 가로질러
  source evidence를 보존합니다. 어느 쪽도 사용자 판단, 쓰기 티켓, OS actor identity,
  human identity를 공급하지 않습니다.
- 검증된 운영 session은 로컬에서 관찰한 협력적 session 소유권과 현재 프로젝트 권한을
  성립시킵니다. Client, actor, 운영체제 사용자, human identity를 성립시키지는 않습니다.
- 내부 runtime/project session ID는 비공개 로컬 상관관계 좌표이며 host-native identity,
  actor identity, credential이 아닙니다.
- 변경 불가능한 Connection 통합 instance ID와 integration generation은 Runtime Home
  Store 소유 lifecycle 좌표입니다. 현재 소유자 입력과 함께 로컬 lifecycle 및
  상관관계 revision을 파생합니다.
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
