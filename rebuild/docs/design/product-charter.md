# Volicord 재구축 제품 헌장

- 상태: 제품 결정 확정
- 적용 범위: `rebuild/`에서 진행하는 새 제품 설계, 검증과 구현
- 구현 언어: Rust
- 첫 공식 실행 환경: Linux와 Codex
- 호환성: 기존 공개 API, Runtime Home, 저장 스키마, workflow, 명령과 데이터는 제품 계약의 대상이 아님
- 변경 원칙: 제품 정체성, 핵심 사용자 약속 또는 신뢰 경계를 바꾸는 경우 사용자의 명시적 판단이 필요함

## 1. 제품 정의

Volicord는 저장소를 이해하고 사용자에게 설명하며, 중요한 제품·설계·구현 판단을 단계적으로 함께 해결하고, 그 이유와 작업 맥락을 여러 에이전트·세션·clone·컴퓨터에서 다시 복구하게 하는 로컬 우선 시스템이다.

Volicord의 중심은 정교한 절차를 수행했다는 기록이 아니라 다음 사용자 경험이다.

1. 사용자가 현재 작업의 목적, 코드와 설계를 이해한다.
2. 중요한 판단이 필요한 이유와 선택 결과를 이해한 뒤 직접 결정한다.
3. 판단, 이유, 관련 코드, 검증과 작업 결과가 함께 기억된다.
4. 새로운 세션과 환경에서도 사용자와 에이전트가 같은 맥락을 복구한다.
5. 사용자는 작업 과정에서 관련 개념과 설계 원리를 학습한다.

## 2. 주 사용자

초기 제품은 다음 사용자를 대상으로 한다.

- 하나의 프로젝트를 책임지는 개인 사용자
- Codex를 포함한 여러 에이전트와 대화를 오가는 사용자
- 여러 작업 세션, 저장소 clone 또는 컴퓨터 사이에서 작업을 이어가는 사용자
- 제품 기획뿐 아니라 시스템 설계와 구현 판단을 이해하고 직접 선택하려는 사용자

초기 범위는 팀 권한, 조직 단위 감사, 중앙 계정 관리와 실시간 공동 편집을 포함하지 않는다. 단일 사용자 환경에서도 사용자, 에이전트 host, agent session, 파일·Git·명령 source와 LLM 생성 설명의 provenance는 구분한다.

## 3. 언어와 실행 환경

세 가지 언어 축을 분리한다.

| 축 | 제품 결정 |
|---|---|
| Volicord 구현 언어 | Rust |
| 사용자 자연어 | allowlist를 두지 않으며 현재 대화 또는 사용자가 요청한 언어를 유지 |
| 분석 대상 저장소 언어 | 단일 언어로 제한하지 않고 capability별 지원 상태를 표시 |

고정 UI와 CLI 문자열의 첫 bundled locale은 한국어와 영어다. 지원 locale에 없는 고정 문자열은 영어로 fallback할 수 있다. 질문, Recall, 코드 설명과 생성 문서는 연결된 모델이 해당 언어를 처리할 수 있는 범위에서 사용자가 선택한 자연어를 실제 본문에 사용한다. Requested-language 메타데이터나 HTML `lang`만 맞고 본문이 영어인 결과는 성공이 아니다. 연결된 모델이 요청 언어를 실현할 수 없으면 해당 generated-content 결과를 `unavailable` 또는 `degraded`로 표시하고 영어 본문을 요청 언어 성공으로 위장하지 않는다. 코드 identifier, 경로, API 이름과 symbol 원문은 번역하지 않는다.

첫 공식 운영 계약은 다음과 같다.

- OS: Linux
- agent host: Codex
- 구조 분석: 외부 semantic provider 없이도 사용 가능
- 사용자 표면: agent conversation, CLI, MCP와 최소 local viewer
- 다른 OS와 host: 실제 acceptance를 통과하기 전 공식 지원으로 표시하지 않음

## 4. 핵심 가치

### 이해

Volicord는 프로젝트 목표, 코드 구조, 주요 데이터 흐름, 현재 상태, 알려진 제약과 불확실성을 source와 함께 설명한다.

### 판단

