# Intestazione
app-title = Gestione attività
process-count = { $count } processi
cpu-label = CPU { $percent }%
ram-label = RAM { $used } / { $total } ({ $percent }%)

# Ricerca e schede
search-placeholder = Cerca processi o PID...
tab-all = Tutti
tab-apps = Applicazioni
tab-top-cpu = Più CPU
tab-top-ram = Più RAM
tab-unresponsive = Bloccati / Sospesi

# Avviso
alert-unresponsive = { $count } processo { $count ->
    [one] sospeso o bloccato rilevato!
   *[other] sospesi o bloccati rilevati!
}
btn-inspect = Ispeziona
btn-kill-all = Termina tutti

# Azioni
btn-stop = Ferma
btn-resume = Riprendi
btn-kill = Termina

# Stati
status-run = In esecuzione
status-sleep = In attesa
status-stopped = FERMATO
status-zombie = ZOMBIE
status-hung = BLOCCATO
status-other = Altro

# Nessun processo
no-processes = Nessun processo corrispondente trovato.

# Impostazioni
settings-title = Impostazioni
settings-theme = Tema aspetto:
theme-system = Sistema
theme-dark = Scuro
theme-light = Chiaro
settings-interval = Intervallo di aggiornamento:
settings-warn-title = Avviso nel pannello
settings-warn-desc = Mostra un indicatore di avviso nel pannello quando i processi si bloccano
settings-done = Fatto

# Messaggi di stato
msg-stopped = Processo PID { $pid } fermato
msg-stop-failed = Impossibile fermare il PID { $pid }: { $error }
msg-resumed = Processo PID { $pid } ripreso
msg-resume-failed = Impossibile riprendere il PID { $pid }: { $error }
msg-killed = Processo PID { $pid } terminato
msg-killed-tree = Terminati il PID { $pid } e { $count } { $count ->
    [one] processo figlio
   *[other] processi figli
}
msg-kill-failed = Impossibile terminare il PID { $pid }: { $error }
msg-killed-all = { $count } processi non rispondenti terminati
