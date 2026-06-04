# MANUAL-QA 템플릿

## 사용 시점

수동 QA가 required, performed, waived, pending 상태이거나 `qa_gate`에 반영되어 있고 해당 기록을 읽기 쉬운 projection으로 볼 때 `MANUAL-QA`를 사용합니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 보증 프로필 보고서입니다. Manual QA record 또는 active QA profile이 있을 때만 렌더링하며 MVP-1 다섯 가지 보기 세트의 일부가 아닙니다.

## 기준 기록

- `manual_qa_records`
- Task와 Change Unit 참조
- `qa_gate`
- 수동 QA profile, setup, checklist, result, waiver, finding
- human inspector 또는 role과 확인한 품질이나 workflow
- screenshot, browser log, `qa_capture`, video, workflow recording, 수동 제공 note artifact 참조와 `redaction_state`
- QA waiver 또는 failure와 관련된 waiver reason, 필요한 경우 QA waiver user judgment refs, Residual Risk refs
- 표시되는 claim이 있을 때 Evidence Manifest, Eval, 작업 수락 context, Approval, 아티팩트 참조, redaction state, 읽기용 보기 최신성(projection freshness)
- `manual_qa` 관련 design-quality validator 결과
- 읽기용 보기 최신성(projection freshness) 입력

## 렌더링 섹션

- 식별 정보
- 권한과 닫기 참조
- 준비 사항
- 확인 목록
- 결과
- 면제와 위험
- 발견 사항
- 근거 참조
- Redaction과 사용 가능성

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

# 수동 QA

> Projection 보기: `source_state_version`와 `updated_at` 기준으로 렌더링되며 수동 QA record와 `qa_gate`를 표시합니다. QA result와 QA waiver는 `manual_qa_records`와 `qa_gate`에 기록됩니다. Product/user risk가 있는 QA waiver는 연결된 QA waiver user judgment를 사용하고, 잔여 위험 수용은 Residual Risk refs에 기록됩니다. Browser QA artifact는 supporting ref일 뿐이며 human 수동 QA judgment, 작업 수락, 분리 검증을 대체하지 않습니다.

## 식별 정보
- manual_qa_record_id: QA-0001 | null
- task_id:
- change_unit_id: CU-01 | null
- qa_profile: ui_quality | workflow | copy | accessibility | browser_smoke | performance_smoke | other
- required: yes | no
- 수행한 사람:

## 권한과 닫기 참조
- 수동 QA record:
- QA waiver user judgment:
- Evidence Manifest:
- Eval:
- Approval:
- 작업 수락 맥락:
- Residual Risk:
- 아티팩트 참조:
- redaction state:
- 보기 최신성:

## 준비 사항
- build/run command:
- test account/data:
- route 또는 screen:
- browser capture support: supported | unsupported | not applicable

## 확인 목록
- [ ] primary workflow works
- [ ] errors are understandable
- [ ] visual layout acceptable
- [ ] accessibility smoke check
- [ ] no obvious regression

## 결과
- 기록 결과: passed | failed | waived | null when no record exists
- qa_gate: not_required | required | pending | passed | failed | waived
- qa_gate note: 기준 close-relevant gate; 이 projection은 표시 전용
- QA waiver 표시: `qa_gate=waived`와 수동 QA record 또는 waiver reason, 필요한 경우 QA waiver user judgment
- 자동 check 상태: {supporting refs only; 수동 QA 결과 아님}
- 검증 상태: {별도 Eval/gate status; 이 template이 만들지 않음}
- 작업 수락 상태: {별도 사용자 판단; 이 template이 만들지 않음}
- 사람의 확인 요약:
- 요약:
- 면제 사유:

## 면제와 위험
- 면제 기록:
- QA waiver user judgment:
- 생략한 확인 또는 대상:
- waiver 전에 표시된 위험:
- 받아들이는 위험:
- 후속 작업:
- Residual Risk refs:
- 받아들인 Residual Risk 요약:
- 닫기 영향:

## 발견 사항
| 심각도 | 발견 사항 | 제안 조치 | 후속 CU |
|---|---|---|---|
| minor | | | |

## 근거 참조
- screenshot:
- qa_capture:
- browser log:
- video:
- note:
- manually supplied artifact:
- unsupported-surface fallback note:

## Redaction과 사용 가능성
| 아티팩트 참조 | 가림 상태 | QA 영향 | 메모 |
|---|---|---|---|
| ART-QA-0001 | secret_omitted | observable finding만 지원 | |
| ART-QA-0002 | blocked | capture 사용 불가; 대체되거나 유효하게 면제되기 전까지 QA 경로는 미해결이며 `qa_gate`는 상황에 따라 pending/failed 또는 `waived` | |
````

## 메모

이 template은 렌더링 결과일 뿐 기준 상태가 아닙니다. `qa_gate`가 기준 close-relevant gate이며, 이 projection은 그 값을 표시만 합니다.

수동 QA 표시는 passed 수동 QA record, failed 수동 QA record, pending required QA, QA waiver를 눈에 띄게 구분해야 합니다. `qa_gate=waived`는 필요한 경우 ref와 accepted risk/follow-up을 동반하는 waiver display입니다. Passed 수동 QA result, 작업 수락, 분리 검증이 아닙니다.

수동 QA는 자동 검증이 아닙니다. Test result, browser smoke, screenshot, Browser QA artifact는 사람의 확인 맥락을 뒷받침할 수 있지만, 수동 QA owner path가 result 또는 valid waiver를 기록하지 않았다면 이 template은 이를 수동 QA pass처럼 렌더링하면 안 됩니다.

수동 QA projection은 안전한 omission note, handle, blocked artifact notice를 보여줄 수 있지만 생략된 secret/PII 값이나 차단된 capture payload를 포함하면 안 됩니다. `secret_omitted` artifact는 보이는 workflow, UI, copy, accessibility, smoke-test observation을 뒷받침할 수 있습니다. `blocked` capture는 replacement, waiver, user judgment outcome, 받아들인 위험, documented fallback이 QA 경로를 해소하지 않는 한 사용할 수 없는 QA 입력입니다.

Screenshot, browser log, video, `qa_capture` output, workflow recording, note는 QA evidence ref입니다. Browser QA Capture는 owner 문서가 명시적으로 승격하고 증명하기 전까지 로드맵 후보입니다. 수동 QA result는 기록된 사람의 확인 또는 유효한 waiver이지, 이런 capture가 존재한다는 사실만으로 만들어지지 않습니다. Browser QA artifact는 별도의 Eval 경로가 verification independence를 충족하지 않는 한 작업 수락 또는 분리 검증도 기록하지 않습니다. 어떤 접점이 browser capture를 지원하지 않으면 사람이 작성한 수동 QA notes와 수동 제공 artifacts를 fallback으로 사용합니다.
