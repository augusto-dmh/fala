# meetings-ui-redesign checks

Profile: light
Plan: `.specs/features/meetings-ui-redesign/plan.md`

### S1 - configurações de reunião em Configurações (PR 1)

**C1** - chave da ElevenLabs, contexto das notas e templates em Configurações → Reuniões e fora da página; sem a chave, a página aponta para lá; os templates relidos ao voltar o foco nos dois lugares
Proof: `bun src/components/meeting/meeting.test.tsx`; `bun src/components/meeting/userTemplates.test.tsx`

### S2 - início de Reuniões (PR 2)

**C2** - reuniões agrupadas por dia local, na ordem da lista: "Hoje", "Ontem" e "seg., 5 de out." (com o ano fora do ano corrente); virada de mês
Proof: `bun src/components/meeting/home.test.tsx`

**C3** - cada linha tem hora, título, duração (ou "rascunho") e um ponto de estado: gravando, processando, notas prontas, falhou
Proof: `bun src/components/meeting/home.test.tsx`

**C4** - busca troca os grupos pelos resultados com o trecho marcado; vazio mostra "Nenhuma reunião ainda" e o arrastar
Proof: `bun src/components/meeting/search.test.tsx`; `bun src/components/meeting/home.test.tsx`

**C5** - a página não usa `SettingsGroup`; "Nova reunião" grava e abre a sessão; gravando, a faixa com o tempo e "Abrir"; órfão em âmbar e erro no topo; abrir troca a lista pela sessão com "← Reuniões"
Proof: `bun src/components/meeting/home.test.tsx`; `meeting.test.tsx`, `meetingFlow.test.tsx`, `lifecycle.test.tsx`, `import.test.tsx`

### Todas as fatias

**C6** - strings novas em pt e en; lint, tsc, format
Proof: `bun run check:translations`; `bun run lint`; `bunx tsc --noEmit`; `bun run format:check`
