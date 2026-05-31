# 결정 패킷 Cookbook

## 한 가지 판단을 또렷하게 묻기

[사용자 가이드](user-guide.md)를 읽은 뒤, 에이전트가 혼자 결정하면 안 되는 선택 때문에 작업이 막혔을 때 사용합니다. 선택지를 보여 달라고, 추천 경로를 말해 달라고, 불확실성을 이름 붙여 달라고, 내가 결정을 미루면 무엇을 계속할 수 있는지와 무엇이 아직 닫기를 막는지 설명해 달라고 요청할 수 있습니다.

에이전트는 왜 지금 결정이 필요한지, 현실적인 선택지가 무엇인지, 어떤 장단점이 사용자 판단인지, 코드베이스나 현재 근거가 답할 수 있는 것은 무엇인지, 근거·QA·검증·작업 수락·잔여 위험 처리가 어떻게 영향을 받을 수 있는지 구체화해야 합니다.

하네스는 사용자 소유 결정을 넓은 승인, 구현 근거, 작업 수락, 잔여 위험 수용과 분리해 보존하도록 돕습니다. 사용자는 필드 목록이 아니라, 짧고 집중된 사용자 결정 요청을 볼 수 있어야 합니다.

결정 패킷은 간단할 수도 있고 상세할 수도 있습니다. 작은 unblocker는 `minimal_decision` profile로 질문, 범위, 선택지, refs만 간결하게 기록할 수 있습니다. 복잡하거나 위험한 선택은 `architecture_tradeoff` 같은 full profile로 detailed options, trade-offs, recommendation, uncertainty, deferral consequence, affected refs를 포함할 수 있습니다.

이 문서는 고급 사용 예시입니다. 기본 사용자 진입점도 아니고, 결정 패킷 동작의 정확한 계약도 아닙니다.

## 이런 때 사용하기

제품, UX, 아키텍처, 보안, QA, 검증, 작업 수락, 잔여 위험, 범위/자율성 판단 때문에 작업이 막혔고 그 판단을 에이전트가 혼자 하면 안 될 때 아래 예시를 사용합니다.

## 핵심 생각

결정 패킷은 빈 허가서가 아니라 판단을 돕는 자료처럼 보여야 합니다. 실제 사용자 소유 선택을 이름 붙이고, 선택지와 장단점을 보여주고, 경로를 추천하고, 불확실성을 말하고, 미루면 어떤 일이 생기는지 설명하며, 관련 근거나 잔여 위험을 연결합니다.

아래 예시는 계약 정의가 아니라 prompt 예시입니다. 정확한 동작은 Reference 문서가 담당합니다.

## 모든 예시에 들어 있는 것

각 cookbook 예시는 다음을 포함합니다.

- 판단 영역
- 필요할 때 결정 profile
- 필요할 때 결정 경로
- 왜 지금 필요한지
- 현실적인 선택지 또는 chosen outcome
- profile에 필요할 때 추천, 불확실성, 미루면 생기는 일
- 해당하는 관련 위험 또는 근거

일부 예시는 에이전트와 통합자가 알아볼 수 있도록 정확한 라벨을 포함합니다. 판단할 때는 그 라벨을 몰라도 됩니다.

## Tiny decision: label wording

간단한 제품 또는 기술 unblocker에 사용자 선택은 필요하지만 full trade-off packet은 안전을 더하지 않고 절차만 무겁게 만들 때 사용합니다.

```text
결정 제목: Settings form button label
결정 profile: 간단한 판단 기록 (`minimal_decision`)
판단 영역: Product / UX (`product_ux`)
결정 경로: product trade-off (`decision_kind=product_tradeoff`)
지금 필요한 이유: scoped settings copy change에서 text와 관련 snapshot을 업데이트하기 전에 label 하나를 정해야 합니다.
선택지:
- Save.
- Update.
관련 refs: settings form copy scope와 관련 snapshot/test ref가 있으면 해당 ref.
확정하지 않는 것: broader settings workflow, localization strategy, final acceptance, residual-risk acceptance, write authority.
```

