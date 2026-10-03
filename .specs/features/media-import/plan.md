# media-import — arquivo local de áudio ou vídeo vira o canal de uma sessão de importação

## Problem

Aulas, vídeos e gravações que já estão no disco não têm como entrar no Fala. O design doc §3.4
item 5 diz que "importação de arquivo e URL via `yt-dlp` e `ffmpeg`" usa a "mesma sessão, mesmo
pipeline" da reunião, e o pitch da fase 2 (F7) registra o custo: "aulas e vídeos ficam sem
transcrição". Hoje nada no workspace chama o ffmpeg, `fala-cli transcribe <arquivo>` é um stub e
não existe um formato combinado para o áudio que uma sessão de importação entrega ao pipeline da
reunião (que está nascendo em paralelo em `crates/meeting`).

A decisão D3 do pitch, confirmada pela proposta de roadmap de 2026-10-02 (decisão 12, opção c),
fixa a distribuição por enquanto: ffmpeg exigido no PATH, sem download e sem embutir. O pitch não
traz números de uso; a evidência é qualitativa (o Granola não importa arquivo nem vídeo).

Quando isto fechar, `fala-cli import aula.mp4` escreve `aula.fala.wav` (WAV mono 48 kHz i16, o
mesmo formato por canal do gravador de reunião), mostra o progresso no stderr, falha com mensagem
clara e tipada quando falta ffmpeg, falta arquivo ou falta trilha de áudio, e o crate
`fala-media` oferece a mesma conversão com progresso e cancelamento para a sessão de reunião
consumir depois.

## Flow

Reusa o ffmpeg e o ffprobe do sistema (D3, opção c) e o `hound` que já está no lockfile para ler
o cabeçalho do WAV escrito; não reimplementa decodificação nem resample, e não depende de
`crates/meeting` nem de `crates/audio`.

1. `fala-cli import <arquivo> [--out <wav>]` -> `Cli` clap em `apps/cli/src/main.rs` (exists) - valida a entrada com `fala_media::ensure_input`, localiza as ferramentas com `Tools::locate` (door 3) e chama `fala_media::import`
2. `fala_media::import` em `crates/media` (door 1, API door 5) - recusa `out` existente, checa o token de cancelamento, roda o ffprobe (door 4) e decide: sem trilha de áudio, ilegível ou longo demais encerra aqui sem criar arquivo
3. `fala_media::import` - inicia o ffmpeg (door 4) escrevendo em `<out>.part`, emite `Progress { processed: 0, total }` e depois um `Progress` por bloco `progress=` do `-progress pipe:1`; a cada ≤ 100 ms confere o `CancelToken` e, se cancelado, mata o ffmpeg e apaga o `.part`
4. `fala_media::import` - com o ffmpeg em status 0, renomeia `<out>.part` para `out`, lê o cabeçalho com `hound` (exists, no lockfile) e devolve `ImportedAudio` (door 2)
5. out: o WAV em `out`; na CLI, tabela Markdown de uma linha no stdout, progresso e erros no stderr, exit 0/1/2

## Impact

| Front | What changes |
| --- | --- |
| domain | novo termo: `import` - converter um arquivo local de áudio ou vídeo no áudio que uma sessão do modo "importação" entrega ao pipeline da reunião; vive em `crates/media` |
| domain | novo termo: `ImportedAudio` - o WAV produzido mais taxa, canais e duração lidos do cabeçalho; é o que `crates/meeting` vai receber no lugar do canal do sistema gravado |
| domain | termo existente: `fala-cli transcribe <arquivo>` continua stub; o pitch diz que ele "passa a ser essa entrada" quando a transcrição (F4) existir; ninguém ramifica nele hoje |
| build | crate novo `fala-media` em `crates/media`, membro por `crates/*`; dependências `hound 3.5.1`, `log`, `thiserror`, todas já no `Cargo.lock`; `fala-cli` ganha a aresta `fala-media`; Code Map do `ARCHITECTURE.md` ganha a linha no mesmo commit |
| runtime | primeiro processo filho do workspace fora do desktop: `ffprobe` e `ffmpeg` resolvidos no PATH, argumentos em vetor, nunca shell |
| CI | o job `rust` (ubuntu-24.04) não instala ffmpeg; os testes que dependem dele ficam `#[ignore]` com o motivo e rodam aqui com `--include-ignored`; os de arquivo inexistente e ffmpeg ausente rodam sempre |
| stored data | nada a migrar: o WAV fica onde `--out` (ou quem chama a API) apontar; nada entra em SQLite, na pasta de dados do app nem no repo |

