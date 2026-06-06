# 문서 점검

문서 편집 뒤와 큰 리뷰 인계 전에 이 문서 점검을 사용합니다. 이 점검은 Markdown 문서를 읽는 전용 점검입니다. 런타임 점검이 아닙니다.

`PASS`, `WARN`, `FAIL`은 문서 유지보수용 label로만 사용합니다. 리뷰어가 다음에 무엇을 볼지 정하는 데 도움을 줄 뿐, 수락이나 준비 상태를 결정하지 않습니다.

## 1. 무엇을 점검하는가

이 문서는 문서 불일치를 찾습니다.

- 깨진 링크, anchor, README 경로
- 영어/한국어 파일 지도 또는 의미 불일치
- 담당 문서 밖의 엄격한 계약 중복
- active/later 경계 불일치
- 보장 수준을 과장하는 보안 표현
- 서로를 대체하는 사용자 판단 경로
- 맥락을 과하게 싣거나 잘못된 담당 문서로 보내는 에이전트 검색 지침
- 오래된 재작성 이력, 해결된 issue 기록, 오래된 리뷰 문장

## 2. 무엇을 증명하지 않는가

이 문서는 런타임 동작, 런타임 적합성, 구현 준비 상태, 문서 수락, 개발 준비 상태, 최종 수락, 닫기 준비 상태, QA, 증거 충분성, 잔여 위험 수락, 서버 코딩 시작 허가를 증명하지 않습니다.

이 점검을 사용해 런타임 상태, `task_events`, 생성된 상태 보기, 생성된 운영 산출물, 실행 가능한 fixture, 적합성 보고서, QA 기록, 수락 기록, 닫기 기록, 잔여 위험 기록, 제품 쓰기를 만들지 않습니다.

`PASS`는 확인한 문서가 해당 항목에서 내부적으로 일관되어 보인다는 뜻일 뿐입니다. `WARN`은 사람이 애매한 문구를 검토해야 한다는 뜻입니다. `FAIL`은 문서 유지보수 불일치가 발견되어 담당 문서로 보내야 한다는 뜻입니다.

## 3. 링크 점검

상대 Markdown 링크, README 경로, 대응 언어 링크, 담당 문서 링크, heading anchor를 봅니다.

Active 링크가 현재 file과 anchor로 해소되면 통과입니다. 삭제된 file, 오래된 heading, inactive migration record, 잘못된 언어의 담당 문서를 가리키면 실패입니다.

## 4. 이중 언어 지도 점검

`docs/en`과 `docs/ko`가 같은 활성 파일 지도, 독자 목적, 섹션 범위, 담당 문서 링크, 정확한 식별자를 유지하는지 봅니다.

의미가 같고 한국어가 자연스러우면 통과입니다. 한국어 file이 영어 문서의 active 의미를 빠뜨리거나, 정확한 식별자를 번역하거나, 담당 문서 경로를 바꾸거나, active 자료를 later로 보내거나, later 자료를 active로 만들면 실패입니다.

## 5. 담당 문서 경계 점검

Schema, DDL, enum 값, 상태 전이, gate 규칙, 알고리즘, fixture 본문 형태, template 본문, storage 규칙, security guarantee, validator ID, 공식 정의를 봅니다.

엄격한 계약 하나가 담당 문서 하나에서만 정의되고, 담당 문서가 아닌 문서는 짧은 local consequence와 링크만 두면 통과입니다. Start, Use, Build, Maintain, README, 또는 담당 문서가 아닌 Reference summary가 두 번째 규범 정의를 만들면 실패입니다.

## 6. active/later 경계 점검

Active schema, API 문서, DDL, Build scope 표현, Later 문서, Roadmap 후보, 예시를 봅니다.

Active block에 active material만 있고 later/profile 후보가 승격 전까지 later/profile 담당 문서에 남으면 통과입니다. Later enum 값, method, table, command, template, assurance behavior, operations behavior, fixture family가 active requirement처럼 보이면 실패입니다.

## 7. 보안 표현 점검

`cooperative`, `detective`, `preventive`, `isolated`, guard, freeze, careful-mode, 샌드박스, 권한, 차단, 변조 방지, 격리 표현을 봅니다.

주장이 문서화된 보장 수준과 맞고 `preventive` 또는 `isolated` 동작에는 담당 문서와 증명 경로가 있으면 통과입니다. `cooperative`나 `detective` 동작을 증명된 담당 문서 경로 없이 OS 권한, 임의 도구 샌드박스, 변조 방지 저장소, 보편적 도구 실행 전 차단, 보안 격리처럼 설명하면 실패입니다.

## 8. 사용자 판단 경계 점검

판단 질문, 예시, 닫기 표현, 승인 표현, 최종 수락 표현, QA 면제 표현, 잔여 위험 표현을 봅니다.

제품 판단, 기술 판단, 범위 판단, 민감 동작 승인, QA 면제 판단, 검증 위험 수락, 최종 수락, 잔여 위험 수락, 취소 판단이 구분되면 통과입니다. 넓은 승인, 민감 동작 승인, 최종 수락, QA 면제 판단, 증거, 검증, 잔여 위험 수락이 다른 경로를 조용히 대신하면 실패입니다.

## 9. 에이전트 검색 점검

에이전트 지침, 맥락 적재 조언, README 경로, Reference 경로, 상시 맥락(always-on context) 예시를 봅니다.

에이전트 대상 문서가 다음 행동에 필요한 담당 섹션만 검색하고 상시 맥락을 짧게 유지하면 통과입니다. 기본으로 넓은 Reference 묶음, 전체 schema, 전체 template, 과거 log, 생성된 artifact, 오래된 migration record를 읽도록 하면 실패입니다.

## 10. 오래된 내용 점검

유지보수 문서와 주변 경로에서 과거 재작성 리뷰, 해결된 issue 기록, 오래된 수락 기록, 오래된 stage label 설명, 오래된 별칭 이력, later-profile 지역화 점검 기록, 과거 번역 문제 기록, 임시 migration plan을 찾습니다.

유지보수 문서가 살아 있는 편집 규칙과 현재 문서 점검만 담고 있으면 통과입니다. 오래된 리뷰 문장이 active guidance로 남거나, 보관 사본이 생기거나, scratch migration file이 편집 뒤에도 남아 있으면 실패입니다.
