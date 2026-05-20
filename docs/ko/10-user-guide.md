# 사용자 가이드

## 문서 역할

이 문서는 사용자가 agent에게 어떻게 말하면 되는지, 상태를 어떻게 읽으면 되는지, 언제 어떤 판단을 내려야 하는지 설명한다.

구현 내부, 설치 절차, 서버 동작 방식은 다루지 않는다.

## 5분 시작 경로

Harness로 작업 하나만 진행하고 싶다면 한 문장으로 시작한다.

```text
이 작업 하네스 기준으로 진행해.
```

Agent는 먼저 세 가지 일상적인 질문에 답해야 한다. 되도록 compact status 또는 Journey Card로 보여주면 좋다.

- scope는 무엇인가: 무엇이 포함되고, 무엇이 범위 밖인가?
- evidence는 무엇인가: 바뀐 것이 있다면 무엇이고, 무엇을 확인했으며, 무엇이 아직 부족한가?
- 지금 필요한 판단은 무엇인가: 방향을 고를지, sensitive step을 승인할지, QA를 확인할지, 표시된 residual risk를 수용할지, 또는 acceptance가 required일 때 결과를 수용할지?

```mermaid
flowchart LR
  Request["Everyday request"] --> Status["compact status or Journey Card"]
  Status --> Scope["scope<br/>included / out of bounds"]
  Status --> Evidence["evidence<br/>changed / checked / missing"]
  Status --> Judgment["judgment<br/>decision / approval / QA / risk / acceptance"]
  Scope --> Next["safe next action"]
  Evidence --> Next
  Judgment --> Next
```

그 답 중 하나가 중요해질 때만 agent는 Decision Packet, Write Authority, Autonomy Boundary, Manual QA, acceptance, residual risk, approval, evidence, verification 같은 더 깊은 label을 사용해야 한다.

대부분의 경우 사용자가 결정하는 것은 몇 가지뿐이다.

- scope가 원하는 작업과 맞는지
- Decision Packet이 나오면 어떤 product direction 또는 trade-off를 택할지
- sensitive change를 approval할지
- Manual QA가 필요한지, 완료/통과되었는지, 또는 validly waived되었는지
- close 전이나 acceptance가 required인 final acceptance 전에 표시된 residual risk를 받아들일지

막혔을 때는 가장 작은 다음 unblocker를 물어본다.

```text
지금 무엇 때문에 막혀 있고, 어떤 결정 하나나 check 하나가 있으면 풀릴까?
```

Close가 가까워지면 결과가 scope와 맞는지, evidence가 required가 아니거나 acceptance criteria를 뒷받침하는지, verification이 required가 아니거나 passed 또는 recorded risk와 함께 명시적으로 waived되었는지, Manual QA가 required가 아니거나 passed/completed 또는 validly waived되었는지, close-relevant residual risk가 표시되었거나 agent가 `ResidualRiskSummary.status=none`을 report했는지, final acceptance 요청이 approval과 별도인지 확인한다.

작업이 특정 gate나 judgment에서 멈췄을 때는 아래 상세 내용을 reference로 사용하면 된다.

## 문장 Reference

일상적인 작업은 명령어가 아니라 대화로 시작한다.

```text
이 작업 하네스 기준으로 진행해.
```

이 말은 "상태를 확인하고, 범위를 잡고, 쓰기 전에 허용 경계를 확인하고, 적용될 때만 relevant evidence, checks, 사용자 판단을 남기면서 진행해"라는 뜻이다.

자주 쓰는 말:

```text
상태 보여줘.
이 작업 이어서 해. 하네스 상태부터 확인해.
이어가기 전에 Journey Card부터 보여줘.
범위와 질문부터 잡아줘.
작은 수정이면 direct로 처리하고, 커지면 work로 전환해.
Decision Packet을 옵션, 추천안, 불확실성까지 보여줘.
Trade-off에는 product-review lens를 사용해. 그 review가 다음 useful step이면 eng-review, design-review, security-review, qa-review, release-handoff를 사용해.
승인해. 범위는 방금 설명한 내용까지만이야.
Active Change Unit scope와 Autonomy Boundary latitude가 모두 맞을 때만 AFK로 진행해. Sensitive categories에는 별도 granted approval이 필요하고, 실제 product write에는 compatible `prepare_write` Write Authorization이 필요해.
detached verify 시작해.
Manual QA가 필요한지 판단해줘.
수용하기 전에 close-relevant residual risk를 보여줘.
PR과 deploy review를 위한 release handoff를 생성해줘.
Final acceptance 요청이 있으면 수용해. 이 작업 닫아.
Final acceptance가 required가 아니면 applicable blockers가 clear된 뒤 닫아.
이 task를 current Change Unit으로 freeze하고, Decision Packet 없이 scope를 넓히지 마.
이 task를 docs/en/09-agent-integration.md와 docs/ko/09-agent-integration.md로 freeze해.
현재 guard level과 실제로 prevent할 수 있는 것을 보여줘.
이 change에는 careful mode를 사용해. Scope를 좁히고, write 전 write authority를 보여주고, product trade-off 전에는 물어봐.
```

