# API 메서드

이 페이지에서 공개 Volicord API 메서드별 참조 담당 문서를 찾을 수 있습니다. 정확한 메서드 계약은 연결된 문서가 정의합니다.

이 문서는 메서드 동작, 요청이나 응답 본문, 공통 스키마, 저장 효과, 오류 의미, 보안 보장, Core 권한 의미를 정의하지 않습니다.

<a id="surface-stability"></a>
## 표면 안정성

아래 라벨은 [문서 정책](../../maintain/documentation-policy.md#surface-stability-labels)의 어휘를 사용합니다.

| 표면 | 안정성 | 비고 |
|---|---|---|
| 아래 표의 지원되는 공개 메서드 이름 | `stable` | 이 이름들이 지원되는 공개 API 메서드 집합을 이룹니다. |
| 연결된 메서드 담당 문서 | `stable` | 각 담당 문서는 자신이 맡은 메서드 동작, 요청과 응답 형태, 효과를 정의합니다. 단, 해당 담당 문서가 더 좁은 내부 표면에 다른 라벨을 붙인 경우는 예외입니다. |

<a id="method-owner-routing-table"></a>

## 메서드 담당 문서

<a id="volicordintake"></a>
<a id="volicordupdate_scope"></a>
<a id="volicordstatus"></a>
<a id="volicordget_operation_result"></a>
<a id="volicordprepare_evidence_capture"></a>
<a id="volicordprepare_write"></a>
<a id="volicordstage_artifact"></a>
<a id="volicordrecord_run"></a>
<a id="volicordrequest_user_judgment"></a>
<a id="volicordrecord_user_judgment"></a>
<a id="volicordrecord_user_observation"></a>
<a id="volicordreconcile_changes"></a>
<a id="volicordcheck_close"></a>
<a id="volicordclose_task"></a>

| 메서드 | 담당 문서 |
|---|---|
| `volicord.intake` | [접수 메서드 담당 문서](method-intake.md) |
| `volicord.update_scope` | [범위 갱신 메서드 담당 문서](method-update-scope.md) |
| `volicord.status` | [상태 메서드 담당 문서](method-status.md) |
| `volicord.get_operation_result` | [작업 결과 조회 메서드 담당 문서](method-get-operation-result.md#volicordget_operation_result) |
| `volicord.prepare_evidence_capture` | [증거 캡처 준비 메서드 담당 문서](method-prepare-evidence-capture.md#volicordprepare_evidence_capture) |
| `volicord.prepare_write` | [쓰기 준비 메서드 담당 문서](method-prepare-write.md) |
| `volicord.stage_artifact` | [아티팩트 스테이징 메서드 담당 문서](method-stage-artifact.md) |
| `volicord.record_run` | [실행 기록 메서드 담당 문서](method-record-run.md) |
| `volicord.request_user_judgment` | [사용자 소유 판단 요청 메서드 담당 문서](method-request-user-judgment.md#volicordrequest_user_judgment) |
| `volicord.record_user_judgment` | [사용자 소유 판단 기록 메서드 담당 문서](method-record-user-judgment.md#volicordrecord_user_judgment) |
| `volicord.record_user_observation` | [사용자 Evidence 관찰 기록 메서드 담당 문서](method-record-user-observation.md#volicordrecord_user_observation) |
| `volicord.reconcile_changes` | [변경 조정 메서드 담당 문서](method-reconcile-changes.md#volicordreconcile_changes) |
| `volicord.check_close` | [닫기 메서드 담당 문서](method-close-task.md#volicordcheck_close) |
| `volicord.close_task` | [닫기 메서드 담당 문서](method-close-task.md#volicordclose_task) |

## 가까운 경로

- 공통 요청/응답 래퍼와 응답 분기 형태: [API 코어 스키마](schema-core.md).
- 메서드와 독립적인 API 값 집합: [API 값 집합](schema-value-sets.md).
- API 오류 묶음: [API 오류](errors.md).
- 메서드나 분기별 저장 효과: [저장 효과](../storage-effects.md).
- 메서드가 사용하는 제품과 Core 개념: [Core 모델](../core-model.md).
