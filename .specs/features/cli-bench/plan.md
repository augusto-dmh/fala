# cli-bench — benchmark de WER/RTF de ASR em pt-BR

## Problem

A ADR-0003 aposta o ditado inteiro em Parakeet-TDT-0.6B-v3 int8 rodando localmente, com base em
números de leaderboard (WER pt ~6 %, treino majoritariamente pt-PT) que nunca foram medidos na voz
do Augusto nem em nenhuma das duas máquinas. O pitch da fase 0 diz literalmente: "Quatro apostas
técnicas sustentam o plano e nenhuma foi medida nesta máquina ou no Windows (...). Se qualquer uma
falhar, a arquitetura muda. Descobrir isso na fase 1 custa semanas; na fase 0 custa dias." Hoje não
existe no repositório nenhuma forma de calcular WER ou RTF de um modelo sobre áudio de referência:
`fala-cli bench` é um stub que sai com erro.

Quando isto fechar, uma pessoa aponta `fala-cli bench` para uma pasta de cortes WAV e uma pasta de
transcrições corrigidas e recebe uma tabela Markdown com WER e RTF por corte e no total, para
qualquer modelo Parakeet ONNX ou GGUF (whisper, Nemotron, Voxtral) e para hipóteses geradas por
scripts externos (faster-whisper). Essa tabela é o que confirma ou substitui a ADR-0003.

## Flow

Reusa os dois runtimes já na árvore de dependências do `apps/desktop`: `transcribe-rs` (Parakeet
ONNX via ONNX Runtime, o caminho que o produto vai embarcar) e `transcribe-cpp` (GGUF: whisper,
Nemotron, Parakeet, Voxtral, com `n_threads` e backend Vulkan/CUDA), em vez de compilar sherpa-onnx
ou NeMo-Speech.cpp; o WER é implementado uma vez, em Rust, e serve às engines externas pelo formato
de hipóteses (door 2).

1. `fala-cli bench --cuts <dir> --refs <dir> (--engine … --model … | --hyp <dir>) [--threads N] [--device cpu|gpu] [--chunk-s N] [--out <dir>] [--tag <label>]` -> `Cli` clap em `apps/cli/src/main.rs` (exists) - valida a combinação de flags; combinação inválida sai com 2 antes de tocar disco
2. leitor de corpus (new, no door - placement per conventions) - lista `<cuts>/*.wav` em ordem de nome, exige `<refs>/<stem>.txt` para cada um, valida o WAV (16 kHz, mono, 16-bit PCM via `hound`, exists na árvore); falha sai com 2 antes de carregar modelo
3. engine (door 1) - `transcribe_rs::onnx::parakeet::ParakeetModel` (exists) ou `transcribe_cpp::Model` + `Session` (exists) - carrega o modelo medindo `load_s`, transcreve cada corte medindo `wall_s` (com `--chunk-s`, janelas consecutivas somadas); ou, com `--hyp`, lê `<hyp>/<stem>.txt` e `<hyp>/<stem>.wall_s` (door 2)
4. WER (new, door 3 fixa a normalização) - normaliza referência e hipótese, Levenshtein por palavra, devolve S/D/I/N por corte e o agregado
5. out: uma linha de legenda e uma tabela Markdown no stdout; com `--out`, grava `<out>/<stem>.txt` e `<out>/<stem>.wall_s` (door 2); sai com 0

Todo o código novo fica em `apps/cli` (código de spike; a migração para `crates/asr` é da fase 1,
como o `ARCHITECTURE.md` já prevê). Nada entra em `crates/` nem em `apps/desktop`.

## Impact

