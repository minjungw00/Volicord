# 테스트 전략

이 가이드는 Volicord Rust 변경에서 어떤 구현 테스트 계층을 사용할지
설명합니다. 테스트는 담당 문서가 정의한 사실을 검증합니다. 테스트가 제품
계약을 정의하거나, 보안을 증명하거나, QA를 완료하거나, 닫기 준비 상태를
확립하거나, 제품 수락을 기록하지 않습니다.

정확한 동작은 [참조 색인](../reference/README.md)을 봅니다. 크레이트별
소스 방향은 [코드베이스 둘러보기](codebase-tour.md)에서 찾습니다.
워크스페이스 형태와 의존 경계 개요는 [구현 아키텍처](architecture.md)를
봅니다. 정확한 Cargo 의존 간선은 워크스페이스와 각 크레이트의
`Cargo.toml` 매니페스트에서 확인합니다. 변경 작업 흐름은
[구현 가이드](change-guide.md)를 봅니다. 문서 명령 예시 검증, 용어 역할 검증,
한영 링크 일치, 검증 보고 경계는
[검증](../maintain/validation.md) 정책을 따릅니다.

## 테스트 계층

| 계층 | 실제 패키지 또는 경로 | 사용할 때 | 사용하면 안 되는 것 |
|---|---|---|---|
| 모듈 단위 테스트 | [`crates/volicord-types/src/lib.rs`](../../../crates/volicord-types/src/lib.rs), [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs), [`crates/volicord-store/src/core_pipeline.rs`](../../../crates/volicord-store/src/core_pipeline.rs), [`crates/volicord-store/src/sqlite.rs`](../../../crates/volicord-store/src/sqlite.rs) 등 구현 모듈 안의 테스트. | 로컬 도우미 동작, 타입 지정 파싱, 정규 해시, 정책 도우미, Store 트랜잭션 경계, 스키마 검증, 설정 작업 흐름 분기, 연결 선택/출력 도우미, guard 통합 계획/감사 도우미, 코드 가까이의 작은 분기 점검. | 계층 간 수락 테스트나 제품 계약 출처. |
| 플랫폼 파일시스템 파사드 테스트 | `volicord-platform-fs` 패키지의 [`crates/volicord-platform-fs/src/lib.rs`](../../../crates/volicord-platform-fs/src/lib.rs)에 함께 있는 테스트. | 플랫폼 고유 이름 공간 연산을 둘러싼 안전한 결과 분류와 대상별 파사드 동작. | 관리 파일 소유권 정책, 호출자 복구 동작, 파일시스템 전체 이식성 주장, 보안 증명. |
| Core 메서드 테스트 | `volicord-core` 패키지의 [`crates/volicord-core/src/methods/tests/`](../../../crates/volicord-core/src/methods/tests/). `status.rs`, `intake.rs`, `prepare_write.rs` 같은 메서드별 파일로 나뉩니다. | 메서드 계획, `CoreService`를 통한 공유 사전 점검, dry-run/효과 없음/커밋 분기, 재실행, 상태 버전 효과, 아티팩트 스테이징 구분, 메서드에 보이는 Store 효과. | MCP 전송 범위나 전체 공개 동작 권위. |
| 저장소 DDL 계약 테스트 | `volicord-store` 패키지의 `storage_ddl_contract` 대상인 [`crates/volicord-store/tests/storage_ddl_contract.rs`](../../../crates/volicord-store/tests/storage_ddl_contract.rs). | Storage DDL 담당 문서와 구현 사이의 정합성, 기준 SQL 원본, 스키마 초기화와 검증, 테이블, 열, 제약, 인덱스, 유지되는 트리거. | 일반 저장 효과 동작이나 런타임 적합성. |
| 관리 CLI 바이너리 테스트 | `volicord-cli` 패키지의 `binary_admin` 대상인 [`crates/volicord-cli/tests/binary_admin.rs`](../../../crates/volicord-cli/tests/binary_admin.rs). | `volicord` 바이너리와 설정, 프로젝트 감지, 연결 관리, `volicord inbox ...`, 쓰기 없는 dry-run, 호스트 상태 검증, 연결 프로젝트 생명주기, 생성 guard 출력 변경, 잔류 효과, 호스트 설정 쓰기, 사전 점검 실패, doctor 진단, 명령줄 오류처럼 바이너리를 통해 관찰해야 하는 동작. | 공개 API 메서드 동작. |
| Guard 명령 테스트 | `volicord-cli` 패키지의 `guard_command` 대상인 [`crates/volicord-cli/tests/guard_command.rs`](../../../crates/volicord-cli/tests/guard_command.rs). | `session-start`, `pre-tool`, `post-tool`, `prompt-capture`, `stop`의 guard 훅 생명주기, 기록된 관찰, `expected write` 일치, 쓰기 티켓 범위, 호스트 고유 렌더링, 프롬프트 캡처 명령, guard 생명주기 픽스처. | 보안 증명, 사용자 승인 기록, 제품 수락 기록, Core 메서드 테스트 대체물. |
| MCP 전송 바이너리 테스트 | `volicord-cli` 패키지의 `mcp_transport` 대상인 [`crates/volicord-cli/tests/mcp_transport.rs`](../../../crates/volicord-cli/tests/mcp_transport.rs). | `volicord mcp` 하위 명령, 도움말/버전, `--check`, 표준 입출력 프레이밍, JSON-RPC 동작, 재연결, 응답 래핑. | Core 메서드 의미. |
| 로컬 HTTP 전송 테스트 | `volicord-cli` 패키지의 `serve_transport` 대상인 [`crates/volicord-cli/tests/serve_transport.rs`](../../../crates/volicord-cli/tests/serve_transport.rs). | `volicord serve --transport local-http` 프로세스 경로, 루프백 리스너 시작, 토큰과 Origin 점검, HTTP 세션, 방어 헤더, 로컬 HTTP 전송을 통한 MCP 요청 처리. | 일반 MCP 메서드 테스트나 보안 증명. |
| 명시적으로 실행하는 실제 호스트 스모크 테스트 | `volicord-cli` 패키지의 `live_host_smoke` 대상인 [`crates/volicord-cli/tests/live_host_smoke.rs`](../../../crates/volicord-cli/tests/live_host_smoke.rs). | 해당 호스트에 맞게 준비된 환경에서 설치된 Codex 또는 Claude Code 실행 파일을 명시적으로 점검할 때. 설정 점검은 `VOLICORD_RUN_*_SMOKE=1`, 대화형 판단 왕복은 `VOLICORD_RUN_*_JUDGMENT_SMOKE=1`을 사용하며 모든 실제 점검은 기본적으로 무시됩니다. | 기본 워크스페이스 테스트 신호, 이식 가능한 호스트 적합성, 호스트 신뢰, 자격 증명이나 네트워크 가용성, 보안 증명. |
| MCP 통합 테스트 | `volicord-integration-tests` 패키지의 `mcp_connection` 대상인 [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs). | MCP, Core, Store, Agent Connection 바인딩, 작업 범주 파생, 도구 노출, 재실행 맥락 바인딩, MCP에서 보이는 저장소 효과 없음 분기. | 집중 메서드 테스트나 참조 담당 문서의 대체물. |
| 공개 계약 스냅샷 테스트 | `volicord-integration-tests` 패키지의 `public_contract_snapshots` 대상인 [`tests/integration/public_contract_snapshots.rs`](../../../tests/integration/public_contract_snapshots.rs). | 생성된 API 요청 스키마와 MCP 도구 계약 스냅샷이 현재 소스에서 생성한 계약과 어긋나는지 점검합니다. | 생성 스냅샷 직접 편집, 의미 기준 참조 검토, 공개 계약이 올바르다는 증명. |
| 적합성 구현 테스트 | `volicord-conformance-tests` 패키지의 `baseline` 대상인 [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs). | Core 쪽 API를 통한 기준 범위 교차 메서드 시나리오. 재실행, 쓰기 티켓, 아티팩트, 판단, 닫기 준비 상태, 오류 처리 경로, 손상 처리 등을 포함합니다. | 제품 수락, 보안 증명, 닫기 준비 상태, 또는 제품 규칙의 유일한 출처. |
| 공유 테스트 지원 | `volicord-test-support` 패키지의 [`crates/volicord-test-support/src/lib.rs`](../../../crates/volicord-test-support/src/lib.rs). | 폐기 가능한 Runtime Home 픽스처, 등록된 프로젝트와 Agent Connection 설정, 요청 빌더, Store 검사 도우미, 공유 픽스처 구성. | 프로덕션 동작이나 오래 유지될 Runtime Home. |
| CLI 통합 테스트 지원 | [`crates/volicord-cli/tests/support/`](../../../crates/volicord-cli/tests/support/). | `binary_admin`, `guard_command`, `mcp_transport`, `serve_transport`가 재사용하는 바이너리 픽스처, 가짜 호스트와 MCP 프로세스, guard 생명주기 픽스처, JSON 및 단언 도우미. | 제품 계약 출처나 오래 유지될 런타임 상태. |
| 문서 유지보수 도구 테스트 | `xtask` 패키지의 [`xtask/tests/docs_check.rs`](../../../xtask/tests/docs_check.rs). | 읽기 전용 문서 검증기, 메타데이터 파싱, 한영 대응 범위, 로컬 링크와 앵커 점검, 용어 경로와 역할 점검, 명령 예시 검증, 공개 언어 점검, 임시 픽스처 동작. | 의미 번역 검토, 기술 정확성 검토, 제품 계약 출처. |

