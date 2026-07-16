# 소스 지도

이 문서는 현재 Volicord Rust 워크스페이스의 가이드 수준 소스 경로 지도를
담당합니다. 유지보수자가 코드 읽기와 코드 변경 질문을 올바른 모듈로 보낼 수
있도록 소스 경로와 구현 책임을 문서화합니다.

이 문서는 실행 흐름, 요청 생명주기, 저장소 트랜잭션, 공개 API 동작, 스키마
의미, 저장 효과, 보안 보장, Core 권한 의미, 제품 계약을 설명하지 않습니다.
상위 수준 아키텍처와 의존 경계는 [구현 아키텍처](architecture.md), 첫 번째
학습 경로는 [코드베이스 둘러보기](codebase-tour.md), 관리 CLI 작업 흐름 경계는
[CLI 작업 흐름](cli-workflows.md), 대표 메서드 흐름은
[요청 생명주기](request-lifecycle.md), Store 커밋과 아티팩트 경계는
[저장소와 트랜잭션](storage-and-transactions.md), 테스트 계층 선택은
[테스트 전략](testing-strategy.md), 정확한 계약은 [참조 색인](../reference/README.md)을
사용합니다.

모든 소스와 테스트 경로는 저장소 루트 기준입니다.

## 워크스페이스 구성 요소

| 경로 | Cargo 패키지 | 담당 범위 |
|---|---|---|
| `crates/volicord-types` | `volicord-types` | 공유 Rust 요청, 응답, 스키마 형태, 값 집합, MCP 도구 이름, 식별자, 정규화된 해시, 호스트 기능 구현 타입, 정적 매트릭스, 단일 기능 지원 상태 평가. |
| `crates/volicord-store` | `volicord-store` | SQLite, Runtime Home, 부트스트랩, 프로젝트 Store, 아티팩트 저장소, 검사, `guard`와 세션 관찰 저장, 변경 불가능한 호스트 역량 검증 이력과 현재 상태 평가, 로컬 웹 동의 저장, 내보내기 스냅샷, 저장소 오류 구현. |
| `crates/volicord-core` | `volicord-core` | Core 서비스, 공유 요청 파이프라인, 메서드 계획, 정책 점검, 응답 구성, Store 조율. |
| `crates/volicord-cli` | `volicord-cli` | 로컬 `volicord` 관리 바이너리, 재사용 명령 모듈, Runtime Home 설정, 프로젝트와 Agent Connection 등록, 호스트 어댑터, `guard` 훅, User Channel 명령, 공개 `volicord mcp` 프로세스 디스패치. |
| `crates/volicord-platform-fs` | `volicord-platform-fs` | Store 소유자 검증과 로컬 어댑터가 사용하는 플랫폼 고유 파일시스템 이름 공간 연산 및 정규 읽기 전용 Git layout snapshot을 위한 내부 안전 파사드. |
| `crates/volicord-mcp` | `volicord-mcp` | 시작 검증, 도구 목록, `tools/call` 디코딩과 디스패치, 표준 입출력 프레이밍, 로컬 HTTP 전송, Core 호출을 위한 로컬 MCP 어댑터 라이브러리. |
| `crates/volicord-test-support` | `volicord-test-support` | 구현 테스트가 공유하는 폐기 가능한 Runtime Home과 Product Repository 설정, Store 검사, Core 요청 빌더, Agent Connection 설정, 기타 도우미. |
| `tests/conformance` | `volicord-conformance-tests` | 담당 문서가 정의한 동작을 Core 쪽 API와 공유 픽스처로 실행하는 기준 범위 교차 메서드 시나리오. |
| `tests/integration` | `volicord-integration-tests` | MCP, Core, Store, Agent Connection 바인딩, 작업 범주, 공개 스키마 스냅샷을 가로지르는 테스트. |
| `tests/release-validation` | `volicord-release-validation-tests` | 테스트 전용 create-new 정확한 후보 설명자 생산, 외부 셀·manifest 게이트, 별도 프로세스 audit 재계산. |
| `xtask` | `xtask` | 문서 검증을 위한 저장소 유지보수 도구. Volicord 런타임 아키텍처의 일부가 아닙니다. |

## 공유 타입

