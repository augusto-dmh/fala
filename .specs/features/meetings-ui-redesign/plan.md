# meetings-ui-redesign — a área de Reuniões como notas, não como página de configurações

Profile: light · Fase 2, beta · quatro PRs empilhados

## Problem

`MeetingsPage.tsx` empilha cinco `SettingsGroup` num `max-w-3xl` (gravador, lista, sessão
aberta, contexto das notas, templates). A lista é plana e a sessão aberta aparece embaixo dela,
na mesma rolagem. O onboarding não tem passo de reunião.

## Slices

O início de Reuniões passa de 1000 linhas com os testes; a mudança das configurações sai antes,
numa fatia própria.

- **PR 1** (`feat/meetings-ui-redesign`): chave da ElevenLabs, contexto das notas e templates
  saem da página e vão para Configurações → Reuniões; sem a chave, a página só aponta para lá.
- **PR 2** (`-2`): página de uma coluna (`max-w-2xl`) sem `SettingsGroup`. Topo: "● Nova
  reunião" (grava e abre a sessão; a seta ao lado abre modo, som da reunião, título, rascunho de
  pauta e copiar aviso), "Importar" e a busca. Lista agrupada por dia local ("Hoje", "Ontem",
  "seg., 6 de out."), linha com ponto de estado, hora, título e duração. Estados: vazio, faixa de
  gravação com "Abrir", órfão em âmbar, erro em linha. Abrir uma reunião troca a lista pela
  sessão ("← Reuniões").
- **PR 3** (`-3`): a sessão em tela própria com título editável, "Minhas anotações | Notas",
  barra inferior fixa, transcrição em folha lateral e menu "⋯".
- **PR 4** (`-4`): passo opcional "Reuniões" no onboarding.

## Doors

1. O gravador ao vivo (pausar, parar, níveis, avisos) só aparece na sessão que grava; a home
   mostra a faixa com o tempo e "Abrir". O aviso de terceiros (ADR-0005) continua antes da
   primeira gravação.
2. "Importar" continua o ponto de entrada da importação (o CSV do Granola, #145, entra nele).
