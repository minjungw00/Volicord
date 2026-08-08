# 재구축 결정 기록과 미해결 질문

- 상태: 활성
- 목적: 이미 확정된 제품 선택과 앞으로 해결해야 할 질문을 같은 형식으로 보존
- 원칙: 구현 편의로 미해결 질문을 조용히 결정하지 않음

## 1. 상태 값

| 상태 | 의미 |
|---|---|
| `accepted` | 사용자가 선택했고 현재 기준선에 반영됨 |
| `open` | 사용자 또는 실험 결과가 필요한 질문 |
| `delegated` | 사용자가 에이전트 또는 구현 담당자에게 명시적으로 위임 |
| `research` | 저장소·환경·외부 근거 조사로 답해야 함 |
| `prototype` | 실제 동작을 만들어 경험한 뒤 결정해야 함 |
| `deferred` | 영향과 재검토 시점을 알고 의도적으로 미룸 |
| `out_of_scope` | 현재 제품 범위 밖으로 제외 |
| `superseded` | 후속 결정이 대체함 |

## 2. 확정된 제품 선택

| ID | 상태 | 선택 | 현재 의미 |
|---|---|---|---|
| D1 | accepted | 별도 재설계 후 교체 gate 통과 시 기존 구현 제거 | 실제 산출물에 제품 세대 분기나 임시 재구축 명칭을 남기지 않음 |
| D2 | accepted | 한 명의 프로젝트 소유자와 여러 에이전트·세션 | 팀 권한보다 source와 session provenance를 우선 구현 |
| D3 | accepted | 사용자 소유 portable canonical context | cloud 강제 없이 다른 clone·컴퓨터에서 복구 가능해야 함 |
| D4 | accepted | Project, Source, Question, Decision, Context Item, Checkpoint | Task lifecycle 대신 프로젝트 맥락과 판단을 중심으로 함 |
| D5 | accepted | Canonical, Candidate, Derived 세 계층 | 자동 관찰 편의와 장기 기억의 신뢰 경계를 분리 |
| D6 | accepted | material decision을 필요한 만큼 단계적으로 질문 | 제품·설계·구현 관점을 모두 포함하되 확인 가능한 사실은 조사 |
| D7 | accepted | Question에 연결된 현재 host 답변을 사용자 판단으로 인정 | 고위험 효과만 더 강한 확인 경로 사용 |
| D8 | accepted | Continuity 기본, 위험 행동에 선택적 Guarded 정책 | 기존 workflow 전체를 선택 프로필로 보존하지 않음 |
| D9 | accepted | 작업·검증·사용자 검토·수락 상태를 독립 표현 | final acceptance가 모든 완료의 필수 blocker가 아님 |
| D10 | accepted | 사용자와 에이전트가 같은 canonical basis로 Recall | 역할별 표현은 달라도 source와 freshness는 같아야 함 |
| D11 | accepted | repository-wide 코드 이해와 문서 출력을 일급 기능으로 제공 | Repository Intelligence를 Canonical Context Kernel과 분리 |
| D12 | accepted | 첫 결과물부터 실제 설치·분석·판단·재개에 사용 가능해야 함 | 데모용 API slice가 아니라 교체 가능한 제품 gate를 사용 |

## 3. 결정 기록 형식

