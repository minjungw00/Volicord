# Later 후보 색인

이 문서는 현재 활성인 유일한 later 후보 색인입니다. 상세 later profile 문서, later schema appendix, future fixture catalog, 예전 로드맵 서사, later-profile 템플릿 본문을 이 색인으로 접었습니다.

아래 항목은 승격 전 후보 목록입니다. MVP-1 요구사항, 활성 API/schema 계약, fixture 본문, template 본문, 런타임 동작, 구현 작업, 생성 산출물, 수락 증거가 아닙니다. 상세 본문 없음.

<a id="boundary"></a>
## 1. 경계

| later 후보 | 현재 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| Later candidate index | 활성 문서 색인일 뿐입니다. | 세부 계약을 되살리려면 future owner가 좁은 후보를 먼저 승격해야 합니다. | 없음 |
| Current repository phase | 문서 전용 계획/검토 기준입니다. | 런타임 작업 전 문서 수락과 별도의 구현 계획 준비 결정이 필요합니다. | 없음 |
| Candidate authority | 후보 row는 Core state, evidence, verification, QA, final acceptance, residual-risk acceptance, close readiness, conformance, security guarantee를 만들지 않습니다. | 승격된 owner path의 owner record, exact contract, proof가 필요합니다. | 없음 |
| Bilingual parity | 영어와 한국어 색인은 같은 후보 묶음과 승격 경계를 담습니다. | 의미가 바뀌면 같은 batch에서 양쪽을 함께 고칩니다. | 없음 |

<a id="promotion-rule"></a>
## 2. 승격 규칙

| later 후보 | 현재 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| Owner decision | 후보는 승격 전까지 보관 상태입니다. | 이름 있는 future stage/profile owner, 좁은 범위, fallback behavior, 명시적인 non-claim이 필요합니다. | 없음 |
| Contract placement | 여기에는 full later schema, template body, fixture body, scenario script가 없습니다. | Exact API, schema, storage, security, projection, fixture, template, operator contract는 알맞은 owner 문서에 둡니다. | 없음 |
| Proof expectation | 후보로 적혀 있다는 사실은 proof가 아닙니다. | 정확히 승격된 동작에 대한 fixture/conformance target 또는 owner가 정한 증거가 필요합니다. | 없음 |
| Non-substitution | 후보 output은 Core state, user judgment, evidence, verification, Manual QA, final acceptance, residual-risk acceptance, close readiness를 대신할 수 없습니다. | Core-owned record와 owner별 lifecycle rule을 계속 분리해야 합니다. | 없음 |
| Early-stage inflation check | Later material은 기본적으로 Engineering Checkpoint와 MVP-1 밖에 남습니다. | 승격 후보가 earlier stage에 unsupported requirement를 더하지 않는다는 확인이 필요합니다. | 없음 |

<a id="assurance-candidates"></a>
## 3. 보증 후보

| later 후보 | 현재 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| assurance profile | Later 후보입니다. 상세 본문 없음. | Exact gate, fallback behavior, proof expectation을 갖춘 owner-scoped assurance profile이 필요합니다. | 없음 |
| Evidence Manifest | 상세 증거 범위와 공백을 다루는 later 후보입니다. | Evidence owner rule, artifact/ref 처리, redaction behavior, close impact가 필요합니다. | 없음 |
| Manual QA | 사람이 수행한 QA record, finding, waiver, QA gate impact를 다루는 later 후보입니다. | Manual QA owner policy, waiver route, artifact ref, close-blocker behavior가 필요합니다. | 없음 |
| Eval / detached verification | 독립 검증 결과 기록을 다루는 later 후보입니다. | Eval owner path, independence semantics, baseline freshness, artifact integrity, assurance update rule이 필요합니다. | 없음 |
| Decision Packet full-format presentation | 확장된 user-judgment display를 위한 later 후보입니다. | User-judgment owner가 `presentation=full`을 켜야 하며, 기본 MVP path로 만들면 안 됩니다. | 없음 |
| Risk review and residual-risk visibility | 더 풍부한 위험 lifecycle 표시를 위한 later 후보입니다. | Visible risk, acceptance, expiry, close impact에 대한 Core/user-judgment owner rule이 필요합니다. | 없음 |
| 전체 설계 품질 later 정책 후보 | 이름만 남깁니다: full `shared_design` profile, `domain_language`, `vertical_slice`, `feedback_loop`, `tdd_trace`, `deep_module_interface`, `codebase_stewardship`, detailed `manual_qa`, `two_stage_review_display`, detached-verification policy, steward profile. | 향후 owner가 좁은 family 하나를 exact scope, fallback behavior, validator boundary, 면제와 증거 규칙, proof expectation과 함께 승격해야 합니다. | 없음 |

