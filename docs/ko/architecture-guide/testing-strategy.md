# 테스트 전략

테스트는 소유자가 정의한 현재 동작을 보호합니다. 테스트가 제품 계약을 만들거나 삭제된
표면을 보존하거나 더 넓은 지원 주장을 정당화하지 않습니다.

## 가장 좁은 계층 선택

| 테스트 계층 | 용도 |
|---|---|
| Unit test | 순수 파싱, 정규 인코딩, 폐쇄 값, 정책 결정. |
| Crate integration test | 어댑터 경계, Store 읽기/쓰기, 프로세스 동작, 엄격한 저장 레코드 거부. |
| Conformance test | 공개 교차 메서드 결과, 오류 범주, replay, 효과, projection. |
| Release-integrity 테스트 | Volicord target, 버전, 패키지, checksum, commit된 tree의 소스 번들, workflow, 실제 바이너리 스모크 불변조건. |
| 아키텍처 검사 | 워크스페이스 패키지 선언, 의존성 종류와 방향, 프로덕션과 테스트 지원 분리, Core 의존 계층 적격성. |
| 문서 검사 | 소유자 경로, 링크, 용어, 언어 동등성, 예시, 생성 소스 drift. |

`volicord-store`에서는 aggregate별 unit test를 자신이 보호하는 mutation 입력,
저장 검증과 적용, 읽기 projection, 엄격한 decoder 옆에 둡니다. Transaction,
replay 순서, rollback, 내구성, aggregate 간 저장 효과 테스트는
`CoreProjectStore` commit 경계에 둡니다. Assertion은 typed 결과와 관찰 가능한
저장 효과를 우선하며, 정규 SQL 담당 문서가 해당 byte를 현재 계약으로 정한
경우에만 완전한 SQL text를 직접 비교합니다.

Store 테스트는 모든 public Store-to-Core 또는 Store-to-service record 경계의 물리
영속화, transaction, 엄격한 row decoding을 담당합니다. 잘못된 물리 enum, JSON,
timestamp, Product Repository 경로, 빠진 값, 중복 column 사이의 모순을 주입하고
Store 소유 영속 데이터 손상을 확인합니다. Core와 서비스 테스트는 Store가 구성한
유효한 typed record를 사용해 의미 policy와 invariant failure를 검증합니다. 물리
decoder를 반복하거나 Store 손상을 구성하지 않습니다. 이 구분은 workflow policy,
Write Ticket, replay identity, reconciliation observation, UserAction과 그 밖의
Core 지향 record family 모두에 적용됩니다. Write Ticket 경계 테스트는
`StoredWriteTicket` accessor를 사용하면 compile에 성공하고 외부 코드가 struct
literal, 비공개 field 접근, destructuring을 시도하면 compile에 실패하는지
확인합니다. Compiler message text가 아니라 compile 결과를 검증합니다.

`volicord-core`에서는 재사용 가능한 의미 담당자 테스트를 그 테스트가 보호하는
`identity.rs`, `artifact.rs`, 집중 fact, projection, guidance, summary text,
Change Unit planning, Task policy 모듈, `continuity/`, `write_ticket/`,
`close_readiness/`, 집중 `error_boundary/` 모듈 옆에 둡니다. 순수 projection
테스트는 typed fact를 사용하며 Store handle을 받지 않습니다. Write Ticket
read-model 테스트는 정책을 검증하지 않고 typed ticket, Task,
workflow policy, UserAction resolution, 증거 취득과 Store 오류 전파를 다룹니다.
공유 현재 fact fixture에는 active 현재 유효성 평가에 쓰이는 비승인 Task 및
workflow-control 입력만 있습니다. 현재 유효성 테스트는 이 fact와 typed
`WriteTicketApprovalAssessment`를 받아 active, invalidated, consumed, revoked,
effective expiry 전이를 다루며 terminal record가 현재 fact를 읽기 전에 완결되고
평가된 모든 stored 상태가 필수 ticket ID를 유지함을 검증합니다. 과거/표시 선택
테스트는 `StoredWriteTicketEvaluation` 값만 받고 stored 상태 우선순위와 동률
해소를 담당합니다. Prepare Write 호환성 선택 테스트는 분류가 끝난 전체 후보 집합을 받고,
active 후보 없음, active지만 호환되지 않는 후보, 호환 ticket 정확히 하나, 호환
ticket 둘 이상, 호환 후보와 비호환 후보가 섞인 집합을 구분합니다. 이 테스트는
결정적인 모호성 identity 순서를 검증하되 그 순서를 권한으로 취급하지 않습니다.
승인 담당자 테스트는 원시 UserAction 권한 fact를 정규 승인 담당자에게만 전달하고,
그 비공개 현재 집합이 typed 발급 근거 또는 typed 영속 근거 평가를 만드는지
검증합니다. 이 정책 unit test는 요구사항 구성, 현재 민감 동작 승인 구성, Store가
검증한 영속 근거 평가, typed 의미 변경 사유, 여러 승인 참조, 현재 및 오래된 전체
resolution identity를 다룹니다. 소비자 경로를 호출하거나 그 경로의 coverage를
주장하지 않습니다.

