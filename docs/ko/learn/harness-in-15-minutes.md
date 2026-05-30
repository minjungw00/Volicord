# 15분 만에 보는 하네스

## 이 문서로 할 수 있는 일

무거운 Reference 문서를 읽기 전에 여섯 개의 짧은 시나리오로, 일상적인 AI 지원 작업에서 하네스가 어떻게 느껴지는지 이해합니다.

읽고 나면 어떤 작업이 아주 작게 유지될 수 있는지, 언제 Discovery가 필요한지, 결정 패킷이 왜 작업을 막을 수 있는지, 근거가 어떤 역할을 하는지, 왜 close가 아직 막힐 수 있는지, Markdown projection이 왜 state가 아닌지 구분할 수 있어야 합니다.

## 이런 때 읽기

하네스를 처음 접하고, 정확한 gate, schema, DDL, projection rule, conformance fixture를 배우기 전에 실용적인 예시를 먼저 보고 싶을 때 읽습니다.

## 읽기 전에

하네스 배경지식은 필요하지 않습니다. 더 긴 이해 모델을 먼저 보고 싶다면 [개요](overview.md)를 읽습니다. 이 문서 뒤에 하나의 전체 작업 흐름을 보고 싶다면 [하나의 작업으로 보는 하네스](harness-in-one-task.md)를 읽습니다.

## 핵심 생각

하네스는 AI 지원 작업을 따라갈 수 있게 몇 가지를 명시합니다. 무엇을 하려는지, 무엇을 바꿀 수 있는지, 사용자가 무엇을 결정해야 하는지, 완료 주장을 무엇이 뒷받침하는지, 어떤 잔여 위험이 있는지, Task를 닫을 수 있는지를 보이게 합니다.

아래 예시는 온보딩용 예시이지 schema나 새 authority path가 아닙니다. 정확한 behavior는 끝부분에 연결한 Reference owner에 남아 있습니다.

## 시나리오 1: 아주 작은 문서 수정

사용자가 말합니다.

```text
이 설치 안내의 오탈자를 고쳐줘.
```

유용한 하네스 형태는 의도적으로 작습니다.

- 범위: 이름 붙은 문서의 한 문장이나 한 문단.
- 범위 밖: 의미 변경, link behavior 변경, rendered output 변경, contract 변경, 주변 정리.
- 변경: 오탈자 수정.
- 근거: 변경 경로와 spelling-only라는 짧은 self-check.
- 닫기: 작은 결과와 escalation이 있었는지 보고.

사용자가 보는 결과는 짧아야 합니다.

```text
`docs/install.md`의 오탈자를 고쳤습니다.
Self-check: spelling-only, 의미나 contract 변경 없음.
Tiny direct로 닫았습니다. 잔여 위험: 이번 close에는 알려진 것 없음.
```

Tiny direct는 여전히 `direct` 아래에 있습니다. 별도 mode가 아니며 사용자 소유 판단, security boundary, evidence, scope, 쓰기 허가 기록, 잔여 위험 표시, close rule을 우회하지 않습니다. 문서 수정이 의미를 바꾸거나, link/render proof가 필요하거나, 엄격한 Reference contract에 닿거나, changed path와 self-check support를 넘어서면 같은 Task를 일반 `direct` 또는 `work`로 옮겨야 합니다.

