# Encabezado
app-title = Administrador de tareas
process-count = { $count } procesos
cpu-label = CPU { $percent }%
ram-label = RAM { $used } / { $total } ({ $percent }%)

# Búsqueda y pestañas
search-placeholder = Buscar procesos o PID...
tab-all = Todos
tab-apps = Aplicaciones
tab-top-cpu = Mayor CPU
tab-top-ram = Mayor RAM
tab-unresponsive = Detenidos / Bloqueados

# Banner de alerta
alert-unresponsive = ¡Se { $count ->
    [one] detectó { $count } proceso detenido o bloqueado!
   *[other] detectaron { $count } procesos detenidos o bloqueados!
}
btn-inspect = Inspeccionar
btn-kill-all = Terminar todos

# Acciones
btn-stop = Detener
btn-resume = Reanudar
btn-kill = Forzar salida

# Estados de proceso
status-run = Ejecución
status-sleep = Suspendido
status-stopped = DETENIDO
status-zombie = ZOMBI
status-hung = BLOQUEADO
status-other = Otro

# Sin resultados
no-processes = No se encontraron procesos coincidentes.

# Configuración
settings-title = Configuración
settings-theme = Tema de apariencia:
theme-system = Sistema
theme-dark = Oscuro
theme-light = Claro
settings-interval = Intervalo de actualización:
settings-warn-title = Alerta en panel
settings-warn-desc = Mostrar advertencia en el panel cuando haya procesos bloqueados o detenidos
settings-done = Listo

# Mensajes de estado
msg-stopped = Proceso PID { $pid } detenido
msg-stop-failed = Error al detener PID { $pid }: { $error }
msg-resumed = Proceso PID { $pid } reanudado
msg-resume-failed = Error al reanudar PID { $pid }: { $error }
msg-killed = Proceso PID { $pid } terminado
msg-kill-failed = Error al terminar PID { $pid }: { $error }
msg-killed-all = Se { $count ->
    [one] terminó { $count } proceso bloqueado
   *[other] terminaron { $count } procesos bloqueados
}
