# 에이전트 세션 흐름

## 이 문서로 할 수 있는 일

이 문서는 Harness를 쓰는 에이전트 세션이 사용자에게 어떻게 보여야 하는지 설명합니다. 무엇을 보여주고, 언제 묻고, 언제 계속하고, 언제 멈춰야 하는지를 다룹니다.

커넥터 계약, 전체 능력 프로필, MCP 스키마, 접점별 cookbook은 여기서 정의하지 않습니다. 그런 내용은 [Agent 통합 참조](../reference/agent-integration.md)와 [Surface Cookbook](../reference/surface-cookbook.md)이 담당합니다.

## 이런 때 읽기

에이전트가 상태, 막힘, 쓰기, 확인, 닫기를 사용자에게 어떻게 보여줘야 하는지 확인할 때 읽습니다.

## 읽기 전에

사용자 관점의 흐름을 먼저 보고 싶다면 [사용자 가이드](user-guide.md)를 먼저 읽습니다.

## 핵심 생각

사용자의 다음 판단에 영향을 주는 상태, 막힘, 판단, 다음 행동만 보여줍니다.

매 turn의 always-on context는 compact Harness envelope여야 합니다. 여기에는 active Task id와 mode, next safe action, active Change Unit summary, blocking decisions, write authority status, guarantee level, gate summary, projection freshness가 들어갑니다. Evidence, Run, Eval, Manual QA, artifact, log, screenshot, diff, large trace는 default로 ref와 짧은 outcome만 보여주고, 다음 action이 내용을 실제로 살펴봐야 할 때만 pull합니다.

## 세션 시작

Harness가 연결되어 있으면 사용자가 Harness 사용을 명시적으로 요청했을 때뿐 아니라 Harness가 추적해야 할 모양의 작업을 요청했을 때도 상태 확인이나 요청 정리로 시작합니다. 사용자가 꼭 "Harness"라고 말할 필요는 없습니다. 요청의 모양을 보고 판단하되, 첫 답변은 짧게 유지합니다.

일상적인 말로 들어온 요청도 범위, 판단, 근거, 닫기 상태가 계속 보여야 하는 모양이면 추적합니다.

- 제품 파일 쓰기나 프로젝트 상태를 바꾸는 작업
- 범위가 흐트러질 위험이나 모호한 요구사항
- 여러 파일 변경, 구조 변경, migration, 경계를 넘는 작업
- auth, security, billing, destructive/data-loss risk, privacy, compliance처럼 민감하거나, accessibility, design quality처럼 정책·품질 판단이 필요한 영역
- 사용자가 소유하는 제품 판단 또는 비용·호환성·보안·유지보수·migration·interface·dependency·위험 영향이 큰 중요한 기술 판단
- 근거, 검증, Manual QA, 수용, 남은 위험이 필요한 작업

작은 direct 작업은 가볍게 유지합니다. 질문에 답하거나, 코드를 살펴보거나, 결과를 설명하거나, 이미 좁은 모양이 분명한 작고 위험이 낮은 변경을 처리하는 데 불필요한 절차를 덧붙이지 않습니다.

보여줄 것:

- active 또는 예상 Task id와 모드: `advisor`, `direct`, `work`
- 현재 또는 제안 범위
- 범위 밖
- 다음 안전한 행동
- 진행을 막는 질문이 있다면 그 질문
- 쓰기가 가능하거나 가까울 때 write authority status
- guarantee level과 접점이 실제로 block할 수 있는 것 또는 detect만 할 수 있는 것
- compact gate, Manual QA, residual-risk, projection freshness status
- guard, freeze, careful mode가 관련될 때 실행 전에 실제로 막을 수 있는 것과 실행 뒤에만 감지할 수 있는 것

넓은 자연어 요청만으로 바로 제품 파일을 쓰기 시작하면 안 됩니다. 먼저 범위와 의도한 변경에 맞는 쓰기 권한을 확정해야 합니다.

## 이어가기

중요한 작업을 이어가기 전에는 Harness 상태를 읽고 현재 위치를 보여줍니다. 상태를 읽을 수 있는데 오래된 채팅만 보고 권한이나 범위를 재구성하면 안 됩니다.

좋은 이어가기 답변:

```text
활성 작업을 찾았습니다. 현재 범위는 X입니다. 다음 안전한 행동은 Y입니다. 제품 파일 쓰기는 아직 허용되지 않았습니다. 대기 중인 결정은 Z 하나입니다.
```

Projection, `source_state_version`, 읽기용 상태가 stale이거나 unknown이면 그 사실을 말하고, 거기에 의존하기 전에 refresh 또는 reconcile합니다. 기준 상태를 직접 읽을 수 있으면 그 상태에서 계속할 수 있지만, 읽기용 projection은 권한의 출처가 아니라고 알려야 합니다.

표시 문제는 구분해서 말합니다. Stale projection은 읽기용 card/report가 뒤처졌을 수 있으므로 신뢰할 수 있는 context로 쓰기 전에 refresh 또는 reconcile이 필요하다는 뜻입니다. Stale state, stale baseline, stale evidence는 실제 입력이 이동했거나 부족해져 write나 close를 막을 수 있다는 뜻입니다. MCP unavailable은 에이전트가 필요한 Harness/Core capability에 닿지 못한다는 뜻입니다. 그 capability가 다시 사용 가능해지기 전에는 기준 상태 변경, approval, gate update, projection repair, close를 주장하면 안 됩니다.

## 요청 정리

요청 정리는 사용자가 Harness 용어로 말하지 않아도 평범한 요청을 실제로 진행 가능한 작업 모양으로 바꾸는 단계입니다.

세션 시작에서 보는 것과 같은 작업 모양 신호를 살핍니다. 제품 파일 쓰기, 범위가 흐트러질 위험, 모호한 요구사항, 여러 파일이나 구조 변경, 민감하거나 정책·품질 판단이 필요한 영역, 사용자가 판단해야 하는 결정, 근거, 검증, Manual QA, 수용, 남은 위험이 필요한 작업이 여기에 해당합니다. 이런 신호가 있으면 평범한 요청을 예상 모드, 범위, 범위 밖, 다음 안전한 행동으로 바꿔 제안합니다.

다음 안전한 행동을 바꾸는 질문만 합니다. 긴 양식보다 추천이 딸린 막힘 질문 하나가 낫습니다.

질문하기 전에는 agent가 안전하게 직접 확인할 수 있는 답을 사용 가능한 최신 저장소, 코드베이스, 문서, Harness state에서 먼저 찾아봅니다. 이미 보이는 파일 경로, 기존 동작, 용어, 제약을 사용자에게 다시 설명해 달라고 요구하지 않습니다. 소스가 없거나 오래됐으면 그것을 권위 있는 현재 사실처럼 쓰지 말고, 불확실성으로 표시합니다.

한 번에 하나의 막힘 질문을 묻는다는 말이 구체화도 한 번이면 끝난다는 뜻은 아닙니다. 요청이 넓거나 설계 판단이 크면 목표, 범위, 비목표, 수용 기준, 영향받는 제품 영역, 사용자 화면이나 흐름, 모듈, interface, 민감 카테고리(sensitive categories), 사용자가 소유하는 제품 또는 중요한 기술 절충 판단, 검증 또는 Manual QA 기대 수준, 알려진 제품·구현·검증·QA·후속 위험이 첫 번째 안전한 Change Unit을 제안할 수 있을 만큼 잡힐 때까지 짧은 확인을 여러 차례 이어갈 수 있습니다.

각 막힘 질문은 무엇이 불확실한지, 가능한 선택지, agent의 추천안, 결정을 미뤄도 계속할 수 있는 일 또는 결정 전에는 진행하면 안 되는 이유를 함께 보여줘야 합니다. Agent가 둔 가정은 사용자가 소유하는 선택, Approval, QA 판단, 수용, 위험 수용과 따로 기록합니다.

좋은 요청 정리:

```text
이 변경이 설정 화면 문구 안에 머물면 direct로 처리할 수 있습니다. 계정 동작까지 바꾸면 work로 전환해야 합니다. 추천은 설정 문구만 direct로 시작하는 것입니다. 의도한 범위가 맞나요?
```

## advisor/direct/work로 분류하기

`advisor`는 읽기, 설명, 비교, 리뷰처럼 제품 파일을 쓰지 않는 일에 씁니다.

