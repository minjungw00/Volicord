# 하네스 문서

이 문서는 하네스 한국어 문서 세트의 길잡이입니다.

하네스는 AI 지원 제품 작업을 위한 로컬 작업 권한 서버입니다. 대화에만 남아 있으면 쉽게 흔들리는 작업 기준을 대화 밖에 두는 것이 하네스의 역할입니다. 하네스는 범위, 사용자 소유 판단, 근거, 확인과 검증 기대, 최종 수락, 닫기 가능 여부, 잔여 위험의 로컬 근거를 보존합니다. 에이전트가 판단하면 안 되는 일은 사용자에게 다시 돌려보냅니다.

| 하네스가 아닌 것 | 하네스가 하는 일 |
|---|---|
| 프롬프트 묶음이나 대화 스크립트. | 작업 권한을 프롬프트와 대화 밖에 둡니다. |
| MCP 자체나 API 래퍼. | MCP/API 접점을 사용할 수 있지만 제품 명제는 로컬 작업 권한 기록입니다. |
| 워크플로 엔진, 보고서 생성기, 대시보드. | 작업의 근거를 기록하고 그 기록에서 읽기용 보기를 만들 수 있습니다. |
| 호스팅 에이전트 플랫폼. | 로컬 하네스 서버/설치를 중심으로 설계됩니다. |
| 샌드박스나 OS 권한 시스템. | OS 수준 격리나 임의 도구 권한 제어를 주장하지 않고 권한 경계를 보존합니다. |

이 저장소는 현재 문서 전용이며 향후 역할은 하네스 서버 소스 저장소입니다. 사용자의 제품 저장소가 아니고 하네스 런타임 홈도 아닙니다. 아직 하네스 서버, 런타임, 생성된 읽기용 요약 시스템, conformance runner, 런타임 데이터, 제품 구현 코드, 생성된 운영 아티팩트는 없습니다. 문서 수락만으로 구현은 허가되지 않으며, 서버/런타임 구현은 문서 수락과 별도의 구현 계획 준비 결정 이후에만 시작할 수 있습니다.

## 최소 첫 읽기 경로

어디서 시작해야 할지 모를 때는 이 순서로 읽습니다.

1. [개요](learn/overview.md)에서 첫 번째 이해 모델을 잡습니다.
2. [하나의 작업](learn/one-task.md)에서 사용자 작업 하나가 어떻게 느껴지는지 봅니다.
3. [핵심 개념](learn/concepts.md)에서 최소 어휘를 봅니다.
4. [사용자 가이드](use/user-guide.md)에서 실제 사용자와 에이전트 상호작용을 봅니다.
5. 향후 하네스 서버 구현을 검토하거나 계획할 때만 [구현 개요](build/implementation-overview.md)를 봅니다.
6. 정확한 계약이 필요할 때만 [Reference 색인](reference/README.md)을 봅니다.

이 경로는 일부러 짧게 잡았습니다. 하네스를 처음 이해하기 위해 큰 Reference 문서부터 읽을 필요는 없습니다.

## 독자별 경로

| 독자 | 먼저 읽기 | 이어서 보기 |
|---|---|---|
| 일반 사용자 | [개요](learn/overview.md) | 작업 흐름의 느낌은 [하나의 작업](learn/one-task.md); 실제 세션 동작은 [사용자 가이드](use/user-guide.md); 용어 이름이 필요할 때만 [핵심 개념](learn/concepts.md). |
| 에이전트 지침 작성자 | [에이전트 가이드](use/agent-guide.md) | 정확한 connector/context 계약이 필요할 때만 [Agent 통합 참조](reference/agent-integration.md)와 [Surface Cookbook](reference/surface-cookbook.md). |
| 서버 구현자 | [구현 개요](build/implementation-overview.md) | [내부 엔지니어링 점검](build/engineering-checkpoint.md) -> [MVP-1 사용자 작업 루프](build/mvp-user-work-loop.md) -> [MVP API](reference/api/mvp-api.md) -> [Storage](reference/storage.md) -> [보안 참조](reference/security.md). [런타임 설계 흐름](build/runtime-walkthrough.md)은 의도한 request-to-close 설계 경로를 볼 때만 사용합니다. |
| 문서 유지보수자 | [문서 작성 가이드](maintain/authoring-guide.md) | [문서 점검표](maintain/documentation-checks.md), [번역 가이드](maintain/translation-guide.md), [재작성 계획](maintain/rewrite-plan.md), [재작성 수락 리뷰](maintain/rewrite-acceptance-review.md), 엄격한 의미를 확인할 때만 Reference 담당 문서. |
| 이후 프로필 독자 | [보증 프로필](later/assurance-profile.md) | [운영 프로필](later/operations-profile.md), [향후 Fixtures](later/future-fixtures.md), [로드맵](roadmap.md). Owner가 승격하기 전까지 MVP 경로 밖에 둡니다. |