## 기본 흐름

기본 흐름은 작업 관리 시스템이 아니라 짧은 대화처럼 느껴져야 한다. 사용자는 모든 internal record가 아니라 compact status card, 다음 safe action, 그리고 때로는 optional recommended playbooks를 주로 본다.

1. 상태 확인 또는 intake.
2. `advisor`, `direct`, `work` 중 하나로 분류.
3. 범위와 Change Unit 확인.
4. 제품 판단이 진행을 막고 있으면 Decision Packet을 읽고 답하기.
5. 유용할 때 optional recommended playbooks로 절차 선택을 안내.
6. 제품 파일을 쓰기 전에 agent 또는 Harness가 `prepare_write`를 확인.
7. 변경이나 advice가 있으면 agent가 relevant result를 기록하고, evidence가 적용될 때 evidence를 기록.
8. Task path가 요구할 때 verify, Manual QA 기록, close-relevant residual risk 표시, acceptance 요청.
9. Close.

```mermaid
flowchart LR
  Status["status or intake"] --> Classify["classify<br/>advisor / direct / work"]
  Classify --> Writes{"product write needed?"}
  Writes -->|no| Advisor["advisor / no-write path"]
  Advisor --> RecordOnly["record answer or result"]
  Writes -->|yes| Scope["scope + Change Unit"]
  Scope --> Decision{"product judgment blocked?"}
  Decision -->|yes| Packet["Decision Packet"]
  Decision -->|no| Prepare["prepare_write before writes"]
  Packet --> Prepare
  Prepare --> Evidence["record result + evidence"]
  Evidence --> Checks["verify / QA / residual risk / acceptance"]
  RecordOnly --> Checks
  Checks --> Close["close when blockers clear"]
```

많은 advisor 또는 direct tasks는 뒤쪽 check 중 일부를 건너뛴다. Final acceptance가 required가 아니라면 status가 그 사실을 말하거나, applicable blockers가 clear되고 residual risk가 표시되었거나 none으로 confirmed된 뒤 close하면 된다.

Gate는 "왜 지금 task를 안전하게 진행하거나 닫을 수 없는지"로 설명되어야 한다. Evidence 부족은 추상적인 내부 조건이 아니라 acceptance criterion별로 보여야 한다. Cooperative guarantee가 보이면 surface가 Harness decision을 따를 것으로 기대되지만, 모든 범위 위반 write를 실행 전에 물리적으로 막지는 못할 수 있다는 뜻이다.

```text
Close blocked:
- AC-02 evidence가 없습니다.
- UI copy에 대한 Manual QA가 pending입니다.
- Verification을 waive하면 detached verified가 아니라 risk accepted close로 닫힙니다.
```

## 보통 사용자가 결정하는 것 (What You Usually Decide)

대부분의 session에서 사용자가 주로 결정할 것은 명확하지 않은 scope, product 또는 design trade-off, sensitive approval, QA, 표시된 residual risk, 그리고 task path가 요구할 때의 final acceptance입니다.

사용자는 작업 방향과 받아들일 수 있는 risk의 owner이지, internal records를 직접 조작하는 operator가 아닙니다.

## Harness가 맡아야 할 것 (What Harness Should Handle)

Harness가 맡아야 할 것은 state recording, `prepare_write` checks, artifact registration, evidence mapping, projection freshness, close blockers입니다.

Harness는 사용자의 판단을 recorded state와 clear blockers로 바꾸어, 사용자가 bookkeeping이 아니라 ownership에 집중하게 해야 합니다.

