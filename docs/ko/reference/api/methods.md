# API 메서드

이 문서는 공개 Volicord API 메서드 동작의 담당 문서를 찾기 위한 읽기용 메서드 경로 안내입니다. 기계가 읽는 정확한 담당 경로는 [`docs/doc-index.yaml`](../../../doc-index.yaml)을 사용합니다.

이 문서는 메서드 동작, 요청이나 응답 본문, 공통 스키마, 저장 효과, 오류 의미, 보안 보장, Core 권한 의미를 정의하지 않습니다.

<a id="method-owner-routing-table"></a>

## 메서드 담당 문서

<a id="volicordintake"></a>
<a id="volicordupdate_scope"></a>
<a id="volicordstatus"></a>
<a id="volicordprepare_write"></a>
<a id="volicordstage_artifact"></a>
<a id="volicordrecord_run"></a>
<a id="volicordrequest_user_judgment"></a>
<a id="volicordrecord_user_judgment"></a>
<a id="volicordclose_task"></a>

| 메서드 | 담당 문서 |
|---|---|
| `volicord.intake` | [접수 메서드 담당 문서](method-intake.md) |
| `volicord.update_scope` | [범위 갱신 메서드 담당 문서](method-update-scope.md) |
| `volicord.status` | [상태 메서드 담당 문서](method-status.md) |
| `volicord.prepare_write` | [쓰기 준비 메서드 담당 문서](method-prepare-write.md) |
| `volicord.stage_artifact` | [아티팩트 스테이징 메서드 담당 문서](method-stage-artifact.md) |
| `volicord.record_run` | [실행 기록 메서드 담당 문서](method-record-run.md) |
| `volicord.request_user_judgment` | [사용자 소유 판단 요청 메서드 담당 문서](method-request-user-judgment.md#volicordrequest_user_judgment) |
| `volicord.record_user_judgment` | [사용자 소유 판단 기록 메서드 담당 문서](method-record-user-judgment.md#volicordrecord_user_judgment) |
| `volicord.close_task` | [Task 닫기 메서드 담당 문서](method-close-task.md) |

## 가까운 경로

- 공통 요청/응답 래퍼와 응답 분기 형태: [API 코어 스키마](schema-core.md).
- 메서드와 독립적인 API 값 집합: [API 값 집합](schema-value-sets.md).
- API 오류 묶음: [API 오류](errors.md).
- 메서드나 분기별 저장 효과: [저장 효과](../storage-effects.md).
- 메서드가 사용하는 제품과 Core 개념: [Core 모델](../core-model.md).
