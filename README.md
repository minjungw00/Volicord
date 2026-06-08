# Harness Project / 하네스 프로젝트

Harness is a planned local work-authority server for AI-assisted product work. Its planned authority is over Harness records and state transitions for scope, user-owned judgment, evidence, verification expectations, final acceptance, close readiness, and residual risk. It is not a prompt pack, operating-system permission control, arbitrary-tool sandboxing, tamper-proof storage, default pre-tool blocking, or security isolation.

하네스는 AI 지원 제품 작업을 위한 향후 로컬 작업 권한 서버입니다. 하네스가 다루려는 권한은 범위, 사용자 소유 판단, 증거, 검증 기대, 최종 수락, 닫기 가능 여부, 잔여 위험에 대한 하네스 기록과 상태 전이입니다. 프롬프트 묶음, 운영체제 권한 제어, 임의 도구 샌드박스, 변조 방지 저장소, 기본 도구 실행 전 차단, 보안 격리가 아닙니다.

## Repository State / 저장소 상태

This repository is documentation-only today. It contains bilingual planning source material for a future local Harness Server. It has no server/runtime implementation, product implementation code, runtime state, generated projections, generated operational artifacts, executable fixtures, or conformance runner.

이 저장소는 현재 문서 전용입니다. 향후 로컬 하네스 서버를 위한 한영 문서 동시 유지 계획 자료를 담고 있습니다. 서버/런타임 구현, 제품 구현 코드, 런타임 상태, 생성된 상태 보기, 생성된 운영 산출물, 실행 가능한 fixture, 적합성 실행기는 없습니다.

This repository is not the user's Product Repository and not a Harness Runtime Home. Documentation acceptance, when it happens, is a maintainer documentation milestone only. Server/runtime implementation still requires a separate readiness decision in the MVP plan.

이 저장소는 사용자의 제품 저장소도, 하네스 런타임 홈도 아닙니다. 문서 수락이 이루어져도 유지보수자의 문서 이정표일 뿐입니다. 서버/런타임 구현에는 MVP 계획의 별도 준비 결정이 필요합니다.

## Active MVP Boundary / 현재 MVP 경계

The active MVP is closed to plain-language intake and Task creation, `harness.update_scope`, user judgment and sensitive approval recording, path-level `harness.prepare_write` and Write Authorization, `harness.record_run`, staged artifact registration through `harness.stage_artifact`, `EvidenceSummary`, `harness.close_task` blocker calculation, read-time read-only status/projection output, verified local surface access through a registered surface, cooperative guarantees, and detective guarantees only after the relevant capability check has passed.

현재 MVP는 평소 말 입력과 Task 생성, `harness.update_scope`, 사용자 판단과 민감 동작 승인 기록, 경로 수준 `harness.prepare_write`와 Write Authorization, `harness.record_run`, `harness.stage_artifact`를 통한 스테이징된 아티팩트 등록, `EvidenceSummary`, `harness.close_task` 차단 사유 계산, 읽을 때 계산되는 읽기 전용 상태/Projection 출력, 등록된 접점에서 확인된 로컬 접점 접근, 협력형 보장, 관련 역량 확인이 통과한 뒤의 탐지형 보장에만 닫혀 있습니다.

Later-only material remains in [Later Index](docs/en/later/index.md) / [이후 후보 색인](docs/ko/later/index.md): `captured_artifact`, native artifact capture, projection reconcile, persistent projection jobs, managed block drift repair, full Evidence Manifest, `qa_gate` / `verification_gate`, command/network/secret observation or pre-tool blocking, Question Queue, Assumption Register, and Discovery Brief as a persistent artifact.

이후 전용 자료는 [Later Index](docs/en/later/index.md) / [이후 후보 색인](docs/ko/later/index.md)에 남아 있습니다. 여기에는 `captured_artifact`, 접점 자체 아티팩트 캡처, projection reconcile, 영속 Projection 작업, 관리 블록 불일치 복구, 전체 Evidence Manifest, `qa_gate` / `verification_gate`, 명령/네트워크/비밀값 관찰이나 도구 실행 전 차단, Question Queue, Assumption Register, 영속 아티팩트로서의 Discovery Brief가 포함됩니다.

## Current Routes / 현재 경로

Start at [docs/README.md](docs/README.md) to choose a language. English and Korean docs are both active, must keep semantic parity, and do not require line-by-line translation.

언어 선택은 [docs/README.md](docs/README.md)에서 시작합니다. 영어와 한국어 문서는 모두 활성 문서이며 의미 일치를 유지해야 합니다. 줄 단위 번역은 요구하지 않습니다.

