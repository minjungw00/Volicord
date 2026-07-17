# 시스템 요구사항

이 문서는 Volicord 최초 릴리스의 운영 환경 전제 조건을 담당합니다. 사용할 수
있는 플랫폼 환경, WSL2 토폴로지, 실행 파일과 파일 시스템 전제 조건, Runtime
Home과 Product Repository 배치, 설정 또는 검증을 중단해야 하는 조건을 정의합니다.

이 문서는 릴리스 셀이 실제로 실행되거나 통과했다고 주장하지 않습니다. 정확히
최종 확정된 아티팩트의 결과는 [호스트 릴리스 증거](host-release-evidence.md)가,
관리형 binding과 영수증 의미는 [Agent Connection](agent-connection.md)이 담당합니다.

<a id="surface-stability"></a>
## 표면 안정성

네 `PlatformEnvironment` 값, WSL2 토폴로지와 ext4 경계, 관리형 stdio MCP 전제
조건, 중단 기준은 안정 계약입니다. 특정 runner 이미지, 패키지 관리자 명령,
실행 파일 위치와 진단 문구는 다른 담당 문서가 안정으로 지정하지 않는 한 릴리스
또는 구현 세부사항입니다.

## 최초 릴리스 환경 행렬

Volicord에는 서로 독립적인 네 릴리스 대상 환경이 있습니다.

| `platform_environment` | 필수 경계 |
|---|---|
| `linux` | Volicord, Codex, Product Repository, Runtime Home이 네이티브 Linux에서 실행됩니다. |
| `macos` | Volicord, Codex, Product Repository, Runtime Home이 네이티브 macOS에서 실행됩니다. |
| `native_windows` | Volicord, Codex, Product Repository, Runtime Home이 네이티브 Windows 구성요소로 실행됩니다. WSL 경로, 프로세스, binding, 영수증은 사용할 수 없습니다. |
| `wsl2` | 모든 구성요소가 아래 조건을 만족하는 같은 WSL2 배포판 내부에서 실행되고 그 Linux 파일 시스템을 사용합니다. |

환경에 대한 릴리스 지원 주장은 정확히 일치하는 `CodexReleaseCell`의
`validation_evidence.status=passed`일 때만 성립합니다. 한 행의 통과는 다른 행을
성립시키지 않습니다. 저장소 테스트, 교차 컴파일, 패키징이나 비슷한 target
triple은 실제 셀 실행을 대신하지 않습니다.

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
  └─ 배포판 ext4 파일 시스템의 Volicord Runtime Home