| Front | What changes |
| --- | --- |
| domain | novo termo: `cut` - um trecho WAV de referência (16 kHz mono) com transcrição corrigida de mesmo `stem`; vive em `apps/cli` e no `benchmarks/README.md` do `fala-research` |
| domain | novo termo: `hypothesis` - o texto bruto que uma engine devolveu para um `cut`, antes de qualquer normalização; é o que os scripts externos trocam com a CLI |
| domain | novo termo: `RTF` - `wall_s / audio_s` da transcrição, sem o tempo de carga do modelo (menor é mais rápido; o relatório 03 usa o inverso, "x RT") |
| build | `cargo build -p fala-cli` passa a compilar ONNX Runtime (binário baixado pelo crate `ort` no build, como já acontece no desktop) e transcribe.cpp (CMake + toolchain C++); o job `fala-cli builds without WebView` do CI fica mais lento mas não ganha dependência de sistema nova (Vulkan/glslc já são instalados para o desktop) |
| build | `Cargo.lock` muda só nas arestas de `fala-cli`; nenhum crate novo entra no grafo do workspace |
| stored data | nada a migrar: cortes, referências e hipóteses ficam em `~/projects/fala-research/benchmarks/` (WAVs derivados fora do git) e não são lidos por mais ninguém |

## Relations

`None - no stored-data shape change` (a CLI lê e escreve arquivos soltos fora do repositório; nada
persiste no app).

## Surface

Só o subcomando que esta feature implementa; `dictate`, `record` e `transcribe` seguem stubs.

| Route | In | Out | Status |
| --- | --- | --- | --- |
| `fala-cli bench` | `--cuts <dir>`, `--refs <dir>`, um de `--engine parakeet-onnx` / `--engine gguf` com `--model <path>`, ou `--hyp <dir>`; opcionais `--threads N` (só gguf), `--device cpu` / `--device gpu` (só gguf, default `cpu`), `--chunk-s N` (default 0 = arquivo inteiro), `--out <dir>`, `--tag <label>` | stdout: 1 linha de legenda `engine=… model=… threads=… device=… load_s=… tag=…` + tabela Markdown `cut · audio_s · wall_s · rtf · wer_% · sub · del · ins · ref_words` com linha final `total`; stderr: diagnósticos; `--out`: `<stem>.txt` e `<stem>.wall_s` | exit `0` sucesso · `1` falha de engine ou backend indisponível · `2` entrada inválida (flags, corpus, WAV, referência vazia); não há status HTTP (comando local, códigos 200-599 n/a) |

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. `apps/cli` passa a depender dos dois runtimes de ASR já na árvore | em `apps/cli/Cargo.toml`: `transcribe-rs = { version = "0.3.8", features = ["onnx"] }`, `transcribe-cpp = { version = "0.2.3", default-features = false }`, `hound = "3.5.1"`, `log`, `env_logger`; `[features] vulkan = ["transcribe-cpp/vulkan"]`, `cuda = ["transcribe-cpp/cuda"]` (GPU só por feature de build; default CPU) | `sherpa-rs`: exige build CMake do sherpa-onnx fora da árvore e mede um runtime que o produto não embarca. `whisper-rs`: segundo binding do whisper.cpp quando `transcribe-cpp` já está no lockfile e ainda cobre Nemotron/Voxtral/Parakeet GGUF |
| 2. formato de troca de hipóteses com scripts externos | diretório com `<stem>.txt` (hipótese bruta, UTF-8, sem normalizar) e, opcional, `<stem>.wall_s` (um número decimal ASCII, segundos de parede da transcrição, sem carga de modelo); `--out` escreve, `--hyp` lê | JSON por engine: cada script externo reimplementaria o WER ou um schema; texto plano é diffável e deixa o WER num só lugar |
| 3. normalização de texto do WER | NFC → minúsculas → toda sequência de caracteres que não sejam letra ou dígito Unicode vira um espaço → split em espaço; WER = (S+D+I)/N em % com 2 casas; agregado = Σ(S+D+I)/ΣN, nunca média de WERs; hífen e apóstrofo separam palavras nos dois lados | normalização padrão do `jiwer` (só em Python; dois cálculos divergem entre engines); `EnglishTextNormalizer` do Whisper (só inglês) |

- Nothing else in this change is hard to reverse: nomes de flag, layout do módulo e formato exato da tabela mudam num commit e ninguém fora do repo os consome além do `benchmarks/README.md`, que é editado à mão.

## Criteria

### S1: WER e agregado (P1)

Dado um par referência/hipótese, o sistema calcula S/D/I/N e o WER, e agrega por corpus.

**Acceptance Criteria**

