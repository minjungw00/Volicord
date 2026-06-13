# 저장소 버전 관리

이 문서는 기준 범위 저장소 원천 설계의 `state_version`, 멱등성, 이벤트 의미, 잠금, 마이그레이션 의미를 담당합니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- 공개 프로젝트 전체 `project_state.state_version` 충돌 기준.
- 저장소 의미 수준의 `state_version` 증가 규칙.
- 멱등성과 `request_hash` 재실행 의미.
- `task_events`의 이벤트 의미.
- 잠금 정책.
- 마이그레이션 의미와 기준 범위/지원 범위 밖 마이그레이션 경계.
- 실패 뒤 재시도할 때 상태 버전과 멱등성 식별자를 어떻게 해석하는지.

이 문서는 담당하지 않습니다.

- 기록 배치나 DDL; [저장소 기록](storage-records.md)을 봅니다.
- 어떤 메서드 분기가 효과를 만드는지; [저장 효과](storage-effects.md), [API 메서드](api/methods.md), 메서드 담당 문서를 봅니다.
- 공개 오류 코드와 우선순위; [API 오류](api/errors.md)를 봅니다.
- 아티팩트 생명주기; [아티팩트 저장소](storage-artifacts.md)를 봅니다.
- 보안 보장 표현; [보안](security.md)을 봅니다.
- 런타임 배포나 운영 명령.

## 상태 버전의 의미

의미:

- 기준 범위에는 공개 상태 시계가 하나만 있습니다. 바로 `project_state.state_version`입니다.
- `project_state.state_version`은 프로젝트 전체에 적용되며, 기준 범위의 공개 API 상태 변경에서 권한 부여, 충돌, 최신성, 동시성 판단에 쓰는 유일한 기준입니다.
- `Task` 라우팅은 담당 `Task`, 차단 사유, 닫기 상태, 증거, 사용자 판단을 찾는 데 중요합니다.
- `Task` 라우팅은 별도 `Task`별 상태 시계를 고르지 않습니다.
- 커밋된 상태 변경 응답은 커밋 뒤 결과 프로젝트 전체 버전을 보고합니다.
- 읽기 전용 결과, `ToolDryRunResponse` 미리보기, 임시 스테이징 응답은 그 응답이 관찰한 현재 프로젝트 전체 버전을 보고합니다.

증가하는 경우:

- `dry_run=false` 상태 변경 호출이 담당 문서가 허용한 분기로 커밋될 때 증가합니다.

증가하지 않는 경우:

- 응답이 상태를 관찰만 하거나, dry-run 효과를 미리 보여 주거나, 임시 데이터를 스테이징하거나, 커밋 전에 거절되면 증가하지 않습니다.

재시도 동작:

- 오래된 쓰기는 커밋 전에 `ToolEnvelope.expected_state_version`을 현재 `project_state.state_version`과 비교합니다.
- 커밋된 상태 변경 요청의 전송 불확실성은 멱등 재실행으로 처리하며, 상태 버전 증가를 하나 더 만들지 않습니다.

담당 문서 링크:

- 분기별 지속 효과는 [저장 효과](storage-effects.md)와 [API 메서드](api/methods.md)가 안내하는 메서드 담당 문서가 담당합니다.

아래 요약 표는 분기별 결과만 보여 줍니다. 세부 블록은 조건, 결과, 예외를 분리합니다.

