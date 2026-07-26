# 실패 모델

이 문서는 Core, Store, 어댑터, 전송, 관리 명령, 진단에 공통으로 쓰는 제품 전체
실패 범주를 담당합니다. 정확한 범주 식별자와 의미 경계, 영속 상태 처리, 실패를
합성 값이나 기본 성공 값으로 바꾸지 않는 규칙을 정의합니다.

특정 API 응답 envelope, 전송 오류 형태, 메서드 저장 효과, 도메인별 사유 목록,
repair 명령은 정의하지 않습니다. 그 세부사항은 영향을 받는 표면의 집중 담당
문서에 남습니다.

<a id="surface-stability"></a>
## 표면 안정성

다섯 범주의 의미, machine-readable identifier, 영속 권한 및 정책 데이터의
fail-closed 규칙, 기본값 합치기 금지 규칙은 stable 계약입니다. 사람이 읽는 표시
문구와 도메인별 진단 세부사항은 집중 담당 문서가 달리 정하지 않는 한
diagnostic입니다.

## 정식 실패 범주

모든 기계 판독 실패 분류는 아래의 정확한 주 범주 식별자 중 하나를 사용합니다.

| 범주 | 식별자 | 의미 |
|---|---|---|
| `Rejected` | `rejected` | 요청 형태 또는 필수 맥락이 구조적으로 유효하지 않아 정책 평가나 성공 동작 분기로 진행하지 못했습니다. |
| `NotAllowed` | `not_allowed` | 구조적으로 유효한 요청과 완전한 필수 맥락이 정책 평가에 도달했지만 정책이 요청한 동작을 허용하지 않았습니다. |
| `Unavailable` | `unavailable` | 동작, 보조 capability, 필수 조회를 현재 수행할 수 없지만 사용 가능한 데이터가 영속 계약 손상을 확정하지는 않습니다. |
| `Degraded` | `degraded` | 핵심 동작은 계속할 수 있지만 명시적으로 식별된 검증, 진단, 보조 정보 구성 요소가 불완전합니다. |
| `Corrupt` | `corrupt` | 영속되었거나 신뢰되는 담당 데이터가 선언된 스키마, type, canonical encoding, 필드 간 계약을 위반합니다. |

기계 판독 결과나 진단은 정확한 범주 식별자를 담아야 합니다. 도메인이 같은 범주
안에서 원인을 구분한다면 도메인 담당 문서가 정의한 사유 식별자도 담아야 합니다.
사람이 읽는 문구만으로는 범주나 사유가 되지 않습니다. 사유 식별자가 범주의
의미를 암묵적으로 바꾸면 안 됩니다.

## 범주 선택 경계

`Rejected`와 `NotAllowed`는 정책 평가 여부로 구분합니다. 필수 맥락이 없거나
유효하지 않으면 `Rejected`이며 이를 정책상 비허용으로 표현하면 안 됩니다.
`NotAllowed`는 구조적으로 유효한 요청, 해석된 필수 맥락, 실제 정책 판단을
요구합니다. 메서드 담당 문서가 커밋되는 비허용 결과를 정의할 수는 있지만 범주
자체가 커밋을 허용하지는 않습니다.

`Unavailable`과 `Degraded`는 핵심 동작을 진실하게 계속할 수 있는지로 구분합니다.
필수 동작이나 조회를 수행할 수 없으면 `Unavailable`입니다. 핵심 동작은
유효하지만 이름이 붙은 보조 검증이나 정보 출처가 불완전하면 `Degraded`이며,
빠진 부분과 그 영향을 계속 보이게 해야 합니다.

지원되는 영속 또는 신뢰 계약을 따른다고 표시된 데이터가 그 계약을 위반하면
`Corrupt`입니다. 아직 영속 담당 상태가 되지 않은 신뢰할 수 없는 경계 입력이
잘못되었거나 알려지지 않은 경우는 `Corrupt`가 아니라 `Rejected`이며 지원되는
형태로 추정하지 않습니다.

### Runtime Home setup 진행 중

