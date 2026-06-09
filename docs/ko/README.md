# 하네스 문서

이 문서는 현재 하네스 문서 세트의 한국어 진입점입니다. 하네스는 AI 지원 제품 작업을 위한 향후 로컬 작업 권한 서버입니다. 하네스가 다루려는 권한은 범위, 사용자 소유 판단, 증거, 검증 기대, 최종 수락, 닫기 가능 여부, 잔여 위험에 대한 하네스 기록과 상태 전이입니다.

이 저장소는 현재 문서 전용입니다. 서버/런타임 구현, 런타임 상태, 생성된 상태 보기, 생성된 운영 산출물, 실행 가능한 fixture, 적합성 실행기, 제품 구현 코드는 없습니다. 사용자의 제품 저장소도, 하네스 런타임 홈도, 실행 중인 하네스 인스턴스도 아닙니다.

하네스는 프롬프트 묶음, 운영체제 권한 제어, 임의 도구 샌드박스, 변조 방지 저장소, 기본 도구 실행 전 차단, 보안 격리가 아닙니다. [MVP 계획](build/mvp-plan.md)의 유지보수자 인계 상태가 다르게 말하지 않는 한, 이 문서는 향후 서버를 위한 계획 원천 자료로 봅니다.

## 현재 경로

이 진입점은 현재 간결한 활성 구조와 경로 색인만 가리킵니다.

| 목적 | 경로 |
|---|---|
| 첫 이해 모델 | [시작하기](start.md) |
| 사용자 작업 흐름 | [사용자 가이드](use/user-guide.md) |
| 에이전트 동작 | [에이전트 가이드](use/agent-guide.md) |
| 사용자 소유 판단 예시 | [판단 예시](use/judgment-examples.md) |
| 현재 MVP 계획과 구현 준비 결정 | [MVP 계획](build/mvp-plan.md) |
| 정확한 계약의 담당 문서 색인 | [참조 색인](reference/README.md) |
| 이후 후보 자료 | [이후 후보 색인](later/index.md) |
| 문서 작성 규칙 | [작성 가이드](maintain/authoring-guide.md) |
| 번역과 의미 일치 규칙 | [번역 가이드](maintain/translation-guide.md) |
| 수동 문서 점검 | [문서 점검](maintain/checks.md) |
| 안정적인 `doc_id` 경로 정보 | [doc-index.yaml](../doc-index.yaml) |

## 읽는 방법

먼저 [시작하기](start.md)를 읽습니다. 작업에 따라 [사용자 가이드](use/user-guide.md)나 [에이전트 가이드](use/agent-guide.md)를 이어서 봅니다. 현재 MVP 범위와 서버 코딩 전 결정은 [MVP 계획](build/mvp-plan.md)에서 확인합니다. 정확한 스키마, API 동작, 저장소, 상태 전이, 보안 표현, 상태 보기와 템플릿 규칙, 적합성 의미, 통합 동작, 용어, 문서 점검, 번역 규칙의 담당 문서는 [참조 색인](reference/README.md)에서 찾습니다.

`ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`, `MethodResult`, `response_kind`, `effect_kind` 같은 공개 API 응답 분기를 확인할 때는 참조 색인에서 공통 분기와 활성 값 집합은 API Schema Core로, 메서드별 응답 공용체와 상태 효과는 MVP API로, 거절/차단/dry-run 오류 경계는 API Errors로 이동합니다.

`ToolDryRunResponse`는 모든 `dry_run=true` 요청을 가리키는 포괄 응답이 아닙니다. API 담당 문서는 Core 커밋이나 스테이징 동작의 유효한 dry-run 미리보기와 읽기 전용 선택 동작을 구분합니다. `harness.status dry_run=true`와 `harness.close_task intent=check dry_run=true`는 `effect_kind=read_only`인 `StatusResult` 또는 `CloseTaskResult`를 반환합니다.