| 소스 경로 | 책임 |
|---|---|
| `crates/volicord-types/src/lib.rs` | 공유 Rust API와 도메인 형태 값의 공개 크레이트 표면. |
| `crates/volicord-types/src/methods.rs` | 형식화된 공개 요청과 결과 모델, 메서드 요청 스키마 생성, 메서드와 `operation_category` 매핑. |
| `crates/volicord-types/src/schema.rs` | 공유 요청 래퍼, 응답, 상태, 아티팩트, 판단, 표시, 저장용 보조 형태. |
| `crates/volicord-types/src/tool_names.rs` | 공개 메서드와 어댑터 유틸리티 도구 집합을 위한 공유 MCP 노출 도구 이름 상수. |
| `crates/volicord-types/src/values.rs` | 문서화된 값 이름을 위한 제어된 Rust 열거형과 상수. |
| `crates/volicord-types/src/ids.rs` | 불투명 식별자 래퍼와 영속 ID 생성 도우미. |
| `crates/volicord-types/src/canonical.rs` | 결정적인 기준 JSON 직렬화와 요청 해시. |
| `crates/volicord-types/src/host_feature_support.rs` | 닫힌 호스트 기능과 최종 출력 하위 역량 식별자, 호스트 종류 기준 구현 사실, 검토된 버전·클라이언트 증거 좌표, 정규 Codex 버전 파싱, 어댑터·진단·릴리스 검증이 소비하는 공유 정적 구현 매트릭스와 단일 기능 지원 상태 우선순위. |

## 플랫폼 파일시스템 파사드

| 소스 경로 | 책임 |
|---|---|
| `crates/volicord-platform-fs/src/lib.rs` | Store와 로컬 어댑터에 필요한 좁은 플랫폼 고유 파일시스템 이름 공간 기본 연산과 정규 Git common-directory, worktree identity, branch/HEAD, fingerprint snapshot을 위한 안전한 Rust 파사드. 운영체제 고유 실패 효과를 보고하며 대상 상태 검증, 권한 비교, 정리, 복구, 제품 정책 결정은 각 호출자가 담당합니다. |

## Store

| 소스 경로 | 책임 |
|---|---|
| `crates/volicord-store/src/lib.rs` | 저장소 기록, 스키마 초기화, 아티팩트 배관, Store 도우미의 공개 크레이트 표면. |
| `crates/volicord-store/src/runtime_home.rs` | Runtime Home 경로 해석과 Runtime Home/Product Repository 위치 검증 도우미. |
| `crates/volicord-store/src/bootstrap.rs` | Runtime Home 메타데이터 초기화, 설치 프로필 저장, 프로젝트 등록, 현재 프로젝트 도우미. |
| `crates/volicord-store/src/agent_connections.rs` | Agent Connection 행, 자연 키, Connection Projects 멤버십, 모드와 상태 값, Agent Connection 조회와 갱신 도우미. |
| `crates/volicord-store/src/host_capabilities.rs` | 변경 불가능한 호스트 역량 검증 게시, 현재 포인터 읽기, 정확한 결속 검증, 반개구간 최신성 평가, 닫힌 상태로 실패하는 자격 결과. |
| `crates/volicord-store/src/schema.rs`와 `crates/volicord-store/src/schema/` | 레지스트리와 프로젝트의 기준 SQL 원본, 스키마 초기화와 검증 연결. |
| `crates/volicord-store/src/sqlite.rs` | 레지스트리와 프로젝트의 SQLite 경로, 열기, 검증, 트랜잭션 도우미. |
| `crates/volicord-store/src/core_pipeline.rs` | Store 쪽 Core 기록, 읽기 도우미, 변이 타입, 커밋 입력/출력 타입, 재실행 도우미, Core를 위한 공개 Store 경계. |
| `crates/volicord-store/src/core_pipeline/open.rs` | 프로젝트 로컬 Store 핸들 열기와 실행 맥락 검증. |
| `crates/volicord-store/src/core_pipeline/replay.rs` | 재실행 행 조회와 재실행 맥락 일치. |
| `crates/volicord-store/src/core_pipeline/commit.rs` | 원자적 Core 변이 커밋 트랜잭션, 상태 버전 전진, 권한 이벤트 추가, 재실행 행 삽입. |
| `crates/volicord-store/src/core_pipeline/mutation_apply.rs` | `CoreStorageMutation` 값의 트랜잭션 범위 SQL 적용. |
| `crates/volicord-store/src/core_pipeline/validation.rs` | 공유 저장 값 검증과 디코딩 도우미. |
| `crates/volicord-store/src/artifacts.rs` | 일시적 아티팩트 스테이징과 영속 아티팩트 본문 검증 도우미. |
| `crates/volicord-store/src/guards.rs` | `guard` 설치와 이벤트, 프롬프트 캡처, 예상 쓰기, 미기록 변경 관찰을 저장하는 도우미. |
| `crates/volicord-store/src/session_watch.rs` | 세션 단위 Product Repository 감시 스냅샷, 관찰, 미해결 변경 저장 도우미. |
| `crates/volicord-store/src/user_action_channel.rs` | 요청 결속 로컬 User Channel token 생성, 검증, 만료, transaction 범위 소비 도우미. |
| `crates/volicord-store/src/diagnostics.rs` | 독립된 bounded 로컬 diagnostics session/event 저장소, retention, redaction 검증, aggregate 읽기. |
| `crates/volicord-store/src/inspection.rs` | Runtime Home, 레지스트리, 프로젝트, Agent Connection, 설정 상태의 읽기 전용 검사 스냅샷. |
| `crates/volicord-store/src/export.rs` | 프로젝트 기록과 아티팩트 메타데이터를 담는 읽기 전용 권한 번들 스냅샷 조립. |
| `crates/volicord-store/src/error.rs` | Store 오류 타입과 저장소 실패 처리 경로. |