| 상황 | 결과 | 세부사항 |
|---|---|---|
| 읽기 전용 상태 조회 | 증가하지 않음 | [읽기 전용 상태 조회](#state-version-read-only-status) |
| 거부 응답 | 증가하지 않음 | [거부 응답](#state-version-rejected-response) |
| 성공한 상태 변경 | 증가함 | [성공한 상태 변경](#state-version-successful-mutation) |
| 커밋된 차단 결과 | 메서드별 | [커밋된 차단 결과](#state-version-committed-blocked-result) |

<a id="state-version-read-only-status"></a>
**읽기 전용 상태 조회**

의미:

- `harness.status`처럼 현재 상태를 관찰하는 읽기 전용 호출입니다.

증가하는 경우:

- 해당 없음. 읽기 전용 호출 자체로는 증가하지 않습니다.

증가하지 않는 경우:

- 호출이 현재 상태를 관찰만 할 때 증가하지 않습니다.
- 호출은 현재 기록을 만들거나 바꾸면 안 되고, 이벤트를 추가하거나 재실행 행을 만들면 안 됩니다.

재시도 동작:

- 반복된 읽기 전용 호출은 그 시점의 현재 프로젝트 전체 버전을 관찰합니다. 멱등 재실행이 아닙니다.

담당 문서 링크:

- 메서드별 효과 없음 분기 세부사항은 [저장 효과](storage-effects.md)가 담당합니다.

<a id="state-version-rejected-response"></a>
**거부 응답**

의미:

- `ToolRejectedResponse`가 커밋 전에 반환됩니다.

증가하는 경우:

- 해당 없음. 커밋 전 거절 자체로는 증가하지 않습니다.

증가하지 않는 경우:

- 요청된 상태 변경이 수행되지 않습니다.
- `project_state.state_version`이 증가하지 않습니다.

재시도 동작:

- 재시도는 거절 사유를 따릅니다. 오래된 상태 버전 충돌이면 상태를 새로 읽고, 검증 실패면 입력을 고치며, 판단이나 권한 부여가 여전히 필요하면 담당 경로를 다시 사용합니다.

담당 문서 링크:

- 공개 오류 코드 경로는 [API 오류](api/errors.md)가 담당합니다.
- 분기별 저장 효과는 [저장 효과](storage-effects.md)가 담당합니다.

<a id="state-version-successful-mutation"></a>
**성공한 상태 변경**

의미:

- `dry_run=false` 상태 변경이 커밋됩니다.

증가하는 경우:

- 프로젝트 전체 상태가 바뀝니다.
- `project_state.state_version`은 커밋당 정확히 한 번 증가합니다.

증가하지 않는 경우:

- 요청이 미리보기, 거절, 재실행, 읽기 전용 결과, 그 밖의 효과 없음 분기에만 도달하면 증가하지 않습니다.

재시도 동작:

- 같은 커밋 응답을 멱등 재실행으로 다시 받으면 원래 응답을 반환하며 상태 변경을 반복하지 않습니다.

담당 문서 링크:

- 메서드별 저장 효과는 [저장 효과](storage-effects.md)와 메서드 담당 문서가 담당합니다.

<a id="state-version-committed-blocked-result"></a>
**커밋된 차단 결과**

의미:

- 메서드 담당 문서가 차단 결과 커밋을 허용합니다.
- 차단 결과가 상태 효과를 가질 수 있는지는 메서드 담당 문서와 [커밋된 차단 결과의 저장 효과](storage-effects.md#committed-blocked-result)가 정합니다.

증가하는 경우:

- 메서드 담당 문서가 차단 사유나 다른 현재 행 변경 저장을 허용하고, [저장 효과](storage-effects.md)가 그 분기의 `state_version` 효과를 허용할 때만 증가합니다.

증가하지 않는 경우:

- 차단 결과에 담당 문서가 정의한 상태 효과가 없으면 증가하지 않습니다.
- 차단 결과라는 사실만으로 `project_state.state_version`이 자동 증가하지 않습니다.

재시도 동작:

- 차단 결과를 만든 분기의 메서드 담당 문서와 실패/재시도 규칙을 따릅니다.

담당 문서 링크:

- 차단 결과 저장 효과는 [저장 효과](storage-effects.md#committed-blocked-result)와 그 메서드 담당 문서가 담당합니다.

기준 범위 첫 스키마에서는 `tasks.state_version`을 생략해야 합니다. 구현이 레거시 또는 프로토타입 `tasks.state_version` 열을 만나더라도 그 값은 무시되는 메타데이터일 뿐입니다.

`tasks.state_version`은 아래 기준으로 쓰면 안 됩니다.

- 승인.
- `STATE_VERSION_CONFLICT`.
- 오래된 상태 판단.
- `Write Authorization`.
- 멱등성.
- 잠금.
- 동시성.

관련 저장 필드는 프로젝트 전체 시계를 기록합니다.

- `write_authorizations.basis_state_version`은 Core가 권한을 준비할 때 사용한 `project_state.state_version`입니다.
- `tool_invocations.basis_state_version`은 호출이 커밋 전 관찰한 프로젝트 전체 상태 버전입니다.
- `task_events.state_version`은 커밋된 이벤트 뒤의 결과 프로젝트 전체 버전입니다.

## 증가하는 경우

의미:

- 증가는 커밋된 프로젝트 전체 상태 변경 하나를 뜻합니다.
- 하나의 공개 호출이 `Task` 생명주기 필드와 프로젝트 수준 필드를 함께 바꾸더라도 하나의 상태 변경이면 증가는 한 번뿐입니다.

증가하는 경우:

- 새 `dry_run=false` 호출이 실제 상태 변경을 커밋합니다.
- `project_state.state_version`은 정확히 1 증가합니다.
- 예: `harness.close_task intent=supersede`가 `tasks.lifecycle_phase`와 `project_state.active_task_id`를 같은 커밋에서 바꾸면 프로젝트 전체 버전은 한 번 증가합니다.

증가하지 않는 경우:

- 커밋된 차단 결과에 담당 문서가 정의한 상태 효과가 없으면 증가하지 않습니다.
- 커밋된 차단 결과라는 사실만으로 `project_state.state_version`이 자동 증가하지 않습니다.

재시도 동작:

- 이미 커밋된 응답의 재실행은 증가를 하나 더 만들지 않습니다.

담당 문서 링크:

- 메서드별 지속 효과는 [저장 효과](storage-effects.md)와 [API 메서드](api/methods.md)가 안내하는 메서드 담당 문서가 담당합니다.

## 증가하지 않는 경우

의미:

- 효과 없음 분기는 관찰한 `state_version`을 보고할 수 있지만 새 버전을 만들지 않습니다.

증가하는 경우:

- 해당 없음. 이 절에 나열한 분기는 증가하지 않습니다.

증가하지 않는 경우:

- `harness.status`.
- `harness.close_task intent=check`.
- `harness.close_task intent=check`의 `dry_run=true`.
- `ToolDryRunResponse` 미리보기 호출.
- 잘못된 요청.
- 커밋 전 검증 실패.
- 커밋 전 상태 버전 충돌.
- 오래된 `WriteAuthorization.basis_state_version`.
- 멱등 재실행.
- 효과가 없는 거부 응답.

이 분기들은 아래 항목을 만들면 안 됩니다.

- 현재 기록.
- `task_events`.
- 재실행 행.
- 아티팩트 승격.
- 증거 요약.
- `Write Authorization` 생성 또는 소비.
- `close_state` 변경.
- `project_state.state_version` 증가.

재시도 동작:

- 이미 커밋된 원래 응답을 멱등 재실행으로 반환할 수는 있습니다.
- 이때도 새 상태 변경, 새 이벤트, 새 `state_version` 증가는 없습니다.

담당 문서 링크:

- 효과가 없는 분기의 세부 목록과 메서드별 예외는 [저장 효과](storage-effects.md)가 담당합니다.

## `expected_state_version`

의미:

- `expected_state_version`은 오래된 상태에 대한 쓰기를 막는 최신성 조건입니다.
- 새 `dry_run=false` 상태 변경 API 호출은 커밋 전에 `ToolEnvelope.expected_state_version`을 현재 `project_state.state_version`과 비교합니다.
- `expected_state_version`은 사용자 소유 판단, 민감 동작 승인, 최종 수락, 잔여 위험 수락, `Write Authorization`을 대신하지 않습니다.

증가하는 경우:

- 값이 맞고 다른 검증을 통과한 뒤 호출이 담당 문서가 허용한 상태 변경으로 커밋될 때 증가합니다.

증가하지 않는 경우:

- 값이 맞지 않으면 증가하지 않습니다.
- Core는 `STATE_VERSION_CONFLICT`를 `ToolRejectedResponse.errors`에만 담아 반환합니다.

오래된 상태 충돌은 아래 항목을 만들거나 바꾸지 않습니다.

- `CloseReadinessBlocker`.
- 현재 기록.
- `task_event` 또는 `task_events` 추가.
- 아티팩트.
- 증거 요약.
- `Write Authorization` 생성 또는 소비.
- `close_state` 변경.
- 재실행 행.
- `project_state.state_version` 증가.

재시도 동작:

- 현재 상태를 다시 읽습니다.
- 최신 `project_state.state_version`으로 새 요청을 보냅니다.

공개 API 경계:

- 프로젝트 전체 상태 버전 불일치에 쓰는 기준 범위의 유일한 공개 `ErrorCode`는 `STATE_VERSION_CONFLICT`입니다.
- 기준 범위의 공개 호출은 둘 이상의 공개 `expected_state_version`을 요구하거나 받지 않습니다.
- 이 불일치를 공개 API로 드러낼 때도 `STATE_VERSION_CONFLICT`를 사용합니다.

관련 저장 필드:

- 오래된 `Write Authorization`인지 판단할 때는 `write_authorizations.basis_state_version`을 현재 `project_state.state_version`과 비교합니다.

담당 문서 링크:

- 공개 오류 코드 경로는 [API 오류](api/errors.md)가 담당합니다.

허용되지 않는 것:

- 호출은 소비 전에 거절되어야 합니다.
- 다른 현재 계약이 명시적으로 말하지 않는 한 `Write Authorization` 상태도 바꾸면 안 됩니다.

## 이벤트 의미

`task_events`는 커밋된 Core 상태 변경을 순서대로 기록합니다. 감사와 순서 기록이지 일반 운영에서 현재 상태를 재구성하는 기본 출처가 아닙니다. 현재 행은 계속 현재 상태로 남습니다.

- `tasks`
- `change_units`
- `user_judgments`
- `write_authorizations`
- `runs`
- `artifacts`
- `artifact_links`
- `evidence_summaries`
- `blockers`

일반적인 기준 범위 운영에서 `task_events`는 추가 전용입니다. 이벤트가 커밋된 뒤에는 Core가 그 행을 갱신하거나 삭제해 기록을 바꾸면 안 됩니다. 수정이나 복구는 담당 경로를 통한 새 이벤트와 현재 행 갱신으로 기록합니다.

이벤트를 추가하지 않는 분기는 아래와 같습니다.

- 멱등 재실행.
- `dry_run`.
- 잘못된 요청.
- 커밋 전 실패.
- 효과가 없는 거부 응답.

새 커밋된 `dry_run=false` 상태 변경에서는 아래 효과가 하나의 트랜잭션으로 커밋되어야 합니다.

- 현재 행 쓰기
- `task_events` 추가
- 프로젝트 전체 `state_version` 증가
- `tool_invocations` 재실행 행 삽입

스테이징 핸들 소비, 아티팩트 승격, 아티팩트 연결 같은 아티팩트 생명주기 효과는 [아티팩트 저장소](storage-artifacts.md), [저장 효과](storage-effects.md), 메서드 담당 문서가 허용할 때만 같은 커밋 트랜잭션에 들어갑니다.

트랜잭션의 어느 부분이라도 실패하면 아래 부분 결과가 남으면 안 됩니다.

- 권한 행.
- 스테이징 소비.
- 지속 아티팩트 승격 또는 연결.
- `Write Authorization` 소비.
- 증거 업데이트.
- 이벤트.
- 닫기 효과.
- 재실행 행.
- 상태 버전 증가.

## 멱등성과 재실행

의미:

- `tool_invocations`는 API 메서드별 상태 효과 행이 재실행 행 생성을 허용한, 커밋된 `dry_run=false` Core `MethodResult` 응답의 정확한 재실행만 저장합니다.
- 저장소 고유 키는 `(project_id, tool_name, idempotency_key)`입니다.
- `request_hash`는 그 행에 저장하는 충돌 판별자입니다.
- `tool_invocations.response_json`은 재실행 행을 만드는 상태 효과가 있는 커밋된 `dry_run=false` Core `MethodResult` 응답만 정확히 저장합니다.

증가하는 경우:

- 원래 커밋된 상태 변경 요청만 `state_version` 증가를 만들 수 있습니다.
- 재실행 행은 그 원래 커밋 응답과 함께 저장됩니다.

증가하지 않는 경우:

- 같은 `idempotency_key`와 같은 `request_hash`가 재실행되면 증가하지 않습니다.
- Core가 원래 커밋된 응답을 반환하면 증가하지 않습니다.
- 같은 `idempotency_key`를 다른 `request_hash`로 재사용해 Core가 거절하면 증가하지 않습니다.

저장되지 않는 분기:

- `ToolRejectedResponse`.
- `ToolDryRunResponse`.
- 읽기 전용 결과.
- 읽기 전용 `MethodResult`.
- `StatusResult`.
- 성공한 `StageArtifactResult` 스테이징 결과.

재시도 동작:

- 같은 `idempotency_key`와 같은 `request_hash`가 재실행되면 Core는 원래 커밋된 응답을 반환합니다.
- 재실행은 이벤트, 아티팩트 승격 또는 연결, `Write Authorization` 소비, 상태 변경을 다시 만들지 않습니다.
- 같은 `idempotency_key`가 다른 `request_hash`로 재사용되면 Core는 [상태 버전 충돌](api/errors.md#state-conflict-behavior)이 정의한 `STATE_VERSION_CONFLICT`를 반환합니다.

담당 문서 링크:

- 공개 충돌 동작은 [API 오류](api/errors.md#state-conflict-behavior)가 담당합니다.
- 분기별 저장 효과는 [저장 효과](storage-effects.md)가 담당합니다.

비주장: `request_hash`를 두 번째 고유 키에 넣어 같은 `idempotency_key`가 여러 커밋 응답으로 갈라질 수 있게 만들면 안 됩니다.

## 잠금 정책

의미:

- 런타임 변경은 Core가 소유한 상태 변경 경로를 통해 직렬화합니다.
- Core는 일반 SQLite 트랜잭션과 필요한 경우 프로세스/프로젝트 잠금을 사용합니다.
- 잠금은 동시 상태 쓰기를 보호합니다.

증가하는 경우:

- 보호된 작업이 일반 `state_version` 규칙에 따라 담당 문서가 허용한 상태 변경을 커밋할 때 증가합니다.

증가하지 않는 경우:

- 잠금 획득이나 해제 자체는 공개 상태 변경을 정의하지 않습니다.
- 기준 범위는 `persistent_locks` 테이블을 요구하지 않습니다.
- 영속 잠금/복구 메타데이터는 담당 문서가 승격하기 전까지 지원 범위 밖 운영 자료입니다.

재시도 동작:

- 전송 불확실성 뒤의 재시도도 멱등성과 상태 버전 규칙을 따릅니다.
- 잠금은 오래된 `expected_state_version`, 사용자 소유 판단, 권한 부여 경계를 우회하지 않습니다.

담당 문서 링크:

- 권한 배치는 [런타임 경계](runtime-boundaries.md)가 담당합니다.
- 보안 보장 표현과 비주장은 [보안](security.md)이 담당합니다.

## 마이그레이션 경계

의미:

- 마이그레이션 의미는 수락된 저장소 프로필 또는 스키마 버전 변경이 Core 권한 기록을 어떻게 보존하는지 설명합니다.
- 지원되는 마이그레이션 실행은 [범위 참조](scope.md)와 영향받는 저장소 담당 문서가 지원 경로를 정의할 때만 존재합니다.
- 마이그레이션 세부사항은 자신이 담당하는 버전, 저장소 프로필, 검증, 복구, 제약 강화 동작을 밝혀야 합니다.

증가하는 경우:

- 마이그레이션 담당 문서가 명시적으로 정의하지 않는 한, 마이그레이션에 대한 공개 API `state_version` 증가는 정의되지 않습니다.
- 수락된 마이그레이션은 자신의 담당 문서에서 버전과 저장소 프로필 동작을 밝힙니다.

증가하지 않는 경우:

- 상태 카드, 간결한 상태 보기, 상태 보기 최신성, 닫기 준비 상태, 보고서 문장은 현재 기록에서 읽는 시점에 파생합니다.
- 읽기 시점에 파생되는 자료는 마이그레이션 권한, 복구 입력, 저장소 변경 경로가 아닙니다.

재시도 동작:

- 마이그레이션 복구와 재시도는 담당 문서가 정의한 마이그레이션 경로를 따릅니다.

담당 문서 링크:

- 기록 배치와 DDL은 [저장소 기록](storage-records.md)이 담당합니다.
- 런타임 홈 분리는 [런타임 경계](runtime-boundaries.md)가 담당합니다.

기준 범위 마이그레이션 경계는 아래와 같습니다.

- 런타임 홈 메타데이터와 `project_state`, 또는 유지보수자가 수락한 동등한 메커니즘에 스키마/프로필 버전을 저장합니다.
- 커밋 전과 제약 강화 전에 담당 형태 JSON을 검증합니다.
- 담당 문서가 소유한 알 수 없는 상태 또는 enum 값은 담당 문서가 정의하기 전까지 유효하지 않은 값으로 취급합니다.
- null 허용 필드, 외래 키, enum 검사, JSON 검증을 강화할 때는 기존 행을 먼저 검증하거나 담당 문서가 정의한 복구 상태로 라우팅해야 합니다.
- `task_events.event_seq`를 유지한다면 그 순서를 보존합니다.
- 아티팩트 해시와 담당 연결을 보존하거나 영향을 받은 참조를 복구 대상으로 유효하지 않게 표시합니다.
- 커밋된 `tool_invocations` 재실행 행을 보존해 마이그레이션 뒤 멱등성이 갈라지지 않게 합니다.

이 문서는 기준 범위 밖 DDL 묶음, 마이그레이션 카탈로그, 프로필별 마이그레이션 세부사항을 의도적으로 제외합니다.

## 실패와 재시도

의미:

- 커밋 전 실패는 저장 효과가 없습니다.
- 트랜잭션 실패는 부분 결과를 남기면 안 됩니다.

예시는 아래와 같습니다.

- 오래된 `expected_state_version`
- 오래된 `WriteAuthorization.basis_state_version`
- 검증 실패
- 잘못된 요청
- 멱등 요청 해시 충돌

증가하는 경우:

- 완전히 커밋된 상태 변경 트랜잭션만 `state_version`을 증가시킵니다.

증가하지 않는 경우:

- 이런 실패는 커밋 전에 `ToolRejectedResponse`로 끝납니다.
- 새 커밋된 `dry_run=false` 상태 변경의 어느 한 부분이라도 실패합니다.

새 커밋된 `dry_run=false` 상태 변경에서 어느 한 부분이라도 실패하면 아래 결과가 부분적으로 남지 않아야 합니다.

- 현재 행 쓰기
- 이벤트
- 재실행 행
- 아티팩트 효과
- `Write Authorization` 소비
- 증거 업데이트
- 닫기 효과
- `state_version` 증가

재시도 동작:

- 재시도 규칙은 실패 종류에 따라 다릅니다.
- 요약 표는 세부 블록으로 연결합니다.

| 상황 | 재시도 경로 |
|---|---|
| 오래된 `expected_state_version` | [오래된 `expected_state_version`](#retry-stale-expected-state-version) |
| 같은 요청의 전송 불확실성 | [전송 불확실성](#retry-transport-uncertainty) |
| 같은 `idempotency_key`로 다른 요청 | [같은 키의 다른 요청](#retry-different-request-same-key) |
| 커밋 전 검증 실패 | [커밋 전 검증 실패](#retry-pre-commit-validation-failure) |

<a id="retry-stale-expected-state-version"></a>
**오래된 `expected_state_version`**

재시도 동작:

- 현재 상태를 다시 읽습니다.
- 최신 `project_state.state_version`으로 새 요청을 보냅니다.

주의:

- 최신성 확인일 뿐 사용자 소유 판단을 대신하지 않습니다.

<a id="retry-transport-uncertainty"></a>
**전송 불확실성**

재시도 동작:

- 같은 `idempotency_key`와 같은 `request_hash`로 재시도합니다.

주의:

- 이미 커밋됐다면 원래 응답이 재실행으로 반환되고 상태 변경은 반복되지 않습니다.

<a id="retry-different-request-same-key"></a>
**같은 키의 다른 요청**

재시도 동작:

- 같은 키로 재시도하지 않습니다.
- 새 멱등성 식별자를 사용합니다.

주의:

- 같은 키와 다른 `request_hash`는 `STATE_VERSION_CONFLICT`입니다.

<a id="retry-pre-commit-validation-failure"></a>
**커밋 전 검증 실패**

재시도 동작:

- 요청 내용을 고칩니다.
- 새 요청으로 다시 보냅니다.

주의:

- 실패한 요청은 재실행 행을 만들지 않습니다.

재시도는 사용자 판단 경계를 낮추지 않습니다. 실패 뒤에 새 수락, 민감 동작 승인, 잔여 위험 수락, `Write Authorization`이 필요하면 그 담당 경로를 다시 사용해야 합니다.

담당 문서 링크:

- 공개 충돌 오류는 [API 오류](api/errors.md)가 담당합니다.
- 분기별 저장 효과는 [저장 효과](storage-effects.md)가 담당합니다.

## 관련 담당 문서

- [API 오류](api/errors.md): `STATE_VERSION_CONFLICT` 같은 공개 충돌 오류.
- [저장 효과](storage-effects.md): 어떤 분기가 상태를 올리거나 올리지 않는지.
- [저장소 기록](storage-records.md): 버전 관리나 재실행 데이터를 저장하는 열.
- [아티팩트 저장소](storage-artifacts.md): 아티팩트 생명주기와 보존 경계.
- [런타임 경계](runtime-boundaries.md): 런타임 홈 분리.
