# 재구축 실사용 Acceptance 시나리오

- 상태: 초기 기준선
- 목적: 기존 절차 수행 여부가 아니라 사용자의 이해·판단·기억·재개와 실제 사용 가능성을 검증
- 적용: 재구축 구현, 설치, adapter, Repository Intelligence, UI와 portable context

## 1. 기존 기준과의 관계

기존 SignalBox 시나리오는 Task intake, CLI 전용 UserAction resolution, Change Unit, Write Ticket, Run, Evidence, final acceptance와 close ceremony를 성공 조건으로 삼았다. 이 흐름은 새 제품의 active acceptance contract가 아니다.

다만 다음 실패 방지 원칙은 보존한다.

- 모호한 대화를 사용자 판단으로 위조하지 않는다.
- 기존 dirty change를 현재 작업 결과로 잘못 귀속하지 않는다.
- 실행하지 않은 검증이나 관찰하지 않은 결과를 성공으로 주장하지 않는다.
- cooperative 기록을 OS 수준 보안 강제로 표현하지 않는다.
- 부분 실패, source 부재와 분석 누락을 숨기지 않는다.

## 2. 공통 통과 원칙

모든 시나리오는 다음을 만족해야 한다.

1. 사용자는 같은 판단을 다른 인터페이스에서 반복하지 않는다.
2. 에이전트 추천, 사용자 판단, 구조적 사실과 semantic 설명이 구분된다.
3. 일반 파일 수정에 Volicord 사전 허가가 필요하지 않다.
4. Canonical Context는 사용자가 볼 수 있고 수정·supersede·삭제할 수 있다.
5. Derived State를 삭제하거나 재구축해도 Canonical Context가 유지된다.
6. 코드 설명은 source snapshot, coverage, freshness와 known gaps를 표시한다.
7. 새 세션은 결과뿐 아니라 결정 이유와 열린 질문을 복구한다.
8. 작업 완료, 자동 검증, 사용자 검토와 수락을 독립적으로 표현한다.
9. Context 또는 분석 기능 실패가 실제 repository 작업 결과를 왜곡하지 않는다.
10. 고위험 효과에만 더 강한 confirmation을 요구한다.

## 3. 시나리오 형식

각 시나리오는 다음 항목을 가진다.

- 시작 상태
- 사용자 요청
- 기대 사용자 경험
- Canonical Context 변화
- Derived State 변화
- 금지 행동
- 자동 검증
- 수동 평가

## A. 새 환경 설치와 프로젝트 연결

### 시작 상태

- Volicord가 설치되지 않은 지원 환경
- Git 저장소 하나
- 기존 Runtime Home 없음

### 사용자 요청

Volicord를 설치하고 현재 저장소를 프로젝트로 연결한다.

### 기대 사용자 경험

- 설치 명령과 결과가 명확하다.
- Codex 연결 또는 지원 host 연결 상태를 확인할 수 있다.
- stable Project ID와 현재 clone binding이 만들어진다.
- canonical, candidate와 derived data의 저장 위치를 확인할 수 있다.
- semantic analysis와 외부 provider 사용 여부가 명시된다.
- health 결과는 성공, degraded와 실패를 구분한다.

### Canonical Context 변화

- Project identity
- clone/repository source binding
- 사용자 설정 중 portable해야 하는 항목

### Derived State 변화

- 초기 repository discovery
- local indexes 또는 빈 analysis state

### 금지 행동

- 기존 Runtime Home을 조용히 덮어씀
- 사용자 동의 없이 source를 외부 provider에 전송
- 연결 실패를 성공으로 보고

### 자동 검증

- clean install smoke test
- restart 후 Project ID 유지
- health output schema
- 분리된 runtime path 확인

### 수동 평가

- 사용자가 무엇이 저장되고 전송되는지 이해할 수 있는가

## B. Repository-wide 이해와 source 탐색

### 시작 상태

- 연결된 지원 저장소
- 아직 분석 snapshot 없음

### 사용자 요청

저장소의 목적, architecture, 주요 component와 데이터 흐름을 설명한다.

