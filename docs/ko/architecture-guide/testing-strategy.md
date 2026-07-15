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
| 명시적으로 실행하는 실제 호스트 설정 스모크 테스트 | `volicord-cli` 패키지의 `live_host_smoke` 대상인 [`crates/volicord-cli/tests/live_host_smoke.rs`](../../../crates/volicord-cli/tests/live_host_smoke.rs). | 설치된 Codex 또는 Claude Code 실행 파일의 설정을 `VOLICORD_RUN_CODEX_SMOKE=1` 또는 `VOLICORD_RUN_CLAUDE_SMOKE=1`로 명시적으로 점검할 때. | 호스트가 최종 출력 이벤트를 전달했거나, 고정 UI를 표시했거나, User Judgment 왕복을 완료했다는 증거. |
| 명시적으로 실행하는 실제 최종 출력 매트릭스 | `live_host_smoke`의 Codex/Claude Code와 Record/Detective 조합 네 개 테스트. | 관리 설정 픽스처, 생성 래퍼 직접 응답, 실제 호스트 이벤트, 실제 고정 UI, Detective 결정, 상태 명령 대체 안내, 정확한 재생 증거를 각각 기록할 때. | 픽스처나 래퍼 직접 출력을 실제 호스트 전달 또는 UI 증거로 취급하거나, 한 호스트·프로필 셀을 다른 셀의 증거로 취급하는 것. |
| 명시적으로 실행하는 실제 Judgment 왕복 | `live_host_smoke`의 Codex 및 Claude Code `*_live_user_action_round_trip_is_opt_in` 테스트. | 인증된 환경에서 사람이 참여해 호스트 고유 Judgment 선택과 그 결과 권한 기록을 확인할 때. | 최종 출력 매트릭스 증거. Judgment elicitation과 최종 출력 고지는 서로 다른 검증 관심사입니다. |
| 명시적으로 실행하는 실제 증거 관찰 로컬 웹 왕복 | `live_host_smoke`의 Codex 및 Claude Code `*_live_evidence_observation_round_trip_is_opt_in` 테스트. | 설치된 호스트가 정확한 모델 비가시적 capability를 협상하고, 모델 맥락 밖의 host 전용 `_meta` handoff를 표시하며, 모델 가시 projection은 summary로만 유지한 채 사람이 정규 루프백 `local_web_consent` form을 제출하는 과정을 확인할 때. | 호스트 고유 Judgment elicitation, CLI 복구, 최종 출력 매트릭스 증거, 또는 호스트가 모델 비가시적 표면을 증명하지 못한 상태의 통과. 각 항목은 서로 다른 릴리스 검증 셀입니다. |
| 명시적으로 실행하는 실제 CLI 대체 경로 왕복 | `live_host_smoke`의 Codex 및 Claude Code `*_live_cli_fallback_round_trip_is_opt_in` 테스트. | 사람이 선택한 답을 실제 CLI User Channel로 제출하고, 정확한 CLI 재시도와 같은 Agent Connection의 설치된 호스트 재개를 확인할 때. | 호스트 고유 Judgment elicitation, 증거 관찰 로컬 웹, 최종 출력 매트릭스 증거. 모든 릴리스 검증 표면은 서로 분리됩니다. |
| 정확한 호스트 릴리스 게이트 | `tests/release-validation`, Cargo 패키지 `volicord-release-validation-tests`. | [호스트 릴리스 증거](../reference/host-release-evidence.md)에 따라 외부의 정확한 최종 후보 하나, 협력적 lease를 사용하는 고정 12개 셀의 append-only 게시, 주장 상태와 독립적인 정규 도출, 새 manifest, 별도 프로세스 재계산 audit을 검증할 때. | 운영 런타임 신뢰, Core 증거, host attestation, 희소 행렬, 여러 호스트 버전 집계, CLI 출력을 정규 평가기로 사용하는 것. |
| MCP 통합 테스트 | `volicord-integration-tests` 패키지의 `mcp_connection` 대상인 [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs). | MCP, Core, Store, Agent Connection 바인딩, 작업 범주 파생, 도구 노출, 재실행 맥락 바인딩, MCP에서 보이는 저장소 효과 없음 분기. | 집중 메서드 테스트나 참조 담당 문서의 대체물. |
| 공개 계약 스냅샷 테스트 | `volicord-integration-tests` 패키지의 `public_contract_snapshots` 대상인 [`tests/integration/public_contract_snapshots.rs`](../../../tests/integration/public_contract_snapshots.rs). | 생성된 API 요청 스키마와 MCP 도구 계약 스냅샷이 현재 소스에서 생성한 계약과 어긋나는지 점검합니다. | 생성 스냅샷 직접 편집, 의미 기준 참조 검토, 공개 계약이 올바르다는 증명. |
| 적합성 구현 테스트 | `volicord-conformance-tests` 패키지의 `baseline` 대상인 [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs). | Core 쪽 API를 통한 기준 범위 교차 메서드 시나리오. 재실행, 쓰기 티켓, 아티팩트, 판단, 닫기 준비 상태, 오류 처리 경로, 손상 처리 등을 포함합니다. | 제품 수락, 보안 증명, 닫기 준비 상태, 또는 제품 규칙의 유일한 출처. |
| 공유 테스트 지원 | `volicord-test-support` 패키지의 [`crates/volicord-test-support/src/lib.rs`](../../../crates/volicord-test-support/src/lib.rs). | 폐기 가능한 Runtime Home 픽스처, 등록된 프로젝트와 Agent Connection 설정, 요청 빌더, Store 검사 도우미, 공유 픽스처 구성. | 프로덕션 동작이나 오래 유지될 Runtime Home. |
| CLI 통합 테스트 지원 | [`crates/volicord-cli/tests/support/`](../../../crates/volicord-cli/tests/support/). | `binary_admin`, `guard_command`, `mcp_transport`, `serve_transport`가 재사용하는 바이너리 픽스처, 가짜 호스트와 MCP 프로세스, guard 생명주기 픽스처, JSON 및 단언 도우미. | 제품 계약 출처나 오래 유지될 런타임 상태. |
| 문서 유지보수 도구 테스트 | `xtask` 패키지의 [`xtask/tests/docs_check.rs`](../../../xtask/tests/docs_check.rs). | 읽기 전용 문서 검증기, 메타데이터 파싱, 한영 대응 범위, 로컬 링크와 앵커 점검, 용어 경로와 역할 점검, 명령 예시 검증, 공개 언어 점검, 임시 픽스처 동작. | 의미 번역 검토, 기술 정확성 검토, 제품 계약 출처. |

