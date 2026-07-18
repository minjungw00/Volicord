# 외부 계약

이 문서는 Git 객체 ID를 받는 Volicord 경계가 공유하는 검증 및 canonicalization
규칙을 담당합니다. 다른 현재 외부 형식은 집중 Reference 담당 문서가 직접
정의합니다. Volicord에는 범용 외부 descriptor registry, decoder probing, 형식 간
호환성 범위가 없습니다.

<a id="surface-stability"></a>
## 표면 안정성

공통 Git 객체 ID 검증 규칙은 stable입니다. 어댑터 소스 배치와 parsing helper는
internal입니다.

<a id="shared-git-object-id-contract"></a>
## 공통 Git 객체 ID 계약

Git 객체 ID를 받는 모든 Volicord 경계는 같은 규칙을 사용합니다.

- 입력은 정확히 40자 또는 정확히 64자의 ASCII hexadecimal입니다.
- 허용하는 byte는 `0-9`, `a-f`, `A-F`뿐입니다.
- 대문자와 소문자 hexadecimal 입력을 모두 허용합니다.
- canonical 표현은 소문자 ASCII hexadecimal입니다.
- 39자, 41자부터 63자, 65자를 포함해 그 밖의 모든 길이는 거절합니다.
- 앞뒤 공백, `0x` 같은 prefix, ASCII가 아닌 숫자, Unicode 유사 문자, 구분자,
  빈 값은 trim하거나 정규화하지 않고 거절합니다.

저장, 비교, digest 입력, 어댑터 출력에는 canonical 소문자 표현을 사용합니다.
허용된 호출자의 대문자 표기는 별도 identity가 아닙니다. 이 계약은 식별자만으로
Git 객체 종류나 저장소를 추론하지 않습니다.

잘못되었거나 알려지지 않은 신뢰할 수 없는 경계 입력은 `Rejected`입니다. 현재
담당 문서가 정의한 형태라고 주장하는 영속 데이터가 이를 위반하면 `Corrupt`입니다.
Volicord는 fallback decoder를 시도하거나 payload 특징으로 현재 경계를 재해석하지
않습니다.

## 이웃 담당 문서

- 제품 전체 실패 범주와 기본값 합치기 금지 규칙: [실패 모델](failure-model.md).
- 저장소별 manifest 및 데이터베이스 열기 처리:
  [저장소 버전 관리](storage-versioning.md).
- 관리 호스트와 연결 의미: [Agent Connection](agent-connection.md).
- 공개 API 오류 분기와 코드: [API 오류](api/errors.md).
