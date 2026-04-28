# Wiki Canonicality Policy

この文書は、この repo の current wiki schema を定義する repo-local policy。

- skill には一般原則だけ置く
- current note 名と責務はここに置く
- schema が変わったら、この文書を正本として更新する

## Current Principles

- `/Sources/raw/...` が原資料正本
- `/Wiki/...` は整理済み知識面
- raw transcript を `/Wiki/...` の正本として重複保持しない
- exact evidence と recap を混ぜない
- unresolved state を settled fact として昇格しない
- old value と new value がある場合、current value を `facts.md` で明示する

## Current Note Roles

- `facts.md`
  - 確定した stable fact と stable attribute
  - exact fact、current value、selected option、stable relationship / duration を置く
  - topic-only mention、曖昧情報、未解決事項、future / pending、chronology-only event、recap prose は置かない
- `events.md`
  - 起きた事実の時系列
  - completed event と dated event だけを置く
  - 解釈や要約、future / pending は置かない
- `plans.md`
  - future / pending、予定、意図、次アクション
  - scope 固有の明示指示、一時的な制約、運用方針
- `preferences.md`
  - 嗜好、判断基準、選好
- `open_questions.md`
  - 未解決事項、要確認事項、競合情報
- `summary.md`
  - 人間向け recap
  - exact evidence source にしない
  - stable fact の正本にしない
- `provenance.md`
  - raw source id / path / import metadata / 参照位置

## Current Anti-Rules

- `facts.md` に topic-only line を入れない
- `facts.md` に future / pending、chronology-only event、recap prose を入れない
- `facts.md` に old value だけを置いて current value を欠落させない
- `summary.md` に exact fact、causal claim、resolution claim を入れない
- unresolved contradiction を settled note に入れない
- raw transcript を `/Wiki/...` に正本として複製しない
