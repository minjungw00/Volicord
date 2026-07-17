# 검증

유지 문서를 편집한 뒤에는 이 정책을 사용합니다. 이 문서는 구조 점검, 사람이
하는 의미 검토, Rust 구현 검증, 결과 보고를 구분합니다.

이 검증은 유지보수 검증입니다. Volicord 런타임 적합성, 제품 수락, QA 완료, 닫기
준비 상태, 보안 증명, 잔여 위험 수락이 아닙니다. 저장소 로컬 자동 문서
검증기는 아래 명령입니다.

```sh
cargo run -p xtask -- docs-check
```

## 구조 점검

문서 메타데이터, 경로, 링크, 용어 경로를 바꿨다면 저장소 루트에서
`cargo run -p xtask -- docs-check`를 실행합니다. 이 명령은 읽기 전용이며
기계로 확인할 수 있는 형태를 검증합니다.

- `docs/doc-index.yaml`이 YAML로 파싱되고 `version: 3`을 갖습니다.
- 필요한 최상위 섹션이 있으며 지원되지 않는 최상위 필드는 거부됩니다.
- `owner_areas` 카탈로그와 `applicability` 카탈로그는 안정적인 식별자와 문자열
  설명을 사용합니다. 정확히 하나의 적용 가능성 항목이
  `version_source: workspace_package`를 사용하여 현재 작업 공간 패키지 버전
  설명을 표시합니다.
- 루트 `Cargo.toml`이 TOML로 파싱되고 `[workspace.package].version`이 문자열이며,
  표시된 적용 가능성 설명이 같은 버전을 식별하는지 확인합니다. 다른 곳의 이전
  릴리스 참조는 비교하지 않습니다.
- 모든 공유 항목은 `doc_id`, `path`, `kind`, `summary`, `normative_level`,
  `owner_area`, `created_on`, `last_updated_on`, `last_verified_on`,
  `applies_to`, `primary_audience`, `journeys`, `canonical_for`,
  `depends_on`만 사용합니다.
- 모든 대응 항목은 `doc_id`, `path_en`, `path_ko`, `kind`, `summary`,
  `normative_level`, `translation_policy`, `owner_area`, `created_on`,
  `last_updated_on`, `last_verified_on`, `applies_to`, `primary_audience`,
  `journeys`, `canonical_for`, `depends_on`만 사용합니다.
- 공유 항목과 대응 항목에 필요한 필드가 있습니다.
- `owner_area`는 최상위 담당 영역 카탈로그로 해석됩니다.
- `applies_to`는 비어 있지 않고 중복이 없는 목록이며 모든 값이 최상위 적용
  가능성 카탈로그로 해석됩니다.
- `created_on`, `last_updated_on`, `last_verified_on`은 유효한
  `YYYY-MM-DD` 달력 날짜이며 `created_on <= last_updated_on <= last_verified_on`
  순서를 지킵니다.
- `kind` 값은 `landing`, `tutorial`, `how_to`, `explanation`, `reference`,
  `maintenance`만 사용합니다.
- `normative_level` 값은 `contract`, `guide`, `example`, `maintenance`만
  사용합니다.
- 유지되는 영어/한국어 대응 쌍의 `translation_policy`는
  `semantic_parity`입니다.
- `primary_audience`, `journeys`, `canonical_for`, `depends_on`은 있을 때
  목록입니다.
- `doc_id` 값은 고유합니다.
- 색인된 모든 경로가 존재합니다.
- 모든 `depends_on` 값이 색인된 `doc_id`로 해석됩니다.
- `docs/en/`과 `docs/ko/` 아래의 모든 유지되는 대응 Markdown 파일이 같은 상대
  구조의 항목으로 색인되어 있습니다.
- 정확한 루트 쌍 `README.md`와 `README.ko.md`만 유지되는 루트 수준 의미 일치
  대응 쌍으로 허용됩니다.
- `README.ko.md`가 있으면 루트 README 쌍으로 `README.md`와 함께 색인되어야
  하며, 색인된 루트 README 경로가 없을 때는 일반 경로 존재 규칙으로 보고됩니다.
- 루트 README 쌍에도 다른 색인 경로와 같은 기존 파일 규칙과 중복 경로 규칙이
  적용됩니다.
