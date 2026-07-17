# 외부 계약

이 문서는 Volicord 외부 계약 설명자, 경계 어댑터의 정확한 선택, 단일 기준 내부
모델 경계, 호환성 범위, 공통 Git 객체 ID 검증 계약을 담당합니다.

특정 저장소, 호스트, 전송, 릴리스 아티팩트의 payload 스키마는 정의하지 않습니다.
그 형태는 해당 집중 담당 문서에 남습니다. API 응답 envelope이나 저장 효과도
정의하지 않습니다. 여러 표면에 공통으로 적용되는 실패 범주 의미는
[실패 모델](failure-model.md)이 담당합니다.

<a id="surface-stability"></a>
## 표면 안정성

`ExternalContractDescriptor` 형태, 정확한 registry key, 단일 기준 경계 흐름,
`unsupported_external_contract` 사유, 호환성 범위, Git 객체 ID 검증 규칙은
stable 계약입니다. 어댑터 소스 배치, registry 구현, decoder helper type은
internal입니다.

## `ExternalContractDescriptor`

단일 기준 내부 모델의 경계를 통과하는 모든 Volicord 소유 외부 형식은 아래의
정확한 설명자로 식별합니다.

```yaml
ExternalContractDescriptor:
  contract_id: string
  schema_digest: string
  capabilities: string[]
```

필드 의미:

| 필드 | 계약 |
|---|---|
| `contract_id` | 계약의 정확한 의미적 종류입니다. 형식이 무엇을 나타내는지 식별하며 숫자 스키마 revision을 나타내지 않습니다. |
| `schema_digest` | 형식 구조와 canonical encoding의 정확한 digest입니다. 의미적 종류가 같지만 구조가 다른 계약을 구분합니다. |
| `capabilities` | 설명된 형식이 제공하는 전체 capability 집합입니다. 누락된 capability를 추론하거나 기본값으로 채우지 않습니다. |

설명자는 구조적 경계 입력입니다. `contract_id`와 `schema_digest`는 모두 있어야
하고 비어 있지 않아야 하며 정확히 비교해야 합니다. Volicord가 소유한
`contract_id`는 `-v1`, `-v2` 같은 숫자 revision 접미사를 호환성 identity로
사용하면 안 됩니다. 구조 변경은 `schema_digest`로 식별하고 capability 제공
여부는 `capabilities`로 나타냅니다.

특정 외부 형식의 담당 문서는 그 형식의 canonical encoding, digest 구성,
capability 어휘, payload 제한을 정의합니다. 이 문서는 해당 사실을 사용해
Volicord가 경계 어댑터를 고르는 방법을 담당하며 별도의 digest 알고리즘이나
capability 목록을 만들지 않습니다.

## 정확한 어댑터 registry 선택

경계 어댑터 registry key는 아래의 정확한 쌍입니다.

```text
contract_id + schema_digest
```

두 값은 형식 담당 문서가 생성한 그대로 정확히 대조합니다. 설명자 검증 뒤 registry를
선택할 때 어느 값도 trim, 대소문자 접기, 숫자 비교, 부분 일치, 정규화하지
않습니다. Capability 검사는 정확한 registry entry를 선택한 뒤에만 수행하며,
선택한 entry는 수신 경계가 요구하는 모든 capability를 충족해야 합니다.

경계에서 다음 동작을 하면 안 됩니다.

- 숫자 버전을 비교하거나 계약 식별자를 버전 순서로 정렬
- 여러 decoder를 차례로 시도
- 필드 존재 여부, 빈 값, payload 내용으로 형식을 추론
- parsing 실패 뒤 다른 decoder로 재시도
- 누락된 설명자 또는 payload 필드를 기본값으로 채움
- 등록되지 않은 `contract_id` 또는 `schema_digest`를 수용
- Core 또는 Store에 외부 형식별 분기를 둠

형태가 올바른 설명자의 정확한 쌍이 registry에 없거나 등록된 capability가 수신
경계를 충족하지 못하면 machine-readable reason
`unsupported_external_contract`를 가진 `UnsupportedContract`입니다. 형태가
잘못된 설명자는 `Rejected`이며 다른 어댑터를 검색하지 않습니다. 해당 어댑터나
전송 계층이 이 범주와 사유를 자신이 담당하는 응답 형태로 표시합니다.

