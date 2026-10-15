# live-transcript — prévia local ao vivo durante a reunião (ADR-0019)

Profile: light

**Problem.** A reunião só ganha texto quando para e a Scribe responde; durante a call a tela fica
vazia e o toast diz "Nada foi transcrito ainda".

**Approach.** O modelo do ditado transcreve, durante a gravação, os trechos que o Silero fecha em
cada canal (mic = Você, sistema = Eles). A transcrição de registro (ADR-0005, e o fallback local do
painel `retry`) substitui a prévia; a prévia nunca vira registro.

- Fonte: `MeetingRecorder::set_tap`, os frames do WAV depois do hold (mic zerado no ditado,
  ADR-0015). `recorder.rs` só repassa o tap; `open_mic`/`reopen_mic` e o contador não mudam.
- Segmentação: `LiveSegmenter` reusa a `Session` do ditado com teto de 8 s.
- Prioridade: com ditado em curso a prévia não chama o modelo; `transcribe_preview` devolve
  `ModelBusy` em vez de esperar e nunca carrega/descarrega; `transcribe` espera o empréstimo.
- Persistência: só memória, descartada quando a transcrição de registro é gravada. Evento
  `MeetingPreview` e comando `meeting_preview`.

**Slices.** 1 ADR + `fala-audio` (tap, segmentador) · 2 prévia no desktop (fila, prioridade,
evento) · 3 painel ao vivo e textos da pílula/toast · 4 troca prévia → registro na reunião parada.