### 기대 사용자 경험

- repository-wide 분석 진행과 coverage를 볼 수 있다.
- 주요 package, module, entry point, test와 config 관계를 탐색한다.
- 설명의 핵심 주장마다 file 또는 symbol source로 이동할 수 있다.
- 구조적 사실과 semantic explanation을 구분한다.
- excluded, unsupported와 failed 영역을 표시한다.

### Canonical Context 변화

- 사용자가 명시적으로 저장하지 않는 한 분석 결과 자체는 canonical이 아님
- 분석에서 발견한 중요한 질문이나 fact는 candidate로 제안 가능

### Derived State 변화

- Analysis Snapshot
- Code Entity와 Structural Relation
- Coverage와 fingerprint
- 선택적 Semantic Annotation

### 금지 행동

- 분석하지 못한 영역을 포함해 “전체를 완전히 이해했다”고 주장
- LLM annotation을 parser fact로 저장
- 분석만으로 사용자 Decision 생성

### 자동 검증

- fixture repository의 known entity·relation 추출
- coverage count와 excluded path
- 동일 snapshot의 안정적 결과
- 변경 파일만 증분 재분석
- semantic provider 없이 structural mode 동작

### 수동 평가

- 사용자가 주요 구조와 흐름을 실제 source를 통해 이해할 수 있는가

## C. 단계적 Inquiry와 사용자 Decision

### 시작 상태

- 프로젝트 Context와 Repository Intelligence 사용 가능
- 여러 material decision이 dependency를 가짐

### 사용자 요청

설계 또는 구현을 시작하기 전에 필요한 판단을 충분히 정리한다.

### 기대 사용자 경험

- 에이전트가 먼저 repository와 환경에서 사실을 조사한다.
- 현재 frontier의 질문만 배경, 선택지, 권장안과 trade-off와 함께 제시한다.
- 사용자는 선택, 수정안, 위임, 조사, prototype 또는 보류로 답할 수 있다.
- 답변에 따라 다음 질문이 열리거나 닫힌다.
- 세션을 중단하고 새 세션에서 이어갈 수 있다.
- 같은 질문을 CLI에서 다시 입력하지 않는다.

### Canonical Context 변화

- 열린 Question과 dependency
- 명시적 사용자 Decision 또는 delegated/deferred 상태
- 사용자 rationale가 있으면 별도 필드로 저장
- 당시의 agent recommendation과 source

### Derived State 변화

- Question candidate와 materiality ranking
- 설명용 option 비교와 semantic summary

### 금지 행동

- 코드에서 확인할 수 있는 사실을 사용자에게 질문
- 모호한 과거의 “좋아요”를 Decision으로 적용
- agent recommendation을 사용자 choice로 저장
- 답을 모른다는 사용자에게 추측을 강요

### 자동 검증

- Question frontier 계산
- dependency별 후속 질문
- user turn과 Question revision 연결
- restart 후 open frontier 복구
- answered Question 반복 방지

### 수동 평가

- 질문이 제품·설계·구현 결과에 실제로 중요한가
- 사용자가 선택의 의미와 영향을 설명할 수 있는가

## D. 일반 작업과 Checkpoint

### 시작 상태

- 목표와 필요한 Decision이 충분히 확정됨
- repository working tree에 기존 unrelated dirty change가 있을 수 있음

### 사용자 요청

확정된 범위의 실제 코드 또는 문서 작업과 검증을 수행한다.

### 기대 사용자 경험

- 에이전트는 일반 도구로 작업하며 ordinary write permission을 요청하지 않는다.
- 변경된 path와 적용 Decision을 구분한다.
- existing dirty change를 현재 작업 결과로 잘못 귀속하지 않는다.
- 의미 있는 작업 단위가 끝나면 하나의 Checkpoint를 만든다.
- Checkpoint는 변경, 이유, 검증, 한계, non-goals와 다음 단계를 설명한다.

### Canonical Context 변화

- Checkpoint
- 적용한 Decision refs
- changed Source refs 또는 observed path
- verification result source
- 새 Context Item, risk, known limit와 open Question

