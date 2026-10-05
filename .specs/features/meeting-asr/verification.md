# meeting-asr verification

**Verdict**: PASS
**Profile**: light
**Diff range**: 3e0332d..47a8292
**Round**: 1 - full
**Verifier**: independent sub-agent (author != verifier) - Verifier despachado pelo orquestrador, sem contexto herdado; o autor é o sub-agente executor do meeting-asr

Escopo: os 3 commits de feature sobre o plano `3e0332d` (`3d78a91`, `2ca3392`, `47a8292`), com a
base sendo o merge `3eaedf9` de `origin/main` na branch da trilha A. Todas as 17 checks (18
proofs) rodadas em HEAD `47a8292` com o comando literal do `checks.md`, cada uma com
`running 1 test` e o nome do teste aparecendo individualmente como `ok`. O diff do `checks.md` em
`3e0332d..HEAD` só acrescenta o marcador `(built)` a cada check: nenhuma afirmação, proof ou linha
de cobertura mudou depois da aprovação. O `plan.md` ganhou 4 linhas de Assumptions (decididas pelo
executor) e a pergunta aberta 3; nenhum AC nem door mudou.

## Binding sources

Perfil `light`: o passo 1 não roda. Fontes lidas só para as confirmações pedidas: ADR-0005
(`docs/decisions/0005-...md`, seção Decisão), ADR-0008 (BYOK, keyring), ADR-0003 (via plano).
Nenhuma contradição encontrada entre elas e as checks.

## Checks