새 질문은 다음 구조를 사용한다.

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
What this unlocks
What remains open
Revisit trigger
Sources
```

답변되지 않은 질문은 Decision으로 저장하지 않는다. 질문 branch는 결정, 위임, 조사, prototype, 보류, 범위 제외 또는 supersession 중 하나로 terminal 상태가 되어야 한다.

## 4. 다음 라운드의 미해결 질문

아래 순서는 dependency를 반영한다. 앞 질문의 답이 뒤 질문의 선택지와 acceptance 범위를 바꿀 수 있다.

### Q1. 단계적 질문 세션은 언제 시작하고 언제 종료하는가

- 상태: `open`
- 필요한 이유: 질문 횟수에는 제한을 두지 않기로 했지만 모든 작업을 질문 ceremony로 만들면 새 제품도 비효율적이 될 수 있음
- 확정된 사실:
  - 결과를 실질적으로 바꾸는 판단만 질문함
  - repository나 환경에서 확인 가능한 사실은 에이전트가 조사함
  - 중단 후 열린 질문 frontier를 복구해야 함
- 결정할 항목:
  - 사용자가 직접 deep inquiry를 시작하는 경로
  - 에이전트가 질문 세션을 제안하거나 자동 시작할 조건
  - 한 라운드에 독립 질문을 묶는 방식
  - 추천안 일괄 채택, branch 위임, 보류와 pause/resume
  - shared understanding 확인과 종료 기준
- 현재 권고안:
  - 사용자는 언제든 명시적으로 시작 가능
  - 에이전트는 material uncertainty가 있을 때 제안 가능
  - 되돌리기 어렵거나 장기 영향이 큰 판단이 실제 작업을 막을 때만 자동 시작
  - 모든 material branch가 terminal 상태이고 미해결 prerequisite가 없을 때 종료
- 이 결정이 여는 작업: Inquiry 모델, Question 상태 기계, host interaction contract

### Q2. 첫 Repository Intelligence의 공식 지원 범위는 무엇인가

- 상태: `open`
- 선행: Q1과 독립적이나 첫 acceptance 저장소 선택과 연결됨
- 필요한 이유: “전체 코드를 설명한다”는 약속을 coverage가 명확한 실사용 계약으로 바꿔야 함
- 결정할 항목:
  - 첫 공식 지원 언어와 repository 유형
  - framework별 분석 범위
  - call graph와 data-flow의 보증 수준
  - macro, generated code, build script와 dynamic behavior 처리
  - unsupported repository fallback
- 현재 권고안:
  - Volicord dogfood를 위해 Rust, Cargo workspace, Markdown, TOML, YAML와 Git metadata부터 지원
  - workspace, package, crate, module, file, public item, impl, dependency, test 관계와 주요 entry point를 구조적으로 제공
  - 전체 동적 call graph를 보증하지 않고 직접 확인된 관계와 best-effort inference를 분리
- 필요한 선행 연구: parser 기술 비교 spike와 실제 Volicord coverage 측정

### Q3. LLM과 개인정보 경계는 무엇인가

- 상태: `open`
- 선행: Q2의 분석 단위
- 필요한 이유: 코드 설명은 semantic model에 이익을 얻지만 source code 전송과 장기 저장 책임이 생김
- 결정할 항목:
  - source code를 host model 또는 외부 provider에 전달할 수 있는 조건
  - local-only structural mode
  - 사용자가 semantic analysis를 끄는 방법
  - 민감 파일, secret와 ignore 정책
  - semantic annotation의 보존 기간과 provenance
- 현재 권고안:
  - 구조 분석은 로컬 수행
  - semantic analysis는 명시적으로 구성한 provider에만 선택적으로 전달
  - 전달 범위와 제외 파일을 사용자에게 표시
  - semantic analysis 없이도 구조 탐색, Decision과 Recall이 작동
  - provider, model, source snapshot, 생성 시각과 uncertainty를 annotation에 기록
- 이 결정이 여는 작업: provider interface, privacy UI, redaction and exclusion policy

### Q4. 첫 사용자 인터페이스의 책임 분리는 무엇인가

- 상태: `open`
- 선행: Q1, Q2
- 필요한 이유: 대화와 CLI만으로는 축적된 코드 관계와 기억을 사용자가 지속적으로 검사하기 어려움
- 결정할 항목:
  - agent chat, local viewer, CLI와 MCP의 책임
  - 기억 수정·삭제 위치
  - 코드 탐색과 Context Map의 최소 기능
  - viewer가 없는 환경의 fallback
- 현재 권고안:
  - agent chat: 질문, 판단, Recall과 작업 중 설명
  - local viewer: 프로젝트 개요, 코드 map, Decision trail, 기억 수정·삭제, 문서 preview
  - CLI: init, health, analyze, export/import, repair와 고위험 fallback
  - MCP: recall, explain, search, request decision, checkpoint, analyze와 document
- 이 결정이 여는 작업: UI architecture, adapter surface, accessibility acceptance

### Q5. 첫 문서 출력 계약은 무엇인가

- 상태: `open`
- 선행: Q2, Q4
- 결정할 항목:
  - 초기 문서 종류
  - Markdown, HTML 또는 다른 형식
  - 저장 위치와 Product Repository 자동 쓰기 여부
  - 사용자가 편집한 문서를 canonical context로 다시 가져오는 방식
- 현재 권고안:
  - Project overview, Architecture guide, Decision report, Design document, Implementation plan, Change impact와 Agent handoff
  - Markdown을 portable export, self-contained HTML을 preview/export로 제공
  - 자동으로 저장소에 쓰지 않고 사용자가 대상 경로를 명시
  - 사용자 수정본은 별도 import/review를 거쳐 Source 또는 Context로 채택
- 이 결정이 여는 작업: document projection schema, renderer와 adoption workflow

### Q6. portable bundle 충돌은 어떻게 처리하는가

- 상태: `open`
- 선행: Canonical 모델 spike
- 필요한 이유: 다른 컴퓨터에서 같은 Project가 독립적으로 바뀔 수 있음
- 결정할 항목:
  - bundle revision과 공통 base 표현
  - record별 자동 merge 범위
  - 상충하는 Decision 수정 처리
  - source repository가 없는 환경의 동작
- 현재 권고안:
  - 조용한 전체 자동 merge 금지
  - 독립적인 record 추가는 자동 병합 가능
  - 같은 Question, Decision 또는 Context의 의미 충돌은 three-way 비교 후 사용자가 merge·선택·branch
  - source가 없으면 canonical context를 읽되 code relation은 unavailable로 표시
- 필요한 prototype: 두 clone에서 divergent bundle을 만든 뒤 merge 복구

### Q7. 기억 수정, supersession과 삭제의 정확한 의미는 무엇인가

- 상태: `open`
- 선행: Canonical 모델 spike
- 결정할 항목:
  - 오탈자·표현 보정과 의미 변경의 구분
  - 사용자 직접 수정과 새 Decision의 관계
  - 삭제 후 tombstone 범위
  - 개인정보 삭제와 audit 이력 충돌
  - 자동 분석과 사용자 correction 충돌
- 현재 권고안:
  - 비의미적 보정은 revision
  - 판단 의미 변경은 supersession
  - 사용자는 삭제 가능하고 개인정보 삭제가 immutable audit보다 우선
  - 원문을 보존하지 않는 최소 tombstone만 선택적으로 유지
  - 객관적 source 충돌은 자동 덮어쓰기 대신 `contradicted` 또는 `review_due`
- 이 결정이 여는 작업: revision model, forget semantics, UI operations

### Q8. 교체 gate와 기존 데이터 처리는 어떻게 확정하는가

- 상태: `open`
- 선행: acceptance spike 결과
- 필요한 이유: 기존 구현은 실제 사용자 제품으로 유지할 계획이 없지만 삭제 전에 replacement가 실사용을 증명해야 함
- 결정할 항목:
  - acceptance에 사용할 실제 repository 수와 종류
  - installer와 Codex integration의 필수 수준
  - 기존 Runtime Home 자동 변환 여부
  - historical export 필요 여부
  - rollback 보존 기간
- 현재 권고안:
  - Volicord 자체, 소규모 단일 언어 앱, 문서·다중 언어가 섞인 중간 규모 저장소에서 전체 여정 통과
  - 기존 Runtime Home 자동 변환은 기본 범위에서 제외
  - 감지, backup 안내와 선택적 historical export만 제공
  - 교체 commit 전 archive tag를 보존
- 이 결정이 여는 작업: final cutover checklist와 removal batch

## 5. 질문 라운드 진행 규칙

- 한 라운드에는 서로 독립적이고 사용자가 함께 비교할 수 있는 질문만 묶는다.
- 답변으로 새 prerequisite가 생기면 다음 라운드에서 제시한다.
- 각 질문에는 권장안이 있어야 하며, 권장안이 없는 경우 조사 또는 prototype이 필요한 이유를 설명한다.
- “추천안 모두 채택”, “이 branch는 에이전트에게 위임”, “보류”, “prototype 후 다시 질문”을 명시적 답변으로 지원한다.
- 사용자의 답변은 Question ID와 revision에 연결한다.
- 이전에 답한 Question을 표현만 바꾸어 반복하지 않는다.
