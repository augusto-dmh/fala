// Prova dos checks de UI de apagar um trecho (`meeting-session-edit`, E*). Rode com
// `bun src/components/meeting/erase.test.tsx`: imprime `<check> ok` e sai com erro na primeira
// falha. O `t` lê as chaves em pt, então toda string visível passa por elas.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingLine } from "@/bindings";
import {
  ERASE_MAX_LINES,
  EraseBar,
  EraseConfirm,
  stretchOf,
} from "./EraseStretch";
import type { Translate } from "./meetingModel";
import { TranscriptView } from "./TranscriptView";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const view = read("./MeetingView.tsx");

function ok(id: string) {
  console.log(`${id} ok`);
}

const lookup = (key: string) =>
  key
    .split(".")
    .reduce<unknown>(
      (node, part) => (node as Record<string, unknown> | undefined)?.[part],
      pt,
    );

// With `count`, the plural form i18next picks in pt, or the key itself without plural forms.
const tr: Translate = (key, options = {}) => {
  const count = options.count as number | undefined;
  const plural = `${key}_${count === 1 ? "one" : "other"}`;
  const value =
    count !== undefined && lookup(plural) !== undefined
      ? lookup(plural)
      : lookup(key);
  assert.equal(typeof value, "string", `chave sem texto em pt: ${key}`);
  return (value as string).replace(/\{\{(\w+)\}\}/g, (_, name) =>
    String(options[name]),
  );
};

const noop = () => {};

const line = (
  seq: number,
  t0_ms: number,
  over: Partial<MeetingLine> = {},
): MeetingLine => ({
  seq,
  channel: over.person ? "system" : "mic",
  person: null,
  t0_ms,
  t1_ms: t0_ms + 4_000,
  text: `fala ${seq}`,
  ...over,
});

// s1 eu 0 s, s2 Pessoa 1 10 s, s3 eu 20 s, s4 Pessoa 1 25 s (cruza o fim), s5 omitido, s6 eu 40 s.
const lines = [
  line(1, 0),
  line(2, 10_000, { person: 1 }),
  line(3, 20_000),
  line(4, 25_000, { person: 1, t1_ms: 31_000 }),
  line(5, 26_000, { omitted: true, t1_ms: 27_000 }),
  line(6, 40_000),
];

// E1: the stretch runs from the first picked line to the last, in either order, both channels.
{
  assert.equal(stretchOf(lines, []), null);
  assert.equal(stretchOf(lines, [2]), null);
  const stretch = stretchOf(lines, [4, 2]);
  // s3 sits between them in the other channel and goes too; the marker does not count.
  assert.deepEqual(stretch, {
    t0_ms: 10_000,
    t1_ms: 31_000,
    lines: 3,
    taken: new Set([2, 3, 4]),
  });
  // A line that crosses an edge leaves whole, and the stretch grows to its end, which may take
  // one more: s8 (22-30 s) crosses s3's end, then s4 (25-31 s) crosses s8's.
  const crossing = [...lines, line(8, 22_000, { person: 2, t1_ms: 30_000 })];
  assert.deepEqual(stretchOf(crossing, [2, 3]), {
    t0_ms: 10_000,
    t1_ms: 31_000,
    lines: 4,
    taken: new Set([2, 3, 4, 8]),
  });
  // A line that only touches the edge stays.
  assert.deepEqual(stretchOf(lines, [1, 2])?.lines, 2);
  ok("E1 stretch in both channels");
}