Store 기반 교차 소비자 배선 적합성 suite는
`crates/volicord-core/src/write_ticket/tests/approval_consumer_conformance.rs`에
있으며 crate 비공개 메서드 통합 하네스에 연결됩니다. 공유 시나리오 표에는 현재
승인, 승인이 필요하지 않은 경우, 새로 필요해진 승인, 오래된 resolution, 변경된 승인
범위, ticket 만료, 소비, 철회, 호환되고 재사용 가능한 ticket 정확히 하나, 호환되고
재사용 가능한 ticket 여러 개를 만드는 원본 fact만 둡니다. 각 시나리오는 필요한
유효한 Task, Change Unit, UserAction 요청과 resolution, Write Ticket record를
영속화합니다. 실제 `CoreService::status`, `CoreService::check_close`,
`CoreService::prepare_write`, `CoreService::record_run` 경로를 호출하고 projection된
상태, Store 효과, admission 결과, Write Ticket 차단 사유 identity를 검증합니다.
Record Run은 앞선 Prepare Write 무효화가 자체 admission 평가를 대신하지 않도록 같은
원본 fact를 새로 구체화한 fixture에서 실행합니다. Fixture helper는 승인 참조를
비교하거나 resolution ID 집합을 구성하거나 현재 여부를 판단하거나 호환 ticket을
선택하거나 무효화 사유를 재현하지 않습니다. 호환 ticket이 여러 개면 status는
표시용으로 선택한 active summary 하나를 projection하되 영속 후보 집합은 계속 조회할
수 있고, 닫기 준비 상태는 현재 ticket마다 차단 사유 하나를 projection합니다.
Prepare Write는 재사용 없이 차단되며, Record Run은 요청이 소비 대상으로 명시한
ticket만 승인합니다.

Summary 테스트는 Store fixture나 정책 재평가 없이
`PlannedWriteTicket`과 평가 완료 stored 상태를 각각 변환합니다. 집중 service
테스트는 terminal 사전 평가, active에 한정한 현재 fact 취득, 선택 뒤 증거 읽기와
영속, 무효화, approval-dependent, dry-run, 실패 경로의 대표 사례를 검증합니다.
일반 compilation과 함수 signature는 reuse가 `ReusableStoredWriteTicket`을 요구하고,
admission이 `AdmissibleStoredWriteTicket`을 반환하며, terminal 상태가 이 경로에
들어갈 수 없음을 증명하는 일차 수단입니다. Production Write Ticket 모듈 lint는
panic 기반 domain narrowing을 거절합니다.
Mutation planning 테스트는 typed plan과 schema 담당 모듈의 정확한 field
accessor를 검증합니다. 그
밖의 담당자 테스트는 typed fact, policy 판단, retry 동작, 정확한 경계 매핑 하나를
검증합니다. 공개 메서드 조율, 응답 계열, replay, 커밋 효과 matrix는
`methods/tests/`에 둡니다. 메서드 통합 테스트가 재사용 담당자 로직의 유일한
coverage가 되면 안 됩니다. Write Ticket planning 테스트는 응답 metadata가 없는
typed 의미 validation 오류, 폐쇄형 발급·재사용·ticket 없음 계획 family, ID 없는
발급 draft, 불변 조건을 지키는 `WriteTicketPathScope`, 폐쇄형 구체화, 발급
plan에서 완전히 typed인 `WriteTicketInsert`를 파생하는 과정을 검증합니다. 재사용과
ticket 없음이 삽입을 만들 수 없음도 검증합니다. 공개 응답을 구성하거나 dry-run
intent를 의미 planning fact로 취급하지 않습니다. Prepare Write 메서드 테스트는
공개 오류 metadata, state-version이 있는 reference projection, durable ID 할당,
dry-run 발급 및 재사용 결과, ticket 없음 결과를 담당합니다. 또한 현재 공개되는
경로, 유효성, 만료, Task, Change Unit, 승인 근거 fact에 대해 발급 또는 재사용된
중첩 ticket, 최상위 결과 fact, typed planned 또는 stored 원본, Store mutation
입력, 다시 읽은 record를 비교합니다. Ticket 없음 테스트는 null ticket identity 및
reference field와 insertion 부재를 검증합니다. 승인 의존 메서드 시나리오는 Prepare
Write 선택이 원시 UserAction 권한 fact가 아니라 정규 typed 평가를 받는지도
검증합니다. Store 기반 Prepare Write 모호성
coverage는 호환되는 active ticket 여러 개를 영속화하고 실제 메서드 경로를 호출해
메서드 소유 차단 결과와 정렬된 후보 참조를 검증하며, 어느 후보도 재사용, 소비,
무효화, 선택되지 않았음을 확인합니다. Replay는 정확한 응답 coverage를 유지합니다.

Record Run도 소스 책임에 따라 이 구분을 적용합니다. 요청 및 fact 취득, 캡처 권한,
증거 관찰과 재사용, artifact 검증과 승격, typed mutation 계획, 의미 오류
variant, `RecordRunResultFacts` projection은
`crates/volicord-core/src/recording/tests/`에 둡니다. 닫기 근거 및 잔여 위험
coverage는 `close_readiness/tests/recording.rs`에, ticket 호환성, 승인, 소비,
무효과 거절 coverage는 `write_ticket/tests/record_run_admission.rs`에 둡니다.
집중된 typed mutation plan은 이 담당 시나리오와 Store commit 경계에서
검증합니다. 이 테스트는 typed reusable ticket과 일치하는 정확한 attempt 호환성
증명으로 admission에 진입하고 admissible ticket만 mutation planning과
consumption으로 전달합니다. 커밋된 product-write 시나리오는 Run row, Run의 ticket
effect payload, 소비된 ticket이 같은 admissible ticket identity를 사용함도
검증합니다. 승인 의존 admission 시나리오는 로컬에서 취득한 원시 권한을 정규 승인
담당자에게 직접 전달하고 반환된 typed 평가로만 현재 유효성을 검증합니다. 작은
`methods/tests/record_run.rs` suite에는 대표 요청 조율,
중립 실행 carrier와 공개 결과 field로의 변환, dry-run 및 state-version
metadata를 보존하는 의미 오류 routing, commit 및 무효과 대안, 증거 및 artifact
경로, ticket 및 stale-state 거절, rollback 전파, replay 일관성을 남깁니다. 중립
`OperationPlan` 테스트는 메서드 독립 실행 입력을 검증합니다. 완전한 도메인 정책
행렬은 공개 메서드 suite에 두지 않습니다.

