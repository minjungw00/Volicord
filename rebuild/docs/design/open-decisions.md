# 재구축 제품 결정 등록부

- 상태: 필수 제품 결정 완료
- 목적: 확정된 제품 선택, 적용 범위와 재검토 조건을 하나의 기준선으로 보존
- 미해결 필수 제품 질문: 없음
- 기술 검증: `validation-plan.md`가 소유
- 원칙: 기술 실험이 어렵다는 이유로 accepted product contract를 조용히 축소하지 않음

파일명은 재구축 첫 커밋과 작업 경로의 연속성을 위해 유지한다. 현재 문서는 미해결 질문 목록이 아니라 accepted decision register다.

## 1. 상태 값

| 상태 | 의미 |
|---|---|
| `accepted` | 사용자가 선택했고 현재 제품 기준선에 반영됨 |
| `delegated` | 제품 경계를 유지하는 구현 선택을 담당자에게 위임 |
| `deferred` | 영향과 재검토 시점을 알고 의도적으로 미룸 |
| `out_of_scope` | 현재 제품 범위 밖으로 제외 |
| `superseded` | 후속 결정이 대체함 |

기술 연구와 prototype의 진행 상태는 이 표의 제품 결정 상태와 섞지 않는다. 검증 결과는 `validation-plan.md`와 각 검증 보고서에 기록한다.

## 2. 상위 제품 선택

| ID | 상태 | 선택 | 현재 의미 |
|---|---|---|---|
| D1 | accepted | 별도 재설계 후 replacement gate 통과 시 기존 구현 제거 | 실제 산출물에 제품 세대 분기나 임시 재구축 명칭을 남기지 않음 |
| D2 | accepted | 한 명의 프로젝트 소유자와 여러 에이전트·세션 | 팀 권한보다 source와 session provenance를 우선 구현 |
| D3 | accepted | 사용자 소유 portable canonical context | cloud 강제 없이 다른 clone·컴퓨터에서 복구 가능해야 함 |
| D4 | accepted | Project, Source, Question, Decision, Context Item, Checkpoint | Task lifecycle 대신 프로젝트 맥락과 판단을 중심으로 함 |
| D5 | accepted | Canonical, Candidate, Derived 세 계층 | 자동 관찰 편의와 장기 기억의 신뢰 경계를 분리 |
| D6 | accepted | material decision을 필요한 만큼 단계적으로 질문 | 제품·설계·구현 관점을 모두 포함하되 확인 가능한 사실은 조사 |
| D7 | accepted | Question에 연결된 현재 host 답변을 사용자 판단으로 인정 | 고위험 effect만 더 강한 confirmation 사용 |
| D8 | accepted | Continuity 기본, 위험 행동에 선택적 Guarded 정책 | 기존 workflow 전체를 선택 profile로 보존하지 않음 |
| D9 | accepted | 작업·검증·사용자 검토·수락 상태를 독립 표현 | final acceptance가 모든 완료의 필수 blocker가 아님 |
| D10 | accepted | 사용자와 에이전트가 같은 canonical basis로 Recall | 역할별 표현은 달라도 source와 freshness는 같아야 함 |
| D11 | accepted | repository-wide 코드 이해와 문서 출력을 일급 기능으로 제공 | Repository Intelligence를 Canonical Context Kernel과 분리 |
| D12 | accepted | 첫 결과물부터 실제 설치·분석·판단·재개에 사용 가능해야 함 | 데모용 API slice가 아니라 교체 가능한 제품 gate를 사용 |

## 3. 결정 기록 형식

후속 제품 질문이 실제로 발생하면 다음 구조를 사용한다.

```text
ID
Status
Question
Why this matters now
Established facts
Options
Recommendation
Trade-offs and uncertainty
User choice or delegated action
User rationale
Scope and consequences
What this unlocks
What remains open
Revisit trigger
Sources
```

답변되지 않은 질문은 Decision으로 저장하지 않는다. 질문 branch는 결정, 위임, 조사, prototype, 보류, 범위 제외 또는 supersession 중 하나로 terminal 상태가 되어야 한다.

## 4. Q1 — Inquiry 시작, 진행과 종료