1. WHEN o sistema compara uma referência e uma hipótese THEN o sistema SHALL normalizar as duas pela door 3 e SHALL reportar `sub`, `del`, `ins` e `ref_words` da menor edição por palavra (Levenshtein), com `wer_%` = (sub+del+ins)/ref_words × 100 com 2 casas
2. WHEN a referência `Olá, mundo!` é comparada com a hipótese `olá mundo` THEN o sistema SHALL reportar `wer_%` = `0.00`
3. WHEN a referência `o guarda-chuva ficou` é comparada com `o guarda chuva ficou aqui` THEN o sistema SHALL reportar `sub`=0 `del`=0 `ins`=1 `ref_words`=4 e `wer_%` = `25.00`
4. WHEN todos os cortes foram pontuados THEN o sistema SHALL imprimir uma linha `total` com `ref_words` = Σ ref_words, `wer_%` = Σ(sub+del+ins)/Σ ref_words × 100 e `rtf` = Σ wall_s / Σ audio_s
5. IF uma referência tem 0 palavras após a normalização THEN o sistema SHALL sair com 2 nomeando o arquivo, antes de carregar qualquer modelo

**Independent test:** `cargo test -p fala-cli` com os pares de 2 e 3 como casos; `fala-cli bench --hyp` sobre um corte cuja hipótese é igual à referência imprime `0.00`.

### S2: engine `parakeet-onnx` (P1)

O caminho da ADR-0003: Parakeet v3 int8 via `transcribe-rs`, medido por corte.

**Acceptance Criteria**

6. WHEN `--engine parakeet-onnx --model <dir>` é dado THEN o sistema SHALL carregar `<dir>` com `ParakeetModel::load(dir, Quantization::Int8)`, transcrever cada corte com os 250 ms de silêncio inicial padrão da engine e imprimir uma linha por corte com `audio_s`, `wall_s`, `rtf` = wall_s/audio_s (3 casas) e as colunas de WER
7. The system SHALL medir `load_s` como o tempo de `ParakeetModel::load` (ou `Model::load` + `session` no gguf), reportá-lo só na linha de legenda e SHALL excluí-lo de `wall_s`
8. IF `--threads` ou `--device gpu` é dado com `--engine parakeet-onnx` THEN o sistema SHALL sair com 2 dizendo que a engine não expõe essa opção (o ONNX Runtime usa o número de threads padrão)

**Independent test:** `fala-cli bench --cuts … --refs … --engine parakeet-onnx --model ~/.local/share/com.pais.handy/models/parakeet-tdt-0.6b-v3-int8` sobre 1 corte de 1 min imprime legenda + 2 linhas de tabela (corte e total) com `rtf` < 1.

### S3: engine `gguf` com threads e device (P1)

Whisper large-v3-turbo, Nemotron 3.5 e qualquer GGUF que o `transcribe.cpp` reconheça, com controle de threads e de backend.

**Acceptance Criteria**

9. WHEN `--engine gguf --model <arquivo.gguf|.bin>` é dado THEN o sistema SHALL carregar com `transcribe_cpp::Model::load_with`, abrir a sessão com `SessionOptions.n_threads` = `--threads` (default: `std::thread::available_parallelism`), rodar `Session::run` com `language = Some("pt")` e imprimir uma linha por corte como no AC 6
10. WHEN `--device gpu` é dado e a build tem um backend de GPU compilado (`vulkan` ou `cuda`) com dispositivo disponível THEN o sistema SHALL carregar o modelo nesse backend e SHALL escrever `device=<nome do Model::device()>` na legenda
11. IF `--device gpu` é dado e nenhum backend de GPU está compilado ou nenhum dispositivo está disponível THEN o sistema SHALL sair com 1 nomeando o backend ausente, sem cair para CPU
12. IF a engine devolve erro num corte THEN o sistema SHALL escrever o erro e o `stem` no stderr e sair com 1, mantendo no stdout as linhas já impressas

**Independent test:** `fala-cli bench --cuts … --refs … --engine gguf --model ggml-large-v3-turbo.bin --threads 8` imprime `threads=8 device=cpu` na legenda; `--device gpu` numa build sem feature sai com 1.

### S4: corpus e recorte (P1)

O leitor de corpus rejeita tudo o que faria o número mentir, antes de gastar tempo de modelo.

**Acceptance Criteria**

