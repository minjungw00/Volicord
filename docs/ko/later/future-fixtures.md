# 이후: 향후 Fixtures

## 이 문서로 할 수 있는 일

이 문서는 향후 fixture 시나리오 묶음을 작게 정리한 목록입니다. MVP-1 이후에 유용할 수 있는 설계 지식은 남기되, 그것이 MVP 요구사항, 실행 가능한 conformance suite, 필수 fixture 파일 묶음, 서버 구현 계획처럼 읽히지 않게 합니다.

이 문서는 향후 설계 문서일 뿐입니다. MVP-1 요구사항이 아니고, 구현된 런타임 동작이 아니며, active API나 DDL도 아니고, 현재 conformance도 아닙니다. 현재 저장소는 문서 전용입니다. 실행 가능한 Harness Server conformance test, generated conformance artifact, 실행 가능한 fixture catalog file, 서버 구현, 런타임 상태가 없습니다. 현재 단계와 인계 상태는 [MVP 계획](../build/mvp-plan.md#문서-수락-상태)에 있습니다.

예전의 긴 pseudo-fixture payload는 의도적으로 제거했습니다. 완전성을 이유로 이 문서에 scenario script를 다시 만들지 않습니다. 향후 owner가 scenario를 승격하면 exact behavior와 exact fixture body는 아래에 이름 붙인 owner Reference 경로에 둡니다.

## Catalog 경계

[Conformance Fixtures 참조](../reference/conformance-fixtures.md)는 핵심 conformance model, active MVP structured fixture draft, exact structured fixture draft shape, future runner behavior, assertion semantics, fixture profile, 현재 단계 상태, 좁은 내부 엔지니어링 점검 Kernel Smoke 작성 queue를 담당합니다.

이 catalog는 향후 시나리오 목록만 담당합니다. 각 row는 후보 묶음일 뿐입니다. Fixture body, public request schema, storage schema, DDL row, stage exit criterion, generated artifact, runtime result, implementation task가 아닙니다. Catalog row를 어떤 behavior가 이미 존재한다는 증거, 어떤 API가 active라는 증거, 어떤 table을 지금 구현해야 한다는 증거, MVP-1 scope가 넓어졌다는 증거로 인용하면 안 됩니다.

Projection output은 나중에 살펴볼 display로 언급할 수 있습니다. 하지만 projection output은 conformance truth가 아닙니다. 승격된 scenario는 Core-owned state, owner record, event, artifact, error, 관련 owner contract로 동작을 증명해야 합니다.

## 승격 조건

향후 시나리오 묶음은 owner promotion path를 거쳐야 active가 됩니다. 승격 대상은 두 가지입니다.

| 승격 대상 | 승격 전 최소 조건 |
|---|---|
| Active behavior example | Owner 문서가 behavior, delivery stage 또는 profile, 사용자에게 보이는 결과, 영향을 받는 owner record, fallback behavior, security 또는 guarantee wording, non-claim을 이름 붙입니다. Owner가 executable fixture도 구체화하지 않는 한 example은 설명용으로만 남습니다. |
| Runtime conformance case | Accepted implementation-planning scope, exact API와 storage owner, exact structured fixture body field, 필요한 seed-state expansion rule, runner behavior, assertion semantics, request/response observation, storage/event/artifact/blocker/error observation, forbidden-side-effect assertion이 있습니다. Rendered prose만이 아니라 Core state와 owner record를 증명해야 합니다. |

모든 승격은 아래 조건도 충족해야 합니다.

- 대상이 내부 엔지니어링 점검, MVP-1 사용자 작업 루프, 보증 프로필, 운영 프로필, 로드맵 중 어디인지 말합니다.
- Exact schema, DDL, security wording, fixture shape를 담당하는 Reference section을 식별합니다.
- 승격된 owner와 중복될 수 있는 catalog row를 삭제하거나 좁힙니다.
- 한국어 문서에도 같은 의미를 반영합니다.
- 지원하지 못하거나 더 약한 surface는 fallback 또는 reduced-guarantee wording으로 정직하게 표시합니다.

## 버킷 지도

시나리오 묶음을 읽기 전에 이 지도를 사용합니다.

| 버킷 | 여기에 보관하는 scenario material | 승격 경로 |
|---|---|---|
| 보증 프로필 | 검증 강화, 수동 QA, 상세 증거, 위험 검토, design-quality, stewardship, TDD trace, feedback-loop, context-hygiene family 중 보증 behavior를 지원하는 것. | [보증 프로필](assurance-profile.md), 그다음 관련 Reference owner. |
| 운영 프로필 | Export, recovery, handoff, artifact integrity, projection refresh, reconcile, operator readiness, `doctor`/readiness, future conformance-run family. | [운영 프로필](operations-profile.md), 그다음 [운영과 Conformance 참조](../reference/operations-and-conformance.md). |
| 로드맵 | Dashboard, hosted workflow, team workflow, broad connector, Browser QA Capture automation, Cross-Surface Verification automation, remote/shared MCP, preventive guard expansion, hook, orchestration, metrics, 그 밖의 확장 후보. | [로드맵](../roadmap.md) 승격 조건. |

Catalog entry는 가장 가까운 technical concern 아래에 있을 수 있습니다. 하지만 stage를 해석할 때는 위 버킷이 기준입니다. 여기에 묶음이 나열되어도 stage-required가 되지 않습니다.

## Catalog 전용 Future Families

아래 family는 의도적으로 내부 엔지니어링 점검과 MVP-1 사용자 작업 루프 밖에 둡니다.

| Future family | Catalog boundary |
|---|---|
| Full Manual QA | Full policy matrix, browser/manual capture expansion, QA waiver detail, QA dashboard는 더 좁은 owner path가 승격되기 전까지 future 또는 보증 프로필 범위에 남습니다. |
| Eval systems and detached verification automation | Cross-surface evaluator orchestration, detailed Eval report, independence hardening, assurance upgrade는 future 또는 보증 프로필 범위에 남습니다. MVP-1은 compatible verification record가 실제로 있을 때가 아니면 detached verification을 주장하지 않아야 합니다. |
| TDD trace and feedback-loop policy | RED/GREEN trace, feedback-loop execution policy, policy-specific test-path scenario는 future 또는 보증 프로필 범위에 남습니다. |
| Module map and interface contract | Domain, module, interface stewardship scenario는 owner docs가 exact record와 validator를 승격하기 전까지 candidate로 남습니다. |
| Journey, Spine, and detailed report projections | Journey Card, Journey Spine, Run Summary, detailed Evidence Manifest, detailed Eval, polished report projection은 derived-output candidate입니다. State가 되거나 MVP-required projection kind가 되지 않습니다. |
| Export, recover, release handoff, and artifact-integrity operations | Export/recover, release handoff, retention, redaction export, artifact check scenario는 승격 전까지 운영 프로필 또는 이후 범위에 남습니다. |
| Dashboard, team workflow, and orchestration | Hosted UI, dashboard, shared/team workflow, 승인, parallel-lane, orchestration scenario는 로드맵 후보입니다. |
| Advanced connector and security behavior | Broad connector ecosystem, remote/shared MCP, browser capture automation, preventive guard, isolated profile, hook, sidecar, higher security claim은 covered operation에 대해 owner-defined mechanism과 fixture proof가 있어야 승격할 수 있습니다. |

<a id="staged-fixture-coverage"></a>
<a id="fixture-예시-지도"></a>
<a id="fixture-example-map"></a>

## 시나리오 묶음 목록

아래 section은 예전 staged coverage map과 긴 fixture example payload를 대체합니다. Family 이름과 의도만 보존합니다. 만들 fixture 파일의 checklist가 아닙니다.

<a id="intake와-decision-catalog-entries"></a>
<a id="intake-and-decision-catalog-entries"></a>

### Intake와 Decision Catalog Entries

| 시나리오 묶음 | 나중에 검증하거나 보여 줄 기능/경계 | 승격 메모 |
|---|---|---|
| Natural-language intake and plain routing | 사용자가 startup phrase를 말하지 않아도 Harness로 추적할 만한 작업을 시작하거나 이어갈 수 있고, 평범한 사용자 말이 compatible Task, 범위, user judgment, 다음 안전한 행동으로 연결됩니다. | Core/API intake owner를 통해서만 승격합니다. Product write를 승인하지 않는다는 wording이 필요합니다. |
| Tiny direct without authority bypass | 아주 작은 명백한 작업은 Direct mode에 남을 수 있지만 `tiny` mode를 만들거나 scope, user judgment, 민감 동작 승인, Write Authorization을 우회하지 않습니다. | Active Direct profile이 좁은 behavior와 escalation path를 정의한 뒤에만 승격합니다. |
| Codebase-answerable before user question | Current refs와 제공된 facts를 먼저 사용하고, 해결되지 않은 제품 판단 또는 중요한 기술 판단은 계속 사용자에게 라우팅합니다. | Agent Integration 또는 Core owner가 context-source freshness rule을 정의해야 합니다. |
| User judgment quality and separation | 제품 판단, 기술 판단, 민감 동작 승인, 수동 QA, 최종 수락, 잔여 위험 수락은 서로 구분됩니다. | User judgment와 gate owner를 통해 승격합니다. Broad approval을 되살리면 안 됩니다. |
| Residual risk visibility before acceptance or close | Known close-relevant residual risk는 acceptance 또는 close 전에 보여야 합니다. `ResidualRiskSummary.status=none`은 known close-relevant risk가 없을 때만 유효합니다. | 실행 가능한 fixture를 쓰기 전에 Core Model과 residual-risk owner를 통해 승격합니다. |

<a id="core-fixture-예시"></a>
<a id="core-fixture-examples"></a>

### Core, Evidence, Verification, Close Catalog Entries

| 시나리오 묶음 | 나중에 검증하거나 보여 줄 기능/경계 | 승격 메모 |
|---|---|---|
| Write scope and Write Authorization lifecycle | Active Change Unit이 없으면 write가 막히고, compatible `prepare_write`는 durable Write Authorization을 만들며, missing/consumed/stale/violated authorization은 dependent claim을 막거나 stale로 만듭니다. | [Core Model 참조](../reference/core-model.md)와 [MVP API](../reference/api/mvp-api.md)를 통해 승격합니다. |
| Evidence and close readiness | Direct docs-only 작업은 evidence가 충분할 때만 close할 수 있고, acceptance criteria support 누락, pending verification, pending QA는 close를 막습니다. | Report text가 아니라 Core와 evidence/close owner를 통해 승격합니다. |
| Detached verification boundary | Manual bundle review, subagent review, same-session review, verification-risk acceptance, visible accepted risk는 서로 다른 assurance path로 남습니다. | Eval/verification owner를 통해 승격합니다. Same-session self-review가 detached assurance를 만들면 안 됩니다. |
| Projection failure with current state | Projection이 stale, skipped, failed여도 current Core state가 authoritative합니다. | Projection과 Core owner를 통해 승격합니다. Rendered Markdown이 gate를 충족하면 안 됩니다. |

<a id="artifact-redaction-and-export-non-leakage-catalog-entries"></a>

### Artifact Redaction And Export Non-Leakage Catalog Entries

| 시나리오 묶음 | 나중에 검증하거나 보여 줄 기능/경계 | 승격 메모 |
|---|---|---|
| Secret or PII omitted, visible evidence only | Evidence, QA, Eval, projection, report view는 보이는 nonsecret material만 인용하고 omitted value는 사용할 수 없게 둡니다. | Artifact, evidence, redaction, export owner를 통해 승격합니다. |
| Blocked input as metadata-only notice | Blocked payload는 forbidden bytes를 노출하지 않고 안전한 메타데이터와 unresolved downstream effect만 남길 수 있습니다. | Artifact storage와 redaction behavior가 exact해진 뒤에만 승격합니다. |
| Untrusted staged URI and task-scoped refs | 임의 path, traversal, symlink escape, cross-Task artifact relation은 trusted artifact evidence가 될 수 없습니다. | Storage와 ArtifactRef owner를 통해 승격합니다. |
| Artifact integrity affects dependent claims | Missing file, missing `sha256` 또는 `size_bytes`, `hash_mismatch`, owner-link mismatch는 dependent evidence, QA, Eval, projection, export, close readiness를 막습니다. | Artifact integrity와 operations owner를 통해 승격합니다. |
| Export and Release Handoff non-leakage | Exported snapshot과 handoff report는 omission/block note를 보여 주되 raw omitted value나 blocked payload를 leak하지 않습니다. | 운영 프로필, export, security owner를 통해 승격합니다. |

<a id="agency-fixture-예시"></a>
<a id="agency-fixture-examples"></a>

### Agency Catalog Entries

| 시나리오 묶음 | 나중에 검증하거나 보여 줄 기능/경계 | 승격 메모 |
|---|---|---|
| User judgment before product or technical trade-off writes | 사용자 소유 제품 판단이나 중요한 기술 판단에 달린 write는 compatible judgment가 있을 때까지 보류됩니다. | User judgment, Change Unit, `prepare_write` owner를 통해 승격합니다. |
| 민감 동작 승인 is not judgment or close | Approval 모양의 승인은 제품 판단, evidence, verification, QA, 최종 수락, 잔여 위험 수락, close를 충족하지 않습니다. | MVP sensitive-action judgment와 later Approval profile record의 분리를 exact하게 해야 합니다. |
| AFK Autonomy Boundary stop conditions | AFK 또는 높은 자율성 작업은 product, public API, security, privacy, 그 밖의 stop condition이 나타나면 멈추거나 judgment로 라우팅합니다. | Agency와 connector capability owner를 통해 승격하고 guarantee wording을 정직하게 둡니다. |
| Acceptance and residual-risk sequencing | 최종 수락과 잔여 위험 수락은 별도 user judgment이며, known close-relevant risk는 먼저 보여야 합니다. | Close, acceptance, residual-risk owner를 통해 승격합니다. |

<a id="connector-fixture-예시"></a>
<a id="connector-fixture-examples"></a>
<a id="connector-agency-catalog-entries"></a>

### Connector Catalog Entries

| 시나리오 묶음 | 나중에 검증하거나 보여 줄 기능/경계 | 승격 메모 |
|---|---|---|
| Guarantee display and capability honesty | Connector는 cooperative, detective, preventive, isolated guarantee를 실제로 지원할 수 있는 수준으로만 표시합니다. | Agent Integration과 Security owner를 통해 승격합니다. Preventive claim에는 action 전 차단을 증명한 fixture가 필요합니다. |
| MCP unavailable or capability mismatch holds unsafe writes | MCP 부재, stale capability profile, artifact capture 부재, QA capture 부재, 약한 redaction, 더 약한 guard capability는 affected write 또는 close-relevant path를 보류합니다. | API error precedence와 connector capability owner를 통해 승격합니다. |
| Generated file or managed instruction drift routes to reconcile | Generated connector file과 managed block은 감지되어 reconcile로 라우팅되며 owner record를 조용히 rewrite하지 않습니다. | Projection/Reconcile과 connector manifest owner를 통해 승격합니다. |
| Current-position context before significant resume | Resume은 instruction bundle을 만들기 전에 current Task state, refs, pending judgment, residual risk, projection freshness를 읽습니다. | Context push/pull profile owner를 통해 승격합니다. |
| Guard, freeze, and careful mode do not create authority | 이 label들은 behavior를 좁히거나 write를 보류할 수 있지만, stronger guarantee, Write Authorization, Approval, verification, QA, 최종 수락, 잔여 위험 수락, close, assurance upgrade를 스스로 만들지 않습니다. | Exact surface capability proof와 fallback behavior가 있어야 승격합니다. |
| Local-only MCP and local security posture | Non-loopback, forwarded, tunneled, unauthenticated, weak local exposure는 정직하게 보고되며 authority를 만들지 않습니다. | Security와 Operations owner를 통해 승격합니다. |

<a id="design-quality-fixture-예시"></a>
<a id="design-quality-fixture-examples"></a>
<a id="stewardship-fixture-예시"></a>
<a id="stewardship-fixture-examples"></a>
<a id="stewardship-catalog-entries"></a>

### Design-Quality와 Stewardship Catalog Entries

| 시나리오 묶음 | 나중에 검증하거나 보여 줄 기능/경계 | 승격 메모 |
|---|---|---|
| Shared Design required and continued while unknowns remain | Ambiguous work는 goal, non-goal, acceptance criteria, affected flow, module/interface impact, verification, QA, risk가 확인되거나 user judgment로 라우팅될 때까지 design shaping을 계속합니다. | Design Quality와 Shared Design owner를 통해 승격합니다. |
| Codebase-answerable stewardship facts first | Module ownership, domain language, public interface impact, affected paths, test/QA affordance처럼 current refs에 이미 있는 facts는 사용자에게 묻기 전에 사용합니다. | Source freshness와 owner-record rule이 필요합니다. |
| Horizontal exceptions, feedback loops, and TDD trace | Horizontal exception reason, behavior feedback loop, RED/GREEN trace, non-test write guard는 policy owner가 exact behavior를 정의한 뒤에만 assurance check가 됩니다. | 이 catalog가 아니라 Design Quality Policies를 통해 승격합니다. |
| Manual QA required or waived through owner paths | Manual QA requirement, waiver reason, product-risk waiver judgment, QA gate effect는 최종 수락과 verification과 구분됩니다. | Manual QA와 user judgment owner를 통해 승격합니다. |
| Public interface, module, and domain language stewardship | Public boundary change, interface-contract review, future-change risk, domain-language conflict는 owner record와 close blocker로 라우팅됩니다. | Stewardship, module map, interface contract owner를 통해 승격합니다. |
| Findings route to existing owner paths | Run, Eval, Manual QA, design-quality finding은 evidence, user judgment, feedback loop, Manual QA, Eval, residual risk, validator result, gate, close blocker에 영향을 주되 여기에서 새 finding schema를 만들지 않습니다. | 관련 owner contract 안에서만 승격합니다. |
| Review Stage display is not authority | Spec Compliance Review와 Code Quality / Stewardship Review는 분리해서 표시할 수 있지만 display text만으로 close, risk acceptance, evidence creation, QA/verification satisfaction, Approval creation, Write Authorization creation을 할 수 없습니다. | Core non-substitution rule과 projection/display owner를 통해 승격합니다. |

<a id="context-hygiene-fixture-예시"></a>
<a id="context-hygiene-fixture-examples"></a>
<a id="context-hygiene-catalog-entries"></a>

### Context Hygiene Catalog Entries

| 시나리오 묶음 | 나중에 검증하거나 보여 줄 기능/경계 | 승격 메모 |
|---|---|---|
| Stale PRD, stale projection, or old design doc is pull-only | Stale context는 살펴볼 ref를 가리킬 수 있지만 current Task state, acceptance criteria, Change Unit scope, 제품 판단, gate state를 대체할 수 없습니다. | Context hygiene, projection freshness, Core owner를 통해 승격합니다. |
| Resume uses current state, not chat memory | Significant resume은 stale chat memory가 아니라 Core state, current-position refs, evidence refs, active user judgments, residual-risk summary, projection freshness를 사용합니다. | Agent Integration과 context profile owner를 통해 승격합니다. |
| Compact context by phase | Always-on context는 refs-first, current, 한 화면 이하, profile-relevant여야 합니다. Full docs, schema, log, artifact contents, future catalog material은 pull-on-demand로 남습니다. | Agent Integration Reference를 통해 승격합니다. |
| Retrieved or indexed context is non-authority | Search, memory, indexed context는 ref나 excerpt를 제공할 수 있지만 write authorization, gate satisfaction, final acceptance, residual-risk acceptance, projection freshness update, close를 만들 수 없습니다. | Context Index 또는 동등한 로드맵 owner가 승격된 뒤에만 승격합니다. |
| Evaluator bundle freshness | Verification bundle은 asserted evaluation에 충분히 current해야 하며, material context가 stale 또는 missing이면 detached verification passed를 설정할 수 없습니다. | Eval/verification owner를 통해 승격합니다. |

<a id="core-projection-reconcile-verification-boundary-catalog-entries"></a>
<a id="core-projection-reconcile-and-verification-boundary-catalog-entries"></a>

### Core, Projection, Reconcile, Verification Boundary Catalog Entries

| 시나리오 묶음 | 나중에 검증하거나 보여 줄 기능/경계 | 승격 메모 |
|---|---|---|
| Current state versus stale projection | Projection이 stale 또는 failed여도 Core state는 읽을 수 있고 authoritative합니다. Close/readiness는 stale Markdown에서 readiness를 추론하면 안 됩니다. | Projection과 Core owner를 통해 승격합니다. |
| Managed-block edits route to reconcile | Managed block이나 generated output 안의 human edit은 reconcile candidate를 만들고, Core를 통한 accepted decision 전까지 owner record를 바꾸지 않습니다. | Projection/Reconcile owner를 통해 승격합니다. |
| Same-session self-review is not detached verification | Same-session review는 useful context일 수 있지만 detached verification을 충족하거나 assurance를 올릴 수 없습니다. | Eval/verification owner를 통해 승격합니다. |

<a id="operations-profile-catalog-entries"></a>

### 운영 프로필 Catalog Entries

| 시나리오 묶음 | 나중에 검증하거나 보여 줄 기능/경계 | 승격 메모 |
|---|---|---|
| Release Handoff does not close or deploy | Handoff report는 close readiness, blocker, evidence ref, verification ref, 수동 QA ref, residual-risk ref, changed file, projection freshness, artifact, advisory checklist를 요약할 수 있지만 Task state를 변경하거나 deploy하지 않습니다. | [운영 프로필](operations-profile.md)과 export/handoff owner를 통해 승격합니다. |
| Export, recover, and artifact integrity | Export/recover operation은 retention, redaction, integrity, availability를 보고하지만 state를 조용히 repair하거나 secret access를 넓히지 않습니다. | Operations, Storage, ArtifactRef, Security owner를 통해 승격합니다. |
| Doctor and readiness diagnostics | `doctor`, `connect`, `serve mcp`, readiness, future conformance-run entrypoint는 operator posture를 보고하되 early-stage requirement를 암시하지 않습니다. | Operations And Conformance와 Security owner를 통해 승격합니다. |

<a id="로드맵-browser-qa-capture-candidate-entries"></a>
<a id="roadmap-browser-qa-capture-candidate-entries"></a>

### 로드맵 Browser QA Capture Candidate Entries

Browser QA Capture는 로드맵 후보입니다. 내부 엔지니어링 점검, MVP-1 사용자 작업 루프, 보증 프로필, 운영 프로필, Kernel Smoke 요구사항이 아닙니다. Capability profile, redaction과 secret/PII policy, test environment, artifact retention, conformance target, fallback semantics, no projection-as-canonical dependency가 정의된 뒤에만 실행 가능해질 수 있습니다.

| 시나리오 묶음 | 나중에 검증하거나 보여 줄 기능/경계 | 승격 메모 |
|---|---|---|
| Browser capture artifacts attach to Manual QA | Surface가 capture를 지원할 때 screenshot, QA capture, log, console log, network trace, accessibility snapshot, workflow recording이 Manual QA record를 보조합니다. | Browser QA Capture와 Manual QA owner를 통해 승격합니다. |
| Capture is not final acceptance or detached verification | Browser artifact는 evidence를 보조할 수 있지만 human Manual QA judgment, final acceptance, residual-risk acceptance, detached verification을 대체하지 않습니다. | QA, acceptance, residual-risk, Eval owner를 통해 승격합니다. |
| Unsupported surface falls back to human notes | Capture를 지원하지 않는 surface는 missing capability를 보고하고 human Manual QA notes 또는 manually supplied artifacts를 추천합니다. Automation이 없다는 이유만으로 staged delivery를 실패 처리하지 않습니다. | Connector capability와 fallback owner를 통해 승격합니다. |

<a id="agency-stewardship-context-design-quality-suite"></a>
<a id="agency-stewardship-context-and-design-quality-suites"></a>

## Agency, Stewardship, Context, Design-Quality Suites

Agency, stewardship, context hygiene, design-quality는 owner docs가 승격하기 전까지 catalog-only 보증 프로필 suite candidate입니다. 승격되면 Core entrypoint 또는 Core를 호출하는 operator action으로 response fact, Core state, storage row, event, artifact, blocker, error, forbidden side effect를 test해야 합니다. Journey Card, user judgment, residual-risk, review-stage, status, report prose와 문구가 맞는지만 보고 통과하면 안 됩니다.

Role Lens와 Browser QA recommendation을 포함한 status와 `next` recommendation은 later public mutation이 owner record를 기록하지 않는 한 read response로만 관찰됩니다. Recommendation alone으로 state 변경, gate 충족, projection 대기열 추가, evidence 생성, verification 기록, QA 기록, final acceptance, residual-risk acceptance, Task close, assurance upgrade가 일어나면 안 됩니다.

<a id="catalog-only-fixture-skeleton-guidance"></a>

### Catalog-Only Fixture Skeleton Guidance

이 catalog는 더 이상 fixture skeleton을 담지 않습니다. Future exact-shape fixture materialization은 [Conformance Fixtures 참조](../reference/conformance-fixtures.md)와 관련 API, Storage, Core, Security, Projection, Operations, Agent Integration, policy owner에 둡니다. Delivery-stage mapping은 catalog prose가 아니라 suite metadata 또는 Build owner에 둡니다.

<a id="later-profile-fixture-shorthand-notes"></a>

### Later-Profile Fixture Shorthand Notes

Later-profile shorthand는 planning convenience일 뿐입니다. 내부 엔지니어링 점검이나 MVP-1 사용자 작업 루프에는 active가 아니고, executable runner contract도 아니며, 두 번째 API도 아닙니다. 향후 fixture가 executable이 되려면 shorthand는 DDL/API docs가 명시적으로 소유하는 owner record, validator run, residual-risk record, 그 밖의 state로 expand되어야 합니다. Fixture-only storage row나 alternate request payload branch를 만들면 안 됩니다.

<a id="fixture-suites"></a>

## Fixture Suites

Future suite name은 planning label이며 필수 file set이 아닙니다. 승격된 뒤에만 [Conformance Fixtures 참조](../reference/conformance-fixtures.md#검증-프로파일별-증명-동작)의 fixture profile 아래에서 inventory family를 묶습니다.

| Suite label | Inventory boundary |
|---|---|
| `core` | Minimal 내부 엔지니어링 점검 subset을 넘는 write scope, Write Authorization, evidence, close readiness, verification boundary, residual-risk visibility, projection failure separation. |
| `connector` | Natural-language routing, capability honesty, MCP availability, generated-file drift, artifact capture fallback, current-position resume context, local security posture. |
| `artifact-redaction` | Registered artifact boundary, redaction/blocked metadata, task-scoped ref, integrity check, export 또는 handoff non-leakage. |
| `connector-guard-freeze` | Cooperative/detective guard와 freeze behavior, careful-mode non-authority, capability mismatch honesty, surface-specific proof가 있을 때만 가능한 preventive claim. |
| `agency` | User judgment quality, user-owned trade-off guard, AFK stop condition, Approval separation, acceptance sequencing, residual-risk visibility. |
| `stewardship` | Shared design, codebase-answerable facts, feedback loop, TDD trace, public interface review, domain language, finding routing, managed-block reconcile, review-stage non-authority. |
| `context-hygiene` | Compact current context, stale projection/PRD/chat handling, retrieved context non-authority, evaluator bundle freshness, Core state에서 resume. |
| `design-quality` | Kernel authority를 다시 정의하거나 validator ID를 duplicate하거나 낮은 severity finding을 숨기거나 새 gate를 만들지 않고 existing validator와 gate behavior를 조합하는 policy-pack smoke coverage. |
| `operations` | 운영 프로필 승격 이후 export, recover, handoff, artifact integrity, readiness, diagnostic, future conformance-run entrypoint. |
| `browser-qa-capture` | Roadmap-only capture automation, artifact mapping, Manual QA attachment, detached-verification boundary, final-acceptance boundary, unsupported-surface fallback. |

## 제거한 상세 경계

Pseudo-fixture YAML, 긴 scenario script, detailed assertion payload, renderer-output expectation, future runner output requirement는 이 catalog에서 제거했습니다. Active MVP structured draft는 이제 Conformance Fixtures 참조에 둡니다. Future-profile detail은 특정 active behavior나 runtime conformance case를 증명하는 데 필요하고 owner Reference 문서가 승격한 경우에만 다시 둡니다.