<a id="operations-candidates"></a>
## 4. 운영 후보

| later 후보 | 현재 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| 운영 프로필(Operations Profile) | Later 후보입니다. 상세 본문 없음. 활성 Reference owner는 없습니다. | Owner가 범위를 정한 운영 프로필이 필요합니다. 정확한 운영자 명령, 진단, 대체 동작, 증명 기대치를 함께 정의해야 합니다. | 현재 MVP 영향 없음 |
| 향후 로컬 운영자 명령 묶음 | 이름만 남긴 later 후보입니다: `harness connect`, `harness serve mcp`, `harness doctor`, `harness projection refresh`, `harness reconcile`, `harness recover`, `harness export`, `harness artifacts check`, `harness conformance run`. 이 이름은 예시일 뿐이며 명령 구문, 절차, 런타임 동작, 활성 명령 목록을 정의하지 않습니다. | 승격된 운영 owner가 정확한 명령 범위, 보안 태세, Storage/API 효과, 보고, 대체 동작, 증명 기대치를 정의해야 합니다. | 현재 MVP 영향 없음 |
| Export | Task bundle, redaction/omission note, retained/unavailable artifact, integrity summary를 위한 later 후보입니다. | 운영/export owner 계약, 저장소/아티팩트 규칙, 가림/생략 규칙, 유출 방지 증명이 필요합니다. | 현재 MVP 영향 없음 |
| Release Handoff | 보고서/export handoff만을 위한 later 후보입니다. | Handoff owner가 배포, 병합, 롤백, production 권한을 별도 승격 전까지 Harness 밖에 두어야 합니다. | 현재 MVP 영향 없음 |
| Recovery and reconcile | Lock, projection, artifact, managed output repair path를 위한 later 후보입니다. | Operations, Storage, Projection, Reconcile, Security owner 규칙이 필요합니다. | 현재 MVP 영향 없음 |
| 운영자 준비 상태와 `doctor` 접점 | 로컬 상태, 진단, 다음 운영자 행동을 위한 later 후보입니다. | 운영 owner 명령, capability check, 보안 태세 표현, 지원되지 않는 접점의 대체 동작이 필요합니다. | 현재 MVP 영향 없음 |
| Projection refresh와 최신성 진단 | 파생 보기 상태 확인을 위한 later 후보입니다. | Projection이 non-authoritative로 남도록 하는 Projection owner 동작이 필요합니다. | 현재 MVP 영향 없음 |
| 향후 conformance run entrypoint | Runtime fixture가 생긴 뒤의 later 후보입니다. 현재 명령이나 runner가 아닙니다. | 정확한 runner, suite, assertion, 요청/응답, storage/event/artifact/error, 보고 계약이 필요합니다. | 현재 MVP 영향 없음 |

문서 점검 메모: 이 tree에는 아직 `docs/*/maintain/checks.md`가 없습니다. 문서 점검 guidance를 운영 후보나 runtime conformance에 섞지 않습니다. dedicated maintain-docs rewrite가 필요하면 그 route를 만들도록 둡니다. 현재 MVP 영향 없음.

<a id="later-api-candidates"></a>
## 5. Later API 후보

| later 후보 | 현재 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| `harness.next` | Later/compatibility read 후보입니다. | 별도 next-action payload를 위한 owner activation이 필요합니다. MVP는 계속 `harness.status.next_actions`를 씁니다. | 없음 |
| `harness.launch_verify` | Detached verification launch 또는 manual evaluator bundle을 위한 assurance 후보입니다. | Eval/verification owner path, capability handling, baseline freshness, honest isolation wording이 필요합니다. | 없음 |
| `harness.record_eval` | Verification result recording을 위한 assurance 후보입니다. | Eval owner contract, independence validation, artifact ref, gate/assurance update rule이 필요합니다. | 없음 |
| `harness.record_manual_qa` | Human QA outcome recording을 위한 assurance 후보입니다. | Manual QA owner contract, waiver route, artifact, finding, gate impact가 필요합니다. | 없음 |
| Later read-only resources | Policy, evidence-manifest, surface, report, bundle, journey, design read를 위한 later 후보입니다. | Resource별 owner contract와 no mutation side effect가 필요합니다. | 없음 |
| Later `harness.record_run` branches | Verification input, feedback-loop update, TDD trace update를 위한 later 후보입니다. | `record_run` owner activation과 one-branch payload rule이 필요합니다. | 없음 |
| Later user-judgment branches | Waiver, reconcile, residual-risk, richer acceptance visibility를 위한 later 후보입니다. | Non-substitution rule을 지키는 user-judgment owner activation이 필요합니다. | 없음 |

