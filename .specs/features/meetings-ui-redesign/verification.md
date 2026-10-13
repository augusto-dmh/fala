# meetings-ui-redesign verification

Windows 11 (Alienware), Bun 1.4.2. Uma linha por check, na fatia que o prova.

| Check | Evidência |
| --- | --- |
| C1 | `bun src/components/meeting/meeting.test.tsx`: passa (C36-key: o campo só em `MeetingKeySetting`, os três blocos fora da página e dentro do grupo Reuniões de Configurações); `bun src/components/meeting/userTemplates.test.tsx`: UT-settings ok (o hook relê no foco; página e Configurações o usam) |
| C2 | `bun src/components/meeting/home.test.tsx`: C2 ok ("Hoje"/"Ontem", "seg., 5 de out." em pt-BR, "Mon, Oct 5" em en, ano fora do corrente, 1º de novembro → ontem; a pauta criada segunda e gravada hoje fica em "Hoje", acima de "Ontem", embora a lista venha por criação) |
| C3 | `home.test.tsx`: C3 ok (precedência gravando > processando > falhou > notas prontas; duração, "rascunho", `bg-rec` com o título do estado, texto `sr-only`) |
| C4 | `home.test.tsx`: C4 ok; `bun src/components/meeting/search.test.tsx`: passa (trecho em `<mark>`, HTML do trecho fica texto) |
| C5 | `home.test.tsx`: C5 ok (sem `SettingsGroup`, `max-w-2xl`, "Nova reunião" → `start(null)` → abre a sessão, troca lista ↔ sessão pelo breadcrumb, o fim de uma transcrição ou das notas não reabre a sessão que a pessoa deixou, parando/processando sem os controles de gravar, faixa só gravando/pausada, aviso de canal, órfão âmbar); `meeting`, `meetingFlow`, `lifecycle`, `import`, `output`, `notesPolish`, `speakerRename`, `meetingPeople` e `shell.test.tsx` passam sem mudança de comportamento |
| C7 | `bun src/components/meeting/screen.test.tsx`: C7 ok (título em `<h1>` editável com o nome acessível e o "Reunião de …" como placeholder, `setMeetingTitle` no Enter ou ao sair; "meeting.home.today · 14:02 · 37:30 · Online", rascunho, importada e "gravando" no lugar da duração enquanto grava) |
| C8 | `screen.test.tsx`: C8 ok (abre em Notas quando existem, em Minhas anotações sem elas e gravando; os dois painéis montados, só um visível; notas que chegam trocam a aba uma vez; rascunho sem abas; "/" sozinho mostra um botão por template, que escolhe e limpa; o campo só zera com outra sessão); `meeting.test.tsx`: um só `<textarea>` gravando, a página sem `AnnotationsField` |
| C9 | `screen.test.tsx`: C9 ok (participantes só pelo "⋯", sugestões sempre; apagar desligado enquanto o pipeline escreve as notas); `meeting.test.tsx`: os quatro itens do menu, copiar transcrição só com transcrição, apagar desligado ocupado; `lifecycle.test.tsx`: a confirmação de apagar |
| C10 | `bun src/components/meeting/bar.test.tsx`: C10 ok (barra `role="toolbar"`; gravando, 12:34, pausar/retomar e parar; parando, só a linha de processamento; rascunho, gravar e o template, sem onda; depois, transcrever, cancelar só transcrevendo, gerar com o template atrás da seta; onda com `aria-pressed`) |
| C11 | `bar.test.tsx`: C11 ok (folha `hidden` até abrir, a fonte de uma nota a abre e rola, Esc fecha, folha em z-30 sob a barra em z-40; `RecorderPanel` sem controles na reunião, com o aviso e duas faixas âmbar para canal mudo e disco); `notesPolish.test.tsx`: as três entradas de gerar passam pela mesma regra |
| C12 | `bun src/components/onboarding/onboarding.test.tsx`: 15 checks ok, C22 novo (duas barras `role="meter"`, aviso e "Entendi", dois campos de chave, pular e concluir; `setMeetingNotesKey`, consentimento só no clique, o teste para ao sair e nada abre na prévia), C1 e C8 com os quatro passos; `cargo test -p fala --lib meeting::`: 41 ok, 2 novos (`rms` do silêncio e de uma onda quadrada; `stop` sem teste e com um rodando); `cargo clippy -p fala --all-targets -D warnings` limpo; `src/bindings.ts` regerado pelo `fala.exe --list-devices` portátil (3 comandos e o evento `MeetingSoundLevel`) |
| C6 | `bun run check:translations` ok; `bun run lint` limpo; `bunx tsc --noEmit` limpo; `bun run format:check` limpo |

Not tested:

- A página no app: clicar "Nova reunião", ver a faixa, abrir e voltar, o menu da seta, arrastar
  um arquivo na lista vazia. Nenhuma mudança em Rust nesta fatia; o app de debug não foi
  dirigido (sem ferramenta para clicar na janela e sem gravar reunião real). Roteiro: Reuniões
  com a lista vazia → arrastar um `.mp3`; "Nova reunião" → aceitar o aviso → a sessão abre
  gravando; "← Reuniões" → faixa com o tempo → "Abrir"; Parar; a linha ganha o ponto verde
  quando as notas chegam.
- Configurações → Reuniões no app: salvar a chave da ElevenLabs (é a chave paga do Augusto).

Revisão das fatias 1 e 2: `/code-review high` sobre as duas fatias achou 10 pontos; 8 corrigidos (reabrir a
sessão ao fim de uma execução, importar arrastando por cima de uma sessão, ordem dos dias pela
criação, controles de gravar durante o processamento, região viva com o relógio, formatadores
recriados a cada quadro, campo da chave sem nome acessível, volta feita à mão em vez do
breadcrumb). Dois ficaram: o ponto vermelho de falha só vale enquanto a página está aberta (o
armazenamento não guarda a falha da última execução) e a duplicação do agrupamento por dia com
o histórico de ditados.

Revisão das fatias 3 e 4: `/code-review high` achou 10 pontos, todos corrigidos. Na fatia 3:
trocar de aba desmontava o editor e a próxima tecla gravava por cima do texto (os dois painéis
agora ficam montados); a aba pulava para Notas a cada montagem; "/" dentro do `<label>` e o
mesmo template não limpava (viraram botões fora do rótulo); a faixa de gravação não seguia o
título renomeado; apagar liberado com as notas do pipeline rodando; a duração parada enquanto
grava; `useDismiss` mudou para `src/hooks`. Na fatia 4: a folha cobria Parar e não fechava no
Esc; dois seletores de template enquanto grava.
