# 시스템 요구사항

이 문서는 Volicord 최초 릴리스의 운영 환경 전제 조건을 담당합니다. 사용할 수
있는 플랫폼 환경, WSL2 토폴로지, 실행 파일과 파일 시스템 전제 조건, Runtime
Home과 Product Repository 배치, 설정 또는 검증을 중단해야 하는 조건을 정의합니다.

일반 빌드, 패키지, checksum, 플랫폼, 게시 검증은
[검증](../maintain/validation.md)이, 관리 운영 session 권한은
[Agent Connection](agent-connection.md)이 담당합니다. 실행 파일 path와 version 관찰은
현재 운영 검증의 diagnostic 입력입니다.

<a id="surface-stability"></a>
## 표면 안정성

네 `PlatformEnvironment` 값, 게시 target triple 다섯 개, 정확한 최초 릴리스 WSL2 배포판 식별 정보,
WSL2 토폴로지와 ext4 경계, 관리형 stdio MCP 전제 조건, 중단 기준은 안정
계약입니다. 그 밖의 runner 이미지, 패키지 관리자 명령, 실행 파일 위치와 진단
문구는 다른 담당 문서가 안정으로 지정하지 않는 한 릴리스 또는 구현
세부사항입니다.

<a id="first-release-environment-matrix"></a>
## 최초 릴리스 환경 행렬

Volicord는 binary target 다섯 개를 게시합니다. 해당 binary가 지원하는 실행 환경은
다음과 같습니다.

| `target_triple` | `platform_environment` | 필수 경계 |
|---|---|---|
| `x86_64-unknown-linux-gnu` | `linux` | 모든 구성요소가 native x86-64 Linux에서 실행됩니다. |
| `aarch64-unknown-linux-gnu` | `linux` | 모든 구성요소가 native AArch64 Linux에서 실행됩니다. |
| `aarch64-apple-darwin` | `macos` | 모든 구성요소가 native Apple Silicon macOS에서 실행됩니다. |
| `x86_64-apple-darwin` | `macos` | 모든 구성요소가 native Intel x86-64 macOS에서 실행됩니다. |
| `x86_64-pc-windows-msvc` | `native_windows` | 모든 구성요소가 native x86-64 Windows에서 실행됩니다. WSL 좌표는 사용할 수 없습니다. |
| `x86_64-unknown-linux-gnu` | `wsl2` | 모든 구성요소가 아래 조건을 만족하는 같은 WSL2 배포판 내부에서 실행되고 그 Linux 파일 시스템을 사용합니다. |

Target 호환성은 Volicord 플랫폼 제약입니다. 같은 x86-64 Linux binary를 실행하더라도
native Linux와 WSL2는 별도 환경입니다. 한
architecture나 환경은 다른 환경의 런타임 전제 조건을 성립시키지 않습니다. 릴리스
패키징은 계속 게시 target을 각각 빌드하고 점검합니다.

모든 행의 최초 릴리스 제품 표면은 다음과 같습니다.

- `host_kind=codex`
- `integration_profile=record`
- `connection_scope=personal` 또는 `connection_scope=shared`
- 관리형 stdio MCP
- 사용자 행동을 위한 CLI inbox

다른 호스트, profile, transport와 User Channel은 이 릴리스 범위 밖입니다.

<a id="wsl2-topology"></a>
## WSL2 토폴로지

지원하는 WSL2 토폴로지는 하나의 완결된 환경입니다.

```text
고정된 Ubuntu LTS WSL2 배포판 하나
  ├─ Codex 프로세스
  ├─ Volicord 프로세스
  ├─ 배포판 ext4 파일 시스템의 Product Repository
  ├─ 배포판 ext4 파일 시스템의 Volicord Runtime Home
  └─ 배포판 ext4 파일 시스템의 Codex/Volicord 실행 파일, 관리 설정,
     생성된 관리 아티팩트
```

최초 릴리스의 WSL2 경계와 배포판 식별 정보는 다음과 같습니다.

| 관찰 | 요구사항 |
|---|---|
| `/proc/sys/kernel/osrelease` | 지원되는 Microsoft WSL2 커널 |
| `/etc/os-release` `ID` | `ubuntu` |
| `/etc/os-release` `VERSION_ID` | `24.04` |

