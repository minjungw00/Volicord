# DIRECT-RESULT Template

## 사용 시점

작은 direct work가 닫혔거나 `work`로 전환된 뒤 결과를 간단히 보여줘야 할 때 `DIRECT-RESULT`를 사용합니다.

## 기준 기록

- direct run 기록
- direct product write에 있는 경우 consumed Write Authorization 참조
- changed path
- performed check
- artifact 참조
- escalation flag
- close assurance
- 해당되는 경우 근거, 검증, Manual QA, 수용, Residual Risk 관련 close 영향 요약

## 렌더링 섹션

- Request
- Scope
- Changed Files
- Checks And Validator Outcomes
- Outcome
- Assurance
- close 영향 요약
- Escalation
- Evidence Refs

## 전체 템플릿

````md
---
doc_type: direct_result
task_id: TASK-0001
run_id: RUN-20260506-093015-LEAD-01
result: passed
assurance_level: self_checked
surface_id: reference
source_state_version: 41
updated_at: 2026-05-06T09:40:00+09:00
---

# DIRECT-RESULT

> Projection 보기: `source_state_version`와 `updated_at` 기준으로 렌더링되며 direct Run result를 표시합니다. 이 문서를 편집해도 result, assurance, escalation, close state는 바뀌지 않습니다.

## Request
- user request:

## Scope
- direct run scope:
- limits:
- write authorization:
- allowed paths:
- allowed tools:
- allowed commands:
- approval refs:

## Changed Files
- `path/to/file`

## Checks And Validator Outcomes
### Core Checks And Command Checks
- changed_paths:
- approval_scope:
- test:
- build:

### ValidatorResult IDs
- context_hygiene_check:
- surface_capability_check:

## Outcome
- result summary:

## Assurance
- assurance_level:
- meaning:
- detached verify needed:

## close 영향 요약
- 근거:
- 검증:
- Manual QA:
- 수용:
- Residual Risk:
- 후속 작업:

## Escalation
- escalated_to_work: yes | no
- reason:

## Evidence Refs
- logs:
- diff:
- 후속 보고서:
- 생략/차단 artifact 영향:
````

## 메모

정책 또는 사용자가 detached verification 또는 다른 gate를 요구하지 않으면 direct work는 기본적으로 self-checked 상태로 close될 수 있습니다. Consumed Write Authorization 참조를 표시할 수 있지만, projection이 기준 authorization 기록이 되는 것은 아닙니다.

Direct Result의 checks와 tests는 evidence 또는 self-check 맥락입니다. 조건을 충족하는 Eval 없이는 detached verification이 되지 않고, Manual QA 결과 또는 유효한 waiver 없이는 Manual QA가 되지 않으며, 최종 수용을 암시하지도 않습니다. Direct work가 위험 수용으로 close된다면 close 영향 요약은 결과를 detached verified처럼 보여주는 대신 accepted Residual Risk refs, 필요한 경우 risk acceptance를 기록한 Decision Packet, 후속 작업을 가리켜야 합니다.

Direct Result의 ArtifactRef는 `redaction_state`를 보이게 유지해야 합니다. `secret_omitted`는 visible nonsecret evidence만 뒷받침하고, `blocked`는 replacement, waiver, Decision Packet outcome, accepted risk, documented fallback으로 해소될 때까지 raw input이 unavailable하다는 뜻입니다.
