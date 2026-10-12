// Prova dos checks de UI do `meeting-session-lifecycle` (L*). Rode com
// `bun src/components/meeting/lifecycle.test.tsx`: imprime `<check> ok` e sai com erro na
// primeira falha. O `t` lê as chaves em pt, então toda string visível passa por elas.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import {
  RetentionConfirm,
  retentionDays,
  retentionNeedsConfirm,
  retentionOptions,
  retentionValue,
} from "../settings/MeetingAudioSettings";
import type { MeetingOrphan } from "@/bindings";
import { DeleteConfirm } from "./DeleteMeeting";
import type { Translate } from "./meetingModel";
import { orphanMinutes, orphanTime, RecoverBanner } from "./RecoverBanner";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const page = read("./MeetingsPage.tsx");

function ok(id: string) {
  console.log(`${id} ok`);
}

const lookup = (tree: unknown, key: string) =>
  key
    .split(".")
    .reduce<unknown>(
      (node, part) => (node as Record<string, unknown> | undefined)?.[part],
      tree,
    );

const tr: Translate = (key, options = {}) => {
  const value = lookup(pt, key);
  assert.equal(typeof value, "string", `chave sem texto em pt: ${key}`);
  return (value as string).replace(/\{\{(\w+)\}\}/g, (_, name) =>
    String(options[name]),
  );
};

const noop = () => {};

// L1: the delete confirmation says what goes and that it is for good, with two buttons.
{
  const html = renderToStaticMarkup(
    <DeleteConfirm onConfirm={noop} onCancel={noop} t={tr} />,
  );
  assert.match(html, /role="alertdialog"/);
  assert.ok(html.includes(tr("meeting.delete.body")));
  assert.match(tr("meeting.delete.body"), /áudio.*transcrição.*desfazer/);
  assert.equal(html.match(/<button/g)?.length, 2);
  assert.ok(html.includes(tr("meeting.delete.confirm")));
  // The page deletes only through the confirmation, never on the first click.
  assert.match(page, /onDelete=\{\(\) => void remove\(/);
  assert.equal(page.match(/commands\.deleteMeeting\(/g)?.length, 1);
  ok("L1 delete confirmation");
}

// L2: retention keeps by default; presets and a stored custom number round-trip.
{
  assert.equal(retentionValue(null), "keep");
  assert.equal(retentionDays("keep"), null);
  assert.equal(retentionDays("30"), 30);
  const options = retentionOptions(null, tr);
  assert.deepEqual(
    options.map((o) => o.value),
    ["keep", "7", "30", "90", "180", "365"],
  );
  assert.equal(options[0].label, "Manter");
  assert.equal(options[2].label, "Apagar após 30 dias");
  assert.deepEqual(
    retentionOptions(45, tr).map((o) => o.value),
    ["keep", "7", "30", "45", "90", "180", "365"],
  );
  // What deleting the audio takes away is said before choosing it.
  assert.match(
    tr("settings.meetings.retention.description"),
    /tocar um trecho.*transcrever de novo/,
  );
  // Shortening (or leaving "keep") deletes audio now: it asks; lengthening or keeping does not.
  assert.equal(retentionNeedsConfirm(null, 30), true);
  assert.equal(retentionNeedsConfirm(90, 30), true);
  assert.equal(retentionNeedsConfirm(30, 90), false);
  assert.equal(retentionNeedsConfirm(30, null), false);
  const confirm = renderToStaticMarkup(
    <RetentionConfirm days={30} onConfirm={noop} onCancel={noop} t={tr} />,
  );
  assert.ok(confirm.includes("mais de 30 dias será apagado agora"));
  assert.equal(confirm.match(/<button/g)?.length, 2);
  ok("L2 retention options");
}

// L3: "Recover the recording from 14:02 (12 min)?" from the orphan the backend lists.
{
  const orphan: MeetingOrphan = {
    id: "01K0000000000000000000000A",
    title: "",
    started_at: "2026-10-12T14:02:00-03:00",
    recorded_ms: 719_400,
  };
  assert.equal(orphanTime(orphan), "14:02");
  assert.equal(orphanMinutes(orphan), 12);
  assert.equal(orphanMinutes({ ...orphan, recorded_ms: 4_000 }), 1);
  const html = renderToStaticMarkup(
    <RecoverBanner orphans={[orphan]} onRecover={noop} t={tr} />,
  );
  assert.ok(html.includes("Recuperar gravação de 14:02 (12 min)?"));
  assert.equal(html.match(/<button/g)?.length, 1);
  const none = <RecoverBanner orphans={[]} onRecover={noop} t={tr} />;
  assert.equal(renderToStaticMarkup(none), "");
  assert.match(page, /meetingOrphans\(\)/);
  ok("L3 recover banner");
}