## Relations

`None - no stored-data shape change` (um WAV por importação, no caminho que o chamador escolhe;
a pasta da sessão em `audio/<id>/` é porta da F1, não desta feature).

## Surface

Só o que esta feature adiciona.

| Route | In | Out | Status |
| --- | --- | --- | --- |
| `fala-cli import` | `<arquivo>` (obrigatório), `--out <wav>` (default `<stem do arquivo>.fala.wav` no diretório atual) | stdout: tabela Markdown de uma linha `out`, `sample_rate`, `channels`, `duration_s`; stderr: uma linha de progresso por evento e erros; disco: o WAV | exit `0` sucesso · `1` ffmpeg ou ffprobe fora do PATH, ou conversão falhou · `2` arquivo inexistente, ilegível, sem trilha de áudio, longo demais ou saída já existente; sem status HTTP (comando local; 200-599 n/a) |
| `fala_media::import` (API Rust para `crates/meeting`) | `&Tools`, origem `&Path`, destino `&Path`, `&CancelToken`, `&mut dyn FnMut(Progress)` | `Ok(ImportedAudio { path, sample_rate, channels, duration })` | `Ok` · `Err(InputNotFound)` · `Err(OutputExists)` · `Err(Unreadable)` · `Err(NoAudioTrack)` · `Err(TooLong)` · `Err(Ffmpeg)` · `Err(Cancelled)` · `Err(Io)`; sem status HTTP (chamada em processo; 200-599 n/a) |
| `fala_media::Tools::locate` / `locate_in` | o PATH do processo, ou um valor de PATH explícito | `Ok(Tools)` com os caminhos do `ffmpeg` e do `ffprobe` | `Ok` · `Err(ToolNotFound { tool: "ffmpeg" })` · `Err(ToolNotFound { tool: "ffprobe" })`; sem status HTTP (chamada em processo; 200-599 n/a) |

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. crate novo | `crates/media`, pacote `fala-media`, `fala-media = { path = "crates/media" }` em `[workspace.dependencies]`, linha no Code Map; deps `hound = "3.5.1"`, `log = "0.4"`, `thiserror.workspace`; sem `cfg` de plataforma (ADR-0007) | dentro de `crates/audio`: é o crate de captura e plataforma e está com outro painel; a importação não captura nada e só precisa de subprocesso. Dentro de `crates/meeting`: em construção por outro painel e faria a CLI depender da máquina de estados para converter um arquivo |
| 2. formato do áudio importado | WAV RIFF, PCM s16le, 1 canal, 48 000 Hz, primeira trilha de áudio da origem misturada para mono pelo `-ac 1`; escrito em `<out>.part` e renomeado para `out` só com o ffmpeg em status 0 | 16 kHz mono (pronto para o Parakeet): o arquivo vira o Opus retido (F3, 48 kHz) e a entrada da Scribe; reduzir antes degrada os dois, e o resample para 16 kHz já é do fallback local. Estéreo com o mic (L) em silêncio: o layout da sessão é porta da F1. Opus direto pelo ffmpeg: a D2 põe o encoder Opus em `crates/audio` (libopus); dois encoders para o mesmo arquivo retido |
| 3. onde as ferramentas moram | `ffmpeg` e `ffprobe` (mais `std::env::consts::EXE_SUFFIX`) procurados em cada diretório de `std::env::split_paths(PATH)`, primeiro arquivo encontrado vence; nada é baixado (D3, opção c) | baixar no primeiro uso com hash fixado (D3 b, ADR candidata 0017): abre URLs externas e fica para antes do tissue test. Crate `which`: dependência nova para 20 linhas. Só ffmpeg, lendo `Duration:` do stderr: formato humano que muda entre versões; o ffprobe vem em toda distribuição do ffmpeg |
| 4. invocação dos subprocessos | ffprobe: `-v error -protocol_whitelist file -show_entries stream=codec_type:format=duration -of default=nw=1 file:<origem absoluta>`. ffmpeg: `-nostdin -hide_banner -loglevel error -protocol_whitelist file -i file:<origem absoluta> -map 0:a:0 -vn -sn -dn -ac 1 -ar 48000 -c:a pcm_s16le -f wav -progress pipe:1 -stats_period 0.5 -nostats -y file:<out>.part`; sempre `Command::arg`, nunca shell | caminho sem `file:`: o ffmpeg interpreta prefixos como `http:` e `concat:`. Sem `-protocol_whitelist file`: uma playlist local (`.m3u8`, `.sdp`) pode abrir rede dependendo da versão do ffmpeg, e a importação de arquivo local não deve fazer chamada de rede |
| 5. API que `crates/meeting` vai consumir | `pub fn import(tools: &Tools, input: &Path, out: &Path, cancel: &CancelToken, on_progress: &mut dyn FnMut(Progress)) -> Result<ImportedAudio, MediaError>`; `CancelToken` é `Clone` sobre `Arc<AtomicBool>` com `cancel()`/`is_cancelled()`; `Progress { processed: Duration, total: Option<Duration> }`; `ImportedAudio { path: PathBuf, sample_rate: u32, channels: u16, duration: Duration }`; `MediaError` com as variantes da Surface | API assíncrona ou por canal: nenhum crate tem runtime async; o desktop roda a chamada numa thread e repassa o `Progress` como evento. Receber um `Session` da F1: o tipo ainda não existe em `main` e criaria dependência circular com o painel de `crates/meeting` |

