# 검증 결과 카드(Verification Result Card) 템플릿

## 사용 시점

Eval 결과의 판정(verdict), assurance 영향, 독립성 경계, 검토한 근거, 남은 작업, 사용자 후속 조치를 간결하게 보여줄 때 Verification Result Card를 사용합니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 보증 프로필 보고서입니다. Verification/Eval display가 active일 때 사용하며 detailed `EVAL` projection은 future/diagnostic 범위입니다.

## 기준 기록

- Eval 기록
- assurance 영향과 검증 독립성 상태
- detached-candidate, self-checked, detached-verified, waived-with-accepted-risk 표시 문구
- 수동 QA와 작업 수락 영향
- 검토된 task, run, Evidence Manifest, TDD trace, diff, log, approval, design 참조
- blocker 또는 rework
- 사용자 후속 조치
- 닫기 맥락이 렌더링될 때 수동 QA, 작업 수락, Residual Risk, 검증 면제 user judgment refs, `verification_gate` status
- Eval, Evidence Manifest, 수동 QA, 작업 수락 context, Residual Risk, 검증 면제 user judgment, Artifact refs, redaction state, projection freshness를 위한 compact refs

닫기 맥락과 검증 면제 placeholder는 Eval 기록, gate 상태, QA/작업 수락 상태, Residual Risk ref, waiver user judgment ref에서 파생한 표시 전용 요약입니다. Waiver path는 그런 ref를 렌더링하거나 아직 기록이 필요하다고 표시해야 합니다.

## 렌더링 섹션

- 검증 완료
- Eval 식별 정보
- 판정
- assurance 영향
- 검증 독립성
- 수동 QA
- 작업 수락
- 검토한 근거
- 닫기 맥락
- 남은 작업
- 사용자 후속 조치

## 전체 템플릿

````text
검증이 완료되었습니다.
표시 전용: Eval record와 gate state가 기준으로 남습니다.

{eval_id}
참조: eval={eval_id}; evidence={evidence_manifest_ref|none}; manual_qa={manual_qa_ref|none}; acceptance={acceptance_context_ref|none}; residual_risk={residual_risk_refs|none}; verification_waiver={verification_waiver_user_judgment_ref|none}; artifacts={artifact_refs|none}; redaction={redaction_availability_summary|none}; freshness={projection_freshness}
판정(Verdict): {verdict}
Assurance 영향: {assurance_impact}
사용자 표시 검증 상태: {self-checked|detached candidate|detached verified|waived with accepted risk}
검증 독립성: {verification_independence}
자체 확인과 분리된 경계(Self-check vs detached boundary): {self_check_or_detached_boundary}
수동 QA: {manual_qa_impact}
작업 수락: {acceptance_impact; 별도 사용자 판단이며 이 card가 기록하지 않음}

검토한 근거:
- task summary: {task_summary_ref}
- run summary: {run_summary_ref}
- evidence manifest: {evidence_manifest_ref}
- TDD trace: {tdd_trace_ref}
- diff: {diff_ref}
- logs: {logs_ref}
- approvals: {approval_refs|none}
- approval refs는 later Approval owner profile이 active가 아닌 한 minimum MVP-1에서 `none`입니다.
- design refs: {design_refs}
- redaction 또는 차단 입력: {redaction_availability_summary|none}

닫기 맥락:
- 검증한 내용:
- 검증하지 않은 내용:
- bundle 또는 baseline 최신성: {current|stale|not_applicable}
- 수동 QA: {manual_qa_status_or_needed}
- QA waiver display: {qa_gate=waived with 수동 QA 또는 waiver refs|none}
- 작업 수락: {acceptance_status_or_needed}
- Residual Risk: {residual_risk_summary|none}
- verification waiver 표시: {필요한 경우 user judgment ref; waived이면 `verification_gate=waived_by_user`|none}
- 관련 refs: {verification_waiver_refs|none}
- 닫기 영향: {verification_waiver_close_impact|none}

남은 작업:
{blockers_or_rework}

사용자 후속 작업:
{user_followup}
````

## 메모

이 template은 렌더링 결과인 카드 형태일 뿐 검증 권한 자체가 아닙니다. Eval 기록과 gate 상태가 기준입니다.

검증(Verification)은 기록된 review boundary에서 correctness를 확인합니다. 수동 QA를 기록하거나, 사용자 작업 수락을 암시하거나, 잔여 위험을 받아들이지 않습니다. 같은 세션의 self-review는 자체 확인(self-check) 또는 review note로 보여줄 수 있지만 분리 검증으로 렌더링하면 안 됩니다. Verification waiver는 required인 경우 사용자 소유 waiver를 기록한 user judgment, `verification_gate=waived_by_user`, 생략한 확인, 받아들이는 위험, 후속 작업, 관련 refs, 닫기 영향을 보여줘야 하며, 분리 검증을 만들거나 assurance를 높이지 않습니다.

검증 통과는 작업 수락이 기록됐다는 뜻이 아닙니다. 작업 수락이 required이면 이 card는 작업 수락 상태나 필요한 action을 보여줄 수 있지만, 작업 수락은 계속 user judgment path에 남습니다.

Verification을 표시하는 동안 QA가 waive됐다면 QA waiver는 Eval verdict와 assurance line과 분리해 둡니다. QA waiver display는 `qa_gate=waived`, 수동 QA record 또는 waiver reason, 필요한 경우 QA waiver user judgment를 cite합니다. Passed 수동 QA result나 분리 검증이 아닙니다.

사용자 표시 문구는 신중하게 사용합니다. "self-checked"는 구현 경로가 자기 작업을 확인했다는 뜻입니다. "detached candidate"는 경계가 qualify할 수 있지만 아직 detached assurance를 만들지 않았다는 뜻입니다. "detached verified"는 passed Eval이 valid independence와 current inputs를 갖는다는 뜻입니다. "waived with accepted risk"는 close가 accepted visible risk에 의존하며 risk-accepted close path를 사용해야 한다는 뜻입니다. 이 표현들은 표시 문구이며 `assurance_level` 값을 추가하지 않습니다.

이 카드는 stale evaluator bundle 또는 baseline drift를 assurance blocker로 보여야 합니다. Stale bundle은 reviewed artifact로 남을 수 있지만 replacement 또는 compatible re-검증이 기록되지 않았다면 detached verified로 표시하면 안 됩니다.

이 카드는 생략되었거나 차단된 원본 bytes를 검토한 것처럼 암시하면 안 됩니다. `secret_omitted`는 보이는 nonsecret claim만 뒷받침할 수 있고, `blocked`는 문서화된 해소가 없는 한 사용할 수 없는 입력입니다.
