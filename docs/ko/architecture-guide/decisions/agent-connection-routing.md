# Agent Connection과 호스트 라우팅

## 맥락

Volicord는 Codex와 Claude Code의 설정을 관리하고, 일반 MCP 호스트에는 사용자가
직접 설정할 수 있는 안내를 제공해야 합니다. 하나의 런타임 홈에 여러
`Product Repository`(제품 저장소)가 등록될 수도 있습니다. MCP 루트와 시작
디렉터리는 호스트가 제공하는 힌트일 뿐입니다. Volicord 권한이 아니므로 그
정보만으로 프로젝트를 안전하게 선택할 수 없습니다.

## 결정

Volicord는 로컬 MCP 호스트 연결 하나를 나타내는 지속 레지스트리 단위로
`Agent Connection`(에이전트 연결)을 사용합니다. `volicord mcp --stdio`
프로세스는 `--connection <connection_id>`로 시작합니다. 생성된 호스트 항목이
연결 프로젝트 하나에 안전하게 묶이면 `--project <project_id>`도 사용할 수
있습니다. 여러 프로젝트를 다루는 연결은 프로세스 시작 때 프로젝트를 고정하지
않습니다. 도구를 호출할 때마다 프로젝트를 선택하고 검증합니다.

이 설계는 아래 책임을 분리합니다.

- 레지스트리는 Agent Connection 식별자, 호스트 종류와 범위, 대상 메타데이터,
  연결 모드, 활성 상태, 검증 상태, 명시적인 `Connection Projects`(연결 프로젝트)
  멤버십을 저장합니다.
- `volicord mcp --stdio`는 시작할 때 Agent Connection을 검증하고 현재 연결
  맥락을 파생합니다. 연결 모드에 맞는 MCP 도구를 노출하고,
  `volicord.list_projects`를 제공하며, 모호한 프로젝트 선택을 거부합니다.
- 관리 CLI는 지원되는 호스트 연결 설정을 만들고, 검증하고, 갱신하고,
  제거합니다.
- 호스트 신뢰와 프로젝트 승인, OAuth, 다시 불러오기, 재시작, 모델 동작은 외부
  호스트와 사용자가 담당합니다.

## 결과

- 사용자 범위 호스트 설정은 등록된 모든 프로젝트를 허용하지 않고도 명시적으로
  연결된 여러 프로젝트를 다룰 수 있습니다.
- 여러 프로젝트를 다루는 MCP 명령이 이미 같은 `connection_id`를 가리킨다면,
  연결 프로젝트를 추가하거나 제거할 때 명령을 다시 쓸 필요가 없습니다. 프로젝트
  바인딩이 바뀌면 프로젝트에 묶인 생성 항목은 다시 생성될 수 있습니다.
- 프로젝트 선택이 없거나 모호하면 어댑터가 일관된 오류를 보고합니다. 에이전트는
  연결 프로젝트 목록을 확인하라는 안내를 받을 수 있습니다.
- 프로젝트에 묶인 시작 경로는 도구 처리 전에 `session-watch` 기준 상태를 만들 수
  있습니다. 여러 프로젝트를 다루는 시작 경로는 프로젝트가 명시적으로 선택될
  때까지 관찰 범위를 `pending`으로 보고합니다.
- 호스트 설정 상태는 호스트의 후속 작업을 기다리는 상태와 검증이 끝난 상태를
  구분할 수 있습니다.
- 프로젝트 범위의 생성 설정에는
  `volicord mcp --stdio --connection <connection_id> --project <project_id>`를
  우선 사용합니다. 연결 맥락이나 행위자 출처를 전달하는 환경 변수는 필요하지
  않습니다. 여러 연결 프로젝트를 의도적으로 다루는 흐름에서는 프로젝트를
  지정하지 않은 연결 전용 항목을 사용합니다.

## 비목표

- 이 결정은 공개 Volicord API 메서드를 추가하지 않습니다.
- CLI 명령을 공개 API 메서드로 만들지 않습니다.
- MCP 루트, 현재 작업 디렉터리, 호스트 라벨, 복사된 `connection_id` 값을
  Volicord 권한으로 만들지 않습니다.
- 사용자 범위 연결에 등록된 모든 프로젝트를 부여하지 않습니다.
- 저장소 안내, MCP 서버 지침, 호스트 규칙 파일이 모델 동작을 강제한다고
  정의하지 않습니다.
- Volicord 런타임 상태, SQLite 데이터베이스, 생성 로그, QA 결과, 수락 기록, 닫기 준비 상태, 잔여 위험 기록을 `Product Repository`에 둘 수 있게 하지 않습니다.

## 관련 구현 영역

- [`crates/volicord-mcp`](../../../../crates/volicord-mcp): 연결에 묶인 시작,
  MCP 초기화, 도구 탐색, 프로젝트 선택, Core 호출 전 어댑터 검증.
- [`crates/volicord-cli`](../../../../crates/volicord-cli): 공개 `volicord mcp`
  프로세스 진입점, 호스트 설정 명령 생성, 연결·상태·검증·제거 관리 흐름.
- [`crates/volicord-store`](../../../../crates/volicord-store): 레지스트리 스키마
  초기화와 검증, Agent Connection 기록, 연결 프로젝트 멤버십, Runtime Home 접근.
- 위 크레이트가 저장 값 집합과 기계 판독용 관리 출력에 사용하는 공유 타입.

## 관련 테스트와 참조 담당 문서

이 설계의 테스트는 시작 검증, 프로젝트 선택, 멤버십 취소, 호스트 설정 상태,
프로젝트 범위의 저장소 쓰기 승인, 관리 마커 교체, 지원하지 않는 시작 형태의
거부를 다뤄야 합니다.

참조 담당 문서:

- [Agent Connection](../../reference/agent-connection.md)
- [MCP 전송](../../reference/mcp-transport.md)
- [관리 CLI](../../reference/admin-cli.md)
- [런타임 경계](../../reference/runtime-boundaries.md)
- [저장소 기록](../../reference/storage-records.md)
- [저장소 DDL](../../reference/storage-ddl.md)
- [저장소 버전 관리](../../reference/storage-versioning.md)
- [보안](../../reference/security.md)