닫기 준비 상태의 Write Ticket 테스트는 `StoredWriteTicketEvaluation` 값만 받고
active 및 terminal 평가 상태에서 차단 사유가 파생되는지 검증합니다. 원시
UserAction 권한 fact를 구성하거나 승인 policy 평가를 반복하지 않습니다.

`volicord-user-action-service`에서는 의미 검증, 정규 body와 identity 구성,
authority, lifecycle, materialization, 영속화 매핑, resolution, continuity,
neutral projection 동작을 unit test가 담당합니다. Core 테스트는 요청 조율,
생성된 식별자와 timestamp, replay, transaction 순서, 서비스 오류 매핑을
담당합니다. UserAction 중복 표현, 빠진 물리 값, 요청-resolution identity 또는
action-kind 불일치는 계속 Store 테스트에 둡니다.

Product Repository 경로 테스트도 같은 소유권 분리를 따릅니다.
`volicord-types`는 임시 directory 없이 어휘 값과 순수 관계를 테스트합니다.
`volicord-platform-fs`는 실제 일회용 directory로 기존 경로, 아직 없는 경로,
가장 가까운 기존 상위, 접근 불가 경로, link escape를 테스트합니다. Core
테스트는 플랫폼 관찰을 다시 구현하지 않고 typed 플랫폼 결과를 사용해 중립 운영
routing을 검증합니다. UserAction 서비스 테스트는 파일시스템을 사용하지 않습니다.
Adapter 테스트는 안정적인 operation 및 resource identity projection을 검증합니다.

`volicord-mcp-wire` 테스트는 정확한 MCP 직렬화, JSON-RPC envelope, 의미 descriptor,
discriminator 우선 중첩 union 선택, 필수 nullable 동작, branch local issue context, 결정적
issue 순서와 한도, typed canonical example, 결정적 input/output schema, descriptor 무결성을
담당합니다. 잘못됐거나 없는 discriminator가 선택되지 않은 branch field를 노출할 수 없고
같은 이름의 sibling field가 metadata를 주고받을 수 없음을 증명합니다. `volicord-mcp`
테스트는 registry, response 수준 selected variant와 canonical example, compact 인자 오류,
정확한 decode parity, 출력, 한도 있는 discovery projection에서 같은 validator tree를
소비합니다. Descriptor에는 유효하지만 정확한 request decoder가 거절한 값은 사용자 field
issue가 아니라 Core 전 내부 diagnostic 실패입니다.
`volicord-types` 테스트는 neutral 공개 schema만 담당합니다. 담당자 간 coverage는 공개
메서드 schema에 MCP 전용 구조가 없고 MCP adapter가 neutral Core 운영 실패를 현재 wire
오류로 변환하는지 확인합니다. Conformance package도 같은 descriptor와 example을 직접
소비하며 JSON fixture나 schema metadata를 복사하지 않습니다. MCP 의미 case는 모든
canonical 값이 정확히 검증 및 decode되는지 요구하고 선언된 각 discriminator를 변형해
branch 추측 없는 branch local 거절을 증명합니다.

일회용 Runtime Home과 Product Repository를 사용합니다. fixture는 최소이며 typed여야
합니다. fixture는 parser 또는 구현 동작만 증명하며 실제 Codex 설치의 행동이나 플랫폼
지원을 증명하지 않습니다.

`volicord-test-process`는 저장소 테스트와 스모크 하네스가 공유하는 한도 있는 자식
프로세스 실행을 담당합니다. `volicord-platform-process`가 담당하는 프로세스 그룹,
Windows Job Object, 비차단 파이프 primitive를 조합하며 해당 OS 구현을 복제하지
않습니다. 제품 MCP 감독 정책, 프로토콜 프레이밍, lifecycle 진행 상태, 진단은 계속
`volicord-cli`가 담당합니다.

## 변경된 파일의 담당 경로 지정

`cargo run -p xtask -- owner-route --changed`는 Git 변경 경로에서 저장소 유지보수
범위를 도출합니다. `--base <revision>`을 명시하면 commit된 변경 series와 현재
working tree를 함께 포함합니다. 패키지 소속은 Cargo metadata에서, 유지 문서 및
대응 언어 identity는 `docs/doc-index.yaml`에서 가져옵니다. 이 담당 원본에 없는
연결만 검증되는 `docs/owner-routing.yaml` catalog에 둡니다.

경로 지정 테스트는 폐기 가능한 Git 저장소를 사용합니다. Rust 패키지, 대응 문서,
저장소 지침, workflow 파일, 알 수 없는 경로, dirty working tree, 명시적 기준
revision, 안정적인 정렬, 사람용/JSON 일치, 읽기 전용 working tree 경계를
검사합니다. 테스트는 두 번째 워크스페이스 패키지 목록을 두거나 산문을 검색해
담당 경로를 찾지 않습니다.

`cargo run -p xtask -- validate focused --base <revision>`은 그 경로를 중간 명령
계획으로 바꿉니다. 변경된 워크스페이스 패키지와 직접 적용되는 문서, 아키텍처,
MCP specification, release/workflow, 위생 점검만 선택합니다. 알 수 없는 경로는 담당
경로 사전 점검에서 실패합니다. 루트 workspace manifest나 lockfile 변경은 해당 루트
파일을 담당하는 패키지가 없어도 아키텍처와 workspace 컴파일 점검을 추가합니다.
집중 계획은 정확한 workspace aggregate를 계획하지 않습니다.

Commit 범위 사전 점검은 평범한 형식, scope가 있는 형식, breaking 형식의 Conventional
Commit header를 해석합니다. `test` commit의 프로덕션 패키지 manifest는 commit 전후의
의미 기반 TOML을 비교하고 test target과 개발 의존성 변경만 허용합니다. 연관된
lockfile은 같은 commit의 나머지 경로가 모두 test 전용일 때만 허용합니다.

