# EVIDENCE-MANIFEST Template

## Used when

Use `EVIDENCE-MANIFEST` when Harness needs a readable map from acceptance criteria, completion conditions, and close-relevant claims to supporting evidence and artifact refs.

Boundary: projection template only; it does not authorize runtime/server implementation or generated operational outputs. Shared phase and projection rules live in [Template Reference](README.md#used-when).

Implementation tier: Future/diagnostic projections. The user-facing MVP needs an evidence summary, not the full detailed Evidence Manifest projection.

## Source records

- evidence manifest record
- acceptance criteria
- completion conditions
- changed file coverage
- design-quality coverage
- approval refs
- artifact refs with hash, size, redaction state, retention/availability, owner relation, and downstream evidence impact
- related Run, Eval, Feedback Loop, Manual QA, and TDD trace refs
- close-relevant verification, Manual QA, work acceptance, and Residual Risk summaries when rendered with close context
- compact authority refs for Write Authorization, Decision Packet, Approval, Evidence Manifest, Eval, Manual QA, Acceptance context, Residual Risk, Artifact refs, redaction state, and projection freshness when rendered with close context

## Rendered sections

- Identity
- Summary
- Close Summary
- Authority And Close Refs
- Acceptance Criteria Coverage
- Completion Conditions Coverage
- Changed File Coverage
- Design Quality Coverage
- Approval Refs
- Evidence Refs
- Redaction And Availability
- Stale If

## Full template

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

# EM-0001 Evidence Manifest

> Projection view: rendered from `source_state_version` at `updated_at`; maps owner records and artifact refs. Close follows canonical `evidence_gate` and related state, not Markdown edits.

## Identity
- task_id:
- change_unit_id:
- baseline_ref:
- run_summary:
- latest_eval:

## Summary
- evidence state:
- unsupported criteria:
- omitted or blocked evidence impact:
- stale conditions:
- next evidence action:

## Close Summary
- evidence supports:
- evidence does not replace: verification, Manual QA, work acceptance, residual-risk visibility, and residual-risk acceptance
- verification status:
- Manual QA status:
- work acceptance status:
- residual-risk visibility:
- residual-risk acceptance:
- close/assurance display distinction:
- next close action:

## Authority And Close Refs
- compact refs: write={write_authorization_ref|none}; decision={decision_packet_refs|none}; approval={approval_refs|none}; evidence={evidence_manifest_id}; eval={eval_ref|none}; manual_qa={manual_qa_ref|none}; acceptance={acceptance_context_ref|none}; residual_risk={residual_risk_refs|none}; artifacts={artifact_refs|none}
- redaction state:
- projection freshness:

## Acceptance Criteria Coverage
| AC ID | Statement | Coverage State | Run Refs | ArtifactRef Refs | Supporting State Refs | Notes |
|---|---|---|---|---|---|---|
| AC-01 | | supported | RUN-0001 | ART-TEST-0001, ART-DIFF-0001 | FBL-0001 | |
| AC-02 | | unsupported | | | | |

## Completion Conditions Coverage
| Condition | Coverage State | Run Refs | ArtifactRef Refs | Supporting State Refs | Notes |
|---|---|---|---|---|---|
| | supported | RUN-0001 | ART-0001 | | |
| | unsupported | | | | |

## Changed File Coverage
| Path | Covered Criteria | Evidence Refs |
|---|---|---|
| `src/...` | AC-01 | DIFF-0001, LOG-0001 |

## Design Quality Coverage
| Item | Coverage / Gate Display | Evidence Refs | Notes |
|---|---|---|---|
| vertical_slice_shape | passed | CU-01 | |
| decision_quality_check | passed | DEC-0001 | |
| autonomy_boundary_check | passed | CU-01 | |
| feedback_loop_check | passed | FBL-0001, TDD-0001, LOG-0001 | |
| tdd_trace_required | passed | TDD-0001, RED-LOG-0001, GREEN-LOG-0001 | RED, GREEN, and relevant refactor/check coverage link back to acceptance criteria and changed files. |
| module_interface_review | passed | module_map_item: MMI-0001, interface_contract: IFACE-0001, DEC-0001 | |
| codebase_stewardship_check | passed | domain_term: TERM-0001, module_map_item: MMI-0001, interface_contract: IFACE-0001, feedback_loop: FBL-0001 | |
| residual_risk_visibility_check | pending | RR-0001 | |
| manual_qa_required | pending | qa_gate; no satisfying Manual QA record yet | |

`Coverage / Gate Display` is the evidence coverage or close-relevant gate display state for this manifest. Values such as `pending` in this column are not `ValidatorResult.status` values.

## Approval Refs
- APR-0001:

## Evidence Refs
- run summary:
- feedback loop:
- TDD trace:
- TDD RED target / plan:
- TDD red:
- TDD green:
- TDD refactor/check:
- Manual QA:
- diff:
- logs:
- bundle:
- checkpoint:
- tests:
- build:

## Redaction And Availability
| Artifact Ref | Hash / Size | Redaction State | Retention / Availability | Evidence Effect | Note |
|---|---|---|---|---|---|
| ART-0001 | sha256:abc123... / 12 KB | secret_omitted | retained ref; raw secret omitted | supports visible nonsecret facts only | |
| ART-0002 | sha256:def456... / 1 KB | blocked | metadata-only notice | unavailable input; claim remains insufficient until resolved | |

## Stale If
- baseline drift from the recorded baseline
- changed files are modified after supporting Run or Eval
- approval scope expires or drifts
- supporting artifact is missing, blocked, or fails integrity
- supporting artifact hash or size no longer matches its registered ref
- relevant config changes
- relevant Shared Design, domain term, module map item, or interface contract records change
````

## Notes

Where evidence is required, close depends on the canonical `evidence_gate`, not the report text alone.

Evidence sufficiency depends on coverage of acceptance criteria, completion conditions, and close-relevant claims, not on artifact count. A manifest with many artifacts remains partial when a required row has no current supporting refs; a small direct docs-only task may be sufficient with one Run ref and one diff artifact when they cover every required condition.

Example coverage mappings:

| Criterion / Condition | Run Refs | ArtifactRef Refs | Supporting State Refs | Sufficiency Note |
|---|---|---|---|---|
| AC-01 docs typo corrected without meaning change | RUN-DOCS-001 | ART-DIFF-001 | | Sufficient only when the changed doc path and self-check cover the stated docs-only condition. |
| AC-02 login form submits email | RUN-FEATURE-001 | ART-DIFF-002, ART-TEST-002 | FBL-001 | Supported when the Run, diff, and test/log refs map to this AC rather than only to the Task in general. |
| AC-03 final button copy is readable in target viewport | RUN-UI-001 | ART-SCREENSHOT-001, ART-DIFF-003 | QA-0001 | If Manual QA is required, screenshot or browser smoke alone does not satisfy the QA path. |
| AC-04 export contains only approved redacted fields | RUN-EXPORT-001 | ART-EXPORT-MANIFEST-001, ART-LOG-001 | APR-0001, DEC-0001 | Approval and Decision refs show scope or user judgment context; redacted artifact refs still need to prove the nonsecret claim. |
| Completion condition: independent verifier reviewed the changed scope | RUN-VERIFY-001 | ART-BUNDLE-001 | EVAL-0001 | Valid only when the Eval reviewed current refs and has the required independence for the requested close. |

Evidence Manifest supports claims; it does not prove correctness by itself, create detached verification, record Manual QA, imply work acceptance, make residual risk visible, or accept residual risk. When a close summary is rendered from this template, it should keep those lines separate so a passing test, a self-check, a QA waiver, or work acceptance is not mistaken for another close condition.

When close context is shown, the manifest should render risk-accepted close, waived verification, QA waiver, self-checked, and `detached_verified` as distinct display states with owner refs or explicit absence. Those labels are readable summaries of owner records, not Evidence Manifest authority.

Coverage rows should point to owner records and ArtifactRef refs rather than embedding large evidence. If no ref supports a criterion, condition, or claim, show it as unsupported, insufficient, stale, or blocked instead of filling the gap with prose.

Chat text and Markdown report prose may explain the evidence story, but they are not enough to prove sufficiency unless they point to compatible owner records and registered ArtifactRef refs.

Large logs, diffs, screenshots, traces, and bundles should stay as registered ArtifactRef refs with short outcomes. The manifest should show redaction state and availability before any reader drills into the artifact body.

`secret_omitted` artifacts may support claims whose nonsecret evidence remains visible, but not claims that require omitted values. `blocked` artifacts are committed metadata-only notices, not available raw evidence; dependent criteria remain unsupported, insufficient, or blocked until a replacement, waiver, Decision Packet outcome, accepted risk, or documented fallback resolves the evidence path. This template must not include omitted secret/PII values or blocked payload bytes.
