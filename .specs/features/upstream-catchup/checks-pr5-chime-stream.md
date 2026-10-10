# upstream-catchup PR 5 (chimes sem abrir um stream WASAPI por som) checks

Profile: light
Plan: none - o diff cabe numa frase (AGENTS.md, "Fluxo por feature")

## Intent

A issue do upstream #1712 (defeito A) mostra que abrir um stream de saída WASAPI novo a cada chime,
duas vezes por ditado e junto com a parada do stream do mic, emperra o motor de áudio do Windows:
o stream congela sem erro e `sleep_until_end` nunca volta. No Fala, `audio_feedback.rs` ainda abre
`OutputStreamBuilder` em todo som; o defeito B já não existe (o laço do gravador usa
`recv_timeout`). Com este PR, os chimes vão para uma thread de reprodução que mantém o stream
aberto enquanto os chimes chegam e o reaproveita, reabrindo só quando o dispositivo muda, quando o
callback de erro do stream dispara, depois de uma falha ou depois de 30 s sem chime. A reprodução
tem prazo (a duração do som + 1 s, no máximo 10 s), e uma thread presa num chime por mais de 15 s,
ou morta, é trocada por uma nova no chime seguinte. Quem espera um chime (o de início antes do
mute, o som de teste) espera até 2 s para ele começar e depois a duração do som.

Decisões (Confirmed? y — delegado):

- Fecha o stream depois de 30 s ocioso em vez de mantê-lo para sempre: um stream de saída aberto
  roda o mixer o tempo todo e pesaria na meta de idle (< 1 % de CPU, ARCHITECTURE). Os dois chimes
  de um ditado caem no mesmo stream.
- "Default" vira a chave `default:<nome do padrão do SO>`, relida a cada chime (uma consulta de
  nome, não um stream), para trocar fone e caixa continuar funcionando. Um dispositivo escolhido
  pelo nome que some é detectado pelo callback de erro do stream.
- Rodada 1 do Verifier (FAIL): com uma thread só, um stream congelado silenciava todo chime seguinte
  e um worker morto perdia chimes sem log. A correção não move o stream de thread (ele guarda um
  `cpal::Stream`): a thread presa fica para trás com o stream dela, e a próxima chama nasce limpa.
  Isso troca um vazamento raro de uma thread por nunca ficar sem chime.
- A espera bloqueante em duas fases substitui o teto fixo de 3 s, que mutava no meio um som de
  início custom mais longo.

## Checks

### S1 - stream de saída reaproveitado · 1 file · 10 KB · ~3k

**C1** - `CachedOutput::get_or_open` abre uma vez para duas chamadas com a mesma chave, reabre
quando a chave muda (`default:Speakers` → `default:Headphones`) e reabre depois de `close()`;
3 aberturas em 4 chamadas
Proof: `cargo test -p fala --lib -- audio_feedback::tests::chime_stream_is_reused_while_the_device_stays_the_same`

**C2** - Uma abertura que falha devolve o erro, não deixa stream em cache e a chamada seguinte
abre de novo
Proof: `cargo test -p fala --lib -- audio_feedback::tests::chime_stream_that_fails_to_open_is_retried_next_time`

**C3** - A espera de um chime bloqueante tem teto: sem início, `wait_for_chime` devolve `false` em
menos de 1 s com espera de início de 50 ms; com o remetente solto (chime acabou ou falhou), `true`
na hora; um chime de 100 ms que nunca termina é abandonado entre 1 s e 2 s (duração + 1 s)
Proof: `cargo test -p fala --lib -- audio_feedback::tests::blocking_chime_wait_is_bounded`

**C4** - Só a thread de reprodução abre stream de saída: `open_output_stream` é chamada só por
`play_chime`; o worker fecha o stream depois de `CHIME_STREAM_IDLE = 30 s` sem pedido e depois de
um `play_chime` com erro; e `play_chime` não usa `sleep_until_end`
Proof: `test "$(grep -c 'open_output_stream(' apps/desktop/src/audio_feedback.rs)" -eq 2 && grep -q 'const CHIME_STREAM_IDLE: Duration = Duration::from_secs(30);' apps/desktop/src/audio_feedback.rs && awk '/^fn chime_worker/{f=1} f&&/^}/{exit} f&&/output.close\(\)/{n++} END{exit !(n==2)}' apps/desktop/src/audio_feedback.rs && ! grep -q '\.sleep_until_end()' apps/desktop/src/audio_feedback.rs`