플랫폼 점검은 커널 릴리스를 사용하여 네이티브 Linux, WSL1, WSL2를 구분합니다.
지원되는 WSL2 커널에서는 `/etc/os-release`로 Ubuntu ID와 버전을 확인합니다. WSL
환경 변수는 전제 조건이 아닙니다. 파일 시스템 관찰은 지원 토폴로지를 집행합니다.
운영 권한은 현재 Connection과 session 소유권을 별도로 검증합니다.

WSL2 런타임 경계는 환경이 WSL2임을 명시적으로 확인하고
`target_triple=x86_64-unknown-linux-gnu`를 요구해야 합니다. 일반 Linux `target_os`
결과만으로는 부족합니다. 운영 Connection과 session 기록은 해당 Runtime Home과
플랫폼 환경에만 남으며 `linux`나 `native_windows` 권한으로 변환하지 않습니다.

Product Repository, Runtime Home, Codex 실행 파일, Volicord 실행 파일, 관리
Codex 설정, 생성된 모든 관리 아티팩트는 해당 배포판의 Linux ext4 파일 시스템
내부로 해석되어야 합니다. 다른 배포판, 이미지, 파일 시스템을 동등하다고
추정하지 않습니다.

다음 WSL 토폴로지는 지원하지 않으며 설치 또는 관리 launch 전에 machine-readable
unsupported-environment reason으로 실패해야 합니다.

- WSL1
- Codex는 Windows에서, Volicord·저장소·Runtime Home 중 하나는 WSL2에서 실행하는 구성
- Codex는 WSL2에서, Volicord 프로세스·저장소·Runtime Home 중 하나는 네이티브 Windows에 두는 구성
- Product Repository나 Runtime Home을 `/mnt/c`, `/mnt/d`, 다른 `/mnt/*` 경로 또는 DrvFS mount에 두는 구성
- Windows와 Linux 경로, PID, 환경 값, Connection, runtime session 또는 project session을 변환하거나 서로 같다고 추정하는 동작
- 네이티브 Windows Runtime Home session 기록을 WSL2에서 사용하거나 WSL2 session 기록을 네이티브 Windows에서 사용하는 동작
- `/etc/os-release` 식별 정보가 현재 최초 릴리스 WSL2 범위 밖인 배포판

WSL 종료나 재시작은 살아 있는 관리 runtime session을 끝냅니다. 그 project session은
이후 호출에 권한을 줄 수 없으며, 새로운 관리 MCP lifecycle이 새 runtime session과
project session을 기록해야 합니다.

플랫폼 관찰과 지원하지 않는 토폴로지 결과는 기계 판독할 수 있습니다. 호스트를
분류하는 데 필요한 커널 릴리스를 읽을 수 없으면 `Unavailable` 결과인
`platform_environment_unavailable`을 사용합니다. 이 커널 분류가 네이티브 Linux로
끝나면 배포판 식별 정보를 관찰하지 않습니다. 지원되는 WSL2 커널에서
`/etc/os-release`를 읽을 수 없거나 필수 `ID` 또는 `VERSION_ID`가 없거나 잘못된
형식이면 `Unavailable` 사유인 `wsl2_distribution_unavailable`을 사용합니다. 형식이
올바른 식별 정보의 `ID` 또는 `VERSION_ID`가 지원 범위 밖이면 `Rejected` 사유인
`unsupported_wsl2_distribution`을 사용합니다. `unsupported_wsl1`은 WSL1,
`unsupported_wsl2_filesystem`은 ext4가 아닌 구성요소를 뜻합니다. 관찰할 수 없는
결과를 거절된 환경이나 네이티브 환경으로 바꾸지 않습니다.

<a id="toolchain-requirements"></a>
## 도구 체인 요구사항

workspace 빌드와 테스트에는 저장소가 선언한 Rust 도구 체인이 필요합니다. 현재
유지되는 workspace는 Rust 1.85 이상 호환 stable Rust를 대상으로 합니다. 포맷,
검사, lint, 테스트에는 같은 도구 체인의 Cargo를 사용합니다.

런타임 전제 조건은 다음과 같습니다.

- 선택한 정확한 target 및 플랫폼 환경용으로 최종 확정된 Volicord 실행 파일
- 관리 구성을 시작할 수 있는 사용 가능한 Codex 실행 파일
- Volicord 빌드가 제공하는 SQLite 지원
- 선택한 네이티브 플랫폼 또는 WSL2 adapter가 요구하는 파일 시스템 동작
- 관리형 MCP 프로세스 경계를 보존하는 stdio pipe

