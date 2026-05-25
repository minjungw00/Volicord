# Verification Result Card Template

## Used when

Use the verification result card when an Eval result needs a compact user-facing display of verdict, assurance impact, reviewed evidence, remaining work, and user follow-up.

## Source records

- Eval record
- assurance impact and verification independence state
- Manual QA and acceptance impact
- reviewed task, run, Evidence Manifest, TDD trace, diff, log, approval, and design refs
- blockers or rework
- user follow-up
- Manual QA, acceptance, Residual Risk, verification-waiver Decision Packet refs, and `verification_gate` status when close context is rendered

Close context and verification-waiver placeholders are derived display summaries from Eval records, gate state, QA/acceptance status, Residual Risk refs, and waiver Decision Packet refs. Waiver paths should render those refs or say that recording is still needed.

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
Verdict: {verdict}
Assurance: {assurance_impact}
Verification independence: {verification_independence}
Manual QA: {manual_qa_impact}
Acceptance: {acceptance_impact}

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
- Manual QA: {manual_qa_status_or_needed}
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

Verification checks correctness from the recorded review boundary. It does not record Manual QA, imply user acceptance, or accept residual risk. Same-session self-review may be shown as a self-check or review note, but it must not be rendered as detached verification. A verification waiver should show the Decision Packet that records the user-owned waiver when required, `verification_gate=waived_by_user`, skipped check, accepted risk, follow-up, relevant refs, and close impact; it does not create detached verification or upgrade assurance.

The card must not imply omitted or blocked raw bytes were reviewed. `secret_omitted` can support only visible nonsecret claims; `blocked` is unavailable input unless a documented resolution exists.