### Derived State 변화

- 변경 source에 대한 증분 분석
- impact candidate

### 금지 행동

- 일반 파일 쓰기를 Work Ticket이나 동등한 ceremony로 차단
- 실행하지 않은 테스트를 성공으로 기록
- 기존 dirty file을 현재 Checkpoint 변경으로 자동 포함
- user review가 없다는 이유만으로 실제 완료 상태를 왜곡

### 자동 검증

- checkpoint serialization과 restart
- changed path basis
- verification state 독립성
- known limits와 non-goals 보존

### 수동 평가

- Checkpoint만 읽고 무엇을 왜 했는지 이해할 수 있는가

## E. 완전히 새로운 세션의 Recall

### 시작 상태

- 하나 이상의 Decision과 Checkpoint 존재
- 이전 대화 context 없음

### 사용자 요청

현재 프로젝트 작업을 이어간다.

### 기대 사용자 경험

한 번의 Recall로 다음을 제공한다.

- 현재 목표와 중요성
- 중요한 Decision과 rationale
- 현재 구현 상태와 최근 변경
- 관련 code entity와 source
- 열린 Question, 가정, 위험과 한계
- 다음 의미 있는 단계
- freshness와 analysis coverage

사용자용과 에이전트용 표현은 깊이가 달라도 같은 canonical record를 사용한다.

### Canonical Context 변화

- read-only가 기본
- 사용자가 correction을 요청한 경우에만 수정 흐름 시작

### Derived State 변화

- relevance ranking과 Resume Brief projection

### 금지 행동

- 해결된 질문 반복
- superseded Decision을 현재 선택으로 제시
- stale source에 근거한 설명을 현재 사실로 표시
- user-visible brief와 agent-only hidden memory가 서로 다른 사실 사용

### 자동 검증

- deterministic core selection
- source unavailable와 stale 표시
- truncation과 omitted count
- record identity 일치

### 수동 평가

- 새 에이전트가 추가 설명 없이 올바른 다음 작업을 제안할 수 있는가

## F. 다른 clone 또는 컴퓨터에서 portable 복구

### 시작 상태

- source 환경 A에 canonical context와 derived indexes 존재
- 동일 repository의 다른 경로 또는 환경 B 존재

### 사용자 요청

Context bundle을 내보내 환경 B에서 프로젝트를 이어간다.

### 기대 사용자 경험

- canonical bundle export/import
- stable Project ID 유지
- 새 clone source 재연결
- fingerprint와 freshness 재검증
- derived indexes 로컬 재구축
- source가 없을 때도 Decision과 Checkpoint를 읽을 수 있음

### Canonical Context 변화

- import revision과 source binding
- 충돌이 없으면 record identity 유지

### Derived State 변화

- 환경 B의 새 analysis snapshot과 index

### 금지 행동

- path 차이로 새 Project를 조용히 생성
- embeddings나 parser cache를 canonical truth로 취급
- divergent Decision을 조용히 덮어씀

### 자동 검증

- deterministic bundle
- import/export round trip
- clone path-independent source refs
- index deletion and rebuild

### 수동 평가

- 사용자가 두 환경이 같은 프로젝트 맥락을 공유한다고 확인할 수 있는가

## G. 기억 수정, supersession과 삭제

### 시작 상태

- 잘못된 Context Item, 바뀐 Decision 또는 민감한 record 존재

### 사용자 요청

기억을 수정하거나 대체하거나 삭제한다.

### 기대 사용자 경험

- 표현 보정, 의미 변경, supersession과 forget의 차이를 설명한다.
- 어떤 Recall과 문서가 영향을 받는지 preview한다.
- 사용자가 결과를 확인한다.
- 삭제된 민감 내용은 derived index와 cache에서도 제거된다.

### Canonical Context 변화

- revision, supersession 또는 deletion/tombstone
- user provenance

### Derived State 변화

- 관련 index, summary와 document preview 무효화·재구축

### 금지 행동

- 사용자 correction을 LLM이 자동 거부
- 삭제한 원문을 immutable audit log에 계속 보존
- superseded Decision을 current Recall에 사용