참조 색인은 공개 `ErrorCode` 계약과 `STATE_VERSION_CONFLICT`, 프로젝트 전체 `project_state.state_version`, 요청 수준 `VerifiedSurfaceContext.access_class`, `run_recording`, `artifact_registration`, `artifact_read`, `harness.record_run`, `harness.stage_artifact`, `StagedArtifactHandle` 승격, `existing_artifact` / `ArtifactRef` 영속 연결, 별도 아티팩트 본문 읽기, 확인된 로컬 접점 접근, `SensitiveActionScope`, 제품 파일 쓰기 범위인 `AuthorizedAttemptScope`, `CompletionPolicy`, `EvidenceSummary`, `close_task` 차단 사유, 읽기 전용 Projection, 역량 프로필, 탐지형 보장 조건, 사용자 소유 판단, 구체화 준비 상태의 활성 담당 문서로 안내합니다. 문서 작업 중 오류 코드, `access_class`, 아티팩트 생명주기 일관성은 문서 점검에서 확인합니다.

현재 MVP 경로 밖의 자료는 [이후 후보 색인](later/index.md)에서 봅니다. 이후 후보 자료는 관련 담당 문서가 범위와 증명 기대를 함께 승격하기 전까지 활성 전달 범위가 아닙니다.

문서 작업에는 [작성 가이드](maintain/authoring-guide.md), [번역 가이드](maintain/translation-guide.md), [문서 점검](maintain/checks.md)을 사용합니다. 문서 점검은 수동 유지보수 보조 자료입니다. 점검 라벨은 런타임 적합성, 최종 수락, 닫기 준비 상태, 구현 준비, 서버 코딩 시작 허가를 만들지 않습니다.

## 현재 MVP 경계

현재 MVP는 평소 말 입력과 Task 생성, `harness.update_scope`, 사용자 판단 기록, 민감 동작 승인 기록, 경로 수준 `harness.prepare_write`와 Write Authorization, `access_class=run_recording`으로 처리하는 `harness.record_run`, `access_class=artifact_registration`으로 처리하는 `harness.stage_artifact` 아티팩트 스테이징, `StagedArtifactHandle` 출처와 범위 검증을 통과한 스테이징된 아티팩트 승격, `existing_artifact` / `ArtifactRef` 영속 연결, `access_class=artifact_read`가 필요한 별도 아티팩트 본문 읽기, 간결한 `EvidenceSummary`, `harness.close_task` 차단 사유 계산, 읽을 때 계산되는 읽기 전용 상태/Projection 출력, 등록된 접점에서 확인된 로컬 접점 접근, 협력형 보장 표시, 관련 역량 확인이 통과한 뒤의 탐지형 보장 표시에만 닫혀 있습니다.

현재 MVP에는 `captured_artifact`, 접점 자체 아티팩트 캡처, projection reconcile, 영속 Projection 작업, 관리 블록 불일치 복구, 전체 Evidence Manifest, `qa_gate`, `verification_gate`, 명령 실행 관찰, 네트워크 관찰, 비밀값 접근 관찰, 명령/네트워크/비밀값 도구 실행 전 차단, Question Queue, Assumption Register, 영속 아티팩트로서의 Discovery Brief가 포함되지 않습니다. 이 항목들은 승격 전까지 [이후 후보 색인](later/index.md)의 이후 전용 자료입니다.

## 품질 규칙

의미가 바뀌는 문서 편집을 한 언어에만 남긴 채 마치지 않습니다. 리뷰 이력, 정리 메모, 임시 마이그레이션 계획을 활성 문서에 넣지 않습니다.

profile-gated 값을 기본 현재 MVP 값처럼 나열하지 않습니다. 이후 후보를 활성 요구사항처럼 설명하지 않습니다. 예방, 격리, 샌드박스, 변조 방지, 기본 도구 차단에 대한 근거 없는 보안 주장을 만들지 않습니다.

## 한영 문서 동시 유지

영어와 한국어 문서는 모두 활성 문서입니다. 주요 활성 문서는 `docs/en`과 `docs/ko` 아래에 대응 경로를 가져야 합니다. 영어 진입점은 [../en/README.md](../en/README.md)입니다.

대응 문서는 의미 일치를 유지해야 하지만 줄 단위 번역일 필요는 없습니다. 한국어 문서는 자연스러운 한국어 기술 문장으로 쓰고 정확한 식별자는 그대로 보존합니다.

에이전트는 작은 현재 맥락을 유지하고 필요한 담당 문서만 불러와야 합니다. 번역이나 의미 일치 검토가 필요한 경우가 아니면 같은 `doc_id`의 영어/한국어 문서를 한 프롬프트에 함께 넣지 않습니다. 이것이 에이전트 중복 주입 금지의 기본 규칙입니다.