## 문서별 역할

Learn, Use, Build, Reference, Later, Maintain 문서는 서로 다른 일을 담당합니다.

| 문서군 | 역할 |
|---|---|
| Learn | 하네스가 왜 필요한지, 권한이 어디에 있는지, 엄격한 계약 전에 필요한 개념. |
| Use | 하네스 기준으로 작업할 때 사용자와 에이전트가 상호작용하는 법. |
| Build | 향후 구현 순서, 단계 경계, 유지보수자 인계. |
| Reference | 정확한 owner 계약: schema, DDL, gate, 상태 전이, 읽기용 요약(Projection) 규칙, 보안 의미, conformance 의미, template, 용어. |
| Later | MVP 구현 경로 밖에 두는 보증, 운영, 향후 fixture, 로드맵 후보. |
| Maintain | 문서 규칙, 재설계 범위, 의미 일치 기대, drift 처리. |

## 학습 문서

정확한 계약에 들어가기 전에 권한 경계 중심의 전체 그림을 잡을 때 사용합니다.

| 문서 | 고유 역할 |
|---|---|
| [개요](learn/overview.md) | 가장 먼저 읽는 문서입니다. 하네스가 무엇이고 왜 필요한지, 무엇을 분리하는지, 하네스가 아닌 것을 설명합니다. |
| [하나의 작업](learn/one-task.md) | 기본 학습 흐름입니다. 평소 요청 하나를 구체화, 범위, 근거, 확인, 잔여 위험, 최종 수락, 닫기까지 따라갑니다. |
| [핵심 개념](learn/concepts.md) | 처음 읽는 사람에게 필요한 최소 어휘입니다. 내부 라벨은 선택 사항으로 둡니다. |
| [15분 만에 보는 하네스](learn/harness-in-15-minutes.md) | 오래된 링크를 위한 짧아진 경로입니다. 현재 학습 경로로 안내합니다. |
| [목적과 원칙](learn/purpose-and-principles.md) | 검토자를 위한 선택 문서입니다. 가치, 실패 모델, 비목표, MVP 경계를 확인할 때 사용합니다. |

## 사용 문서

AI 지원 개발 세션을 하네스 기준으로 진행하거나 설명할 때 사용합니다.

- [사용자 가이드](use/user-guide.md)는 기본 사용자 진입점입니다.
- [에이전트 가이드](use/agent-guide.md)는 에이전트/통합 동작 지침입니다.
- [사용자 소유 판단 예시](use/decision-packet-cookbook.md)는 고급 사용 예시이자 Reference 인접 결정 예시입니다.

## 구현 문서

