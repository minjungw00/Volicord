# 호스트 기능 지원 상태 평가

## 맥락

기존 호스트 통합은 구현 존재 여부, 생성 파일 설정 점검, 실제 호스트 증거라는 서로 다른
세 사실을 `supported`, `verified` 같은 boolean 이름으로 합쳤습니다. 그 결과 설정
픽스처가 정확한 설치 호스트와 최종 Volicord 아티팩트로 입증하지 않은 동작을 검증한 것처럼
보일 수 있었습니다. 오래된 결과가 관찰 당시의 바이너리, 호스트 버전, 어댑터 프로필,
런타임 전제 조건보다 오래 남을 수도 있었습니다.

이 모호함은 호스트 고유 사용자 행동, local-web 전달, 증거 producer 경로, 등록 연결 관찰,
두 최종 출력 프로필 모두에 영향을 줬습니다. Doctor, 연결 상태, 릴리스 검증이 같은 사실을
서로 다르게 해석할 수도 있었습니다.

## 결정

Agent Connection과 API 값 집합 담당 문서가 정의한 여섯 기능 ID와 최종 출력 하위 역량에
공유 정적 구현 평가기 하나를 사용합니다. 공유 타입은 닫힌 식별자, 검토된
호스트·버전·클라이언트 사실, 정규 Codex 버전 문법, 버전별 구현 매트릭스, 정확한 증거와
현재 준비 상태를 반영하는 단일 기능 지원 상태 우선순위를 담당합니다. CLI 진단, MCP 전달
자격, 릴리스 검증은 호스트 종류 fallback, 버전 표, 기능 상태 우선순위를 다시 만들지 않고
이 결과를 소비해야 합니다.

CLI는 공유 단일 기능 결과를 여섯 기능 매트릭스와 각 최종 출력 프로필에 걸쳐 집계합니다.
정확한 실제 증거와 현재 런타임 준비 상태는 별도 입력으로 남습니다. 설정은 `configured`와
`configuration_verified`로만 보고하며 지원 상태를 올리지 않습니다. MCP는 저장된 검증을
조회하기 전에 정적으로 지원되지 않는 local-web 표면을 거부하고, 전달 lease를 발급할 때도
같은 점검을 반복해야 합니다.

Doctor, 연결 상태, 릴리스 기능 매트릭스는 같은 평가 결과를 사용합니다. 정확한 재생도
평가기를 다시 실행합니다. 증거는 현재 최종 Volicord 아티팩트, source revision, 빌드와
target, 설치 호스트와 버전, 어댑터 프로필, 연결 신원, 증거 아티팩트, 최신성 구간에
결속되어야 합니다. 증거가 없거나, 오래됐거나, 만료됐거나, 형식이 잘못됐거나, 일치하지
않는 상태를 일시적인 런타임 중단으로 취급할 수 없습니다.

최종 출력 진단은 `support_status`, 설정 사실, 정확한 프로필별
`required_subcapabilities`, 적용되는 `authority_display`,
`authenticated_exact_replay`, `block_finalization` 상태만 담는 map을 사용합니다. 구현과
설정이 있으면 민감하지 않은 표시가 최선형으로 동작할 수 있지만 typed 상태는 검증되지
않았거나 지원되지 않은 상태로 남습니다. 이는 지원 또는 릴리스 주장을 성립시키지 않습니다.

저장된 `host_capability_json` 내부 스키마는 v1에서 v2로 이동합니다. V2는 모호한
`final_output_authority_disclosure_supported` boolean 대신 명시적인 구현·설정 사실을
기록합니다. 이전 v1 기록은 현재 진단 입력으로 유효하지 않으며 지원되는 init 절차를 다시
실행해 복구합니다. 이전 boolean에서 대체 상태를 추론하지 않습니다.

## 결과

- 구현된 기능을 검증됐다고 표현하지 않고도 보고할 수 있습니다.
- 알려진 호스트 소유 표면 부재와 일시적 중단을 구분할 수 있습니다.
- 설정 점검은 실제 호스트 증거가 되지 않은 채 유용한 사실로 남습니다.
- 호스트, 바이너리, 어댑터, 증거, 최신성 불일치는 과거 통과를 물려받지 않고 정확한 기능을
  낮춥니다.
- Record와 Detective 최종 출력 주장은 서로 다른 필수 하위 역량을 드러내면서 최신 권한
  projection 코드는 계속 공유합니다.
- MCP, CLI 진단, 릴리스 검증은 같은 정규 호스트·버전·기능에 서로 다른 정적 구현 상태를
  부여할 수 없습니다.

## 공개 및 진단 호환성