`runtime_home.mutation.setup_in_progress`는 setup이 `ExclusiveSetup`을 보유해
지원하는 일반 writer가 `SharedWriter`를 획득하지 못할 때 반환하는 안정적인 typed
coordination condition입니다. Policy `NotAllowed`, 영속 데이터 `Corrupt`, type 없는
SQLite busy failure가 아닙니다. Fact는 정규 Runtime Home, 변경 domain, 요청 mode,
wait policy, 경과 시간, 재시도 가능 여부로 제한하며 coordination 파일 경로를
노출하지 않습니다.

필수 CLI 또는 MCP 변경·관찰에서는 연산이 `Unavailable`이며 typed 비성공 결과를
반환하고 Runtime Home 효과를 만들지 않습니다. `record` Guard hook에서는 기존
nonblocking policy에 따라 host 동작을 계속하면서 관찰 영속화를 사용할 수 없음을
명시합니다. 응답은 `persisted=false`를 보고하며 `Deny`를 만들어 내지 않습니다.
Setup은 publication, checkpoint, 확인, rollback 전체에서 배타 context 하나를
유지합니다. Publication ID 소유권 검증과 함께 적용되어 setup이 삭제하거나 복원할
상태에 승인된 외부 변경이 끼어들지 못하게 합니다.

### Semantic host contract 거부

Codex wire 입력은 명시적으로 선택한 profile로만 decode합니다.
`CodexMcpTurnMetadata`는 `codex-mcp-turn-metadata`를 선택하고 별도
`CodexCommandHooks`는 `codex-command-hooks`를 선택합니다. 한 계약의 실패를 다른 계약으로 다시
시도하거나 재해석하거나 다른 계약의 field로 보완하지 않습니다. 알 수 없는 추가 field는
허용하지만, 필수 field가 없거나 유효하지 않은 경우, 예상하지 않은 event 값, 일관되지 않은
MCP thread 좌표, 계약의 크기 또는 depth 한도를 넘은 입력은 Store 또는 policy 평가 전에
`Rejected`입니다.

Typed host-contract error는 닫힌 error code와 정적인 field label만 보관합니다. 전체 hook
payload, MCP metadata, tool input, tool response를 보관하거나 투영하지 않습니다. Hook
failure는 session, environment, runtime state에서 thread 좌표를 만들어 내지 않습니다. 등록된
managed-session 상관관계는 profile decode가 성공한 뒤에도 독립된 MCP 권한 check로 남습니다.

`record` profile의 Guard hook에서는 이 `Rejected` 분류가 관찰 입력을 설명합니다. 이는
`NotAllowed` policy 결과가 아니며 host 동작이 거부됐다는 뜻도 아닙니다. Store를 사용할 수
있으면 Guard는 호환되지 않는 관찰을 기록하고 `GuardPolicyDecision`을 비워 두며, 그 event를
phase 충족에 사용하지 않고 host adapter에 제한된 feedback과 함께 계속하라고 요청합니다.
Event 영속화를 사용할 수 없으면 관찰 결과는 명시적으로 unavailable로 남고, 영속화 실패만으로
policy denial이 되지 않습니다. 실제 Guard `NotAllowed` 결과에는 호환되는 입력이 policy에
도달해 `Deny`를 낸 사실이 필요합니다.

Guard probe acquisition은 routing, decoding, semantic tool relevance, identity,
correlation을 하나의 failure로 합치지 않습니다. Callable과 catalog 소유 role은 probe
전용 좌표보다 먼저 해석합니다. `UnrelatedRoutedTool`은 어떤 probe 좌표를 주장하더라도
workflow control과 그 밖의 known tool을 위한 nonterminal trace입니다. 알 수 없는
same-server callable도 정확한 현재 verification ID를 주장하지 않으면 nonterminal입니다.
어느 경우도 repair reason, retry, proof, acknowledgement, root finding,
status-read-budget effect를 선택하지 않습니다. `HookPayloadIncompatible`,
`CallableIdentityUnknown`, `CallableIdentityMismatch`, `VerificationIdMismatch`,
`SessionMismatch`, `TurnMismatch`, `ToolUseMismatch`는 Volicord가 관찰할 수 있었던 마지막
한정 terminal stage를 나타냅니다. `HookEventNotObserved`는 의도적으로 더 약한
의미입니다. Probe event가 Volicord에 도달하지 않았음을 뜻하며 host가 event를 내지
않았는지 구성된 routing이 선택하지 않았는지 증명할 수 없습니다. 따라서 status tool
자체의 routed Pre/Post hook은 부재를 `CallableIdentityMismatch`로 바꿀 수 없습니다.
Acquisition record에는 범주형 fact와 한도가 있는 callable identity만 넣고 제한 없는
hook payload는 넣지 않습니다.

