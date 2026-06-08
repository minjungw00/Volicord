# 참조 색인

참조 문서는 정확한 하네스 계획 계약의 담당 문서를 찾을 때 사용합니다. 향후 하네스 서버 검토를 위한 색인이며, 처음 읽는 튜토리얼이나 구현 계획이 아닙니다.

이 문서들은 현재 문서 검토 중인 향후 하네스 서버 계약을 설명합니다. 지금 이 저장소에 서버/런타임, Harness Runtime Home, 생성된 Projection 시스템, 적합성 실행기, 런타임 데이터, 구현 완료 동작이 있다는 뜻은 아닙니다.

## 읽기 규칙

- 참조 문서 전체를 기본으로 읽지 않기: 지금 질문에 맞는 담당 문서 하나를 고르고, 그 담당 문서가 더 엄격한 세부사항을 위임할 때만 링크를 따라갑니다.
- 같은 담당 문서의 영어/한국어 대응 문서를 같은 프롬프트에 함께 넣지 않습니다. 작업 언어를 하나 고르고, 이중 언어 비교는 별도의 작은 확인으로만 합니다.
- 이 README는 색인으로 유지합니다. 계약 세부사항을 여기로 복사하지 않습니다.
- active/later 경계는 활성 담당 문서와 [이후 후보 색인](../later/index.md)에 둡니다.

## 현재 MVP 경계

현재 MVP 경계는 [MVP 계획](../build/mvp-plan.md)에 닫힌 목록으로 정해져 있습니다. 여기에는 평소 말 입력과 Task 생성, `harness.update_scope`, 사용자 판단 기록, 민감 동작 승인 기록, 경로 수준 `harness.prepare_write`와 Write Authorization, `harness.record_run`, `harness.stage_artifact`를 통한 스테이징된 아티팩트 등록, 간결한 `EvidenceSummary`, `harness.close_task` 차단 사유 계산, 읽을 때 계산되는 읽기 전용 상태/Projection 출력, 등록된 접점에서 확인된 로컬 접점 접근, 협력형 보장 표시, 관련 역량 확인이 실제로 통과한 뒤의 탐지형 보장 표시만 포함됩니다.

그 밖의 항목은 담당 참조 문서가 범위, 대체 동작, 증명 기대치를 함께 명시적으로 승격하기 전까지 이후 전용입니다. 여기에는 `captured_artifact`, 접점 자체 아티팩트 캡처, projection reconcile, 영속 Projection 작업, 관리 블록 불일치 복구, 전체 Evidence Manifest, `qa_gate`, `verification_gate`, 명령 실행 관찰, 네트워크 관찰, 비밀값 접근 관찰, 명령/네트워크/비밀값 도구 실행 전 차단, Question Queue, Assumption Register, 영속 아티팩트로서의 Discovery Brief가 포함됩니다. 이후 전용 이름과 승격 경계는 [이후 후보 색인](../later/index.md)에서 봅니다.

## 담당 문서 라우팅

아래 표는 에이전트와 구현자가 현재 존재하는 간결한 담당 문서 하나로 이동하도록 안내합니다.

