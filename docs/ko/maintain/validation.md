# 검증

유지 문서를 편집한 뒤에는 이 정책을 사용합니다. 이 문서는 구조 점검, 사람이
하는 의미 검토, Rust 구현 검증, 결과 보고를 구분합니다.

이 검증은 유지보수 검증입니다. 하네스 런타임 적합성, 제품 수락, QA 완료, 닫기
준비 상태, 보안 증명, 잔여 위험 수락이 아닙니다. 저장소 로컬 자동 문서
검증기는 아래 명령입니다.

```sh
cargo run -p xtask -- docs-check
```

## 구조 점검

문서 메타데이터, 경로, 링크, 용어 경로를 바꿨다면 저장소 루트에서
`cargo run -p xtask -- docs-check`를 실행합니다. 이 명령은 읽기 전용이며
기계로 확인할 수 있는 형태를 검증합니다.

- `docs/doc-index.yaml`이 YAML로 파싱되고 `version: 2`를 갖습니다.
- 모든 공유 항목은 `doc_id`, `path`, `kind`, `summary`, `normative_level`,
  `primary_audience`, `journeys`, `canonical_for`, `depends_on`만 사용합니다.
- 모든 대응 항목은 `doc_id`, `path_en`, `path_ko`, `kind`, `summary`,
  `normative_level`, `translation_policy`, `primary_audience`, `journeys`,
  `canonical_for`, `depends_on`만 사용합니다.
- 공유 항목과 대응 항목에 필요한 필드가 있습니다.
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
- 상대 링크가 존재하는 파일로 해석됩니다.
- 조각 링크와 숨김 앵커가 사용되는 곳에서 해석됩니다.
- `docs/terminology-map.yaml`의 `primary_owner`와 `related_references` 경로가
  존재하고 `doc-index.yaml`에 표현되어 있습니다.
- README, 경로 문서, 참조 문서, 개발 문서, `AGENTS.md`, 용어 링크가 폐기된
  문서 경로를 가리키지 않습니다.

자동 구조 검증 뒤에는 저장소 위생을 사람이 확인합니다.

- 생성된 기록, 런타임 홈, SQLite 파일, 생성 로그, 보관 사본, 변환 메모, 부수
  메모, 임시 목록, 작업 로그가 유지 문서에 남아 있지 않습니다.

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

API와 참조 예시는 필요할 때 메서드 안의 정합성, 요청과 응답 형태, 필드 이름,
필수 필드, `null` 허용 여부, enum 형태 값, `state_version`, 참조, 아티팩트
참조, 실행 참조, 판단 참조, 닫기 차단 사유, 응답 분기, 적용되는 담당 문서
링크를 확인합니다.

코드 이동 때문에 개발자 학습 문서가 바뀌었다면 관련 개발 문서가 오래 유지될
크레이트, 모듈, 진입점, 실행 단계, 책임 경계를 설명하는지 확인합니다. 구현
세부사항을 제품 계약 문구로 바꾸지 않습니다.

자동 `docs-check` 명령은 한영 의미 검토, 계약 담당 문서 검토, 기술 정확성
검토, 번역 판단, API 예시 정합성 검토, 제품 의미 검토를 수행하지 않습니다. 이
점검은 계속 사람이 하고 담당 문서로 경로를 잡습니다.

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

## 보고

검증 결과는 저장소 파일이 아니라 대화에 보고합니다. 변경 파일, 수행한 점검,
결과, 건너뛴 점검과 이유, 남은 문서 위험을 포함합니다.

`PASS`, `WARN`, `FAIL`, `SKIP`은 문서 유지보수 또는 구현 점검 결과로만
사용합니다. 통과한 검증 단계를 하네스 런타임 적합성, 제품 수락, QA 완료, 닫기
준비 상태, 보안 보장, 잔여 위험 수락으로 설명하지 않습니다.
