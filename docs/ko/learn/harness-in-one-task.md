# 하나의 작업으로 보는 Harness

## 이 문서로 할 수 있는 일

이 문서는 엄격한 참고 정의를 읽기 전에, 두 개의 구체적인 작업 흐름으로 Harness를 설명합니다.

읽고 나면 Task, Change Unit, Decision Packet, Approval, Write Authorization, 근거, 검증, Manual QA, 수락, 남은 위험, 닫기가 왜 필요한지 감을 잡을 수 있습니다. 내부 기록 세부사항을 몰라도 흐름을 따라갈 수 있어야 합니다.

이 문서는 Learn 문서입니다. 문서 세트가 구현 계획에 사용할 수 있다고 승인되기 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture 파일, runtime data를 만들라는 뜻이 아닙니다. 첫 구현/증명 대상은 계속 Kernel Smoke입니다. Agency-Hardened MVP와 post-MVP automation은 owner 문서가 승격하고 증명하기 전까지 범위 밖입니다.

## 이런 때 읽기

엄격한 용어를 배우기 전에, 구체적인 작업 흐름으로 Harness를 이해하고 싶을 때 읽습니다.

## 읽기 전에

[개요](overview.md)를 먼저 읽는 것을 권장하지만 필수는 아닙니다. 이 문서는 Harness가 중요한 작업 사실을 대화 밖에 오래 남기는 시스템이라는 점만 알고 있다고 가정합니다.

## 핵심 생각

Harness는 하나의 Task 이야기로 보면 이해하기 쉽습니다. 무엇을 하려는지 이름 붙이고, 무엇을 바꿀 수 있는지 경계를 정하고, 필요한 순간에 사용자 판단을 묻고, 근거를 기록하고, 보이는 막힘이 처리되었을 때만 닫습니다.

## 왜 예시를 먼저 보는가

Harness는 용어부터 보면 실제보다 무겁게 느껴질 수 있습니다. 참고 문서는 로컬 시스템이 상태를 기록하고 게이트를 확인하는 방식을 정확히 정의해야 하므로 엄격합니다. 하지만 사용자가 Harness를 만날 때는 보통 몇 가지 실용적인 질문으로 보입니다.

1. 무엇을 하려는가?
2. 어디까지 바꿔도 되는가?
3. 지금 사용자의 판단이 필요한가?
4. 무엇이 바뀌었고, 무엇이 그것을 뒷받침하는가?
5. 아직 어떤 불확실성이 남았는가?
6. 이 Task를 닫아도 되는가?

아래 예시는 여기서 시작합니다. 작은 작업은 가볍게 유지하고, 큰 작업은 따라갈 수 있을 만큼 구조화하는 방식을 보여 줍니다.

## 예시 A: `direct` 작업

### 사용자 요청: "버튼 문구를 바꿔줘."

사용자가 작은 UI 문구 문제를 가리키며 말합니다.

```text
버튼 문구를 바꿔줘.
```

주변 맥락상 대상이 분명하다고 가정합니다. 예를 들어 프로필 페이지의 버튼이 지금 "저장"이고, 이를 "프로필 업데이트"로 바꿔야 합니다.

### 접수

에이전트는 요청을 쉬운 말의 Task로 바꿉니다.

```text
Task: 프로필 페이지 버튼 문구를 "저장"에서 "프로필 업데이트"로 바꾼다.
예상 모드: direct.
```

Task가 있어야 작업이 하나의 오래 남는 단위가 됩니다. Task가 없으면 변경, 확인, 닫기 판단이 모두 대화 안에만 남습니다.

### 범위

에이전트는 파일을 건드리기 전에 범위를 좁힙니다.

```text
포함: 프로필 페이지 버튼 문구, 그리고 그 문구만 직접 확인하는 snapshot 또는 문구 테스트.
제외: 프로필 페이지 재설계, 저장 동작 변경, localization 전략 변경.
```

작은 작업에서도 범위는 중요합니다. 버튼 문구가 여러 화면에서 공유되는 디자인 token으로 만들어진다는 사실이 드러나면, 이 작업은 더 이상 단순한 문구 수정이 아닐 수 있습니다.

### `direct`로 분류하기

Harness는 이 작업을 `direct`로 봅니다. 변경이 작고, 위험이 낮고, 자체 확인이 쉽기 때문입니다. `direct`는 "기록하지 않는다"는 뜻이 아닙니다. 필요한 기록을 가볍게 유지한다는 뜻입니다.