Proofs verified at 47a8292. `G` = `crates/asr/src/meeting/guard.rs`, `S` =
`crates/asr/src/meeting/segments.rs`, `L` = `crates/asr/src/meeting/local.rs`, `T` =
`crates/asr/tests/meeting_scribe.rs`.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | `ElevenLabsScribe` não é `Transcriber`; envio recebe `&MeetingRecording` | `cargo test -p fala-asr --lib meeting::guard::tests::scribe_is_not_a_dictation_transcriber -- --exact` exit 0, 1 passed | `crates/asr/src/meeting/guard.rs:76` - `assert_not_impl_any!(ElevenLabsScribe: Transcriber)`; `crates/asr/src/meeting/guard.rs:78` - `let _send: ScribeSend = <ElevenLabsScribe as MeetingTranscriber>::transcribe_session` (tipo fixa `&MeetingRecording`); os outros `pub fn` do tipo são só `new` e `with_base_url` (`elevenlabs.rs:65`, `:74`), `post` é privado | PASS |
| C2 | dois arquivos -> caminhos e modo; falta um -> `MissingChannel(nome)` | `cargo test -p fala-asr --lib meeting::guard::tests::from_session_dir_needs_both_channels -- --exact` exit 0, 1 passed | `crates/asr/src/meeting/guard.rs:97` - `assert_eq!(recording.mic(), both.join("mic.opus"))` (e `:98`, `:99` para `sys.opus` e o modo); `crates/asr/src/meeting/guard.rs:104` - `matches!(err, AsrError::MissingChannel("sys.opus"))`; `crates/asr/src/meeting/guard.rs:112` - `matches!(err, AsrError::MissingChannel("mic.opus"))` | PASS |
| C3 | `meeting`: 2 POSTs, bytes exatos, diarize false/true, conjunto exato de campos | `cargo test -p fala-asr --test meeting_scribe request::meeting_sends_exactly_the_enumerated_fields -- --exact` exit 0, 1 passed | `crates/asr/tests/meeting_scribe.rs:278` - `assert_eq!(requests.len(), 2)`; `:282` - `r.target == "/v1/speech-to-text"`; `:283`-`:285` - `model_id`=`["scribe_v2"]`, `language_code`=`["por"]`, `timestamps_granularity`=`["word"]`; `:287`-`:296` - nomes == `[diarize, file, language_code, model_id, timestamps_granularity]`; `:297` - `r.parts().len() == 5`; `crates/asr/tests/meeting_scribe.rs:300` - mic (achado por `r.file() == MIC_BYTES`) `diarize == ["false"]`; `:302` - sys `diarize == ["true"]` | PASS |
| C4 | keyterms ligado: exatamente `Fala`, `ADR`; desligado: 0 | `cargo test -p fala-asr --test meeting_scribe request::keyterms_follow_the_setting -- --exact` exit 0, 1 passed | `crates/asr/tests/meeting_scribe.rs:321` - `assert_eq!(r.field("keyterms"), ["Fala", "ADR"])`; `crates/asr/tests/meeting_scribe.rs:337` - `assert!(r.field("keyterms").is_empty())` | PASS |
| C5 | `in_person`: mic com `diarize=true` | `cargo test -p fala-asr --test meeting_scribe request::in_person_diarizes_the_mic -- --exact` exit 0, 1 passed | `crates/asr/tests/meeting_scribe.rs:349` - `assert_eq!(mic.field("diarize"), ["true"])` | PASS |
| C6 | "detectar": sem `language_code` | `cargo test -p fala-asr --test meeting_scribe request::auto_language_omits_language_code -- --exact` exit 0, 1 passed | `crates/asr/tests/meeting_scribe.rs:369` - `assert!(!r.field_names().contains("language_code"))` (com `:367` exigindo 2 requisições) | PASS |
| C7 | chave só no header `xi-api-key`; erro de 401 sem a chave | `cargo test -p fala-asr --test meeting_scribe request::key_only_in_header_and_never_in_errors -- --exact` exit 0, 1 passed | `crates/asr/tests/meeting_scribe.rs:384` - `assert_eq!(r.header("xi-api-key"), Some(KEY))`; `:385` - `!r.target.contains(KEY)`; `:386` - `find(&r.body, KEY.as_bytes()).is_none()`; `crates/asr/tests/meeting_scribe.rs:416` - `assert!(!err.to_string().contains(KEY))` sobre um 401 cujo corpo ecoa a chave (`:410` `status: Some(401)`) | PASS |
| C8 | agrupa por falante; `audio_event` não cria falante | `cargo test -p fala-asr --lib meeting::segments::tests::groups_consecutive_words_by_speaker -- --exact` exit 0, 1 passed | `crates/asr/src/meeting/segments.rs:233` - `t0_ms: 120` / `:234` `t1_ms: 900` (1ª e 2ª palavra) e `:239` `t0_ms: 1_600` (3ª), dentro do `assert_eq!(grouped, vec![..])` de `:228`; `crates/asr/src/meeting/segments.rs:249` - falantes == `[person(1), person(2)]` | PASS |
| C9 | `meeting`: mic é `me`; sistema por primeira aparição | `cargo test -p fala-asr --lib meeting::segments::tests::mic_is_me_system_numbered_by_first_appearance -- --exact` exit 0, 1 passed | `crates/asr/src/meeting/segments.rs:277` - `assert!(mic.iter().all(..s.speaker == Speaker::Me))` (todos os segmentos do mic); `crates/asr/src/meeting/segments.rs:285` - `[("bom", person(1)), ("dia", person(2)), ("certo", person(1))]` (`speaker_7` antes de `speaker_2`); `:293` - `system_only` | PASS |
| C10 | `in_person`: mic 1..k, sistema continua em k+1 | `cargo test -p fala-asr --lib meeting::segments::tests::in_person_numbering_continues_across_channels -- --exact` exit 0, 1 passed | `crates/asr/src/meeting/segments.rs:314` - `(Channel::System, "d", person(3))`; `:315` - `(Channel::Mic, "b", person(2))`, no `assert_eq!(got, [..])` de `:310` | PASS |
| C11 | merge por `t0`, mic antes em empate | `cargo test -p fala-asr --lib meeting::segments::tests::merge_orders_by_t0_mic_first_on_tie -- --exact` exit 0, 1 passed | `crates/asr/src/meeting/segments.rs:347` - `(5_000, Channel::Mic)` seguido de `:348` `(5_000, Channel::System)`, no `assert_eq!` de `:342` com a sequência literal 0/2000/5000/5000/9000 | PASS |
| C12 | forma JSON exata da door 4 e ida e volta | `cargo test -p fala-asr --lib meeting::segments::tests::segment_serialized_form -- --exact` exit 0, 1 passed | `crates/asr/src/meeting/segments.rs:372` - `{"channel":"mic","speaker":"me","t0_ms":0,"t1_ms":1200,"text":"oi"}`; `:374` - volta igual; `crates/asr/src/meeting/segments.rs:386` - `{"channel":"system","speaker":{"person":2},...}`; `:388` - volta igual | PASS |
| C13 | 429/503/fechada -> retriable; 401/422 -> não, com status; áudio intacto | `cargo test -p fala-asr --test meeting_scribe progress::failures_keep_audio_and_classify_retry -- --exact` exit 0, 1 passed | `crates/asr/tests/meeting_scribe.rs:449` - `assert_eq!(retriable, want_retriable)` sobre a tabela `:435`-`:439`; `:451` - `assert_eq!(status, want_status)`; `crates/asr/tests/meeting_scribe.rs:456` - `std::fs::read(recording.mic()) == MIC_BYTES` (e `:457`-`:461` para `SYS_BYTES`) | PASS |
| C14 | resposta em 3 s: ≥ 3 chamadas, nenhum intervalo > 1 s | `cargo test -p fala-asr --test meeting_scribe progress::reports_at_least_every_second -- --exact` exit 0, 1 passed (3.01 s) | `crates/asr/tests/meeting_scribe.rs:483` - `assert!(calls.len() >= 3)`; `crates/asr/tests/meeting_scribe.rs:488` - `assert!(gap <= Duration::from_secs(1))` em cada par a partir do início | PASS |
| C15 | cancelar em 500 ms com servidor de 5 s: `Cancelled` em < 1,5 s | `cargo test -p fala-asr --test meeting_scribe progress::cancel_returns_within_a_second -- --exact` exit 0, 1 passed (0.51 s) | `crates/asr/tests/meeting_scribe.rs:513` - `matches!(err, AsrError::Cancelled)`; `crates/asr/tests/meeting_scribe.rs:514` - `elapsed < Duration::from_millis(1_500)` | PASS |
| C16 | "só local": 0 conexões; janelas 0/60 000/120 000 ms | `cargo test -p fala-asr --lib meeting::local::tests::windows_of_60_s_shift_timestamps -- --exact` exit 0, 1 passed; `cargo test -p fala-asr --test meeting_scribe local::local_only_opens_no_connection -- --exact` exit 0, 1 passed | `crates/asr/src/meeting/local.rs:217` - janelas `[960_000, 960_000, 160_000]` amostras; `crates/asr/src/meeting/local.rs:226` - `(System, 60_000, 120_000, ..)` e `:227` `(System, 120_000, 130_000, ..)`; `crates/asr/tests/meeting_scribe.rs:563` - `assert_eq!(server.connections(), 0)` e `:564` `requests().is_empty()` | PASS |
| C17 | depois de `Network`, `transcribe_with_fallback` devolve os segmentos locais | `cargo test -p fala-asr --lib meeting::local::tests::network_error_falls_back_to_local -- --exact` exit 0, 1 passed | `crates/asr/src/meeting/local.rs:297` - `assert_eq!(cloud.calls, 1)`; `crates/asr/src/meeting/local.rs:305` - `[(Channel::Mic, 0, 30_000), (Channel::System, 0, 30_000)]` (saída do transcritor falso) | PASS |

