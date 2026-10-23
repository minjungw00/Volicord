# Reader experience design

Status: accepted experience requirements, adopted 2026-10-19. Implementation,
focused technical checks, semantic quality and human acceptance are separate.

This is the maintained entry for D-01–D-37 and AC-01–AC-35.
D-19–D-27 adopt the subsequent 2026-10-19 user decisions; they do not
retroactively qualify the rejected review_101901 output. Specialized owners
retain architecture, domain, privacy, versioning and failure authority. Read this
entry with [Projection](projections-and-documents.md) and the affected owner.
Recommendations labelled 설계 권고 remain implementation choices, not user decisions.

The supplied original, basis manifest and four reference images remain unchanged
under ignored `rebuild/.local/redesign_2026-10-19/`. Their Markdown basis SHA-256 is
`b1c36f07e6390eec6074e399dcedb719921c121bcb6f31e875ced02a997a22bc`.
Reference pictures provide hierarchy, blue-gray palette and evidence-panel direction;
they supply no Product prose, status, source coordinates or evaluator answers.

The earlier delivered Product UI was rejected by subsequent user feedback.
[Historical integration evaluation](../../validation/viewer-design/product-integration-review.md)
remains an observation of its own candidate. Its pending manifest does not erase
that rejection. New output is unassessed until actually reviewed.

## 2. 제품 목표와 보존할 경계

### 2.1 사람이 이해할 결과가 핵심 콘텐츠다

저장소 설명은 시스템이 하는 일, 중요한 구성요소의 책임, 구성요소가 협력하는 방식, 대표적인 요청·데이터 흐름, 중요한 설계 이유·절충·제약을 설명해야 한다. Work 설명은 요청된 문제, 수행한 구현 또는 조사, 결과와 확인 범위, 관련 선택, 남은 일을 연결해야 한다.

파일·함수·관계·Source 목록, 정확한 해시, coverage 통계는 설명을 만드는 재료와 확인 수단이다. 목록이 충실해도 독자가 구조를 스스로 합성해야 한다면 설명 기능은 충분하지 않다. **근거를 숨기지 않는 것과 근거를 주 콘텐츠로 나열하는 것은 다르다.**

정보의 정확성·충분성과 시각적 표현·탐색 품질은 상쇄하지 않는다. 사용자는 무슨 작업인지 구별하고, 중요한 내용을 읽고, 필요한 코드로 확인한 다음 읽던 문맥으로 돌아올 수 있어야 한다.

### 2.2 바꿀 수 있는 구현과 유지할 의미

| 바꿀 수 있는 직접 관련 구현 | 계속 유지할 제품 의미 |
|---|---|
| `QuestionAnswers` 구성, 문단 유형 배열, 설명 제출 형식, Projection, UI 데이터 계약 | 원래 사용자 요청·기록과 생성된 해석의 구분 |
| `reading.rs`, `render.rs`, DOM, CSS, 라우팅·선택 상태, 컴포넌트 배치 | 정확한 Project·Work·Decision·Source identity와 revision·scope |
| 제목·요약의 생산·갱신 방식과 독립적인 읽기 단위 | 근거·수명·삭제·권한을 갖춘 Derived 콘텐츠라는 성격 |
| 초기 발췌 선택, 필요한 근거의 제한적 조회·페이징, 캐시 구현 | 허용된 Source만 읽고 변경·만료·삭제 상태를 올바르게 반영 |
| 기능을 찾는 내비게이션과 문서·도구의 표현 | 기존 기능과 보호된 동작의 접근 가능성 |
| 필요한 현재 설명 format 및 결합된 producer/consumer | current-only 정책, 안전한 실패와 원본 증거 보존 |

화면을 바꾸기 어렵다는 이유로 기존 타입과 렌더링 구조를 보존하지 않는다. 반대로 큰 변경 자체가 목표도 아니다. 실제 요구를 만족하는 최소한의 **일관된 책임 범위**를 선택한다.

---

## 3. 최신 확정 요구사항

| ID | 확정 요구 | 구현에서 제외해야 할 오해 | 근거 |
|---|---|---|---|
| **D-01** | 사람이 저장소·작업의 구조와 동작을 이해할 수 있는 설명이 핵심이다. | 분석 성공·근거 수·그래프 노드 수를 설명 품질로 대체하지 않는다. | U1, U2 |
| **D-02** | 디자인과 정보 설계를 비슷한 중요도의 필수 조건으로 다룬다. | 데이터 완료 후 마지막에 CSS만 정리하는 작업으로 축소하지 않는다. | U2 |
| **D-03** | 기존 정보·시각 구조를 유지하는 것 자체를 목표로 삼지 않는다. | 기존 필드별 섹션·fallback·중복 메뉴를 새 스타일 아래 유지하지 않는다. | U2, U4 |
| **D-04** | 브랜드는 좌측, 내비게이션 목록은 우측에 두고 저장소 설명을 포함한다. | 가운데 메뉴와 우측 읽기 전용 배지를 유지하지 않는다. | U4 |
| **D-05** | D-28 supersedes the earlier menu consolidation: precisely Overview / Repository explanation / Work form the single navigation. | 중복된 상·하단 Project 메뉴를 만들지 않는다. | U4 |
| **D-06** | 읽기 전용 표시는 내비게이션 밖에 표시한다. | 수정 가능한 도구까지 제품 전체가 읽기 전용인 것처럼 표시하지 않는다. | U4; 권한 범위는 R8 |
| **D-07** | 클릭 가능한 주요 요소에 버튼 등 명확하고 일관된 UI를 적용한다. | 기본 선 장식만으로 동작을 표시하거나 native 상호작용 의미를 없애지 않는다. | U4 |
| **D-08** | 개요는 Project 이름·Project Purpose 중심으로 구성하고 중복된 ‘저장소 이해하기’ 요약 영역을 대체한다. | 저장소 기능 설명이나 제품 헌장을 canonical Project Purpose로 승격하지 않는다. | U4; 의미 경계는 R7 |
| **D-09** | 개요에는 `PROJECT OVERVIEW`를 사용하고 `RECENT WORK` 작은 heading을 제거한다. | ‘최근 작업’ 본제목까지 삭제하거나 새 중복 eyebrow를 추가하지 않는다. | U4 |
| **D-10** | 개요 heading은 해당 Project의 실제 이름이다. | `Volicord`를 하드코딩하거나 저장소 설명 제목으로 대체하지 않는다. | U4; 현황 확인 A1, R2 |
| **D-11** | Work 카드 제목은 실제 요청을 바탕으로 생성된 작업 이름, 본문은 문제와 해결·조사 결과의 요약이다. | `작업 기록`과 긴 Goal 원문을 완성된 카드로 판정하지 않는다. 결과가 없으면 해결을 발명하지 않는다. | U4, U5 |
| **D-12** | Work 카드에 작업 생성 시각과 마지막 작업 시각을 구분해 제공한다. | 하나의 `observed_at`을 두 사건으로 재사용하거나 Git·파일 시각으로 추정하지 않는다. | U4; 정확한 사건 정의는 §7 |
| **D-13** | D-29 supersedes Viewer-wide Decision inspection: related Decisions belong in their actual Work; unassociated Decisions remain outside Viewer. | Decision 저장·조회 기능을 삭제하거나 모든 Project 결정을 임의의 Work에 붙이지 않는다. | U4 |
| **D-14** | Work 상세는 문제·수행 방식·메커니즘·결과·판단·한계를 연결하는 독자용 문서다. | 기존 레코드 필드의 이름만 바꿔 늘어놓는 구성을 재사용하지 않는다. | U1, U4, U5 |
| **D-15** | 저장소 전체 이해는 Work가 없어도, Work가 있어도 독립적으로 제공한다. | 가짜 Work를 만들거나 Work 인용 목록으로 저장소 설명을 대체하지 않는다. | U1 |
| **D-16** | 프로토타입은 디자인·UI 참고로 사용하며 실제 문장·결과·상태·코드 위치는 채택하지 않는다. | prototype 또는 평가 정답을 생성 입력·Product 데이터에 주입하지 않는다. | U3 |
| **D-17** | 실제 Product 생산 결과와 실제 화면으로 평가하고, 내용·디자인·기술·인간 수용을 구분한다. | 손편집·합성 fixture·브라우저 성공으로 실제 설명 품질과 수용을 대신하지 않는다. | U1, U2, U5 |
| **D-18** | 새 기술의 디자인 효과, 성능·용량·설치·실행·유지보수 비용을 검토한다. 기술 선택은 미확정이다. | React·Vue 도입 자체를 완료나 자동 승인으로 간주하지 않는다. | U2 |

### Subsequent reader and collection decisions

The following rows extend the existing identifiers; specialized owners define
record meaning, read authority, lifetime and validation. The ignored handoff and
original Korean plan are preserved evidence of feedback, not parallel authority.

