# 수동 QA 카드 템플릿

## 사용 시점

수동 QA가 필요할 때 기록, gate, 프로필(profile), 대상, 확인 목록(checklist), 기록할 근거, 면제와 위험 표시(waiver/risk visibility)를 사람이 확인하기 쉬운 간결한 안내 카드로 보여주기 위해 수동 QA 카드를 사용합니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 보증 프로필 보고서입니다. Manual QA profile이 명시적으로 active일 때 사용하며 full Manual QA policy coverage는 이후 staged scope입니다.

## 기준 기록

- 수동 QA requirement와 `qa_gate`
- 존재하는 경우 수동 QA 기록
- QA profile(수동 QA 프로필)
- human inspector 또는 role과 요청되는 사람의 판단
- 대상 화면(screen) 또는 흐름(flow)
- checklist item
- 예상 screenshot, walkthrough note, browser log, Browser QA artifact, 수동 제공 artifact 근거
- QA가 면제되거나 미뤄질 때 waiver reason, 필요한 경우 QA waiver user judgment refs, Residual Risk refs
- 검증, 작업 수락, 닫기 영향 요약
- 수동 QA record, QA waiver user judgment, Evidence Manifest, Eval, 작업 수락 context, Residual Risk, 아티팩트 참조, redaction state, 읽기용 보기 최신성(projection freshness)을 위한 간결한 refs

닫기 맥락과 waiver placeholder는 QA 기록, `qa_gate`, 관련 gate 상태, user judgment ref, Residual Risk ref에서 파생한 표시 전용 요약입니다. Waiver path는 그런 ref를 렌더링하거나 아직 기록이 필요하다고 표시해야 합니다.

## 렌더링 섹션

- 수동 QA 필요 여부
- 기록
- gate
- 프로필
- 대상
- 확인 목록
- 기록할 근거
- 닫기 맥락
- 면제 기록
- 결과 안내

## 전체 템플릿

````text
수동 QA가 필요합니다.
표시 전용: `qa_gate`와 QA record가 기준으로 남습니다.
사람의 확인만 수동 QA입니다. 자동 검사, screenshot, browser log, Browser QA artifact는 맥락을 뒷받침할 수 있지만 그 자체로 수동 QA가 되지는 않습니다.
브라우저 QA 캡처(Browser QA Capture): 승격되고 지원될 때 유용합니다. 작업 수락이 아니며, independent Eval 없이는 분리 검증이 아니고, required human inspection을 대체하지 않습니다.

기록: {manual_qa_record_id|none until recorded}
Gate(관문): {qa_gate display: not_required|required|pending|passed|failed|waived}
참조: manual_qa={manual_qa_record_id|none}; qa_waiver_judgment={qa_waiver_user_judgment_ref|none}; evidence={evidence_manifest_ref|none}; eval={eval_ref|none}; acceptance={acceptance_context_ref|none}; residual_risk={residual_risk_refs|none}; artifacts={artifact_refs|none}; redaction={redaction_availability_summary|none}; freshness={projection_freshness}
프로필(Profile): {profile}
요청되는 사람의 확인: {human_inspection_summary}
대상(Target): {screen_or_flow}
확인 목록(Checklist):
- {checklist_item}

기록할 근거:
- screenshot 또는 walkthrough note
- 승격되고 지원될 때 qa_capture artifact
- 관련 있을 때 browser log
- browser capture가 지원되지 않을 때 수동 제공 artifact 또는 사람이 작성한 note
- 근거를 원본 content로 기록할 수 없을 때의 redaction/omission/block note

닫기 맥락:
- 자동 검사: {check_refs|none; 수동 QA 결과 아님}
- Browser QA artifacts: {artifact_refs|none; 뒷받침 refs only}
- QA waiver 표시: {qa_gate=waived with waiver refs|none}
- 검증 영향: {verification_impact}
- 작업 수락 영향: {acceptance_impact; 이 card가 기록하지 않음}
- Residual Risk 또는 후속 작업: {residual_risk_or_follow_up|none}

면제 기록:
- 생략한 수동 QA 대상:
- waiver 전에 표시된 위험:
- 받아들이는 위험:
- 후속 작업:
- 관련 refs:
- 닫기 영향:
- waiver 출처: {manual_qa_record_id와 waiver_reason; 사용자 소유 위험이 있으면 qa_waiver_user_judgment_ref}

수동 QA 결과를 기록하거나, 허용된 낮은 위험의 QA waiver 사유를 기록하거나, 사용자 소유 위험이 있으면 QA waiver user judgment를 요청하시겠습니까?
````

## 메모

이 template은 렌더링 결과인 카드 형태일 뿐 기준 QA 상태가 아닙니다. `qa_gate`는 close-relevant gate로 남습니다.

수동 QA는 사람이 확인한 기록입니다. 테스트 통과, browser smoke, screenshot capture, Browser QA Capture artifact, 검증, 사용자의 작업 수락은 닫기 맥락을 뒷받침할 수 있지만, `record_manual_qa`가 수동 QA 결과를 기록했거나 유효한 QA waiver가 waiver reason과 함께 `qa_gate=waived`를 갱신하고, 사용자 소유 위험이 있으면 호환되는 QA waiver user judgment를 포함한 경우가 아니면 수동 QA가 되지 않습니다. Browser QA Capture는 owner 문서가 명시적으로 승격하고 증명하기 전까지 로드맵 후보이며, captured artifact는 별도 Eval 경로가 independence를 충족하지 않는 한 작업 수락 또는 분리 검증을 기록하지 않습니다. Waiver에 닫기 영향이나 위험을 받아들이는 판단이 걸려 있는 경우 가벼운 채팅 문장만으로는 충분하지 않습니다.

이 카드는 pending QA, passed QA, failed QA, waived QA를 별도 표시 상태로 렌더링해야 합니다. Waived QA는 수동 QA record 또는 waiver reason, 필요한 경우 QA waiver user judgment, 해당되는 Residual Risk refs, close impact를 cite하며 passed inspection이 아닙니다.

결과 안내는 수동 QA result 또는 QA waiver path만 물어야 합니다. 작업 수락이나 잔여 위험 수용을 같은 답변처럼 요청하면 안 됩니다.

Artifact가 `secret_omitted` 또는 `blocked`라면 이 카드는 대체 근거 또는 면제 기록을 요청할 수 있지만, 생략된 값 또는 차단된 원본 캡처 내용을 표시하면 안 됩니다. Browser capture가 해당 접점에서 지원되지 않으면 이 카드는 capture absence를 QA result로 다루지 말고 사람이 작성한 수동 QA notes와 수동 제공 artifacts를 요청해야 합니다.