`direct`는 작고 위험이 낮은 일을 좁은 범위에서 빠르게 처리할 때 씁니다. `direct`도 제품 파일을 쓰기 전에는 활성 범위와 쓰기 권한이 필요하지만, 절차는 가벼워야 합니다.

`work`는 기능 추가, 구조 변경, 위험한 수정, 여러 파일 변경, 요구가 불명확한 일, 의미 있는 근거와 독립 검증이 필요한 일에 씁니다.

`direct` 작업이 커지면 같은 Task를 `work`로 전환하고 이유를 보여줍니다.

## 범위와 Change Unit

제품 파일을 쓰기 전에 활성 범위를 Change Unit으로 잡습니다. 사용자에게는 다음이 보여야 합니다.

- 포함되는 동작이나 파일
- 범위 밖 동작이나 파일
- 완료 조건
- 알려진 민감 영역
- 에이전트가 멈추고 물어야 하는 조건

첫 번째 안전한 Change Unit을 제안할 만큼 충분히 안다는 것은 위 항목들을 해소되지 않은 사용자 판단을 숨기지 않고 말할 수 있다는 뜻입니다. 아직 그러지 못하면 요청 정리를 이어가며 다음 막힘 질문을 하거나, 해소되지 않은 영역을 피하는 더 작은 Change Unit을 제안합니다.

Autonomy Boundary는 쓰기 권한이 아닙니다. 사용자가 다시 판단하지 않아도 에이전트가 어디까지 판단할 수 있는지만 설명합니다. 실제 제품 쓰기에는 여전히 의도한 변경과 맞는 쓰기 확인이 필요합니다.

멈춤과 허가를 설명할 때는 다음처럼 구분합니다.

| 개념 | 쉬운 질문 | 허용하는 것 | 허용하지 않는 것 |
|---|---|---|---|
| Change Unit scope | 어떤 작업 영역이 범위 안인가? | 작업이 둘러싼 동작, 파일, paths, tools, commands, network targets, sensitive categories를 이름 붙입니다. | 사용자 소유의 제품 판단이나 중요한 기술 판단을 결정하거나 그 자체로 Write Authorization을 만들지 않습니다. |
| Autonomy Boundary | 그 범위 안에서 agent가 무엇을 혼자 판단해도 되는가? | 포괄된 구현 세부사항은 추가 사용자 결정 없이 agent가 선택할 수 있게 합니다. | Paths, tools, commands, network, secrets, sensitive categories, approval, write authority를 부여하지 않습니다. |
| Approval | 이 민감한 단계를 진행해도 되는가? | 기록된 scope와 expiry 안에서 이름 붙인 sensitive action을 허용합니다. | 사용자 소유 판단, correctness, 위험 수용, Write Authorization을 대신하지 않습니다. |
| Write Authorization | 지금 이 정확한 write attempt를 해도 되는가? | 필요한 확인 뒤 Core가 compatible write attempt 하나를 허용했다는 기록입니다. | 재사용할 수 없고 scope, Autonomy Boundary, Approval을 넓히지 않습니다. |

Prompt나 status에서 "approved" 또는 "승인"이라는 말을 쓸 때는 실제 authority 또는 judgment path를 이름 붙입니다. Sensitive-action Approval, scope confirmation, Decision Packet resolution, residual-risk acceptance, final acceptance, Write Authorization status를 구분해서 말하고, "승인"을 모든 허가와 판단을 뭉뚱그리는 catch-all label로 쓰면 안 됩니다.

예시:

- Dependency install approval: install을 실행하거나 dependency 파일을 갱신해도 된다는 승인은 그 dependency가 올바른 architecture 선택이라는 결정이 아닙니다. 그 선택이 호환성, rollback, 비용, 유지보수에 영향을 주면 Decision Packet을 사용합니다.
- Secret access approval: 요청된 scope 안에서 secret을 읽거나 사용해도 된다는 승인은 secret 값을 artifacts, projections, exports, logs, screenshots, summaries에 노출해도 된다는 뜻이 아닙니다.
- Auth/system change approval: auth 파일, permission, system configuration을 만져도 된다는 승인은 session auth, JWT, social login, role model, lockout behavior, user notice를 선택하는 결정이 아닙니다.
- Public API change decision: API 방향이 해소됐다는 것은 이 Task의 contract 선택을 결정했다는 뜻입니다. Deployment 권한, merge 권한, 재사용 가능한 Write Authorization이 아닙니다.
- Final acceptance: 결과 수용은 추가 write를 authorize하거나, 새 sensitive action을 approve하거나, 빠진 evidence, QA, verification, Write Authorization을 나중에 충족시켜 주지 않습니다.

