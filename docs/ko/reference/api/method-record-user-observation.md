<a id="volicordrecord_user_observation"></a>

# `volicord.record_user_observation` 참조

## 이 문서가 담당하는 것

이 문서는 User Channel 전용 전이인
`volicord.record_user_observation`의 공개 계약, 즉 요청과 결과, 권한 검사,
freshness 규칙, 메서드별 효과를 담당합니다.

이 문서는 `ArtifactRef`, `EvidenceTarget`, 공통 응답 분기, 저장소 DDL,
닫기 준비 정책을 다시 정의하지 않습니다.

## 목적

`volicord.record_user_observation`은 이미 영속화된 정확한 아티팩트 바이트가
특정 Evidence 대상과 관련 있다는 사용자의 평가를 기록합니다. 이 메서드는
`UserEvidenceObservation`을 만들며, `UserJudgment`를 만들거나 해결하지 않고
최종 수락을 부여하거나 Run을 기록하지도 않습니다.

이 메서드는 직접 User Channel 표면입니다. 로컬 CLI의
`volicord inbox observe`로 사용할 수 있으며 Agent Connection MCP 도구로는
노출되지 않습니다.

## 요청 스키마

```yaml
RecordUserObservationRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string
  target: EvidenceTarget
  relevance_status: supported | contradicted
  artifact_ids: string[]
  summary: string
  observed_at: string
```

모든 멤버는 필수이며 `artifact_ids`는 비어 있으면 안 됩니다. Core는 각 ID를
현재의 정규 `ArtifactRef`로 해석합니다. 이 메서드는 호출자가 제출하는 해시,
크기, producer 라벨을 받지 않습니다.

## 접근 및 검증

검증된 호출은 `actor_source=local_user`, `operation_category=user_only`를
사용해야 합니다. 커밋 요청에는 null이 아닌 `idempotency_key`와 현재
`expected_state_version`도 필요합니다.

Core는 다음을 요구합니다.

- 요청이 지칭하는 현재 Task와 활성 Change Unit
- 현재 Task baseline
- 현재 acceptance criterion 또는 같은 Task의 기존 supplemental claim
- 저장 본문 바이트가 여전히 사용 가능하고 무결성 검증된 같은 Task의 영속
  아티팩트 하나 이상
- 비어 있지 않은 요약과 미래가 아닌 `observed_at`

Core는 현재 `scope_revision`, baseline, 정확한 정규 아티팩트 ref, 검증된
로컬 사용자 actor, 실제 User Channel 검증 근거를 기록합니다. 이 현재 좌표가
바뀌면 해당 레코드는 새 근거에서 Strong observation으로 사용할 수 없습니다.

## 성공 결과

```yaml
RecordUserObservationResult:
  base: ToolResultBase
  user_observation_ref: StateRecordRef
  user_observation: UserEvidenceObservation
```

커밋 결과는 `project_state.state_version`을 한 번 증가시키고,
`user_evidence_observations` 행 하나와
`user_evidence_observation_recorded` 이벤트, 일반 커밋 replay 행을 기록합니다.
이 메서드만으로 Evidence coverage를 갱신하지는 않습니다. 이후
`volicord.record_run`이 `user_observation_ref`, 같은 대상, 정확히 같은 정규
아티팩트 출력을 참조해야 Core가 `user_observation` / `user_observed`
provenance를 파생합니다.

## Dry run과 거부

Dry run은 같은 요청 및 권한 검사를 수행하지만 레코드, 이벤트, replay 행,
상태 버전 변경을 만들지 않습니다. stale 상태, 사용자가 아닌 호출, 누락되거나
변경된 아티팩트 바이트, stale Task 좌표, 알 수 없는 대상, 잘못된 relevance
입력은 효과 없이 거부됩니다.

## 권한 경계

- 이 레코드는 자신이 지칭하는 정확한 저장 바이트와 근거에 대해서만 사용자
  관찰 및 대상 relevance를 설정합니다.
- 외부 도구가 그 바이트를 만들었다는 사실은 증명하지 않습니다.
- 이는 Evidence provenance이며 사용자 소유 판단, 승인, 최종 수락,
  잔여 위험 수락, 정확성 증명이 아닙니다.
- `record_run`과 닫기 검사는 producer 레코드를 다시 읽고 바이트 무결성 및
  정확한 출력 identity를 재검증하며 `relevance_status=supported`를
  요구합니다. 누락, 모순, stale, 손상, 불일치 레코드는 weak입니다.

## 관련 담당 문서

- Evidence 파생: [`volicord.record_run`](method-record-run.md).
- Evidence 형태: [API 상태 스키마](schema-state.md#evidence-and-run-snapshot-shapes).
- 값 집합: [API 값 집합](schema-value-sets.md#evidence-observation-values).
- 정확한 효과: [저장 효과](../storage-effects.md#volicordrecord_user_observation).
- 저장 레코드: [저장 레코드](../storage-records.md).
