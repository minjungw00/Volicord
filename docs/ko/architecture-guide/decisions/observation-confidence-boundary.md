# 관찰 신뢰도 경계

## 맥락

Guard 입력에는 정확한 structured path와 덜 정밀한 command 또는 repository 관찰이
함께 있습니다. 모든 관찰을 같게 취급하면 권한을 과장하거나 미기록 작업을 숨기게 됩니다.

## 결정

결정적인 structured path fact를 suspected effect와 분리해 분류합니다. 정확한 pre-action
fact는 owner-defined Write Ticket 점검에 참여할 수 있습니다. 불확실한 관찰은
post-action repository 비교가 경로를 확인할 때까지 비권한 상태로 남습니다.

확인된 미기록 변경은 reconciliation에 들어갑니다. suppression은 정확히 일치하는
owner-defined expected-write만 제거합니다. suppression이 unavailable이면 관찰된 전체
경로 집합을 유지하며 부분 best-effort 결과를 완료로 보고하지 않습니다.

Prompt capture는 Guard가 관찰한 것을 기록합니다. 사용자 답변을 기록하거나 UserAction을
해결하지 않습니다.

## 결과

- 관찰은 actor identity를 증명하거나 OS sandbox를 제공하지 않습니다.
- suspected effect는 묵시적으로 confirmed authority가 될 수 없습니다.
- close-readiness projection은 저장된 권한 상태와 명시적인 unresolved observation을
  사용합니다.
- 관찰 coverage 부족은 제한으로 보고하며 추론으로 채우지 않습니다.

[Guard Suppression](../../reference/guard-suppression.md),
[Security](../../reference/security.md),
[Reconcile Changes](../../reference/api/method-reconcile-changes.md)를 봅니다.
