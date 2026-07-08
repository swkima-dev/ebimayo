# コードレビュー結果 — feature/refactor-main-loop

対象: 未コミットの作業ツリー変更（`agent.rs` 新規追加、`channel.rs` / `cli.rs` の InboundEvent 化、`main.rs` の縮小）
検証: 8角度のファインダー → 候補ごとの検証パス。CONFIRMED 7件 / PLAUSIBLE 3件。

## 総評

リファクタリングの骨格——`agent_loop` の分離、シグネチャ（`&Agent<M>` /
`&dyn Channel` / `&mut rx`）、`InboundEvent` の導入、`tool_call.id` の
request_id 流用——は設計どおり。問題は承認フローの実装詳細に集中しており、
根本原因は実質2つ（下記 A・B）。

---

## 根本原因A: 「表示系variantでErrを返す」規約

`StatusUpdate::InvalidApproval` / `ApprovalExpected` に対して CliChannel の
`send_status` が常に `Err(ChannelError::InvalidApproval)` を返し、呼び出し側の
`?` がそれをプロセス終了に変換してしまう。指摘 1・5・6 の正体。

**修正方向**: この2 variantを StatusUpdate から削除し、回復判断を
`agent_loop` 側に移す。`send_status` の Err は実際のI/O失敗に限定する。

### 指摘1 [CONFIRMED] 承認待ち中の UserMessage でプロセス全体が終了する — `src/agent.rs:135` ✅

承認待ち中に UserMessage を受けると `send_status(ApprovalExpected)` の Err が
`?` で main まで伝播し exit(1)。会話メモリごと失われる。さらにこの腕は
tool_result を積まず承認の再待機もしないため、仮に Err でなくても次の
completion で tool_use が未解決のまま送信され Anthropic API が 400 を返す。

**修正方向**: 以前合意した「`push_user` して承認待ちを継続する
（rx.recv() をループで回し、ApprovalResponse が来るまで待つ）」に置き換える。
これで tool_result 欠落による API エラーも同時に消える。

### 指摘5 [PLAUSIBLE] 迷子の承認応答1つで run() が終了する — `src/agent.rs:68` ✅

run() トップレベルの ApprovalResponse 腕が `send_status(InvalidApproval)` を
`?` で呼ぶ → CliChannel は常に Err → プロセス終了。回復可能なノイズは
無視/ログで済ませるべき場面。

### 指摘6 [CONFIRMED/設計] StatusUpdate にプロトコル違反レポートが混入 — `src/channels/channel.rs:34`

`InvalidApproval` / `ApprovalExpected` はユーザー向けステータスではなく
agent_loop 内部の違反レポート。しかも「この2 variantでは Err を返す」という
文書化されていない規約をチャンネル実装が握っている。将来の DiscordChannel が
素直に Ok を返すと agent_loop の終了セマンティクスが暗黙に変わり、
CliChannel を真似ると1メッセージでプロセスが死ぬ。

---

## 根本原因B: `wating_approval` ハンドシェイクの競合

reader スレッドと agent_loop が `stdin_locked` / `wating_approval` という
2つの共有状態を通じて暗黙に協調しているが、更新順序が保証されていない。

### 指摘2 [CONFIRMED] 複数ツールコールでデッドロック — `src/agent.rs:128`

1レスポンスに ToolCall A, B が含まれると: A の承認読み取り後に reader が
`stdin_locked=true`（cli.rs:52）→ B の ApprovalNeeded は `wating_approval`
しか触らない → `stdin_locked` を解除するのは respond() だけだが、次の
completion まで呼ばれない → reader は cli.rs:32-34 で永久スピン、agent_loop
は rx.recv() で永久待機。旧コードにも存在した問題だが、コードが新ファイルに
移った今が直し時。

**修正方向**: ロック解除の責任を見直す（例: ApprovalNeeded 送信時にも
アンロックする）。

### 指摘3 [PLAUSIBLE] TOCTOU レース + 無条件クリアで承認が回答不能に — `src/channels/cli.rs:88`

