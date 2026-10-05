# kiln-rust

Kiln 컴파일러의 Rust 구현. 스펙은 `docs/SPEC.md`.

## 브랜치

- `master`, `develop`: 통합 브랜치. 작업은 `develop`에서 `feature/*`를 따서 진행
- 푸시는 사용자가 직접 한다 (이 환경에는 GitHub SSH 키 없음)

## 커밋 컨벤션

출처: https://velog.io/@shin6403/Git-git-%EC%BB%A4%EB%B0%8B-%EC%BB%A8%EB%B2%A4%EC%85%98-%EC%84%A4%EC%A0%95%ED%95%98%EA%B8%B0
템플릿: `.gitmessage.txt` (`git config commit.template .gitmessage.txt`)

```
type: Subject

body

footer
```

- type (소문자): `feat` `fix` `docs` `style` `refactor` `test` `chore`
- subject: 최대 50자, 마침표/특수기호 금지, 원형 동사(Fix, Add), 첫 글자 대문자, 완전한 문장 아님
- body: 제목과 빈 줄로 분리, 줄당 최대 72자, "무엇을/왜" 중심 (어떻게 X)
- footer(선택): `Resolves: #123`, `Ref: #456`, `Related to: #48`
- 커밋 메시지에 Claude/Co-Authored-By 표기를 넣지 않는다