구현 방향을 파악하고 계획을 검토할 때 사용합니다. [문서 수락 상태](build/implementation-overview.md#문서-수락-상태)가 구현 계획 준비를 명시적으로 수락하기 전까지 Build 문서는 계획 지침이며 하네스 서버/런타임 구현을 허가하지 않습니다.

서버 구현자 빠른 경로:

1. [구현 개요](build/implementation-overview.md): 현재 상태, 유지보수자 인계, 향후 저장소 역할.
2. [내부 엔지니어링 점검](build/engineering-checkpoint.md): 제품 MVP가 아닌 첫 내부 권한 루프 smoke.
3. [MVP-1 사용자 작업 루프](build/mvp-user-work-loop.md): 첫 사용자 가치 구현 계획과 서버 코딩 전 결정 기록.
4. [MVP API](reference/api/mvp-api.md), [API Schema Core](reference/api/schema-core.md), [API Errors](reference/api/errors.md): active MVP-1 tool, shared shape, resource, error, idempotency, state conflict.
5. [Storage](reference/storage.md): runtime layout, staged storage profile, lock, artifact, migration.
6. [보안 참조](reference/security.md): MVP-1의 cooperative/limited-detective guarantee wording과 local-access boundary.

[런타임 설계 흐름](build/runtime-walkthrough.md)은 의도한 동작의 설계 walkthrough이며 런타임이 존재한다는 증거가 아닙니다. 정확한 request-to-close state behavior는 [Core Model 참조](reference/core-model.md)가 담당합니다.

Future/diagnostic material은 Build 또는 Reference owner가 해당 단계로 명시적으로 승격하기 전까지 MVP 구현 경로 밖에 둡니다.

## 참조 문서

정확한 계약을 찾아볼 때 사용합니다. Reference 전체를 기본으로 읽지 말고, 지금 필요한 질문의 담당 문서만 고릅니다. [Reference 색인](reference/README.md)이 간결한 담당 계약 지도입니다.

| 필요한 것 | 담당 문서 |
|---|---|
| Core 권한, entity, gate, 상태 전이, 쓰기 전 범위 확인 / Write Authorization, 닫기 의미 | [Core Model 참조](reference/core-model.md) |
| MVP public tool, envelope, schema, error, idempotency, state conflict behavior, shared ref, validator result schema | [MVP API](reference/api/mvp-api.md), [API Schema Core](reference/api/schema-core.md), [API Errors](reference/api/errors.md) |
| 이후/profile-gated API method와 future schema material | [API Schema Later](reference/api/schema-later.md)와 [보증 프로필](later/assurance-profile.md) |
| Runtime layout, DDL profile, storage JSON, lock, artifact, migration, baseline, projection job storage, validator storage | [Storage](reference/storage.md) |
| 읽기용 view, projection freshness, managed block, template body | [Projection과 Template 참조](reference/projection-and-templates.md)와 [Template 참조](reference/templates/README.md) |
| 신뢰 경계, asset, threat category, control, 보장 수준 표현 | [보안 참조](reference/security.md) |
| Operator behavior, diagnostic, recover/reconcile/export/artifact check, conformance run entrypoint | [운영과 Conformance 참조](reference/operations-and-conformance.md) |
| Fixture model, fixture body, runner/assertion semantics, Kernel Smoke queue, later scenario inventory | [Conformance Fixtures 참조](reference/conformance-fixtures.md)와 [향후 Fixtures](later/future-fixtures.md) |
| Connector profile, context push/pull, fallback behavior, surface recipe, user-facing integration pattern | [Agent 통합 참조](reference/agent-integration.md)와 [Surface Cookbook](reference/surface-cookbook.md) |
| Design-quality policy, validator ID, severity composition, waiver semantics, policy close impact | [설계 품질 정책](reference/design-quality-policies.md) |
| Public/internal terminology와 owner routing | [용어집 참조](reference/glossary.md) |
| Runtime space, Core transaction placement, architecture flow, artifact, projection/reconcile placement, recovery overview | [런타임 아키텍처 참조](reference/runtime-architecture.md) |

## 이후 문서

이후 문서는 owner가 승격하기 전까지 MVP 구현 경로 밖에 두는 내용을 다룹니다.

- [보증 프로필](later/assurance-profile.md)
- [운영 프로필](later/operations-profile.md)
- [향후 Fixtures](later/future-fixtures.md)
- [로드맵](roadmap.md)

## 유지보수 문서

문서와 이후 하네스 시스템의 일관성을 유지할 때 사용합니다. Maintain 문서는 런타임 동작이 아니라 문서 유지보수를 관리합니다. 문서 점검표는 읽기 전용 docs-maintenance 점검입니다. `PASS`, `WARN`, `FAIL` label은 리뷰에 도움을 주지만 runtime conformance, manual acceptance, close readiness, implementation readiness를 만들지 않습니다.

- [문서 작성 가이드](maintain/authoring-guide.md)
- [문서 점검표](maintain/documentation-checks.md)
- [번역 가이드](maintain/translation-guide.md)
- [재작성 계획](maintain/rewrite-plan.md)
- [재작성 수락 리뷰](maintain/rewrite-acceptance-review.md)

## 현재 상태 모델

현재 상태는 문서 검토 상태, 구현 계획 준비 상태, 런타임 구현 상태를 분리해서 읽어야 합니다.

| 상태 범주 | 현재 상태 |
|---|---|
| 문서 검토 상태 | 재설계 이후 검토 상태이며 문서 수락 후보입니다. 유지보수자가 아직 문서를 수락하지 않았습니다. |
| 구현 계획 준비 상태 | 아직 수락되지 않았습니다. 첫 런타임 배치 계획 전에 유지보수자가 구현 준비 조건을 확인해야 합니다. |
| 런타임 구현 상태 | 시작하지 않았습니다. 아직 런타임 아티팩트나 conformance 결과가 없습니다. |
| 구현 결정 상태 | 서버 코딩 전 열린 결정은 [MVP-1 사용자 작업 루프](build/mvp-user-work-loop.md#서버-코딩-전-필요한-구현-결정)에 기록되어 있습니다. 서버/런타임 구현 결정은 코드 작성용으로 공식 수락되지 않았습니다. 영향을 받는 구현 작업은 관련 결정이 수락되거나 단계 영향과 함께 명시적으로 미뤄질 때까지 기다려야 합니다. |

문서 수락은 유지보수자 검토 이정표입니다. 문서가 수락되더라도 그것만으로 런타임/서버 구현이 시작되거나 런타임 conformance가 증명되지는 않습니다.

## 문서 인계

하네스 서버 코드를 시작하기 전 구현자는 다음을 읽어야 합니다.

1. [문서 인계 요약](build/implementation-overview.md#문서-인계-요약).
2. [문서 수락 상태](build/implementation-overview.md#문서-수락-상태).
3. [재작성 수락 리뷰](maintain/rewrite-acceptance-review.md).
4. [하네스 서버 구현 준비 조건](build/implementation-overview.md#하네스-서버-구현-준비-조건).
5. [서버 코딩 전 필요한 구현 결정](build/mvp-user-work-loop.md#서버-코딩-전-필요한-구현-결정).

이 인계는 문서가 maintainer 수락 검토 후보라는 뜻이며, Implementation Overview의 문서 수락 상태와 MVP-1 사용자 작업 루프의 서버 코딩 전 열린 결정 기록을 분리합니다. 서버/런타임 구현 결정은 코드 작성용으로 공식 수락되지 않았고, 영향을 받는 구현 작업은 관련 결정이 수락되거나 단계 영향과 함께 명시적으로 미뤄질 때까지 기다려야 합니다. 문서가 이미 수락되었거나, 구현 준비가 끝났거나, 서버/런타임 구현이 시작되었다는 뜻이 아닙니다.

## 지금 보는 저장소

하네스는 세 공간을 분리합니다.

| 공간 | 들어가는 것 |
|---|---|
| 제품 저장소 | 사용자의 제품 작업 공간입니다. 제품 코드, 테스트, 제품 문서, 향후 제품에 대해 생성되는 사람이 읽는 하네스 보기가 여기에 속합니다. |
| 하네스 서버 소스 저장소 | 로컬 하네스 서버/설치 프로그램의 미래 코드베이스입니다. 이 저장소는 문서 수락과 구현 계획 준비 이후 그 소스 저장소가 될 예정입니다. |
| 하네스 런타임 홈 | 사용자별/설치별 운영 데이터 공간입니다. 상태 데이터베이스, 아티팩트 저장소, 읽기용 요약 출력, 로그, 로컬 등록/설정 정보가 여기에 속합니다. |

이 저장소의 현재 역할은 문서 검토와 재설계입니다. 문서 수락만으로 구현 권한, 런타임 상태, conformance, 서버 코드가 생기지 않습니다.

## 비교

하네스는 에이전트 지침, MCP, 재사용 워크플로, 테스트, 리뷰, 보고서, 대시보드, 호스팅 에이전트 플랫폼, 샌드박스, 사양서와 같은 역할을 하지 않습니다.

| 인접 개념 | 그 역할 | 하네스의 역할 |
|---|---|---|
| AGENTS.md / 에이전트 지침 파일 | 저장소나 세션에서 에이전트가 어떻게 행동해야 하는지 알려 줍니다. | 하네스는 그런 지침을 사용할 수 있지만, 범위, 사용자 소유 판단, 근거, close readiness, 잔여 위험을 로컬 기록으로 유지합니다. |
| MCP / API 접점 | 도구, 리소스, 호출을 연결하는 프로토콜 경계입니다. | 하네스는 MCP/API 접점을 노출할 수 있지만, 그것은 메커니즘입니다. 제품 권한은 Core가 소유한 로컬 상태와 아티팩트 참조에서 나옵니다. |
| 스킬 / 재사용 워크플로 | 에이전트가 반복해서 따를 수 있는 지침이나 절차를 묶습니다. | 하네스는 그런 워크플로 안에서 사용될 수 있지만, 지금 진행 중인 작업 상태를 기록하고 이 작업의 판단을 정해진 경로로 보냅니다. |
| 테스트 실행기 | 검사를 실행하고 결과를 냅니다. | 하네스는 관련 결과를 근거로 연결하고, 검증의 강도와 최종 수락을 따로 둡니다. |
| 코드 리뷰 | 변경을 사람이 또는 팀이 검토합니다. | 하네스는 리뷰 결과를 참조할 수 있지만, 리뷰를 최종 수락, 잔여 위험 수락, 닫기로 바꾸지 않습니다. |
| 보고서 / 대시보드 | 읽기 쉬운 요약, 상태, 분석을 보여 줍니다. | 하네스는 읽기용 보기를 만들 수 있지만, 보기의 문장은 운영 기록이 아닙니다. |
| 호스팅 에이전트 플랫폼 | 에이전트를 서비스로 실행하거나 조율합니다. | 하네스는 호스팅 에이전트 플랫폼이 아니라 로컬 작업 권한 서버를 중심으로 설계됩니다. |
| 샌드박스 / OS 권한 시스템 | 시스템 경계에서 격리나 권한 강제를 제공합니다. | 하네스는 해당 동작에 대해 증명된 메커니즘이 없으면 OS 수준 격리나 임의 도구 권한 제어를 주장하지 않습니다. |
| 사양서 | 의도한 동작, 설계, 제약을 설명합니다. | 하네스는 사양서를 입력으로 사용할 수 있지만, 실제 작업의 운영 상태인 범위, 결정, 근거, QA 기대, 최종 수락, 잔여 위험을 기록합니다. |

## 에이전트 맥락 불러오기

독자별 읽기 경로 전체를 에이전트에 항상 주입하면 안 됩니다. 연결된 에이전트에 항상 주입되는 맥락은 한 화면 이하로 유지합니다. 현재 Task 요약, 작업 모양, 범위/하지 않을 일, 대기 중인 사용자 판단, 활성 막힘, 다음 안전한 행동, 근거 공백, 닫기 막힘, 잔여 위험 요약, 보장 수준, 출처 참조와 최신성만 기본으로 둡니다.

단계별 담당 섹션만 필요할 때 불러옵니다. 자세한 단계별 맥락 지도는 [Agent 통합 참조: Context Push/Pull Principles](reference/agent-integration.md#context-pushpull-principles)가 담당하고, 사용자에게 보이는 동작은 [에이전트 가이드](use/agent-guide.md)가 요약합니다.

## 로드맵

- [로드맵](roadmap.md)

향후 후보 항목은 로드맵에 둡니다. 향후 담당자가 로드맵 기준을 통해 항목을 명시적으로 승격하기 전까지 로드맵 항목은 Build 문서가 담당하는 단계별 전달에 포함되지 않습니다.

## 언어 의미 일치

영어 문서와 한국어 문서는 같은 파일 지도와 의미상 같은 내용을 유지합니다. 한국어 문서는 영어 문장을 한 줄씩 옮기기보다 자연스러운 한국어 제목과 흐름을 사용할 수 있습니다.
