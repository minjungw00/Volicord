# EVAL Template

## Used when

Use `EVAL` when Harness needs a readable verification result with independence context.

This is template reference documentation. It does not authorize runtime/server implementation, generated operational files, executable fixtures, or runtime data before the redesigned docs are accepted. The first implementation/proof target remains Kernel Smoke; Agency-Hardened MVP and post-MVP automation stay out of scope unless their owner docs promote and prove them.

## Source records

- Eval record
- verification target
- verdict
- independence qualifier
- self-check versus detached verification boundary
- baseline relationship and evaluator-bundle freshness
- checks performed
- evidence reviewed
- blockers
- artifact refs with redaction state and input availability

## Rendered sections

- Target
- Verdict
- Environment And Independence
- Checks And Validator Outcomes
- Evidence Reviewed
- Acceptance Criteria Review
- Design Quality Review
- Rationale
- Blockers Or Rework
- Redaction And Availability
- User Follow-Up

## Full template

````md
---
doc_type: eval
eval_id: EVAL-0001
task_id: TASK-0001
change_unit_id: CU-01
verdict: passed
surface_id: reference
source_state_version: 45
updated_at: 2026-05-06T10:05:00+09:00
---

# EVAL-0001 Verification Result

> Projection view: rendered from `source_state_version` at `updated_at`; displays Eval state and reviewed refs. Verdict, assurance, and gate effects change only through Eval and Core gate records.

## Target
- task_id:
- change_unit_id: CU-01 | null
- target_run_id:
- evaluator_run_id:

## Verdict
- verdict: passed | failed | blocked | inconclusive
- assurance impact:
- verification gate impact:
- detached candidate status:
- self-check vs detached boundary:
- Manual QA impact:
- acceptance impact:
- next action:

## Environment And Independence
- fresh run:
- evaluator surface:
- context independence: same_session | subagent_context | fresh_session | fresh_worktree | sandbox | manual_bundle
- same-session self-review guard:
- write capable:
- product file write allowed:
- baseline verified:
- bundle freshness:
- repo drift observed:
- source input: chat_history | task_summary | bundle | allowed_raw_artifacts | refs_with_redaction_notes
- source bundle:
- parent run:

## Checks And Validator Outcomes
### Core Checks And Preconditions
- [ ] changed_paths
- [ ] approval_scope
- [ ] same_session_verify_guard
- [ ] evidence_sufficiency
- [ ] bundle_integrity
- [ ] acceptance_review
- [ ] baseline_freshness
- [ ] public_interface_change_review
- [ ] lint
- [ ] test
- [ ] build

### ValidatorResult IDs
- [ ] vertical_slice_shape
- [ ] shared_design_alignment
- [ ] decision_quality_check
- [ ] autonomy_boundary_check
- [ ] feedback_loop_check
- [ ] tdd_trace_required
- [ ] domain_language_consistency
- [ ] module_interface_review
- [ ] codebase_stewardship_check
- [ ] residual_risk_visibility_check
- [ ] manual_qa_required
- [ ] surface_capability_check

## Evidence Reviewed
- task summary:
- Journey Spine:
- Decision Packets:
- Residual Risks:
- Autonomy Boundary:
- domain term refs:
- module map item refs:
- interface contract refs:
- run summary:
- feedback loop:
- TDD trace:
- Manual QA:
- evidence manifest:
- diff:
- bundle:
- logs:
- approvals:
- decisions:

## Redaction And Availability
| Artifact Ref | Redaction State | Verification Effect | Note |
|---|---|---|---|
| ART-EVAL-0001 | secret_omitted | visible nonsecret facts reviewed; omitted value not proven | |
| ART-EVAL-0002 | blocked | unavailable input; verdict cannot depend on raw payload | |

## Acceptance Criteria Review
| AC ID | Statement | Evidence Reviewed | Result | Notes |
|---|---|---|---|---|

## Design Quality Review
- vertical slice:
- Decision Packets:
- Autonomy Boundary:
- Residual Risks:
- feedback loop:
- TDD trace:
- module/interface:
- architecture drift:
- domain language consistency:

## Rationale
-

## Blockers Or Rework
-

## User Follow-Up
- trade-off needing confirmation:
- remaining options:
- Manual QA need:
````

## Notes

An Eval verdict alone does not upgrade assurance. `detached_verified` requires a passed verification with valid independence, current baseline and bundle inputs, and no same-session self-review violation.

If independence is invalid or the review is same-session self-check only, render that boundary explicitly and leave detached assurance unchanged. A `subagent_context` review is not detached by default; render it as a detached candidate only when the recorded context satisfies `fresh_session`, `fresh_worktree`, `sandbox`, or `manual_bundle` requirements.

If the evaluator bundle, baseline, included artifacts, Evidence Manifest, approval/Decision Packet refs, or close-relevant Residual Risk refs are stale, render the stale input and keep assurance unchanged until replacement or compatible re-verification is recorded.

Eval projections must not imply omitted or blocked raw bytes were reviewed. `secret_omitted` evidence can support only visible nonsecret claims. If the Eval depends on a `blocked` payload, the result must remain `blocked` or `inconclusive`, or surface `EVIDENCE_INSUFFICIENT`, until a replacement, waiver, Decision Packet outcome, accepted risk, or documented fallback resolves the verification path.