Connection-integration 검증은 도구 호출 거부와 run lifecycle을 구분합니다. Malformed ID,
다른 runtime, native session 또는 turn의 호출은 attempt를 변경하지 않고 거부합니다.
Semantic host contract가 bounded observation policy를 선택합니다. 현재 Codex command
hook에서는 probe acknowledgement 뒤 synchronous status read 한 번이 완료를 관찰하거나
`repair_required`를 영속하며 TTL을 기다리지 않습니다.

Repair reason은 hook event 누락, 비호환 payload, callable identity, verification ID,
session, turn, tool-use, integration revision, hook definition, policy, deferred deadline
실패를 구분합니다. Retry policy는 별도로 `no_automatic_retry`,
`new_turn_required`, `host_reload_required`, `hook_review_required`,
`repair_required` 중 하나입니다. 정확한 좌표 replay는 같은 ID와 `awaiting_probe`,
`awaiting_observation`, `complete`, `repair_required` 상태를 반환합니다. 뒤의 두 상태는
불변 terminal 상태이며 어떤 replay도 이를 다시 활성화하거나 다른 caller 좌표에
acknowledgement를 노출하지 않습니다. Cleanup expiry는 보관에만 영향을 줍니다. Retry
policy가 새 attempt를 허용하더라도 필요한 repair transition으로 실제 새 semantic 좌표가
생긴 뒤에만 가능하며 같은 turn에서 자동으로 만들지 않습니다. 이 workflow 상태는
Connection check 사실이며 위의 제품 전체 실패 범주를 다시 정의하지 않습니다.

활성 연결 검증은 구성된 Codex 실행 파일을 찾고 version 명령을 실행하며 동작 probe를
아래에서 정의하는 다섯 가지 상태 모델로 보고합니다. 실행 파일을 찾거나 실행하지 못한
경우 일반 실패 범주 경계에서는 `Unavailable`, 연결 보고서에서는 실패한
`host_executable` check입니다. PATH executable version은 설치 probe로 남으며 실제 MCP
peer `clientInfo`를 대신하지 않습니다. 두 version이 다르다는 이유만으로 유효한 managed
session을 무효화하지 않습니다.

관리 connection command report는 필수 check가 하나 이상 실패했거나 blocked인 typed
운영 결과에 `failed`를 사용합니다. Host 관찰이 pending이면 `action_required`이며
`Degraded`, stale/broken 공개 상태, 예상하지 못한 런타임 오류로 표시하지 않습니다.
사용법 오류와 예상하지 못한 런타임 또는 직렬화 실패는 성공이나 action-required
보고서로 꾸미지 않고 CLI 오류 채널에 남깁니다.

어느 범주도 다른 범주의 의미를 포함하지 않습니다. 특히 다음 규칙을 지킵니다.

- `Unavailable`은 비어 있는 성공 결과가 아닙니다.
- `Degraded`는 완전한 검증이나 제한 없는 성공 상태가 아닙니다.
- `Corrupt`는 선택적인 값의 부재가 아닙니다.
- `NotAllowed`는 구조적 거부가 아닙니다.

## 영속 권한 및 정책 데이터

선언된 계약에 따라 decode하고 검증할 수 없는 영속 권한 또는 정책 데이터는
`Corrupt`이며 실패 시 닫히도록 처리합니다. 그 데이터에 의존하는 동작은 권한을
도출하거나, 정책 판단을 내리거나, 성공 효과를 기록하거나, 종속 담당 상태를
변경하기 전에 중단해야 합니다.

타입이 정해진 영속 JSON은 선언된 전체 type으로 decode해야 합니다. 문법 실패,
잘못된 최상위 형태, 알 수 없는 closed variant, 필수 필드 누락, 담당 문서가 추가
필드를 거절하는 곳의 추가 필드, 필드 간 invariant 위반은 모두 `Corrupt`입니다.
어느 경우도 빈 배열, 빈 객체, 값 부재, 호스트별 기본값으로 바꾸면 안 됩니다.

