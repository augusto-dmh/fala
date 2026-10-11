# meeting-notes-context

## Problem

As notas de reunião (#84) saem sem o contexto de quem as pede. O `NotesPayload` leva título, início, idioma, dicionário, template, anotações e transcrição, mas a ADR-0016 também enumera o perfil do usuário e os nomes dos participantes, que o código ainda não manda. O idioma das notas segue o idioma do ditado, então uma call mista pt/en pode virar notas em inglês. A sessão sem título fica "Reunião + data". Os falantes ficam "Pessoa N" para sempre, porque não há como dar nome a eles. O dicionário vai à Scribe como `keyterms`, mas o desktop liga isso fixo, sem ajuste (pitch F4: +US$ 0,05/h).

Quando isto entra, a pessoa escreve um perfil curto e escolhe o idioma das notas (pt-BR por padrão) nas Reuniões, digita "Quem estava" numa sessão, e a geração de notas devolve, além das seções, um título (usado só se a sessão não tem título) e uma sugestão de nome por "Pessoa N", com uma frase de justificativa e um botão "aplicar". Nada é aplicado sem clique (ADR-0016). Os `keyterms` da Scribe passam a seguir um ajuste ligado por padrão.

## Flow

1. Configurações da página Reuniões -> `notes_profile`, `notes_language`, `meeting_keyterms` em `AppSettings` (door 5)
2. "Quem estava" de uma sessão -> `set_meeting_participants` -> tabela `meeting_participants` (door 4)
3. "Gerar notas" -> `notes_input` monta `NotesInput` com perfil, idioma das notas, participantes e os nomes já aplicados (`Speaker::Named`) -> `NotesPayload::build` (door 1) -> `Claude` (`SYSTEM_PROMPT` e schema da resposta, door 2)
4. `fala-notes` lê a resposta (door 2): renderiza as seções como antes; `title` só vira `Notes::title` se o título da entrada está vazio; `speaker_suggestions` passam pelo filtro (Pessoa N existe e ainda sem nome, nome é um dos participantes, um por pessoa)
5. desktop grava o Markdown, o título gerado só se a sessão ainda está sem título (`UPDATE ... WHERE trim(title) = ''`), e as sugestões em `meeting_speakers` (door 4); os rótulos da transcrição não mudam
6. "aplicar" num chip -> `apply_speaker_suggestion` -> `meeting_speakers.name` -> a transcrição mostra o nome, e a próxima geração manda o nome como rótulo
7. Transcrever de novo apaga `meeting_speakers` da sessão (os números de "Pessoa N" mudam)
8. Transcrever -> `ScribeOptions.keyterms = settings.meeting_keyterms` (door 3)

## Impact

| Front | What changes |
| --- | --- |
| domain | `NotesInput` ganha `profile` e `participants`; `Notes` ganha `title: Option<String>` e `speaker_suggestions: Vec<SpeakerSuggestion>` (`fala-notes`) |
| stored data | duas tabelas novas, aditivas (`CREATE TABLE IF NOT EXISTS`, sem subir `user_version`), como `dictation_metrics`; sessões antigas ficam sem linhas nelas |
| settings | três campos com `serde(default)`; arquivo de settings antigo lê com os padrões |
| UI | `NotesContextSettings` (página Reuniões), `ParticipantsField` e `SpeakerSuggestions` (montados no `MeetingView` com uma linha cada); `MeetingLine.name` mostra o nome aplicado |

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. payload enviado (ADR-0016) | `{"title":"","started_at":"2026-10-02T14:02","language":"pt-BR","profile":"PM do Fala; reuniões de produto","dictionary":["Fala","Parakeet"],"participants":["Ana Souza","Bruno"],"template":{"name","purpose","style","sections":[{"title","instruction"}]},"annotations":[{"id":"a1","text"}],"transcript":[{"id":"s12","t0_ms","t1_ms","speaker":"Pessoa 1","text"}]}`. `language` é o idioma das notas (ajuste, não o do ditado). `profile` aparado e cortado em 300 caracteres. `participants`: nomes aparados, sem vazios, sem repetidos (sem diferenciar maiúsculas), e **sem nenhuma entrada que contenha `@`** (nunca e-mail, nunca a parte local de um). Chaves sempre presentes, mesmo vazias | mandar `is_short_transcript` ou o resumo de outra sessão (Granola): fora da lista da ADR-0016; mandar o participante com e-mail e deixar o LLM ignorar: a ADR-0016 proíbe e-mail |
| 2. resposta pedida ao LLM | `{"title":"Planejamento do lançamento","sections":[{"title","lines":[{"text","sources"}]}],"speaker_suggestions":[{"person":1,"name":"Ana Souza","reason":"Diz que cuida do contrato, como a Ana na anotação a2."}]}`, as três chaves `required` no `json_schema`; `person` é o N de "Pessoa N"; o `SYSTEM_PROMPT` descreve `profile`, `participants`, o título e as sugestões (só com participantes, só nomes da lista, nunca para "Eu" nem para quem já tem nome). Na leitura, `title` e `speaker_suggestions` são opcionais (`serde(default)`) | `title` sempre aplicado: trocaria um título digitado; sugestão aplicada direto: a ADR-0016 exige clique (o Wispr acerta 3 de 5 vozes) |
| 3. requisição à Scribe (ADR-0005) | multipart `model_id`, `language_code`, `diarize`, `timestamps_granularity`, um `keyterms` por termo do dicionário quando `meeting_keyterms` é `true`, e `file`; nada mais (já era o shape da door 3 do `meeting-asr`; muda só que o desktop lê o ajuste) | `keyterms` sempre ligado: a pessoa não teria como evitar os +US$ 0,05/h |
| 4. tabelas novas | `meeting_participants(meeting_id TEXT REFERENCES meetings(id) ON DELETE CASCADE, position INTEGER >= 1, name TEXT, PK(meeting_id, position))` e `meeting_speakers(meeting_id TEXT REFERENCES meetings(id) ON DELETE CASCADE, person INTEGER >= 1, name TEXT NULL, suggested_name TEXT NULL, suggested_reason TEXT NULL, PK(meeting_id, person))` | colunas novas em `meetings`: exigiria subir `user_version` e uma migração ALTER, num arquivo que outros PRs abertos também tocam; o rótulo nomeado dentro de `meeting_segments.speaker`: uma nova transcrição o apagaria sem rastro e o nome teria de ser escrito em N linhas |
| 5. settings | `notes_profile: String` (padrão `""`), `notes_language: "pt-BR" \| "en" \| "app"` (padrão `"pt-BR"`; `app` segue `app_language`, `en*` -> en, o resto -> pt-BR), `meeting_keyterms: bool` (padrão `true`) | o idioma das notas seguir o do ditado (como hoje): numa call mista o resumo podia sair em inglês, e o pitch pede pt-BR por padrão |

- Nothing else in this change is hard to reverse

## Fatias

O diff inteiro passa de 1.800 linhas, acima do teto de 1.000 do CI, então a feature vira PRs empilhados:

- **PR A** (`feat/meeting-notes-context`): doors 1, 2, 3 e 5. O payload e a resposta completos em `fala-notes`, os três ajustes, o título gerado (`set_generated_meeting_title`) e o grupo "Contexto das notas". O desktop ainda manda `participants: []`, então o filtro devolve zero sugestões.
- **PR B** (`feat/meeting-speaker-names`, sobre A): door 4. As tabelas, "Quem estava", as sugestões guardadas, "aplicar" e o nome na transcrição.
- **PR C** (`feat/meeting-notes-templates`, sobre B): templates do usuário.

## Decisões tomadas sem o Augusto (registradas no PR)

- O renomear de `meeting-notes-polish` não existe ainda (nem como PR). Este PR traz o mínimo para "aplicar": o nome aplicado por pessoa, guardado em `meeting_speakers`, mostrado na transcrição e mandado como rótulo. Renomear à mão, desfazer e editar ficam para o `meeting-notes-polish`, que pode reusar a tabela.
- "Configurações → Reuniões": a página Reuniões não tem aba de configurações, e a página de Configurações não tem seção de reunião. Os três ajustes ficam num grupo novo da própria página Reuniões, perto da chave da ElevenLabs.
- Uma sugestão cujo nome não é um dos participantes é descartada, assim como uma para "Pessoa N" que não existe ou que já tem nome.

## Criteria

1. WHEN o payload é montado THEN as chaves de topo SHALL ser exatamente `title`, `started_at`, `language`, `profile`, `dictionary`, `participants`, `template`, `annotations`, `transcript`
2. WHEN os participantes são `["Ana Souza", "bruno@empresa.com", "  ", "ana souza", "Carla <carla@x.com>"]` THEN `participants` SHALL ser `["Ana Souza"]` e nenhum valor do JSON contém `empresa.com` nem `carla@`
3. WHEN o perfil tem mais de 300 caracteres THEN o payload SHALL levar os primeiros 300, aparados
4. WHEN a sessão tem título digitado THEN `Notes::title` SHALL ser `None` mesmo com `title` na resposta; sem título, SHALL ser o título da resposta numa linha
5. WHEN a resposta sugere nome para uma "Pessoa N" inexistente, para "Eu", para quem já tem nome, ou um nome fora dos participantes THEN a sugestão SHALL ser descartada; sem participantes, a lista SHALL ser vazia
6. WHEN uma sugestão volta do servidor falso THEN os rótulos da entrada SHALL continuar os mesmos (nada aplicado)
7. WHEN a sessão é "só local" THEN `generate_notes` SHALL não abrir conexão, sem título e sem sugestões
8. WHEN `meeting_keyterms` é `true` THEN a Scribe falsa SHALL receber um `keyterms` por termo e, além deles, só os cinco campos de sempre; `false` -> nenhum `keyterms`
9. WHEN o desktop grava notas com título gerado THEN o título da sessão SHALL mudar só se estava vazio
10. WHEN uma sugestão é aplicada THEN `meeting_speakers.name` SHALL ser o nome sugerido, a sugestão some, e a próxima `notes_input` SHALL rotular aquela pessoa com o nome; transcrever de novo SHALL apagar os nomes
11. The UI SHALL mostrar um chip "Sugestão: <nome> · aplicar" por pessoa com a justificativa, e nada muda até o clique