13. WHEN `--cuts <dir>` contém arquivos `.wav` THEN o sistema SHALL processá-los em ordem crescente de nome de arquivo (ordem de bytes) e ignorar qualquer outra extensão
14. IF `--cuts <dir>` não contém nenhum `.wav` THEN o sistema SHALL sair com 2
15. IF um corte não tem `<refs>/<stem>.txt` THEN o sistema SHALL sair com 2 nomeando o `stem`, antes de carregar qualquer modelo
16. IF um WAV não é 16 000 Hz, 1 canal, 16-bit PCM inteiro THEN o sistema SHALL sair com 2 nomeando o arquivo e a especificação encontrada (taxa, canais, bits)
17. WHERE `--chunk-s N` com N > 0 é dado THEN o sistema SHALL transcrever janelas consecutivas de N segundos sem sobreposição, juntar os textos com um único espaço e reportar `wall_s` como a soma das janelas

**Independent test:** pasta com um WAV de 48 kHz sai com 2 e a mensagem cita `48000`; `--chunk-s 30` num corte de 65 s produz 3 janelas e uma única linha de tabela.

### S5: hipóteses externas (P2)

faster-whisper e outras engines sem binding Rust entram pelo formato da door 2.

**Acceptance Criteria**

18. WHEN `--out <dir>` é dado com uma engine THEN o sistema SHALL gravar `<dir>/<stem>.txt` com a hipótese bruta e `<dir>/<stem>.wall_s` com o `wall_s` do corte, criando `<dir>` se não existir
19. WHEN `--hyp <dir>` é dado sem `--engine` THEN o sistema SHALL pontuar `<dir>/<stem>.txt` para cada corte e imprimir a mesma tabela; a legenda SHALL trazer `engine=hyp model=<dir>`
20. IF `<hyp>/<stem>.wall_s` não existe THEN o sistema SHALL imprimir `-` em `wall_s` e `rtf` daquele corte e `-` no `rtf` da linha `total`
21. IF `--hyp` é combinado com `--engine`, `--model`, `--threads`, `--device` ou `--chunk-s` THEN o sistema SHALL sair com 2
22. IF `--hyp <dir>` não tem `<stem>.txt` para algum corte THEN o sistema SHALL sair com 2 nomeando o `stem`

**Independent test:** rodar a engine com `--out h/` e depois `--hyp h/` sobre os mesmos cortes imprime as mesmas colunas de WER.

### S6: saída e diagnóstico (P1)

O stdout é a tabela e só a tabela; o conteúdo das entrevistas não vaza no terminal.

**Acceptance Criteria**

23. The system SHALL imprimir no stdout exatamente uma linha de legenda `engine=<v> model=<basename> threads=<n|-> device=<v> load_s=<v> tag=<v|->` seguida do cabeçalho e das linhas da tabela Markdown; tudo o mais SHALL ir para o stderr
24. The system SHALL escrever texto de referência ou de hipótese apenas nos arquivos de `--out` e no log em nível `debug` (via `log`), nunca no stdout nem em nível `info` ou acima
25. WHEN a CLI termina um corte THEN o sistema SHALL imprimir a linha daquele corte imediatamente (stdout com flush por linha), para que uma execução longa mostre progresso

**Independent test:** `fala-cli bench … | head -1` é a legenda; `grep -c` de uma palavra da referência no stdout devolve 0.

### S7: relatório e tabela de rodadas (P2)

O spike fecha com evidência gravada nos dois repositórios, sem recomendação.

**Acceptance Criteria**

26. WHEN o spike fecha THEN `docs/spikes/01-asr-pt-br.md` SHALL existir com os títulos `## Objetivo`, `## Como reproduzir` e `## Evidência medida`, e SHALL NOT conter título com "Recomenda"
27. The section `## Como reproduzir` SHALL listar os comandos exatos (`ffmpeg` dos cortes, origem de cada modelo, cada invocação de `fala-cli bench` e do script de faster-whisper) e o commit do `fala` usado
28. The section `## Evidência medida` SHALL comparar o WER e o RTF medidos do Parakeet v3 int8 com os valores que a ADR-0003 cita (WER pt ~6 %; 10-20x tempo real) e SHALL registrar, para cada rabbit hole (Vulkan na iGPU; Nemotron GGUF ou NeMo-Speech.cpp), se coube na hora ou foi abandonado
29. The section `## Rodadas` de `~/projects/fala-research/benchmarks/README.md` SHALL ganhar uma tabela por rodada com data, máquina, engine, modelo, versão, threads, device, corte, WER e RTF, uma linha por (engine, modelo, threads, device, corte)
30. WHERE a evidência for medida só no Linux THEN o relatório SHALL marcar a seção de Windows (Whisper turbo CUDA, Voxtral, Parakeet no Core 7) com `TODO(windows)` e as invocações prontas para rodar lá

