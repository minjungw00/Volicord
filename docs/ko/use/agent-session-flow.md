# 에이전트 세션 흐름

## 이 문서로 할 수 있는 일

이 문서는 Harness를 쓰는 에이전트 세션이 사용자에게 어떻게 보여야 하는지 설명합니다. 무엇을 보여주고, 언제 묻고, 언제 계속하고, 언제 멈춰야 하는지를 다룹니다.

커넥터 계약, 전체 능력 프로필, MCP 스키마, 접점별 cookbook은 여기서 정의하지 않습니다. 그런 내용은 [Agent 통합 참조](../reference/agent-integration.md)와 [Surface Cookbook](../reference/surface-cookbook.md)이 담당합니다.

이 문서는 Use 문서입니다. 재설계 문서가 승인되기 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture 파일, runtime data를 만들라는 뜻이 아닙니다. 첫 구현/증명 대상은 계속 Kernel Smoke입니다. Agency-Hardened MVP와 post-MVP automation은 owner 문서가 승격하고 증명하기 전까지 범위 밖입니다.

## 이런 때 읽기

에이전트가 상태, 막힘, 쓰기, 확인, 닫기를 사용자에게 어떻게 보여줘야 하는지 확인할 때 읽습니다.

## 읽기 전에

사용자 관점의 흐름을 먼저 보고 싶다면 [사용자 가이드](user-guide.md)를 먼저 읽습니다.

## 핵심 생각

사용자의 다음 판단에 영향을 주는 상태, 막힘, 판단, 다음 행동만 보여줍니다.

매 턴마다 계속 보여주는 맥락은 간결한 Harness 맥락 묶음(envelope)이어야 합니다. 여기에는 활성 Task id와 모드, 범위, 범위 밖, 다음 안전한 행동, 가장 먼저 해소할 막힘, 가장 작은 해소 방법, 활성 Change Unit 요약, 막고 있는 결정, 쓰기 권한 상태, 근거, 검증, Manual QA, 남은 위험, 보장 수준(guarantee level), gate 요약, 읽기용 보기 최신성(projection freshness)이 들어갑니다. 근거(Evidence), Run, Eval, Manual QA, artifact, log, screenshot, diff, large trace는 기본적으로 ref와 짧은 결과만 보여주고, 다음 행동이 내용을 실제로 살펴봐야 할 때만 가져옵니다.

## 세션 시작

Harness가 연결되어 있으면 사용자가 Harness 사용을 명시적으로 요청했을 때뿐 아니라 Harness가 추적해야 할 모양의 작업을 요청했을 때도 상태 확인이나 요청 정리로 시작합니다. 사용자가 꼭 "Harness"라고 말할 필요는 없습니다. 요청의 모양을 보고 판단하되, 첫 답변은 짧게 유지합니다.

일상적인 말로 들어온 요청도 범위, 판단, 근거, 닫기 상태가 계속 보여야 하는 모양이면 추적합니다.

- 제품 파일 쓰기나 프로젝트 상태를 바꾸는 작업
- 범위가 흐트러질 위험이나 모호한 요구사항
- 여러 파일 변경, 구조 변경, migration, 경계를 넘는 작업
- 인증(auth), 보안(security), 결제(billing), 파괴적 작업이나 데이터 손실 위험, 개인정보, 규정 준수처럼 민감하거나, 접근성(accessibility), 디자인 품질처럼 정책·품질 판단이 필요한 영역
- 사용자가 소유하는 제품 판단 또는 비용·호환성·보안·유지보수·migration·interface·dependency·위험 영향이 큰 중요한 기술 판단
- 근거, 검증, Manual QA, 수락, 남은 위험이 필요한 작업

작은 `direct` 작업은 가볍게 유지합니다. 질문에 답하거나, 코드를 살펴보거나, 결과를 설명하거나, 이미 좁은 모양이 분명한 작고 위험이 낮은 변경을 처리하는 데 불필요한 절차를 덧붙이지 않습니다.

보여줄 것:

- 활성 또는 예상 Task id와 모드: `advisor`, `direct`, `work`
- 현재 또는 제안 범위
- 범위 밖
- 다음 안전한 행동
- 진행을 막는 질문이 있다면 그 질문
- 가장 먼저 해소할 막힘, 다음 움직임의 소유자, 가장 작은 해소 방법
- 후속 경로에 계속 영향을 줄 때만 추가 막힘
- 쓰기가 가능하거나 가까울 때 쓰기 권한 상태
- 다음 결정이나 닫기 준비에 영향을 주는 경우 근거, 검증, Manual QA, 남은 위험, 수락 상태
- 보장 수준(guarantee level)과 접점이 실제로 막을 수 있는 것 또는 감지만 할 수 있는 것
- 간결한 gate와 읽기용 보기 최신성 상태
- guard, freeze, careful mode가 관련될 때 실행 전에 실제로 막을 수 있는 것과 실행 뒤에만 감지할 수 있는 것

넓은 자연어 요청만으로 바로 제품 파일을 쓰기 시작하면 안 됩니다. 먼저 범위와 의도한 변경에 맞는 쓰기 권한을 확정해야 합니다.

## 이어가기

중요한 작업을 이어가기 전에는 Harness 상태를 읽고 현재 위치를 보여줍니다. 상태를 읽을 수 있는데 오래된 채팅만 보고 권한이나 범위를 재구성하면 안 됩니다.

좋은 이어가기 답변:

```text
활성 작업을 찾았습니다. 현재 범위는 X입니다. 다음 안전한 행동은 Y입니다. 제품 파일 쓰기는 아직 허용되지 않았습니다. 대기 중인 결정은 Z 하나입니다.
```

Projection, `source_state_version`, 읽기용 상태가 stale이거나 unknown이면 그 사실을 말하고, 거기에 의존하기 전에 refresh 또는 reconcile합니다. 기준 상태를 직접 읽을 수 있으면 그 상태에서 계속할 수 있지만, 읽기용 projection은 권한의 출처가 아니라고 알려야 합니다.

표시 문제는 구분해서 말합니다. 오래된 projection(stale projection)은 읽기용 카드나 보고서가 뒤처졌을 수 있으므로 신뢰할 수 있는 맥락으로 쓰기 전에 refresh 또는 reconcile이 필요하다는 뜻입니다. 오래된 state, baseline, evidence는 실제 입력이 이동했거나 부족해져 쓰기나 닫기를 막을 수 있다는 뜻입니다. MCP에 닿지 못하는 상태(MCP unavailable)는 에이전트가 필요한 Harness/Core 기능에 닿지 못한다는 뜻입니다. 그 기능이 다시 사용 가능해지기 전에는 기준 상태 변경, Approval, 결과 수락, 남은 위험을 받아들이는 판단, gate 갱신, projection 복구, 닫기가 처리됐다고 주장하면 안 됩니다.

