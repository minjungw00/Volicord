# 핵심 개념

## 이 문서로 할 수 있는 일

이 문서는 Harness 참고 사양을 읽기 전에 필요한 가장 작은 개념 묶음을 소개합니다. 각 개념은 먼저 쉬운 예시로 시작하고, 그 뒤에 조금 더 엄밀한 설명을 붙입니다.

커널, 런타임, MCP API, 문서 Projection 참조는 이제 reference 경로에 있습니다.

이 문서는 Learn 문서입니다. 문서 세트가 구현 계획에 사용할 수 있다고 승인되기 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture 파일, runtime data를 만들라는 뜻이 아닙니다. 첫 구현/증명 대상은 계속 Kernel Smoke입니다. Agency-Hardened MVP와 post-MVP automation은 owner 문서가 승격하고 증명하기 전까지 범위 밖입니다.

## 이런 때 읽기

예시, 상태 요약, 참고 사양에서 Harness 용어가 보이기 시작했고 가장 작은 어휘부터 잡고 싶을 때 읽습니다.

## 읽기 전에

[개요](overview.md)를 먼저 읽으면 좋습니다. 스키마나 구현 지식은 필요하지 않습니다.

## 핵심 생각

Harness의 어휘는 요청, 범위, 판단, 근거, 확인, 수락, 남은 위험, 닫기로 이어지는 작은 작업 흐름에 이름을 붙입니다.

## 가장 작은 개념 묶음

Harness는 작업 흐름에서 시작하면 이해하기 쉽습니다.

- 사용자가 Task를 요청합니다.
- 작업에 구체화가 필요하면 Shared Design이 목표, 범위, 가정, 첫 번째 안전한 모양을 기록합니다.
- 제품 파일 쓰기는 Change Unit 안에서 일어납니다.
- 중요한 주장은 근거로 남깁니다.
- 민감한 행동에는 Approval이 필요하고, 제품 파일을 쓰기 전에는 Write Authorization이 필요합니다.
- 확인 결과는 검증으로 남고, 사람이 직접 봐야 하는 부분은 Manual QA가 될 수 있습니다.
- Task 경로가 요구하면 사용자가 결과를 수락합니다.
- 남은 불확실성은 남은 위험으로 기록합니다.
- 읽기용 문서는 Projection이고, 사람이 문서를 고친 내용은 Reconcile을 거쳐야 상태가 됩니다.

이 문서에서는 일부러 개념을 작게 다룹니다. 엄격한 커널 정의는 [커널 참조](../reference/kernel.md)에 있고, 공개 API 정의는 [MCP API와 스키마](../reference/mcp-api-and-schemas.md)에 있으며, Projection 규칙은 [문서 Projection 참조](../reference/document-projection.md)에 있습니다. 여러 참조 문서에 걸치는 개념은 아래에서 owner map 전체를 반복하지 않고 짧게 경계만 이름 붙입니다.

## Task

사용자가 "이메일 로그인을 추가하고 비밀번호가 틀렸을 때 도움이 되는 오류를 보여 주세요"라고 말합니다. 대화는 여러 번 오갈 수 있지만, 오래 남는 단위 하나가 필요합니다. 그 단위는 사용자가 원하는 일이 무엇인지, 지금 작업이 어떤 상태인지 말해 줍니다.

Task는 사용자가 얻고 싶은 가치의 단위입니다. 완료하고 싶은 일, 답을 얻고 싶은 질문, 조사하거나 결정하고 싶은 대상이 될 수 있습니다. Harness는 Task를 중심으로 상태, 다음 행동, 막힌 지점, 근거, QA, 수락, 닫기 판단을 연결합니다.

참조: [커널 참조](../reference/kernel.md).

## Shared Design

사용자가 "onboarding을 더 좋게" 해 달라고 합니다. Implementation이 plan으로 굳어지기 전에 agent와 user는 목표, 비목표, 수용 기준, assumptions, 영향받는 화면, domain term, module 또는 interface impact, 첫 번째 안전한 slice에 대한 shared understanding이 필요합니다.