## 픽스처와 지원 구조

공유 픽스처 구조는 테스트 전략에 속합니다. `volicord-test-support`는 Core,
적합성, 통합 테스트가 사용하는 폐기 가능한 Runtime Home 픽스처, 등록된 프로젝트와
Agent Connection 설정, 요청 빌더, Store 검사 도우미, 통제된 손상 도우미, 공유
단언을 담당합니다. 이 픽스처들은 구현 검증을 돕지만 프로덕션 런타임 상태나 계약
담당 문서가 아닙니다.

`crates/volicord-cli/tests/support/`는 바이너리 실행, 가짜 호스트와 MCP 프로세스,
guard 생명주기 설정, JSON 파싱, 재사용 단언을 위한 CLI 통합 도우미를 담당합니다.
`crates/volicord-cli/tests/fixtures/host_contracts/` 아래의 호스트 계약 픽스처는 호스트
어댑터와 guard 생명주기 테스트를 지원합니다. 도우미로 검증하는 사실도 해당 참조
담당 문서로 연결해야 합니다.

## 명시적으로 실행하는 실제 호스트 스모크 테스트

`live_host_smoke`는 일반 Cargo 테스트 대상이고 내부의 네 개의 실제 호스트 점검에
`#[ignore]`가 붙어 있습니다. 따라서 일반 워크스페이스 테스트 실행은 이 점검들을
무시된 항목으로 보고합니다. 호스트 실행 파일이 설치되어 있고 해당 선택 변수를 설정한
환경에서 호스트 하나의 점검만 실행합니다.