| ID | Accepted outcome | Excluded interpretation |
| --- | --- | --- |

| **D-19** | Work reads as a flexible grounded technical article: background, supported cause, actual approach/comparison, outcome and limits as relevant. | No compulsory empty sections, invented errors/retries/changes, or enum-to-section mapping. |
| **D-20** | Decision explains the recorded problem, alternatives, agent recommendation/rationale, actual user choice and separately retained user rationale. | Missing user rationale stays absent; unavailable technical Source does not erase a retained recommendation reason. |
| **D-21** | Verification identifies named check, execution Source/run, target basis, time, outcome/termination, coverage and limits. | Exit zero is not assertion coverage; a later unrelated pass never resolves an earlier failure or establishes a retest. |
| **D-22** | One accessible persistent document section navigation; warnings distinguish current blocking, historical failure, unverified and unavailable evidence. | Remove duplicated shortcuts; color alone and a generic disclaimer cannot carry material limits. |
| **D-23** | A meaning-based evidence index connects article claims to code, commands and original records; raw objects require intentional inspection. | Raw-dump-first presentation and evidence counts do not establish explanation quality. |
| **D-24** | D-30 supersedes the Viewer inspector requirement. Scoped inspection and protected operations remain available through Operations, CLI and MCP. | No free SQL, browser DB access, arbitrary execution, cross-Project aggregate leaks or weaker protected actions. A scoped inspector is an implementation recommendation. |
| **D-25** | Explicit authorized task-time collection records actual states, attempts and checks with original identity/time/provenance. | No historical reconstruction, hidden capture, transcript/chain-of-thought retention or widened host/provider grant. |
| **D-26** | Campaign admission is per case × claim, with a genuine prospective positive case for task-time collection. | Historical records, replay and synthetic controls keep useful applicability but cannot substitute for that positive case. |
| **D-27** | Technical, evidence sufficiency, semantic, presentation, human and authority outcomes remain independent. | Source-first assessment precedes authored interpretation; presentation-first readers receive no answer oracle. Missing positive evidence cannot become a pass by exclusion. |

### Focused Viewer boundary — accepted 2026-10-20

These user decisions explicitly supersede the Viewer location of D-05, D-13 and
D-24, and conflicting AC-10/15/22 wording. Their underlying record, permission,
privacy and non-Viewer capabilities remain accepted. Earlier passing checks do
not establish this new boundary; original feedback and evidence stay unchanged.

| ID | Accepted outcome | Excluded interpretation |
| --- | --- | --- |
| **D-28** | Exactly Overview / Repository explanation / Work; brand left, navigation right, read-only disclosure outside navigation. | No Tools disclosure or second Project row. |
| **D-29** | Explain actual related Decisions within Work, preserving alternatives, recommendation, user choice/rationale and applicability. | No standalone catalog, invented relation or lost unassociated records; CLI/MCP/Operations retain those reads. |
| **D-30** | Remove Viewer management/inspection routes and selectors, with explicit controlled failures. | No orphan UI, silent fallback, hidden renderer or compatibility alias. |
| **D-31** | Explain relevant structure and mechanisms contextually; exact cited code remains readable with original bytes and return point. | No unrelated generic Code catalog or loss of claim-specific evidence. |
| **D-32** | Preserve existing non-Viewer operations and exact request-scoped guarded fallback with unchanged authority. | No navigable administration destination or implicit confirmation. |
| **D-33** | Live/offline reading loads only required projections; offline carries bounded explanations and admitted Sources with truthful omissions/currentness. | No four Tools pages, document previews, bulk diagnostic/code export or generation on reads. |

### Evidence hierarchy and repository breadth — accepted 2026-10-21

These decisions adopt the latest user rejection of indiscriminate Source actions,
the scroll-then-open outline failure, task-sized repository explanations and
quality loss from publication/budget choices. Earlier automatic browser passes
are technical observations, never human approval. Originals stay immutable.

| ID | Accepted outcome | Excluded interpretation |
| --- | --- | --- |
| **D-34** | Recorded/admitted proof and default visible evidence actions have separate responsibilities. Names and nonassertive headings/labels normally have no inline Source buttons; material mechanism/cause/result claims keep obvious exact claim-specific actions. Original heading/lead provenance stays deliberately inspectable. | Hiding an action never waives grounding, Source validation or lifecycle; no generic file index replaces per-claim authority. |
| **D-35** | The compact TOC remains persistent when expanded after scrolling: trigger and bounded independently scrollable list stay usable. Selection/close restores exact reading position and focus; offline supports native disclosure/fragments without script. | No open-state static-position workaround, offscreen trigger/list, duplicate navigation or obscured target/Source pane. |
| **D-36** | Repository explanation describes the system across its important responsibilities, cooperation and representative end-to-end flows, independently of any Work. A short truthful scope and important unresolved failure remain near its heading; routine coverage/inventory/capability detail requires deliberate inspection. | A single task or one subsystem article is useful partial evidence, not repository-wide semantic acceptance; no claim of complete analysis from inventory counts. |
| **D-37** | Publication boundaries and byte budgets serve useful grounded explanations. Evaluate missing causal bridges and repository breadth separately from UI density and resource checks; retain admitted proof. | No citation removal, speculative admission weakening, silent truncation or quality pass based on a safe compact screen. Authoring/admission changes require their own evidenced follow-up. |

#### Reader evidence classification

| Content | Default reader action | Deliberate supporting path |
| --- | --- | --- |
| Goal-grounded Work name, authored article/section heading, nonassertive label | No prominent inline Source action; a heading is navigational structure, not proof of its section | Exact admitted unit in Evidence index; independent name publication in record provenance |
| Lead / synopsis | Quiet provenance in Evidence index by default; substantive mechanisms/results must also be explained with precise actions in the body | Exact original lead claim/range; absent body coverage is a semantic gap, never inferred from matching files |
| Technical mechanism, cause, flow, condition or result | Clear adjacent Source action for each separately admitted range/purpose | Exact Source panel, original bytes, claim and return point |
| Recorded test / Decision | Recorded authority and outcome visible in context; actual command/check and choice/reasons have their own supporting reading layer | Named verification report, original Decision and exact permitted cited ranges; never equate a record with code proof |
| Optional provenance / diagnostics | Closed supporting disclosure | All admitted unit/evidence-key/citation bindings, original records, identities and lifecycle; exact CLI/MCP diagnostics remain available |

Repeated Source actions in a subsection justify themselves by independent claims,
ranges or reading purposes. The same file may support multiple distinct assertions:
never deduplicate across claims or substitute one file-level authority. Exact duplicate
actions within one unit may share one visible action only when their entire citation
binding is identical; every original binding remains inspectable. No prose heuristics
or CSS-only hiding may strand code. Heading roles cannot mechanically prove a heading
nonassertive; a materially assertive heading/lead needs source-first semantic review
and a precise body explanation. This UI delivery does not change v6 admission.

### 3.1 ‘취소선’ 의견의 처리

사용자는 클릭 요소의 선 장식을 없애고 버튼 등의 UI를 요구했다. 직전 코드·HTML 검토에서는 실제 취소선 요소가 아니라 밑줄 링크와 버튼의 혼재가 확인되었다. 따라서 **기존 동작이 취소선을 사용했다는 사실을 새로 만들지 않되, 주요 동작을 명확한 버튼형·제어형 UI로 정리한다는 요구를 적용한다.** 이동은 `a`, 동작은 `button`, 펼침은 적절한 disclosure 의미와 키보드 동작을 유지할 수 있다. 이것은 태그 이름을 전부 버튼으로 바꾸라는 지시가 아니다.

---

## 5. 전체 정보 구조와 공통 UI

### 5.1 단일 내비게이션

**확정 요구:** 브랜드 좌측, 탐색 목록 우측. `개요`, `저장소 설명`, `작업`을 동일한 주 탐색 체계에서 제공한다. 별도의 Project 읽기 메뉴 행을 없앤다.

**2026-10-20 accepted supersession (D-28–33):** Viewer is an explanation reader. Its only general destinations are Overview / Repository explanation / Work. Remove Tools, standalone Decisions, inspection, management and the generic Code catalog, including direct URLs and offline fragments. Work-linked Decisions and exact claim-specific Sources remain in the explanation. Project-wide/unassociated Decisions remain accessible through CLI/MCP/Operations; never fabricate a Work relation.

좁은 화면에서는 같은 탐색 체계를 한 메뉴로 접거나 재배치할 수 있다. 두 번째 영구 메뉴 행을 다시 만들지 않고, 선택 상태·키보드 접근·현재 Project 문맥을 유지한다.

**읽기 전용 표시:** 문서의 상태 영역이나 페이지 메타정보에 조용하게 둔다. static export와 실제 수정 가능한 도구를 구별한다. 배지 이동은 요청 인증이나 동작 권한 변경이 아니다.

