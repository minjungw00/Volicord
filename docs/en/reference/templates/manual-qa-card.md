# Manual QA Card Template

## Used when

Use the Manual QA card when required Manual QA needs a compact prompt showing the record, gate, profile, target, checklist, and evidence to record.

## Source records

- Manual QA requirement and `qa_gate`
- Manual QA record, if one exists
- QA profile
- target screen or flow
- checklist items
- expected screenshot, walkthrough note, or browser log evidence
- waiver reason, QA waiver Decision Packet refs when required, and Residual Risk refs when QA is waived or deferred
- verification, acceptance, and close-impact summaries

Close context and waiver placeholders are derived display summaries from QA records, `qa_gate`, related gate states, Decision Packet refs, and Residual Risk refs. Waiver paths should render those refs or say that recording is still needed.

## Rendered sections

- Manual QA requirement
- record
- gate
- profile
- target
- checklist
- evidence to record
- close context
- waiver recording
- result prompt

## Full template

````text
Manual QA is required.
Display only: `qa_gate` and QA records remain canonical.

Record: {manual_qa_record_id|none until recorded}
Gate: {qa_gate display: pending|passed|failed|waived|not_required}
Profile: {profile}
Target: {screen_or_flow}
Checklist:
- {checklist_item}

Evidence to record:
- screenshot or walkthrough note
- browser log when relevant
- redaction/omission/block note when evidence cannot be recorded as raw content

Close context:
- automated checks: {check_refs|none; not a Manual QA result}
- verification impact: {verification_impact}
- acceptance impact: {acceptance_impact}
- residual risk or follow-up: {residual_risk_or_follow_up|none}

Waiver recording:
- skipped Manual QA surface:
- accepted risk:
- follow-up:
- relevant refs:
- close impact:
- waiver record: {manual_qa_record_id and waiver_reason; waiver_decision_packet_ref when user-owned risk is involved}

Record the Manual QA result, record an allowed low-risk QA waiver reason, or request a QA waiver Decision Packet for user-owned risk?
````

## Notes

This template is a rendered card shape, not canonical QA state. `qa_gate` remains the close-relevant gate.

Manual QA is human inspection. Passing tests, browser smoke, screenshot capture, verification, or user acceptance may support the close context, but they do not become Manual QA unless `record_manual_qa` records a Manual QA result or a valid QA waiver updates `qa_gate=waived` with a waiver reason and, when user-owned risk is involved, a compatible QA waiver Decision Packet. A chat statement alone is not enough when the waiver affects close or accepted risk.

The card may ask for replacement evidence or waiver recording when an artifact is `secret_omitted` or `blocked`, but it must not display omitted values or blocked raw capture content.