- 상태: `accepted`
- 결정:
  - 사용자는 언제든 deep inquiry를 명시적으로 시작할 수 있다.
  - 에이전트는 material uncertainty가 있을 때 inquiry를 제안할 수 있다.
  - 되돌리기 어렵거나 장기 영향이 큰 판단이 실제 작업을 막을 때만 자동 시작할 수 있다.
  - 한 라운드에는 현재 dependency frontier의 서로 독립적인 질문만 묶는다.
  - 추천안 일괄 채택, branch 위임, 조사, prototype, 보류와 pause/resume를 지원한다.
  - 질문 수와 라운드 수에는 고정 상한을 두지 않는다.
  - 모든 material branch가 terminal 상태이고 미해결 prerequisite가 없으면 inquiry를 종료한다.
- terminal branch:
  - `decided`
  - `delegated`
  - `resolved_by_research`
  - `requires_prototype`
  - `deferred`
  - `out_of_scope`
  - `superseded`
- 적용 범위: 제품, architecture, data, API, security, privacy, implementation strategy, maintenance와 user experience 판단
- 재검토 조건:
  - 실제 사용에서 질문 관련성이 낮거나 불필요한 중단이 반복됨
  - material branch를 놓쳐 구현 후 중대한 재작업이 반복됨
  - host가 required interaction을 표현할 수 없음
- 검증 참조: `validation-plan.md` V05, `acceptance-scenarios.md` F

## 5. Q2 — 다중 언어 Repository Intelligence 계약

- 상태: `accepted`
- 결정:
  - Volicord 구현 언어는 Rust다.
  - 사용자 자연어에는 allowlist를 두지 않는다.
  - 분석 대상 저장소의 프로그래밍 언어를 하나로 제한하지 않는다.
  - 저장소·언어별 지원을 이진 값이 아니라 capability profile로 표현한다.
- capability:
  - `inventory`: 파일, 언어, manifest, config, 문서와 Git 경계
  - `agent_assisted`: source-grounded 설명, 질의응답과 architecture 해석
  - `structural`: parser가 확인한 entity, range와 구문 관계
  - `semantic`: definition, reference, type와 implementation 관계
  - `ecosystem`: build, package, workspace와 toolchain 문맥
- 공통 계약:
  - 모든 텍스트 기반 저장소는 `inventory`를 제공한다.
  - Codex와 연결된 첫 공식 환경에서 모델이 해석할 수 있는 source에는 `agent_assisted`를 제공한다.
  - 구조적으로 검증되지 않은 설명은 agent interpretation으로 표시한다.
  - capability availability, coverage, unsupported, failed와 degradation reason을 표시한다.
- 첫 구조 분석 최소 언어:
  - Java
  - Python
  - JavaScript
  - TypeScript
  - C
  - C++
  - Rust
- 첫 ecosystem profile:
  - Java: Maven, Gradle
  - Python: 일반 package, `pyproject.toml`
  - JavaScript·TypeScript: Node, `package.json`, `tsconfig`, 일반 monorepo
  - C·C++: CMake 또는 `compile_commands.json`
  - Rust: Cargo package와 workspace
- 공통 보조 형식:
  - Markdown, JSON, YAML, TOML, XML, shell script와 Git metadata
- 첫 replacement gate:
  - 위 언어 전체에서 `structural` capability
  - 위 생태계 중 최소 세 곳에서 `semantic` capability
  - polyglot repository에서 언어별 coverage와 cross-component 설명
- 위임:
  - parser framework, semantic protocol, language server, compiler/indexer와 첫 세 semantic ecosystem 선택은 검증 결과에 따라 구현 담당자가 결정한다.
- 금지:
  - Rust dogfood를 제품 언어 제한으로 바꿈
  - agent explanation을 구조적 사실로 저장
  - analyzer가 없다는 이유로 repository 등록·Decision·Checkpoint·Recall 전체를 거부
  - “전체 코드 이해”를 완전한 정적·동적 의미 보증으로 표현
- 재검토 조건:
  - 동일한 공통 모델이 초기 언어들에서 실용적인 coverage를 제공하지 못함
  - 구조 분석 비용이 기본 사용을 막음
  - 특정 언어 capability의 신뢰도가 source-grounded 설명에 불충분함
- 검증 참조: `validation-plan.md` V01, V02, `acceptance-scenarios.md` B–E

## 6. Q3 — LLM과 개인정보 경계

