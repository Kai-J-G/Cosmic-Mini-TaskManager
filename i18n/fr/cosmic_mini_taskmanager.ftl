# En-tête
app-title = Gestionnaire des tâches
process-count = { $count } processus
cpu-label = CPU { $percent }%
ram-label = RAM { $used } / { $total } ({ $percent }%)

# Recherche et onglets
search-placeholder = Rechercher des processus ou un PID...
tab-all = Tous
tab-apps = Applications
tab-top-cpu = Top CPU
tab-top-ram = Top RAM
tab-unresponsive = Bloqués / Suspendus

# Bannière d'alerte
alert-unresponsive = { $count } processus arrêté{ $count ->
    [one] {" "}ou bloqué détecté !
   *[other] s ou bloqués détectés !
}
btn-inspect = Examiner
btn-kill-all = Tout tuer

# Actions sur les processus
btn-stop = Arrêter
btn-resume = Reprendre
btn-kill = Tuer

# États des processus
status-run = Actif
status-sleep = En veille
status-stopped = SUSPENDU
status-zombie = ZOMBIE
status-hung = BLOQUÉ
status-other = Autre

# Liste vide
no-processes = Aucun processus correspondant trouvé.

# Paramètres
settings-title = Paramètres
settings-theme = Thème d'apparence :
theme-system = Système
theme-dark = Sombre
theme-light = Clair
settings-interval = Intervalle de rafraîchissement :
settings-warn-title = Alerte sur le panneau
settings-warn-desc = Afficher un indicateur sur le panneau quand des processus sont bloqués ou suspendus
settings-done = Terminé

# Messages d'état
msg-stopped = Processus PID { $pid } suspendu
msg-stop-failed = Échec de suspension du PID { $pid } : { $error }
msg-resumed = Processus PID { $pid } repris
msg-resume-failed = Échec de reprise du PID { $pid } : { $error }
msg-killed = Processus PID { $pid } tué
msg-kill-failed = Échec de l'arrêt forcé du PID { $pid } : { $error }
msg-killed-all = { $count } processus non répondant{ $count ->
    [one] {" "}tué
   *[other] s tués
}