<a id="later-schema-candidates"></a>
## 6. Later schema 후보

| later 후보 | 현재 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| later schema extensions | 후보일 뿐입니다. Full schema body는 삭제했습니다. | 승격된 owner가 active owner contract 안에서 exact field와 validator를 정의해야 합니다. | 없음 |
| Later close and assurance fields | `verifying`, `qa`, `completed_verified`, `detached_verified`, verification gate, QA gate, assurance blocker를 위한 후보입니다. | Close non-substitution rule을 포함한 Core/API owner activation이 필요합니다. | 없음 |
| Later next-action values | `launch_verify`, `record_eval`, `record_manual_qa`, `reconcile` 후보입니다. | 대응하는 API/profile owner activation이 필요합니다. | 없음 |
| Recommended playbooks and judgment context | 더 풍부한 read-only routing metadata 후보입니다. | Metadata가 state를 변경하거나 만족시키지 못하게 하는 Agent Integration/API owner rule이 필요합니다. | 없음 |
| Later ref and artifact values | Bundle, manifest, QA capture, export component, design, Eval, Manual QA, TDD, projection 관련 ref 후보입니다. | ArtifactRef, StateRecordRef, Storage, profile owner activation이 필요합니다. | 없음 |
| ValidatorResult later stable IDs | Design, autonomy, feedback-loop, TDD, stewardship, residual-risk, shared-design, manual-QA, context-hygiene check 후보입니다. | Validator owner contract, stable ID, severity, waiver, fixture proof가 필요합니다. | 없음 |
| Waiver, reconcile, and residual-risk branches | 더 풍부한 user-judgment payload 후보입니다. | User-judgment, Core, close owner rule이 필요합니다. | 없음 |

<a id="later-template-candidates"></a>
## 7. Later template 후보

| later 후보 | 현재 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| Decision Packet full-format presentation (`DEC`) | Later template 후보입니다. 상세 본문 없음. | Complex/full presentation을 위한 user-judgment owner activation이 필요합니다. | 없음 |
| `APR` | Later template 후보입니다. 상세 본문 없음. | Sensitive-action Approval owner path와 non-substitution wording이 필요합니다. | 없음 |
| Approval Card | Later template 후보입니다. 상세 본문 없음. | Approval display owner path가 필요합니다. | 없음 |
| `RUN-SUMMARY` | Later template 후보입니다. 상세 본문 없음. | Evidence/run owner path와 compact MVP summary fallback이 필요합니다. | 없음 |
| `EVIDENCE-MANIFEST` | Later template 후보입니다. 상세 본문 없음. | Evidence Manifest owner path가 필요합니다. | 없음 |
| `EVAL` | Later template 후보입니다. 상세 본문 없음. | Eval owner path와 detached-verification proof가 필요합니다. | 없음 |
| Verification Result Card | Later template 후보입니다. 상세 본문 없음. | Verification/Eval display owner path가 필요합니다. | 없음 |
| `MANUAL-QA` | Later template 후보입니다. 상세 본문 없음. | Manual QA owner path가 필요합니다. | 없음 |
| Manual QA Card | Later template 후보입니다. 상세 본문 없음. | Manual QA display owner path가 필요합니다. | 없음 |
| `TASK` | Later template 후보입니다. 상세 본문 없음. | Later continuity/report owner path가 필요합니다. | 없음 |
| `DIRECT-RESULT` | Later template 후보입니다. 상세 본문 없음. | Later direct-result report owner path가 필요합니다. | 없음 |
| `JOURNEY-CARD` | Later template 후보입니다. 상세 본문 없음. | Journey/current-position diagnostic owner path가 필요합니다. | 없음 |
| `DESIGN` | Later template 후보입니다. 상세 본문 없음. | Shared-design/design-quality owner path가 필요합니다. | 없음 |
| `DOMAIN-LANGUAGE` | Later template 후보입니다. 상세 본문 없음. | Domain language owner path가 필요합니다. | 없음 |
| `MODULE-MAP` | Later template 후보입니다. 상세 본문 없음. | Module map owner path가 필요합니다. | 없음 |
| `INTERFACE-CONTRACT` | Later template 후보입니다. 상세 본문 없음. | Interface contract owner path가 필요합니다. | 없음 |
| `TDD-TRACE` | Later template 후보입니다. 상세 본문 없음. | TDD/feedback-loop policy owner path가 필요합니다. | 없음 |
| `EXPORT` | Later template 후보입니다. 상세 본문 없음. | Operations/export owner path가 필요합니다. | 없음 |

