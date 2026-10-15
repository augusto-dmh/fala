// Prova do painel ao vivo da reunião (ADR-0019). Rode com `bun src/components/meeting/live.test.tsx`:
// imprime `<check> ok` e sai com erro na primeira falha. Sem instância do i18next, `t` devolve a chave.
import assert from "node:assert/strict";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingPreview, PreviewLine } from "@/bindings";
import { channelLabel, LiveLines, liveTicker } from "./LiveTranscript";
import { atEnd, emptyLive, mergeLive } from "./liveModel";
import type { Translate } from "./meetingModel";

const keys: Translate = (key) => key;
const ok = (id: string) => console.log(`${id} ok`);

const line = (
  channel: PreviewLine["channel"],
  t0_ms: number,
  text: string,
): PreviewLine => ({ channel, t0_ms, t1_ms: t0_ms + 1_000, text });
const event = (
  lines: PreviewLine[],
  over: Partial<MeetingPreview> = {},
): MeetingPreview => ({
  id: "s",
  lines,
  mic_listening: false,
  system_listening: false,
  replaced: false,
  ...over,
});

// Linhas dos dois canais entram na ordem do tempo, sem repetir a que o snapshot já trouxe.
{
  let live = mergeLive(emptyLive, event([line("system", 5_000, "b")]));
  live = mergeLive(live, event([line("mic", 2_000, "a")]));
  live = mergeLive(
    live,
    event([line("system", 5_000, "b"), line("mic", 9_000, "c")], {
      system_listening: true,
    }),
  );
  assert.deepEqual(
    live.lines.map((l) => l.text),
    ["a", "b", "c"],
  );
  assert.equal(live.system, true);
  assert.equal(live.mic, false);
  // The transcript of record replaced it: nothing of the preview stays.
  assert.deepEqual(mergeLive(live, event([], { replaced: true })), emptyLive);
  ok("merge");
}

// Rolagem: segue o fim só quando a pessoa está nele.
{
  assert.equal(atEnd(968, 500, 1_500), true, "at the end");
  assert.equal(atEnd(400, 500, 1_500), false, "scrolled up to read");
  ok("stick");
}

// Você à direita, Eles à esquerda, tempo na troca de canal, "ouvindo…" onde ainda não há texto.
{
  const live = mergeLive(
    emptyLive,
    event(
      [
        line("system", 61_000, "Bom dia"),
        line("system", 63_000, "pode começar"),
        line("mic", 66_000, "Vamos lá"),
      ],
      { mic_listening: true },
    ),
  );
  const html = renderToStaticMarkup(
    <LiveLines live={live} mode="meeting" recording t={keys} />,
  );
  assert.match(html, /meeting\.live\.note/);
  assert.match(html, /meeting\.live\.them · 01:01/);
  assert.doesNotMatch(html, /01:03/, "same speaker: no new header");
  assert.match(html, /items-end[^>]*>.*meeting\.live\.you · 01:06/);
  assert.match(html, /meeting\.live\.you · meeting\.live\.listening/);
  ok("lines");
}

// Gravando sem fala: espera; parado sem prévia: nada; parado com prévia: rotulada como prévia.
{
  const waiting = renderToStaticMarkup(
    <LiveLines live={emptyLive} mode="meeting" recording t={keys} />,
  );
  assert.match(waiting, /meeting\.live\.waiting/);
  assert.equal(
    renderToStaticMarkup(
      <LiveLines live={emptyLive} mode="meeting" recording={false} t={keys} />,
    ),
    "",
  );
  const kept = mergeLive(emptyLive, event([line("mic", 0, "oi")]));
  const stopped = renderToStaticMarkup(
    <LiveLines live={kept} mode="meeting" recording={false} t={keys} />,
  );
  assert.match(stopped, /meeting\.live\.stopped/);
  ok("states");
}

// Presencial: o mic é a sala. A barra mostra a última fala, ou que alguém fala.
{
  assert.equal(channelLabel("mic", "in_person", keys), "meeting.live.room");
  assert.equal(channelLabel("mic", "meeting", keys), "meeting.live.you");
  assert.equal(liveTicker(emptyLive, "meeting", keys), null);
  assert.equal(
    liveTicker({ ...emptyLive, system: true }, "meeting", keys),
    "meeting.live.listening",
  );
  const live = mergeLive(emptyLive, event([line("system", 0, "tudo bem?")]));
  assert.equal(
    liveTicker(live, "meeting", keys),
    "meeting.live.them: tudo bem?",
  );
  ok("labels");
}