이는 첫 주요 릴리스 전 `0.9.0`의 의도적인 진단 계약 변경입니다. 이전 최종 출력
`supported`, `verified` 필드와 `native_host_output_adapter_verified` 이름은 별칭으로
남기지 않고 제거합니다. 설정 전용 대체 이름은
`native_host_output_adapter_config_verified`입니다. 소비자는 `support_status`를 읽어야
하며 설정 필드에서 이를 추론하면 안 됩니다.

저장된 v2 capability JSON 변경에는 v1 호환 decoder나 합성 migration이 없습니다. Init을
다시 실행해 현재 관리 기록과 파일을 재생성합니다. 공개 Core 메서드 입력에는 호스트 지원
selector를 추가하지 않으며, 지원 상태는 Core 권한이나 새 공개 메서드를 만들지 않습니다.

## 비목표

- 현재 Codex 또는 Claude Code 실제 셀이 통과했다고 주장하지 않습니다.
- 픽스처, 생성 래퍼, 무시된 테스트, 과거 결과 파일을 실제 호스트 증거로 바꾸지 않습니다.
- 호스트 attestation, 사용자 신원, OS 집행, 보안 증명을 제공하지 않습니다.
- 최선형 표시를 지원되는 재생 또는 block 최종화와 같게 만들지 않습니다.

## 거부한 대안

- 별도 `supported`, `configured`, `verified` boolean을 유지하는 방안은 의미가 겹치고
  설정이 동작 검증처럼 보일 수 있어 거부했습니다.
- 증거 누락을 `temporarily_unavailable`로 취급하는 방안은 일시 상태가 정확한 현재 증거를
  전제로 하므로 거부했습니다.
- V1 별칭을 유지하거나 이전 capability JSON에서 v2 상태를 추론하는 방안은 모호한 계약을
  보존하므로 거부했습니다.
- 각 명령이 독립적으로 지원 상태를 계산하는 방안은 우선순위, 최신성, 하위 역량 집계가
  달라질 수 있어 거부했습니다.
- 검토된 버전 매트릭스를 CLI에 두는 방안은 MCP와 테스트 전용 릴리스 검증이 의존 방향을
  뒤집거나 결정을 복제하지 않고 어댑터 바이너리 크레이트에 의존할 수 없으므로 거부했습니다.

## 관련 구현 영역

- [`crates/volicord-types/src/host_feature_support.rs`](../../../../crates/volicord-types/src/host_feature_support.rs):
  닫힌 식별자, 검토된 호스트·버전·클라이언트 사실, 정규 파싱, 공유 정적 구현 매트릭스,
  단일 기능 상태 우선순위.
- [`crates/volicord-cli/src/host_integration/capability_status.rs`](../../../../crates/volicord-cli/src/host_integration/capability_status.rs):
  공유 정적 결과와 단일 기능 지원 결과에 프로필별 최종 출력과 여섯 기능 진단 집계를 적용.
- [`crates/volicord-mcp/src/adapter.rs`](../../../../crates/volicord-mcp/src/adapter.rs):
  같은 정적 결과를 쓰는 최초 및 발급 시점 local-web 전달 자격 점검.
- [`tests/release-validation`](../../../../tests/release-validation):
  같은 정적 결과를 쓰는 정확한 아티팩트 주장 평가.
- [`crates/volicord-cli/src/connection_command.rs`](../../../../crates/volicord-cli/src/connection_command.rs)와
  [`doctor_command.rs`](../../../../crates/volicord-cli/src/doctor_command.rs): 진단 소비자.
- [`crates/volicord-cli/src/guard_integration/`](../../../../crates/volicord-cli/src/guard_integration/):
  v2 capability 메타데이터와 설정 audit 사실.
- [`crates/volicord-cli/tests/live_host_smoke.rs`](../../../../crates/volicord-cli/tests/live_host_smoke.rs):
  제품 상태와 분리된, 크기가 제한된 정확한 아티팩트 증거 입력.

## 참조 담당 문서

정확한 상태 값과 형태는 [API 값 집합](../../reference/api/schema-value-sets.md)과
[API 상태 스키마](../../reference/api/schema-state.md)가 담당합니다. 평가, 호스트 기준 상태,
재생, 최종 출력 하위 역량은 [Agent Connection](../../reference/agent-connection.md)이
담당합니다. 관리 출력과 동일 신원 복구 및 마이그레이션 거부의 구분은
[관리 CLI](../../reference/admin-cli.md)가 담당합니다. 정확한 닫힌 저장 v2 capability 형태,
의미 관계, 소유자 바인딩은 [저장소 기록](../../reference/storage-records.md)이 담당하고,
방어적인 필수 단계 projection 규칙은 API 상태 스키마가 담당합니다. 환경 전제 조건은
[시스템 요구 사항](../../reference/system-requirements.md)이 담당합니다.