```sh
VOLICORD_RUN_CODEX_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_smoke_is_opt_in -- --ignored --nocapture
VOLICORD_RUN_CLAUDE_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_smoke_is_opt_in -- --ignored --nocapture
VOLICORD_RUN_CODEX_JUDGMENT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_judgment_round_trip_is_opt_in -- --ignored --nocapture
VOLICORD_RUN_CLAUDE_JUDGMENT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_judgment_round_trip_is_opt_in -- --ignored --nocapture
```

판단 변형은 사람 참여형 점검입니다. 폐기 가능한 Runtime Home과 Product Repository를
만들고 선택한 호스트를 설정한 뒤, 쓰기를 하지 말라는 초기 지시와 함께 설치된 호스트를
대화형으로 실행합니다. 실행자의 일반 호스트 인증 환경을 재사용하며 픽스처의 격리된
Runtime Home으로 자격 증명을 복사하지 않습니다. 호스트가 요구하면 운영자가 프로젝트나
MCP 항목을 승인하고, 호스트 고유 MCP elicitation UI에서 직접 답을 선택하고, 상태 보고가
끝난 뒤 호스트를 종료해야 합니다.

판단 변형이 통과하면 표식 `Task`와 판단 생성, `mcp_elicitation_user_channel` 근거의
호스트 고유 프롬프트/응답 기록, 그에 따른 Task 상태 전환, 권한 이벤트, 내용 없는 해당
세션 진단을 검증한 것입니다. Judgment에는 고정된 route option 두 개가 있습니다. 사람이
하나를 선택하면 에이전트는 기본 간결한 Judgment 결과를 소비하고, 해당 option에 매핑된
정확한 요약 표식을 가진 Product Repository 비쓰기 `shaping_update` Run을 기록해야 합니다.
테스트 하네스는 저장된 `selected_option_id`와 최신 Run을 모두 읽고 매핑, kind, 빈 변경 경로,
비쓰기 관찰이 일치하는지 확인합니다. 고유 elicitation을 사용할 수 없으면 테스트 하네스는 대기 판단이
`volicord inbox`에 보이는지 확인하고 정확한 `volicord inbox answer` 명령을 출력한 뒤,
대체 경로를 고유 프롬프트 성공으로 취급하지 않고 점검을 실패시킵니다. 운영자는 복구에
그 명령을 사용할 수 있지만, 그렇게 해도 실패한 실제 고유 프롬프트 점검이 통과 결과로
바뀌지는 않습니다.