```

WSL2 릴리스 셀은 환경이 WSL2임을 명시적으로 확인해야 합니다. 일반 Linux
`target_os` 결과만으로는 부족합니다. `ManagedHostBinding`과
`HostVerificationReceipt`는 `platform_environment=wsl2`를 결속하며 `linux`나
`native_windows` 증거로 재사용할 수 없습니다.

현재 WSL2 셀은 릴리스 증거에 고정된 Ubuntu LTS 이미지 한 종류만 사용합니다.
다른 배포판이나 이미지를 동등하다고 추정하지 않습니다. Product Repository와
Runtime Home은 배포판 Linux ext4 파일 시스템 내부로 해석되어야 합니다.

다음 WSL 토폴로지는 지원하지 않으며 설치 또는 영수증 사용 전에 machine-readable
unsupported-environment reason으로 실패해야 합니다.

- WSL1
- Codex는 Windows에서, Volicord·저장소·Runtime Home 중 하나는 WSL2에서 실행하는 구성
- Codex는 WSL2에서, Volicord 프로세스·저장소·Runtime Home 중 하나는 네이티브 Windows에 두는 구성
- Product Repository나 Runtime Home을 `/mnt/c`, `/mnt/d`, 다른 `/mnt/*` 경로 또는 DrvFS mount에 두는 구성
- Windows와 Linux 경로, PID, 환경 값, process binding 또는 영수증을 변환하거나 서로 같다고 추정하는 동작
- 네이티브 Windows 영수증을 WSL2에서 사용하거나 WSL2 영수증을 네이티브 Windows에서 사용하는 동작
- 현재 WSL2 릴리스 셀이 명시하지 않은 배포판

WSL 종료나 재시작은 살아 있는 프로세스 identity를 무효화합니다. 프로세스 또는
최신성 좌표가 더 이상 맞지 않는 binding과 영수증은 stale로 거절해야 하며, 새
verify 흐름을 통해 새 영수증을 만들어야 합니다.

<a id="toolchain-requirements"></a>
## 도구 체인 요구사항

workspace 빌드와 테스트에는 저장소가 선언한 Rust 도구 체인이 필요합니다. 현재
유지되는 workspace는 Rust 1.85 이상 호환 stable Rust를 대상으로 합니다. 포맷,
검사, lint, 테스트와 릴리스 검증 계약 테스트에는 같은 도구 체인의 Cargo를
사용합니다.

런타임 전제 조건은 다음과 같습니다.

- 선택한 플랫폼 환경용으로 최종 확정된 Volicord 실행 파일
- 현재 플랫폼, `record` profile, 필수 capability와 정확히 일치하는 통과 릴리스 셀에 등록된 Codex 실행 파일
- Volicord 빌드가 제공하는 SQLite 지원
- 선택한 네이티브 플랫폼 또는 WSL2 adapter가 요구하는 파일 시스템 동작
- 관리형 MCP 프로세스 경계를 보존하는 stdio pipe

workflow가 Git 객체 ID를 제공하거나 검증할 때, 또는 선택한 Product Repository
동작이 Git을 명시적으로 요구할 때 Git이 필요합니다. Git 객체 ID 표기는
[외부 계약](external-contracts.md#shared-git-object-id-contract)을 따릅니다.

## 실행 파일과 프로세스 요구사항

관리 프로세스는 설정과 검증이 결속할 정확한 Codex 아티팩트를 찾아 실행할 수
있어야 합니다. 명령 이름을 찾은 것만으로는 지원이 성립하지 않습니다. 검증은
해석한 실행 파일을 hash하고, 현재 플랫폼의 정확한 릴리스 manifest 항목과
대조하며, binding이 요구하는 프로세스와 capability 관찰을 기록하고, 모든 adapter
점검이 성공한 뒤에만 영수증을 발행합니다.

관리 Codex 설정은 의도한 Volicord 실행 파일을 관리형 stdio MCP로 시작해야
합니다. adapter는 정규 `ManagedHostBinding`을 통해 정확한 command, arguments,
forwarded environment, configuration target, process binding, required capabilities와
platform environment를 검증합니다. 비어 있는 환경 값과 없는 환경 값은 다릅니다.

실행 파일 identity, 설정 identity, 프로세스 identity와 영수증 최신성은 서로
독립된 점검입니다. 하나의 일치가 다른 점검을 대신하지 않습니다.

## Runtime Home 요구사항

해당 런타임 담당자가 허용하는 경우 `VOLICORD_HOME`이 Volicord Runtime Home을
선택합니다. 결과 경로는 비어 있지 않고 선택한 플랫폼의 경로 규칙에 따라
절대 경로여야 하며, 초기화와 관리 repair에 쓸 수 있고 관리형 Volicord 프로세스가
접근할 수 있어야 합니다.

Runtime Home은 하나의 플랫폼 환경 안에 있어야 합니다. 네이티브 Windows와 WSL2
경로를 변환하거나 공유하지 않습니다. WSL2에서는 배포판 ext4 파일 시스템에 두고
`/mnt/*` 아래에 두지 않습니다.

새 개발 데이터는 현재 기준 SQLite 계약으로 만듭니다. 다른 manifest를 가진 기존
데이터베이스를 upgrade, import 또는 재해석하지 않습니다. 새 Runtime Home이나
명시적으로 비어 있는 새 대상을 사용합니다.

## Product Repository 요구사항

personal connection은 명시적인 Product Repository 하나를 결속합니다. shared
connection은 저장소에 이식 가능한 관리 Codex 설정을 설치하고, 공유 파일에 개발자
로컬 project ID나 Runtime Home 경로를 넣지 않은 채 현재 clone을 해석합니다.

저장소 경로는 다음 조건을 만족해야 합니다.

- 비어 있지 않고 현재 플랫폼의 정규 경로 규칙으로 해석됩니다.
- 관리 binding 및 영수증이 사용하는 같은 저장소를 식별합니다.
- 요청한 install, repair 또는 uninstall 동작에 필요한 쓰기만 허용합니다.
- Codex, Volicord와 Runtime Home과 같은 플랫폼 환경에 있습니다.
- WSL2에서는 `/mnt/*` 밖의 배포판 ext4 파일 시스템으로 해석됩니다.

저장소 이동, 정규 identity 변경 또는 connection scope 변경은 이전 binding이나
영수증을 불일치 상태로 만듭니다. 다시 검증해야 하며 호출자가 이전 좌표를
암묵적으로 고쳐 쓰면 안 됩니다.

## Codex 설정 요구사항

Codex adapter는 관리 항목의 탐색, 엄격한 parsing, 정규 projection, 원자적 적용,
검증, drift 탐지, repair와 안전한 uninstall을 담당합니다. 설정은 관련 없는 사용자
설정을 보존하고, 소유하지 않은 충돌을 덮어쓰지 않고 거절해야 합니다.

personal 및 shared 설정 위치는 adapter가 담당하는 세부사항입니다. Core와 Store는
정규 binding 데이터와 typed 검증 영수증만 받습니다. Codex 설정 파일을 읽거나,
셸 명령을 tokenization하거나, wrapper marker를 검사하거나, 플랫폼 경로를 추정하지
않습니다.

Repair는 탐지한 reason을 보고한 뒤 adapter가 소유한 설정과 복구 가능한 typed
값만 다시 만들 수 있습니다. Uninstall은 정확히 현재 소유한 항목만 제거하며,
변경되었거나 소유하지 않은 항목은 거절합니다.

## 관리형 MCP 환경 요구사항

관리 프로세스의 공개 MCP transport는 stdio뿐입니다. 현재 관리 launch 계약이
요구하는 정확한 project, connection, Runtime Home, host, profile, binding과 플랫폼
좌표를 받아야 합니다. 필수 좌표가 없거나, 비었거나, 중복되거나, 충돌하거나,
알 수 없으면 거절합니다. 현재 디렉터리, 이웃 설정이나 다른 connection에서
추정하지 않습니다.

비밀과 관련 없는 ambient 환경 값은 관리 설정에 복사하지 않습니다. 진단은 token,
전체 민감 payload 또는 가리지 않은 민감 절대 경로를 출력하면 안 됩니다.

## 중단 기준

다음 조건 중 적용되는 것이 있으면 설치, 검증, repair, 관리 launch 또는 영수증
사용을 중단해야 합니다.

- 호스트와 profile이 정확히 `codex`, `record`가 아닙니다.
- 플랫폼 환경이 없거나 모호하거나 네 값 집합 밖입니다.
- 정확한 Codex 아티팩트와 필수 capability가 현재 플랫폼의 통과 manifest 셀에 없습니다.
- 실행 파일, 프로세스, binding, 설정, project, connection, policy, capability 또는 최신성 좌표가 일치하지 않습니다.
- 관리 설정이 손상되었거나 소유하지 않은 항목이거나 repair 가능한 담당 경계 밖으로 drift했습니다.
- 저장된 typed 설정 행동이나 필요한 다른 담당 값이 손상되었습니다.
- 네이티브 Windows/WSL2 교차, WSL1, `/mnt/*`, DrvFS 또는 지원하지 않는 WSL 배포판이 관찰됩니다.
- Runtime Home이나 Product Repository를 안전하게 해석하거나 접근할 수 없습니다.
- 관리형 stdio를 구성할 수 없습니다.
- 필수 읽기나 플랫폼 primitive를 사용할 수 없습니다.

결과는 해당 `Rejected`, `Unavailable`, `Corrupt` 또는 `UnsupportedContract` 범주와
도메인 reason을 보존해야 합니다. 기본 binding, 합성 영수증, fallback 호스트,
추정 플랫폼 또는 부분 성공을 만들면 안 됩니다.

## 인접 담당 문서

- 최초 릴리스 포함 및 제외 표면: [범위](scope.md)
- binding, 영수증, 저장된 설정 행동 및 adapter/Core 경계: [Agent Connection](agent-connection.md)
- 정확한 아티팩트 및 플랫폼 증거: [호스트 릴리스 증거](host-release-evidence.md)
- 런타임 경로 및 저장소 경계: [런타임 경계](runtime-boundaries.md)
- SQLite 형식 수용: [저장소 버전 관리](storage-versioning.md)
- 제품 전체 실패 의미: [실패 모델](failure-model.md)
- 위협 모델 및 비보장: [보안](security.md)