Core 자체에 닿을 수 없으면 표시 문제는 `MCP_SERVER_UNAVAILABLE`입니다. Core에 닿지 않는다고 말하고, 상태가 바뀌었다고 주장하기 전에 다시 연결하거나 진단합니다. Core 또는 operator가 현재 접점에서 MCP를 사용할 수 없다고 알 수 있으면 표시 문제는 `SURFACE_MCP_UNAVAILABLE`입니다. 이 접점이 필요한 Harness 도구를 사용할 수 없다고 말한 뒤, 제품 파일 쓰기는 지시로 보류하거나 필요한 기능을 가진 접점으로 전환합니다. 해당 작업에 실행 전 차단을 입증한 guard가 적용된 경우에만 실행 전에 차단됐다고 말합니다.

## 상태와 막힘 읽기

MCP 결과를 기준으로 삼되, 사용자에게는 이해하기 쉬운 말로 설명합니다.

정확한 오류 분류, 전체 대응표, 우선순위는 [MCP API와 스키마](../reference/mcp-api-and-schemas.md)가 담당합니다. 여기는 세션 응답에서 자주 쓰는 짧은 표시 예시만 제공하며, 전체 목록이 아닙니다.

- `harness.status`는 "지금 어디에 있는가?"라는 뜻입니다.
- `harness.next`는 "다음 안전한 행동 또는 가장 작은 해소 방법은 무엇인가?"라는 뜻입니다.
- `harness.prepare_write`는 "지금 이 정확한 제품 파일 쓰기를 해도 되는가?"라는 뜻입니다.
- `harness.record_run`은 "무슨 일이 일어났고, 어떤 근거가 바뀌었으며, 다음은 무엇인가?"라는 뜻입니다.
- `harness.close_task`는 "이 Task를 지금 끝내거나 취소할 수 있는가?"라는 뜻입니다.

응답에 오류나 막힘이 있으면 가장 먼저 해소할 막힘 하나를 먼저 말합니다. API precedence로 선택된 첫 `ToolError`를 쓰거나, `harness.close_task`가 blockers를 반환했다면 첫 close blocker를 사용합니다. 그다음 가장 작은 해소 방법을 평범한 말로 보여줍니다. 추가 막힘은 가장 먼저 해소할 막힘이 해소된 뒤에도 의미가 있을 때만 계속 보여줍니다.

모든 막힘 표시는 사용자에게 보이는 말로 소유자를 함께 말해야 합니다.

- 사용자 소유: 제품 방향, 중요한 기술 방향, sensitive-action Approval, Manual QA 판단 또는 waiver, 남은 위험을 받아들이는 판단, 최종 수락처럼 사용자가 결정해야 하는 일.
- 에이전트가 해소 가능: 상태 refresh 또는 reconcile, `prepare_write` 재시도, 빠진 근거 수집, 범위 안 check 실행, artifact 복구나 교체, 사용자 소유 판단을 바꾸지 않는 Change Unit 축소.
- 접점 또는 시스템: Core unavailable, surface MCP unavailable, capability insufficient처럼 재연결, 다른 접점, operator repair가 필요한 상태.

에이전트가 해소할 수 있는 막힘을 사용자에게 떠넘기면 안 됩니다. 그 행동이 범위를 바꾸거나 Approval을 요구하거나 새 사용자 소유 위험을 만들지 않는다면, 에이전트가 다음에 무엇을 할지 말합니다.

자주 쓰는 표시 예시:

