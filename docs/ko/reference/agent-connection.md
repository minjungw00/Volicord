# Agent Connection 참조

이 문서는 최초 릴리스의 Agent Connection 계약을 정의합니다. 정확한
`host_kind=codex` record 연결 표면, 정규 연결 검증 보고서, 정규 관리형 호스트 결속,
호스트 검증 영수증, Codex 어댑터와 Core의 경계를 담당합니다.

<a id="owns-and-does-not-own"></a>

## 담당 범위

이 문서가 담당합니다.

- 허용하는 정확한 `host_kind`, integration profile, 연결 의도, 전송, 사용자 행동
  전달 경로, 플랫폼 환경 값
- 정규 `ConnectionVerificationReport`, 닫힌 상태 값, 결정적 집계, 엄격한 인코딩,
  보고서 부재 projection
- Connection과 프로젝트 통합 revision, 권위 있는 managed-host 운영 session evidence
- `ManagedHostBinding` 필드, 정규 인코딩, digest 의미
- `HostVerificationReceipt` 필드와 Core가 영수증을 소비하기 전에 수행하는 검증
- Codex 어댑터의 탐색, 설치, 검증, repair, 제거 책임
- Agent Connection 경계에서 정확한 Codex 아티팩트의 지원 가능 여부

이 문서는 아래 항목을 담당하지 않습니다.

- stdio 프레이밍, MCP 초기화, 도구 처리 경로, 종료:
  [MCP 전송](mcp-transport.md)
- 관리 명령 문법, 출력, 종료 코드:
  [관리 CLI](admin-cli.md)
- 정확한 데이터베이스 테이블이나 저장 효과:
  [저장소 기록](storage-records.md), [저장 효과](storage-effects.md)
- 릴리스 셀 실행과 아티팩트 증거:
  [호스트 릴리스 증거](host-release-evidence.md)
- 운영체제 배치와 파일시스템 전제 조건:
  [시스템 요구사항](system-requirements.md)
- Core `UserActionRequest`와 `UserActionResolution` 스키마:
  [API User Action 스키마](api/schema-user-action.md)
- 제품 전체 실패 범주와 보안 의미:
  [실패 모델](failure-model.md), [보안](security.md)

<a id="surface-stability"></a>

## 표면 안정성