| Need / 필요 | English | 한국어 |
|---|---|---|
| Start / 시작 | [docs/en/start.md](docs/en/start.md) | [docs/ko/start.md](docs/ko/start.md) |
| User work / 사용자 작업 | [docs/en/use/user-guide.md](docs/en/use/user-guide.md) | [docs/ko/use/user-guide.md](docs/ko/use/user-guide.md) |
| Agent behavior / 에이전트 동작 | [docs/en/use/agent-guide.md](docs/en/use/agent-guide.md) | [docs/ko/use/agent-guide.md](docs/ko/use/agent-guide.md) |
| Judgment examples / 판단 예시 | [docs/en/use/judgment-examples.md](docs/en/use/judgment-examples.md) | [docs/ko/use/judgment-examples.md](docs/ko/use/judgment-examples.md) |
| Current MVP plan / 현재 MVP 계획 | [docs/en/build/mvp-plan.md](docs/en/build/mvp-plan.md) | [docs/ko/build/mvp-plan.md](docs/ko/build/mvp-plan.md) |
| Contract owner index / 계약 담당 문서 색인 | [docs/en/reference/README.md](docs/en/reference/README.md) | [docs/ko/reference/README.md](docs/ko/reference/README.md) |
| Later candidates / 이후 후보 | [docs/en/later/index.md](docs/en/later/index.md) | [docs/ko/later/index.md](docs/ko/later/index.md) |
| Authoring rules / 작성 규칙 | [docs/en/maintain/authoring-guide.md](docs/en/maintain/authoring-guide.md) | [docs/ko/maintain/authoring-guide.md](docs/ko/maintain/authoring-guide.md) |
| Translation rules / 번역 규칙 | [docs/en/maintain/translation-guide.md](docs/en/maintain/translation-guide.md) | [docs/ko/maintain/translation-guide.md](docs/ko/maintain/translation-guide.md) |
| Documentation checks / 문서 점검 | [docs/en/maintain/checks.md](docs/en/maintain/checks.md) | [docs/ko/maintain/checks.md](docs/ko/maintain/checks.md) |
| Route index / 경로 색인 | [docs/doc-index.yaml](docs/doc-index.yaml) | [docs/doc-index.yaml](docs/doc-index.yaml) |

## Quality Rules / 품질 규칙

Active routes stay in the compact structure above. Use the reference index to find exact contract owners. Do not restore stale routes, historical rewrite notes, old cleanup records, or migration notes into active docs.

활성 경로는 위의 현재 간결 구조에만 둡니다. 정확한 계약 담당 문서는 참조 색인에서 찾습니다. 오래된 경로, 과거 재작성 기록, 예전 정리 기록, 마이그레이션 메모를 활성 문서로 되돌리지 않습니다.

The [Reference Index](docs/en/reference/README.md) / [참조 색인](docs/ko/reference/README.md) routes named owner documents for active MVP boundary questions, verified local surface access, project-wide `state_version`, `SensitiveActionScope`, product-file `AuthorizedAttemptScope`, staged artifacts, `CompletionPolicy`, `EvidenceSummary`, `close_task` blockers, read-only projections, capability profiles, detective guarantee gating, user-owned judgments, shaping readiness, maintain checks, and translation rules.

[Reference Index](docs/en/reference/README.md) / [참조 색인](docs/ko/reference/README.md)은 현재 MVP 경계, 확인된 로컬 접점 접근, 프로젝트 전체 `state_version`, `SensitiveActionScope`, 제품 파일 쓰기 범위인 `AuthorizedAttemptScope`, 스테이징된 아티팩트, `CompletionPolicy`, `EvidenceSummary`, `close_task` 차단 사유, 읽기 전용 Projection, 역량 프로필, 탐지형 보장 조건, 사용자 소유 판단, 구체화 준비 상태, 문서 점검, 번역 규칙의 담당 문서로 안내합니다.

Do not list profile-gated values as default active MVP values, describe later candidates as active requirements, or make unsupported preventive, isolation, sandboxing, tamper-proof, or default tool-blocking security claims.

profile-gated 값을 기본 현재 MVP 값처럼 나열하지 않습니다. 이후 후보를 활성 요구사항처럼 설명하지 않습니다. 근거 없는 예방형, 격리, 샌드박스, 변조 방지, 기본 도구 차단 보안 주장을 만들지 않습니다.

Korean docs should read as natural Korean technical prose while preserving exact identifiers such as file paths, `doc_id`, schema fields, enum values, error codes, table names, validator IDs, and template names.

한국어 문서는 자연스러운 한국어 기술 문서로 씁니다. 파일 경로, `doc_id`, 스키마 필드, enum 값, 오류 코드, 테이블 이름, validator ID, 템플릿 이름 같은 정확한 식별자는 그대로 보존합니다.

## Contributor Notes / 기여자 참고

Future agents should follow [AGENTS.md](AGENTS.md) before editing. Keep current context small, pull owner docs only when needed, and do not load paired English/Korean docs for the same `doc_id` in one prompt unless translation or semantic-parity review requires it.

향후 에이전트는 편집 전에 [AGENTS.md](AGENTS.md)를 따라야 합니다. 작은 현재 맥락을 유지하고 필요한 담당 문서만 불러오며, 번역이나 의미 일치 검토가 필요한 경우가 아니면 같은 `doc_id`의 영어/한국어 문서를 한 프롬프트에 함께 넣지 않습니다. 이것이 에이전트 중복 주입 금지의 기본 규칙입니다.
