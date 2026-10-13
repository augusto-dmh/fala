# meetings-ui-redesign verification

Windows 11 (Alienware), Bun 1.4.2. Fatias 1 e 2 (PR 1 e PR 2); as fatias 3 e 4 entram aqui
quando forem feitas.

| Check | Evidência |
| --- | --- |
| C1 | `bun src/components/meeting/meeting.test.tsx`: passa (C36-key: o campo só em `MeetingKeySetting`, os três blocos fora da página e dentro do grupo Reuniões de Configurações); `bun src/components/meeting/userTemplates.test.tsx`: UT-settings ok (o hook relê no foco; página e Configurações o usam) |
| C2 | `bun src/components/meeting/home.test.tsx`: C2 ok ("Hoje"/"Ontem", "seg., 5 de out." em pt-BR, "Mon, Oct 5" em en, ano fora do corrente, 1º de novembro → ontem; a pauta criada segunda e gravada hoje fica em "Hoje", acima de "Ontem", embora a lista venha por criação) |
| C3 | `home.test.tsx`: C3 ok (precedência gravando > processando > falhou > notas prontas; duração, "rascunho", `bg-rec` com o título do estado, texto `sr-only`) |
| C4 | `home.test.tsx`: C4 ok; `bun src/components/meeting/search.test.tsx`: passa (trecho em `<mark>`, HTML do trecho fica texto) |
| C5 | `home.test.tsx`: C5 ok (sem `SettingsGroup`, `max-w-2xl`, "Nova reunião" → `start(null)` → abre a sessão, troca lista ↔ sessão pelo breadcrumb, o fim de uma transcrição ou das notas não reabre a sessão que a pessoa deixou, parando/processando sem os controles de gravar, faixa só gravando/pausada, aviso de canal, órfão âmbar); `meeting`, `meetingFlow`, `lifecycle`, `import`, `output`, `notesPolish`, `speakerRename`, `meetingPeople` e `shell.test.tsx` passam sem mudança de comportamento |
| C6 | `bun run check:translations` ok; `bun run lint` limpo; `bunx tsc --noEmit` limpo; `bun run format:check` limpo |

Not tested:

- A página no app: clicar "Nova reunião", ver a faixa, abrir e voltar, o menu da seta, arrastar
  um arquivo na lista vazia. Nenhuma mudança em Rust nesta fatia; o app de debug não foi
  dirigido (sem ferramenta para clicar na janela e sem gravar reunião real). Roteiro: Reuniões
  com a lista vazia → arrastar um `.mp3`; "Nova reunião" → aceitar o aviso → a sessão abre
  gravando; "← Reuniões" → faixa com o tempo → "Abrir"; Parar; a linha ganha o ponto verde
  quando as notas chegam.
- Configurações → Reuniões no app: salvar a chave da ElevenLabs (é a chave paga do Augusto).

Revisão: `/code-review high` sobre as duas fatias achou 10 pontos; 8 corrigidos (reabrir a
sessão ao fim de uma execução, importar arrastando por cima de uma sessão, ordem dos dias pela
criação, controles de gravar durante o processamento, região viva com o relógio, formatadores
recriados a cada quadro, campo da chave sem nome acessível, volta feita à mão em vez do
breadcrumb). Dois ficaram: o ponto vermelho de falha só vale enquanto a página está aberta (o
armazenamento não guarda a falha da última execução) e a duplicação do agrupamento por dia com
o histórico de ditados.