라벨은 [문서 정책](../maintain/documentation-policy.md#surface-stability-labels)의
기준 어휘를 따릅니다.

| 표면 | 안정성 | 계약 |
|---|---|---|
| 최초 릴리스 값 집합, `ConnectionVerificationReport`, 통합 revision, 권위 있는 운영 session evidence, `PlatformEnvironment`, `ManagedHostBinding` 필드와 digest, `HostVerificationReceipt` 필드 | `stable` | 정확한 경계 계약입니다. |
| Codex 탐색, 관리 설치, 검증, repair, 제거, 설정 불일치 결과의 의미 | `stable` | 관찰 가능한 계약을 유지하면서 구현을 바꿀 수 있습니다. |
| 어댑터 모듈, 파일시스템 helper, encoder, Store query helper | `internal` | 안정된 경계를 보존해야 하지만 공개 표면은 아닙니다. |
| 사람이 읽는 검증, 저하 상태, repair 안내 | `diagnostic` | Machine-readable 범주, 사유, typed 필드가 권위 있는 값입니다. |

<a id="first-release-surface"></a>

## 최초 릴리스 표면

최초 릴리스는 아래 Agent Connection 표면만 허용합니다.

| 차원 | 정확한 값 |
|---|---|
| 호스트 | `host_kind=codex` |
| Integration profile | `integration_profile=record` |
| 연결 의도 | `personal` 또는 `shared` |
| 전송 | `volicord mcp --stdio`로 시작하는 Volicord 관리형 stdio MCP |
| 사용자 소유 행동 전달 | CLI inbox |
| 플랫폼 환경 | `linux`, `macos`, `native_windows`, `wsl2` |

정규 결속의 `connection_scope` 필드는 선택한 연결 의도를 담으므로 `personal` 또는
`shared`만 허용합니다. `personal` 연결은 사용자가 소유하는 Codex 로컬 설정을
설치합니다. `shared` 연결은 선택한 `Product Repository` 안에 지원되는 프로젝트 소유
Codex 설정을 설치합니다. 두 의도 모두 선택한 프로젝트, 연결, Runtime Home, 플랫폼
환경, 정확한 관리 설정에 계속 결속됩니다.

Agent Connection은 `Volicord Runtime Home`에 저장하는 로컬 통합 기록입니다. 연결
하나와 허용된 프로젝트를 식별하지만 운영체제 권한을 부여하거나, 사용자 신원을
성립시키거나, Codex가 관리 항목을 불러왔음을 증명하지 않습니다. 관리형 stdio MCP
프로세스 하나는 현재 Agent Connection 하나에 결속됩니다.

사용자 소유 행동은 CLI inbox로 전달합니다. 에이전트 대상 MCP 연결은 담당 문서가
정의한 행동을 요청할 수 있지만, 로컬 사용자 채널로 동작하거나 사용자를 대신해 그
행동을 해결할 수 없습니다.

<a id="connection-verification-report"></a>

## `ConnectionVerificationReport`

작은 보고서 하나가 정규 직렬화 연결 검증 상태입니다. 정확한 닫힌 형태는 다음과
같습니다.

```yaml
ConnectionVerificationReport:
  status: complete | action_required | failed
  checked_at: UtcTimestamp
  checks: ConnectionCheck[]
  actions: ConnectionAction[]

ConnectionCheck:
  id: ConnectionCheckId
  status: passed | pending | failed
  code: string | null
  summary: string
  details: object | null
  observed_at: UtcTimestamp | null

ConnectionAction:
  id: string
  instruction: string
  command: string | null
```

Nullable 구성원과 배열을 포함해 표시한 모든 구성원은 필수입니다. 알 수 없는 구성원,
중복 JSON key, 중복 check ID, 중복 action ID, 비정규 순서, 알 수 없는 상태 값은
유효하지 않습니다. Check ID, action ID, null이 아닌 check code는 ASCII 1~128
byte이고 `[a-z][a-z0-9_]*`와 일치해야 합니다. `summary`, `instruction`, null이
아닌 `command`는 UTF-8 1~4,096 byte이고 NUL을 포함하지 않습니다. Null이 아닌
`details`는 직렬화 형태가 최대 16 KiB인 JSON 객체입니다. 보고서는 check를 최대
64개, action을 최대 32개 포함하며 직렬화 형태는 최대 64 KiB입니다.

Check는 `id`의 UTF-8 byte 오름차순으로 정렬합니다. Action도 `id`를 기준으로 같은
순서를 사용합니다. Producer는 이 순서를 결정적으로 구성합니다. 엄격한 decoding은
다른 순서의 저장 또는 외부 보고서를 조용히 정규화하지 않고 거부합니다.

보고서의 모든 check는 그 보고서에 필수입니다. 최상위 상태는 check에서 파생되며 서로
불일치할 수 없습니다.

1. `failed` check가 하나라도 있으면 `status=failed`입니다.
2. 그렇지 않고 `pending` check가 하나라도 있으면 `status=action_required`입니다.
3. 그렇지 않으면 `status=complete`입니다.

`dry_run`은 작업 모드이며 연결 상태나 check 상태가 아닙니다. 설정 일치, 실행 파일
가용성, protocol 및 host version, capability 관찰, 관찰 timestamp는 check
사실입니다. 이 사실은 `code`, `details`, `observed_at`에 두며 별도의 공개 또는 영속
상태 enum을 만들지 않습니다.

연결 검증에서 나온 사용자 지시는 이 보고서 안의 `actions`에만 둡니다. Registry
저장소는 독립된 검증 상태나 action 배열을 저장하지 않습니다. 완료된 영속 보고서가
없는 연결은 `verification_not_run` pending check 하나와 명확한 검증 action 하나를
포함하는 합성 `status=action_required` 보고서로 projection합니다. 이 projection의
`checked_at`은 projection 시각이고 check의 `observed_at`은 null입니다. 읽었다는
이유만으로 합성 보고서를 쓰지는 않습니다.

운영 호환성은 어댑터가 실제로 수행한 check와 관찰한 host 동작에서 보고합니다.
`complete`는 정확한 host artifact의 release certification, 운영체제 집행, 행위자
identity 증명, correctness 증명, 조작 방지 기록을 뜻하지 않습니다.

## 통합 Revision과 운영 Evidence

현재 Connection 통합 revision은 타입이 지정되고 domain-separated된 canonical SHA-256
digest입니다. Basis는 Agent Connection identity, host kind, intent, scope, mode, server
name, configuration target, 현재의 정확한 managed-configuration fingerprint입니다. 이
fingerprint는 관리 server command와 entry를 포함합니다. Revision 구성은 관찰한 host
version, executable digest, support-catalog 좌표, release evidence, certified capability
set을 읽지 않습니다.

Managed-host runtime session은 성공한 initialization, initialized notification, 현재
필수 tool set을 포함한 실제 `tools/list` 응답, 지정된 안전/읽기 전용 Volicord 호출의
성공을 영속 기록한 뒤에만 해당 현재 revision의 운영 evidence를 충족합니다. 조회는
`session_source=managed_host`만 받으므로 CLI self-test나 preflight session은 충족할 수
없습니다. Terminal protocol failure가 있는 row는 성공 evidence로 선택하지 않습니다.

프로젝트 통합 revision은 Connection revision에 현재 프로젝트 workflow-policy
fingerprint와 현재 Guard installation identity/policy hash 또는 Guard ownership의 명시적
부재를 더합니다. 프로젝트 Agent Session은 이 revision을 보관하며 다른 Connection이나
프로젝트에 다시 결속할 수 없습니다.

이 기록은 현재 구성에서 관찰한 협력적 protocol 동작을 보여 줍니다. MCP client
name/version과 관찰한 host executable version은 diagnostic 관찰이며 제한 안의 임의의
미래 값을 받고 identity 증명이나 allowlist 입력으로 사용하지 않습니다. 나중 검증은 현재
관찰한 host version이 성공 session의 version과 다르면 새 관찰을 요청할 수 있지만 version
비교로 host binary를 인증하거나 금지하지 않습니다.

<a id="external-contract-linkage"></a>

## 외부 계약 연결

정규 호스트 결속 payload는 Volicord가 소유하는 외부 형식입니다. 단일 기준
Agent Connection 모델에 도달하기 전에
[외부 계약](external-contracts.md)이 담당하는 정확한 설명자로 경계 어댑터를
선택합니다.

```yaml
ExternalContractDescriptor:
  contract_id: string
  schema_digest: string
  capabilities: string[]
```

어댑터 registry key는 정확한 `contract_id + schema_digest` 쌍입니다. 설명자의
capability 집합은 Agent Connection 수신 경계가 요구하는 모든 capability를 포함해야
합니다. 설명자 필드 누락, 알 수 없는 쌍, capability 누락, decoding 실패를 다른 형식
탐색이나 기본값 채우기로 복구하지 않습니다.

현재 1.0 이전 릴리스는 현재 설명자만 허용합니다. 선택된 설명자는 Core나 Store를
호출하기 전에 하나의 정규 `ManagedHostBinding`으로 decoding됩니다. Core와 Store는
설명자 세대, 호스트 설정 문법, payload 특징에 따라 분기하지 않습니다.

<a id="platform-environment"></a>

## `PlatformEnvironment`

`PlatformEnvironment`는 아래의 닫힌 값 집합입니다.

| 값 | 의미 |
|---|---|
| `linux` | 네이티브 Linux 릴리스 셀입니다. |
| `macos` | 네이티브 macOS 릴리스 셀입니다. |
| `native_windows` | 네이티브 Windows 릴리스 셀입니다. |
| `wsl2` | 독립적인 WSL2 릴리스 셀입니다. `linux`에서 추론하지 않습니다. |

결속과 영수증은 정확히 같은 값을 담아야 합니다. 검증은 한 플랫폼의 결과를 다른
플랫폼에 대입하지 않습니다. 특히 `wsl2`에는 명시적인 WSL2 탐지와
[시스템 요구사항](system-requirements.md#wsl2-topology)의 배치가 필요합니다.

런타임 플랫폼 관찰은 실행 중인 Volicord binary의 정확한 `ReleaseTargetTriple`도
파생합니다. 닫힌 게시 target 집합과 허용 환경 셀은
[시스템 요구사항](system-requirements.md#first-release-environment-matrix)이 담당합니다.
지원 조회는 관찰한 target을 직접 사용하며 운영체제 이름만으로 identity를 만들지
않습니다. 최초 릴리스에서 WSL2는 명시적으로 탐지하고
`x86_64-unknown-linux-gnu`에만 대응합니다.

<a id="platform-release-coordinate"></a>

## `PlatformReleaseCoordinate`

모든 binding과 receipt는 필수인 닫힌 `platform_release_coordinate` 객체 하나를
담습니다. 네이티브 Linux, macOS, 네이티브 Windows는 정확히 다음 형태를
사용합니다.

```yaml
kind: native
```

WSL2는 정확히 다음 형태를 사용합니다.

```yaml
kind: wsl2
distribution_name: Ubuntu-24.04
distribution_id: ubuntu
distribution_version: "24.04"
environment_image: Ubuntu-24.04-LTS-WSL2
```

알 수 없는 필드, `platform_environment=wsl2`와 함께 쓴 `native` 좌표, 네이티브
환경과 함께 쓴 WSL2 좌표, 다른 WSL2 값은 유효하지 않습니다. 배포판 값 세 개는
현재 WSL2 프로세스에서 관찰합니다. `environment_image`는 이 배포판 사실에
등록된 정확한 런타임 지원 정책 이미지입니다. 정확한 지원 조회는 이 값을 내장 지원
entry와 대조하므로 다른 배포판 이미지의 entry는 binding을 승인할 수 없습니다.

<a id="codex-capability"></a>

## `CodexCapability`

이 문서는 `ManagedHostBinding`, `HostVerificationReceipt`,
`CodexSupportEntry`, `CodexReleaseEvidenceEntry`가 사용하는 capability 식별자를
담당합니다.
`CodexCapability`는 아래의 닫힌 값 집합입니다.

| 값 | 첫 릴리스 필수 동작 |
|---|---|
| `managed_stdio_mcp` | 아티팩트가 관리형 stdio MCP 경계를 시작하고 유지할 수 있습니다. |
| `record_workflow` | 아티팩트가 첫 릴리스 Record 프로필 작업 흐름을 완료할 수 있습니다. |
| `personal_managed_binding` | 아티팩트가 정확한 `personal` 관리 binding 생명주기를 지원합니다. |
| `shared_managed_binding` | 아티팩트가 정확한 `shared` 관리 binding 생명주기를 지원합니다. |

`FirstReleaseCodexCapabilities`는 이 네 값을 모두 담는 집합입니다. 첫 릴리스의
모든 binding, receipt, 지원 entry, 릴리스 증거 entry는 UTF-8 바이트 오름차순으로
정렬한 다음의 정확한 집합을 담습니다.

```text
managed_stdio_mcp
personal_managed_binding
record_workflow
shared_managed_binding
```

알 수 없는 값, 중복 값, 다른 순서, 엄격한 부분집합은 유효하지 않습니다.
Capability 집합은 정확한 아티팩트에서 검증한 동작을 나타내며
`connection_scope`, 명령 이름, 선택한 플랫폼에서 추론하지 않습니다.

<a id="managed-host-binding"></a>

## `ManagedHostBinding`

정규 binding과 중첩 record는 아래의 정확한 닫힌 형태를 가집니다. 적힌 필드
순서가 정규 record 순서이기도 합니다.

```yaml
ManagedHostBinding:
  host_kind: codex
  connection_scope: personal | shared
  command: ManagedCommand
  arguments: string[]
  forwarded_environment: EnvironmentForwarding[]
  configuration_target: ConfigurationTarget
  process_binding: ProcessBinding
  required_capabilities: CodexCapability[]
  platform_environment: PlatformEnvironment
  platform_release_coordinate: PlatformReleaseCoordinate

ManagedCommand:
  resolution: path_lookup | absolute_path
  program: string

EnvironmentForwarding:
  source_name: string
  target_name: string

ConfigurationTarget:
  owner: user | project
  path: string

ProcessBinding:
  process_id: u64
  process_start_token: string
  platform_instance_token: string
  executable_path: string
  executable_digest: string
```

표시한 모든 구성원은 필수이며 알 수 없는 구성원은 유효하지 않습니다. JSON
decoding은 중복 key도 거절합니다. `host_kind`는 정확히 `codex`이고,
`connection_scope`는 저장된 연결 의도와 일치합니다.
`required_capabilities`는 정확히 `FirstReleaseCodexCapabilities`이며
`platform_environment`는 정확히 탐지한 플랫폼입니다.
`platform_release_coordinate`는 위에서 정의한 정확한 네이티브 또는 WSL2
좌표와 일치합니다.

`ManagedCommand.resolution=path_lookup`일 때 `program`은 경로 구분자가 없는
비어 있지 않은 basename 하나여야 합니다. `absolute_path`일 때는 아래의
정규화된 절대 경로여야 합니다. `arguments`는 필수이며 모든 항목과 순서를
보존합니다. 빈 목록과 빈 인자는 누락 데이터가 아니라 identity에 포함되는
값입니다. 각 문자열은 유효한 UTF-8이고 NUL이 없으며 최대 4,096바이트입니다.

`forwarded_environment`는 주변 값을 담지 않고 전달 선언을 담습니다. 각 이름은
`[A-Z_][A-Z0-9_]*`와 일치해야 합니다. Entry는 `target_name`, 이어서
`source_name`의 UTF-8 바이트 순으로 정렬하며 `target_name` 중복은
유효하지 않습니다. 선택한 관리 설정에 환경 전달이 필요하지 않을 때만 빈 목록을
명시적으로 인코딩할 수 있습니다.

`ConfigurationTarget.owner`는 `personal`이면 `user`, `shared`이면
`project`입니다. `path`는 정확한 관리 Codex 설정 파일을 식별합니다.
`ProcessBinding.process_id`는 0이 아닙니다. 두 token 필드는 어댑터가 관찰한
불투명한 1~256바이트 UTF-8 값이며 제어 문자가 없어야 합니다. 두 값을 함께
사용해 PID 재사용과 플랫폼 인스턴스 재시작을 구분합니다. `executable_path`는
현재 관찰한 Codex 실행 파일의 해석된 정규 경로입니다.
`executable_digest`는 그 실행 파일 바이트의 SHA-256을 나타내는 정확히 64자의
소문자 16진수입니다.

Linux, macOS, WSL2 정규 경로는 `/`로 시작하고 `/` 구분자를 사용하며 `.`,
`..` 세그먼트, 중복 구분자, 루트가 아닌 경로의 마지막 구분자를 포함하지
않습니다. 네이티브 Windows 정규 경로는 `C:/...`처럼 대문자 드라이브 prefix와
슬래시를 사용합니다. UNC, device, 상대 경로, DrvFS, Windows에서 WSL로 변환한
표기는 유효하지 않습니다. 드라이브 prefix 뒤 component 표기는 보존합니다.
런타임 포함 규칙은 [런타임 경계](runtime-boundaries.md)가 계속 담당합니다.

Codex가 시작되기 전에 관리 설정을 설치할 수는 있지만, 어댑터가 실제
`ProcessBinding`을 관찰하고 검증하기 전에는 완전한 `ManagedHostBinding`이
되지 않습니다. 필드 누락과 허용되지 않은 빈 문자열은 유효하지 않으며 필드를
optional로 취급하거나 합성하지 않습니다.

<a id="canonical-binding-encoding"></a>

### 정규 인코딩과 Digest

Binding codec은 JSON, YAML, Serde map 순서, 호스트 endian과 무관합니다. 다음
primitive을 사용합니다.

```text
u32be(n)     = n as exactly four unsigned big-endian bytes
u64be(n)     = n as exactly eight unsigned big-endian bytes
blob(b)      = u32be(byte_length(b)) || b
string(s)    = blob(UTF8(s))
list(items)  = u32be(item_count) || blob(item_1_encoding) || ...
record(fields in declared order)
              = u32be(field_count)
                || string(field_1_name) || blob(field_1_encoding)
                || ...
```

Enum은 정확한 literal 표기의 `string`, `process_id`는 `u64be`, 문자열은
`string`, 배열은 `list`, 중첩 객체는 재귀적인 `record`로 인코딩합니다.
`canonical_binding_bytes`는 위 순서로 적힌 `ManagedHostBinding`의 `record`
인코딩입니다. 중첩 record도 표시한 순서를 사용합니다. `arguments`는 순서를
보존하고 환경 전달 선언과 capability는 필수 정규 순서를 사용합니다. 필드 이름도
인코딩하므로 허용되는 빈 문자열이나 목록도 이름 붙은 현재 필드로 남고 값 부재와
충돌하지 않습니다.

`PlatformReleaseCoordinate`는 중첩 record입니다. 네이티브 record에는
`kind=native`만 들어갑니다. WSL2 record에는 `kind=wsl2`,
`distribution_name`, `distribution_id`, `distribution_version`,
`environment_image`가 이 순서로 들어갑니다.

모든 개수와 바이트 길이는 `u32`에 들어가야 합니다. 검증과 경로 정규화는
인코딩 전에 수행합니다. Encoder는 trim, 대소문자 접기, 경로 변환, 기본값 삽입,
생략, map 순회를 수행하지 않습니다.

```text
binding_digest = "sha256:" || lowercase_hex(sha256(
  "volicord.managed-host-binding\0"
  || canonical_binding_bytes
))
```

따라서 `binding_digest`는 정확히 `sha256:` 뒤에 소문자 16진수 64자가 오는
형태입니다. 정확히 검증한 binding 내용을 식별하며 형식 버전 번호가 아닙니다.
내용 특성으로 다른 codec을 선택하면 안 됩니다.

<a id="codex-adapter-responsibilities"></a>

## Codex 어댑터 책임

Codex 어댑터는 호스트별 조사와 변경을 모두 담당합니다.

- 현재 결속이 가리키는 Codex 설치, 설정 대상, 현재 플랫폼 환경 탐색
- 정규 결속이 나타내는 관리 항목만 설치
- `ManagedHostBinding`과 그 digest 구성
- 정확한 `PlatformReleaseCoordinate` 관찰 및 결속
- 생성한 모든 관리 아티팩트의 digest 계산
- 정확한 Codex 아티팩트와 실행 파일 identity 검사
- 현재 프로세스 결속 검증
- 누락되거나, 변경되거나, 추가된 관리 설정을 설정 불일치로 탐지
- 전체 결속을 검증하고 typed `HostVerificationReceipt` 발급
- 현재 정규 입력으로 담당자가 정의한 관리 상태 repair
- 일치하는 Volicord 관리 상태만 제거

탐색만으로 아티팩트가 지원되는 것은 아닙니다. 어댑터는
`codex_artifact_digest`가 `process_binding.executable_digest`와 같고,
`target_triple` 및 `platform_environment`가 관찰한 Volicord target과 현재 환경에 일치하며,
`integration_profile`이 `record`이고, `verified_capabilities`가
`required_capabilities`와 정확히 같은 내장 `CodexSupportEntry` 하나만
허용합니다. 정확한 `platform_release_coordinate`도 binding 좌표와 같아야 합니다.
따라서 현재 binding, receipt, 지원 entry의 플랫폼 좌표, 실행 파일 digest, 프로필,
정확한 정규 capability 집합이 모두 일치해야 합니다. 이 조회에서는 외부 릴리스 증거를
읽지 않습니다. 알아볼 수 있는 명령 이름, 보고된 버전 범위, 비슷한 아티팩트, 일부
capability 일치, 다른 플랫폼 entry는 충분하지 않습니다. 어떤 부재나 불일치도
machine-readable reason
`unsupported_host_artifact`를 가진 `UnsupportedContract`입니다.

Repair는 정규 관리 상태와 호스트 설정 행동 데이터를 다시 생성합니다. 관련 없는 Codex
설정을 덮어쓰거나 선택한 프로젝트, 연결, 의도, profile, 플랫폼 환경을 암묵적으로
바꾸지 않습니다. 제거는 현재 identity가 Volicord 소유와 계속 일치하는 내용만
삭제합니다.

Core는 Codex 설정, 셸 문법, 생성 파일, 명령 문자열, 파일시스템 배치 규칙,
프로세스 문법을 parsing하지 않습니다. 정규 결속 identity와 typed 영수증만 받습니다.

<a id="host-verification-receipt"></a>

## `HostVerificationReceipt`

어댑터는 모든 검증 점검이 성공한 뒤에만 영수증을 발급합니다. 닫힌 형태는
다음과 같습니다.

```yaml
HostVerificationReceipt:
  contract_id: volicord.host-verification-receipt
  project_id: string
  connection_id: string
  host_kind: codex
  integration_profile: record
  platform_environment: PlatformEnvironment
  platform_release_coordinate: PlatformReleaseCoordinate
  required_capabilities: CodexCapability[]
  verified_capabilities: CodexCapability[]
  binding_digest: string
  generated_artifacts_digest: string
  executable_digest: string
  policy_digest: string
  verifier_build_digest: string
  observed_at: string
  expires_at: string
  result: verified
```

모든 구성원은 필수입니다. 알 수 없는 구성원과 중복 JSON key는 유효하지 않으며
기본값을 채우지 않습니다. `project_id`와 `connection_id`는 정확한 현재 Store
식별자입니다. 각각 1~1,024바이트 UTF-8이고 공백이 아닌 문자를 하나 이상
포함하며 제어 문자가 없어야 하고 trim하지 않고 보존합니다. 두 capability 배열은
`FirstReleaseCodexCapabilities`와 정확히 같습니다.

`platform_release_coordinate`는 정규 binding의 좌표 및 현재 독립적으로 관찰한
플랫폼 사실과 정확히 일치합니다. 따라서 `platform_environment`가 여전히
`wsl2`이더라도 WSL2 receipt를 다른 배포판이나 이미지에 재사용할 수 없습니다.

`binding_digest`는 위에서 정의한 정규 `sha256:<64-lowercase-hex>` 형태입니다.
`executable_digest`는 관찰한 Codex 실행 파일의 raw 64자 소문자 16진수
SHA-256이며 `process_binding.executable_digest` 및 대조한 지원 entry의
`codex_artifact_digest`와 정확히 같습니다. `policy_digest`는
`sha256:<64-lowercase-hex>` 형태의 정확한 현재 정규 `policy_fingerprint`입니다.
`verifier_build_digest`는 정확한 Volicord verifier 실행 파일 byte의 raw 64자
소문자 16진수 SHA-256입니다.

`generated_artifacts_digest`는 binding codec의 `string`, `list`, `record`
primitive을 사용합니다. 각 생성 아티팩트 entry는 `path`, 이어서 `digest`인
두 필드 record입니다. `path`는 정규화된 절대 플랫폼 경로이고 `digest`는
아티팩트 byte의 raw 64자 소문자 16진수 SHA-256입니다. 경로 중복을 거절한 뒤
`path` UTF-8 바이트 순으로 정렬합니다.

```text
generated_artifacts_digest =
  "sha256:" || lowercase_hex(sha256(
    "volicord.generated-managed-artifacts\0"
    || list(generated_artifact_entry_records)
  ))
```

`observed_at`과 `expires_at`은 정규 RFC 3339 UTC timestamp이며
`observed_at < expires_at`을 만족해야 합니다. `result`는 정확히
`verified`입니다. 검증이 실패했거나 unavailable, degraded, corrupt,
unsupported 상태이면 적용되는 실패 모델 결과를 반환하며 다른 `result` 값을
가진 receipt를 발급하지 않습니다.

발급한 receipt는 변경할 수 없습니다. 정확한 binding 하나에 대한 증거이며 bearer
token, 사용자 identity, 호스트 attestation, 독립적인 Core 권한 출처가 아닙니다.

<a id="core-receipt-validation"></a>

## Core 영수증 검증

Core는 typed 영수증만 소비하며 영수증에 의존하는 동작을 진행하기 전에 아래 항목을
모두 검증합니다.

- `project_id`가 해석한 현재 프로젝트와 일치
- `connection_id`가 해석한 현재 Agent Connection과 일치
- `host_kind=codex`와 `integration_profile=record`가 연결과 일치
- `platform_environment`가 현재 연결, binding, `process_binding`, 정확한 지원
  entry와 일치
- `platform_release_coordinate`가 현재 독립적으로 관찰한 좌표, binding, receipt,
  지원 entry와 정확히 일치
- `required_capabilities`와 `verified_capabilities`가 서로 같고,
  `FirstReleaseCodexCapabilities` 및 지원 entry의 `verified_capabilities`와
  정확히 일치
- `policy_digest`가 현재 policy 근거와 일치
- `binding_digest`와 `generated_artifacts_digest`가 현재 저장된 결속 및 관리
  아티팩트 identity와 일치
- `executable_digest`가 현재 process binding 및 정확한 지원 entry의
  `codex_artifact_digest`와 일치
- `verifier_build_digest`가 현재 허용한 verifier build와 일치
- `contract_id=volicord.host-verification-receipt`, `result=verified`이고
  `observed_at <= current_time < expires_at`
- 영수증이 현재 프로젝트, 연결, 결속, policy, capability 요구사항을 포함한 현재 Store
  기록에 결속됨

비교하는 정보 가운데 하나라도 무효화하는 Store 변경이 있으면 `expires_at` 전이라도
영수증은 오래된 상태입니다. `process_binding`을 무효화하는 플랫폼 수명 주기 변경도
영수증을 오래된 상태로 만듭니다. WSL2 재시작 동작은
[시스템 요구사항](system-requirements.md#wsl2-topology)이 정의합니다. Core는
유효하지 않은 영수증을 보정하려고 호스트 파일이나 실행 파일을 다시 조사하지 않습니다.

<a id="threat-model"></a>

## 위협 모델

신뢰 대상:

- 동일 운영체제 사용자 계정
- 해당 계정이 소유한 `Volicord Runtime Home`
- 해당 계정의 Store 쓰기 권한

비신뢰 대상:

- 외부 호스트 입력
- 오래된 영수증
- 다른 프로젝트 또는 연결의 영수증
- 수동으로 변경한 설정
- 변경된 실행 파일 또는 생성 아티팩트
- 지원 manifest에 없는 Codex 아티팩트

동일 사용자 권한으로 실행되는 악성 프로세스의 Runtime Home 변조는 최초 릴리스 위협
범위 밖입니다. 이 계약은 영수증 서명, 운영체제 keystore, key rotation, revocation을
추가하지 않습니다.

<a id="adjacent-owners"></a>

## 인접 담당 문서

- 외부 설명자 선택:
  [외부 계약](external-contracts.md)
- 정규 실패 범주:
  [실패 모델](failure-model.md)
- 관리형 stdio MCP 동작:
  [MCP 전송](mcp-transport.md)
- 설치, 검증, repair, 제거 명령:
  [관리 CLI](admin-cli.md)
- 플랫폼 셀과 WSL2 배치:
  [시스템 요구사항](system-requirements.md)
- 정확한 Codex 릴리스 아티팩트와 capability:
  [호스트 릴리스 증거](host-release-evidence.md)
- Runtime Home 및 Product Repository 경로 경계:
  [런타임 경계](runtime-boundaries.md)
- 보안 보장과 비보장:
  [보안](security.md)