### 5.2 공통 표현과 상호작용

Blue-gray 시각 방향, 명확한 제목·도입·본문·보조 정보의 위계, 충분한 여백, 읽기 폭, 문서형 Work 상세를 유지한다. 기존 DOM·CSS와 정확히 일치하는 것이 목표가 아니라 선택한 디자인 원리와 최신 화면 요구를 함께 구현하는 것이 목표다.

주요 행동은 일관된 버튼형 또는 명확한 제어 UI로 제공한다. hover·focus·선택·disabled 상태와 키보드 동작을 함께 설계한다. 불필요한 장식 테두리를 제거하더라도 포커스 표시를 없애지 않는다.

감사 정보의 항목 수에 비례해 초기 화면을 늘리지 않는다. 중요한 실패·scope 불일치처럼 결론을 바꾸는 정보는 본문 가까이에 짧게 두고, 상세 provenance·전체 이력은 명시적인 추가 읽기에서 제공한다. 같은 일반 면책 문구를 카드·문단·Source마다 반복하지 않는다.

검토 페이지·벤치마크·manifest·에이전트 작업 설명은 Product 내비게이션에 섞지 않는다.

---

## 6. 개요 설계

### 6.1 역할과 기본 구성

개요의 역할은 **이 Project가 무엇을 하려는지 이해하고, 관련 작업으로 진입하는 것**이다. 저장소 전체 설명이나 모든 Decision의 요약 화면이 아니다.

기본 정보 순서는 `PROJECT OVERVIEW` → 실제 Project 이름 → Project Purpose → 최근 Work와 전체 작업 진입이다. `최근 작업` 본제목은 유지하되 그 위의 `RECENT WORK`는 제거한다.

`저장소 이해하기`의 별도 제목·생성 요약·중복 면책 카드 영역은 Project Purpose로 대체한다. 저장소 설명은 주 내비게이션에서 직접 접근하므로 개요에서 생성 요약을 다시 반복할 필요가 없다. 정확한 분석 scope와 설명 한계는 해당 저장소 설명에 둔다. Project 전체 읽기를 막는 실제 runtime 장애는 개요에서도 필요한 범위로 표시할 수 있다.

Project 결정 목록은 개요에서 제거한다. 이것은 `ProjectUnderstanding`이 다른 소비자에게 필요한 Decision 데이터를 제공하지 말라는 의미가 아니다.

### 6.2 Project 이름과 목적

| 데이터 | 표시 규칙 | 금지되는 대체 |
|---|---|---|
| Project 이름 | 현재 선택한 canonical Project의 이름을 heading으로 표시 | 설명 Heading, 디렉터리명 추측, `Volicord` 하드코딩 |
| Project Purpose | 실제 Purpose 역할의 기록과 그 근거를 표시 | 최근 Goal, 저장소 기능 요약, 제품 헌장 자동 채택 |
| Purpose 없음 | 짧은 부재 상태를 표시하고 다른 사용은 계속 가능 | 가짜 목적 생성, 기존 요청 원문을 목적이라고 표시 |
| Work 없음 | 작업 부재를 명확히 표시하고 저장소 설명은 정상 접근 | 가짜 Work 생성, 다른 Project의 Work 섞기 |

Workless와 Work-rich 비교는 동일 이름을 의도한 시나리오라면 setup에서 같은 표시 이름을 사용한다. 각각의 실제 Project ID와 상태는 분리한다. 원본 archive를 고치거나 UI가 임의로 Project 이름을 바꾸지 않는다.

---

## 7. Work 카드와 시간 의미

### 7.1 카드의 완료 조건

**정상적으로 콘텐츠가 준비된 Work 카드**는 다음 정보를 제공한다.

| 정보 | 필수 의미 |
|---|---|
| 작업 이름 | 실제 요청을 바탕으로 생성한 간결한 주제·행위 이름. 원래 Goal과 별도인 Derived 해석 |
| 작업 요약 | 해결하려는 문제와 실제 수행한 해결·조사 또는 현재 진행 상태를 짧게 연결 |
| 기록된 상태 | 같은 Work의 실제 lifecycle state. 테스트·수용 여부에서 임의 계산하지 않음 |
| 생성 시각 | Work의 생성 사건을 뜻하는 시각과 확인 가능한 basis |
| 마지막 작업 시각 | 어떤 작업 기록을 기준으로 한 최신 시각인지 식별 가능한 값 |
| 읽기 행동 | 정확한 해당 Work 상세로 가는 주 행동 하나 |

아직 시작되지 않았다면 요약은 문제와 예정된 접근·미착수 상태를 설명한다. 해결되지 않은 조사·실패·중단·no-change 결과에도 적용한다. `문제와 해결`을 요구한다는 이유로 해결 성공을 만들어내지 않는다.

상태 이력·Source 목록·검토·수락·모든 다음 단계·전체 Goal은 카드에 반복하지 않는다. 중요한 제한이 작업의 결론을 바꾸면 간단한 표시를 검토하되 상세에 연결한다.

### 7.2 제목·요약은 명시적인 생산 책임이다

제목과 요약이 없으면 renderer가 첫 문단을 자르거나 Goal 전체를 옮겨 대신하지 않는다. **생성 가능한 근거가 충분한데도 이름·요약이 없는 결과는 데이터 생산·연결의 미완료**다.

목표 경로는 권한 있는 host의 Work 처리에서 제목·요약을 작성하고, 결과가 바뀌면 필요한 부분을 명시적으로 갱신하여 보존·읽기까지 연결하는 것이다. 이것은 GET에서 모델을 호출하거나 canonical Work 기록에 숨은 외부 호출을 붙이라는 뜻이 아니다. 구체적인 trigger·wire 형식은 §11의 미확정 구현 선택으로 둔다.

생성 실패·권한 부재가 원래 Work 생성이나 일반 작업을 취소·차단해서는 안 된다. 안전한 축소 화면은 제공하지만 그 화면을 목표 카드 완성으로 보고하지 않는다.

### 7.3 생성 시각과 마지막 작업 시각

**두 사건을 구분해 제공한다는 요구는 확정이다.** 어떤 기존 필드가 해당 사건을 정확히 나타내는지는 검증이 필요하다.

| 항목 | 설계 권고 및 검증할 basis | 부재·특수 사례 |
|---|---|---|
| 생성 시각 | Work identity를 만든 최초 Goal의 기록 시각. 수정된 최신 revision의 시각과 구분 | 최초 기록 시각을 복구할 수 없으면 확인 불가. import·런타임 생성·문서 생성 시각으로 대체하지 않음 |
| 마지막 작업 시각 | 우선 같은 Work의 최신 Checkpoint 기록 시각을 후보로 검토. UI는 `마지막 작업 기록`처럼 사건을 밝히는 표현 사용 | Checkpoint가 없으면 생성 시각을 ‘마지막 작업’으로 복제하지 않고 해당 기록 없음 표시 |
| 그 밖의 활동 | Work에 명시적으로 연결된 다른 사건을 포함할지는 별도 계약 결정 | Source 조회·Viewer 읽기·요약 재생성·Git commit만으로 자동 갱신하지 않음 |

현재 `observed_at`은 위 두 사건 중 하나를 상황에 따라 담는다. 이름만 바꾸거나 두 곳에 같은 값을 표시하는 방식은 해결책이 아니다. 첫 Goal 시각과 최신 같은-Work Checkpoint 시각이 실제 원본·수정·import 이력에서 무엇을 뜻하는지 확인한 뒤 Projection에 별도 basis와 함께 노출한다.

원본 instant는 보존하고 표시 timezone을 일관되게 제공한다. 정렬·동일 시각 tie-break, 뒤늦게 들어온 과거 기록, 수정·중단·재개 등도 독립 검증한다. 최신 시각이 바뀌었다고 Work 완료나 새 실행이 있었다고 단정하지 않는다.

---

## 8. Work 상세와 Decision

### 8.1 작업을 설명하는 문서

Work 상세는 `QuestionAnswers`의 enum 이름마다 제목을 붙이는 화면이 아니다. 독자가 이해할 이야기를 구성한다. 다음 질문에 해당 Work의 실제 자료가 뒷받침하는 깊이로 답한다.

**왜 필요한 작업인가 → 무엇을 어떻게 구현·조사했는가 → 어떤 동작 원리와 경계가 중요한가 → 무엇이 보고·관측·확인되었는가 → 어떤 판단이 왜 관련되는가 → 무엇이 남았는가.**

이 순서는 기본 읽기 가이드다. 매 Work에 같은 다섯 개 빈 섹션을 강제하지 않는다. 내용에 맞는 절 제목·단락·조건 비교·코드 안내를 사용한다. 하나의 중요한 판단이 접근 방식을 설명한다면 그 위치에서 함께 다룬다. 실패가 결과를 바꾸면 문서 초반에도 짧게 알린다.

