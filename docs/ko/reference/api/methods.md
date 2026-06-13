# API 메서드

이 참조 문서는 지원되는 공개 API 메서드 목록과 각 메서드의 동작 담당 문서 경로를 담당합니다. 메서드 동작, 공통 스키마 본문, 저장 효과, 공개 오류 의미, 예시 정합성 규칙은 정의하지 않습니다.

<a id="baseline-scope-method-behavior"></a>

## 지원 메서드 경계

이 문서에 나열된 메서드만 지원되는 공개 API 메서드입니다. 여기에 없는 메서드 이름은 지원되는 공개 메서드 묶음 밖에 있습니다.

메서드별 동작은 각 메서드 담당 문서가 담당합니다. 범위 밖 API나 스키마 기능은 [범위](../scope.md)와 영향받는 담당 문서가 지원 동작으로 정의하지 않는 한 이 메서드 경로에 포함되지 않습니다.

<a id="method-owner-routing-table"></a>

## 지원되는 API 메서드 목록

아래 표는 지원되는 공개 메서드 목록이자 메서드 동작 질문의 첫 담당 경로입니다.

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

요청과 응답 동작은 표에 연결된 메서드 담당 문서를 사용합니다. `harness.close_task`의 차단 사유 생성 분기는 Task 닫기 메서드 담당 문서에 남기고, [API 차단 사유 처리 경로](blocker-routing.md)는 닫기 차단 사유와 API 응답 사이의 처리 경로 의미를 확인할 때만 사용합니다.