Volicord는 결과를 실질적으로 바꾸는 제품·설계·구현 질문을 찾아 배경, 선택지, 권장안, trade-off, 불확실성과 후속 영향을 제시한다.

### 존중

사용자의 명시적 답변만 사용자 판단으로 기록한다. 에이전트 추천, 과거 대화에서 추론한 선호, 자동 분석과 사용자의 선택을 혼합하지 않는다. 같은 판단을 다른 인터페이스에서 반복하도록 강제하지 않는다.

### 기억

Volicord는 결과뿐 아니라 목표, 질문, 선택지, 판단 이유, 적용 범위, source, 검증, 알려진 한계와 재검토 조건을 보존한다.

### 재개

새로운 에이전트나 환경은 한 번의 Recall을 통해 현재 목표, 중요한 결정과 이유, 구현 상태, 열린 질문, 위험과 다음 단계를 복구할 수 있다.

### 학습

코드 설명과 Decision trail은 사용자가 선택에 필요한 개념을 이해하고, 자신의 과거 판단과 실제 결과를 돌아볼 수 있게 한다.

## 5. 기본 사용자 흐름

```text
첫 project-scoped 요청의 bounded Recall
→ 저장소와 현재 맥락 이해
→ Engineering Choice Discovery와 authority/learning-value assessment
→ 필요한 경우 단계적 Inquiry 또는 Learning Deliberation
→ 사용자 Decision 또는 위임·조사·prototype·보류
→ 일반 도구로 작업
→ 의미 있는 경계에서 Checkpoint
→ 새 세션·환경에서 Recall 또는 문서 출력
```

일반 파일 수정은 Volicord의 사전 Write Ticket이나 동등한 허가를 요구하지 않는다. 명시적 확인은 파괴적 작업, 외부 배포, 비용 발생, 비밀정보 접근, 개인정보 또는 source code의 외부 전송 등 고위험 효과에 한정한다.

## 6. 핵심 정보 모델

Canonical Context의 최소 개념은 다음과 같다.

| 개념 | 책임 |
|---|---|
| `Project` | 여러 clone과 환경에서 공유되는 프로젝트 정체성 |
| `Source` | 파일, symbol, commit, 명령, URL, 대화 turn, artifact 등 근거 |
| `Question` | 아직 해결되지 않았거나 위임·조사·prototype이 필요한 판단 지점 |
| `Decision` | 질문에 대한 사용자 선택 또는 명시적 위임과 당시의 이유·대안·영향 |
| `Context Item` | 목표, 사실, 가정, 제약, 선호, 위험, 학습, 알려진 한계 |
| `Checkpoint` | 특정 시점의 상태, 변경, 검증, 한계, 열린 질문과 다음 단계 |

Task나 Workstream은 필요할 경우 정보를 묶는 view가 될 수 있지만, 모든 작업의 시작·쓰기·완료를 통제하는 필수 authority state machine이 되지 않는다.

## 7. 기억의 세 계층

### Canonical Context

사용자와 에이전트가 미래 세션에서 복구해야 하는 portable 기록이다. 사용자가 검사, 수정, supersede, 삭제할 수 있어야 한다.

### Session Candidates

작업 중 자동 관찰된 잠정 정보다. 발견한 사실 후보, 질문 후보, semantic claim, checkpoint 후보 등을 포함할 수 있지만 자동으로 사용자 판단이나 장기 사실이 되지 않는다.

### Derived State

삭제 후 다시 만들 수 있는 데이터다. 코드 그래프, full-text index, embedding, fingerprint, ranking, semantic summary cache와 시각화 layout 등이 여기에 속한다.

Derived State의 손실은 Canonical Context의 손실을 일으키지 않아야 한다.

## 8. Candidate 수집과 Canonical 승격

자동 수집은 최소 구조화 정보로 제한한다.

