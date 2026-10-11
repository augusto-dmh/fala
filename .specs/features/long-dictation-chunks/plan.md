# long-dictation-chunks — ditado longo transcrito e formatado por blocos enquanto se fala

Profile: light · Fonte: `fala-research/plans/features/long-dictation-chunks.md` (vale como pitch)

## Problem

O uso real é ditado longo em hands-free (p50 31 s, p90 148 s, máximo no teto de 20 min). No
desktop, o Parakeet roda no áudio inteiro depois de soltar: `DictationRecorder::stop` devolve a
gravação e `TranscribeAction::stop` chama `tm.transcribe(samples)` uma vez. Com RTF 0,045-0,064 em
CPU, 10 min de fala custam ~27-38 s de espera depois de soltar. E o texto de ~1.500 palavras vai ao
Gemini de uma vez: não volta em 2 s (`INSERT_DEADLINE`), cai nas regras, e passa dos 10 s da edição
tardia. É o mesmo defeito do Wispr ("ditado longo perde a formatação"), com espera maior.

Quando isto entrar, a espera ao soltar depende só do último bloco (≤ ~55 s de fala), não do
ditado inteiro, e cada bloco é formatado pelo mesmo `Postprocessor` enquanto se fala.

## Flow

S1 (PR 1, ASR por blocos):

1. `TranscribeAction::start` (exists) - monta o transcritor de blocos (`tm.transcribe`) quando o
   VAD é `Offline` e o modelo não descarrega a cada uso; `try_start_recording` (exists) arma a
   fila do `AudioRecordingManager` com ele (ou a desarma com `None`) só quando sai do `Idle`, para
   um start recusado não mexer na gravação em curso
2. `DictationCapture` (exists, `crates/audio`) - com `with_blocks()`, junta as utterances num bloco
   e o fecha na primeira utterance terminada por silêncio (hangover de 450 ms) depois de 25 s de
   fala; sem silêncio, corta no teto duro de 40 s. `take_blocks()` entrega os fechados
3. `Processor` (exists, `dictation_capture.rs`) - depois de cada drenagem, passa os blocos
   fechados ao callback de blocos do recorder (new), na thread do worker
4. `BlockQueue` (new, `long_dictation.rs`) - armada, transcreve cada bloco numa thread de fundo, na
   ordem; desarmada, ignora
5. `TranscribeAction::stop` (exists) - depois do `stop_recording`, `finish(&samples)`: sem bloco
   fechado devolve `None` e segue o caminho de hoje (`tm.transcribe(samples)`); com blocos, manda
   o resto (`samples[fechados..]`) como último bloco, espera a fila e junta os textos com um
   espaço, na ordem
6. Esc (`cancel_recording`, exists) - `BlockQueue::cancel` descarta a fila; o próximo `finish` ou
   `arm` espera o bloco que estava no motor terminar, para o motor estar livre
7. out: métricas com o número de blocos (`dictation_metrics.blocks`, `FALA_TRACE blocks=`); o
   histórico guarda o bruto e o final inteiros, como hoje. A CLI `dictate` usa o mesmo fatiador

S2 (PR 2, formatação por bloco): cada texto de bloco que sai da fila vai ao `Postprocessor`
(mesmo nível, dicionário e app); ao soltar, formata-se só o último. Bloco que não voltou do LLM até
o soltar entra com o texto das regras, o histórico marca "IA parcial" e a edição tardia vale por
bloco. Detalhe em "S2" abaixo, fechado antes do PR 2.

## Impact

| Front | What changes |
| --- | --- |
| domain | termo novo: bloco - utterances seguidas do ditado, 25-40 s de fala, fechado no silêncio do VAD. `BLOCK_MIN_SAMPLES`/`BLOCK_MAX_SAMPLES` em `fala-audio` |
| domain | termo novo: `BlockQueue` - fila de ASR de um ditado, armada por gravação, dona da thread de fundo |
| behaviour | ditado com ≥ 25 s de fala e uma pausa depois disso é transcrito em pedaços durante a gravação; o texto final é a junção. Ditado sem bloco fechado segue byte a byte o caminho de hoje |
| behaviour | com VAD desligado, modelo de streaming ou descarga imediata do modelo, nada muda (a fila fica desarmada) |
| behaviour | `fala-cli dictate` passa a mandar blocos ao ASR, não utterances: o trace `event=utterance` vira um por bloco |
| stored data | `dictation_metrics` ganha `blocks INTEGER NOT NULL DEFAULT 1`; bancos antigos recebem a coluna por `ALTER TABLE` no `open` |

## Relations

`dictation_metrics.blocks` é aditiva, fora do `user_version` como a tabela; linhas antigas valem 1
(um ASR por ditado, que era a verdade). Nenhuma FK nova.

## Surface

`None - nothing consumed outside`. `DictationCapture::with_blocks`/`take_blocks` são API nova de
`fala-audio`, consumida pelo desktop e pela CLI; as chamadas antigas não mudam.

## Landing

- A coluna `blocks` fica no banco do usuário (aditiva, com default; reverter o código não a quebra)
- Nothing else in this change is hard to reverse

## Decisions

- **Corte no silêncio:** o `DictationCapture` sabe por que cada utterance fechou (silêncio, teto de
  15 s, stop). Só o fechamento por silêncio fecha bloco depois de 25 s; o teto de 15 s cai no meio
  da fala e não conta. Por construção o bloco é um prefixo da gravação, então a junção não duplica
  nem perde amostra.