Autonomy Boundary 안에서는 agent가 기존 helper를 재사용할지, private function을 어떻게 나눌지, focused tests를 어디에 둘지, 합의된 결과에 맞는 보수적인 내부 접근을 고를지 같은 일상적인 구현 세부사항을 판단할 수 있습니다. Public API 또는 module contract 변경, security 또는 privacy trade-off, UX 또는 제품 trade-off, dependency나 migration 같은 중요한 기술 방향 선택, scope expansion, 남은 위험 수용 전에는 사용자 판단을 위해 멈춰야 합니다.

## 사용자 판단으로 막힐 때

사용자가 소유하는 제품 판단이나 중요한 기술 판단이 진행을 막고 있으면 Decision Packet을 보여주거나 요청합니다. 이를 넓은 승인 질문이나 막연한 "계속할까요?"로 바꾸면 안 됩니다.

사용자가 보는 Decision Packet에는 다음이 있어야 합니다.

- 왜 지금 이 결정이 필요한지
- 정확히 무엇을 결정해야 하는지
- 옵션과 장단점
- 추천과 불확실성
- 결정을 미루면 계속할 수 있는 일, 또는 결정 전에는 진행하면 안 되는 이유
- 남는 위험이나 후속 작업

유용한 예시:

- 로그인 실패 UX: inline message, toast, modal/layer를 비교하고, 흐름, 접근성, 방해 정도, 문구 위험을 기준으로 하나를 추천합니다. 결정을 미루면 backend auth 작업은 계속할 수 있지만 최종 로그인 실패 경험이 완료됐다고 말하면 안 됩니다.
- 로그인 실패 문구: 보안을 우선한 짧은 문구, 복구를 쉽게 설명하는 평문 문구, field-level guidance를 더 구체적으로 주는 방식을 비교합니다. 계정 enumeration 위험, 명확성, support burden, product tone을 기준으로 추천합니다. 결정을 미루면 validation wiring은 계속할 수 있지만 release-ready copy와 Manual QA는 열어 둬야 합니다.
- Product taste와 Manual QA 필요성: 사람이 시각적으로 확인해야 하는 polished interaction과 test 및 browser smoke로 확인 가능한 더 보수적인 동작을 비교합니다. Taste trade-off, QA 비용, 사용자 영향, Manual QA를 미뤘을 때 계속할 수 있는 일 또는 결정 전에는 진행하면 안 되는 이유를 설명합니다.
- 세션 방식: session auth, token auth, social login을 비교하고, 폐기 가능성, CSRF/XSS 노출, client 호환성, 운영 복잡도, migration 비용을 설명합니다. 결정을 미루면 session model에 약속을 만들지 않는 범위에서만 form scaffold를 계속할 수 있습니다.
- Dependency 또는 migration 선택: dependency를 추가할지, 기존 utility를 쓸지, capability를 미룰지 비교합니다. Schema/data-model migration에서는 additive migration, compatibility shim, breaking cleanup을 비교합니다. 영향 범위, rollback, test boundary, 유지보수 비용을 설명합니다.
- Public API/interface 또는 module boundary: 현재 interface를 유지할지, 좁은 extension을 추가할지, 책임을 module boundary 너머로 옮길지 비교합니다. Caller 영향, compatibility risk, boundary test, future-change cost를 설명합니다.
- 보안 민감 변경: secret 접근, 권한 변경, 데이터 export에 대한 approval은 approval boundary일 뿐입니다. 역할, 필드, redaction, audit logging, retention, rollback, user notice에는 별도의 제품 또는 보안 판단이 여전히 필요할 수 있습니다.
- QA 또는 verification waiver: 해당 Task의 Decision Packet 또는 필요한 기록된 판단 경로를 사용하고, 생략하는 확인이나 대상, 수용하는 위험, 후속 작업, 관련 refs, close 영향을 이름 붙입니다. 예를 들어 copy-only 변경에서 mobile Safari Manual QA를 면제한다면 viewport wrapping 위험을 수용하고 release 전 browser pass를 후속 작업으로 남깁니다.
- close 전 남은 위험 수용: 남은 한계, 이미 있는 근거, 그래도 close가 가능하다고 볼 수 있는 이유, 남는 후속 작업을 보여줍니다.