유지되는 Codex 또는 Claude Code 판단 경로를 지원한다고 명시하는 릴리스를 게시하기 전에
수동 릴리스 검증 체크리스트에서 릴리스 후보를 대상으로 해당 판단 변형을 실행하고 호스트
버전, Volicord `build_id`, 통과/실패 결과를 보존해야 합니다. 호스트, 인증 환경, 고유
elicitation 표면을 사용할 수 없으면 통과한 왕복이 아니라 건너뛴 검증으로 보고합니다.

명시적으로 선택한 점검은 선택 변수나 호스트 실행 파일을 사용할 수 없으면
실패합니다. 통과 결과는 설치된 호스트와 로컬 테스트 환경에서 스모크 테스트가 관찰한
단언만 확인합니다. 이식 가능한 호스트 동작, 호스트 신뢰나 승인, 자격 증명이나 네트워크
가용성, 보안 집행, 일반 제품 정확성을 증명하지 않습니다.

## 생성 출력과 문서 검증

생성 출력 변경 점검은 생성되거나 소스에서 파생된 저장소 산출물이 현재 소스와 계속 맞는지
확인합니다. `public_contract_snapshots`는 생성된 API 요청 스키마와 MCP 도구 계약
스냅샷을 점검합니다. CLI 바이너리와 guard 테스트는 지원되는 테스트 경로에 나타나는
생성 호스트 및 guard 출력을 점검합니다. 불일치는 소스 변경, 담당 문서, 검증 기준,
재생성 단계 중 하나를 검토해야 한다는 뜻입니다. 올바름을 증명하지는 않습니다.

유지 문서에는 `cargo run -p xtask -- docs-check`가 저장소의 구조 점검입니다.
이 명령은 `docs/doc-index.yaml`, 유지 경로, 링크와 앵커, 한영 로컬 링크 일치,
용어 담당 경로와 역할, 명령 예시 형태, 공개 언어 점검을 확인합니다. 이것은 문서와
공개 언어 검증이며, 의미 기준 한영 검토, 기술 정확성 검토, 참조 담당 문서 검토,
제품 적합성을 대체하지 않습니다.

## 변경 영역별 검증 지도

코드베이스 둘러보기나 아키텍처 문서로 영향을 받는 크레이트나 문서를 찾은 뒤
이 지도를 사용합니다. 이 표는 고려할 만한 점검을 이름 붙이는 것이며, 작은
편집마다 나열된 모든 테스트를 실행해야 한다는 규칙이 아닙니다.

