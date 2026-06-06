# 작성 가이드

하네스 문서를 고치기 전에 이 작성 가이드를 사용합니다. 이 문서는 문서 작업을 위한 살아 있는 편집 규칙입니다. 하네스 서버/런타임 구현, 제품 저장소 쓰기, 생성된 운영 파일, 런타임 상태, 생성된 상태 보기, 증거 기록, QA 기록, 수락 기록, 닫기 기록, 잔여 위험 기록, 실행 가능한 fixture, 적합성 실행기를 승인하지 않습니다.

이 저장소는 현재 문서 전용입니다. 유지보수자 인계 담당 문서가 다르게 말하지 않는 한 문서 검토 상태로 봅니다. 문서는 향후 하네스 서버를 위한 원천 자료입니다. 수락된 구현 준비 상태나 구현된 런타임 동작으로 설명하지 않습니다.

## 1. 편집 규칙

- 이 저장소에서 작업하기 전에 root `AGENTS.md`를 읽습니다.
- 문서 편집 전에는 이 작성 가이드를 읽습니다.
- 이중 언어 편집이나 용어에 영향을 주는 편집이면 [번역 가이드](translation-guide.md)를 읽습니다.
- 영어 문서 의미도 바꾸면 [영어 작성 가이드](../../en/maintain/authoring-guide.md)와 [영어 번역 가이드](../../en/maintain/translation-guide.md)를 함께 봅니다.
- 작업은 문서 전용으로 유지합니다. 런타임 상태, 생성된 상태 보기, 생성된 운영 산출물, 제품 코드, 서버 코드, 실행 가능한 fixture, 적합성 보고서, 임시 하네스 런타임 객체를 만들지 않습니다.
- 작은 작업 묶음을 선호합니다. 변경한 파일과 삭제한 파일을 보고합니다.
- 사용자가 명시적으로 요청하지 않으면 커밋을 만들지 않습니다.

오래된 문구가 현재 제품 명제, 담당 문서 경계, 한국어 품질 규칙, active/later 경계, 정직한 보안 표현과 충돌하면 다시 씁니다. 오래된 섹션 모양이 아니라 오래 남는 원칙을 보존합니다.

## 2. 이중 언어 대응 규칙

영어와 한국어 문서는 대응됩니다. `docs/en`의 의미가 바뀌면 같은 작업 묶음에서 `docs/ko`에 반영합니다. 한국어 편집 중 영어 의미 문제가 드러나면 영어에도 같은 의미를 반영합니다.

대응 문서는 같은 활성 파일 지도, 독자 목적, 의미상 섹션 범위, 담당 문서 링크, active/later 경계, 정확한 식별자를 유지해야 합니다. 한국어 제목과 문단은 자연스럽고 의미가 같다면 영어와 달라도 됩니다.

파일 경로, `doc_id` 값, API 메서드 이름, schema 이름, field 이름, enum 값, error code, table 이름, validator ID, 코드 형태 문자열은 양쪽 언어에서 정확히 보존합니다.

## 3. 계약 하나에는 담당 문서 하나

모든 엄격한 계약에는 담당 문서가 하나만 있습니다. 정확한 field, enum 값, DDL, schema, 알고리즘, 상태 전이, gate 규칙, fixture 본문 형태, template 본문, storage 규칙, security guarantee, error precedence, 공식 정의는 담당 문서에서만 정의합니다.

담당 문서가 아닌 문서는 독자에게 보이는 결과를 말하고 담당 문서로 연결할 수 있습니다. 두 번째 정의를 만들면 안 됩니다.

담당 문서 경계는 아래처럼 봅니다.