사용자에게 보이는 예산은 작습니다. Scope를 이름 붙이고, write를 위한 활성 최소 Change Unit을 유지하고, write authority를 확인하고, 좁은 변경을 적용하고, self-check한 뒤 결과를 보고하면 됩니다. 한 줄 문구 변경 때문에 사용자가 양식을 채울 필요는 없습니다.

에이전트가 이 문구가 checkout, billing, profile 화면에 모두 영향을 준다는 사실을 발견하면 멈춰야 합니다. 그때는 같은 Task를 `work`로 옮기는 편이 맞습니다.

### 최소 Change Unit

Change Unit은 요청을 끝내는 데 필요한 가장 작은 제품 쓰기 경계입니다.

```text
Change Unit: 프로필 버튼 문구만.
예상 경로: 프로필 화면 파일, 직접 관련된 문구 테스트가 있으면 그 테스트.
```

Change Unit은 작은 요청이 조용히 전체 UI 정리로 커지는 것을 막습니다.

비슷하게 작은 작업에서는 에이전트가 요청에서 최소 Change Unit 내용을 자동으로 만들 수 있습니다. 아래 예시는 설명용이며 새 schema가 아닙니다.

```text
문서 오탈자: 이름 붙은 문서의 한 문장 수정; 의미나 계약 변경 없음.
UI 문구만 변경: label과 직접 관련된 copy test 변경; 동작, layout, localization strategy 변경 없음.
좁은 test 변경: 보고된 case에 대한 regression test 하나 추가; Task가 escalation되지 않는 한 implementation edit 없음.
```

### 제품 파일을 쓰기 전에 prepare_write 확인하기

수정하기 전에 에이전트는 이 쓰기가 지금 허용되는지 Harness에 확인합니다. 사용자가 보는 요약은 구체적이어야 합니다.

```text
Write Authorization: 프로필 화면 파일과 관련 문구 테스트에 대해 allowed.
```

수정하려는 파일이 Change Unit 밖이면 쓰기는 멈춰야 합니다. 사용자는 예상 밖의 수정 대신 범위 질문을 봐야 합니다.

실행 뒤 범위 밖 변경 경로가 발견되면 `direct` 결과에 자연스럽게 포함하면 안 됩니다. 에이전트는 불일치를 보여주고, 추가 변경을 제거하거나 분리하거나, 범위 결정을 묻거나, 같은 Task를 `work` 쪽으로 옮겨야 합니다.

### 변경 기록

수정 뒤 에이전트는 실제로 일어난 일을 기록합니다.

```text
변경: 프로필 페이지 버튼 문구.
변경 경로: 프로필 화면 파일, 필요한 경우 문구 테스트.
```

이 기록은 구현을 Task와 Change Unit에 연결합니다. 나중에 전체 대화를 다시 읽지 않아도 상태 요약을 만들 수 있게 해 줍니다.

### 가벼운 근거

`direct` 작업의 근거는 단순할 수 있습니다.

```text
근거: diff에서 문구 변경 확인, 문구 테스트 통과 또는 component 렌더링 자체 확인.
```

목표는 절차를 늘리는 것이 아닙니다. "끝났다"는 말을 나중에도 확인할 수 있을 정도의 근거를 남기는 것입니다.

### 자체 확인

에이전트는 좁은 결과를 확인합니다.

```text
자체 확인(Self-check): 프로필 버튼이 이제 "프로필 업데이트"로 표시된다.
```

자체 확인(Self-check)은 유용하지만 독립 검증은 아닙니다. `direct` 작업은 보통 사용자가 추가 검증을 요청하지 않는 한 자체 확인(self-checked) 수준으로 닫힙니다.

### 닫기

범위가 좁게 유지되었고, 쓰기가 허용되었고, 변경 경로가 Change Unit과 맞고, 가벼운 근거가 있으며, 닫기에 영향을 주는 알려진 위험이 없다면 Harness는 Task를 닫을 수 있습니다.

닫기 요약은 간단해야 합니다.

```text
`direct` 작업으로 닫음. 자체 확인(self-checked). 남은 위험: 이번 close에는 없음.
```

