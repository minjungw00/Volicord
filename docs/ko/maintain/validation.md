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
  설명을 사용합니다.
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

유지되는 온보딩, 설치, 에이전트 호스트 설정, 문제 해결, 담당 경로 문서를
추가하거나 의미 있게 바꿀 때는 대표 사용자 사용성 검증을 사용합니다. 이는 실제
참가자가 수행하는 사람 대상 사용성 시험입니다. 자동 `docs-check`, Rust 구현
테스트, 적합성 점검, 사람이 하는 의미 검토, 에이전트가 수행한 책상 검토와
구분합니다. 에이전트 책상 검토는 문서 유지보수 차단 사유를 찾을 수 있지만, 첫
사용자인 사람이 흐름을 완료할 수 있다는 증거는 아닙니다.

참가자 집합은 최소한 아래를 포함해야 합니다.

- Volicord 경험이 없는 기술적으로 능숙한 사용자 두 명
- Volicord 경험이 없는 MCP 호스트 운영자 한 명
- API 또는 스키마 참조 문서를 찾아야 하는 구현자 한 명

작업은 참가자가 아래를 할 수 있는지 포함해야 합니다.

1. 자신의 환경이 적합한 것으로 문서화되어 있는지 판단합니다.
2. 실행 파일을 빌드하거나 선택합니다.
3. 실행 파일 준비 상태를 확인합니다.
4. Codex 또는 Claude Code 설정 경로 하나를 선택하고 따릅니다.
5. `action_required`를 해석하고 필요한 다음 행동을 식별합니다.
6. 사용할 수 없거나 잘못 선택한 실행 파일 상태에서 복구합니다.
7. 허용 프로젝트가 없거나 프로젝트 선택이 모호한 상태를 해석합니다.
8. 안전한 제거 뒤 무엇이 남는지 설명합니다.
9. `StateRecordRef` 또는 `EvidenceSummary`의 자세한 스키마 담당 문서를 찾습니다.

유지 문서를 개선하는 데 필요한 관찰을 기록합니다. 여기에는 참가자가 멈춘 위치,
유도 없이 묻는 질문, 잘못된 상태 해석, 안전하지 않은 쓰기 또는 삭제 시도,
성공을 스스로 확인했는지, 복구가 완료되었는지, 문서 전환의 수와 종류, 실패한
검색어가 포함됩니다.

사용성 검증을 통과하려면 첫 사용자가 작성자의 설명 없이 실행 파일 준비와 호스트
경로 하나를 완료하고, 문서화된 성공 상태를 독립적으로 식별하며,
`action_required`를 설명 없는 치명적 실패로 취급하지 않고, 관련 없는 사용자 설정
또는 제품 데이터를 삭제하지 않은 채 복구하며, 작성자의 도움 없이 자세한 스키마
담당 문서를 찾아야 합니다. 중요한 차단 사유는 작업 완료를 막거나, 안전하지 않은
쓰기 또는 삭제 시도를 유발하거나, 성공 상태를 잘못 해석하게 하거나, 담당 경로를
깨뜨리는 문제입니다. 중요한 차단 사유는 적용되는 유지 담당 문서에서 고치고, 대응
문서가 바뀌면 영어와 한국어 의미를 맞게 유지하며, 해당 자동 및 수동 유지보수
점검을 다시 실행하고, 관련 참가자 프로필로 영향을 받은 작업을 다시 시험한 뒤
해결된 것으로 취급합니다.

사용성 검증 결과는 대화나 저장소가 승인한 오래 유지될 연구 위치에 보고하며,
유지 문서 안에 개별 시험 기록으로 저장하지 않습니다. 참가자 메모, 스크린샷, 녹화,
세션 로그, 작업 로그, 조작된 완료율, 조작된 인용문, 비공개 참가자 데이터를 유지
문서에 커밋하지 않습니다. 실제 대표 참가자가 작업을 수행했고 그 참여를 확인할 수
있을 때만 대표 사용자 시험이 있었다고 말합니다. 자동 검증은 자신이 담당하는 기계
확인 가능 속성만 증명하고, Rust 테스트는 구현 점검만 증명하며, 에이전트 책상
검토는 유지보수자가 객관적 차단 사유를 문서에서 검토했다는 점만 증명합니다.

<a id="live-host-connection-readiness-sequence"></a>
## 실제 호스트 연결 준비 순서

아래에서 `Task`에 결속되는 모든 실제 호스트 체크리스트를 실행하기 전에 이 순서를
적용합니다. 먼저 선택한 Product Repository에 활성 `Task`가 없는 상태에서 정확히
준비한 Agent Connection으로 설치된 호스트를 시작하고 읽기 전용
`volicord.status` 호출을 관찰합니다. 그 호스트가 종료된 뒤 같은 연결에 대해 관리
`volicord connection verify ... --json`을 실행하고 담당 문서가 정의한 `complete`
결과를 요구합니다. 그 후에만 워크플로 `Task`를 만들거나 활성화하고 Task 결속 호스트
실행을 시작합니다. 첫 관찰은 관리 검증을 대신하지 않습니다.