respond() のアンロック（cli.rs:99）が `send_status(ApprovalNeeded)` の
`wating_approval` セット（cli.rs:116-117）より先に起きるため、reader が
None を観測して承認入力を UserMessage として分類しうる。さらに送信後の
無条件 `*approval_needed = None`（cli.rs:88-89）が、read_line ブロック中に
並行セットされた request_id を消し、以降すべての入力が UserMessage になって
承認が永久に回答不能になる。

**修正方向**: reader 側を `lock().unwrap().take()` の1回ロックにして、
無条件クリアをやめる（take が読み取りとクリアを原子的に行う）。

---

## 独立した修正

### 指摘4 [CONFIRMED] stdin EOF でプロセスがハングする — `src/agent.rs:19`

run() が元の `tx` を最後まで保持するため、Ctrl-D で reader スレッドが
`tx_cli` を drop しても rx.recv() は None を返さず、while let が永久 pending。

**修正方向**: `channel.start(tx)` に clone せず渡すか、start 後に `drop(tx)`。

### 指摘7 [CONFIRMED] unwrap が到達可能な panic になった — `src/agent.rs:101, 126`

旧 `ChannelError` は空 enum で unwrap が静的に安全だったが、この diff で
`InvalidApproval` が追加され有人型になった。respond()（101行）と
`send_status(ApprovalNeeded)`（126行）の `.unwrap()` は、Err を返しうる
Channel 実装が現れた時点で panic になる。同じ関数内の他の send_status は
`?` を使っており不統一。

**修正方向**: `?` に統一。

### 指摘8 [PLAUSIBLE] request_id 不一致が「ユーザーの拒否」として記録される — `src/agent.rs:141`

`if approved && request_id == tool_call.id` は「拒否」と「id不一致
（別の質問への回答）」を混同している。approved=true でも id が古ければ
虚偽の "Tool use was denied by user" がモデルの文脈に入る。

**修正方向**: id 不一致は回答として扱わず、正しい ApprovalResponse を
待ち続ける（指摘1の recv ループ化と同じ構造で自然に書ける）。

---

## クリーンアップ

### 指摘9 [CONFIRMED] ツールの二重登録 — `src/agent.rs:31`

ToolSet（31-33行）と rig agent builder（42-44行）に同じ3ツールを手書きで
二重登録。ツール追加時に片方を忘れると、builder のみ → `tools.call()` が
ToolNotFoundError で agent_loop が落ちる / ToolSet のみ → モデルから
ツールが見えない。

**修正方向**: rig 0.38.1 の `Agent` は公開フィールド
`tool_server_handle`（`call_tool(name, args)`）を持つので、
`tools.call(...)` を `agent.tool_server_handle.call_tool(...)` に
置き換えれば builder の登録が唯一のリストになり、別建て ToolSet ごと消せる。

### 指摘10 [CONFIRMED] reader スレッドの read_line ブロック重複 — `src/channels/cli.rs:44`

match 両腕に prompt/flush/read_line/EOF・エラー処理/stdin_locked.store が
ほぼ逐語的に重複。既にドリフトしている: 承認側は `println!`（flush が
無意味・`:`  が改行前にぶら下がる）、通常側は `print!`（flush が必須）。
`approval_needed` は読み取り前に確定しているので、プロンプト選択と
InboundEvent 構築だけを分岐に残して共通部を巻き上げられる。

---

## 枠外の細かい点

- `cargo clippy` を一度かける: `wating_approval` のタイポ（フィールドは
wating、ローカルは waiting で同じ状態に2つの名前）、
`let approved: bool; if...else` の数行 → `user_input.trim() == "y"` の
1行に、`approved: approved` の冗長フィールド表記、あたりが機械的に拾える
- cli.rs:80-83 の構造体リテラル → 既存の `IncomingMessage::new` を使う
（将来フィールドが増えたとき構築箇所が1つで済む）
- `main.rs:5` の `eprintln!("{}", e)` は Display のみで anyhow の
"Caused by:" チェーンが落ちる。`{:?}` にすると旧 `main() -> Result` と
同等の出力に戻る
- MAX_ITERATIONS の `1..` → `0..` 修正は確認済み（改善）。ただし上限到達時の
ケア（警告 or エラー）は未実装のまま