- 상태: `accepted`
- 결정:
  - 구조 분석과 Canonical Context 처리는 로컬에서 수행한다.
  - semantic provider 없이도 inventory, 가능한 structural analysis, Decision, Checkpoint와 Recall이 동작한다.
  - 현재 host에서 사용자가 요청한 interactive explanation은 host가 이미 가진 source 접근 범위 안에서 수행할 수 있다.
  - background 또는 batch semantic analysis와 외부 provider 전송은 Project 단위 opt-in이며 기본값은 꺼짐이다.
  - provider, model, 전달 source 범위, exclude와 secret 정책을 사용자에게 표시한다.
  - raw source body는 portable bundle에 기본 포함하지 않는다.
  - 사용자는 semantic annotation과 cache를 삭제할 수 있다.
- annotation provenance:
  - provider
  - model
  - source snapshot
  - included source refs
  - generated_at
  - uncertainty
  - stale state
- 재검토 조건:
  - host access와 background transmission을 기술적으로 구분할 수 없음
  - secret 또는 exclude 정책이 실제 repository에서 신뢰할 수 없음
  - local-only mode가 핵심 사용 흐름을 수행하지 못함
- 검증 참조: `validation-plan.md` V07, `acceptance-scenarios.md` Q

## 7. Q4 — 사용자 표면과 학습

- 상태: `accepted`
- 결정:
  - Agent conversation: Inquiry, Decision, Recall, 현재 작업과 코드 설명
  - Local viewer: 프로젝트 개요, Repository Map, Decision trail, Checkpoint timeline, memory 수정·삭제, document preview
  - CLI: init, bind, health, analyze, export/import, repair, reindex, privacy 설정과 고위험 fallback
  - MCP: recall, explain/search, request decision, checkpoint, analyze와 document의 고수준 기능
  - viewer가 없어도 agent conversation과 CLI로 핵심 record를 읽고 수정할 수 있다.
  - 저수준 record CRUD를 수십 개 MCP 도구로 노출하지 않는다.
- 학습:
  - 설명 깊이는 `overview`, `working`, `deep`을 제공한다.
  - 개념 설명을 code entity, Source와 Decision에 연결한다.
  - 사용자 숙련도를 조용히 추론해 영구 profile로 저장하지 않는다.
  - 사용자가 명시적으로 저장한 설명 선호만 Context Item으로 보존한다.
- locale:
  - 첫 bundled UI locale은 한국어와 영어다.
  - 사용자 생성 콘텐츠와 대화에는 인위적인 자연어 allowlist를 두지 않는다.
- 재검토 조건:
  - viewer가 첫 실사용을 지연시키지만 chat/CLI만으로 memory control을 충분히 제공할 수 있음이 입증됨
  - MCP surface가 agent 사용성을 방해하거나 기능 중복을 만듦
- 검증 참조: `acceptance-scenarios.md` A, N

## 8. Q5 — 첫 문서 출력 계약

- 상태: `accepted`
- 첫 필수 문서:
  - Project & Architecture Guide
  - Decision Report
  - Implementation Plan
  - Handoff / Resume Document
- 형식:
  - Markdown: portable 기본 export
  - self-contained HTML: preview와 공유 export
  - PDF와 DOCX: 첫 필수 범위 밖
- 저장:
  - Product Repository에 자동 쓰지 않는다.
  - 사용자가 대상 경로를 명시한 경우에만 저장한다.
  - generated document는 자동 canonical truth가 아니다.
  - 사용자 편집본은 review/import 후 Source 또는 Context로 명시적으로 채택한다.
- 필수 metadata:
  - Project와 source snapshot
  - included Decisions
  - capability와 analysis coverage
  - excluded, unsupported와 failed 영역
  - known gaps와 uncertainty
  - generator identity와 generated_at
- 재검토 조건:
  - 네 문서로 실제 handoff와 설계 판단을 충분히 표현하지 못함
  - Markdown/HTML이 필요한 portability 또는 accessibility를 충족하지 못함
- 검증 참조: `validation-plan.md` V06, `acceptance-scenarios.md` N

## 9. Q6 — Project identity, portable bundle과 충돌

- 상태: `accepted`
- 결정:
  - Project ID는 repository 경로나 remote URL만으로 추론하지 않고 초기화 시 생성한다.
  - Product Repository에 tracked marker를 자동 생성하지 않는다.
  - 다른 clone은 bundle import 후 명시적으로 Project에 bind한다.
  - source가 없어도 canonical context를 읽을 수 있고 code relation은 `unavailable`로 표시한다.
  - 독립적인 새 record는 안전한 경우 자동 병합할 수 있다.
  - 같은 Question, Decision 또는 Context의 의미 변경은 three-way 비교 후 사용자 선택·병합·branch가 필요하다.
  - 삭제와 수정, 상충하는 Decision은 조용히 병합하지 않는다.