| 원래 조건 | 먼저 말할 내용 | 가장 작은 해소 방법 |
|---|---|---|
| `STATE_CONFLICT` | 이 보기 이후 상태가 바뀌었습니다. | 상태를 새로 읽고 현재 state version으로 다시 시도합니다. |
| `MCP_UNAVAILABLE`(`details.mcp_unavailable_kind=server_unavailable`) 또는 진단상 `MCP_SERVER_UNAVAILABLE` | Core에 닿을 수 없습니다. | 기준 상태 변경을 주장하기 전에 Core 연결을 복구하거나 진단합니다. |
| `MCP_UNAVAILABLE` 또는 `CAPABILITY_INSUFFICIENT`(`details.mcp_unavailable_kind=surface_mcp_unavailable`) 또는 진단상 `SURFACE_MCP_UNAVAILABLE` | 이 접점은 필요한 Harness 도구를 사용할 수 없습니다. | 접점을 복구하거나 사용할 수 있는 접점으로 전환합니다. 실행을 막는 guard가 입증된 경우가 아니면 제품 파일 쓰기는 지시로 보류합니다. |
| 유용한 detail이 없는 `MCP_UNAVAILABLE` | Harness/Core 기능을 사용할 수 없습니다. | 기준 상태 변경을 주장하기 전에 다시 연결하거나 접점을 복구하거나 사용할 수 있는 접점으로 전환합니다. |
| `CAPABILITY_INSUFFICIENT` | 이 접점은 필요한 보장 수준을 제공할 수 없습니다. | 필요한 profile을 쓰거나, 작업을 줄이거나, 그 기능이 필요 없는 경로를 선택합니다. |
| `NO_ACTIVE_TASK` | 선택된 active Task가 없습니다. | 계속하기 전에 Task를 선택하거나 만듭니다. |
| `WRITE_AUTHORIZATION_REQUIRED` 또는 `WRITE_AUTHORIZATION_INVALID` | 쓰기 권한이 없거나 최신이 아닙니다. | 정확한 의도한 쓰기에 대해 `harness.prepare_write`를 다시 시도합니다. |
| `DECISION_REQUIRED` 또는 `DECISION_UNRESOLVED` | 사용자 판단이 필요합니다. | Decision Packet 또는 간결한 판단 요청을 보여줍니다. |
| `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, 또는 `APPROVAL_EXPIRED` | Sensitive-action Approval이 필요하거나 사용할 수 없습니다. | Approval을 요청, 해소, 갱신한 뒤 쓰기 확인을 다시 시도합니다. |
| `PROJECTION_STALE` | 읽기용 상태 보기가 오래됐습니다. | 그 보기에 의존하기 전에 projection을 refresh 또는 reconcile합니다. |
| `ARTIFACT_MISSING` | Artifact가 없거나 integrity check에 실패했습니다. | Artifact를 근거로 쓰기 전에 다시 첨부하거나, 생성하거나, 교체합니다. |

정확한 Harness 용어는 도움이 될 때만 괄호 안에 붙이고, 평범한 문장을 먼저 둡니다. 예: "쓰기 권한이 최신이 아닙니다(`WRITE_AUTHORIZATION_INVALID`). 가장 작은 해소 방법: 현재 파일 목록으로 `harness.prepare_write`를 다시 실행합니다."

## 요청 정리

요청 정리는 사용자가 Harness 용어로 말하지 않아도 평범한 요청을 실제로 진행 가능한 작업 모양으로 바꾸는 단계입니다.

세션 시작에서 보는 것과 같은 작업 모양 신호를 살핍니다. 제품 파일 쓰기, 범위가 흐트러질 위험, 모호한 요구사항, 여러 파일이나 구조 변경, 민감하거나 정책·품질 판단이 필요한 영역, 사용자가 판단해야 하는 결정, 근거, 검증, Manual QA, 수락, 남은 위험이 필요한 작업이 여기에 해당합니다. 이런 신호가 있으면 평범한 요청을 예상 모드, 범위, 범위 밖, 다음 안전한 행동으로 바꿔 제안합니다.

다음 안전한 행동을 바꾸는 질문만 합니다. 긴 양식보다 추천이 딸린 막힘 질문 하나가 낫습니다.

질문하기 전에는 에이전트가 안전하게 직접 확인할 수 있는 답을 사용 가능한 최신 저장소, 코드베이스, 문서, Harness state에서 먼저 찾아봅니다. 이미 보이는 파일 경로, 기존 동작, 용어, 제약을 사용자에게 다시 설명해 달라고 요구하지 않습니다. 소스가 없거나 오래됐으면 그것을 현재 사실의 근거로 삼지 말고, 불확실성으로 표시합니다.

한 번에 하나의 막힘 질문을 묻는다는 말이 구체화도 한 번이면 끝난다는 뜻은 아닙니다. 요청이 넓거나 설계 판단이 크면 목표, 범위, 비목표, 수용 기준, 영향받는 제품 영역, 사용자 화면이나 흐름, 모듈, interface, 민감 카테고리(sensitive categories), 사용자가 소유하는 제품 또는 중요한 기술 장단점 판단, 검증 또는 Manual QA 기대 수준, 알려진 제품·구현·검증·QA·후속 위험이 첫 번째 안전한 Change Unit을 제안할 수 있을 만큼 잡힐 때까지 짧은 확인을 여러 차례 이어갈 수 있습니다.

각 막힘 질문은 무엇이 불확실한지, 가능한 선택지, 에이전트의 추천안, 의미 있을 때 영향을 받는 gate 또는 수용 기준, source 또는 evidence refs, 결정을 미뤄도 계속할 수 있는 일 또는 결정 전에는 진행하면 안 되는 이유를 함께 보여줘야 합니다. 에이전트가 둔 가정은 사용자가 소유하는 선택, Approval, QA 판단, 수락, 남은 위험을 받아들이는 판단과 따로 기록합니다.

좋은 요청 정리:

```text
이 변경이 설정 화면 문구 안에 머물면 direct로 처리할 수 있습니다. 계정 동작까지 바꾸면 work로 전환해야 합니다. 추천은 설정 문구만 direct로 시작하는 것입니다. 의도한 범위가 맞나요?
```

## advisor/direct/work로 분류하기

`advisor`는 읽기, 설명, 비교, 리뷰처럼 제품 파일을 쓰지 않는 일에 씁니다.

`direct`는 작고 위험이 낮은 일을 좁은 범위에서 빠르게 처리할 때 씁니다. `direct`도 제품 파일을 쓰기 전에는 활성 범위와 쓰기 권한이 필요하지만, 절차는 가벼워야 합니다.

`work`는 기능 추가, 구조 변경, 위험한 수정, 여러 파일 변경, 요구가 불명확한 일, 의미 있는 근거와 독립 검증이 필요한 일에 씁니다.

`direct` 작업이 커지면 같은 Task를 `work`로 전환하고 이유를 보여줍니다.

## `direct` 절차 예산

`direct` mode는 가벼운 사용자 경험이지 더 낮은 권한 경로가 아닙니다. `direct` 작업에서는 사용자에게 보이는 내용을 가장 작은 유용한 묶음으로 유지합니다.

- 요청을 `direct`로 분류하고 좁은 범위를 말합니다.
- 관련 있을 때 범위 밖 동작, 파일, 결정을 이름 붙입니다.
- 제품 파일을 쓰기 전에 활성 최소 Change Unit을 만들거나 선택합니다.
- 정확한 쓰기 시도 전에 write authority를 보여줍니다.
- 변경 경로, 자체 확인 또는 다른 가벼운 근거, escalation 여부, 닫기에 영향을 주는 위험을 보고합니다.

Task 모양, policy, 변경된 표면, 감지된 위험, 사용자 요청 때문에 필요해진 경우가 아니라면 Decision Packet을 만들거나, Manual QA를 요구하거나, detached verification을 요청하거나, 전체 close checklist를 보여주지 않습니다.

대상이 더 이상 분명하지 않거나, 변경 경로가 활성 Change Unit을 넘거나, 여러 제품 영역에 영향을 주거나, public API 또는 module contract를 바꿀 수 있거나, 민감하거나 위험한 동작이 나타나거나, Manual QA 또는 detached verification이 중요해지거나, 사용자 소유의 제품 판단 또는 중요한 기술 판단이 필요하면 같은 Task를 `work`로 전환합니다.

## 범위와 Change Unit

제품 파일을 쓰기 전에 활성 범위를 Change Unit으로 구체화합니다. 사용자에게는 다음이 보여야 합니다.

- 포함되는 동작이나 파일
- 범위 밖 동작이나 파일
- 완료 조건
- 알려진 민감 영역
- 에이전트가 멈추고 물어야 하는 조건

첫 번째 안전한 Change Unit을 제안할 만큼 충분히 안다는 것은 위 항목들을 해소되지 않은 사용자 판단을 숨기지 않고 말할 수 있다는 뜻입니다. 아직 그러지 못하면 요청 정리를 이어가며 다음 막힘 질문을 하거나, 해소되지 않은 영역을 피하는 더 작은 Change Unit을 제안합니다.

Autonomy Boundary는 쓰기 권한이 아닙니다. 사용자가 다시 판단하지 않아도 에이전트가 어디까지 판단할 수 있는지만 설명합니다. Change Unit scope는 어디에서 무엇이 바뀔 수 있는지 답하고, Autonomy Boundary는 그 scope 안에서 에이전트가 어떤 선택을 혼자 할 수 있는지 답합니다. 실제 제품 쓰기에는 여전히 의도한 변경과 맞는 쓰기 확인이 필요합니다.

멈춤과 허가를 설명할 때는 다음처럼 구분합니다.

| 개념 | 쉬운 질문 | 허용하는 것 | 허용하지 않는 것 |
|---|---|---|---|
| Change Unit scope | 어떤 작업 영역이 범위 안인가? | 작업이 둘러싼 동작, 파일, paths, tools, commands, network targets, sensitive categories를 이름 붙입니다. | 사용자 소유의 제품 판단이나 중요한 기술 판단을 결정하거나 그 자체로 Write Authorization을 만들지 않습니다. |
| Autonomy Boundary | 그 범위 안에서 에이전트가 무엇을 혼자 판단해도 되는가? | 포괄된 구현 세부사항은 추가 사용자 결정 없이 에이전트가 선택할 수 있게 합니다. | Paths, tools, commands, network, secrets, sensitive categories, Approval, 쓰기 권한을 부여하지 않습니다. |
| Approval | 이 민감한 단계를 진행해도 되는가? | 기록된 scope와 expiry 안에서 이름 붙인 sensitive action을 허용합니다. | 사용자 소유 판단, 정확성(correctness), 남은 위험을 받아들이는 판단, Write Authorization을 대신하지 않습니다. |
| Decision Packet | 어떤 사용자 소유 판단을 기록하는가? | 이름 붙은 제품 판단, 중요한 기술 판단, waiver, acceptance, residual-risk, reconcile 선택을 resolved, deferred, rejected, blocked 상태로 기록합니다. | Approval record에 연결된 Approval 형태 packet이 아닌 한 sensitive-action Approval을 부여하지 않습니다. |
| Acceptance | 최종 수락이 required일 때 결과를 받아들일 수 있는가? | 닫기에 영향을 주는 남은 위험이 보였거나 없다고 확인된 뒤 사용자의 최종 결과 판단을 기록합니다. | Evidence, verification, Manual QA, Approval, Write Authorization, residual-risk acceptance를 대체하지 않습니다. |
| Residual-risk acceptance | 이번 close에서 알려진 남은 위험을 받아들일 수 있는가? | 보이는 close-relevant risk의 수용을 기록하며 다른 gate가 허용할 때 risk-accepted close를 뒷받침합니다. | Detached verification, correctness proof, QA pass를 만들지 않고, close를 위험 없는 일반 close로 만들지 않습니다. |
| Write Authorization | 지금 이 정확한 쓰기 시도(write attempt)를 해도 되는가? | 필요한 확인 뒤 Core가 compatible write attempt 하나를 허용했다는 기록입니다. | 재사용할 수 없고 scope, Autonomy Boundary, Approval을 넓히지 않습니다. |

작은 `direct` 작업에서는 활성 Change Unit을 사용자의 요청과 주변 맥락에서 만들 수 있습니다. 예시는 설명용이며 schema를 새로 정의하지 않습니다.

- 문서 또는 문구 수정: purpose "이 문구 변경"; non-goals "동작 또는 계약 변경 없음"; scoped paths "이름 붙은 문서/component와 직접 관련된 test가 있으면 그 test"; stop if "의미, localization strategy, public promise가 바뀜."
- 좁은 test 수정: purpose "보고된 case를 cover"; non-goals "implementation refactor 없음"; scoped paths "관련 test"; stop if "fix에 product code가 필요함."

프롬프트나 상태에서 "approved" 또는 "승인"이라는 말을 쓸 때는 실제 권한이나 기록되는 판단을 이름 붙입니다. Sensitive-action Approval, 범위 확인(scope confirmation), Decision Packet resolution, 남은 위험을 받아들이는 판단(residual-risk acceptance), 최종 수락(final acceptance), Write Authorization status를 구분해서 말하고, "승인"을 모든 허가와 판단을 뭉뚱그리는 포괄 라벨로 쓰면 안 됩니다.

예시:

- Dependency install Approval: install을 실행하거나 dependency 파일을 갱신해도 된다는 승인은 그 dependency가 올바른 architecture 선택이라는 결정이 아닙니다. 그 선택이 호환성, rollback, 비용, 유지보수에 영향을 주면 Decision Packet을 사용합니다.
- Secret access Approval: 요청된 scope 안에서 secret을 읽거나 사용해도 된다는 승인은 secret 값을 artifacts, projections, exports, logs, screenshots, summaries에 노출해도 된다는 뜻이 아닙니다.
- Auth/system change Approval: auth 파일, permission, system configuration을 만져도 된다는 승인은 session auth, JWT, social login, role model, lockout behavior, user notice를 선택하는 결정이 아닙니다.
- Public API change decision: API 방향이 해소됐다는 것은 이 Task의 contract 선택을 결정했다는 뜻입니다. Deployment 권한, merge 권한, 재사용 가능한 Write Authorization이 아닙니다.
- 최종 수락(Final acceptance): 결과 수락은 추가 쓰기를 허가하거나, 새 민감 동작을 승인하거나, 빠진 근거, QA, verification, Write Authorization을 나중에 충족시켜 주지 않습니다.

Autonomy Boundary 안에서는 에이전트가 기존 helper를 재사용할지, private function을 어떻게 나눌지, focused tests를 어디에 둘지, 합의된 결과에 맞는 보수적인 내부 접근을 고를지 같은 일상적인 구현 세부사항을 판단할 수 있습니다. Public API 또는 module contract 변경, security 또는 privacy trade-off, UX 또는 제품 trade-off, dependency나 migration 같은 중요한 기술 방향 선택, scope expansion, 남은 위험을 받아들이는 판단 전에는 사용자 판단을 위해 멈춰야 합니다.

## 사용자 판단으로 막힐 때

사용자가 소유하는 제품 판단이나 중요한 기술 판단이 진행을 막고 있으면 Decision Packet을 보여주거나 요청합니다. 이를 넓은 승인 질문이나 막연한 "계속할까요?"로 바꾸면 안 됩니다.

"approved", "승인", "go ahead", "진행해" 같은 말은 underlying choice가 제품 장단점, architecture 방향, QA waiver, verification risk, final acceptance, residual-risk acceptance라면 충분하지 않습니다. Prompt는 decision route, 사용자가 결정하는 것, 결정하지 않는 것, evidence 또는 risk refs, 에이전트가 사용자 없이 결정해도 되는 일, close나 write 영향을 이름 붙여야 합니다.

사용자가 보는 Decision Packet에는 다음이 있어야 합니다.

- 왜 지금 이 결정이 필요한지
- 정확히 무엇을 결정해야 하는지
- 옵션과 장단점
- 추천과 불확실성
- 영향을 받는 gate와 수용 기준
- source refs와 evidence refs
- 결정을 미루면 계속할 수 있는 일, 또는 결정 전에는 진행하면 안 되는 이유
- 에이전트가 사용자 없이 결정해도 되는 일
- 남는 위험이나 후속 작업

정확한 public field는 [`harness.request_user_decision`](../reference/mcp-api-and-schemas.md#harnessrequest_user_decision)이 소유하고, 기준 authority는 [Decision Packet](../reference/kernel.md#decision-packet)과 [Decision Gate](../reference/kernel.md#decision-gate)가 소유합니다. 사용자 prompt에 schema body를 복사하지 말고, 쉬운 말로 결정을 렌더링한 뒤 필요한 refs를 drill-down할 수 있게 둡니다.

Decision-centered prompt는 경로와 맞는 동사를 씁니다. 선택, defer, reject, waive, accept, reconcile입니다. "approve" 또는 "승인"은 sensitive-action Approval일 때만 사용합니다. 좋은 prompt 형태는 다음과 같습니다.

```text
이 Change Unit에 어떤 로그인 실패 UX를 기록할까요: 인라인 메시지, 토스트, 모달/레이어? 추천은 흐름과 접근성을 유지하는 인라인 메시지입니다. 결정을 미루면 backend auth wiring은 계속할 수 있지만 최종 로그인 실패 UX가 완료됐다고 말할 수는 없습니다.
```

```text
이번 close에서 남은 mobile Safari wrapping risk를 받아들이겠다고 기록할까요, 아니면 Manual QA를 실행할 때까지 close를 막아 둘까요? 추천은 release timing 때문에 waiver가 필요한 경우가 아니라면 막아 두는 것입니다. 영향받는 gate: qa_gate; 영향받는 기준: AC-03 onboarding copy layout.
```

유용한 예시:

- 로그인 실패 UX: 인라인 메시지, 토스트, 모달/레이어를 비교하고, 흐름, 접근성, 방해 정도, 문구 위험을 기준으로 하나를 추천합니다. 결정을 미루면 backend auth 작업은 계속할 수 있지만 최종 로그인 실패 경험이 완료됐다고 말하면 안 됩니다.
- 로그인 실패 문구: 일반적인 문구, 더 구체적인 문구, hybrid 문구를 비교합니다. 계정 열거(account enumeration) 위험, 명확성, 복구 도움 정도, 지원 부담, 제품 톤을 기준으로 추천합니다. 결정을 미루면 validation wiring은 계속할 수 있지만 release-ready copy와 Manual QA는 열어 둬야 합니다.
- 제품 감각과 Manual QA 필요성: 사람이 시각적으로 확인해야 하는 완성도 높은 상호작용과 test 및 browser smoke로 확인 가능한 더 보수적인 동작을 비교합니다. 감각상의 장단점, QA 비용, 사용자 영향, Manual QA를 미뤘을 때 계속할 수 있는 일 또는 결정 전에는 진행하면 안 되는 이유를 설명합니다.
- Auth 방식: session cookie, JWT, social login을 비교하고, 폐기 가능성, CSRF/XSS 노출, client 호환성, 운영 복잡도, migration 비용을 설명합니다. 결정을 미루면 session model에 약속을 만들지 않는 범위에서만 form scaffold를 계속할 수 있습니다.
- Dependency 선택: install 또는 dependency 파일 갱신을 허용하는 Approval과, 그 dependency를 채택하는 architecture decision을 분리합니다. Dependency를 추가할지, 기존 utility를 쓸지, capability를 미룰지 비교하고 호환성, rollback, 비용, 유지보수 영향을 설명합니다.
- Schema/data-model migration: additive migration, compatibility shim, breaking cleanup을 비교합니다. Migration evidence, data-backfill risk, rollback path, test boundary, 유지보수 비용을 설명합니다.
- Public API/interface 또는 module boundary: 현재 interface를 유지할지, 좁은 extension을 추가할지, 책임을 module boundary 너머로 옮길지 비교합니다. Caller 영향, compatibility 또는 breaking-change risk, boundary test, documentation promise, migration path, future-change cost를 설명합니다.
- Scope 또는 Autonomy Boundary 확장: current small scope를 유지할지, requested surface를 추가할지, follow-up Change Unit으로 분리할지 비교합니다. 영향을 받는 paths, user-facing behavior, 계속 범위 밖에 남는 것, write 영향, agent가 혼자 판단해도 되는 일을 설명합니다.
- 보안 민감 변경: secret 접근, 권한 변경, 데이터 export에 대한 Approval은 Approval 경계일 뿐입니다. 역할, 필드, redaction, audit logging, retention, rollback, user notice에는 별도의 제품 또는 보안 판단이 여전히 필요할 수 있습니다.
- QA 또는 verification waiver: 해당 Task에서 요구하는 기존 기록 방식을 사용합니다. QA waiver는 Manual QA/gate 상태와 `qa_gate=waived`로 기록하고, product/user risk 또는 policy-required judgment가 있으면 QA waiver Decision Packet을 사용합니다. Verification waiver는 `verification_gate=waived_by_user`로 기록하고, 사용자 소유 판단이 필요하면 관련 Decision Packet을 사용합니다. 생략하는 확인이나 대상, 받아들이는 위험, 후속 작업, 관련 refs, 닫기 영향을 이름 붙입니다. 예를 들어 copy-only 변경에서 mobile Safari Manual QA를 면제한다면 viewport wrapping 위험을 받아들이고 release 전 browser pass를 후속 작업으로 남깁니다.
- 닫기 전 남은 위험을 받아들이는 판단: 남은 한계, 이미 있는 근거, 그래도 닫을 수 있다고 볼 수 있는 이유, 남는 후속 작업을 보여줍니다.

가능하면 한 번에 하나의 막힘 질문만 묻습니다.

## 검토 관점과 표시

사용자가 product, engineering, design, security, QA, release-handoff 관점으로 봐 달라고 하면 `product-review`, `eng-review`, `design-review`, `security-review`, `qa-review`, `release-handoff`를 Role Lens 또는 권장 playbook 표시로 다룹니다. 라벨은 검토 관점을 고를 뿐이며 새 mode, Approval, Write Authorization, gate, close path가 아닙니다. 정확한 Role Lens 경계는 [Agent Integration](../reference/agent-integration.md#role-lens-동작)이 담당합니다.

검토 결과에서는 두 질문을 분리합니다.

- Spec Compliance Review: 현재 scope와 권한 안에서 요청한 것을 만들었는가?
- Code Quality / Stewardship Review: 결과가 codebase 안에서 유지보수 가능하고 일관적인가?

같은 세션에서 하는 검토(review)는 조건을 충족하는 independent Eval 또는 verification record가 없는 한 자체 확인(self-check) 또는 stewardship signal입니다. Decision Packet 후보, 근거 부족, Eval 또는 검증 필요, Manual QA 필요, Residual Risk 후보, Approval 필요, Change Unit 업데이트 추천, close blocker를 찾을 수는 있지만, 영향받는 write나 Task 닫기가 진행되기 전에 이런 발견 사항은 기존 경로로 연결해야 합니다.

## AFK 작업과 public commitment

사용자가 자리를 비운 동안 계속하라고 했더라도, 그것은 이미 기록된 latitude를 쓰라는 뜻이지 새 권한을 만든다는 뜻이 아닙니다. 에이전트는 active Change Unit, active Autonomy Boundary, granted sensitive approvals, 각 제품 파일 쓰기에 맞는 `prepare_write` / Write Authorization 안에서만 계속할 수 있습니다.

Scope expansion, Approval 없는 새 sensitive action, Autonomy Boundary breach, residual-risk acceptance, final acceptance, QA 또는 verification waiver, public API 또는 module contract 변경, release/support promise, 사용자나 다른 시스템이 의존할 수 있는 다른 public commitment 전에는 멈추고 가장 작은 unblocker를 보여줘야 합니다.

AFK stop을 보여줄 때는 guarantee level을 이름 붙입니다. Cooperative 또는 detective surface에서 "멈춤"은 지시에 따른 보류이거나, profile이 지원할 때 실행 뒤 감지 및 보고가 가능하다는 뜻입니다. 해당 operation에 대해 연결된 profile이 실행 전 차단을 입증한 경우에만 preventive wording을 씁니다.

## 제품 파일 쓰기

제품 파일을 쓰기 전에는 에이전트가 의도한 작업에 대한 쓰기 권한을 확인해야 합니다.

짧은 쓰기 권한 요약을 보여줍니다.

```text
쓰기 권한: src/auth/login.ts와 tests/auth/login.test.ts에 허용됨
범위 근거: email login Change Unit
한계: 협조형 접점이라서 범위를 벗어난 쓰기는 사후 changed-path validation으로만 감지합니다.
```

외부 영향(side effect)이 있을 때는 실행 전의 주장과 실행 뒤의 기록을 분리합니다. 실행 전에는 의도한 영향, 민감 범주(sensitive category), Approval 또는 Decision Packet 필요 여부, 보장 수준을 말합니다. 실행 뒤에는 실제로 일어난 일, 기록된 Run/artifact/evidence ref, redaction/omission/block/stale/violation 여부를 말합니다. 정확한 보장 수준 의미는 [런타임 아키텍처 참조](../reference/runtime-architecture.md#보장-수준)가 담당합니다.

Cooperative 또는 detective hold를 실행 전에 막는 것처럼 설명하면 안 됩니다. 지시로 쓰기를 보류한다고 말하거나, 연결된 profile이 해당 validation을 지원할 때 실행 뒤에 위반을 감지할 수 있다고 말합니다. 실행 전 차단(preventive) 표현은 해당 operation에 대해 입증된 실행 전 차단이 있을 때만 씁니다.

쓰기 권한이 막혔거나, 확인할 수 없거나, 최신이 아니거나, 의도한 변경과 맞지 않으면 제품 파일 쓰기를 멈추고 가장 작은 해소 방법을 설명합니다.

관찰된 변경 경로가 사용된 Write Authorization 또는 active Change Unit 밖이면, 승인된 작업처럼 요약하지 않습니다. 불일치를 보여주고 추가 제품 파일 쓰기를 멈춘 뒤 복구로 연결합니다. 추가 변경을 되돌리거나 분리할지, 범위 결정을 요청할지, 더 넓은 변경이 의도된 것이라면 `work`로 전환할지 선택해야 합니다.

문서 유지보수 편집은 별도의 docs-only 흐름입니다. 이 문서의 제품 파일 쓰기 흐름이 아니라
[문서 작성 가이드](../maintain/authoring-guide.md)가 다룹니다.

## 근거와 확인

조언, 변경, 실행, 리뷰 뒤에는 결과를 필요한 수준으로 기록합니다. 사용자가 보는 근거는 수용 기준이나 명시된 작업 목표와 연결되어야 합니다.

Sufficiency는 양이 아니라 coverage로 표시합니다. 중요한 질문은 어떤 수용 기준, completion condition, close-relevant claim에 current supporting refs가 있는지입니다. 긴 artifact list는 missing criterion을 supported로 만들지 않으며, chat text나 Markdown report prose만으로 evidence가 sufficient하다고 증명하면 안 됩니다.

좋은 근거 표시:

```text
근거:
- AC-01: 이메일 필드가 있는 로그인 폼 렌더링을 RUN-008 테스트 결과가 뒷받침합니다.
- AC-02: 로그인 실패 메시지는 RUN-009와 ART-TEST-009가 뒷받침합니다. 최종 문구는 아직 Manual QA가 필요합니다.
```

근거가 부족하면 어떤 기준이나 주장이 뒷받침되지 않는지 말합니다. 단순히 "근거 게이트가 실패했습니다"라고만 말하지 않습니다.

근거 표시는 먼저 참조를 보여주는 방식(refs-first)으로 합니다. Evidence, Run, Eval, Manual QA, artifact, log, screenshot, diff, trace ref와 짧은 결과를 보여주고, 사용자나 evaluator가 다음 행동을 결정하기 위해 내용을 살펴봐야 할 때만 excerpt를 본문에 넣습니다.

Task shape에 따라 "충분함"의 모습은 달라집니다. Advisor work는 recorded evidence가 요청된 경우에만 보통 source refs 또는 review bundle을 cite합니다. Direct docs-only work는 changed path, diff 또는 patch summary, self-check로 뒷받침될 수 있습니다. Direct code는 focused check 또는 automated check가 적용되지 않는다는 recorded reason을 더합니다. Feature work는 각 criterion을 Run과 artifact refs에 map합니다. UI/UX/copy work는 visual evidence와 Manual QA를 분리합니다. Sensitive work는 Approval, redaction, omission refs를 visible하게 유지하지만 Approval을 correctness로 취급하지 않습니다. Verification-required work에는 reviewed evidence를 이름 붙이는 Eval이 필요합니다.

Evidence가 stale이 되면 이유를 쉬운 말로 말하고 가장 작은 repair를 이름 붙입니다. 흔한 원인은 baseline drift, supporting Run 또는 Eval 이후 changed files 변경, approval drift 또는 expiry, missing 또는 failed-integrity artifacts, relevant Shared Design, domain term, module map, interface contract changes입니다.

## 검증, Manual QA, 남은 위험, 수락

에이전트 답변에서는 이 항목들을 분리해서 보여줘야 합니다.

| 항목 | 사용자가 이해해야 하는 것 |
|---|---|
| 근거 | 결과나 수용 기준이 충족됐다는 주장을 무엇이 뒷받침하는가. |
| 검증 | 정확성(correctness)을 무엇이 확인했고, 그 검증자(verifier)가 독립 보증(detached assurance)에 충분히 독립적이었는가. |
| Manual QA | 사람이 봐야 하는 품질을 무엇으로 확인했는가. |
| 수락 | 그런 판단이 요구되는 경우 사용자가 결과를 받아들이는가. |
| 남은 위험 | 어떤 불확실성, 한계, 확인하지 못한 조건, 장단점이 남았는가. |

검증은 기술적으로 무엇을 어떻게 확인했는지에 답합니다. 같은 세션에서 하는 자체 검토는 유용할 수 있지만, 분리된 검증은 아닙니다. 테스트 통과는 근거가 될 수 있고 검증을 뒷받침할 수 있지만, 테스트만으로 Manual QA가 수행됐다고 말하면 안 됩니다. Detached candidate는 valid independence와 current reviewed inputs가 있는 passing Eval이 기록된 뒤에만 detached verified가 됩니다.

사용자 표시 label은 일관되게 사용합니다.

| Label | 사용할 때 |
|---|---|
| Self-checked | 구현 경로가 자기 결과를 확인했을 때. |
| Detached candidate | Fresh session, fresh worktree, sandbox, manual bundle, 또는 qualifying subagent path가 독립적일 수 있지만 아직 detached assurance를 만들지 않았을 때. |
| Detached verified | Eval이 valid independence, same-session self-review 문제 없음, stale baseline 또는 bundle input 없음으로 pass했을 때. |
| Waived with accepted risk | Verification 또는 다른 close-relevant check가 waived되었고 보이는 remaining risk가 risk-accepted close를 위해 accepted되었을 때. |

Manual QA는 UX, 흐름, 시각 결과, 문구, 접근성 해석처럼 사람이 봐야 하는 품질을 확인했는지에 답합니다. Manual QA 결과가 실제로 기록되었거나 타당하게 면제된 것이 아니라면 browser smoke, screenshot capture, verifier note를 Manual QA처럼 보여주면 안 됩니다.

남은 위험은 알려진 한계, 불확실성, 확인하지 못한 조건, 장단점입니다. 위험을 받아들이고 닫거나 최종 수락을 하기 전에는 반드시 보여야 합니다. 남은 위험을 받아들이는 판단은 보장 수준을 높이지 않고, verification이나 QA를 대체하지 않습니다.

최종 수락은 Task 경로가 요구할 때 사용자가 결과를 받아들이는 판단입니다. 승인, 검증, QA, 남은 위험을 받아들이는 판단, 정확성 증명과 다릅니다.

Verification waiver와 QA waiver는 assurance를 높이지 않습니다. Verification waiver는 detached verification을 충족하지 않은 상태로 두며, close가 otherwise 허용될 때 accepted verification risk 경로로 닫습니다. 이를 `completed_verified`로 요약하면 안 됩니다. QA waiver는 이름 붙인 QA requirement만 닫고 evidence, verification, acceptance, residual-risk 처리는 각각의 gate에 그대로 남깁니다.

닫기 적용 예시:

- Direct 작업: 변경 파일, 근거 refs, 자체 확인(self-check), `work`로 전환됐는지 여부를 보여줍니다. 조건을 충족하는 Eval 없이 detached verified라고 부르면 안 됩니다.
- UI/UX 작업: 테스트, browser smoke, Manual QA, 수락을 각 줄로 분리합니다. Manual QA를 면제한다면 생략한 대상, 받아들이는 위험, 후속 작업을 보여줍니다.
- Auth 또는 security 작업: Approval을 security 또는 product decision과 분리해서 보여준 뒤 근거와 검증을 보여줍니다. Secret이나 permission을 만지는 Approval은 redaction, audit, role, retention, user notice 선택을 대신하지 않습니다.
- Public API 작업: caller compatibility, migration 또는 documentation 영향, 근거, 검증을 따로 보여줍니다. 테스트 통과만으로 API contract 결정이 끝난 것은 아닙니다.
- 위험을 받아들이고 닫기: 남은 한계, 이미 있는 근거, 빠졌거나 면제된 verification 또는 QA, 받아들인 위험, 후속 작업을 보여줍니다. 결과를 detached verified처럼 표시하면 안 됩니다.

## 닫기

닫기는 활성 Task 경로의 막힘이 사라졌을 때만 합니다.

작은 `direct` 작업은 결과를 가볍게 보여줍니다. 요청, 범위, 변경된 파일 또는 파일 변경이 없었다는 결과, 확인, 전환 여부, 닫기에 영향을 주는 위험이나 후속 작업 정도면 충분합니다.

`work` Task의 닫기 요약은 닫을 수 있는 근거를 보여줘야 합니다. 적용된 변경 범위, 근거 범위, 검증, Manual QA, 남은 위험, 수락, close reason을 해당되는 만큼 표시합니다. Gate가 waived, `not_required`, failed, pending, blocked 중 하나라면 일반적인 성공 문장에 묻지 말고 그대로 말해야 합니다.

작업 모양에 맞는 닫기 표시를 사용합니다. `DIRECT-RESULT`는 direct 작업의 간결한 결과 표시이고, `TASK` Close Summary는 진행 중이거나 최근 닫힌 `work` Task의 이어가기 표시이며, Journey Card close context는 compact status/resume 표시입니다. 이 표시들은 state, gate, 수락, QA, verification, 남은 위험을 받아들이는 판단, close, write authority를 만들지 않습니다.

성공적으로 닫기 전에 보여주거나 확인할 것:

- 범위와 결과가 맞는지
- 근거가 충분한지 또는 근거 요구가 없는지
- stale evaluator bundle 또는 baseline drift가 있으면 이를 포함한 검증 상태, 또는 필요한 경우 관련 Decision Packet과 accepted risk refs가 있는 `verification_gate=waived_by_user`
- Manual QA 상태 또는 타당한 면제
- 닫기에 영향을 주는 남은 위험 표시 또는 알려진 닫기 관련 남은 위험(close-relevant residual risk)이 없다는 보고
- 필요한 경우 최종 수락 기록

일반적인 "다 괜찮나요?" 같은 prompt로 최종 수락을 요청하지 않습니다. 최종 수락 prompt는 close basis, 즉 evidence, verification, Manual QA, residual-risk visibility 또는 `none`, acceptance가 대체하지 않는 것, 요청하는 close가 normal close인지 risk-accepted close인지를 보여줘야 합니다.

닫기가 막혔으면 정확한 이유와 가장 작은 해소 방법을 말합니다.

## 좋은/나쁜 에이전트 응답 예시

좋은 시작:

```text
Harness 상태로 가볍게 추적할 작은 작업으로 보입니다. 요청은 문구 하나만 바꾸는 일이라 direct로 보입니다. 범위는 설정 화면 문구 하나입니다. 범위 밖은 계정 동작과 API 변경입니다. 다음 안전한 행동은 해당 파일의 쓰기 권한을 확인한 뒤 수정하고 자체 확인하는 것입니다.
```

나쁜 시작:

```text
승인되었습니다. 필요한 것은 전부 수정하고 Task를 닫겠습니다.
```

좋은 결정 멈춤:

```text
사용자가 소유하는 제품 판단 때문에 막혔습니다. AC-02의 빈 상태 동작을 선택해야 합니다. 옵션은 바로 설정을 유도하거나 데이터가 생길 때까지 조용히 두는 것입니다. 추천은 설정 유도이며 불확실성은 낮습니다. 결정을 미루면 데이터 로딩 리팩터링은 끝낼 수 있지만 빈 상태 UX가 완료됐다고 말할 수는 없습니다.
```

나쁜 결정 멈춤:

```text
계속하려면 승인해주세요.
```

좋은 승인 요약:

```text
승인된 것: CU-03 안에서 zod를 install하고 package lock을 갱신하는 일. 아직 결정되지 않은 것: validation을 public API의 일부로 만들지 여부. 다음 안전한 행동: 정확한 dependency 파일 쓰기에 대해 `prepare_write`를 다시 시도합니다.
```

나쁜 승인 요약:

```text
승인됐으니 auth 설계를 끝내고, 구현하고, 배포하고, Task를 닫겠습니다.
```

좋은 닫기 막힘:

```text
온보딩 문구에 대한 사용자 소유 Manual QA 판단 때문에 닫기가 막혀 있습니다. 가장 작은 해소 방법은 문구를 받아들일 수 있는지 기록하는 것입니다. 추가 막힘: AC-02 근거가 아직 없으며, Manual QA 뒤에 에이전트가 수집할 수 있습니다.
```

나쁜 닫기 막힘:

```text
상태 모델이 게이트를 만족하지 않습니다.
```

나쁜 닫기 주장:

```text
테스트가 통과했으니 Manual QA와 수락도 끝났습니다.
```

좋은 direct 결과:

```text
direct로 완료했습니다. 범위는 설정 레이블 하나였고 계정 동작은 범위 밖으로 유지했습니다. `src/settings/Profile.tsx`를 변경했습니다. 기존 copy test로 자체 확인(self-check)이 통과했습니다. `work` 전환은 없고, 알려진 닫기 관련 남은 위험도 없습니다.
```

좋은 work 닫기 요약:

```text
닫기 요약: 변경 범위는 로그인 폼, 로그인 API 호출, 세션 저장 안에 머물렀습니다. 근거는 RUN-018과 EVIDENCE-009로 AC-01과 AC-02를 뒷받침합니다. 검증은 자체 확인(self-check)이며, 이 경로에서는 detached Eval이 필요하지 않았습니다. Manual QA는 최종 문구와 레이아웃에 대해 통과했습니다. 남은 위험: mobile Safari는 확인하지 않았고, DEC-022에서 후속 작업과 함께 받아들였습니다. 최종 수락이 기록되었습니다. close reason: completed with accepted residual risk.
```

좋은 쓰기 멈춤:

```text
새 auth 파일에 대한 쓰기 권한이 없어 제품 파일 쓰기를 멈춥니다. 가장 작은 해소 방법은 범위에 해당 파일을 포함하도록 업데이트하거나 변경을 기존 login module 안으로 줄이는 것입니다.
```

나쁜 쓰기 멈춤:

```text
Harness가 불만이라 멈췄습니다.
```

좋은 MCP unavailable 보류:

```text
이 접점(surface)에서 Harness/Core에 닿을 수 없어 제품 파일 쓰기를 보류합니다. 상태 변경, gate 갱신, Approval, 결과 수락, 남은 위험을 받아들이는 판단, 닫기가 처리됐다고 주장하지 않겠습니다. 가장 작은 해소 방법은 Core를 다시 연결하거나 가능한 접점(surface)에서 계속하는 것입니다.
```

좋은 오래된 projection 복구:

```text
Journey Card는 stale이지만 Core state는 current입니다. 오래된 Markdown을 권한처럼 쓰지 않고, 읽기용 보기를 refresh 또는 reconcile한 뒤 Core state에서 계속하겠습니다.
```

좋은 관리 영역(managed block) 응답:

```text
생성된 report의 관리 영역(managed block)을 사람이 직접 편집했습니다. 이 편집은 reconcile proposal로 다루겠습니다. Reconcile 또는 다른 Core 상태 변경 경로가 기록하기 전까지는 state가 아닙니다.
```

좋은 검토 경계:

```text
같은 세션 검토(review)에서 stewardship concern 하나를 찾았고 scope mismatch는 없었습니다. 이것은 자체 확인 신호(self-check signal)이지 detached verification이 아닙니다. Detached verification에는 조건을 충족하는 Eval 또는 독립 review boundary가 필요합니다.
```
