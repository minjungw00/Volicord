# 테스트 전략

테스트는 소유자가 정의한 현재 동작을 보호합니다. 테스트가 제품 계약을 만들거나 삭제된
표면을 보존하거나 더 넓은 지원 주장을 정당화하지 않습니다.

## 가장 좁은 계층 선택

| 테스트 계층 | 용도 |
|---|---|
| Unit test | 순수 파싱, 정규 인코딩, 폐쇄 값, 정책 결정. |
| Crate integration test | 어댑터 경계, Store 읽기/쓰기, 프로세스 동작, 엄격한 저장 레코드 거부. |
| Conformance test | 공개 교차 메서드 결과, 오류 범주, replay, 효과, projection. |
| Release-integrity 테스트 | Volicord target, 버전, 패키지, checksum, workflow, 실제 바이너리 스모크 불변조건. |
| 문서 검사 | 소유자 경로, 링크, 용어, 언어 동등성, 예시, 생성 소스 drift. |

일회용 Runtime Home과 Product Repository를 사용합니다. fixture는 최소이며 typed여야
합니다. fixture는 parser 또는 구현 동작만 증명하며 실제 Codex 설치의 행동이나 플랫폼
지원을 증명하지 않습니다.

`volicord-test-process`는 저장소 테스트와 스모크 하네스가 공유하는 한도 있는 자식
프로세스 실행을 담당합니다. `volicord-platform-process`가 담당하는 프로세스 그룹,
Windows Job Object, 비차단 파이프 primitive를 조합하며 해당 OS 구현을 복제하지
않습니다. 제품 MCP 감독 정책, 프로토콜 프레이밍, lifecycle 진행 상태, 진단은 계속
`volicord-cli`가 담당합니다.

## 고정된 MCP 명세 입력

`tests/conformance/mcp-spec/`은 결정론적인 MCP 적합성 작업에 필요한 최소한의 버전별
upstream schema와 라이선스 저작자 표시를 담당합니다. 이 경로의 manifest는 확정된
초기화 기반 revision과 pre-release 전용 입력을 분리하고, 전체 upstream commit을
고정하며, handshake family와 release 분류를 기록하고, 모든 로컬 artifact의 checksum을
관리하며, 검토된 `production_supported`와 `pre_release_only` 사실을 기록합니다.
프로덕션 지원에는 릴리스되었고 pre-release 전용이 아닌 항목, 고정 artifact,
`ProtocolRegistry`의 정확히 일치하는 profile이 필요합니다. 추적 중인 pre-release
항목은 프로덕션 지원 밖에 둡니다.

`cargo run -p xtask -- mcp-spec-check`는 오프라인 무결성 gate입니다. 네트워크 접근 없이
manifest를 parsing하고, 분류와 변경 불가능한 참조를 검증하며, schema 존재 여부,
schema family, 저작자 표시, checksum뿐 아니라 릴리스 상태이면서
`production_supported=true`인 manifest 항목과 컴파일된 프로덕션 protocol profile의
정확한 집합 일치를 확인합니다. 보고서는 전체 고정 revision, 프로덕션 지원 revision,
추적 중인 pre-release revision 수를 결정론적으로 제공합니다.
`cargo run -p xtask -- mcp-spec-sync`는 명시적으로 실행하는 유지보수 작업입니다. 기록된
release가 고정 commit으로 해석되는지 확인하고 임시 디렉터리에 내려받으면서 검토된 지원
metadata를 보존한 다음, 후보 전체를 검증한 뒤에만 fixture를 교체합니다.
일반 build와 test는 네트워크를 사용하는 sync 경로를 실행하지 않습니다.

실행 가능한 wire 적합성은 독립 gate인
`cargo test -p volicord-mcp --test protocol_conformance`가 확인합니다. 일반 runner는
`ProtocolRegistry::production().oldest_to_newest()`를 직접 순회하므로 프로덕션 profile을
추가하면 같은 집중 case가 자동으로 matrix에 들어갑니다. Manifest는 검토된 upstream 및
지원 사실만 기록하며 실행 가능한 테스트가 수행되었는지는 기록하지 않습니다. Runner는
별도의 conformance revision 배열이나 revision별 coverage boolean을 소유하지 않으며
registry 직접 순회가 matrix를 정합니다.

## 필수 경계 coverage

해당하는 지속 테스트는 다음을 다룹니다.