`cargo run -p xtask -- validate final --base <revision>`은 series의 모든 commit이
준비된 뒤 완전한 저장소 정책을 계획합니다. 각 자식 프로세스는 실행 중에 stdout과
stderr를 `target/volicord-validation/<run-id>/` 아래 파일에 직접 기록합니다.
Runner는 정확한 호출, timestamp, exit code를 담는 기계 판독 summary와 명령별
결과를 checkpoint하므로 terminal handle을 잃어도 완료된 결과를 확인할 수
있습니다.

검증 runner 테스트는 두 번째 검증 엔진을 호출하지 않고 주입한 명령 결과를
사용합니다. 집중 계획, 변경 패키지 선택, 문서 경로, 명령 실행 전 run 찾기 정보,
동시 활성 record, 오래 남는 log와 복구, exit code 보존, 생략 명령, 사람용/JSON
분류 일치, aggregate 재시도 한도, 같은 패키지 분해, 두 번째 실패에서 바뀌거나 서로
다른 패키지, 모호한 실패 출력, 정확한 전체 summary를 검사합니다.

## 워크스페이스 아키텍처 검증

`cargo run -p xtask -- architecture-check`는 Cargo가 보고하는 현재 워크스페이스
패키지와 내부 일반, 개발, 빌드 의존 간선을 루트 `Cargo.toml`의
`workspace.metadata.architecture.packages` 아래의 패키지 항목과 비교합니다. 이
검사는 담당 원본에 없는 워크스페이스 패키지, Cargo에 없는 담당 원본 항목,
해결되지 않는 허용 목록 대상, 출발 패키지의 종류별 허용 목록 밖으로 향하는 모든
간선을 거부합니다. 또한 프로덕션 패키지에서 테스트 지원 패키지로 향하는 일반
또는 빌드 의존성, Core 쪽에서 어댑터나 표현 패키지로 향하는 의존성, 필수
UserAction 서비스·Core·공유 타입·Store 경계 위반, 일반·빌드 의존 그래프의
순환을 독립적으로 거부합니다. 의미 기반 wire-family 규칙은 출발점이 일치하는 adapter
또는 검증 도구나 테스트가 아니면 `*-wire` 담당자 의존성을 거부합니다.

집중 검증기 테스트는 중립적인 합성 패키지와 그룹 이름으로 유효한 현재
메타데이터, 종류별 허용되지 않은 간선, 등록되지 않은 패키지, 잘못된 의존 종류,
프로덕션과 테스트 지원의 분리, Core와 어댑터의 독립성, 일치하는 adapter의 wire
접근, 관련 없는 adapter 및 foundational package의 wire 접근 거부, 순환을 다룹니다.
별도 테스트는 같은 검증기를 현재 Cargo 워크스페이스와 담당 원본에 실행합니다.
테스트에 워크스페이스 패키지 그래프의 두 번째 사본을 두지 않습니다. 아키텍처
규칙은 현재 그래프에 직접 적용하며 패키지, 스키마, 프로토콜 버전으로 선택하지
않습니다.

현재 워크스페이스 coverage는 `volicord-user-action-service`에 전용 책임 항목이
있고, 일반 의존성으로 `volicord-types`와 `volicord-store`만, 개발 의존성으로
`volicord-test-support`만 허용하는지도 확인합니다. Core, CLI, MCP, presentation
의존성은 아키텍처 gate를 실패시킵니다.

Core의 일반 허용 목록은 typed 호출 범위 저장소 관찰을 위해
`volicord-platform-fs`를 허용합니다. 공유 타입과 UserAction 서비스 그룹은 이
의존성을 허용하지 않으므로 활성 파일시스템 관찰이 의미 값이나 검증으로 이동할 수
없습니다.

Core 테스트는 typed local-user authority 또는 검증된 Agent Connection authority로
호스트 중립 요청을 구성합니다. CLI, MCP, application, host-contract 담당자는 자신의
명령 문법, 설치 및 시작 구성, host별 값 검증, 경로, 렌더링을 테스트합니다. Adapter
테스트는 동등한 adapter 연산과 직접 Core 연산을 typed domain result 경계에서
비교합니다. 아키텍처 집행은 Cargo 패키지 graph를 검사하고, 동작 테스트는 공개 typed
경계와 담당자 출력을 실행합니다.

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
`cargo test -p volicord-mcp --test protocol_conformance`가 확인합니다. 이 테스트가 유일한
전체 profile harness입니다. 일반 runner는
`ProtocolRegistry::production().oldest_to_newest()`를 직접 순회하고 지원되는 모든
profile에 동일하게 적용되는 scenario를 실행합니다. Assertion은 result carrier 형식,
structured content, `isError`, output schema, annotation, title, `_meta`, initialize field,
client-capability 형태, committed-result 복구를 profile의 의미 기반 capability에서
도출합니다. 별도 registry 및 projection 테스트는 capability data가 고유하고 완전한지
확인하고, 지원되지 않는 식별자를 대체 없이 거부하며, projection이 revision 순서로
동작을 선택하지 않음을 입증합니다. Manifest는 검토된 upstream 및 지원 사실만 기록하며
실행 가능한 테스트가 수행되었는지는 기록하지 않습니다. Runner는 별도의 conformance
revision 배열이나 revision별 coverage boolean을 소유하지 않으며 registry 직접 순회가
matrix를 정합니다.

## 필수 경계 coverage

해당하는 지속 테스트는 다음을 다룹니다.

- 알 수 없는 멤버, 중복 키, 잘못된 폐쇄 값, 손상된 저장 owner record
- 정책, replay, ticket 무효화, mutation 전에 일어나는 구조적 거부
- 정규 메서드 선언에서 파생한 정확한 공개 응답 계열 coverage. 여기에는 선언되지
  않은 미리보기 분기에 대한 decoder, 스키마, 설명자, Core 분기, adapter 거부가
  포함됩니다.