Recommended playbooks는 optional procedure hints입니다. Agent가 review, TDD, QA, guard-check, release-handoff, browser-QA path, role lens를 고르는 데 도움을 줄 수 있지만 approval, write authority, evidence, verification, QA result, acceptance, close가 아닙니다.

## Freeze And Guard 사용하기

Freeze와 guard는 plain-language safety control입니다. Agent에게 얼마나 조심해야 하는지 알려주기 때문에 유용하지만, 그 강도는 connected surface에 달려 있습니다.

`Freeze`는 "여기서 hold" 또는 "더 좁은 posture를 사용하라"라는 뜻입니다. Agent에게 task를 current Change Unit, specific paths, read-only advice, 또는 Journey Card refresh 전까지 freeze하라고 요청할 수 있습니다. Freeze는 product write를 hold하거나, next action을 더 strict하게 만들거나, existing scope가 request와 맞지 않을 때 `prepare_write`가 block 또는 hold하게 만들 수 있습니다. 그 자체로 work를 authorize하거나 scope를 직접 rewrite하지 않습니다. Change Unit scope, allowed paths, Autonomy Boundary, AFK stop conditions의 persistent change는 여전히 normal Core state-changing path 또는 Decision Packet route를 거칩니다.

`Guard`는 "이 work 주변의 available protection을 보여주거나 사용하라"라는 뜻입니다. Cooperative surface에서는 agent가 boundary 안에 머물도록 instruction을 받습니다. Detective surface에서는 Harness가 changed-path, log, artifact, projection violation을 사후 detect할 수 있습니다. Preventive surface에서는 hook, wrapper, permission layer, sidecar가 covered violation을 실행 전에 block할 수 있습니다. Isolated path에서는 risky work 또는 verification이 별도 boundary에서 실행됩니다.

`Careful mode`는 stricter posture이지 new authority tier가 아닙니다. Narrower scope, 더 명시적인 `prepare_write` checks, 더 clear한 evidence mapping, 더 fresh한 status, user judgment에 대한 더 많은 one-question stop을 기대하면 됩니다.

Useful phrases:

```text
이 task를 current Change Unit으로 freeze해.
Decision Packet에 답할 때까지 writes를 freeze해.
현재 guard level과 limitation을 보여줘.
이 surface에서 가능한 가장 강한 guard를 사용하고, cooperative/detective/preventive/isolated 중 무엇인지 말해줘.
Careful mode를 사용하되, approval이나 write authority로 취급하지 마.
```

## Role Lens 사용하기

Role Lens는 agent에게 product, engineering, design, security, QA, release handoff 같은 익숙한 posture로 review하라고 요청하는 방법입니다.

```text
이 auth change에 security-review lens를 사용해.
release-handoff를 사용해서 close를 막는 것이 무엇인지 보여줘.
```

Lens는 Decision Packet, evidence, Manual QA, residual risk, validator route, release handoff input, 다른 playbook을 recommend할 수 있습니다. 하지만 그 자체로 approval, write authorization, check waiver, risk acceptance, result acceptance, assurance upgrade, task close를 할 수 없습니다.

TASK 문서는 Implementation Micro-Plan도 보여줄 수 있습니다. 이는 agent가 다음에 무엇을 하려는지 작은 step으로 읽기 쉽게 보여주는 view입니다. 실행을 따라가기 쉽게 하기 위한 것이며, 사용자가 손으로 관리할 필요는 없습니다. 또한 write를 authorize하지 않습니다. Product write는 여전히 active Change Unit scope와 `prepare_write`가 control합니다.

TDD가 required이면 agent가 implementation 전에 feedback loop와 RED target을 먼저 말하고, failing RED check를 작성하거나 실행한 뒤 GREEN check가 pass할 때까지 구현한다고 보면 됩니다. RED check에 대한 plan은 test를 만드는 데 도움이 되지만 그 자체가 RED evidence는 아닙니다. Agent가 TDD를 생략해야 한다면 Harness는 이유와 behavior를 증명할 alternate feedback loop를 기록해야 합니다. Actual RED evidence 또는 그 waiver가 없으면 non-test implementation write가 block될 수 있습니다.

Harness 또는 connected surface가 MCP를 reliable하게 사용할 수 없으면 product/runtime/code 변경은 connection 또는 surface setup이 diagnose될 때까지 pause해야 합니다. Exact path에 대해 명시적으로 granted된 documentation-only bootstrap override는 Harness authorization과 같지 않습니다.

