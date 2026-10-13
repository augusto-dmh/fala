# qa-fixes P2 checks: keep the dictation text, expire only the recordings

Profile: light
Plan: none. Uma frase: a retenção (`preserve_limit`, dias, "nunca") apaga só o WAV dos ditados antigos; a linha e o ditado ficam, e a UI diz isso.
Fonte: `fala-research/research/26-qa-fala.md` Q1 e as decisões do orquestrador no brief "correções do QA" (2026-10-13).

## Intent

Com o padrão `history_limit = 5` + `preserve_limit`, cada ditado novo apagava a linha inteira do sexto mais antigo (texto, ditado em `fala.sqlite`, `.md` e WAV), enquanto o rótulo falava só de gravações. Depois deste PR o texto de todo ditado fica, sem limite; a contagem e os períodos valem só para o áudio.

Decisões (do orquestrador, registradas no PR):

- D1: texto sem limite; `history_limit` passa a contar só os WAVs mantidos sob `preserve_limit`.
- D2: migração sem mexer na configuração: quem tem `history_limit = 5` + `preserve_limit` mantém os 5 últimos WAVs, e o texto deixa de expirar. O que já foi apagado não volta.
- D3: a linha sem áudio marca `audio_deleted` (migração 8 de `history.db`), some o player e o "re-transcrever"; ao expandir, aparece "O áudio deste ditado já foi apagado; o texto fica."
- D4: os itens salvos (estrela) continuam fora da retenção.
- D5: rótulos: "Gravações de áudio" (Manter todas / Manter as últimas N / Apagar após …) e "Gravações a manter", este só visível com "Manter as últimas N".

## Checks

**C1** - `cleanup_by_count_with` com limite 1 sobre 3 entradas apaga os 2 WAVs mais antigos, mantém as 3 linhas e os 3 ditados de `fala.sqlite`, e marca `audio_deleted` só nos dois. Prova: `retention_deletes_only_the_recordings` em `apps/desktop/src/managers/history_dictations.rs`.

**C2** - `cleanup_by_time_with` apaga o WAV restante pela data e mantém a linha e o ditado. Prova: o mesmo teste.

**C3** - Um item salvo mantém o WAV com limite 0 e com corte por data. Prova: `retention_keeps_saved_recordings`.

**C4** - A migração 8 adiciona `audio_deleted` (NOT NULL, padrão 0) e a linha antiga sobrevive com `false`. Prova: `migration_eight_adds_audio_deleted` em `apps/desktop/src/managers/history.rs`.

**C5** - `retry_history_entry_transcription` recusa uma entrada com `audio_deleted`. Prova: leitura de `apps/desktop/src/commands/history.rs`.

**C6** - A linha com `audio_deleted` não mostra o player nem o botão de re-transcrever e explica por quê. Prova: leitura de `HistorySettings.tsx`; `bunx tsc --noEmit`.

**C7** - Os rótulos en/pt dizem que só o áudio expira, e "Gravações a manter" só aparece com `preserve_limit`. Prova: `bun run check:translations`; leitura de `AdvancedSettings.tsx`.

**C8** - Dois ditados no mesmo segundo dividem o WAV; quando ele sai, as duas linhas ficam `audio_deleted`. Prova: `retention_marks_every_row_sharing_the_wav`.

**C9** - Cada linha que perdeu o WAV é anunciada como `Updated` (a lista aberta perde o player sem recarregar); o `Added` sai antes da limpeza. Prova: leitura de `cleanup_old_entries` e `save_entry`; os testes conferem os ids devolvidos.