"알려진 닫기 관련 위험이 없다"는 명시적인 상태 claim이지, 숨겨진 위험을 무시해도 된다는 뜻이 아닙니다. Harness가 아직 사용자에게 보이지 않은 close-relevant risk를 알고 있다면, 표시는 그 risk가 아직 보이지 않는다고 말하고 risk와 refs를 보여줄 때까지 close 또는 acceptance를 막아야 합니다.

### 사용자가 보는 것

사용자는 내부 장치가 아니라 짧은 결과를 봐야 합니다.

```text
프로필 버튼 문구를 "프로필 업데이트"로 바꿨습니다.
RUN-031과 diff ART-DIFF-031로 렌더링된 문구를 확인했습니다.
Write Authorization WA-031이 consumed됐고, Evidence Manifest EM-031이 direct claim을 뒷받침합니다.
작은 `direct` 작업으로 닫았습니다. 남은 위험: 이번 close에는 없음.
```

### 참고 문서가 더 엄격하게 정의하는 것

참고 문서는 이 단순한 흐름 뒤의 정확한 규칙을 정의합니다. `direct`와 `work`의 차이, 활성 Change Unit의 조건, `prepare_write`가 Write Authorization을 만드는 방식, 변경 경로 기록 방식, `direct` 작업이 self-checked로 닫힐 수 있는 조건을 더 엄격하게 다룹니다.

이 튜토리얼에서는 일부러 그 세부 규칙을 펼치지 않습니다.

## 예시 B: `work` 작업

### 사용자 요청: "로그인 플로우에 remember me를 추가해줘."

사용자가 말합니다.

```text
로그인 플로우에 remember me를 추가해줘.
```

겉으로는 간단해 보이지만 이 작업은 인증 동작, 세션 유지 시간, UI 문구, 저장 방식, 테스트, 보안 정책에 닿을 수 있습니다. Harness는 구현 전에 작업의 모양을 보이게 만들어야 합니다.

### 접수

에이전트는 Task를 시작합니다.

```text
Task: 로그인 플로우에 remember me 동작을 추가한다.
예상 모드: work.
```

그리고 가장 먼저 도움이 되는 질문을 쉬운 말로 묻습니다.

```text
"remember me"가 이 기기에서 로그인 세션을 더 오래 유지한다는 뜻인가요, 이메일 주소를 기억한다는 뜻인가요, 아니면 둘 다인가요?
```

### 범위 잡기

에이전트는 첫 범위를 제안합니다.

```text
포함: 로그인 폼 checkbox, 선택된 remember-me 동작, 해당 동작의 test, 사용자에게 보이는 문구.
제외: passwordless login, account recovery, 전체 session-management 재설계.
```

이 범위가 Change Unit의 출발점이 됩니다. Change Unit은 단순한 파일 목록이 아닙니다. Task를 만족시키기 위해 에이전트가 바꿀 수 있는 제한된 제품 조각입니다.

### 절충점이 드러나는 순간

에이전트는 두 가지 해석이 모두 그럴듯하다는 것을 발견합니다.

1. 사용자의 이메일만 기억한다. 위험은 낮고 편의성은 좋아지지만, 로그인 상태를 유지하지는 않는다.
2. 현재 기기에서 세션을 더 오래 유지한다. 많은 사용자가 기대하는 remember me에 가깝지만, 세션 유지 시간과 저장 방식에 대한 보안 판단이 필요하다.

이 선택이 제품 동작이나 위험을 바꾼다면 에이전트가 조용히 고르면 안 됩니다.

### Decision Packet

사용자 판단이 진행을 막을 때 Harness는 Decision Packet을 사용합니다. Packet은 읽기 쉬워야 합니다.

```text
필요한 결정: 이 제품에서 "remember me"는 무엇을 의미해야 하는가?
Option A: 이메일만 기억한다.
Option B: 현재 기기에서 세션을 더 오래 유지한다.
추천: 제품이 세션 유지 위험을 받아들이고 그 선택을 기록할 수 있을 때만 Option B를 선택한다.
```

Decision Packet이 필요한 이유는 이것이 단순한 수정 승인 문제가 아니기 때문입니다. 제품과 보안의 절충이며, 그 선택은 사용자가 소유합니다.

### Change Unit

사용자가 선택하면 에이전트는 그 결정에 맞게 Change Unit을 정리합니다.

```text
Change Unit: 로그인 폼 checkbox, 선택된 remember-me 동작, test, 직접 관련된 문구.
```