workflow가 Git 객체 ID를 제공하거나 검증할 때, 또는 선택한 Product Repository
동작이 Git을 명시적으로 요구할 때 Git이 필요합니다. Git 객체 ID 표기는
[외부 계약](external-contracts.md#shared-git-object-id-contract)을 따릅니다.

## 실행 파일과 프로세스 요구사항

관리 프로세스는 구성된 Codex 실행 파일을 찾아 실행할 수 있어야 합니다. 활성 검증에서는
실행 파일 탐색과 version 명령이 성공해야 합니다. 검증은 해석한 path와 관찰한 host version을
diagnostic으로 보고합니다. 관찰한 version이 달라지면 관리 Codex 동작을 다시 관찰할 때까지
현재 운영 관찰이 pending이 됩니다. 실행 파일 사용 가능성만으로 agent, 운영체제 사용자,
human identity가 성립하지는 않습니다.

관리 Codex 설정은 의도한 Volicord 실행 파일을 관리형 stdio MCP로 시작해야
합니다. Adapter는 정규 관리 시작 계약을 통해 관리 entry, command, arguments, 개인
정적 또는 공유 전달 Runtime Home binding, configuration target, 플랫폼 전제 조건을
검증합니다. 관리 launch marker는 협력적 routing 맥락이지 credential이 아닙니다. 비어
있는 환경 값과 없는 환경 값은 다릅니다.

실행 파일, 설정, 프로세스, client, version 관찰은 diagnostic 또는 설정 사실입니다.
Runtime 권한은 현재 Connection, project membership, 허용 mode, Store가 소유한 관리
runtime/project session과 정확한 binding을 검증합니다.

## Runtime Home 요구사항

해당 런타임 담당자가 허용하는 경우 `VOLICORD_HOME`이 Volicord Runtime Home을
선택합니다. 결과 경로는 비어 있지 않고 선택한 플랫폼의 경로 규칙에 따라
절대 경로여야 하며, 초기화와 관리 repair에 쓸 수 있고 관리형 Volicord 프로세스가
접근할 수 있어야 합니다.

Runtime Home은 하나의 플랫폼 환경 안에 있어야 합니다. 네이티브 Windows와 WSL2
경로를 변환하거나 공유하지 않습니다. WSL2에서는 경로와 가장 가까운 기존 상위
경로가 배포판 ext4 파일 시스템에 있어야 하고 `/mnt/*` 아래에 두지 않습니다.
Linux 형태의 경로 문자열만으로는 충분하지 않습니다.

새 개발 데이터는 현재 기준 SQLite 계약으로 만듭니다. 다른 manifest를 가진 기존
데이터베이스를 upgrade, import 또는 재해석하지 않습니다. 새 Runtime Home이나
명시적으로 비어 있는 새 대상을 사용합니다.

## Product Repository 요구사항

personal connection은 명시적인 Product Repository 하나를 결속합니다. shared
connection은 저장소에 이식 가능한 관리 Codex 설정을 설치하고, 공유 파일에 개발자
로컬 project ID나 Runtime Home 경로를 넣지 않은 채 현재 clone을 해석합니다.

저장소 경로는 다음 조건을 만족해야 합니다.

- 비어 있지 않고 현재 플랫폼의 정규 경로 규칙으로 해석됩니다.
- 선택한 Connection에 현재 등록된 저장소를 식별합니다.
- 요청한 install, repair 또는 uninstall 동작에 필요한 쓰기만 허용합니다.
- Codex, Volicord와 Runtime Home과 같은 플랫폼 환경에 있습니다.
- WSL2에서는 `/mnt/*` 밖의 배포판 ext4 파일 시스템으로 해석됩니다.

저장소 이동, 정규 identity 변경, Connection scope 변경 또는 integration revision
증가는 이전 project session을 stale로 만듭니다. 새로운 관리 MCP project session이
필요하며 호출자가 이전 좌표를 암묵적으로 고쳐 쓰면 안 됩니다.

## Codex 설정 요구사항

Codex adapter는 관리 항목의 탐색, 엄격한 parsing, 정규 projection, 원자적 적용,
검증, drift 탐지, repair와 안전한 uninstall을 담당합니다. 설정은 관련 없는 사용자
설정을 보존하고, 소유하지 않은 충돌을 덮어쓰지 않고 거절해야 합니다.

personal 및 shared 설정 위치는 adapter가 담당하는 세부사항입니다. Core는
`ValidatedAgentSession`만 받으며 Codex 설정 파일을 읽거나, 셸 명령을 tokenization하거나,
wrapper marker를 검사하거나, 플랫폼 경로를 추정하지 않습니다. Store는 MCP가 이
경계를 검증할 때 사용하는 Connection, membership, integration revision, 관리 runtime
session, project session 기록을 담당합니다.

Repair는 탐지한 reason을 보고한 뒤 adapter가 소유한 설정과 복구 가능한 typed
값만 다시 만들 수 있습니다. Uninstall은 정확히 현재 소유한 항목만 제거하며,
변경되었거나 소유하지 않은 항목은 거절합니다.

## 관리형 MCP 환경 요구사항

관리 프로세스의 공개 MCP transport는 stdio뿐입니다. 관리 launch 맥락에서
정규 관리 시작 binding을 받아야 합니다. Personal entry는 Connection과 선택한 정규 절대
Runtime Home을 정적 값으로 담고 프로젝트 선택자는 담지 않으며 환경 이름을 전달하지
않습니다. 권위 있는 저장소 연결 관계는 해당 Connection에서 Store가 소유하는 프로젝트
membership으로 남습니다. 저장소에 이식 가능한 shared discovery는 `VOLICORD_HOME`만
전달하고 머신 로컬 ID나 경로를 넣지 않은 채 등록된 현재 clone을 해석합니다. 필수 launch
맥락이 없거나 비었거나 충돌하거나 알 수 없으면 거절하며 다른 Connection에서 추정하지
않습니다. Host/profile marker는 이 협력적 경로를 선택할 뿐 tool 호출을 승인하지 않습니다.

Initialize 때 MCP는 제한된 client name/version과 선택적 host version diagnostic을
포함한 managed-host runtime session 하나를 기록합니다. 각 project tool 호출에서는
project session을 기록하거나 선택하고, Core 맥락을 만들기 전에 현재 Connection
활성화, membership, mode, runtime/project session 소유권, 두 integration revision을
검증합니다. 새로 관찰한 제한 안의 host version은 운영 관찰을 갱신해 호환성을
확인합니다.

비밀과 관련 없는 ambient 환경 값은 관리 설정에 복사하지 않습니다. 진단은 token,
전체 민감 payload 또는 가리지 않은 민감 절대 경로를 출력하면 안 됩니다.

## 중단 기준

다음 조건 중 적용되는 것이 있으면 설치, 검증, repair, 관리 launch 또는 project 호출을
사용을 중단해야 합니다.

- 호스트와 profile이 정확히 `codex`, `record`가 아닙니다.
- 플랫폼 환경이 없거나 모호하거나 네 값 집합 밖입니다.
- target triple이 없거나 알 수 없거나 플랫폼 환경과 일치하지 않습니다.
- 관리 설정, project, Connection, membership, mode, runtime session, project session 또는 현재 integration revision이 일치하지 않습니다.
- 관리 설정이 손상되었거나 소유하지 않은 항목이거나 repair 가능한 담당 경계 밖으로 drift했습니다.
- 저장된 typed 설정 행동이나 필요한 다른 담당 값이 손상되었습니다.
- 네이티브 Windows/WSL2 교차, WSL1, `/mnt/*`, DrvFS 또는 지원하지 않는 WSL 배포판이 관찰됩니다.
- Runtime Home이나 Product Repository를 안전하게 해석하거나 접근할 수 없습니다.
- 관리형 stdio를 구성할 수 없습니다.
- 필수 읽기나 플랫폼 primitive를 사용할 수 없습니다.

결과는 해당 `Rejected`, `Unavailable`, `Corrupt` 범주와 도메인 reason을 보존해야
합니다. 기본 session, 합성 권한, fallback 호스트, 추정 플랫폼 또는 부분 성공을
만들면 안 됩니다.

## 인접 담당 문서

- 최초 릴리스 포함 및 제외 표면: [범위](scope.md)
- 운영 session, 저장된 설정 행동 및 adapter/Core 경계: [Agent Connection](agent-connection.md)
- 빌드, 패키지, 플랫폼, 릴리스 검증: [검증](../maintain/validation.md)
- 런타임 경로 및 저장소 경계: [런타임 경계](runtime-boundaries.md)
- SQLite 형식 수용: [저장소 버전 관리](storage-versioning.md)
- 제품 전체 실패 의미: [실패 모델](failure-model.md)
- 위협 모델 및 비보장: [보안](security.md)
