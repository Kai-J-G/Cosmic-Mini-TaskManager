# Header
app-title = Task Manager
process-count = { $count } processes
cpu-label = CPU { $percent }%
ram-label = RAM { $used } / { $total } ({ $percent }%)

# Search & Tabs
search-placeholder = Search processes or PID...
tab-all = All
tab-apps = Apps
tab-top-cpu = Top CPU
tab-top-ram = Top RAM
tab-unresponsive = Stopped / Hung

# Alert Banner
alert-unresponsive = { $count } stopped or unresponsive { $count ->
    [one] process
   *[other] processes
} detected!
btn-inspect = Inspect
btn-kill-all = Kill All

# Process Row Actions
btn-stop = Stop
btn-resume = Resume
btn-kill = Kill

# Process States
status-run = Run
status-sleep = Sleep
status-stopped = STOPPED
status-zombie = ZOMBIE
status-hung = HUNG
status-other = Other

# Empty State
no-processes = No matching processes found.

# Settings
settings-title = Settings
settings-theme = Appearance Theme:
theme-system = System
theme-dark = Dark
theme-light = Light
settings-interval = Refresh Interval:
settings-warn-title = Panel Warning Alert
settings-warn-desc = Show warning indicator in panel when processes hang or stop
settings-done = Done

# Status Messages
msg-stopped = Stopped process PID { $pid }
msg-stop-failed = Failed to stop PID { $pid }: { $error }
msg-resumed = Resumed process PID { $pid }
msg-resume-failed = Failed to resume PID { $pid }: { $error }
msg-killed = Killed process PID { $pid }
msg-killed-tree = Killed PID { $pid } and { $count } child { $count ->
    [one] process
   *[other] processes
}
msg-kill-failed = Failed to kill PID { $pid }: { $error }
msg-killed-all = Killed { $count } unresponsive { $count ->
    [one] process
   *[other] processes
}
