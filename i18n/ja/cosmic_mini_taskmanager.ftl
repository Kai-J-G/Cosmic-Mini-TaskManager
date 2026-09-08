# ヘッダー
app-title = タスクマネージャー
process-count = { $count } 個のプロセス
cpu-label = CPU { $percent }%
ram-label = RAM { $used } / { $total } ({ $percent }%)

# 検索とタブ
search-placeholder = プロセス名またはPIDを検索...
tab-all = すべて
tab-apps = アプリ
tab-top-cpu = CPU上位
tab-top-ram = メモリ上位
tab-unresponsive = 停止 / 応答なし

# アラートバナー
alert-unresponsive = { $count } 個の停止または応答不能なプロセスが検出されました！
btn-inspect = 確認
btn-kill-all = すべて強制終了

# プロセス行アクション
btn-stop = 一時停止
btn-resume = 再開
btn-kill = 強制終了

# プロセス状態
status-run = 実行中
status-sleep = スリープ
status-stopped = 停止
status-zombie = ゾンビ
status-hung = 応答なし
status-other = その他

# リスト空状態
no-processes = 一致するプロセスが見つかりません。

# 設定
settings-title = 設定
settings-theme = 外観テーマ:
theme-system = システム
theme-dark = ダーク
theme-light = ライト
settings-interval = 更新間隔:
settings-warn-title = パネル警告表示
settings-warn-desc = プロセスが停止または応答しなくなった場合にパネルに警告を表示
settings-done = 完了

# 状態メッセージ
msg-stopped = PID { $pid } のプロセスを停止しました
msg-stop-failed = PID { $pid } の停止に失敗しました: { $error }
msg-resumed = PID { $pid } のプロセスを再開しました
msg-resume-failed = PID { $pid } の再開に失敗しました: { $error }
msg-killed = PID { $pid } のプロセスを強制終了しました
msg-kill-failed = PID { $pid } の強制終了に失敗しました: { $error }
msg-killed-all = { $count } 個の応答不能なプロセスを強制終了しました