- Nothing else in this change is hard to reverse: o nome da flag `--out`, o default `<stem>.fala.wav`, o texto das mensagens, o teto de 12 h e o layout dos módulos mudam num commit, e nada fora do repo os consome ainda.

## Criteria

### S1: arquivo local vira WAV mono 48 kHz (P1)

Um mp3, um mp4 com vídeo ou um ogg viram o WAV que a sessão de importação entrega ao pipeline.

**Acceptance Criteria**

1. WHEN `import` recebe um arquivo com trilha de áudio (mp3, mp4 com vídeo e áudio estéreo, ogg/opus) com um tom de 440 Hz de 1 s THEN o sistema SHALL escrever em `out` um WAV PCM 16 bits, 1 canal, 48 000 Hz, com 1 s ± 60 ms de duração, contendo o tom (440 ± 10 Hz por cruzamentos de zero, pico ≥ 0,1 da escala cheia)
2. WHEN `import` conclui THEN o sistema SHALL devolver `ImportedAudio` com `path` = `out`, `sample_rate` = 48 000, `channels` = 1 e `duration` igual à duração lida do cabeçalho do WAV escrito
3. WHEN a origem tem duas trilhas de áudio (440 Hz na primeira, 880 Hz na segunda) THEN o sistema SHALL converter só a primeira (o WAV tem 440 ± 10 Hz)
4. IF `out` já existe THEN o sistema SHALL devolver `OutputExists(out)` sem iniciar o ffprobe nem o ffmpeg e sem alterar um byte de `out`
5. The system SHALL escrever a conversão em `<out>.part` e só renomeá-la para `out` com o ffmpeg em status 0, de modo que após um `Ok` `<out>.part` não exista

**Independent test:** `cargo test -p fala-media -- --include-ignored` gera os três arquivos com o próprio ffmpeg e confere o WAV.

### S2: falhas tipadas, nunca panic (P1)

Cada falha previsível volta como uma variante de `MediaError` com mensagem que diz o que fazer.

**Acceptance Criteria**

6. IF nenhum diretório do PATH dado contém `ffmpeg` THEN `Tools::locate_in` SHALL devolver `ToolNotFound { tool: "ffmpeg" }`, cuja mensagem contém `ffmpeg` e `PATH`
7. IF o PATH dado contém `ffmpeg` e nenhum diretório contém `ffprobe` THEN `Tools::locate_in` SHALL devolver `ToolNotFound { tool: "ffprobe" }`
8. IF a origem não existe ou não é um arquivo THEN `import` SHALL devolver `InputNotFound(origem)` sem iniciar subprocesso e sem criar `out` nem `<out>.part`
9. IF a origem não tem trilha de áudio (mp4 só com vídeo) THEN `import` SHALL devolver `NoAudioTrack(origem)` sem iniciar o ffmpeg e sem criar `out` nem `<out>.part`
10. IF o ffprobe não reconhece a origem (um arquivo de texto) THEN `import` SHALL devolver `Unreadable { path, detail }` com `detail` não vazio, sem criar `out` nem `<out>.part`
11. IF a duração sondada passa de 12 h (o limite de 4 GiB do WAV a 96 000 bytes/s) THEN `import` SHALL devolver `TooLong { duration, max: 12 h }` sem iniciar o ffmpeg
12. IF o ffmpeg sai com status diferente de 0 THEN `import` SHALL devolver `Ffmpeg { code, detail }` com o final do stderr (no máximo 2 000 bytes, não vazio) e sem deixar `out` nem `<out>.part`