## Core

| 소스 경로 | 책임 |
|---|---|
| `crates/volicord-core/src/lib.rs` | Core 쪽 서비스와 어댑터 독립 메서드 진입점의 공개 크레이트 표면. |
| `crates/volicord-core/src/authority_status.rs` | MCP, CLI Stop, 최종 출력 고지 소비자를 위해 담당 문서가 정의한 status/`AuthorityReceipt` 대응 관계를 검증하는 Core 소유 형식 검증. |
| `crates/volicord-core/src/pipeline.rs` | `CoreService`, 호출 맥락, 공통 사전 점검, 요청 해시, Store 열기, 재실행 처리, 효과 경로 선택, 응답 구성, Core 커밋 조율. |
| `crates/volicord-core/src/methods/` | 메서드별 검증, 계획, 저장소 변이 목록, 이벤트 페이로드, `dry-run` 미리보기 요약, 결과 필드. |
| `crates/volicord-core/src/methods/status.rs` | `volicord.status` 계획과 읽기 전용 결과 구성. |
| `crates/volicord-core/src/methods/intake.rs` | `volicord.intake` 계획과 `Task`/`Change Unit` 변이 준비. |
| `crates/volicord-core/src/methods/update_scope.rs` | `volicord.update_scope` 계획과 범위 변이 준비. |
| `crates/volicord-core/src/methods/prepare_write.rs` | `volicord.prepare_write` 계획, 호환성 점검, 쓰기 티켓 변이 준비. |
| `crates/volicord-core/src/methods/record_run.rs` | 실행과 증거 관련 변이를 위한 `volicord.record_run` 계획. |
| `crates/volicord-core/src/methods/user_action.rs` | Agent workflow `volicord.request_user_action` 및 User Channel 소유 `volicord.resolve_user_action` 검증, 정규 요청 구성, 정확한 대상/artifact/basis mutation 계획. |
| `crates/volicord-core/src/methods/reconcile_changes.rs` | 해결되지 않은 Product Repository 관찰을 위한 `volicord.reconcile_changes` 계획. |
| `crates/volicord-core/src/methods/close_task.rs` | `volicord.close_task` 계획과 닫기 준비 상태 결과 처리. |
| `crates/volicord-core/src/methods/session_watch.rs` | 세션 감시 메서드 계획과 관찰 조율. |
| `crates/volicord-core/src/methods/stage_artifact.rs` | 일시적 아티팩트 스테이징 메서드 처리. |
| `crates/volicord-core/src/policy/` | 접근 점검, 재실행 맥락, Product Repository 경로 정규화, 쓰기 티켓 호환성, 증거 상태, 사용자 행동 관련성, 연속성, 효과 계약, 닫기 준비 상태 계산을 위한 재사용 Core 정책 도우미. |
| `crates/volicord-core/src/methods/tests/` | 메서드 계획기 가까이에 있는 Core 메서드와 파이프라인 테스트. |

## CLI

