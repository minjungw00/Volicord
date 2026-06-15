# API 메서드

이 문서는 공개 하네스 API 메서드 동작의 담당 문서를 찾기 위한 메서드 묶음 경로입니다. 정확한 기계 판독 담당 문서 경로는 [`docs/doc-index.yaml`](../../../doc-index.yaml)을 사용합니다.

이 문서는 메서드 동작, 요청이나 응답 본문, 공통 스키마, 저장 효과, 오류 의미, 보안 보장, Core 권한 의미를 정의하지 않습니다.

<a id="method-owner-routing-table"></a>

## 메서드 담당 문서

<a id="harnessintake"></a>
<a id="harnessupdate_scope"></a>
<a id="harnessstatus"></a>
<a id="harnessprepare_write"></a>
<a id="harnessstage_artifact"></a>
<a id="harnessrecord_run"></a>
<a id="harnessrequest_user_judgment"></a>
<a id="harnessrecord_user_judgment"></a>
<a id="harnessclose_task"></a>

| 메서드 | 담당 문서 |
|---|---|
| `harness.intake` | [접수 메서드 담당 문서](method-intake.md) |
| `harness.update_scope` | [범위 갱신 메서드 담당 문서](method-update-scope.md) |
| `harness.status` | [상태 메서드 담당 문서](method-status.md) |
| `harness.prepare_write` | [쓰기 준비 메서드 담당 문서](method-prepare-write.md) |
| `harness.stage_artifact` | [아티팩트 스테이징 메서드 담당 문서](method-stage-artifact.md) |
| `harness.record_run` | [실행 기록 메서드 담당 문서](method-record-run.md) |
| `harness.request_user_judgment` | [사용자 판단 메서드 담당 문서](method-user-judgment.md#harnessrequest_user_judgment) |
| `harness.record_user_judgment` | [사용자 판단 메서드 담당 문서](method-user-judgment.md#harnessrecord_user_judgment) |
| `harness.close_task` | [Task 닫기 메서드 담당 문서](method-close-task.md) |

## 가까운 경로

- 공통 요청/응답 래퍼와 응답 분기 형태: [API 코어 스키마](schema-core.md).
- 메서드와 독립적인 API 값 집합: [API 값 집합](schema-value-sets.md).
- API 오류 묶음: [API 오류](errors.md).
- 메서드나 분기별 저장 효과: [저장 효과](../storage-effects.md).
- 메서드가 사용하는 제품과 Core 개념: [Core 모델](../core-model.md).
