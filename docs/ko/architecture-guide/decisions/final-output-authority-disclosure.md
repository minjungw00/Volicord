# 최종 출력 권한 고지

## 맥락

Core는 `volicord.status`와 함께 최신 `AuthorityReceipt`를 반환할 수 있지만,
에이전트 턴이 끝날 때 현재 receipt가 유용하려면 모델이 작성한 문장이 이를
보존하거나 해석하게 하지 않고 호스트가 직접 노출해야 합니다. 기존 소스 구조에서
사용자에게 보이는 receipt 경로는 Detective profile의 Stop 훅에 결합되어 있습니다.
이 훅은 검증된 receipt를 별도 호스트 UI 표면에 놓을 수 있지만, allow 또는 deny
결정은 최종 출력 고지와 다른 책임이며 Record profile에서는 사용할 수 없습니다.

MCP 변경 결과 새로 고침과 CLI Stop 단계는 같은 status/receipt 관계를 서로 인접하지만
별도인 코드로 검증합니다. 검증기가 나뉘면 담당 문서가 정의한 식별 정보, 상태, Task,
범위, Change Unit, 증거, 닫기 사실 중 서로 다른 일부만 받아들이도록 어긋날 수
있습니다.

이 결정은 구현 구조를 설명합니다. 정확한 receipt, 새로 고침, 호스트 지원, fallback,
출력 동작은 계속 집중 참조 담당 문서가 정의합니다.

## 결정

Detective Stop 집행과 분리된 어댑터 독립 최종 출력 권한 고지 경로 하나를
사용합니다.

1. 지원되는 최종 출력 이벤트는 이벤트 시점에 선택된 프로젝트와 Task의 최신 읽기 전용
   `volicord.status` 결과를 요청합니다. 재실행한 과거 작업 결과, 앞선 변경 응답, 앞선
   Stop 결과를 최신성 출처로 사용하지 않습니다.
2. Core 소유 형식 검증기는 status 결과와 후보 `AuthorityReceipt`를 함께 받습니다.
   소비자가 고른 일부가 아니라 집중 담당 문서가 요구하는 모든 대응 관계를 확인합니다.
   MCP 변경 결과 새로 고침, Detective Stop, 최종 출력 고지는 이 검증기를 함께
   사용합니다. 검증기는 호스트별 문장이 아니라 형식화된 검증 사실을 반환합니다.
3. CLI는 검증된 결과를 메모리 내 고지 계획 하나로 바꿉니다. 선택된 Task가 있으면 전체
   정규 receipt 또는 크기가 제한된 Task별 `volicord status` fallback을 사용합니다.
   Task가 없거나 새로 고침이 실패했거나 결과가 잘못되었거나 일치하지 않으면 담당
   문서가 정의한 명시적 fallback 또는 진단 결과가 됩니다. receipt를 만들어 내거나
   일부만 출력하지 않습니다.
4. 내장 Codex와 Claude Code 어댑터 경로는 이 계획을 호스트 고유 고정 UI 표면으로
   변환합니다. 프로필과 무관한 역량 및 관리 설정 계획을 통해 Record profile과
   Detective profile 모두에서 고지 경로를 사용할 수 있게 합니다. 범용, 사용자 관리,
   미지원, 비활성, 저하된 경로는 참조 담당 문서가 지원하는 fallback과 진단 동작만
   노출합니다. 호스트 표면이 설치되거나 관찰되었다고 주장하지 않습니다.
5. Detective Stop은 별도 소비자로 남습니다. 호스트 고유 allow 또는 deny 결과를
   결정할 때 같은 최신 검증 사실을 사용할 수 있지만, 그 집행 결과가 기준 최종 출력
   표시가 되지는 않습니다. Record profile 고지는 차단하지 않습니다.

고지 계획은 어댑터 상태 보기이며 새로운 Core 결과, 공개 스키마, Store 기록, 권한
출처가 아닙니다. 최종 출력 이벤트마다 다시 만들고 현재 권한으로 저장하거나 캐시하지
않으며, 모델 문장이나 생성 안내가 제공하지 않습니다. 이 결정은 새 공개 API 메서드,
테이블, 저장소 migration을 도입하지 않습니다.

## 결과

- 최종 출력 권한 고지는 Detective profile 활성화나 Stop allow 경로 도달에 의존하지
  않습니다.
- MCP 새로 고침, Stop 집행, 최종 출력 표시는 receipt/status 대응 관계 확인을 서로
  독립적으로 약화할 수 없습니다.
- 고지 실패는 더 오래된 권한을 조용히 재사용할 수 없고 UI 출력을 만들기 위해 Core
  상태를 바꾸지 않습니다.
- 호스트 바이트 한계는 정규 JSON을 자르거나 receipt 필드를 버리지 않고 전체 receipt와
  fallback 중 하나를 선택해 처리합니다.
- 호스트 설정 픽스처와 렌더러 테스트는 생성된 어댑터 바이트를 확인합니다. 실제 Codex
  또는 Claude Code 버전이 의도한 UI 표면을 불러오고 표시하고 보존했다는 사실은
  증명하지 않습니다. 그 증거에는 실제 호스트 실행이 필요합니다.
- 정확한 공개 동작, 지원 환경, 명령 문장, 바이트 상한, 비보장은 집중 참조 담당 문서에
  남습니다.

## 호환성과 migration

