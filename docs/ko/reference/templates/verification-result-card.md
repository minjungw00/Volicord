# Verification Result Card Template

## 사용 시점

Eval 결과의 판정(verdict), assurance 영향, 독립성 경계, 검토한 근거(evidence), 남은 작업, 사용자 후속 조치를 간결하게 보여줄 때 Verification Result Card를 사용합니다.

이 문서는 template 참조 문서입니다. 재설계 문서가 승인되기 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture 파일, runtime data를 만들라는 뜻이 아닙니다. 첫 구현/증명 대상은 계속 Kernel Smoke입니다. Agency-Hardened MVP와 post-MVP automation은 owner 문서가 승격하고 증명하기 전까지 범위 밖입니다.

## 기준 기록

- Eval 기록
- assurance 영향과 verification independence 상태
- detached-candidate, self-checked, detached-verified, waived-with-accepted-risk 표시 문구
- Manual QA와 결과 수락 영향
- 검토된 task, run, Evidence Manifest, TDD trace, diff, log, approval, design 참조
- blocker 또는 rework
- 사용자 후속 조치
- 닫기 맥락이 렌더링될 때 Manual QA, 결과 수락, Residual Risk, verification-waiver Decision Packet refs, `verification_gate` status

닫기 맥락과 verification-waiver placeholder는 Eval 기록, gate 상태, QA/결과 수락 상태, Residual Risk ref, waiver Decision Packet ref에서 파생한 표시 전용 요약입니다. Waiver path는 그런 ref를 렌더링하거나 아직 기록이 필요하다고 표시해야 합니다.

## 렌더링 섹션

- verification completion
- Eval identity
- verdict
- assurance
- verification independence
- Manual QA
- 결과 수락
- 검토한 evidence
- 닫기 맥락
- 남은 작업
- 사용자 후속 조치

## 전체 템플릿

````text
검증이 완료되었습니다.
표시 전용: Eval record와 gate state가 기준으로 남습니다.

{eval_id}
판정(Verdict): {verdict}
Assurance 영향: {assurance_impact}
사용자 표시 검증 상태: {self-checked|detached candidate|detached verified|waived with accepted risk}
검증 독립성(verification independence): {verification_independence}
자체 확인과 분리된 경계(Self-check vs detached boundary): {self_check_or_detached_boundary}
Manual QA: {manual_qa_impact}
결과 수락: {acceptance_impact}

검토한 근거(evidence):
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
- bundle 또는 baseline freshness: {current|stale|not_applicable}
- Manual QA: {manual_qa_status_or_needed}
- 결과 수락: {acceptance_status_or_needed}
- Residual Risk: {residual_risk_summary|none}
- verification waiver 표시: {필요한 경우 Decision Packet ref; waived이면 `verification_gate=waived_by_user`|none}
- 관련 refs: {verification_waiver_refs|none}
- 닫기 영향: {verification_waiver_close_impact|none}

남은 작업:
{blockers_or_rework}

사용자 후속 작업:
{user_followup}
````

## 메모

이 template은 렌더링 결과인 카드 형태일 뿐 verification 권한 자체가 아닙니다. Eval 기록과 gate 상태가 기준입니다.

검증(Verification)은 기록된 review boundary에서 correctness를 확인합니다. Manual QA를 기록하거나, 사용자 결과 수락을 암시하거나, 남은 위험을 받아들이지 않습니다. 같은 세션의 self-review는 자체 확인(self-check) 또는 review note로 보여줄 수 있지만 detached verification으로 렌더링하면 안 됩니다. Verification waiver는 required인 경우 사용자 소유 waiver를 기록한 Decision Packet, `verification_gate=waived_by_user`, 생략한 확인, 받아들이는 위험, 후속 작업, 관련 refs, 닫기 영향을 보여줘야 하며, detached verification을 만들거나 assurance를 높이지 않습니다.

사용자 표시 문구는 신중하게 사용합니다. "self-checked"는 구현 경로가 자기 작업을 확인했다는 뜻입니다. "detached candidate"는 경계가 qualify할 수 있지만 아직 detached assurance를 만들지 않았다는 뜻입니다. "detached verified"는 passed Eval이 valid independence와 current inputs를 갖는다는 뜻입니다. "waived with accepted risk"는 close가 accepted visible risk에 의존하며 risk-accepted close path를 사용해야 한다는 뜻입니다. 이 표현들은 표시 문구이며 `assurance_level` 값을 추가하지 않습니다.

이 카드는 stale evaluator bundle 또는 baseline drift를 assurance blocker로 보여야 합니다. Stale bundle은 reviewed artifact로 남을 수 있지만 replacement 또는 compatible re-verification이 기록되지 않았다면 detached verified로 표시하면 안 됩니다.

이 카드는 생략되었거나 차단된 원본 bytes를 검토한 것처럼 암시하면 안 됩니다. `secret_omitted`는 보이는 nonsecret claim만 뒷받침할 수 있고, `blocked`는 문서화된 해소가 없는 한 사용할 수 없는 입력입니다.