| 정보 | 기본 처리 |
|---|---|
| 명시적 사용자 답변 | 연결된 `Question`의 `Decision`으로 canonical 기록 |
| 사용자가 밝힌 목표·제약·선호 | user turn을 `Source`로 연결해 canonical 기록 가능 |
| 파일·Git·명령에서 직접 관찰한 사실 | source와 agent provenance가 있으면 canonical fact로 기록 가능 |
| 의미 있는 작업 결과 | Checkpoint 조건을 충족하면 agent-authored canonical 기록 가능 |
| 에이전트 추천 | 사용자 선택과 분리하여 보존 |
| 에이전트 가설·semantic 해석 | Candidate 또는 `Semantic Annotation` |
| 질문 후보 | materiality 검토 전에는 Candidate |
| raw prompt, 전체 tool argument와 source body | 기본 장기 저장하지 않음 |
| stdout·stderr | bounded observation으로 시작하며 명시적으로 채택된 경우에만 장기 source로 보존 |

Candidate는 로컬에 저장하고 사용자가 수집을 끌 수 있어야 한다. 접근 빈도나 시간 경과는 검색 순위에 영향을 줄 수 있지만 canonical Decision이나 fact의 효력을 자동으로 제거하지 않는다.

## 9. 단계적 Inquiry와 Decision

질문의 총 횟수에는 고정 상한을 두지 않는다. 대신 질문 대상은 작업 결과를 실질적으로 바꾸는 material decision으로 제한한다.

- 사용자는 언제든 deep inquiry를 명시적으로 시작할 수 있다.
- 에이전트는 material uncertainty를 발견하면 질문 세션을 제안할 수 있다.
- 되돌리기 어렵거나 장기 영향이 큰 판단이 실제 작업을 막을 때만 자동으로 inquiry를 시작할 수 있다.
- 코드와 환경에서 확인할 수 있는 사실은 에이전트가 조사한다.
- 사용자의 가치·선호가 필요한 선택은 사용자에게 질문한다.
- 위임 가능한 기술 선택은 권장안과 위임 선택을 함께 제공한다.
- 대화만으로 판단하기 어려운 UX나 동작은 prototype 또는 experiment로 전환한다.
- 사용자가 모른다고 답한 사항에 추측을 강요하지 않는다.
- 질문 branch는 결정, 위임, 조사, prototype, 보류, 범위 제외 또는 supersession으로 종료한다.
- 모든 material branch가 terminal 상태이고 미해결 prerequisite가 없으면 inquiry를 종료한다.
- 각 라운드 후 열린 질문과 결정 상태를 보존하여 중단 후 재개할 수 있게 한다.

사용자의 답변은 표시된 Question ID와 revision, 현재 host user turn에 연결한다. 일반 대화의 모호한 동의 표현을 과거 질문에 임의로 적용하지 않는다.

일반 작업 전에는 Goal과 repository evidence에서 의미 있는 engineering fork를 먼저
발견한다. 발견은 ownership 판정과 별개이며, broad feature Goal은 subordinate public API,
failure, persistence, privacy/security, compatibility 또는 다른 observable semantics를
자동으로 결정하지 않는다. 각 discovered choice의 exact identity, credible alternatives,
technical consequences, Source basis, effect category와 independent/coupled 관계를 보존한다.
두 approach가 실제 consequence 차이 없이 mechanically equivalent하거나 syntax, local naming,
private helper split에 그치면 discovery-worthy choice로 만들지 않는다.

## 10. Decision 적용과 재검토

과거 Decision은 다음 정보와 함께 보존한다.

- Project와 적용되는 path·component·work context
- 전제와 source basis
- 당시 선택지, agent recommendation과 사용자 rationale
- expected consequence와 known uncertainty
- revisit trigger
- supersession 관계

동일한 적용 범위와 전제가 유지되고 충돌이 없으면 Decision을 재사용한다. 사용자가 재검토를 요청하거나, 적용 범위·전제·source가 바뀌거나, revisit trigger·충돌·예상 밖 결과가 발생한 경우에만 다시 질문한다. 사용자 선호는 추천을 조정할 수 있지만 새로운 material Decision을 자동 생성하지 않는다.

## 11. Repository Intelligence

Volicord는 repository-wide 코드 이해와 설명을 일급 제품 기능으로 직접 제공한다. Canonical Context Kernel과 분리된 first-party subsystem으로 구현한다.

### 11.1 Capability profile

저장소 또는 언어에 단일 `supported` 값을 부여하지 않는다. 분석 snapshot, 언어와 영역별로 다음 capability를 표시한다.