이 결정은 첫 major 이전의 의도된 `0.8.0` clean-break batch에 포함됩니다. 기존 관리
Record profile과 Detective profile의 관리 호스트 상태 보기는 지원되는 정확한 init 작업
흐름을 다시 실행해 재생성해야 합니다. 구현은 두 번째 legacy 최종 출력 경로, 호환
별칭, 캐시한 Stop receipt로 되돌아가는 fallback을 유지하지 않습니다. 기존 사용자 관리
또는 충돌하는 호스트 설정을 덮어쓰지 않으며, 관리 작업 흐름이 담당 문서가 정의한
충돌이나 복구 작업을 보고합니다.

이 변경은 공개 API나 저장소 형태가 아니라 어댑터 동작과 관리 호스트 설정을
추가합니다. 따라서 Runtime Home 변환이나 스키마 baseline 변경이 필요하지 않습니다.
버전 영향은 이 cluster에 별도 버전 증가를 부여하지 않고 관련 `0.8.0` 공개 계약
batch에서 한 번 평가합니다.

## 거부한 대안

- 안내나 모델이 작성한 최종 문장이 receipt를 반복하게 하는 방식은 생성 문장이 고정
  호스트 UI 표면도 권한 출처도 아니므로 거부했습니다.
- 최신 변경, replay, Stop receipt를 재사용하는 방식은 과거 receipt가 현재 상태가
  아니므로 거부했습니다.
- MCP, Stop, 최종 출력 검증기를 별도로 유지하는 방식은 받아들이는 필드 집합과 실패
  동작이 서로 어긋날 수 있으므로 거부했습니다.
- Detective profile을 요구하는 방식은 고지와 집행의 책임이 다르므로 거부했습니다.
- 최종 출력 receipt를 저장하거나 새 공개 메서드 또는 테이블을 추가하는 방식은 표시가
  읽기 전용 현재 status에서 파생되므로 거부했습니다.
- receipt를 자르는 방식은 일부만 남은 정규 권한 JSON의 의미가 바뀌므로 거부했습니다.
  크기가 제한된 fallback이 정확한 조회 경로를 보존합니다.
- 생성 픽스처 출력을 실제 호스트 UI 동작의 증거로 다루는 방식은 호스트 로딩과 표시가
  계속 외부 관찰이므로 거부했습니다.

## 관련 구현과 테스트

- [`crates/volicord-core/src/methods/status.rs`](../../../../crates/volicord-core/src/methods/status.rs):
  읽기 전용 status 구성.
- [`crates/volicord-core/src/authority_status.rs`](../../../../crates/volicord-core/src/authority_status.rs):
  공유 형식 status/receipt 검증 경계.
- [`crates/volicord-mcp/src/stdio.rs`](../../../../crates/volicord-mcp/src/stdio.rs):
  공유 검증기를 사용하는 소비자 중 하나인 기준 변경 결과 새로 고침.
- [`crates/volicord-cli/src/final_output_command.rs`](../../../../crates/volicord-cli/src/final_output_command.rs):
  관리 바인딩 검증, 최신 고지 상태 보기, 완전한 receipt 또는 fallback 계획, 전체 응답
  바이트 예산 렌더링.
- [`crates/volicord-cli/src/guard_command.rs`](../../../../crates/volicord-cli/src/guard_command.rs),
  [`guard_command/phase/stop.rs`](../../../../crates/volicord-cli/src/guard_command/phase/stop.rs),
  [`guard_command/render.rs`](../../../../crates/volicord-cli/src/guard_command/render.rs):
  숨겨진 Detective 이벤트 조율, 분리된 Stop 집행과 재생, 과거 결정 렌더링, 공유 최신 고지
  상태 보기와의 합성.
- [`crates/volicord-cli/src/host_integration/`](../../../../crates/volicord-cli/src/host_integration/):
  Codex와 Claude Code 어댑터 경계를 포함한 프로필과 무관한 호스트 역량 계약과
  설정·픽스처 검증.
- [`crates/volicord-cli/src/guard_integration/hosts/`](../../../../crates/volicord-cli/src/guard_integration/hosts/):
  최종 출력 전용 단계 부분 집합과 더 넓은 Detective 생명주기를 위한 호스트별 생성 처리기
  계획.
- [`crates/volicord-cli/tests/guard_command.rs`](../../../../crates/volicord-cli/tests/guard_command.rs),
  [`final_output_command.rs`](../../../../crates/volicord-cli/tests/final_output_command.rs),
  [`binary_admin.rs`](../../../../crates/volicord-cli/tests/binary_admin.rs): 검증기, 새로 고침,
  렌더링, 프로필, 설정, 실패, fallback을 검증합니다.
- [`crates/volicord-cli/tests/live_host_smoke.rs`](../../../../crates/volicord-cli/tests/live_host_smoke.rs):
  픽스처 전용 단언과 구분되는 선택적 실제 호스트 증거.

## 참조 담당 문서

정확한 계약은 [Status](../../reference/api/method-status.md),
[API 상태 스키마](../../reference/api/schema-state.md),
[상태 보기와 템플릿](../../reference/projection-and-templates.md),
[템플릿 본문](../../reference/template-bodies.md),
[Agent Connection](../../reference/agent-connection.md),
[관리 CLI](../../reference/admin-cli.md), [보안](../../reference/security.md),
[범위](../../reference/scope.md), [시스템 요구 사항](../../reference/system-requirements.md)에
남습니다. 대표 기준은 [적합성](../../reference/conformance.md)에 남습니다.
[Core와 어댑터 의존 경계](core-adapter-boundary.md)도 함께 봅니다.