- authority event나 `state_version` 증가가 없는 read-only branch
- 하나의 원자적 성공 mutation과 정확한 replay 동작
- owner-defined corrupt-data failure로 라우팅되는 current-contract 불일치
- Runtime Home의 `Absent`, `Ready`, `Incompatible`, `Corrupt` 검사, singleton과
  installation metadata를 포함한 같은 상위 directory의 staged creation, 정확한
  manifest, 불투명 publication provenance 및 relation fact, 공개 전 각 실패 지점의
  정리, 정규 home별 공유·배타 변경 승인, 동시 shared writer, 같은 lock 영역에서의
  배타 충돌, lexical 및 symlink alias 통합, 서로 다른 home의 독립성, 즉시 및 한도 있는
  획득, 영속 coordination 파일의 비소유성, 공유·배타 lease의 프로세스 종료 시 해제,
  네이티브 Unix와 Windows OS lock 동작, 빌린 permit의 target/mode 결속,
  소유자 하나만 만드는 no-replace 공개, token-backed rollback 재검증, 소유권 상실 및 managed-host 소비 시 보존,
  효과 전 재귀 실패, 일부 제거 또는 분류 불가 제거, 제거 확인 뒤 상위 directory 동기화
  실패, terminal 재시도 동작, 확인 오류와 rollback fact를 함께 담는 composite 실패,
  관련 없는 replacement 안전성, 기존 비호환 상태의 변경되지 않은 bytes와 timestamp
- 완전한 같은 상위 directory의 staging carrier를 통한 진단 최초 생성, 정확한 검증이
  끝날 때까지 유지되는 최종 경로 부재, 서로 다른 모든 session을 보존하는 결정론적 동시
  `SharedWriter` 공개, 패배 및 공개 전 실패에서 각 호출이 만든 파일로 한정한 정리,
  외부에서 만든 유효하지 않은 최종 파일 보존, 상위 directory 동기화 실패를 가로질러
  유지되는 공개 확인 효과, 기존 유효하지 않은 최종 파일의 정확한 거부, read-only
  접근의 staging 무시, Unix 최종 permission, 플랫폼 공개 primitive의 네이티브 coverage
- 행 없음 또는 비적격 operation result에서 유지되는
  `OPERATION_RESULT_UNAVAILABLE`
- 숨은 context의 MCP 거부와 CLI-only UserAction resolution
- 모든 폐쇄형 workflow 거부 코드, typed 현재 mode/phase와 수신 요청, allowed 대안,
  authoritative workflow, retryability, 정확한 recovery action key, 변경되지 않은 effect count와 state
  version, pending User Channel presentation, 정확한 command 생성, phase 전환의 no-write-
  ticket fact
- workflow rejection 관찰 총수와 final-answer surface 총수를 비교하는 agent evaluation
  observation. 관찰한 거부가 하나라도 final answer에서 빠지면 기본 task가 성공했더라도
  결과는 incomplete입니다.
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
- 지원 revision의 정확한 선택, 지원되지 않는 식별자의 명시적 거부, capability 기반
  initialize, batching, `tools/list`, `tools/call` wire projection
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
- typed Hook 경로 안전성 평가의 긍정 근거가 되는 정확한 현재 Codex hook 구성, Git root
  기반 dispatch, 모든 필수 phase wrapper, owner 및 managed-command binding, policy hash,
  host output, hash, permission, verified·failed·not-recorded·not-checked·not-applicable 차원의
  구분, policy·owner·root-resolution 위반의 정확한 실패 reason, 한도가 있고 결정적인 근거,
  실패 및 불완전 phase 집계, 입력 순서와 무관한 JSON
- Unknown, setup review, 현재 definition 관찰, policy 관리, 호출별 bypass, 명시적
  disabled를 포함한 정확한 `HookActivationState` 근거 우선순위와 합성 trusted 상태 부재
- `project_trust`를 독립적으로 유지하면서 configured, reload, hook review/unknown,
  managed MCP observation, Guard verification, complete, failed를 지나는
  `IntegrationActivationState` 전환
- complete, host reload, hook review/unknown, MCP observation, Guard verification,
  failed, hook disabled, policy-managed hook, invocation-bypassed hook 상태에서 선택한
  status와 list의 사람용 label parity 및 두 JSON projection의 안정적인 밑줄 표기 유지
- all-passed, pending, blocked, failed, not-applicable, mixed 입력을 위한 단일 Connection
  count projection, concise `Passed`, `Blocked`, `Pending`, `Failed` field, 같은 list 어휘와
  순서, verbose not-applicable 개수
- 증거 없음, 전체 통과, Registry 실패, 정확한 프로젝트 쓰기 실패 ID, initialize,
  지정된 safe tool, shutdown 실패, host compatibility 실패를 위한 공유 concise/verbose
  활성 검증 projection, 성공한 production revision 5개를 oldest-to-newest 순서로 compact하게
  표시, 실패 row의 완전한 lifecycle 및 diagnostic fact 펼침, 성공한 host row의 compact 표시,
  정확히 영속된 증거 시각과 사람이 읽는 source, Store 쓰기 가능성 집계, malformed 근거의
  엄격한 거부, contradictory 근거의 펼침, 완전한 JSON을 보존하는 사람용/JSON fact parity,
  concise 출력의 내부 ID 생략
- ambient와 correlated Guard check의 분리, 즉 ambient passed와 correlated failed의
  동시 표현, attempt가 없을 때 ambient pending, correlated complete, repair-required를
  pending으로 projection하지 않음, 더 오래된 proof가 더 최신 failed attempt를 숨기지
  않음
- awaiting-probe, awaiting-observation, complete, repair-required, no-run 상태마다 typed 영속
  lifecycle 사실에서 상관관계 Guard 증거 시각 선택, 엄격한 시간 순서와 attempt/proof
  identity 불일치 실패, 보고서 시각이 달라지는 읽기 전용 status 평가에서도 증거 시각과
  detail 유지 및 Store 비변경, 목록 평가 시각 분리, verbose/JSON timestamp 일치
