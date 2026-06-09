# Harness Documentation / 하네스 문서

This directory contains the active bilingual documentation set for a future local Harness Server. The repository is documentation-only today. It is not a running Harness instance, not the user's Product Repository, and not a Harness Runtime Home.

이 디렉터리는 향후 로컬 하네스 서버를 위한 현재 한영 문서 세트를 담고 있습니다. 이 저장소는 현재 문서 전용입니다. 실행 중인 하네스 인스턴스도, 사용자의 제품 저장소도, 하네스 런타임 홈도 아닙니다.

Harness documentation is planning source material. It is not runtime state, generated projections, evidence, QA, final acceptance, residual-risk records, close records, server code, or product code.

하네스 문서는 계획을 위한 원천 자료입니다. 런타임 상태, 생성된 상태 보기, 증거, QA, 최종 수락, 잔여 위험 기록, 닫기 기록, 서버 코드, 제품 코드가 아닙니다.

## Choose A Language / 언어 선택

| Language / 언어 | Entry / 진입점 |
|---|---|
| English | [en/README.md](en/README.md) |
| 한국어 | [ko/README.md](ko/README.md) |

## Current Routes / 현재 경로

English and Korean docs are both active. Every major active doc should have a paired path. Keep semantic parity across paired docs; line-by-line translation is not required.

영어와 한국어 문서는 모두 활성 문서입니다. 주요 활성 문서에는 대응 경로가 있어야 합니다. 대응 문서는 의미 일치를 유지합니다. 줄 단위 번역은 요구하지 않습니다.

| Purpose / 목적 | English | 한국어 |
|---|---|---|
| Start / 시작 | [Start](en/start.md) | [시작하기](ko/start.md) |
| User guide / 사용자 가이드 | [User Guide](en/use/user-guide.md) | [사용자 가이드](ko/use/user-guide.md) |
| Agent guide / 에이전트 가이드 | [Agent Guide](en/use/agent-guide.md) | [에이전트 가이드](ko/use/agent-guide.md) |
| Judgment examples / 판단 예시 | [Judgment Examples](en/use/judgment-examples.md) | [판단 예시](ko/use/judgment-examples.md) |
| Current MVP / 현재 MVP | [MVP Plan](en/build/mvp-plan.md) | [MVP 계획](ko/build/mvp-plan.md) |
| Contract owner index / 계약 담당 문서 색인 | [Reference Index](en/reference/README.md) | [참조 색인](ko/reference/README.md) |
| Later candidates / 이후 후보 | [Later Index](en/later/index.md) | [이후 후보 색인](ko/later/index.md) |
| Authoring guide / 작성 가이드 | [Authoring Guide](en/maintain/authoring-guide.md) | [작성 가이드](ko/maintain/authoring-guide.md) |
| Translation guide / 번역 가이드 | [Translation Guide](en/maintain/translation-guide.md) | [번역 가이드](ko/maintain/translation-guide.md) |
| Checks / 문서 점검 | [Checks](en/maintain/checks.md) | [문서 점검](ko/maintain/checks.md) |
| Route index / 경로 색인 | [doc-index.yaml](doc-index.yaml) | [doc-index.yaml](doc-index.yaml) |

## Reader Guidance / 독자 안내

Use `start.md` for the first model, `use/*` for user and agent behavior, `build/mvp-plan.md` for current MVP planning and implementation-readiness decisions, `reference/README.md` for exact contract owners, `later/index.md` for later-only candidate material, `maintain/*` for documentation work, and `doc-index.yaml` for stable `doc_id` routing metadata.

첫 이해 모델은 `start.md`에서 봅니다. 사용자와 에이전트 동작은 `use/*`, 현재 MVP 계획과 구현 준비 결정은 `build/mvp-plan.md`, 정확한 계약의 담당 문서는 `reference/README.md`, 이후 전용 후보 자료는 `later/index.md`, 문서 작업 규칙은 `maintain/*`, 안정적인 `doc_id` 경로 정보는 `doc-index.yaml`에서 봅니다.