### 자동 검증

- revision history
- supersession query
- deletion propagation
- export bundle에 삭제 원문 없음

### 수동 평가

- 사용자가 무엇이 남고 사라지는지 이해할 수 있는가

## H. source-grounded 문서 생성

### 시작 상태

- Canonical Context와 Repository Intelligence 사용 가능

### 사용자 요청

architecture, Decision report, design, implementation plan 또는 handoff 문서를 출력한다.

### 기대 사용자 경험

- 문서 종류와 범위를 선택한다.
- preview에서 source, Decision, coverage와 known gaps를 확인한다.
- 대상 형식과 저장 경로를 명시한다.
- 생성본은 자동으로 canonical truth가 되지 않는다.
- 필요하면 사용자가 편집본을 review/import할 수 있다.

### Canonical Context 변화

- 명시적 채택 시 document Source 또는 관련 Context 추가

### Derived State 변화

- document projection과 preview

### 금지 행동

- 사용자 승인 없이 Product Repository에 문서 쓰기
- source가 없는 semantic claim을 사실로 표현
- stale snapshot에서 생성한 문서를 최신이라고 표시

### 자동 검증

- metadata 포함
- source ref 유효성
- deterministic structural section
- unavailable provider fallback

### 수동 평가

- 문서가 현재 코드와 Decision을 정확히 설명하는가

## I. 부분 실패, 강제 종료와 복구

### 시작 상태

- 큰 repository 분석 또는 장기 명령 진행 중

### 실패 주입

- parser 일부 실패
- semantic provider unavailable
- process 강제 종료
- derived index 손상
- canonical write 직전 또는 직후 crash

### 기대 사용자 경험

- Core 저장 성공과 projection 실패를 구분한다.
- 분석 coverage와 빠진 영역을 표시한다.
- structural mode 또는 이전 canonical context를 계속 사용할 수 있다.
- derived state는 repair/reindex로 재구축한다.
- canonical transaction은 중복·부분 record 없이 복구된다.
- 장기 명령의 stdout, stderr, exit state가 한도 내에서 보존된다.

### 금지 행동

- partial analysis를 complete로 표시
- context 저장 실패를 성공으로 응답
- index 손상을 canonical loss로 처리
- command 종료 상태를 잃은 채 성공 추론

### 자동 검증

- fault injection
- restart recovery
- idempotent retry
- reindex from canonical/source
- bounded process cleanup

### 수동 평가

- 사용자가 신뢰할 수 있는 정보와 사용할 수 없는 정보를 구분할 수 있는가

## J. Volicord 자체 재구축 dogfood

### 시작 상태

- Canonical Context, Repository Intelligence, Inquiry, Checkpoint와 Recall의 최소 기능 완성

### 사용자 여정

```text
Volicord 저장소 분석
→ 남은 재구축 질문 해결
→ 구현 작업
→ Checkpoint
→ 새 Codex 세션 Recall
→ 다른 clone import
→ architecture 및 handoff 문서 생성
→ 잘못된 기억 수정
```

### 통과 기준

- 현재 재구축 작업이 기존 workflow 없이 진행됨
- 중요한 Decision과 코드 관계를 source로 추적 가능
- 새 세션이 목표와 이유를 복구
- 사용자에게 불필요한 절차 질문을 반복하지 않음
- 최소 한 번의 분석 실패·index 재구축·bundle import를 실제로 수행

## 4. 교체 가능 제품의 최소 gate

기존 구현을 제거하기 전에 다음이 모두 실제 환경에서 통과해야 한다.

- clean install과 supported host 연결
- repository-wide 분석과 coverage 표시
- 단계적 질문, 중단과 재개
- 현재 host에서 한 번의 사용자 답변으로 Decision 기록
- ordinary work와 source-linked Checkpoint
- 완전히 새 세션의 Recall
- 다른 clone의 portable 복구
- 기억 수정, supersession과 deletion
- source-grounded 문서 출력
- partial failure, crash와 derived-state repair
- Volicord 자체를 포함한 여러 실제 저장소에서 반복 사용