가능하면 한 번에 하나의 막힘 질문만 묻습니다.

## 제품 파일 쓰기

제품 파일을 쓰기 전에는 에이전트가 의도한 작업에 대한 쓰기 권한을 확인해야 합니다.

짧은 쓰기 권한 요약을 보여줍니다.

```text
쓰기 권한: src/auth/login.ts와 tests/auth/login.test.ts에 허용됨
범위 근거: email login Change Unit
한계: 협조형 접점이라서 범위를 벗어난 쓰기는 사후 changed-path validation으로만 감지합니다.
```

Cooperative 또는 detective hold를 실행 전에 막는 것처럼 설명하면 안 됩니다. 지시로 write를 보류한다고 말하거나, connected profile이 해당 validation을 지원할 때 실행 뒤에 위반을 감지할 수 있다고 말합니다. Preventive 표현은 해당 operation에 대해 입증된 실행 전 차단이 있을 때만 씁니다.

쓰기 권한이 막혔거나, 확인할 수 없거나, 최신이 아니거나, 의도한 변경과 맞지 않으면 제품 파일 쓰기를 멈추고 가장 작은 해소 방법을 설명합니다.

문서 유지보수 편집은 별도의 docs-only 흐름입니다. 이 문서의 제품 파일 쓰기 흐름이 아니라
[문서 작성 가이드](../maintain/authoring-guide.md)가 다룹니다.

## 근거와 확인

조언, 변경, 실행, 리뷰 뒤에는 결과를 필요한 수준으로 기록합니다. 사용자가 보는 근거는 수용 기준이나 명시된 작업 목표와 연결되어야 합니다.

좋은 근거 표시:

```text
근거:
- AC-01: 이메일 필드가 있는 로그인 폼 렌더링을 RUN-008 테스트 결과가 뒷받침합니다.
- AC-02: 로그인 실패 메시지는 아직 Manual QA가 필요합니다.
```

근거가 부족하면 어떤 기준이나 주장이 뒷받침되지 않는지 말합니다. 단순히 "근거 게이트가 실패했습니다"라고만 말하지 않습니다.

근거 표시는 refs-first로 합니다. Evidence, Run, Eval, Manual QA, artifact, log, screenshot, diff, trace ref와 짧은 outcome을 보여주고, 사용자나 evaluator가 다음 action을 결정하기 위해 내용을 inspect해야 할 때만 excerpt를 embed합니다.

## 검증, Manual QA, 남은 위험, 수용

에이전트 답변에서는 이 항목들을 분리해서 보여줘야 합니다.

| 항목 | 사용자가 이해해야 하는 것 |
|---|---|
| 근거 | 결과나 수용 기준이 충족됐다는 주장을 무엇이 뒷받침하는가. |
| 검증 | correctness를 무엇이 확인했고, 그 verifier가 detached assurance에 충분히 독립적이었는가. |
| Manual QA | 사람이 봐야 하는 품질을 무엇으로 확인했는가. |
| 수용 | 그런 판단이 요구되는 경우 사용자가 결과를 받아들이는가. |
| 남은 위험 | 어떤 불확실성, 한계, 확인하지 못한 조건, 장단점이 남았는가. |

검증은 기술적으로 무엇을 어떻게 확인했는지에 답합니다. 같은 세션에서 하는 자체 검토는 유용할 수 있지만, 분리된 검증은 아닙니다. 테스트 통과는 근거가 될 수 있고 검증을 뒷받침할 수 있지만, 테스트만으로 Manual QA가 수행됐다고 말하면 안 됩니다.

Manual QA는 UX, 흐름, 시각 결과, 문구, 접근성 해석처럼 사람이 봐야 하는 품질을 확인했는지에 답합니다. Manual QA 결과가 실제로 기록되었거나 타당하게 면제된 것이 아니라면 browser smoke, screenshot capture, verifier note를 Manual QA처럼 보여주면 안 됩니다.