- 상대 링크가 존재하는 파일로 해석됩니다.
- 조각 링크와 숨김 앵커가 사용되는 곳에서 해석됩니다.
- 유지되는 영어/한국어 대응 쌍의 로컬 Markdown 독자 경로 링크가, 색인된 대상은
  `doc_id`로 정규화하고 유효하지만 색인되지 않은 저장소 대상은 저장소 상대
  경로로 정규화하며 조각을 보존한 뒤 동등한지 확인됩니다. 정확한 루트 README
  쌍도 같은 로컬 의미 링크와 조각 일치 메커니즘을 사용합니다. 이 일치 점검에서는
  외부 링크, 이미지, 코드 펜스 안의 텍스트를 무시합니다.
- 셸 펜스 안의 실행 가능한 `volicord` 명령 예시는 지원되는 공개 CLI 명령 형태와
  옵션을 사용합니다.
- 신원에 민감한 용어의 역할 메타데이터는 허용된 역할 집합을 사용하고, 공개
  선택자, 저장소 내부, MCP 프로세스 바인딩, 진단에 필요한 역할을 포함합니다.
- `docs/terminology-map.yaml`의 `primary_owner`와 `related_references` 경로가
  존재하고 `doc-index.yaml`에 표현되어 있습니다.
- 문서 정책이 표면 라벨을 요구하는 집중 참조 담당 문서는 표면 안정성 섹션을
  포함하고, 기준 어휘로 연결하며, `stable`, `beta`, `internal`,
  `diagnostic` 라벨만 사용합니다.
- 공개 출력 소스는 Volicord 보장을 과장할 수 있는 넓은 보안 단어를 한정 없이
  사용하지 않습니다. 정확한 보안 보장 의미는 보안 담당 문서와 브랜드 주장
  담당 문서에 남아 있습니다.
- 유지되는 산문과 공개 출력 소스는 호스트 전체, 프로필 전체, Agent Connection에 대한
  모호한 지원 형용사와 서술어를 피합니다. 이 점검은 단어 경계를 사용하므로 에이전트
  호스트, 이름 붙은 관리 호스트, 프로필, 연결 전체에 대한 주장을 다른 단어의 부분 문자열
  때문에 오인하지 않습니다. `unsupported_by_host` 같은 정확한 식별자와 상태 값, 호스트가
  미지원임을 명시하는 문장은 허용합니다.

자동 구조 검증 뒤에는 저장소 위생을 사람이 확인합니다.

- 생성된 기록, 런타임 홈, SQLite 파일, 생성 로그, 보관 사본, 변환 메모, 부수
  메모, 작업용 목록, 작업 로그가 유지 문서에 남아 있지 않습니다.

## 사람이 하는 의미 검토

한영 변경에서는 영어와 한국어를 의미 단위로 비교합니다. 독자 목적, 규범 강도,
담당 경로, 기준 범위와 지원 범위 밖 경계, 사용자 판단 경계, 부정 절, 비주장,
보장 강도, 제목, 표, 목록, 예시, 링크, 정확한 식별자를 보존합니다.

계약과 가까운 편집에서는 정확한 API 동작, 스키마 의미, 오류 의미, 저장 효과,
보안 표현, 접근 경계, 닫기 준비 상태 의미, 값 집합 의미, Core 권한 의미가 집중
참조 담당 문서에 남아 있는지 확인합니다. 담당 문서가 아닌 곳은 요약하고
링크해야 하며 두 번째 계약 본문이 되면 안 됩니다.

용어 변경에서는 정확한 식별자, 선호 표현, 피해야 할 표현, 한국어 혼합어 통제,
담당 경로 무결성을 용어 지도에서 확인합니다.

브랜드 표현이나 넓은 주장 문구를 바꿀 때는 [브랜드 지침](brand-guidelines.md)에서
Volicord 표기, 공식 한영 브랜드 문구, 구성 요소 표현, 테스트 하네스 용어 경계,
시각 원칙, 주장 제한을 확인합니다. 정확한 제품 동작, API 동작, 저장 효과,
스키마, 보안 보장, Core 권한 의미가 계속 참조 담당 문서로 연결되는지 확인합니다.