**Independent test:** `cargo test -p fala-media` (sem o ffmpeg) cobre 6-8; o resto com `--include-ignored`.

### S3: progresso e cancelamento (P1)

Quem chama vê o avanço e pode parar no meio, sem lixo no disco.

**Acceptance Criteria**

13. WHEN o ffmpeg é iniciado THEN o sistema SHALL chamar `on_progress` com `Progress { processed: 0, total: Some(duração sondada) }` antes de qualquer outro evento
14. WHILE o ffmpeg converte, o sistema SHALL chamar `on_progress` uma vez por bloco `progress=` do `-progress pipe:1` (o ffmpeg é invocado com `-stats_period 0.5`), com `processed` = `out_time_us` do bloco, nunca decrescente, e o último evento com `processed` = duração do WAV ± 60 ms
15. WHEN `CancelToken::cancel()` é chamado com o ffmpeg rodando THEN o sistema SHALL matar o ffmpeg e devolver `Cancelled` em até 1 s, sem deixar `out` nem `<out>.part`
16. IF o token já está cancelado quando `import` é chamado THEN o sistema SHALL devolver `Cancelled` sem iniciar subprocesso e sem criar `out` nem `<out>.part`

**Independent test:** teste que cancela dentro do primeiro `on_progress` e confere o tempo e o disco.

### S4: importação local não abre rede (P1)

O arquivo é local; nada nele faz o ffmpeg conectar.

**Acceptance Criteria**

17. IF a origem é uma playlist local `.m3u8` cujo segmento é `http://127.0.0.1:<porta>/a.ts` THEN `import` SHALL devolver erro (`Unreadable`, `NoAudioTrack` ou `Ffmpeg`) e o socket que escuta em `<porta>` SHALL ter recebido 0 conexões

**Independent test:** o teste abre um `TcpListener` em 127.0.0.1:0, escreve a playlist apontando para ele e confere `accept` = `WouldBlock`.

### S5: `fala-cli import` (P1)

O mesmo caminho, sem UI.

**Acceptance Criteria**

18. WHEN `fala-cli import <arquivo> --out <wav>` conclui THEN o sistema SHALL sair com 0, escrever o WAV e imprimir no stdout só a tabela Markdown de uma linha `| out | sample_rate | channels | duration_s |` com `48000`, `1` e a duração com 3 casas
19. WHERE `--out` não é dado THEN o sistema SHALL escrever `<stem do arquivo>.fala.wav` no diretório atual
20. WHILE converte, o sistema SHALL escrever no stderr uma linha `import: <processed> s / <total> s (<pct> %)` por evento de progresso, com 1 casa nos segundos e `?` no total e no percentual quando o total é desconhecido
21. IF o ffmpeg ou o ffprobe não está no PATH THEN o sistema SHALL sair com 1 e o stderr SHALL conter `ffmpeg` (ou `ffprobe`) e `PATH`
22. IF o arquivo não existe THEN o sistema SHALL sair com 2 antes de procurar o ffmpeg, nomeando o arquivo no stderr
23. IF a origem não tem trilha de áudio ou `--out` já existe THEN o sistema SHALL sair com 2

**Independent test:** `PATH= fala-cli import x.mp3` sai com 1; `fala-cli import nao-existe.mp3` sai com 2; `fala-cli import tom.mp3 --out t.wav` sai com 0 e `ffprobe t.wav` diz 48000/1.

## Out of scope

Product capabilities only. Process and harness rules live in AGENTS.md or as Observable `n/a`.

