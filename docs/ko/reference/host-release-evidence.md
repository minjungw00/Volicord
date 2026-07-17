# 호스트 릴리스 증거

이 문서는 정확히 최종 확정된 Codex 아티팩트의 첫 릴리스 지원 증거를 담당합니다.
`CodexReleaseCell`, 저장소에 체크인하는 지원 manifest, 서로 독립적인 플랫폼 셀,
필수 릴리스 검증 시나리오, 정직한 셀 실행 상태를 정의합니다.

관리 호스트 설정, receipt 의미, 런타임 신뢰, 운영체제 전제 조건은 정의하지
않습니다. 해당 계약은 각각의 집중 담당 문서에 남습니다. 릴리스 검증 fixture와
결과는 테스트 및 릴리스 증거이며 운영 런타임의 신뢰 입력이 아닙니다.

<a id="surface-stability"></a>
## 표면 안정성

아래 라벨은
[표면 안정성 어휘](../maintain/documentation-policy.md#surface-stability-labels)를
사용합니다. `CodexReleaseCell` 형태, 정확한 아티팩트 및 capability 대조,
`unsupported_host_artifact`, 서로 독립적인 네 플랫폼 셀, 셀 상태 의미는
`stable`입니다. 그 경계 아래의 테스트 runner module과 fixture 배치는
`internal`입니다.

## `CodexReleaseCell`

첫 릴리스는 아래의 정확한 닫힌 형태를 가진 엄격한 셀을 기록합니다.

```yaml
CodexReleaseCell:
  artifact_digest: string
  platform: PlatformEnvironment
  observed_capabilities: CodexCapability[]
  integration_profile: record
  validation_evidence: CodexReleaseValidationEvidence
```

`artifact_digest`는 이 셀이 실행한 정확히 최종 확정된 Codex 실행 파일 byte의
raw 64자 소문자 16진수 SHA-256입니다. `platform`은 닫힌
`PlatformEnvironment` 집합, `observed_capabilities`는 닫힌
`CodexCapability` 집합을 사용하며 두 집합은
[Agent Connection](agent-connection.md#platform-environment)이 담당합니다.
첫 릴리스 셀은 필수 정규 순서의 정확한 `FirstReleaseCodexCapabilities`를
담습니다. `integration_profile`은 정확히 `record`입니다.

모든 구성원은 필수입니다. 알 수 없는 구성원, 중복 JSON key, 잘못된 digest,
담당 문서의 닫힌 집합 밖 값은 셀을 무효화합니다. 검증 증거의
`artifact_digest`, `platform`, `observed_capabilities`,
`integration_profile`은 소유 셀의 좌표와 정확히 같아야 하며 증거가 좌표를
넓히거나 복구할 수 없습니다.

<a id="codex-release-validation-evidence"></a>

## `CodexReleaseValidationEvidence`

중첩 증거와 runner 좌표는 아래의 정확한 닫힌 형태와 필드 순서를 가집니다.

```yaml
CodexReleaseValidationEvidence:
  status: passed | failed | unavailable
  artifact_digest: string
  platform: PlatformEnvironment
  observed_capabilities: CodexCapability[]
  integration_profile: record
  volicord_artifact_digest: string
  runner: CodexReleaseRunnerCoordinate
  scenario_results: CodexReleaseScenarioResult[]
  evidence_digest: string
  observed_at: string

CodexReleaseRunnerCoordinate:
  runner_id: string
  target_triple: string
  architecture: x86_64 | aarch64
  os_release: string
  environment_image: string

CodexReleaseScenarioResult:
  scenario_id: CodexReleaseScenarioId
  status: passed | failed | unavailable | not_run
  reason: string | null
  evidence_digest: string | null
  observed_at: string | null
```

Nullable 구성원을 포함해 모든 구성원은 필수이며 알 수 없는 구성원과 중복 JSON
key는 유효하지 않습니다. `volicord_artifact_digest`, null이 아닌 시나리오
`evidence_digest`, 증거 객체 수준의 `evidence_digest`는 모두 raw 64자 소문자
16진수 SHA-256입니다. Null이 아닌 timestamp는 정규 RFC 3339 UTC입니다. Runner
문자열은 비어 있지 않고 제어 문자가 없는 UTF-8입니다. `runner_id`와
`target_triple`은 최대 256바이트, `os_release`와 `environment_image`는 최대
512바이트입니다. Runner 필드는 정확한 실행 환경을 식별하며 다른 셀에서 복사하거나
추론할 수 없습니다. WSL2 셀의 `environment_image`는 고정한 Ubuntu LTS 배포판
이미지를 이름 붙입니다.

`reason`은 `passed`일 때 null이고, 그 밖에는
`[a-z][a-z0-9_]{0,127}`과 일치하는 비어 있지 않은 기계 판독 코드입니다.
`passed` 또는 `failed` 시나리오는 null이 아닌 digest와 timestamp를 가집니다.
`unavailable` 시나리오는 null이 아닌 reason과 timestamp를 가지며 크기가 제한된
증거 아티팩트를 만들 수 없을 때만 digest가 null일 수 있습니다. `not_run`
시나리오는 null이 아닌 reason과 null digest 및 timestamp를 가집니다.

모든 필수 시나리오가 `passed`일 때만 증거 `status`가 `passed`입니다. 하나
이상의 시나리오가 `failed`이면 `failed`입니다. 실패 시나리오가 없고 하나 이상이
`unavailable`이며 자격을 갖춘 시도가 끝까지 진행되지 못했을 때만
`unavailable`입니다. 진행하지 못한 뒤쪽 시나리오는 명시적 `not_run` 결과로
남습니다. 최상위 `not_run` 증거 객체는 없습니다. 자격을 갖춘 시도가 없었던
플랫폼은 manifest 셀 자체가 없습니다.

<a id="codex-release-scenario-catalog"></a>

### 닫힌 시나리오 카탈로그

WSL2가 아닌 모든 셀은 다음 기본 시나리오를 이 순서대로 정확히 한 번씩 담습니다.

```text
fresh_install
runtime_home_creation
personal_managed_binding
shared_managed_binding
receipt_create_and_validate
configuration_drift_detection
repair_after_drift
safe_uninstall
symlink_and_canonical_path
codex_restart
project_move
record_write_workflow
suppression_unavailable
unsupported_host
unsupported_host_artifact
```

WSL2 셀은 다음 시나리오를 이 순서대로 정확히 한 번씩 뒤에 붙입니다.

```text
wsl_shutdown_restart
wsl2_ext4_project
wsl2_drvfs_rejection
wsl2_cross_topology_rejection
wsl1_rejection
wsl2_native_windows_receipt_reuse_rejection
```

알 수 없거나, 중복되거나, 누락되거나, 순서가 다른 시나리오 ID는 증거를
무효화합니다. Personal 및 shared 시나리오는 첫 릴리스의 두 연결 의도를 각각
실행합니다.

<a id="release-evidence-digest"></a>

### 증거 Digest

증거는 [Agent Connection](agent-connection.md#canonical-binding-encoding)의
정확한 `u32be`, `u64be`, `blob`, `string`, `list`, `record` primitive을
사용합니다. Nullable 값은 다음 primitive을 추가합니다.

```text
nullable(null)  = 0x00
nullable(value) = 0x01 || blob(value_encoding)
```

`canonical_evidence_without_digest_bytes`는 선언한 순서의
`CodexReleaseValidationEvidence`에서 `evidence_digest` 필드를 제외한
`record` 인코딩입니다. 중첩 record는 선언 순서를 사용하고
`scenario_results`는 카탈로그 순서를 보존합니다.

```text
evidence_digest = lowercase_hex(sha256(
  "volicord.codex-release-validation-evidence\0"
  || canonical_evidence_without_digest_bytes
))
```

Runner는 모든 시나리오 결과를 만든 뒤 digest를 계산하고 review는 다시 계산합니다.
JSON serializer 순서, 생략한 null, 기본값, 수작업으로 편집한 증거는 유효하지
않습니다.

<a id="exact-finalized-artifact-evidence"></a>
## 정확히 최종 확정된 아티팩트 증거

셀은 signing, stripping, 패키지 추출, 그 밖의 후처리를 포함해 게시자가 제어하는
모든 byte 변경이 끝난 뒤 실행 파일을 hash합니다. 검증은
`artifact_digest`가 이름 붙인 정확한 byte를 실행합니다. 시나리오 모음 실행 전과
후에 runner가 실행 파일을 다시 열고 같은 byte digest인지 확인해야 합니다. 명령
이름, 경로, 버전 범위, 패키지 라벨, 빌드 식별자, 별도로 다시 빌드한 실행 파일은
최종 확정 byte를 대신할 수 없습니다.

지원 주장은 아래 조합이 정확히 일치할 때만 유효합니다.

- `artifact_digest`
- `platform`
- `observed_capabilities=FirstReleaseCodexCapabilities`
- `integration_profile=record`
- `validation_evidence.status=passed`

지원은 다른 아티팩트, 다른 capability, 다른 플랫폼으로 전파되지 않습니다. 한
셀에서 관찰한 capability를 Codex 전체 capability 주장으로 넓히면 안 됩니다.

현재 `ProcessBinding.executable_digest`, `PlatformEnvironment`,
`integration_profile=record`, 전체 정규 `CodexCapability` 집합과 정확히 일치하는
`passed` 셀 하나가 있을 때만 아티팩트를 지원 대상으로 등록합니다. Receipt의
`executable_digest`, 플랫폼, 프로필, `required_capabilities`,
`verified_capabilities`도 같은 셀 좌표와 같아야 합니다. 알 수 없는 digest,
플랫폼·프로필·capability 불일치, 통과하지 않은 셀에만 있는 digest는
machine-readable reason `unsupported_host_artifact`를 반환합니다. 명령 이름,
넓은 버전 범위, 인접 아티팩트, fixture, capability 부분집합이나 상위집합 일치로
지원을 추론하면 안 됩니다.

<a id="canonical-checked-in-manifest"></a>
## 체크인하는 단일 기준 manifest

지원 정보의 단일 원본은 다음 파일입니다.

```text
tests/release-validation/contracts/codex-release-manifest.json
```

이 파일은 runner가 실제로 만들고 review한 `CodexReleaseCell` 객체를 0~4개 담는
엄격한 UTF-8 JSON 배열입니다. 플랫폼별 셀은 최대 하나이며 존재하는 셀은
`linux`, `macos`, `native_windows`, `wsl2` 순서로 둡니다. 새로 도입했거나
아직 실행하지 않은 원본은 `[]`일 수 있습니다. 셀이 없는 플랫폼의 파생 릴리스
상태는 `not_run`입니다. 원본은 placeholder 셀, digest, runner 좌표, 증거 객체를
꾸며 내면 안 됩니다.

실제 자격을 갖춘 시도만 `failed` 또는 `unavailable` 셀을 만들 수 있으며,
생성된 증거를 review가 수락한 뒤에만 원본에 넣습니다. 운영 지원 조회에는
`passed` 셀만 참여합니다. 다른 소스 파일, fixture, 생성 상수, 문서 표, 런타임
데이터베이스에 두 번째 지원 목록을 두면 안 됩니다. 런타임 빌드 projection은 이
원본과 재현 가능하게 대조해야 하며 아티팩트, 플랫폼, 상태, capability를 추가하면
안 됩니다.

플랫폼 entry 하나를 하나의 review 작업으로 갱신합니다.

1. 해당 플랫폼의 배포 Codex 아티팩트를 최종 확정하고 최종 byte에서
   `artifact_digest`를 계산합니다.
2. 그 정확한 byte에 대해 해당 플랫폼의 전체 릴리스 검증 셀을 실행합니다. Runner는
   모든 필수 시나리오의 셀 및 크기가 제한된 증거를 생성합니다.
3. 아티팩트 digest와 정확한 플랫폼, 프로필, capability, runner, 시나리오,
   증거 digest 결속을 다시 확인합니다.
4. 생성 셀을 review한 뒤 canonical 플랫폼 순서를 보존하면서 해당 플랫폼의 기존
   entry가 있으면 교체합니다. 실행하지 않은 셀을 수작업으로 만들거나 다른 플랫폼의
   결과를 복사하거나 과거 호환성 entry를 남기면 안 됩니다.
5. Manifest를 다시 평가합니다. 네 플랫폼 릴리스는 각 플랫폼에 현재 `passed` 셀이
   정확히 하나씩 있고, 네 셀이 모두 `FirstReleaseCodexCapabilities`를 담으며,
   현재 릴리스 후보 아티팩트를 가리킬 때만 자격이 있습니다.

아티팩트 byte, 플랫폼 좌표, capability 집합, 프로필, 검증 증거가 바뀌면 해당
정확한 셀을 새로 실행해야 합니다. Manifest 편집으로 runner가 만들지 않은 증거를
승격할 수 없습니다.

<a id="explicit-test-only-descriptor"></a>
## 명시적인 테스트 전용 설명자

최종 확정 Codex 아티팩트를 실행하지 않는 단위 테스트와 통합 테스트는
`CodexReleaseCell`과 분리된 명시적 설명자를 사용합니다.

```yaml
TestOnlyCodexDescriptor:
  test_only: true
  fixture_id: string
  artifact_digest: string
  platform: linux | macos | native_windows | wsl2
  observed_capabilities: CodexCapability[]
```

Marker는 정확한 boolean `true`여야 합니다. 이 설명자는 테스트 빌드에서 parsing,
routing, 부정 사례, 어댑터 projection을 실행할 때 사용할 수 있습니다. 체크인된
manifest loader와 모든 운영 지원 조회는 이 설명자를 거절합니다. 이 설명자는
`validation_evidence.status=passed`를 만들거나 호스트 아티팩트 및 capability를
등록할 수 없습니다. 테스트 fixture, 테스트 전용 주입, 복사한 manifest entry,
저장소 테스트 통과는 런타임 신뢰가 아니며 최종 확정 아티팩트 증거도 아닙니다.

<a id="independent-platform-cells"></a>
## 독립 플랫폼 셀

릴리스 자격이 있는 matrix는 서로 독립적인 네 통과 셀을 포함합니다.

| 플랫폼 | 필수 환경 경계 |
|---|---|
| `linux` | native Linux runner와 Linux 아티팩트를 사용합니다. 이 결과는 WSL2에 대해 아무것도 증명하지 않습니다. |
| `macos` | native macOS runner와 macOS 아티팩트를 사용합니다. Linux나 Unix 계열 동작으로 대신할 수 없습니다. |
| `native_windows` | native Windows runner와 native Windows 아티팩트를 사용합니다. WSL 경로, 프로세스, binding, receipt는 사용할 수 없습니다. |
| `wsl2` | 아래에 정의된 고정 Ubuntu LTS WSL2 환경, WSL2 아티팩트, 그 환경 안에 있는 모든 구성 요소를 사용합니다. |

각 셀은 자체 아티팩트, 환경, capability, 증거 좌표를 실행하고 기록하고
보고합니다. `linux` 통과로 `wsl2`가 통과하지 않으며, `native_windows` 통과로
`wsl2`가 통과하지 않습니다. 한 아티팩트의 통과는 다른 셀에서 사용한 아티팩트를
지원하지 않습니다. 셀이 없거나 통과하지 못하면 다른 셀에서 추론하지 않고 네
플랫폼 릴리스 주장을 막습니다.

<a id="wsl2-cell-boundary"></a>
### WSL2 셀 경계

WSL2 셀은 manifest 증거에 고정된 Ubuntu LTS 이미지 하나를 사용합니다. Codex,
Volicord, `Product Repository`, `Volicord Runtime Home`은 모두 같은 WSL2 배포판
안에서 실행합니다. `Product Repository`와 `Volicord Runtime Home`은 배포판의
Linux ext4 파일 시스템을 사용합니다.

WSL2 셀은 다음 구성을 거절합니다.

- WSL1
- Windows에서 실행하는 Codex와 WSL2의 Volicord 또는 프로젝트 상태 조합
- WSL2에서 실행하는 Codex와 native Windows의 Volicord 프로세스, 프로젝트,
  `Volicord Runtime Home` 조합
- `/mnt/*` 또는 다른 DrvFS mount 아래의 `Product Repository`나
  `Volicord Runtime Home`
- native Windows binding, 프로세스 identity, 검증 receipt 재사용
- 현재 셀 증거가 이름 붙이지 않은 배포판 또는 Ubuntu LTS 이미지

Windows 경로, PID, 환경 값, receipt를 WSL2 값으로 변환하거나 동등하게 취급하지
않습니다. WSL 종료 또는 재시작은 담당 계약이 실제 프로세스를 요구하는
프로세스 결속 증거를 무효화하고, 만료되거나 일치하지 않는 receipt를 stale로
만듭니다. 셀은 새 receipt를 기록하기 전에 해당 거절을 관찰해야 합니다.

<a id="required-release-validation-scenarios"></a>
## 필수 릴리스 검증 시나리오

모든 플랫폼 셀은 자체 플랫폼 및 Codex 어댑터 경계를 통해 같은 도메인 시나리오
집합을 실행합니다.

- 새 설치
- `Volicord Runtime Home` 생성 및 검증
- personal 관리 Codex binding 설치
- shared 관리 Codex binding 설치
- 검증 receipt 생성 및 현재 유효성 검증
- 설정 drift 탐지
- 지원되는 drift의 repair
- 안전한 uninstall
- symlink 및 canonical path 처리
- Codex 재시작과 stale receipt 거절
- `Product Repository` 이동과 stale binding 또는 receipt 거절
- 완전한 Record 프로필 쓰기 작업 흐름 하나
- 관찰 경로를 숨기지 않는 보수적인 `suppression unavailable` 동작
- 미지원 호스트 거절
- 등록되지 않았거나 플랫폼이 일치하지 않는 아티팩트에 대한
  `unsupported_host_artifact`

WSL2 셀은 WSL 종료와 재시작, ext4 `Product Repository`, `/mnt/*` Product
Repository 거절, native Windows와 WSL2의 교차 구성 거절, WSL1 거절, native
Windows receipt 재사용 거절도 실행합니다.

공유 시나리오는 하나의 기준 도메인 설정을 만들고 하나의 기준 도메인 결과를
검증합니다. 플랫폼 module은 플랫폼별 설정, 파일 시스템, 프로세스, projection
assertion만 제공합니다. 플랫폼별 단축 경로가 공유 시나리오를 제거하거나 약하게
만들면 안 됩니다.

<a id="cell-execution-status"></a>
## 셀 실행 상태

`validation_evidence.status`는 정확히 다음 값 중 하나입니다.

| 상태 | 의미 | 릴리스 영향 |
|---|---|---|
| `passed` | 정확히 최종 확정된 아티팩트를 정확한 셀 환경에서 실행했고, 모든 필수 시나리오가 통과했으며, 증거가 완전하고 모든 결속이 정확합니다. | 이 아티팩트, 플랫폼, 프로필, 관찰된 capability 집합만 지원 대상으로 등록합니다. |
| `failed` | 셀이 하나 이상의 필수 assertion을 실패로 분류할 만큼 실행되었거나 아티팩트 또는 증거 무결성 검사가 실패했습니다. | 지원 대상으로 등록하지 않으며 네 플랫폼 릴리스 주장을 막습니다. |
| `unavailable` | 필수 runner, 호스트, credential, 환경, 그 밖의 실행 전제 조건을 사용할 수 없어 전체 시나리오 모음의 통과나 실패를 확정하지 못했습니다. | 지원 대상으로 등록하지 않으며 네 플랫폼 릴리스 주장을 막습니다. |

`not_run`은 manifest에 해당 플랫폼 셀이 없을 때의 파생 플랫폼 상태입니다.
`validation_evidence.status` 값이 아니며 placeholder 셀을 허용하지 않습니다.
시나리오 결과는 위 교차 필드 규칙에 따라 `not_run`을 사용할 수 있습니다.
`unavailable`과 파생 `not_run`을 `passed`로 보고, 요약, 집계해서는 안 됩니다.
저장소 단위 테스트, fixture 결과, 다른 플랫폼의 통과, 이전 아티팩트의 증거는
이 의미를 바꿀 수 없습니다.

<a id="release-validation-target-layout"></a>
## 릴리스 검증 목표 배치

유지할 목표 구조는 다음과 같습니다.

```text
tests/release-validation/
  contracts/
    codex-release-manifest.json
  fixtures/
  scenarios/
  hosts/
    codex/
  platforms/
    linux/
    macos/
    windows/
    wsl2/
```

`contracts/`는 엄격한 manifest parsing과 정확한 지원 조회를 담당합니다.
`fixtures/`에는 명시적 테스트 전용 설명자와 크기가 제한된 테스트 입력만 둡니다.
`scenarios/`는 공유 도메인 설정과 기준 결과를 담당합니다. `hosts/codex/`는 실제
Codex 실행과 관찰 동작을 담당합니다. 각 `platforms/` module은 자체 플랫폼
환경과 어댑터별 assertion만 담당합니다. 단일 플랫폼 module, fixture, 실제
호스트 테스트 파일 하나가 전체 릴리스 계약을 담당하면 안 됩니다.

<a id="trust-and-owner-boundaries"></a>
## 신뢰 및 담당 경계

체크인된 manifest와 검증 증거는 릴리스 판단을 뒷받침하지만 사용자를 attest하거나,
receipt에 서명하거나, Core 권한을 부여하거나, 호스트 격리를 증명하거나, 런타임
credential이 되지 않습니다. 운영 런타임 신뢰는 현재 관리 binding, 현재 Store
상태, 해당 런타임 담당 문서가 정의한 검증 receipt 계약에서만 나옵니다. 릴리스
fixture를 운영 신뢰 입력으로 불러오면 안 됩니다.

이웃 담당 문서:

- 첫 릴리스 제품 범위: [범위](scope.md).
- 운영체제 및 WSL2 전제 조건: [시스템 요구사항](system-requirements.md).
- 외부 설명자와 공통 Git 객체 ID 규칙: [외부 계약](external-contracts.md).
- 관리 binding 및 receipt 의미: [Agent Connection](agent-connection.md).
- install, verify, repair, uninstall 동작: [관리 CLI](admin-cli.md).
- 호스트 신뢰 및 비보장: [보안](security.md).
- 미지원 계약 범주 의미: [실패 모델](failure-model.md).
- 유지 검증 명령 및 릴리스 보고: [검증](../maintain/validation.md).