- 보고서 시각이 달라지는 반복 읽기 전용 status 평가에서도 마지막 활성 검증의 증거 시각,
  source, 집계 결과, Store 쓰기 가능성을 유지하고 concise, verbose, JSON에 같은 증거
  시각을 표시하며 Store나 filesystem을 변경하지 않음
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
  init fault injection, 새 상태와 기존 상태의 정확한 복원, 두 최초 획득 순서, 성공 뒤 해제,
  rollback 완료 뒤 해제, mutation 없는 typed busy와 lease를 획득한 dry run, 해제 뒤 새
  시도를 결정적으로 검증하는 전체 init 경합, lease 보유 중 외부 publisher가 최종 경로를
  만들 때 오래된 plan 중단, 동시 외부 bytes 보존, 모든 setup
  publication 결과와 `planned`, `committed`, `preserved`, `rolled_back`,
  `partially_rolled_back` 보고 projection, 읽기 전용 dry-run 일치, replay idempotence,
  동기화된 제거·동기화되지 않은 제거·불완전한 제거·정책 보존·소유권 상실을 구분하는
  typed JSON 및 사람용 출력, 효과를 인식하는 Project Home 정리, commit 뒤에만 activation
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
- 정확한 pre-tool 기준선, post-tool 결과, 결정론적 net delta, 정확한 host 상관관계,
  `open`, `complete`, `unavailable` 상태를 포함하는 호출 범위 Guard 저장소 관찰
- 완전한 delta를 대상으로 한 expected-write 일치, 기존 dirty 상태 attribution 경계,
  unmatched-delta Unrecorded Change, 실제 변경과 분리된 unavailable 진단
- Codex 구성 drift와 행동 probe 실패 보고
- 성공, stdin 전달, 0이 아닌 종료, 시간 초과, 결정론적 stdout/stderr truncation,
  동시 stream, 자손이 유지하는 pipe, stdin 쓰기 실패 후 정리, 반복 정리, 네이티브
  Unix 프로세스 그룹, 네이티브 Windows Job Object, 공백이 있는 경로와 인자, 명시적
  환경 추가·제거를 아우르는 재사용 가능한 한도 테스트 자식 실행

## Runtime Home 변경 승인 회귀 테스트

변경 승인 테스트는 [Runtime Boundaries](../reference/runtime-boundaries.md), 집중된 Store
소유자 문서, 해당 CLI·MCP·Guard 소유자 문서가 정의한 동작을 조합합니다. 이 안내서는
coverage 구성을 설명할 뿐 해당 계약을 다시 정의하지 않습니다.

재사용 가능한 자식 프로세스 프로토콜은 즉시 획득과 한도 있는 획득 모두에 대해 다음
lock matrix 전체를 실행합니다.

| 첫 번째 프로세스 | 두 번째 프로세스 | 필수 관찰 결과 |
|---|---|---|
| `SharedWriter` | `SharedWriter` | 두 프로세스 모두 승인을 획득합니다. |
| `SharedWriter` | `ExclusiveSetup` | setup이 busy이거나 한도 있는 대기를 소진합니다. |
| `ExclusiveSetup` | `SharedWriter` | writer가 busy이거나 한도 있는 대기를 소진합니다. |
| `ExclusiveSetup` | `ExclusiveSetup` | 두 번째 setup이 busy이거나 한도 있는 대기를 소진합니다. |

같은 프로토콜은 두 mode 모두에서 정상 반환, 오류 반환, panic, 강제 프로세스 종료 뒤
OS handle이 해제되는지 증명합니다. 네이티브 runner는 각 플랫폼의 OS lock을 실제로
실행합니다. 교차 컴파일만 한 결과는 별도로 보고하며 lock 실행으로 간주하지 않습니다.
유지되는 네이티브 matrix에는 Linux, Windows, macOS가 포함됩니다. WSL2의 영향을 받는
Unix case는 네이티브 Linux 분기를 검증할 때 `WSL_DISTRO_NAME`을 제거합니다.

Setup 경합 coverage는 경과 시간 sleep 대신 barrier, channel, 획득 신호, setup fault
point를 사용합니다. 새 Runtime Home 공개 matrix는 공개 뒤와 이후 Store, Product
Repository, Codex 구성, rollback 지점에서 일시 정지합니다. 실제 외부 writer는 setup이
보고하거나 rollback할 때까지 어떤 변경도 남기지 않아야 하며, 해제 뒤 재시도는 그
결과인 현재 상태 또는 부재 상태만 관찰해야 합니다. 기존 Runtime Home checkpoint
case는 setup Store commit 뒤 checkpoint 전에 멈춥니다. 외부 writer는 그 checkpoint에
들어갈 수 없고, setup은 자체 snapshot만 복원하며, writer가 해제 뒤 받아들여진
재시도는 계속 남아야 합니다. 경합상 어느 프로세스든 먼저 획득할 수 있는 경우 두 획득
순서를 모두 다룹니다.

대표 writer-domain matrix는 실제 프로젝트와 Connection 명령, 공개 Core commit,
artifact staging, evidence capture, inbox resolution, change reconciliation, policy
application, managed launch 및 runtime-session 관찰, `tools/list`와 verification
milestone, integration-verification event, Guard hook 수집, 진단 영속화, 운영 finding을
조합합니다. 각 case는 소유자 operation 전에 typed busy/no-effect projection을 확인하고,
해당하는 row, 파일, `state_version`, timestamp, finding, event, receipt가 정확히
변하지 않았는지 확인한 다음 lease 해제 뒤 같은 operation의 성공적인 재시도를
검증합니다. 일반 dummy write로 소유자 operation을 대신할 수 없습니다.

