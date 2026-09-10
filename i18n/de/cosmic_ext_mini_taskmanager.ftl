# Kopfbereich
app-title = Taskmanager
process-count = { $count } Prozesse
cpu-label = CPU { $percent }%
ram-label = RAM { $used } / { $total } ({ $percent }%)

# Suche und Filter
search-placeholder = Prozesse oder PID suchen...
tab-all = Alle
tab-apps = Anwendungen
tab-top-cpu = Höchste CPU
tab-top-ram = Höchster RAM
tab-unresponsive = Angehalten / Blockiert

# Warnbanner
alert-unresponsive = { $count } angehaltene{ $count ->
    [one] r oder blockierter Prozess erkannt!
   *[other]  oder blockierte Prozesse erkannt!
}
btn-inspect = Überprüfen
btn-kill-all = Alle beenden

# Aktionen
btn-stop = Anhalten
btn-resume = Fortsetzen
btn-kill = Beenden

# Prozessstatus
status-run = Läuft
status-sleep = Ruht
status-stopped = ANGEHALTEN
status-zombie = ZOMBIE
status-hung = BLOCKIERT
status-other = Andere

# Leere Liste
no-processes = Keine passenden Prozesse gefunden.

# Einstellungen
settings-title = Einstellungen
settings-theme = Erscheinungsbild:
theme-system = System
theme-dark = Dunkel
theme-light = Hell
settings-interval = Aktualisierungsintervall:
settings-warn-title = Leisten-Warnung
settings-warn-desc = Warnanzeige in der Leiste einblenden, wenn Prozesse blockieren oder anhalten
settings-done = Fertig

# Statusmeldungen
msg-stopped = Prozess PID { $pid } angehalten
msg-stop-failed = Fehler beim Anhalten von PID { $pid }: { $error }
msg-resumed = Prozess PID { $pid } fortgesetzt
msg-resume-failed = Fehler beim Fortsetzen von PID { $pid }: { $error }
msg-killed = Prozess PID { $pid } beendet
msg-killed-tree = PID { $pid } und { $count } untergeordnete{ $count ->
    [one] n Prozess
   *[other]  Prozesse
} beendet
msg-kill-failed = Fehler beim Beenden von PID { $pid }: { $error }
msg-killed-all = { $count } nicht reagierende{ $count ->
    [one] r Prozess beendet
   *[other]  Prozesse beendet
}