## 픽스처와 지원 구조

릴리스 검증 패키지는 비공개 테스트 배치를 계약으로 만들지 않고 담당 문서의 불변조건
계열을 소비해야 합니다. 필수 셀이 없거나 형식이 잘못되면 manifest 생성을 막는 구조
오류입니다. 존재하는 구현 셀이 ignored, running, 오래됨, 불일치이면 오래 유지되는
하향 조정 사례이며 정적 `unsupported_by_host`와 구분합니다. 후보, 셀, manifest,
audit fixture는 파싱과 도출을 보호할 수 있지만 요청한 검증됨 릴리스 주장은 정확한
외부 실제 실행만 충족할 수 있습니다.

오래 유지되는 게시 테스트는 호스트 시작 전 lease 경쟁, 기존 최종 이름 거부, 경쟁 최종
항목을 보존하는 no-replace, 셀을 게시하지 않는 증거 stage 실패, 증거 게시 뒤에도 생산자
셀이 없는 실패, 정적 미지원 셀만의 게시, 증거 뒤에만 셀이 보이는 순서, 누락 셀 또는 셀
디렉터리의 추가 stage에 대한 게이트와 audit 거부, 참조되지 않은 증거의 비채택, 새 result
root에서만 성공하는 복구, 비-clean 상태 아래의 완전한 최종 이름 집합 거부, 커밋 전
중단된 시도 뒤 동기화된 `active` 상태, 보이는 잔여물이 없어도 같은 root의 재획득을
거부하는 동작을 다룹니다. 정확한 `clean`을 관찰 가능한 상태 커밋 표식으로 검사하며,
실패한 파일시스템 동기화 호출이 durable해졌는지는 추론하지 않습니다. 이 테스트는 비공개
정리 순서가 아니라
[append-only 실제 셀 게시 불변조건](../reference/host-release-evidence.md#append-only-live-cell-publication)을
검증합니다.

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

`live_host_smoke`는 일반 Cargo 테스트 대상이며 실제 호스트 점검에는 `#[ignore]`가
붙어 있습니다. 따라서 일반 워크스페이스 테스트 실행은 이 점검들을 무시된 항목으로
보고합니다. 순수 결과 경로·운영자 토큰 점검과 폐기 가능한 MCP-to-Core 회귀 점검은
무시되지 않고 일반 CI에서 실행됩니다. 호스트 실행 파일이 설치되어 있고 해당 선택
변수를 설정한 환경에서만 실제 점검을 실행합니다.

호스트 설정 점검은 계속 별도로 실행합니다.

```sh
VOLICORD_RUN_CODEX_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_smoke_is_opt_in -- --ignored --nocapture
VOLICORD_RUN_CLAUDE_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_smoke_is_opt_in -- --ignored --nocapture
```

### Typed 호스트 기능 지원 평가기

호스트 통합 단위 테스트는 설정 픽스처 및 출력 렌더링과 분리해 중앙 평가기를 검증합니다.
최소 표는 다음과 같습니다.

| 구현 사실 | 정확한 현재 증거 | 런타임 준비 상태 | 예상 `HostFeatureSupportStatus` |
|---|---|---|---|
| 호스트가 지원하지 않음 | 모두 | 모두 | `unsupported_by_host` |
| 구현됨 | 누락, 오래됨, 만료, 형식 오류, 불일치 | 모두 | `implemented_unverified` |
| 구현됨 | 현재 상태이며 정확히 결속됨 | 일시적으로 사용할 수 없음 | `temporarily_unavailable` |
| 구현됨 | 현재 상태이며 정확히 결속됨 | 준비됨 | `verified` |

테스트는 `unsupported_by_host`, `implemented_unverified`,
`temporarily_unavailable`, `verified` 순서의 집계 우선순위도 검증합니다. 설정 존재 여부와
설정 감사 결과는 독립적으로 바꾸며 예상 지원 상태를 변경해서는 안 됩니다.

표 기반 호스트 기준 테스트는 안정된 여섯 기능 키를 모두 다룹니다. 실제 증거가 없을 때
Codex는 앞의 네 기능을 `implemented_unverified`, 두 최종 출력 기능을
`unsupported_by_host`로 보고합니다. Claude Code는 여섯 기능을 모두
`implemented_unverified`, Generic은 모두 `unsupported_by_host`로 보고합니다. 최종 출력
테스트는 Record가 `authority_display`, `authenticated_exact_replay`만 사용하고 Detective가
`block_finalization`을 더하며, 프로필에 적용되는 키만 직렬화하는지도 검증합니다.

연결 상태, Doctor, 릴리스의 모든 셀은 하나의 평가에서 같은 여섯 키
`host_feature_support` map을 projection해야 합니다. 바이너리 테스트는 정확한 필드 경로,
모든 필수 키, 추가 키 부재, 결정적인 Doctor 행, 프로필별 최종 출력 세부정보가 map을
대체하지 않는다는 사실을 검증합니다. 저장 guard capability 테스트는 명시적인 구현 및
설정 사실을 담은 내부 `host_capability_json` schema v2를 요구합니다. V1 기록은 현재
입력으로 거부하고 init을 다시 실행해야만 복구하며, v1 boolean에서 typed 지원 상태를
추론하면 안 됩니다.

실제 테스트 harness의 `verified`, `unavailable`, `not_applicable`, `failed` 증거 상태는 제품
지원 상태와 분리해 유지합니다. 특히 harness의 `not_applicable`은 제품
`HostFeatureSupportStatus`가 아니며 픽스처 통과로 릴리스 셀을 올릴 수 없습니다. 릴리스
테스트는 `verified`를 기대하기 전에 현재 증거를 정확한 최종 실행 파일과 담당 문서가
정의한 모든 호스트, 빌드, 어댑터, 연결, 증거, 최신성 좌표에 결속합니다.

### 최종 출력 호스트·프로필 매트릭스

최종 출력 점검은 명시적인 네 개 셀로 구성됩니다. 각 셀은 고유한 선택 변수와 테스트를
가지며, 한 셀의 결과가 다른 셀을 충족할 수 없습니다.

| 호스트 | Record profile | Detective profile |
|---|---|---|
| Codex | `codex_record_live_final_output_is_opt_in`, `VOLICORD_RUN_CODEX_RECORD_FINAL_OUTPUT_SMOKE=1` | `codex_detective_live_final_output_is_opt_in`, `VOLICORD_RUN_CODEX_DETECTIVE_FINAL_OUTPUT_SMOKE=1` |
| Claude Code | `claude_code_record_live_final_output_is_opt_in`, `VOLICORD_RUN_CLAUDE_RECORD_FINAL_OUTPUT_SMOKE=1` | `claude_code_detective_live_final_output_is_opt_in`, `VOLICORD_RUN_CLAUDE_DETECTIVE_FINAL_OUTPUT_SMOKE=1` |

네 명령은 다음과 같습니다.

```sh
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/codex-record.json VOLICORD_RUN_CODEX_RECORD_FINAL_OUTPUT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_record_live_final_output_is_opt_in -- --ignored --nocapture
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/codex-detective.json VOLICORD_RUN_CODEX_DETECTIVE_FINAL_OUTPUT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_detective_live_final_output_is_opt_in -- --ignored --nocapture
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/claude-record.json VOLICORD_RUN_CLAUDE_RECORD_FINAL_OUTPUT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_record_live_final_output_is_opt_in -- --ignored --nocapture
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/claude-detective.json VOLICORD_RUN_CLAUDE_DETECTIVE_FINAL_OUTPUT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_detective_live_final_output_is_opt_in -- --ignored --nocapture
```

크기가 제한된 각 결과는 `host`, `profile`, 전체 `result`를 이름 붙이고,
`config_fixture`, `generated_wrapper_direct_wire`, `actual_host_event`,
`actual_host_fixed_ui`, `detective_decision`, `status_fallback`, `exact_replay`를
서로 독립된 증거로 둡니다. 증거 상태는 `verified`, `unavailable`,
`not_applicable`, `failed`입니다. 이는 검증 하네스 사실이며 제품 응답 스키마가
아닙니다.

증거 계층은 서로 대신할 수 없습니다.

- `config_fixture`는 관리 설정 형태를 점검합니다. 설치된 호스트가 이를 읽거나 이벤트를
  전달했다는 증거는 아닙니다.
- `generated_wrapper_direct_wire.status_fallback`과
  `generated_wrapper_direct_wire.authority_receipt`는 생성 래퍼를 직접 호출해 크기가
  제한된 두 호스트 응답 분기를 분리해서 점검합니다. 둘 다 검증해야 하지만 실제 호스트
  전달이나 고정 UI 표시를 증명하지는 않습니다.
- `actual_host_event.status_fallback_event`와
  `actual_host_event.authority_receipt_event`는 설치된 호스트의 두 전달을 분리해서 기록하고,
  `actual_host_fixed_ui.authority_receipt`는 모델 산문이 아니라 고정 UI에 표시된 현재
  Task의 receipt 전체를 별도로 요구합니다. 어느 쪽도 다른 쪽을 증명하지 않습니다.
  `actual_host_fixed_ui.status_fallback`은 Task가 없을 때의 고정 UI 분기를 독립적으로
  확인합니다.
  Record는 의도적으로 지속 Guard 관찰을 만들지 않습니다. 따라서 event 항목은 인증된
  호스트 소유 관리 UI 전달을 근거로 밝히고, 전후 개수 검사로 Guard 이벤트나 Agent
  Session이 생성되지 않았음을 증명합니다. 지속 Record 관찰을 꾸며내지는 않습니다.
- 최상위 `status_fallback` 증거는 Task 없음 UI 확인을 정확한 생성 명령
  `volicord status --json`과 결속합니다. 래퍼 직접 출력은 UI 관찰을 대신할 수 없고,
  고정 UI receipt도 대체 안내를 대신할 수 없습니다. 운영자는 Task 없음 관리 UI 문구
  전체를 복사하고 하네스는 Task별 명령 부재까지 포함해 정확히 같은지 검사합니다.
  모든 셀은 `actual_host_fixed_ui`
  아래의 두 분기와 별도 명령 증거를 모두 검증해야 합니다.
- `exact_replay.generated_wrapper_identical_payload`는 생성 래퍼에 같은 payload를 반복
  전달한 결과를 기록하고, `exact_replay.actual_host_replay`는 실제 호스트 진입점을 통한
  재생을 기록합니다. 생성 래퍼 검사는 두 번의 동일 전달 사이 Task 권한 상태를
  전진시키고 두 번째 wire에 더 최신인 receipt가 나오는지 요구하며, Detective에서는
  저장된 과거 Stop 행이 정확히 그대로여야 합니다. 래퍼 직접 재생을 실제 호스트
  재생으로 보고하면 안 됩니다.

Record profile의 최종 출력 경로는 차단하지 않고 관찰을 기록하지 않습니다.
`detective_decision` 증거는 Guard 이벤트나 결정이 없고 최종 출력을 차단하지 않았다는
사실도 함께 확인할 때만 `not_applicable`입니다. 반복 전달은 관찰을 만들지 않은 채
읽기 전용 표시를 새로 고쳐야 합니다.

Detective profile의 결정 증거는 `allow`와 `block`을 모두 다룹니다. 정확한 재생은 변경할 수
없는 과거 Guard 이벤트와 결정을 보존하면서 별도 고정 UI에서 현재 권한을 새로
조회합니다. 따라서 나중의 최신 receipt는 과거 receipt와 다를 수 있습니다. 설치된
호스트가 안전한 `block` 진입점이나 실제 호스트 정확한 재생 진입점을 제공하지 않으면
해당 증거는 `unavailable`이고 전체 결과는 `incomplete`로 남습니다. 실행 파일, 인증
환경, 대화형 TTY, 이벤트 전달 표면, 현재 Task의 receipt가 표시되는 UI, Task가 없을
때의 대체 안내 UI를 사용할 수 없을 때도 같습니다. 이런 실행은 `incomplete`로 남기고
`PASS`가 아니라 `SKIP` 또는 `FAIL`로 보고하며, 픽스처나 생성 래퍼 증거로 결과를 올릴 수
없습니다.

이 호스트·프로필 경로를 릴리스 지원 범위로 주장하기 전에는 대응하는
[실제 호스트 최종 출력 릴리스 검증 체크리스트](../maintain/validation.md#live-host-final-output-release-validation)를
따릅니다. 정확한 최종 출력, receipt, 재생, 대체 안내 동작은
[Agent Connection](../reference/agent-connection.md#managed-final-output-authority-disclosure)과
[관리 CLI](../reference/admin-cli.md#managed-final-output-authority-disclosure)를 포함한 적용
참조 담당 문서에 남습니다.

### Judgment 왕복

Judgment 점검은 최종 출력 매트릭스와 별도로 유지합니다.

```sh
VOLICORD_RUN_CODEX_USER_ACTION_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_user_action_round_trip_is_opt_in -- --ignored --nocapture
VOLICORD_RUN_CLAUDE_USER_ACTION_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_user_action_round_trip_is_opt_in -- --ignored --nocapture
```

판단 변형은 사람 참여형 점검입니다. 폐기 가능한 Runtime Home과 Product Repository를
만들고 선택한 호스트를 설정한 뒤, 쓰기를 하지 말라는 초기 지시와 함께 설치된 호스트를
대화형으로 실행합니다. 실행자의 일반 호스트 인증 환경을 재사용하며 픽스처의 격리된
Runtime Home으로 자격 증명을 복사하지 않습니다. 호스트가 요구하면 운영자가 프로젝트나
MCP 항목을 승인하고, 호스트 고유 MCP elicitation UI에서 직접 답을 선택하고, 상태 보고가
끝난 뒤 호스트를 종료해야 합니다.

판단 변형이 통과하면 표식 `Task`와 판단 생성, `mcp_elicitation_user_channel` 근거의
호스트 고유 프롬프트/응답 기록, 그에 따른 Task 상태 전환, 권한 이벤트, 내용 없는 해당
세션 진단을 검증한 것입니다. Task는 advisor 모드를 사용하고 판단을 요청하기 전에 현재
Change Unit과 baseline을 만듭니다. Judgment에는 고정된 route option 두 개가 있습니다.
사람이 하나를 선택하면 에이전트는 기본 간결한 Judgment 결과를 소비하고, 해당 option에
매핑된 정확한 요약과 close-assessment 표식을 가진 Product Repository 비쓰기
`shaping_update` Run을 기록해야 합니다. 호스트 종료 뒤 운영자가 선택한 고정 option을
확인하면 하네스는 그 값이 저장된 `selected_option_id`와 같은지 검사합니다. 이어서 최신
영수증의 `latest_run_ref`가 가리키는 정확한 Run 행을 읽고, 일치하는 사용자 판단과 Run
권한 이벤트 payload와 event sequence가 선택 후 소비 순서를 증명하는지 검사합니다.
고유 elicitation을 사용할 수 없으면 하네스는 대기 inbox 표시와 현재 답변 명령 형태를
검사하고 임시 경로가 없는 명령 템플릿을 내보낸 뒤 실제 고유 왕복 점검을 실패시킵니다.
그 뒤 폐기 가능한 픽스처가 삭제되므로 이 템플릿은 실행 가능한 복구 명령이 아니며,
실패한 실제 고유 프롬프트 점검을 통과 결과로 바꾸지 않습니다.

판단 변형은 호스트의 `--version` 출력과 Volicord `build_id`도 수집하고, 최신 CLI
상태를 읽어 `authority_receipt`가 같은 Project, Task, 정확한 Run, `close_state=ready`,
빈 close blocker 집합, `state_version`에 결속됐는지 확인합니다. 또한 호스트 실행 전
cursor 뒤에 정확히 하나의 새 Task 결속 Detective Stop 이벤트가 생겼는지, 이유와 close
blocker가 없는 `allow`인지, 저장된 receipt가 최신 status와 같은지 확인합니다. 운영자는
호스트 소유의 별도 관리 UI에서 완전한 canonical receipt JSON을 복사해야 하며, 하네스는
`state_version` 하나만 받지 않고 전체가 정확히 같은지 검사합니다. 크기가 제한된 JSON
요약을 출력합니다. 인증된 12개 셀 생산자는 모두
[append-only 실제 셀 게시 계약](../reference/host-release-evidence.md#append-only-live-cell-publication)에
설명된 외부 `RESULT_ROOT/cells` 디렉터리 바로 아래의 존재하지 않는 경로를
`VOLICORD_LIVE_HOST_RESULT_PATH`로 지정해야 합니다. 하네스는 폐기 가능한 Runtime Home을
결속하고 다시 검사한 뒤 협력적 result-root lease를 획득하며, 기존 최종 이름이 있으면
거부하고 비공개 시도 상태를 `active`로 동기화한 뒤 호스트를 시작합니다. 임시 `running`
셀은 만들지 않습니다. 성공한 구현 셀 게시는
동기화된 증거를 먼저 설치하고 셀을 마지막에 설치하며, 정적 미지원 게시는 셀만
설치합니다. 최종 게시 뒤에만 하네스가 `active`를 정확한 `clean` 레코드로 바꾸기 시작하며,
생산자의 동기화 반환이 불확정이어도 후속 exact-clean 관찰이 권위 있습니다. 게시 실패는
담당 문서가 허용하는 크기 제한 stage 또는 설치된 최종 이름 prefix를 남길 수 있고 하네스는
이를 삭제하거나 재사용하지 않습니다. 결과에는 검증 사실만 들어가며 대화 기록, 자격 증명,
비밀값, 전체 프롬프트는 들어가지 않습니다.

Task 결속 Stop 이벤트와 완전한 receipt UI는 Judgment 실행이 권위 있는 완료 상태에
도달했음을 확인하는 필수 증거입니다. 이 증거는 네 개 셀 최종 출력 매트릭스의 셀이나
증거 필드를 채울 수 없고, 최종 출력 매트릭스 증거도 고유 Judgment elicitation을
증명할 수 없습니다. Judgment 실행 중 관찰한 그 밖의 최종 출력, 대체 안내, 재생은
해당 실행의 진단 자료일 뿐입니다.
Judgment inbox 대체 경로는 User Channel 복구 증거이며 최종 출력 `status_fallback`
증거가 아닙니다.

유지되는 호스트 판단 경로를 지원한다고 명시하는 릴리스를 게시하기 전에는 대응하는
[실제 호스트 판단 릴리스 검증 체크리스트](../maintain/validation.md#live-host-judgment-release-validation)를
따릅니다. 이 체크리스트는 릴리스 후보에서 두 호스트별 실행을 모두 요구하고 외부 결과
보존, UI 확인, 대체 경로, 건너뛴 검증 보고를 담당합니다. 호스트, 인증 환경, 대화형
TTY, 고유 elicitation 표면을 사용할 수 없으면 통과한 왕복으로 취급하지 않습니다.

### User Channel projection 경계 회귀 테스트

집중 Core 테스트는 유지되는 모든 action kind를 table로 구성하고 공개 create,
pending status, pending close, reconcile, 중첩 `StateSummary` projection을 정확한 세
필드 Agent 안전 summary와 비교합니다. 또한 별도로 권한을 검증하고 직렬화하지 않는
User Channel projection이 CLI 직접 및 같은 session host 제출에서 완전한 정규 form을
유지하는 반면 Agent Connection, MCP, Stop, final-output, fixture 맥락은 닫힌 상태로
실패함을 증명합니다.

집중 Store 테스트는 변경 불가능한 호스트 역량 행을 게시하고 passed, failed,
unavailable, revoked, 만료, 잘못된 형태, 모든 정확한 결속 불일치 사례를 table로
구성합니다. 정규 UTC 입력,
`observed_at <= created_at`, `observed_at < expires_at <= observed_at + 86,400 seconds`,
통과 행의 `created_at < expires_at`, 반개구간 최신성, 정확한 중복의 멱등성, 같은 ID와 다른
내용의 충돌, 원자적 현재 포인터 교체, 중복 게시에 의한 더 새로운 포인터 후퇴 없음, 더
오래된 pass로 뒤로 찾지 않음, Agent Connection 삭제 연쇄 효과를 검증합니다. 24시간은
기본 수명으로 가정하지 않고 최대값으로 테스트합니다.

집중 MCP 테스트는 capability의 정확한 `true`, 생략, `false`, 잘못된 타입, 잘못된
namespace, listener 사용 불가, 관리 시작 원천, 보존된 `clientInfo`, 현재 영속 검증을
table로 구성합니다. Token과 닫힌 host 전용 `_meta` handoff에는 관리되는 generic이 아닌
stdio 경로, 준비된 listener, 정확한 선언, 최신의 정확히 일치하는 `outcome=passed` 행이라는
평가기 입력이 모두 필요합니다. 운영 자격에는 행의 `evidence_artifact_sha256`이 같은 역량,
호스트·클라이언트, 어댑터, 빌드, source, target, 실행 파일 다이제스트에 결속된 별도로
검증한 외부 정확한 최종 아티팩트 릴리스 증거 manifest 또는 receipt의 예상 다이제스트와
일치해야 한다는 조건도 있습니다. 현재 어댑터에는 그 manifest를 신뢰해 획득하는 경로가
없으므로 운영 선택은 닫힌 상태로 실패합니다.

현재 집중 테스트는 누락·비통과 상태, 만료, 교체, 선택된 결속 불일치, 현재 pass가 없는
generic 자기 선언, 관리 positive fixture를 증명합니다. 시작 원천 테스트는 수동 stdio와
CLI 검증을 별도로 분류하고 Local HTTP에는 전송 중심 테스트가 있습니다. 그러나 그 밖의
모든 값이 정확한 현재 pass를 게시한 뒤 수동 stdio, CLI 검증, Local HTTP 각 경로에서
비발급을 증명하는 테스트는 아직 없습니다. 이 세 경로가 다뤄졌다고 주장하려면 그런
exact-pass negative 회귀 테스트가 필요합니다.

MCP suite는 별도로 준비됐던 listener를 저하하거나 deferred 선택과 최종 materialization
사이에 검증을 교체하여 일반 CLI fallback, `_meta` 부재, token 행 0개, project clock 효과
없음을 증명합니다. 모델 가시 상태 보기는 모든 form 및 credential 필드를 빼야 합니다.
별도 동시성 invariant 테스트는 무효화가 사용 불가 상태를 즉시 게시하고 새 lease를
차단하며, listener가 종료되기 전에 이미 부여된 발급 lease를 모두 drain함을 증명합니다.
삽입 경계 assertion은 token creator가 그 lease를 보유한 동안 실행됨을 증명합니다. Budget
경계 테스트는 token을 만들기 전에 완전한 안전 결과와 handoff를 사전 검사하여 정확한
compact 및 full 한계에서는 성공하지만 그 다음 1 byte에서는 일반 CLI 안내로 저하되고
orphan token을 만들지 않음을 증명합니다. Replay 테스트는
기존 full-form, 기존 `StateSummary`, 혼합 형태, 잘못된 메서드 저장 행에 대한 직접 retry,
resume, close, operation-result 첫 page를 다루며 거부된 행이 상태나 정리 효과를 만들지
않음을 확인합니다.

### 증거 관찰 로컬 웹 왕복

증거 관찰 점검은 호스트 고유 Judgment elicitation, 실행 가능한 CLI 복구, 호스트 설정
스모크 테스트, 최종 출력 매트릭스와 각각 분리된 두 개의 호스트별 셀입니다.

```sh
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/codex-evidence-observation.json VOLICORD_RUN_CODEX_EVIDENCE_OBSERVATION_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_evidence_observation_round_trip_is_opt_in -- --ignored --nocapture
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/claude-code-evidence-observation.json VOLICORD_RUN_CLAUDE_EVIDENCE_OBSERVATION_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_evidence_observation_round_trip_is_opt_in -- --ignored --nocapture
```

Credential을 포함한 왕복은 자신의 통과 호스트 역량 행을 bootstrap할 수 없습니다. 현재
workspace에는 별도의 비밀이 아닌 challenge/import 경로도, 외부 정확한 최종 아티팩트
릴리스 증거 manifest 또는 receipt를 신뢰해 획득하고 검증하는 경로도 없습니다. 그
manifest는 역량, 호스트·클라이언트, 어댑터, 빌드, source, target, 최종 실행 파일
다이제스트, 예상 `evidence_artifact_sha256`에 결속되어야 하고 현재 행의 다이제스트가 그
예상값과 정확히 일치해야 합니다. 따라서 이 셀의 local-web 부분은 현재 검증되지 않았으며
`passed`를 기록할 수 없습니다. 선택한 실행은 그 부분에 `unavailable`을 기록하며 fixture나
direct wrapper는 이를 통과로 올릴 수 없습니다. 이후 검증 전용 경로가 정확한 행과
manifest를 게시하더라도 별도 일반 실행에서 manifest를 검증하고 그 현재 행을 release
binary와 런타임 `clientInfo`에 맞춰 다시 평가해야 합니다.

각 셀은 픽스처 전용 준비로 폐기 가능한 시작 상태를 만든 뒤, 실제 설치 호스트가 준비된
Agent Connection에서 증거 관찰 요청 하나를 만들고 재개하도록 요구합니다. 캡처한
초기화 교환에서
`params.capabilities.experimental["io.volicord/user-channel"].model_invisible_user_surface`가
정확한 boolean `true`임을 관찰해야 합니다. 이 선언은 필요하지만 충분하지 않습니다. 현재의
정확히 일치하는 `outcome=passed` 검증, 검증된 외부 manifest, 관리 시작·listener 입력도
중앙 평가기를 통과해야 합니다. 그때만 Agent나 하네스가 아닌 호스트가
`CallToolResult._meta["io.volicord/user-channel"]` handoff를 모델 맥락 밖에 표시하고,
사람은 그 표면을 사용해 루프백 consent form을 열어 준비된 대상과 아티팩트,
`supported`, 크기가 제한된 비밀값 없는 요약을 제출합니다. 이어서 호스트가 그 해결을
Run에서 소비해야 합니다. Store 검사, 권한 이벤트 순서, 새 상태, Task 결속 Stop 이벤트,
완전한 관리 UI receipt 확인이 계층 간 관찰 단언을 제공합니다. 픽스처 준비와 어댑터 전용
점검은 별도로 구분하며 실제 설치 호스트 관찰을 대신할 수 없습니다.

실제 하네스는 제한된 boolean, count, digest만 기록하며 실제 create, pending status,
pending close, 정확한 operation-result, resume 교환을 검사합니다. MCP 호출에서는
`content`, `structuredContent`, 호환·진단 text, replay된 Agent Workflow 본문도
검사합니다. 각 모델 가시 projection은 정확한 세 필드 pending summary를 담고 전체 요청,
질문, option, context, form, capture path, command, raw URL, bearer token을 빼야 합니다.
URL은 관찰된 host 소유 `_meta` 전달에만 존재할 수 있습니다. 이 단언은 비밀값 탐지,
호스트 고유 elicitation, 외부 호스트의 일반적인 보안을 증명하지 않습니다.

이 내용은 릴리스 테스트 단언이며 두 번째 API 계약이 아닙니다. 정확한 요청과 재개 동작은
[`volicord.request_user_action`](../reference/api/method-request-user-action.md), 해결 동작은
[`volicord.resolve_user_action`](../reference/api/method-resolve-user-action.md), 공통 요청과
해결 형태는 [API 사용자 행동 스키마](../reference/api/schema-user-action.md), 소비 Run과
증거 효과는 [`volicord.record_run`](../reference/api/method-record-run.md), 로컬 웹 경로는
[MCP 전송](../reference/mcp-transport.md#local-web-consent-fallback)이 담당합니다. 새 상태와
receipt 비교는 [상태 메서드](../reference/api/method-status.md)와
[API 상태 스키마](../reference/api/schema-state.md)를 따릅니다.

실제 셀은 호스트 대화 기록이나 raw tool body를 보존하지 않습니다. 그럼에도 제한된
교환 observer는 create, resume, status, close, operation-result projection에 대해 위의 안전한
형태와 금지 필드 사실을 확인해야 합니다. 설치된 호스트가 정확한 capability를
누락하거나 잘못 표현하거나, 정확한 현재 검증이 없거나 통과하지 않았거나 만료·손상·불일치
상태이거나, 모델 맥락 밖에 handoff를 표시하지 못하거나, host 전용
`_meta`와 모델 가시 결과 데이터를 구별하는 관찰 경계를 제공하지 못하면 셀은
`passed`가 아닌 `unavailable`을 기록합니다. 집중 스키마와 어댑터 회귀 테스트는
계속 필요하지만 이 호스트 관찰 불가 결과를 통과로 올릴 수 없습니다.

크기가 제한된 외부 결과에는 안전한 검증 좌표와 요약 일치 사실만 기록하며 consent URL,
bearer token, 원문 요약, 프롬프트, 대화 기록은 담지 않습니다. 결과의 생명주기와 보존
규칙은 아래에서 연결하는 대응 체크리스트가 관리합니다.

이 무시된 셀을 실행하려면 설치된 호스트 실행 파일, 평소 인증 환경, 대화형 TTY,
정확한 모델 비가시적 capability 협상, 관찰 가능한 host 전용 handoff 표면, 사용할 수 있는
로컬 브라우저, 호스트가 요구하는 신뢰나 승인, 새로운 외부 결과 경로가 필요합니다.
일반 Cargo 실행에서 테스트가 무시됐다는 보고, 선택 변수를 설정하지 않은 실행, 필요한
조건을 사용할 수 없는 실행은 통과가 아닙니다. 릴리스에서 이 경로를 지원한다고 명시하기
전에는 대응하는
[실제 호스트 증거 관찰 릴리스 검증 체크리스트](../maintain/validation.md#live-host-evidence-observation-release-validation)를
따릅니다. 유지되는 두 호스트를 모두 지원한다고 명시하려면 두 호스트별 셀이 모두
통과해야 합니다. 이 셀은 호스트 고유 Judgment, CLI 대체 경로, 설정, 최종 출력 셀을
충족할 수 없고 그 반대도 마찬가지입니다. 정확한 제품 동작은 위에서 연결한 집중 담당
문서에 남습니다.

### CLI 대체 경로 Judgment 왕복

실제로 실행하는 CLI 대체 경로 점검은 호스트 고유 Judgment elicitation과 네 셀 최종
출력 매트릭스에서 모두 분리된 호스트별 두 셀입니다.

```sh
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/codex-cli-fallback.json VOLICORD_RUN_CODEX_CLI_FALLBACK_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_cli_fallback_round_trip_is_opt_in -- --ignored --nocapture
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/claude-code-cli-fallback.json VOLICORD_RUN_CLAUDE_CLI_FALLBACK_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_cli_fallback_round_trip_is_opt_in -- --ignored --nocapture
```

각 셀은 선택한 Detective Agent Connection에 `advisor` Task, 현재 Change Unit,
baseline, 두 선택지가 있는 현재 대기 상태 product-decision 요청을 준비합니다. 사람
운영자가 `route_alpha` 또는 `route_beta`를 고르면 하네스는 실제
`volicord inbox --json` 결과에서 요청을 확인하고, 실제
`volicord inbox resolve ... --choice ... --json` 명령으로 그 선택을 제출합니다. 이어서
정확히 같은 명령을 반복하고 JSON byte가 동일하며 `state_version`이 바뀌지 않는지
확인합니다. 이는 실행된 User Channel 해결이며, 호스트 고유 Judgment 셀이 실패할 때
출력하는 임시 경로 없는 명령 템플릿 진단이 아닙니다.

그다음 하네스는 준비에 사용한 것과 같은 Agent Connection으로 설치된 호스트를
실행합니다. 호스트는 정확한 요청 ID로 `volicord.request_user_action`의
`request.operation=resume`을 호출하고, product-decision 요청을 새로 만들지 않은 채 CLI로
선택된 option을 소비하며, 그 Agent Connection을 통해 매핑된 Product Repository 비쓰기
`shaping_update` Run을 기록해야 합니다. 새 CLI status는 차단 사유가 없는 ready
`AuthorityReceipt`에서 정확한 Run을 가리켜야 합니다. 같은 실제 호스트 경로는 최신
status와 저장 receipt가 같은 새 Task 결속 Detective Stop `allow` 이벤트도 하나 만들어야
합니다. 운영자는 별도의 호스트 소유 관리 UI에서 완전한 canonical receipt를 복사해
정확히 같은지 확인해야 합니다.

크기가 제한된 외부 결과는
`kind=live_host_cli_fallback_release_validation`을 사용하고, CLI resolution ID,
`actor_source=local_user`, `channel_kind=cli`,
`verification_basis=cli_direct_user_channel`, 두 CLI 상태 버전, 정확한 재시도 사실,
같은 연결 재개 증거, 매핑된 Run과 권한 이벤트 순서, Stop 좌표, 최신 receipt, 관리 UI
확인을 기록합니다. 또한 호스트 고유 Judgment와 최종 출력 매트릭스 범위는 false로
표시합니다. 이 셀의 결과는 두 표면 중 어느 것도 충족할 수 없고, 그 표면의 증거도 이
셀을 충족할 수 없습니다.

유지되는 호스트의 실행 가능한 CLI 복구를 지원한다고 명시하기 전에는 대응하는
[실제 호스트 CLI 대체 경로 릴리스 검증 체크리스트](../maintain/validation.md#live-host-cli-fallback-release-validation)를
사용합니다. 두 호스트를 모두 지원한다고 명시하려면 두 셀이 모두 통과해야 합니다.
실행 파일, 인증 환경, 대화형 TTY, 같은 연결 재개 경로, Task 결속 Stop, 완전한 receipt
UI 중 하나라도 사용할 수 없으면 통과가 아니라 `SKIP` 또는 `FAIL`입니다.

명시적으로 선택한 점검은 선택 변수, 호스트 실행 파일, 그 밖의 필수 실제 전제 조건을
사용할 수 없으면 통과할 수 없습니다. 적용되는 체크리스트에 따라 `SKIP` 또는 `FAIL`로
보고합니다. 통과 결과는 설치된 호스트와 로컬 테스트 환경에서 스모크 테스트가 관찰한
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