사용자가 세션 연장을 선택하면 Change Unit에 세션 유지 코드와 보안 관련 test가 포함될 수 있습니다. 이메일만 기억하는 선택이라면 범위는 더 좁게 유지됩니다.

### Write Authorization

제품 파일을 쓰기 전에 에이전트는 의도한 수정이 활성 Task, Change Unit, sensitive-action Approval, 해결된 결정과 맞는지 Harness에 확인합니다.

사용자에게 보이는 요약은 무엇이 허용되고 무엇이 아닌지 말해야 합니다.

```text
Write Authorization: 로그인 폼, 세션 유지 코드, 관련 test에 대해 allowed.
Not allowed: 관련 없는 account recovery 또는 전체 auth 재설계.
```

선택한 동작에 sensitive-action Approval이 필요하면 Harness는 쓰기 전에 멈추고 별도 Approval을 요청해야 합니다. Sensitive-action Approval은 "이 민감한 행동을 진행해도 되는가?"에 답합니다. Decision Packet, 테스트, QA, 남은 위험을 받아들이는 판단, 최종 수락을 대신하지 않습니다.

### 구현

에이전트는 허용된 경계 안에서 구현합니다.

1. Checkbox와 문구를 추가한다.
2. 선택된 persistence 동작을 추가한다.
3. 선택된 동작을 확인하는 test를 추가하거나 수정한다.
4. 새 범위 결정 없이 관련 없는 auth 정리를 하지 않는다.

현재 세션 시스템이 더 큰 재설계 없이는 선택된 동작을 지원할 수 없다는 사실이 드러나면, 에이전트는 멈추고 더 작은 Change Unit 또는 새 Decision Packet을 제안해야 합니다.

### 근거

근거는 주장과 기록을 연결합니다.

```text
근거: 로그인 폼과 세션 코드의 diff, remember-me 동작 테스트 출력, 구현 실행 메모.
```

근거 덕분에 사용자는 나중에 "remember me가 동작한다는 주장을 무엇이 뒷받침하지?"라고 물을 수 있습니다. 답은 대화 기억이 아니라 구체적인 기록이어야 합니다. 그 답은 수용 기준과 completion conditions를 Run refs, artifact refs, 다른 supporting refs에 map해야 합니다. 어딘가에 artifact가 많다는 것만으로는 충분하지 않고, Markdown report가 "covered"라고 말하는 것만으로도 충분하지 않습니다.

이 쉬운 생각의 엄격한 세부사항은 커널의 Evidence Gate와 Evidence Manifest, artifact registration과 storage integrity, conformance proof에 걸쳐 있습니다.

### 검증

`work`에서는 사용자가 검증 위험을 명시적으로 받아들이지 않는 한 `direct` 자체 확인(self-check)보다 더 강한 확인이 기대됩니다.

유용한 검증은 이런 모양일 수 있습니다.

```text
분리된 검증: 별도 확인이 remembered session은 브라우저 재시작 후에도 유지되고, non-remembered session은 유지되지 않음을 확인한다.
```

분리된 검증을 실행할 수 없다면 에이전트는 그 사실을 분명히 말하고 남은 검증 위험을 보여야 합니다. 검증 없이 닫는 것은 위험을 받아들이고 닫는 것이지 분리된 검증이 아닙니다.

엄격한 세부사항은 Verification Gate 의미, assurance level, Eval 및 tool schema, detached-verification independence, conformance fixture에 걸쳐 있습니다.

### Manual QA

Manual QA는 사람이 실제 경험을 확인했는지 묻습니다.

```text
Manual QA: 로그인 화면의 checkbox가 분명히 보이고, 문구가 이해 가능하며, keyboard와 screen-reader 흐름이 유지되고, remembered-session 동작이 선택한 옵션과 맞는다.
```

Manual QA가 필요한 이유는 테스트가 통과해도 경험이 혼란스럽거나, 화면에서 잘리거나, 접근성이 나쁘거나, 사용자의 기대와 다를 수 있기 때문입니다.

엄격한 세부사항은 Manual QA policy, QA Gate 의미, Manual QA record와 tool shape, conformance proof에 걸쳐 있습니다.

### 남은 위험

결과 수락 또는 위험을 받아들이고 닫기 전에 에이전트는 알려진 남은 불확실성을 보여 줍니다.

