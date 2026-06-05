# 실행/근거 요약 템플릿

## 사용 시점

조언, 실행, 확인, 변경 뒤 무엇이 일어났고 현재 주장에 어떤 근거가 생겼는지 최소한으로 보여줘야 할 때 `run-evidence-summary`를 사용합니다.

구현 계층: MVP-1 사용자 작업 루프 보기입니다. 상세 [RUN-SUMMARY](later-profile/run-summary.md)와 [EVIDENCE-MANIFEST](later-profile/evidence-manifest.md) 보고서는 later/full-profile 템플릿입니다.

경계: 이 템플릿은 Run과 근거 참조를 표시할 뿐입니다. 근거 자체, 전체 Evidence Manifest, 검증, 수동 QA, 작업 수락, 닫기 준비 상태 권한이 아닙니다.

## 기준 기록

- Run 참조와 command/check 요약
- 변경 경로 또는 파일 변경 없음 결과
- 관련 있을 때 소비된 Write Authorization 참조, no-write basis, 또는 attempted invalid authorization context
- ArtifactRefs, `evidence_ref` 참조, `redaction_state`, 무결성 또는 availability 메모
- 근거가 뒷받침하는 수용 기준, 완료 주장, 닫기 관련 주장
- 근거 공백, 오래된 입력, 아직 해소되지 않은 뒷받침 부족
- 다음 근거 행동

## 렌더링 섹션

- 실행 또는 행동
- 변경 경로
- 확인
- 근거 참조
- 뒷받침하는 주장
- 공백 또는 오래된 근거
- 가림 처리와 availability
- 다음 근거 행동

## 전체 템플릿

````text
실행/근거 요약
표시 전용: ref와 요약일 뿐이며 근거, 검증, QA, 작업 수락, 닫기가 아닙니다.

행동: {run_or_action_summary}
변경 경로: {changed_paths|none}
확인: {checks_run_or_reason_not_run}
쓰기 권한: {consumed_write_authorization_ref|no_product_write|attempted_invalid_ref_only|none}
근거 요약: status={evidence_summary.status}; summary={evidence_summary.summary}
근거 참조: {evidence_refs|none}
아티팩트 참조: {artifact_refs|none}; redaction={redaction_summary|none}
뒷받침하는 것: {supported_claims_or_criteria|none}
아직 빠졌거나 오래된 것: {evidence_gaps_or_stale_inputs|none}
다음 근거 행동: {next_evidence_action|none}
출처/최신성: state={source_state_version}; refs={source_refs}; rendered={updated_at}; freshness={freshness_state}
````

## 메모

근거 충분성은 양이 아니라 coverage입니다. 현재 뒷받침하는 참조가 없는 주장은 공백과 `evidence_summary.status`로 보여줘야 하며, 긴 artifact 목록이나 report 문장을 증명처럼 취급하면 안 됩니다.

Product-write Run의 쓰기 권한으로 표시할 수 있는 것은 compatible하게 소비된 Write Authorization뿐입니다. Attempted invalid authorization ref는 violation/audit 또는 validator-finding context로만 보여줄 수 있으며, consumed authority나 completion evidence처럼 렌더링하면 안 됩니다.
