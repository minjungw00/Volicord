# Translation Guide

Use this guide when editing English and Korean Harness documentation together. It is a living bilingual editing guide, not a history of the redesign and not a runtime conformance or implementation-readiness record.

## 1. Semantic parity, not line parity

English docs define the reference meaning for the bilingual documentation set. Korean docs preserve that meaning, but they should read like natural Korean technical documentation.

The goal is semantic parity, not sentence-by-sentence translation. Headings, paragraph breaks, and examples may differ when the Korean version is clearer and the same meaning, owner routing, active/later boundary, and exact identifiers remain intact.

When meaning changes, update English and Korean in the same batch. When Korean editing reveals an English meaning problem, fix both sides.

## 2. Exact identifiers to preserve

Preserve these exactly in both languages:

- file paths and anchors
- `doc_id` values
- API method names, tool names, and resource names
- schema names and schema fields
- enum values and status values
- error codes and validator IDs
- DDL, table names, column names, and storage identifiers
- code identifiers, literal markers, placeholder names, and code-like strings

Do not translate exact strings inside code blocks, schemas, API examples, file paths, or field lists. Localized display labels are rendering text, not canonical identifiers.

## 3. Natural Korean rule

Korean documentation should be natural Korean technical prose.

- Prefer short, clear sentences.
- Put the Korean concept first in user-facing prose.
- Add the exact English identifier only when precision, search, or owner alignment needs it.
- Avoid Korean sentences made mostly of English nouns with Korean particles attached.
- Keep exact identifiers exact even when the surrounding sentence is fully Korean.

Good Korean may rearrange the English sentence. It should not become a literal line-by-line copy.

## 4. User-facing terminology

For Korean user-facing prose, prefer these ordinary terms:

| Concept | Korean wording |
|---|---|
| authoring guide | 작성 가이드 |
| translation guide | 번역 가이드 |
| documentation checks | 문서 점검 |
| semantic parity | 의미 일치 |
| not line parity | 줄 단위 번역 아님 |
| owner document | 담당 문서 |
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

Use `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, and `task_events` only when the exact Harness label helps the reader follow a blocker, record, API, template, or owner link.

## 5. Internal identifier terminology

Internal identifiers stay exact. Explain them in Korean prose when useful, but do not translate the identifier itself.

Common examples:

| Identifier | Korean explanation when needed |
|---|---|
| `user_judgment` | 사용자 판단 기록 |
| `UserJudgment` | 사용자 판단 schema |
| `judgment_kind` | 판단 종류 field |
| `product_decision` | 제품 판단 value |
| `technical_decision` | 기술 판단 value |
| `scope_decision` | 범위 판단 value |
| `sensitive_approval` | 민감 동작 승인 value |
| `qa_waiver` | QA 면제 판단 value |
| `verification_risk_acceptance` | 검증 위험 수락 value |
| `final_acceptance` | 최종 수락 value |
| `residual_risk_acceptance` | 잔여 위험 수락 value |
| `presentation` | 표시 형식 field |
| `display_label` | 표시 라벨 field; rendered display text, not a schema value |
| `prepare_write` | 쓰기 전 범위 확인을 다루는 API/action identifier |
| `record_run` | 실행/확인 기록 API/action identifier |
| `close_task` | 닫기 확인 API/action identifier |
| `ArtifactRef` | 아티팩트 참조 schema |
| `ProjectionKind` | projection 종류 identifier |

Korean labels such as `제품 판단`, `기술 판단`, and `범위 판단` may appear in prose or rendered examples. They must not replace canonical values such as `product_decision`, `technical_decision`, `scope_decision`, or `judgment_kind`.

## 6. Bilingual review checklist

- [ ] The Korean page preserves the same meaning as the English page.
- [ ] The paired files keep the same active file path, reader purpose, semantic section coverage, owner links, and active/later boundary.
- [ ] Korean prose reads naturally to a Korean technical reader.
- [ ] Exact identifiers, paths, API/schema names, enum values, error codes, table names, and validator IDs are preserved.
- [ ] Korean display labels are treated as localized display text, not schema identifiers.
- [ ] User-facing Korean uses natural Korean before Harness labels when both are needed.
- [ ] Non-owner duplicate contracts are summarized with owner links instead of translated as full contract copies.
- [ ] Link changes were made in both languages in the same batch.
- [ ] Translation review was not described as runtime state, evidence, QA, acceptance, close readiness, runtime conformance, or implementation readiness.