표시 전용 또는 보조 데이터가 없다고 해서 자동으로 손상은 아닙니다. 집중 담당
문서가 이를 `Unavailable` 또는 `Degraded`로 분류하고 핵심 동작을 계속할 수
있는지 정해야 합니다. 유효한 빈 배열이나 객체는 선언된 스키마가 그 정확한 값을
명시적으로 허용할 때만 유효한 빈 값입니다.

복구 가능한 상태는 담당 문서가 정의한 명시적 verify 또는 repair 흐름을 통해서만
다시 만들 수 있습니다. 읽기와 일반 실행은 실패를 분류하는 동안 데이터를 변경하거나,
추정하거나, 암묵적으로 교체하면 안 됩니다.

Runtime Home bootstrap은 setup 변경 전에 이 규칙을 적용합니다. 기존 상태는 읽기 전용으로
검사하여 `Ready`, `Incompatible`, `Corrupt`로 분류하고, manifest 또는 물리 schema
불일치는 기존 bytes와 timestamp를 보존하면서 한도가 있는 typed fact를 보고합니다.
최종 경로가 없을 때만 staged creation과 기존 대상을 교체하지 않는 원자적 공개를 시작할
수 있습니다. 공개 전 실패는 staging을 제거하고 최종 경로를 만들지 않습니다. Rename이
성공한 뒤에는 호출자가 이미 invocation별 publication guard를 보유하므로 상위 directory
동기화, read-back, manifest 검증 실패에도 명시적인 rollback 또는 보존 권한이 남습니다.
그 결과인 composite 실패는 주 확인 오류, publication 발생 여부, rollback 결과, 최종 경로
존재 상태, 상위 entry 내구성을 보존하며 rollback 오류가 주 오류를 대체하지 않습니다.

Setup lease 경합은 검사, plan 구성, setup mutation 전에 발생합니다. 실패한 `setup_plan`
check code `setup_lease_busy`, finding code `setup.lease_busy`, action
`action.setup.wait_for_current_transaction`을 사용합니다. 한도 있는 fact는 정규 Runtime
Home, 요청 operation, immediate wait policy, elapsed time, 다른 setup의 lease 소유를
식별하지만 owner PID나 identity를 주장하지 않습니다. Action은 coordination 파일을
삭제하라고 하지 않고 해당 setup이 끝날 때까지 기다렸다가 다시 실행하도록 요구합니다.

Setup transaction 실패는 실패한 `setup_plan` check를 사용합니다. 일반적인 commit
실패에는 `finding.setup.transaction_failed`, planning 뒤 bytes가 바뀐 입력에는
`finding.setup.concurrent_modification`을 사용합니다. Lease를 보유한 publication 중
예상하지 않은 최종 경로를 만난 경우도 여기에 포함합니다. 이후 상태를 덮어쓰지 않고는 복원할 수 없는
target에는 `finding.setup.partial_rollback`을 사용합니다. 대응하는 diagnostic code는
각각 `setup.transaction_failed`, `setup.concurrent_modification`,
`setup.partial_rollback`입니다. 새 외부 bytes는 보존해야 합니다. 최종 mutation을
하나도 commit하지 않았으면 result disposition은 `preserved`, commit한 교체 가능
mutation을 모두 복원했으면 `rolled_back`, 안전하게 끝내지 못한 복원이 하나라도
있으면 `partially_rolled_back`입니다. 실패 details에는 disposition, 한도가 있는
rollback 개수와 오류를 담습니다. Activation은 commit한 setup에만 속하므로 실패
activation plan에는 host activation step이 없습니다.

Runtime Home rollback은 성공한 publisher의 guard가 즉시 정확한 재검증을 통과한 경우에만
허용됩니다. Publication ID, Runtime Home ID, manifest, 경로, schema, installation
불일치는 소유권 상실이며
`runtime_home_publication=ownership_lost_during_rollback`을 보고합니다. 최종 경로 부재는
보존으로 표현하지 않고 부재로 유지합니다. Setup 정책이나
managed-host 소비는 `owned_publication_preserved`를 보고합니다. Guard를 통한 제거는 부재가
관찰되는 즉시 `owned_publication_rolled_back`을 보고하며, 상위 directory 동기화 실패도
여기에 포함됩니다. 이 내구성 실패 때문에 setup이 `partially_rolled_back`이 될 수 있지만
제거 효과는 바뀌지 않습니다. 재귀 제거가 실패하고 target이 present이거나 분류할 수 없으면
`owned_publication_removal_incomplete`를 보고하고 정확한 실패 단계, 효과, 경로 관찰을
유지합니다.

