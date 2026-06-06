# 적합성 참조

## 1. 현재 상태

이 저장소는 현재 문서 전용이며 문서 검토 상태입니다. Harness Server runtime, 적합성 실행기, 실행 가능한 fixture 파일, 생성된 conformance report, 생성된 runtime artifact, 현재 runtime conformance result는 없습니다.

이 문서는 현재 적합성 계획의 담당 문서입니다. 실행 가능한 suite 아님. Test catalog도 아니고, 향후 서버 동작이 이미 실행되었다는 증거도 아닙니다. 현재 단계와 인계 상태는 [MVP 계획](../build/mvp-plan.md#문서-수락-상태)이 담당합니다.

## 2. 적합성이 뜻하는 것

적합성은 향후 Harness 구현이 특정 동작을 Harness가 소유한 권한 기록에 맞게 증명할 수 있다는 뜻입니다. 향후 점검은 owner가 정의한 Core, API, operator action 하나를 실행하고, captured fact를 structured expectation과 비교해야 합니다.

아래 세 층을 분리합니다.

| 층 | 의미 | 현재 상태 |
|---|---|---|
| 문서 점검 | Link, terminology, owner boundary, active/later wording, security wording, 영어/한국어 의미 일치를 보는 읽기 전용 Markdown maintenance check. | 현재 문서 유지보수 보조 자료일 뿐입니다. Runtime conformance가 아닙니다. |
| 동작 예시 | 첫 smoke와 현재 MVP에서 기대하는 동작을 작게 보여주는 예시. | 계획 참조일 뿐입니다. Fixture 파일도 아니고 pass/fail 기준도 아닙니다. |
| runtime conformance | 구현된 Core/API/storage/operator 동작을 대상으로 하는 향후 실행 점검. | 아직 없습니다. |

Conformance는 생성된 prose를 판단하지 않습니다. 향후에는 owner-state effect, response fact, storage effect, 승격된 stable event, artifact ref, blocker, error, forbidden side effect를 판단합니다.

## 3. 아직 없는 것

아래 항목은 향후 구현 작업이며 현재 저장소 내용이 아닙니다.

- 실행 가능한 fixture 파일 또는 fixture directory
- conformance runner 또는 `harness conformance run` 구현
- 생성된 conformance artifact, report, projection, runtime state, Harness Runtime Home data
- 현재 runtime `PASS`, `WARN`, `FAIL` result
- 현재 MVP 또는 later 후보를 위한 active fixture suite
- 예방적 차단, OS 권한 제어, 임의 도구 샌드박스, 변조 방지 저장소, 보안 격리, profile-gated `preventive` / `isolated` 보장에 대한 현재 증명

이 문서의 예시는 구현 계획을 도울 수 있습니다. 하지만 runtime state, acceptance evidence, close readiness, implementation readiness를 만들지 않습니다.

## 4. fixture 형태

향후 fixture는 Harness Server가 생긴 뒤 runner가 읽는 일반 structured input이어야 합니다. 이 문서는 의도한 fixture 형태만 기록합니다. Full YAML body는 제공하지 않습니다.

승격된 fixture에는 아래 부분이 필요합니다.

| 부분 | 목적 |
|---|---|
| `scenario_id` | 검토할 동작의 stable identifier. |
| owner scope | Action을 해석하는 데 필요한 Task, Change Unit, surface, state-version, owner ref. |
| action | Owner request schema를 사용하는 public Core/API/operator request 하나. |
| initial authority context | 동작 전의 관련 Core 상태, storage row, artifact ref, 접점 역량. |
| expected authoritative assertions | Structured response, state, storage, event, artifact, blocker, error, guarantee, forbidden-side-effect fact. |
| owner links | Exact value를 정의하는 API, Core, Storage, Security, ArtifactRef, policy owner. |

Materialized fixture는 public owner schema를 사용해야 합니다. Fixture-only enum value, pseudo-field, 상태값으로 쓰는 localized display label, prose-only expectation, later 후보 전용 value를 만들면 안 됩니다.

## 5. 주장 권한

주장 권한은 scenario prose보다 좁습니다.

향후 권한 있는 assertion은 다음입니다.

- public owner API가 반환한 response fact
- Core가 소유한 Task, Change Unit, user judgment, Write Authorization, Run, evidence summary, blocker, close, residual-risk state
- Storage가 소유한 row effect, JSON `TEXT` owner field, idempotency/replay fact, state-version effect
- Core owner가 이름을 승격한 뒤의 stable `task_events`
- artifact proof가 필요할 때의 `ArtifactRef`, artifact link, `sha256`, `size_bytes`, `content_type`, `redaction_state`, retention, availability, file-integrity fact
- API/Core owner의 primary `ErrorCode`, error detail, structured blocker field
- Security와 Agent Integration owner에 맞는 guarantee-level fact
- durable authorization 없음, Run row 없음, artifact mutation 없음, close-state change 없음 같은 forbidden side effect 부재 assertion

현재 활성 예시는 `cooperative`와 지원되는 `detective` fact만 assertion할 수 있습니다. `preventive` 또는 `isolated` assertion은 승격된 profile과 담당 문서가 정의한 증명이 있을 때만 유효합니다. Conformance 계획 문구만으로 이런 보장이 현재 실행 가능하거나 증명된 것이 되지 않습니다.

권한이 없는 자료는 다음입니다.

- prose scenario description
- comment와 author note
- rendered Markdown, status prose, Journey Card prose, close report prose, agent summary
- documentation-check `PASS`, `WARN`, `FAIL` label
- projection. 단, projection support가 명시적으로 범위에 들어온 경우 freshness 또는 availability assertion은 예외입니다.

## 6. 대표 예시

이 대표 예시는 compact behavior reference입니다. Fixture 파일도 아니고, full YAML도 아니고, 현재 실행 가능한 suite도 아니며, runtime pass/fail 기준도 아닙니다.

| 예시 | 동작 | 향후 fixture가 사용할 structured assertion |
|---|---|---|
| `MVP-ACTIVE-prepare-write-blocked-or-dry-run-no-durable-authorization` | `prepare_write`가 blocked 또는 dry-run 정보를 반환하지만 durable authorization을 만들지 않습니다. | Response에 소비 가능한 Write Authorization이 없습니다. `write_authorizations`에는 inserted active row가 없습니다. Run, artifact, evidence, close, final-acceptance, residual-risk effect가 생기지 않습니다. Blocker 또는 dry-run fact는 API/Core owner와 맞습니다. |
| `MVP-ACTIVE-prepare-write-committed-scoped-authorization` | Committed allowed `prepare_write`가 scope가 정해진 single-use Write Authorization을 기록합니다. | Response authorization scope, Core state, `write_authorizations.attempt_scope_json`이 Task, Change Unit, state version, surface, intended paths/tools/commands/network/secrets/sensitive categories, baseline ref, related judgment, guarantee level에서 같은 의미를 가집니다. |
| `MVP-ACTIVE-close-task-blocks-missing-acceptance-or-risk-condition` | 필요한 final acceptance가 없거나 close-relevant residual risk가 요구 수준으로 보이지 않거나 수락되지 않으면 `close_task`가 차단됩니다. | Response blocker는 owner category와 필요한 경우 `required_judgment_kind`를 사용합니다. Task는 completed가 되지 않습니다. Close record가 missing acceptance나 risk acceptance를 대신하지 않습니다. Evidence, final acceptance, residual-risk state는 분리되어 남습니다. |

## 7. Catalog-only future 경계

Future fixture family는 [Later 후보 색인: Future fixture families](../later/index.md#future-fixture-families)에만 둡니다. 그 색인은 future candidate 이름만 나열합니다. Full scenario script, fixture body, active API payload example, suite requirement를 담으면 안 됩니다.

현재 future fixture family 이름은 다음입니다.

- Intake and decision routing
- Core, evidence, verification, and close
- Artifact redaction and export non-leakage
- Agency and user-judgment separation
- Connector capability honesty
- Design-quality and stewardship
- Context hygiene and resume freshness
- Projection, reconcile, and verification boundary
- Operations diagnostics, export, recover, and handoff
- Browser QA Capture

Family 이름을 적었다고 현재 MVP나 later 후보 requirement가 되지는 않습니다. 향후 owner가 좁은 동작을 scope, fallback behavior, exact contract, proof expectation과 함께 승격해야 실행 가능한 fixture 자료가 생깁니다.

## 8. Metrics 경계

현재 문서 세트에서 metrics는 적합성 권한이 아닙니다. Future local derived metrics는 진단이나 계획에 유용할 수 있지만, owner가 승격하기 전까지 읽기 전용 파생 표시로 남습니다.

Metrics는 Core state를 만들거나, evidence를 충족하거나, QA 또는 verification을 통과시키거나, write를 authorize하거나, final result를 수락하거나, residual risk를 수락하거나, work를 close하거나, implementation readiness를 증명하거나, runtime conformance를 대신하면 안 됩니다. 향후 metric이 승격되면 owner가 source record, freshness boundary, display wording, non-substitution rule을 정의해야 합니다.