집중된 inbox resolution coverage는 project database를 사용할 수 없게 한 상태에서
`ExclusiveSetup`을 유지하고, Registry 조회, project Store open, 후보 planning, 진단 생성
전에 typed setup-busy가 반환됨을 증명합니다. Lease 해제 뒤에는 같은 명령이 정상적으로
재시도됩니다. Choice 및 evidence-observation case는 승인된 단일 snapshot의 정규 후보
검증, 잘못된 선택의 no-effect, text와 JSON projection, 정확한 immutable replay, 동시
Core 재검증, best-effort 진단을 실행합니다. 이 case를 네이티브 Windows에서 실행하면
승인 전에 열린 SQLite handle이 setup 교체나 rollback을 막지 않음도 함께 증명합니다.

Alias coverage는 대기 choice, evidence observation, 변경 불가능한 replay, change
reconciliation, 추가 승인 Core operation을 lexical Runtime Home alias로 실행합니다.
Unix에서는 symlink alias도 실행합니다. 이 case는 Registry project 하나, 같은 typed
Store/Core identity와 일관된 UserAction snapshot, 한 번만 commit된 resolution, 승인된
home의 diagnostic correlation을 확인합니다. 별도 negative case는 다른 Runtime Home,
project, verification basis가 계속 권한을 얻지 못하는지 검증합니다. 네이티브 Windows
runner는 지원하는 alias case를 실제 실행하며 compile-only 검증은 별도로 보고합니다.

MCP lifecycle 테스트는 runtime-session 생성 전에 setup을 시작하고, Core 효과 전에
변경 call을 거부하며, 승인을 얻을 수 없을 때 관찰을 영속화하는 read도 no-effect로
유지합니다. 유휴 server는 `SharedWriter`를 계속 보유하면 안 되며 승인은 operation마다
획득합니다. Guard record-profile 테스트는 협력적인 host 활동의 계속 진행을 보존하면서
거부된 hook을 관찰 성공 phase로 세지 않고 이후 hook이 정상적으로 기록되는지
증명합니다. Connection list와 status, 진단 lookup, project list/current, authority export,
MCP preflight를 포함한 소유자 정의 read-only 명령은 writer lease 없이 계속 사용할 수
있어야 하며 Runtime Home byte, row, `state_version`, modification time을 보존해야
합니다.

Binary 수준의 맥락별 출력 coverage는 하나의 폐기 가능한 Runtime Home과 Product
Repository에서 `volicord status`, compact/verbose/JSON doctor, 사람용/JSON 개인정보
footprint를 반복 실행합니다. 활성 Task 없음의 명시적 상태, 적용 가능한 사람용 field,
완전한 구조화 report, section으로 나눈 개인정보 주장, 정확한 terminal hygiene,
변하지 않은 Store effect counter와 authority snapshot을 검증합니다. 집중 개인정보
coverage는 두 mode의 stdout을 바이트로 직접 수집합니다. 유효한 UTF-8과 JSON을 요구하고,
모든 범주 문자열 및 출력 범위 스칼라를 정규 typed 정의와 사람용 section에 대조하며,
`when diagnostics are present`를 포함한 완전한 diagnostics 문장을 보호합니다. 또한 각
주장이 정확히 한 번 나타나는지와 사람용 출력의 마지막 newline이 하나인지 확인하고 tab이나
다른 control 문자 손상을 거부합니다. 같은 fixture는 각 명령 전후의 모든 영속 entry,
파일 바이트, modification time을 대조합니다. 여기에는 Registry, diagnostics 및 project
Store, Product Repository, 관리 구성, installation profile, Hook 파일, 영속 verification
report가 포함됩니다. 순수 renderer 테스트는 ready, warning, action-required 또는 failed
Doctor 상태, 현재 모든 check의 의도적인 제목, 결정적인 의미 그룹 및 check 순서, 정상 명령
집계, CLI 또는 MCP 명령 부재, 구성 경로와 해석된 PATH의 불일치, 모순된 사실의 엄격한
거부, 선택적인 host detection, 구조화된 비성공 detail, 긴 경로, 빈 collection, terminal
hygiene, 완전한 JSON check 동등성을 다룹니다. Doctor remediation coverage는 명시적인
finding과 direct candidate를 순수 report-finalization 경계에 제공합니다. Finding action 포함,
direct action 포함, command 보강,
required-over-recommended urgency, 결정적인 priority 및 code ordering, 충돌 거부,
required/recommended의 엄격한 분할, JSON과 사람용 projection 전체의 primary action 하나,
action 없는 warning의 정확한 문구, 렌더링 중 Store 또는 Product Repository mutation이
없음을 검증합니다.

Build 표시 coverage는 exact profile, class-only, profile metadata 누락을 구성하고 공유 typed
담당자에서 나온 Version과 Doctor의 사람용 문구를 대조합니다. 또한 JSON이 병렬 표시 label
field 없이 정확한 `class_only` 기계 값을 유지하는지 증명합니다. 집중 Doctor renderer
coverage는 `not recorded`, `not applicable`, `not checked`, 실제 빈 collection을 나타내는
`none`을 구별합니다. 간결한 build 평가가 전체 provenance를 반복하지 않고, 검증된 현재
Hook artifact 6개는 세 안전성 차원과 개수 하나로 축약하며, 실패한 Hook artifact는 경로,
phase, source, reason, installation ID를 펼치는지도 증명합니다. Connection verbose coverage는
검증된 artifact의 source 개수와 failed, not-recorded, not-checked, mixed, 한도 도달 근거를
명시적인 누락 표시와 함께 failure-first 순서로 펼치는 동작을 별도로 증명합니다. JSON은
typed report와 byte-for-byte로 동일하게 유지하고, 모든 사람용 branch는 tab 없이 마지막
newline 하나만 가지며, rendering은 Store, filesystem, terminal 상태를 바꾸지 않습니다.