## Release Handoff 생성하기

External PR, review, deploy, rollback, monitoring process를 위한 release-readiness report가 필요할 때 Release Handoff를 요청합니다.

```text
이 task의 release handoff를 생성해줘.
release-handoff를 사용하고 PR과 deploy checklist note를 포함해줘.
```

Report에는 close readiness, blocker, evidence ref, verification ref, Manual QA ref, residual-risk ref, changed file, projection freshness, redaction note, suggested checklist item이 포함된다고 보면 됩니다. 이 report는 external review 또는 deployment 준비를 돕지만, Harness는 handoff만으로 merge, deploy, production monitoring, approve, QA 또는 verification waiver, gate satisfaction, residual risk acceptance, result acceptance, assurance upgrade, task close를 하지 않습니다.

Future context index나 local metric은 status와 handoff report를 더 쉽게 inspect하게 만들 수 있습니다. 하지만 pointer와 diagnostic으로만 취급해야 합니다. Current state, evidence, gate, decision, QA, verification, risk acceptance, close는 여전히 Harness state와 normal Core flow에서 나옵니다.

## 상태 카드 읽기

좋은 하네스 세션은 먼저 짧은 상태 카드를 보여준다. 중요한 작업을 재개할 때는 일반 status card보다 Journey Card 또는 그에 준하는 current-position view가 먼저 나와야 한다.

```text
TASK-0044 이메일 로그인 플로우 추가
모드: work
현재 상태: shaping
다음 행동: 로그인 실패 UX 결정
범위: 로그인 폼, 로그인 API 호출, 세션 저장
Decision Gate: pending
Decision Packet: DEC-0012 로그인 실패 UX
Autonomy Boundary: 합의된 로그인 플로우 세부 구현만 진행 가능
Write Authority: not yet requested
Recommended playbooks: product-review, guard-check
Approval: dependency_change 승인 필요
Evidence: none
Verification: not started
Manual QA: pending
Acceptance: pending
Residual risk: none recorded
Projection: current
```

볼 것은 다음 항목들이다.

- 요청과 범위가 맞는가.
- 내가 답해야 할 Decision Packet이 있는가.
- Autonomy Boundary 안에서 agent가 어디까지 진행할 수 있는가.
- Write Authority가 아직 요청 전인지, blocked인지, intended write에 allowed인지.
- 다음 safe action에 유용한 recommended playbook이 있는가.
- 표시된 경우 Implementation Micro-Plan이 기대하는 다음 작은 step과 맞는가.
- approval, evidence, verification, Manual QA, residual risk, acceptance 중 무엇이 남았는가.
- 다음 행동이 안전하게 진행 가능한가.

```mermaid
flowchart TD
  Card["Status card"] --> Scope["request matches scope?"]
  Card --> Decision["Decision Packet waiting?"]
  Card --> Boundary["Autonomy Boundary clear?"]
  Card --> Write["Write Authority requested, blocked, or allowed?"]
  Card --> Remaining["approval, evidence, verification, QA, risk, acceptance"]
  Remaining --> Next["safe next action"]
```

상태가 이상해 보이면 이렇게 말한다.

```text
state 기준으로 현재 상태와 다음 행동을 다시 보여줘.
```

## Journey Card 보기

Journey Card는 일이 지금 어디에 있는지 보여주는 카드다. 오래 멈춘 작업을 다시 시작할 때, 큰 변경을 이어갈 때, close가 가까울 때 먼저 확인하면 좋다.

확인할 줄:

- `Next action`: agent가 지금 안전하다고 보는 다음 행동
- `Decision Packet`: 사용자가 답해야 할 제품 판단이 있는지
- `Autonomy Boundary`: 추가 질문 없이 agent가 할 수 있는 일
- `Write Authority`: intended write에 대한 specific `prepare_write` authorization이 있는지. Autonomy Boundary와 별개다.
- `Evidence`, `Verification`, `Manual QA`: 무엇이 확인되었는지
- `Residual risk`: 아직 남은 불확실성, 제한, trade-off
- `Projection`: 사람이 읽는 뷰가 믿을 만큼 최신인지

