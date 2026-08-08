# Volicord 재구축 제품 헌장

- 상태: 재구축 기준선
- 적용 범위: `rebuild/`에서 진행하는 새 제품 설계와 구현
- 호환성: 기존 공개 API, Runtime Home, 저장 스키마, workflow와의 호환은 목표가 아님
- 변경 원칙: 제품 정체성이나 핵심 사용자 약속을 바꾸는 경우 사용자의 명시적 판단이 필요함

## 1. 제품 정의

Volicord는 저장소를 이해하고 사용자에게 설명하며, 중요한 제품·설계·구현 판단을 단계적으로 함께 해결하고, 그 이유와 작업 맥락을 여러 에이전트·세션·clone·컴퓨터에서 다시 복구하게 하는 로컬 우선 시스템이다.

Volicord의 중심은 정교한 절차를 수행했다는 기록이 아니라 다음 사용자 경험이다.

1. 사용자가 현재 작업의 목적과 구현을 이해한다.
2. 중요한 판단이 필요한 이유와 선택 결과를 이해한 뒤 직접 결정한다.
3. 판단, 이유, 관련 코드와 작업 결과가 함께 기억된다.
4. 새로운 세션과 환경에서도 사용자와 에이전트가 같은 맥락을 복구한다.
5. 사용자는 작업 과정에서 관련 개념과 설계 원리를 학습한다.

## 2. 주 사용자

초기 제품은 다음 사용자를 대상으로 한다.

- 하나의 프로젝트를 책임지는 개인 사용자
- Codex를 포함한 여러 에이전트와 대화를 오가는 사용자
- 여러 작업 세션, 저장소 clone 또는 컴퓨터 사이에서 작업을 이어가는 사용자
- 제품 기획뿐 아니라 시스템 설계와 구현 판단을 이해하고 직접 선택하려는 사용자

초기 범위는 팀 권한, 조직 단위 감사, 중앙 계정 관리와 실시간 공동 편집을 포함하지 않는다. 단일 사용자 환경에서도 사용자, 에이전트 host, agent session, 파일·Git·명령 source와 LLM 생성 설명의 provenance는 구분한다.

## 3. 핵심 가치

### 이해

Volicord는 프로젝트 목표, 코드 구조, 주요 데이터 흐름, 현재 상태, 알려진 제약과 불확실성을 source와 함께 설명한다.

### 판단

Volicord는 결과를 실질적으로 바꾸는 제품·설계·구현 질문을 찾아, 배경, 선택지, 권장안, trade-off, 불확실성과 후속 영향을 제시한다.

### 존중

사용자의 명시적 답변만 사용자 판단으로 기록한다. 에이전트 추천, 과거 대화에서 추론한 선호, 자동 분석과 사용자의 선택을 혼합하지 않는다. 같은 판단을 다른 인터페이스에서 반복하도록 강제하지 않는다.

### 기억

Volicord는 결과뿐 아니라 목표, 질문, 선택지, 판단 이유, 적용 범위, source, 검증, 알려진 한계와 재검토 조건을 보존한다.

### 재개

새로운 에이전트나 환경은 한 번의 Recall을 통해 현재 목표, 중요한 결정과 이유, 구현 상태, 열린 질문, 위험과 다음 단계를 복구할 수 있다.

### 학습

코드 설명과 Decision trail은 사용자가 선택에 필요한 개념을 이해하고, 자신의 과거 판단과 실제 결과를 돌아볼 수 있게 한다.

## 4. 기본 사용자 흐름

```text
Recall
→ 저장소와 현재 맥락 이해
→ 필요한 경우 단계적 Inquiry
→ 사용자 Decision 또는 위임·조사·prototype
→ 일반 도구로 작업
→ Checkpoint
→ 새 세션·환경에서 Recall 또는 문서 출력
```

일반 파일 수정은 Volicord의 사전 Write Ticket이나 동등한 허가를 요구하지 않는다. 명시적 확인은 파괴적 작업, 외부 배포, 비용 발생, 비밀정보 접근, 개인정보 전송 등 고위험 효과에 한정한다.

## 5. 핵심 정보 모델

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

## 6. 기억의 세 계층

### Canonical Context

사용자와 에이전트가 미래 세션에서 복구해야 하는 portable 기록이다. 사용자가 검사, 수정, supersede, 삭제할 수 있어야 한다.

### Session Candidates

작업 중 자동 관찰된 잠정 정보다. 발견한 사실 후보, 질문 후보, semantic claim, checkpoint 후보 등을 포함할 수 있지만 자동으로 사용자 판단이나 장기 사실이 되지 않는다.

### Derived State

삭제 후 다시 만들 수 있는 데이터다. 코드 그래프, full-text index, embedding, fingerprint, ranking, semantic summary cache와 시각화 layout 등이 여기에 속한다.

Derived State의 손실은 Canonical Context의 손실을 일으키지 않아야 한다.

## 7. 단계적 질문과 판단

질문의 총 횟수에는 고정 상한을 두지 않는다. 대신 질문 대상은 작업 결과를 실질적으로 바꾸는 material decision으로 제한한다.

- 코드와 환경에서 확인할 수 있는 사실은 에이전트가 조사한다.
- 사용자의 가치·선호가 필요한 선택은 사용자에게 질문한다.
- 위임 가능한 기술 선택은 권장안과 위임 선택을 함께 제공한다.
- 대화만으로 판단하기 어려운 UX나 동작은 prototype 또는 experiment로 전환한다.
- 사용자가 모른다고 답한 사항에 추측을 강요하지 않는다.
- 질문 branch는 결정, 위임, 조사, prototype, 보류, 범위 제외 또는 supersession으로 종료한다.
- 각 라운드 후 열린 질문과 결정 상태를 보존하여 중단 후 재개할 수 있게 한다.

