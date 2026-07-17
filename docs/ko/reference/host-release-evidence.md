# 호스트 릴리스 증거

이 문서는 Codex 런타임 지원 정책과 정확히 최종 확정된 아티팩트의 릴리스 증거를
엄격히 분리하는 계약을 담당합니다. 실행 파일에 포함하는 `CodexSupportCatalog`,
외부 `CodexReleaseEvidenceManifest`, 서로 독립적인 플랫폼 셀, 필수 릴리스 검증
시나리오, 사실에 맞는 셀 실행 상태를 정의합니다.

관리 호스트 설정, receipt 의미, 런타임 신뢰, 운영체제 전제 조건은 정의하지
않습니다. 해당 계약은 각각의 집중 담당 문서에 남습니다. 릴리스 검증 fixture와
결과는 테스트 및 릴리스 증거이며 운영 런타임의 신뢰 입력이 아닙니다.

<a id="surface-stability"></a>
## 표면 안정성

아래 라벨은
[표면 안정성 어휘](../maintain/documentation-policy.md#surface-stability-labels)를
사용합니다. `CodexSupportCatalog`, `CodexReleaseEvidenceManifest`, 정확한
아티팩트 및 capability 대조, `unsupported_host_artifact`, 서로 독립적인 네 플랫폼
셀, 셀 상태 의미는 `stable`입니다. 그 경계 아래의 테스트 runner module과
fixture 배치는 `internal`입니다.

<a id="codex-support-catalog"></a>

## `CodexSupportCatalog`

실행 파일은 아래의 정확한 닫힌 형태와 필드 순서를 가진 런타임 정책 카탈로그 하나를
포함합니다.

```yaml
CodexSupportCatalog:
  contract_id: volicord.codex-support-catalog
  entries: CodexSupportEntry[]

CodexSupportEntry:
  codex_artifact_digest: string
  platform_environment: PlatformEnvironment
  platform_release_coordinate: PlatformReleaseCoordinate
  integration_profile: record
  verified_capabilities: CodexCapability[]
```

`codex_artifact_digest`는 런타임 정책이 허용하는 정확히 최종 확정된 Codex 실행
파일 byte의 raw 64자 소문자 16진수 SHA-256입니다. 첫 릴리스 entry는
[Agent Connection](agent-connection.md#platform-environment)이 담당하는 정확한
플랫폼 및 릴리스 좌표, 정확한 `integration_profile=record`, 정규 순서의 정확한
`FirstReleaseCodexCapabilities`를 사용합니다. Entry는 중복 없이 `linux`,
`macos`, `native_windows`, `wsl2` 순서로 둡니다. 카탈로그는 entry를 0~4개
담을 수 있으며 빈 카탈로그는 모든 Codex 아티팩트를 거절합니다.

이 카탈로그에는 Volicord 실행 파일 digest, 검증 결과, 시나리오 상태, 증거 경로,
workflow run 식별자, 릴리스 셀 timestamp, 최종 Volicord 실행 파일 byte에서
파생되는 다른 값을 넣지 않습니다. 알 수 없는 구성원, 중복 JSON key, 잘못된
digest, 비정규 필드 순서, 담당 문서의 닫힌 집합 밖 값은 카탈로그를 무효화합니다.
런타임 조회는 이 내장 카탈로그만 읽고 digest, 플랫폼, 릴리스 좌표, profile, 전체
capability가 정확히 일치하도록 요구합니다. 릴리스 증거는 읽지 않습니다.

카탈로그 identity는 릴리스 증거와 독립적이며 아래에 선언한 정규 `record` 및
`list` 인코딩을 사용합니다.

```text
support_catalog_identity_digest = lowercase_hex(sha256(
  "volicord.codex-support-catalog\0"
  || record(contract_id, entries)
))
```

각 entry와 중첩 플랫폼 릴리스 좌표는 선언한 필드 순서로 인코딩합니다. 외부
Volicord 아티팩트 digest, 검증 결과, runner, 시나리오 결과, 증거 경로, workflow
좌표, timestamp를 바꾸어도 이 identity는 달라지지 않습니다.

<a id="codex-release-evidence-manifest"></a>

## `CodexReleaseEvidenceManifest`

릴리스 증거는 실행 파일 외부에 남으며 아래의 정확한 닫힌 형태와 필드 순서를
사용합니다.

```yaml
CodexReleaseEvidenceManifest:
  contract_id: volicord.codex-release-evidence-manifest
  entries: CodexReleaseEvidenceEntry[]

CodexReleaseEvidenceEntry:
  codex_artifact_digest: string
  platform_environment: PlatformEnvironment
  observed_capabilities: CodexCapability[]
  integration_profile: record
  validation_evidence: CodexReleaseValidationEvidence
```

모든 구성원은 필수입니다. 바깥 좌표는 릴리스 셀이 실행한 정확한 Codex 아티팩트,
플랫폼 환경, 관찰 capability 집합, profile을 결속합니다. 검증 증거의
`codex_artifact_digest`, `platform_environment`, `observed_capabilities`,
`integration_profile`은 소유 entry와 정확히 같아야 하며 증거가 좌표를 넓히거나
복구할 수 없습니다.

<a id="codex-release-validation-evidence"></a>

## `CodexReleaseValidationEvidence`

중첩 증거와 runner 좌표는 아래의 정확한 닫힌 형태와 필드 순서를 가집니다.

```yaml
CodexReleaseValidationEvidence:
  validation_result: passed | failed | unavailable
  codex_artifact_digest: string
  platform_environment: PlatformEnvironment
  observed_capabilities: CodexCapability[]
  integration_profile: record
  volicord_artifact_digest: string
  runner: CodexReleaseEvidenceRunner
  scenario_results: CodexReleaseScenarioResult[]
  evidence_digest: string
  observed_at: string

CodexReleaseEvidenceRunner:
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

Nullable 구성원을 포함해 모든 구성원은 필수이며 알 수 없는 구성원, 중복 JSON
key, 비정규 필드 순서는 유효하지 않습니다. `codex_artifact_digest`,
`volicord_artifact_digest`, null이 아닌 시나리오 `evidence_digest`, 증거 객체
수준의 `evidence_digest`는 모두 raw 64자 소문자
16진수 SHA-256입니다. Null이 아닌 timestamp는 정규 RFC 3339 UTC입니다. Runner
문자열은 비어 있지 않고 제어 문자가 없는 UTF-8입니다. `runner_id`와
`target_triple`은 최대 256바이트, `os_release`와 `environment_image`는 최대
512바이트입니다. Runner 필드는 target과 정확한 실행 환경을 식별하며 다른 셀에서 복사하거나
추론할 수 없습니다. WSL2 셀의 `environment_image`는 고정한 Ubuntu LTS 배포판
이미지를 이름 붙입니다.

`reason`은 `passed`일 때 null이고, 그 밖에는
`[a-z][a-z0-9_]{0,127}`과 일치하는 비어 있지 않은 기계 판독 코드입니다.
`passed` 또는 `failed` 시나리오는 null이 아닌 digest와 timestamp를 가집니다.
`unavailable` 시나리오는 null이 아닌 reason과 timestamp를 가지며 크기가 제한된
증거 아티팩트를 만들 수 없을 때만 digest가 null일 수 있습니다. `not_run`
시나리오는 null이 아닌 reason과 null digest 및 timestamp를 가집니다.

모든 필수 시나리오가 `passed`일 때만 증거 `validation_result`가 `passed`입니다. 하나
이상의 시나리오가 `failed`이면 `failed`입니다. 실패 시나리오가 없고 하나 이상이
`unavailable`이며 자격을 갖춘 시도가 끝까지 진행되지 못했을 때만
`unavailable`입니다. 진행하지 못한 뒤쪽 시나리오는 명시적 `not_run` 결과로
남습니다. 최상위 `not_run` 증거 객체는 없습니다. 자격을 갖춘 시도가 없었던
플랫폼은 manifest entry 자체가 없습니다.

<a id="codex-release-scenario-catalog"></a>

### 닫힌 시나리오 카탈로그

WSL2가 아닌 모든 증거 entry는 다음 기본 시나리오를 이 순서대로 정확히 한 번씩
담습니다.

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
정확한 `u32be`, `blob`, `string`, `list`, `record` primitive을
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
## 정확한 정책 대조와 최종 확정 아티팩트 증거

릴리스 셀은 signing, stripping, 패키지 추출, 그 밖의 후처리를 포함해 게시자가
제어하는 모든 byte 변경이 끝난 뒤 각 실행 파일을 hash합니다. 검증은
`codex_artifact_digest`가 이름 붙인 정확한 Codex byte와
`volicord_artifact_digest`가 이름 붙인 정확한 Volicord byte를 실행합니다.
시나리오 모음 실행 전과 후에 runner가 두 실행 파일을 다시 열고 같은 byte
digest인지 확인해야 합니다. 명령 이름, 경로, 버전 범위, 패키지 라벨, 빌드
식별자, 별도로 다시 빌드한 실행 파일은 최종 확정 byte를 대신할 수 없습니다.

런타임 지원에는 정확한 `CodexSupportEntry` 대조만 필요합니다. 릴리스 자격에는
같은 Codex 좌표, 실제 실행한 정확한 Volicord digest, 완전한 runner 및 시나리오
metadata, `validation_result=passed`를 가진 외부 증거 entry가 추가로 필요합니다.

지원은 다른 아티팩트, 다른 capability, 다른 플랫폼으로 전파되지 않습니다. 한
셀에서 관찰한 capability를 Codex 전체 capability 주장으로 넓히면 안 됩니다.

현재 `ProcessBinding.executable_digest`, `PlatformEnvironment`,
`PlatformReleaseCoordinate`, `integration_profile=record`, 전체 정규
`CodexCapability` 집합과 정확히 일치하는 지원 카탈로그 entry가 있을 때만
아티팩트를 런타임 지원 대상으로 등록합니다. Receipt의 `executable_digest`,
플랫폼, 릴리스 좌표, 프로필, `required_capabilities`, `verified_capabilities`도 같은
정책 좌표와 같아야 합니다. 알 수 없는 digest나 플랫폼·릴리스 좌표·프로필·
capability 불일치는 machine-readable reason `unsupported_host_artifact`를
반환합니다. 명령 이름,
넓은 버전 범위, 인접 아티팩트, fixture, capability 부분집합이나 상위집합 일치로
지원을 추론하면 안 됩니다.

<a id="canonical-checked-in-contracts"></a>
## 체크인하는 기준 계약

런타임 지원 정책 원본은 다음 파일입니다.

```text
crates/volicord-types/contracts/codex-support-catalog.json
```

실행 파일에는 이 파일만 포함합니다. 이 원본에는 릴리스 결과, Volicord digest,
runner metadata, 증거 위치, workflow 식별자, 릴리스 셀 timestamp를 넣으면 안
됩니다. Fixture, 생성 상수, 문서 표, 런타임 데이터베이스, 릴리스 증거 파일이 두
번째 런타임 지원 목록 역할을 해도 안 됩니다.

릴리스 증거 원본은 모든 Volicord 실행 파일 외부에 둡니다.

```text
tests/release-validation/contracts/codex-release-evidence-manifest.json
```

이 파일은 runner가 실제로 만들고 review한 증거 entry를 0~4개 담습니다. 플랫폼별
entry는 최대 하나이며 `linux`, `macos`, `native_windows`, `wsl2` 순서로 둡니다.
아직 실행하지 않은 원본은 `entries: []`입니다. Entry가 없는 플랫폼의 파생 상태는
`not_run`입니다. 원본은 placeholder entry, digest, runner 좌표, 증거 객체를
꾸며 내면 안 됩니다. 실제 자격을 갖춘 시도만 `failed` 또는 `unavailable` 증거를
만들 수 있습니다.

운영 코드는 `include_bytes!`, 생성 Rust, build script 환경 변수, compile된 상수나
동등한 메커니즘으로 이 외부 manifest를 포함하면 안 됩니다. 릴리스 검증은 기준
디스크 경로에서 manifest를 읽고 엄격히 parse한 뒤 모든 증거 entry를 내장 지원
카탈로그와 교차 대조합니다. 카탈로그에 없는 Codex 아티팩트의 증거는 기록된 결과가
`passed`여도 유효하지 않습니다.

플랫폼 하나를 하나의 review 작업으로 갱신합니다.

1. 해당 플랫폼의 Codex 아티팩트를 최종 확정하고 최종 byte에서
   `codex_artifact_digest`를 계산합니다.
2. 릴리스 결과나 Volicord digest 없이 정확한 런타임 정책 entry를 추가하거나
   교체한 뒤 그 카탈로그를 포함하는 Volicord 아티팩트를 빌드하고 최종 확정합니다.
3. 그 정확한 Codex 및 Volicord byte로 전체 릴리스 검증 셀을 실행합니다. Runner는
   모든 필수 시나리오의 크기가 제한된 외부 증거를 생성합니다.
4. 두 아티팩트 digest와 정확한 플랫폼, profile, capability, runner, target,
   시나리오, 증거 digest 결속을 다시 확인합니다.
5. 생성 증거를 review한 뒤 정규 순서를 보존하면서 해당 플랫폼의 외부 entry를
   교체합니다. 실행하지 않은 셀을 수작업으로 만들거나, 다른 플랫폼 결과를
   복사하거나, 결과 라벨을 바꾸거나, 과거 호환성 entry를 남기면 안 됩니다.
6. 내장 카탈로그를 기준으로 외부 manifest를 다시 평가합니다. 네 플랫폼 릴리스는
   플랫폼마다 현재 통과 증거 entry가 하나씩 있고 모든 entry가 런타임 정책과
   정확히 일치할 때만 자격이 있습니다.

어느 아티팩트든 byte, 플랫폼 좌표, capability 집합, profile, 검증 증거가 바뀌면
해당 정확한 셀을 새로 실행해야 합니다. 어느 계약을 편집하더라도 runner가 만들지
않은 증거를 승격할 수 없습니다.

<a id="explicit-test-only-descriptor"></a>
## 명시적인 테스트 전용 설명자

최종 확정 Codex 아티팩트를 실행하지 않는 단위 테스트와 통합 테스트는
두 운영 계약과 분리된 명시적 설명자를 사용합니다.

```yaml
TestOnlyCodexDescriptor:
  test_only: true
  fixture_id: string
  codex_artifact_digest: string
  platform_environment: linux | macos | native_windows | wsl2
  observed_capabilities: CodexCapability[]
```

Marker는 정확한 boolean `true`여야 합니다. 이 설명자는 테스트 빌드에서 parsing,
routing, 부정 사례, 어댑터 projection을 실행할 때 사용할 수 있습니다. 두 계약
loader와 모든 운영 지원 조회는 이 설명자를 거절합니다. 이 설명자는
`validation_result=passed`를 만들거나 호스트 아티팩트 및 capability를 등록할 수
없습니다. 테스트 fixture, 테스트 전용 주입, 복사한 entry, 저장소 테스트 통과는
런타임 신뢰가 아니며 최종 확정 아티팩트 증거도 아닙니다.

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

WSL2 셀은 런타임 정책과 외부 증거에 함께 고정된 Ubuntu LTS 이미지 하나를
사용합니다. Codex,
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

`validation_evidence.validation_result`는 정확히 다음 값 중 하나입니다.

| 상태 | 의미 | 릴리스 영향 |
|---|---|---|
| `passed` | 정확히 최종 확정된 Codex와 Volicord 아티팩트를 정확한 셀 환경에서 실행했고, 모든 필수 시나리오가 통과했으며, 증거가 완전하고 모든 결속이 정확합니다. | 이 정확한 정책 entry와 Volicord digest의 릴리스 증거만 충족합니다. 런타임 지원은 계속 내장 카탈로그에서 나옵니다. |
| `failed` | 셀이 하나 이상의 필수 assertion을 실패로 분류할 만큼 실행되었거나 아티팩트 또는 증거 무결성 검사가 실패했습니다. | 네 플랫폼 릴리스 주장을 막으며 런타임 정책을 바꾸지 않습니다. |
| `unavailable` | 필수 runner, 호스트, credential, 환경, 그 밖의 실행 전제 조건을 사용할 수 없어 전체 시나리오 모음의 통과나 실패를 확정하지 못했습니다. | 네 플랫폼 릴리스 주장을 막으며 런타임 정책을 바꾸지 않습니다. |

`not_run`은 증거 manifest에 해당 플랫폼 entry가 없을 때의 파생 플랫폼
상태입니다. `validation_evidence.validation_result` 값이 아니며 placeholder
entry를 허용하지 않습니다.
시나리오 결과는 위 교차 필드 규칙에 따라 `not_run`을 사용할 수 있습니다.
`unavailable`과 파생 `not_run`을 `passed`로 보고, 요약, 집계해서는 안 됩니다.
저장소 단위 테스트, fixture 결과, 다른 플랫폼의 통과, 이전 아티팩트의 증거는
이 의미를 바꿀 수 없습니다.

<a id="release-validation-target-layout"></a>
## 릴리스 검증 목표 배치

유지할 목표 구조는 다음과 같습니다.

```text
crates/volicord-types/
  contracts/
    codex-support-catalog.json
  src/
    codex_support_catalog.rs
    codex_release_evidence.rs

tests/release-validation/
  contracts/
    codex-release-evidence-manifest.json
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

공유 타입 crate는 엄격한 런타임 카탈로그 parsing과 embedding, 정확한 지원 조회,
외부 원본을 포함하지 않는 엄격한 외부 증거 parsing을 담당합니다. 릴리스 검증의
`contracts/` 경로는 기준 외부 경로와 교차 계약 검증을 담당합니다.
`fixtures/`에는 명시적 테스트 전용 설명자와 크기가 제한된 테스트 입력만 둡니다.
`scenarios/`는 공유 도메인 설정과 기준 결과를 담당합니다. `hosts/codex/`는 실제
Codex 실행과 관찰 동작을 담당합니다. 각 `platforms/` module은 자체 플랫폼
환경과 어댑터별 assertion만 담당합니다. 단일 플랫폼 module, fixture, 실제
호스트 테스트 파일 하나가 전체 릴리스 계약을 담당하면 안 됩니다.

<a id="executable-release-cell-gate"></a>
## 실행 가능한 릴리스 셀 게이트

저장소 기준 후보 생성기와 차단 게이트는 `volicord-release-validation-tests` 패키지의
`codex-release-cell-gate` binary입니다. 계약 우회 경로는 없습니다. 내장 지원
카탈로그와 디스크의 지원 카탈로그 원본이 같도록 요구하고, 릴리스 증거 manifest는
기준 외부 경로에서만 읽은 뒤 모든 증거 entry를 런타임 정책과 교차 대조합니다.

```sh
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --status
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --capture-candidate PLATFORM
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --platform linux
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --platform macos
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --platform native_windows
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --platform wsl2
```

`--status`는 외부 증거의 실제 상태 또는 파생 상태 네 개를 보고하며 셀을 실행하지
않습니다. `--capture-candidate`는 자격을 갖춘 시도 하나를 실행하고 create-new
방식으로 외부 경로에 엄격하게 parse되는 단일 entry 후보 manifest를 기록합니다.
후보의 Codex 좌표는 내장 지원 카탈로그에 이미 있어야 합니다. 상태가 `failed` 또는
`unavailable`인 후보는 보존한 뒤 실패로 종료하며 두 기준 계약을 편집하거나
승격하지 않습니다. `--platform`은 차단 재실행 게이트이며 해당 플랫폼에 런타임
정책과 일치하는 정확한 체크인 `passed` 증거 entry가 이미 있을 때만 성공합니다.
항목이 없으면 `not_run`으로 실패하고 체크인 상태가 `failed` 또는
`unavailable`이어도 실패합니다. 따라서 현재의 사실에 맞는 `entries: []` 원본은
fail closed로 동작하며 게시 게이트를 통과할 수 없습니다.

후보 생성기와 게이트는 다음 순서로 점검합니다.

1. 선택한 독립 runner 경계가 아닌 process를 거부합니다.
2. 실제 runner 좌표를 파생합니다. 후보 생성 시에는 그 값을 기록하고, 차단 재실행
   시에는 증거 entry의 `runner` 값과 정확히 같은지 요구합니다.
3. 실제 Codex와 Volicord 실행 파일 byte를 hash합니다. 차단 재실행 시에는 외부
   증거에 기록된 두 digest를 요구합니다.
4. 두 정확한 경로에서 각각 `--version`을 실행합니다.
5. 플랫폼 담당 시나리오를 기준 순서대로 정확히 한 번씩 프로비저닝된 scenario
   driver에 위임합니다. 통과한 driver는 아래의 엄격한 의미 증거 문서를 생성해야
   합니다. runner는 기준 fixture 구성, 저장소가 선택한 boundary 실행, 저장소가 정한
   domain 결과, boundary와 일치하는 adapter projection, 제한된 정리를 검증한 뒤 그
   증거를 감쌉니다. 네 기준 payload digest를 각각 다시 계산하고 네 record 사이의
   digest 결속도 확인합니다. 결정론적 wrapper는 시나리오 정의, 두 아티팩트 digest,
   플랫폼, driver digest, capability, Record profile을 결합합니다. 증거가 빠지거나,
   추가되거나, 이름이나 catalog 순서가 다르거나, 불투명하거나, driver가 임의로
   선택했거나, 서로 맞지 않으면 실패합니다.
6. 전체 카탈로그 뒤에 두 실행 파일 경로를 다시 열고 hash하여 byte가 바뀌지
   않았는지 요구합니다.
7. 후보 생성은 기준 셀 증거 digest를 계산하고 새로운 외부 후보 경로에만 기록합니다.
   차단 재실행은 모든 현재 시나리오가 통과하고 각 결정론적 증거 digest가 검토된
   체크인 결과와 같은지 요구합니다.

native Linux 경계는 WSL과 container process 경계를 거부합니다. macOS와 native
Windows 경계는 각각 대응하는 native process를 요구합니다. WSL2 경계는
`wsl_shutdown_restart` 중에도 coordinator가 살아남을 수 있도록 의도적으로 native
Windows supervisor에서 게이트와 scenario coordinator를 실행합니다. 선택한 제품
환경은 정확히 `Ubuntu-24.04`입니다. 게이트는 WSL2 kernel, `ID=ubuntu`,
`VERSION_ID=24.04`, 일치하는 `WSL_DISTRO_NAME`을 확인합니다. Codex, Volicord,
셀 work root, 그 아래에서 만드는 Product Repository, `VOLICORD_HOME`은 모두 그
하나의 배포판 안에서 ext4를 사용합니다. Windows supervisor는 테스트 하네스
인프라이며 native Windows 제품 구성 요소를 대신하지 않습니다.

### 후보 생성기와 게이트 입력

릴리스 runner는 후보 생성 또는 차단 재실행 전에 다음 환경 변수를 정확히
프로비저닝합니다.

| 변수 | 필수 값 |
|---|---|
| `VOLICORD_CODEX_RELEASE_CODEX_PATH` | 정확히 최종 확정된 Codex 실행 파일의 symlink 없는 기준 경로입니다. native 셀에서는 host 경로이고 WSL2에서는 선택한 배포판 안의 절대 Linux ext4 경로입니다. |
| `VOLICORD_CODEX_RELEASE_VOLICORD_PATH` | `volicord_artifact_digest`가 이름 붙인 정확한 Volicord 실행 파일의 symlink 없는 기준 경로입니다. 같은 native 또는 WSL2 경로 규칙을 적용합니다. |
| `VOLICORD_CODEX_RELEASE_SCENARIO_DRIVER` | 플랫폼에 프로비저닝된 scenario driver의 기준 host 경로입니다. WSL2에서는 배포판 종료와 재시작 중에도 살아남아 이를 검증할 수 있는 native Windows coordinator입니다. |
| `VOLICORD_CODEX_RELEASE_EVIDENCE_DIR` | source checkout, Cargo target directory, 유지 문서, Product Repository, Runtime Home 밖에 있는 기존의 비어 있고 기준 경로인 host directory입니다. |
| `VOLICORD_CODEX_RELEASE_WORK_ROOT` | 저장소 담당 경로 밖에 있는 기존의 비어 있는 work root입니다. WSL2에서는 선택한 배포판 안의 절대 ext4 directory입니다. |
| `VOLICORD_HOME` | 셀 work root의 존재하지 않는 하위 경로이며 `runtime_home_creation` 시나리오만 생성합니다. |
| `VOLICORD_CODEX_RELEASE_ENVIRONMENT_IMAGE` | 후보 생성에서는 지원 entry, 차단 재실행에서는 외부 증거 entry에 해당하는 정확한 environment-image 좌표입니다. |
| `RUNNER_NAME` | 실제 runner service 식별자이며 `runner.runner_id`와 같아야 합니다. |
| `VOLICORD_CODEX_RELEASE_WSL2_DISTRIBUTION` | WSL2 전용이며 정확히 `Ubuntu-24.04`입니다. native 셀에서는 거부합니다. |
| `VOLICORD_CODEX_RELEASE_CANDIDATE_CELL_PATH` | 후보 생성 전용입니다. 저장소 담당 경로, 증거, work root, Runtime Home 밖에서 기존 기준 부모를 가진 존재하지 않는 절대 경로입니다. 생성기는 create-new 방식으로 엄격한 단일 entry 증거 manifest를 기록합니다. 차단 재실행은 이 변수를 읽지 않습니다. |

게이트는 native process에서 architecture와 target triple을 파생합니다. WSL2에서는
선택한 배포판 안의 `uname -m`을 사용합니다. `os_release`는 Linux와 WSL2에서
`/proc/sys/kernel/osrelease`, macOS에서 `sw_vers -productVersion`, native
Windows에서 `cmd.exe /D /C ver`로 파생합니다. 이 파생 값과 프로비저닝된
environment-image 좌표는 체크인 runner 좌표와 정확히 같아야 합니다.

각 시나리오에서 driver는 정확한 시나리오, 플랫폼, 실행 파일, work root, Runtime
Home, 새 출력 경로 두 개를 받습니다. 명령 형태는 다음과 같습니다.

```text
SCENARIO_DRIVER
  --scenario SCENARIO_ID
  --fixture FIXTURE
  --boundary BOUNDARY
  --projection PROJECTION
  --expected-outcome OUTCOME_CODE
  --platform PLATFORM
  --codex CODEX_PATH
  --volicord VOLICORD_PATH
  --work-root WORK_ROOT
  --runtime-home RUNTIME_HOME
  --evidence-output NEW_DRIVER_EVIDENCE_PATH
  --outcome-output NEW_OUTCOME_JSON_PATH
  [--wsl2-distribution Ubuntu-24.04]
```

결과 문서는 정확히 `scenario_id`, `status`, `reason`, `observed_at`을 가집니다.
`status`는 `passed`, `failed`, `unavailable`, `not_run` 중 하나입니다. `passed`는
null 사유와 기준 UTC 관찰 시각, `failed`와 `unavailable`은 기계 판독 사유와 시각,
`not_run`은 사유와 null 시각을 요구합니다. 통과와 실패 결과에는 크기가 제한된
driver 증거 파일이 필수이고, `not_run`은 이를 금지하며, `unavailable`은 포함할 수
있습니다.

`passed` 증거 파일은 정확히 다음 형태의 엄격한 JSON입니다.

```json
{
  "contract": "volicord.release_scenario_evidence",
  "scenario_id": "fresh_install",
  "platform": "linux",
  "state_setup": {
    "canonical_project_state": {
      "fixture_id": "fresh_install",
      "fixture": "no_installation",
      "platform": "linux"
    },
    "canonical_project_state_digest": "0412001a986fb601aaec49e5ca491f034735eae9d2b79fc3a1f172ac73268725",
    "validated": true
  },
  "boundary_execution": {
    "canonical_invocation": {
      "scenario_id": "fresh_install",
      "platform": "linux",
      "boundary": "cli",
      "canonical_project_state_digest": "0412001a986fb601aaec49e5ca491f034735eae9d2b79fc3a1f172ac73268725"
    },
    "invocation_digest": "e123ca5a50d1b8362a8bc8a9a6366692f3901a09184e014da44eaa1e3a1d9fde",
    "completed": true
  },
  "domain_outcome": {
    "canonical_outcome": {
      "scenario_id": "fresh_install",
      "expectation": "complete_successfully",
      "disposition": "completed",
      "outcome_code": "installation_completed",
      "invocation_digest": "e123ca5a50d1b8362a8bc8a9a6366692f3901a09184e014da44eaa1e3a1d9fde",
      "observed_paths_preserved": null
    },
    "canonical_outcome_digest": "9690aa98449e2944b9477d3ffb6496a918556a15432173cdc48b4a432cee19af",
    "validated": true
  },
  "adapter_projection": {
    "canonical_projection": {
      "scenario_id": "fresh_install",
      "projection": "cli_json",
      "outcome_code": "installation_completed",
      "canonical_outcome_digest": "9690aa98449e2944b9477d3ffb6496a918556a15432173cdc48b4a432cee19af"
    },
    "canonical_projection_digest": "ca3f422db0290cd0bdd328afd8ca794bca9e7f1eee0a509382e1ddff2dfd0a48",
    "validated": true
  },
  "cleanup_complete": true
}
```

중첩된 네 `canonical_*` 객체는 보존되는 변동 없는 기준 payload입니다. 각 객체와
나란히 있는 digest는 그 객체의 기준 JSON에 대한 접두사 없는 소문자 SHA-256입니다.
호출 record는 다시 계산한 프로젝트 상태 digest를, domain 결과 record는 다시 계산한
호출 digest를, projection record는 다시 계산한 domain 결과 digest를 담습니다.
게이트는 네 digest와 결속을 모두 다시 계산한 뒤 각 payload를 선택한 저장소 정의와
정확히 비교합니다. `validated`, `completed`, `cleanup_complete`는 true여야 하지만,
이 flag가 payload, digest, 결속, 의미 검사를 대신하지 않습니다.

저장소는 다음 시나리오 대조표를 정확히 담당합니다.

| 시나리오 | Fixture | Boundary / projection | Outcome code | Expectation / disposition |
|---|---|---|---|---|
| `fresh_install` | `no_installation` | `cli` / `cli_json` | `installation_completed` | `complete_successfully` / `completed` |
| `runtime_home_creation` | `runtime_home_absent` | `cli` / `cli_json` | `runtime_home_created` | `complete_successfully` / `completed` |
| `personal_managed_binding` | `personal_binding_absent` | `cli` / `cli_json` | `personal_managed_binding_installed` | `complete_successfully` / `completed` |
| `shared_managed_binding` | `shared_binding_absent` | `cli` / `cli_json` | `shared_managed_binding_installed` | `complete_successfully` / `completed` |
| `receipt_create_and_validate` | `current_managed_binding` | `managed_host` / `managed_host_state` | `receipt_current` | `complete_successfully` / `completed` |
| `configuration_drift_detection` | `drifted_managed_configuration` | `managed_host` / `managed_host_state` | `configuration_drift_detected` | `complete_successfully` / `completed` |
| `repair_after_drift` | `repairable_managed_configuration_drift` | `cli` / `cli_json` | `configuration_repaired` | `complete_successfully` / `completed` |
| `safe_uninstall` | `installed_managed_binding` | `cli` / `cli_json` | `managed_binding_removed` | `complete_successfully` / `completed` |
| `symlink_and_canonical_path` | `symlinked_managed_path` | `platform` / `platform_result` | `canonical_path_rules_enforced` | `complete_successfully` / `completed` |
| `codex_restart` | `restarted_codex_process` | `managed_host` / `managed_host_state` | `stale_receipt_rejected` | `complete_successfully` / `completed` |
| `project_move` | `moved_product_repository` | `managed_host` / `managed_host_state` | `moved_project_binding_rejected` | `complete_successfully` / `completed` |
| `record_write_workflow` | `record_workflow_ready` | `mcp_stdio` / `mcp_structured_content` | `record_write_completed` | `complete_successfully` / `completed` |
| `suppression_unavailable` | `suppression_provider_unavailable` | `core` / `core_response` | `observed_paths_preserved` | `preserve_observed_paths_when_suppression_unavailable` / `warning` |
| `unsupported_host` | `unsupported_host_selected` | `cli` / `cli_json` | `unsupported_host_rejected` | `reject_unsupported_host` / `rejected` |
| `unsupported_host_artifact` | `unregistered_host_artifact` | `managed_host` / `managed_host_state` | `unsupported_host_artifact_rejected` | `reject_unsupported_host_artifact` / `rejected` |
| `wsl_shutdown_restart` | `stale_wsl2_process_and_receipt` | `platform` / `platform_result` | `stale_wsl2_process_and_receipt_rejected` | `reject_stale_wsl2_process_and_receipt` / `rejected` |
| `wsl2_ext4_project` | `wsl2_ext4_topology` | `platform` / `platform_result` | `wsl2_ext4_accepted` | `accept_wsl2_ext4` / `completed` |
| `wsl2_drvfs_rejection` | `wsl2_drvfs_topology` | `platform` / `platform_result` | `wsl2_drvfs_rejected` | `reject_wsl2_drvfs` / `rejected` |
| `wsl2_cross_topology_rejection` | `wsl2_cross_topology` | `platform` / `platform_result` | `wsl2_cross_topology_rejected` | `reject_wsl2_cross_topology` / `rejected` |
| `wsl1_rejection` | `wsl1_environment` | `platform` / `platform_result` | `wsl1_rejected` | `reject_wsl1` / `rejected` |
| `wsl2_native_windows_receipt_reuse_rejection` | `native_windows_receipt_in_wsl2` | `managed_host` / `managed_host_state` | `native_windows_receipt_reuse_rejected` | `reject_native_windows_receipt_reuse` / `rejected` |

`suppression_unavailable`만 `observed_paths_preserved: true`를 가지며 다른 모든
시나리오는 null이어야 합니다. 불투명한 성공 flag, 알 수 없는 필드, 접두사가
붙었거나 오래된 digest, 끊어진 digest 결속, driver가 임의로 선택한 fixture,
boundary, projection, expectation, disposition, outcome code는 게이트를 충족할 수
없습니다.

Runner는 `VOLICORD_CODEX_RELEASE_CODEX_ARTIFACT_DIGEST`,
`VOLICORD_CODEX_RELEASE_VOLICORD_DIGEST`,
`VOLICORD_CODEX_RELEASE_SCENARIO_DRIVER_DIGEST`,
`VOLICORD_CODEX_RELEASE_CAPABILITIES`,
`VOLICORD_CODEX_RELEASE_INTEGRATION_PROFILE=record`를 제공합니다. 저장소의
scenario catalog가 정확한 fixture, boundary, projection, expectation, disposition,
outcome code를 담당합니다. driver는 그 선택된 boundary를 통한 실행, adapter별 관찰,
제한된 정리를 담당하며 보존되는 기준 record를 닫힌 증거 schema로 보고합니다.
runner는 엄격한 결과 및 증거 parsing, 기준 digest 재계산, digest 결속 검증, catalog
대조 의미 검증, catalog 완전성, 새롭고 정확한 출력 배치, 결정론적 증거 wrapping,
크기가 제한된 각 driver 파일 보존, timeout, 실행 파일 및 driver 안정성을
담당합니다. 관찰
시각은 후보 결과에 기록하지만
결정론적 시나리오별 envelope에서는 제외하므로, 이후 차단 재실행에서도 새로운 시각을
기록하면서 검토된 digest를 재현할 수 있습니다. driver가 성공으로 종료해도 protocol을
완전하게 충족하는 출력이 없으면 실패합니다. prompt, 대화 기록, 자격 증명, token이
workflow 출력이 되지 않도록 driver stdout과 stderr를 숨깁니다.

<a id="trust-and-owner-boundaries"></a>
## 신뢰 및 담당 경계

내장 지원 카탈로그는 정확한 런타임 정책을 다룹니다. 외부 릴리스 증거 manifest는
릴리스 판단을 뒷받침하며 실행 파일 입력이 되지 않습니다. 두 계약 모두 사용자를
attest하거나, receipt에 서명하거나, Core 권한을 부여하거나, 호스트 격리를
증명하거나, 런타임 credential이 되지 않습니다. 운영 런타임 신뢰는 현재 관리
binding, 현재 Store 상태, 내장 지원 정책, 해당 런타임 담당 문서가 정의한 검증
receipt 계약에서만 나옵니다. 릴리스 fixture와 증거를 운영 신뢰 입력으로 불러오면
안 됩니다.

이웃 담당 문서:

- 첫 릴리스 제품 범위: [범위](scope.md).
- 운영체제 및 WSL2 전제 조건: [시스템 요구사항](system-requirements.md).
- 외부 설명자와 공통 Git 객체 ID 규칙: [외부 계약](external-contracts.md).
- 관리 binding 및 receipt 의미: [Agent Connection](agent-connection.md).
- install, verify, repair, uninstall 동작: [관리 CLI](admin-cli.md).
- 호스트 신뢰 및 비보장: [보안](security.md).
- 미지원 계약 범주 의미: [실패 모델](failure-model.md).
- 유지 검증 명령 및 릴리스 보고: [검증](../maintain/validation.md).