현재 Work의 이름·요약은 카드와 의미가 같아야 한다. 본문은 더 구체적인 메커니즘과 관찰을 제공하며, 같은 긴 요약을 코드 패널마다 복제하지 않는다. 원래 Goal·전체 Checkpoint·raw 결과는 정확한 원본으로 보존하되 이를 문서의 주 서사로 삼지 않는다.

`ExpectedEffect`를 ‘결과 / 기대 효과’처럼 관측 결과와 같은 표제로 합치지 않는다. 테스트 코드 존재, 실제 실행, exit code, assertion 결과, 현재 변경에 대한 검증 coverage, 사용자 검토, 사용자 수락을 구별한다.

### 8.2 실제로 관련된 Decision을 맥락에서 설명

관련 Decision은 **이 Work에서 어떤 문제가 있었고 어떤 대안을 선택했으며 왜 그 선택이 적용되는지**를 설명한다. 선택·이유·적용 범위·상태와 근거를 같은 Work 안에서 읽고, 정확한 인용 Source를 확인한 뒤 해당 설명으로 돌아온다.

관계는 명시적인 Work scope 또는 해당 Work에 실제 적용된 Decision 관계를 따른다. 경로 중첩·시간 근접·같은 commit으로 새 관계를 만들지 않는다. Project 범위의 결정이나 미연결 기록은 CLI/MCP/Operations로 검사하며 Viewer 목록이나 임의의 Work 관계로 되돌리지 않는다.

사용자 이유, 에이전트 권고, 모델의 효과 해석은 같은 것이 아니다. 문서·코드에서 효과를 설명할 수 있어도 그것을 사용자나 원 개발자의 의도라고 기록하지 않는다.

### 8.3 설명이 없을 때의 화면

실제 설명이 없으면 부재·stale·deleted·withheld·불러오기 실패를 정확히 구별하고, 허용된 원본과 상태를 읽을 수 있게 한다. 짧은 설명과 명시적인 생성·갱신 경로를 제공할 수 있다. 기존 host/도구에서 어떤 동작이 지원되는지 확인하지 않고 클릭하면 생성된다고 표시하지 않는다.

fallback은 **기록 접근성**의 완료 조건이다. 실제 근거가 충분한 정상 생성 사례가 fallback으로만 끝나면 **설명 콘텐츠·디자인 완료 조건은 실패**다. 빈 내용을 가리기 위해 같은 Work의 원문을 전부 화면 앞에 붙이거나 더 충실한 다른 검토 Work로 바꾸지 않는다.

---

## 9. 저장소 전체 이해와 분석 상태

### 9.1 저장소 설명은 계속 핵심 범위다

Work 카드 개선을 이유로 저장소 전체 이해를 다시 빼지 않는다. Work가 없는 새 Project에서도 저장소의 역할·주요 책임·경계·동작을 설명할 수 있어야 한다. Work가 존재하면 저장소 설명과 해당 Work의 실제 관련 구성요소를 오갈 수 있어야 한다.

저장소 설명의 핵심은 다음이다.

| 독자 질문 | 제공할 설명 | 불충분한 대체 |
|---|---|---|
| 무엇을 하는 시스템인가? | 주요 사용 상황, 입력과 결과, 범위 | README 문장·언어 비율만 표시 |
| 어떻게 책임을 나누는가? | 주요 구성요소의 역할·경계·협력 이유 | 폴더·함수 목록 |
| 중요한 기능은 어떻게 작동하는가? | 시작점, 주요 변환·전달·저장·반환, 중요한 분기 | 선만 연결된 그래프 |
| 왜 이 구조가 필요한가? | 기록된 이유와 현재 구조의 효과·절충을 구분한 설명 | 명명에서 개발 의도를 추측 |
| 변경하면 무엇을 확인해야 하는가? | 확인 가능한 의존 관계와 예상 영향·검증 지점·불확실성 | 모든 이웃 노드를 확정된 영향으로 나열 |

모델의 종합·해석은 필요한 기능이다. 단, 사실·정적 해석·관측된 실행·기록된 판단을 구별한다. “정적 분석만으로 확정할 수 없음”을 이유로 유용한 해석 전체를 포기하지 않고, 해석을 검증된 실행처럼 포장하지도 않는다.

### 9.2 전체 범위와 설명 범위를 구별

저장소 inventory의 포함 범위, 분석 capability의 지원 범위, 실제 조사 범위, 생성 설명의 주제 범위, 화면에 표시한 범위는 다르다. 일부 핵심 경로를 설명했다고 전체 의미를 완전히 이해한 것으로 표시하지 않는다.

예를 들어 Volicord의 활성 `rebuild/`만 조사했다면 해당 범위와 legacy/reference 제외를 알린다. 저장소 이름은 유지하되 실제 설명 범위는 도입 가까이 짧고 이해 가능한 말로 표현한다. 파일 수·오류 수를 먼저 늘어놓아 설명을 가리지 않는다.

### 9.3 분석 상태는 별도의 목적을 가진다

Analysis/diagnostics는 CLI/MCP/Operations에서 분석 범위, capability, 최신성과 복구를 확인하는 기능이다. Viewer에는 해당 Repository/Work 설명의 결론에 영향을 주는 짧은 한계만 둔다. 전체 구조를 설명하는 대신 분석 상태를 나열하던 방식으로 돌아가지 않는다.

부분 분석에서도 허용된 자료로 유용한 설명을 제공할 수 있어야 한다. 다중 언어 계약과 unsupported/degraded 처리는 보존하며, Volicord 자체가 Rust라는 이유로 분석 범위를 Rust만으로 줄이지 않는다.

---

## 10. 데이터 생산 → Projection → 화면 책임

다음은 **논리적 책임 계약**이다. 새로운 crate·데이터베이스·JSON 필드를 모두 이 이름대로 추가하는 지시가 아니다.

| 화면에 필요한 정보 | 원래 자료와 생산 책임 | Projection/읽기 책임 | 없을 때의 처리 | Acceptance |
|---|---|---|---|---|
| Project 이름 | canonical Project | 선택된 Project의 실제 이름·identity 전달 | 프로젝트 선택 오류를 명시 | AC-03 |
| Project Purpose | Purpose 역할의 canonical 기록 | Goal·제품 헌장·생성 요약과 구분 | 짧은 부재 표시 | AC-04 |
| Work 이름 | 실제 요청을 해석하는 권한 있는 host | 원래 Goal과 구별된 Derived 제목·basis 제공 | 생산 상태와 원문 접근. 제목 완성으로 세지 않음 | AC-05 |
| Work 요약 | Goal + 같은 Work의 실제 보고·조사·관측 | 문제·접근·결과/미해결 의미와 각 basis·시점 전달 | 근거가 충분하면 생산 결손. 부족하면 정직한 범위 표시 | AC-05~06 |
| 두 Work 시각 | 정확한 생성·활동 사건을 가진 canonical 이력 | 값·event kind·record/revision·timezone 구분 | unknown/없음. 다른 사건 시각 대체 금지 | AC-07 |
| Work 상태 | 같은 Work의 올바른 상태 선택 | 코드/화면별 다른 계산 금지 | Goal-only 등 기존 정확한 규칙 유지 | AC-08 |
| Work 상세 서사 | 코드·설정·관측·Work 기록을 종합한 host 설명 | 독자 순서·주장·표·읽기 지점을 제공 | 원문 fallback과 설명 완료를 분리 | AC-09 |
| 관련 Decision | 명시된 scope·applied 관계·원래 선택과 이유 | 정확한 Work 문맥과 현재 상태를 전달 | 임의 연관 생성하지 않음 | AC-10 |
| 저장소 설명 | Repository Intelligence + 허용된 Source 조사 + host 종합 | Repository/Analysis basis, 설명 범위와 서사 | Work 없는 상태도 독립 처리 | AC-11 |
| 실행/검증 | 명시적으로 연결된 실제 command·Checkpoint | generated prose와 독립된 사실·coverage 선택 | 기록 없음·not_run·과거·실패 구분 | AC-08 |
| 코드 근거 | 보존된 허용 Source와 실제 claim 결속 | 정당한 범위의 발췌·선택 조회·정확한 반환 상태 | 존재하지만 초기 생략 / 만료·금지 / 미관측 구분 | AC-12 |
| 문서/도식/표 | 의미를 작성하는 producer, 기계적 binding은 도구 | 모든 생성 필드·label의 scope·수명·fingerprint | 검증 밖 보조 문자열 경로를 만들지 않음 | AC-13 |

### 10.1 금지할 책임 전가

Renderer가 의미 있는 제목을 만들어내거나, generic enum에 CSS를 붙여 표와 동작 흐름을 추론하게 하지 않는다. Producer가 필요한 의미를 생산하지 못했는데 Viewer는 존재하는 필드만 안전하게 보여줬다는 이유로 전체 작업을 완료 처리하지 않는다.