| Capability | 의미 |
|---|---|
| `inventory` | 파일, 언어, manifest, 설정, 문서, Git 상태와 분석 대상 경계 |
| `agent_assisted` | source-grounded 코드 설명, 질의응답과 architecture 해석 |
| `structural` | parser가 확인한 entity, source range와 구문 관계 |
| `semantic` | definition, reference, type, implementation과 해석된 symbol 관계 |
| `ecosystem` | build, package, workspace와 toolchain 문맥을 반영한 분석 |

모든 텍스트 기반 저장소는 `inventory`를 사용할 수 있어야 한다. Codex와 연결된 첫 공식 환경에서는 모델이 해석할 수 있는 source에 대해 `agent_assisted` 설명을 제공하되, 구조적으로 검증되지 않은 해석임을 표시한다.

### 11.2 첫 구조 분석 범위

첫 교체 gate의 최소 구조 분석 대상은 다음과 같다.

- Java
- Python
- JavaScript
- TypeScript
- C
- C++
- Rust

첫 ecosystem profile은 다음을 기준으로 한다.

| 생태계 | 초기 범위 |
|---|---|
| Java | Maven과 Gradle 프로젝트 |
| Python | 일반 package와 `pyproject.toml` 프로젝트 |
| JavaScript·TypeScript | Node, `package.json`, `tsconfig`와 일반 monorepo |
| C·C++ | CMake 또는 `compile_commands.json` 기반 프로젝트 |
| Rust | Cargo package와 workspace |

Markdown, JSON, YAML, TOML, XML, shell script와 Git metadata는 공통 보조 형식으로 처리한다. 그 밖의 언어는 등록을 거부하지 않으며 `inventory`와 가능한 `agent_assisted` 설명을 제공하고, 제공되지 않는 capability를 명시한다.

첫 교체 gate에서는 위 구조 분석 언어 전체에 `structural` capability를 요구한다. 그중 최소 세 생태계에서 `semantic` capability를 검증한다. 어떤 세 생태계를 먼저 선택할지는 기술 prototype 결과에 위임하되 제품 계약을 조용히 축소해서는 안 된다.

### 11.3 사실과 해석의 분리

parser, compiler, indexer 또는 repository metadata에서 확인한 구조적·semantic 사실과 LLM이 만든 설명·가설을 구조적으로 구분한다.

모든 분석 결과와 설명은 다음을 표현해야 한다.

- analysis snapshot과 source revision
- covered, excluded, unsupported와 failed 영역
- capability별 availability와 degradation reason
- freshness와 stale state
- structural support와 semantic uncertainty
- provider·model·생성 시각이 적용되는 annotation provenance

지원하지 않거나 분석하지 못한 언어, macro, generated code, dynamic behavior, 외부 서비스와 runtime-only 상태를 숨기지 않는다.

## 12. LLM과 개인정보 경계

- 구조 분석과 canonical record 처리는 로컬에서 수행한다.
- semantic provider가 없어도 `inventory`, 가능한 `structural`, Decision, Checkpoint와 Recall이 작동해야 한다.
- 현재 host에서 사용자가 요청한 interactive explanation은 host가 이미 부여받은 source 접근 범위 안에서 수행할 수 있다.
- background 또는 batch semantic analysis와 외부 provider 전송은 Project 단위 opt-in이며 기본값은 꺼짐이다.
- provider, model, 전송 범위, exclude와 secret 처리 정책을 사용자가 확인할 수 있어야 한다.
- semantic annotation에는 source snapshot, included source refs, 생성 시각, provider, model과 uncertainty를 기록한다.
- raw source body를 portable bundle에 기본 포함하지 않는다.
- 사용자는 semantic annotation과 관련 cache를 삭제할 수 있다.
- 첫 replacement qualification은 truthful provider failure/degradation과 별개로 current
  production background semantic-provider path의 실제 성공을 최소 한 번 포함한다.
- Technical network/credential availability는 source transmission authorization이 아니며,
  qualification은 Project opt-in에 더해 exact provider/purpose/source scope의 별도
  authorization을 요구한다.

## 13. Recall과 Checkpoint