| 변경 영역 | 보통 먼저 보는 코드 또는 문서 | 먼저 고려할 점검 | 필요할 때 더할 점검 |
|---|---|---|---|
| 아키텍처 가이드, 문서 경로, 메타데이터, 링크, 용어 | `docs/en/`, `docs/ko/`, `docs/doc-index.yaml`, `docs/terminology-map.yaml`; 검증기 동작이 바뀌면 `xtask`. | `cargo run -p xtask -- docs-check`, 사람이 하는 의미 일치, 담당 경로, 용어 검토. | 결정적 docs-check 규칙을 추가하거나 바꾸면 `xtask` 테스트. |
| 공개 스키마, 공유 요청/결과 타입, 값 집합, 식별자, 요청 해시 | `crates/volicord-types/src/`와 적용되는 참조 담당 문서. | `volicord-types` 단위 테스트. | 메서드 계획이 바뀌면 Core 메서드 테스트, 도구 스키마나 노출이 바뀌면 `public_contract_snapshots` 또는 MCP 통합 테스트, 유지 문서가 바뀌면 docs-check. |
| 플랫폼 파일시스템 파사드 또는 어댑터 관리 조건부 파일 교체 | `crates/volicord-platform-fs/src/lib.rs`, `crates/volicord-cli/src/guard_integration/files.rs` 같은 호출 어댑터, 적용되는 CLI/런타임/시스템 담당 문서. | `volicord-platform-fs`와 호출 모듈 단위 테스트. | 운영체제 고유 코드가 바뀌면 대상별 컴파일 또는 테스트, 관리 결과가 바이너리에 보이면 `binary_admin`, 유지 담당 문서나 아키텍처 가이드가 바뀌면 `docs-check`. |
| 공개 메서드 동작, Core 파이프라인 동작, 정책 도우미, 재실행, 효과 분기 | `crates/volicord-core/src/pipeline.rs`, `crates/volicord-core/src/methods/`, `crates/volicord-core/src/policy/`. | Core의 함께 있는 단위 테스트와 `crates/volicord-core/src/methods/tests/` 아래의 메서드별 파일. | 교차 메서드 기준 시나리오는 `tests/conformance/baseline.rs`, 어댑터에 보이는 맥락이나 도구 노출은 `tests/integration/mcp_connection.rs`. |
| Store DDL, 기준 SQL, 지속성 도우미, 트랜잭션 경계, 저장 효과, 아티팩트 저장소 | `crates/volicord-store/src/`, [`crates/volicord-store/tests/storage_ddl_contract.rs`](../../../crates/volicord-store/tests/storage_ddl_contract.rs), 저장소 참조 담당 문서. | Store의 함께 있는 단위 테스트. Storage DDL, 기준 SQL, 스키마 검증 변경에는 `cargo test -p volicord-store --test storage_ddl_contract`. | 공개 메서드에서 보이는 저장 동작이 바뀌면 Core 메서드, 적합성, MCP 통합 테스트. |
| MCP 시작, 표준 입출력 또는 로컬 HTTP 전송, 도구 목록, `tools/call`, 프로젝트 선택, Agent Connection 호출 맥락 | `crates/volicord-mcp/src/`, `crates/volicord-cli/tests/mcp_transport.rs`, `crates/volicord-cli/tests/serve_transport.rs`, `tests/integration/mcp_connection.rs`. | `volicord-mcp` 단위 테스트와 바뀐 전송 경로에 맞는 `mcp_transport` 또는 `serve_transport`. | 생성 API 또는 MCP 도구 스키마가 바뀌면 `public_contract_snapshots`, MCP를 통해 Core/Store 동작을 관찰해야 하면 `mcp_connection`, MCP 문서가 바뀌면 docs-check. |
| 설정 작업 흐름 동작과 출력 | `crates/volicord-cli/src/setup_command.rs`, `setup_command/`, `doctor_command.rs`, `crates/volicord-cli/tests/binary_admin.rs`. | 작업 흐름 분기와 렌더링은 설정 모듈 테스트로 확인합니다. 바이너리를 통해 관찰해야 하는 설정 동작은 `binary_admin`으로 확인합니다. | 부트스트랩, 레지스트리, 검사, 스키마 초기화, 설치 프로필 지속성이 바뀌면 Store 테스트를 추가합니다. 설정 문서가 바뀌면 docs-check를 실행합니다. |
| 연결 프로비저닝, 상태, 검증, 출력 | `crates/volicord-cli/src/connection_command.rs`, `connection_command/`, `crates/volicord-store/src/bootstrap.rs`, `agent_connections.rs`, `crates/volicord-cli/tests/binary_admin.rs`. | 연결 명령 모듈 테스트와 `binary_admin`. | 프로세스나 사전 점검 동작이 바뀌면 `mcp_transport`, MCP/Core/Store 동작을 MCP로 관찰해야 하면 `tests/integration/mcp_connection.rs`, CLI 문서가 바뀌면 docs-check. |
| Guard 통합 파일, 역량 기록, 감사 정보 | `crates/volicord-cli/src/guard_integration/`, 연결 상태/출력 코드, `doctor_command.rs`, `binary_admin`의 guard 적용 초기화/상태 테스트. | Guard 통합 모듈 테스트와 생성 guard 출력 변경을 다루는 `binary_admin`의 초기화/상태 사례. | 생명주기 관찰이 생성된 역량 정보를 사용하면 `guard_command`, guard CLI 문서가 바뀌면 docs-check. |
| Guard 훅 생명주기 동작과 호스트 고유 렌더링 | `crates/volicord-cli/src/guard_command.rs`, `guard_command/`, `crates/volicord-cli/tests/guard_command.rs`. | `guard_command` 테스트와 함께 있는 파싱/렌더링 테스트. | 훅 경로가 담당 문서가 정의한 Core 또는 Store 동작에 의존하면 Core 메서드, 적합성, 저장소 테스트. |
| 호스트 설정 어댑터 | `crates/volicord-cli/src/host_integration/`, 특히 `host_integration/codex/`, `host_integration/claude_code/`, `config_edit.rs`, `contracts.rs`, `generic.rs`, `verification.rs`. | 호스트 어댑터 모듈 테스트와 `binary_admin`. | 호스트 고유 훅 출력이 바뀌면 `guard_command`, 시작 또는 사전 점검 동작이 바뀌면 `mcp_transport`. |
| 적합성 시나리오나 공유 픽스처 동작 | `tests/conformance/baseline.rs`, `crates/volicord-test-support/src/lib.rs`, CLI 통합 픽스처는 `crates/volicord-cli/tests/support/`. | 먼저 그 동작의 집중 크레이트/단위 테스트, 그다음 영향을 받는 적합성 또는 CLI 시나리오. | 픽스처 동작이 다른 계층의 관찰 결과를 바꾸면 소비하는 통합 테스트나 메서드 테스트. |