Doctor binary fixture는 정규 현재 artifact를 한 번 초기화한 다음 setup을 다시 실행하지
않고 compact, verbose, JSON Doctor mode를 호출합니다. 검증된 경로 안전성 평가가 있을 때만
`guard_files` check가 통과하는지, 병렬 field가 없는 엄격한 state object 하나, verbose의
명시적 의미 label, compact에서 성공 detail의 부재를 요구합니다. 또한 Registry, project 및
diagnostics Store, 관리 구성, installation profile, Hook 파일, Product Repository의 bytes와
modification time이 바뀌지 않았는지 확인합니다. Registry coverage는 JSON의
`storage_profile`이 구조화된 현재 `StorageManifest`인지 요구하고 verbose field와 capability
목록을 확인합니다. 또한 잘못된 영속 manifest JSON을 주입해 raw 문자열 fallback 없이
엄격하게 실패하는지 증명합니다. 모든 branch는 마지막 newline을 정확히 하나만 유지하고
fixture setup 뒤 폐기 가능한 Runtime Home의 read-only 상태를 보존합니다.

Connection-list lifecycle coverage는 setup, managed-session, complete 단계에서 사용 가능한
각 membership을 선택한 status와 비교하고, 영속 활성 검증이 `action_required`인 채여도
현재 `complete`를 유지하는지 확인하며, complete membership과 대기 중인 membership의
독립성을 증명합니다. 손상 및 unavailable case는 유효한 행을 숨기지 않으면서 등록
metadata, 영속 활성 근거, 관리 구성, project Store 실패를 다룹니다.
Filter case는 선택하지 않은 membership을 평가하지 않는지 증명합니다. JSON과 사람용
projection은 typed summary와 invocation timestamp 하나를 공유하고, 탭 없는 구조화된
경로, compact primary action, verbose에서만 보이는 ID, revision, not-applicable 개수,
모든 step, 한도가 있는 문제 세부사항을 확인합니다. Status와 list를 반복해서 읽어도
Registry, project 및 diagnostic Store, 관리 configuration, Product Repository 내용,
Runtime Home 보고서와 timestamp가 보존되는지도 검증합니다.

운영 상호운용성 coverage는 제한 안의 임의 version 문자열을 받고, initialize와 도구 목록
milestone을 실행하며, 필수 도구와 안전한 읽기 전용 호출, Guard artifact와 필수 phase 관찰,
session 소유권 및 integration revision 격리를 점검합니다.

## 릴리스 무결성과 선택적 호스트 smoke

오래 유지되는 릴리스 테스트 패키지는 `tests/release-integrity`입니다. 게시하는
Volicord target 다섯 개, 버전 일치, 기준 텍스트 바이트, 패키지와 archive 형태,
패키징한 binary identity, checksum 출력, 릴리스 workflow의 일반 빌드와 패키지
구조를 검증합니다.

소스 번들 구현은
`cargo run -p xtask -- source-bundle --output <path>` 하나입니다. 기본값은 `HEAD`이며
추적 중인 index 또는 working tree에 변경이 있으면 거부합니다. 릴리스나 CI 점검에서
다른 정확한 commit이 필요하면 `--commit <commit>`을 사용합니다. 이 명령은 선택한
tree와 blob을 Git에서 읽고, 메타데이터를 정규화한 ZIP을 결정적인 순서로 작성한 뒤
출력을 게시하기 전에 모든 entry를 검증합니다. 일반 파일, 실행 파일, directory,
symlink의 mode는 Git tree에서 가져옵니다. 포함 대상을 정할 때 filesystem을 순회하지
않으므로 Git metadata, untracked 파일, runtime 출력, 기존 untracked archive는 번들
밖에 있습니다.

`cargo run -p xtask -- source-bundle-validate --input <path>`는 ZIP을 독립적으로 다시
열고 선택한 Git tree와 경로, 파일 형식, mode, link target, 내용을 대조합니다. 집중
테스트는 폐기 가능한 Git 저장소에서 추적 상태 변경, untracked 내용, 일반 파일,
실행 파일, symlink, 안전하지 않거나 중복된 ZIP 경로, 압축 해제, 바이트 단위 반복
생성을 다룹니다. 현재 tree 전체 테스트는 같은 구현을 이 저장소에 적용합니다. 일반
CI와 태그 릴리스 게시는 정규 생성 명령을 호출합니다.

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

일반 릴리스 무결성 테스트는 Volicord 플랫폼 빌드, 패키지 artifact, 소스 번들
workflow 경로를 다룹니다. 운영 Codex 상호운용성 테스트는
[Agent Connection](../reference/agent-connection.md)이 정의한 관리 구성, MCP 초기화,
필수 도구, 안전한 도구 왕복, Guard 관찰, session 소유권, revision 격리를 별도로
다룹니다.

실제 Codex 실행은 선택적인 운영 smoke입니다. 제한된 host version을 진단으로
보고할 수 있고 version이 바뀌면 관찰을 다시 수행할 수 있습니다. 결과는 해당 구성과
환경에서 관찰한 행동에만 적용되며 미래 host 동작, human identity, 런타임 권한을
성립시키지 않습니다. smoke 인프라 부재는 일반 Volicord 릴리스 점검을 차단하지 않습니다.

## 문서 검증

의미가 바뀐 문서 쌍은 영문/국문 의미 동등성을 요구합니다. 생성 계약 projection은
소스와 일치해야 합니다. 집중 profile은 담당 경로에서 문서와 diff 점검을
선택합니다. 그다음 diff에서 담당 경로, 정확한 식별자, 경로, 앵커, 저장소 위생을
확인합니다.

## Rust 검증

중간 Rust 변경에는 집중 profile을 사용하고 완전한 commit series 뒤에는 최종
profile을 한 번 사용합니다.

```sh
cargo run -p xtask -- validate focused --base <revision>
cargo run -p xtask -- validate final --base <revision>
```

오래 남는 summary는 더 좁은 명령, 정확한 aggregate, 한도가 있는 재시도나 분해,
생략한 모든 명령과 그 이유를 기록합니다.