## 단일 기준 경계 모델

외부 입력은 한 방향으로만 처리합니다.

```text
외부 형식
-> 정확한 설명자 검증과 registry 선택
-> 하나의 엄격한 경계 decoder 또는 adapter
-> 하나의 기준 내부 type
-> Core
-> Store
```

선택한 decoder는 완전한 기준 내부 type을 만들거나 실패해야 합니다. Core와 Store는
기준 내부 type만 받으며 외부 계약 식별자, 스키마 digest, decoder 세대, 외부 필드
배치에 따라 분기하면 안 됩니다.

출력은 반대 방향의 담당 경계를 따르지만 현재의 canonical 외부 계약 하나만
생성합니다. 과거 설명자, 호환성 alias, 일부만 채운 형태, 호출자 payload의
특징으로 선택한 형식을 출력하면 안 됩니다.

## 호환성 범위

Volicord 1.0 전에는 Volicord가 소유한 외부 계약의 현재 설명자만 지원합니다.
Registry에는 해당 계약의 과거 어댑터, placeholder 어댑터, fallback decoder를 두지
않습니다.

Volicord 1.0부터 경계 registry는 정확히 다음 설명자를 지원합니다.

- 현재 공개 릴리스 설명자
- 바로 이전 공개 릴리스 설명자

각 entry는 계속 정확한 `contract_id + schema_digest`로만 선택합니다. 바로 이전
설명자를 지원하더라도 숫자 버전 dispatch, decoder probing, 더 오래된 설명자
지원은 허용되지 않습니다. Core와 Store의 기준 모델은 하나로 유지합니다.

현재 설명자가 아닌 모든 어댑터 entry에는 구체적인 제거 조건과 그 entry를
지원하는 마지막 Volicord 릴리스를 모두 기록하는 지원 종료 metadata가 있어야
합니다. 둘 중 하나의 경계에 도달하면 entry를 암묵적으로 유지하지 않고
제거합니다. 이 정책은 미래 호환성 범위를 정의합니다. 1.0 전에 이전 설명자용
어댑터를 요구하지 않으며 현재 릴리스에 이를 추가하도록 허용하지도 않습니다.

<a id="shared-git-object-id-contract"></a>
## 공통 Git 객체 ID 계약

Git 객체 ID를 받는 모든 Volicord 경계는 같은 검증 및 canonicalization 규칙을
사용합니다.

- 입력은 정확히 40자 또는 정확히 64자의 ASCII hexadecimal입니다.
- 허용하는 byte는 `0-9`, `a-f`, `A-F`뿐입니다.
- 대문자와 소문자 hexadecimal 입력을 모두 허용합니다.
- canonical 표현은 소문자 ASCII hexadecimal입니다.
- 39자, 41자부터 63자, 65자를 포함해 그 밖의 모든 길이는 거절합니다.
- 앞뒤 공백, `0x` 같은 prefix, ASCII가 아닌 숫자, Unicode 유사 문자, 구분자,
  빈 값은 trim하거나 정규화하지 않고 거절합니다.

저장, 비교, digest 입력, receipt binding, 어댑터 출력에는 canonical 소문자 표현을
사용합니다. 허용된 호출자의 대문자 표기는 별도 identity가 아닙니다. 이 계약은
식별자만으로 Git 객체 종류나 저장소를 추론하지 않습니다.

## 이웃 담당 문서

- 제품 전체 실패 범주와 기본값 합치기 금지 규칙: [실패 모델](failure-model.md).
- 저장소별 manifest 및 데이터베이스 열기 호환성:
  [저장소 버전 관리](storage-versioning.md).
- 관리 호스트와 연결 의미: [Agent Connection](agent-connection.md).
- 정확한 릴리스 아티팩트 증거: [호스트 릴리스 증거](host-release-evidence.md).
- 공개 API 오류 분기와 코드: [API 오류](api/errors.md).
