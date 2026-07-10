# Runtime Home과 Product Repository 분리

## 맥락

Volicord에는 Runtime Home 기록, 프로젝트 상태, registry 메타데이터,
아티팩트 데이터, 운영 설정을 위한 로컬 위치가 필요합니다. 사용자의 제품
파일은 `Product Repository`에 있습니다. 두 위치를 섞으면 구현 경로를
이해하기 어려워지고 생성된 런타임 상태가 제품 작업처럼 보일 수 있습니다.

Volicord 소스와 설치 파일은 별도의 구현 아티팩트 역할입니다. 그 안에
`volicord` 실행 파일과 `volicord-mcp` 같은 구현 크레이트가 있거나 배포될 수 있지만,
정의상 Runtime Home이나 Product Repository는 아닙니다.

## 결정

구현은 `Volicord Runtime Home`과 `Product Repository`를 별도 위치 개념으로
유지합니다.

- Store 코드는 Runtime Home 경로 처리, registry/project 데이터베이스,
  프로젝트 Store 접근, 스키마 초기화와 검증, 검사, Runtime Home 아래 아티팩트
  데이터를 맡습니다.
- CLI 연결 프로비저닝은 Product Repository 경로를 Runtime Home 기록에
  등록하지만 그 저장소를 런타임 상태로 만들지 않습니다.
- CLI 설정과 MCP 시작은 Volicord 설치 파일을 참조할 수 있지만, 설치
  위치가 Runtime Home이나 Product Repository가 되지는 않습니다.
- Core 메서드 코드는 메서드 담당 문서가 그런 입력을 정의할 때 Product
  Repository 경로를 정규화하고 판단할 수 있지만, 공개 API 실행이 제품
  파일을 직접 쓰지는 않습니다.

## 결과

- 폐기 가능한 테스트는 유지 문서나 사용자 제품 데이터에 쓰지 않고 임시
  디렉터리 아래 Runtime Home 상태를 만들 수 있습니다.
- Store와 CLI 설정 코드는 Runtime Home 상태를 Product Repository 파일
  경로와 Volicord 실행 파일 경로와 별도로 검증할 수 있습니다.
- 제품 파일 쓰기는 공개 Volicord API 경로 밖에 남고, Core는 담당 문서가
  정의한 동작으로 호환성, 관찰 사실, 아티팩트 링크, 권한 상태를 기록할 수
  있습니다.
- 문서와 테스트는 런타임 홈, SQLite 데이터베이스, 생성 로그, 아티팩트
  출력을 유지 문서에 저장하지 않아야 합니다.

## 비목표

- 이 결정은 보안 격리를 정의하지 않습니다.
- Runtime Home 위치가 권한 증거가 된다고 정의하지 않습니다.
- 필수 Volicord 설치 루트를 정의하지 않습니다.
- Product Repository 경로 정규화 규칙을 정의하지 않습니다. 그 규칙은
  런타임 경계 담당 문서가 맡습니다.
- 저장소 기록 배치, DDL, 아티팩트 생명주기 규칙을 정의하지 않습니다.

## 관련 구현

- [`crates/volicord-store/src/runtime_home.rs`](../../../../crates/volicord-store/src/runtime_home.rs):
  Runtime Home 해석.
- [`crates/volicord-store/src/bootstrap.rs`](../../../../crates/volicord-store/src/bootstrap.rs):
  Runtime Home 초기화와 프로젝트 등록.
- [`crates/volicord-store/src/agent_connections.rs`](../../../../crates/volicord-store/src/agent_connections.rs):
  Agent Connection 기록과 Connection Project 멤버십.
- [`crates/volicord-store/src/core_pipeline.rs`](../../../../crates/volicord-store/src/core_pipeline.rs):
  `CoreProjectStore` 프로젝트 로컬 Store 접근.
- [`crates/volicord-store/src/artifacts.rs`](../../../../crates/volicord-store/src/artifacts.rs):
  Runtime Home 아티팩트 스테이징과 영구 본문 검증.
- [`crates/volicord-cli/src/connection_command/service.rs`](../../../../crates/volicord-cli/src/connection_command/service.rs):
  연결 프로비저닝, Runtime Home 준비, 프로젝트 등록, Agent Connection 멤버십
  오케스트레이션.
- [`crates/volicord-core/src/policy/path.rs`](../../../../crates/volicord-core/src/policy/path.rs):
  Core 정책에서 쓰는 Product Repository 경로 정규화 도우미.

## 관련 테스트와 참조 담당 문서

- [`crates/volicord-cli/tests/binary_admin.rs`](../../../../crates/volicord-cli/tests/binary_admin.rs)의
  `init_dry_run_does_not_write_runtime_or_repo_files`,
  `init_rejects_invalid_profile_without_artifacts`,
  `init_codex_guarded_writes_policy_mcp_and_guard_status_idempotently`,
  `project_commands_use_current_git_repository_without_user_ids`는 초기화,
  쓰기 없는 실행, 프로젝트 선택 동작을 각각 확인합니다.
- [`crates/volicord-test-support/src/lib.rs`](../../../../crates/volicord-test-support/src/lib.rs)의
  `disposable_runtime_home_stays_under_system_temp`.
- 계층 간 호출 맥락 동작은
  [`tests/integration/mcp_connection.rs`](../../../../tests/integration/mcp_connection.rs)의
  `read_only_mode_rejects_agent_workflow_methods_before_core`.
- [런타임 경계](../../reference/runtime-boundaries.md),
  [저장소](../../reference/storage.md), [아티팩트 저장소](../../reference/storage-artifacts.md),
  [관리 CLI](../../reference/admin-cli.md), [보안](../../reference/security.md).