남은 위험은 알려진 한계, 불확실성, 확인하지 못한 조건, 장단점입니다. 위험을 수용하고 닫거나 최종 수용을 하기 전에는 반드시 보여야 합니다. 위험 수용은 assurance를 높이지 않고, verification이나 QA를 대체하지 않습니다.

최종 수용은 Task 경로가 요구할 때 사용자가 결과를 받아들이는 판단입니다. 승인, 검증, QA, 남은 위험 수용, correctness 증명과 다릅니다.

닫기 적용 예시:

- Direct 작업: changed files, evidence refs, self-check, escalated 여부를 보여줍니다. 조건을 충족하는 Eval 없이 detached verified라고 부르면 안 됩니다.
- UI/UX 작업: 테스트, browser smoke, Manual QA, 수용을 각 줄로 분리합니다. Manual QA를 면제한다면 생략한 대상, 수용하는 위험, 후속 작업을 보여줍니다.
- Auth 또는 security 작업: approval을 security 또는 product decision과 분리해서 보여준 뒤 근거와 검증을 보여줍니다. Secret이나 permission을 만지는 approval은 redaction, audit, role, retention, user notice 선택을 대신하지 않습니다.
- Public API 작업: caller compatibility, migration 또는 documentation 영향, 근거, 검증을 따로 보여줍니다. 테스트 통과만으로 API contract 결정이 끝난 것은 아닙니다.
- Risk-accepted close: 남은 한계, 이미 있는 근거, 빠졌거나 면제된 verification 또는 QA, 수용된 위험, 후속 작업을 보여줍니다. 결과를 detached verified처럼 표시하면 안 됩니다.

## 닫기

닫기는 활성 Task 경로의 막힘이 사라졌을 때만 합니다.

성공적으로 닫기 전에 보여주거나 확인할 것:

- 범위와 결과가 맞는지
- 근거가 충분한지 또는 근거 요구가 없는지
- 검증 상태 또는 수용된 검증 위험
- Manual QA 상태 또는 타당한 면제
- 닫기에 영향을 주는 남은 위험 표시 또는 알려진 close-relevant residual risk가 없다는 보고
- 필요한 경우 최종 수용 기록

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
사용자가 소유하는 제품 판단 때문에 막혔습니다. 빈 상태는 바로 설정을 유도할 수도 있고, 데이터가 생길 때까지 조용히 둘 수도 있습니다. 추천은 설정 유도이며 불확실성은 낮습니다. 결정을 미루면 데이터 로딩 리팩터링은 끝낼 수 있지만 빈 상태 UX는 확정할 수 없습니다.
```

나쁜 결정 멈춤:

```text
계속하려면 승인해주세요.
```

좋은 approval 요약:

```text
승인된 것: CU-03 안에서 zod를 install하고 package lock을 갱신하는 일. 아직 결정되지 않은 것: validation을 public API의 일부로 만들지 여부. 다음 안전한 행동: 정확한 dependency-file write에 대해 prepare_write를 다시 시도합니다.
```

나쁜 approval 요약:

```text
승인됐으니 auth 설계를 끝내고, 구현하고, 배포하고, Task를 닫겠습니다.
```

좋은 닫기 막힘:

```text
온보딩 문구 Manual QA와 AC-02 근거가 없어 닫기가 막혀 있습니다. 가장 작은 해소 방법은 브라우저 smoke 확인을 실행하고 문구를 받아들일 수 있는지 Manual QA 결과로 기록하는 것입니다.
```

나쁜 닫기 막힘:

```text
상태 모델이 게이트를 만족하지 않습니다.
```

나쁜 닫기 주장:

```text
테스트가 통과했으니 Manual QA와 수용도 끝났습니다.
```

좋은 쓰기 멈춤:

```text
새 auth 파일에 대한 쓰기 권한이 없어 제품 파일 쓰기를 멈춥니다. 가장 작은 해소 방법은 범위에 해당 파일을 포함하도록 업데이트하거나 변경을 기존 login module 안으로 줄이는 것입니다.
```

나쁜 쓰기 멈춤:

```text
Harness가 불만이라 멈췄습니다.
```
