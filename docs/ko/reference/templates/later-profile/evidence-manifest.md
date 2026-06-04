# EVIDENCE-MANIFEST 템플릿

## 사용 시점

수용 기준, completion condition, 닫기에 영향을 주는 claim이 어떤 supporting evidence와 artifact ref로 뒷받침되는지 보여줘야 할 때 `EVIDENCE-MANIFEST`를 사용합니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 향후/진단용 projections입니다. MVP-1은 상태 카드 또는 실행/근거 요약으로 근거 요약과 공백을 보여주며, full detailed Evidence Manifest projection은 later/profile scope입니다.

## 기준 기록

- evidence manifest 기록
- 수용 기준
- completion condition
- changed file coverage
- design-quality coverage
- approval refs(later Approval profile only; 그 외에는 none)
- hash, size, redaction state, retention/availability, owner relation, 후속 evidence 영향을 포함한 artifact 참조
- 관련 Run, Eval, Feedback Loop, 수동 QA, TDD trace 참조
- 닫기 맥락으로 렌더링할 때 닫기에 영향을 주는 검증, 수동 QA, 작업 수락, 잔여 위험 요약
- 닫기 맥락으로 렌더링할 때 Write Authorization, User Judgment, Approval, Evidence Manifest, Eval, 수동 QA, acceptance context, Residual Risk, Artifact refs, redaction state, projection freshness를 보여주는 compact authority refs

## 렌더링 섹션

- 식별 정보
- 요약
- 닫기 영향 요약
- 권한과 닫기 참조
- 수용 기준 coverage
- 완료 조건 coverage
- 변경 파일 coverage
- 설계 품질 coverage
- Approval 참조
- 근거 참조
- Redaction과 사용 가능성
- 오래된 것으로 보는 조건

## 전체 템플릿

````md
---
doc_type: evidence_manifest
evidence_manifest_id: EM-0001
task_id: TASK-0001
change_unit_id: CU-01
status: partial
source_state_version: 44
updated_at: 2026-05-06T09:50:00+09:00
---

# EM-0001 근거 목록(Evidence Manifest)

> Projection 보기: `source_state_version`와 `updated_at` 기준으로 렌더링되며 owner record와 artifact ref의 대응을 표시합니다. Close는 Markdown edit가 아니라 기준 `evidence_gate`와 관련 state를 따릅니다.

## 식별 정보
- task_id:
- change_unit_id:
- baseline_ref:
- run_summary:
- latest_eval:

## 요약
- evidence 상태:
- unsupported criteria:
- 생략/차단 evidence 영향:
- stale conditions:
- 다음 evidence action:

## 닫기 영향 요약
- 근거가 뒷받침하는 것:
- 근거가 대체하지 않는 것: 검증, 수동 QA, 작업 수락, 잔여 위험 표시, 잔여 위험 수용
- 검증 상태:
- 수동 QA 상태:
- 작업 수락 상태:
- 잔여 위험 표시:
- 잔여 위험 수용:
- close/assurance 표시 구분:
- 다음 close 조치:

## 권한과 닫기 참조
- 간결한 refs: write={write_authorization_ref|none}; judgment={user_judgment_refs|none}; approval={approval_refs|none}; evidence={evidence_manifest_id}; eval={eval_ref|none}; manual_qa={manual_qa_ref|none}; acceptance={acceptance_context_ref|none}; residual_risk={residual_risk_refs|none}; artifacts={artifact_refs|none}
- approval refs는 minimum MVP-1에서 `none`입니다. 민감 동작 coverage는 later Approval owner profile이 active가 아닌 한 `judgment_type=sensitive_action_approval`인 `user_judgment_refs`로 나타납니다.
- redaction state:
- projection freshness:

## 수용 기준 coverage
| AC ID | Statement | Coverage 상태 | Run Refs | ArtifactRef Refs | Supporting State Refs | Notes |
|---|---|---|---|---|---|---|
| AC-01 | | supported | RUN-0001 | ART-TEST-0001, ART-DIFF-0001 | FBL-0001 | |
| AC-02 | | unsupported | | | | |

