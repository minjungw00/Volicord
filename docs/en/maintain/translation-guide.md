# Translation Guide

Use this guide when editing English and Korean Harness documentation together. It is a living bilingual editing guide, not a history of the redesign and not a runtime conformance or implementation-readiness record.

## 1. Semantic Parity, Not Line Parity

English and Korean docs are both active. Neither language is an archive, appendix, or translation-only copy. The paired docs share one meaning, and the edit should leave both languages aligned.

The goal is semantic parity, not sentence-by-sentence translation. Headings, paragraph breaks, and examples may differ when the Korean version is clearer and the same meaning, compact owner routing, active/later boundary, and exact identifiers remain intact.

When meaning changes, update English and Korean in the same batch. When Korean editing reveals an English meaning problem, fix both sides.

## 2. Route And Context Parity

Route tables in README and Maintain docs should point only to the compact structure and [doc-index.yaml](../../doc-index.yaml). Use [Reference Index](../reference/README.md) to choose exact contract owners instead of adding deep owner paths to route tables.

Agents should load only one language for the same `doc_id` during normal work. Load both languages only when the task is translation or semantic-parity review and the comparison is necessary. Keep the prompt focused on the current task, owner section, scope/non-goals, pending user judgments, blockers, next safe actions, evidence gaps, reasons close is blocked, residual risk, what Harness can verify, and source freshness.

## 3. Exact Identifiers To Preserve

Preserve these exactly in both languages:

- file paths and anchors
- `doc_id` values
- API method names, tool names, and resource names
- schema names and schema fields
- enum values and status values
- error codes and validator IDs
- DDL, table names, column names, and storage identifiers
- template names
- code identifiers, literal markers, placeholder names, and code-like strings

Do not translate exact strings inside code blocks, schemas, API examples, file paths, or field lists. Localized display labels are rendering text, not canonical identifiers.

Exact identifiers are canonical strings used by contracts, routes, schemas, storage, APIs, templates, or search. Explanatory prose is the surrounding reader-facing explanation. Copy exact identifiers unchanged, but translate or rewrite explanatory prose so Korean reads naturally.

In Korean user-facing prose, raw enum names and schema values are not display labels unless the raw value itself is the subject. Use a natural Korean label first, and add the exact English value only when the reader needs contract precision or searchability.

## 4. Natural Korean Rule

Korean documentation should be natural Korean technical prose.

- Prefer short, clear sentences.
- Put the Korean concept first in user-facing prose.
- Add the exact English identifier only when precision, search, or owner alignment needs it.
- Use the same Korean wording for the same concept across files. When a concept needs a new Korean term, update the glossary or this guide before spreading variants.
- Do not leave English noun phrases in Korean prose merely because the English source used them. Unless the phrase is an exact identifier or intentional Harness label, translate the concept into Korean.
- Prefer natural Korean display labels over raw enum or status values in user-facing text, unless the exact raw value is being explained.
- In user-facing templates, render schema and enum concepts as natural display text unless the exact contract value is the subject.
- Do not compress English negative coordination in Korean in a way that reverses meaning. If the condition means "not visible, or not accepted when required," make both negative requirements explicit and do not drop the first negative requirement.
- Avoid Korean sentences made mostly of English nouns with Korean particles attached.
- Keep exact identifiers exact even when the surrounding sentence is fully Korean.

Good Korean may rearrange the English sentence, split or combine clauses, and change paragraph rhythm for readability. It should not become a literal line-by-line copy.

## 5. User-Facing Terminology

For Korean user-facing prose, prefer these ordinary terms:

Use one Korean expression consistently for one concept. Schema fields, method names, enum values, and code identifiers remain exact English in schemas and code-like examples; the Korean wording below is for prose and rendered labels.