- **Teto de 40 s:** corte exato na amostra 640.000 do bloco, só com fala contínua sem pausa de
  450 ms. É a exceção do pitch ("teto duro").
- **Ditado curto:** a regra é "nenhum bloco fechou" (fala < 25 s, ou sem pausa depois dos 25 s
  antes de soltar com < 40 s): `finish` devolve `None` e o caminho de hoje roda inteiro, inclusive
  o `pad` de gravação curta.
- **Resto ao soltar:** o que não fechou bloco vai inteiro como último bloco, mesmo passando de
  40 s (até ~55 s: bloco aberto < 40 s mais a última utterance ≤ 15 s). Não se corta no soltar.
- **Erro num bloco:** o ditado inteiro falha como hoje (entrada vazia para "Recuperar texto" com o
  WAV inteiro); nada de texto parcial colado.
- **Fila desarmada** com VAD `Disabled` (não há silêncio para cortar), modelo de streaming (já tem
  caminho próprio) e `model_unload_timeout = Immediately` (o `transcribe` descarregaria o modelo
  entre blocos).
- **Motor único:** `transcribe` tira o motor do mutex; duas chamadas ao mesmo tempo falham. A fila
  tem uma thread, o último bloco vai pela mesma fila, e depois de um Esc o próximo uso espera o
  bloco em voo terminar.

## Criteria

### S1: ASR por blocos (P1, PR 1)

1. WHEN a fala acumulada no bloco aberto passa de 25 s e uma utterance fecha por silêncio, THE
   capture SHALL fechar o bloco ali; WHEN a utterance fecha pelo teto de 15 s, THE capture SHALL
   NOT fechar o bloco
2. WHEN o bloco aberto chega a 40 s sem fechar, THE capture SHALL cortá-lo em exatamente 40 s e
   seguir o resto no bloco seguinte
3. THE concatenação dos blocos fechados e do resto SHALL ser igual à gravação, amostra a amostra
4. WHEN um WAV sintético de 3 min com pausas passa pelo `Processor`, THE blocos SHALL ter entre 25
   e 40 s e terminar em silêncio; sem pausa, 40 s exatos
5. WHEN a fila está armada e recebe blocos, THE fila SHALL transcrevê-los antes do soltar, e o
   `finish` SHALL juntar os textos na ordem com um espaço, sem repetir nem perder texto de bloco,
   pulando texto vazio
6. WHEN nenhum bloco fechou, THE `finish` SHALL devolver `None` sem chamar o transcritor
7. WHEN o Esc cancela, THE fila SHALL descartar os blocos pendentes e o próximo `finish` SHALL
   devolver `None` só depois de o bloco em voo terminar
8. WHEN a fila está desarmada, THE blocos SHALL ser ignorados
9. WHEN um bloco falha no ASR, THE `finish` SHALL devolver o erro
10. THE métricas SHALL gravar o número de blocos (1 no caminho antigo) e o trace SHALL ter
    `blocks=`; um banco sem a coluna SHALL recebê-la no `open`
11. THE `fala-cli dictate` SHALL mandar blocos ao ASR, com um `event=utterance` por bloco

### S2: formatação por bloco (P1, PR 2)

Desenho (fechado depois do PR 1):

- Na fila, o texto de cada bloco fechado durante a gravação vai a uma thread que roda o mesmo
  `llm_auto::format_with_late_edit` do ditado curto (regras, dicionário, nível, Gemini com 2 s e
  edição tardia), com o app em foco naquele momento. O ASR do bloco seguinte não espera o LLM.
- Ao soltar, o resto é transcrito e formatado igual. A composição junta, na ordem, o texto do
  LLM de cada bloco que respondeu, ou o das regras; um bloco cuja resposta ainda não chegou fica
  com as regras e com a `LateEdit` pendente.
- O ditado é `Editor::Llm` quando algum bloco tem texto do LLM; "IA parcial" quando algum bloco
  ficou com as regras porque o LLM não respondeu a tempo (timeout). As respostas tardias que
  chegarem compõem o texto todo do LLM, aplicado à entrada pela edição tardia de hoje
  ("Aplicar edição da IA"); com todas, a marca parcial sai.
- A marca vai numa coluna nova do `history.db` herdado (`llm_partial`, migração aditiva), não no
  `fala.sqlite`, para não mexer no `user_version` nem no espelho Markdown.
- App da colagem no `llm_disabled_apps` ao soltar: todos os blocos colam o texto das regras.
- Modo `Legacy` (atalho `transcribe_with_post_process`): sem formatação por bloco; a junção
  bruta segue pelo cliente legado como hoje.
- Ditado sem bloco fechado: caminho de hoje, sem mudança.

12. WHEN um bloco fecha e o pós-processamento automático vale, THE fila SHALL formatá-lo durante a
    gravação com o mesmo formatador do ditado curto
13. WHEN todos os blocos respondem do LLM, THE texto colado SHALL ser a junção dos textos do LLM,
    `Editor::Llm`, sem "IA parcial"
14. WHEN um bloco não responde em 2 s (servidor falso lento), THE texto colado SHALL ter as regras
    naquele bloco e o LLM nos outros, e a entrada SHALL ficar com "IA parcial"
15. WHEN a resposta atrasada chega até o prazo tardio, THE entrada SHALL ganhar o texto todo do LLM
    como edição tardia e perder "IA parcial"
16. WHEN o app da colagem está em `llm_disabled_apps`, THE texto colado SHALL ser só regras
17. THE junção formatada SHALL ter um espaço entre blocos, sem bloco vazio