- 알 수 없는 멤버, 중복 키, 잘못된 폐쇄 값, 손상된 저장 owner record
- 정책, replay, ticket 무효화, mutation 전에 일어나는 구조적 거부
- authority event나 `state_version` 증가가 없는 read-only branch
- 하나의 원자적 성공 mutation과 정확한 replay 동작
- owner-defined corrupt-data failure로 라우팅되는 current-contract 불일치
- Runtime Home의 `Absent`, `Ready`, `Incompatible`, `Corrupt` 검사, singleton과
  installation metadata를 포함한 같은 상위 directory의 staged creation, 정확한
  manifest, 불투명 publication provenance 및 relation fact, 공개 전 각 실패 지점의
  정리, Unix와 native Windows에서 소유자 하나와 관찰 전용 패자만 만드는 no-replace
  동시 공개, token-backed rollback 재검증, 소유권 상실 및 managed-host 소비 시 보존,
  관련 없는 replacement 안전성, 기존 비호환 상태의 변경되지 않은 bytes와 timestamp
- 행 없음 또는 비적격 operation result에서 유지되는
  `OPERATION_RESULT_UNAVAILABLE`
- 숨은 context의 MCP 거부와 CLI-only UserAction resolution
- 권위 있는 MCP runtime-session source 분리, milestone ordering, 현재 revision,
  프로젝트 binding, diagnostics 비권위성
- 숨은 launcher 구성의 정확한 형태, 현재 entry drift 거절, 사용하지 않은 lease의
  결정적인 정리, 원자적인 일회성 lease 소비, replay·만료·Connection·revision·fingerprint
  불일치 거절, process 환경과 관계없이 공개 stdio가 `manual_cli`로 남는다는 증명
- 릴리스 상태이면서 `production_supported=true`인 manifest 항목과 프로덕션 protocol
  profile 사이의 정확한 revision 집합 일치 및 프로덕션 지원에서 추적 중인
  pre-release generation 제외
- `AgentToolId` wire 이름의 유일성과 왕복 파싱, 정규 registry identity의 정확한 일치,
  mode별 가용성, CLI·MCP runtime·Store가 함께 사용하는 컴파일 시점
  `ManagedHostRoundTrip` 결합
- `ProtocolRegistry`에서 직접 선택한 모든 프로덕션 profile의 독립된 `initialize`,
  initialized notification, `tools/list`, 고정 schema 검증, 필수 도구, 지정 왕복 identity,
  revision별 정의와 결과 projection, profile에 따른 작업 단계 batch 허용 또는 거절,
  잘못된 lifecycle 동작, 초기화 batch 거절, EOF/종료
- exact-match와 counter-offer 협상, profile별 initialize capability, batching,
  `tools/list`, `tools/call` wire projection
- 프로덕션 protocol registry에서 파생하지 않고 독립적으로 고정하며 revision 적합성을
  대신하지 않는 Codex host fixture, 정확한 `CodexMcpTurnMetadata`,
  `CodexCommandHooks`, `CodexMcpCallableNames` profile coverage, source별 상관관계,
  명시적 server/raw/callable fixture 일치, 완전한 raw name 투영, 정규화 충돌 및 모순되는
  role 거부, 정확한 catalog 역방향 조회, 추가 field와 한도 check, checksum 일치, typed
  host-tool 및 server-namespace/catalog-derived-exact routing, probe target·workflow
  control·unrelated known role의 완전한 coverage, 현재 Guard probe pre/post fixture 전달,
  nonterminal begin/get/status self-observation, 알 수 없는 same-server callable의 정확한 ID
  주장 처리, foreign-server 제외, 생성 matcher와 catalog의 일치, 엄격한 matcher drift
  거부, CLI conformance evidence와 실제 `managed_host` 관찰의 분리
- 읽을 수 있는 read-only Registry 및 프로젝트 database의 불변 MCP preflight 증거,
  선택 database의 변하지 않은 row count와 modification time, 항상 `not_checked`인 쓰기
  가능성, `last_active_verification` 아래에만 저장되는 활성 쓰기 증거, 검증 뒤에도
  변하지 않는 preflight 증거, 활성 timestamp/source, 일회용 conformance 상태,
  concise/verbose/JSON 일치, 결합된 증거 형태의 엄격한 거부
- lifecycle별 진단 구성 및 Store API, 변경할 수 없는 occurrence 삽입, 완전한 current key
  digest와 영속 ID 검증, current snapshot identity 불변성, 해소와 재활성화,
  active/reportable filtering, 명시적 report seed와 한도가 있는 lifecycle-aware 정확한 cause
  chain, occurrence/active/resolved lookup projection, severity와 독립적인 lookup-status process
  exit, typed diagnostic code와 한도 및 민감정보 제거가 적용된 fact, 결정론적 root,
  dependency에 따른 `Blocked` check, 선택한 Connection 또는 정확한 lookup 보고서 각각의
  동등한 사람용 및 JSON projection