| 영역 | 담당 문서 |
|---|---|
| Core 전이, gate, `prepare_write`, Write Authorization, `record_run`, `close_task`, waiver, 대체 불가능한 규칙 | [Core Model 참조](../reference/core-model.md) |
| 공개 API 메서드와 active 요청/응답 형태 | [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), [API Errors](../reference/api/errors.md) |
| later/profile API와 schema 후보 | [Later 후보 색인](../later/index.md#later-schema-candidates) |
| 저장소 배치, DDL, 지속 기록, lock, artifact, migration | [Storage](../reference/storage.md) |
| 상태 보기 규칙, 템플릿 본문, 최신성, 파생 표시 경계 | [Projection과 Template 참조](../reference/projection-and-templates.md) |
| 보안 자산, 신뢰 경계, 보장 수준, 정직한 보안 표현 | [보안 참조](../reference/security.md) |
| 적합성 의미, 향후 fixture 형태, 주장 권한 | [적합성 참조](../reference/conformance.md) |
| 에이전트 connector 동작, capability profile, context 접점, 대체 의미 | [Agent 통합 참조](../reference/agent-integration.md) |
| 런타임 공간 분리와 비격리 주장 | [런타임 경계 참조](../reference/runtime-boundaries.md) |
| 설계 품질 활성화, 발견 사항 심각도, waiver 경계, validator ID | [설계 품질](../reference/design-quality.md) |
| 공식 용어 | [용어집 참조](../reference/glossary.md) |
| 구현 순서와 유지보수자 준비/상태 결정 | [MVP 계획](../build/mvp-plan.md) |

담당 문서 밖에서 계약이 반복되면 먼저 담당 문서를 확인합니다. 필요하면 담당 문서를 고친 뒤, 중복 문구는 짧은 요약과 담당 문서 링크로 바꿉니다.

## 4. active/later 경계

Active 문서는 later/profile, 로드맵, 진단, 운영, 내보내기, 풍부한 template, 향후 적합성 실행기 자료를 활성 요구사항처럼 만들면 안 됩니다.

값, 메서드, table, fixture family, command, template, security guarantee는 담당 문서가 범위, 대체 동작, 증명 기대치와 함께 승격할 때만 active입니다. Reference에 존재한다는 사실만으로 active 전달 범위가 커지지 않습니다.

Later 자료는 `docs/*/later/*`, 명확히 표시된 later/profile 섹션, 또는 승격한 담당 문서에 둡니다. Active schema나 DDL block에 inactive 값이 들어 있으면 주변 문장으로 설명하지 말고 담당 문서 경계를 고칩니다.

## 5. 사용자 판단 경계

하네스는 사용자 소유 판단을 보존합니다. 제품 판단, 중요한 기술 방향, 범위 확장, 민감 동작 승인, QA 면제 판단, 최종 수락, 검증 위험 수락, 잔여 위험 수락, 취소 판단은 서로 다른 경로입니다.

`go ahead`나 `looks good` 같은 넓은 승인을 특정 판단의 대체물로 쓰지 않습니다. 민감 동작 승인은 이름 붙은 민감한 단계만 허용합니다. 제품 동작, 아키텍처, 최종 수락, 잔여 위험을 결정하지 않습니다. 최종 수락은 증거를 만들거나 증거 공백을 지우지 않습니다. 잔여 위험 경로가 따로 묻지 않았다면 잔여 위험을 수락하지 않습니다.

사용자 대상 문서는 사용자가 무엇을 요청할 수 있는지, 에이전트가 무엇을 구체화해야 하는지, 무엇이 막혔는지, 어떤 증거가 있는지, 어떤 판단이 필요한지, 닫기가 무엇을 뜻하는지에서 시작합니다. 내부 라벨은 사용자가 보는 상황이 먼저 분명해진 뒤에만 소개합니다.

## 6. 보안 표현 규칙

보안 표현은 문서화된 보장 수준과 맞아야 합니다.

- Cooperative 표현은 하네스가 행동을 안내하거나 기록할 수 있지만 기술적으로 막지는 못할 때 씁니다.
- Detective 표현은 하네스가 행동 이후 감지하거나 보고할 수 있을 때 씁니다.
- Preventive 표현은 해당 접점이 대상 동작 전에 막을 수 있고 그 동작에 대한 증명 경로가 있을 때만 씁니다.
- Isolated 표현은 문서화된 분리 경계가 있을 때만 씁니다. 그 경계를 이름 붙입니다.

초기 하네스가 OS 권한, 임의 도구 샌드박스, 변조 방지 로컬 파일, 보편적 도구 실행 전 차단, 보안 격리를 제공한다고 암시하지 않습니다. 정확한 메커니즘이 문서화되고 증명된 경우에만 말합니다. Write Authorization은 협력형 하네스 record/check입니다. OS 권한, 샌드박스, 변조 방지 강제, 사전 차단, 격리가 아닙니다.

## 7. 링크 규칙

문서를 이름 바꾸거나, 옮기거나, 나누거나, 합치거나, 삭제하면 양쪽 언어의 링크와 anchor를 같은 작업 묶음에서 고칩니다.

편집 전에는 예전 경로, 예전 제목, 예전 anchor, 예전 제목 문자열, README 경로, 담당 문서 링크, 대응 언어 링크를 검색합니다. 편집 뒤에는 다시 검색합니다. Active 문서는 삭제된 파일, 오래된 anchor, inactive 경로, 이전 안내, 예전 구조를 가리키면 안 됩니다.

2차 요약보다 담당 문서 링크를 선호합니다. 삭제한 maintain page의 archive copy를 만들지 않습니다.

## 8. 오래된 내용 삭제 규칙

유지보수 문서는 앞으로의 편집을 안내해야 합니다. 과거 재작성 리뷰, 해결된 issue 기록, 오래된 수락 note, 오래된 stage label 설명, 오래된 별칭 이력, later-profile 지역화 점검 기록, 과거 번역 문제 기록, 임시 migration plan을 보존하지 않습니다.

오래된 문구를 다룰 때는 아래 durable triage category를 사용합니다.

| 분류 | 사용하는 경우 | 처리 |
|---|---|---|
| `preserve` | 문구가 제품 명제를 지키고, 담당 위치가 맞고, 독자에게 도움이 될 때. | 의미를 유지하고 필요하면 다듬습니다. |
| `shrink` | 방향은 맞지만 너무 길거나, 반복되거나, 내부 용어가 많거나, 문서군에 비해 계약 세부사항이 지나칠 때. | 독자에게 보이는 결과와 담당 문서 링크만 남깁니다. |
| `move` | 내용이 다른 담당 문서나 문서군에 속할 때. | 의미를 그 담당 문서로 옮기거나 링크하고, 예전 copy는 제거합니다. |
| `delete` | 문구가 오래되었거나, 오해를 만들거나, 중복되거나, 과거 기록이거나, 제품 명제, 담당 문서 경계, 한국어 품질 규칙, active/later 경계, 보장 수준과 충돌할 때. | 삭제합니다. 연속성만으로 남기지 않습니다. |
| `decision-needed` | Schema 형태, 상태, API, active/later 경계, security guarantee, fixture 의미, 용어, 구현 준비 상태에 대한 실제 미해결 선택이 드러날 때. | 담당 문서로 판단을 보냅니다. 큰 서버 코딩 전 결정은 흩어진 TODO가 아니라 [MVP 계획](../build/mvp-plan.md)에 둡니다. |

마치기 전에 임시 migration plan과 scratch file을 삭제합니다.

## 9. 사후 편집 체크리스트

- [ ] 편집이 문서 전용으로 남아 있습니다.
- [ ] 영어와 한국어 대응 파일이 같은 의미와 활성 파일 범위를 유지합니다.
- [ ] 한국어 문장이 자연스럽고 줄 단위 번역이 아닙니다.
- [ ] 정확한 식별자, 파일 경로, schema/API 이름, enum 값, error code, table 이름, validator ID가 보존되었습니다.
- [ ] 엄격한 계약은 담당 문서 하나에 남고, 담당 문서 밖 중복은 요약과 링크로 바뀌었습니다.
- [ ] active/later 경계가 흐려지지 않았습니다.
- [ ] 사용자 소유 판단 경로가 서로 섞이지 않습니다.
- [ ] 보안 표현이 문서화된 보장 수준과 맞습니다.
- [ ] 링크, anchor, README 경로, 대응 언어 링크가 해소됩니다.
- [ ] 오래된 재작성 이력, 해결된 issue 기록, 오래된 별칭 이력, 오래된 리뷰 문장을 보관 사본으로 남기지 않고 삭제했습니다.
- [ ] 임시 migration plan이나 scratch file이 남아 있지 않습니다.
- [ ] 관련 [문서 점검](checks.md)을 실행했거나 실행하지 못한 점검을 보고했습니다.