| Concept | Korean wording |
|---|---|
| bilingual active maintenance | 한영 문서 동시 유지 |
| authoring guide | 작성 가이드 |
| translation guide | 번역 가이드 |
| documentation checks | 문서 점검 |
| semantic parity | 의미 일치 |
| not line parity | 줄 단위 번역 아님 |
| no duplicate agent injection | 에이전트 중복 주입 금지 |
| owner document | 담당 문서 |
| current MVP | 현재 MVP |
| profile-gated value | profile-gated 값 |
| active/later boundary | active/later 경계 |
| stale content deletion rule | 오래된 내용 삭제 규칙 |
| work | 작업 |
| scope | 범위 |
| out of scope | 범위 밖 |
| judgment | 판단 |
| user-owned judgment | 사용자 소유 판단 |
| judgment request | 판단 요청 |
| evidence | 증거 |
| detailed evidence list | 증거 목록 |
| check | 확인 |
| verification | 검증 |
| Manual QA | 수동 QA |
| final acceptance | 최종 수락 |
| residual risk | 잔여 위험 |
| residual risk acceptance | 잔여 위험 수락 |
| close readiness | 닫기 가능 여부 또는 닫기 준비 상태 |
| close blocker in user-facing display | 닫기를 막는 이유 |
| `lifecycle_phase` in user-facing display | 현재 단계 |
| Autonomy Boundary in user-facing display | 에이전트가 스스로 판단해도 되는 범위 |
| `guarantee_level` in user-facing display | 하네스가 확인할 수 있는 수준 |
| Change Unit in user-facing display | 이번에 바꿀 가장 작은 작업 단위 |
| EvidenceSummary in user-facing display | 확인 근거 요약 또는 확인한 것 |
| next safe action | 다음 안전한 행동 |
| derived view or projection in user prose | 상태 보기, 요약, 또는 상태 카드 |
| pre-write scope check | 쓰기 전 범위 확인 |
| sensitive-action approval | 민감 동작 승인 |
| verified surface context | 확인된 접점 맥락 |
| local surface registration | 로컬 접점 등록 |
| sensitive action scope | 민감 동작 범위 |
| staged artifact handle | 스테이징된 아티팩트 핸들 |
| completion policy | 완료 정책 |
| shaping readiness | 구체화 준비 상태 |
| project-wide state_version | 프로젝트 전체 `state_version` |
| task-scoped state_version | Task 범위 `state_version` |
| public conflict clock | 공개 충돌 시계 |
| artifact input | 아티팩트 입력 |
| evidence coverage item | 증거 범위 항목 |
| cooperative guarantee | 협력형 보장 |
| detective guarantee | 탐지형 보장 |
| surface identifier in user prose | 접점 식별자. 권한 증거처럼 쓰지 않음 |
| Discovery Brief as a persistent artifact | 영속 아티팩트로서의 Discovery Brief |
| Question Queue | 질문 큐 |
| Assumption Register | 가정 기록부 |
| persistent projection job | 지속 저장되는 상태 보기 작업 |
| projection reconcile | 상태 보기 조정 |
| managed block drift repair | 관리 블록 불일치 복구 |
| native artifact capture | 접점 자체 아티팩트 캡처 |
| task-scoped state clock | Task 범위 상태 시계 |
| `captured_artifact` | `captured_artifact` 값 이름. 산문에서는 이후 전용 캡처된 아티팩트 값이라고 설명 |

