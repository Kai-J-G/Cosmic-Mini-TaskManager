# Cabeçalho
app-title = Gestor de Tarefas
process-count = { $count } processos
cpu-label = CPU { $percent }%
ram-label = RAM { $used } / { $total } ({ $percent }%)

# Pesquisa e abas
search-placeholder = Pesquisar processos ou PID...
tab-all = Todos
tab-apps = Aplicações
tab-top-cpu = Maior CPU
tab-top-ram = Maior RAM
tab-unresponsive = Parados / Bloqueados

# Faixa de alerta
alert-unresponsive = { $count } processo{ $count ->
    [one] {" "}parado ou bloqueado detetado!
   *[other] s parados ou bloqueados detetados!
}
btn-inspect = Inspecionar
btn-kill-all = Terminar todos

# Ações
btn-stop = Parar
btn-resume = Retomar
btn-kill = Terminar

# Estados do processo
status-run = A executar
status-sleep = Suspenso
status-stopped = PARADO
status-zombie = ZOMBIE
status-hung = BLOQUEADO
status-other = Outro

# Nenhum resultado
no-processes = Nenhum processo correspondente encontrado.

# Definições
settings-title = Definições
settings-theme = Tema de aparência:
theme-system = Sistema
theme-dark = Escuro
theme-light = Claro
settings-interval = Intervalo de atualização:
settings-warn-title = Alerta no painel
settings-warn-desc = Mostrar indicador no painel quando existirem processos bloqueados ou parados
settings-done = Concluído

# Mensagens de estado
msg-stopped = Processo PID { $pid } parado
msg-stop-failed = Falha ao parar PID { $pid }: { $error }
msg-resumed = Processo PID { $pid } retomado
msg-resume-failed = Falha ao retomar PID { $pid }: { $error }
msg-killed = Processo PID { $pid } terminado
msg-killed-tree = PID { $pid } e { $count } { $count ->
    [one] processo filho
   *[other] processos filhos
} terminados
msg-kill-failed = Falha ao terminar PID { $pid }: { $error }
msg-killed-all = { $count } processo{ $count ->
    [one] {" "}bloqueado terminado
   *[other] s bloqueados terminados
}