연결 검증의 정확한 동작과 상태 의미는
[Agent Connection](../reference/agent-connection.md)과
[관리 CLI](../reference/admin-cli.md#agent-connection-result-states)가 계속 담당합니다.
이 절은 릴리스 검증 순서만 담당합니다.

<a id="live-cell-result-root"></a>
## 실제 셀 result root 준비와 복구

12개 셀 행렬 하나에는 승인된 새 외부 `RESULT_ROOT` 하나를 만들고, 실제 정규 symlink 없는
`RESULT_ROOT/cells`와 `RESULT_ROOT/evidence` 디렉터리를 미리 만듭니다. `CELL_DIR`는
정확히 `RESULT_ROOT/cells`입니다. 각 행렬 `VOLICORD_LIVE_HOST_RESULT_PATH`는 그 디렉터리
바로 아래의 서로 다른 존재하지 않는 경로이고, 유지되는 생산자는 구현 셀에 대응하는
sidecar를 sibling `evidence` 디렉터리 아래에서 도출합니다. 생산자는 한 번에 하나만
실행합니다. 생산자는 result root에 안정된 비공개 조정 항목을 만들거나 다시 열고 그
협력적 lease를 유지합니다. 호스트 시작 전 `active`를 동기화합니다. 최종 셀과 담당
디렉터리를 동기화한 뒤 정확한 `clean` 상태를 쓰며, 그 완전한 레코드가 관찰 가능한 게시
커밋 표식입니다. 이 항목은 릴리스 증거가 아닙니다. 정확한 게시 동작은
[append-only 실제 셀 게시 계약](../reference/host-release-evidence.md#append-only-live-cell-publication)이
담당합니다.

어떤 생산자든 게시 오류를 보고하거나 비정상 종료하거나 비-clean 상태, 비공개 stage, 고아
증거, 완료 확인 없는 설치된 최종 이름을 남기면 행렬을 중단합니다. 쓰기 뒤 동기화 오류가
정확한 `clean`을 남길 수도 있지만, 그 경우에도 유지되는 운영 절차는 보수적으로 root를
포기합니다. 최종 이름을 삭제하거나 root를 정리하거나 그곳에서 셀을 재시도하거나 이전 셀을
복사하거나 게이트를 실행하지 않습니다. 실패한 root를 필요에 따라 보존하고 보고한 뒤 두
하위 디렉터리를 미리 만든 새 result root에서 12개 셀을 모두 다시 실행합니다. 게이트와
audit은 복구를 수행하지 않습니다. 이 규칙은 12개 릴리스 셀에 적용하며 `auxiliary/` 아래의
별도 CLI fallback 결과에는 적용하지 않습니다.

<a id="live-host-final-output-release-validation"></a>
## 실제 호스트 최종 출력 릴리스 검증

Codex 또는 Claude Code의 Record profile이나 Detective profile에서 관리되는 최종 출력
권한 고지를 지원한다고 명시하는 릴리스를 게시하기 전에 이 체크리스트를 사용합니다.
정확한 릴리스 후보에서 인증된 환경과 사람의 참여로 수행하는 검증입니다. 호스트 설정
픽스처, 생성 래퍼 직접 출력, 일반 워크스페이스 테스트, Judgment 왕복은 이 검증을
대신할 수 없습니다. 정확한 제품 동작은 [Agent Connection](../reference/agent-connection.md#managed-final-output-authority-disclosure),
[관리 CLI](../reference/admin-cli.md#managed-final-output-authority-disclosure), 그리고 그 집중
의존 문서가 계속 담당합니다. 이 체크리스트는 릴리스 검증 실행과 증거 분리만 담당합니다.

[위에서 미리 만든 result root](#live-cell-result-root)를 사용하고 릴리스 후보와 설치된
호스트의 식별 정보를 기록합니다. 각 `VOLICORD_LIVE_HOST_RESULT_PATH`의 최종 셀 이름은
존재하지 않아야 합니다. 아래 네 유지 호스트·프로필 셀 생산자를 각각 호출합니다. 구현
셀만 인증된 대화형 호스트 검증으로 진행하고 정적 미지원 셀은
호스트 turn 전에 끝납니다.

| 호스트 | Record profile | Detective profile |
|---|---|---|
| Codex | `codex_record_live_final_output_is_opt_in` | `codex_detective_live_final_output_is_opt_in` |
| Claude Code | `claude_code_record_live_final_output_is_opt_in` | `claude_code_detective_live_final_output_is_opt_in` |

```sh
VOLICORD_RELEASE_CANDIDATE_PATH=/path/to/CANDIDATE.json VOLICORD_RELEASE_REQUEST_VERIFIED=0 VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/cells/codex-record-final-output.json VOLICORD_RUN_CODEX_RECORD_FINAL_OUTPUT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_record_live_final_output_is_opt_in -- --ignored --nocapture
VOLICORD_RELEASE_CANDIDATE_PATH=/path/to/CANDIDATE.json VOLICORD_RELEASE_REQUEST_VERIFIED=0 VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/cells/codex-detective-final-output.json VOLICORD_RUN_CODEX_DETECTIVE_FINAL_OUTPUT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_detective_live_final_output_is_opt_in -- --ignored --nocapture
VOLICORD_RELEASE_CANDIDATE_PATH=/path/to/CANDIDATE.json VOLICORD_RELEASE_REQUEST_VERIFIED=1 VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/cells/claude-record-final-output.json VOLICORD_RUN_CLAUDE_RECORD_FINAL_OUTPUT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_record_live_final_output_is_opt_in -- --ignored --nocapture
VOLICORD_RELEASE_CANDIDATE_PATH=/path/to/CANDIDATE.json VOLICORD_RELEASE_REQUEST_VERIFIED=1 VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/cells/claude-detective-final-output.json VOLICORD_RUN_CLAUDE_DETECTIVE_FINAL_OUTPUT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_detective_live_final_output_is_opt_in -- --ignored --nocapture
```

정규 Codex `host_version=0.144.4`에서 최종 출력 기능은 정적으로
`unsupported_by_host`이므로 해당 셀은
`VOLICORD_RELEASE_REQUEST_VERIFIED=0`이어야 하며 인증된 호스트 turn 없이
`not_applicable`로 끝납니다. 이 변수에 `1`을 지정하면 구조 오류입니다. 구현된 기능에서
`1`은 설치 호스트를 사용할 수 없어도 릴리스 주장을 유지하므로 게이트를 실패시킵니다.
의도적으로 보고할 제외에만 `0`을 사용합니다.

두 정적 Codex 셀은 정규 `implementation_disposition=unsupported_by_host`,
`requested_verified=false`, `run_state=not_applicable`,
`claimed_status=unsupported_by_host`, null 증거 경로와 digest만 검증합니다. 실제 호스트
증거를 꾸며내면 안 되며 아래의 전체 실제 증거 요구 사항은 적용하지 않습니다.

호스트 경로를 실행하는 각 구현 셀에서는 `host`와 `profile`이 일치하는 크기 제한 결과를
검사합니다. 아래 증거 필드는 서로 분리되어야 하며, 어느 필드도 다른 필드에서 추론하거나
다른 필드로 대신할 수 없습니다.

1. `config_fixture`는 해당 프로필에서 점검한 관리 호스트 설정을 식별합니다. 픽스처
   통과는 설치된 호스트가 그 설정을 읽었다는 뜻이 아닙니다.
2. `generated_wrapper_direct_wire.status_fallback`과
   `generated_wrapper_direct_wire.authority_receipt`는 정확한 생성 래퍼를 직접 호출해
   크기가 제한된 두 응답 분기를 분리해서 점검합니다. 둘 다 `verified`여야 하지만 설치된
   호스트가 이벤트를 전달했거나 UI를 표시했다는 뜻은 아닙니다.
3. `actual_host_event.status_fallback_event`와
   `actual_host_event.authority_receipt_event`는 두 분기에서 실제 호스트가 핸들러로
   전달한 사실을 분리해서 기록합니다.
   Record는 의도적으로 지속 Guard 관찰을 만들지 않으므로, 해당 event 항목은 인증된
   호스트 소유 관리 UI 전달을 근거로 명시합니다. 직접 래퍼 전후 개수 검사는 최종 출력
   핸들러 자체가 Guard 이벤트나 Agent Session을 추가하지 않았음을 증명합니다. 실제 관리
   호스트 turn에서는 호스트가 관리 서버를 재시도하거나 재시작할 때 하나 이상의 MCP stdio
   생명주기가 별도로 시작될 수 있습니다. Codex 기술 정보는 시작 출처만 제공하며 정확한
   클라이언트 identity와 엄격한 호출별 메타데이터가 root 세션과 프로세스 로컬 thread
   다이제스트를 결속할 때까지 생명주기는 대기합니다. Record event 항목은 한정된 양수
   AgentSession 집합을 `managed_mcp_observation`으로 보고하고, 새 행 모두를 같은 Agent
   Connection, `guard_mode=record`, 정확한 관리 세션 결속에 묶으며, 그중 하나 이상에서
   결속된 초기화·도구 목록·도구 호출 생명주기와 결속 시점부터의 명시적 부분 감시 범위를
   요구해야 합니다. 이 집합 전체에서 첫 turn은 정확히
   한 번의 `volicord.status` 수신·완료 호출만 포함하고 다른 도구 호출은 없어야 하며, 두
   번째 no-tool turn에는 도구 호출이 없어야 합니다. 바깥 개수와 rowid 창 검사는 Guard
   이벤트 증가가 없고 새 MCP AgentSession 모두가 관리 생명주기에서 비롯됐음을 확인하여
   이를 최종 출력 전달 효과로 잘못 귀속하지 않습니다. 이는 전달 증거이지 지속 최종 출력
   관찰을 꾸며낸 것이 아닙니다.
   `actual_host_fixed_ui.authority_receipt`는 모델 산문과 구분되는 호스트 소유 고정 UI에서
   현재 Task의 receipt 전체를 별도로 기록하고 Project, Task, `state_version`, 최신 Run,
   닫기 상태, 차단 사유 수를 결속합니다. `actual_host_fixed_ui.status_fallback`은 Task가
   없을 때의 고정 UI 확인을 독립적으로 기록합니다. 한 셀이 통과하려면 두 하위 상태가
   모두 `verified`여야 합니다.
4. Record의 `detective_decision`은 결과가 `non_observing`과 `non_gating`도 확인하고 Guard
   이벤트나 결정이 없음을 확인할 때만 `not_applicable`입니다. Detective에서는 `allow`와
   `block`을 모두 다뤄야 하며 `allow` 결과가 `block`을 대신할 수 없습니다.
5. 최상위 `status_fallback`은 Task 없음 UI 확인을 생성된 `volicord status --json`
   명령 및 Task별 명령 부재와 별도로 결속합니다. 생성 래퍼를 직접 호출해 얻은 대체
   안내 응답이 UI 관찰을 증명하지는 않습니다. 운영자는 관리 UI의 Task 없음 문구
   전체를 복사하고, 하네스는 명령만 적은 토큰으로 Task별 변형까지 확인한 것처럼
   처리하지 않도록 전체가 정확히 같은지 검사합니다. 모든 셀은 이 증거와
   `actual_host_fixed_ui` 아래의 두 분기를 모두 검증해야 하며, 어느 것도 다른 증거를
   대신할 수 없습니다.
6. `exact_replay.generated_wrapper_identical_payload`는 생성 래퍼에 같은 payload를 반복
   전달한 결과를 기록하고, `exact_replay.actual_host_replay`는 실제 호스트 진입점을 통한
   재생을 기록합니다. Record는 읽기 전용 표시를 새로 고치는 동안에도 관찰을 기록하거나
   차단하지 않습니다. Detective의 실제 재생은 변경할 수 없는 과거 Guard 이벤트와 결정을
   그대로 두고 별도 UI에서 현재 권한을 다시 읽습니다. 생성 래퍼 검사는 같은 payload를
   두 번 전달하는 사이 Task 권한 상태를 전진시키고, 두 번째 표시에는 더 최신인 receipt가
   나오면서 저장된 과거 이벤트는 byte 단위로 그대로인지 요구합니다.

구현된 실제 셀 내부의 증거 상태는 `verified`, `unavailable`, `not_applicable`,
`failed`입니다. 이는 제품 응답 필드가 아니라 검증 하네스 사실입니다. 적용되는 모든
증거 항목이 `verified`일 때만 구현 셀이 통과합니다. 이 실제 증거 형태 안에서는 Record에만
적용되는 Detective 결정이 예상되는 유일한 중첩 `not_applicable` 사례입니다. 위에서
설명한 전체 정적 미지원 셀의 `run_state=not_applicable`은 별도 결과입니다. 설치된 호스트에 안전한 `block` 진입점, 실제 호스트 재생
진입점, 현재 Task의 receipt가 표시되는 UI, Task가 없을 때의 대체 안내 UI 중 하나라도
없으면 해당 증거를 `unavailable`로 기록하고 전체 `result=incomplete`를 유지합니다. 생성
래퍼에 같은 payload를 반복한 결과는 실제 호스트 재생을 대신하지 못합니다.

실행 파일, 인증 환경, 대화형 TTY, 이벤트 전달 표면, 현재 Task의 receipt가 표시되는 UI,
Task가 없을 때의 대체 안내 UI, 안전한 Detective `block` 진입점, 실제 호스트 재생 진입점을
사용할 수 없는 결과는 통과가 아닙니다. 하네스가 기록할 수 있으면 구조화된
`unavailable` 또는 `incomplete` 결과를 보존한 뒤 릴리스 검증 결과를 `SKIP` 또는
`FAIL`로 보고합니다.
픽스처, 래퍼 직접 응답, 다른 매트릭스 셀을 근거로 결과를 올리면 안 됩니다. 네 셀은 모두
존재해야 하지만 구현 셀만 통과하고 지원 주장의 근거가 될 수 있습니다. 검토된 Codex
`0.144.4`의 두 최종 출력 셀은 모두 정적으로 미지원이므로 이 Codex 버전에서 유지되는 두
호스트와 두 프로필을 모두 포괄하는 릴리스 지원 주장은 불가능합니다.

일반적인 완료나 unwind에서 기록기는 append-only 종단 게시 하나를 시도합니다. 게시가
성공하면 구현 셀은 증거를 먼저 설치하고 크기가 제한된 셀을 마지막에 설치하며, 정적
미지원 셀은 셀만 설치합니다. 게시 I/O 오류나 비정상 종료는 생산자 셀 없이 담당 문서가
허용한 크기 제한 잔여물만 남길 수 있으므로 새 root 복구 절차를 적용합니다. 기존 최종
목적지는 덮어쓰지 않습니다. `result=incomplete`와 그 밖의 통과하지 않은 구현 셀 종단 결과는
불완전한 증거이며 통과로 세지 않습니다. 정규 정적 `not_applicable` 셀은 유효한 매트릭스
입력이지만 통과한 지원 주장은 아닙니다. 크기가 제한된 결과와 릴리스 승인자의
체크리스트는 그 승인된 외부 릴리스 기록 위치에 보존합니다. 결과 파일, Runtime Home, 스크린샷, 대화
기록, 녹화, 자격 증명, 비밀값, 전체 프롬프트, 비공개 운영자 입력을 커밋하지 않습니다.
이 증거는 관찰한 호스트, 프로필, 릴리스 후보, 환경에만 적용됩니다. 이식 가능한 호스트
적합성, 보안 증명, 제품 수락, 닫기 준비 상태, 일반적인 정확성 주장이 아닙니다.

<a id="live-host-judgment-release-validation"></a>
## 실제 호스트 판단 릴리스 검증

유지되는 Codex 또는 Claude Code 판단 경로를 지원한다고 명시하는 릴리스를 게시하기
전에 이 체크리스트를 사용합니다. 정확한 릴리스 후보에서 인증된 환경과 사람의 참여로
수행하는 릴리스 검증입니다. 스키마 점검, 픽스처, 일반 워크스페이스 테스트, 또는
무시된 것으로 보고된 실제 테스트가 이를 대신하지 않습니다.
위의 네 개 셀 최종 출력 체크리스트와도 구분됩니다. 어느 체크리스트의 증거도 다른
체크리스트를 충족할 수 없습니다.
정확한 상태와 영수증 동작은 [상태 메서드](../reference/api/method-status.md)와
[API 상태 스키마](../reference/api/schema-state.md)가 계속 담당합니다. 이 체크리스트는
릴리스 검증 실행과 증거 처리만 담당합니다.

[위에서 미리 만든 result root](#live-cell-result-root)를 사용하고 정확한 릴리스 후보와 두
호스트의 식별 정보를 기록합니다. 아래의 각 최종 셀 경로는 `RESULT_ROOT/cells` 바로 아래에
있어야 하며 테스트 시작 전에는 존재하지 않아야 합니다. 이전 `result=passed`를 이후 실행
결과로 오인하지 않도록 lease 아래의 사전 검사가 기존 경로를 거부합니다.

```sh
/absolute/candidate_path/from/CANDIDATE.json --version
codex --version
claude --version
```

후보 설명자와 다이제스트가 기준입니다. PATH에서 찾은 `volicord`는 `candidate_path`와 다른
revision일 수 있으므로 대신 사용하면 안 됩니다.

두 호스트의 무시된 판단 테스트를 각각 실행합니다. 승인된 외부 위치에서 테스트마다
서로 다른 절대 결과 경로를 지정합니다.

```sh
VOLICORD_RELEASE_CANDIDATE_PATH=/path/to/CANDIDATE.json VOLICORD_RELEASE_REQUEST_VERIFIED=1 VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/cells/codex-user-action.json VOLICORD_RUN_CODEX_USER_ACTION_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_user_action_round_trip_is_opt_in -- --ignored --nocapture
VOLICORD_RELEASE_CANDIDATE_PATH=/path/to/CANDIDATE.json VOLICORD_RELEASE_REQUEST_VERIFIED=1 VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/cells/claude-code-user-action.json VOLICORD_RUN_CLAUDE_USER_ACTION_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_user_action_round_trip_is_opt_in -- --ignored --nocapture
```

각 호스트에서 릴리스 후보를 기준으로 다음 관찰을 모두 확인합니다.

1. 호스트 고유 판단 선택자가 화면에 표시되고, 에이전트가 아니라 사람 운영자가 제공된
   선택지 하나를 고릅니다. 호스트가 종료된 뒤 운영자는 `choice:route_alpha` 또는
   `choice:route_beta`를 입력하고, 하네스는 이 확인값이 저장된
   `selected_option_id`와 같은지 검사합니다.
2. 에이전트가 `advisor` Task를 만들고 `volicord.update_scope`로 현재 Change Unit과
   baseline을 만든 뒤, 기본 간결한 결과의 선택지를 소비해 그 선택지에 매핑된 비쓰기
   `shaping_update` Run을 `null`이 아닌 최소 close assessment와 함께 기록합니다.
3. 새로 조회한 status가 `close_state=ready`, 빈 close blocker, 그리고
   `AuthorityReceipt.latest_run_ref`를 보고합니다. 하네스는 시간이나 ID 정렬로 행을
   고르지 않고 그 ref가 가리키는 정확한 Run을 읽습니다.
4. 일치하는 `user_action_requested`, `user_action_resolved`, `run_recorded` 권한
   이벤트 payload가 요청, 해결, 선택지, Run, kind, 비쓰기 사실을 보존하고, event
   sequence가 선택 해결 기록 뒤에 해당 Run이 기록됐음을 증명합니다.
5. 실행 전 호스트 cursor 뒤에 정확히 하나의 새 Task 결속 Detective Stop 이벤트가
   나타나고, 이유와 close blocker가 없는 `allow`를 기록하며, 새 status와 같은 완전한
   `AuthorityReceipt`를 저장합니다. 운영자는 호스트 소유의 별도 관리 UI에서 완전한
   canonical receipt JSON을 복사하고, 하네스는 `state_version` 하나만 확인하지 않고
   전체가 정확히 같은지 검사합니다.
6. 크기가 제한된 JSON 결과가 고유 `validation_run.run_id`, 시작·기록 시각, 호스트 버전,
   Volicord `build_id`, 정확한 Agent Connection ID, 운영자가 확인한 선택과 저장된 선택,
   권한 이벤트 순서, 소비한 Run, 최종 `result=passed`를 보고하며 대화 기록이나 프롬프트
   본문은 포함하지 않습니다. 외부 셀은 크기가 제한된 종단 파일로만 새로 만들며 기존
   목적지를 교체하지 않습니다.

5번의 Task 결속 Stop 이벤트와 완전한 최신 receipt UI는 이 테스트의 Judgment 완료에
필수인 증거입니다. 그러나 이 증거는 네 셀 최종 출력 매트릭스의 어떤 항목도 채우지
못합니다. 그 매트릭스의 호스트·profile, Task 없음 fallback, Record 동작, block 동작,
재생 검증은 별도로 남습니다. Judgment 실행 중 관찰한 그 밖의 최종 출력은 해당 실행의
진단 자료일 뿐입니다.

고유 elicitation을 사용할 수 없으면 테스트는 대기 항목이 `volicord inbox`에
표시되고 현재 `volicord inbox resolve` 명령 형태를 사용할 수 있는지 확인합니다.
픽스처의 임시 경로나 ID가 없는 크기 제한 명령 템플릿을 내보내고
`result=failed_native_elicitation`을 기록한 뒤 실패합니다. 테스트가 끝나면 폐기 가능한
Runtime Home이 삭제되므로 이 템플릿은 실행 가능한 복구 명령이 아닙니다. 진단을 위해
이 실패 결과를 보존하되 CLI 대체 경로를 성공한 고유 왕복으로 세면 안 됩니다. 이
Judgment inbox 대체 경로는 User Channel 복구 증거이며 최종 출력 `status_fallback`
증거가 아닙니다. 실행 가능한 CLI 복구는 아래의 별도
[실제 호스트 CLI 대체 경로 체크리스트](#live-host-cli-fallback-release-validation)가
담당하며, 그 결과로 이 호스트 고유 셀을 통과시킬 수 없습니다.

실행 파일, 인증 환경, 대화형 TTY, 신뢰·승인 표면, 고유 선택자를 사용할 수 없는 결과는 `PASS`가
아니라 `SKIP` 또는 `FAIL`입니다. 유지되는 두 호스트를 모두 지원한다고 명시하는
릴리스에서는 두 호스트별 검증이 모두 통과해야 합니다.

하네스는 새 외부 결과 경로를 필수로 요구하며 임시 `running` 셀을 만들지 않습니다.
일반적인 완료나 unwind에서 기록기는 append-only 종단 게시 하나를 시도합니다. 게시가
성공하면 구현 셀은 증거를 먼저 설치하고 크기가 제한된 셀을 마지막에 설치합니다. 게시
I/O 오류나 비정상 종료는 생산자 셀 없이 담당 문서가 허용한 크기 제한 잔여물만 남길 수
있으므로 새 root 복구 절차를 적용합니다. 기존 최종 목적지는 덮어쓰지 않습니다.

크기가 제한된 각 JSON 결과와 릴리스 승인자의 체크리스트 기록은 그 승인된 외부 릴리스
기록 위치에 보존합니다. 결과 파일, Runtime Home, 스크린샷, 대화 기록,
녹화, 자격 증명, 비밀값, 전체 프롬프트, 비공개 운영자 입력을 유지 문서나 소스
저장소에 커밋하지 않습니다. 구조화 결과는 관찰한 호스트와 환경에 대한 릴리스 검증
증거일 뿐이며 이식 가능한 호스트 적합성, 보안 증명, 제품 수락, 닫기 준비 상태,
일반적인 정확성 주장이 아닙니다.

<a id="live-host-evidence-observation-release-validation"></a>
## 실제 호스트 증거 관찰 릴리스 검증

유지되는 Codex 또는 Claude Code의 `local_web_consent` 증거 관찰 경로를 지원한다고
명시하는 릴리스를 게시하기 전에 이 체크리스트를 사용합니다. 정확한 릴리스 후보에서
인증된 환경과 사람의 참여로 수행하는 검증입니다. 실제 설치 호스트가 요청을 만들고
재개하며, 정확한 모델 비가시적 capability를 협상하고, 모델 맥락 밖에 host 전용
handoff를 표시해야 합니다. 사람은 로컬 브라우저에서 정규 form을 제출해야 합니다. 무시된 테스트,
픽스처만 수행한 점검, 일반 워크스페이스 테스트, MCP 어댑터 직접 테스트, 호스트 고유
Judgment 결과, CLI 대체 경로 결과, 최종 출력 결과는 이를 대신할 수 없습니다.

정확한 요청과 재개 동작은
[`volicord.request_user_action`](../reference/api/method-request-user-action.md), 해결 권한은
[`volicord.resolve_user_action`](../reference/api/method-resolve-user-action.md), 공통 요청과
해결 필드는 [API 사용자 행동 스키마](../reference/api/schema-user-action.md), Run과 증거
효과는 [`volicord.record_run`](../reference/api/method-record-run.md), 로컬 웹 경로 선택은
[MCP 전송](../reference/mcp-transport.md#local-web-consent-fallback)이 계속 담당합니다.
정확한 상태와 receipt 동작은 [상태 메서드](../reference/api/method-status.md)와
[API 상태 스키마](../reference/api/schema-state.md)가 담당합니다. 이 체크리스트는 릴리스
검증 실행, 증거 분리, 안전한 결과 보존만 담당합니다.

[위에서 미리 만든 result root](#live-cell-result-root)를 사용하고 정확한 릴리스 후보와
설치된 호스트의 식별 정보를 기록합니다. 로컬 브라우저에서 루프백 consent 수신기에 접근할
수 있어야 합니다. 각 최종 셀 경로는 `RESULT_ROOT/cells` 바로 아래에 있어야 하며 시작
전에는 존재하지 않아야 합니다. Lease 아래의 사전 검사가 기존 경로를 거부합니다. 아래
유지 셀 생산자를 각각 호출합니다. 실제
호스트 경로에 도달하는 구현 셀에는 평소 인증된 호스트 환경의 대화형 TTY가 필요하지만,
정적 미지원 셀은 호스트를 시작하기 전에 끝납니다.

```sh
VOLICORD_RELEASE_CANDIDATE_PATH=/path/to/CANDIDATE.json VOLICORD_RELEASE_REQUEST_VERIFIED=0 VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/cells/codex-evidence-observation.json VOLICORD_RUN_CODEX_EVIDENCE_OBSERVATION_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_evidence_observation_round_trip_is_opt_in -- --ignored --nocapture
VOLICORD_RELEASE_CANDIDATE_PATH=/path/to/CANDIDATE.json VOLICORD_RELEASE_REQUEST_VERIFIED=0 VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/cells/claude-code-evidence-observation.json VOLICORD_RUN_CLAUDE_EVIDENCE_OBSERVATION_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_evidence_observation_round_trip_is_opt_in -- --ignored --nocapture
```

운영 관리 호스트에는 local-web capability를 획득할 신뢰 경로가 없습니다. 검토된 Codex
`host_version=0.144.4` 셀은 정적으로 `unsupported_by_host`이고
`requested_verified=false`가 필요하며 인증된 호스트 turn 없이 `not_applicable`로
끝납니다. Claude Code 셀은 호스트 종류의 구현 대체 표를 사용하며 명시적
`requested_verified=false` 제외입니다. 이 제외는 하향 조정으로 남으며 fixture
capability 데이터나 신뢰되지 않은 경로의 브라우저 성공으로 승격할 수 없습니다.

실제 호스트 경로를 실행하는 각 구현 셀에서 릴리스 후보를 기준으로 다음 관찰을 모두
확인합니다. 정적인 `not_applicable` 결과에는 적용하지 않습니다.

1. Store 검사에서 호스트 실행 전에는 `UserActionRequest`가 없고, 실행 뒤에는 실제 설치
   호스트가 준비된 Agent Connection으로 만든 요청 하나만 관찰되어야 합니다. 대상,
   아티팩트 후보, `required_for` 사실을 위에서 연결한 요청 및 스키마 담당 문서와
   대조합니다.
2. 캡처한 초기화 교환에
   `params.capabilities.experimental["io.volicord/user-channel"].model_invisible_user_surface`
   값이 정확한 boolean `true`로 들어 있어야 합니다. create 응답은 닫힌 local-web
   handoff를 `CallToolResult._meta["io.volicord/user-channel"]`에만 담고, 설치된 호스트가
   이 handoff를 모델 맥락 밖의 host 소유 표면에 눈에 보이게 표시해야 합니다. Agent나
   하네스가 아닌 사람 운영자가 그 표면을 사용해 루프백 form을 열고 준비된 대상과
   아티팩트, `supported`, 크기가 제한된 비밀값 없는 요약을 제출해야 합니다. 이 단언은
   정확한 capability 협상, 선택된 경로, 사람 참여만 확인하며 비밀값 탐지나 호스트 고유
   elicitation을 증명하지 않습니다.
3. Store 검사에서 변경할 수 없는 해결 하나를 관찰하고 집중 해결 및 스키마 담당 문서와
   대조합니다. 정확한 영속 필드는 `resolved_by_actor_source=local_user`,
   `channel_kind=local_web_consent`, 그리고 User Channel 어댑터가 제공하는 인식된 로컬 웹
   근거와 같은 `resolved_verification_basis`입니다. 이 체크리스트는 새로운 안정 basis
   값을 정의하지 않습니다. 저장 본문은 준비된 대상과 `ArtifactRef`, `supported`,
   운영자의 크기 제한 요약과 일치해야 합니다. 호스트 종료 뒤 운영자가 같은 요약을 다시
   입력하면 하네스가 저장 값과 정확히 같은지 확인합니다.
4. 같은 연결의 진단과 Store 검사는 정확한 요청을 대상으로 한 재개 하나,
   `agent_workflow_result_replayed=true`, 추가 요청이나 해결 없음, 이후 정확한 해결 ref
   소비를 관찰합니다. 제한된 교환 observer는 실제 create와 resume의 모델 가시
   projection을 검사하며, MCP `content`, `structuredContent`, 호환·진단 text, replay된
   Agent Workflow 본문도 포함합니다. 과거 pending 요청은 정확히
   `{user_action_request_id, status=pending, next_actor=user}`로 표현되고, 전체 요청, 질문,
   option, context, form, capture path, command, raw URL, bearer token, 사용자 note, 증거
   summary는 없어야 합니다.
5. Store 검사는 소비 Run, 증거 관찰 하나, 생산자와 관련성 앵커, 정확한 아티팩트,
   Core가 파생한 관찰 시각, 필수 기준 coverage, 요청-해결-Run 이벤트 순서를 위에서
   연결한 `record_run` 및 상태 스키마 담당 문서와 대조합니다. 이는 담당 문서가 정의한
   사실을 관찰하는 단언이며 이 체크리스트가 동작을 새로 정의하는 것이 아닙니다.
6. 요청이 pending인 동안 셀은 status 결과 하나, 차단된 close 결과 하나, 정확한
   operation-result의 첫 page도 관찰합니다. 모델 가시 pending projection은 같은 정확한
   세 필드 summary이고 4번의 모든 금지 필드를 빼야 합니다. operation-result page는 저장
   응답 전체가 현재 닫힌 형태를 만족할 때만 반환합니다. 해결 뒤 새 상태는
   `AuthorityReceipt.latest_run_ref`를 따라 소비 Run을 가리켜야 하며, 셀은 관찰한 ready
   상태와 빈 차단 사유 집합을 상태 및 스키마 담당 문서와 대조합니다. 또한 호스트
   실행 전 커서 뒤의 새 Task 결속 Detective Stop `allow` 이벤트 하나와, 저장된 Stop receipt,
   새 상태 receipt, 별도의 호스트 소유 관리 UI에서 복사한 완전한 receipt 사이의
   정확한 일치를 요구합니다.
7. 폐쇄형이며 크기가 제한된 외부 JSON에는
   `kind=live_host_evidence_observation_release_validation`, 안전한 검증 좌표, 담당 문서
   대조 결과, 모델 비가시적 capability 및 host 표시 boolean, projection별 안전 형태 boolean과
   digest, 운영자 요약의 정확한 일치 여부와 제한된 문자 수만 기록합니다. consent URL,
   bearer token, raw tool body, 원문 요약, 프롬프트나 대화 기록 내용, 스크린샷이나 녹화, 자격 증명,
   비밀값, 비공개 운영자 입력을 담으면 안 됩니다.

일반적인 완료나 unwind에서 기록기는 append-only 종단 게시 하나를 시도합니다. 게시가
성공하면 구현 셀은 증거를 먼저 설치하고 크기가 제한된 셀을 마지막에 설치하며, 정적
미지원 셀은 셀만 설치합니다. 게시 I/O 오류나 비정상 종료는 생산자 셀 없이 담당 문서가
허용한 크기 제한 잔여물만 남길 수 있으므로 새 root 복구 절차를 적용합니다. 호스트 실행 파일이
없거나 TTY가 대화형이 아니거나, 정확한 capability가 누락되거나 잘못됐거나, host 전용
표시 표면이 없거나, host 전용 `_meta`와 모델 가시 결과 데이터를 구별할 수 없으면
`result=unavailable`을 기록합니다. 픽스처 준비 실패,
선택된 호스트의 비정상 종료, 저장 상태·Stop·receipt·결과 검증기 invariant 실패는 안전한
단계 식별자만 포함한 `result=failed`로 기록합니다. 인증과 브라우저 실패는 호스트 실행
전에 항상 분류할 수 없습니다. 선택된 호스트 실행이 그 이유로 실패하더라도 결과는
`unavailable`이 아니라 `failed`입니다. 예기치 않은 unwind는
`result=failed_before_completion`을 만듭니다. 결과가 `passed`가 아니거나, 테스트가
단지 무시된 것으로 보고됐거나, 선택 변수 없이 실행했으면 통과가 아닙니다.

호스트를 지원한다고 명시하려면 그 호스트의 구현 셀이 통과해야 합니다. 정적인
`unsupported_by_host` 셀은 유효한 매트릭스 입력이지만 그런 지원 주장의 근거가 될 수
없으므로 검토된 Codex `0.144.4` 행은 로컬 웹 증거 관찰 기능 지원을 주장할 수 없습니다.
이 셀은 관찰된 증거 관찰 로컬 웹 경로만 검증합니다. 호스트 고유 Judgment, 실행 가능한
CLI 대체 경로, 호스트 설정, 최종 출력 셀을 충족할 수 없고 그 반대도 마찬가지입니다.
크기가 제한된 결과와 릴리스 승인자의 체크리스트는 그 승인된 외부 릴리스 기록 위치에 보존합니다. 결과나
Runtime Home을 커밋하지 않습니다. 이 증거는 관찰한 호스트, 릴리스 후보, 환경에만
적용됩니다. 이식 가능한 호스트 적합성, 보안 증명, 호스트 고유 elicitation 증거, 제품
수락, 닫기 준비 상태, 일반적인 정확성 주장이 아닙니다.

<a id="live-host-evidence-producer-release-validation"></a>
## 실제 호스트 증거 생산자 릴리스 검증

설치된 각 유지 호스트에서 생산자 기능 두 개를 한 번씩 실행합니다. 각 명령은 셀을 정확한
후보 설명자에 결속하고 12개 셀 디렉터리 안의 서로 다른 새 경로를 사용합니다.

```sh
VOLICORD_RELEASE_CANDIDATE_PATH=/path/to/CANDIDATE.json VOLICORD_RELEASE_REQUEST_VERIFIED=1 VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/cells/codex-verified-tool-producer.json VOLICORD_RUN_CODEX_VERIFIED_TOOL_PRODUCER_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_verified_tool_producer_is_opt_in -- --ignored --nocapture
VOLICORD_RELEASE_CANDIDATE_PATH=/path/to/CANDIDATE.json VOLICORD_RELEASE_REQUEST_VERIFIED=1 VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/cells/claude-code-verified-tool-producer.json VOLICORD_RUN_CLAUDE_VERIFIED_TOOL_PRODUCER_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_verified_tool_producer_is_opt_in -- --ignored --nocapture
VOLICORD_RELEASE_CANDIDATE_PATH=/path/to/CANDIDATE.json VOLICORD_RELEASE_REQUEST_VERIFIED=1 VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/cells/codex-registered-connection-observation.json VOLICORD_RUN_CODEX_REGISTERED_CONNECTION_OBSERVATION_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_registered_connection_observation_is_opt_in -- --ignored --nocapture
VOLICORD_RELEASE_CANDIDATE_PATH=/path/to/CANDIDATE.json VOLICORD_RELEASE_REQUEST_VERIFIED=1 VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/cells/claude-code-registered-connection-observation.json VOLICORD_RUN_CLAUDE_REGISTERED_CONNECTION_OBSERVATION_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_registered_connection_observation_is_opt_in -- --ignored --nocapture
```

선택한 각 셀에서 필수 assertion 계열 일곱 개를 모두 확인합니다. 인증된 실제 호스트는 먼저
정확한 `volicord.prepare_evidence_capture` intent 하나를 커밋한 뒤 같은 불투명 관리
세션에서 정확한 benign Bash pre/post 이벤트 쌍 또는 intent 이후의 실제 Stop 이벤트
하나를 만듭니다. 하네스는 정확한 capture를 받기 전에 의도적으로 불일치시킨 capture를
영속 효과 없이 거부해야 합니다. 같은 등록 연결에서 실행하는 두 번째 실제 호스트 turn은
receipt에 결속된 생산자, 아티팩트, Strong Evidence 관찰, criterion coverage, Run, 현재
status receipt, close 결과를 정확히 하나씩 완결해야 합니다.

하네스는 인증된 셀 호스트 turn 전에 결속된 깨끗하고 폐기 가능한 Runtime Home에서 정확한
관리 기준선 정체성과 메타데이터 digest를 크기가 제한된 형태로 snapshot하고, 셀을 기록하기
전에 같은 snapshot을 다시 만듭니다. 해당 turn의 정확한 불투명 관리 세션 행 가운데 두
snapshot 사이에 새로 생겼거나 메타데이터 digest가 바뀐 행에서만 클라이언트 정체성을
가져올 수 있습니다. 같은 연결의 변경되지 않은 과거 기준선을 미리 넣은 negative case도
검증해야 합니다. 현재 turn에 성공한 관리 initialize가 없으면 그 기준선이 클라이언트
정체성을 제공해서는 안 됩니다. 조건을 만족하는 행이 없으면 null 클라이언트 집합을
기록하므로 구현 셀을 검증됨으로 만들 수 없고, 일부만 있거나 형식이 잘못되었거나 모호하거나
서로 다른 행이 있으면 기록을 중단합니다.

조건을 만족하는 각 키와 정확한 after snapshot 메타데이터 다이제스트는 종단 기록까지
보존합니다. 변경이 없는 반복 turn과 같은 키를 전진시키는 이후 캡처 turn을 모두
검증합니다. 후자는 before snapshot이 직전 예상 다이제스트와 정확히 일치할 때만 예상
다이제스트를 교체할 수 있습니다. 마지막 snapshot 뒤에 조건을 만족한 같은 기준선을
변경하는 negative case도 실행하고, 기록기가 어느 최종 이름도 게시하기 전에 종단 기록이
실패하는지 확인합니다. 삭제, 같은 키의 행 교체, 반복 turn의 before snapshot 불일치도
같은 닫힌 실패입니다. 기록기는 동시에 존재하는 이름을 제거하거나 이 실패를 정직한 null
정체성 하향 조정으로 처리하면 안 됩니다.

Codex `0.144.4` 셀에서 하네스는 정확한 `clientInfo.name=codex-mcp-client`, 정규
클라이언트·호스트 버전 `0.144.4`, 바깥·안쪽 thread 같음, root 세션 매핑 하나,
변경 불가능한 프로세스 로컬 thread 다이제스트 하나도 증명합니다. 세션·thread
메타데이터가 없거나 일치하지 않으면 관리, 진단, 도구 호출, Core, token, local-web 효과가
모두 없어야 합니다. 같은 결속의 이후 유효한 `turn_id`는 허용됩니다. 환경, PID, cwd,
시각, 훅 근접성은 이 assertion을 충족하지 못합니다.

크기가 제한된 증거 sidecar에는 identifier, 개수, digest, 담당 계약 적합성 boolean만
기록합니다. 프롬프트, 대화 기록, 원본 tool 입력·출력, 원본 native session 또는
invocation identifier, URL, token, credential, 인증 cache를 보존하면 안 됩니다. 호스트가
없으면 실제 null 정체성 ignored 셀로 표현합니다. 주장이 필요하면
`VOLICORD_RELEASE_REQUEST_VERIFIED=1`을 유지하여 부재가 게이트를 실패시키게 합니다. 실행
전에 의도적으로 보고할 제외를 결정한 경우에만 `0`을 선택합니다.

<a id="live-host-cli-fallback-release-validation"></a>
## 실제 호스트 CLI 대체 경로 릴리스 검증

유지되는 Codex 또는 Claude Code 호스트 경로에서 실행 가능한 CLI User Channel 복구를
지원한다고 명시하는 릴리스를 게시하기 전에 이 체크리스트를 사용합니다. 정확한 릴리스
후보에서 인증된 환경과 사람의 참여로 수행하는 릴리스 검증입니다. 호스트 고유 Judgment
셀과 네 셀 최종 출력 매트릭스에서 모두 분리됩니다. 명령 템플릿, 일반 CLI 통합 테스트,
호스트 고유 elicitation 결과, 최종 출력 결과는 이 체크리스트를 충족할 수 없습니다.
정확한 CLI와 재개 동작은 [관리 CLI](../reference/admin-cli.md#user-channel-commands),
[Agent Connection](../reference/agent-connection.md),
[`volicord.resolve_user_action`](../reference/api/method-resolve-user-action.md)이 계속
담당합니다. 이 체크리스트는 릴리스 검증 실행과 증거 분리만 담당합니다.

[정규 외부 릴리스 경로 정책](../reference/host-release-evidence.md#external-release-path-policy)을
충족하는 승인된 릴리스 기록 위치를 준비하고 정확한 릴리스 후보와 설치된 호스트의 식별
정보를 기록합니다. 정규 symlink 없는 `RESULT_ROOT` 하나와 그 정확한 `auxiliary/` 하위
디렉터리를 미리 만듭니다. 각 결과 경로는 `RESULT_ROOT/auxiliary` 바로 아래의 서로 다른
존재하지 않는 경로여야 합니다. 생산자는 호스트 시작 전에 협력적 배타 result-root lease를
획득하고 정확한 `clean` 상태를 요구하지만, auxiliary 실행은 행렬 게시 상태를 `active`로
바꾸지 않습니다. 아래 두 무시된 셀을 각각 실행합니다.

```sh
VOLICORD_RELEASE_CANDIDATE_PATH=/path/to/CANDIDATE.json VOLICORD_RELEASE_REQUEST_VERIFIED=0 VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/auxiliary/codex-cli-fallback.json VOLICORD_RUN_CODEX_CLI_FALLBACK_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_cli_fallback_round_trip_is_opt_in -- --ignored --nocapture
VOLICORD_RELEASE_CANDIDATE_PATH=/path/to/CANDIDATE.json VOLICORD_RELEASE_REQUEST_VERIFIED=0 VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/auxiliary/claude-code-cli-fallback.json VOLICORD_RUN_CLAUDE_CLI_FALLBACK_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_cli_fallback_round_trip_is_opt_in -- --ignored --nocapture
```

각 호스트에서 릴리스 후보를 기준으로 다음 관찰을 모두 확인합니다.

1. 하네스가 설치된 호스트에서 사용할 정확한 Detective Agent Connection에 `advisor`
   Task, 현재 Change Unit, baseline, 두 선택지가 있는 현재 대기 상태 product-decision
   요청 하나를 준비합니다. 에이전트가 아니라 사람 운영자가 `route_alpha` 또는
   `route_beta`를 선택합니다.
2. 실제 `volicord inbox --json` 결과가 그 요청을 정확히 한 번 표시합니다. 하네스가
   실제 `volicord inbox resolve ... --choice ... --json` 명령으로 사람의 선택을 제출한
   뒤, 정확히 같은 명령과 인수를 다시 실행합니다. 두 JSON byte가 같고, 첫 해결만 상태를
   한 번 전진시키며, 재시도와 새 status가 커밋된 `state_version`을 그대로 보존해야
   합니다.
3. 저장된 해결은 resolution ID 하나, `resolved_by_actor_source=local_user`,
   `channel_kind=cli`, 해결 담당 문서가 실제 CLI User Channel 경로에 대해 인정하는
   `resolved_verification_basis`를 가지며 선택된 option이 운영자의 선택과 같아야 합니다.
   임시 경로 없는 명령 템플릿이나 `--help` 결과는 이 항목을 충족하지 않습니다.
4. 설치된 호스트가 준비된 Agent Connection으로 시작해 정확한 요청 ID에 대해
   `request.operation=resume`으로 `volicord.request_user_action`을 호출합니다. 같은 연결의
   진단이 최초 결과의 재생을 관찰해야 하며 Task에는 product-decision 요청이 정확히 하나만
   남아야 합니다. 이어서 호스트가 그 Agent Connection을 이름 붙이는
   `created_by_actor_source`로 option에 매핑된 Product Repository 비쓰기
   `shaping_update` Run 하나를 기록합니다.
5. 일치하는 `user_action_requested`, `user_action_resolved`, `run_recorded` 권한 이벤트가
   요청, 해결, CLI 채널, 정확한 Run, 종류, 비쓰기 사실을 요청-해결-Run 순서로 결속합니다.
   새 status는 `AuthorityReceipt.latest_run_ref`가 가리키는 그 Run을 읽고
   `close_state=ready`와 빈 차단 사유를 보고해야 합니다.
6. 호스트 실행 전 cursor 뒤에 새 Task 결속 Detective Stop 이벤트가 정확히 하나 나타나고,
   이유와 차단 사유가 없는 `allow`를 기록하며, 새 status와 같은 완전한 receipt를
   저장합니다. 운영자는 별도의 호스트 소유 관리 UI에서 완전한 canonical receipt를
   복사하고, 하네스는 `state_version` 하나만이 아니라 전체가 정확히 같은지 검사합니다.
7. 크기가 제한된 JSON 결과는 `kind=live_host_cli_fallback_release_validation`,
   `result=passed`, CLI 근거와 정확한 재시도 사실, 같은 연결의 재개 증거, 매핑된 Run과
   이벤트 순서, Stop 좌표, receipt 좌표, 완전한 관리 UI 확인을 담습니다. 증거 범위는 이
   CLI 대체 경로 셀임을 명시하고 호스트 고유 Judgment와 최종 출력 매트릭스 셀을
   제외합니다.

결과 경로는 필수입니다. Append-only 게시가 성공하면 기록기는 비공개 create-new stage와
원자적 no-replace rename을 통해 크기가 제한된 종단 기록 또는
`failed_before_completion` 기록 하나를 설치합니다. 기존 이름을 덮어쓰거나 삭제하지
않습니다. 게시 실패는 크기가 제한된 비공개 stage 또는 명령이 성공하지 않았는데 이미
설치된 최종 이름을 남길 수 있습니다. 같은 목적지를 정리하거나 재시도하지 말고 존재하지
않는 새 auxiliary 경로를 사용합니다. 조정 상태가 정확한 `clean`이 아니면 새 result root를
사용합니다. 이 auxiliary 복구는 12개 셀 행렬 재실행을 요구하지 않습니다. `passed`가 아닌
모든 결과, 사용할 수 없는 실행 파일, 인증 환경,
대화형 TTY, 같은 연결의 재개 경로, Task 결속 Stop, 완전한 receipt UI는 통과가 아니라
`SKIP` 또는 `FAIL`로 처리합니다. 유지되는 두 호스트를 모두 지원한다고 명시하려면 두
호스트별 셀이 모두 통과해야 합니다.

크기가 제한된 결과와 릴리스 승인자의 체크리스트는 그 승인된 외부 릴리스 기록 위치에 보존합니다. 결과
파일, Runtime Home, 스크린샷, 대화 기록, 녹화, 자격 증명, 비밀값, 전체 프롬프트,
비공개 운영자 입력을 커밋하지 않습니다. 이 증거는 관찰한 호스트, 릴리스 후보, 환경에만
적용됩니다. 이식 가능한 호스트 적합성, 보안 증명, 호스트 고유 Judgment elicitation
증거, 최종 출력 매트릭스 증거, 제품 수락, 닫기 준비 상태, 일반적인 정확성 주장이
아닙니다.

## 정확한 호스트 릴리스 증거 게이트

기준 스키마, 행렬, 평가기, 최신성, 판정, audit, 관리 세션 규칙은
[호스트 릴리스 증거](../reference/host-release-evidence.md)가 담당합니다. 유지관리자는
실행 절차에서 이 계약을 다시 정의하거나 CLI 텍스트로 릴리스 주장을 추론하지 않습니다.

테스트 전용 `tests/release-validation` 패키지가 있으면 다음을 실행합니다.

```sh
cargo test -p volicord-release-validation-tests
cargo run --locked -p volicord-release-validation-tests --bin host-release-gate -- --candidate CANDIDATE.json --cell-dir CELL_DIR --manifest-out MANIFEST.json
cargo run --locked -p volicord-release-validation-tests --bin host-release-audit -- --candidate CANDIDATE.json --cell-dir CELL_DIR --manifest MANIFEST.json --audit-out AUDIT.json
```

깨끗한 소스 revision에서 정확한 profile의 후보를 한 번만 빌드하고
`CANDIDATE.json`이 이름 붙인 외부 경로에 둔 다음 12개 셀 모두에 같은 바이너리를
사용합니다. 모든 행렬 명령은 `VOLICORD_RELEASE_CANDIDATE_PATH`로 그 설명자를 지정하고,
`VOLICORD_RELEASE_REQUEST_VERIFIED=0|1`로 주장을 명시적으로 선택하며, 서로 다른 새 셀
경로를 `CELL_DIR=RESULT_ROOT/cells` 아래에 사용해야 합니다. 생산자는 sibling
`RESULT_ROOT/evidence` 디렉터리 아래에서 증거 sidecar를 도출합니다. 셀 디렉터리에는 최종
`.json` 셀 파일 정확히 12개만 두고 다른 항목을 두면 안 됩니다. 게시가 실패하면
[새 root 복구 규칙](#live-cell-result-root)에 따라 그 result root를 포기합니다.

설치된 호스트 버전과 실행 파일 digest는 그 호스트의 여섯 셀에서 같아야 합니다. 최상위와
environment의 클라이언트 이름·버전 필드 네 개는 별도의 모두 문자열 또는 모두 null
집합입니다. null이 아닌 각 쌍은 그 셀의 성공한 관리 MCP initialize에서 얻어 검증한 정확한
`clientInfo.name`과 `clientInfo.version`이어야 하며 한 호스트의 null이 아닌 모든 셀은 쌍
하나를 사용합니다. 정적 미지원 셀은 호스트 좌표가 null이 아니어도 클라이언트 집합을 null로
유지할 수 있습니다. 호스트를 사용할 수 없어도 유지되는 각 셀 생산자를 호출하여 실제 null
호스트·null 클라이언트 셀을 만듭니다. 구현된 셀은 증거를 가진 `ignored`이고 정적 미지원
셀은 null 증거를 가진 `not_applicable`입니다. 주장이 계속 필요하면
`requested_verified=1`을 사용하여 정직한 부재가 실패하게 하고, 의도적으로 보고할 제외에만
`0`을 사용하여 하향 조정되게 합니다. 호스트나 클라이언트 좌표를 꾸며내거나 누락 파일을
합성하면 안 됩니다.

설치된 Codex `0.144.4`에서는 정확한 probe 외피가 `codex-cli 0.144.4`이고 모든 셀이 bare
정규 `host_version=0.144.4`를 저장합니다. Probe 외피 자체는 이 필드에 유효하지 않습니다.
null이 아닌 모든 Codex 호스트 버전은 공유 bare 버전 parser를 통과해야 합니다. v3 평가기는
정확한 버전별 disposition 표를 사용하며 버전이 없거나 검토되지 않았으면 이 표를 물려받을
수 없습니다. 구현된 exact-live 셀에는 `client_version == host_version`이 필요하고 검토한
Codex `0.144.4`에는 추가로 `client_name=codex-mcp-client`와
`client_version=0.144.4`가 필요합니다.

클라이언트 정체성은 성공한 initialize 값을 보존하는 관리 세션 기준선의 최상위
`metadata_json.client_name`과 `metadata_json.client_version`에서만 읽습니다. 호스트 종류,
실행 파일이나 probe 텍스트, 환경, 설정, 프로토콜 버전, 상수, 이후 도구 메타데이터, 다른
셀에서 추론하지 않으며 원본 initialize 또는 프로토콜·세션·thread·turn payload를
보존하지 않습니다. 구현된 셀의 클라이언트 집합이 null이면
`client_identity_missing`, 버전·중복 복사본·호스트 전체 정체성·검토한 Codex 좌표가
불일치하면 `client_identity_mismatch`를 보고합니다. 어느 조건이든
`implemented_unverified`입니다. 정적 미지원 셀은 이 하향 조정 없이 null 클라이언트
정체성을 사용할 수 있습니다.

게이트 출력과 audit 출력은 외부에 새로 만드는 크기 제한 파일입니다. Audit이 원본 셀
파일 12개를 독립적으로 엄격하게 읽고 manifest, 후보, 셀 아티팩트, 불변조건, finding,
exclusion, 상태, 판정을 별도 프로세스에서 다시 열고 계산하도록 게이트 프로세스를 끝낸 뒤
audit을 시작합니다. 셀 또는 증거 파일이 없거나 잘못된 형식이면 하향 조정이 아니라
manifest를 만들지 않는 구조적 명령 실패입니다.

게이트와 audit은 result root의 협력적 공유 lease를 획득하며 게시 잔여물을 정리하거나
채택하지 않습니다. 생산자가 실행 중이거나 조정 상태가 `active`, 빈 상태, 부분 상태,
잘못된 상태이거나 `CELL_DIR`에 비공개 stage가 더 있거나 최종 셀이 누락되었으면 구조적
실패입니다. 참조되지 않은 증거 stage나 고아 최종 증거 파일은
입력 집합에 들어가지 않으며 행렬을 복구할 수 없습니다.

후보·소스·바이너리 좌표, 두 호스트의 가용성 좌표, 존재할 때 호스트별 null이 아닌 단일
클라이언트 정체성, 각 도출 셀 상태, 요청한 검증됨 주장,
하향 조정, 게이트 판정, manifest SHA-256, audit 셀 입력 SHA-256, audit 판정, 모든
finding과 exclusion을 보고합니다. 호스트 버전을 합치거나 ignored, running, 오래됨,
불일치 셀을 생략하지 않습니다. 구조적으로 누락된 셀은 실패한 게이트 호출로 보고합니다.
패키지가 존재하고 모든 필수 주장이 통과하기 전에는 담당 계약을 실행 결과로
간주하지 말고 구현 검증을 사용할 수 없음 또는 실패로 보고합니다. 운영 local-web 획득은
계속 사용할 수 없고 외부 릴리스 아티팩트는 런타임 신뢰 입력이 아니며 CLI fallback은
보조 수단일 뿐입니다.

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
