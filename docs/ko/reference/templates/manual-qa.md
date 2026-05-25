# MANUAL-QA Template

## 사용 시점

Manual QA가 required, performed, waived, pending 상태이거나 `qa_gate`에 반영되어 있고 해당 기록을 읽기 쉬운 projection으로 볼 때 `MANUAL-QA`를 사용합니다.

## 기준 기록

- `manual_qa_records`
- Task와 Change Unit 참조
- `qa_gate`
- Manual QA profile, setup, checklist, result, waiver, finding
- screenshot, browser log, video, note artifact 참조와 `redaction_state`
- QA waiver 또는 failure와 관련된 waiver reason, 필요한 경우 QA waiver Decision Packet refs, Residual Risk refs
- `manual_qa` 관련 design-quality validator 결과
- projection 최신성 입력

## 렌더링 섹션

- Identity
- Setup
- Checklist
- Result
- Waiver And Risk
- Findings
- Evidence Refs
- Redaction And Availability

## 전체 템플릿

````md
---
doc_type: manual_qa
manual_qa_record_id: null
task_id: TASK-0001
change_unit_id: CU-01
qa_gate: pending
result: null
source_state_version: 45
updated_at: 2026-05-06T10:05:00+09:00
---

# Manual QA

> Projection 보기: `source_state_version`와 `updated_at` 기준으로 렌더링되며 Manual QA record와 `qa_gate`를 표시합니다. QA result와 QA waiver는 `manual_qa_records`와 `qa_gate`에 기록됩니다. Product/user risk가 있는 QA waiver는 연결된 QA waiver Decision Packet을 사용하고, Residual Risk 수용은 Residual Risk refs에 기록됩니다.

## Identity
- manual_qa_record_id: QA-0001 | null
- task_id:
- change_unit_id: CU-01 | null
- qa_profile: ui_quality | workflow | copy | accessibility | browser_smoke | performance_smoke | other
- required: yes | no
- performed by:

## Setup
- build/run command:
- test account/data:
- route or screen:

## Checklist
- [ ] primary workflow works
- [ ] errors are understandable
- [ ] visual layout acceptable
- [ ] accessibility smoke check
- [ ] no obvious regression

## Result
- record result: passed | failed | waived | null when no record exists
- qa_gate: pending | passed | failed | waived | not_required
- qa_gate note: 기준 close-relevant gate; 이 projection은 표시 전용
- summary:
- 면제 사유:

## Waiver And Risk
- 면제 기록:
- 생략한 확인 또는 대상:
- 수용하는 위험:
- 후속 작업:
- Residual Risk refs:
- 수용된 Residual Risk 요약:
- 닫기 영향:

## Findings
| Severity | Finding | Suggested Action | Follow-up CU |
|---|---|---|---|
| minor | | | |

## Evidence Refs
- screenshot:
- browser log:
- video:
- note:

## Redaction And Availability
| Artifact Ref | Redaction State | QA Effect | Note |
|---|---|---|---|
| ART-QA-0001 | secret_omitted | observable finding만 지원 | |
| ART-QA-0002 | blocked | capture 사용 불가; QA는 해소 전까지 pending/failed/waived/blocking | |
````

## 메모

이 template은 렌더링 결과일 뿐 기준 상태가 아닙니다. `qa_gate`가 기준 close-relevant gate이며, 이 projection은 그 값을 표시만 합니다.

Manual QA projection은 안전한 omission note, handle, blocked artifact notice를 보여줄 수 있지만 생략된 secret/PII 값이나 차단된 capture payload를 포함하면 안 됩니다. `secret_omitted` artifact는 보이는 workflow, UI, copy, accessibility, smoke-test observation을 뒷받침할 수 있습니다. `blocked` capture는 replacement, waiver, Decision Packet outcome, accepted risk, documented fallback이 QA path를 해소하지 않는 한 사용할 수 없는 QA input입니다.