Shared Design은 그 이해를 기록한 것입니다. 흐릿한 작업을 안전한 첫 Change Unit으로 바꾸는 데 도움을 주지만 Approval, Write Authorization, final acceptance, QA judgment, residual-risk acceptance는 아닙니다.

구체화 중에 public API direction, domain-language meaning, module boundary movement, architecture direction, known-risk waiver처럼 사용자가 소유하는 선택이 드러나면 그 선택은 Decision Packet으로 라우팅합니다. Shared Design은 그 결정을 가리킬 수 있지만, 그 자체로 결정하지는 않습니다.

참조: [Shared Design](../reference/glossary.md#shared-design), [설계 품질 정책](../reference/design-quality-policies.md#shared-design-shared_design).

## Change Unit

이메일 로그인 작업에는 로그인 폼, API 호출 하나, 세션 처리 변경이 필요할 수 있습니다. 이것은 제한된 한 조각입니다. 작업이 갑자기 인증 시스템 전체 재작성으로 커진다면 범위가 바뀐 것이고, 그 사실이 보여야 합니다.

Change Unit은 Task에서 제품 파일을 쓸 수 있는 제한된 범위입니다. 어느 부분을 바꿀 수 있는지 이름 붙여서, 에이전트와 사용자와 Harness가 쓰기가 합의된 작업 안에 있는지 판단할 수 있게 합니다.

아주 작은 문서, 문구, 좁은 test 수정에서는 Change Unit이 요청에서 생성될 수 있고 매우 작게 유지될 수 있습니다. 핵심은 같습니다. `direct` 작업은 가벼울 수 있지만, 제품 파일 쓰기는 여전히 활성 scope 안에서 일어납니다.

참조: [커널 참조](../reference/kernel.md).

## Autonomy Boundary

이메일 로그인 Change Unit 안에서 에이전트는 기존 helper를 재사용할지, private function을 나눌지, focused test를 추가할지 같은 판단을 다시 묻지 않고 할 수 있습니다. 하지만 JWT를 쓸지, public API를 바꿀지, security trade-off를 받아들일지는 전혀 다른 판단입니다.

Autonomy Boundary는 Change Unit 안에서 에이전트가 행사할 수 있는 판단을 설명합니다. Change Unit scope는 "어떤 작업 표면이 바뀔 수 있는가?"에 답하고, Autonomy Boundary는 "그 안에서 에이전트가 어떤 선택을 다시 묻지 않고 할 수 있는가?"에 답합니다. 둘 다 Write Authorization은 아닙니다.

참조: [커널 참조](../reference/kernel.md).

## Decision Packet

에이전트가 실패한 로그인에 대해 여러 가지 괜찮은 선택지를 찾았습니다. 상호작용은 inline message, toast, modal/layer 중 하나일 수 있고, 문구는 일반적인 문구, 더 구체적인 문구, hybrid 문구 중 하나일 수 있습니다. 다른 Task라면 session cookie, JWT, social login 중에서 고르거나, 호환되는 public API extension과 breaking cleanup 중에서 골라야 할 수도 있습니다. 이런 선택이 진행을 막는 제품, 보안, 호환성, 유지보수 판단이라면 에이전트가 조용히 골라서는 안 됩니다.

Decision Packet은 특정 사용자 소유 판단이 진행, 쓰기, 닫기, 예외 허용, 결과 수락, 남은 위험을 받아들이는 판단, 제품 방향, 중요한 기술 방향, 범위, 설계 절충, 코드베이스 돌봄 판단, 공개 약속을 막을 때 기록합니다. 사용자가 broad approval을 주도록 묻는 대신 이름 붙은 issue를 결정하도록 물어야 합니다. 정확한 record quality와 public fields는 아래 reference 문서가 담당합니다.

참조: [Decision Packet](../reference/kernel.md#decision-packet), [Decision Gate](../reference/kernel.md#decision-gate), [`harness.request_user_decision`](../reference/mcp-api-and-schemas.md#harnessrequest_user_decision).

## 근거

에이전트가 로그인 흐름이 동작한다고 말합니다. 도움이 되는 근거로는 diff, 테스트 출력, 오류 상태 스크린샷, 브라우저에서 직접 확인했다는 기록이 있을 수 있습니다. 이런 기록이 없으면 "동작한다"는 말은 대화 속 주장일 뿐입니다.

근거는 작업에 대한 주장을 뒷받침하는 기록입니다. diff, 로그, 테스트, 스크린샷, 실행 요약, 평가 기록, Manual QA 기록, 작업과 연결된 오래 보관할 파일이 여기에 포함될 수 있습니다.

여기서는 쉬운 개념만 말합니다. 엄격한 동작은 [커널 참조](../reference/kernel.md)의 Evidence Gate, Evidence Manifest, evidence sufficiency, [MCP API와 스키마](../reference/mcp-api-and-schemas.md)와 [Storage와 DDL](../reference/storage-and-ddl.md)의 artifact registration, `ArtifactRef`, storage integrity, 그리고 [운영과 Conformance 참조](../reference/operations-and-conformance.md)의 conformance proof에 걸쳐 있습니다.

## Approval

작업에 새 의존성, 네트워크 호출, 민감한 파일 접근이 필요할 수 있습니다. 유용한 변경이라도 그 종류의 행동을 진행하기 전에 사용자의 Approval이 필요할 수 있습니다.

Sensitive-action Approval은 정해진 범위 안에서 민감한 행동을 진행해도 되는지 답합니다. Approval은 최종 결과 수락, 설계 절충 선택, 남은 위험을 받아들이는 판단과 다릅니다.

예를 들어 package install에 대한 sensitive-action Approval은 그 package를 아키텍처 방향으로 선택했다는 뜻이 아닙니다. Secret 접근 sensitive-action Approval은 secret 값을 Evidence, projection, export, log, screenshot, summary에 노출해도 된다는 뜻이 아닙니다.

참조: [커널 참조](../reference/kernel.md).

## Write Authorization

에이전트가 로그인 코드를 수정할 준비가 되었습니다. Harness는 활성 Change Unit이 있는지, 대상 경로가 범위 안인지, 필요한 sensitive-action Approval이 있는지, 먼저 풀어야 하는 결정이 있는지 확인해야 합니다.

Write Authorization은 지금 제품 파일 쓰기를 진행해도 되는지에 대한 Harness의 판단입니다. 현재 사양 용어로는 `prepare_write`가 제품 파일 쓰기 판단 지점입니다.

엄격한 동작은 [커널 참조](../reference/kernel.md)의 write-gate 의미와 state effect, [MCP API와 스키마](../reference/mcp-api-and-schemas.md)의 public `prepare_write` shape, 그리고 [Agent 통합 참조](../reference/agent-integration.md)의 connected-surface behavior에 걸쳐 있습니다.

## 검증

에이전트가 로그인 흐름을 수정한 뒤 테스트를 실행합니다. 이것은 유용하지만, 다른 세션이나 다른 도구 경로 또는 평가 묶음이 확인한 독립적인 검증과 같지는 않습니다.

검증은 결과를 어떻게 확인했는지, 그 확인이 얼마나 분리되어 있었는지 기록합니다. Harness는 자체 확인과 분리된 검증을 구분해서 확신과 독립성을 혼동하지 않게 합니다.

엄격한 동작은 [커널 참조](../reference/kernel.md)의 verification gate, assurance, verification independence 의미, [MCP API와 스키마](../reference/mcp-api-and-schemas.md)의 Eval과 verification tool schema, 그리고 [운영과 Conformance 참조](../reference/operations-and-conformance.md)의 conformance fixture에 걸쳐 있습니다.

## Manual QA

테스트가 통과해도 오류 문구가 헷갈리거나, 모바일에서 잘리거나, 화면의 다른 부분과 어울리지 않을 수 있습니다. 사람이 결과를 보고 무엇을 확인했는지 남겨야 할 때가 있습니다.

Manual QA는 사람이 실제 경험을 직접 확인하는 기록입니다. 특히 UI, UX, 문구, 접근성, 시각 결과, 제품 감각처럼 사람의 판단이 중요한 곳에서 필요합니다.

Manual QA를 면제한다면 생략한 대상, 받아들이는 위험, 후속 작업, 닫기 영향을 이름 붙여야 합니다. Waiver는 기록된 판단이지 테스트 결과(test result)가 아닙니다.

엄격한 동작은 [설계 품질 정책](../reference/design-quality-policies.md)의 Manual QA requirement와 waiver policy, [커널 참조](../reference/kernel.md)의 QA Gate 의미, [MCP API와 스키마](../reference/mcp-api-and-schemas.md)의 Manual QA record와 tool shape, 그리고 [운영과 Conformance 참조](../reference/operations-and-conformance.md)의 conformance proof에 걸쳐 있습니다.

## 수락

작업이 구현되고 확인까지 되었더라도, 사용자는 결과가 요청을 만족하는지와 남은 절충을 받아들일 수 있는지 판단해야 합니다.

수락은 작업 결과를 받아들일 수 있다는 사용자의 판단입니다. Approval, 검증, Manual QA, 남은 위험과 별개입니다.

참조: [커널 참조](../reference/kernel.md).

## 남은 위험

로그인 흐름은 끝났지만 이번 작업에 속도 제한은 넣지 않았을 수 있습니다. 또는 현재 환경에서 분리된 검증을 실행하지 못했을 수 있습니다. 이런 남은 불확실성은 "완료" 뒤로 사라지면 안 됩니다.

남은 위험은 작업 뒤에 남는 알려진 불확실성, 제한, 절충입니다. 작업을 닫는 데 그 위험을 받아들이는 판단이 필요하다면, 그 판단이 명시적으로 기록되어야 합니다.

참조: [커널 참조](../reference/kernel.md).

## Projection

Harness는 기록된 상태에서 읽기 쉬운 작업 보고서나 Journey Card를 만들 수 있습니다. 사용자는 그것을 빠르게 읽을 수 있지만, 보고서를 편집했다고 해서 운영 기록이 조용히 바뀌어서는 안 됩니다.

Projection은 Harness 상태 기록과 근거 참조를 사람이 읽을 수 있게 보여 주는 결과입니다. Markdown 보고서, Journey Card, Journey Spine은 Projection입니다. 상태를 보여 주지만 상태를 대체하지는 않습니다.

엄격한 동작은 [문서 Projection 참조](../reference/document-projection.md)의 projection authority, managed block, freshness, [MCP API와 스키마](../reference/mcp-api-and-schemas.md)의 `ProjectionKind`와 projection ref, 그리고 [Template Reference](../reference/templates/README.md)의 rendered template body와 display card shape에 걸쳐 있습니다.

## Reconcile

사용자가 생성된 보고서의 메모 영역을 고쳐서 다른 다음 행동을 제안합니다. Markdown 한 줄이 바뀌었다고 운영 상태가 바뀐 것처럼 취급하면 안 됩니다. 제안은 의도적인 경로를 거쳐 상태로 들어가야 합니다.

Reconcile은 사람이 편집한 메모, 제안, 읽기용 문서와 실제 상태의 차이를 받아들인 상태 변경, 거절된 제안, 메모, 결정, 나중으로 미룬 항목으로 정리하는 명시적 경로입니다.

엄격한 동작은 [문서 Projection 참조](../reference/document-projection.md)의 human-editable input, drift, reconcile item, [MCP API와 스키마](../reference/mcp-api-and-schemas.md)의 public reconcile decision shape, 그리고 reconcile 결과가 바꾸는 해당 state record의 owner reference에 걸쳐 있습니다.