| 소스 경로 | 책임 |
|---|---|
| `crates/volicord-cli/src/main.rs` | `volicord` 프로세스 진입, 관리 명령 디스패치, `volicord mcp`와 로컬 HTTP 프로세스 모드 인계, 설정 완료 여부 점검, 바이너리 종료 동작. |
| `crates/volicord-cli/src/lib.rs` | 재사용 명령 모듈을 위한 공유 관리 CLI 크레이트 표면. |
| `crates/volicord-cli/src/setup_command.rs`와 `crates/volicord-cli/src/setup_command/` | 설정 명령 진입점, 설정 작업 흐름 실행, 실행 파일 탐색, 명령 링크와 셸 시작 계획, 대화형 선택, 설정 결과 표시. |
| `crates/volicord-cli/src/connection_command.rs`와 `crates/volicord-cli/src/connection_command/` | `volicord init`, `volicord connection add`, `volicord connection list`, `volicord connection status/verify/mode/remove`의 파싱, 구성, 선택, 검증, MCP 프로세스 점검, typed 호스트 기능 지원 진단, 결과 표시. `connection_command/service.rs`는 Store 부트스트랩과 Agent Connection 도우미를 통해 프로젝트와 Agent Connection 구성을 조율합니다. |
| `crates/volicord-cli/src/final_output_command.rs` | 숨겨진 관리 최종 출력 명령, 읽기 전용 바인딩 검증, 새 상태 보기, 완전한 정규 receipt 또는 fallback 계획, Record 최종 출력과 Detective Stop 전달·재생이 공유하는 전체 호스트 응답 바이트 예산 렌더링. |
| `crates/volicord-cli/src/guard_command.rs`와 `crates/volicord-cli/src/guard_command/` | 숨겨진 호스트 이벤트 명령 디스패치, 인수 파싱, 이벤트 정규화, 도구 관찰 추출, 변경 분류, 단계 처리, 프롬프트 캡처와 사용자 행동 명령, 쓰기 티켓 점검, 분리된 Detective Stop 집행과 불변 재생, 과거 Stop 결정과 공유 최신 최종 출력 상태 보기의 합성. |
| `crates/volicord-cli/src/guard_integration/` | `guard` 통합 계획, 생성 파일 적용, 역량 메타데이터와 정책 도우미, 프로필과 무관한 최종 출력 처리기 계획, 더 넓은 Detective 생명주기 계획, 연결 상태와 진단에 쓰는 사실 감사 도우미. |
| `crates/volicord-cli/src/guard_integration/plan.rs` | 호스트 역량, 프로필, 프로젝트, 런타임 사실을 바탕으로 `guard` 통합 계획 조립. |
| `crates/volicord-cli/src/guard_integration/files.rs` | 생성된 `guard` 파일과 관리 정책 파일 계획, 고정된 Product Repository 경로 순회, 대상 스냅샷, 조건부 동일 디렉터리 교체, 연산 후 복구 검사. |
| `crates/volicord-cli/src/guard_integration/apply.rs` | 계획된 `guard` 파일과 관리 상태 보기의 렌더링 및 적용 디스패치. |
| `crates/volicord-cli/src/guard_integration/capability.rs` | 역량 메타데이터와 기록된 `guard` 설치 메타데이터 도우미. |
| `crates/volicord-cli/src/guard_integration/policy.rs` | `guard` 정책 값과 생명주기 단계 도우미. |
| `crates/volicord-cli/src/guard_integration/hooks.rs`와 `crates/volicord-cli/src/guard_integration/hosts/` | 최종 출력 전용 단계 부분 집합과 더 넓은 Detective 생명주기를 위한 호스트 이벤트 명령 및 호스트별 생성 파일 계획. |
| `crates/volicord-cli/src/guard_integration/audit.rs` | 기록된 역량 메타데이터, 생성 파일, 래퍼 스크립트, 훅 명령 경로, 관리 상태 보기에 대한 사실 점검. 이 사실은 진단 관찰이지 보안 보장, 사용자 승인 기록, 정확성 증명이 아닙니다. |
| `crates/volicord-cli/src/doctor_command.rs` | 진단 보고를 위한 설치, 연결, 호스트, `guard` 사실 수집과 공유 typed 호스트 기능 지원 평가 소비. |
| `crates/volicord-cli/src/diagnostics_command.rs` | 내용이 없는 session diagnostics aggregate 선택과 text/JSON 출력. |
| `crates/volicord-cli/src/user_command.rs` | 로컬 User Channel 상태와 `volicord inbox` 명령 파싱 및 조율. 로컬 사용자 호출 사실을 Core의 판단 기록 경로에 전달합니다. |
| `crates/volicord-cli/src/host_integration/` | 호스트 종류, 범위, 역량, 생명주기 단계, 설정 편집, 통합 계약, 동적 호스트 기능 증거와 준비 상태 집계, 프로필과 무관한 최종 출력 고지 역량 계약과 검증, 범용 호스트 fallback 안내, 진단 상태 타입. |
| `crates/volicord-cli/src/host_integration/capability_status.rs` | 공유 정적 결과와 단일 기능 지원 결과에 동적 증거, 최신성, 준비 상태, 설정 입력을 적용해 프로필별 최종 출력과 여섯 기능 진단 매트릭스를 집계합니다. 설정 점검은 별도 입력으로 남으며 지원 상태를 올릴 수 없습니다. |
| `crates/volicord-cli/src/host_integration/contracts.rs` | 관리 호스트 통합 계약 메타데이터와 픽스처·설정 검증. Record profile과 Detective profile이 공유하는 최종 출력 전용 단계 부분 집합을 포함합니다. |
| `crates/volicord-cli/src/host_integration/codex/` | Codex 어댑터 내부 구현. 설정 계획, 실행 파일 점검, 관리 대상 식별 정보, 신뢰 사실, 검증. |
| `crates/volicord-cli/src/host_integration/claude_code/` | Claude Code 어댑터 내부 구현. CLI 명령과 설정 계획, 관리 대상 식별 정보 점검, 호스트 고유 출력 파싱, 검증. |
| `crates/volicord-cli/src/registration.rs` | Runtime Home 기록을 초기화할 때 쓰는 공유 관리 작업 생성자 메타데이터. |
| `crates/volicord-cli/src/project_context.rs` | Product Repository 루트 감지와 `volicord project ...` 명령 조율. |
| `crates/volicord-cli/src/export_command.rs` | 권한 번들 내보내기 명령 파싱과 결과 표시. |
| `crates/volicord-cli/src/changes_command.rs` | 로컬 변경 조정 명령 파싱과 조율. |
| `crates/volicord-cli/src/serve_command.rs` | 로컬 HTTP 서비스 명령 파싱과 서버 설정 인계. |
| `crates/volicord-cli/src/disclosure.rs`, `managed_block.rs`, `shell_path.rs`, `setup_report.rs`, `summary_card.rs` | 고지 문구, 관리 파일 블록, 셸 경로 처리, 설정 보고 데이터, 간결한 상태 요약을 위한 공통 CLI 도우미. |

