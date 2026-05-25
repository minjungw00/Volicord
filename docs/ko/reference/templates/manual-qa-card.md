# Manual QA Card Template

## 사용 시점

필수 Manual QA의 기록, gate, profile, 대상, checklist, 기록할 근거를 간결한 안내 카드로 보여줄 때 Manual QA Card를 사용합니다.

## 기준 기록

- Manual QA requirement와 `qa_gate`
- 존재하는 경우 Manual QA 기록
- QA profile
- 대상 screen 또는 flow
- checklist item
- 예상 screenshot, walkthrough note, browser log 근거
- QA가 면제되거나 미뤄질 때 waiver reason, 필요한 경우 QA waiver Decision Packet refs, Residual Risk refs
- 검증, 수용, close 영향 요약

Close context와 waiver placeholder는 QA 기록, `qa_gate`, 관련 gate 상태, Decision Packet ref, Residual Risk ref에서 파생한 표시 전용 요약입니다. Waiver path는 그런 ref를 렌더링하거나 아직 기록이 필요하다고 표시해야 합니다.

## 렌더링 섹션

- Manual QA requirement
- 기록
- gate
- profile
- 대상
- checklist
- 기록할 근거
- close 맥락
- 면제 기록
- 결과 안내

## 전체 템플릿

````text
Manual QA가 필요합니다.
표시 전용: `qa_gate`와 QA record가 기준으로 남습니다.

Record: {manual_qa_record_id|none until recorded}
Gate: {qa_gate display: pending|passed|failed|waived|not_required}
Profile: {profile}
Target: {screen_or_flow}
Checklist:
- {checklist_item}

기록할 evidence:
- screenshot or walkthrough note
- browser log when relevant
- evidence를 원본 content로 기록할 수 없을 때의 redaction/omission/block note

close 맥락:
- 자동 검사: {check_refs|none; Manual QA 결과 아님}
- 검증 영향: {verification_impact}
- 수용 영향: {acceptance_impact}
- Residual Risk 또는 후속 작업: {residual_risk_or_follow_up|none}

면제 기록:
- 생략한 Manual QA 대상:
- 수용하는 위험:
- 후속 작업:
- 관련 refs:
- close 영향:
- waiver record: {manual_qa_record_id와 waiver_reason; 사용자 소유 위험이 있으면 waiver_decision_packet_ref}

Manual QA 결과를 기록하거나, 허용된 low-risk QA waiver 사유를 기록하거나, 사용자 소유 위험이 있으면 QA waiver Decision Packet을 요청하시겠습니까?
````

## 메모

이 template은 렌더링 결과인 카드 형태일 뿐 기준 QA 상태가 아닙니다. `qa_gate`는 close-relevant gate로 남습니다.

Manual QA는 사람이 확인한 기록입니다. 테스트 통과, browser smoke, screenshot capture, 검증, 사용자 수용은 close 맥락을 뒷받침할 수 있지만, `record_manual_qa`가 Manual QA 결과를 기록했거나 유효한 QA waiver가 waiver reason과 함께 `qa_gate=waived`를 갱신하고, 사용자 소유 위험이 있으면 호환되는 QA waiver Decision Packet을 포함한 경우가 아니면 Manual QA가 되지 않습니다. Waiver에 close 영향이나 위험 수용이 걸려 있는 경우 가벼운 채팅 문장만으로는 충분하지 않습니다.

Artifact가 `secret_omitted` 또는 `blocked`라면 이 card는 replacement evidence 또는 면제 기록을 요청할 수 있지만, omitted value 또는 blocked raw capture content를 표시하면 안 됩니다.
