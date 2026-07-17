# 상태 결속 Write Ticket 유효성

## 맥락

Write Ticket은 특정한 현재 작업과 project 권한에 대해 준비된 쓰기 하나를 허용합니다.
고정된 lifetime만으로는 관련 workflow, scope, baseline, approval, workspace, path
authority 변경을 감지할 수 없습니다.

## 결정

ticket 발급과 재사용을 owner-defined Task, 현재 Change Unit, 정규화된 scope, baseline,
workspace, approval basis, connection/project context, 정규화된 write-authority
fingerprint에 결속합니다. Core는 ticket lookup 전에 구조적 사전 조건을 검증하고 쓰기
전에 모든 compatibility 좌표를 다시 검증합니다.

관련 없는 전역 변경은 project counter가 바뀌었다는 이유만으로 ticket을 무효화하지
않습니다. 관련 좌표가 하나라도 다르면 ticket을 사용할 수 없습니다. 성공 consumption은
보호된 mutation과 같은 원자적 commit에서 일어납니다.

## 결과

- ticket은 이전 가능한 capability나 사용자 identity가 아닙니다.
- 현재 Change Unit 부재는 ticket 효과 전의 구조적 거부입니다.
- replay는 같은 ticket을 두 번 소비할 수 없습니다.
- path normalization과 fingerprint 구성은 결정적이어야 합니다.
- owner-defined authority 좌표 밖 Guard 관찰은 ticket scope를 묵시적으로 넓히지
  않습니다.

[Prepare Write](../../reference/api/method-prepare-write.md),
[Storage Effects](../../reference/storage-effects.md),
[Security](../../reference/security.md)를 봅니다.