```text
남은 위험: 세션 동작은 로컬 브라우저 경로에서 확인했지만, 지원하는 모든 브라우저 정책 조합에서는 확인하지 않았다.
```

닫기에 영향을 주는 알려진 남은 위험이 없다면 에이전트는 그 사실을 분명히 말해야 합니다. 알려진 위험을 숨기는 것과 알려진 위험이 없는 것은 다릅니다.

### 수락

최종 수락이 필요한 경로라면 사용자는 근거, 검증 또는 받아들인 검증 위험, Manual QA 상태, 남은 위험을 본 뒤 결과를 받아들입니다.

```text
수락합니다. remember-me 동작은 선택한 옵션과 맞고, 표시된 남은 위험도 받아들일 수 있습니다.
```

수락은 sensitive-action Approval과 다릅니다. Approval은 민감한 단계를 진행하게 할 수 있지만, 수락은 완료된 결과가 충분히 좋은지 판단합니다.

### 닫기

Harness는 관련 blocker가 처리된 뒤에만 닫습니다. 범위가 맞고, 결정이 해결되었거나 유효하게 미뤄졌고, 쓰기가 허용되었고, 근거가 Task에 충분하고, 검증과 QA가 통과했거나 명시적으로 처리되었고, 남은 위험이 보였고, 필요한 수락이 기록되어야 합니다.

닫기 요약은 짧아야 합니다.

```text
`work` 작업으로 닫음. 근거 기록됨. 검증과 Manual QA 처리됨. 필요한 경우 남은 위험을 표시하고 받아들인 판단을 기록함.
```

### 사용자가 보는 것

사용자는 참고 문서가 아니라 작업의 흐름을 봐야 합니다.

```text
remember me를 현재 기기의 세션 연장으로 구현했습니다.
로그인 폼, 세션 유지 코드, 테스트를 변경했습니다.
Evidence Manifest EM-009가 RUN-018과 ART-TEST-018로 AC-01과 AC-02를 뒷받침합니다.
Eval EVAL-012가 remembered session과 non-remembered session을 검증했습니다.
Manual QA MQA-006이 로그인 화면 흐름에 대해 통과했습니다.
Residual Risk RISK-004가 표시되고 DEC-022에서 받아들여졌습니다. 지원하는 모든 브라우저 정책 조합에서는 확인하지 않았다는 위험입니다.
최종 수락은 DEC-023에 기록됐고, `work` 작업으로 닫았습니다.
```

### 참고 문서가 더 엄격하게 정의하는 것

참고 문서는 모드 규칙, Decision Packet compatibility, sensitive-action Approval 처리, Write Authorization 동작, 근거 충분성, 검증 독립성, QA 게이트, 남은 위험 표시, 수락 시점, 닫기 의미를 정확히 정의합니다. Evidence, Verification, Manual QA는 각각 둘 이상의 reference owner에 걸치므로, 아래 표에서는 그 경계를 짧게만 보여 줍니다.

이 튜토리얼은 그 조각들이 왜 존재하는지만 보여 줍니다.

## 자주 만나게 되는 다른 작업 모양

위의 두 흐름은 기준점일 뿐, 모든 상황의 목록은 아닙니다. Harness는 여러 종류의 작업에서도 실용적으로 보여야 합니다.