```mermaid
flowchart LR
  Journey["Journey Card"] --> Next["Next action"]
  Journey --> Gate["Decision Packet / gates"]
  Journey --> Boundaries["Autonomy Boundary + Write Authority"]
  Journey --> Checks["Evidence + Verification + Manual QA"]
  Journey --> Risk["Residual risk"]
  Journey --> Freshness["Projection freshness"]
```

자연스럽게 이렇게 말하면 된다.

```text
잠깐. Decision Packet부터 보여줘.
그 다음 행동은 괜찮아. 그 경계 안에서 계속해.
이번 run 끝나면 Journey Card를 새로 보여줘.
```

Write가 이미 authorized된 경우에도 line은 specific해야 한다.

```text
Write Authority: WA-0017 allowed for src/auth/login.ts and tests/auth/login.test.ts
Guarantee: cooperative; changed-path validation detects violations after the fact
```

## Decision Packet 읽기

먼저 사용자 질문에서 시작한다: "이 맥락에서 이 방향을 선택할까, 미룰까, 더 작은 Change Unit을 요청할까?"

Decision Packet은 넓은 의미의 "승인해도 돼요?"가 아니다. 진행, close, QA waiver, verification risk acceptance, residual-risk acceptance에 사람 판단이 필요할 때 쓰는 판단 단위다.

읽는 순서:

- 왜 지금 결정해야 하는가?
- 내가 정확히 무엇을 결정하는가?
- option과 trade-off는 무엇인가?
- agent의 추천은 무엇이고, 얼마나 불확실한가?
- 내 판단 없이 agent가 결정해도 되는 것은 무엇인가?
- 미루면 어떤 일이 생기는가?
- 어떤 residual risk나 follow-up이 남는가?

```mermaid
flowchart LR
  Why["why now?"] --> Decide["what decision?"]
  Decide --> Options["options + trade-offs"]
  Options --> Recommend["recommendation + uncertainty"]
  Recommend --> Boundary["what may proceed without me?"]
  Boundary --> Defer["deferral effect"]
  Defer --> Risk["residual risk or follow-up"]
```

좋은 답변은 구체적이다.

```text
Option A로 가자. 실패 메시지는 일반적으로 유지하고, 보안 trade-off를 기록해.
이 결정은 smoke test 후로 미뤄. follow-up risk로 남겨줘.
이 trade-off는 수용하지 않아. 더 작은 Change Unit을 제안해줘.
```

## advisor, direct, work

`advisor`는 읽고 설명하고 비교하고 검토하는 모드다. 제품 파일을 쓰지 않는다.

```text
이 모듈 역할을 설명해줘.
이 설계 선택의 trade-off를 정리해줘.
```

`direct`는 작고 저위험인 변경을 빠르게 처리하는 모드다. Direct도 제품 파일을 쓰려면 active scoped Change Unit이 있어야 하며, 기본 assurance는 `self_checked`다.

```text
프로필 저장 버튼 오타 고쳐줘. 작으면 direct로 처리해.
```

`work`는 기능 추가, 구조 변경, 위험한 수정, 여러 파일에 걸친 작업처럼 범위 정리와 evidence와 독립 검증이 필요한 모드다.

```text
이메일 로그인 플로우 추가해줘. 하네스 기준으로 진행해.
```

작게 시작했지만 범위가 커지면 agent는 같은 Task를 `work`로 전환한다고 알려야 한다.

## Small Direct Work Should Stay Light

작고 명확한 작업에서는 Harness가 좁은 scope를 active Change Unit으로 정하고, `prepare_write`로 write permission을 확인하고, changed paths와 self-check evidence를 기록한 뒤 blocker가 없으면 close해야 한다.

작업이 커지면 같은 Task를 `work`로 옮기고 scope, decisions, evidence, risk를 보여야 한다. Direct mode가 조용한 broad autonomy로 바뀌면 안 된다.

## 사용자 판단 (User Judgments)

Product judgment, approval, assurance, Manual QA, residual-risk acceptance, final acceptance는 서로 다른 질문에 답한다.

| Judgment | 답하는 질문 | 대신할 수 없는 것 |
|---|---|---|
| Product judgment / Decision Packet | 어떤 제품 방향, trade-off, waiver, close-relevant decision을 택할 것인가? | approval, implementation, verification, QA, acceptance |
| Approval | 이 민감 변경을 진행해도 되는가? | product judgment, verification, QA, acceptance |
| Assurance | 기술적으로 어디까지 확인되었는가? | approval, QA, acceptance |
| Manual QA | 사람이 실제 경험 품질을 확인했는가? | verification, acceptance |
| Residual-risk acceptance | 알려진 남은 위험이나 제한을 사용자가 받아들이는가? | approval, evidence, verification, Manual QA, final acceptance |
| Final acceptance | 결과와 남은 trade-off를 최종 수용하는가? | approval, evidence, verification, Manual QA, residual-risk acceptance |

