# DIRECT-RESULT Template

## 사용 시점

작은 direct 작업이 닫혔거나 `work`로 전환된 뒤 결과를 간결하고 부담 없이 보여줘야 할 때 `DIRECT-RESULT`를 사용합니다. 전체 Task gate 보고서가 아니라 direct 결과처럼 읽혀야 합니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: Future/diagnostic projections입니다. 해당 profile이 active일 때 optional compact direct-work result display로 사용하며 v0.2 First User-Value Slice projection이나 first kernel proof에는 필요하지 않습니다.

## 기준 기록

- direct run 기록
- direct 작업에 제품 파일 쓰기가 있었다면 consumed Write Authorization 참조
- 변경 경로
- 범위 밖 또는 유지된 범위 summary
- 실행한 check
- 표시되는 claim이 있을 때 Decision Packet, Approval, Evidence Manifest, Eval, 수동 QA, Acceptance Decision Packet, Residual Risk, Artifact refs
- redaction state와 availability를 포함한 artifact 참조
- 읽기용 보기 최신성(projection freshness) 입력
- escalation flag
- close assurance
- 해당되는 경우 근거, 검증, 수동 QA, 작업 수락, 잔여 위험 표시, 잔여 위험 수용 관련 닫기 영향 요약

닫기 요약 줄(Close Summary line)은 기존 gate와 owner-record ref에서 파생한 표시 전용 요약입니다. Direct 작업은 자신이 요약하는 기록 밖에 별도의 close field를 만들지 않습니다.

## 렌더링 섹션

- Request
- Scope
- Outcome
- Changed Scope
- Checks
- Assurance
- Authority Refs
- 닫기 영향 요약
- Escalation
- Evidence Refs
- Projection Freshness

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
- write authorization ref:
- allowed paths:
- approval refs:

## Outcome
- result summary:
- close reason:

## Changed Scope
- changed files: `path/to/file`
- no-file result:
- 범위 밖 유지:

## Checks
- self-check:
- tests/build:
- validator outcomes:
- artifact refs and redaction state:
- artifact availability:

## Assurance
- assurance_level:
- meaning:
- detached verify needed:
- self-check refs:
- 분리 검증 Eval ref:
- 검증 면제 ref:
- QA waiver ref:
- risk-accepted close refs:

## Authority Refs
- write authorization:
- Decision Packet:
- approval:
- Evidence Manifest:
- Eval:
- 수동 QA:
- Acceptance Decision Packet:
- Residual Risk:
- Artifact refs:
- redaction state:
- projection freshness:

## 닫기 영향 요약
- display state label (plain text, schema value 아님):
- 근거:
- 검증:
- 수동 QA:
- 작업 수락:
- 잔여 위험 표시:
- 잔여 위험 수용:
- 검증 면제 ref:
- QA waiver ref:
- 후속 작업:

## Escalation
- escalated_to_work: yes | no
- reason:

## Evidence Refs
- logs:
- diff:
- 후속 보고서:
- 생략/차단 artifact 영향:

## Projection Freshness
- freshness:
- source_state_version:
- stale 또는 reconcile 영향:
````

## 메모

정책 또는 사용자가 분리 검증 또는 다른 gate를 요구하지 않으면 direct 작업은 기본적으로 자체 확인(self-checked) 상태로 닫힐 수 있습니다. Consumed Write Authorization 참조를 표시할 수 있지만, projection이 기준 authorization 기록이 되는 것은 아닙니다.

Direct Result는 self-checked, `detached_verified`, verification-waived, QA-waived, risk-accepted-close 상태를 별도 줄로 표시해야 합니다. Waiver 줄은 waiver ref를 가리키거나 아직 기록되지 않았다고 말하며, verification 또는 QA가 되지 않습니다. Risk-accepted close는 detached verified처럼 보이지 않게, accepted Residual Risk refs와 필요한 Decision Packet을 가리킵니다.

Direct Result의 checks와 tests는 근거 또는 자체 확인(self-check) 맥락입니다. 조건을 충족하는 Eval 없이는 분리 검증이 되지 않고, 수동 QA 결과 또는 유효한 waiver 없이는 수동 QA가 되지 않으며, 작업 수락을 암시하지도 않습니다. Direct 작업이 잔여 위험 수용으로 닫힌다면 닫기 영향 요약은 결과를 detached verified처럼 보여주는 대신 받아들인 Residual Risk refs, 필요한 경우 잔여 위험 수용을 기록한 Decision Packet, 후속 작업을 가리켜야 합니다. 알려진 close-relevant risk가 없다면 gate 목록을 덧붙이기보다 그 사실을 직접 말합니다.

Direct Result의 authority claim은 source ref 또는 명시적인 absence를 cite해야 합니다. Write permission에는 Write Authorization, 민감 동작 permission에는 Approval, evidence sufficiency에는 Evidence Manifest, 분리 검증에는 Eval, QA에는 수동 QA record 또는 waiver path, 작업 수락에는 Acceptance Decision Packet, 잔여 위험 표시에는 Residual Risk refs 또는 `ResidualRiskSummary.status=none`, 잔여 위험 수용에는 accepted Residual Risk refs를 사용합니다. `not_visible` 잔여 위험을 "none"처럼 렌더링하면 안 됩니다.

`DIRECT-RESULT`는 direct 작업을 위한 가벼운 close impact 표시입니다. `TASK`는 진행 중이거나 최근 닫힌 `work` Task의 이어가기용 Close Summary 표시를 담당하고, Journey Card close context는 간결한 status/resume 표시입니다. 이 표시들은 [projection/report 경계](../document-projection.md#projection-principles)를 따르며, close와 gate effect는 여전히 owner record에서 옵니다.

Direct Result의 ArtifactRef는 `redaction_state`를 보이게 유지해야 합니다. `secret_omitted`는 보이는 nonsecret evidence만 뒷받침하고, `blocked`는 replacement, waiver, Decision Packet outcome, 받아들인 위험, documented fallback으로 해소될 때까지 원본 입력을 사용할 수 없다는 뜻입니다.