- bundle 기본 포함:
  - Project identity
  - Source manifest
  - Question
  - Decision
  - Context Item
  - Checkpoint
  - revision, supersession과 tombstone metadata
- bundle 기본 제외:
  - embedding과 full-text index
  - parser cache와 graph layout
  - raw tool traffic와 전체 chat transcript
  - 전체 source copy
  - temporary Candidate
- 재검토 조건:
  - record-level three-way model이 실제 divergence를 표현하지 못함
  - stable Project ID와 clone binding이 일반 Git workflow를 방해함
- 검증 참조: `validation-plan.md` V03, V04, `acceptance-scenarios.md` J

## 10. Q7 — 수정, supersession과 삭제

- 상태: `accepted`
- 결정:
  - 오탈자, 표현과 형식의 비의미적 보정은 revision이다.
  - 사용자 판단의 의미 또는 선택 변경은 새 Decision의 supersession이다.
  - 객관적 source 충돌은 조용히 덮어쓰지 않고 `contradicted` 또는 `review_due`로 표시한다.
  - 사용자는 canonical record와 semantic annotation을 삭제할 수 있다.
  - 개인정보 삭제는 immutable audit보다 우선한다.
  - 삭제 원문을 별도 immutable log에 남기지 않는다.
  - 필요한 경우 원문이나 복구 가능한 hash가 없는 최소 tombstone만 유지한다.
  - 자동 분석은 사용자 correction을 조용히 덮어쓰지 않는다.
- 재검토 조건:
  - tombstone 없이 referential integrity를 유지할 수 없음
  - export된 bundle 간 삭제 전파가 개인정보 요구를 충족하지 못함
- 검증 참조: `validation-plan.md` V03, V04, `acceptance-scenarios.md` K

## 11. Q8-A — 첫 공식 환경

- 상태: `accepted`
- 결정:
  - OS: Linux
  - agent host: Codex
  - 구현 언어: Rust
  - Repository Intelligence: Q2 capability contract
  - local-only structural mode 필수
  - CLI, MCP와 최소 local viewer 포함
- 지원 표현:
  - 다른 OS와 host에서 우연히 동작하더라도 acceptance를 통과하기 전 공식 지원으로 표시하지 않는다.
- replacement journey:

```text
clean install
→ Project init and clone binding
→ repository inventory and analysis
→ source-grounded explanation
→ staged inquiry and user Decision
→ ordinary work and Checkpoint
→ process restart and new-session Recall
→ bundle export/import to another clone
→ memory correction and deletion
→ document output
→ degraded failure recovery
```

- 재검토 조건:
  - 실제 dogfood 환경이 Linux/Codex와 다름
  - Codex host contract가 필수 Decision 또는 Recall provenance를 제공하지 못함
- 검증 참조: `validation-plan.md` V08, `acceptance-scenarios.md` A, O

## 12. Q8-B — 기존 구현과 데이터

- 상태: `accepted`
- 선택 결과: 기존 구현과 데이터는 새 제품의 범위 밖인 별도 서비스로 취급
- 포함하지 않음:
  - legacy Runtime Home 감지 또는 읽기
  - 자동·수동 migration과 importer
  - historical export
  - legacy backup 안내
  - 기존 ID, API, command와 schema compatibility
  - dual-read, dual-write와 동시 runtime 제공
  - 기존 Task, UserAction, Run, Evidence 또는 continuity record 변환
  - migration, compatibility와 legacy rollback 제품 테스트
- 개발 경계:
  - 기존 Runtime Home과 새 Runtime Home을 물리적으로 분리한다.
  - 새 구현은 legacy path를 테스트 fixture나 입력으로 사용하지 않는다.
  - 교체 후 clean initialization만 지원한다.
- 역사 보존:
  - 기존 source와 문서는 Git history 또는 tag에 남을 수 있다.
  - 이는 제품 지원, migration 또는 rollback path를 뜻하지 않는다.
- 재검토 조건:
  - 없음. 이 범위를 다시 열려면 별도의 새로운 사용자 요구와 제품 판단이 필요하다.
- 검증 참조: `cutover-plan.md`, `acceptance-scenarios.md` P

## 13. Q9 — Recall 자동 실행과 사용자 가시성

