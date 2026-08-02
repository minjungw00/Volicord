# 저장소 관찰

이 문서는 Guard, 예상 쓰기, 미기록 변경이 사용하는 호출 범위 Product Repository
관찰 계약을 담당합니다.

## 표면 안정성

관찰 상태, 정확한 호스트 상관관계, 스냅샷과 delta 무결성, 예상 쓰기 일치,
관찰 불가 동작, 미기록 변경 생성 규칙은 `stable`입니다. 관찰자 구현과 제한된
자원 한도는 `internal`입니다.

## 정확한 호출

저장소 관찰 하나는 호환되는 Codex 도구 호출 하나에만 속합니다. 의미 좌표에는
다음 항목이 들어갑니다.

- 프로젝트와 Agent Connection identity
- 정확한 호스트 세션, 호스트 turn, 호스트 tool-use ID, 정규 호스트 도구 이름
- 호환되는 `PreToolUse` Guard 이벤트와 호출이 끝났을 때 정확히 일치하는
  `PostToolUse` Guard 이벤트
- 관리 Guard 경계가 요구할 때의 현재 Guard Installation
- 의미 기반 저장소 관찰자 계약 digest

Store는 이 완전한 호출 좌표마다 관찰을 최대 하나만 허용합니다. 다른 turn,
tool-use ID, 도구 이름, Connection, 프로젝트, Guard 이벤트는 해당 좌표를 만족할
수 없습니다. 보고된 효과, 경로 hint, 주변 저장소 상태, 도구 이름만으로는
상관관계를 만들 수 없습니다.

## 관찰 용어

- **정확한 호스트 호출**은 위의 완전한 프로젝트, Connection, session, turn,
  tool-use ID, 정규 tool-name, 호환 Guard 이벤트 좌표입니다.
- **저장소 기준선**은 해당 호출을 위해 영속한 정확하고 안정적인 `PreToolUse`
  스냅샷입니다.
- **저장소 결과**는 정확히 일치하는 안정적인 `PostToolUse` 스냅샷입니다.
- **저장소 delta**는 그 기준선에서 결과까지의 결정적인 순 전이입니다.
- **불일치 delta**는 완전한 저장소 delta 가운데 해당 관찰의 정확한 예상 쓰기가
  포함하지 않는 비어 있지 않은 부분입니다.
- **관찰 불가**는 완전한 저장소 delta가 있다고 주장하지 않는 닫힌 관찰
  결과입니다.

## 일반 파일 콘텐츠 증거

Worktree에서 직접 관찰한 일반 파일은 두 콘텐츠 영역의 타입이 지정된 증거를 모두
보관합니다.

- 파일에서 읽은 정확한 worktree 바이트의 SHA-256 식별값
- 같은 바이트 스트림과 정규 Product Repository 상대 경로를 사용해 Git의 현재
  경로별 clean 변환으로 얻은 정규 Git blob 식별값

이 변환은 읽기 전용입니다. Git 객체를 쓰지 않고 저장소, 경로, attribute,
configuration, encoding, clean filter 맥락을 사용합니다. 정확한 바이트 해시와 Git
변환은 파일 스트림 하나를 함께 사용합니다. 변경 불가능한 tree에서 얻은 일반 파일은
정규 Git blob 식별값만 가지며 worktree 바이트 식별값을 만들지 않습니다. 일반 파일
상태는 실행 비트도 보관합니다.

일반 파일 비교는 중앙화된 규칙 하나를 사용합니다.

- 일반 파일과 일반 파일이 아닌 상태는 서로 다릅니다.
- 실행 비트가 다르면 서로 다릅니다.
- Worktree에서 직접 관찰한 상태끼리는 정확한 worktree 바이트 식별값을 비교합니다.
- Tree에서 얻은 상태가 하나라도 있으면 정규 Git blob 식별값을 비교합니다.
- Tree에서 얻은 일반 파일 상태끼리는 정규 Git blob 식별값을 비교합니다.

Symbolic link는 타입이 지정된 정확한 대상 식별값을, Gitlink는 정규화된 checkout
commit 식별값을 보관합니다. Git 변환 실패, filter 실패, 시간 초과, 잘못된 출력,
프로세스 격리 실패, 자원 한도 소진은 성공한 일부 snapshot이나 빈 snapshot이 아니라
관찰 불가 결과가 됩니다.

## 관찰 상태

```schema
RepositoryObservationState:
  open
  complete
  unavailable
```