반대로 host에게 권한·identity·hash 검증의 정확성을 전부 맡기지 않는다. 가능한 기계적 결속은 Product 도구가 계산·검사하고, 모델은 의미·관련성·서사 구성에 집중한다.

---

## 11. 생성·보존·갱신 설계

### 11.1 생산 경로와 명시적 의도

정상 경로는 **요청·범위 확인 → 필요한 실제 코드·설정·문서·관측 조사 → 의미 종합 → 독자용 제목·요약·서사 작성 → 정확한 claim/Source 결속 → 기록·읽기 → 실제 화면 검토**다.

하나의 조사나 모델 호출로 수행할 수 있으면 불필요하게 나누지 않는다. 숫자 해시·byte 좌표를 host가 수작업 보정하거나 사람이 JSON을 고쳐야만 기록되는 상태를 정상 생산 성공으로 세지 않는다. 최초 시도·실패·수정 시도·사람이 편집한 결과를 분리한다.

**설계 권고:** Work를 설명하는 현재 권한 있는 host 작업에서 이름·요약을 함께 생산하고, 같은 Work 결과가 바뀌면 필요한 내용을 명시적으로 갱신한다. 초기 Goal만 있는 경우는 요청된 문제와 예정된 방향까지만 설명한다. 정확히 어느 작업 API가 이를 요청하고 어떻게 복구할지는 기존 capability를 확인해 결정한다.

이 권고는 새로운 background 호출, 자동 전송, 모든 canonical 쓰기에서의 강제 모델 호출을 승인하지 않는다. 생성 실패가 원래 작업의 수행을 막지 않으면서도, 제품 평가에서는 미완성 콘텐츠가 명확히 드러나야 한다.

### 11.2 제목·요약·상세의 의존성과 수명

현재는 전체 설명의 유효성에 제목·요약 표시가 함께 의존한다. 이것을 반드시 별도 데이터베이스로 나누라고 확정하지 않는다. 다음 기능적 요구를 검증한다.

**상세 코드 설명의 자료가 없다는 이유만으로, 실제 요청·작업 기록만으로 충분히 생성할 수 있는 Work 이름·요약까지 영구히 만들 수 없는 구조여서는 안 된다.** 다만 코드의 현재 동작을 주장하는 요약이라면 관련 Source 의존성을 당연히 유지해야 한다.

후보는 하나의 현재 형식 안에서 독립적인 읽기 단위를 갖는 방식 또는 동일한 managed Derived 체계의 별도 범위 산출물 방식이다. 실제 기록·삭제·수명·조회 비용으로 선택한다. 새 canonical 제목 저장소나 병렬 진실 저장소를 기본안으로 두지 않는다.

부분 독립 읽기를 도입하려면 생성·기록 때부터 각각의 실제 근거와 연결된 산출물 identity를 갖춰야 한다. 이미 stale/deleted된 상세 설명에서 제목만 추출해 새로 유효하다고 표시하거나, 사후에 의존성을 제거해서는 안 된다. 서로 다른 생성본의 제목·요약·본문을 출처 없이 합치지 않는다.

### 11.3 의미와 기계적 binding

Source identity·revision·Repository/Analysis Snapshot·관측 시각·retention·field·range와 claim span을 정확히 연결한다. 같은 문구가 여러 번 나와 span이 모호하면 명시적으로 해결하거나 거부한다. 같은 파일이라는 이유만으로 관련성을 확정하지 않는다.

문단에 여러 주장과 근거가 있으면 각 주장이 필요한 범위를 가리킨다. 전체 문단을 인용하는 것이 항상 오류는 아니지만, 원문 조각 하나로 문단의 모든 인과관계를 뒷받침한다고 표시하지 않는다. 정밀한 claim이 없으면 Viewer가 임의의 token-level 근거를 발명하지 않는다.

### 11.4 서사·표·도식의 표현

Latest user requirement (2026-10-24): flowcharts/UML are required for relational
and sequence understanding. The first real milestones are Volicord concept roles
and Music Deck tab-close synchronization, each with element → concise claim →
exact implementing Source → publication-bound return. Earlier conditional diagram
non-adoption is superseded. Static sequences are not execution traces. Diagram
meaning and accessible text share admitted semantics; caption/conditions/order
are grounded alongside labels. See the Projection owner for current supported shapes.

자유로운 산문을 중심으로 필요한 비교 조건·관측 결과·흐름 단계·구성요소·읽기 지점을 typed 데이터로 제공할 수 있다. 이 의미를 현재 문단 배열이 충분히 제공하면 재사용할 수 있으나, 타입 이름을 늘리는 것만으로 성공이라고 판정하지 않는다.

여러 실제 entity를 묶은 ‘저장 계층’ 등 설명용 개념은 Derived interpretation으로 표현할 수 있다. 해당 개념이나 추론 관계를 분석기가 확인한 새로운 canonical/structural node·edge로 승격하지 않는다. 설명용 도식은 실제 topology와 다른 근거·권위임을 구분하며 owner 계약을 함께 정리한다.

### 11.5 보존·크기·삭제

모든 생성 문자열, 요약, 표 셀, 도식 라벨, 코드 안내는 동일하게 검증·fingerprint·dependency·retention·forgetting 대상이다. 새 필드가 기존 validator 순회를 우회하지 않아야 한다.

큰 문서는 개요와 상세를 연결할 수 있으나 정확한 하위 산출물 identity를 고정한다. 문서 분할이나 선택적 읽기를 한도 우회로 사용하지 않는다. 요청·페이지·export의 전체 비용과 omitted/unknown 상태를 보존한다. 실패한 새 기록은 이전에 유효한 기록을 손상시키지 않는다.

현재 설명 v4 및 관련 managed store는 출발점이다. 새 의미가 durable 형식 변경을 필요로 하면 해당 형식과 모든 consumer를 함께 수정한다. canonical·portable bundle·Analysis 형식은 실제로 변경 필요가 생긴 경우에만 다룬다. 호환성만을 위한 dual decoder·old/new production 경로는 만들지 않는다.

---

## 12. 코드 탐색·권한·성능·설치

### 12.1 중요한 코드를 실제로 읽을 수 있어야 한다

핵심 주장 → 허용된 관련 코드의 실제 바이트 → 원래 주장으로의 복귀가 동작해야 한다. 초기 예산으로 생략된 인용은 생략 이유만 보여주는 데서 끝나지 않고, 현재 허용된 범위라면 구현된 선택 조회를 통해 읽을 수 있어야 한다. 현재 후보의 인용 조회 기반은 보호·재검증 대상이지 다시 없는 기능으로 취급하지 않는다.

같은 Source를 여러 문단이 사용하더라도 claim과 복귀 위치는 구별한다. source 순서의 첫 항목을 무조건 대표 메커니즘으로 선택하지 않는다. 표시한 코드 일부, 전체 cited range, 실제 보존한 영역을 구분한다.

데스크톱에서는 본문이 계속 읽히는 오른쪽 비모달 패널을 사용한다. 모바일은 집중 화면과 정확한 닫기·복귀·포커스 복원을 제공한다. 기존 작은 코드 표시 요청을 존중하되 실제 긴 identifier·들여쓰기·줄바꿈에서 독해를 확인한다. 선택적 확대·줄 접기는 검토할 수 있으나 원문 바이트를 바꾸지 않는다. 변경 표시와 해당 설명의 핵심 줄 강조는 별도 의미다.

### 12.2 읽기는 생성·권한 확대가 아니다

조회·내비게이션·export는 새 모델 호출, Source 관측·보존 연장, canonical 변경 또는 허가 없는 외부 전송을 하지 않는다. 허용된 기존 Source를 선택해 읽는 로컬 요청은 사용할 수 있지만, 임의 파일 경로나 Source ID만으로 다른 범위를 읽지 못하게 한다.

Project·subject·revision·snapshot·realization·claim·citation 범위·수명을 확인한다. direct URL, reload, history, 연속 클릭, fetch 중 대상 변경에서 다른 Work/claim이나 만료된 응답이 패널을 덮지 않도록 검증한다. 실제 데이터 접근·현재성 판단의 권한은 프런트엔드로 이전하지 않는다.

Host/Origin, 변경 요청 인증, untrusted 텍스트의 escape/XSS 방어, 로컬 자산, 삭제·withheld·retention 경계는 유지한다. Candidate/Learning 검사 권한이나 background provider 권한을 화면 개선의 직접 결과로 확대하지 않는다.

### 12.3 오프라인과 기존 기능

