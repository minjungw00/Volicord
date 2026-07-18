# Agent Connection 참조

이 문서는 최초 릴리스 Agent Connection 계약을 정의합니다. 정확한
`host_kind=codex` Record 연결 표면, 정규 연결 검증 보고서, 관리 구성 소유권, 통합
revision, Codex 어댑터와 Core 사이의 검증된 운영 session 경계를 담당합니다.

<a id="owns-and-does-not-own"></a>

## 담당 범위

이 문서가 담당합니다.

- 허용하는 `host_kind`, integration profile, 연결 의도, 전송, 사용자 행동 전달 경로,
  mode, 플랫폼 환경 값
- 정규 `ConnectionVerificationReport`, 닫힌 상태 값, 결정적 집계, 엄격한 인코딩,
  보고서 부재 projection
- Connection과 프로젝트 통합 revision
- 권위 있는 managed-host runtime/project session 소유권
- `ValidatedAgentSession`과 Core가 이를 소비하기 전에 필요한 검사
- Codex 어댑터의 탐색, 설치, 검증, repair, 제거 책임

이 문서는 아래 항목을 담당하지 않습니다.

- stdio 프레이밍, MCP 초기화, 도구 처리 경로, 종료:
  [MCP 전송](mcp-transport.md)
- 관리 명령 문법, 출력, 종료 코드: [관리 CLI](admin-cli.md)
- 정확한 데이터베이스 테이블이나 저장 효과:
  [저장소 기록](storage-records.md), [저장 효과](storage-effects.md)
- 릴리스 셀 실행과 정확한 아티팩트 증거:
  [호스트 릴리스 증거](host-release-evidence.md)
- 운영체제 배치와 파일시스템 전제 조건: [시스템 요구사항](system-requirements.md)
- Core `UserActionRequest`와 `UserActionResolution` 스키마:
  [API User Action 스키마](api/schema-user-action.md)
- 제품 전체 실패 범주와 보안 의미:
  [실패 모델](failure-model.md), [보안](security.md)

<a id="surface-stability"></a>

## 표면 안정성