이 범주는 전역 파일시스템 원자성을 주장하지 않습니다. Prepare는 commit 전에 끝나고,
각 관리 파일은 같은 directory의 원자 교체를 사용하며, 서로 독립적인 Runtime Home,
Codex 구성, Product Repository, Store 경계에 걸친 rollback에는 한도가 있습니다.

## 구조화된 Diagnostic Finding

`DiagnosticFinding`은 공유 read-only report projection입니다. Producer는
`DiagnosticFindingLifecycle::Occurrence` 또는
`DiagnosticFindingLifecycle::CurrentState`를 명시적으로 선택하고 해당 lifecycle type을
구성해야 합니다. Runtime-session 존재 여부로 lifecycle을 고르지 않습니다. 두 형태 모두
namespaced code, domain, stage, severity, producer source, typed subject, 안전하게 projection한
facts, 0개 이상의 cause reference와 권장 action, 관찰 timestamp, 적용 가능한 correlation
좌표를 담습니다. Domain 담당자는 closed code vocabulary와 오류를 finding으로 변환하는
규칙을 계속 담당합니다. Namespaced `code`가 안정적인 기계 판독용 diagnostic
identity입니다. 각 domain의 typed diagnostic kind가 이 code와 action 정책을 선택하며,
한도가 있는 사람이 읽는 세부사항은 identity와 분리되고 또 다른 identity field로 저장하거나
projection하지 않습니다.

`DiagnosticFinding` 자체는 쓰기 가능한 lifecycle 입력이 아닙니다. Store mutation은 삽입에
`OccurrenceDiagnosticFinding`, snapshot 활성화 또는 갱신에
`CurrentDiagnosticFinding`, 명시적 해소에 `CurrentDiagnosticKey`를 받습니다.

`OccurrenceDiagnosticFinding`은 runtime, process, protocol 또는 그 밖의 event 성격 관찰
하나를 기록합니다. 각 occurrence에는 새로 생성한 opaque `DiagnosticOccurrenceId`가
부여됩니다. 동일한 diagnostic data를 반복해도 ID가 다르고 서로 독립적인 불변 row가
생깁니다. Runtime correlation 유무와 관계없이 occurrence graph 삽입은 insert-only입니다.

`CurrentDiagnosticFinding`은 불변 `CurrentDiagnosticKey`와 교체 가능한
`CurrentDiagnosticSnapshot`으로 구성됩니다. Key에는 scope kind와 완전한 opaque scope
identity, 전체 diagnostic code, domain, stage, source, 하나의
`DiagnosticSubjectIdentity`가 포함됩니다. 이 subject identity는 typed subject 담당자가
domain separation, version, 길이 prefix를 적용한 정규 identity byte에서만 파생하는 검증된
opaque `sha256:<64 lowercase hex>` token입니다. 표시 문자열이 아니며 원래 path, installation
identifier, event identifier, 그 밖의 identity 입력을 노출하지 않습니다. Current key의 정규
identity는 별도의 domain separation과 version을 사용하며, 완전한 subject identity token을
포함한 모든 가변 구성 요소에 길이 prefix를 붙인 binary encoding입니다. ID는 이 encoding의
전체 SHA-256 digest를 사용한 정확한 `finding.current.sha256:<64 lowercase hex>` 형식입니다.
따라서 identity 차이를 모두 보존하면서 path와 그 밖의 subject identity 입력을 ID 밖에
둡니다.

교체 가능한 snapshot은 severity, facts, actions, correlation 좌표, integration revision, 관찰
시각, 나가는 cause edge, active 또는 resolved 상태와 함께 안전한 `DiagnosticSubject` 표시를
담습니다. 같은 key를 다시 관찰하면 이 안전한 표시 subject와 다른 snapshot field를 교체하고
resolved condition을 다시 active로 전환할 수 있습니다. `DiagnosticSubjectIdentity`를 포함한
identity field는 비교만 하며 갱신하지 않습니다. Redaction, 형식, 그 밖의 안전한 표시
세부사항만 바뀌어도 finding ID는 바뀌지 않습니다. 명시적 resolution은 `resolved_at`을
기록하고 현재 action과 나가는 cause를 제거하되 마지막 안전한 subject와 facts를 유지하며
active-current report에서는 해당 row를 제외합니다. 명시적 ID read는 resolved snapshot도
반환할 수 있습니다. Read는 안전한 표시 subject와 독립적으로 저장된 subject identity에서
key를 복원하고 current digest와 ID를 다시 계산하며, 불일치하면 영속 상태 corruption으로
처리합니다.