스크립트 없는 오프라인 출력은 독립적으로 본문과 포함된 근거를 읽을 수 있어야 한다. 추가 조회가 불가능하면 그 사실과 포함 범위를 정확히 알린다. live와 layout은 달라도 canonical 의미·출력 identity·중요 상태는 같아야 한다. 오프라인 문서에 현재 runtime의 쓰기 권한이 있는 것처럼 표시하지 않는다.

기존 CLI/MCP, Decisions, Analysis, Documents, Memory/Evidence, correction·supersession·forgetting과 보호된 확인 흐름의 기능은 보존한다. 구식 화면 배치를 없애는 것과 기능을 없애는 것을 혼동하지 않는다.

### 12.4 비용과 기술 선택

Vue·React 또는 다른 FE의 채택은 미확정이다. 실제 정상 콘텐츠와 목표 상호작용을 놓고 구현성·UI 품질·성능·배포·설치·유지보수 비용을 비교한다. 같은 UI의 기술별 구현은 비용 비교이고, 다른 정보 위계의 비교는 UX 비교라는 점을 구분한다. 매번 새로운 프레임워크 실험을 다시 만들 필요는 없다.

일반 사용자에게 불필요한 Node/npm 작업·상시 UI 프로세스·CDN·폰트·telemetry 의존성을 추가하지 않는 방향을 유지한다. 개발 빌드, source install, 사전 빌드 배포, 정상 실행을 구분해 검증한다. Rust 구현 언어 계약을 변경할 필요가 있으면 필요한 범위와 사용자 판단을 명시한다. 프레임워크 선택은 아직 승인되지 않았다.

분석·host 생성·Projection·Rust 서버·브라우저·전체 export 비용을 따로 측정한다. 기존 약 21초·28.5초 관측을 현재 SLA나 현재 원인으로 사용하지 않는다. 실제 후보의 반복 측정·stage profile·읽은 바이트·응답 크기·DOM·메모리·설치 크기로 원인을 확인한다. 보안 확인을 생략하거나 필수 내용을 숨기거나 한도를 임의 상향해 성능 개선으로 보고하지 않는다.

---

## 13. Acceptance criteria와 검증

### 13.1 사용자 요구별 완료 기준

| ID | 요구 연결 | 실행 책임 | 검증 가능한 완료 조건 |
|---|---|---|---|
| **AC-01** | D-04~D-07 | Viewer shell·navigation·스타일 | 브랜드 좌측·목록 우측의 단일 탐색에 저장소 설명이 있다. 두 번째 Project 메뉴 행과 우측 읽기 전용 배지가 제거된다. 버튼/링크 의미·focus·narrow 메뉴가 작동한다. |
| **AC-02** | D-08~D-09, D-13 | Overview projection·renderer | `PROJECT OVERVIEW`와 최근 작업 본제목은 유지된다. 중복 저장소 요약 카드, `RECENT WORK`, 개요의 Project 결정 목록은 없다. |
| **AC-03** | D-10 | Project 선택·setup·Overview | 어떤 Project 이름도 원본대로 표시한다. Workless/Work-rich 비교의 명명 의도는 setup에서 맞추고 ID·원본 기록은 바꾸지 않는다. |
| **AC-04** | D-08 | Purpose reader | Purpose가 있으면 정확한 해당 기록을 사용하고, 없으면 부재를 표시한다. Goal·charter·생성 저장소 설명을 대신 넣지 않는다. |
| **AC-05** | D-11, D-17 | Work 콘텐츠 producer→record→Projection→card | 실제 근거가 있는 여러 Work에 구별 가능한 생성 이름과 문제·실제 접근/결과 요약이 정상 경로로 제공된다. 세 카드 모두 중립 라벨·원문이면 정상 콘텐츠 검사는 실패한다. |
| **AC-06** | D-11, D-14 | 상태·생성·갱신 | 미착수·진행·실패·조사 보류·no-change·완료를 정직하게 요약한다. 미확인 해결·성과를 생성하지 않는다. 같은 Work 결과 변경 시 요약의 basis·갱신/비현재 상태가 일치한다. |
| **AC-07** | D-12 | canonical 이력→시간 Projection→UI | 서로 다른 생성·마지막 작업 시각을 해당 사건 근거와 표시한다. 수정·import·동일 시각·Checkpoint 부재에서도 조작하지 않는다. 조회·재생성은 작업 시각을 바꾸지 않는다. |
| **AC-08** | D-11, D-14 | Work selection·facts | 목록과 상세는 동일 Work다. 실행·보고·검증 coverage·검토·수락·다음 행동이 분리된다. 상세 prose 부재가 허용된 독립 사실을 지워서는 안 된다. |
| **AC-09** | D-01~D-03, D-14 | host 조사·문서 모델·Viewer | 실제 Work의 문제·수행 방식·메커니즘·결과를 연결해 이해할 수 있다. 원문 결과나 enum별 제목 나열만으로 통과하지 않는다. |
| **AC-10** | D-13 | Decision linkage·Work detail | 관련 선택·이유·대안·적용 범위를 정확한 Work에서 읽는다. Viewer 결정 목록은 없고 미연결 결정은 기존 CLI/MCP/Operations에서 검사한다. 잘못된 path/시점 연결은 거부한다. |
| **AC-11** | D-01, D-15 | Repository Intelligence·host·읽기 | Work0/Work-rich 모두 저장소의 역할·주요 책임·하나 이상의 중요한 완결된 메커니즘을 설명한다. 미분석·legacy 제외·동적/언어 경계를 숨기지 않는다. map/list만 있으면 실패한다. |
| **AC-12** | D-01, D-02 | Source reader·HTTP·패널 | 중요한 claim의 관련 코드가 실제로 보이고 원문으로 돌아온다. 초기 생략·추가 조회·만료·삭제·오래된 링크·연속 선택·offline 제약을 정확히 처리한다. |
| **AC-13** | D-01, D-16 | 설명 모델·recorder·privacy | 모든 제목·요약·표·도식·안내의 근거·revision·수명·fingerprint를 검증한다. 프로토타입 답안 주입, 무관한 근거, stale에서 임의로 잘라 살린 제목, 혼합 realization을 거부한다. |
| **AC-14** | D-02, D-17 | UI·브라우저·사용자 관찰 | 실제 콘텐츠로 desktop/narrow 타이포·밀도·중요도·코드 가독성을 평가한다. native zoom·화면낭독기는 실제 검사 여부를 구분한다. 기술 체크와 실제 인간 이해·수용을 별도 기록한다. |
| **AC-15** | D-17~D-18 | HTTP·install·전체 소비자 | local-only·권한·비변경 읽기·protected action·CLI/MCP·offline 기능이 유지되며 관리·검사·문서 도구는 Viewer 밖에서 제공한다 (D-30/32). 비용·설치 의존성을 실제 측정하고 부재를 0으로 표시하지 않는다. |
| **AC-16** | D-03, D-17 | 통합·검증 owner | 관련 owner·fixture·schema·consumer·실제 테스트 의미가 새 계약과 일치한다. 현재 후보 기술 gate와 archive, 의미·디자인·사용자 수용을 분리한다. |

### Subsequent acceptance criteria

| ID | Requirement | Responsibility | Observable condition |
| --- | --- | --- | --- |

| **AC-17** | D-19 | Work article | Grounded task-specific background/mechanism/approach/result; no-change, investigation-only, error-free, no-Decision and no-retry cases invent no events. |
| **AC-18** | D-20 | Decision article | Exact historical recommendation reason is readable; missing user reason and unavailable supporting Source remain independent. New interpretation is labeled. |
| **AC-19** | D-21 | Verification | Named command Source/run, selected target snapshot, original times, numeric exit/termination and bounded output remain inspectable; missing inspection/coverage/retest identity stays explicit. |
| **AC-20** | D-22 | Document navigation/warnings | One persistent accessible control set works with fragments, keyboard, desktop pane, narrow focus and script-free export; warning severity reflects actual relevance. |
| **AC-21** | D-23 | Evidence index | Claim-purpose navigation reaches permitted exact evidence and original records without default raw object dumps. |
| **AC-22** | D-24, D-30 | Operations scoped inspection outside Viewer | Server enforces Project/Work/Source/revision, retention, paging and aggregate boundaries; SQL/filter injection, XSS and implicit writes/transmission are rejected; protected tools survive. |
| **AC-23** | D-25 | Operations/public caller | Authorized actual task-time file/command capture reaches an exact permitted reader; two observed states or explicit missing-before, original time, outcomes and failed writes remain truthful. Missing trusted activation/source authority makes the prospective claim Blocked. |
| **AC-24** | D-26 | Campaign admission | Per-task/claim applicability binds candidate, original input/Source/output and producer format/lifetime; historical positives remain alongside precise gaps; required prospective positives cannot be dropped to pass. |
| **AC-25** | D-27 | Evaluation | Source-first and presentation-first order/provenance are recorded separately; technical checks, semantic sufficiency, presentation, human rejection/acceptance and authority have distinct results. |