- Leaf code fix는 여전히 `direct`일 수 있습니다. "date formatter에서 null crash를 고쳐줘" 같은 요청이 function 하나와 focused test 안에 머문다면, 변경 경로 요약, test 출력, 자체 확인(self-check)으로 닫을 수 있습니다. 고친 결과가 public behavior나 shared contract를 바꾸면 같은 Task를 `work` 쪽으로 옮겨야 합니다.
- Evidence shape는 task shape를 따라야 합니다. Advisor work는 recorded evidence가 요청되지 않는 한 보통 cited sources만 있으면 됩니다. Direct docs-only work는 changed path, diff 또는 patch summary, self-check를 사용할 수 있습니다. Direct code는 focused test, command, log, 또는 automated check가 적용되지 않는다는 reason을 더합니다. Work feature는 각 criterion을 Run과 artifact refs에 map합니다. UI/UX/copy work에는 visual evidence와 Manual QA가 필요할 수 있습니다. Sensitive work는 Approval과 redaction context를 correctness와 분리합니다. Verification-required work에는 current evidence를 review한 Eval이 필요합니다.
- UI/UX 선택에는 Decision Packet이 필요할 수 있습니다. Checkout error를 inline message, toast, modal/layer 중 어디에 보여줄지 선택해야 한다면 flow 방해 정도, 접근성, 문구 위험, 제품 톤을 비교해야 합니다. Backend validation은 최종 경험을 확정하지 않는 범위에서 계속할 수 있지만, UX가 완료됐다고 말하면 안 됩니다.
- Auth 선택은 제품 판단과 보안 판단이 섞입니다. Session cookie, JWT, social login 중 무엇을 쓸지에 따라 폐기 가능성, CSRF/XSS 노출, client 지원, 운영 비용이 달라집니다. 실패한 로그인 문구도 비슷합니다. 일반적인 문구, 더 구체적인 문구, hybrid 문구 중 무엇을 고르느냐에 따라 account-enumeration 위험, 명확성, 지원 부담, 톤이 달라집니다.
- Dependency 추가에는 사용자 답이 두 개 필요할 수 있습니다. Install 또는 dependency 파일 갱신을 허용하는 sensitive-action Approval과, 그 dependency를 아키텍처 방향으로 채택할지 결정하는 Decision Packet은 다릅니다. 호환성, rollback, 비용, 유지보수 영향이 있으면 별도 결정이 필요합니다.
- Public API 변경은 test 통과만으로 충분하지 않습니다. 필수 request field를 추가하거나, response field를 바꾸거나 제거하거나, error code를 바꾸거나, caller path를 제거한다면 compatibility 또는 breaking-change Decision Packet, migration note, caller-impact evidence, 관련 경계에서의 verification이 필요할 수 있습니다.
- Schema 변경은 migration 근거와 rollback risk를 보여줘야 합니다. Column을 추가하는 additive migration은 test된 migration으로 낮은 위험에 머물 수 있습니다. 파괴적인 cleanup이나 data backfill은 명시적인 사용자 판단, backup 또는 rollback note, 기존 shape와 새 shape를 모두 다뤘다는 evidence가 필요할 수 있습니다.
- Secret access는 secret 노출이 아닙니다. Approval은 Task 안에서 secret을 읽거나 사용할 수 있게 할 수 있지만, Evidence, artifact, projection, export, log, screenshot, summary에는 raw value가 아니라 redacted handle, omission note, nonsecret fact를 써야 합니다.
- Manual QA는 사람의 판단을 위한 것입니다. UX, 문구, accessibility 해석, 시각적 완성도, 제품 감각(product taste)은 사람이 결과를 봐야 할 수 있습니다. QA를 면제한다면 생략한 대상, 받아들이는 위험, 후속 작업, 닫기 영향을 이름 붙여야 합니다. 이는 test 통과와 같지 않습니다.
- 복구 상황은 눈에 보이되 평범하게 처리되어야 합니다. MCP를 사용할 수 없으면 Harness/Core에 다시 닿거나 사용할 수 있는 접점(surface)으로 옮길 때까지 기준 상태 변경, 제품 파일 쓰기, gate 갱신을 보류하고, Approval, 결과 수락, 남은 위험을 받아들이는 판단, 닫기가 처리됐다고 주장하지 않습니다. 읽기용 보기(Projection)가 stale이지만 Core state가 current라면, stale projection을 기준으로 삼지 말고 읽기용 보기를 refresh 또는 reconcile합니다. 관리 영역(managed block)을 사람이 직접 고쳤다면 표시 편집이 state를 바꾼 척하지 말고 Reconcile로 보냅니다.
- Evidence는 실제적인 이유로 stale이 될 수 있습니다. Baseline이 움직였거나, supporting run 또는 eval 뒤에 file이 바뀌었거나, Approval이 drift 또는 expire됐거나, artifact가 missing 상태이거나, relevant design record가 바뀐 경우입니다. Repair는 report prose를 고치는 것이 아니라 supporting refs를 refresh하거나 replace하는 것입니다.
- 같은 세션에서 하는 검토(review)는 유용하지만 detached verification은 아닙니다. 에이전트는 이를 자체 확인(self-check) 또는 stewardship signal로 사용할 수 있습니다. Detached verification에는 충분히 독립적인 Eval, verifier, session, review boundary가 필요합니다.

## 같은 개념을 한 표로 보기