첫 project-scoped 요청에서 bounded, read-only Recall을 자동 수행한다. 단순 인사나 프로젝트와 무관한 대화에는 수행하지 않는다. Recall은 canonical record를 변경하지 않으며, 사용자가 어떤 Decision·Checkpoint·Source가 사용됐는지 확인할 수 있어야 한다.

기본 Resume Brief는 다음을 포함한다.

- 무엇을 달성하려는가
- 왜 중요한가
- 중요한 결정과 이유
- 현재 구현 상태와 최근 변경
- 열린 질문
- 알려진 가정, 위험과 한계
- 다음 의미 있는 단계
- source, freshness와 analysis coverage

Checkpoint는 의미 있는 작업 완료, 일시 중지 또는 handoff 경계에서 agent가 source-grounded canonical record로 만들 수 있다. 단순 상태 조회, 변경 없는 설명, source 없는 추측 요약은 canonical Checkpoint를 만들지 않는다. 기존 unrelated dirty change를 현재 Checkpoint의 작업 결과로 귀속하지 않는다.

## 14. 사용자 표면과 학습

| 표면 | 초기 책임 |
|---|---|
| Agent conversation | Inquiry, Decision, Recall, 현재 작업과 코드 설명 |
| Local viewer | Project Understanding을 기본으로 한 목표, 완료·현재·남은 작업, 다음 단계, Decision과 이유, 관련 코드, component·architecture·flow, Repository Map과 문서 preview; record·audit 상세는 더 깊은 inspection으로 제공 |
| CLI | repository-relative ordinary use와 task-oriented discovery를 제공하는 init/bind, understand/status, analyze, document, export/import, privacy, repair/reindex와 고위험 fallback |
| MCP | recall, explain/search, request decision, checkpoint, analyze와 document의 고수준 기능 |

viewer가 없는 환경에서도 agent conversation과 CLI를 통해 핵심 기록을 읽고 수정할 수 있어야 한다. record별 저수준 CRUD를 수십 개 MCP 도구로 노출하지 않는다.

Project Understanding은 Canonical Context와 Repository Intelligence를 읽어 만드는 derived/read-side interpretation이며 새 canonical truth가 아니다. 기본 Viewer는 verified structural/semantic fact와 generated interpretation을 출처·statement role·uncertainty로 구분하고, 관계를 시각적으로 이해하는 데 실질적인 도식을 사용한다. 도식의 node와 edge 존재는 inspectable repository relation 또는 Decision/Context relation에서 와야 하며 generated interpretation은 해당 topology를 발명하지 않는다.

CLI는 사용자 작업과 다음 안전한 행동을 중심으로 help·status·error를 제공하고, 일반적인 bound repository 사용에서 current directory를 기준으로 Project를 resolve한다. 안정적인 Project identity는 내부·portable 계약으로 유지하되 일상 명령에서 사용자가 opaque Project ID를 찾아 복사하거나 반복 입력하게 하지 않는다. Human-readable output이 기본이고 structured output은 automation을 위한 explicit mode다.

설명 깊이는 최소 `overview`, `working`, `deep` 수준으로 사용자가 선택할 수 있다. Volicord는 사용자 숙련도를 조용히 추론해 영구 프로필로 저장하지 않으며, 사용자가 명시적으로 저장한 설명 선호만 Context Item으로 보존한다.

사용자가 bounded work/session에 명시적으로 learning participation을 활성화하면 authority와
독립적인 learning-value assessment를 적용한다. Agent-owned choice도 consequence,
transferability, subtlety와 credible alternatives 때문에 학습 가치가 크면 affected work 전에
사용자가 alternatives를 reasoning할 기회를 제공한다. 이 opt-in은 proficiency, behavior,
conversation style 또는 `overview`/`working`/`deep` 설명 깊이에서 추론하지 않으며, 활성화되지
않은 normal mode는 기존의 low-interruption agent autonomy를 유지한다.

초기 participation state는 이 bounded Goal/baseline review의 `inactive` 또는 exact current-host
user-turn Source와 verbatim statement로 입증한 `active`뿐이다. Agent-owned choice의 learning
assessment는 `routine` 또는 significance, transferability와 non-obvious trade-off evidence를 가진
`deliberation-worthy`뿐이며 authority를 바꾸지 않는다. Active learning interaction은 Session
Candidate에 남고 select/delegate/skip은 user-owned Decision을 만들지 않는다. 더 오래 보존할
가치가 있는 lesson은 user가 명시적으로 기록한 기존 `Learning` Context Item과 Source를 사용한다.

