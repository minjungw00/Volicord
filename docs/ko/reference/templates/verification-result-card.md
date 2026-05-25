# Verification Result Card Template

## 사용 시점

Eval 결과의 verdict, assurance 영향, 검토한 evidence, 남은 작업, 사용자 후속 조치를 간결하게 보여줄 때 Verification Result Card를 사용합니다.

## 기준 기록

- Eval 기록
- assurance 영향과 verification independence 상태
- Manual QA와 수용 영향
- 검토된 task, run, Evidence Manifest, TDD trace, diff, log, approval, design 참조
- blocker 또는 rework
- 사용자 후속 조치
- 닫기 맥락이 렌더링될 때 Manual QA, 수용, Residual Risk, verification-waiver Decision Packet refs, `verification_gate` status

닫기 맥락과 verification-waiver placeholder는 Eval 기록, gate 상태, QA/acceptance 상태, Residual Risk ref, waiver Decision Packet ref에서 파생한 표시 전용 요약입니다. Waiver path는 그런 ref를 렌더링하거나 아직 기록이 필요하다고 표시해야 합니다.

## 렌더링 섹션

- verification completion
- Eval identity
- verdict
- assurance
- verification independence
- Manual QA
- 수용
- 검토한 evidence
- 닫기 맥락
- 남은 작업
- 사용자 후속 조치

## 전체 템플릿

````text
Verification이 완료되었습니다.
표시 전용: Eval record와 gate state가 기준으로 남습니다.

{eval_id}
Verdict: {verdict}
Assurance: {assurance_impact}
Verification independence: {verification_independence}
Manual QA: {manual_qa_impact}
Acceptance: {acceptance_impact}

검토한 evidence:
- task summary: {task_summary_ref}
- run summary: {run_summary_ref}
- evidence manifest: {evidence_manifest_ref}
- TDD trace: {tdd_trace_ref}
- diff: {diff_ref}
- logs: {logs_ref}
- approvals: {approval_refs}
- design refs: {design_refs}
- redaction 또는 차단 입력: {redaction_availability_summary|none}

닫기 맥락:
- 검증한 내용:
- 검증하지 않은 내용:
- Manual QA: {manual_qa_status_or_needed}
- 수용: {acceptance_status_or_needed}
- Residual Risk: {residual_risk_summary|none}
- verification waiver display: {필요한 경우 Decision Packet ref; waived이면 `verification_gate=waived_by_user`|none}
- 관련 refs: {verification_waiver_refs|none}
- 닫기 영향: {verification_waiver_close_impact|none}

남은 작업:
{blockers_or_rework}

사용자 후속 작업:
{user_followup}
````

## 메모

이 template은 렌더링 결과인 카드 형태일 뿐 verification 권한 자체가 아닙니다. Eval 기록과 gate 상태가 기준입니다.

Verification은 기록된 review boundary에서 correctness를 확인합니다. Manual QA를 기록하거나, 사용자 수용을 암시하거나, Residual Risk를 수용하지 않습니다. 같은 세션의 self-review는 self-check 또는 review note로 보여줄 수 있지만 detached verification으로 렌더링하면 안 됩니다. Verification waiver는 required인 경우 사용자 소유 waiver를 기록한 Decision Packet, `verification_gate=waived_by_user`, 생략한 확인, 수용하는 위험, 후속 작업, 관련 refs, 닫기 영향을 보여줘야 하며, detached verification을 만들거나 assurance를 높이지 않습니다.

이 card는 생략되었거나 차단된 원본 bytes를 검토한 것처럼 암시하면 안 됩니다. `secret_omitted`는 보이는 nonsecret claim만 뒷받침할 수 있고, `blocked`는 documented resolution이 없는 한 사용할 수 없는 input입니다.
