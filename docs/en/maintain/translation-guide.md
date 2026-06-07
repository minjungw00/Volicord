# Translation Guide

Use this guide when editing English and Korean Harness documentation together. It is a living bilingual editing guide, not a history of the redesign and not a runtime conformance or implementation-readiness record.

## 1. Semantic Parity, Not Line Parity

English and Korean docs are both active. Neither language is an archive, appendix, or translation-only copy. The paired docs share one meaning, and the edit should leave both languages aligned.

The goal is semantic parity, not sentence-by-sentence translation. Headings, paragraph breaks, and examples may differ when the Korean version is clearer and the same meaning, compact owner routing, active/later boundary, and exact identifiers remain intact.

When meaning changes, update English and Korean in the same batch. When Korean editing reveals an English meaning problem, fix both sides.

## 2. Route And Context Parity

Route tables in README and Maintain docs should point only to the compact structure and [doc-index.yaml](../../doc-index.yaml). Use [Reference Index](../reference/README.md) to choose exact contract owners instead of adding deep owner paths to route tables.

Agents should load only one language for the same `doc_id` during normal work. Load both languages only when the task is translation or semantic-parity review and the comparison is necessary. Keep the prompt focused on the current task, owner section, scope/non-goals, pending user judgments, blockers, next safe actions, evidence gaps, close blockers, residual risk, guarantee level, and source freshness.

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

## 4. Natural Korean Rule

Korean documentation should be natural Korean technical prose.

- Prefer short, clear sentences.
- Put the Korean concept first in user-facing prose.
- Add the exact English identifier only when precision, search, or owner alignment needs it.
- Do not leave English noun phrases in Korean prose unless they are exact identifiers or intentional Harness labels.
- Avoid Korean sentences made mostly of English nouns with Korean particles attached.
- Keep exact identifiers exact even when the surrounding sentence is fully Korean.

Good Korean may rearrange the English sentence, split or combine clauses, and change paragraph rhythm for readability. It should not become a literal line-by-line copy.

## 5. User-Facing Terminology

For Korean user-facing prose, prefer these ordinary terms:

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
| close readiness | 닫기 가능 여부 or 닫기 준비 상태 |
| close blocker | 닫기 차단 사유 |
| next safe action | 다음 안전한 행동 |
| derived view or projection in user prose | 상태 보기, 요약, or 상태 카드 |
| pre-write scope check | 쓰기 전 범위 확인 |
| sensitive-action approval | 민감 동작 승인 |

Use `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, and `task_events` only when the exact Harness label helps the reader follow a blocker, record, API, template, or owner route.

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
| `qa_waiver` | QA 면제 판단 값 |
| `verification_risk_acceptance` | 검증 위험 수락 값 |
| `final_acceptance` | 최종 수락 값 |
| `residual_risk_acceptance` | 잔여 위험 수락 값 |
| `presentation` | 표시 형식 필드 |
| `display_label` | 표시 라벨 필드; rendered display text, not a schema value |
| `prepare_write` | 쓰기 전 범위 확인을 다루는 API/action identifier |
| `record_run` | 실행/확인 기록 API/action identifier |
| `close_task` | 닫기 확인 API/action identifier |
| `ArtifactRef` | 아티팩트 참조 스키마 |
| `ProjectionKind` | Projection 종류 식별자 |

Korean labels such as `제품 판단`, `기술 판단`, and `범위 판단` may appear in prose or rendered examples. They must not replace canonical values such as `product_decision`, `technical_decision`, `scope_decision`, or `judgment_kind`.

## 7. Boundary Vocabulary

Do not translate active/later, security, or judgment boundaries into stronger claims.

- "profile-gated" means available only under the named profile, capability, connector mode, or future configuration. It is not a default active MVP value.
- "later candidate" means deferred material. It is not an active requirement unless the owner promotes it with scope and proof expectations.
- Cooperative or detective security wording must not become preventive, isolated, sandboxed, tamper-proof, or default tool-blocking wording in Korean.
- Broad approval, final acceptance, QA waiver, verification-risk acceptance, and residual-risk acceptance remain distinct judgment routes.

## 8. Bilingual Review Checklist

- [ ] English and Korean pages preserve the same meaning.
- [ ] The paired files keep the same active file path, reader purpose, semantic section coverage, owner routing, and active/later boundary.
- [ ] Route tables point only to the compact structure and `docs/doc-index.yaml`.
- [ ] Korean prose reads naturally to a Korean technical reader.
- [ ] Exact identifiers, paths, API/schema names, enum values, error codes, table names, validator IDs, template names, and `doc_id` values are preserved.
- [ ] Korean display labels are treated as localized display text, not schema identifiers.
- [ ] User-facing Korean uses natural Korean before Harness labels when both are needed.
- [ ] English noun phrases are not left in Korean prose unless they are exact identifiers or intentional Harness labels.
- [ ] Non-owner duplicate contracts are summarized with compact owner routes instead of translated as full contract copies.
- [ ] Link changes were made in both languages in the same batch.
- [ ] Translation review was not described as runtime state, evidence, QA, acceptance, close readiness, runtime conformance, or implementation readiness.