```mermaid
flowchart TD
  Judgments["User judgments stay separate"] --> Product["Product judgment<br/>Decision Packet"]
  Judgments --> Approval["Approval<br/>sensitive action may proceed"]
  Judgments --> Assurance["Assurance<br/>technical check level"]
  Judgments --> QA["Manual QA<br/>human experience check"]
  Judgments --> Risk["Residual-risk acceptance<br/>known risk accepted"]
  Judgments --> Acceptance["Final acceptance<br/>result accepted"]
```

Approval이 필요한 예시는 dependency 추가, auth/permission 변경, data model 변경, public API 변경, destructive write, secret access, production config 변경이다. Approval은 correctness나 acceptance가 아니다.

Approval 자체에 사용자의 판단이 필요하면 Harness는 approval-shaped Decision Packet으로 보여줄 수 있다. 이때 사용자가 결정하는 것은 sensitive scope를 허용할지 여부다. 이 답변은 product option을 선택하거나, QA나 verification을 waive하거나, residual risk를 accept하거나, 이후 write check 없이 agent가 edit하게 만들지 않는다.

Product judgment가 진행을 막으면 Decision Packet으로 보여야 한다. Packet에는 option, trade-off, recommendation, uncertainty, decision을 미뤘을 때의 결과가 있어야 한다.

Assurance는 보통 `none`, `self_checked`, `detached_verified`로 보인다. `detached_verified`는 같은 세션의 자기 검토가 아니라 별도 검증 경계에서 통과했다는 뜻이다.

사용자가 verification risk를 수용하고 닫을 수는 있다. 하지만 그 close는 `detached_verified`가 아니라 risk-accepted close다. Residual-risk acceptance는 알려진 위험을 close 가능한 상태로 만들 수 있지만, approval, evidence, verification, Manual QA, final acceptance를 대신하지 않는다.

## AFK로 진행하게 할 때

AFK로 진행한다는 말은 사용자가 자리를 비워도 agent가 계속 진행해도 된다는 뜻이다. 하지만 AFK는 active Change Unit scope, Autonomy Boundary latitude, applicable한 granted sensitive approval이 모두 맞을 때만 허용된다. 실제 product write에는 writing 전에 compatible `prepare_write` / Write Authorization도 필요하다.

```mermaid
flowchart TD
  AFK["AFK work request"] --> Ready{"scope, boundary, approval,<br/>and write authority apply?"}
  Ready -->|no| StopGap["stop: boundary, scope,<br/>approval, or Write Authority gap"]
  Ready -->|yes| Proceed["proceed inside boundary"]
  Proceed --> Judgment{"human-held judgment appears?"}
  Judgment -->|yes| StopJudgment["stop and ask user"]
  Judgment -->|no| Continue["continue allowed work"]
```

Autonomy Boundary는 scope grant나 write permission이 아니다. Agent는 여전히 `prepare_write`, active Change Unit scope, allowed paths, allowed tools, allowed commands, network targets, secret access, applicable한 sensitive approval을 따라야 한다.

Agent는 보통 합의된 세부 구현, 허용된 check 실행, evidence 수집, summary 업데이트를 진행하고, 명확한 blocker를 남기고 멈출 수 있다.

Agent가 멈춰야 하는 일:

- planning direction
- product trade-off
- scope expansion
- sensitive-change approval
- QA waiver
- verification risk acceptance
- final acceptance

자연스러운 말:

```text
이 경계 안에서는 AFK로 진행해. 제품 판단, QA waiver, 검증 리스크, acceptance가 필요하면 멈춰.
```

## Evidence 부족

Evidence는 "했음"이라는 말이 아니라 acceptance criteria를 뒷받침하는 기록이다.

```text
Evidence: partial
Close blocked: AC-02 supporting evidence가 없습니다.
```