**C6** - Um som de início longo toca inteiro antes do mute: com duração de 5 s e fim em 300 ms,
`wait_for_chime` devolve `true`; `play_limit` vale 6 s para 5 s, `CHIME_PLAY_MAX` (10 s) para 60 s
e para duração desconhecida
Proof: `cargo test -p fala --lib -- audio_feedback::tests::a_long_start_sound_is_waited_out_before_mute`

**C7** - Uma reprodução congelada é abandonada no prazo: `wait_until` com condição sempre falsa e
prazo de 80 ms devolve `false` entre 80 ms e 1 s; com a condição verdadeira na 3ª consulta, `true`
Proof: `cargo test -p fala --lib -- audio_feedback::tests::frozen_playback_is_abandoned_at_its_limit`

**C8** - Uma thread de reprodução morta (receptor solto) é trocada: o chime chega à thread nova e
`spawn` roda uma vez
Proof: `cargo test -p fala --lib -- audio_feedback::tests::a_dead_playback_thread_is_replaced`

**C9** - Uma thread ocupada há 5 s (limite 15 s) recebe o chime sem `spawn`; ocupada há 20 s, o
chime vai para uma thread nova e a presa não recebe nada
Proof: `cargo test -p fala --lib -- audio_feedback::tests::a_stuck_playback_thread_is_replaced_and_a_busy_one_kept`

**C10** - Sem conseguir criar a thread, `send` devolve `false`, e os dois chamadores logam
`was not played` nesse caso; o callback de erro do stream marca `failed` e `play_chime` fecha um
stream marcado antes de reusar
Proof: `cargo test -p fala --lib -- audio_feedback::tests::a_thread_that_cannot_start_reports_the_chime_lost`
Proof: `test "$(grep -c 'was not played: no playback thread' apps/desktop/src/audio_feedback.rs)" -eq 2 && grep -q 'flag.store(true, Ordering::SeqCst);' apps/desktop/src/audio_feedback.rs && grep -q 'chime.failed.load(Ordering::SeqCst)' apps/desktop/src/audio_feedback.rs`

**C5** - TODO(windows): no build do Windows com som de feedback ligado e música tocando, 30 ditados
seguidos sem chime faltando e sem pill presa; trocar o padrão de saída entre fone e caixa no meio
e o chime seguinte sai no novo dispositivo; com um fone USB escolhido pelo nome, desconectá-lo no
meio e o chime seguinte sai no padrão
Proof: `TODO(windows)` manual - contar chimes ouvidos (esperado 60/60) e ditados concluídos (30/30)

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| motivos para reabrir o stream (5) | sem stream C1 · dispositivo mudou C1 · erro do stream C10 · falha C2, C4 · ocioso 30 s C4 | - |
| desfechos da espera bloqueante (4) | não começou C3 · acabou ou falhou C3 · durou demais C3 · som longo que acaba C6 | - |
| estados da thread de reprodução (4) | ociosa ou ocupada no prazo C9 · presa C9 · morta C8 · não criada C10 | - |
| chamadores que tocam chime (3) | assíncrono (`play_feedback_sound`) C4, C10 · bloqueante (início, teste) C3, C6, C10 · aviso de limite C4 | - |

- C4 e parte de C10 são estruturais (reproduzir áudio de verdade não roda no CI); C5 é a prova do
  sintoma.

## Swept

- validation: n/a - nenhuma entrada de usuário nova
- failure modes: C2, C3, C7, C8, C9
- idempotency: C1 (mesma chave reaproveita)
- authorization: n/a - nada de acesso novo
- concurrency: C4 (uma thread só abre e toca)
- data lifecycle: C4 (o stream fecha ocioso)
- dependency failure: C2, C3, C7, C10 (dispositivo recusa, emperra ou some)
- state transitions: C1
- observability: C10 (chime perdido vira `warn!` com o caminho do som)

## Handoff

- S1 = ~5k (1 arquivo, 18 KB / 4), sob o orçamento de 150k - one builder