// E2: the bar says what to click, how many lines go, and refuses over 500.
{
  const first = renderToStaticMarkup(
    <EraseBar
      picked={0}
      stretch={null}
      onErase={noop}
      onCancel={noop}
      t={tr}
    />,
  );
  assert.ok(first.includes("Clique na primeira fala do trecho."), first);
  assert.match(first, /disabled=""[^>]*>Apagar trecho</);
  const second = renderToStaticMarkup(
    <EraseBar
      picked={1}
      stretch={null}
      onErase={noop}
      onCancel={noop}
      t={tr}
    />,
  );
  assert.ok(second.includes("Agora clique na última."), second);
  const ready = renderToStaticMarkup(
    <EraseBar
      picked={2}
      stretch={{
        t0_ms: 724_000,
        t1_ms: 820_000,
        lines: 9,
        taken: new Set<number>(),
      }}
      onErase={noop}
      onCancel={noop}
      t={tr}
    />,
  );
  assert.ok(
    ready.includes("9 falas de 12:04 a 13:40, nos dois canais."),
    ready,
  );
  assert.doesNotMatch(ready, /disabled=""[^>]*>Apagar trecho</);
  const many = renderToStaticMarkup(
    <EraseBar
      picked={2}
      stretch={{
        t0_ms: 0,
        t1_ms: 1,
        lines: ERASE_MAX_LINES + 1,
        taken: new Set<number>(),
      }}
      onErase={noop}
      onCancel={noop}
      t={tr}
    />,
  );
  assert.ok(many.includes("Mais de 500 falas"), many);
  assert.match(many, /disabled=""[^>]*>Apagar trecho</);
  ok("E2 picking bar");
}

// E3: the confirmation gives the times, both channels and the count, and says it is for good;
// without audio it says only the text goes.
{
  const stretch = {
    t0_ms: 724_000,
    t1_ms: 820_000,
    lines: 9,
    taken: new Set<number>(),
  };
  const html = renderToStaticMarkup(
    <EraseConfirm
      stretch={stretch}
      audio
      onConfirm={noop}
      onCancel={noop}
      t={tr}
    />,
  );
  assert.match(html, /role="alertdialog"/);
  assert.ok(html.includes(">Apagar este trecho?<"), html);
  assert.ok(
    html.includes(
      "O áudio de 12:04 a 13:40 nos dois canais e as 9 falas desse intervalo saem deste computador. Não dá para desfazer.",
    ),
    html,
  );
  const silent = renderToStaticMarkup(
    <EraseConfirm
      stretch={{ ...stretch, lines: 1 }}
      audio={false}
      onConfirm={noop}
      onCancel={noop}
      t={tr}
    />,
  );
  assert.ok(silent.includes("o áudio já não está aqui"), silent);
  assert.ok(!silent.includes("nos dois canais"), silent);
  ok("E3 confirmation");
}

// E4: the transcript shows the erased marker as a line, not a bubble, and picks by click.
{
  const transcript = (
    picking?: Parameters<typeof TranscriptView>[0]["picking"],
  ) =>
    renderToStaticMarkup(
      <TranscriptView
        lines={[
          ...lines,
          line(7, 50_000, { erased: true, text: "[trecho apagado]" }),
        ]}
        audioAvailable
        playing={null}
        highlighted={null}
        onPlay={noop}
        picking={picking}
        t={tr}
      />,
    );
  const plain = transcript();
  assert.match(plain, /data-speaker="erased"[^>]*>Trecho apagado/);
  assert.ok(!plain.includes("aria-pressed"), plain);
  const picking = transcript({
    picked: [2],
    taken: new Set([2, 3]),
    onPick: noop,
  });
  assert.equal(picking.match(/aria-pressed="true"/g)?.length, 1);
  assert.equal(picking.match(/ring-danger/g)?.length, 2);
  ok("E4 transcript marker and picking");
}

// E5: the session wires the erase: the command, the reload, and "the transcript changed" with
// [Regenerar notas] through the edited-notes confirmation.
{
  assert.match(view, /commands\.eraseMeetingRange\(/);
  assert.match(
    view,
    /t\("meeting\.erase\.changed", \{ count: erased\.notes_lines \}\)/,
  );
  assert.match(view, /erased !== null && detail\.notes_md !== null/);
  assert.match(view, /setErased\(null\);/);
  // While it runs, a status line; picking shows every line, with the search off; the WAV a
  // retention has not converted counts as audio.
  assert.match(view, /\{erasingNow \? \(\s*<p role="status"/);
  assert.match(view, /lines=\{chosen === null \? found : detail\.lines\}/);
  assert.match(view, /disabled=\{chosen !== null\}/);
  assert.match(
    view,
    /audio=\{detail\.audio_available \|\| !!detail\.audio_pending\}/,
  );
  assert.equal(
    tr("meeting.erase.changed", { count: 2 }),
    "A transcrição mudou. 2 linhas das notas citavam o trecho apagado.",
  );
  ok("E5 session wiring");
}