`open`은 엄격하게 decode한 정규 저장소 기준선과 검증한 digest를 담습니다.
post-tool 이벤트, 저장소 결과, 저장소 delta, 관찰 불가 이유, 완료 시각,
terminal 결과는 담지 않습니다.

`complete`는 정확히 일치하는 post-tool 이벤트, 엄격하게 decode한 정규 post-tool
저장소 결과, 결정적인 저장소 순 delta, 검증한 스냅샷 및 delta digest, 완료
시각을 담습니다. Delta는 비어 있을 수 있습니다.

`unavailable`은 닫힌 이유 하나와 완료 시각을 담습니다. 완전한 delta가 있다고
주장하지 않습니다. 호출이 거부됐거나 post-tool 완료를 관찰할 수 없게 된
경우에도 유효한 baseline은 남아 있을 수 있습니다.

`open` 관찰은 다음 세 가지 정확한 생명주기 경계 중 하나에서 닫힙니다.

- 정확히 일치하는 `PostToolUse`는 post-tool 관찰 결과에 따라 `complete` 또는
  `unavailable`을 만듭니다.
- 같은 관리 프로젝트 session에서 다음으로 수락된 `UserPromptSubmit`은 서로 다른
  확립된 turn의 open 관찰을 `unavailable(post_tool_not_observed)`로 닫습니다. Prompt의
  정확한 현재 turn에 속한 관찰은 병렬 tool 호출이 끝날 수 있도록 open으로 남깁니다.
- 소유한 `managed_host` runtime이 권위 있게 종료되면 정확하고 한도가 있는
  project-session binding의 나머지 관찰을
  `unavailable(managed_session_terminated)`로 닫습니다.

Turn identity는 lexical 또는 numeric 순서가 아니라 typed exact equality로 비교합니다.
수락한 prompt capture와 이전 turn terminalization은 immediate bounded project
transaction 하나를 공유합니다. Runtime cleanup은 정확한 Registry project-session
binding만 사용합니다. Runtime Home recovery는 Registry에서 이미 권위 있게 종료된
session에 대해서만 이 cleanup을 반복하며, replay는 terminal row를 변경하지 않습니다.

생명주기 terminalization은 pre-tool baseline을 보존하고 reason, completion time, 안정적인
terminal result 하나를 기록하며 delta는 unavailable로 둡니다. Product Repository scan,
expected-write match, write-ticket 소비, Unrecorded Change 또는 finding 생성, 합성 path 생성,
actor 귀속, 인과관계 주장을 하지 않습니다. Row 검증이 하나라도 실패하면 한도가 있는
project transaction 전체를 rollback합니다.

Terminal 관찰은 `open`으로 돌아가지 않습니다. 정확한 replay는 Product
Repository를 다시 scan하지 않고 저장된 terminal 결과를 반환합니다. 충돌하는
두 번째 `PostToolUse` 이벤트는 거부합니다.

스냅샷, 결과, delta, 제한된 metadata decoder는 알 수 없는 필드, 잘못된 Product
Repository 경로, 유효하지 않은 상태 조합, 비정규 encoding, 중복되거나 정렬되지
않은 transition, 의미상 비어 있는 transition, digest 불일치를 저장 데이터 손상으로
거부합니다.

## Pre-tool aggregate

Guard는 호환되는 모든 도구 호출에 대해 안정적인 pre-tool Product Repository
스냅샷을 capture하려고 시도합니다. Typed `CodexHookToolCorrelation`을 parse하며
일반 invocation 필드를 검색하지 않습니다.

`may_write_product`와 `unknown_product_effect`에서는 Guard가 허용 결정을 반환하기
전에 안정적인 baseline을 capture하고 영속해야 합니다. Baseline capture나
aggregate 영속화가 실패하면 typed reason으로 호출을 거부합니다.

`no_product_write`에서는 관찰 불가 상태를 명시적으로 기록하고 호출을 계속할 수
있습니다. 결과는 Product Repository 변경이 관찰되지 않았다고 주장하지 않습니다.
Baseline capture가 성공했다면 read-only로 선언된 도구의 비어 있지 않은 post-tool
delta도 탐지할 수 있습니다.

Immediate Store transaction 하나가 다음 항목을 기록합니다.

- 호환되는 `PreToolUse` Guard 이벤트
- 정확한 호스트 호출
- baseline이 있는 `open` 관찰 또는 terminal `unavailable` 관찰
- 현재 쓰기 권한에 따라 필요한 정확한 예상 쓰기

