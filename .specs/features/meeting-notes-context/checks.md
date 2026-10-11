# meeting-notes-context checks

Profile: light
Plan: `.specs/features/meeting-notes-context/plan.md`

17 checks · 5 one-way doors · 0 open · PR A: C1-C10, C17 · PR B: C11-C16, C17

Todo `cargo` roda com `CARGO_TARGET_DIR=C:\f\meeting-notes-context`. O crate `fala` (desktop) só compila com a trava `.houston/cargo-lock.txt`. Nenhum teste toca a rede: Claude e Scribe são servidores falsos em `127.0.0.1:0`, com chave falsa.

## Checks

### S1 - payload e resposta (`fala-notes`) · PR A

**C1** - As chaves de topo do payload são exatamente `title`, `started_at`, `language`, `profile`, `dictionary`, `participants`, `template`, `annotations`, `transcript`, com `profile` e `participants` da fixture (AC 1, door 1)
Proof: `cargo test -p fala-notes --lib payload::tests::payload_keys_are_exactly_the_enumerated_list -- --exact`

**C2** - Participantes `["  Ana   Souza ", "bruno@empresa.com", "  ", "ana souza", "Carla <carla@x.com>", "Dani"]` viram `["Ana Souza", "Dani"]`; nenhum valor do JSON contém `@` nem `empresa.com`; sem participantes, `[]` (AC 2, door 1)
Proof: `cargo test -p fala-notes --lib payload::tests::participants_are_names_never_emails -- --exact`

**C3** - Perfil com 350 `é` entre espaços sai com exatamente 300 caracteres; perfil só de espaço sai `""` (AC 3, door 1)
Proof: `cargo test -p fala-notes --lib payload::tests::profile_is_trimmed_and_capped -- --exact`

**C4** - Com título digitado, `Notes::title` é `None` mesmo com `title` na resposta; sem título, é a resposta numa linha (`"Data do lançamento de novembro"`), cortada em 80; título em branco ou ausente na resposta e sessão "só local" dão `None` (AC 4, door 2)
Proof: `cargo test -p fala-notes --lib render::tests::generated_title_never_replaces_the_users -- --exact`

**C5** - Das oito sugestões da fixture sobra só `{1, "Bruno Lima", "Propõe a data, como o Bruno na pauta."}`: caem a segunda para a mesma pessoa, "Pessoa 7" e "Pessoa 0" inexistentes, "Ana" (já é rótulo), "Carla" (fora dos participantes), o e-mail e o nome repetido; sem participantes e em "só local", nenhuma (AC 5, door 2)
Proof: `cargo test -p fala-notes --lib render::tests::invalid_speaker_suggestions_are_dropped -- --exact`

**C6** - Servidor falso: o conteúdo da mensagem tem as nove chaves, `profile` e `language` `pt-BR`, `participants` `["Bruno Lima"]`, o corpo cru não contém `empresa.com`, o schema pede `title`, `sections` e `speaker_suggestions`; a resposta dá `title` e uma sugestão; a geração seguinte ainda manda `"Pessoa 1"` (AC 1, 2, 6, doors 1 e 2)
Proof: `cargo test -p fala-notes --test fake_claude payload_carries_profile_and_names_never_emails -- --exact`

**C7** - Sessão "só local": nenhuma conexão no servidor falso, chave não lida, `title` `None` e nenhuma sugestão (AC 7)
Proof: `cargo test -p fala-notes --test fake_claude local_only_never_connects_nor_reads_key -- --exact`

### S2 - Scribe, ajustes e título no desktop · PR A

**C8** - Com `keyterms` ligado e dicionário `["Fala", "ADR"]`, cada requisição tem `keyterms` = `["Fala", "ADR"]`, os nomes de campo são exatamente `diarize`, `file`, `keyterms`, `language_code`, `model_id`, `timestamps_granularity` e há 7 partes; desligado, nenhum `keyterms` (AC 8, door 3)
Proof: `cargo test -p fala-asr --test meeting_scribe request::keyterms_follow_the_setting -- --exact`

**C9** - Padrões: `notes_language` `pt-BR`, `meeting_keyterms` `true`, `notes_profile` `""`; `app` segue a interface (`en-US` -> en, `pt-BR`/`es` -> pt-BR); perfil, idioma e dicionário dos ajustes chegam ao `NotesSettings` e ao `NotesInput`; `scribe_options` segue `meeting_keyterms` (AC 8, door 5)
Proof: `cargo test -p fala --lib meeting::context::tests` e `cargo test -p fala --lib meeting::pipeline::tests`

**C10** - O título gerado só entra numa sessão com título em branco, e só uma vez (AC 9)
Proof: `cargo test -p fala-storage --test meetings generated_title_never_replaces_a_typed_one -- --exact`

### S3 - nomes de falante · PR B

**C11** - Um banco aberto tem `meeting_participants` e `meeting_speakers`; reabrir não falha (door 4)
Proof: `cargo test -p fala-storage --test meeting_people tables_exist_after_open -- --exact`

**C12** - Participantes trocados por inteiro, na ordem; sessão inexistente dá `NotFound` (door 4)
Proof: `cargo test -p fala-storage --test meeting_people participants_are_replaced_in_order -- --exact`

**C13** - Sugestão gravada não muda `name` nem os trechos; `apply` devolve o nome, limpa a sugestão e uma segunda vez dá `NotFound`; uma geração nova troca as sugestões sem mexer no nome aplicado (AC 10, door 4)
Proof: `cargo test -p fala-storage --test meeting_people suggestion_becomes_a_name_only_when_applied -- --exact`

**C14** - Transcrever de novo apaga nomes e sugestões da sessão (AC 10)
Proof: `cargo test -p fala-storage --test meeting_people transcribing_again_forgets_names -- --exact`

**C15** - Um nome aplicado vira `Speaker::Named` no `NotesInput` e "Eu" nunca muda; os participantes digitados são aparados, sem vazios, até 30, cada um até 80 caracteres (AC 10)
Proof: `cargo test -p fala --lib meeting::people::tests` e `cargo test -p fala --lib meeting::pipeline::tests`

**C16** - UI: `parseParticipants` separa por vírgula, `;` e linha; a lista mostra a sugestão, "aplicar" e a frase, e nada sem sugestão; `applySpeakerSuggestion` aparece uma vez, atrás do clique; `lineText` usa o nome aplicado; todas as chaves novas existem em pt e en (AC 11)
Proof: `bun src/components/meeting/meetingPeople.test.tsx`

### Gates · PR A e PR B

**C17** - `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, `bun run lint`, `bunx prettier --check .`, `bunx tsc --noEmit`, `bun run check:translations`
Proof: os comandos acima, sem erro

## Sem prova automática

- Uma reunião real com chaves (Windows): título gerado, sugestões aplicadas por clique, notas em pt-BR numa call com trechos em inglês. Fica para o Augusto.
