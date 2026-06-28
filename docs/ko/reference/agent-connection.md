# Agent Connection 참조

이 문서는 로컬 MCP 호스트 통합을 위한 Agent Connection과 현재 연결 맥락의 경계를 담당합니다. 요청이 Core에 들어가기 전에 Agent Connection, 연결 프로젝트, 연결 모드, `actor_source`, `operation_category`를 어떻게 해석하는지 정의합니다.

공개 API 스키마, 메서드 동작, 저장 효과, 보안 보장 의미, `volicord-mcp` 와이어 동작, Core 권한 의미는 이 문서가 정의하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- Agent Connection 의미와 Connection Projects 멤버십 규칙
- MCP 호스트 호출의 현재 연결 맥락 경계
- `actor_source`와 `operation_category` 출처 경계
- 권한을 지니는 판단 해결에서 User Channel과 Agent Connection의 경계
- 호출별 MCP 프로젝트 선택과 프로젝트 가용성 경계
- 담당 결과와 Agent Connection 사이의 에이전트 맥락 전달 규칙
- 선택된 Agent Connection이나 현재 연결 맥락을 사용할 수 없거나, 맞지 않거나, 오래되었거나, 충분하지 않을 때의 대체 표시

이 문서는 담당하지 않습니다.

- API 요청 래퍼, 응답 분기, 스키마 형태, 동작 범주 값 이름: [API 코어 스키마](api/schema-core.md), [API 메서드](api/methods.md), 메서드 담당 문서, [API 값 집합](api/schema-value-sets.md)
- `volicord-mcp` 실행 파일 시작, 프로세스 환경, stdio 프레이밍, 시작 검증, 응답 래핑, 종료: [MCP 전송](mcp-transport.md)
- 관리 Agent Connection 명령, 호스트 설정, 상태, 검증, 제거 동작: [관리 CLI](admin-cli.md)
- 저장소 배치, 아티팩트 생명주기, 스테이징 핸들 검증: [참조 색인](README.md)에서 고르는 저장소와 아티팩트 담당 문서
- 보안 보장 의미나 접근 경계 표현: [보안](security.md)
- 권한과 파생 표시의 구분 규칙: [상태 보기와 템플릿 표시 경계](projection-and-templates.md)
- 렌더링 본문 문구, 공개 표시 라벨, 템플릿 표현: [템플릿 본문](template-bodies.md)

## Agent Connection

Agent Connection은 `connection_id`로 식별되는 로컬 MCP 호스트 연결 단위입니다. 하나의 `volicord-mcp --connection <connection_id>` 프로세스는 고정된 `Product Repository` 하나가 아니라 Agent Connection 하나에 묶입니다.

저장되는 Agent Connection 필드는 아래를 포함합니다.

- `connection_id`
- `host_kind`
- `host_scope`
- `server_name`
- `config_target`
- `mode`
- `enabled`
- `managed_fingerprint`
- `last_verified_status`
- 생성 및 갱신 시각

규칙:

- Agent Connection은 에이전트 대상이며 로컬 `User Channel`로 동작할 수 없습니다.
- 연결은 호스트 설정을 편집하지 않고도 켜거나 끌 수 있습니다.
- 연결 등록은 `Volicord Runtime Home`의 모든 프로젝트를 자동으로 부여하지 않습니다.
- 연결은 Connection Projects 기록에 명시적으로 들어 있는 프로젝트만 다룰 수 있습니다.
- `connection.mode=read_only`는 읽기와 프로젝트 탐색 동작을 노출합니다. 워크플로 쓰기 역량이 아닙니다.
- `connection.mode=workflow`는 읽기와 프로젝트 탐색 동작에 더해 에이전트 워크플로 동작을 노출합니다. 사용자 전용 판단 기록은 노출하지 않습니다.
- `connection_id`, 연결 모드, 호스트 설정, MCP 서버 지침은 OS 권한, 호스트 신뢰, 비밀 격리, 파일시스템 ACL, 네트워크 정책, 사용자 권한이 아닙니다.

