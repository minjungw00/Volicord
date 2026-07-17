# MCP 전송 참조

이 문서는 최초 릴리스의 로컬 MCP 프로세스 경계인 관리 stdio 시작, 엄격한 binding,
JSON-RPC lifecycle, 도구 검색, 공개 인자 projection, 응답 wrapping, 종료를 담당합니다.
Core 메서드, Codex 구성, 저장 효과, 릴리스 증거는 각각의 집중 담당 문서에 남습니다.

<a id="surface-stability"></a>
## 표면 안정성

라벨은 [문서 정책](../maintain/documentation-policy.md#surface-stability-labels)을 사용합니다.

| 표면 | 안정성 |
|---|---|
| `volicord mcp --stdio`, 초기화, `tools/list`, `tools/call`, 응답 wrapping | `stable` |
| stable 프로세스와 메서드 집합에 나열하지 않은 pre-1.0 추가 표면 | `beta` |
| 프로세스 binding 값과 생성 구성 세부사항 | `internal` |
| 시작과 프로토콜 진단 | `diagnostic` |

## 프로세스 모델

`volicord mcp --stdio`는 관리 Codex 구성이 시작하는 자식 프로세스입니다. stdin과
stdout으로 줄 단위 JSON-RPC를 교환하며 TCP, HTTP, Unix domain socket 또는 그 밖의
네트워크 listener를 열지 않습니다.

```text
volicord mcp --stdio --connection <connection_id> [--project <project_id>]
volicord mcp --stdio --discover-repository --host codex
volicord mcp --check --connection <connection_id> [--project <project_id>]
```

결속 형태는 정확한 저장 식별자를 사용합니다. 저장소 검색은 정규 공유 Codex binding
전용이며 정확한 Runtime Home과 정규 Git 작업 트리에서 identity를 해결합니다. cwd만으로
연결을 추론하거나 주변 저장소를 검색하거나 다른 host selector를 받지 않습니다.
`--check`는 stdio loop에 들어가지 않고 사전 점검만 수행합니다.

## 환경과 시작

`VOLICORD_HOME`은 [런타임 경계](runtime-boundaries.md)에 따라 Runtime Home을
선택합니다. 공유 구성은 머신 로컬 경로를 내장하지 않고 값을 전달합니다.

MCP 요청을 읽기 전에 어댑터는 현재 `ExternalContractDescriptor`,
`ManagedHostBinding`, 선택한 연결, 허용 프로젝트, Runtime Home/Product Repository
분리, 정확한 `StorageManifest`, 필요한 저장 읽기 가능성을 검증합니다. 알 수 없는
descriptor, 지원하지 않는 아티팩트, 손상된 기록, 모호한 선택, 사용할 수 없는 저장소에는
[실패 모델](failure-model.md)을 적용합니다. 시작은 다른 형식을 탐색하거나 빠진 필드를
채우거나 다른 전송을 시작하지 않습니다.

## MCP wire 동작

비어 있지 않은 stdin 각 줄은 완전한 UTF-8 JSON-RPC 2.0 요청 하나입니다. 잘못된
JSON은 `-32700`, 잘못된 요청은 `-32600`, 알 수 없는 메서드는 `-32601`, 잘못된
인자는 `-32602`, 내부 프로토콜 실패는 `-32603`을 반환합니다. 응답은 요청 `id`를
보존합니다.

`initialize`가 `tools/list`와 `tools/call`보다 먼저 와야 합니다. 프로세스는 지원 MCP
protocol version만 협상하고 `notifications/initialized`를 받습니다. 초기화 전 호출,
반복 initialize, batch 입력, 지원하지 않는 version은 Core 전에 실패합니다.

## 도구 검색

| 모드와 저장소 | MCP에 보이는 도구 |
|---|---|
| `workflow`, 쓰기 가능 | `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.get_operation_result`, `volicord.prepare_write`, `volicord.prepare_evidence_capture`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_action`, `volicord.reconcile_changes`, `volicord.check_close`, `volicord.close_task`, `volicord.list_projects` |
| `workflow`, 읽기만 가능 | `volicord.status`, `volicord.get_operation_result`, `volicord.request_user_action`(resume만), `volicord.check_close`, `volicord.list_projects` |
| `read_only`, 읽기 가능 | `volicord.status`, `volicord.get_operation_result`, `volicord.check_close`, `volicord.list_projects` |
| 읽을 수 있는 허용 프로젝트 없음 | `volicord.list_projects` |

Task 상태와 이전 호출은 도구를 동적으로 추가하지 않습니다. 숨긴 mutation은 Core 효과
없이 실패합니다. `volicord.resolve_user_action`은 공개 Core API 메서드이지만 MCP 도구는
아닙니다.

## 공개 인자 projection

`tools/call`은 문자열 `params.name`과 선택적인 객체 `params.arguments`를 사용합니다.
공개 schema는 Core envelope, 내부 연결/프로젝트 ID, protocol metadata, idempotency
필드, actor source, operation category, verification basis를 숨깁니다. 숨긴 필드는 Core
전에 거부합니다. 간결한 검색 schema는 담당 문서의 완전한 요청 검증을 느슨하게 하지
않습니다.

<a id="mutation-authority-receipt-projection"></a>
## 응답 wrapping

읽기 전용 도구는 공개 메서드 결과를 structured content로 반환합니다. Mutation은
선택한 `summary`, `workflow`, `full` projection에 새 `AuthorityReceipt`, 정확한 효과
identity, replay 사실, 제한된 복구 정보를 담습니다. Text는 사람용 rendering이며 다른
권한 출처가 아닙니다.

Core 효과를 커밋한 뒤 전달이 실패하면 operation-result 좌표를 보존합니다. 응답 직렬화나
전송이 실패했다는 이유로 mutation을 다시 시도하지 않습니다.

## UserAction 요청

MCP 에이전트는 `volicord.request_user_action`으로 대기 요청을 만들거나 명시적인 읽기
전용 resume 분기를 사용할 수 있습니다. 나중에 현재 상태와 불변 CLI resolution
identity의 안전한 snapshot을 관찰할 수 있습니다. 비공개 inbox form, note, submission
identity, credential은 받지 않습니다.

어댑터는 요청에 답하거나 해결하지 않고 서버가 시작하는 resolution 요청도 보내지
않습니다. 사용자는 `volicord inbox resolve`로만 해결합니다. Guard prompt 관찰이
있다면 권한이 아닌 관찰로 남습니다.

## 종료와 재연결

EOF는 처리 중인 응답 뒤 loop를 닫습니다. 새 프로세스는 시작 검증과 MCP 초기화를 다시
수행하며 이전 프로세스의 연결, 프로젝트, receipt, 현재 상태를 상속하지 않습니다.

## 관련 담당 문서

- [Agent Connection](agent-connection.md)
- [관리 CLI](admin-cli.md)
- [API 메서드](api/methods.md)
- [API UserAction 스키마](api/schema-user-action.md)
- [저장 효과](storage-effects.md)
- [보안](security.md)