| 일상적인 말 | Harness term | 왜 필요한가 | 더 읽을 곳 |
|---|---|---|---|
| "무엇을 하는 중이지?" | Task | 사용자가 원하는 결과, 상태, blocker, 근거, 닫기 판단을 하나로 묶는다. | [사용자 가이드](../use/user-guide.md); [커널 참조](../reference/kernel.md). |
| "어디까지 바꿔도 되지?" | Change Unit | 제품 쓰기 범위를 제한해 작업이 조용히 커지지 않게 한다. | [사용자 가이드](../use/user-guide.md); [커널 참조](../reference/kernel.md). |
| "이건 사용자가 결정해야 해." | Decision Packet | 사용자가 소유한 제품 판단이나 중요한 기술 판단을 넓은 승인과 분리한다. | [사용자 가이드](../use/user-guide.md); [커널 참조](../reference/kernel.md). |
| "이 민감한 단계를 진행해도 되나?" | Approval | 정해진 범위 안에서 민감한 행동을 진행해도 되는지 답한다. 사용자 소유 판단이나 최종 수락을 대신하지 않는다. | [커널 참조](../reference/kernel.md). |
| "지금 이 파일을 수정해도 되나?" | Write Authorization | 의도한 쓰기가 현재 Task, Change Unit, 결정, sensitive-action Approval과 맞는지 확인한다. | 엄격한 동작: [커널 참조](../reference/kernel.md), [MCP API와 스키마](../reference/mcp-api-and-schemas.md); agent 접점 동작: [Agent 통합 참조](../reference/agent-integration.md). |
| "이 주장을 뒷받침하는 것은 이것이다." | 근거 | diff, log, check, screenshot 같은 기록으로 "끝났다"는 말을 확인 가능하게 만든다. | [사용자 가이드](../use/user-guide.md); 엄격한 동작: [커널 참조](../reference/kernel.md), [MCP API와 스키마](../reference/mcp-api-and-schemas.md), [Storage와 DDL](../reference/storage-and-ddl.md), [운영과 Conformance 참조](../reference/operations-and-conformance.md). |
| "독립적으로 확인했나?" | 검증 | 자체 확인과 분리된 검증을 구분한다. | [사용자 가이드](../use/user-guide.md); 엄격한 동작: [커널 참조](../reference/kernel.md), [MCP API와 스키마](../reference/mcp-api-and-schemas.md), [운영과 Conformance 참조](../reference/operations-and-conformance.md). |
| "사람이 실제 경험을 봤나?" | Manual QA | 테스트가 놓칠 수 있는 UX, 문구, 접근성, 시각 품질, 작업 흐름 판단을 다룬다. | [사용자 가이드](../use/user-guide.md); 엄격한 동작: [설계 품질 정책](../reference/design-quality-policies.md), [커널 참조](../reference/kernel.md), [MCP API와 스키마](../reference/mcp-api-and-schemas.md), [운영과 Conformance 참조](../reference/operations-and-conformance.md). |
| "이 결과를 받아들일 수 있나?" | 수락 | Task 경로가 요구할 때 사용자의 최종 판단을 기록한다. | [사용자 가이드](../use/user-guide.md); [커널 참조](../reference/kernel.md). |
| "아직 어떤 불확실성이 남았나?" | 남은 위험 | 닫기나 수락 전에 알려진 제한과 위험을 보이게 한다. | [사용자 가이드](../use/user-guide.md); [커널 참조](../reference/kernel.md). |
| "이제 끝났다고 해도 되나?" | 닫기 | 관련 blocker가 처리된 뒤에만 Task를 완료한다. | [커널 참조](../reference/kernel.md); agent 접점 세부 담당: [Agent 통합 참조](../reference/agent-integration.md). |

## 다음에 읽을 문서

- [핵심 개념](concepts.md)에서 예시를 읽은 뒤 필요한 더 단단한 어휘를 봅니다.
- [사용자 가이드](../use/user-guide.md)에서 Harness 세션 중 사용자가 자연스럽게 말할 수 있는 표현을 봅니다.
- Task, Change Unit, Decision Packet, Approval, Write Authorization, 수락, 남은 위험, 닫기의 엄격한 동작이 필요하면 [커널 참조](../reference/kernel.md)를 봅니다.
- 에이전트가 이 흐름을 불필요한 내부 세부사항 없이 어떻게 보여줘야 하는지 알고 싶다면 [에이전트 세션 흐름](../use/agent-session-flow.md)을 봅니다.