관리 호스트 주장은 각 문장을 환경 적용 가능성, 설정 또는 구성 상태, 정규 결속,
검증 영수증, 릴리스 증거로 나누어 검토합니다. 각 계층은
[시스템 요구사항](../reference/system-requirements.md),
[Agent Connection](../reference/agent-connection.md#host-verification-receipt),
[호스트 릴리스 증거](../reference/host-release-evidence.md)를 기준으로 확인합니다. 설정,
구성, 구현, 픽스처, 테스트에 관한 사실은 엄격한 현재 영수증을 성립시키지 않으며,
영수증은 정확한 최종 아티팩트 릴리스 증거를 대신하지 않습니다.

API와 참조 예시는 필요할 때 메서드 안의 정합성, 요청과 응답 형태, 필드 이름,
필수 필드, `null` 허용 여부, enum 형태 값, `state_version`, 참조, 아티팩트
참조, 실행 참조, 판단 참조, 닫기 차단 사유, 응답 분기, 적용되는 담당 문서
링크를 확인합니다.

코드 이동 때문에 아키텍처 가이드 문서가 바뀌었다면 관련 문서가 오래 유지될
크레이트, 모듈, 진입점, 실행 단계, 책임 경계를 설명하는지 확인합니다. 구현
세부사항을 제품 계약 문구로 바꾸지 않습니다.

자동 `docs-check` 명령에는 유지되는 영어/한국어 대응 쌍의 로컬 문서 링크 일치
점검이 포함되지만, 한영 의미 검토, 계약 담당 문서 검토, 기술 정확성 검토, 번역
판단, API 예시 정합성 검토, 제품 의미 검토를 수행하지 않습니다. 로컬 링크 일치
점검 통과는 기계로 비교할 수 있는 로컬 독자 경로만 확인합니다. 나머지 점검은
계속 사람이 하고 담당 문서로 경로를 잡습니다.

## 오래 유지될 테스트와 일회성 감사

문서나 구현 변경 때문에 새 자동 점검을 고려할 때는 그것이 오래 유지될 계약
테스트인지 일회성 감사인지 먼저 판단합니다. 오래 유지될 테스트는 현재 허용되는
오래 유지될 동작, 계약, 상태 전이, 사용자 가치, 안정적인 추상화 경계, 유지되는
검증 규칙을 검증할 때 저장소에 둡니다. 일회성 감사는 정리 작업에만 관련된
텍스트, 플래그, 필드, 예시가 제거되었는지 확인할 뿐일 때 변경 절차 안에서
수행합니다.

파일 길이, 문서 길이, LOC 수는 오래 유지될 품질 점검이 아닙니다. 품질 기준
경계는 [제품 및 유지보수 헌장](product-maintenance-charter.md)을 사용하고, 담당
경로, 계약, 링크, 예시, 상태 전이, 독자 사용성을 검증하는 점검을 선호합니다.

구현 계층 배치와 테스트 작성 예시는 [테스트 전략](../architecture-guide/testing-strategy.md)을
사용합니다. 이 검증 정책은 그런 점검의 유지보수 점검, 검토, 보고 경계를
담당합니다.

"예전 옵션 이름이 더 이상 나타나지 않는다" 같은 정리 전용 문자열 검색만 검증하는
영구 테스트를 추가하지 않습니다. 그런 검색은 필요할 때 감사로 실행하고, 결과는
저장소 파일 밖에서 보고합니다. 어떤 부재가 오래 유지될 계약으로 중요하다면 대신
현재 형태를 긍정적으로 테스트합니다.

- CLI 도움말은 해당 명령의 현재 공개 옵션 허용 목록만 노출합니다.
- 유지되는 셸 예시는 지원되는 `volicord` 명령과 옵션을 사용합니다.
- 저장소 스키마 점검은 현재 기준 SQL, 테이블, 컬럼, 인덱스, 제약, 초기화,
  검증 동작을 확인합니다.
- MCP 사전 점검과 전송/스키마 점검은 현재 시작 동작, 공개 도구 노출, 공개 스키마
  형태를 검증합니다. 공개 MCP 스키마는 내부 envelope와 호출 필드를 숨기는
  안정적인 추상화 계약을 계속 지켜야 합니다.
- 용어 검증은 `connection_id`나 `project_id` 같은 식별자에 대해 넓은 산문 금지어
  검색을 추가하지 않고, 신원에 민감한 역할 메타데이터를 확인합니다.

오래 유지될 테스트 이름은 현재 계약을 기준으로 짓습니다. 예시는
`connect_help_exposes_only_public_connect_options`,
`documented_volicord_commands_match_public_cli_contract`,
`export_help_lists_authority_bundle`,
`mcp_public_schema_hides_internal_envelope_fields`,
`terminology_map_defines_identity_sensitive_roles`,
`storage_registry_contains_current_contract_columns`입니다. `removed_options_are_gone`,
`legacy_flags_are_removed`, `old_strings_do_not_remain`,
`cleanup_removed_project_id` 같은 이름과 구조는 피합니다.

## 유지보수성 보고서

검토자가 크거나 복잡한 저장소 표면을 빠르게 파악해야 할 때 유지보수성 보고서를
사용합니다.

```sh
cargo run -p xtask -- maintainability-report
```

이 보고서는 검토자 안내입니다. 가장 큰 Rust 파일, 테스트 파일, Markdown 파일,
명령 파싱/실행/렌더링 신호가 함께 보이는 휴리스틱 결과, 쉽게 추론할 수 있는
테스트 범위 힌트 같은 신호를 나열합니다. LOC 제한, LOC 예외 허용 목록, 긴 파일이
무효라는 상태, 또는 응집된 파일을 줄 수만으로 나누라는 요구를 정의하지 않습니다.
보고된 크기와 신호는 담당 범위, 가독성, 테스트 범위, 소스 구조에 관해 검토자가
질문할 계기로 다룹니다.

CI에서 이 보고서를 실행할 때 실패 종료 상태는 명령이 저장소를 검사하지 못했다는
뜻입니다. 큰 파일, 긴 문서, 혼합 신호, 테스트 범위 힌트 자체는 CI 실패가 아닙니다.

## 온보딩 사용성 검증

온보딩, 설치, Codex 설정, 문제 해결, 담당 경로가 실질적으로 바뀌면 대표 사용자를
대상으로 사용성 검증을 수행합니다. 이는 사람이 참여하는 검증이며 자동 검사나 에이전트의
문서 검토가 실제 참여자를 대신하지 않습니다.

처음 사용하는 운영자가 다음 작업을 완료할 수 있는지 확인합니다.

1. 문서에 나온 플랫폼과 저장소 토폴로지가 지원되는지 판단합니다.
2. 정확한 `volicord`와 Codex 실행 파일을 설치하거나 선택합니다.
3. `personal` 또는 `shared` 범위에서 `record` 프로필의 `codex` Agent Connection을
   프로비저닝합니다.
4. 연결을 검증하고 성공이 아닌 각 결과를 해석합니다.
5. `volicord inbox`로 대기 중인 `UserActionRequest`를 찾고
   `volicord inbox resolve`로 해결한 뒤 에이전트 작업을 재개합니다.
6. Volicord가 관리하는 구성만 복구하거나 제거합니다.
7. 정확한 계약의 집중 스키마 또는 저장소 담당 문서를 찾습니다.

결과는 유지 문서 밖에 기록합니다. 참여자 메모, 스크린샷, 녹화, 자격 증명, 대화
기록, Runtime Home, 꾸며 낸 완료 주장을 커밋하지 않습니다.

## Codex 릴리스 검증 검토

릴리스 검증은 [호스트 릴리스 증거](../reference/host-release-evidence.md)가 정의하는
정확한 최종 Codex 및 Volicord 아티팩트와 `linux`, `macos`, `native_windows`,
`wsl2`에 걸친 정확한 target/environment 셀 여섯 개로 제한됩니다. 선택한 셀은 담당 문서의 닫힌 시나리오 목록을 실행하고 정확히
제한된 증거 형태를 게시해야 합니다. 저장소 테스트, 구성 fixture, 다른 플랫폼 결과,
복사한 증거 항목은 셀을 통과시킬 수 없습니다.

실제 검증에는 폐기 가능한 Product Repository, Runtime Home, 외부 결과 위치만
사용합니다. 자격 증명, prompt, 대화 기록, token, 스크린샷, 런타임 데이터를 저장소
밖에 둡니다. runner나 전제 조건이 없으면 `unavailable`, 적격 시도가 없으면
`not_run`으로 보고합니다.

## 정확한 호스트 릴리스 증거 게이트

[`CodexSupportCatalog`](../reference/host-release-evidence.md#codex-support-catalog),
[`CodexReleaseEvidenceManifest`](../reference/host-release-evidence.md#codex-release-evidence-manifest),
정확한 아티팩트 규칙, 독립 target/environment 셀, 필수 시나리오, 셀 상태 의미는 호스트 릴리스
증거가 담당합니다. 유지관리자는 실행 절차에서 이 계약을 다시 정의하거나 CLI 텍스트로
릴리스 주장을 추론하지 않습니다.

테스트 전용 `tests/release-validation` 패키지가 있으면 다음을 실행합니다.

```sh
cargo test --locked -p volicord-release-validation-tests --all-targets --all-features
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --status
```

테스트 명령은 정규 체크인 byte, 내장 카탈로그와 디스크 카탈로그의 일치, 분리된
계약 parsing, 명시적인 테스트 전용 설명자 분리, 부정 사례, 정확한 런타임 지원
조회, 외부 증거 비포함 경계, 릴리스 target 일관성, 증거와 카탈로그의 교차 대조를
검증합니다. 정적 계약이 유효하면 비어 있거나 검토된 entry가 있는 지원 카탈로그와
비어 있거나 일부 또는 전체 셀을 담은 증거 manifest를 허용합니다. 최종 확정
아티팩트를 실행하거나 릴리스 완전성을 판단하지 않으며 어떤 플랫폼 셀도 `passed`로
만들 수 없습니다. 상태 명령은 셀을 실행하지 않고 여섯 셀의 실제 또는 파생 외부
증거 상태를 보고하며, entry가 없는 셀은 각각 `not_run`으로 보고합니다.

기준 텍스트 계약은 저장소 byte에 LF를 사용합니다. 루트 `.gitattributes`는 runner의
전역 `core.autocrlf` 설정과 관계없이 지원 플랫폼에서 동일한 LF byte가 체크아웃되도록
강제합니다. CRLF를 포함한 계약 파일은 비정규 형식이며 exact-byte 계약 게이트가
거부합니다.

실행 명령과 모든 정확한 입력 의미는
[실행 가능한 릴리스 셀 게이트](../reference/host-release-evidence.md#executable-release-cell-gate)가
담당합니다. 아티팩트를 바꾸거나 digest 또는 통과 결과를 손으로 작성하지 말고 다음
릴리스 순서를 사용합니다.

1. 지원하려는 각 환경의 정확한 Codex 실행 파일을 확보합니다.
2. 각 파일에 정확한 target, 환경, `record` profile, 선언 capability를 지정해
   `codex-release-cell-gate --generate-support-entry`를 실행합니다.
3. 결정론적 출력을 검토하고 기준 지원 카탈로그를 commit합니다.
4. 그 카탈로그와 source revision에서 게시할 Volicord target을 각각 한 번 빌드합니다.
5. 정확히 그 Codex 파일과 다운로드한 Volicord binary byte로 모든 필수 릴리스 셀을 실행합니다.
6. 전체 build 및 evidence root를 대상으로 `--verify-publish-evidence`를 실행해 새로운
   외부 `verified-release-index.json`을 만듭니다.
7. 검증한 동일 binary byte를 게시하고 그 검증 index를 함께 첨부합니다.

예를 들어 제안 entry 명령은 다음 형태이며 카탈로그를 편집하지 않고 JSON entry를
출력합니다.

```sh
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --generate-support-entry --codex-path CODEX_PATH --target TARGET --platform PLATFORM --profile record --capabilities managed_stdio_mcp,personal_managed_binding,record_workflow,shared_managed_binding
```

실제 실행 전에 다음 runner 경계를 프로비저닝합니다.

| Target/environment 셀 | 릴리스 runner 전제 조건 |
|---|---|
| `x86_64-unknown-linux-gnu` / `linux` | `self-hosted`, `volicord-release`, `linux`, `native-linux`, `x64` 라벨을 가진 자체 호스팅 x86-64 native Linux runner입니다. WSL이나 container이면 안 됩니다. |
| `aarch64-unknown-linux-gnu` / `linux` | `self-hosted`, `volicord-release`, `linux`, `native-linux`, `arm64` 라벨을 가진 자체 호스팅 AArch64 native Linux runner입니다. WSL이나 container이면 안 됩니다. |
| `aarch64-apple-darwin` / `macos` | `self-hosted`, `volicord-release`, `macos`, `native-macos`, `arm64` 라벨을 가진 자체 호스팅 Apple Silicon native macOS runner입니다. |
| `x86_64-apple-darwin` / `macos` | `self-hosted`, `volicord-release`, `macos`, `native-macos`, `x64` 라벨을 가진 자체 호스팅 Intel x86-64 native macOS runner입니다. |
| `x86_64-pc-windows-msvc` / `native_windows` | `self-hosted`, `volicord-release`, `windows`, `native-windows`, `x64` 라벨을 가진 자체 호스팅 x86-64 native Windows runner입니다. |
| `x86_64-unknown-linux-gnu` / `wsl2` | `self-hosted`, `volicord-release`, `windows`, `wsl2`, `ubuntu-24.04`, `x64` 라벨을 가진 자체 호스팅 x86-64 native Windows supervisor이며 정확한 `Ubuntu-24.04` WSL2 배포판이 이미 설치되어 있어야 합니다. Ubuntu GitHub runner는 이 경계가 아닙니다. |

각 runner service는 담당 문서가 정의한 환경 변수를 통해 정확히 최종 확정된 Codex
경로, 플랫폼 scenario driver, environment-image 좌표를 미리 프로비저닝합니다.
Volicord 릴리스 후보는 runner service가 선택하지 않습니다. `RUNNER_NAME`은 runner
service에서 가져오며 `VOLICORD_CODEX_RELEASE_SOURCE_REVISION`은 정확한 빌드
revision을 지정합니다. Workflow는 target별 변경 불가능한 raw 빌드 아티팩트를
다운로드하고 revision과 digest를 검증한 뒤 그 경로를
`VOLICORD_CODEX_RELEASE_VOLICORD_PATH`로 설정합니다. 또한 새로운 외부 증거 및 work
root와 work root 아래의 존재하지 않는 `VOLICORD_HOME`을 만듭니다. WSL2 supervisor는
다운로드한 Linux x86-64 byte를 `Ubuntu-24.04` 내부의 별도 ext4 경로로 옮기고 WSL2
안에서 digest를 검증합니다. Scenario driver는 `wsl_shutdown_restart` 중에도 살아남아
이를 검증할 수 있는 native Windows coordinator입니다.

첫 자격 시도 또는 재수집에서는 각 runner의 서로 다른 외부 미존재 경로를
`VOLICORD_CODEX_RELEASE_CANDIDATE_CELL_PATH`로 설정한 뒤, 각 독립 환경에서
일치하는 생성 명령 하나를 정확히 실행합니다.

```sh
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --capture-candidate x86_64-unknown-linux-gnu --platform linux
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --capture-candidate aarch64-unknown-linux-gnu --platform linux
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --capture-candidate aarch64-apple-darwin --platform macos
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --capture-candidate x86_64-apple-darwin --platform macos
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --capture-candidate x86_64-pc-windows-msvc --platform native_windows
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --capture-candidate x86_64-unknown-linux-gnu --platform wsl2
```

후보 생성은 정확한 Codex 좌표가 내장 지원 카탈로그에 있기를 요구하지만 기존 통과
증거는 요구하지 않습니다. 전체 플랫폼 카탈로그를 실행하고 정확한 runner 및 두
아티팩트 좌표를 기록하며, create-new 방식의 외부 단일 entry 후보 manifest만
기록합니다. 두 기준 계약을 편집하거나 승격하지 않습니다. `failed` 또는
`unavailable` 후보는 검토할 수 있도록 보존하지만 생성기는 실패로 종료합니다. runner가
없으면 시도하지 않은 상태로 남고, 아티팩트, driver, 환경 좌표, 토폴로지 전제 조건이
없으면 자격을 갖춘 후보 생성을 시작할 수 없습니다. job을 다른 runner로 우회하지 말고
이 결과를 정확히 보고합니다.

체크인 재실행이나 독립 audit을 수행할 때는 후보 manifest 여섯 개를 검토하고 한
번의 릴리스 작업으로
[체크인하는 기준 계약](../reference/host-release-evidence.md#canonical-checked-in-contracts)의
외부 증거 원본을 정규 exact-identity 순서로 교체합니다.
과거 entry를 추가하거나,
셀 사이에 증거를 복사하거나, 결과를 편집해 `passed`로 만들거나, 테스트 전용 설명자를
불러오거나, 검토 전에 후보 출력을 체크인 증거로 취급하면 안 됩니다. 검토한 manifest를
대상으로 계약 테스트를 다시 실행합니다.

그런 다음 같은 각 독립 환경에서 차단 재실행 명령을 한 번씩 실행합니다. 이 수동
재실행 경로는 다운로드한 각 빌드를 한 번만 검증하고 시나리오 카탈로그를 두 번째로
실행하지 않은 채 새 증거를 확인하는 운영 릴리스 workflow와 구분됩니다.

```sh
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --target x86_64-unknown-linux-gnu --platform linux
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --target aarch64-unknown-linux-gnu --platform linux
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --target aarch64-apple-darwin --platform macos
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --target x86_64-apple-darwin --platform macos
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --target x86_64-pc-windows-msvc --platform native_windows
cargo run --locked -p volicord-release-validation-tests --bin codex-release-cell-gate -- --target x86_64-unknown-linux-gnu --platform wsl2
```

재실행 게이트는 전체 플랫폼 카탈로그를 driver에 위임하기 전에 정확한 내장 지원
entry와 기존 체크인 통과 증거를 요구합니다. 증거가 없으면 `not_run`으로 실패합니다.
증거가 통과 상태가 아니거나, runner 좌표, Codex 또는 Volicord 아티팩트, scenario
driver, 시나리오 증거, 토폴로지가 달라도 선택한 job이 실패합니다. 정적 계약의
유효성은 릴리스 성공 판단이 아닙니다. 빈 카탈로그는 런타임 조회, 후보 생성, 운영
게시를 차단하고, 누락되거나 일부만 있거나 실패, 사용 불가, `not_run`인 필수 증거는
적용되는 재실행 또는 운영 완전성 게이트를 차단합니다.

일반 `.github/workflows/ci.yml`은 정적 계약 테스트만 실행합니다.
`.github/workflows/release.yml`의 pull request도 실제 job을 건너뜁니다. 릴리스
workflow는 target 다섯 개를 각각 한 번 빌드하고 raw 실행 파일, target, source
revision, digest metadata를 업로드합니다. Tag push 또는 수동 workflow dispatch는 그
아티팩트를 대상으로 native job 다섯 개와 Windows가 감독하는 독립 WSL2 job을
예약합니다. `publish-release`는 셀 여섯 개와 빌드 matrix 모두에 의존합니다. Raw
binary를 패키징하기 전에 빌드 다섯 개, 통과 manifest 여섯 개, 보존된 시나리오 증거
전체를 엄격히 검증하고 archive에서 추출한 각 실행 파일을 다시 hash합니다. 패키징
직전에 같은 최종 verifier가 외부 verified release index를 기록하며 workflow는 그
index를 릴리스 asset으로 준비하고 Volicord binary에 내장하지 않습니다. 대기,
건너뜀, 사용 불가, 실패, `not_run` 셀, 빠진 증거, digest 불일치는 게시를 막습니다.

각 target에서 게시자가 제어하는 모든 byte 변경을 끝낸 Codex 실행 파일과 하나의 raw
Volicord 빌드 아티팩트를 최종 확정하고 두 SHA-256 digest를 계산한 뒤, 다운로드한
Volicord byte를 일치하는 모든
[독립 플랫폼 셀](../reference/host-release-evidence.md#independent-platform-cells)에서
실행합니다. [필수 시나리오 집합](../reference/host-release-evidence.md#required-release-validation-scenarios)을
모두 실행하고 두 실행 파일을 다시 열어 hash를 재확인합니다. Digest를 검증한 WSL2
ext4 사본만 후보 위치를 바꿀 수 있습니다. 다시 빌드하거나 검증하지 않은 사본,
다르게 처리한 실행 파일, 다른 플랫폼용 실행 파일로 해당 byte를 대신할 수 없습니다.
패키징은 검증한 binary를 감쌀 수 있지만 archive 구성원의 digest는 그대로여야 합니다.

계약의 셀 여섯 개를 각각 담당 환경에서 독립적으로 실행합니다.
실제 [셀 실행 상태](../reference/host-release-evidence.md#cell-execution-status)를 기록합니다.
Runner나 다른 전제 조건을 사용할 수 없으면 `unavailable`, 자격을 갖춘 시도를 하지 않았으면
`not_run`으로 보고합니다. 어느 값도 통과가 아닙니다. Linux 결과를 WSL2에, native Windows
결과를 WSL2에, 어떤 아티팩트나 capability 결과를 다른 셀에 복사하면 안 됩니다.
Linux x86-64와 Linux AArch64, Intel macOS와 Apple Silicon도 어느 방향으로든 서로
대신할 수 없습니다. 셀이 누락되거나 통과하지 못하면 전체 릴리스 주장을 막습니다.

플랫폼별 정확한 명령과 runner 좌표, 담당 문서가 정의한 셀 좌표와 증거 digest, 모든 필수
시나리오 결과, 여섯 셀의 실제 상태를 보고합니다. 건너뛰었거나 사용할 수 없는 실행은 이유와
함께 보고합니다. 셀을 생략하거나 `not_run`을 `passed`로 요약하면 안 됩니다. 저장소 테스트와
릴리스 작업 출력은 운영 런타임 신뢰 입력이 아닙니다.

## Rust 구현 검증

Rust 소스, Cargo 매니페스트, 테스트, 픽스처, 빌드 설정을 바꾸지 않았다면 Rust
검증은 필요하지 않습니다.

Rust 구현을 편집한 뒤에는 워크스페이스나 변경된 크레이트에서 적용되는 Rust
검증을 실행합니다.

- `cargo fmt`
- `cargo clippy --all-targets --all-features`
- `cargo test --all-targets --all-features`

더 좁은 Cargo 명령은 저장소 구조나 작업 범위가 분명히 요구할 때만 사용하고 그
이유를 보고합니다.

## 생성 참조와 계약 드리프트 점검

생성되었거나 원본에서 파생되는 참조 표면은 안정적인 점검 명령을 사용합니다.

- `cargo run -p xtask -- docs-check`는 유지 문서 구조, 생성 또는 원본 파생 문서
  표면, 실행 가능한 `volicord` 명령 예시, 용어 메타데이터 담당 경로와 역할, 그리고
  `crates/volicord-store/src/schema/registry.sql` 및
  `crates/volicord-store/src/schema/project.sql`에 대한 기준 Storage DDL SQL
  블록을 점검합니다.
- `cargo test -p volicord-integration-tests --test public_contract_snapshots`는 API 요청
  스키마 투영과 MCP `workflow`/`read_only` 도구 투영의 생성 공개 계약 스냅샷이
  Rust 원본과 일치하는지 점검합니다.
- 의도적인 원본 변경 뒤 공개 계약 스냅샷을 다시 생성하려면
  `VOLICORD_UPDATE_CONTRACT_SNAPSHOTS=1 cargo test -p volicord-integration-tests --test public_contract_snapshots`
  를 실행하고 `tests/integration/snapshots/` 아래의 생성 파일을 검토합니다.

공개 계약 스냅샷 파일은 `_generated`로 표시된 생성 테스트 산출물입니다. 손으로
편집하지 말고 먼저 스키마 또는 MCP 원본을 바꾼 뒤 다시 생성합니다. CLI 공개 명령
드리프트는 별도의 CLI JSON 스키마를 새로 만들지 않고 실행 가능한 문서 예시와
`volicord-cli`의 `binary_admin`, `mcp_transport` 같은 CLI 도움말/출력 테스트
대상으로 계속 다룹니다.

## 저장소 DDL 계약 점검

저장소 DDL, `volicord-store` 기준 SQL, 스키마 검증 코드를 편집했다면 담당
문서와 구현 사이의 정합성을 확인하는 집중 점검을 실행합니다.

```sh
cargo test -p volicord-store --test storage_ddl_contract
```

이 점검은 권위 있는 영어와 한국어 저장소 DDL SQL을 기준 registry/project SQL에서
초기화한 인메모리 SQLite 데이터베이스의 스키마와 비교합니다. Markdown 산문이나 SQL
서식을 비교하지 않고 테이블, 열, 기본값, 제약, 외래 키, 인덱스, 부분 인덱스, 유지되는
트리거 같은 스키마 의미를 확인합니다.

저장소 문서 점검은 영어와 한국어 저장소 DDL의 표시된 기준 SQL 블록이 기준
registry/project SQL 원본 파일과 일치하는지도 확인합니다.

이 점검은 저장소 유지보수와 구현 정합성 점검입니다. 일반 문서 구조 검증, 공개
런타임 적합성, 제품 수락, QA 완료, 닫기 준비 상태, 보안 증명, 잔여 위험 수락과
구분됩니다.

## 보고

검증 결과는 저장소 파일이 아니라 대화에 보고합니다. 변경 파일, 수행한 점검,
결과, 건너뛴 점검과 이유, 남은 문서 위험을 포함합니다.

`PASS`, `WARN`, `FAIL`, `SKIP`은 문서 유지보수 또는 구현 점검 결과로만
사용합니다. 통과한 검증 단계를 Volicord 런타임 적합성, 제품 수락, QA 완료, 닫기
준비 상태, 보안 보장, 잔여 위험 수락으로 설명하지 않습니다.