## 15. 문서 출력

첫 필수 문서 유형은 다음 네 가지다.

- Project & Architecture Guide
- Decision Report
- Implementation Plan
- Handoff / Resume Document

Markdown을 portable 기본 export로 사용하고 self-contained HTML을 preview 또는 공유 export로 제공한다. PDF와 DOCX는 첫 필수 범위가 아니다. 생성 문서를 Product Repository에 자동으로 쓰지 않으며 사용자가 대상 경로를 명시한 경우에만 저장한다.

생성 문서는 기본적으로 source-grounded projection이며 다음 metadata를 가진다.

- 생성 시각
- Project와 source snapshot
- 포함한 Decision
- capability와 analysis coverage
- 제외·미지원·실패 영역
- 알려진 빈틈과 불확실성
- generator identity

사용자가 편집한 문서는 review/import를 거쳐 명시적으로 채택할 때만 preserved `Source` 또는 Canonical Context의 입력이 된다.

## 16. 이동성, Project identity와 충돌

Canonical Context는 경로와 독립적인 stable Project ID를 사용하고 portable bundle로 export/import할 수 있어야 한다. Product Repository에 tracked marker를 자동 생성하지 않는다. 다른 clone에서는 bundle을 가져온 뒤 명시적으로 Project에 bind한다.

bundle에는 canonical record와 source manifest를 포함하고 embedding, parser cache, raw tool traffic와 전체 source copy는 기본적으로 포함하지 않는다. source repository가 없더라도 Decision과 Checkpoint를 읽을 수 있고 코드 관계는 `unavailable`로 표시한다.

공통 base에서 독립적으로 추가된 record는 안전한 경우 자동 병합할 수 있다. 같은 Question, Decision 또는 Context의 의미 변경, 삭제와 수정, 상충하는 Decision은 조용히 병합하지 않으며 three-way 비교 후 사용자가 선택·병합·branch한다.

## 17. 수정, supersession과 삭제

- 오탈자, 표현과 형식의 비의미적 보정은 revision으로 처리한다.
- 사용자 판단의 의미 또는 선택이 바뀌면 새 Decision이 이전 Decision을 supersede한다.
- source와 충돌하는 사실은 조용히 덮어쓰지 않고 `contradicted` 또는 `review_due`로 표시한다.
- 사용자는 canonical record와 semantic annotation을 삭제할 수 있다.
- 개인정보 삭제는 immutable audit보다 우선한다.
- 삭제 원문을 별도 audit log에 보존하지 않는다.
- 필요한 경우 민감 원문이나 복구 가능한 hash가 없는 최소 tombstone만 유지한다.
- 자동 재분석은 사용자 correction을 조용히 되돌리지 않는다.

## 18. 위험 적응형 정책

- **Continuity:** 기본 정책. Recall, 중요한 Decision, Checkpoint와 source-linked memory를 제공한다.
- **Guarded:** 열거된 고위험 효과에 action-scoped confirmation을 추가한다.
- **Assured:** 감사·규제·엄격한 변경 통제가 실제 사용자 요구로 확인된 뒤 별도 정책으로 검토한다.

Guarded 대상의 초기 범주는 다음과 같다.

- 파괴적 파일·데이터 삭제
- irreversible 또는 대규모 migration
- 외부 배포와 공개 게시
- 결제 또는 지속적인 비용 발생
- 비밀정보·credential 접근이나 변경
- 개인정보 또는 source code의 외부 전송
- 외부 시스템에 메시지·이메일·issue를 보내는 행위
- production 데이터 변경
- 권한·인증·보안 설정 변경

confirmation은 exact action, target, expected effect, risk, scope와 expiration에 연결한다. 일반 코드 수정, 로컬 테스트와 로컬 구조 분석은 Guarded 대상이 아니다. cooperative confirmation을 OS 수준 sandbox나 보안 enforcement로 표현하지 않는다.

## 19. 완료와 검증 의미

다음은 서로 독립적인 사실이다.