이 예시가 좋은 이유는 작은 범위의 사용자 소유 선택을 명시적으로 기록하면서, 중요하지 않은 pros/cons, uncertainty, architecture-style detail을 강제하지 않기 때문입니다.

## UX 결정: 인라인 메시지 vs 토스트 vs 모달

구현이나 QA를 끝내기 전에 사용자에게 보이는 동작을 선택해야 할 때 사용합니다.

```text
결정 제목: 로그인 실패 안내 방식
결정 profile: Product/UX trade-off (`product_ux_tradeoff`)
판단 영역: Product / UX (`product_ux`)
결정 경로: product trade-off (`decision_kind=product_tradeoff`)
지금 필요한 이유: 로그인 흐름의 최종 UI 연결, 문구 확인, 수동 QA 전에 실패 안내 방식을 하나 정해야 합니다.
선택지:
- 입력 필드 근처에 인라인 메시지 표시.
- 제출 실패 뒤 토스트 표시.
- 흐름을 멈추는 모달 표시.
추천: 인라인 메시지를 선택합니다.
불확실성: 기존 design system이 인라인 오류와 스크린 리더 알림을 어떻게 지원하는지 확인해야 합니다.
미루면 생기는 일: API 오류 매핑과 상태 연결은 계속할 수 있지만 최종 UI 동작, 문구, screenshot, 수동 QA는 기다려야 합니다.
관련 위험 또는 근거: account-enumeration 문구 위험, 접근성 근거, screenshot 또는 브라우저 smoke 확인 참조, 구현 뒤 수동 QA 참조.
```

이 예시가 좋은 이유는 사용자에게 "로그인 변경을 승인할까요?"가 아니라 UX 선택을 묻기 때문입니다. 또한 사용자가 결정하는 동안 무엇을 계속할 수 있는지도 말합니다.