CLI 소유 운영 diagnostic은 code, domain, stage, source, 기본 severity, summary를 담는 불변
definition 하나를 가집니다. 폐쇄형 typed subject는 scope를 담당하고, 자체 정규 identity
encoding에서 `DiagnosticSubjectIdentity`를 구성하며, 별도의 안전한 표시 projection을
제공합니다. Typed subject namespace는 그 정규 encoding에 직접 포함되므로 서로 다른 subject
family의 표시 text가 같아도 identity가 하나로 합쳐지지 않습니다. Path를 담는 subject는 opaque
identity를 파생하기 전에 filesystem alias를 정규화하며 그 정규 path byte를 저장하지 않습니다.
선택적인 활성 검증은 담당자별 완전한 관찰 집합을 reconcile합니다. 관찰한 condition은
활성화하거나 갱신하고, 이전에는 active였지만 그 집합에서 빠진 담당 condition은 명시적으로
해소합니다.

현재 보고서는 명시적인 provenance를 가진 overlay로 선택한 ID를 해석합니다. 현재 평가가
계산한 inline finding을 Store 조회보다 먼저 사용합니다. 그다음 명시적인 영속 seed를 불변
occurrence 또는 active current-state row에서 해석하며, 같은 한도 있는 cause graph를 이어갈
수 있습니다. 명시적인 영속 reference인데 Store row가 없을 때만
`diagnostics.finding_record_missing`이 되며, 계산한 inline finding은 이 치환을 받지 않습니다.

Safe facts는 저장하거나 렌더링하기 전에 한도를 검증합니다. Typed projection은 민감한
key를 가리고 text 크기, collection 크기, nesting depth를 제한합니다. Raw environment map,
request body, tool argument set, credential, 제한 없는 child-process output은 diagnostic
fact가 아닙니다. Producer는 이런 입력을 finding에 옮기는 대신 한도가 있는 안전한 요약을
제공해야 합니다.

Registry는 공유 finding을 구조화된 column과 한도가 있는 subject, facts, action JSON으로
저장합니다. Current-state row는 검증된 opaque subject identity token도 저장하며 occurrence
row는 저장하지 않습니다. Cause reference는 별도 edge입니다. 모든 edge는 기존 finding을 가리켜야 하며,
중복 edge와 self edge는 거부하고, 검증된 graph 삽입과 Registry constraint가 cycle을
거부하며, 한도가 있는 traversal은 결정적입니다. Finding graph 삽입은 transaction
하나입니다. 유효하지 않은 node, 없는 cause, 중복 edge, cycle이 있으면 부분 graph나
dangling edge가 남지 않습니다. MCP terminal finding 삽입과 runtime-session 연결도 같은
방식으로 transaction 하나에서 수행할 수 있습니다.

Root cause 선택은 이 typed cause edge만 사용합니다. Summary를 parsing하거나 stage 또는
enum 순서를 비교하거나 첫 번째 실패 check를 고르지 않습니다. 선택 결과는 ID로 정렬해
결정적이며, 서로 독립인 root를 모두 유지하고 선택한 ancestor가 이미 설명하는 downstream
증상은 제외합니다. 따라서 여러 선택 finding이 같은 ancestor로 모여도 그 ancestor는 한
번만 나타납니다. Traversal은 cause edge 32개로 제한합니다. 알 수 없는 reference, cycle,
한도를 넘는 경로가 있으면 root를 추정하지 않고 선택을 거부합니다.
`DiagnosticReport.root_cause_ids`는 report finding에서 계산한 결과이며 caller가 별도로
선택해서 제공할 수 없습니다.