Nível: C3-C7, C13-C15 e a segunda proof de C16 cruzam a fronteira HTTP de verdade (servidor
`TcpListener` em `127.0.0.1:0`, requisição crua parseada byte a byte), o nível que a afirmação
pede. C8-C12 testam funções puras de agrupamento/numeração/serialização, que é onde a afirmação
mora. C17 usa uma nuvem falsa que devolve `Network` sintético; a ida de HTTP -> `Network` já está
provada em C13, então o par cobre o caminho.

## Coverage

Perfil `light`: o recompute do join não é obrigatório. Conferência pontual feita contra o plano,
não contra a tabela do autor:

| Set (size) | Recomputed from | Member -> proof | Unproven |
| --- | --- | --- | --- |
| statuses de `POST /v1/speech-to-text` (6) | `Surface` do plano | `200` C3 · `401` C7, C13 · `422` C13 · `429` C13 · `5xx` C13 (`503`) · rede C13 (`Reply::Close`) | - |
| campos multipart da door 3 (6) | `Landing` door 3 | `model_id`, `file`, `language_code`, `diarize`, `timestamps_granularity` C3 (conjunto exato) · `keyterms` C4 | - |
| `MeetingRoute` (3) | `local.rs:111` | `Cloud` C17 (2º bloco) · `CloudThenLocal` C17 · `LocalOnly` C16 | - |

## Faults injected

Não exigido no perfil `light`; rodado a pedido, num worktree descartável
(`/tmp/claude-1000/verify-masr`, `git worktree add ... HEAD --detach`, removido depois). O
`git status --porcelain` do worktree real era vazio antes e continuou vazio depois.

| Mutation | Location | Killed |
| --- | --- | --- |
| `diarize = mode == InPerson` -> `true` (mic de `meeting` com diarização) | `crates/asr/src/meeting/elevenlabs.rs:180` | yes - C3 FAILED |
| `retriable` sem o `status == 429` | `crates/asr/src/meeting/elevenlabs.rs:279` | yes - C13 FAILED |
| tirar o `return Err(Cancelled)` do laço de espera | `crates/asr/src/meeting/elevenlabs.rs:136` | yes - C15 FAILED (5.01 s) |
| numeração presencial desligada (mic vira `me` em `in_person`) | `crates/asr/src/meeting/segments.rs:122` | yes - C10 FAILED |
| `MeetingRoute::LocalOnly` chama a nuvem | `crates/asr/src/meeting/local.rs:132` | yes - C16 (integração) FAILED |

## Confirmações pedidas

- `cargo check -p fala-cli` no tree mesclado: exit 0 (`Checking fala-cli v0.1.0 ... Finished`).
  O merge `3eaedf9` resolveu `apps/cli/Cargo.toml` e `apps/cli/src/main.rs`; nenhum marcador de
  conflito nos dois (`grep '<<<<<<<\|>>>>>>>'` vazio).