- 상태: `accepted`
- 결정:
  - 새 agent session의 첫 project-scoped 요청에서 bounded, read-only Recall을 자동 수행한다.
  - 단순 인사나 프로젝트와 무관한 대화에는 수행하지 않는다.
  - Recall은 Canonical Context를 변경하지 않는다.
  - 사용자는 어떤 Decision, Checkpoint와 Source가 사용됐는지 확인할 수 있다.
  - 매번 전체 brief를 강제로 표시하지 않되 사용 사실, 핵심 항목과 펼쳐볼 경로를 제공한다.
  - user와 agent projection은 깊이가 달라도 record identity, source, freshness, uncertainty, supersession과 omission 상태가 같다.
- 출력:
  - goal과 why
  - active Decisions와 rationale
  - current state와 recent Checkpoint
  - open Questions
  - risks, assumptions와 known limits
  - next meaningful step
  - source, capability, freshness와 omitted count
- 재검토 조건:
  - 자동 Recall이 unrelated context를 반복 주입함
  - bounded selection이 중요한 판단을 지속적으로 누락함
- 검증 참조: `validation-plan.md` V09, `acceptance-scenarios.md` I

## 14. Q10 — Candidate 수집과 Canonical 승격

- 상태: `accepted`
- 결정:
  - 최소 구조화 관찰만 Candidate로 자동 수집한다.
  - Candidate는 로컬에 저장하고 사용자가 수집을 끌 수 있다.
  - Candidate의 존재, 종류와 보존 상태를 사용자가 확인할 수 있다.
  - 정보 종류별 승격 권한을 다르게 적용한다.
- 승격:
  - 명시적 사용자 답변 → Decision
  - 사용자 목표·제약·선호 → user turn Source가 있는 Context Item 가능
  - 파일·Git·명령의 직접 관찰 → source와 agent provenance가 있는 fact 가능
  - 의미 있는 작업 결과 → Q11 조건을 충족한 Checkpoint 가능
  - agent recommendation → 사용자 choice와 분리된 record
  - agent hypothesis → Candidate 또는 Semantic Annotation
  - Question candidate → materiality 검토 후 open Question
- 기본 비수집:
  - 전체 prompt
  - 전체 tool argument
  - source body 전체
  - 모든 stdout·stderr의 무제한 장기 보존
- 재검토 조건:
  - 최소 Candidate만으로 crash/session 종료 후 의미 있는 Checkpoint를 복구할 수 없음
  - Candidate retention이 개인정보 또는 저장 비용을 과도하게 만듦
- 검증 참조: `validation-plan.md` V03, V09, `acceptance-scenarios.md` G, H

## 15. Q11 — Checkpoint 생성 정책

- 상태: `accepted`
- 결정:
  - agent는 의미 있는 작업 완료, 일시 중지 또는 handoff 경계에서 source-grounded Checkpoint를 canonical로 기록할 수 있다.
  - 사용자 review를 모든 Checkpoint의 선행 조건으로 요구하지 않는다.
  - source와 작업 경계를 충분히 확인하지 못하면 Candidate로 남긴다.
- canonical 조건:
  - 의미 있는 코드·문서 변경 또는 명시적 pause/handoff
  - 실제 changed source 또는 path basis
  - 수행한 verification과 결과
  - 적용한 Decision 또는 작업 이유
  - known limits, non-goals와 next step
  - 기존 unrelated dirty change의 분리
- canonical로 만들지 않음:
  - 단순 상태 조회
  - 변경과 새로운 판단이 없는 설명
  - source 없는 추측 요약
  - 확인되지 않은 session-end 자동 요약
- 독립 상태:
  - work state
  - automated verification
  - user review
  - user acceptance
- 재검토 조건:
  - 자동 canonical Checkpoint의 오류율이 높음
  - 사용자에게 의미 있는 작업 경계를 안정적으로 감지하지 못함
- 검증 참조: `validation-plan.md` V09, `acceptance-scenarios.md` H

## 16. Q12 — Guarded effect와 confirmation

- 상태: `accepted`
- 결정: 열거된 고위험 effect에만 action-scoped confirmation을 요구
- 초기 범주:
  - 파괴적 파일·데이터 삭제
  - irreversible 또는 대규모 migration
  - 외부 배포와 공개 게시
  - 결제 또는 지속적인 비용 발생
  - secret·credential 접근이나 변경
  - 개인정보 또는 source code의 외부 전송
  - 외부 시스템에 메시지·이메일·issue를 보내는 행위
  - production 데이터 변경
  - 권한·인증·보안 설정 변경