- Guard manifest의 exact shape와 owner binding, hash가 없는 policy command와 hash에
  결속된 runtime command의 구분, wrapper/file drift, 플랫폼 독립적인 script executable
  기대값, 현재 definition hook hash, 바뀌지 않은 manifest의 관찰 보존, 바뀐 definition의
  무효화, 현재 소유권의 hook 관찰, 이전 event 제외, 서로 다른 부재/malformed/unknown
  callable/상관관계 불일치 acquisition stage, 검증에서 same-server non-probe tool 제외,
  payload 없이 한도가 있는 callable evidence
- Unknown, setup review, 현재 definition 관찰, policy 관리, 호출별 bypass, 명시적
  disabled를 포함한 정확한 `HookActivationState` 근거 우선순위와 합성 trusted 상태 부재
- `project_trust`를 독립적으로 유지하면서 configured, reload, hook review/unknown,
  managed MCP observation, Guard verification, complete, failed를 지나는
  `IntegrationActivationState` 전환
- ambient와 correlated Guard check의 분리, 즉 ambient passed와 correlated failed의
  동시 표현, attempt가 없을 때 ambient pending, correlated complete, repair-required를
  pending으로 projection하지 않음, 더 오래된 proof가 더 최신 failed attempt를 숨기지
  않음
- concise, verbose, JSON 사이 Guard report parity, 최상위 Guard runtime session과
  verification ID, managed/Guard session role의 정규 중복 제거, 복구 가능한 failed
  check의 `action_required` 집계, 모든 typed repair reason 및 acquisition stage의 안정적인
  code 직접 mapping
- 단일 `IntegrationActivationPlan`, 고정 semantic step ID, 서로 구분된
  initiator/executor, `codex_chat` 요청 channel, 완료 check, root-finding 순서,
  prerequisite 위상 순서와 중복·cycle·알 수 없는 prerequisite·불일치 metadata·최상위
  nested tool·필수 diagnostic-only step의 엄격한 거부
- Reload, hook review, 사용자 수준 요청 하나, status의 init 출력 개수와 순서,
  `Required next steps` block 하나, 정확한 개수와 단복수 표현, current-status suffix,
  typed repair-required plan, 분리된 optional active diagnostics
- Runtime Home 준비, Store 복구 준비, Runtime Home rename 뒤 상위 directory 동기화,
  publication read-back, manifest 검증 단계, 모든 관리 hook/rule/guidance 교체 뒤,
  Codex 구성 교체 전후, integration revision commit 전, rollback 중의 transactional
  init fault injection, 새 상태와 기존 상태의 정확한 복원, 두 승자 순서와 이후 패자 실패를
  포함하는 동기화 경계가 있는 전체 init 경쟁, 동시 외부 bytes 보존, 모든 setup
  publication 결과와 `planned`, `committed`, `preserved`, `rolled_back`,
  `partially_rolled_back` 보고 projection, 읽기 전용 dry-run 일치, replay idempotence,
  commit 뒤에만 activation
- 정규 요청, 모든 tagged workflow kind와 그 상태가 반환하는 정규 tool, nested
  list/begin/probe/status 순서, unavailable 경로, shell sleep/poll loop, same-turn 자동
  재시작, raw stdio, 직접 작성한 `_meta`, resource discovery를 proof로 쓰지 않는다는
  경계를 보존하는 생성 AGENTS, Codex rule, MCP server instruction
- 불변 semantic 좌표의 begin replay, 새 ID가 없는 terminal same-turn replay, 새 turn
  attempt, prompt 소유권, first-write-wins probe acknowledgement, 중복 begin concurrency
- 고정된 현재 Codex semantic 계약의 synchronous one-read observation policy, numeric
  version 분기 부재, TTL 대기 없는 누락 event 즉시 repair, 서로 다른
  payload/callable/verification/session/turn/tool-use repair reason, 불변 complete 및 repair
  terminal, 실제 새 좌표에 대한 retry-policy gate
- 결정적인 begin, probe 한 번, policy가 정한 status read 한 번, stop 순서와 sleep,
  반복 polling, 자동 same-turn retry의 명시적인 부재를 확인하는 생성 guidance
- 도달 가능한 모든 tagged variant, Store projection 하나에 대한 begin/probe/get 일치,
  모순된 state/tool 조합 거부, 모든 production MCP revision의 상태에 맞는 응답
- 적용된 setup, launch lease, managed MCP milestone, 같은 turn의 Guard prompt/pre/post
  검증, complete begin replay, 정확한 complete probe replay, activation complete, 일치하는
  bounded get까지 비관리 source로 대신하지 않고 다루는
  `crates/volicord-cli/tests/operational_host_e2e.rs`
- 안정적인 identity를 유지하는 반복 Guard 초기화와 관련 없는 repository content 보존
- Guard 관찰과 미기록 변경 suppression 결과
- Codex 구성 drift와 행동 probe 실패 보고
- 성공, stdin 전달, 0이 아닌 종료, 시간 초과, 결정론적 stdout/stderr truncation,
  동시 stream, 자손이 유지하는 pipe, stdin 쓰기 실패 후 정리, 반복 정리, 네이티브
  Unix 프로세스 그룹, 네이티브 Windows Job Object, 공백이 있는 경로와 인자, 명시적
  환경 추가·제거를 아우르는 재사용 가능한 한도 테스트 자식 실행