- `cargo clippy -p fala-asr --all-targets -- -D warnings`: exit 0, sem aviso.
- `scripts/check-no-tauri-in-crates.sh`: `ok: no tauri in crates/`, exit 0.
- Sem rede real nem chave real nos testes: a única URL externa em `crates/asr` é
  `SCRIBE_BASE_URL` (`elevenlabs.rs:26`); todo `ElevenLabsScribe` de teste passa por
  `with_base_url(&server.url)` (`meeting_scribe.rs:251`), e o servidor é `127.0.0.1:0`
  (`meeting_scribe.rs:142`). A única chave é `sk-teste-123` (`meeting_scribe.rs:23`). Os testes de
  lib não constroem cliente HTTP.
- Erros sem o corpo da resposta: `AsrError::Network` de status não-2xx carrega só
  `format!("HTTP {status}")` (`elevenlabs.rs:281`) e os de transporte só o `ureq::Error`
  (`elevenlabs.rs:306`-`311`); o corpo nunca é lido nesses caminhos, e C7 prova isso com um 401
  que ecoa a chave. Ressalva (fora de qualquer check): `AsrError::InvalidResponse` usa o
  `to_string()` do `serde_json` (`elevenlabs.rs:161`), que em erro de tipo cita o valor
  inesperado (ex.: `invalid type: string "abc", expected f64`), então um 200 malformado pode pôr
  um fragmento do corpo no erro. Não carrega a chave (que não vem no corpo de um 200) e não há log
  no módulo, mas é o único caminho em que algo do corpo chega a um `AsrError`.
- ADR-0005: a nuvem só é chamada pela rota que a sessão escolhe. `transcribe_with_fallback`
  despacha por `MeetingRoute` (`local.rs:131`-`142`); `LocalOnly` nunca toca a nuvem, provado por
  C16 com 0 conexões e pela falta F5 acima. O mic de `system_only`/`import` não sai da máquina
  (`elevenlabs.rs:179`, `has_mic()`). Ainda não há chamador que ligue o ajuste "só local" da
  sessão a `MeetingRoute::LocalOnly`; isso fica para quem integrar (desktop/CLI).
- ADR-0008: `ElevenLabsScribe::new` só aceita `fala_secrets::ApiKey` (`elevenlabs.rs:65`), cujo
  `Debug` é `ApiKey([REDACTED])` e que não tem `Display` (`crates/secrets/src/lib.rs:34`);
  `ElevenLabsScribe` não deriva `Debug`; a chave só sai por `key.expose()` no header
  (`elevenlabs.rs:268`). Nenhum `log::`/`tracing`/`println!` em `crates/asr/src/meeting`
  (`grep` vazio).
- `tests/manifest.rs` continua exato, não afrouxado: `assert_eq!` sobre a lista ordenada inteira
  de `[dependencies]` com 8 nomes (`crates/asr/tests/manifest.rs:29`) e sobre a lista inteira de
  `[dev-dependencies]` (`crates/asr/tests/manifest.rs:48`, `hound` e `static_assertions`); a pinagem
  de `transcribe-rs` e as negativas `sherpa`/`tauri` seguem iguais. A lista cresceu exatamente nas
  deps novas do `Cargo.toml` e do `Cargo.lock`.

## Observações (não mudam o veredito)

1. `merge_orders_by_t0_mic_first_on_tie` (`segments.rs:352`-`357`): o comentário diz "sistema
   primeiro na entrada", mas a chamada é `merge(mic, system)`, como no primeiro bloco. Como
   `merge` concatena mic antes do sistema e `sort_by_key` é estável, o desempate por `channel` em
   `segments.rs:144` é redundante hoje (mutante equivalente); o teste não exercita a ordem de
   entrada que o comentário promete.
2. C13 afirma "mesmo sha256"; o teste compara os bytes inteiros (`meeting_scribe.rs:456`), o que
   é equivalente ou mais forte.
3. `transport_error` marca todo `ureq::Error` como `retriable: true`, inclusive estouro do teto de
   64 MB da resposta (`elevenlabs.rs:287`-`289`), que repetir não resolve. Não está em AC.
4. A pergunta aberta 1 do plano (nomes de campo da Scribe v2) continua bloqueando o go-live; o
   servidor falso prova a forma da door 3, não que a ElevenLabs a aceite.

## Gate

`cargo test -p fala-asr` - 22 passed, 0 failed, 1 ignored (`tests/parakeet.rs::transcribes_real_speech`, da trilha A, precisa de `FALA_TEST_PARAKEET_DIR`): lib 10, `manifest` 1, `meeting_scribe` 9, `parakeet` 1, `trait_shape` 1.
