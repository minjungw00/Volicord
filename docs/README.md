# Harness Documentation / 하네스 문서

This directory contains the active bilingual documentation set for a future local Harness Server. The repository is documentation-only today. It is not a running Harness instance, not the user's Product Repository, and not a Harness Runtime Home.

이 디렉터리는 향후 로컬 하네스 서버를 위한 현재 한영 문서 세트를 담고 있습니다. 이 저장소는 현재 문서 전용입니다. 실행 중인 하네스 인스턴스도, 사용자의 제품 저장소도, 하네스 런타임 홈도 아닙니다.

Harness documentation is source material for planning. It is not runtime state, generated projections, evidence, QA, final acceptance, residual-risk, close records, server code, or product code.

하네스 문서는 계획을 위한 원천 자료입니다. 런타임 상태, 생성된 상태 보기, 증거, QA, 최종 수락, 잔여 위험, 닫기 기록, 서버 코드, 제품 코드가 아닙니다.

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
| Reference owners / Reference 담당 문서 | [Reference Index](en/reference/README.md) | [Reference 색인](ko/reference/README.md) |
| Later material / 이후 자료 | [Later Index](en/later/index.md) | [Later 색인](ko/later/index.md) |
| Authoring guide / 작성 가이드 | [Authoring Guide](en/maintain/authoring-guide.md) | [작성 가이드](ko/maintain/authoring-guide.md) |
| Translation guide / 번역 가이드 | [Translation Guide](en/maintain/translation-guide.md) | [번역 가이드](ko/maintain/translation-guide.md) |
| Checks / 문서 점검 | [Checks](en/maintain/checks.md) | [문서 점검](ko/maintain/checks.md) |

## Reader Guidance / 독자 안내

Use `start.md` for the first model, `use/*` for user and agent behavior, `build/mvp-plan.md` for current MVP planning and implementation-readiness decisions, `reference/README.md` for exact contract owners, `later/index.md` for deferred material, and `maintain/*` for documentation work.

첫 이해 모델은 `start.md`에서 봅니다. 사용자와 에이전트 동작은 `use/*`, 현재 MVP 계획과 구현 준비 결정은 `build/mvp-plan.md`, 정확한 계약의 담당 문서는 `reference/README.md`, 이후 자료는 `later/index.md`, 문서 작업 규칙은 `maintain/*`에서 봅니다.

Documentation checks are manual maintenance aids. Their `PASS`, `WARN`, and `FAIL` labels do not decide documentation acceptance, implementation readiness, runtime conformance, or permission to start server coding.

문서 점검은 수동 유지보수 보조 자료입니다. `PASS`, `WARN`, `FAIL` 라벨은 문서 수락, 구현 준비, 런타임 적합성, 서버 코딩 시작 허가를 결정하지 않습니다.

## Agent Context / 에이전트 맥락

Agents should keep a small current context, pull owner docs only when needed, and avoid duplicate injection. Do not load paired English/Korean docs for the same `doc_id` in one prompt unless the task is translation or semantic-parity review.

에이전트는 작은 현재 맥락을 유지하고 필요한 담당 문서만 불러와야 합니다. 에이전트 중복 주입 금지도 지켜야 합니다. 번역이나 의미 일치 검토가 필요한 작업이 아니라면 같은 `doc_id`의 영어/한국어 문서를 한 프롬프트에 함께 넣지 않습니다.