**Independent test:** `grep -c '^## ' docs/spikes/01-asr-pt-br.md` ≥ 3 e `grep -ci recomenda` = 0; a tabela do `fala-research` tem ≥ 1 linha por engine medida.

## Out of scope

Product capabilities only. Process and harness rules live in AGENTS.md or as Observable `n/a`.

| Excluded | Why |
| --- | --- |
| Download ou catálogo de modelos na CLI | o modelo é apontado por caminho; o Parakeet v3 já está em `~/.local/share/com.pais.handy/models/` e o whisper turbo vem à mão da URL já catalogada no desktop (nenhuma URL nova para o CDN do upstream) |
| Resample ou conversão de áudio na CLI | `ffmpeg -ss … -t … -ar 16000 -ac 1` faz os cortes; aceitar só o formato final mantém o código pequeno e o número honesto |
| Threads configuráveis para o Parakeet ONNX | `ParakeetModel::load` não expõe `intra_threads`; a varredura 4/8/12 do relatório 03 §8 vale só para a engine gguf; registrar como limitação |
| Latência ponta a ponta de ditado (VAD fecha → texto no campo) | relatório 03 §8 item 3; é da fase 1 com `FALA_TRACE=1` sobre o app real |
| Build do NeMo-Speech.cpp e script do faster-whisper | scripts descartáveis em `~/projects/fala-research/benchmarks/scripts/`, fora do PR; entregam hipóteses no formato da door 2; o NeMo-Speech.cpp só se não houver GGUF do Nemotron 3.5 para o `transcribe.cpp` em 1 hora |
| Medição no Windows (CUDA, Voxtral, Core 7 240H) | esta máquina é Linux CPU-first; a CLI e o relatório ficam prontos para rodar lá (`TODO(windows)`) |
| Trade study Parakeet vs Nemotron | documento próprio em `docs/spikes/`, escrito só quando as duas medições existirem (tarefa 4 do handoff) |
| Otimizar WER (dicionário, prompt inicial, fine-tuning) | o pitch proíbe antes do baseline: o spike mede, não melhora |
| Diarização, timestamps, saída JSON | nada os consome na fase 0 |
| Substituir ou confirmar a ADR-0003 no texto | ADR aceita não se edita; confirmação vai no relatório, substituição é ADR nova escrita pelo Augusto |

## Assumptions

Defaults that are not already a numbered criterion. Drop a row once it is.

| Assumption | Chosen default | Rationale | Confirmed? |
| --- | --- | --- | --- |
| "Parakeet via sherpa-onnx" no pitch e na ADR-0003 | medir o Parakeet ONNX int8 pelo `transcribe-rs` (ONNX Runtime), o mesmo runtime que o `apps/desktop` já embarca; o relatório diz isso explicitamente | o que interessa é o número do runtime que vai para o produto; sherpa-onnx exigiria build C++ à parte e mediria outra coisa | n |
| Nemotron 3.5 | primeiro caminho: GGUF para o `transcribe.cpp` (a engine gguf já o cobre, `EngineType::TranscribeCpp` do desktop lista Nemotron); segundo: NeMo-Speech.cpp por script externo; 1 hora no total | evita compilar um segundo runtime se o primeiro já lê o modelo | n |
| Idioma na engine gguf | `language = Some("pt")` sempre; engines que ignoram o campo não mudam | evita o custo e o risco da autodetecção do whisper em cortes curtos | n |
| Threads padrão da engine gguf | `std::thread::available_parallelism()` (12 aqui) | é o que uma pessoa obtém sem flag; a varredura 4/8/12 é feita pelo relatório | n |
| Origem do whisper large-v3-turbo GGML | `curl` manual da URL `ggml-large-v3-turbo.bin` já catalogada em `apps/desktop/src/managers/model.rs`, com o sha256 de lá conferido; caminho passado por `--model` | nenhuma URL nova para o CDN do upstream entra no código | n |
| Precisão das colunas | `audio_s` e `wall_s` com 2 casas, `rtf` com 3, `wer_%` com 2 | suficiente para comparar engines; mais casas é ruído | n |
| Cortes de ditado são fala conversacional | registrar a limitação no relatório (benchmarks/README já a nomeia) | decisão do Augusto de 2026-09-27: sem gravação nova | y |
| Logging da CLI | `env_logger` lendo `RUST_LOG`, default `info`, tudo no stderr | é o que o `transcribe-rs` já usa nos exemplos; `log` é a regra do AGENTS.md | n |