- 작업 진행 상태: 진행 중, 일시 중지, 완료, 중단, superseded
- 자동 검증 상태: 미실행, 부분, 성공, 실패
- 사용자 검토 상태: 미요청, 대기, 검토, 수락, 거절
- 알려진 한계와 미검증 영역

사용자 수락이 없다는 이유만으로 구현 결과를 항상 미완료로 유지하지 않는다. 대신 무엇이 구현·검증·검토되었고 무엇이 남았는지 정직하게 표시한다.

## 20. 기존 구현과 데이터

현재 구현과 새 제품은 별개의 서비스로 취급한다. 다음은 제품 범위가 아니다.

- 기존 Runtime Home 감지 또는 읽기
- 자동·수동 migration과 importer
- historical export
- 기존 데이터 backup 안내
- 기존 ID, API, command와 schema 호환
- dual-read, dual-write 또는 두 runtime의 동시 제공
- 기존 workflow를 새 모델로 변환하는 기능

개발 중에는 새 Runtime Home과 기존 Runtime Home을 물리적으로 분리한다. 교체 후 사용자는 새 제품을 clean initialization으로 시작한다. 기존 source와 문서는 Git history에 남을 수 있지만 제품 호환 계약이나 사용 경로가 아니다.

## 21. 비목표

초기 제품은 다음을 목표로 하지 않는다.

- 작업 속도, 토큰 절감 또는 자동 오케스트레이션을 주된 가치로 삼는 것
- 모든 prompt, tool argument, stdout와 source body를 영구 저장하는 것
- 모든 파일 쓰기에 사전 허가를 요구하는 것
- 사용자 판단을 LLM이 추론하거나 자동 해결하는 것
- 모든 언어와 framework에 대한 완전한 정적·동적 분석
- 모든 언어에 같은 capability와 품질을 제공한다고 주장하는 것
- 완전한 correctness oracle 또는 보안 sandbox
- 팀 권한, 조직 감사와 hosted collaboration server
- 기존 공개 API, Runtime Home, database schema와 workflow의 호환
- 기존 구현과 새 구현을 영구적으로 병행 제공하는 것

## 22. 재구축과 교체 원칙

새 설계는 별도 workspace에서 병렬로 구현한다. 실제 산출물에는 제품 세대 명칭을 사용하지 않는다. portable bundle, database, analysis snapshot과 generated document에는 장기 해석을 위한 독립적인 schema 또는 format version을 둔다.

새 구현이 Linux에서 설치, Codex 연결, 다중 언어 저장소 이해, 단계적 판단, 작업 Checkpoint, 새 세션 Recall, 다른 clone 복구, 기억 수정·삭제, 문서 출력과 실패 복구를 실제 저장소에서 통과한 뒤 기존 구현을 제거한다. 구체적인 gate와 검증은 `acceptance-scenarios.md`, `validation-plan.md`와 `cutover-plan.md`가 소유한다.

## 23. 필수 불변 조건

1. 사용자 판단, agent recommendation, observed fact와 generated explanation을 구조적으로 구분한다.
2. Canonical Context Kernel은 repository analyzer, LLM provider, MCP, CLI, viewer와 renderer에 의존하지 않는다.
3. Derived State를 삭제해도 Canonical Context가 손상되지 않는다.
4. 모든 설명은 source, capability, coverage, freshness와 uncertainty를 확인할 수 있다.
5. 분석 실패나 누락을 전체 성공으로 숨기지 않는다.
6. 같은 판단을 다른 interface에서 반복하도록 강제하지 않는다.
7. 질문 수는 제한하지 않지만 중요하지 않은 질문은 하지 않는다.
8. 일반 작업을 절차로 차단하지 않으며 고위험 effect에만 강한 확인을 사용한다.
9. 사용자는 canonical memory를 inspect, correct, supersede와 forget할 수 있다.
10. 기술 prototype은 accepted product contract를 조용히 축소하거나 legacy contract를 복원하지 않는다.
11. Project Understanding은 canonical/analysis basis를 읽는 derived projection이며 record browser나 새 canonical authority가 아니다.
12. Requested-language generated content의 성공은 실제 본문 실현을 요구하며, 불가능하면 명시적 degradation으로 남긴다.