정확한 결정 패킷 동작은 [결정 패킷](../reference/kernel.md#decision-packet)과 [Decision Gate](../reference/kernel.md#decision-gate)가 담당합니다. 수동 QA 동작은 [QA Gate](../reference/kernel.md#qa-gate)가 담당합니다.

## Auth 결정: session cookie vs bearer/JWT vs OAuth/OIDC vs social login

인증 방향이 저장 방식, 폐기 가능성, 클라이언트 동작, 보안 자세에 영향을 줄 때 사용합니다.

```text
결정 제목: 로그인 세션 구조
결정 profile: 상세 기술 구조 판단 (`architecture_tradeoff`)
판단 영역: Technical architecture (`technical_architecture`)
결정 경로: architecture choice (`decision_kind=architecture_choice`)
지금 필요한 이유: 저장 방식, 미들웨어, 테스트, 위협 검토의 범위를 잡기 전에 세션 모델을 정해야 합니다.
선택지:
- 자사 웹 로그인용 서버 측 세션 쿠키.
- 클라이언트가 다루는 JWT 또는 bearer token.
- 필요한 경우 별도의 로컬 세션 또는 토큰 전략을 두는 OAuth/OIDC ID 제공자.
- Provider별 계정 연결과 지원 영향을 포함하는 social-login provider integration.
추천: 자사 웹 앱이라면 지금 외부 ID 제공자 로그인, 소셜 로그인 전환, 브라우저가 아닌 클라이언트가 필요하지 않은 한 서버 측 세션 쿠키를 선택합니다.
불확실성: 현재 클라이언트 구성, 기존 인증 미들웨어, 폐기 요구사항, SSO 요구사항, 배포 제약.
미루면 생기는 일: Discovery가 현재 인증 코드를 살피고 좁은 Change Unit 초안을 만들 수는 있지만 저장 방식, 토큰 수명, 미들웨어 동작에는 commit하지 않아야 합니다.
관련 위험 또는 근거: CSRF/XSS 노출, 폐기 가능성 근거, 세션 수명 테스트, 마이그레이션 메모, 보안 검토 참조.
```

이 예시가 좋은 이유는 저장 방식, 폐기 가능성, client behavior, security posture, migration, tests, review에 영향을 주는 선택이므로 full architecture profile을 쓰기 때문입니다. 또한 ID 제공자 선택과 세션/저장 방식 선택을 분리합니다. OAuth/OIDC도 로컬 세션이나 토큰 전략이 필요할 수 있으므로, 이 packet은 세 선택지가 서로 완전히 같은 층위라고 가정하지 않습니다.

정확한 민감 동작 승인(Approval)과 사용자 소유 architecture judgment 경계는 [Approval](../reference/kernel.md#approval), [결정 패킷](../reference/kernel.md#decision-packet), [Sensitive Categories](../reference/mcp-api-and-schemas.md#sensitive-categories)가 담당합니다.

## 보안 결정: PII 로그

기능, 디버그 경로, run, export, 아티팩트가 personal data를 노출할 수 있을 때 사용합니다.

```text
결정 제목: 로그인 진단용 PII 로그 정책
판단 영역: Security / privacy (`security_privacy`)
결정 경로: design choice (`decision_kind=design_choice`)
지금 필요한 이유: 진단이나 테스트를 추가하기 전에 agent가 log와 evidence artifact에 무엇을 쓸 수 있는지 알아야 합니다.
선택지:
- PII를 로그에 기록하지 않고 요청 ID와 식별 불가능한 error code를 사용합니다.
- Redacted 또는 토큰화된 식별자를 로그에 기록합니다.
- 감사 통제가 있는 짧은 보관 기간 동안 제한된 원본 필드를 로그에 기록합니다.
추천: raw PII는 로그에 기록하지 않습니다. Debugging에 필요할 때만 요청 ID와 redacted 또는 토큰화된 식별자를 사용합니다.
불확실성: 지원/디버깅 요구사항, retention policy, 컴플라이언스 의무, 기존 로그 가림 처리가 증명됐는지.
미루면 생기는 일: PII 로그 기록 없이 implementation은 계속할 수 있지만 user identifier가 필요한 진단은 기다려야 합니다.
관련 위험 또는 근거: 개인정보 노출, artifact redaction notes, log sample evidence, retention/audit 참조, debugging value가 줄어드는 경우의 잔여 위험.
```

이 예시가 좋은 이유는 개인정보 보호를 숨은 구현 세부사항이 아니라 제품/보안 판단으로 다루기 때문입니다. 민감 동작도 필요하다면 그 Approval은 정책 판단과 분리됩니다.

정확한 보안 개념은 [보안 위협 모델 참조](../reference/security-threat-model.md)에 있습니다. 정확한 Approval과 근거 권한은 [Approval](../reference/kernel.md#approval)과 [Evidence Gate](../reference/kernel.md#evidence-gate)가 담당합니다.

## QA waiver

필수 human QA를 완료할 수 없고, 사용자가 waiver를 증명이나 잔여 위험 수용처럼 취급하지 않으면서 닫기를 어떻게 처리할지 결정해야 할 때 사용합니다.

```text
결정 제목: 반응형 로그인 레이아웃 수동 QA 면제
판단 영역: QA/작업 수락 (`qa_acceptance`)
결정 경로: QA waiver (`decision_kind=qa_waiver`)
지금 필요한 이유: 반응형 로그인 흐름의 required 수동 QA가 passed가 아니어서 닫기가 막혔습니다.
선택지:
- 지금 수동 QA를 수행합니다.
- 이번 닫기에 대한 수동 QA waiver를 기록합니다. 닫기에 영향을 주는 잔여 위험이 남아 있다면 잔여 위험 수용 판단도 별도 소유자 경로로 route하거나 기록해야 합니다.
- Task를 열어 두고 닫기 전에 QA를 schedule합니다.
추천: user-facing login workflow라면 수동 QA를 수행합니다. Environment가 unavailable이고 change가 low risk 또는 time-bound일 때만 waive합니다.
불확실성: 작은 화면 layout, keyboard flow, 스크린 리더 해석, 시각적 완성도를 사람이 아직 inspect하지 않았습니다.
미루면 생기는 일: implementation은 complete 상태로 남을 수 있지만, 수동 QA가 passed되거나 유효한 QA 면제 판단과 필요한 잔여 위험 수용 경로가 기록될 때까지 닫기는 blocked 상태여야 합니다.
관련 위험 또는 근거: existing test logs, available screenshot, skipped viewport list, 수동 QA requirement, 잔여 위험 후속 작업.
```

이 예시가 좋은 이유는 skipped inspection을 이름 붙이기 때문입니다. QA waiver는 QA가 통과했다는 근거가 아니며, 필요한 잔여 위험 수용 경로가 함께 기록되지 않는 한 그 자체로 잔여 위험을 수락하지 않습니다.

정확한 QA 동작은 [QA Gate](../reference/kernel.md#qa-gate), [`harness.record_manual_qa`](../reference/mcp-api-and-schemas.md#harnessrecord_manual_qa), [`harness.record_user_decision`](../reference/mcp-api-and-schemas.md#harnessrecord_user_decision)이 담당합니다.

## Verification waiver

분리 검증이 필수이거나 기대되지만, 사용자가 그것 없이 진행하고 싶어 할 때 사용합니다.

```text
결정 제목: invoice export fix 분리 검증 면제
판단 영역: QA/작업 수락 (`qa_acceptance`)
결정 경로: 검증 면제 (`decision_kind=verification_waiver`)
지금 필요한 이유: compatible detached Eval이 없어서 verified close가 blocked이고, 사용자는 오늘 닫기를 원합니다.
선택지:
- Fresh bundle 또는 fresh worktree에서 분리 검증을 실행합니다.
- Independent 검증을 사용할 수 있을 때까지 Task를 열어 둡니다.
- Verification을 waive하고, 잔여 위험이 보이고 수용된 때만 잔여 위험을 수용하고 닫는 경로를 사용합니다.
추천: billing/export behavior라면 분리 검증을 실행합니다. Change blast-radius가 낮고 existing self-check 근거가 강할 때만 waive합니다.
불확실성: 같은 세션 편향, review되지 않은 export 예외 상황, 오래된 bundle 위험, self-check가 영향받는 형식을 덮었는지.
미루면 생기는 일: Task는 detached verified로 닫을 수 없습니다. 닫기는 기다리거나, 허용될 때 문서화된 잔여 위험 수용 닫기 경로를 사용합니다.
관련 위험 또는 근거: self-check run refs, missing Eval ref, 영향받는 export 형식, 잔여 위험 refs, 후속 검증 계획.
```

이 예시가 좋은 이유는 보증 표현을 정직하게 유지하기 때문입니다. Verification waiver는 잔여 위험을 받아들이고 닫는 경로를 열 수는 있지만 분리 검증을 만들지는 않습니다.

정확한 검증과 닫기 동작은 [Verification Gate](../reference/kernel.md#verification-gate), [Verification Independence Profiles](../reference/kernel.md#verification-independence-profiles), [잔여 위험](../reference/kernel.md#residual-risk), [`close_task`](../reference/kernel.md#close_task)가 담당합니다.

## 잔여 위험 수용

구현과 근거 뒤에도 닫기에 영향을 주는 알려진 잔여 위험이 남아 있고, 사용자가 이번 닫기에서 그 위험을 받아들일지 결정해야 할 때 사용합니다.

```text
결정 제목: 레거시 CSV 인코딩 한계 수용
판단 영역: 잔여 위험 (`residual_risk`)
결정 경로: 잔여 위험 수용 (`decision_kind=residual_risk_acceptance`)
지금 필요한 이유: Export fix는 현재 UTF-8 파일에는 동작하지만 레거시 인코딩은 아직 지원되지 않으며, 닫기에는 risk decision이 필요합니다.
선택지:
- 닫기 전에 레거시 인코딩 지원을 고칩니다.
- 이번 닫기에서 범위가 제한된 위험을 받아들이고 후속 작업을 만듭니다.
- 남은 limitation이 requested outcome을 바꾸므로 Task를 cancel하거나 supersede합니다.
추천: 레거시 인코딩이 rare하고 documented이며 owner-visible 후속 작업이 있을 때만 accept합니다. 그렇지 않으면 close 전에 고칩니다.
불확실성: 실제 고객 사용 빈도, 지원 영향, existing imports에 레거시 파일이 있는지.
미루면 생기는 일: risk가 resolved되거나 non-close-relevant가 되거나 소유자 경로를 통해 accepted될 때까지 작업 수락 또는 닫기가 blocked일 수 있습니다.
관련 위험 또는 근거: 통과한 UTF-8 export 테스트, 누락된 레거시 인코딩 테스트 범위, 알려진 한계 메모, 후속 작업 ref, 표시된 잔여 위험 참조.
```

이 예시가 좋은 이유는 작업 수락 전에 남은 한계를 보이게 하기 때문입니다. 사용자는 결과만 받아들이는 것이 아니라, 이름 붙은 잔여 위험을 이번 닫기에서 받아들일 수 있는지도 결정합니다.

정확한 잔여 위험 동작은 [잔여 위험](../reference/kernel.md#residual-risk), [Acceptance Gate](../reference/kernel.md#acceptance-gate), [`harness.record_user_decision`](../reference/mcp-api-and-schemas.md#harnessrecord_user_decision), [`close_task`](../reference/kernel.md#close_task)가 담당합니다.

## 소유자 링크

Cookbook 예시에서 정확한 동작이 필요할 때는 다음 기준 문서 소유자를 사용합니다.

| 필요 | Owner |
|---|---|
| 결정 패킷 의미와 gate aggregation | [결정 패킷](../reference/kernel.md#decision-packet), [Decision Gate](../reference/kernel.md#decision-gate) |
| Public request와 answer shape | [`harness.request_user_decision`](../reference/mcp-api-and-schemas.md#harnessrequest_user_decision), [`harness.record_user_decision`](../reference/mcp-api-and-schemas.md#harnessrecord_user_decision) |
| 민감 동작 승인(Approval) | [Approval](../reference/kernel.md#approval) |
| Evidence sufficiency | [Evidence Gate](../reference/kernel.md#evidence-gate) |
| Verification과 검증 면제 영향 | [Verification Gate](../reference/kernel.md#verification-gate) |
| 수동 QA와 QA waiver impact | [QA Gate](../reference/kernel.md#qa-gate) |
| 작업 수락과 잔여 위험 visibility | [Acceptance Gate](../reference/kernel.md#acceptance-gate), [잔여 위험](../reference/kernel.md#residual-risk) |
| 닫기 막힘과 close reasons | [`close_task`](../reference/kernel.md#close_task) |

## 좋은 답변 패턴

결정 패킷에 답할 때는 평범한 말로 선택지를 고르고, 중요하게 생각하는 경계를 덧붙입니다.

```text
로그인 실패 안내는 인라인 메시지로 해주세요. 문구는 일반적으로 유지하고, 모달은 추가하지 말고, 계정 복구는 이 Task 범위 밖으로 둡니다.
```

이런 답변은 이름 붙은 선택만 해결하고 다른 모든 권한을 부여하는 것처럼 보이지 않기 때문에 유용합니다. 에이전트에게는 여전히 쓰기 허가 기록, 근거, QA, 검증, 작업 수락, 잔여 위험 수용 판단, 닫기를 위한 일반 소유자 경로가 필요합니다.