### Focused Viewer acceptance and evidence routing

| ID | Requirement | Responsibility | Observable condition |
| --- | --- | --- | --- |
| **AC-26** | D-28 | Viewer shell/live/offline/browser | All three destinations work in workless and multiple-Work Projects at desktop/mobile widths; full HTML has no retired destination or second navigation row. |
| **AC-27** | D-29 | Work/CLI/MCP/Operations | Related Decisions retain recorded authority, rationale and scope in the Work; unassociated records are absent from Viewer and remain readable outside it. |
| **AC-28** | D-30 | HTTP/selector/CLI | Tools/Decisions/Code selectors, inspection GETs and obsolete memory/document POSTs fail explicitly without reading retired UI or mutating; wrong Project, Host/Origin and XSS remain denied. |
| **AC-29** | D-32 | Host/Operations/CLI/guarded HTTP | Correction, supersession, forgetting, documents and exact guarded confirmation keep actual callers and permission/lifetime negatives. Only request-scoped non-navigable guarded flow may remain. |
| **AC-30** | D-31 | Repository/Work/citation/browser | Exact permitted cited bytes and origin/return survive; relevant components are explained contextually without a bulk Code catalog. |
| **AC-31** | D-33 | Materialization/offline/publication | No ordinary privacy/health/document/bulk-code projection; script-free selected explanations and bounded Sources remain readable and publication revalidates dependencies. Exact-current small captures are technical controls, with human acceptance unassessed. |

### Evidence hierarchy acceptance and routing

| ID | Requirement | Owner / consumer | Actual check and unfinished condition |
| --- | --- | --- | --- |
| **AC-32** | D-34 | Projection/Privacy → live/offline Reader, `/citation` | Viewer `citation_reading`, evidence-priority controls and `document_navigation_browser.cjs`: quiet heading/lead, exact body and quiet provenance open/return, independent same-file claims, malicious labels, deletion/expiry/foreign refusal and identical bindings. Semantic classification and human usefulness remain pending review. |
| **AC-33** | D-35 | Reader HTML/CSS/JS → live/no-JS/offline/browser | `document_navigation_browser.cjs`: real deep scroll before opening at 1440/1024/768/390/320, trigger/list geometry, internal scroll, 0/1/12 long mixed-language headings, nested details, selection/close/focus/Escape, Source/return, history/reload/resize and native fallback. Native zoom/assistive use stay not_observed unless actually tested. |
| **AC-34** | D-36 | Repository Intelligence/Projection → repository Reader, CLI/MCP | Scope remains concise and visible; exact routine coverage moves under existing inspection; Workless/populated/no-article and important-failure controls. Source-first system-wide semantic review and real host breadth remain pending; presentation alone cannot pass AC-11. |
| **AC-35** | D-37 | Projection/Versioning/Validation → authoring/admission and readers | Compare retained units/bindings/bytes and publication currentness before/after; inspect budget/publication effects without changing admission or invoking providers. Future causal authoring/admission work, independent semantic adequacy and human acceptance remain pending. |

### 13.2 반드시 구분해 평가할 세 부류

**정상 콘텐츠 사례:** 근거와 권한이 충분한 상태에서 실제 host가 제목·요약·설명을 만들고 정상 경로로 기록한다. 의미 있는 내용을 제공하는지가 통과 조건이다.

**부재·실패 사례:** 자료·권한·현재성·생성이 부족할 때 정확한 상태와 허용된 원본을 제공한다. 안전한 fallback은 이 부류만 통과시킨다.

**실제 독자 사례:** 사람이 구조와 작업을 읽고, 올바르게 설명하고, 근거를 찾고, 읽던 문맥으로 복귀한다. 위 두 부류의 자동화 결과는 인간 판단의 대체가 아니다.

정상 사례가 모두 missing인데 부재 화면 테스트만 통과하면 제품의 콘텐츠 Acceptance는 미완료다. 실제 사용자에게 불만을 받은 결과를 “human feedback 없음”으로 보고하지 않는다.

### 13.3 필수 교차 시나리오

Work 없는 Project, 여러 Work가 있는 Project, 같은 제목·같은 파일을 공유하는 서로 다른 Work, 긴 혼합 언어 Goal, 새 요청만 있는 Work, 성공·실패·보류·no-change, 과거 pass 이후 변경, 관련 Decision과 미연결 Decision, Source가 일부만 있거나 deleted/withheld/expired인 경우, 전체 설명은 없지만 제목·요약의 실제 basis는 있는 경우를 포함한다.

생성 이후 원래 Goal 수정, 기록된 결과 변경, 제목에만 의존하는 Source 삭제, 표/도식 label만 수정, 같은 Source를 인용하는 복수 claim, 범위 넘침·잘못된 UTF-8·잘못된 subject·snapshot, 오래된 deep link도 검사한다.

데스크톱과 1024·768·390·320 CSS px 등의 기존 검증 폭을 활용하되 이것을 미승인된 고정 성능·접근성 규격으로 승격하지 않는다. 실제 지원 브라우저·한글 폰트·native zoom·보조기술 여부를 남긴다.

### 13.4 기대값과 평가 순서

Source-first에서는 원본을 먼저 조사하여 책임·메커니즘·중요한 한계의 기대를 정한다. Presentation-first에서는 설명을 먼저 읽고 독자가 이해한 내용을 확인한다. 독립 평가를 수행할 실제 도구·격리 조건이 없으면 그 한계를 표시한다.

프로토타입 문장, 기존 생성 답안, 평가자가 정답으로 고른 코드 위치를 측정용 생성 입력에 주지 않는다. 구조 검사에 필요한 fixture는 쓸 수 있으나 실제 신규 생성과 분리한다. 문자열 일치·필드 수·click 성공만으로 설명 충분성을 평가하지 않는다.

### 13.5 기술 검증과 완료 판정

구현 중에는 기존 `rebuild/scripts/validate focused <label> -- <command>`와 관련 검사로 정상·부정·실제 downstream 동작을 검증한다. 현재 repository가 지원하는 runner·owner mapping·로그/종료 코드 보존을 확인해 사용한다.

통합 완료 시 유일한 최종 검증 책임이 정확한 clean committed HEAD에서 표준 `validate gate`를 수행하고 archive를 독립 검증한다. 직접 `validate final` 호출이나 불필요한 ordered suite 중복 실행은 하지 않는다. 현재 명령 계약이 달라졌으면 실제 지원 interface를 따른다. 과거 후보 성공은 새 후보 성공이 아니다.

결과는 구현·직접 회귀, source/권한 정합성, 의미 정확성·충분성, 시각·상호작용, 실제 인간 수용, 기술 gate/자원, 후속 naturalistic qualification으로 나누어 기록한다. 하나의 총점으로 실패를 상쇄하지 않는다.

---


## Implementation responsibility and acceptance ledger

Reading applicability is provided as typed shared data: explained snapshot,
known generation time, read/export comparison basis and confirmed limitation.
Changed/expired/deleted/withheld/comparison-unavailable/unavailable/missing/corrupt/
unsupported states are distinct from
initial and offline Source-window omissions. Source actions visibly identify
path/range while retaining quiet heading provenance, exact claim identities,
desktop nonmodal inspection and narrow-screen focus. Closing returns to the exact
Source trigger if available; deep links fall back to the originating claim.
Scripted figures initially frame a first-row node at native size; subsequent local
pan and exact navigation returns retain the reader's position. No-script readers
retain native scrolling and the same complete relationship navigation.
These technical contracts do not establish human readability or acceptance.

D/AC wording above defines outcomes, not an API or crate taxonomy. The direct
checks below establish only their stated scope. The ledger records earlier candidate observations, not current passes for superseded Viewer surfaces. AC-26–31 require fresh validation; human acceptance remains separate.
Existing safe-read/v4/citation tests establish regression infrastructure, not sufficient
prose or human acceptance. Exact outputs and first host attempts remain ignored local
evidence; no generated Product wording is checked into these design owners.