<a id="future-fixture-families"></a>
## 8. Future fixture families

이 section은 이름만 나열합니다. Catalog-only future candidate일 뿐이며 fixture body, scenario script, suite requirement, active API payload, executable check, current conformance result, implementation task가 아닙니다.

| future fixture family |
|---|
| Intake and decision routing |
| Core, evidence, verification, and close |
| Artifact redaction and export non-leakage |
| Agency and user-judgment separation |
| Connector capability honesty |
| Design-quality and stewardship |
| Context hygiene and resume freshness |
| Projection, reconcile, and verification boundary |
| Operations diagnostics, export, recover, and handoff |
| Browser QA Capture |

<a id="roadmap-candidates"></a>
## 9. 로드맵 후보

| later 후보 | 현재 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| Dashboard, hosted workflows, artifact dashboard, richer cards, richer visualizations | Roadmap 후보입니다. | Read-only derived display rule이 필요합니다. Authority, readiness, QA, verification, acceptance 효과는 없어야 합니다. | 없음 |
| Browser capture automation | Roadmap 후보입니다. | Capture profile, redaction/PII handling, artifact retention, fallback behavior, QA/acceptance non-substitution이 필요합니다. | 없음 |
| Cross-surface verification | Roadmap 후보입니다. | Core-owned return record, Eval independence semantics, unsupported-surface fallback이 필요합니다. | 없음 |
| Broader connectors, connector marketplace, hosted UI, hosted/remote runtime | Roadmap 후보입니다. | Connector/API/security owner와 local-authority boundary proof가 필요합니다. | 없음 |
| Native hooks, preventive guard expansion, advanced sidecar watcher | Roadmap 후보입니다. | Preventive, isolation, arbitrary-tool-control claim 전에 covered mechanism proof가 필요합니다. | 없음 |
| Context Index, local derived metrics, long-term metrics | Roadmap 후보입니다. | Read-only retrieval/diagnostic owner가 필요하며 authority나 close effect가 없어야 합니다. | 없음 |
| Team workflows, permissions, shared profiles, orchestration, parallel lanes | Roadmap 후보입니다. | Scope, authority, permission, user-owned judgment owner가 필요합니다. | 없음 |
| Advanced exports, release/deployment/canary/rollback/merge/production-monitoring automation | Roadmap 후보입니다. | 별도 owner scope가 필요합니다. 명시적으로 승격하기 전까지 deploy와 production authority는 외부에 남습니다. | 없음 |
| Advanced validators and language or interface checks | Roadmap 후보입니다. | Exact policy, validator ID, severity, waiver, fixture behavior가 필요합니다. | 없음 |

<a id="explicitly-retired-material"></a>
## 10. 명시적으로 폐기한 상세 자료

| later 후보 | 현재 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| Full future designs in Later pages | Active documentation에서 폐기했습니다. 상세 본문 없음. | 좁은 범위를 가진 promoted owner 문서에서만 다시 도입합니다. | 없음 |
| Full later schema definitions | Active documentation에서 폐기했습니다. 상세 본문 없음. | 승격 뒤 active 또는 profile-gated owner schema로만 다시 도입합니다. | 없음 |
| Full later template bodies | Active documentation에서 폐기했습니다. 상세 본문 없음. | Template owner가 profile을 켜고 source record를 정의할 때만 다시 도입합니다. | 없음 |
| Full YAML fixtures and scenario scripts | Active documentation에서 폐기했습니다. 상세 본문 없음. | 승격 뒤 conformance owner 아래 exact-shape fixture material로만 다시 도입합니다. | 없음 |
| Old roadmap narratives, resolved goals, and historical cleanup notes | Active documentation에서 폐기했습니다. | 필요하면 현재 후보 row 또는 maintainer-owned history로만 다시 도입합니다. | 없음 |
| Archive copies, migration notes, and scratch files for this collapse | 만들지 않았고 보존하지 않습니다. | 승격 path는 없습니다. Maintainer가 별도 non-active migration record를 요청할 때만 다시 만듭니다. | 없음 |
