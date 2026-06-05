# 용어집 참조

## 이 문서로 할 수 있는 일

다른 문서를 읽다가 하네스의 공식 용어, 대소문자, record name, 서로 대체할 수 없는 경계를 확인할 때 이 용어집을 사용합니다.

이 문서는 향후 Harness 동작을 위한 참조 문서입니다. 현재 저장소 단계와 구현 인계 상태는 [구현 개요](../build/implementation-overview.md#문서-수락-상태)에 있습니다.

## 이런 때 읽기

하네스 용어를 확인하거나, 권한 경로를 섞지 않도록 점검하거나, 정확한 동작을 담당하는 Reference 문서를 찾을 때 읽습니다.

## 읽기 전에

하네스 개념을 처음 이해하려면 Learn 경로를 사용합니다. 정확한 동작이 필요하면 아래 owner link나 개별 정의 안의 link를 따라갑니다.

## 핵심 생각

용어집은 찾아보기 도구이자 담당 문서 지도입니다. 공개 용어, 내부 구현 용어, 대소문자, record name, 짧은 non-substitution reminder를 일관되게 유지하지만, 담당 참조 문서를 대체하지는 않습니다.

## 참조 범위

이 용어집은 공식 용어 표현, 대소문자 안내, record-name 방향 잡기, 담당 문서 연결을 담당합니다. Kernel behavior, public MCP schema, storage DDL, projection rule, template body, connector capability profile, conformance fixture semantics는 담당하지 않습니다.

## 공개 용어

사용자용 문서, 프롬프트, 상태 요약에서는 아래 여섯 가지 개념을 먼저 씁니다. 사용자가 record name을 배우지 않아도 하네스를 쓸 수 있도록 일부러 쉬운 말로 둡니다.

| 공개 용어 | 쉬운 뜻 |
|---|---|
| 작업 | 사용자가 끝내거나, 답을 얻거나, 조사하거나, 결정하고 싶은 일. 내부 기록을 이름 붙일 때만 `Task`를 씁니다. |
| 범위 | 무엇이 바뀔 수 있고, 무엇은 범위 밖이며, 에이전트가 어디에서 멈춰야 하는지. 작게 나눈 범위는 `작업 조각`으로 설명할 수 있습니다. |
| 판단 / 결정할 것 | 사용자가 소유하는 선택입니다. 사용자에게 보이는 표시는 제품 판단, 기술 판단, 범위 판단, 민감 동작 승인, QA 면제 판단, 검증 위험 수락, 최종 수락, 잔여 위험 수락, 취소 판단 중 하나를 사용합니다. |
| 증거 | 작업에 대한 주장을 뒷받침하는 오래 남는 자료입니다. 변경 경로, diff, 로그, 테스트 출력, 스크린샷, 검사 메모, artifact ref를 포함할 수 있습니다. |
| 확인 / 검증 | 테스트, 변경 차이 검토, 사람의 확인, 출처 확인처럼 일반적인 확인입니다. 공식 기록 경로를 말할 때만 `Verification` 또는 `검증`을 씁니다. 수동 QA는 사람의 판단이 필요한 표면을 사람이 확인하는 경우입니다. |
| 마무리 / 닫기 | 작업을 끝내거나 닫기 전에 아직 무엇이 필요한지입니다. 차단 사유, 필요한 최종 수락, 다음 안전한 행동, 남은 위험이 있을 때 함께 보여줍니다. |

사용자용 문서는 쉬운 개념을 먼저 설명해야 합니다. 요구사항 구체화, 판단 요청, 판단 요약, 증거 참조, 증거 목록, 상태 보기, 상태 카드, 수동 QA, 최종 수락, 잔여 위험, 남은 위험, 닫기 차단 사유, 닫기 가능 여부, 닫기 준비 상태, 다음 안전한 행동 같은 더 구체적인 표현은 유용할 때 쓸 수 있지만, 여섯 개념을 돕는 표현이어야지 새 사용자가 외워야 하는 더 큰 개념 모델이 되면 안 됩니다. 정확한 하네스 라벨은 경계, 차단 사유, 출처 참조, Reference 링크를 설명하는 데 도움이 될 때만 괄호로 덧붙입니다.

한국어 사용자용 prose에서는 보통 Discovery를 `요구사항 구체화`, Change Unit을 `범위` 또는 `작업 조각`, Decision Packet을 `판단 요청` 또는 `판단 요약`, Write Authorization을 `쓰기 전 범위 확인`, Projection을 `상태 보기`, `요약`, `상태 카드`, Evidence Manifest를 `증거 목록`, Close Readiness를 `닫기 가능 여부` 또는 `닫기 준비 상태`, Residual Risk를 `잔여 위험` 또는 `남은 위험`으로 씁니다. Record, schema, API, file path, heading, stable product identifier, owner link 때문에 정밀도가 필요할 때만 정확한 English label을 붙입니다.

## 사용자용 용어 규칙

- 사용자 예시를 내부 용어로 시작하지 않습니다. 작업, 범위, 판단 또는 결정할 것, 증거, 확인 또는 검증, 마무리 또는 닫기를 먼저 말합니다.
- 사용자가 "Discovery", "Change Unit", "Decision Packet", "Write Authorization", "Evidence Manifest", "Projection", "Gate", `task_events` 같은 말을 해야만 진행되는 구조로 쓰지 않습니다.
- 사용자용 한국어에서는 `판단 요청`, `무엇을 결정해야 하나요?`처럼 자연스러운 말을 씁니다. 영어 문서에서는 "judgment request"를 기본 표현으로 씁니다.
- 내부 라벨은 쉬운 의미가 먼저 분명해진 뒤, 선택적/내부 설명으로만 소개합니다.
- 한국어 prose는 한국어 개념을 먼저 두고, 정확한 English label은 필요할 때만 붙입니다. 영어 명사에 한국어 조사만 붙인 문장을 피하고 한국어 기술 독자에게 자연스럽게 씁니다.
- "승인", "진행해", "좋아" 같은 넓은 표현을 제품 판단, 기술 판단, 범위 판단, 민감 동작 승인, QA 면제 판단, 검증 위험 수락, 최종 수락, 잔여 위험 수락, 취소 판단, 쓰기 전 범위 확인이나 쓰기 승인 기록까지 넓혀 해석하면 안 됩니다.

## 내부 / 참조 용어

아래 용어는 reference, API, schema, record, 상태 참조에서 쓰는 구현 라벨입니다. 사용자가 프롬프트에서 이 용어를 쓸 필요는 없습니다. 에이전트가 평소 말로 들어온 요청을 알맞은 하네스 절차로 바꿔야 합니다.

| 내부 용어 | 쉬운 설명 |
|---|---|
| Task | 사용자가 끝내거나, 답을 얻거나, 조사하거나, 결정하고 싶은 일을 오래 남기는 내부 단위입니다. 처음 읽는 사용자용 prose에서는 `작업`을 씁니다. |
| Discovery | 구현 계획이나 쓰기 전 범위 확인 전에 하는 요구사항 구체화의 내부 이름입니다. 사용자는 "구현 전에 요구사항을 구체화해줘"처럼 말하면 됩니다. |
| Change Unit | 제품 파일 쓰기를 위한 내부 scoped work unit입니다. 무엇이 바뀔 수 있는지 말하지만 그 자체로 쓰기를 승인하지는 않습니다. 사용자용 문서는 record 이름보다 `범위`나 `작업 조각`을 먼저 설명합니다. |
| User Judgment | 진행, 쓰기, 최종 수락, 위험 처리, 닫기를 막는 특정 사용자 소유 판단을 기록하는 canonical 경로입니다. Public refs는 `record_kind=user_judgment`를 사용합니다. |
| Decision Packet | 복잡한 `user_judgment`를 위한 full judgment presentation이며, 오래된 reference에서는 legacy label로도 남아 있습니다. 기본 사용자 표시 메커니즘도, 별도 authority family도 아닙니다. |
| Write Authorization | Stored `AuthorizedAttemptScope` 하나에 대해 `dry_run=false`인 `prepare_write.decision=allowed`일 때만 생성하는 내부 협력형 하네스 기록입니다. 이 scope는 Core가 나중에 `record_run`에서 비교하는 operation/path/tool/command/class/product-write/network/secret/sensitive-category/baseline/Task/Change Unit/state/surface/judgment/guarantee boundary입니다. Lifecycle status는 `active`, `consumed`, `expired`, `stale`, `revoked`입니다. `allowed`와 `blocked`는 prepare-write decision이지 durable lifecycle status가 아닙니다. OS 권한, sandboxing, 변조 방지 enforcement, 사전 차단, 격리가 아닙니다. |
| Evidence Manifest | 완료 조건이나 수용 기준을 증거 참조와 연결하는 자세한 증거 목록 기록입니다. |
| Eval | 검증 결과 기록입니다. 대상, verdict, 수행한 확인, 검토한 증거, 독립성, 최신성, 차단 사유, artifact ref를 남깁니다. |
| Projection | 하네스 상태에서 만든 파생 보기입니다. 상태 보기, 요약, 상태 카드, 보고서처럼 상태를 보여 주지만 상태를 대체하지 않습니다. |
| Gate | 커널의 준비 상태 또는 호환성 조건입니다. 사용자용 문서에서는 보통 `gate` 이름보다 차단 사유나 확인을 쉬운 말로 먼저 보여줍니다. |
| Autonomy Boundary | 활성 범위 안에서 에이전트가 다시 묻지 않고 판단해도 되는 선택의 경계입니다. |
| `task_events` | 작업 상태 변화를 남기는 내부 event log table입니다. 사용자용 어휘가 아니라 reference/schema 용어입니다. |

## Schema/API 식별자

Schema, API 문서, code-like example, record, DDL/table 문맥, method name, field name, enum value, file path, literal marker, stable product identifier, 진단 출력에서는 아래 이름을 정확히 유지합니다. 사용자에게 설명할 때는 뜻을 쉬운 말로 풉니다.

| 식별자 | 의미와 표시 원칙 |
|---|---|
| `user_judgment` | 사용자 소유 판단의 canonical public record family입니다. MVP-1 민감 동작 승인 판단도 포함합니다. |
| `UserJudgment` | User judgment를 위한 canonical schema shape입니다. Schema/API context에서는 정확히 유지합니다. |
| `judgment_kind` | Compact internal judgment type입니다. 값은 `product_decision`, `technical_decision`, `scope_decision`, `sensitive_approval`, `qa_waiver`, `verification_risk_acceptance`, `final_acceptance`, `residual_risk_acceptance`, `cancellation`입니다. |
| `presentation` | Prompt/detail field입니다. Compact prompt는 `short`, full-format Decision Packet presentation은 `full`을 사용합니다. |
| `display_label` | Surface가 그 이름을 노출할 때의 compatibility 또는 response-only 사용자 표시 판단 라벨입니다. Active canonical schema/storage field가 아니며, 렌더러가 `judgment_kind`와 locale에서 라벨을 파생합니다. |
| `request_user_decision`, `record_user_decision` | `request_user_judgment`, `record_user_judgment`의 compatibility alias입니다. Compatibility docs 또는 migration note에서만 보존합니다. |
| `judgment_type`, `judgment_domain`, `decision_kind`, `decision_profile` | `judgment_kind`, route-specific payload validation, `presentation`으로 mapping되는 compatibility alias입니다. Compatibility docs 또는 오래된 payload에서만 보존합니다. |
| `judgment_category`, `judgment_route`, `display_depth` | 오래된 Decision Packet draft의 legacy 또는 implementation routing term입니다. 새 public docs, example, fixture에서는 primary concept로 쓰지 않습니다. |
| `Task`, `UserJudgment`, `ArtifactRef`, `ProjectionKind`, `ValidatorResult` | Schema/API shape 이름입니다. Contract를 이름 붙일 때 정확히 유지합니다. |
| `prepare_write`, `record_run`, `close_task`, `harness.request_user_judgment`, `harness.record_user_judgment` | Tool/API 식별자입니다. 정확히 유지하고, 사용자에게 보이는 결과는 쉬운 말로 설명합니다. |

## 향후 / 나중 프로필 용어

아래 라벨은 roadmap, reference, template, diagnostic material에서 보일 수 있습니다. 담당 profile이 기능을 승격하기 전에는 첫 설명의 사용자 어휘나 필수 명령으로 만들지 않습니다.

| 나중 프로필 용어 | 상태와 표시 원칙 |
|---|---|
| Context Index | 나중의 읽기 전용 retrieval 지원입니다. 살펴볼 source를 제안할 수 있지만 write 승인, gate 충족, 위험 수락, close를 대신하지 않습니다. |
| Journey Card / Journey Spine | 나중의 continuity display입니다. 활성화되고 최신일 때 방향을 잡는 데 도움을 주지만 Core가 소유한 상태는 아닙니다. |
| Browser QA Capture | Browser artifact capture를 위한 로드맵 후보입니다. 그 자체로 수동 QA, 최종 수락, 분리 검증이 아닙니다. |
| Standalone `DEC` projection | 기능이 켜졌을 때 가능한 선택적 full-format Decision Packet Markdown rendering입니다. 사용자 판단 visibility가 standalone `DEC` 파일 읽기에 의존하면 안 됩니다. |
| 운영 프로필 표시 | 나중 또는 profile-gated 운영/보고 표면입니다. Owner record를 보여주거나 내보낼 뿐 Core 권한을 대체하지 않습니다. |

## 전달 라벨

Active delivery label은 일관되게 씁니다.

| 라벨 | 상태 |
|---|---|
| 내부 엔지니어링 점검(Engineering Checkpoint) | 내부 authority-loop smoke입니다. Product MVP도 아니고 첫 사용자 가치 slice도 아닙니다. |
| MVP-1 사용자 작업 루프(MVP-1 User Work Loop) | 첫 좁은 사용자 가치 milestone입니다. |
| 보증 프로필(Assurance Profile) | 이후 보증 동작을 단단하게 하는 범위입니다. |
| 운영 프로필(Operations Profile) | 이후 운영과 handoff 동작을 단단하게 하는 범위입니다. |
| 로드맵(Roadmap) | Owner 문서가 승격하고 증명하기 전까지 future scope입니다. |

`Kernel Smoke`는 stage가 아닙니다. 내부 엔지니어링 점검 아래의 좁은 future smoke-check 작성 라벨로만 씁니다. `v0.1 Core Authority Smoke`, `v0.2 First User-Value Slice`, `v0.3 Agency Assurance Pack`, `v0.4 Operations & Handoff Pack`, `v1+ Expansion` 같은 예전 label은 현재 단계 이름이 아니며 오래된 별칭으로만 남길 수 있습니다.

## 담당 문서 지도

| 용어 묶음 | 담당 참조 문서 |
|---|---|
| Task, Change Unit, gate(관문), close, 민감 동작 승인, 최종 수락, 검증, 수동 QA, 잔여 위험, 쓰기 전 범위 확인 / Write Authorization | [Core Model Reference](core-model.md) |
| MCP resource, MCP tool, public schema, error, `ValidatorResult`, `ProjectionKind` | [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), [API Schema Later](api/schema-later.md) |
| SQLite record, artifact layout, enum hardening, `tree_hash`, `request_hash` storage use | [Storage](storage.md) |
| 파생 보기 / Projection, managed block, projection freshness, Markdown 보고서, template body | [Projection과 Template 참조](projection-and-templates.md); [Template 참조](templates/README.md) |
| Discovery와 Shared Design, design quality, stewardship, Feedback Loop finding routing, context hygiene, severity composition, policy contract | [설계 품질 정책](design-quality-policies.md) |
| Surface capability, guarantee display, connector behavior | [Agent 통합 참조](agent-integration.md) |
| Security asset, trust boundary, threat category, high-risk control expectation, guarantee-level 의미 | [보안 참조](security.md) |
| Operator procedure, conformance run overview, docs-maintenance 보고 | [운영과 Conformance 참조](operations-and-conformance.md) |
| 핵심 적합성 모델, 정확한 fixture body, runner 동작, assertion semantics, fixture profile, 축소된 Kernel Smoke 작성 순서 | [Conformance Fixtures 참조](conformance-fixtures.md) |
| 간결한 향후 scenario family 목록, 승격 조건, suite-family label, catalog-only future candidate | [향후 Fixtures](../later/future-fixtures.md) |

## 공식 용어

### Agency Conformance

하네스 동작, 상태 보기와 요약, validator, 닫기 판단이 사용자의 Strategic Agency를 얼마나 보존하는지 나타내는 정도입니다. 작업 여정을 따라갈 수 있는지, 사용자 소유 판단이 명시적인지, Autonomy Boundary가 지켜지는지, 차단하는 사용자 소유 판단에 user judgment가 있는지, 최종 수락 전에 잔여 위험이 보이는지 확인합니다.

### Acceptance

한국어 기준 표현: 최종 수락.

증거, 검증, 수동 QA 상태, scope, 민감 동작 승인, 닫기에 영향을 주는 잔여 위험이 보였거나 없다고 확인된 뒤, 작업 결과를 최종 수락할 수 있다는 사용자의 최종 판단입니다. Required 최종 수락은 `judgment_kind=final_acceptance`인 `user_judgment`, `task_gates.acceptance_gate`, `state.sqlite.task_events`를 포함하는 kernel 최종 수락 경로를 통해 기록됩니다. 최종 수락은 민감 동작 승인, assurance, 검증, 수동 QA, evidence sufficiency, QA 면제 판단, 검증 위험 수락, 잔여 위험 수락과 구분됩니다. 추가 write를 승인하지 않고, 민감 동작을 승인하지 않으며, 알려진 위험을 그 자체로 수락하지 않고, 잔여 위험을 지우거나 빠진 check를 나중에 충족된 것으로 만들지 않습니다.

### Acceptance Gate

Required 최종 수락을 위한 kernel gate입니다. Value set과 compatibility meaning은 [Acceptance Gate](core-model.md#acceptance-gate)가 담당합니다. 최종 수락은 QA나 검증을 대신할 수 없습니다.

현재 reference model에서 required 최종 수락은 `user_judgment`, `task_gates.acceptance_gate`, `state.sqlite.task_events`를 통해 기록됩니다. 별도의 acceptance record 또는 table은 없습니다.

### Approval

한국어 기준 표현: 민감 동작 승인.

정의된 scope 안에서 특정 sensitive action 또는 경계가 정해진 민감 동작을 진행할 수 있게 하는 제한된 사전 user authorization입니다. 민감 동작 승인은 paths, tools, commands 또는 command classes, network targets, secret scope, baseline, sensitive categories, expiry conditions에 묶입니다. Minimum MVP-1에서 Core는 `judgment_payload.approval_scope`를 가진 `judgment_kind=sensitive_approval` user judgment를 기록합니다. Later Approval profile은 연결된 committed Approval record를 추가로 만들 수 있습니다. Granted 민감 동작 승인이 있어도 쓰기 승인 기록이 생기려면 이후 compatible `prepare_write` result가 필요합니다. Approval은 민감 동작 승인일 뿐입니다. 막연한 동의, 제품 판단, 기술 판단, 범위 판단, 최종 수락, 잔여 위험 수락, QA 면제 판단, 검증 위험 수락, correctness proof, 취소 판단의 대체물이 아닙니다.

### Approval Gate

민감 동작 승인을 위한 kernel gate입니다. Sensitive categories가 있을 때만 required입니다. Granted 민감 동작 승인은 correctness를 증명하지 않고, 최종 수락을 뜻하지 않으며, 잔여 위험을 수락하거나 QA 면제 판단 또는 검증 위험 수락을 기록하지 않습니다. 사용자 소유 판단을 해소하거나 쓰기 승인 기록을 만들지도 않습니다.

### Assumption Register

구현 계획 전에 에이전트가 사용하는 가정을 정리한 Discovery 또는 Shared Design 보조 상태 보기/요약 목록입니다. Source, confidence, owner, 가정이 틀릴 때 바뀌는 일을 이름 붙여야 합니다. 이는 권장 표시 또는 보조 내용이지 독립 schema나 기준 record 필드 목록이 아닙니다. Assumption Register는 Discovery Brief, 안전한 다음 작업 후보, 작업 분할 제안, First Safe Change Unit Candidate를 구체화하는 데 도움을 줍니다. 하지만 사용자 동의, 민감 동작 승인, 최종 수락, 잔여 위험 수락, 증거, 닫기 준비 상태, 범위 권한, 쓰기 승인 기록은 아닙니다.

### Artifact

Core가 허용된 source에서 받아들이고 integrity, redaction, owner, retention metadata를 기록한 뒤 증거, 복구, 감사에 사용하는 registered output입니다. Evidence-file 경계는 Raw Artifact를 참고합니다.

### Artifact Reference

한국어 기준 표현: 아티팩트 참조.

Artifact store에 등록된 artifact file 또는 안전한 메타데이터 알림을 가리키는 구조화된 포인터입니다. Artifact identity, owner scope, kind, `uri`, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `produced_by`, relation owner, `retention_class`, availability metadata를 포함합니다. `ArtifactRef`는 이 pointer shape의 정확한 schema name이며 caller가 넘긴 임의 path가 아닙니다. [Storage](storage.md)에서 아티팩트 참조와 `artifact_links`는 Task-scoped입니다. `bundle`, `manifest`, `export_component` 같은 artifact kind는 file을 설명합니다. Owner link는 여전히 기존 상태 또는 Task-scoped projection record를 가리킵니다.

### Autonomy Boundary

추가 사용자 소유 판단 없이 에이전트가 진행할 수 있는 판단 경계를 기록하는 Change Unit semantics입니다. 쉽게 말해 active Change Unit 안에서 에이전트가 무엇을 혼자 판단해도 되는지 말합니다. 일상적인 구현 세부사항은 경계 안에 있을 수 있습니다. 하지만 public API 또는 module contract 변경, 보안/개인정보 절충, UX 또는 제품 동작 장단점, 중요한 dependency 또는 migration 방향, scope expansion, 잔여 위험 수락은 명시적인 사용자 소유 판단이 필요하며 넓은 자율성에서 추론하면 안 됩니다.

이는 scope grant나 쓰기 전 범위 확인 / Write Authorization이 아니며 active Change Unit 밖의 paths, tools, commands, network targets, secret access, sensitive categories를 승인하지 않습니다. User judgment가 Autonomy Boundary update나 Change Unit update proposal을 승인할 수는 있습니다. 하지만 resulting write에는 여전히 compatible Change Unit scope와 sensitive categories에 필요한 민감 동작 승인이 필요합니다. 정확한 kernel behavior는 [Autonomy Boundary](core-model.md#autonomy-boundary)가 담당하고, policy placement는 [설계 품질 정책](design-quality-policies.md#autonomy-boundary-autonomy_boundary)이 담당합니다.

### Assurance

기록된 check와 verification independence가 뒷받침하는 technical confidence level입니다.

```text
none | self_checked | detached_verified
```

Eval verdict만으로 assurance가 올라가지 않습니다. `detached_verified`에는 valid independence가 있는 passed verification과 same-session self-review violation 없음이 필요합니다.

### Baseline

Scope, approval drift, evidence freshness, verification validity를 판단하는 데 사용하는 captured repository state입니다.

### Blocker

한국어 기준 표현: 차단 사유.

진행, 쓰기, 닫기 또는 요청된 다음 단계를 해결하거나 유효하게 미루기 전까지 막는 구체적인 조건입니다. 사용자용 prose에서는 `차단 사유`를 쓸 수 있고, API/reference 문맥에서는 `blocker`를 유지하거나 `차단 조건(blocker)`으로 설명합니다. 정확한 field name, template key, enum-like value, schema name은 번역하지 않습니다. 유용한 차단 사유 표시는 무엇이 막혔는지, 다음 움직임을 누가 소유하는지, 가장 작은 해소 방법이 무엇인지, 관련 소유자 ref가 무엇인지 보여줍니다. 차단 사유는 일반 note, evidence 자체, 최종 수락, 잔여 위험 수락, 민감 동작 승인이 아닙니다.

### `tree_hash`

Ignored paths를 제외한 뒤 sorted NFC-normalized relative POSIX paths, file bytes, size, executable bit, symlink target handling을 사용해 계산하는 baseline file snapshot의 deterministic hash입니다. 세부 규칙은 [Storage](storage.md)이 정의합니다.

### Capability Profile

연결된 에이전트 접점의 실제 능력을 선언 및 검증된 설명으로 기록한 것입니다. target profile, support tier, guarantee level, supported features, risks, fallbacks, last verification time을 기록합니다. 하네스는 제품 이름만으로 capability를 infer하지 않습니다.

### Capability Tier

연결된 접점에 대한 coarse integration level입니다.

```text
T0 Context | T1 Skill | T2 MCP | T3 Capture |
T4 Guard | T5 Isolation | T6 QA Capture
```

Capability tiers는 available integration support를 설명할 뿐 kernel gates가 아닙니다.

### Change Unit

한국어 사용자 표현: 범위 / 작업 조각.

Product writes의 범위를 정하는 scoped implementation unit입니다. Product write에는 intended paths, tools, commands, network targets, sensitive categories를 cover하는 active Change Unit이 필요하지만, Change Unit 자체가 write 권한을 부여하지는 않습니다. Core가 `prepare_write`와 applicable gates를 통해 write 허용 여부를 판단합니다.

사용자용 문서는 보통 관련 범위나 작업 조각을 먼저 설명합니다. 내부 scoped work unit, record, reference owner를 이름 붙일 때만 `Change Unit`을 씁니다.

### Close Reason

Task가 terminal close state에 도달한 기준 reason입니다.

```text
none | completed_verified | completed_self_checked |
completed_with_risk_accepted | cancelled | superseded
```

### Close Readiness

한국어 기준 표현: 닫기 가능 여부 / 닫기 준비 상태.

작업을 지금 닫을 수 있는지, 정직하게 닫기 전에 무엇이 남았는지를 보여주는 사용자용 요약입니다. 닫기 차단 사유, 빠진 증거, 검증 또는 수동 QA 상태, 필요한 최종 수락, 보이는 잔여 위험, 잔여 위험 수락 필요 여부, 다음 안전한 행동을 보여줄 수 있습니다. 닫기 가능 여부는 owner record와 gate에서 파생됩니다. 그 자체가 최종 수락, 잔여 위험 수락, 증거, 검증, QA, 쓰기 승인 기록, close event는 아닙니다.

### Codebase Stewardship

제품 코드베이스를 durable asset으로 지키는 책임입니다. Domain language, module 경계, interface contracts, dependency direction, testability, maintainability, future-change risk를 살피는 일을 포함합니다.

### Common Tool Envelope

Public MCP tool calls가 공통으로 갖는 fields입니다. `request_id`, `idempotency_key`, `expected_state_version`, `project_id`, optional `task_id`, `surface_id`, optional `run_id`, `actor_kind`, `dry_run`을 포함합니다.

### Core-owned State

한국어 기준 표현: Core가 소유한 상태.

Harness Core가 커밋된 소유자 기록과 `state.sqlite.task_events`를 통해 소유하는 운영 상태입니다. Core가 소유한 상태는 gate, decision, 쓰기 승인 기록, 증거 상태, QA, 검증, 최종 수락, 잔여 위험, 닫기의 기준입니다. Chat, 생성된 Markdown 상태 보기나 요약, connector 파일, 제품 저장소 문서는 소유자 경로를 통해 Core에 정보를 줄 수 있지만 Core가 소유한 상태를 대체하지 않습니다.

### Cooperative Guarantee

연결된 에이전트 접점이 하네스 지침과 MCP 결정을 따를 것으로 기대하는 협력형(cooperative) guarantee level입니다. 하네스는 행동을 안내할 수 있지만 hard pre-execution enforcement가 제공되지 않을 수 있습니다.

### Connector Manifest

Connector가 생성하거나 관리하는 path, MCP config snippet, managed block hash, capability/profile 최신성, capture/guard/isolation 설명 또는 mechanism, 수동 fallback 설명, drift 또는 stale status를 기록하는 generated manifest입니다. 생성되거나 관리되는 접점 file이 조용히 overwrite되지 않게 합니다. 전체 manifest contract는 [Agent 통합 참조](agent-integration.md#generated-manifest-기대사항)가 담당합니다.

### Context Hygiene

항상 주입되는 맥락을 짧고 최신으로 유지하는 policy입니다. Compact rule set은 한 화면 이하로 유지합니다. Current status 또는 현재 위치 맥락을 먼저 읽고, Journey Card는 해당 projection/profile이 활성화되어 있고 최신일 때만 사용하며, 더 큰 record는 pull-on-demand로 둡니다. 항상 주입되는 envelope에는 현재 Task 요약, 작업 모양, 범위/하지 않을 일, 대기 중인 사용자 판단, 활성 차단 사유, 다음 안전한 행동, 증거 공백, 닫기 차단 사유, 잔여 위험 요약, 보장 수준, 출처 참조와 최신성만 담습니다. 오래된 PRD, design, log, module map, old projection, closed issue, Reference contract, full artifact contents, future catalog material은 계획/구체화, 쓰기 준비, 실행/Run 기록, 증거 검토, 닫기 준비 상태, 사용자 판단 요청, 복구/오류 또는 verification bundle이 필요로 할 때만 pull합니다. Indexed, retrieved, remembered, summarized context는 ref나 source에 연결된 excerpt로 여기에 포함될 수 있습니다. 무엇을 살펴볼지 정하는 데 도움을 줄 뿐, 무엇이 승인되었는지, 검증되었는지, 결과가 수락되었는지, 요구사항이 면제되었는지, risk-accepted 되었는지, Task가 닫혔는지를 결정하지는 않습니다.

오래된 chat memory는 pull-only context입니다. 담당 소유자 경로가 관련 변화를 기록하지 않는 한 write를 승인하거나, gate를 충족하거나, Task를 close하거나, 결과를 수락하거나, QA 또는 검증을 면제하거나, 잔여 위험을 수락하거나, 현재 상태를 대체하거나, stale projection을 고칠 수 없습니다.

### Context Index

관련 projection, 아티팩트 참조, repo file, doc, note를 보여줄 수 있는 later read-only context provider입니다. 담당 문서로 승격되기 전까지는 로드맵 후보이자 권한 없는 retrieval only입니다. 승격 이후에도 해당 담당 문서가 명시적으로 바뀌지 않는 한 기존 권한 경로를 대체할 수 없습니다. Retrieved context는 살펴볼 source를 가리킬 수 있지만 write를 승인하거나, decision을 해소하거나, Approval을 부여하거나, evidence를 만들거나, verification을 수행하거나, 위험을 수락하거나, gate를 충족하거나, Task를 close하면 안 됩니다. Context Index는 로드맵 후보로 남습니다. [로드맵: 후보 항목 목록](../roadmap.md#후보-항목-목록)을 보고, connector 처리는 [Agent Integration](agent-integration.md#context-pushpull-principles)을 봅니다.

### Decision Gate

진행, write, close 전에 필요한 차단하는 사용자 소유 판단을 나타내는 Task-level aggregate gate입니다. 기준 field는 `decision_gate`이며 value set과 recompute rule은 [Decision Gate](core-model.md#decision-gate)가 담당합니다. 관련 blocking user judgment와 감지된 blockers에서 다시 계산되며 민감 동작 승인, 검증, 수동 QA, 최종 수락, 잔여 위험 수락을 대신하지 않습니다.

### User Judgment

한국어 기준 표현: 사용자 판단.

사용자 소유 판단을 위한 canonical record family입니다. `UserJudgment`는 정확한 질문, `judgment_kind`, `presentation`, pending options 또는 chosen outcome, 영향받는 Task/Change Unit/write/close scope, 영향받는 object refs, supporting refs, recommendation, rationale, uncertainty, no-decision consequence, why the agent cannot decide, owner, status, next action을 이름 붙입니다. Public refs는 `StateRecordRef.record_kind=user_judgment`를 사용합니다. Active stage/profile이 요구하는 user judgment visibility는 Task/status/next/judgment-context와 user-judgment resources로 제공됩니다. Standalone `DEC` Markdown rendering은 enabled된 경우 optional full-format projection입니다.

지원하는 `judgment_kind` 값은 `product_decision`, `technical_decision`, `scope_decision`, `sensitive_approval`, `qa_waiver`, `verification_risk_acceptance`, `final_acceptance`, `residual_risk_acceptance`, `cancellation`입니다.

에이전트는 추천할 수 있지만 사용자 소유 판단은 사용자가 결정합니다. 넓은 승인 표현은 specific pending `judgment_kind`, option, affected object, scope, consequence, "does not settle" boundary에 답하지 않는 한 user judgment를 충족하지 않습니다. "yes, do it", "proceed", "go ahead", "looks good", "좋아", "진행해"는 자동으로 민감 동작 승인, 최종 수락, QA 면제 판단, 검증 위험 수락, 잔여 위험 수락, 취소 판단, 범위 변경이 되면 안 됩니다.

### Judgment Kind Display

한국어 기준 표현: 판단 종류 표시.

Pending user-owned judgment의 구체적인 종류를 보여주는 user-facing label입니다. 새 문서에서는 아래 label만 사용합니다.

- 제품 판단
- 기술 판단
- 범위 판단
- 민감 동작 승인
- QA 면제 판단
- 검증 위험 수락
- 최종 수락
- 잔여 위험 수락
- 취소 판단

민감 동작 승인은 이름 붙은 민감한 단계만 허용합니다. 최종 수락은 사용자의 결과 판단을 기록하며 알려진 잔여 위험을 그 자체로 수락하지 않습니다. 잔여 위험 수락은 수락하는 위험을 이름 붙여야 하며 검증이나 QA가 통과했다는 뜻이 아닙니다. QA 면제 판단은 QA 증거나 통과한 QA 결과를 만들지 않습니다. 검증 위험 수락은 분리 검증을 만들지 않습니다. 범위 확장은 `scope_decision`을 사용하며 넓은 승인이 범위를 확장하지 않습니다.

### Presentation

Prompt 길이와 detail을 제어하는 schema field입니다. Compact one-screen prompt에는 `presentation=short`를 사용하고, full-format Decision Packet-style presentation에는 `presentation=full`을 사용합니다. Presentation은 authority path가 아니며 어떤 judgment가 기록되는지 바꾸지 않습니다.

### Decision Packet

한국어 사용자 표현: 판단 요청 / 판단 요약.

복잡한 `UserJudgment`를 위한 full judgment presentation이며, 오래된 reference에서는 legacy label로도 남아 있습니다. Decision Packet은 active profile이 더 많은 context를 요구할 때 recommendation, uncertainty, detailed trade-offs, evidence, residual risk, approval scope, waiver context, acceptance context, reconcile target을 렌더링할 수 있습니다. 모든 판단의 기본 user-facing mechanism도, 별도 authority record family도, `user_judgment`의 대체물도 아닙니다.

Legacy `decision_packet` refs, `DecisionPacket` shapes, `DEC-*` projection ids는 compatibility 또는 migration note에 남을 수 있습니다. 새 docs, examples, fixtures, payloads는 full-format Decision Packet presentation을 명시적으로 설명하는 경우가 아니면 `user_judgment`와 `UserJudgment`를 사용해야 합니다.

### Judgment Route

오래된 Decision Packet draft의 legacy 또는 implementation routing terminology입니다. 새 public docs에서는 `judgment_route`를 primary concept로 쓰지 않습니다. 오래된 payload가 나타나면 route를 `judgment_kind`과 route-specific payload validation으로 mapping하고 결과를 ordinary language로 설명합니다.

### Display Depth

오래된 Decision Packet draft의 legacy prompt-depth terminology입니다. 새 public docs에서는 `presentation=short` 또는 `presentation=full`을 사용합니다.

### Judgment Category

오래된 Decision Packet draft의 legacy grouping terminology입니다. 새 public docs에서는 `judgment_kind`와 locale에서 파생한 표시 라벨을 사용합니다. 이 old field는 compatibility docs 또는 old payload migration note에서만 나타날 수 있습니다.

### User Judgment Request

Canonical `UserJudgment`를 가리킬 수 있는 optional routing, interaction, idempotency replay, compatibility handoff metadata입니다. Minimal 내부 엔지니어링 점검 구현은 이를 생략할 수 있습니다. User Judgment Request는 judgment authority가 아니며 그 자체로 `decision_gate`, 민감 동작 승인, 최종 수락, 면제 판단, 잔여 위험 수락, close를 절대 충족하지 않습니다. 관문 집계에는 compatible linked `user_judgment_id`가 있을 때만 관련됩니다.

### Decision Request

User Judgment Request의 legacy name입니다. Migration note 또는 old payload compatibility에서만 사용합니다.

### Design Gate

Enabled design-quality policy finding을 라우팅하는 kernel gate surface입니다. 활성 MVP에서 write 또는 close를 기본 차단하는 finding은 [설계 품질 정책: 활성 MVP 차단 집합](design-quality-policies.md#활성-mvp-차단-집합)의 작은 Core-backed 집합뿐입니다. 더 넓은 domain-language, TDD, module/interface, stewardship, feedback-loop, Manual QA, detached-verification catalog finding은 active owner path가 승격하지 않는 한 candidate 또는 advisory/later입니다.

### Design-Quality Policy Pack

Design-quality policy contract, impact class, routed action, severity composition의 담당 문서입니다. Shared design, decision quality, autonomy 경계, domain language, vertical slice, feedback loop, TDD trace, module/interface review, Codebase Stewardship, 수동 QA, context hygiene를 다룹니다. Finding은 허용 route와 active owner path를 통해서만 gate, validator, evidence, user judgment request, residual-risk marker, advisory next action, write blocker, close blocker에 영향을 줄 수 있습니다. Kernel state machine을 재정의하지 않습니다.

### Detached Verification

한국어 기준 표현: 분리 검증.

Fresh session, fresh worktree, sandbox, manual evaluator bundle처럼 의미 있는 독립성 경계를 가로질러 수행되는 분리 검증입니다. 이는 verification independence와 stale-context control을 뒷받침하지만, 자동으로 OS 수준 보안 격리를 뜻하지는 않습니다. Same-session self-review는 분리 검증이 아니며, subagent context도 기본적으로 detached가 아닙니다.

### Discovery

한국어 기준 표현: 요구사항 구체화.

구현 계획과 쓰기 전 범위 확인 전에 에이전트가 요구사항을 구체화하는 작업 자세의 내부 이름입니다. 목표, 사용자 가치, 비목표, 수용 기준, 저장소/문서/하네스 상태에서 에이전트가 확인할 수 있는 사실, 가정, 사용자만 판단할 수 있는 항목, 제품 판단 후보, 기술 판단 후보, 범위 판단 후보, QA와 검증 기대 수준, 남은 불확실성, 안전한 다음 작업 후보 또는 작업 분할 제안을 분리합니다. Codebase와 현재 하네스 맥락이 답할 수 없는 판단만 사용자에게 묻고, judgment area별로 여러 초점을 맞춘 질문을 물을 수 있으며, 확인 가능한 사실과 사용자 소유 판단이 분리되고, 목표/비목표/수용 기준과 중요한 판단 후보가 충분히 분명하며, 해소되지 않은 판단을 숨기지 않고 안전한 다음 작업 또는 작업 분할을 제안할 수 있고, 남은 불확실성이 명시되면 잠시 멈추거나 진행할 수 있습니다. 요구사항 구체화 출력은 Shared Design, user judgment 후보, Change Unit 모양 잡기로 라우팅합니다. `안전한 다음 작업 후보`와 `작업 분할 제안` 같은 표현은 제안 또는 보조 표현이며 독립 schema 필드, 기준 record type, `gate` 값, Projection 종류, 권한 경로가 아닙니다. 이 자세는 일반 동의, 민감 동작 승인, 쓰기 승인 기록, 증거, 검증, QA, 최종 수락, 잔여 위험 수락, 닫기, 범위 권한, 새 권한 경로가 아닙니다.

### Discovery Brief

구체화된 목표, 사용자 가치, 비목표, 수용 기준, 확인 가능한 사실, Question Queue, Assumption Register, 분리된 사용자 소유 판단, 제품 판단, 기술 판단, 범위 판단, QA와 검증 기대 수준, 남은 불확실성, 안전한 다음 작업 후보 또는 작업 분할 제안을 담은 짧은 Discovery 또는 Shared Design 보조 요약입니다. 제품 쓰기가 가까워졌다면 First Safe Change Unit Candidate를 포함할 수 있습니다. 이는 권장 표시 또는 보조 내용이지 독립 schema나 기준 record 필드 목록이 아닙니다. Discovery Brief는 Shared Design, user judgment 후보, Change Unit 모양 잡기에 정보를 줄 수 있지만, 그 자체로 기준 범위를 만들거나, 결정을 해소하거나, 쓰기를 승인하거나, 증거를 증명하거나, 잔여 위험 수락을 기록하거나, 결과를 최종 수락하거나, Task를 닫지 않습니다.

### Detective Guarantee

하네스가 observation 후 violations를 감지하고 상태를 `blocked`, `stale`, `partial`, `failed`로 표시할 수 있는 탐지형(detective) guarantee level입니다.

### Direct

Scope와 result가 명확한 작고 low-risk인 changes를 위한 work mode입니다. Direct product writes에도 active scoped Change Unit이 필요합니다. Direct에는 trivial typo, single-sentence docs, obvious rename work를 위한 tiny direct profile이 포함됩니다. Tiny는 top-level mode가 아니며 사용자 소유 판단, 민감 동작 승인, security boundary, evidence, scope, 쓰기 승인 기록, 잔여 위험 표시, close rule을 우회하지 않습니다.

### Docs-Maintenance Conformance

Bilingual parity, links, owner 경계, stable catalogs, glossary terms, 기준 기록 표현, TODO usage, non-owner duplicate contracts의 drift를 감지하는 read-only documentation maintenance check profile입니다. Rule bodies는 [문서 작성 가이드](../maintain/authoring-guide.md#docs-maintenance-checks)가 담당하고, operator 보고와 entrypoint expectation은 [운영과 Conformance 참조](operations-and-conformance.md#docs-maintenance-프로필)가 담당합니다. Runtime conformance나 Task state 권한이 아닌 docs-only profile입니다.

### Domain Language

Later design/stewardship profile에서 쓰는 Product의 기준 vocabulary와 meanings입니다. 기준 기록은 `domain_terms`이며 Markdown domain-language documents는 projections이자 proposal 접점입니다. 해당 profile이 active일 때 term conflict는 policy validation을 통해 `design_gate`에 영향을 줄 수 있고, meaning 선택이 사용자 소유 제품 판단이나 기술 판단이면 user judgment로 라우팅합니다.

### Domain Term

Product term, meaning, code representation, related terms, source, status, `"not this"` 같은 경계를 저장하는 `domain_terms`의 later/profile 기준 structured record입니다. Public state refs는 owning design/stewardship profile이 active일 때만 `record_kind=domain_term`을 사용합니다.

### Evidence

Work에 대한 주장을 뒷받침하는 기록된 증거입니다. 변경 차이, 로그, 테스트, 실행 요약, 스크린샷, Eval records, 수동 QA 기록, evidence summary, 등록된 아티팩트 참조 등이 여기에 해당합니다. Minimum MVP-1 증거 표시는 `evidence_ref`, Run refs, ArtifactRefs, 보이는 gap을 사용합니다. Evidence summary는 그 ref에서 파생됩니다. Full Evidence Manifest profile은 Evidence Manifest records를 통해 criteria-to-evidence mapping을 더합니다. 에이전트가 작업이 끝났다고 말하는 것 자체나 Markdown 보고서 문장만으로는 Evidence가 충분해지지 않습니다.

### Evidence Gate

Required evidence coverage를 위한 kernel gate입니다. Value set과 닫기 의미는 [Evidence Gate](core-model.md#evidence-gate)가 담당합니다.

### Evidence Manifest

한국어 사용자 표현: 증거 목록.

Acceptance criteria 또는 completion conditions를 이를 뒷받침하는 evidence references에 매핑하는 자세한 증거 목록 state record입니다. Sufficiency는 artifact 개수나 report prose가 아니라, 그 criteria와 conditions가 current owner records와 `ArtifactRef` refs로 뒷받침되는지에 달려 있습니다. Minimum MVP-1은 이 full record를 요구하지 않고 evidence summary, Run refs, ArtifactRefs, 보이는 gap을 표시할 수 있습니다.

### Evidence Profile

Task shape에 충분한 evidence가 무엇인지 validators에 알려주는 named evidence sufficiency profile입니다. 예: `advisor`, `direct docs-only`, `direct code`, `work feature`, `UI/UX/copy work`, `sensitive work`, `verification-required work`. Tiny direct docs-only work는 Direct evidence expectation 아래에서 가장 작은 changed-path, patch-summary 또는 diff-ref, self-check support로 처리되며, 별도 authorization path가 아닙니다.

### Evidence Sufficiency

필수 수용 기준 또는 completion conditions에 compatible current support가 있는지에 대한 close-relevant judgment입니다. Minimum MVP-1은 evidence summary, Run refs, ArtifactRefs, 보이는 gap으로 알려진 증거를 표시합니다. Full criteria-to-evidence sufficiency는 full Evidence Manifest profile이 active일 때만 Evidence Manifest records를 사용합니다. Chat text나 Markdown 보고서 prose만으로 판단하지 않으며, baseline drift, changed files, 민감 동작 승인 또는 Approval drift, missing artifacts, relevant design record changes로 stale이 될 수 있습니다.

### Eval

Verification result record입니다. verdict, checks performed, evidence reviewed, independence qualifier, blockers, 아티팩트 참조를 포함합니다.

### Feedback Loop

Checks와 findings가 상태, 범위, 설계, 증거, 후속 작업, 닫기 상태로 되돌아가는 later/profile 기준 보조 기록이자 기록된 경로입니다. 입력에는 테스트, typecheck, lint, build, 브라우저 간단 확인, TDD red/green/refactor 추적, 수동 QA, Eval findings, 사용자 판단, 운영 발견사항, 잔여 위험 판단이 포함될 수 있습니다. Owning profile이 active일 때만 public refs는 `StateRecordRef.record_kind=feedback_loop`를 사용하며, public mutation은 `record_run`의 `FeedbackLoopUpdate` 또는 수동 QA execution link를 사용합니다. Feedback loops는 findings가 chat 속에서 사라지지 않게 하며, 해당하는 경우 Evidence Manifest 범위, user judgment, Change Unit update, 잔여 위험 record, 수동 QA 또는 Eval record, close blocker, 후속 Task/Change Unit record 같은 기존 소유자 경로로 연결합니다.

### Finding

Run, Eval, 수동 QA 기록, validator, review display, operator diagnostic, conformance check에서 나온 관찰된 issue, gap, risk, blocker, noteworthy result입니다. Finding은 독립 권한 경로가 아니며 chat이나 report prose에 남아 있는 것만으로 `gate` 또는 닫기에 영향을 주지 않습니다. 기존 소유자 기록 또는 구조화된 결과를 통해 라우팅될 때만 상태와 관련됩니다. 예: Evidence Manifest gap, user judgment 후보 또는 record, Change Unit update, Feedback Loop 또는 TDD Trace update, 수동 QA 또는 Eval record, 잔여 위험 record, reconcile item, close blocker, 후속 Task/Change Unit record. 라우팅 계약은 [설계 품질 정책](design-quality-policies.md#finding-라우팅)과 [Core Model 참조](core-model.md#finding-라우팅)가 담당합니다.

### First Safe Change Unit Candidate

제품 쓰기가 가까워졌을 때 안전한 다음 작업 후보를 내부 Change Unit 모양으로 표현한 것입니다. 해소되지 않은 사용자 소유 판단을 숨기지 않고 포함되는 동작, 범위 밖 동작, 완료 조건, 알려진 민감 영역, 중지 조건을 이름 붙여야 합니다. Discovery 또는 Shared Design은 에이전트가 확인할 수 있는 사실과 사용자 소유 판단을 분리하고 안전한 다음 작업이 충분히 분명해진 뒤 이것을 만들 수 있지만, Discovery가 오직 이 후보를 찾기 위해 존재하는 것은 아닙니다. 이는 권장 표시 또는 보조 내용이지 독립 schema나 기준 record 필드 목록이 아닙니다. 이것은 후보일 뿐입니다. 제품 쓰기 전에는 여전히 활성 Change Unit, 호환되는 범위 `gate` 상태, 이후 `prepare_write`가 필요합니다.

### Fixture Assertion Semantics

`expected_state`, `expected_events`, `expected_artifacts`, `expected_projection`, `expected_error`를 captured Core results와 어떻게 match하는지 정하는 conformance comparison rules입니다. [Conformance Fixtures 참조](conformance-fixtures.md#fixture-assertion-semantics)가 담당하며 fixture body 밖에 있고, prose-only matching으로 fixture를 pass시키는 것을 허용하지 않습니다.

### Fresh Session

Evaluator가 lead chat context를 이어받지 않고 task/evidence bundle에서 시작해 Evidence Manifest와 changed files를 검토하고 Eval을 기록하는 verification independence profile입니다.

### Fresh Worktree

Evaluator가 별도 worktree 또는 동등하게 independent repository state에서 baseline, changed paths, artifacts, Evidence Manifest를 확인하는 verification independence profile입니다. Fresh worktree는 scope, freshness, drift detection을 뒷받침할 수 있지만, 자동으로 OS sandbox 격리, 권한 경계, 변조 불가능한 보안 경계가 되지는 않습니다.

### Freeze

Current work 주변의 보류 또는 narrower posture를 request하는 user-facing safety control입니다. Freeze는 product write를 보류하거나, next action을 더 strict하게 만들거나, existing scope가 incompatible할 때 `prepare_write`가 block 또는 보류하게 만들 수 있습니다. Change Unit scope, allowed paths, Autonomy Boundary, AFK stop conditions, related owner records를 직접 변경하지 않습니다. Persistent owner-record change는 existing Core state-changing path, user judgment path, owner-record update path를 사용합니다. Freeze는 쓰기 승인 기록, approval, evidence, verification, QA, 최종 수락, 잔여 위험 수락, close, new authority tier를 만들지 않습니다.

### Gate

Task가 write, proceed, close할 수 있는지 control하는 기준 kernel field입니다. Gates는 state이며 display text가 아닙니다.

### Generated File

Connector, projector, operator tool이 produced한 repository file 또는 managed block입니다. Generated files가 기준 상태에서 drift될 수 있으면 manifest 또는 projection job으로 track해야 합니다.

### Guarantee Display

Status 또는 write decision에 대한 actual guarantee level을 user-facing 및 connector-facing으로 보여주는 display입니다. Enforcement가 cooperative 또는 detective인 경우 limitation notes를 포함합니다.

### Guarantee Level

연결된 접점 또는 runtime path에서 available한 honest enforcement strength입니다.

```text
cooperative | detective | preventive | isolated
```

Capability는 validator results, blocked reasons, display에 영향을 주지만 Approval, 쓰기 승인 기록, verification, QA, 최종 수락, 잔여 위험 수락, 닫기 가능 여부나 닫기 준비 상태, kernel gate는 아닙니다. 정확한 level meanings는 [보안 참조](security.md#정직한-guarantee-display)가 담당합니다.

### Guard

Connected profile의 actual enforcement 또는 detection layer를 적용하는 user-facing safety control입니다. Guard는 협력형(cooperative), 탐지형(detective), 예방형(preventive), 격리형(isolated)일 수 있습니다. Proven `T4` path가 operation을 cover하지 않는 한 이름만으로 실행 전 차단을 imply하지 않습니다.

### 강화된 로컬 기준 목표

영어 label: hardened local reference target.

MVP-1 사용자 작업 루프 이후 담당 문서가 정의한 보증 프로필과 운영 프로필을 완료해 도달하는 로컬 기준 동작 전체입니다. 별도 delivery stage도, 첫 구현 batch도, fixture profile이나 suite name도 아닙니다.

강화된 로컬 기준 목표는 내부 엔지니어링 점검, MVP-1 사용자 작업 루프, 로드맵 경계를 대체하지 않습니다. Conformance는 내부 엔지니어링 점검 fixtures, MVP-1 사용자 작업 루프 fixtures, 보증 프로필 fixtures, 운영 프로필 또는 승격된 로드맵 fixtures라는 이름의 fixture profile로 증명합니다.

### Harness Core

한국어 기준 표현: 하네스 Core.

상태 전이, gate updates, validator interpretation, artifact registration, projection job 대기열 추가, close decisions를 담당하는 runtime component입니다.

### Harness Server

한국어 기준 표현: 하네스 서버.

에이전트 요청을 받고, Core를 통해 상태 변경을 검증하거나 기록하며, validator를 실행하고, 상태 보기와 요약을 만드는 향후 로컬 하네스 프로그램과 도구 접점입니다. 이 문서 저장소의 향후 역할은 하네스 서버 소스 저장소입니다. 제품 저장소나 하네스 런타임 홈은 아닙니다. 아직 이곳에는 하네스 서버/런타임 구현이 없으며, 구현을 시작하려면 문서 수락과 별도의 구현 계획 준비 결정이 모두 필요합니다.

### Harness Runtime Home

한국어 표현: 하네스 런타임 홈.

`registry.sqlite`, per-project `project.yaml`, per-project `state.sqlite`, artifact directories를 포함하는 local runtime storage area입니다.

### Human-editable Area

사람이 notes, proposals, questions, corrections를 쓸 수 있는 Markdown area입니다. Input 접점이지 기준 상태가 아닙니다. Authority path는 `human-editable input -> reconcile_items -> accepted state event/record`입니다.

### Implementation Micro-Plan

작은 execution step 또는 slice, purpose, active Change Unit scope alignment 또는 likely paths, relevant한 경우 selected feedback loop 또는 TDD status, expected evidence, stop condition을 보여주는 managed `TASK` projection section입니다. Execution aid이지 기준 상태, `ProjectionKind`, scope authority, approval, 쓰기 승인 기록이 아닙니다. 이 text를 edit해도 accepted reconcile outcome 또는 Core state-changing action을 통하지 않으면 상태를 변경하지 않습니다.

### Isolated Guarantee

Work 또는 verification이 문서화된 separation boundary 뒤에서 실행되는 격리형(isolated) guarantee level입니다. Worktree 또는 fresh evaluator bundle은 scope, freshness, blast-radius 분리를 제공할 수 있지만, profile이 exact isolation mechanism을 증명하지 않는 한 자동으로 OS sandbox 격리, 권한 경계, 변조 불가능한 보안 경계가 되지는 않습니다. Isolation만으로 민감 동작 승인, verification, 최종 수락, 잔여 위험 수락, close, assurance upgrade가 생기지 않습니다.

### Journey Card

현재 Task 위치를 간결하게 보여주는 human-readable 상태 카드입니다. state, next action, scope, active scoped Change Unit, Autonomy Boundary, blockers, active user judgment, Write Authority Summary, 수용 기준, 민감 동작 승인 status, evidence, verification, QA, 최종 수락, 잔여 위험, 상태 보기 최신성을 포함합니다. Journey Card는 display이며 기준 상태가 아니고, 오래된 chat memory가 아니라 current owner record에서 렌더링됩니다.

### Judgment Category

한국어 기준 표현: 판단 category.

오래된 Decision Packet draft의 legacy grouping terminology입니다. 새 public docs에서는 `judgment_kind`와 locale에서 파생한 표시 라벨을 사용합니다. 이 old field는 compatibility docs 또는 old payload migration note에서만 나타날 수 있습니다.

### Journey Spine

Task의 ordered work journey를 state에서 파생해 이어 주는 continuity model입니다. Chat memory가 아니라 Task, Change Unit, Run, User Judgment, Approval, Evidence Manifest, Eval, 수동 QA, 잔여 위험, `task_gates.acceptance_gate`, 최종 수락 user judgment state, close events, 아티팩트 참조, `state.sqlite.task_events`에서 재구성됩니다. Journey Card와 Journey Spine Markdown views는 projections입니다.

### Journey Spine Entry

Existing state events나 owner records만으로 완전히 재구성하기 어려운 durable continuity annotations를 위한 기준 support record입니다. Journey Spine Entry records는 Journey Spine을 보완하지만 Task, Change Unit, Run, User Judgment, 잔여 위험, evidence, verification, QA, 최종 수락 gate/judgment state, close state/events, artifact, event authority를 대체하지 않습니다.

### Interface Contract

모듈 또는 외부 경계의 공개 인터페이스, 입력, 출력, 오류, 호환성 영향, 호출자, 경계 테스트에 대한 later/profile 기준 record입니다. 기준 기록은 `interface_contracts`입니다. Public state refs는 owning design/stewardship profile이 active일 때만 `record_kind=interface_contract`를 사용합니다. 이 record는 인터페이스 이해를 문서화할 뿐이며 민감 동작 승인, 최종 수락, 잔여 위험 수락, 쓰기 승인 기록이 아닙니다. 공개 인터페이스 또는 호환성 선택에 사용자 소유 판단이 필요하면 기존 design-quality 및 user judgment 경로로 라우팅합니다.

### JSON `TEXT` Field

저장된 값이 JSON인 SQLite `TEXT` column입니다. `TEXT` type은 reference storage flexibility일 뿐입니다. Core는 commit 전에 API-owned 또는 storage-owned shape에 맞게 값을 검증해야 하며, malformed JSON 또는 schema-incompatible JSON은 invalid state입니다.

### Local Derived Metrics

`state.sqlite.task_events`, runs, validator results, projection jobs, reconcile items 같은 local record에서 파생되는 later diagnostic-only metric입니다. Owner 문서로 승격되기 전까지 metric 표시는 rate, count, duration, guard-trigger summary를 읽기 전용 진단으로만 보여줄 수 있습니다. Local Derived Metrics는 로드맵 후보로 남습니다. [로드맵: 후보 항목 목록](../roadmap.md#후보-항목-목록)을 봅니다.

### Manual QA

한국어 기준 표현: 수동 QA.

수동 QA는 UX, 작업 흐름, 문구, 시각 결과, 접근성, 제품 적합성처럼 사람이 판단해야 하는 경험 품질을 사람이 확인한 기록입니다. 필수일 때는 수동 QA 기록 또는 유효한 QA 면제 판단 경로로 기록됩니다. 브라우저 간단 확인, 스크린샷, 브라우저 QA 아티팩트, 테스트, 검증자 메모는 맥락을 뒷받침할 수 있지만 그 자체로 수동 QA 판단이 아닙니다. 정확한 `gate` 동작은 [QA Gate](core-model.md#qa-gate)가 담당하고, 정책 요구사항은 [설계 품질 정책](design-quality-policies.md#수동-qa-manual_qa)이 담당합니다.

### Manual Bundle

Human 또는 separate evaluator에게 verification을 handoff하는 package입니다. task summary, 수용 기준, Change Unit scope, approval scope, diff/log/test artifacts, Evidence Manifest, known risks, Eval verdict를 기록하기에 충분한 context를 포함합니다.

### Manual QA Record

한국어 기준 표현: 수동 QA 기록.

기록 수준의 수동 QA 결과입니다. 수행자, 프로필, 결과, 아티팩트, 발견사항, 해당하는 경우 면제 이유, 다음 행동을 포함합니다. 결과 값 집합은 [QA Gate](core-model.md#qa-gate)와 later/profile-gated [`harness.record_manual_qa`](api/schema-later.md#harnessrecord_manual_qa)가 담당합니다. 대기 중인 필수 QA는 `qa_gate=pending`으로 표현하며 수동 QA 기록 결과가 아닙니다.

### `managed_hash`

`HARNESS:BEGIN`과 `HARNESS:END` marker lines를 제외한 projector-owned managed block body의 drift-detection hash입니다. 기준 상태가 아니며 Markdown 상태 보기나 요약을 authoritative하게 만들지 않습니다.

### Managed Block

하네스 markers로 delimit되고 projector가 state records와 아티팩트 참조에서 regenerate하는 Markdown block입니다. Managed block에 대한 direct edits는 drift 또는 reconcile candidates를 만들며 그 자체로 state가 되지 않습니다.

### MCP Resource

Current project, task, design, policy, status, bundle information을 위한 read-only MCP 접점입니다. Resources는 상태를 변경하지 않습니다.

### MCP Server Unavailable

`MCP_SERVER_UNAVAILABLE`은 tool call이 Core에 닿을 수 없는 diagnostic condition입니다. Authoritative Core response가 불가능하며, caller는 상태 변경을 주장하기 전에 diagnose 또는 reconnect해야 합니다. Stable public error code는 계속 `MCP_UNAVAILABLE`입니다.

### Surface MCP Unavailable

`SURFACE_MCP_UNAVAILABLE`은 Core 또는 operator가 연결된 접점에서 사용할 수 있는 MCP가 없거나, MCP configuration이 최신이 아니거나, required MCP tools를 호출할 수 없음을 관찰할 수 있는 diagnostic condition입니다. Product writes는 cooperative 접점에서는 instruction으로 보류되고 available한 stronger guard에서는 차단됩니다. Core responses는 `details.mcp_unavailable_kind`와 함께 `MCP_UNAVAILABLE` 또는 `CAPABILITY_INSUFFICIENT`를 사용할 수 있으며, 이 diagnostic label은 public `ErrorCode` value가 아닙니다.

### MCP Tool

Core에 상태를 검증, 기록, 전이, 닫기 처리하도록 요청하는 public MCP operation입니다. 상태 변경은 resource reads가 아니라 tools 또는 reconcile actions를 통해야 합니다.

### Markdown Report

State records와 아티팩트 참조에서 generated된 human-readable document입니다. Markdown 보고서는 기본적으로 projection이며 기준 상태나 기준 증거가 되지 않습니다.

### Natural-Language Consent

한국어 기준 표현: 자연어 동의.

"yes, do it", "go ahead", "proceed", "looks good", "좋아", "진행해" 같은 사용자 발화는 활성 질문이 정확한 `judgment_kind`, 선택지, 영향받는 object, 범위, 영향을 받는 `gate`, 결과, 그리고 아직 승인/수락/면제/취소되지 않은 항목을 모호하지 않게 보여줄 때만 대기 중인 질문에 대한 답으로 사용할 수 있습니다. 자연어 동의는 독립 권한 경로가 아닙니다. 모호한 동의는 민감 동작 승인, 최종 수락, 잔여 위험 수락, QA 면제 판단, 검증 위험 수락, 취소 판단, 범위 변경, 쓰기 승인 기록으로 넓혀 해석하지 말고 다시 확인해야 합니다.

### Module Map

Later/profile에서 쓰는 Product의 modules, responsibilities, public interfaces, dependency direction, internal complexity, test 경계, owner 결정, watchpoints map입니다. 기준 기록은 `module_map_items`입니다. Module boundary update는 공유된 technical understanding을 기록할 뿐이며 write를 승인하거나 risk를 수락하지 않습니다. Boundary change가 product commitment, caller obligation, architecture direction을 바꾸고 사용자 소유 판단이 필요하면 design-quality policy와 user judgment path로 라우팅합니다.

### Module Map Item

Module role, public interface, dependencies, internal complexity, test 경계, owner 결정, watchpoints를 저장하는 `module_map_items`의 later/profile 기준 structured record입니다. Public state refs는 owning design/stewardship profile이 active일 때만 `record_kind=module_map_item`을 사용합니다.

### Policy Contract

Design-quality policies가 사용하는 standard form입니다. `name`, `applies_when`, `default_requirement`, `allowed_waiver`, `required_record`, `validator`, `evidence`, `close_impact`를 포함합니다.

### Preventive Guarantee

하네스 또는 connector가 covered violating action을 실행 전에 차단할 수 있는 예방형(preventive) guarantee level입니다. 해당 exact path에 대한 owner-defined mechanism과 fixture proof가 있어야 합니다. 이 label은 무엇이 covered인지 이름 붙여야 하며, 임의 도구 차단, OS sandboxing, 권한 격리, 변조 방지 storage, 더 넓은 authority를 뜻하지 않습니다.

### Product Repository

한국어 기준 표현: 제품 저장소.

사용자의 실제 제품 작업 공간입니다. 소스 코드, 테스트, 제품 문서, 제품 저장소에 쓰이는 하네스 요약이나 보고서가 여기에 속합니다. 제품 저장소는 제품 내용의 기준 위치로 남습니다. 하네스 런타임 홈이 아니며, 제품 파일은 기존 Core, artifact-registration, reconcile, owner-record path가 관련 Harness fact를 기록할 때만 하네스 운영 사실이 됩니다.

### Projection

한국어 기준 표현: 상태 보기 / 요약 / 상태 카드.

Projection은 Core 상태 record와 아티팩트 참조에서 생성된 파생 보기입니다. 상태 보기, 요약, 상태 카드, Markdown 보고서가 될 수 있습니다. 읽고 판단하는 데 유용하지만 기준 상태를 덮어쓰거나 대체할 수 없습니다.

### ProjectionKind

Projection job과 template kind를 나타내는 API enum입니다. Support class, active value, extension rule은 [API Schema Core](api/schema-core.md#projectionkind-support)가 담당하고, later/profile-gated value는 [API Schema Later](api/schema-later.md#full-profile-gated-ref-values)에 남습니다. Support class label은 내부 엔지니어링 점검 run obligation이 아닙니다. 내부 엔지니어링 점검에는 소유자 경로가 이미 만든 freshness/read fact를 보존하는 것 외의 projection rendering exit requirement가 없습니다. 어떤 ProjectionKind도 Projection을 기준 상태로 만들지 않습니다.

### Projection Freshness

Projection과 source record, managed hash, 아티팩트 참조, projection job state 사이의 관계입니다. Value set은 [API Schema Core](api/schema-core.md#projectionkind-support)와 [Projection과 Template 참조](projection-and-templates.md)가 담당합니다.

### Projection Job

Committed state records와 아티팩트 참조에서 Markdown 상태 보기나 요약을 렌더링하도록 projector에 요청하는 later/profile-promoted 지속 outbox record입니다. MVP-1의 작은 보기 output에는 `projection_jobs` table이 필요하지 않습니다. Projection job profile이 active일 때 `record_kind=projection` identity는 `projection_jobs.projection_job_id`입니다. Project-level projection jobs는 현재 Task-scoped artifact DDL에서 그 자체로 project-scoped artifact links를 만들지 않습니다.

### Question Queue

열린 질문을 blocking, useful-but-not-blocking, codebase-answerable로 분류한 Discovery 또는 Shared Design 보조 상태 보기/요약 목록입니다. 이는 권장 표시 또는 보조 내용이지 독립 schema나 기준 record 필드 목록이 아닙니다. Blocking question은 사용자 소유 판단이 필요할 때 user judgment 후보로 라우팅될 수 있습니다. Useful-but-not-blocking question은 남겨 두거나, defer하거나, 후속 작업으로 바꿀 수 있습니다. Codebase-answerable question은 사용자에게 묻지 말고 현재 저장소, 문서, 하네스 상태, 출처 참조에서 답해야 합니다. Queue는 user judgment, `gate`, 민감 동작 승인, 증거, 최종 수락, 닫기, 쓰기 승인 기록이 아닙니다.

### QA Gate

Required 수동 QA를 위한 기준 kernel gate입니다. `manual_qa_record.result`는 record-level이고, `qa_gate`는 close-relevant aggregate state입니다.

`qa_gate=pending`은 required QA가 충족 기록을 아직 만들지 못했거나 latest relevant record가 policy를 충족하지 못한다는 뜻입니다.

### Raw Artifact

Harness staging, approved capture adapter, 또는 이미 commit된 artifact ref에서 등록된 뒤 artifact store에 보관되는 지속 증거 파일입니다. Diff, log, bundle, 화면 캡처, checkpoint, 매니페스트 파일이 여기에 속할 수 있습니다. 등록된 아티팩트 파일은 state records와 Markdown 보고서와 구분됩니다. Close-relevant evidence가 의존하려면 `ArtifactRef`, owner relation, integrity, redaction, retention metadata가 필요합니다.

### Reconcile

사람이 편집할 수 있는 입력 또는 projection drift를 accepted state change, rejected proposal, note, decision, deferred item으로 바꾸는 process입니다.

### Reconcile Item

Reconcile decision이 accept, reject, convert, defer하기 전에 사람이 편집할 수 있는 입력 또는 projection drift에서 생성되는 기준 candidate record입니다.

### Reference Surface

내부 엔지니어링 점검이 target하는 단일 에이전트 접점입니다. Kernel과 커넥터 계약을 demonstrate하기 위한 범위이며 broad connector-surface support를 뜻하지 않습니다.

### Recommended Playbook

Current state와 policy/playbook context에서 계산되는 non-authoritative status/next display guidance입니다. Current stage에 맞는 procedure를 제안하며 review, TDD, QA, guard check, release handoff, browser-QA candidacy 같은 항목을 제안할 수 있습니다. `playbook_id`는 stable display/routing string identifier이지 Core-owned closed enum이나 DDL-backed value set이 아닙니다. 기준 kernel record가 아니고 자체 DDL table, task event, projection job이 없으며 write 권한을 만들거나, gate를 충족하거나, 결과를 수락하거나, 잔여 위험을 수락하거나, Task를 close하지 않습니다. 사용자 소유 판단이나 필요한 동작은 user judgment path 또는 다른 기존 Core/MCP mutation path로 라우팅합니다.

### Release Handoff

External PR, review, deployment, rollback, monitoring process를 위한 release readiness를 요약하는 optional 보고서/export profile입니다. Close readiness, blocker, evidence ref, verification ref, 수동 QA ref, residual-risk ref, changed file, 상태 보기 최신성, redaction note, suggested checklist item을 포함합니다. 정확한 보고서/export 권한 경계는 [Operations And Conformance](operations-and-conformance.md#release-handoff-export-profile)가 담당합니다.

### Role Lens

사용자가 product, engineering, design, security, QA, release-handoff 검토 관점을 요청할 수 있게 하는 non-authoritative skill 또는 playbook 접점입니다. Role Lens output은 `RecommendedPlaybook`, `UserJudgmentCandidate`, validator/check route, evidence, Eval 또는 verification, 수동 QA, Approval, residual-risk, Change Unit update, release handoff route 같은 existing route를 재사용합니다. 기존 Core/MCP path가 underlying action을 기록하기 전까지 read-only guidance이므로, 그 자체로 state를 mutate하거나, write를 승인하거나, gate를 충족하거나, 결과를 수락하거나, 잔여 위험을 수락하거나, Task를 close하거나, assurance를 올리지 않습니다. 정확한 권한 없음 경계는 [Agent Integration](agent-integration.md#role-lens-동작)이 담당합니다.

### Report Projection

Task 보고서, approval 보고서, run summary, 증거 목록 보고서, Eval 보고서, direct-result 보고서처럼 state records와 아티팩트 참조에서 생성되는 Markdown 보고서입니다.

이름 있는 보고서 ProjectionKind 값은 state records와 아티팩트 참조에서 생성되는 Projection입니다. State authority는 Core records에 남고, evidence-file authority는 등록된 아티팩트 파일에 남습니다. 정확한 Projection rule은 [Projection과 Template 참조](projection-and-templates.md)가 담당하며, 전체 rendered body는 [Template 참조](templates/README.md)가 담당합니다.

### Review Stages

Spec Compliance Review와 Code Quality / Stewardship Review를 분리하는 관리형 표시/절차 구분입니다. Spec Compliance Review는 현재 하네스 권한 안에서 요청한 작업이 완료됐는지 묻습니다. Code Quality / Stewardship Review는 구현이 코드베이스 안에서 유지보수하기 좋은지 묻습니다. Review Stages는 발견 사항을 validator results, 증거 공백, user judgment 후보, Eval 또는 verification 필요, 수동 QA 필요, 민감 동작 승인 필요, 해당 profile이 active일 때 later Approval 필요, 잔여 위험 후보, Change Unit 갱신 추천, 닫기 차단 사유로 라우팅할 수 있습니다. 기준 기록, `ProjectionKind` value, 민감 동작 승인 / Approval, evidence, verification, QA, 최종 수락, 잔여 위험 수락, close, 쓰기 승인 기록은 아닙니다. 정확한 표시 전용 경계는 [Design Quality Policies](design-quality-policies.md#two-stage-review-display)가 담당합니다. Same-session Review Stages는 `assurance_level=detached_verified`를 만들지 않습니다.

### `request_hash`

`tool_name`, schema-normalized request body, `request_id`와 `idempotency_key`를 제외한 envelope fields를 포함하는 정규화된 UTF-8 JSON에서 계산하는 tool request idempotency hash입니다.

### Residual Risk

한국어 기준 표현: 잔여 위험 / 남은 위험.

잔여 위험은 Evidence, verification, QA, 최종 수락 검토 이후에도 남는 알려진 불확실성, 절충, 한계, 확인하지 못한 조건을 위한 기준 닫기 관련 보조 기록입니다. 출처 참조, 영향받는 범위, 해당하는 경우 관련 user judgment, 표시 상태, 수락된 위험, 후속 작업 필요 여부, 닫기 영향을 기록합니다. 닫기에 영향을 주는 것으로 알려진 잔여 위험은 성공적인 최종 수락 또는 close 전에 보여야 하며, 알려진 닫기 관련 위험이 없으면 `ResidualRiskSummary.status=none`으로 확인되어야 합니다. 잔여 위험 수락은 사용자가 이름 붙은 알려진 잔여 위험을 명시적으로 수락하는 판단입니다. 이는 결과가 검증됐거나, 최종 수락됐거나, 민감 동작이 승인됐거나, 면제 판단이 끝났다는 뜻이 아닙니다. 현재 참조 모델에서 수락된 위험은 잔여 위험 record 위의 메타데이터/상태이며 별도의 `accepted_risk` state record가 아닙니다.

### Risk Accepted Close

사용자가 표시된 닫기 관련 잔여 위험을 수락하는 성공적인 close입니다. Verification 위험을 사용자가 수락한 경우도 포함합니다. `close_reason=completed_with_risk_accepted`를 사용합니다. MVP-1에서는 compatible `judgment_kind=residual_risk_acceptance` user judgment와 visible blocker/evidence ref가 필요하고, rich Residual Risk ref는 later/profile-promoted입니다. `assurance_level=detached_verified`로 표시하면 안 됩니다. 사용자용 요약은 이를 일반 `completed_verified` 또는 `completed_self_checked` close와 구분해야 합니다.

### Run

에이전트, evaluator, operator, 기타 actor가 Task와 optional Change Unit에 대해 수행하는 execution attempt입니다. Run은 baseline, 접점, observed changes, commands, artifacts, summary를 기록합니다. Rejected pre-commit `record_run` request는 Run이 아니며 fabricated Run ID를 받으면 안 됩니다. Audit 또는 violation attempt는 Core가 deliberate하게 commit할 때만 Run이 됩니다.

### Scope Gate

Product writes가 active scoped Change Unit으로 covered되어야 함을 요구하는 kernel gate입니다. Approval이 required가 아니어도 write-capable `direct`와 `work` modes에는 scope가 required입니다. Scope Gate는 민감 동작 승인을 부여하거나, 사용자 소유 판단을 해소하거나, 쓰기 승인 기록을 만들지 않습니다. Exact values와 compatibility는 [Scope Gate](core-model.md#scope-gate)가 담당합니다.

### Severity Composition

여러 applicable task-shape default, policy contract, validator finding을 merge하는 policy-owned rule입니다. Same concern은 전체 Task나 단순히 같은 validator ID가 아니라 같은 policy-relevant target입니다. 이 rule은 모든 finding을 보이게 유지하고, 서로 다른 affected gate 또는 blocker target의 impact를 보존하며, 같은 concern에서 경쟁하는 impacts에만 가장 강한 applicable impact를 사용합니다. Validators, gates, write blockers, close blockers, waivers, user judgment needs에 영향을 주지만 public primary `ToolError` 선택은 API가 소유합니다. 정확한 policy behavior는 [Severity composition rule](design-quality-policies.md#severity-composition-rule)이 담당합니다.

### Shared Design

구현이 계획으로 굳어지기 전에 Task에 대해 최소한으로 기록한 공유된 이해입니다. 목표, 사용자 가치, 범위, 비목표, 수용 기준, 확인 가능한 사실, 가정, 결정, 거부한 선택지, 남은 불확실성, 도메인/모듈/인터페이스 영향, QA와 검증 기대 수준, 안전한 다음 작업을 포함합니다. Discovery Brief, Question Queue, Assumption Register, 안전한 다음 작업 후보 또는 작업 분할 제안, First Safe Change Unit Candidate가 Shared Design에 입력될 수 있습니다. Shared Design은 모양 잡기와 `design_gate` 준비 상태를 도울 수 있지만 민감 동작 승인, 최종 수락, 잔여 위험 수락, QA 판단, 증거, 닫기 준비 상태, 쓰기 승인 기록은 아닙니다. Shared Design의 Markdown 렌더링 결과는 파생 보기(Projection)이자 제안용 접점입니다. 정확한 정책 요구사항은 [설계 품질 정책](design-quality-policies.md#shared-design-shared_design)이 담당합니다.

### Source-of-truth

어떤 fact에 대한 기준 정보입니다. 하네스에서 운영 상태의 기준 기록은 `state.sqlite` current records입니다. `state.sqlite.task_events`는 audit와 ordering history이며 일반적인 current-state source가 아닙니다. Raw evidence files의 기준 위치는 artifact store이며, Markdown documents는 projections입니다. Product repository files는 product content의 source로 남습니다. Existing Core, reconcile, artifact-registration, owner-record path가 관련 하네스 fact를 기록하기 전까지는 하네스 operational state가 되지 않습니다.

### `state.sqlite.task_events`

`state.sqlite` 안의 추가 전용 event history table입니다. Reference event storage는 별도의 event store를 사용하지 않습니다. Deterministic order는 timestamp나 event ID가 아니라 `task_events.event_seq`입니다.

### Stable Event Catalog

Staged/reference conformance fixtures가 `expected_events`에서 검증할 수 있는 `task_events.event_type` names에 대한 kernel-owned compact list입니다. Stable event names를 prose examples, fixture shorthand, non-stable implementation-local detail 또는 audit events, validator IDs, Core check names, projection status shorthands, future extension events와 구분합니다.

### State Record

Kernel state 안의 기준 structured record입니다. Task, Change Unit, User Judgment, Journey Spine Entry, 잔여 위험, Run, Approval, 쓰기 승인 기록, Evidence Manifest, Eval, 수동 QA 기록, Artifact record, Shared Design record, Domain Term, Module Map Item, Interface Contract, Feedback Loop, TDD Trace, Reconcile Item 등이 있습니다.

### State Version

Core-resolved state scope를 위한 optimistic-concurrency clock입니다. 적용되는 경우 Core는 tool-specific `task_id`, `ToolEnvelope.task_id`, active Task resolution 순서로 primary Task를 찾습니다. `expected_state_version`, `ToolResponseBase.state_version`, `EventRef.state_version`, `task_events.state_version`은 하나의 global event-store sequence가 아니라 해당 영향받는 scope에 따라 해석됩니다.

### Project State Version

`project_state.state_version`에 저장되는 project-scoped state clock입니다. Core-resolved primary Task가 없는 project-scoped mutations는 `expected_state_version`을 이 값과 비교하고 resulting value를 primary response `state_version`으로 반환합니다.

### Task State Version

`tasks.state_version`에 저장되는 task-scoped state clock입니다. Task-scoped mutations는 `expected_state_version`을 Core-resolved primary Task의 값과 비교하고 resulting value를 primary response `state_version`으로 반환합니다.

### Strategic Agency

사용자가 작업 여정을 이해하고 목표, 범위, 설계, 장단점 비교, Codebase Stewardship, QA, 최종 수락, 잔여 위험에 대해 판단하거나 보류할 수 있는 지속적인 권한입니다. 하네스는 chat 밖에서 state, decisions, evidence, blockers, remaining risk를 명시해 Strategic Agency를 보존합니다.

### Secret Handle

Credential, token, certificate, key, 기타 secret value 같은 민감한 material을 원문 값 없이 가리키는 표시 안전 참조입니다. Secret handle은 raw secret을 artifact, connector manifest, projection, export, 화면 캡처, log, summary, 모델에 전달되는 맥락에 저장하지 않고 evidence 또는 approval scope를 뒷받침할 수 있습니다. Exact storage behavior는 [Storage](storage.md)가 담당하고, exact API behavior는 [MVP API](api/mvp-api.md)와 [API Schema Core](api/schema-core.md)가 담당합니다.

<a id="security-threat-model"></a>

### 보안 참조

하네스 security asset, trust boundary, threat category, control expectation, guarantee level, 정직한 보안 표현을 담당하는 reference owner입니다. Repo docs의 prompt injection, projection tampering, stale approval replay, out-of-scope write, MCP unavailable 상태에서의 state claim, evidence artifact를 통한 secret leakage, artifact `hash_mismatch`, 악성 generated connector 파일, capability overclaiming, stale context poisoning 같은 위험을 설명합니다. Exact DDL, 공개 API 스키마, Core Model transition은 담당하지 않습니다.

### Surface Capability Check

연결된 에이전트 접점의 required 하네스 behavior 충족 가능성을 보고하는 validator입니다. Blocked reasons와 guarantee display에 영향을 주지만 kernel gate는 아닙니다.

### Surface Cookbook

접점별 connector notes, generated file details, profile examples를 담은 reference 문서입니다. Common integration rules는 cookbook이 아니라 에이전트 통합 문서에 둡니다.

### Subagent Context

Subagent 또는 helper가 일부 inherited implementation context를 가지고 work를 review하는 verification independence profile입니다. 기본적으로 detached가 아니며, stricter profile metadata가 실제 독립성 경계를 입증할 때만 qualify될 수 있습니다.

### Task

Kernel이 추적하는 사용자 가치 단위입니다. mode, 상태 흐름 단계, gates, result, close reason, assurance, current summary, decisions, evidence, projection status를 가집니다.

### Task Level

Task shape를 표시하고 routing하기 위한 label입니다. Tiny, Direct, Work, High-risk Work가 있습니다. Tiny는 `direct` 아래 profile입니다. Direct는 작고 low-risk인 code 또는 docs work입니다. Work는 feature, UX workflow, auth-facing behavior, schema, public API/interface, multi-file 또는 multi-step delivery를 다룹니다. High-risk Work는 auth, security, privacy, secrets, infra, 비슷하게 민감한 category를 다룹니다. Task Level은 새 kernel `mode` enum, gate, schema field, approval, 쓰기 승인 기록 source가 아닙니다.

### TDD Trace

Change Unit 또는 behavior slice에 대한 red, green, refactor evidence record 또는 policy가 허용하는 recorded non-TDD justification입니다. RED target 또는 plan은 intended failing check를 설명하고, RED evidence는 actual failing test artifact/log/result 또는 policy가 명시적으로 인정한 failing-check evidence를 뜻합니다. Required인 경우 normal path는 non-test implementation write 전에 RED evidence를 기록하고, implementation 후 GREEN evidence를 기록하며, relevant한 경우 refactor/check evidence를 기록한 뒤 trace를 Evidence Manifest coverage에 link합니다. TDD Trace는 Feedback Loop의 execution evidence가 될 수 있지만 기준 selected-loop record는 아닙니다. Waiver는 behavior를 증명할 alternate Feedback Loop로 돌아가는 ref를 가져야 합니다.

### Tiny Direct Profile

Typo, 문서 한 문장, obvious rename처럼 scope, result, 사용자 판단이 필요 없다는 경계가 즉시 분명한 Direct 하위 profile입니다. Interaction을 최소화하지만 scope가 넓어져도 여전히 low-risk이고 좁거나, Evidence Manifest coverage, 아티팩트 참조, link/render proof, 또는 tiny result note를 넘는 다른 evidence가 필요하면 일반 Direct로 상향해야 합니다. 제품 판단, 기술 판단, architecture choice, public interface/API impact, UX workflow, schema, sensitive category, multi-step delivery가 나타나면 Work로 라우팅해야 합니다.

### Trust Boundary

하네스 surface, file, caller, runtime space 사이의 분리입니다. 한쪽의 input은 소유자 경로 없이 다른 쪽의 authority로 취급하면 안 됩니다. 예를 들어 chat text, 제품 저장소 문서, projection, generated connector 파일, artifact bytes, MCP caller claim은 하네스에 정보를 줄 수 있지만, Core 또는 문서화된 다른 소유자 경로가 그 의미를 받아들이기 전까지 canonical operational state가 되지 않습니다. Trust-boundary map은 [보안 참조](security.md)가 담당합니다.

### Verification

결과가 관련 기준을 충족하는지 확인하는 과정입니다. Verification은 valid Eval path와 independence profile을 통해 기록될 때 assurance를 뒷받침할 수 있지만, same-session self-check는 분리 검증이 아닙니다. Verification은 approval, 수동 QA, 최종 수락, 잔여 위험 수락과 구분됩니다. 정확한 gate와 independence behavior는 [Verification Gate](core-model.md#verification-gate)와 later/profile-gated [`harness.record_eval`](api/schema-later.md#harnessrecord_eval)이 담당합니다.

### Verification Gate

Required verification을 위한 kernel gate입니다. User waiver는 `verification_gate=waived_by_user`를 set하지만 `detached_verified` assurance를 만들지 않습니다.

### Verification Independence Profile

Eval independence context의 named minimum qualification입니다. 예: `same_session`, `subagent_context`, `fresh_session`, `fresh_worktree`, `sandbox`, `manual_bundle`. Passed Eval은 `detached_verified`를 뒷받침하기 전에 valid profile을 만족해야 하며, 보안 격리 주장은 profile이 별도로 이름 붙이고 증명해야 합니다.

### Validator Result

Validator의 structured result입니다. status, guarantee level, target, findings, blocked reasons, suggested next action을 포함합니다.

### Vertical Slice

Trigger/input에서 도메인 로직, persistence 또는 state, caller/API 경계, 관찰 가능한 출력, 테스트, 선택적 수동 QA까지 얇은 경로를 연결하는 Change Unit shape입니다.

### Waiver

한국어 기준 표현: 면제 판단.

Policy가 허용하는 gate 또는 policy requirement에 대한 명시적으로 기록된 예외입니다. 면제 판단은 policy 또는 gate, Task와 Change Unit, 생략하는 check 또는 surface, reason, actor, 필요할 때 expiry 또는 follow-up, 영향받는 gate 또는 닫기 영향, 그리고 필요할 때 잔여 위험 경로로 보여주거나 수락해야 하는 close-relevant 잔여 위험을 이름 붙입니다. QA 면제 판단은 정의된 rules 아래 명시적이고 범위가 정해진 경우에만 허용됩니다. 검증을 수행하지 않은 위험을 사용자가 수락하는 경우에는 `judgment_kind=verification_risk_acceptance`로 기록하고, 그 판단은 분리 검증을 만들지 않습니다. Successful completion을 위해 product-write scope, 민감 동작 승인, required evidence coverage, required 최종 수락은 면제되지 않습니다. QA 면제 판단은 assurance를 높이거나, 최종 수락을 암시하거나, unrelated 잔여 위험을 수락하거나, 생략된 check가 passed된 것처럼 만들지 않습니다.

### Write Authorization

한국어 사용자 표현: 쓰기 전 범위 확인. 내부 기록 표현: 쓰기 승인 기록.

쓰기 승인 기록은 stored `AuthorizedAttemptScope` 하나에 대해 `dry_run=false`인 `prepare_write.decision=allowed`일 때만 만드는 내부 협력형 durable state record입니다. Stored scope는 Core가 나중에 `record_run`에서 비교할 intended operation, paths, tools, commands와 command classes, product-file-write intent, network targets, secret handles 또는 scope, sensitive categories, baseline, Task, Change Unit, `basis_state_version`, `surface_id`, related user judgment refs, `guarantee_level`을 보존합니다. 서로 다른 compatible `dry_run=false` `prepare_write` request는 서로 다른 active authorization을 만듭니다. Dry-run allowed response는 candidate일 뿐이고, idempotent replay는 original committed response를 반환합니다. Committed implementation 또는 direct Run에 single-use이며, Change Unit scope, 민감 동작 승인, user judgment compatibility, evidence, verification, 수동 QA, 최종 수락, 잔여 위험 표시를 대체하지 않습니다. OS 권한, sandboxing, 변조 방지 enforcement, 사전 차단, 격리가 아닙니다.

### Write Authorization Lifecycle Events

내부 기록 표현: 쓰기 승인 기록 상태 흐름 이벤트.

쓰기 승인 기록 생성, 반환, 사용, 만료, 오래됨, 철회, 위반 감지를 위한 stable event-name set입니다. Exact vocabulary와 `scope_violation_detected`와의 관계는 [Core Model Stable Event Catalog](core-model.md#stable-event-catalog)가 담당합니다.

### Write Authority Summary

의도한 동작에 대해 현재 쓰기 전 범위 확인과 쓰기 승인 기록 상태를 보여주는 user-facing display summary입니다. Active Change Unit scope, `prepare_write`, approval, baseline, guarantee, user judgment refs, 쓰기 승인 기록 ref에서 파생됩니다. 별도 authority record가 아닌 display이며 그 자체로 work 권한을 부여하지 않습니다.