| Excluded | Why |
| --- | --- |
| Importação por URL (`yt-dlp -x`), título e URL no frontmatter | fica fora desta rodada por instrução do orquestrador; a D3 ainda decide a distribuição do yt-dlp e a URL do usuário vira argumento de subprocesso (porta própria) |
| Baixar ffmpeg/ffprobe no primeiro uso, com hash fixado | D3 opção (b), ADR candidata 0017, antes do tissue test; abre URLs externas |
| Criar a sessão do modo "importação", a pasta `audio/<id>/` e o registro no storage | F1 (`crates/meeting`, outro painel) e F5; esta feature entrega só a conversão e a API que eles vão chamar |
| Codificar o Opus 24 kbps retido | F3, encoder libopus em `crates/audio` (D2) |
| Transcrever o áudio importado; `fala-cli transcribe` deixa de ser stub | F4 |
| Modo "gravar áudio do sistema" sem mic | parte de gravação da F7, depende do gravador da F1 |
| Escolher a trilha de áudio (idioma, comentário do diretor) | a primeira trilha cobre aula e vídeo; seletor só se aparecer o caso |
| Ctrl+C limpo no `fala-cli import` | o cancelamento é provado na API; na CLI o Ctrl+C mata o processo e pode deixar `<out>.part`, que a próxima execução sobrescreve |

## Assumptions

Defaults que não são critério numerado. Decisões que viraram porta estão no `Landing`; aqui ficam
as que o painel tomou sem porta.

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| formato do áudio que a sessão de importação consome | WAV mono 48 kHz s16 como canal "sistema" (door 2) | é o formato por canal do gravador de reunião (cli-record door 2) e a entrada do Opus retido da F3; 16 kHz é só do fallback local | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| exigir também o ffprobe no PATH | sim (door 3) | toda distribuição do ffmpeg traz o ffprobe; a saída key=value é estável, o `Duration:` do stderr não | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| testes que precisam do ffmpeg no CI | `#[ignore = "precisa de ffmpeg e ffprobe no PATH"]`, rodados aqui com `--include-ignored`; o job `rust` do CI não instala ffmpeg e `.github/` está fora da fronteira deste painel | mesmo padrão dos testes de `record` que precisam do PipeWire; follow-up: instalar `ffmpeg` no CI e tirar o `ignore` | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| interface para `crates/meeting` | só a função `import` e os tipos da door 5; nenhuma dependência de `crates/meeting` | o painel da sessão escolhe o destino (`audio/<id>/…`) e chama `import` numa thread; registrado no status da rodada | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| default de `--out` | `<stem>.fala.wav` no diretório atual, nunca sobrescreve | um `.wav` de origem no diretório atual não colide; a recusa de sobrescrever protege o arquivo do usuário | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |
| janela de console do ffmpeg no Windows | `TODO(windows)`: um app GUI precisa de `CREATE_NO_WINDOW`, que é `cfg(windows)` e fica para `apps/desktop` ou para um ajuste em ADR quando a F7 ligar no desktop | ADR-0007 não permite `cfg` em `crates/media`; a CLI roda num console, então não pisca | y — delegado pelo Augusto em 2026-10-02, decidido pelo painel |

**Open questions:** none - all resolved or logged above.

## Observable

| Surface | Decision | Landing |
| --- | --- | --- |
| comando `fala-cli import` | formato de saída e verbosidade | AC 18, AC 20 |
| comando `fala-cli import` | cada flag e seu default | AC 19 (`--out`); `<arquivo>` é obrigatório pelo clap |
| comando `fala-cli import` | exit codes | AC 18, AC 21, AC 22, AC 23 |
| comando `fala-cli import` | o que imprime quando falha no meio | AC 12 (stderr do ffmpeg no erro), AC 5 e AC 12 (sem arquivo parcial) |
| API `fala_media::import` | forma da resposta | AC 2 |
| API `fala_media::import` | forma do erro e variantes | AC 4, AC 8-12, AC 15-16 |
| API `fala_media::Tools::locate*` | forma do erro | AC 6, AC 7 |
| API `fala_media::*` | quem pode chamar | n/a - biblioteca local em processo, sem fronteira de autorização |
| API `fala_media::*` | versionamento | n/a - `publish = false`, consumida só dentro do workspace; mudar a assinatura muda os chamadores no mesmo commit |
| API `fala_media::*` | limite de taxa | n/a - uma chamada por arquivo, iniciada pelo usuário |

## Sources

- `docs/design/2026-10-fala-v1.md` §3.4 item 5 - importação de arquivo pela mesma sessão e pelo mesmo pipeline da reunião
- `fala-research/pitches/fase-2-reuniao-videos.md` F7 e D3 (opção c agora) - escopo e distribuição do ffmpeg
- `docs/decisions/0005` - dois canais retidos localmente, WAV durante e Opus depois
