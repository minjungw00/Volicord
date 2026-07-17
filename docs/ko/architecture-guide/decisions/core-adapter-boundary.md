# Core와 어댑터 의존성 경계

## 맥락

같은 workflow에 로컬 CLI와 관리 stdio MCP 프로세스가 접근합니다. transport 세부사항,
사용자 인터페이스 관심사, host 구성이 Core 권한 의미를 바꾸면 안 됩니다.

## 결정

`volicord-core`는 공유 타입과 Store-facing interface에 의존하며 CLI나 MCP crate에는
의존하지 않습니다. Core는 공통 사전 점검, 구조 검증 순서, 메서드 계획, replay, 정책,
응답 구성, commit 선택을 소유합니다.

MCP 어댑터는 stdio 생명주기, JSON-RPC 프레이밍, 도구 metadata, 공개 인수 디코딩,
서버 소유 invocation context, 안전한 projection을 소유합니다. CLI는 관리 명령 파싱,
Codex 설정, 진단, CLI 받은 편지함 표시, local-user provenance를 소유합니다. 둘 다 typed
Core-facing interface를 호출합니다.

Store는 엄격한 저장 레코드 검증과 transaction 적용을 소유합니다. 어댑터 입력에서 제품
계약을 추론하지 않습니다.

## 결과

- 공개 인수는 connection, actor, project 권한을 주입할 수 없습니다.
- 어댑터 projection은 Core 결과를 넓힐 수 없습니다.
- MCP는 UserAction 요청을 생성하거나 재개할 수 있지만 해결할 수 없습니다.
- 현재 typed 계약에 실패한 저장 owner data는 어댑터 availability 실패가 아니라
  corrupt-data 실패입니다.
- 새 어댑터 동작에는 구현 전 owner-defined 계약이 필요합니다.

[요청 생명주기](../request-lifecycle.md),
[MCP Transport](../../reference/mcp-transport.md),
[Failure Model](../../reference/failure-model.md)을 함께 봅니다.