`DiagnosticReport`는 선택한 Connection을 위한 lossless diagnostic JSON envelope입니다.
`schema_version`은 `2`입니다. Typed `operation`, 집계 `status`, 생성 timestamp, 선택적인
Connection context, 전체 check 배열, 한도가 있는 finding graph, 계산한 root-cause ID,
중복 제거한 report action, operation별 typed detail, report limit을 담습니다. Report
action은 namespaced code, 한도가 있는 summary, 해당 action이 복구하는 정확한 root ID를
담습니다. Connection context에는 관련 Guard verification ID와 role을 보존하는
runtime-session evidence인 `latest_managed_attempt`,
`latest_managed_capability_proof`, `guard_verification_attempt`,
`guard_verification_proof`가 들어갑니다. ID는 finding correlation뿐 아니라 check
evidence에서도 수집하고, session 하나가 여러 role을 가지면 정규 role 목록이 있는 항목
하나로 나타냅니다. 이 role은 현재 attempt health, managed capability, 상관관계가 확인된
Guard 근거를 구분합니다.
`2`가 아닌 schema version, 알 수 없는 최상위 구성원, 중복 check 또는 finding ID,
잘못된 cause graph, 계산 결과와 다른 supplied root list, 중복 action code, root가 아닌
finding을 가리키는 action은 역직렬화에서 거부합니다.

정확한 finding 및 runtime-session read는 별도의 schema 1
`DiagnosticLookupReport`를 사용합니다. `lookup_status`는 정확히 `found` 또는
`not_found`이며 Connection 집계 status나 check status를 사용하지 않습니다. 찾은 finding
root와 한도가 있는 cause graph의 모든 entry는 `StoredDiagnosticFinding`을 사용합니다.
Occurrence는 `lifecycle=occurrence`로 표시하고, current record는
`lifecycle=current_state`로 표시하면서 명시적인 `current_state_status`와 `resolved_at`을
함께 담습니다. 같은 envelope는 서로 다른 runtime-session root를 담으면서 lifecycle을
보존한 terminal 및 correlated occurrence를 유지할 수 있습니다. Finding severity와
current-state status는 저장된 condition을 설명하며, 어느 쪽도 성공한 lookup을 실패한
lookup으로 바꾸지 않습니다.

Machine consumer는 관찰 결과를 구조적으로 구분합니다. 관찰 부재는 생략한 optional 값
또는 담당자가 정의한 typed `observation_state=absent`, 관찰한 빈 collection은 값이 있는
`[]`, 관찰 실패는 cause finding이 있는 `failed` check, prerequisite 때문에 막힌 관찰은
해당 root ID가 있는 `blocked` check입니다. Producer는 consumer가 parsing해야 하는 사람용
summary로 이 상태를 encode하면 안 됩니다. 렌더러는 typed fact를 골라 라벨을 붙일 수
있지만 산문에서 cause edge나 action category를 만들 수 없습니다.

Connection 검증은 이 cause graph를 정확히 다섯 가지 check 상태로 사용합니다. `passed`는
check가 성공적으로 끝났다는 뜻입니다. `pending`은 필요한 외부 관찰 또는 사용자가
일으키는 event가 아직 없고 이를 막는 실패 prerequisite도 없다는 뜻입니다. `failed`는
check 자체가 실패를 관찰했다는 뜻입니다. `blocked`는 prerequisite finding이 실패해
check를 실행하거나 관찰할 수 없었다는 뜻입니다. `not_applicable`은 해당 Connection 또는
profile에 check가 적용되지 않는다는 뜻입니다. Blocked check는 해소된 root finding ID를
담습니다. Root 기반 action은 중복을 제거하며, blocker가 해소되기 전에는 blocked
downstream 관찰을 위한 action을 만들지 않습니다. 기준 check dependency와 report 집계
규칙은 [Agent Connection](agent-connection.md)이 담당합니다.

Registry를 열기 전에 실패하면 공유 stderr fallback envelope은 아래 정확한 형태의 한도가
있는 단일 행 하나뿐입니다.

```text
VOLICORD_DIAGNOSTIC_V1 <bounded-json>
```

JSON은 정확히 현재 공유 `DiagnosticFinding` 하나입니다. Format과 parse는 공유 필드 검증,
safe-fact 한도, 정확한 prefix, 단일 행 형태, 전체 envelope byte 한도를 집행합니다. 이
fallback은 환경 dump를 허용하지 않으며 두 번째 diagnostic model을 만들지 않습니다.

## 합성 값과 기본값으로 합치기 금지

