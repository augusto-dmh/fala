// Prova dos checks de UI da lixeira (`meeting-session-edit`, T*). Rode com
// `bun src/components/meeting/trash.test.tsx`: imprime `<check> ok` e sai com erro na primeira
// falha. O `t` lê as chaves em pt, então toda string visível passa por elas.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type {
  MeetingDetail,
  MeetingSummary,
  TrashedMeetingSummary,
} from "@/bindings";
import { DeleteConfirm, trashNeedsConfirm } from "./DeleteMeeting";
import type { Translate } from "./meetingModel";
import { daysInTrash, sinceLabel, TrashView } from "./TrashView";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const page = read("./MeetingsPage.tsx");

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

const summary = (over: Partial<MeetingSummary> = {}): MeetingSummary => ({
  id: "01J9Z3K8M4Q7R2S5T6V7W8X9YZ",
  title: "Contrato",
  mode: "meeting",
  created_at: "2026-10-02T13:50:00-03:00",
  started_at: "2026-10-02T14:02:30-03:00",
  ended_at: "2026-10-02T14:40:00-03:00",
  recorded_ms: 2_250_000,
  audio_retained: false,
  transcribed: true,
  has_notes: false,
  imported_from: null,
  ...over,
});

const detail = (audio: boolean, retained: boolean): MeetingDetail => ({
  summary: summary({ audio_retained: retained }),
  annotations: "",
  lines: [],
  notes_md: null,
  notes_template: null,
  notes_edited: false,
  notes_running: false,
  audio_available: audio,
  sleeps: [],
});

// T1: moving asks only when there is audio, and says the audio goes now and the rest stays.
{
  assert.equal(trashNeedsConfirm(detail(true, true)), true);
  assert.equal(trashNeedsConfirm(detail(true, false)), true, "WAV still");
  assert.equal(trashNeedsConfirm(detail(false, false)), false);
  const html = renderToStaticMarkup(
    <DeleteConfirm onConfirm={noop} onCancel={noop} kind="trash" t={tr} />,
  );
  assert.ok(html.includes(">Mover para a lixeira?<"), html);
  assert.ok(html.includes(">Mover para a lixeira</button>"), html);
  assert.match(
    tr("meeting.trash.confirm.body"),
    /áudio é apagado agora.*30 dias na lixeira/,
  );
  ok("T1 confirmation only with audio");
}

// T2: every move shows the toast with [Undo] for 5 s, and [Undo] restores.
{
  assert.match(page, /const UNDO_MS = 5_000;/);
  assert.match(page, /commands\.trashMeeting\(id\)/);
  assert.match(
    page,
    /toast\(t\("meeting\.trash\.moved"\), \{\s*duration: UNDO_MS,\s*action: \{\s*label: t\("meeting\.trash\.undo"\),\s*onClick: \(\) => void restore\(id, wasOpen \? "open" : "none"\)/,
  );
  assert.equal(tr("meeting.trash.moved"), "Reunião na lixeira");
  ok("T2 toast with undo");
}

// T3: the trash lists each session with Restore and Delete forever, and the 30-day rule.
{
  const now = new Date("2026-10-13T12:00:00-03:00");
  const trashed: TrashedMeetingSummary[] = [
    { summary: summary(), trashed_at: "2026-10-10T11:00:00-03:00" },
    {
      summary: summary({ id: "b", title: "" }),
      trashed_at: "2026-10-13T09:00:00-03:00",
    },
  ];
  assert.equal(daysInTrash(trashed[0].trashed_at, now), 3);
  assert.equal(daysInTrash("2026-10-14T00:00:00-03:00", now), 0);
  // Calendar days, like the date beside it: last night at 23:00 seen at 09:00 is "1 dia".
  const lastNight = new Date(2026, 9, 12, 23, 0).toISOString();
  assert.equal(daysInTrash(lastNight, new Date(2026, 9, 13, 9, 0)), 1);
  assert.equal(sinceLabel(1, tr), "apagada há 1 dia");
  const html = renderToStaticMarkup(
    <TrashView
      trashed={trashed}
      onBack={noop}
      onRestore={noop}
      onDelete={noop}
      locale="pt-BR"
      t={tr}
      now={now}
    />,
  );
  assert.ok(html.includes(">Contrato<"), html);
  assert.ok(html.includes("apagada há 3 dias"), html);
  assert.ok(html.includes("apagada hoje"), html);
  // An untitled session shows "Reunião de …".
  assert.ok(html.includes(">Reunião de "), html);
  assert.equal(html.match(/Restaurar/g)?.length, 2);
  assert.equal(html.match(/Apagar para sempre/g)?.length, 2);
  assert.ok(html.includes(tr("meeting.trash.footer")));
  assert.match(tr("meeting.trash.footer"), /30 dias/);
  ok("T3 trash view");
}

// T4: empty, it says so; the page's link shows the count and hides at zero.
{
  const html = renderToStaticMarkup(
    <TrashView
      trashed={[]}
      onBack={noop}
      onRestore={noop}
      onDelete={noop}
      locale="pt-BR"
      t={tr}
    />,
  );
  assert.ok(html.includes("Nada na lixeira"), html);
  assert.ok(!html.includes("Restaurar"), html);
  assert.equal(tr("meeting.trash.link", { count: 2 }), "Lixeira (2)");
  assert.match(page, /\{trashed\.length > 0 && \(/);
  // Restoring from the trash offers to open the session.
  assert.match(page, /onRestore=\{\(id\) => void restore\(id, "offer"\)\}/);
  // [Undo] on the open session reopens it; the trash is never left behind a session.
  assert.match(page, /if \(then === "open"\) void open\(id\);/);
  assert.match(page, /setShowTrash\(false\);\s*setSelected\(result\.data\);/);
  assert.match(page, /toast\.success\(t\("meeting\.trash\.restored"\)/);
  ok("T4 empty trash and link");
}
