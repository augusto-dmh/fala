---
status: proposed
date: 2026-10-02
---
# Ditado durante reunião gravada: uma captura do mic e zeros no canal do mic

## Contexto e problema
A ADR-0003 diz que o áudio de ditado nunca sai da máquina, e o guarda dela é de tipo: o áudio de ditado não implementa serialização para os clientes HTTP. A ADR-0005 grava o mic como canal da reunião e o manda à Scribe v2. Se o usuário dita durante uma reunião gravada (o Wispr Flow permite: "the mic switches between meeting recording and voice typing"), a voz do ditado entra no canal do mic e sobe pelo arquivo da reunião, um caminho que o guarda de tipos não vê. As duas ADRs valem; falta a regra de convivência, e ela muda a forma da captura em `crates/audio` e a máquina de estados da sessão, decisões difíceis de reverter na fase 2.

O spike 04 no Windows (PR #14, rodada 1) terminou a hora com −312 ms entre os canais, em quatro degraus de até ~−130 ms que coincidiram com as janelas em que o app abria e fechava ~1200 gravações curtas no `Voicemeeter Out B2`, que carrega o mesmo mic físico. O desvio foi no canal do sistema, e o próprio relatório diz que a rodada não separa essa atividade do resto; com o notebook quieto (rodada 3), a diferença ficou em ±10 ms. Abrir e fechar streams no mesmo mic é a suspeita, não a causa provada, e é o que um ditado durante a reunião faria.

Evidência: `docs/spikes/04-captura-dupla.md`, seção "Windows", no commit `8c09d1b` do PR #14 (não integrado; trocar pela referência em `main` quando integrar); `fala-research/research/16-wispr-flow-notas-e-lacunas.md` §1.1 e §4.2; `fala-research/pitches/fase-2-reuniao-videos.md` (D11, F1, rabbit hole "Ditado e reunião no mesmo microfone"); `fala-research/plans/roadmap-proposta-2026-10-02.md` §0 item 2, §3 (candidata 0015) e decisão 3. A pasta `fala-research` não é versionada neste repo.

## Opções consideradas
* Bloquear o ditado enquanto uma reunião grava.
* Permitir, com uma única captura do mic e fan-out em `crates/audio`; durante o ditado, o gravador da reunião escreve zeros no canal do mic (no arquivo retido e, portanto, no que sobe) e o transcript ganha um marcador "trecho de ditado omitido".
* Permitir, retendo o mic completo localmente e zerando só a cópia enviada (dois arquivos de mic).
* Permitir sem tratar.

## Decisão
Permitir o ditado durante a gravação, com uma captura do mic e zeros no canal do mic da reunião durante o ditado. Cumpre a ADR-0003 ao pé da letra, não abre e fecha streams no mesmo mic (a suspeita dos degraus do spike 04) e se testa com servidor falso. Bloquear atrapalha ditar no chat da própria call; reter o mic completo dobra arquivo e complexidade para guardar uma fala que não é da reunião; não tratar viola a ADR-0003.

As regras são estas:
* `crates/audio` abre **um único stream por dispositivo de entrada** e distribui os frames aos consumidores (ditado, gravador da reunião). Nunca um segundo stream no mesmo mic.
* O **intervalo do ditado** vai do início do pré-buffer até o fim do ditado (soltar, cancelar, toque curto descartado ou corte por tempo), medido no relógio de captura do mic da reunião. Quando ditado e reunião usam o mesmo dispositivo, ele cobre exatamente as amostras que entram no áudio do ditado. Quando usam dispositivos diferentes, o canal do mic da reunião é zerado no mesmo intervalo, porque a voz do ditado chega a ele pelo ar.
* O trecho zerado é a interseção do intervalo do ditado com os trechos gravados: um ditado que começa antes de "Gravar", cruza "Pausar"/"Retomar" ou termina depois de "Parar" (ou do teto de duração) zera só o que foi gravado. Intervalos que se sobrepõem viram um só.
* O gravador segura o canal do mic em memória por pelo menos o pré-buffer mais uma margem antes de escrever em disco: nenhuma amostra que possa virar áudio de ditado chega ao WAV de trabalho sem ter sido zerada, nem num crash.
* O canal do sistema não muda.
* A sessão ganha, no mesmo intervalo, um segmento marcador "trecho de ditado omitido" no canal do mic. O marcador vem do gravador, não do ASR, e passa intacto pela transcrição, pelo storage e pelo espelho.
* O texto do ditado segue o caminho normal do ditado (ADR-0004; ADR-0012, se aceita) e não entra no transcript nem nas notas da reunião.
* No modo presencial a regra é a mesma, e a pill mostra que o mic da reunião está omitido enquanto o ditado dura.
* Se o fan-out não estiver pronto quando a gravação de reunião existir, o ditado fica bloqueado durante a gravação, com uma mensagem; abrir um segundo stream não é alternativa.

### Consequências
* Bom: ditar no chat da call continua possível; a voz do ditado não chega à ElevenLabs nem fica no áudio retido.
* Ruim: o que outras pessoas disserem perto do mic durante o ditado se perde (no modo presencial, isso é a reunião); a reprodução de um trecho marcado toca silêncio; segurar o mic em memória atrasa a escrita do WAV por alguns centésimos de segundo; a captura do mic do pipeline de ditado e o gravador de reunião em `crates/audio` precisam nascer prontos para o fan-out, senão a sessão de reunião reabre o crate.
* Obrigatório: o gravador se inscreve nos eventos de início e fim de ditado; a máquina de estados da sessão trata ditado como evento dentro de `Recording`, não como estado novo; o detector de canal mudo ignora os intervalos marcados como ditado; no modo "só sistema" não há canal do mic e o ditado segue normal.
* Quando esta ADR for aceita, o `ARCHITECTURE.md` muda assim: o invariante "O áudio de ditado nunca é enviado pela rede" ganha "nem pelo arquivo da reunião: durante um ditado, o canal do mic da reunião gravada é zerado e marcado"; os invariantes ganham "um único stream por dispositivo de entrada"; a linha de `crates/audio` no Code Map passa a citar a captura única por dispositivo com fan-out.

## Confirmação
* Teste com fonte de áudio falsa, mesmo dispositivo: uma reunião simulada com um ditado no meio tem zeros exatos no canal do mic do PCM que vai ao encoder, no intervalo de amostras do áudio do ditado; o canal do sistema bate com a entrada. No Opus decodificado e no payload que o servidor falso da ElevenLabs recebe, o intervalo fica abaixo de −90 dBFS, com a tolerância de borda do encoder declarada no plano.
* Variantes do mesmo teste: dispositivos diferentes; ditado que cruza o início, uma pausa e o fim da gravação; dois ditados sobrepostos (um marcador); toque curto descartado.
* Teste de crash: o processo morre no meio de um ditado, e o WAV órfão recuperado já tem zeros no intervalo gravado.
* Teste: o segmento "trecho de ditado omitido" aparece no transcript, no SQLite e no `.md`, e o aviso de canal mudo não dispara durante um ditado longo.
* Teste no backend falso de `crates/audio`: ditar durante a gravação não abre um segundo stream no dispositivo (contador de aberturas igual a 1).