## 오래 유지될 계약 테스트와 일회성 감사

[검증](../maintain/validation.md)에서 오래 유지될 테스트와 정리 전용 감사를
구분하는 전체 원칙을 확인합니다. 구현 테스트는 제거 이력이 아니라 현재 지원되는
형태를 검증해야 합니다.

현재 테스트에서 다음과 같은 작성 방식을 확인할 수 있습니다.

- `crates/volicord-cli/tests/binary_admin.rs`의
  `binary_help_options_match_supported_contracts`는 현재 CLI 도움말의 옵션 허용
  목록을 검증합니다.
- `crates/volicord-store/tests/storage_ddl_contract.rs`의
  `initial_schemas_satisfy_connection_storage_contract`는 현재 스키마 구조를
  검증합니다.
- `tests/integration/mcp_connection.rs`의
  `public_mcp_arguments_reject_internal_envelope_and_invocation_fields`는 공개 MCP
  스키마 경계를 검증합니다.
- `xtask/tests/docs_check.rs`의 `reports_required_terminology_role_failure`와
  `accepts_supported_volicord_shell_command_examples`는 현재 문서 검증 규칙을
  보호합니다.

이 테스트들은 구현 또는 유지보수 경계를 보호합니다. 검증하는 제품 사실은 계속
집중 참조 담당 문서가 정의합니다.

## 경계를 보여 주는 테스트

일부 테스트는 아키텍처 경계를 이해하는 데 특히 유용합니다.

- `tool_sets_follow_connection_mode_and_exclude_user_only_recording`,
  `volicord_mcp_subcommand_tools_list_respects_connection_mode_and_schema_boundary`,
  `generated_mcp_workflow_tool_contract_snapshot_matches_sources`,
  `generated_mcp_read_only_tool_contract_snapshot_matches_sources`는 각 계층에서
  도구 집합 스키마와 표준 입출력 `tools/list` 노출을 확인합니다.
- `status_is_read_only_including_dry_run`과
  `status_include_false_omits_optional_sections_without_effect`는 Core 상태
  분기를 확인합니다. `mcp_status_succeeds_with_readonly_storage`와
  `mcp_status_does_not_advance_state_version`은 전체 응답 동등성을 주장하지 않고
  MCP에서 보이는 읽기 전용 속성을 확인합니다.
- `rejected_branch_has_no_storage_effect`, `dry_run_branch_has_no_storage_effect`,
  `read_only_branch_has_no_storage_effect`는 커밋 없는 분기를 보호합니다.
- `committed_mutation_increments_state_version_once`와 Store 트랜잭션 재실행
  테스트는 원자적 커밋 경계를 보호합니다.
- `stage_artifact_creates_transient_handle_without_core_commit`는 스테이징
  경로가 정상 Core 변이 커밋과 혼동되지 않도록 보호합니다.
- `no_effect_branches_state_version_and_idempotency_are_stable`은 Core 쪽 API를
  통해 교차 메서드 효과 없음과 재실행 안정성을 보여 줍니다.

이 테스트들은 구현 점검입니다. Volicord 런타임 적합성 주장, 제품 수락
기록, QA 완료, 보안 증명, 닫기 준비 상태 결과, 잔여 위험 수락이 아닙니다.

## 검증 기본값

Rust 구현을 편집했을 때 저장소 기본값은 아래와 같습니다.

```sh
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all-targets --all-features
```

문서만 편집했다면 적용되는 문서 점검을 사용합니다. 문서 작업이 소스
검증을 요구하면 `cargo metadata --no-deps --format-version 1`, 저장소 검색,
요청된 테스트 명령이 적절한 구현 점검입니다.

유지 문서 구조 점검은 아래 명령으로 실행합니다.

```sh
cargo run -p xtask -- docs-check
```

그다음 바뀐 문서에 맞는 한영 의미 검토, 계약 담당 문서 검토, 기술 정확성
검토를 사람이 완료합니다.