| # | Kind | Question | Until answered |
| --- | --- | --- | --- |
| 1 | blocks go-live | Quais trechos das 4 entrevistas viram cortes (início, duração, quem fala) e as referências corrigidas do Scribe v2 por corte: só o Augusto ouve o áudio e roda o Scribe | a CLI é construída e verificada com um corte sintético; nenhum número da tabela de rodadas existe até os cortes e as referências estarem em `benchmarks/audio/cuts/` e `benchmarks/reference/` |

**Open questions:** o item 1 acima é o único; nada bloqueia o código.

## Observable

Worksheet, not the review. `n/a` needs its reason. One row may group the same decision
across several routes.

| Surface | Decision | Landing |
| --- | --- | --- |
| command `fala-cli bench` | output format | AC 23 |
| command `fala-cli bench` | verbosity and what never prints | AC 24, AC 25 |
| command `fala-cli bench` | every flag and its default | AC 6, AC 9, AC 13, AC 17, AC 18, AC 19; `--tag` default `-` (AC 23) |
| command `fala-cli bench` | exit codes | AC 5, 8, 14, 15, 16, 21, 22 (2); AC 11, 12 (1); AC 4 (0) |
| command `fala-cli bench` | what it prints when it fails halfway | AC 12 |
| command `fala-cli bench` | progress on a long run | AC 25 |
| document `docs/spikes/01-asr-pt-br.md` | structure | AC 26 |
| document `docs/spikes/01-asr-pt-br.md` | depth (reproducible commands) | AC 27 |
| document `docs/spikes/01-asr-pt-br.md` | what the reader does next | AC 28 - compara com a ADR-0003 sem recomendar; a decisão fica com o Augusto |
| document `docs/spikes/01-asr-pt-br.md` | tone | existing - formato do relatório 11 §3b: objetivo, como reproduzir, evidência medida, sem recomendação |
| collection `benchmarks/README.md` § Rodadas | grouping, naming, ordering, duplicates | AC 29 - uma tabela por rodada, uma linha por (engine, modelo, threads, device, corte); rodada repetida é linha nova com data, nunca sobrescrita |
| collection `benchmarks/audio/cuts/` + `reference/` | naming and the exception | existing - `<stem>.wav` ↔ `<stem>.txt` já definido em `benchmarks/README.md`; a exceção (corte sem referência) é AC 15 |
| screen | n/a - a fase 0 não tem UI (pitch: "Nenhuma UI") |
| API or webhook | n/a - nada é exposto pela rede; o áudio nunca sai da máquina pela CLI |

## Sources

- `~/projects/fala-research/pitches/fase-0-spikes.md` - o spike 1, os rabbit holes de 1 hora e o "sem recomendação"
- `docs/decisions/0003-asr-do-ditado-local-parakeet-via-sherpa-onnx.md` - a aposta que o benchmark confirma ou substitui e a seção Confirmação (`fala-cli bench` reporta WER e RTF)
- `~/projects/fala-research/benchmarks/README.md` - máquinas, áudios de referência, layout `audio/cuts/` + `reference/`, seção Rodadas
- `~/projects/fala-research/research/03-modelos-asr-e-llm.md` §8 - o método (jiwer, threads 4/8/12, chunks do Nemotron, Vulkan)
- `~/projects/fala-research/research/11-kit-de-documentos-proposto.md` §3b - o formato do relatório em `docs/spikes/`