## MCP 어댑터

| 소스 경로 | 책임 |
|---|---|
| `crates/volicord-mcp/src/lib.rs` | CLI 바이너리가 사용하는 공개 크레이트 표면과 다시 내보낸 어댑터 진입점. |
| `crates/volicord-mcp/src/tool_registry.rs` | Agent Connection 모드별 MCP 노출 도구 메타데이터와 도구 집합. |
| `crates/volicord-mcp/src/repository_discovery.rs` | 형식이 지정된 호스트 선택자와 복제본에서 그대로 쓸 수 있는 정확한 저장소 MCP 기술 정보의 생성 및 검증. |
| `crates/volicord-mcp/src/routing.rs` | Agent Connection 시작 검사, 정규화된 Git 저장소 발견, 고유한 로컬 공유 연결 해석, 프로젝트 가용성과 허용 목록 점검, 요청 시점 프로젝트 선택 도우미. |
| `crates/volicord-mcp/src/adapter.rs` | 형식화된 공개 `tools/call` 디코딩, 어댑터 보조 호출, `operation_category`와 `actor_source` 파생, Core 호출, 응답 래핑 도우미. |
| `crates/volicord-mcp/src/stdio.rs` | JSON-RPC 표준 입출력 프레이밍, 초기화, 응답 래핑, 사용자 입력 요청 처리, 공유 검증을 거친 권한 새로 고침 사용, 명시적 바인딩 및 저장소 발견 표준 입출력 시작, `volicord mcp`가 쓰는 사전 점검 실행기. |
| `crates/volicord-mcp/src/local_http.rs` | 로컬 루프백 HTTP 서버 설정, 엔드포인트 처리 경로, 토큰 처리, 로컬 HTTP MCP 제공. |
| `crates/volicord-mcp/src/local_web_consent.rs` | User Channel 답변을 위한 로컬 웹 동의 요청과 완료 처리. |
| `crates/volicord-mcp/src/http.rs` | 공통 HTTP 파싱과 응답 도우미. |
| `crates/volicord-mcp/src/constants.rs` | MCP 모듈이 공유하는 어댑터 상수. |
| `crates/volicord-mcp/src/errors.rs` | MCP 어댑터와 로컬 HTTP 오류 타입. |
| `crates/volicord-mcp/src/prelude.rs`와 `crates/volicord-mcp/src/util.rs` | 내부 공통 가져오기와 작은 어댑터 보조 함수. |
| `crates/volicord-mcp/src/tests.rs` | 크레이트 내부 MCP 어댑터와 전송 테스트. |

