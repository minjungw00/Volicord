# 적합성 참조

## 1. 현재 상태

이 저장소는 문서 전용이며 아직 문서 검토 단계입니다. 여기에는 Harness Server 런타임, 적합성 실행기, 실행 가능한 fixture 파일, 생성된 적합성 보고서, 생성된 런타임 산출물, 현재 런타임 적합성 결과가 없습니다.

이 문서는 실행 가능한 적합성 테스트 모음이 아닙니다. 현재는 적합성의 의미, 향후 fixture 형식, 주장 권한, 간결한 대표 예시를 다루는 계획 담당 문서입니다. 현재 단계와 인계 상태는 [MVP 계획](../build/mvp-plan.md#문서-수락-상태)이 담당합니다.

## 2. 적합성이 뜻하는 것

적합성은 Harness Server와 실행기가 생긴 뒤, 향후 실행 점검이 담당 문서가 정의한 특정 동작을 담당 문서의 권한 기록과 비교할 수 있다는 뜻입니다. 향후 점검은 Core, API, 운영자 동작 하나를 실행하고, 응답에 담긴 사실과 담당 문서가 소유하는 상태 변경 효과를 수집한 뒤 구조화된 기대값과 비교합니다. 금지된 부작용이 없어야 한다는 주장도 여기에 포함됩니다.

문서 점검은 별도입니다. Markdown 문서 점검은 링크, 용어, 담당 문서 경계, active/later 문구, 보안 표현, 한영 문서 의미 일치를 확인합니다. 이는 현재 문서 유지보수 보조 도구일 뿐이며 런타임 적합성이 아닙니다.

적합성은 생성된 글, 에이전트 요약, 렌더링된 보고서, 상태 문구를 판단하지 않습니다. 담당 문서가 권한 있는 사실로 정한 것만 판단합니다.

## 3. 아직 존재하지 않는 것

아래 항목은 향후 구현 작업이며 현재 저장소 내용이 아닙니다.

- Harness Server 런타임 또는 Harness Runtime Home 데이터
- 실행 가능한 fixture 파일 또는 fixture 디렉터리
- 적합성 실행기 또는 `harness conformance run` 구현
- 생성된 적합성 보고서, 생성된 런타임 산출물, Projection, 운영 파일, 런타임 상태
- 현재 MVP 동작이나 이후 후보에 대한 현재 런타임 결과
- 기준 `reference-local-mcp` 접점의 현재 `changed_path_detection_verification=passed` 결과
- 예방적 차단, OS 권한 제어, 임의 도구 샌드박스, 변조 방지 저장소, 보안 격리, profile-gated `preventive` / `isolated` 보장 주장에 대한 현재 런타임 증명

이 문서의 예시는 계획을 도울 수 있습니다. 하지만 런타임 상태, 수락 증거, 닫기 준비 상태, 잔여 위험 수락, 생성된 보고서, 구현 준비 상태를 만들지 않습니다.

## 4. fixture 형식

fixture 형식은 향후 구조를 설명할 뿐 현재 파일을 만들지 않습니다. Harness Server와 실행기가 생긴 뒤 승격된 fixture는 아래 부분을 담은 작은 구조화 기록이어야 합니다.

| 부분 | 목적 |
|---|---|
| `scenario_id` | 검토할 동작의 안정적인 식별자입니다. |
| 권한 맥락 | 동작 전에 필요한 Task, Change Unit, 상태 버전, 접점, 담당 문서 참조, Core 상태, 저장소 행, `ArtifactRef`, 접점 기능 사실입니다. |
| 동작 | 담당 요청 스키마를 사용하는 공개 Core, API, 운영자 요청 하나입니다. |
| 기대 주장 | 구조화된 응답에 담긴 사실, 담당 문서가 소유하는 상태 변경 효과, 저장소 또는 아티팩트 사실, 차단 사유 사실, 오류 사실, 보장 표시 사실, 금지된 부작용의 필수 부재입니다. |
| 담당 문서 링크 | 정확한 값과 의미를 정의하는 API, Core, Storage, Security, Agent Integration, ArtifactRef, 정책 담당 문서입니다. |

구체화된 fixture는 공개 담당 스키마를 사용해야 합니다. fixture 전용 enum 값, 가짜 필드, 상태로 쓰는 지역화 표시 라벨, 글로만 된 기대값, 이후 후보 전용 값을 만들면 안 됩니다.

## 5. 주장 권한

주장 권한은 향후 fixture가 판단할 수 있는 사실의 좁은 범위입니다. 권한은 시나리오 설명이나 생성된 요약이 아니라 담당 문서가 정의한 사실에서 옵니다.

향후 권한 있는 주장은 다음을 사용할 수 있습니다.

- 공개 담당 API가 반환한 응답에 담긴 사실
- Core가 소유하는 Task, Change Unit, 사용자 판단, Write Authorization, Run 또는 증거 요약, 차단 사유, 닫기, 잔여 위험 상태
- Storage가 소유하는 행 변경 효과, 멱등성/재실행 사실, 프로젝트 전체 `project_state.state_version` 사실, 아티팩트가 범위에 있을 때의 아티팩트 무결성 사실
- 공개 API 요청 하나에 정확히 하나만 있는 요청 수준 `VerifiedSurfaceContext.access_class` 사실
- `harness.stage_artifact` 담당 문서가 범위에 있을 때의 임시 `StagedArtifactHandle` 응답 사실. 여기에는 서버가 기록한 `created_by_surface_id`와 `created_by_surface_instance_id`가 포함됩니다. 단, 지속 아티팩트 권한은 호환되는 `record_run` 승격이 그 출처 기록 필드를 현재 확인된 `surface_id`와 `surface_instance_id`에 맞게 검증한 뒤에만 주장할 수 있습니다.
- Core 담당 문서가 이벤트 이름을 승격한 뒤의 안정적인 `task_events`
- API, Core, Security, Agent Integration 담당 문서와 맞는 주 `ErrorCode`, 구조화된 차단 사유 필드, 보장 표시 사실
- 지속되는 승인 없음, Run 행 없음, 아티팩트 변경 없음, 닫기 상태 변경 없음 같은 금지된 부작용의 부재 주장

현재 활성 예시는 `cooperative`와 지원되는 `detective` 사실만 주장할 수 있습니다. 기준 `reference-local-mcp` 접점에서는 `changed_path_detection_verification=passed`일 때만, 그리고 검증된 변경 경로 탐지 범위 안에서만 `detective` 주장이 유효합니다. `not_run`, 예전 `planned_not_run` 문구, `failed`, `stale`은 통과한 주장 상태가 아니며 `detective` 라벨을 정당화하지 못합니다. `preventive` 또는 `isolated` 주장은 승격된 프로필과 그 프로필에 대해 담당 문서가 정의한 증명 경로가 있을 때만 유효합니다. 적합성 계획 문구만으로 이런 표시 값이 현재 실행 가능하거나 증명된 것이 되지 않습니다.

권한이 없는 자료에는 시나리오 설명, 주석, 작성자 메모, 렌더링된 Markdown, 생성된 보고서, 상태 문구, 에이전트 요약, 문서 점검 라벨, Projection이 포함됩니다. Projection 지원이 명시 범위에 있을 때만 최신성 또는 가용성 주장이 예외로 가능할 수 있습니다.

## 6. 현재 MVP 대표 예시

아래 항목은 문서 수준의 간결한 동작 참조일 뿐입니다. fixture 파일, 전체 YAML 본문, 현재 런타임 결과, 완전한 적합성 테스트 모음, 생성된 런타임 객체, 구현 계획이 아닙니다. 이 행의 응답 분기 기대값은 문서 수준 적합성 예시입니다. 실행 가능한 fixture, 테스트 실행기, 생성된 운영 파일, 런타임 상태를 만들지 않습니다. [MVP 계획](../build/mvp-plan.md#첫-내부-스모크-목표)의 첫 내부 문서 스모크 목표는 이 행들을 참고할 수 있지만, 향후 실행 가능한 점검에는 담당 문서가 승격한 fixture와 실행기가 따로 필요합니다.

| 예시 | 동작 | 향후 주장 초점 |
|---|---|---|
| `MVP-ACTIVE-registered-surface-mismatch-blocks-mutation` | `surface_id`가 저장된 `LocalSurfaceRegistration`을 고르더라도, 도달한 전송/세션/바인딩이 등록 내용과 맞지 않는 변경 요청은 커밋 전에 멈춰야 합니다. | `VerifiedSurfaceContext.verified=false`이고 `failure_reason=mismatch` 또는 `revoked`이면 `LOCAL_ACCESS_MISMATCH`로 이어집니다. 담당 기록, 이벤트, 재실행 행, Write Authorization, Run, 아티팩트, 닫기 효과, `project_state.state_version` 증가는 생기지 않습니다. 복사된 `surface_id`는 권한이 아닙니다. |
| `MVP-ACTIVE-verified-local-surface-allows-owner-mutation` | 같은 프로젝트의 활성 로컬 등록이 있고 서버가 해당 메서드의 필수 `access_class`에 대해 `VerifiedSurfaceContext.verified=true`를 파생할 때만 변경 요청이 다음 확인으로 넘어갈 수 있습니다. | 요청은 메서드별 범위, 판단, 아티팩트, 닫기, 현재 `ToolEnvelope.expected_state_version` 확인을 거칩니다. 그 담당 확인이 모두 통과하면 커밋된 변경은 메서드가 소유한 효과만 만들고 `project_state.state_version`을 정확히 한 번 올립니다. 접점 검증만으로는 범위, 승인, 증거, Write Authorization, 닫기 상태가 생기지 않습니다. |
| `MVP-ACTIVE-single-access-class-per-public-request` | 현재 MVP는 공개 API 요청 하나마다 요청 수준 `VerifiedSurfaceContext.access_class` 하나만 평가합니다. | `ArtifactInput[]`에 `source_kind=staged_artifact`가 있어도 `harness.record_run`은 `run_recording`만 요구합니다. `harness.stage_artifact`는 `artifact_registration`만 요구하고, 아티팩트 본문 읽기는 `artifact_read`를 요구합니다. 하나의 공개 요청이 `run_recording`과 `artifact_registration`을 모두 요구한다고 주장하면 안 됩니다. |
| `MVP-ACTIVE-detective-display-capability-gated` | `detective` 보장 표시는 대상 관찰 범위에 필요한 활성 역량 확인이 통과한 뒤에만 유효합니다. | 기준 `reference-local-mcp`에서는 `changed_path_detection_verification=passed`가 필요하며, 검증된 변경 경로 탐지 범위에만 적용됩니다. `not_run`, 예전 `planned_not_run`, `failed`, `stale`이면 표시를 `cooperative`로 유지하거나 더 강한 역량이 필수일 때 `CAPABILITY_INSUFFICIENT`를 반환합니다. 명령, 네트워크, 비밀값, 아티팩트 캡처, 차단, 격리 사실을 검증된 것으로 표시하지 않습니다. |
| `MVP-ACTIVE-shaping-readiness-gap-blocks-or-asks` | `ShapingReadiness` 읽기는 목표, 범위, Change Unit, Autonomy Boundary, 이름 붙은 사용자 소유 차단 사유, 다음 안전한 행동이 비어 있음을 보여줄 수 있지만 지속 계획 아티팩트를 만들지 않습니다. | 준비 공백은 담당 경로에 따라 활성 차단 사유, `StateSummary.shaping_readiness` 공백, 또는 대기 중인 `UserJudgment` 후보로 돌아옵니다. Discovery Brief, Question Queue, Assumption Register, 증거, 최종 수락, 잔여 위험 수락, 닫기 상태, 이후 계획 아티팩트는 이 읽기만으로 생기지 않습니다. |
| `MVP-ACTIVE-project-state-version-stale-mutation-rejected` | 상태를 바꾸는 non-dry-run 요청의 `ToolEnvelope.expected_state_version`이 현재 프로젝트 전체 `project_state.state_version`보다 오래되었으면 요청은 커밋 전에 실패합니다. `tasks.state_version`은 활성 충돌 기준이나 동시성 기준이 아닙니다. | 예상 응답 분기는 `ToolRejectedResponse`이고 공개 `ErrorCode`는 `STATE_VERSION_CONFLICT`입니다. 실패한 시도는 현재 기록, 이벤트, 아티팩트, 증거, Write Authorization, 닫기 상태, 재실행 행, 상태 버전 증가를 만들지 않습니다. 성공적으로 커밋된 변경만 프로젝트 전체 `project_state.state_version`을 정확히 1 올립니다. |
| `MVP-ACTIVE-sensitive-approval-records-sensitive-action-scope` | 민감 동작 승인은 `SensitiveActionScope`를 가진 `judgment_kind=sensitive_approval` 사용자 판단으로 기록되며, 경로 수준 `AuthorizedAttemptScope`와 분리됩니다. | 기록된 범위는 민감 동작과 정직한 `SensitiveActionScope.capability_claim`을 이름 붙입니다. 하지만 Write Authorization, 증거, 최종 수락, 잔여 위험 수락, OS 강제, 샌드박스, 차단, 격리, 아티팩트 권한을 만들지 않습니다. 제품 파일 Write Authorization에는 여전히 담당 `prepare_write` 경로가 필요합니다. |
| `MVP-ACTIVE-prepare-write-requires-compatible-scope-and-approval` | `harness.prepare_write`는 의도한 제품 파일 쓰기가 활성 Task, Change Unit, 범위, 기준선, Autonomy Boundary, 필요한 사용자 판단, 필요한 별도 민감 동작 승인, 접점 역량, 현재 프로젝트 전체 상태 버전과 호환될 때만 `decision=allowed`를 반환할 수 있습니다. | 커밋된 차단 결과의 예상 응답 분기는 `PrepareWriteResult`이고 `decision=blocked`, `approval_required`, 또는 `decision_required`를 씁니다. 프로젝트 전체 버전 불일치는 공개 `ErrorCode` `STATE_VERSION_CONFLICT`가 있는 `ToolRejectedResponse`이며 별도 `prepare_write` decision 값이 아닙니다. 차단 결과와 dry-run은 소비 가능한 Write Authorization, Run, 아티팩트, 증거, 닫기, 최종 수락, 잔여 위험 상태를 만들지 않습니다. |
| `MVP-ACTIVE-authorized-attempt-scope-product-file-write-only` | `AuthorizedAttemptScope`는 허용된 `prepare_write`가 저장하고 나중에 `harness.record_run`이 비교하는 경로 수준 제품 파일 쓰기 시도 범위일 뿐입니다. | 이 범위는 Task, Change Unit, 프로젝트 전체 `basis_state_version`, `surface_id`, 의도한 제품 파일 경로, 제품 파일 쓰기 민감 범주, 기준선, 관련 사용자 판단 참조, 정직한 `guarantee_level`을 다룹니다. 명령 실행, 의존성 설치, 네트워크 효과, 비밀값 접근, 배포, 파괴적 동작, 시스템 접근, 접점 자체 아티팩트 캡처, 도구 실행 전 차단, 격리는 `AuthorizedAttemptScope` 필드가 아닙니다. 이를 검증된 쓰기 범위처럼 표현하려는 시도는 승인 생성 없이 거부되거나 차단됩니다. |
| `MVP-ACTIVE-record-run-consumes-write-authorization-once` | 호환되는 제품 쓰기 `harness.record_run`은 맞는 활성 Write Authorization을 정확히 한 번 소비합니다. | Run은 호환되는 Write Authorization 하나와 연결되고, 그 Write Authorization은 해당 Run이 소비한 것으로 표시됩니다. 멱등 재실행은 다시 소비하지 않고 원래 응답을 반환합니다. Write Authorization의 프로젝트 전체 근거 버전이 오래됐으면 `STATE_VERSION_CONFLICT`를 반환하고, Write Authorization이 없으면 `WRITE_AUTHORIZATION_REQUIRED`를 사용합니다. 만료, 철회, 이미 소비됨, 비호환, 승인 범위 밖 관찰 시도는 담당 코드로 처리합니다. 이런 실패 시도는 성공 Run, 증거, 아티팩트 승격, 닫기 상태, 권한 소비, 상태 버전 증가를 만들지 않습니다. |
| `MVP-ACTIVE-stage-artifact-temporary-handle-only` | 확인된 로컬 접점이 `VerifiedSurfaceContext.access_class=artifact_registration`인 상태에서 `harness.stage_artifact`가 성공하면, 호출자가 제공한 안전한 바이트나 안전 공지는 같은 프로젝트/같은 Task의 임시 `StagedArtifactHandle`로만 스테이징됩니다. | 성공한 응답과 스테이징 상태는 성공한 요청의 `VerifiedSurfaceContext`에서 온 `created_by_surface_id`와 `created_by_surface_instance_id`를 기록합니다. 이 필드는 사용자나 에이전트가 권한 주장으로 제출하는 값이 아닙니다. 아직 지속 `ArtifactRef`는 없고 증거 충분성도 바뀌지 않습니다. 스테이징은 Core 기록, 증거 요약, 차단 사유, 이벤트, `tool_invocations` 재실행 행, 닫기 효과, `project_state.state_version` 증가를 만들지 않습니다. 핸들은 범위가 정해져 있고 만료되며 한 번만 소비될 수 있어야 합니다. 증거 권한도, 임의 로컬 파일 권한도, bearer token도 아닙니다. Projection 파일, 생성된 Markdown, 대화 텍스트, Product Repository 파일, 에이전트 기억은 이 출처 기록을 만들 수 없습니다. |
| `MVP-ACTIVE-record-run-artifact-input-validation-order` | `harness.record_run`은 커밋 전에 아티팩트 입력을 결정적 메서드 순서로 검증합니다. | 순서는 요청 수준 `VerifiedSurfaceContext.access_class=run_recording`, 프로젝트 전체 `ToolEnvelope.expected_state_version`, 참조된 Task와 Change Unit, 제품 파일 쓰기를 기록할 때 호환되는 Write Authorization, `staged_artifact` 핸들 검증, 스테이징된 핸들의 `project_id`/`task_id`/`created_by_surface_id`/`created_by_surface_instance_id`/만료 여부/소비 상태/`sha256`/`size_bytes`/`redaction_state` 확인, 스테이징된 핸들 승격, 스테이징된 핸들 소비 표시, 같은 프로젝트와 허용된 범위의 지속 `ArtifactRef`인지 보는 `existing_artifact` 검증, 아티팩트 본문 읽기 없음입니다. 아티팩트 본문 읽기는 `artifact_read`를 요구합니다. |
| `MVP-ACTIVE-record-run-promotes-staged-artifact-to-artifact-ref` | 요청이 `VerifiedSurfaceContext.access_class=run_recording`으로 확인되었고, 스테이징된 핸들이 같은 `project_id`, `task_id`, 서버가 기록한 `created_by_surface_id`, 서버가 기록한 `created_by_surface_instance_id`와 맞으며, 만료되지 않았고 아직 소비되지 않았고 `sha256`과 `size_bytes`가 맞으면 `harness.record_run`은 그 핸들을 소비할 수 있습니다. | `record_run`은 성공하고, 같은 커밋 트랜잭션에서 스테이징된 핸들을 소비된 것으로 표시하며, 스테이징된 `sha256`, `size_bytes`, `content_type`, `redaction_state`, 프로젝트, Task, 서버가 기록한 생성 접점 출처 기록과 맞는 지속 `ArtifactRef`를 만듭니다. Run 증거는 새 `ArtifactRef`에 연결됩니다. 이 승격은 `run_recording`으로 처리하며, `artifact_registration`은 앞선 `harness.stage_artifact` 요청에만 속합니다. |
| `MVP-ACTIVE-record-run-rejects-staged-artifact-surface-instance-mismatch` | 요청은 `VerifiedSurfaceContext.access_class=run_recording`으로 확인되었지만, 존재하는 스테이징된 핸들의 서버 기록 `created_by_surface_id` 또는 `created_by_surface_instance_id`가 현재 확인된 `surface_id` 또는 `surface_instance_id`와 다르면 `harness.record_run`은 승격 전에 실패합니다. | 예상 응답 분기는 `ToolRejectedResponse`이고 공개 `ErrorCode`는 `VALIDATION_FAILED`이며 `ToolError.details.artifact_input_error.reason=staged_handle_surface_mismatch`가 들어갑니다. 요청 수준 접점 검증이나 역량 확인 자체가 실패한 경우가 아니라면 예상 오류는 `LOCAL_ACCESS_MISMATCH`도 아니고 `CAPABILITY_INSUFFICIENT`도 아닙니다. 거절 응답은 `run_summary`, Run, 지속 `ArtifactRef`, 증거 업데이트, 승인 소비, 재실행 행, 스테이징된 핸들 소비, 상태 버전 증가를 만들지 않습니다. |
| `MVP-ACTIVE-record-run-links-existing-artifact-without-registering-bytes` | `ArtifactInput.source_kind=existing_artifact`이면 참조된 `ArtifactRef`는 이미 지속 아티팩트여야 하며 같은 프로젝트와 허용된 Task 범위에서 유효해야 합니다. | `record_run`은 기존 `ArtifactRef`를 Run 증거에 연결할 뿐입니다. 새 아티팩트 바이트를 등록하거나, 새 바이트를 스테이징하거나, `StagedArtifactHandle`을 소비하거나, 아티팩트 본문을 읽지 않습니다. 유효하지 않거나 범위를 벗어난 기존 아티팩트는 커밋 전에 실패하며 아티팩트, 증거, 재실행, 상태 버전 효과를 만들지 않습니다. |
| `MVP-ACTIVE-captured-artifact-rejected-in-active-mvp` | `captured_artifact`, 캡처된 핸들, 접점 자체 아티팩트 캡처, 원시 캡처 어댑터 출력, 원시 파일시스템 경로, 원시 로그는 현재 MVP 아티팩트 권한이 아닙니다. | `ArtifactInput.source_kind`는 `staged_artifact` 또는 `existing_artifact`만 받습니다. `captured_artifact` 입력 형태는 변경 전에 `VALIDATION_FAILED`로 거부되거나 비활성 기능으로 남습니다. Run, 지속 `ArtifactRef`, 아티팩트 연결, 증거 요약, 닫기 효과, 상태 버전 증가는 생기지 않습니다. |
| `MVP-ACTIVE-close-task-blocks-evidence-insufficient` | `harness.close_task intent=complete`는 활성 `CompletionPolicy`에 따라 필수 증거가 없거나, 부분적이거나, 오래됐거나, 차단됐거나, 충분하지 않으면 차단됩니다. | 예상 응답 분기는 `CloseTaskResult`이고 `CloseTaskResult.close_state=blocked`, `CloseBlocker.category=evidence`, 선택적 주 오류 `EVIDENCE_INSUFFICIENT`를 포함할 수 있습니다. Task는 열린 상태로 남습니다. 최종 수락과 잔여 위험 수락은 `EvidenceCoverageItem.required_for_close=true`인 필수 증거 공백을 충분한 증거로 바꾸지 못합니다. |
| `MVP-ACTIVE-close-task-blocks-required-artifact-unavailable` | `harness.close_task intent=complete`는 닫기에 필요한 `ArtifactRef`가 누락, 사용 불가, 무결성 실패, 허용된 안전 공지를 넘어선 차단, 또는 사용할 수 없는 상태이면 차단됩니다. | 예상 응답 분기는 `CloseTaskResult`이고 `CloseBlocker.category=artifact_availability`와 선택적 주 오류 `ARTIFACT_MISSING`을 포함할 수 있습니다. Task는 열린 상태로 남고 최종 닫기 상태는 커밋되지 않습니다. 증거 충분성과 아티팩트 가용성은 별개이며, 최종 수락이나 잔여 위험 수락은 필요한 아티팩트를 대신하지 않습니다. |
| `MVP-ACTIVE-close-task-blocks-final-acceptance-missing` | `harness.close_task intent=complete`는 증거와 닫기 관련 아티팩트가 통과한 뒤에도 필요한 `final_acceptance`가 없거나, 거절됐거나, 오래됐거나, 보이는 닫기 근거와 연결되지 않았으면 차단됩니다. | 예상 응답 분기는 `CloseTaskResult`이고 `CloseBlocker.category=final_acceptance`, `CloseBlocker.required_judgment_kind=final_acceptance`, 선택적 주 오류 `ACCEPTANCE_REQUIRED`를 포함할 수 있습니다. Task는 열린 상태로 남으며, 증거, 상태 문구, 채팅, 잔여 위험 수락에서 최종 수락을 추론하지 않습니다. |
| `MVP-ACTIVE-close-task-blocks-visible-unaccepted-residual-risk` | `harness.close_task intent=complete`는 닫기에 영향을 주는 잔여 위험이 보이지만 호환되는 `residual_risk_acceptance`가 없으면 차단됩니다. | 예상 응답 분기는 `CloseTaskResult`이고 `CloseBlocker.category=residual_risk_acceptance`, `CloseBlocker.required_judgment_kind=residual_risk_acceptance`, `DECISION_REQUIRED` 또는 `DECISION_UNRESOLVED`를 포함합니다. Task는 열린 상태로 남습니다. 이 경우는 사용자가 판단할 만큼 위험이 보이지 않을 때의 `RESIDUAL_RISK_NOT_VISIBLE`과 구분됩니다. |
| `MVP-ACTIVE-close-task-check-read-only` | `close_task intent=check`는 상태를 바꾸지 않고 닫기 가능 여부와 차단 사유를 계산합니다. | 예상 응답 분기는 `CloseTaskResult`이고 `base.effect_kind=read_only`입니다. `tasks`, `blockers`, `task_events`, `tool_invocations`, 닫기 상태, 증거 요약, 아티팩트, Write Authorization, `project_state.state_version` 변경이 없습니다. |
| `MVP-ACTIVE-close-task-supersede-one-state-version` | `close_task intent=supersede`는 성공 완료가 아닌 종료 경로이며, 기존 Task와 활성 Task 포인터를 함께 바꿀 수 있습니다. | 유효한 supersede는 `Task.lifecycle_phase=superseded`, `Task.close_reason=superseded`, `Task.result=superseded`를 저장하고, 유효한 열린 같은 프로젝트 `superseding_task_id`를 `project_state.active_task_id`로 설정하거나 담당 규칙에 따라 포인터를 비웁니다. 생명주기와 포인터 변경은 하나의 프로젝트 전체 상태 변경이며 `project_state.state_version`을 정확히 한 번만 올립니다. 유효하지 않은 supersession은 완료에 필요한 증거, 수락, 잔여 위험 요구사항을 숨기지 않고 적용 가능한 차단 사유를 반환합니다. |

## 7. 향후 항목을 목록으로만 유지하는 경계

향후 fixture 계열은 [이후 후보 색인: Future fixture families](../later/index.md#future-fixture-families)에 둡니다. 그 색인은 이후 후보 이름만 보존하며, 이 문서는 그 목록을 반복하지 않습니다.

향후 계열 이름은 시나리오 스크립트, fixture 본문, 활성 API 페이로드 예시, 실행기 또는 보고 요구사항, 현재 MVP 범위, 구현 작업, 현재 결과, 현재 런타임 증명이 아닙니다. 향후 담당 문서가 좁은 동작을 범위, 대체 동작, 정확한 계약, 향후 승격에 필요한 증명 경로 기대치와 함께 승격해야 실행 가능한 fixture 자료가 생깁니다.

## 8. 지표 경계

현재 문서 세트에서 지표는 적합성 권한이 아닙니다. 향후 로컬 지표는 진단이나 계획에 유용할 수 있지만, 담당 문서가 승격하기 전에는 읽기 전용 파생 표시로 남습니다.

지표는 Core 상태를 만들거나, 증거를 충족하거나, QA 또는 검증을 통과시키거나, 쓰기를 승인하거나, 최종 결과를 수락하거나, 잔여 위험을 수락하거나, 작업을 닫거나, 구현 준비 상태를 증명하거나, 런타임 적합성을 대신하면 안 됩니다. 향후 지표가 승격되면 담당 문서가 원천 기록, 최신성 경계, 표시 문구, 대체 불가 규칙을 정의해야 합니다.