For public API response branch questions, including `ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`, `MethodResult`, `response_kind`, and `effect_kind`, the Reference Index routes shared branches and active value sets to API Schema Core, method-specific response unions and state effects to MVP API, and rejected/blocked/dry-run error boundaries to API Errors.

`ToolDryRunResponse` is not the umbrella response for every `dry_run=true` request. The API owners distinguish valid dry-run previews for selected Core-commit or staging operations from read-only selections such as `harness.status dry_run=true` and `harness.close_task intent=check dry_run=true`, which return `StatusResult` or `CloseTaskResult` with `effect_kind=read_only`.

`ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`, `MethodResult`, `response_kind`, `effect_kind` 같은 공개 API 응답 분기를 확인할 때는 참조 색인에서 공통 분기와 활성 값 집합은 API Schema Core로, 메서드별 응답 공용체와 상태 효과는 MVP API로, 거절/차단/dry-run 오류 경계는 API Errors로 이동합니다.

`ToolDryRunResponse`는 모든 `dry_run=true` 요청을 가리키는 포괄 응답이 아닙니다. API 담당 문서는 Core 커밋이나 스테이징 동작의 유효한 dry-run 미리보기와 읽기 전용 선택 동작을 구분합니다. `harness.status dry_run=true`와 `harness.close_task intent=check dry_run=true`는 `effect_kind=read_only`인 `StatusResult` 또는 `CloseTaskResult`를 반환합니다.

The Reference Index routes active owner documents for the public `ErrorCode` contract and `STATE_VERSION_CONFLICT`, project-wide `project_state.state_version`, request-level `VerifiedSurfaceContext.access_class`, `run_recording`, `artifact_registration`, `artifact_read`, `harness.record_run`, `harness.stage_artifact`, `StagedArtifactHandle` promotion, persistent `existing_artifact` / `ArtifactRef` linking, separate artifact body reads, verified local surface access, `SensitiveActionScope`, product-file `AuthorizedAttemptScope`, `CompletionPolicy`, `EvidenceSummary`, `close_task` blockers, read-only projections, capability profiles, detective guarantee gating, user-owned judgments, shaping readiness, maintain checks for error-code and access-class consistency, and translation rules.

참조 색인은 공개 `ErrorCode` 계약과 `STATE_VERSION_CONFLICT`, 프로젝트 전체 `project_state.state_version`, 요청 수준 `VerifiedSurfaceContext.access_class`, `run_recording`, `artifact_registration`, `artifact_read`, `harness.record_run`, `harness.stage_artifact`, `StagedArtifactHandle` 승격, `existing_artifact` / `ArtifactRef` 영속 연결, 별도 아티팩트 본문 읽기, 확인된 로컬 접점 접근, `SensitiveActionScope`, 제품 파일 쓰기 범위인 `AuthorizedAttemptScope`, `CompletionPolicy`, `EvidenceSummary`, `close_task` 차단 사유, 읽기 전용 Projection, 역량 프로필, 탐지형 보장 조건, 사용자 소유 판단, 구체화 준비 상태, 오류 코드와 `access_class` 일관성 문서 점검, 번역 규칙의 활성 담당 문서로 안내합니다.

## Active MVP Boundary / 현재 MVP 경계

The active MVP is closed to plain-language intake and Task creation, `harness.update_scope`, user judgment recording, sensitive approval recording, path-level `harness.prepare_write` and Write Authorization, `harness.record_run` with `access_class=run_recording`, artifact staging through `harness.stage_artifact` with `access_class=artifact_registration`, staged artifact promotion after `StagedArtifactHandle` provenance and scope validation, persistent `existing_artifact` / `ArtifactRef` linking, separate artifact body reads with `access_class=artifact_read`, `EvidenceSummary`, `harness.close_task` blocker calculation, read-time read-only status/projection output, verified local surface access through a registered surface, cooperative guarantee display, and detective guarantee display only after the relevant capability check has passed.