운영 상호운용성 coverage는 제한 안의 임의 version 문자열을 받고, initialize와 도구 목록
milestone을 실행하며, 필수 도구와 안전한 읽기 전용 호출, Guard artifact와 필수 phase 관찰,
session 소유권 및 integration revision 격리를 점검합니다.

## 릴리스 무결성과 선택적 호스트 smoke

오래 유지되는 릴리스 테스트 패키지는 `tests/release-integrity`입니다. 게시하는
Volicord target 다섯 개, 버전 일치, 기준 텍스트 바이트, 패키지와 archive 형태,
패키징한 binary identity, checksum 출력, 릴리스 workflow의 일반 빌드와 패키지
구조를 검증합니다.

`cargo run -p volicord-release-smoke -- --bin <path>`는 게시하지 않는 전용 플랫폼
공통 실제 바이너리 스모크 패키지를 호출합니다. 폐기 가능한 Git Product Repository,
Runtime Home, Codex home과 안정적인 테스트 소유 Codex fixture 실행 파일을 만들고
공개 `volicord init`을 실행한 뒤 Serde로 JSON을 역직렬화하며, 공개
`volicord mcp serve --connection <connection-id>`를 시작합니다. Protocol
registry의 선호 서버 리비전을 요청하고 initialization과 `tools/list`를 완료한 뒤,
정규 `AgentToolId` identity로 대표 공개 도구를 검사하고 사용자 전용 resolution
operation이 없음을 증명합니다. Codex fixture는 스모크 실행 파일을 플랫폼별 Codex
파일 이름으로 복사한 것입니다. `--version`만 성공하며 한도가 있는 의미 기반 fixture
버전 `codex-fixture 0.145.0-test`를 보고합니다.

이 패키지는 릴리스 전용 orchestration, transcript 검증, fixture 설정, 결과 보고를
담당합니다. Lifecycle 및 수집 한도를 `volicord-test-process`에 전달하며, 이 공유
경계는 재사용 가능한 한도 자식 실행, 프로세스 트리 정리, 직접 자식 회수를 담당합니다.

`.github/actions/volicord-release-smoke`는 재사용 workflow 호출 경계입니다. 일반 CI는
로컬 debug `volicord` 바이너리를 빌드한 뒤 action을 정확히 한 번 호출합니다. 네이티브
릴리스 matrix의 각 항목도 artifact staging 전에 같은 action을 정확히 한 번 호출하여
이미 빌드한 정확한 Linux, macOS, Windows 바이너리를 전달합니다. Release-integrity
테스트는 완전한 shell 명령 형식 대신 YAML 의미를 기준으로 build, smoke, staging
순서, matrix target과 binary 참조, 정확히 한 번인 호출 수를 검증합니다. 이 프로세스는
공개 수동 전송이므로 `manual_cli`로 남습니다. 숨은 managed-host launcher를 호출하지
않으며 managed-host 증거를 제공하지 않습니다.

일반 릴리스 무결성 테스트는 Volicord 플랫폼 빌드와 패키지 artifact를 다룹니다. 운영
Codex 상호운용성 테스트는 [Agent Connection](../reference/agent-connection.md)이 정의한
관리 구성, MCP 초기화, 필수 도구, 안전한 도구 왕복, Guard 관찰, session 소유권,
revision 격리를 별도로 다룹니다.

실제 Codex 실행은 선택적인 운영 smoke입니다. 제한된 host version을 진단으로
보고할 수 있고 version이 바뀌면 관찰을 다시 수행할 수 있습니다. 결과는 해당 구성과
환경에서 관찰한 행동에만 적용되며 미래 host 동작, human identity, 런타임 권한을
성립시키지 않습니다. smoke 인프라 부재는 일반 Volicord 릴리스 점검을 차단하지 않습니다.

## 문서 검증

의미가 바뀐 문서 쌍은 영문/국문 의미 동등성을 요구합니다. 생성 계약 projection은
소스와 일치해야 합니다. 다음을 실행합니다.

```sh
cargo run -p xtask -- docs-check
git diff --check
```

그다음 변경에 맞는 구식 표면 targeted scan을 실행하고 diff에서 owner routing, 정확한
식별자, 경로, anchor, 저장소 위생을 확인합니다.

## Rust 검증

일반 workspace gate는 다음과 같습니다.

```sh
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all-targets --all-features
```

더 좁은 명령이 필요하면 이유와 실행하지 않은 workspace 검사를 인계에 기록합니다.