Use `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, and `task_events` only when the exact Harness label helps the reader follow an internal blocker, record, API, template, or owner route. In user-facing cards and examples, prefer the plain display wording above.

## 6. Internal Identifier Terminology

Internal identifiers stay exact. Explain them in Korean prose when useful, but do not translate the identifier itself.

Common examples:

| Identifier | Korean explanation when needed |
|---|---|
| `user_judgment` | 사용자 판단 기록 |
| `UserJudgment` | 사용자 판단 스키마 |
| `judgment_kind` | 판단 종류 필드 |
| `product_decision` | 제품 판단 값 |
| `technical_decision` | 기술 판단 값 |
| `scope_decision` | 범위 판단 값 |
| `sensitive_approval` | 민감 동작 승인 값 |
| `qa_waiver` | later/reserved QA 면제 판단 값 |
| `verification_risk_acceptance` | later/reserved 검증 위험 수락 값 |
| `final_acceptance` | 최종 수락 값 |
| `residual_risk_acceptance` | 잔여 위험 수락 값 |
| `presentation` | 표시 형식 필드 |
| `display_label` | 표시 라벨 필드; rendered display text, not a schema value |
| `prepare_write` | 쓰기 전 범위 확인을 다루는 API/action identifier |
| `record_run` | 실행/확인 기록 API/action identifier |
| `close_task` | 닫기 확인 API/action identifier |
| `ArtifactRef` | 아티팩트 참조 스키마 |
| `ArtifactInput` | 아티팩트 입력 스키마 |
| `StagedArtifactHandle` | 스테이징된 아티팩트 핸들 |
| `EvidenceCoverageItem` | 증거 범위 항목 |
| `CompletionPolicy` | 완료 정책 스키마 |
| `ShapingReadiness` | 구체화 준비 상태 파생 보기 |
| `LocalSurfaceRegistration` | 로컬 접점 등록 사실 |
| `VerifiedSurfaceContext` | 확인된 접점 맥락 |
| `SensitiveActionScope` | 민감 동작 범위 스키마 |
| `AuthorizedAttemptScope` | 제품 파일 쓰기 시도 범위 스키마 |
| `surface_id` | 접점 식별자 값. 권한, 접근, 바인딩, 역량의 증거가 아님 |
| `state_version` | 상태 버전 필드명. 공개 충돌 기준은 담당 문서가 정함 |
| `project_state.state_version` | 프로젝트 전체 상태 시계 |
| `tasks.state_version` | Task 범위 상태 시계. 현재 MVP 공개 충돌 기준으로 쓰지 않음 |
| `ProjectionKind` | 상태 보기 종류 식별자 |
| `detective` | 보장 수준 값. 산문에서는 탐지형 보장이라고 설명 |
| `EvidenceSummary` | 확인 근거 요약 스키마 또는 표시 개념 |

Korean labels such as `제품 판단`, `기술 판단`, and `범위 판단` may appear in prose or rendered examples. They must not replace canonical values such as `product_decision`, `technical_decision`, `scope_decision`, or `judgment_kind`.

## 7. Boundary Vocabulary

Do not translate active/later, security, or judgment boundaries into stronger claims.

- "profile-gated" means available only under the named profile, capability, connector mode, or future configuration. It is not a default active MVP value.
- "later candidate" means deferred material. It is not an active requirement unless the owner promotes it with scope and proof expectations.
- Cooperative or detective security wording must not become preventive, isolated, sandboxed, tamper-proof, or default tool-blocking wording in Korean.
- Broad approval, final acceptance, and residual risk acceptance remain distinct. Later/reserved QA waiver or verification-risk acceptance terminology belongs to the Later Candidate Index until promoted and is not part of the active MVP judgment-kind list.
- `surface_id` is an identifier, not proof of authority, local access, binding, or capability. Do not translate it into Korean wording that sounds like permission or verification has already succeeded.
- `captured_artifact` remains a later-only captured-artifact value until promoted. Do not describe it in Korean as an active artifact input path.
- `sensitive_approval` / `SensitiveActionScope` is separate from product-file `AuthorizedAttemptScope` and Write Authorization.
- `detective` requires passed capability verification for the covered observable scope. Without that passed check, use cooperative or capability-limited wording.
- Active MVP public conflict wording uses project-wide `project_state.state_version` unless an owner promotes another clock. Do not expose both task-scoped and project-scoped `state_version` as public conflict clocks.
- Projection reconcile and `reconcile` stay later-only unless promoted, and must not become a Core state mutation path through translation.
- Final acceptance and residual risk acceptance do not fill missing required evidence.
- User-facing templates should not expose internal enum or schema terms such as `EvidenceSummary`, `CloseReadinessBlocker.category`, `judgment_kind`, or `guarantee_level` unless the contract value itself is being explained.

## 8. Bilingual Review Checklist

- [ ] English and Korean pages preserve the same meaning.
- [ ] The paired files keep the same active file path, reader purpose, semantic section coverage, owner routing, and active/later boundary.
- [ ] Route tables point only to the compact structure and `docs/doc-index.yaml`.
- [ ] Korean prose reads naturally to a Korean technical reader.
- [ ] Exact identifiers, paths, API/schema names, enum values, error codes, table names, validator IDs, template names, and `doc_id` values are preserved.
- [ ] Exact identifiers were copied unchanged, while explanatory prose was translated or rewritten as natural Korean.
- [ ] Korean display labels are treated as localized display text, not schema identifiers.
- [ ] User-facing Korean uses natural Korean before Harness labels when both are needed.
- [ ] English noun phrases are not left in Korean prose unless they are exact identifiers or intentional Harness labels.
- [ ] Korean prose does not preserve English sentence order or literal translation where natural Korean explanation is required.
- [ ] Korean translations do not compress negative coordination in a way that reverses meaning.
- [ ] Non-owner duplicate contracts are summarized with compact owner routes instead of translated as full contract copies.
- [ ] Later-only concepts remain marked later-only, and reference docs do not imply that later-only features are active MVP requirements.
- [ ] User-facing templates use natural display wording instead of unnecessary internal enum or schema terms.
- [ ] Link changes were made in both languages in the same batch.
- [ ] Translation review was not described as runtime state, evidence, QA, acceptance, close readiness, runtime conformance, or implementation readiness.