| Acceptance | Primary implementation / semantic owner | Direct check and unfinished condition |
| --- | --- | --- |
| AC-01 | Viewer shell / Projection | `navigation`, actual `product_review.cjs` geometry/keyboard/live/offline checks pass; human usability unassessed |
| AC-02 | Viewer Overview / Projection | Overview/removal assertions and actual live/offline browser checks pass |
| AC-03 | canonical Project → Overview | actual names preserved in Work0/populated/unavailable browser checks; broader Project matrix remains unfinished |
| AC-04 | Purpose reader / Projection | canonical Purpose present in navigation fixture, absence in actual Projects; controls pass |
| AC-05 | host → Operations → managed explanation → Viewer | one real imported Work generated by the current host via CLI prepare/compose/record/read; actual same-Work card/detail inspected. Multiple real Work generation and independent semantic quality remain unfinished |
| AC-06 | producer and shared lifecycle / Projection, Privacy | MCP `compose_work_name_summary_and_unicode_citations_without_manual_bindings` passes changed-result invalidation, failed-record preservation and fresh record-only production; broad outcome-language quality remains unassessed |
| AC-07 | canonical history → Work reading / Domain | canonical `original_revision_event_survives_correction_and_later_import` and projection `work_events_use_original_history_and_latest_recorded_instant_without_substitution` pass; live/offline dates checked |
| AC-08 | shared Work selection / Projection | `work_history`, Host consumers, exact-scope and actual same-Work card/detail checks pass; reader understanding unassessed |
| AC-09 | active host and reader document / Projection | one real record-supported Work narrative retained and displayed; human understanding and current-code explanation for this historical Work remain unassessed/unavailable |
| AC-10 | Work Decision relations / Projection | Prior Decision/tool journeys covered a superseded candidate; revalidate contextual Decisions and non-Viewer inspection under AC-27/29; human comprehension unassessed |
| AC-11 | repository explanation / Repository Intelligence, Projection | existing Workless/current v4 public consumers retained; nine focused repository lifecycle regressions pass. Fresh real repository semantic quality remains unfinished |
| AC-12 | scoped Sources, HTTP and evidence panel / Privacy, Projection | six `citation_reading` lifecycle tests and 74 actual browser citation reads pass (en/ko, five widths, offline/deletion); reader comprehension unassessed |
| AC-13 | producer/admission/common traversal / Projection, Privacy | MCP Work lifecycle, citation and budget controls pass Unicode, title-only deletion, wrong scope, formats and atomic failure; automatic bindings do not prove semantic entailment |
| AC-14 | Viewer / validation owner | early exact normal/unavailable/Work0 Product package captured at five widths; native zoom, assistive use and human acceptance unobserved |
| AC-15 | Operations/host/Viewer / Privacy, validation owner | CLI/MCP/offline/security/no-write checks and actual response/export measurements; full installation, analysis/generation-stage cost and memory remain unobserved |
| AC-16 | routed owners / validation owner | routed owner check and explicit supersession pass; full technical gate, broader semantic quality and whole-redesign completion unclaimed |

### Implementation choices for the first Product path

Use current explanation format 6 with independently admitted Work name, summary
and article publications, as specified by Projection. Name is grounded in the Goal;
summary binds the Goal, selected actual report (or explicitly unreported outcome),
latest state, applicable recorded verification/coverage, independent review and
acceptance including resets, material limits and current direction. Projection owns
the narrow field-selection rules; blank reports do not hide current progress.
Technical claims retain their own permitted Source dependencies. Each publication
has its own preparation, exact language/realization, retention and replacement.
Unavailable article support withholds that article without erasing a separately
current card. Never extract text from an invalid realization, remove dependencies
after admission or silently combine scopes under one article identity. Goal correction
or forgetting still invalidates all genuinely dependent scopes. New selected progress
requires fresh summary admission while a Goal-only name remains current.

The explicit producer is the current authorized host using Work explanation
prepare/generate, composition and record. It must author heading and synopsis and
connect problem, actual approach and supported outcome (including incomplete/failure).
Composition derives citation metadata from selected permitted Source ranges; it
creates no prose, persists nothing and dispatches no model. Recording/admission is
not semantic approval. Failed generation/composition/record leaves ordinary Work
operations and prior valid output usable, while normal-content acceptance stays
incomplete. Recovery is explicit fresh prepare, permitted Source inspection and
host authoring; GET and canonical writes never dispatch generation.

Creation means the original Goal revision-1 recording event. Latest work record
means the greatest same-Work Checkpoint `(recorded_at, identity)`; identity only
breaks ties, never changes an instant. Projection carries event kind, value, record
identity and revision separately. Missing original history means unknown; no
Checkpoint means no work-record event. Correction is not creation, import preserves
origin instants and late arrival does not outrank a newer recorded instant. Reading,
export, generation, Git and local import times cannot substitute for either event.

### First-path production and technology disposition

The actual recorded Work was imported through the supported current portable
interface into a fresh ignored Runtime; original bundle, prior evaluations and
redesign inputs were preserved. The first host generation and later fresh
preparation after an instruction-budget repair retain distinct candidate/diff,
plan, source and output identities. No admitted output or durable metadata was
manually repaired. This current-host work is authorized by the active user request
and existing repository access; it creates no background provider grant.

The first implementation keeps the existing Rust renderer, local bundled assets
and small presentation script. It removes a persistent menu row and duplicate
content without adding a frontend framework, npm runtime, UI process, CDN or font
download requirement. Browser tooling remains development-only. Alternative
frameworks were not implemented or benchmarked; no comparative performance claim
is made. Focused Product review records repeated HTTP response time/bytes and
export time/bytes separately. Installation and memory were not measured here and
must not be reported as zero.

### Collection campaign disposition

The existing right-aligned navigation, Purpose, grounded Work name/synopsis,
creation/latest-Work-record times, Work-less repository subject and citation
permissions above are revalidation baselines. They are not newly invented features
or evidence that the following outcomes are complete.

AC-17–22 and AC-25 remain pending end-to-end narrative/presentation/human evaluation.
AC-23 has bounded Operations/file MCP and a tested explicit local CLI caller; genuine
prospective capture remains Blocked without current trusted activation. AC-24 has
per-claim admission diagnostics and an applicability report routed by validation-plan.md;
its required genuine positive remains unsatisfied.
The original rejected sample has one actual host-produced record-only Work (eight
units, zero code citations), two missing Work documents, and missing Decision and
repository documents. It supports historical-record and missing-state tests, not
code-grounded repair narration or task-time capture. New output is unreviewed.

The source-to-document responsibility ledger lives in
[Repository Intelligence](repository-intelligence.md#reader-question-to-source-responsibility-ledger).
Projection owns narrative/verification/evidence separation, Domain owns observation
meaning, Inquiry owns recorded reasons, Privacy owns collection/inspection authority,
Versioning owns actual format changes, and Validation owns campaign admission.
Current, unknown, pending and blocked are independent conditions, never synonyms
for absent data or completed acceptance.

### Flexible article implementation evidence

The shared section index now supplies authored primary-topic links to the single
persistent navigator. Verification/limits sections retain authored subheadings
inside their independent reading layers, rather than appearing as more article
paragraphs. Original unit indices remain the claim/Source return coordinates.
When an article is current, its independent card summary stays accessible in
record inspection instead of becoming a competing lead. When detail is absent,
the separately current summary remains visible with its own publication identity.
The evidence index uses the same section/layer assignment and avoids a second
generic navigation list or repeated per-entry interpretation disclaimers.
These are presentation/structure guarantees, not semantic or human acceptance.

Current v6 composition and reading implement authored section names/order rather
than obligatory question headings. Host `work_explanation` and
`decision_explanation` controls exercise the public generation-request, composition,
record and read interfaces; they are authored validation controls, not actual model
generation. Label-only Source dependence invalidates the whole article. Existing
recorded recommendation rationale survives separately from absent user reason.
AC-17/18 real semantic adequacy and AC-23 genuine prospective capture remain
unestablished without current trusted host activation and source authority; no
historical or authored control replaces those positives. Human comprehension is
unobserved. The independent verification responsibility remains separately tested.

### Local data inspection acceptance detail

The former Tools inspector is retired from Viewer by D-30. Operations retains
bounded Project queries, typed filters, exact revision/field reads and admitted
relations. CLI/MCP preserve explicit memory, privacy and document operations.
Viewer keeps contextual original records, verification and exact cited Sources;
no general record browser or management form survives in live/offline reading.


The explicit scoped verification producer supplies D-21 with named check/purpose,
retained Command Source, original execution and sampled artifact basis. Reader
comparison requires an exact recorded prior run and supported declared comparison
basis; missing/different environment or coverage and unstable/contradictory targets
remain typed non-comparable. Earlier failures remain visible beside later passes.
Historical inputs remain unobserved rather than acquiring inferred identities.
Artifact scope is bounded and operator assertion interpretation remains attributed;
current whole-tree coverage, trusted-host capture and user acceptance are separate.

### Critical claim review correspondence

AC-12/30/32 require a selected important claim's own Source journey. The maintained
Product and article browser drivers bind that task to exact public realization,
unit, citation and claim/range hashes. Required code cannot pass on an unrelated last
link or an absent control; live paging must finish and offline code must be complete
for that declared critical scope. Explicit negative and record-only cases retain
their own applicability. The validation owner's critical-task controls independently
remove/replace the important Source in disposable DOM while preserving other links.
This strengthens technical correspondence and preserves independent publication and
scoped Decision controls; it does not establish semantic sufficiency, visual benefit,
human acceptance or unseen-case generalization. Source context reports the actual
shared excerpt budget using the producer-owned constant.