## 테스트와 유지보수 지원

| 소스 경로 | 책임 |
|---|---|
| `crates/volicord-test-support/src/lib.rs` | 폐기 가능한 Runtime Home과 Product Repository 설정, Core용 요청 빌더, 픽스처 전용 Store 검사 도우미, 구현 테스트용 공유 단언. |
| `crates/volicord-cli/tests/support/` | CLI 통합 테스트용 바이너리 픽스처, 모의 호스트와 MCP 프로세스, JSON 도우미, 단언, `guard` 생명주기 픽스처. |
| `crates/volicord-cli/tests/binary_admin.rs` | 설정, 프로젝트, 연결, 상태, 받은 편지함, 사전 점검, 프로필과 무관한 최종 출력 설정, 호스트 설정 동작을 바이너리 수준에서 검증. |
| `crates/volicord-cli/tests/guard_command.rs` | `guard` 훅 생명주기, 프롬프트 캡처, 관찰된 변경, 예상 쓰기, 쓰기 티켓 일치, 공유 권한 검증, 최종 출력 고지와 fallback, 분리된 Stop 집행, 보호된 초기화와 상태를 검증. |
| `crates/volicord-cli/tests/final_output_command.rs` | 최종 출력 이벤트 drain, 개인정보 경계, 최신 권한 상태 보기, 비관찰 Record 동작을 바이너리 수준에서 검증. |
| `crates/volicord-cli/tests/mcp_transport.rs` | `volicord mcp` 하위 명령, `--check`, 표준 입출력 프레이밍, 재연결, MCP 응답 래핑을 검증. |
| `crates/volicord-cli/tests/serve_transport.rs` | 로컬 HTTP 서비스 명령과 전송을 검증. |
| `crates/volicord-cli/tests/live_host_smoke.rs` | 픽스처와 렌더러 테스트가 확립할 수 없는 UI 관찰을 포함해 테스트 환경에서 사용할 수 있을 때 실행하는 선택적 실제 호스트 스모크 테스트. |
| `tests/conformance/baseline.rs` | Core 쪽 API를 통한 교차 메서드 기준 시나리오. |
| `tests/integration/mcp_connection.rs` | MCP, Core, Store, Agent Connection을 가로지르는 동작 검증. |
| `tests/integration/public_contract_snapshots.rs`와 `tests/integration/snapshots/` | 공개 스키마와 MCP 도구 계약 스냅샷 검증. |
| `tests/release-validation/src/candidate.rs`와 `tests/release-validation/src/bin/host-release-candidate.rs` | 이미 외부에 배치한 정확한 최종 후보의 create-new 설명자 생산. 패키지가 공유하는 경로 및 후보 불변조건 평가를 사용합니다. |
| `tests/release-validation` | [호스트 릴리스 증거](../reference/host-release-evidence.md)가 담당하는 정확한 외부 후보 설명자 생산과 셀·manifest·audit 검증, 고정 12개 셀 게이트, 별도 프로세스 재계산. 운영 crate가 의존하면 안 됩니다. |
| `xtask/src/main.rs`와 `xtask/src/lib.rs` | 문서 검증을 포함한 읽기 전용 저장소 유지보수 명령. |

이 소스 설명은 구현 배치 지침입니다. 이 지도와 집중 참조 담당 문서가 제품
동작에 대해 어긋나 보이면, 소스 배치에서 제품 계약을 추론하지 말고 담당 경로
공백이나 구현 공백으로 다룹니다.
