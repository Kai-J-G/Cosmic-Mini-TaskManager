# 标题栏
app-title = 任务管理器
process-count = { $count } 个进程
cpu-label = CPU { $percent }%
ram-label = 内存 { $used } / { $total } ({ $percent }%)

# 搜索与标签
search-placeholder = 搜索进程名或 PID...
tab-all = 全部
tab-apps = 应用
tab-top-cpu = 最高 CPU
tab-top-ram = 最高内存
tab-unresponsive = 已暂停 / 无响应

# 警告横幅
alert-unresponsive = 检测到 { $count } 个已停止或无响应的进程！
btn-inspect = 查看
btn-kill-all = 全部结束

# 进程行操作
btn-stop = 暂停
btn-resume = 恢复
btn-kill = 强制结束

# 进程状态
status-run = 运行中
status-sleep = 睡眠
status-stopped = 已停止
status-zombie = 僵尸
status-hung = 无响应
status-other = 其他

# 空状态
no-processes = 未找到匹配的进程。

# 设置
settings-title = 设置
settings-theme = 外观主题：
theme-system = 系统
theme-dark = 暗色
theme-light = 亮色
settings-interval = 刷新间隔：
settings-warn-title = 面板警告指示
settings-warn-desc = 当有进程卡死或暂停时在面板图标上显示警告
settings-done = 完成

# 状态提示
msg-stopped = 已暂停进程 PID { $pid }
msg-stop-failed = 暂停 PID { $pid } 失败: { $error }
msg-resumed = 已恢复进程 PID { $pid }
msg-resume-failed = 恢复 PID { $pid } 失败: { $error }
msg-killed = 已结束进程 PID { $pid }
msg-kill-failed = 强制结束 PID { $pid } 失败: { $error }
msg-killed-all = 已强制结束 { $count } 个无响应进程