## 8. Repository Intelligence

Volicord는 repository-wide 코드 이해와 설명을 일급 제품 기능으로 직접 제공한다. 다만 Canonical Context Kernel과 분리된 first-party subsystem으로 구현한다.

Repository Intelligence는 다음을 제공해야 한다.

- 저장소 snapshot과 분석 coverage
- file, module, type, function, test, config, document 등 code entity
- contains, imports, calls, implements, reads, writes, tests, configures 등의 관계
- source-grounded architecture와 주요 flow 설명
- 증분 분석과 stale 상태
- Decision, Context, Checkpoint와 code entity의 연결

parser 또는 repository metadata에서 확인한 구조적 사실과 LLM이 만든 semantic annotation은 구조적으로 구분한다. 지원하지 않거나 분석하지 못한 언어, macro, generated code, dynamic behavior, 외부 서비스와 runtime-only 상태를 숨기지 않는다.

## 9. Recall과 사용자 표면

사용자와 에이전트는 같은 canonical basis를 사용한다. 표현의 깊이는 다를 수 있지만 record identity, source, freshness, uncertainty와 supersession은 일치해야 한다.

기본 Resume Brief는 다음을 포함한다.

- 무엇을 달성하려는가
- 왜 중요한가
- 중요한 결정과 이유
- 현재 구현 상태와 최근 변경
- 열린 질문
- 알려진 가정, 위험과 한계
- 다음 의미 있는 단계
- source, freshness와 analysis coverage

초기 사용자 표면은 agent conversation, 최소 local viewer, CLI와 MCP adapter로 구성한다. 세부 역할은 `open-decisions.md`에서 확정한다.

## 10. 문서 출력

사용자가 요청할 때 프로젝트 개요, architecture guide, component 설명, Decision report, 설계 문서, implementation plan, change impact report와 agent handoff를 생성할 수 있어야 한다.

생성 문서는 기본적으로 source-grounded projection이며 다음 metadata를 가진다.

- 생성 시각
- project와 source snapshot
- 포함한 Decision
- analysis coverage
- 제외·미지원 영역
- 알려진 빈틈과 불확실성
- generator identity

생성 문서는 사용자가 명시적으로 채택할 때만 preserved Source 또는 canonical context의 입력이 된다.

## 11. 이동성과 소유권

Canonical Context는 경로와 독립적인 stable Project ID를 사용하고 portable bundle로 export/import할 수 있어야 한다. bundle에는 canonical record와 source manifest를 포함하고, embedding, parser cache, raw tool traffic와 전체 source copy는 기본적으로 포함하지 않는다.

동기화 서비스는 초기 필수 기능이 아니다. 다른 컴퓨터나 clone에서는 bundle을 가져온 뒤 source를 재연결하고 derived index를 재구축한다. 충돌 정책은 `open-decisions.md`에서 확정한다.

## 12. 위험 적응형 정책

- **Continuity:** 기본 정책. Recall, 중요한 Decision, Checkpoint와 source-linked memory를 제공한다.
- **Guarded:** 특정 고위험 효과에 명시적 확인을 추가한다.
- **Assured:** 감사·규제·엄격한 변경 통제가 실제 사용자 요구로 확인된 뒤 별도 정책으로 검토한다.

기존 Task phase, Change Unit, 일반 Write Ticket, Evidence-gated close와 final-acceptance ceremony를 Assured라는 이름으로 그대로 보존하지 않는다.

## 13. 완료와 검증 의미

다음은 서로 독립적인 사실이다.

- 작업 진행 상태: 진행 중, 일시 중지, 완료, 중단, superseded
- 자동 검증 상태: 미실행, 부분, 성공, 실패
- 사용자 검토 상태: 미요청, 대기, 검토, 수락, 거절
- 알려진 한계와 미검증 영역

사용자 수락이 없다는 이유만으로 구현 결과를 항상 미완료로 유지하지 않는다. 대신 무엇이 구현·검증·검토되었고 무엇이 남았는지 정직하게 표시한다.

## 14. 비목표

초기 제품은 다음을 목표로 하지 않는다.

- 작업 속도, 토큰 절감 또는 자동 오케스트레이션을 주된 가치로 삼는 것
- 모든 prompt, tool argument, stdout와 source body를 영구 저장하는 것
- 모든 파일 쓰기에 사전 허가를 요구하는 것
- 사용자 판단을 LLM이 추론하거나 자동 해결하는 것
- 모든 언어와 framework에 대한 완전한 정적·동적 분석
- 완전한 correctness oracle 또는 보안 sandbox
- 팀 권한, 조직 감사와 hosted collaboration server
- 기존 공개 API, Runtime Home, database schema와 workflow의 호환
- 기존 구현과 새 구현을 영구적으로 병행 제공하는 것

## 15. 재구축과 교체 원칙

새 설계는 별도 workspace에서 병렬로 구현한다. 실제 산출물에는 제품 세대 명칭을 사용하지 않는다. portable bundle과 database에는 장기 해석을 위한 schema/version metadata를 둔다.

새 구현이 설치, 저장소 이해, 단계적 판단, 작업 Checkpoint, 새 세션 Recall, 다른 clone 복구, 기억 수정·삭제, 문서 출력과 실패 복구를 실제 저장소에서 통과한 뒤 기존 구현을 제거한다. 구체적인 gate는 `cutover-plan.md`가 소유한다.