다음 값을 사용해 실패를 일반 값으로 바꾸면 안 됩니다.

- 계약에 실제로 없던 빈 문자열, 배열, 객체, 0 값
- 합성 식별자, placeholder record, 조작한 timestamp, 조작한 capability
- decode 또는 조회 실패 뒤 고른 기본 enum variant
- fallback 호스트, 어댑터, decoder, 외부 계약, 저장소 형태
- 현재 계약 밖의 표현을 현재 값으로 취급한 경우
- 사람용 문구로만 실패를 표시하는 성공 응답

`unwrap_or_default()` 같은 구현 편의 기능은 이 계약을 바꾸지 않습니다. 기본값은
영속 전에 담당 문서가 정의한 typed construction 경로에서 만들어졌고, 그 결과로
저장된 값 자체가 선언된 전체 계약을 통과할 때만 유효합니다.

### Activation condition을 합치지 않기

Connection activation은 다음 구분을 보존합니다.

- `unknown`은 권위 있는 hook 상태와 현재 definition 관찰이 없다는 뜻입니다.
  `disabled`, failure, untrusted와 같지 않습니다.
- `review_required_by_setup`은 setup이 definition을 바꿔 host review가 남았다는
  뜻이며 configuration failure가 아닙니다.
- `bypassed_for_invocation`은 호출에 한정된 명시적 host 근거이며 지속적인 activation이
  아닙니다.
- `disabled`에는 명시적 host 근거가 필요하며 `repair_hook_contract`로 routing합니다.
- 이전 `latest_managed_capability_proof`가 남아 있어도 현재 managed session이 terminal이면
  failure입니다.
- 현재 Guard attempt가 없으면 pending이며 사용자 수준
  `request_integration_verification`으로 routing합니다. Workflow가 지시한 Guard probe는
  그 step 안에 중첩됩니다.
- `repair_required`는 failed `correlated_guard_verification`으로 유지합니다. Typed
  recoverability가 집계를 `action_required`로 만들 수 있지만 terminal attempt를
  pending으로 바꾸거나 무조건 probe를 다시 실행하도록 허용하지 않습니다.
- `ambient_hook_coverage` 통과는 상관관계가 확인된 Guard 성공을 증명하지 않으며 오래된
  `guard_verification_proof`는 더 최신 실패 attempt를 숨기지 못합니다.

Guard probe root finding은 typed repair reason과 acquisition stage에서 직접 고릅니다.
안정적인 범주는 `guard.probe.hook_event_not_observed`,
`guard.probe.payload_incompatible`, `guard.probe.callable_mismatch`,
`guard.probe.verification_id_mismatch`, `guard.probe.session_mismatch`,
`guard.probe.turn_mismatch`, `guard.probe.tool_use_mismatch`,
`guard.probe.current_contract_changed`입니다. Summary parsing은 diagnostic 분류 경계가
아닙니다.

Failed 및 blocked check는 typed root finding ID를 유지합니다. Remediation은 닫힌
activation step `reload_codex`, `review_project_hooks`,
`request_integration_verification`, `read_connection_status`,
`repair_hook_contract`, `repair_managed_configuration`에서 고릅니다.
`run_optional_active_diagnostics`는 필수 remediation 밖에 둡니다. 렌더러는 산문에서
step을 추론하지 않습니다. Project/configuration trust는 hook-source activation과
분리합니다.

## 효과와 응답 처리 경로

실패 범주 자체는 상태 변경, 재시도 가능성, HTTP 또는 JSON-RPC 상태, CLI 종료
코드, API 응답 분기, 표시 문구를 정의하지 않습니다. 영향을 받는 메서드,
어댑터, 전송, CLI, 저장 효과 담당 문서는 이 범주 의미를 보존하면서 해당 표시
형태를 정의합니다.

이웃 담당 문서:

- 공개 API 응답 분기와 공개 코드: [API 오류](api/errors.md).
- 영속 기록 계약: [저장 기록](storage-records.md).
- Runtime Home 배치와 Registry 이전 stderr fallback 경계:
  [런타임 경계](runtime-boundaries.md).
- 메서드별 저장 효과와 효과 없음 분기: [저장 효과](storage-effects.md).
- 관리 검증 및 repair 명령: [관리 CLI](admin-cli.md).
