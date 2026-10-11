# meeting-source-playback (F2 "tocar o trecho" + F9)

Profile: light · budget 150k · definição: `fala-research/plans/features/meeting-source-playback.md`

## Problem

A nota da reunião aparece crua num `<pre>`: os ponteiros `[[#^s12|01:05]]` e o marcador
`<!-- fala:ia -->` ficam visíveis, o texto da IA e o digitado têm a mesma cor e não há como ver de
onde uma linha veio. O áudio retido (`audio/<id>/mic.opus` e `sys.opus`, ADR-0014) não toca em
lugar nenhum: quem quer "ouvir de novo um número ou um nome" não tem como.

## Acceptance criteria

1. `fala_retention::decode_range(path, start, end)` devolve o PCM mono 48 kHz de `[start, end)`
   (amostras depois do pre-skip): a duração bate ± 1 quadro de 20 ms, um bipe cai na posição
   certa ± 20 ms, um trecho de silêncio volta zerado, o fim é recortado no fim do arquivo e um
   início além do fim devolve vazio.
2. `decode_range` pula páginas Ogg pelo granule sem decodificar e decodifica a partir de uma
   página ~1 s antes do início (pré-rolagem); a 2 h de um arquivo de 2 h ele decodifica no
   máximo ~3 s de pacotes.
3. `meeting_clip(id, t0_ms, t1_ms)` devolve um WAV mono 48 kHz 16 bits em memória, pela resposta
   binária do IPC: margem de 0,5 s de cada lado, teto de 60 s, `mic.opus` + `sys.opus` somados
   com saturação. Sem os dois arquivos, erro `no_audio`.
4. `MeetingDetail.audio_available` é `true` só quando os dois `.opus` existem.
5. `notesModel.ts` lê o `notes_md` em títulos e linhas: linha com `<!-- fala:ia -->` é gerada,
   as outras são humanas (âncora `^aN` removida); extrai `[[#^sN|…]]` e `[[#^aN|…]]`; ignora
   `^sN` cujo `seq` não está na transcrição.
6. A nota renderizada põe a linha gerada em `text-2` e a humana em `text`, e não mostra `[[#^`
   nem `<!-- fala:ia -->`; cada linha gerada tem um botão de lupa (visível no hover e no foco).
7. A lupa lista as fontes sem LLM: segmento com falante, `mm:ss`, texto e ▶; anotação com o texto
   digitado; linha sem fonte mostra "Esta linha não tem trecho de origem". Clicar no `mm:ss`
   rola a transcrição até o segmento e o destaca.
8. A transcrição vira conversa: "Eu" à direita (`me`), "Pessoa N" à esquerda (`them`), rótulo e
   `mm:ss` só na troca de falante, ▶ por balão; sem áudio, ▶ desabilitado com o motivo.
9. O ▶ toca por `Blob` URL num único `<audio>`, um trecho por vez; nada muda no `assetProtocol`,
   nenhum arquivo temporário, nenhuma chamada nova ao LLM (ADR-0016).

## Flow

Clique ▶ (balão ou lupa) → `invoke("meeting_clip", {id, t0Ms, t1Ms})` → desktop
`MeetingManager::clip` acha `audio/<id>/`, calcula a janela (margem, teto) → `decode_range`
em cada `.opus` → soma → WAV em `Vec<u8>` → `tauri::ipc::Response` → `ArrayBuffer` →
`Blob("audio/wav")` → `<audio>.play()`.

## Doors

1. **Resposta binária fora do tauri-specta.** O specta não tem `Type` para `tauri::ipc::Response`;
   o `meeting_clip` fica num `generate_handler!` próprio, despachado pelo nome antes do handler do
   specta, e o front o chama por `invoke` tipado à mão em `clipPlayer.ts`. Alternativa recusada:
   base64 em `String` (+33% e cópia no JS) ou `Vec<u8>` em JSON (~4× maior).
2. **Erro sem áudio reusa `no_audio`.** A definição fala em `audio_missing`; o `MeetingError` já
   tem `NoAudio` ("No retained audio") com texto traduzido, e um segundo nome para o mesmo
   estado só duplicaria a tradução.
3. **Busca linear por cabeçalho de página**, sem a bisseção do `ogg::seek_absgp` (que pode parar
   numa página com pacote continuado e cuja semântica de "absgp ≥ alvo" exige outra página de
   recuo para a pré-rolagem). Ler cabeçalhos de 2 h a 24 kbps é ~7 200 páginas, sem decodificar.
4. **Mixagem por soma saturada**, não média: mic e sistema raramente falam juntos, e a média
   reduziria cada voz à metade.
5. **Dois PRs empilhados** (regra de ~400 linhas): (A) crate + comando + `audio_available`;
   (B) a UI.

## Out

Chat, editar/regenerar nota, busca na transcrição, renomear falante, tocar um canal só,
velocidade, forma de onda, espelho `.md`.

## Manual (Windows, do Augusto)

Recorded as a checklist in the PR B body, outside the checks table: gravar 2–3 min falando
números em instantes anotados, transcrever, gerar notas, clicar ▶ em 10 segmentos e conferir o
trecho ± 1 s em 10 de 10; conferir que o WebView2 toca o WAV por `Blob` URL.