라벨은 [문서 정책](../maintain/documentation-policy.md#surface-stability-labels)의 기준
어휘를 따릅니다.

| 표면 | 안정성 | 계약 |
|---|---|---|
| 최초 릴리스 값 집합, `ConnectionVerificationReport`, 통합 revision, 권위 있는 운영 session, `ValidatedAgentSession` | `stable` | 정확한 경계 계약입니다. |
| Codex 탐색, 관리 설치, 검증, repair, 제거, drift 결과 의미 | `stable` | 관찰 가능한 계약을 유지하면서 구현을 바꿀 수 있습니다. |
| 어댑터 모듈, 파일시스템 helper, 생성된 시작 marker, Store query helper | `internal` | 안정된 경계를 보존하지만 공개 표면은 아닙니다. |
| 사람이 읽는 검증 안내와 client/host version 관찰 | `diagnostic` | Machine-readable 범주, 사유, typed 필드가 권위 있는 값입니다. |

<a id="first-release-surface"></a>

## 최초 릴리스 표면

최초 릴리스는 아래 Agent Connection 표면만 허용합니다.

| 차원 | 정확한 값 |
|---|---|
| 호스트 | `host_kind=codex` |
| Integration profile | `integration_profile=record` |
| 연결 의도 | `personal` 또는 `shared` |
| Connection mode | `read_only` 또는 `workflow` |
| 전송 | `volicord mcp --stdio`로 시작하는 Volicord 관리형 stdio MCP |
| 사용자 소유 행동 전달 | CLI inbox |
| 플랫폼 환경 | `linux`, `macos`, `native_windows`, `wsl2` |

`personal` 연결은 사용자 소유 로컬 Codex 구성을 설치합니다. `shared` 연결은 선택한
`Product Repository` 안에 프로젝트 소유 Codex 구성을 설치합니다. 두 경우 모두
Volicord가 생성한 관리 시작/구성 맥락으로 등록된 Connection 하나와 허용 프로젝트를
식별합니다.

Agent Connection은 `Volicord Runtime Home`에 저장하는 로컬 통합 기록입니다. 운영체제
권한을 부여하거나 사용자 identity를 성립시키거나 Codex가 관리 entry를 불러왔음을
증명하지 않습니다. 관리 stdio MCP 프로세스 하나는 현재 Agent Connection 하나에
결속됩니다.

사용자 소유 행동은 CLI inbox로 전달합니다. MCP 에이전트는 담당 문서가 정의한 행동을
요청할 수 있지만 로컬 사용자 채널로 동작하거나 사용자를 대신해 해결할 수 없습니다.

<a id="connection-verification-report"></a>

## `ConnectionVerificationReport`

작은 보고서 하나가 정규 직렬화 연결 검증 상태입니다.

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

Check는 `id`의 UTF-8 byte 오름차순으로 정렬합니다. Action도 같은 순서를 사용합니다.
엄격한 decoding은 다른 순서를 조용히 정규화하지 않고 거부합니다.

보고서의 모든 check는 그 보고서에 필수입니다. 최상위 상태는 check에서 파생됩니다.

1. `failed` check가 하나라도 있으면 `status=failed`입니다.
2. 그렇지 않고 `pending` check가 하나라도 있으면 `status=action_required`입니다.
3. 그렇지 않으면 `status=complete`입니다.

`dry_run`은 작업 mode이며 연결 상태나 check 상태가 아닙니다. 구성 일치, 실행 파일
가용성, protocol/host version, capability 관찰, 관찰 timestamp는 check 사실에 두며
별도 공개 또는 영속 상태 enum을 만들지 않습니다.

사용자 지시는 이 보고서의 `actions`에만 둡니다. Registry 저장소는 독립된 검증 상태나
action 배열을 저장하지 않습니다. 완료된 영속 보고서가 없는 연결은
`verification_not_run` pending check 하나와 검증 action 하나를 포함하는 합성
`status=action_required` 보고서로 projection합니다. 읽었다는 이유로 이를 저장하지
않습니다.

운영 호환성은 어댑터가 실제로 수행한 check와 관찰한 동작에서 보고합니다. `complete`는
정확한 host artifact의 release certification, 운영체제 집행, actor identity 증명,
correctness 증명, 조작 방지 기록을 뜻하지 않습니다. 연결 검증은 runtime 권한
credential을 발급하지 않습니다.

## 통합 Revision과 운영 Session

현재 Connection 통합 revision은 타입이 지정되고 domain-separated된 canonical SHA-256
digest입니다. Basis는 Agent Connection identity, host kind, intent, scope, mode, server
name, configuration target, 현재의 정확한 managed-configuration fingerprint입니다. 이
fingerprint는 관리 server command와 entry를 포함합니다.

Revision 구성은 관찰한 host version, executable path/digest, support-catalog 좌표,
release evidence, certified capability set, MCP client name/version을 제외합니다. 이 값은
권한을 바꿀 수 없습니다.

각 MCP process 시작은 host thread metadata가 생기기 전에 불투명 Registry runtime
session ID를 만듭니다. `session_source`는 정확히 `managed_host` 또는
`cli_preflight`입니다. `managed_host`만 Agent Connection 호출을 승인할 수 있습니다.
Runtime session은 소유 Connection과 Connection 통합 revision을 보관합니다.

프로젝트 통합 revision은 Connection revision에 현재 프로젝트 workflow-policy
fingerprint와 현재 Guard installation identity/policy hash 또는 Guard ownership의 명시적
부재를 더합니다. 프로젝트 Agent Session은 이 revision을 보관하며 다른 runtime
session, Connection, 프로젝트에 다시 결속할 수 없습니다.

이 기록은 현재 구성 아래 로컬에서 관찰한 협력적 protocol/session 소유권을 보여
줍니다. Binary, host, client, actor, 운영체제 사용자, human identity를 식별하지
않습니다. MCP client name/version과 관찰한 host executable version은 제한 안의 임의의
미래 값을 받고 diagnostic으로만 남습니다.

<a id="validated-agent-session"></a>

## `ValidatedAgentSession`

Core는 직렬화할 수 없는 아래 typed 경계로만 Agent Connection 호출 권한을 받습니다.

```rust
struct ValidatedAgentSession {
    connection_id: AgentConnectionId,
    project_id: ProjectId,
    runtime_session_id: AgentRuntimeSessionId,
    project_session_id: AgentSessionId,
    integration_revision: IntegrationRevision,
}
```

다음 현재 사실을 모두 검증한 뒤에만 이 값을 만듭니다.

1. Agent Connection이 존재하고 활성 상태입니다.
2. 프로젝트가 존재하고 현재 Connection Project입니다.
3. Runtime session이 해당 Connection에 속합니다.
4. 프로젝트 session이 해당 runtime session, Connection, 프로젝트에 속합니다.
5. Runtime과 프로젝트 session revision이 현재 Connection/프로젝트 통합 revision과
   일치합니다.
6. Connection mode가 요청한 operation category를 허용합니다.
7. `ActorSource::AgentConnection`이 검증된 Connection을 정확히 이름 붙입니다.
8. 프로젝트 범위 operation이 검증된 프로젝트를 정확히 이름 붙입니다.
9. Runtime session의 `session_source=managed_host`이며 `cli_preflight`가 아닙니다.
10. Client name/version과 host version을 권한에 사용하지 않습니다.

어댑터는 프로젝트 도구를 호출할 때마다 Core 호출 맥락을 만들기 전에 권위 있는 runtime
및 프로젝트 row를 검증합니다. Receipt, release-evidence, compatibility, fallback 경로는
없습니다.

Core는 감사 basis를 결정적으로 만듭니다.

```text
connection:<connection_id>/session:<project_session_id>/revision:<project_integration_revision>
```

이 basis는 로컬 운영 소유권을 이름 붙입니다. Certificate, receipt, identity proof,
bearer token, host attestation, trusted host digest가 아닙니다.

## Codex 어댑터 책임

Codex 어댑터는 host별 구성 조사와 변경을 담당합니다.

- Codex configuration target과 플랫폼 환경 탐색
- 현재 Connection 입력이 선택한 관리 entry만 설치
- 시작 시 Connection과 선택적 프로젝트를 선택하는 command, arguments, Runtime Home
  forwarding, 관리 시작 marker 생성
- 누락되거나 변경되거나 추가된 관리 구성을 drift로 탐지
- executable 가용성과 제한된 host version diagnostic 보고
- 현재 정규 입력으로 담당 문서가 정의한 관리 상태 repair
- 일치하는 Volicord 관리 상태만 제거

Runtime 권한은 parent executable을 hash하거나, platform release coordinate를 비교하거나,
내장 support catalog를 조회하거나, binding/verifier-build digest를 계산하거나, host 검증
receipt를 발급·읽기·검증하지 않습니다. 알아볼 수 있는 command name, process path,
version string, 환경 값, local session은 actor identity가 아닙니다. 관리 시작 맥락과
권위 있는 Store session은 위의 협력적 소유권 경계만 세웁니다.

Repair는 관련 없는 Codex 구성을 덮어쓰거나 선택한 프로젝트, Connection, intent,
profile, 플랫폼 환경을 암묵적으로 바꾸지 않습니다. 제거는 현재 관리 identity가
Volicord 소유와 계속 일치하는 내용만 삭제합니다.

## 위협 모델

신뢰 대상:

- 동일 운영체제 사용자 계정
- 해당 계정이 소유한 `Volicord Runtime Home`
- 해당 계정의 Store 쓰기 권한

비신뢰 대상:

- 외부 host/client 입력
- CLI-preflight, 오래됨, 닫힘, revision 불일치 session
- 다른 프로젝트, runtime, Connection의 session
- 수동으로 변경한 구성
- identity 주장으로 쓰는 client/host version과 process metadata

동일 사용자 권한으로 실행되는 악성 프로세스의 Runtime Home 변조는 최초 릴리스 위협
범위 밖입니다. 이 계약은 binary attestation, 운영체제 keystore, signing, key rotation,
revocation을 추가하지 않습니다.

## 인접 담당 문서

- 관리 stdio MCP 동작: [MCP 전송](mcp-transport.md)
- 설치, 검증, repair, 제거 명령: [관리 CLI](admin-cli.md)
- 플랫폼 셀과 WSL2 배치: [시스템 요구사항](system-requirements.md)
- 정확한 Codex 릴리스 아티팩트와 release-only capability:
  [호스트 릴리스 증거](host-release-evidence.md)
- Runtime Home 및 Product Repository 경계: [런타임 경계](runtime-boundaries.md)
- 보안 보장과 비보장: [보안](security.md)