저장 기록 계열과 DDL은 [저장소 기록](storage-records.md)과 [저장소 DDL](storage-ddl.md)이 담당합니다. 관리 생성, 갱신, 검증, 제거 명령은 [관리 CLI](admin-cli.md)가 담당합니다.

## Connection Projects

Connection Projects는 Agent Connection과 등록 프로젝트 사이의 명시적 레지스트리 관계입니다.

멤버십 필드:

- `connection_id`
- `project_id`
- 생성 시각
- `connection_id`와 `project_id`의 복합 기본 키

규칙:

- 프로젝트 멤버십은 프로젝트 상태, 경로 분리, 저장소 실행 가능성, Agent Connection 모드, 메서드 담당 호출 요구사항을 우회하지 않습니다.
- 유효하지 않은 현재 프로젝트 등록은 연결 프로젝트 기록으로 반환하지 말고 Connection Projects 목록 조회와 접근 해석에서 거절해야 합니다.
- inactive이거나 그 밖의 이유로 실행 부적격인 유효한 프로젝트는 멤버십이 있어도 실행 시점에 계속 사용할 수 없습니다.
- Connection Project 제거 또는 Agent Connection 비활성화는 호스트 설정을 다시 쓰지 않아도 효력을 가져야 합니다.
- 연결 프로젝트가 없는 Agent Connection은 저장된 상태로 남을 수 있으며, 호스트 설정도 디스크에 남을 수 있습니다. 이 저장 상태는 새 `volicord-mcp` 프로세스가 성공적으로 시작될 수 있다는 뜻이 아닙니다.
- 새 MCP stdio 시작과 `volicord-mcp --check --connection <connection_id>`는 Agent Connection에 연결 프로젝트가 하나도 없으면 시작 검증에 실패합니다.
- 하나 이상의 프로젝트가 연결되어 있을 때 이미 시작된 `volicord-mcp` 프로세스는 호스트 설정을 다시 쓰지 않아도 이후 멤버십 변경을 관찰할 수 있습니다. 마지막 멤버십이 제거된 뒤 `volicord.list_projects`는 빈 프로젝트 목록을 반환할 수 있지만, 연결 프로젝트가 남아 있지 않으므로 프로젝트 라우팅이 필요한 공개 도구는 정상 진행할 수 없습니다.
- 프로젝트가 연결되고 시작 또는 호출별 프로젝트 점검이 필요한 프로젝트 상태를 검증할 수 있어야 Agent Connection을 다시 실행할 수 있습니다.

## 호스트 설정 인벤토리

저장된 Agent Connection은 Volicord가 관리하는 호스트 설정과 검증 상태를 위한 관리 인벤토리입니다. 호스트 설정 파일은 외부 호스트의 운영상 원천으로 남습니다. 레지스트리 기록은 관리 인벤토리와 마지막으로 알려진 검증 상태일 뿐이며 호스트 설정을 대신하지 않습니다.

지원되는 호스트와 범위 행렬:

| 호스트 종류 | 기준 범위 | 범위 의미 |
|---|---|---|
| `codex` | `user`, `project` | 사용자 범위는 사용자의 Codex 프로젝트들에서 로드될 수 있습니다. 프로젝트 범위는 프로젝트 범위 Codex MCP 설정을 쓰며, 호스트가 이를 로드하려면 Codex 프로젝트 신뢰가 필요합니다. |
| `claude_code` | `local`, `project`, `user` | local과 project 범위는 연결된 프로젝트에서만 로드됩니다. user 범위는 사용자의 Claude Code 프로젝트들에서 로드될 수 있습니다. |
| `generic` | `export` | Volicord는 사용자가 관리하는 호스트를 위한 명시적 설정을 내보내며 직접 설치를 주장하지 않습니다. |

규칙:

- project와 local 범위는 연결된 `Product Repository` 하나만 허용합니다.
- user 범위는 명시적으로 추가된 여러 `Product Repository` 등록을 허용할 수 있습니다.
- 호스트 신뢰, 프로젝트 신뢰, 프로젝트 MCP 승인, OAuth, 또는 그와 비슷한 호스트 통제 승인은 Volicord가 우회할 수 없습니다.
- 호스트 설정 쓰기는 파일 작업으로 성공했더라도 호스트가 아직 서버를 신뢰, 승인, 로드, 초기화, 노출하지 않았다면 결과 상태가 `action_required`로 남을 수 있습니다.
- `last_verified_status=complete`는 [관리 CLI](admin-cli.md#agent-connection-result-states)가 담당하는 운영 게이트를 만족한 관리 검증 결과에 대해서만 저장할 수 있습니다. Volicord가 직접 시작한 MCP handshake만으로는 충분하지 않습니다.
- `last_verified_status=action_required`는 Volicord가 설정을 관리하거나 내보낼 수 있지만 호스트가 소유한 신뢰, 승인, OAuth, reload, restart 동작이 남아 있을 때의 예상 상태입니다.
- `generic` export는 사용자가 관리하는 설정 인벤토리로 남습니다. 외부 호스트가 로드했다는 사실을 증명하지 않으며, 나중에 호스트별 담당 문서가 관찰 가능한 로드 가능성 게이트를 정의하지 않는 한 `complete`가 되면 안 됩니다.
- 거절됨, 없음, 변경됨, 사용할 수 없음, 알 수 없음 호스트 상태는 `complete` Agent Connection 상태가 아닙니다.
- Product Repository 지침, 생성된 호스트 지침, MCP 서버 지침은 도구 선택을 개선할 수 있지만 강제 메커니즘이 아니며 모델이 항상 Volicord 도구를 선택한다고 보장할 수 없습니다.

<a id="current-connection-context"></a>
## 현재 연결 맥락

현재 연결 맥락은 MCP 도구 호출 하나에 대해 파생되는 로컬 호출 맥락입니다. 로컬 어댑터가 선택된 Agent Connection, 선택된 프로젝트, 호출된 메서드, 요청 래퍼에서 파생합니다. 이는 공개 요청 페이로드가 아닙니다.

MCP 세션은 어댑터 시작 시 정확히 하나의 `connection_id`에 묶입니다. 선택 프로젝트는 프로세스 수명 동안 고정되는 것이 아니라 공개 MCP 도구 호출마다 결정됩니다.

공개 MCP 메서드 호출의 프로젝트 선택은 결정적입니다.

1. `ToolEnvelope.project_id`가 있으면 그 값을 사용합니다.
2. 없고 Agent Connection에 연결되어 사용 가능한 프로젝트가 정확히 하나 있으면 그 프로젝트를 사용합니다.
3. 그 밖의 경우 호출을 모호함으로 거절하고 에이전트에게 `volicord.list_projects`를 호출하라고 안내합니다.

어댑터는 폴더 이름, 프로세스 현재 작업 디렉터리, 호스트 roots, 호스트 라벨, 저장소가 반환한 첫 행에서 프로젝트를 추측하면 안 됩니다. MCP roots는 향후 또는 호스트가 제공하는 선택적 힌트로만 사용할 수 있습니다. roots는 위의 결정적 선택 순서를 바꾸지 않습니다.

`volicord.list_projects`는 읽기 전용 MCP 어댑터 유틸리티 도구입니다. 이 도구는 묶인 Agent Connection에 명시적으로 연결된 프로젝트만 나열하고, 가용성을 보고하며, 에이전트가 유효한 `project_id`를 선택할 수 있을 만큼의 프로젝트 식별 정보를 제공합니다. 이는 공개 Volicord Core API 메서드 목록 밖에 있으며 그 목록에 추가되면 안 됩니다.

공개 도구 호출이 Core에 들어가기 전에 MCP 어댑터는 아래를 검증해야 합니다.

- Agent Connection이 존재하고 활성화되어 있습니다.
- 선택된 프로젝트가 그 Agent Connection에 명시적으로 연결되어 있습니다.
- 선택된 프로젝트가 active이고 실행 가능합니다.
- 연결 모드가 메서드의 `operation_category`를 허용합니다.

연결 모드와 동작 범주:

| Agent Connection 모드 | MCP를 통해 허용되는 동작 범주 | MCP에 보이는 공개 메서드 도구 | MCP 어댑터 유틸리티 도구 |
|---|---|---|---|
| `read_only` | `read` | 2개: `volicord.status`, `volicord.close_task` | `volicord.list_projects` |
| `workflow` | `read`, `agent_workflow` | 8개: `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.prepare_write`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_judgment`, `volicord.close_task` | `volicord.list_projects` |

`volicord.record_user_judgment`는 `operation_category=user_only`입니다. User Channel 경로를 위한 공개 Core API 메서드이지만 Agent Connection에는 노출되지 않습니다. 권한을 지니는 답변을 기록하는 지원 로컬 사용자 경로는 [관리 CLI](admin-cli.md#user-channel-commands)가 담당하는 `volicord user` 명령군입니다.

내부 행위자 형태이며 공개 API 스키마가 아닙니다.

```yaml
InvocationContext:
  actor_source: local_user | system | agent_connection:<connection_id>
  operation_category: read | agent_workflow | user_only | admin_local
  verification_basis: string
  assurance_level: string
```

기준 `assurance_level`은 협력적 로컬 출처를 뜻하며 암호학적 인간 신원 증명이 아닙니다. 권한을 지니는 사용자 판단 해결에는 `actor_source=local_user`, `operation_category=user_only`, 호환 User Channel 출처, 메서드가 정의한 호환성이 필요합니다. Agent Connection은 복사된 사용자 텍스트나 생성 지침을 제출해 사용자 권한을 얻을 수 없습니다.

조건:

- 공개 API 요청 하나에는 파생된 `InvocationContext`가 정확히 하나 있습니다.
- 공개 `ToolEnvelope.project_id`는 있을 때 Agent Connection의 연결 프로젝트로 제한되는 결정적 프로젝트 선택자입니다. 호출자 권한이 아니며 목록에 없거나 inactive이거나 무효인 프로젝트 접근을 부여할 수 없습니다.
- `ToolEnvelope`는 `actor_source`나 `operation_category`를 노출하지 않습니다. 원시 MCP 인자에 이 필드가 들어 있으면 어댑터는 Core 실행 전에 호출을 거절합니다.
- `ArtifactInput`이나 `StagedArtifactHandle` 같은 중첩 페이로드는 두 번째 호출 맥락을 추가하지 않습니다.
- 해결된 권한 판단의 권한 출처 필드는 호출자 텍스트, 라벨, 답변 본문, 복사된 참조, 생성된 Markdown, Product Repository 지침이 아니라 파생된 `InvocationContext`에서 옵니다.
- 보호된 읽기, 상태 변경, 아티팩트 동작은 메서드 담당 문서가 파생된 호출 맥락을 받아들일 때만 그 호출에 의존할 수 있습니다.

에이전트가 할 수 있는 것:

- 담당 결과 맥락을 표시하거나 전달할 때 파생된 호출 맥락을 보존할 수 있습니다.
- 맥락이 없거나 호환되지 않으면 사용 불가, 불일치, 오래됨, 충분하지 않은 Agent Connection 상태로 표시할 수 있습니다.

에이전트가 하면 안 되는 것:

- `InvocationContext`를 요청 페이로드로 제출하면 안 됩니다.
- `verified=true`를 스스로 주장하면 안 됩니다.
- Agent Connection에서 `actor_source=local_user`나 `operation_category=user_only`를 제출해 사용자 권한을 만족시키면 안 됩니다.
- 임의의 검증 근거 문구를 공개 요청 권한으로 제출하면 안 됩니다.
- 스테이징된 아티팩트 출처를 꾸며 내면 안 됩니다.
- 복사된 식별자, 생성된 Markdown, 대화 텍스트, 상태 보기 텍스트, 에이전트 기억을 현재 연결 맥락의 대체물로 쓰면 안 됩니다.

담당 문서 링크:

- 정확한 요청 래퍼와 응답 형태는 [API 코어 스키마](api/schema-core.md), [API 메서드](api/methods.md), 메서드 담당 문서가 담당합니다.
- `operation_category` 값 이름은 [API 값 집합](api/schema-value-sets.md)이 담당합니다.
- `volicord-mcp` 시작, 연결 바인딩, 환경 변수, stdio 프레이밍, 시작 검증, 응답 래핑, 종료는 [MCP 전송](mcp-transport.md)이 담당합니다.

## User Channel과 Agent Connection

Agent Connection은 에이전트 대상 연결입니다. 모델이 사용자의 말을 전달하고 있더라도 `User Channel`이 아닙니다.

조건:

- 사람이 대기 중인 판단을 확인하고 Core 생성 선택지를 골라 기록하는 지원 로컬 CLI 경로는 [관리 CLI](admin-cli.md#user-channel-commands)가 담당하는 `volicord user` 명령군입니다.
- 권한을 지니는 사용자 판단 해결에는 `actor_source=local_user`, `operation_category=user_only`, 호환 User Channel 출처가 필요합니다.
- `actor_source=agent_connection:<connection_id>`는 사용자의 텍스트를 전달해도 `local_user` 출처가 될 수 없습니다.

에이전트가 할 수 있는 것:

- 메서드 담당 문서가 그 경로를 지원할 때 빠진 사용자 소유 판단을 요청할 수 있습니다.
- 담당 결과가 반환한 대기 판단 상태와 Core 생성 선택지를 표시할 수 있습니다.
- 사람 사용자를 지원되는 `User Channel`로 안내할 수 있습니다.

에이전트가 하면 안 되는 것:

- Agent Connection에서 권한을 지니는 사용자 결정을 기록하면 안 됩니다.
- 자연어 승인, 채팅 답변, 생성된 Markdown 상태, 렌더링된 상태 보기를 User Channel 출처로 취급하면 안 됩니다.
- 선택지 하나를 최종 수락, 잔여 위험 수락, 민감 동작 승인, 범위 수락, 또는 다른 판단 종류로 넓히면 안 됩니다.
- 표시된 판단 문구에서 증거 충분성, 수락, 잔여 위험 수락, 닫기 준비 상태, 보안 권한을 만들면 안 됩니다.

담당 문서 링크:

- [Core 모델](core-model.md)은 사용자 소유 판단, 최종 수락, 잔여 위험 수락, 증거, 닫기 준비 상태의 권한 의미를 담당합니다.
- [사용자 판단 기록 메서드](api/method-record-user-judgment.md)는 대기 판단 하나를 해결하는 공개 메서드 동작을 담당합니다.
- [상태 보기와 템플릿 표시 경계](projection-and-templates.md)는 생성 표시와 상태 보기 권한 경계를 담당합니다.

## 에이전트 동작 지침

에이전트 동작 지침은 두 계층으로 나뉩니다.

- MCP 서버 지침은 MCP 초기화 중 서버가 항상 제공합니다.
- 선택적 `Product Repository` 지침은 관리 명령이 지원하고 사용자가 명시적으로 승인한 경우에만 설치됩니다.

규칙:

- MCP 서버 지침은 Volicord 도구 전체에 적용되는 도구 간 흐름, 프로젝트 선택 규칙, 제한을 설명할 수 있습니다.
- 선택적 저장소 지침은 [런타임 경계](runtime-boundaries.md#explicit-integration-files-in-product-repositories)가 담당하는 경계 안에서만 `Product Repository` 안의 Volicord 관리 블록이나 호스트별 규칙 파일을 추가할 수 있습니다.
- 지침은 도구 선택을 개선할 수 있지만 권한, 접근 통제, 사용자 판단, 보안 강제, 모델이 Volicord 도구를 선택한다는 증거가 아닙니다.

## 에이전트 맥락 전달

에이전트 맥락 전달은 다음 행동에 필요한 담당 맥락만 에이전트에 제공하되, 그 패킷을 권한 기록으로 만들지 않는 규칙입니다.

조건:

- 에이전트 맥락에는 다음 행동에 필요한 담당 결과와 그 행동에 영향을 주는 현재 연결 맥락의 한계만 담아야 합니다.
- 맥락 패킷은 지원 맥락일 뿐 Core 상태, 저장소 상태, 증거, 수락, 잔여 위험 수락, 닫기 출력이 아닙니다.

에이전트가 할 수 있는 것:

- 현재 `Task` 요약, 현재 적용 범위, `state_version`, 대기 중인 사용자 소유 판단, 차단 사유, 다음 안전한 행동, 증거와 아티팩트 요약, 닫기 준비 상태와 잔여 위험 요약, 담당 문서가 뒷받침하는 보장 표시, 출처 또는 제한 메모를 담은 압축 맥락을 전달할 수 있습니다.
- 다음 행동에 필요할 때만 정확한 담당 문서 섹션을 가져올 수 있습니다.
- 한영 문서 유지보수에서 의미 일치 검토가 필요할 때만 같은 `doc_id`의 두 언어 문서를 함께 가져올 수 있습니다.

에이전트가 하면 안 되는 것:

- 전체 스키마, DDL, 과거 로그, 아티팩트 본문, 관련 없는 계약 자료, 지원 범위 밖 기능 목록, 정확한 템플릿 본문, 같은 `doc_id`의 두 언어 문서를 기본으로 주입하면 안 됩니다.
- 오래되었거나 복사된 맥락 패킷을 담당 결과나 기반 기록보다 최신 권한처럼 취급하면 안 됩니다.

담당 문서 링크:

- [템플릿 본문](template-bodies.md)은 에이전트 맥락 패킷 문구를 담당합니다.
- [참조 색인](README.md)은 정확한 담당 문서 섹션 경로를 안내합니다.
- [번역 정책](../maintain/translation-policy.md)은 한영 의미 일치 검토 지침을 담당합니다.

## 대체 경계

현재 연결 맥락이나 필요한 연결 모드를 사용할 수 없거나, 맞지 않거나, 오래되었거나, 충분하지 않을 때 대체 표시를 사용합니다.

에이전트가 할 수 있는 것:

- 적절한 연결 모드나 다른 연결 프로젝트로 옮길 수 있습니다.
- 동작을 좁힐 수 있습니다.
- 빠진 사용자 소유 판단을 요청할 수 있습니다.
- 사용자가 그 방식을 명시적으로 선택한 경우에만 Volicord 밖에서 계속할 수 있습니다.

에이전트가 해야 하는 것:

- 제한을 지원 문구나 표시 문구에 드러내야 합니다.
- 기계 판독용 실패 의미는 [API 오류 코드](api/error-codes.md)와 [API 오류 세부사항](api/error-details.md)으로 보내야 합니다.
- 사용자에게 보이는 문구는 [템플릿 본문](template-bodies.md)으로 보내야 합니다.

에이전트가 하면 안 되는 것:

- 권한을 지어내면 안 됩니다.
- 사용 불가, 불일치, 오래됨, 충분하지 않은 맥락 상태를 일반 성공 문구 속에 숨기면 안 됩니다.
- 사용자의 명시적 선택 없이 Volicord 밖에서 계속하면 안 됩니다.
