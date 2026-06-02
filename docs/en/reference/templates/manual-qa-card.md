# Manual QA Card Template

## Used when

Use the Manual QA card when Manual QA needs a compact human-inspection prompt showing the record, gate, profile, target, checklist, evidence to record, and waiver/risk visibility.

Boundary: projection template only; it does not authorize runtime/server implementation or generated operational outputs. Shared phase and projection rules live in [Template Reference](README.md#used-when).

Implementation tier: Agency assurance reports. Use when a Manual QA profile is explicitly active; full Manual QA policy coverage remains later staged scope.

## Source records

- Manual QA requirement and `qa_gate`
- Manual QA record, if one exists
- QA profile
- human inspector or role and the human judgment being requested
- target screen or flow
- checklist items
- expected screenshot, walkthrough note, browser log, Browser QA artifact, or manually supplied artifact evidence
- waiver reason, QA waiver Decision Packet refs when required, and Residual Risk refs when QA is waived or deferred
- verification, work acceptance, and close-impact summaries
- compact refs for Manual QA record, QA waiver Decision Packet, Evidence Manifest, Eval, acceptance context, Residual Risk, Artifact refs, redaction state, and projection freshness

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
Human inspection only: automated checks, screenshots, browser logs, and Browser QA artifacts can support context, but they are not Manual QA by themselves.
Browser QA Capture: useful when promoted and supported; not work acceptance, not detached verification without an independent Eval, and not a replacement for required human inspection.

Record: {manual_qa_record_id|none until recorded}
Gate: {qa_gate display: not_required|required|pending|passed|failed|waived}
Refs: manual_qa={manual_qa_record_id|none}; qa_waiver_decision={qa_waiver_decision_packet_ref|none}; evidence={evidence_manifest_ref|none}; eval={eval_ref|none}; acceptance={acceptance_context_ref|none}; residual_risk={residual_risk_refs|none}; artifacts={artifact_refs|none}; redaction={redaction_availability_summary|none}; freshness={projection_freshness}
Profile: {profile}
Human inspection requested: {human_inspection_summary}
Target: {screen_or_flow}
Checklist:
- {checklist_item}

Evidence to record:
- screenshot or walkthrough note
- qa_capture artifact when promoted and supported
- browser log when relevant
- manually supplied artifact or human note when browser capture is unsupported
- redaction/omission/block note when evidence cannot be recorded as raw content

Close context:
- automated checks: {check_refs|none; not a Manual QA result}
- Browser QA artifacts: {artifact_refs|none; supporting refs only}
- QA waiver display: {qa_gate=waived with waiver refs|none}
- verification impact: {verification_impact}
- work acceptance impact: {acceptance_impact; not recorded by this card}
- residual risk or follow-up: {residual_risk_or_follow_up|none}

Waiver recording:
- skipped Manual QA surface:
- risk visible before waiver:
- accepted risk:
- follow-up:
- relevant refs:
- close impact:
- waiver source: {manual_qa_record_id and waiver_reason; waiver_decision_packet_ref when user-owned risk is involved}

Record the Manual QA result, record an allowed low-risk QA waiver reason, or request a QA waiver Decision Packet for user-owned risk?
````

## Notes

This template is a rendered card shape, not canonical QA state. `qa_gate` remains the close-relevant gate.

Manual QA is human inspection. Passing tests, browser smoke, screenshot capture, Browser QA Capture artifacts, verification, or work acceptance may support the close context, but they do not become Manual QA unless `record_manual_qa` records a Manual QA result or a valid QA waiver updates `qa_gate=waived` with a waiver reason and, when user-owned risk is involved, a compatible QA waiver Decision Packet. Browser QA Capture remains a v1+ Expansion candidate unless owner docs explicitly promote it, and captured artifacts do not record work acceptance or detached verification unless a separate Eval path satisfies independence. A chat statement alone is not enough when the waiver affects close or accepted risk.

The card should render pending QA, passed QA, failed QA, and waived QA as separate display states. Waived QA cites the Manual QA record or waiver reason, the QA waiver Decision Packet when required, residual-risk refs when applicable, and close impact; it is not a passed inspection.

The result prompt should ask only for a Manual QA result or QA waiver path. It must not ask for work acceptance, residual-risk acceptance, or both as if those were the same answer.

The card may ask for replacement evidence or waiver recording when an artifact is `secret_omitted` or `blocked`, but it must not display omitted values or blocked raw capture content. When browser capture is unsupported for the surface, the card should ask for human Manual QA notes and manually supplied artifacts instead of treating capture absence as a QA result.