```mermaid
flowchart TD
  Missing["Evidence missing or stale"] --> Map["map gap to acceptance criterion"]
  Map --> Check["suggest enough checks or artifacts"]
  Check --> Run["fresh run, log, diff, bundle, or scope reconfirmation"]
  Run --> Record["record evidence"]
  Record --> Gate["evidence gate can be reassessed"]
```

이렇게 말한다.

```text
어떤 acceptance 기준에 evidence가 부족한지 보여주고, 무엇을 더 확인하면 충분한지 제안해줘.
```

Evidence가 stale이면 새 실행, 새 로그, 새 diff, 새 verification bundle, 또는 범위 재확인이 필요할 수 있다.

## Reading Review Stages

Harness는 review를 two stages로 보여줄 수 있다. Spec Compliance Review는 requested thing이 실제로 complete한지 묻는다: acceptance criteria, Change Unit completion conditions, write authority compatibility, decisions, evidence, visible residual risk. Code Quality / Stewardship Review는 implementation이 maintainable한지 묻는다: domain language, module/interface boundaries, slice shape, feedback loop 또는 TDD, codebase stewardship, context hygiene, follow-up risk.

이 review stages는 validator results, missing evidence, Decision Packets, Change Unit updates, residual risk, close blockers를 가리킬 수 있다. Detached verification은 아니다. Passed same-session review는 useful하지만 Task를 `detached_verified`로 만들지 않는다.

## Verify

Work는 구현자의 자기 보고만으로 `detached_verified`가 되지 않는다.

```mermaid
flowchart TD
  Need["Need verification?"] --> Boundary{"valid independence boundary?"}
  Boundary -->|yes| Eval["record Eval"]
  Eval --> Passed{"passed?"}
  Passed -->|yes| Detached["may support detached_verified"]
  Passed -->|no| Blocked["blocked or rework"]
  Boundary -->|no / waived| Risk["risk-accepted close path only"]
```

```text
detached verify 시작해.
```

검증이 통과하면 agent는 무엇을 확인했는지, 어떤 기준으로 독립성이 인정되는지, 남은 blocker가 있는지 요약해야 한다.

검증을 지금 하지 않고 닫아야 한다면 이렇게 말한다.

```text
검증 risk를 수용하고 닫아. 남은 위험을 기록해줘.
```

이 경우 성공으로 닫을 수 있지만, assurance는 `detached_verified`로 표시되지 않는다.

## Residual Risk 수용

Residual risk는 이미 알려진 남은 불확실성, 제한, 확인하지 못한 조건, trade-off다. Final acceptance 또는 risk-accepted close 전에는 agent가 close-relevant residual risk를 쉬운 말로 보여줘야 한다. Known close-relevant residual risk가 없다면 agent는 `ResidualRiskSummary.status=none`으로 그 사실을 말해야 한다. 이는 known risk가 hidden인 상태와 다르다. Risk-accepted close에는 여전히 visible and accepted Residual Risk refs가 필요하다.

Residual risk를 수용하면 close가 가능해질 수 있다. 하지만 approval, evidence, verification, Manual QA, acceptance를 대신하지는 않는다.

```mermaid
flowchart TD
  Before["Before close or acceptance"] --> Known{"known close-relevant risk?"}
  Known -->|yes| Show["show risk clearly"]
  Show --> Decide{"user accepts risk?"}
  Decide -->|yes| Accepted["risk accepted close may be possible"]
  Decide -->|no| Rework["rework or add verification"]
  Known -->|no| None["report ResidualRiskSummary.status=none"]
  None --> Next["continue if other blockers are clear"]
```

자주 쓰는 말:

```text
수용하기 전에 close-relevant residual risk를 보여줘.
여기 표시된 residual risk를 수용해. risk accepted로 닫아.
그 risk는 수용하지 않아. rework하거나 verification을 추가해.
```

## Manual QA

Manual QA는 UX, workflow, copy, accessibility, visual result처럼 사람이 봐야 하는 품질에 대한 사용자 판단이다.

```mermaid
flowchart TD
  QA["Manual QA needed?"] --> Needed{"needed for this task shape?"}
  Needed -->|yes| Inspect["human inspects experience"]
  Inspect --> Result{"passed?"}
  Result -->|yes| Record["record Manual QA passed"]
  Result -->|no| Rework["rework or blocked"]
  Needed -->|no| Risk{"waiver carries product/user risk?"}
  Risk -->|no| Waive["record waiver reason"]
  Risk -->|yes| Packet["Decision Packet"]
  Packet --> Decision{"user decision"}
  Decision -->|accept waiver| Waive
  Decision -->|reject waiver| Rework
  Decision -->|defer| Blocked["blocked"]
```