| 계약 영역 | 담당 문서 |
|---|---|
| 현재 MVP 경계, 제외되는 이후 자료, 구현 순서, 유지보수자 준비 결정 | [MVP 계획](../build/mvp-plan.md) |
| Core 권한, 작업 생명주기, `ShapingReadiness` 의미, 사용자 소유 제품/기술/범위/민감 동작/최종/잔여 위험/취소 판단 경계, 최종 수락/잔여 위험 수락 대체 불가, 활성 gate 의미, `CompletionPolicy` 닫기 영향, `EvidenceSummary` 닫기 영향, `close_task` 차단 사유 행렬, waiver, 잔여 위험 | [core-model.md](core-model.md) |
| 활성 공개 API 메서드별 동작, 확인된 로컬 접점 요청 조건, `harness.update_scope`, `harness.prepare_write`가 Write Authorization에 미치는 효과, `harness.stage_artifact`, `harness.record_run`, `harness.close_task` 메서드 동작 | [api/mvp-api.md](api/mvp-api.md) |
| 정확한 활성 메서드 이름 집합, `ToolEnvelope.expected_state_version`, `LocalSurfaceRegistration`, `VerifiedSurfaceContext`, `StagedArtifactHandle`, `ArtifactInput`, `CompletionPolicy`, `EvidenceSummary`, `SensitiveActionScope`, 제품 파일 쓰기 범위인 `AuthorizedAttemptScope`, 닫기 차단 사유 스키마, `ShapingReadiness` 필드, 활성 enum/값 집합, 표시 라벨 경계, `GuaranteeDisplay.level` 값 | [api/schema-core.md](api/schema-core.md) |
| 공개 오류, 오류 우선순위, 로컬 접점 오류, `STATE_VERSION_CONFLICT`, 차단/드라이런 응답 의미, `close_task` 차단 사유의 공개 오류 매핑 | [api/errors.md](api/errors.md) |
| 저장소, DDL, 단일 공개 프로젝트 전체 상태 시계인 `project_state.state_version`, `surfaces`, `write_authorizations`, 스테이징된 아티팩트 저장소, 지속되는 증거 요약 행, 멱등성, 마이그레이션 | [storage.md](storage.md) |
| 런타임 공간, 변경 권한, Product Repository / Harness Server / Runtime Home 분리, 비격리 / OS 샌드박싱 비보장 | [runtime-boundaries.md](runtime-boundaries.md) |
| 보안 보장, 협력형/탐지형 표현, 역량 확인에 기반한 탐지형 보장 조건, OS 샌드박싱 비보장, 민감 동작 승인과 제품 파일 쓰기 범위의 분리, profile-gated `preventive` / `isolated` 표시 라벨 | [security.md](security.md) |
| 에이전트 맥락, 커넥터 동작, `capability_profile`, 에이전트 패킷 안의 확인된 접점 맥락, 역량 확인에서 나온 탐지형 표시 조건, 대체 동작, 접점별 안내, 하나의 `doc_id`에는 한 언어만 싣는 검색 규칙 | [agent-integration.md](agent-integration.md) |
| 읽기 전용 파생 표시인 Projection/상태 카드, Projection 권한 경계, 렌더링된 라벨, 활성 템플릿, 최신성 표현, projection reconcile, 영속 Projection 작업, 관리 블록 불일치 복구가 이후 전용이라는 경계 | [projection-and-templates.md](projection-and-templates.md) |
| 적합성 모델, 향후 fixture 형식, 주장 권한, 활성 스모크 대상 예시, 역량 정직성 검증 주장, 실행 가능한 모음이 아니라는 경계 | [conformance.md](conformance.md) |
| 좁은 설계 품질 라우팅, 닫기 영향, waiver 경계, validator ID 경계 | [design-quality.md](design-quality.md) |
| 공식 용어 | [glossary.md](glossary.md) |
| `captured_artifact`, 접점 자체 아티팩트 캡처, projection reconcile, 영속 Projection 작업, 관리 블록 불일치 복구, 전체 Evidence Manifest, `qa_gate`, `verification_gate`, 전체 형식 판단 표시, 향후 fixture 계열, 향후 운영을 포함한 이후 전용 개념과 승격 경계 | [../later/index.md](../later/index.md) |
| 문서 작성 규칙, 담당 문서 경계 위생, active/later 점검, 한국어 품질, 의미 일치, 문서 점검, 번역 규칙 | [작성 가이드](../maintain/authoring-guide.md), [번역 가이드](../maintain/translation-guide.md), [문서 점검](../maintain/checks.md) |

## 중복 주입 금지

담당 문서가 아닌 문서는 독자에게 보이는 결과만 요약하고 담당 문서로 연결합니다. 스키마, DDL, enum 표, 전이 표, 상태 보기 템플릿 본문, fixture assertion, 공개 오류 우선순위, 보안 보장, 용어 정의를 붙여 넣지 않습니다.

문서 작성, 번역, 검토, 링크 정리, 담당 문서 경계 불일치, 문서 유지보수 점검은 [작성 가이드](../maintain/authoring-guide.md), [번역 가이드](../maintain/translation-guide.md), [문서 점검](../maintain/checks.md)이 담당합니다. 구현 순서와 유지보수자 상태 결정은 [MVP 계획](../build/mvp-plan.md)이 담당합니다.