현재 MVP는 평소 말 입력과 Task 생성, `harness.update_scope`, 사용자 판단 기록, 민감 동작 승인 기록, 경로 수준 `harness.prepare_write`와 Write Authorization, `access_class=run_recording`으로 처리하는 `harness.record_run`, `access_class=artifact_registration`으로 처리하는 `harness.stage_artifact` 아티팩트 스테이징, `StagedArtifactHandle` 출처와 범위 검증을 통과한 스테이징된 아티팩트 승격, `existing_artifact` / `ArtifactRef` 영속 연결, `access_class=artifact_read`가 필요한 별도 아티팩트 본문 읽기, `EvidenceSummary`, `harness.close_task` 차단 사유 계산, 읽을 때 계산되는 읽기 전용 상태/Projection 출력, 등록된 접점에서 확인된 로컬 접점 접근, 협력형 보장 표시, 관련 역량 확인이 통과한 뒤의 탐지형 보장 표시에만 닫혀 있습니다.

Future-only material includes `captured_artifact`, native artifact capture, projection reconcile, persistent projection jobs, managed block drift repair, full Evidence Manifest, `qa_gate`, `verification_gate`, command/network/secret observation or pre-tool blocking, Question Queue, Assumption Register, and Discovery Brief as a persistent artifact. Route it through [Later Index](en/later/index.md) until an owner promotes it.

`captured_artifact`, 접점 자체 아티팩트 캡처, projection reconcile, 영속 Projection 작업, 관리 블록 불일치 복구, 전체 Evidence Manifest, `qa_gate`, `verification_gate`, 명령/네트워크/비밀값 관찰이나 도구 실행 전 차단, Question Queue, Assumption Register, 영속 아티팩트로서의 Discovery Brief는 이후 전용 자료입니다. 담당 문서가 승격하기 전까지 [이후 후보 색인](ko/later/index.md)으로 안내합니다.

Documentation checks are manual maintenance aids. Their `PASS`, `WARN`, and `FAIL` labels do not decide documentation acceptance, implementation readiness, runtime conformance, or permission to start server coding.

문서 점검은 수동 유지보수 보조 자료입니다. `PASS`, `WARN`, `FAIL` 라벨은 문서 수락, 구현 준비, 런타임 적합성, 서버 코딩 시작 허가를 결정하지 않습니다.

## Quality Rules / 품질 규칙

Keep route tables on the compact structure above. Keep review history, cleanup notes, and temporary migration plans out of active docs.

경로 표는 위의 현재 간결 구조만 가리켜야 합니다. 리뷰 이력, 정리 메모, 임시 마이그레이션 계획을 활성 문서에 넣지 않습니다.

Do not list profile-gated values as default active MVP values. Do not describe later candidates as active requirements. Do not make unsupported security claims about prevention, isolation, sandboxing, tamper-proof storage, or default tool blocking.

profile-gated 값을 기본 현재 MVP 값처럼 나열하지 않습니다. 이후 후보를 활성 요구사항처럼 설명하지 않습니다. 예방, 격리, 샌드박스, 변조 방지 저장소, 기본 도구 차단에 대한 근거 없는 보안 주장을 만들지 않습니다.

## Agent Context / 에이전트 맥락

Agents should keep a small current context, pull owner docs only when needed, and avoid duplicate injection. Do not load paired English/Korean docs for the same `doc_id` in one prompt unless the task is translation or semantic-parity review.

에이전트는 작은 현재 맥락을 유지하고 필요한 담당 문서만 불러와야 합니다. 에이전트 중복 주입 금지도 지켜야 합니다. 번역이나 의미 일치 검토가 필요한 작업이 아니라면 같은 `doc_id`의 영어/한국어 문서를 한 프롬프트에 함께 넣지 않습니다.
