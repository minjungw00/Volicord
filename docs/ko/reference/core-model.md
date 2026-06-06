# Core Model 참조

이 문서는 향후 하네스 Core의 핵심 모델과 권한 경계를 정의합니다. 문서 소스일 뿐이며, 이 저장소에는 아직 하네스 런타임이나 서버 구현이 없습니다. 현재 문서가 구현 완료 상태인지는 maintainer가 소유하는 [MVP 계획](../build/mvp-plan.md#문서-수락-상태)의 상태만으로 판단합니다.

Core는 작업 범위, 사용자가 소유하는 판단, 증거, 검증 기대치, 닫기 준비 상태, 잔여 위험을 위한 로컬 기준 기록입니다. Core의 권한은 하네스 기록과 하네스 상태 전이에 미칩니다. OS 권한, 임의 도구 실행, 권한 격리, 변조 방지, 보안 격리는 다른 owner가 정확한 메커니즘을 문서화하고 증명하지 않는 한 Core 권한이 아닙니다.

## 1. 소유 / 소유하지 않음

이 문서가 소유합니다.

- Core 불변조건과 권한 경계.
- 상태, 쓰기 호환성, gate 동작, 닫기에 영향을 주는 entity 관계 의미.
- 사용자가 소유하는 판단의 경계와 대체 불가능 규칙.
- Gate 의미, blocker 의미, lifecycle 원칙, 상태 전이 원칙.
- `prepare_write`, Write Authorization, `record_run`, `close_task`, 면제, 잔여 위험 표시, 정직한 닫기.
- Core, API, Storage, Projection, Security, Later 자료가 서로 넘지 말아야 하는 소유자 간 권한 연결.

이 문서는 소유하지 않습니다.

- 공개 MCP 요청/응답 모양. [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md)를 봅니다.
- Storage table, DDL, runtime home layout, lock, migration, persisted JSON layout. [Storage](storage.md)를 봅니다.
- 렌더링된 projection body나 template text. [Projection과 Template 참조](projection-and-templates.md)를 봅니다.
- Connector capability profile과 surface recipe. [Agent 통합 참조](agent-integration.md)를 봅니다.
- Core 권한 결과를 넘어서는 보안 보장 어휘. [보안 참조](security.md)를 봅니다.
- Later/profile catalog. Profile owner가 active scope로 승격하기 전까지 [Later](../later/index.md)에 둡니다.

정확한 API request field와 storage table definition은 여기서 참조로만 이름 붙입니다. Core state value는 권한과 상태 전이 의미를 설명해야 할 때만 다룹니다.

<a id="kernel-invariants"></a>

## 2. 커널 불변조건

1. Core가 소유한 상태가 하네스 동작의 기준입니다. Chat, report, generated Markdown, status card, Projection, template output은 파생 표시이거나 맥락입니다.
2. 하네스는 하네스 기록과 상태 전이를 다룹니다. OS 권한, 임의 도구 실행 제어, 권한 격리를 제공하지 않습니다.
3. 제품 파일 쓰기는 `prepare_write`가 write attempt를 허용하기 전에 명시적이고 호환되는 범위를 가져야 합니다.
4. `dry_run=false`인 호환 allowed `prepare_write` 경로만 consumable Write Authorization을 만듭니다.
5. Write Authorization은 호환되는 attempt 하나에 한 번만 쓰입니다. 재사용 가능한 범위도 아니고 OS 권한도 아닙니다.
6. `record_run`은 실제로 일어난 일을 기록하고 호환되는 쓰기 기준 권한을 소비합니다. 범위, user judgment, 민감 동작 승인, Write Authorization 없이 일어난 일을 사후에 승인할 수 없습니다.
7. 사용자가 소유하는 판단은 에이전트 추론, 포괄적 동의, 생성 문구, 증거, Projection 텍스트로 대체될 수 없습니다.
8. 제품 판단, 기술 판단, 범위 판단, 민감 동작 승인, 최종 수락, QA 면제 판단, 검증 위험 수락, 잔여 위험 수락, 취소 판단은 서로 다릅니다.
9. 증거, 검증, 수동 QA, 최종 수락, 잔여 위험 표시, 잔여 위험 수락, 닫기 준비 상태는 서로를 대신하지 않습니다.
10. 닫기와 관련된 blocker가 남아 있으면 `close_task`는 정직하게 닫았다고 말할 수 없습니다. 성공 닫기가 알려진 잔여 위험에 의존한다면 그 위험은 먼저 보여야 합니다.
11. 현재 활성 MVP 범위와 later/profile 자료는 분리됩니다. Later candidate는 owner가 범위, 대체 동작, 증명 기대와 함께 승격할 때만 active가 됩니다.

<a id="entity-model"></a>

## 3. 엔티티 모델

아래 엔티티는 권한 관계를 정의합니다. Storage table이나 API body를 추가하지 않습니다.

- Task: 사용자 가치 단위입니다. 현재 `mode`, 범위 관계, blocker, 판단 필요성, 증거 상태, 닫기 준비 상태, 최종 수락 상태, 잔여 위험 상태, latest Run 관계를 기록합니다.
- Change Unit: 쓰기가 가능한 작업의 활성 scoped work boundary입니다. 제품 파일 쓰기는 호환되는 active Change Unit으로 cover되어야 합니다.
- <a id="autonomy-boundary"></a>Autonomy Boundary: Change Unit 안에서 agent가 가질 수 있는 판단 latitude입니다. 범위, 민감 동작 승인, evidence, 최종 수락, 잔여 위험 수락이 아닙니다.
- `user_judgment`: 사용자가 소유하는 선택을 위한 기준 기록군입니다. 판단 호환성에 반영되지만 그 자체로 evidence, Write Authorization, close를 만들지는 않습니다.
- <a id="write-authorization"></a>Write Authorization: 호환되는 non-dry-run `prepare_write`만 만드는 durable single-use Core record입니다. Lifecycle은 active, consumed, stale, expired, revoked 중 하나일 수 있습니다. `allowed`는 `prepare_write` decision이지 durable authorization status가 아닙니다. `blocked`도 authorization status가 아닙니다.
- Run: 실행 또는 관찰 기록입니다. Product-write Run은 compatible active Write Authorization을 소비해야 합니다. Read-only 또는 shaping-only Run은 이후 쓰기를 compatible하게 만들지 않습니다.
- 증거 요약: 닫기 관련 claim, Run, blocker, user judgment, `ArtifactRef` value를 연결하는 active compact Core evidence path입니다. 전체 Evidence Manifest는 profile owner가 켜기 전까지 active가 아닙니다.
- `ArtifactRef`: API/Storage가 소유하는 durable evidence reference shape입니다. Core는 등록되어 있고, integrity와 redaction을 보존하며, owner record와 연결될 때만 evidence-eligible로 다룹니다.
- Blocker: progress, write, close가 정직하게 진행될 수 없는 구조화된 이유입니다.
- 잔여 위험 요약: 알려진 남은 불확실성, 확인하지 못한 조건, 한계, trade-off를 보여 주는 active compact path입니다. 상세한 residual-risk record는 승격 전까지 later/profile 자료입니다.
- Projection과 template: Core state와 ref에서 파생한 표시입니다. 읽기 쉽거나 사람이 고쳤다는 이유로 권한이 되지 않습니다.

Discovery와 요구사항 구체화는 Task, Change Unit, `user_judgment` owner path를 통해 지속됩니다. 별도 구체화 brief, design display, journey 또는 reconcile record, 상세 risk record, Eval record, Manual QA record, 전체 Evidence Manifest는 owner가 명시적으로 승격하기 전까지 현재 활성 MVP Core state가 아닙니다.

<a id="finding-routing"></a>
<a id="finding-라우팅"></a>

Command, Run, review, validator, diagnostic, QA, verification에서 나온 finding은 active owner path를 통해 라우팅될 때만 Core에 영향을 줍니다. 예를 들면 blocker, evidence summary, user judgment, Change Unit update, close blocker입니다. Chat이나 report prose에 남은 finding은 상태가 아닙니다.

<a id="judgment-route-boundaries"></a>

## 4. 사용자가 소유하는 판단 경계

사용자가 소유하는 판단은 하네스가 추론하지 않고 사용자에게 묻거나 사용자의 선택으로 보존해야 하는 경계입니다. 정확한 `UserJudgment` schema와 API field는 [API Schema Core](api/schema-core.md)와 [MVP API](api/mvp-api.md)가 담당합니다. 이 section은 판단 경계의 의미를 소유합니다.

판단 종류는 서로 분리됩니다.

- 제품 판단: 제품 동작, UX, 문구, 릴리스에 드러나는 약속, 사용자 가치.
- 기술 판단: architecture, dependency, migration, public interface, compatibility, security/privacy, 중요한 기술 방향.
- 범위 판단: 범위 확장, 비목표 제거, Change Unit boundary, Autonomy Boundary 변경.
- 민감 동작 승인: 경계가 정해진 이름 붙은 민감 단계에 대한 permission.
- QA 면제 판단: policy가 허용하는 Manual QA requirement의 범위 있는 면제.
- 검증 위험 수락: required verification이 빠졌거나 면제된 데 따른 위험 수락.
- 최종 수락: 경로가 acceptance를 요구할 때 사용자가 결과를 판단하는 것.
- 잔여 위험 수락: 요청한 close를 위해 이름 붙은 보이는 잔여 위험을 수락하는 것.
- 취소 판단: 성공 결과 없이 Task를 멈추는 것.

모호한 동의는 좁게 해석합니다. "진행해", "좋아", "looks good" 같은 포괄적 승인은 다른 판단 종류를 조용히 만족할 수 없습니다. 하나의 사용자 답변이 여러 judgment route를 만족하려면 prompt가 그 판단들을 명시적으로 물었고, Core가 각 판단을 affected object, scope, consequence, close/write impact와 함께 호환되게 기록해야 합니다.

<a id="boundaries-and-non-substitutions"></a>
<a id="evidence-verification-qa-final-acceptance-and-risk"></a>
<a id="증거-검증-수동-qa-최종-수락-위험"></a>

## 5. 대체 불가능 규칙

Core는 아래 분리를 지켜야 합니다.

- 대화, 생성된 Markdown, Projection prose, report text는 Core state를 대신하지 않습니다.
- 증거, log, screenshot, artifact, test output은 최종 수락, 수동 QA, 검증, 잔여 위험 수락을 대신하지 않습니다.
- QA는 최종 수락이 아닙니다. QA 면제 판단은 QA 증거나 QA 통과가 아닙니다.
- 검증 위험 수락은 verification, detached verification, assurance upgrade가 아닙니다.
- 민감 동작 승인은 제품 방향, 기술 방향, 범위, correctness, 증거, QA, 최종 수락, 잔여 위험 수락, Write Authorization을 대신하지 않습니다.
- 제품 판단, 기술 판단, 범위 판단은 서로를 대신하지 않습니다.
- 최종 수락은 증거를 만들거나, 증거 공백을 지우거나, QA를 면제하거나, verification을 증명하거나, 민감 동작 승인을 부여하거나, scope를 바꾸거나, 잔여 위험을 수락하거나, blocker를 override하지 않습니다.
- 잔여 위험 수락은 work를 verify하거나, no-risk close를 만들거나, 증거를 만족하거나, QA를 만족하거나, 최종 수락을 암시하지 않습니다.
- Stale 또는 failed Projection은 그 자체로 close를 막거나 허용하지 않습니다. 현재 Core close state와 blocker가 기준입니다.

이 규칙은 사용자에게 보이는 화면이 compact하게 표시될 때도 유지됩니다. Compact output은 친절할 수 있지만 권한 경계를 합치면 안 됩니다.

<a id="gates"></a>
<a id="gate-rule-map"></a>

## 6. 관문

Gate는 진행, 쓰기, Run 기록, 닫기를 위한 Core compatibility dimension입니다. Gate가 reference model에 있다는 사실만으로 모든 Task에 required가 되지는 않습니다. Required 여부는 active stage/profile, user request, task type, policy, sensitivity, explicit requirement가 정합니다.

- <a id="scope-gate"></a>Scope Gate: active scope가 요청한 write 또는 close-relevant work를 cover하는지.
- <a id="decision-gate"></a>Decision Gate: unresolved user-owned judgment가 progress, write, close를 막는지. 민감 동작 승인, evidence, verification, QA, 최종 수락, 잔여 위험 수락을 대신하지 않습니다.
- <a id="approval-gate"></a>Approval Gate: scoped sensitive-action approval이 needed, pending, usable, denied, expired, drifted 중 어디인지. 민감 동작에 대한 permission일 뿐입니다.
- <a id="design-gate"></a>Design Gate: enabled design-quality finding이 Core-backed blocker로 라우팅되는지. Broad design-quality catalog는 기본 active MVP blocker가 아닙니다.
- <a id="evidence-gate"></a>Evidence Gate: required close-relevant evidence가 absent, partial, sufficient, stale, blocked 중 어디인지.
- <a id="verification-gate"></a>Verification Gate: required verification이 satisfied, proper risk path로 waived, failed, pending, blocked 중 어디인지. Verification은 active owner path가 required로 만들 때만 required입니다.
- <a id="qa-gate"></a>QA Gate: required Manual QA가 satisfied, allowed waiver로 waived, failed, pending, blocked 중 어디인지. 스크린샷이나 automated check만으로 Manual QA가 만들어지지 않습니다.
- <a id="acceptance-gate"></a>Acceptance Gate: final acceptance가 required인지, required라면 close basis가 보인 뒤 기록되었는지.
- <a id="capability-boundary"></a>Capability Boundary: surface capability는 blocker, validator finding, guarantee display에 영향을 주지만 authority를 만드는 gate가 아닙니다. Capability가 부족하면 claim을 좁히거나, action을 block하거나, capability blocker를 만들어야 합니다. 검증이나 사전 차단이 실제로 일어난 것처럼 꾸미면 안 됩니다.

공개 응답에서 gate state를 어떻게 노출하는지는 [API Schema Core](api/schema-core.md)와 method owner가 담당합니다. Core는 compatibility meaning과 stale gate summary를 write 또는 close 전에 다시 계산해야 한다는 규칙을 소유합니다.

<a id="lifecycle-and-transitions"></a>

## 7. 작업 생명주기

Lifecycle은 Core 상태 전이 규율입니다. Display script가 아닙니다. Active fixture와 schema owner가 exact value를 노출할 수 있지만 Core 원칙은 다음과 같습니다.

- Task는 owner path를 통해서만 shaping, ready, executing, waiting user judgment, blocked, completed, cancelled, superseded 상태로 움직일 수 있습니다.
- Advice/read-only work는 product-file write를 만들면 안 됩니다. Write-capable direct/tracked work는 compatible scope와 write authority를 거쳐야 합니다.
- 제품 파일 쓰기 경로는 범위 확정, 필요한 user judgment와 sensitive-action check, `prepare_write`, compatible product-write Run 하나, `record_run`, evidence/blocker update, `close_task`를 통과합니다.
- `close_ready`는 derived condition입니다. `lifecycle_phase`가 아니며 Task를 completed로 옮기지 않습니다. Task를 completed로 옮기는 것은 `close_task`뿐입니다.
- Idempotency replay는 state transition, event, Write Authorization, Run, artifact, evidence update, close effect를 중복 만들면 안 됩니다.
- `dry_run` 호출은 가능한 outcome을 설명할 수 있지만 기준 상태, consumable Write Authorization, artifact, close state, replay row를 만들지 않습니다.

<a id="stable-event-catalog"></a>

Stable event name은 Core change를 위한 append-only history label입니다. 그 자체가 권한은 아닙니다. Catalog는 Task lifecycle update, `prepare_write` decision, Write Authorization creation/consumption/staling/expiry/revocation, Run recording, user judgment update, gate recompute, evidence update, blocker update, residual-risk visibility 또는 acceptance, waiver recording, close attempt, close success 또는 cancellation을 cover해야 합니다. Exact event payload와 persistence는 API와 Storage가 담당합니다.

<a id="prepare_write"></a>

## 8. `prepare_write` 권한

`prepare_write`는 제품 파일 쓰기를 위한 유일한 쓰기 전 호환성 판단 지점입니다. Intended operation을 active Task, Change Unit, scope, baseline, Autonomy Boundary, required user-owned judgment, 민감 동작 승인, surface capability, active design-policy precondition과 비교합니다.

Compatible non-dry-run allowed path만 consumable Write Authorization을 만듭니다. Dry-run response, `blocked`, `approval_required`, `decision_required`, `state_conflict`는 response, blocker, error state로만 남습니다. Consumable authorization row, replay row, evidence record, close state, write authority를 만들면 안 됩니다.

Write Authorization은 협력형 하네스 기록입니다. 연결된 에이전트나 접점에게 intended write가 현재 하네스 상태와 호환된다고 알려줄 수 있습니다. OS 권한을 주거나, 샌드박스를 강제하거나, 임의 도구를 막거나, storage를 변조 방지 상태로 만들거나, operation을 격리하지 않습니다.

MCP 또는 연결된 접점이 필요한 cooperative check를 수행할 수 없으면 정직한 결과는 hold, blocker, degraded guarantee display, capability error 중 하나입니다. Preventive 또는 isolated wording은 해당 operation을 cover하는 정확한 boundary가 문서화되고 증명되었을 때만 사용할 수 있습니다.

<a id="record_run"></a>

## 9. `record_run` 권한

`record_run`은 실행 또는 관찰을 기록합니다. Write를 사후 승인하는 두 번째 기회가 아닙니다.

Product-write Run에서는 Core가 compatible active Write Authorization을 load해야 합니다. Surface가 정직하게 관찰할 수 있는 범위에서 observed attempt를 stored authorized attempt와 current state에 비교하고, compatible할 때만 authorization을 정확히 한 번 소비합니다. Missing, stale, expired, revoked, consumed, incompatible, insufficiently observable authorization은 successful consumption으로 기록할 수 없습니다.

`record_run`은 owner-approved artifact path를 통해서만 `ArtifactRef` value를 등록하거나 연결할 수 있습니다. Raw secret, token, forbidden sensitive log, arbitrary caller path, untrusted bytes는 evidence를 완성해 보이게 하려고 저장하면 안 됩니다. Reject, redaction, omitted/blocked 표시, approved safe handle 중 하나로 처리해야 합니다.

Read-only와 shaping-only Run은 product-file change를 보고하지 않을 때만 Write Authorization 없이 기록할 수 있습니다. Active owner path가 지원하면 violation 또는 audit record가 observed problem을 문서화할 수 있습니다. 하지만 관련 owner record를 통해 repair되기 전까지 completion evidence, 최종 수락, 잔여 위험 수락, close readiness, QA, verification을 만족하지 않습니다.

<a id="close_task"></a>

## 10. `close_task` 권한

`close_task`는 단일 완료 판단 지점입니다. Agent summary, final report, acceptance처럼 보이는 chat, Projection, Eval, QA note, evidence display는 close에 정보를 줄 수 있습니다. 하지만 그것만으로 Task를 닫지 않습니다.

성공 닫기에서는 Core가 현재 Task state, open Run, scope, user-owned judgment, 필요한 민감 동작 승인, active design-policy blocker, required evidence sufficiency, close-relevant artifact availability, required final acceptance, applicable residual-risk visibility 또는 acceptance를 close intent와 비교해야 합니다.

MVP close는 later assurance material을 활성 응답 의미로 끌어오면 안 됩니다. Detached verification, `completed_verified`, detailed Manual QA close field, full Evidence Manifest behavior, assurance-profile display detail은 owner가 명시적으로 켜기 전까지 later/profile behavior입니다.

Required scope, judgment, evidence, artifact availability, final acceptance, residual-risk visibility, residual-risk acceptance, 안전 조건이 남아 있으면 `close_task`는 completed라고 꾸미지 말고 blocker를 반환해야 합니다. Public response가 primary error 하나를 고르더라도 secondary 닫기 blocker와 ref는 다음 안전한 행동을 정할 만큼 보여야 합니다.

Cancellation과 supersession은 정직한 terminal path입니다. Successful completion이 아닙니다. Risk-accepted close는 이름 붙은 accepted risk가 있는 successful close입니다. Verified close도 아니고 no-risk close도 아닙니다.

<a id="invalid-state-combinations"></a>

## 11. Blocker

Blocker는 상태 전이가 정직하게 진행될 수 없는 구조화된 이유입니다. Progress, write, Run recording, close를 막을 수 있습니다. 가능한 경우 affected Task 또는 Change Unit, category, missing 또는 incompatible condition, related refs, next safe action을 이름 붙여야 합니다.

흔한 blocker category에는 missing active Task, missing active scope, out-of-scope write intent, unresolved user-owned judgment, missing sensitive-action approval, incompatible Autonomy Boundary, insufficient surface capability, missing 또는 invalid Write Authorization, stale baseline, missing evidence, stale 또는 unavailable artifact support, active design-policy blocker, missing final acceptance, hidden residual risk, unaccepted close-relevant residual risk, unsafe open Run, cancellation conflict, supersession conflict가 있습니다.

Invalid state combination은 blocker, rejection, repair path가 되어야 합니다. Projection prose, 포괄적 승인, 적용되지 않는 면제, conflict를 숨기는 close result로 덮으면 안 됩니다.

<a id="waiver-semantics"></a>

## 12. 면제

면제는 policy가 허용하는 이름 붙은 requirement에 대한 scoped exception입니다. 어떤 requirement를 건너뛰었는지, affected Task와 Change Unit, reason, actor, timing, affected gate 또는 close impact, 필요한 만료 조건 또는 다음 조치, close-relevant residual risk를 보존해야 합니다.

허용되는 면제 경로는 좁습니다.

- Design-policy owner가 허용할 때만 design-policy waiver.
- Required Manual QA가 active이고 policy가 허용할 때만 QA waiver.
- Required verification이 active이고 사용자가 missing 또는 waived verification의 이름 붙은 위험을 수락할 때만 verification-risk acceptance.

허용되지 않습니다.

- Product write에 대한 scope waiver.
- Sensitive-action approval waiver.
- Completion에 evidence가 required인데 evidence waiver.
- Acceptance가 required인데 final acceptance waiver.
- Residual-risk visibility waiver.

Decision deferral은 waiver가 아닙니다. QA waiver는 QA pass가 아닙니다. Verification-risk acceptance는 verification이 아닙니다. Waiver는 이름 붙인 requirement 하나만, 그리고 그 requirement의 owner path가 허용하는 범위에서만 unblock할 수 있습니다.

<a id="13-residual-risk"></a>

## 13. 잔여 위험

잔여 위험은 close에 의미가 있는 알려진 남은 불확실성, 확인하지 못한 조건, 한계, trade-off입니다. 알려진 close-relevant residual risk는 successful close 전에 보여야 합니다. Close가 그 risk acceptance에 의존한다면 Core는 visible risk와 related refs에 연결된 compatible residual-risk acceptance `user_judgment`를 요구합니다.

잔여 위험 수락은 work를 verify하지 않고, evidence를 만족하지 않고, QA를 만족하지 않고, 민감 동작 승인을 주지 않고, 최종 수락을 만들지 않고, no-risk result를 만들지 않습니다. 요청한 close를 위해 이름 붙은 보이는 위험을 사용자가 수락했다는 기록입니다.

현재 활성 경로는 compact residual-risk summary, blocker, evidence ref, `user_judgment` ref를 사용합니다. Rich residual-risk record, review workflow, handoff report, later assurance display는 승격 전까지 later/profile 자료입니다.

## 14. 소유자 간 연결

Core 권한이 다른 계약과 닿을 때는 아래 owner를 사용합니다.

- 공개 API method, request/response shape, envelope, state conflict, error: [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md).
- Storage table, DDL, runtime home layout, lock, migration, artifact storage, enum hardening: [Storage](storage.md).
- Projection freshness, readable view, managed block, human-editable section, active rendered template body: [Projection과 Template 참조](projection-and-templates.md).
- Security guarantee language, cooperative/detective/preventive/isolated label, local access posture: [보안 참조](security.md).
- 런타임 경계 안의 placement와 Core-only mutation authority: [런타임 경계 참조](runtime-boundaries.md).
- Design-quality 활성 역할과 close-impact 경계: [설계 품질](design-quality.md).
- Connector capability profile과 surface-specific fallback behavior: [Agent 통합 참조](agent-integration.md).
- Conformance example, future fixture boundary, operations entrypoint 후보: [적합성 참조](conformance.md), [Later 후보 색인: Future fixture families](../later/index.md#future-fixture-families), [Later 후보 색인: 운영 후보](../later/index.md#operations-candidates).

다른 문서가 exact schema, DDL table, rendered template body, later/profile catalog를 필요로 하면 여기서 다시 정의하지 말고 owner로 연결해야 합니다.