정확한 mode, evidence, close behavior는 [Kernel Reference](../reference/kernel.md#mode), [Evidence Sufficiency Profiles](../reference/kernel.md#evidence-sufficiency-profiles), [`close_task`](../reference/kernel.md#close_task)를 사용합니다.

## 시나리오 2: 직접적인 코드 수정

사용자가 말합니다.

```text
Invoice summary에서 null date formatting을 고쳐줘.
```

여전히 작은 작업이지만 제품 코드가 바뀔 수 있습니다. 하네스는 작업을 좁게 유지해야 합니다.

- 범위: date formatting helper 또는 caller와 focused test.
- 범위 밖: invoice data model 변경, localization strategy, billing behavior, public API 변경.
- 쓰기 전: active Change Unit이 intended path를 포함해야 하고, `prepare_write`가 특정 write attempt를 허용해야 합니다.
- 근거: diff 또는 patch summary와 focused test, 또는 자동 확인이 적용되지 않는다는 기록된 이유.
- 닫기: 작업이 좁게 유지되고 required QA, 분리 검증, acceptance, 잔여 위험 경로가 적용되지 않으면 보통 self-checked로 닫습니다.

사용자가 보는 결과는 여전히 단순할 수 있습니다.

```text
Null invoice date가 "Not set"으로 표시되도록 바꿨습니다.
`invoiceSummary.test`로 확인했습니다.
쓰기 허가 기록이 implementation run에서 consumed됐습니다.
Self-checked로 닫았습니다. 알려진 닫기 관련 잔여 위험은 없습니다.
```

에이전트가 formatter가 export, report, billing email, API response에 공유된다는 사실을 발견하면 이 작업은 더 이상 직접적인 코드 수정이 아닙니다. 하네스는 멈추고 제품 파일을 더 바꾸기 전에 더 넓은 영향을 먼저 정리해야 합니다.

정확한 write와 evidence 권한은 [Change Unit](../reference/kernel.md#change-unit), [쓰기 허가 기록](../reference/kernel.md#write-authorization), [`prepare_write`](../reference/kernel.md#prepare_write), [Evidence Gate](../reference/kernel.md#evidence-gate)를 사용합니다.

## 시나리오 3: Discovery가 필요한 기능 작업

사용자가 말합니다.

```text
Login에 remember-me 동작을 추가해줘.
```

작아 보이지만 제품 동작, security, session lifetime, UI, test, storage에 닿습니다. 하네스는 구현 계획 전에 Discovery를 사용해야 합니다.

```text
Goal: remember-me 동작 추가.
Need to clarify: session을 늘릴지, email을 기억할지, 둘 다인지.
Codebase-answerable: 현재 session lifetime이 어디에서 설정되는지.
First safe Change Unit candidate: login checkbox, 선택된 session behavior, focused tests.
User question: remember-me가 이 기기에서 session을 더 오래 유지한다는 뜻인가요, email address를 미리 채운다는 뜻인가요, 아니면 둘 다인가요?
```

Discovery는 제품, 기술, security, QA, 운영, scope 질문을 분리합니다. Codebase-answerable question은 repository와 현재 하네스 context에서 답하고, codebase가 답할 수 없는 결정만 사용자에게 묻습니다.

Discovery는 Approval, 쓰기 허가 기록, evidence, verification, QA, acceptance, 잔여 위험을 받아들이는 판단, close, scope authority, 새 authority path가 아닙니다. 첫 번째 안전한 Change Unit이 보이도록 요구사항을 구체화하는 작업입니다.

사용자에게 보이는 흐름은 [사용자 가이드](../use/user-guide.md#처음-읽을-때의-경로)와 [Agent 세션 흐름](../use/agent-session-flow.md)를 사용합니다. 용어 뒤의 정확한 owner behavior는 [Kernel Reference](../reference/kernel.md)와 [MCP API와 스키마](../reference/mcp-api-and-schemas.md)를 사용합니다.

## 시나리오 4: 막힌 결정 패킷

Login 작업 중 에이전트가 사용자 소유 UX 선택에 도달합니다.

```text
Failed-login feedback은 inline, toast, modal 중 하나가 될 수 있습니다.
```

이것은 막연한 "승인할까요?" prompt가 되면 안 됩니다. 에이전트는 실제 선택, option, recommendation, uncertainty, deferral consequence를 담은 결정 패킷 형태의 prompt를 보여줘야 합니다.

```text
Judgment type: Product / UX
Why now: 최종 UI 동작과 test에는 failure-feedback pattern 하나가 필요합니다.
Options: inline message, toast, modal.
Recommendation: form 근처 inline message. 지속적으로 보이고 접근성이 좋습니다.
Uncertainty: 기존 design-system error-message support 확인 필요.
Deferral consequence: API와 state wiring은 계속할 수 있지만 final UI behavior와 수동 QA는 기다려야 합니다.
```

결정이 blocking이면 하네스는 사용자 소유 판단을 결정 패킷 path로 기록합니다. Chat text, 넓은 "go ahead", projection prose만으로는 특정 기록된 선택에 답하지 않는 한 결정을 충족하면 안 됩니다. 결정 패킷은 approval-shaped이고 Approval path에 연결된 경우가 아니라면 sensitive-action Approval도 아닙니다.

실용 예시는 [결정 패킷 Cookbook](../use/decision-packet-cookbook.md)을 읽습니다. 정확한 behavior는 [결정 패킷](../reference/kernel.md#decision-packet), [Decision Gate](../reference/kernel.md#decision-gate), [`harness.request_user_decision`](../reference/mcp-api-and-schemas.md#harnessrequest_user_decision), [`harness.record_user_decision`](../reference/mcp-api-and-schemas.md#harnessrecord_user_decision)을 사용합니다.

## 시나리오 5: 근거와 close blocker

에이전트가 기능을 끝내고 말합니다.

```text
Code는 끝났고 test도 통과했습니다.
```

하지만 close-relevant support가 완전하지 않으면 하네스가 close를 막을 수 있습니다. 이것은 작업이 실패했다는 뜻이 아니라, close basis가 아직 완성되지 않았다는 뜻입니다.

흔한 예시는 다음과 같습니다.

- Acceptance criterion을 뒷받침하는 ref가 없어 evidence가 partial입니다.
- Verification이 required인데 compatible Eval이 없습니다.
- UI behavior에 수동 QA가 required인데 아직 passed 또는 valid waiver가 없습니다.
- Final acceptance가 required인데 evidence, QA, verification, 잔여 위험 표시와 함께 요청되지 않았습니다.
- 알려진 닫기 관련 잔여 위험이 있지만 아직 보이거나 받아들여지지 않았습니다.

유용한 close blocker는 가장 작은 unblocker를 이름 붙입니다.

```text
Close blocked: login error workflow의 수동 QA가 아직 pending입니다.
Smallest unblocker: 수동 QA를 기록하거나, skipped check를 이름 붙이고 close-relevant risk가 남아 있다면 잔여 위험을 받아들이는 판단을 별도로 route하는 QA waiver 결정 패킷을 요청합니다.
```

Waiver와 risk-accepted close path는 명시적으로 남아야 합니다. Verification waiver는 분리 검증을 만들지 않습니다. QA waiver는 UI를 검사했다는 증거가 아닙니다. 잔여 위험을 받아들이는 판단은 risk를 사라지게 만들지 않습니다.

정확한 close와 gate behavior는 [`close_task`](../reference/kernel.md#close_task), [Evidence Gate](../reference/kernel.md#evidence-gate), [Verification Gate](../reference/kernel.md#verification-gate), [QA Gate](../reference/kernel.md#qa-gate), [Acceptance Gate](../reference/kernel.md#acceptance-gate), [잔여 위험](../reference/kernel.md#residual-risk)를 사용합니다.

## 시나리오 6: Projection은 state가 아니다

`TASK` Markdown report에 이렇게 보입니다.

```text
Evidence: partial
Next action: 수동 QA 기록
source_state_version: 42
```

이 report는 유용하지만 운영 기록은 아닙니다. 읽기용 투영 문서(projection), 즉 현재 state record와 아티팩트 참조에서 렌더링된 읽기용 view입니다.

사람이 report를 이렇게 수정해도:

```text
Evidence: sufficient
```

그 edit은 Evidence Manifest, gate state, 수동 QA status, acceptance, 잔여 위험, close eligibility를 바꾸지 않습니다. Human-editable section은 note나 reconcile input이 될 수 있지만, accepted state change에는 여전히 owner Core/MCP path가 필요합니다.

실용 규칙은 간단합니다. Projection은 orientation, ref, freshness를 읽는 데 사용하고, 권한은 owner record와 owner action에서 확인합니다. Projection이 stale이거나 틀렸다면 Markdown을 state처럼 취급하지 말고 refresh 또는 reconcile합니다.

정확한 projection boundary는 [문서 Projection 참조](../reference/document-projection.md), 특히 [Projection을 쉽게 말하면](../reference/document-projection.md#projection을-쉽게-말하면)을 사용합니다.

## 이 둘러보기의 Reference owner

| 주제 | 정확한 behavior owner |
|---|---|
| Task, Change Unit, 결정 패킷, gate, evidence, verification, QA, acceptance, 잔여 위험, close | [Kernel Reference](../reference/kernel.md) |
| Public tool request와 response shape | [MCP API와 스키마](../reference/mcp-api-and-schemas.md) |
| Markdown projection authority와 freshness | [문서 Projection 참조](../reference/document-projection.md) |
| 사용자-facing session flow와 status 읽기 | [사용자 가이드](../use/user-guide.md), [Agent 세션 흐름](../use/agent-session-flow.md) |
| 실용 결정 패킷 예시 | [결정 패킷 Cookbook](../use/decision-packet-cookbook.md) |

## 다음에 읽을 문서

- 더 긴 direct와 work task 이야기는 [하나의 작업으로 보는 하네스](harness-in-one-task.md)를 읽습니다.
- 사용자 소유 판단이 진행을 막을 때는 [결정 패킷 Cookbook](../use/decision-packet-cookbook.md)을 읽습니다.
- 실제 세션을 진행할 때는 [사용자 가이드](../use/user-guide.md)를 읽습니다.
- 정확한 계약이 필요할 때만 Reference 문서를 사용합니다.