## 완료 조건 coverage
| Condition | Coverage 상태 | Run Refs | ArtifactRef Refs | Supporting State Refs | Notes |
|---|---|---|---|---|---|
| | supported | RUN-0001 | ART-0001 | | |
| | unsupported | | | | |

## 변경 파일 coverage
| Path | Covered Criteria | 근거 참조 |
|---|---|---|
| `src/...` | AC-01 | DIFF-0001, LOG-0001 |

## 설계 품질 coverage
| Item | Coverage / 관문 표시 상태 | 근거 참조 | Notes |
|---|---|---|---|
| vertical_slice_shape | passed | CU-01 | |
| decision_quality_check | passed | UJ-0001 | |
| autonomy_boundary_check | passed | CU-01 | |
| feedback_loop_check | passed | FBL-0001, TDD-0001, LOG-0001 | |
| tdd_trace_required | passed | TDD-0001, RED-LOG-0001, GREEN-LOG-0001 | RED, GREEN, relevant refactor/check coverage가 수용 기준 및 changed files로 link된다. |
| module_interface_review | passed | module_map_item: MMI-0001, interface_contract: IFACE-0001, UJ-0001 | |
| codebase_stewardship_check | passed | domain_term: TERM-0001, module_map_item: MMI-0001, interface_contract: IFACE-0001, feedback_loop: FBL-0001 | |
| residual_risk_visibility_check | pending | RR-0001 | |
| manual_qa_required | pending | qa_gate; no satisfying 수동 QA record yet | |

`Coverage / 관문 표시 상태`는 이 manifest의 evidence coverage 또는 close와 관련된 관문 표시 상태입니다. 이 column의 `pending` 같은 값은 `ValidatorResult.status` 값이 아닙니다.

## Approval 참조
- Later Approval owner profile이 active일 때만 채웁니다. Minimum MVP-1의 민감 동작 coverage는 `judgment_type=sensitive_action_approval`인 `user_judgment_refs`에 둡니다.
- APR-0001:

## 근거 참조
- run summary:
- feedback loop:
- TDD trace:
- TDD RED target / plan:
- TDD red:
- TDD green:
- TDD refactor/check:
- 수동 QA:
- diff:
- logs:
- bundle:
- checkpoint:
- tests:
- build:

## Redaction과 사용 가능성
| Artifact Ref | Hash / Size | Redaction State | Retention / Availability | Evidence Effect | Note |
|---|---|---|---|---|---|
| ART-0001 | sha256:abc123... / 12 KB | secret_omitted | retained ref; raw secret omitted | 보이는 nonsecret fact만 지원 | |
| ART-0002 | sha256:def456... / 1 KB | blocked | metadata-only notice | 사용할 수 없는 입력; claim은 해소 전까지 insufficient | |

## 오래된 것으로 보는 조건
- recorded baseline에서 baseline drift가 발생함
- supporting Run 또는 Eval 이후 changed files가 수정됨
- Approval 범위가 만료되거나 drift됨
- supporting artifact가 missing, blocked, 또는 integrity failure 상태가 됨
- supporting artifact hash 또는 size가 registered ref와 더 이상 일치하지 않음
- relevant config changes
- relevant Shared Design, domain term, module map item, interface contract records change
````

## 메모

근거(Evidence)가 필요한 경우 닫기 판단은 보고서 문장만이 아니라 기준 `evidence_gate`를 따릅니다.

Evidence sufficiency는 artifact 개수가 아니라 수용 기준, completion conditions, close-relevant claims의 coverage에 달려 있습니다. Required row에 current supporting refs가 없으면 artifact가 많아도 manifest는 partial로 남습니다. 작은 direct docs-only Task는 모든 required condition을 cover한다면 Run ref 하나와 diff artifact 하나만으로도 sufficient일 수 있습니다.

Coverage mapping 예시:

| Criterion / Condition | Run Refs | ArtifactRef Refs | Supporting State Refs | Sufficiency Note |
|---|---|---|---|---|
| AC-01 docs typo corrected without meaning change | RUN-DOCS-001 | ART-DIFF-001 | | Changed doc path와 self-check가 stated docs-only condition을 cover할 때만 sufficient입니다. |
| AC-02 login form submits email | RUN-FEATURE-001 | ART-DIFF-002, ART-TEST-002 | FBL-001 | Run, diff, test/log refs가 Task 전체가 아니라 이 AC에 map될 때 supported입니다. |
| AC-03 final button copy is readable in target viewport | RUN-UI-001 | ART-SCREENSHOT-001, ART-DIFF-003 | QA-0001 | 수동 QA가 required이면 screenshot이나 browser smoke만으로 QA path를 충족하지 않습니다. |
| AC-04 export contains only approved redacted fields | RUN-EXPORT-001 | ART-EXPORT-MANIFEST-001, ART-LOG-001 | APR-0001, DEC-0001 | `APR-0001`은 later Approval profile이 active일 때만 있습니다. Approval과 Decision refs는 scope 또는 사용자 판단 맥락을 보여줍니다. Redacted artifact refs는 여전히 nonsecret claim을 증명해야 합니다. |
| Completion condition: independent verifier reviewed the changed scope | RUN-VERIFY-001 | ART-BUNDLE-001 | EVAL-0001 | Eval이 current refs를 review했고 requested close에 필요한 independence가 있을 때만 valid합니다. |

Evidence Manifest는 주장을 뒷받침하지만 그 자체로 correctness를 증명하거나 분리 검증을 만들거나 수동 QA를 기록하거나 작업 수락을 암시하거나 잔여 위험을 보이게 하거나 잔여 위험을 수용하지 않습니다. 이 template에서 닫기 영향 요약을 렌더링할 때는 테스트 통과, 자체 확인(self-check), QA 면제 판단, 사용자의 작업 수락이 서로 다른 닫기 조건으로 오해되지 않도록 각 줄을 분리해 보여줘야 합니다.

닫기 맥락을 보여줄 때 manifest는 잔여 위험 수용 close, 검증 면제, QA waiver, self-checked, `detached_verified`를 owner ref 또는 명시적인 absence와 함께 서로 다른 표시 상태로 렌더링해야 합니다. 이 label은 owner record를 읽기 쉽게 요약할 뿐이며 Evidence Manifest authority가 아닙니다.

Coverage row는 큰 근거 본문을 붙여 넣는 대신 owner record와 ArtifactRef ref를 가리켜야 합니다. 어떤 criterion, condition, claim을 뒷받침하는 ref가 없다면 문장으로 빈틈을 메우지 말고 unsupported, insufficient, stale, blocked 중 적절한 상태로 보여줍니다.

Chat text와 Markdown report prose는 evidence story를 설명할 수 있지만, 관련 criteria가 compatible owner records와 registered ArtifactRef refs를 가리키지 않는 한 sufficiency를 증명하기에는 충분하지 않습니다.

Large log, diff, screenshot, trace, bundle은 짧은 결과와 함께 registered ArtifactRef ref로 남겨야 합니다. Manifest는 reader가 artifact body를 열어 보기 전에 redaction state와 availability를 먼저 보여줘야 합니다.

`secret_omitted` artifact는 secret이 아닌 evidence가 보이는 주장만 뒷받침할 수 있으며, 생략된 값이 필요한 주장은 뒷받침하지 못합니다. `blocked` artifact는 커밋된 metadata-only notice이지 사용 가능한 원본 근거가 아닙니다. 의존하는 criteria는 replacement, waiver, user judgment outcome, 받아들인 위험, documented fallback이 evidence 경로를 해소할 때까지 unsupported, insufficient, blocked 중 적절한 상태로 남습니다. 이 template은 생략된 secret/PII 값 또는 차단된 payload를 포함하면 안 됩니다.