- confirmation 필드:
  - exact action
  - target
  - expected effect
  - risk
  - scope
  - expiration
  - user response source
- 경로:
  - 가능한 경우 현재 host의 명시적 confirmation
  - host가 지원하지 않으면 local viewer 또는 CLI fallback
- 제외:
  - 일반 코드 수정
  - 로컬 테스트
  - 로컬 repository inventory와 structural analysis
  - 기존 workflow식 ordinary-write admission
- 보증 표현:
  - cooperative confirmation이며 OS sandbox 또는 보안 enforcement가 아님
- 재검토 조건:
  - 실제 effect taxonomy가 중요한 위험을 누락하거나 과도하게 중단시킴
- 검증 참조: `acceptance-scenarios.md` M

## 17. Q13 — 과거 Decision의 재사용과 재질문

- 상태: `accepted`
- 결정:
  - 동일한 Project, 적용 범위와 전제가 유지되고 충돌이 없으면 active Decision을 재사용한다.
  - 사용자 선호는 추천을 조정할 수 있지만 새로운 material Decision을 자동 생성하지 않는다.
- Decision applicability:
  - Project
  - path, component 또는 work context
  - assumptions
  - source basis
  - revisit triggers
  - supersedes와 superseded_by
- 재질문 조건:
  - 사용자가 재검토 요청
  - 적용 범위 변경
  - 전제 또는 source 변경
  - revisit trigger 충족
  - 코드·Context와 Decision 충돌
  - 후속 상충 Decision
  - 실제 결과가 중요한 예상과 다름
- 재질문하지 않음:
  - 동일한 적용 범위와 전제
  - active Decision이며 충돌 없음
  - 이미 선택된 방향의 직접적인 구현 세부사항
- 재검토 조건:
  - applicability가 지나치게 좁아 반복 질문을 막지 못함
  - applicability가 지나치게 넓어 다른 context에 결정을 잘못 적용함
- 검증 참조: `acceptance-scenarios.md` I, L

## 18. 구현 담당자에게 위임된 선택

다음은 accepted product contract를 만족하는 범위에서 구현 담당자가 연구와 prototype 결과로 선택한다.

- parser framework와 언어별 grammar integration
- LSP, SCIP, compiler/indexer 또는 다른 semantic adapter 조합
- 첫 semantic capability 세 ecosystem
- Rust crate와 module 분리
- local database와 portable serialization의 정확한 기술
- ID 형식과 fingerprint 알고리즘
- graph layout과 viewer UI framework
- MCP request field와 wire representation
- full-text, embedding과 ranking 구현
- exact process supervision과 cache strategy

구현 선택은 다음을 바꿀 수 없다.

- 사용자 판단과 agent recommendation의 분리
- 다중 언어 capability contract
- user-owned portable canonical context
- local-only structural mode
- legacy 비호환과 clean initialization
- 일반 작업의 비차단
- source, coverage, freshness와 uncertainty의 가시성

## 19. 기술 검증이 Decision을 다시 여는 조건

기술 검증 실패는 자동으로 제품 계약을 축소하지 않는다. 다음 절차를 따른다.

1. 검증 보고서에 실패한 acceptance와 근거를 기록한다.
2. 다른 구현 접근 또는 범위 내 대안을 평가한다.
3. accepted contract를 현실적으로 만족할 수 없다는 근거가 충분할 때만 새로운 Question을 등록한다.
4. 사용자가 새로운 선택을 하기 전에는 기존 Decision 상태를 `accepted`로 유지한다.
5. 제품 범위를 바꾼 후속 Decision은 기존 Decision을 명시적으로 supersede한다.

## 20. 단계 완료 판정

다음이 모두 참이므로 제품 결정 단계는 완료다.

- Q1–Q13에 `open` 상태가 없다.
- 구현 언어, 사용자 자연어와 분석 대상 저장소 언어가 구분되어 있다.
- 다중 언어 capability와 첫 structural gate가 확정되어 있다.
- Linux와 Codex가 첫 공식 환경으로 확정되어 있다.
- legacy migration, detection, export와 compatibility가 범위에서 제거되어 있다.
- 검증이 필요한 항목은 `validation-plan.md`에 분리되어 있다.
- 남은 선택은 accepted contract를 만족하는 구현 세부사항으로 위임되어 있다.