Card가 `Manual QA: pending`이라고 표시할 때 그것은 `qa_gate` display다. Required QA가 아직 satisfying Manual QA record를 만들지 못했다는 뜻이지, pending Manual QA record result가 있다는 뜻은 아니다.

```text
Manual QA가 필요한지 판단해줘.
```

Manual QA 판단이 "아직 수용할 수 없음"이면 작업은 닫지 않고 rework나 blocked 상태로 돌아간다. 이 Task shape에서 Manual QA가 유용하지 않다면 waiver reason을 기록한다.

```text
이번 내부 CLI 작업은 Manual QA waived 처리해. 이유: 사용자 UI가 없고 test/log로 충분히 확인 가능.
```

QA를 생략하는 일이 product/user risk를 포함하면, 단순한 waiver reason만으로는 부족할 수 있다. 이때는 Decision Packet이 필요할 수 있다.

```text
QA waiver를 결정하기 전에 Decision Packet으로 보여줘.
```

### Browser QA Capture

Connected surface가 Browser QA Capture를 지원하면 agent는 `browser_smoke`, `workflow`, `ui_quality`, `accessibility` 같은 Manual QA profile에 대해 browser QA pass를 제안할 수 있습니다. Capture는 screenshot, `qa_capture` bundle, console log, network trace, accessibility snapshot, workflow recording 같은 artifact refs를 Manual QA record에 attach할 수 있습니다.

```text
연결된 profile이 지원하면 이 checkout flow에 browser QA capture를 실행해. Screenshot과 console log를 Manual QA record에 attach해.
```

Browser QA Capture는 유용한 evidence이지 final acceptance가 아닙니다. Taste, experience quality, accessibility interpretation, copy, visual review에 사람의 판단이 필요하면 human Manual QA judgment를 대체하지 않습니다. Verification path가 별도로 independence requirements를 충족하지 않는 한 detached verification으로도 인정되지 않습니다.

Surface가 browser workflow를 capture할 수 없으면 human Manual QA note와 manually supplied artifacts를 대신 요청합니다.

## Acceptance

Acceptance는 "이 결과를 받아들인다"는 마지막 사용자 판단이다. 이는 task path가 final acceptance를 요구할 때만 나타난다.

Acceptance가 required이면 close-relevant residual risk가 표시되었거나 none으로 report된 뒤, 사용자가 결과와 남은 trade-off를 수용하기 전까지 task는 닫히지 않는다. Verification 통과, Manual QA 완료, approval granted, 특정 residual risk 수용은 그 자체로 final acceptance가 아니다.

```mermaid
flowchart TD
  Required["Final acceptance required?"] --> Risk["show close-relevant residual risk or status none"]
  Risk --> User{"user accepts result?"}
  User -->|yes| Close["close if other blockers are clear"]
  User -->|no| Rework["rework or reject"]
```

```text
수용해. 이 작업 닫아.
```

거절할 수도 있다.

```text
수용하지 않아. 세션 만료 UX를 다시 잡아줘.
```

Acceptance는 approval, verification, Manual QA, residual-risk acceptance가 아니다.

## 이어서 하기

오래된 채팅을 뒤지지 말고 하네스 상태에서 재개한다.

```text
이 프로젝트의 active task 상태 보여줘.
TASK-0044 이어서 해. 하네스 상태부터 확인해.
```

재개할 때 확인할 질문은 두 가지다.

```mermaid
flowchart LR
  Resume["Resume request"] --> State["read harness state"]
  State --> Journey["show Journey Card"]
  Journey --> Next["what is the next action?"]
  Journey --> Stopped["why is work stopped?"]
  Stopped --> Action["continue, decide, reconcile, verify, or close"]
```

```text
지금 다음 행동은 무엇인가?
지금 멈춘 이유는 무엇인가?
```

문서에 메모를 남겼다면 이렇게 말한다.

```text
TASK 문서의 사용자 메모를 확인하고, 상태에 반영해야 할 항목을 reconcile해줘.
```

문서는 사람이 읽는 projection이다. 상태와 문서가 어긋난 것 같으면 projection freshness를 확인하고 state 기준으로 다시 요약하게 한다.
