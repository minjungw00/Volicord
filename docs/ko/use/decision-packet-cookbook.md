# 결정 패킷 Cookbook

## 이 문서로 할 수 있는 일

엄격한 schema를 복사하지 않고도 좋은 결정 패킷 prompt를 알아보고 작성할 수 있도록 실용 예시를 사용합니다.

읽고 나면 초점을 좁힌 사용자 판단을 요청하고, 현실적인 선택지를 비교하고, 경로를 추천하고, 불확실성을 이름 붙이고, 사용자가 미룰 때 어떤 일이 생기는지 설명하고, 필요할 때 관련 위험이나 근거를 연결할 수 있어야 합니다.

## 이런 때 읽기

Agent가 혼자 결정하면 안 되는 product, UX, architecture, security, QA, verification, acceptance, 잔여 위험, scope/autonomy 판단 때문에 작업이 막혔을 때 읽습니다.

## 읽기 전에

일상적인 흐름은 [사용자 가이드](user-guide.md#judgment)를 읽습니다. 정확한 behavior는 [Kernel Reference](../reference/kernel.md#decision-packet)와 [`harness.request_user_decision`](../reference/mcp-api-and-schemas.md#harnessrequest_user_decision)을 사용합니다.

## 핵심 생각

결정 패킷은 빈 허가서가 아니라 판단을 돕는 자료처럼 보여야 합니다. 실제 사용자 소유 선택을 이름 붙이고, 선택지와 장단점을 보여주고, 경로를 추천하고, 불확실성을 말하고, 미루면 어떤 일이 생기는지 설명하며, 관련 근거나 잔여 위험을 연결합니다.

아래 예시는 prompt 예시이지 schema가 아닙니다. "Judgment type"은 사용자에게 보이는 표시 category일 뿐이며, canonical field, gate, owner record, validator input, 새 authority path가 아닙니다.

## 모든 예시에 들어 있는 것

각 cookbook 예시는 다음을 포함합니다.

- judgment type
- why now
- options
- recommendation
- uncertainty
- deferral consequence
- applicable한 related risk 또는 evidence

정확한 state transition, gate effect, waiver semantics, accepted-risk handling, public API field는 Reference owner에 남아 있습니다.

## UX 결정: toast vs modal vs inline

구현이나 QA를 끝내기 전에 사용자에게 보이는 동작을 선택해야 할 때 사용합니다.

```text
Decision title: Failed-login feedback pattern
Judgment type: Product / UX
Why now: Login flow의 final UI wiring, copy test, 수동 QA 전에 failure-feedback pattern 하나가 필요합니다.
Options:
- Form field 근처 inline message.
- Failed submit 뒤 toast.
- Flow를 끊는 modal.
Recommendation: inline message를 선택합니다.
Uncertainty: 기존 design-system의 inline error support와 screen-reader announcement behavior를 확인해야 합니다.
Deferral consequence: API error mapping과 state plumbing은 계속할 수 있지만 final UI behavior, copy, screenshot, 수동 QA는 기다려야 합니다.
Related risk or evidence: account-enumeration copy risk, accessibility evidence, screenshot 또는 browser-smoke refs, implementation 뒤 수동 QA refs.
```

이 예시가 좋은 이유는 사용자에게 "login change를 승인할까요?"가 아니라 UX 선택을 묻기 때문입니다. 또한 사용자가 결정하는 동안 무엇을 계속할 수 있는지도 말합니다.

정확한 결정 패킷 behavior는 [결정 패킷](../reference/kernel.md#decision-packet)과 [Decision Gate](../reference/kernel.md#decision-gate)가 담당합니다. 수동 QA behavior는 [QA Gate](../reference/kernel.md#qa-gate)가 담당합니다.

## Auth 결정: session cookie vs JWT vs OAuth

Authentication 방향이 storage, revocation, client behavior, security posture에 영향을 줄 때 사용합니다.

```text
Decision title: Login session architecture
Judgment type: Technical architecture
Why now: Storage, middleware, tests, threat review의 scope를 잡기 전에 implementation이 session model을 선택해야 합니다.
Options:
- First-party web login용 server-side session cookie.
- Client가 다루는 JWT 또는 bearer token.
- OAuth/OIDC identity provider와 별도의 local session 또는 token strategy.
Recommendation: first-party web app이라면 지금 third-party identity provider sign-in이나 non-browser client가 필요하지 않은 한 server-side session cookie를 선택합니다.
Uncertainty: 현재 client mix, existing auth middleware, revocation requirements, SSO requirements, deployment constraints.
Deferral consequence: Discovery가 현재 auth code를 살피고 좁은 Change Unit을 draft할 수는 있지만 storage, token lifetime, middleware behavior에는 commit하지 않아야 합니다.
Related risk or evidence: CSRF/XSS exposure, revocation evidence, session-lifetime tests, migration notes, security review refs.
```

이 예시가 좋은 이유는 identity-provider 선택과 session/storage 선택을 분리하기 때문입니다. OAuth/OIDC도 local session이나 token strategy가 필요할 수 있으므로, 이 packet은 세 선택지가 서로 완전히 같은 층위라고 가정하지 않습니다.

정확한 sensitive-action Approval과 사용자 소유 architecture judgment 경계는 [Approval](../reference/kernel.md#approval), [결정 패킷](../reference/kernel.md#decision-packet), [Sensitive Categories](../reference/mcp-api-and-schemas.md#sensitive-categories)가 담당합니다.

## Security 결정: PII logging

Feature, debug path, run, export, artifact가 personal data를 노출할 수 있을 때 사용합니다.

```text
Decision title: PII logging policy for login diagnostics
Judgment type: Security / privacy
Why now: Diagnostics나 tests를 추가하기 전에 agent가 log와 evidence artifact에 무엇을 쓸 수 있는지 알아야 합니다.
Options:
- PII를 logging하지 않고 request ID와 식별 불가능한 error code를 사용합니다.
- Redacted 또는 tokenized identifier를 logging합니다.
- Audit control이 있는 짧은 retention window 동안 제한된 raw field를 logging합니다.
Recommendation: raw PII는 logging하지 않습니다. Debugging에 필요할 때만 request ID와 redacted 또는 tokenized identifier를 사용합니다.
Uncertainty: support/debugging requirements, retention policy, compliance obligations, existing log redaction이 증명됐는지.
Deferral consequence: PII logging 없이 implementation은 계속할 수 있지만 user identifier가 필요한 diagnostics는 기다려야 합니다.
Related risk or evidence: privacy exposure, artifact redaction notes, log sample evidence, retention/audit refs, debugging value가 줄어드는 경우의 잔여 위험.
```

이 예시가 좋은 이유는 privacy를 숨은 구현 세부사항이 아니라 product/security judgment로 다루기 때문입니다. Sensitive action도 필요하다면 그 Approval은 policy decision과 분리됩니다.

정확한 security concept은 [보안 위협 모델 참조](../reference/security-threat-model.md)에 있습니다. 정확한 Approval과 evidence authority는 [Approval](../reference/kernel.md#approval)과 [Evidence Gate](../reference/kernel.md#evidence-gate)가 담당합니다.

## QA waiver

Required human QA를 완료할 수 없고, 사용자가 waiver를 proof나 risk acceptance처럼 취급하지 않으면서 close를 어떻게 처리할지 결정해야 할 때 사용합니다.

```text
Decision title: 반응형 로그인 레이아웃 수동 QA 면제
Judgment type: QA / acceptance
Why now: Responsive login flow의 required 수동 QA가 passed가 아니어서 close가 막혔습니다.
Options:
- 지금 수동 QA를 수행합니다.
- 이번 close에 대한 수동 QA waiver를 기록합니다. Close-relevant 잔여 위험이 남아 있다면 잔여 위험 수락 판단도 별도 owner path로 route하거나 기록해야 합니다.
- Task를 열어 두고 close 전에 QA를 schedule합니다.
Recommendation: user-facing login workflow라면 수동 QA를 수행합니다. Environment가 unavailable이고 change가 low risk 또는 time-bound일 때만 waive합니다.
Uncertainty: small-screen layout, keyboard flow, screen-reader interpretation, visual polish를 사람이 아직 inspect하지 않았습니다.
Deferral consequence: implementation은 complete 상태로 남을 수 있지만, 수동 QA가 passed되거나 valid QA waiver와 필요한 잔여 위험 수락 경로가 기록될 때까지 close는 blocked 상태여야 합니다.
Related risk or evidence: existing test logs, available screenshot, skipped viewport list, 수동 QA requirement, 잔여 위험 follow-up.
```

이 예시가 좋은 이유는 skipped inspection을 이름 붙이기 때문입니다. QA waiver는 QA가 통과했다는 증거가 아니며, 필요한 잔여 위험 수락 경로가 함께 기록되지 않는 한 그 자체로 잔여 위험을 수락하지 않습니다.

정확한 QA behavior는 [QA Gate](../reference/kernel.md#qa-gate), [`harness.record_manual_qa`](../reference/mcp-api-and-schemas.md#harnessrecord_manual_qa), [`harness.record_user_decision`](../reference/mcp-api-and-schemas.md#harnessrecord_user_decision)이 담당합니다.

## Verification waiver

분리 검증이 required 또는 expected이지만, 사용자가 그것 없이 진행하고 싶어 할 때 사용합니다.

```text
Decision title: invoice export fix 분리 검증 면제
Judgment type: QA / acceptance
Why now: Compatible detached Eval이 없어서 verified close가 blocked이고, 사용자는 오늘 close하기를 원합니다.
Options:
- Fresh bundle 또는 fresh worktree에서 분리 검증을 실행합니다.
- Independent verification을 사용할 수 있을 때까지 Task를 열어 둡니다.
- Verification을 waive하고, 잔여 위험이 visible이고 accepted일 때만 risk-accepted path로 닫습니다.
Recommendation: billing/export behavior라면 분리 검증을 실행합니다. Change blast-radius가 낮고 existing self-check evidence가 강할 때만 waive합니다.
Uncertainty: same-session bias, review되지 않은 export edge cases, stale bundle risk, self-check가 affected formats를 덮었는지.
Deferral consequence: Task는 detached verified로 닫을 수 없습니다. Close는 기다리거나, 허용될 때 documented risk-accepted path를 사용합니다.
Related risk or evidence: self-check run refs, missing Eval ref, affected export formats, 잔여 위험 refs, follow-up verification plan.
```

이 예시가 좋은 이유는 assurance wording을 정직하게 유지하기 때문입니다. Verification waiver는 risk-accepted close path를 풀 수는 있지만 분리 검증을 만들지는 않습니다.

정확한 verification과 close behavior는 [Verification Gate](../reference/kernel.md#verification-gate), [Verification Independence Profiles](../reference/kernel.md#verification-independence-profiles), [잔여 위험](../reference/kernel.md#residual-risk), [`close_task`](../reference/kernel.md#close_task)가 담당합니다.

## 잔여 위험 수락

Implementation과 evidence 뒤에도 알려진 close-relevant 잔여 위험이 남아 있고, 사용자가 이번 close에서 그 위험을 받아들일지 결정해야 할 때 사용합니다.

```text
Decision title: Accept legacy CSV encoding limitation
Judgment type: Residual risk
Why now: Export fix는 current UTF-8 files에는 동작하지만 legacy encodings는 아직 unsupported이며, close에는 risk decision이 필요합니다.
Options:
- Close 전에 legacy encoding support를 고칩니다.
- 이번 close에서 bounded risk를 받아들이고 follow-up을 만듭니다.
- 남은 limitation이 requested outcome을 바꾸므로 Task를 cancel하거나 supersede합니다.
Recommendation: legacy encoding이 rare하고 documented이며 owner-visible follow-up이 있을 때만 accept합니다. 그렇지 않으면 close 전에 고칩니다.
Uncertainty: 실제 customer frequency, support impact, existing imports에 legacy files가 있는지.
Deferral consequence: risk가 resolved되거나 non-close-relevant가 되거나 owner path를 통해 accepted될 때까지 final acceptance 또는 close가 blocked일 수 있습니다.
Related risk or evidence: passing UTF-8 export tests, missing legacy-encoding test coverage, known limitation note, follow-up ref, visible 잔여 위험 refs.
```

이 예시가 좋은 이유는 acceptance 전에 남은 limitation을 보이게 하기 때문입니다. 사용자는 결과만 받아들이는 것이 아니라, 이름 붙은 잔여 위험을 이번 close에서 받아들일 수 있는지도 결정합니다.

정확한 잔여 위험 behavior는 [잔여 위험](../reference/kernel.md#residual-risk), [Acceptance Gate](../reference/kernel.md#acceptance-gate), [`harness.record_user_decision`](../reference/mcp-api-and-schemas.md#harnessrecord_user_decision), [`close_task`](../reference/kernel.md#close_task)가 담당합니다.

## Owner 링크

Cookbook 예시에서 정확한 behavior가 필요할 때는 다음 Reference owner를 사용합니다.

| 필요 | Owner |
|---|---|
| 결정 패킷 의미와 gate aggregation | [결정 패킷](../reference/kernel.md#decision-packet), [Decision Gate](../reference/kernel.md#decision-gate) |
| Public request와 answer shape | [`harness.request_user_decision`](../reference/mcp-api-and-schemas.md#harnessrequest_user_decision), [`harness.record_user_decision`](../reference/mcp-api-and-schemas.md#harnessrecord_user_decision) |
| Sensitive-action Approval | [Approval](../reference/kernel.md#approval) |
| Evidence sufficiency | [Evidence Gate](../reference/kernel.md#evidence-gate) |
| Verification과 verification waiver impact | [Verification Gate](../reference/kernel.md#verification-gate) |
| 수동 QA와 QA waiver impact | [QA Gate](../reference/kernel.md#qa-gate) |
| Final acceptance와 잔여 위험 visibility | [Acceptance Gate](../reference/kernel.md#acceptance-gate), [잔여 위험](../reference/kernel.md#residual-risk) |
| Close blockers와 close reasons | [`close_task`](../reference/kernel.md#close_task) |

## 좋은 답변 패턴

결정 패킷에 답할 때는 ordinary language로 option을 고르고, 중요하게 생각하는 boundary를 덧붙입니다.

```text
Inline failed-login feedback을 선택합니다. Message는 generic하게 유지하고, modal은 추가하지 말고, account recovery는 이 Task 범위 밖으로 둡니다.
```

이런 답변은 named choice를 해결하면서 다른 모든 권한을 부여하는 것처럼 보이지 않기 때문에 유용합니다. Agent에게는 여전히 쓰기 허가 기록, evidence, QA, verification, acceptance, 잔여 위험 수락 판단, close를 위한 일반 owner path가 필요합니다.