실패하면 aggregate 전체를 rollback합니다. 거부된 호출은 post-tool 이벤트를
기대하는 `open` 관찰을 남기지 않습니다. Guard는 필요한 transaction이 commit된
뒤에만 호스트 결정을 반환합니다.

## Post-tool aggregate

Guard는 정확히 일치하는 호출의 안정적인 post-tool 스냅샷을 capture합니다. 호스트가
제공한 경로는 선택된 호스트 계약이 해당 필드를 담당할 때만 제한된 후보 hint로
사용할 수 있습니다. 이 정보만으로 Product Repository 변경이 되지는 않습니다.

Immediate Store transaction 하나가 다음 순서로 처리합니다.

1. 정확한 `open` 관찰을 읽고 엄격하게 검증합니다.
2. 호환되고 일치하는 `PostToolUse` Guard 이벤트를 기록합니다.
3. 완전한 post 스냅샷과 결정적인 순 delta를 저장하거나 관찰을
   `unavailable`로 닫습니다.
4. 완전한 delta와 정확한 관찰의 예상 쓰기를 대조합니다.
5. 비어 있지 않은 불일치 부분에만 미기록 변경을 만듭니다.
6. 안정적인 terminal 결과를 저장하고 반환합니다.

Baseline이 없거나 충돌하거나 손상됐거나 관찰 불가인 경우 명시적인 관찰 불가
결과와 진단을 만듭니다. 빈 delta나 미기록 변경을 만들지 않습니다. 비어 있는
완전한 delta는 미기록 변경을 만들지 않으며 예상 쓰기를 만족하지 않습니다.

Delta는 정확한 호출 구간 동안의 Product Repository 순 전이를 기록합니다.
Actor identity나 단독 인과관계를 주장하지 않습니다.

## 예상 쓰기

예상 쓰기 각각은 저장소 관찰 하나와 그 정확한 호스트 도구 호출에 속합니다.
현재 쓰기 권한이 다루는 비어 있지 않고 정규화되고 중복 없는 경로 집합을
가집니다.

완전하고 비어 있지 않은 delta만 예상 쓰기를 만족할 수 있습니다.

- 정확히 포함되는 경로를 matched로 기록합니다.
- Delta 전체가 포함되면 미기록 변경을 만들지 않습니다.
- 일부만 포함되면 추가 경로에 대해서만 미기록 변경 하나를 만듭니다.
- 빈 관찰 또는 관찰 불가 상태는 예상 쓰기를 unmatched로 남깁니다.

세션만 사용하는 조회, 시간 구간 조회, post-event 검색, 대체 invocation 식별자는
일치에 참여하지 않습니다.

## 미기록 변경

미기록 변경은 완전히 관찰된 불일치 delta만 나타냅니다. 관찰 경로 집합은 비어
있지 않고 정규화되고 정렬되고 중복이 없습니다. 정확한 저장소 관찰에 연결되고
결정적인 불일치 delta digest를 저장합니다.

Identity는 프로젝트, 정확한 저장소 관찰, 불일치 delta digest에서 파생합니다.
Guard 이벤트 ID를 identity salt로 사용하지 않습니다. 같은 terminal 관찰 replay는
멱등적입니다.

미해결 미기록 변경은 조정과 닫기 준비 상태에 참여합니다. 관찰 불가 진단은 별도
운영 사실로 남고 합성 경로 finding이 되지 않습니다.

## Guard 결과 경계

Guard는 다음 항목이 들어 있는 typed 저장소 관찰 결과 하나를 반환합니다.

- 관찰 상태와 정확한 관찰 identity
- 상태가 `complete`일 때의 완전한 delta 요약
- 상태가 `unavailable`일 때의 닫힌 이유
- 정확한 예상 쓰기 일치 사실
- 불일치 delta에서 만든 미기록 변경

Codex adapter는 호스트 JSON, context, warning, denial, stderr, exit projection을
담당합니다. `PostToolUse` 출력은 이미 끝난 호출을 설명하며 방지나 되돌림을
주장하지 않습니다.

Guard와 저장소 관찰은 협력적 로컬 기록입니다. Actor identity, intent, 완전한
monitoring, OS enforcement, correctness를 증명하지 않습니다.

## 관련 담당 문서

- [런타임 경계](runtime-boundaries.md)
- [저장소 기록](storage-records.md)
- [저장소 DDL](storage-ddl.md)
- [저장 효과](storage-effects.md)
- [저장소 버전 관리](storage-versioning.md)
- [보안](security.md)
- [변경 조정](api/method-reconcile-changes.md)
- [상태 스키마](api/schema-state.md)
