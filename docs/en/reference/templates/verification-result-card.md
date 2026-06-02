# Verification Result Card Template

## Used when

Use the verification result card when an Eval result needs a compact user-facing display of verdict, assurance impact, independence boundary, reviewed evidence, remaining work, and user follow-up.

Boundary: projection template only; it does not authorize runtime/server implementation or generated operational outputs. Shared phase and projection rules live in [Template Reference](README.md#used-when).

Implementation tier: Agency assurance reports. Use when verification/Eval display is active; the detailed `EVAL` projection remains future/diagnostic.

## Source records

- Eval record
- assurance impact and verification independence state
- detached-candidate, self-checked, detached-verified, and waived-with-accepted-risk display wording
- Manual QA and acceptance impact
- reviewed task, run, Evidence Manifest, TDD trace, diff, log, approval, and design refs
- blockers or rework
- user follow-up
- Manual QA, work acceptance and Residual Risk, verification-waiver Decision Packet refs, and `verification_gate` status when close context is rendered
- compact refs for Eval, Evidence Manifest, Manual QA, work acceptance context, Residual Risk, verification-waiver Decision Packet, Artifact refs, redaction state, and projection freshness

Close context and verification-waiver placeholders are derived display summaries from Eval records, gate state, QA/work acceptance status, Residual Risk refs, and waiver Decision Packet refs. Waiver paths should render those refs or say that recording is still needed.

## Rendered sections

- verification completion
- Eval identity
- verdict
- assurance
- verification independence
- Manual QA
- acceptance
- evidence reviewed
- close context
- remaining work
- user follow-up

## Full template

````text
Verification complete.
Display only: Eval records and gate state remain canonical.

{eval_id}
Refs: eval={eval_id}; evidence={evidence_manifest_ref|none}; manual_qa={manual_qa_ref|none}; acceptance={acceptance_context_ref|none}; residual_risk={residual_risk_refs|none}; verification_waiver={verification_waiver_decision_packet_ref|none}; artifacts={artifact_refs|none}; redaction={redaction_availability_summary|none}; freshness={projection_freshness}
Verdict: {verdict}
Assurance: {assurance_impact}
User-facing verification status: {self-checked|detached candidate|detached verified|waived with accepted risk}
Verification independence: {verification_independence}
Self-check vs detached boundary: {self_check_or_detached_boundary}
Manual QA: {manual_qa_impact}
Work acceptance: {acceptance_impact; separate user judgment, not recorded by this card}

Evidence reviewed:
- task summary: {task_summary_ref}
- run summary: {run_summary_ref}
- evidence manifest: {evidence_manifest_ref}
- TDD trace: {tdd_trace_ref}
- diff: {diff_ref}
- logs: {logs_ref}
- approvals: {approval_refs}
- design refs: {design_refs}
- redaction or blocked input: {redaction_availability_summary|none}

Close context:
- verification checked:
- verification did not check:
- bundle or baseline freshness: {current|stale|not_applicable}
- Manual QA: {manual_qa_status_or_needed}
- QA waiver display: {qa_gate=waived with Manual QA or waiver refs|none}
- acceptance: {acceptance_status_or_needed}
- residual risk: {residual_risk_summary|none}
- verification waiver display: {Decision Packet ref when required; `verification_gate=waived_by_user` when waived|none}
- relevant refs: {verification_waiver_refs|none}
- close impact: {verification_waiver_close_impact|none}

Remaining work:
{blockers_or_rework}

User follow-up:
{user_followup}
````

## Notes

This template is a rendered card shape, not verification authority. Eval records and gate state remain canonical.

Verification checks correctness from the recorded review boundary. It does not record Manual QA, imply work acceptance, or accept residual risk. Same-session self-review may be shown as a self-check or review note, but it must not be rendered as detached verification. A verification waiver should show the Decision Packet that records the user-owned waiver when required, `verification_gate=waived_by_user`, skipped check, accepted risk, follow-up, relevant refs, and close impact; it does not create detached verification or upgrade assurance.

Passing verification does not mean the user accepted the result. If work acceptance is required, this card may show the work acceptance status or needed action, but work acceptance remains on the Decision Packet path.

If QA is waived while verification is displayed, keep the QA waiver separate from the Eval verdict and assurance line. QA waiver display cites `qa_gate=waived`, the Manual QA record or waiver reason, and a QA waiver Decision Packet when required; it is not a passed Manual QA result or detached verification.

Use user-facing wording carefully: "self-checked" means the implementing path checked its own work; "detached candidate" means the boundary may qualify but has not yet produced detached assurance; "detached verified" means a passed Eval has valid independence and current inputs; "waived with accepted risk" means close relies on accepted visible risk and must use the risk-accepted close path. These phrases are display wording and do not add `assurance_level` values.

The card must show stale evaluator bundles or baseline drift as an assurance blocker. A stale bundle can remain a reviewed artifact, but it must not be presented as detached verified unless replacement or compatible re-verification has been recorded.

The card must not imply omitted or blocked raw bytes were reviewed. `secret_omitted` can support only visible nonsecret claims; `blocked` is unavailable input unless a documented resolution exists.
