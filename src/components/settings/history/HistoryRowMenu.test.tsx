// Prova do QA Q4: o menu "⋮" da linha do histórico e o ▷ no hover.
// Rode com `bun src/components/settings/history/HistoryRowMenu.test.tsx`.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { HistoryRowMenuItems } from "./HistoryRowMenu";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);
const noop = () => {};
const items = (over: Partial<Parameters<typeof HistoryRowMenuItems>[0]>) =>
  renderToStaticMarkup(
    <HistoryRowMenuItems
      editAction="undo"
      canRetranscribe
      busy={false}
      onSwitchText={noop}
      onRetranscribe={noop}
      onDelete={noop}
      {...over}
    />,
  );
const labels = (html: string) =>
  [...html.matchAll(/role="menuitem"[^>]*>([^<]*)</g)].map((m) => m[1]);

// The menu: the AI edit and transcribe again on top, delete apart and in red.
{
  const html = items({});
  assert.deepEqual(labels(html), [
    "settings.history.undoAiEdit",
    "settings.history.retranscribe",
    "settings.history.delete",
  ]);
  assert.equal(html.match(/role="separator"/g)?.length, 1, html);
  const del = html.slice(html.lastIndexOf("<button"));
  assert.ok(del.includes("text-danger") && del.includes("delete"), del);
  assert.equal(html.match(/text-danger/g)?.length, 1, html);
  assert.equal(
    labels(items({ editAction: "redo" }))[0],
    "settings.history.applyAiEdit",
  );
  ok("Q4-menu");
}

// Without the AI edit or the audio, only delete is left, with no separator; a retry locks it.
{
  const html = items({ editAction: null, canRetranscribe: false });
  assert.deepEqual(labels(html), ["settings.history.delete"]);
  assert.ok(!html.includes('role="separator"'));
  assert.equal(
    items({ busy: true }).match(/disabled=""/g)?.length,
    3,
    "a retry disables every item",
  );
  ok("Q4-empty");
}

// The row: ▷ opens the player and plays, only with the audio; the old inline icons are gone.
{
  const row = read("./HistorySettings.tsx");
  assert.match(
    row,
    /\{!entry\.audio_deleted && \(\s*<IconButton\s*onClick=\{\(\) => setPlaying\(\(open\) => !open\)\}/,
  );
  assert.match(
    row,
    /const showPlayer = playing && !entry\.audio_deleted;[\s\S]*\{showPlayer && \(\s*<AudioPlayer\s*onLoadRequest=\{handleLoadAudio\}\s*autoPlay/,
  );
  assert.match(row, /<HistoryRowMenu\s/);
  // An open menu keeps the row's actions visible, so its items never click invisibly.
  assert.ok(row.includes("has-[[aria-expanded=true]]:opacity-100"));
  for (const gone of ["Trash2", "RotateCcw", "Undo2", "Redo2"]) {
    assert.ok(!row.includes(gone), gone);
  }
  assert.match(
    read("../../ui/AudioPlayer.tsx"),
    /if \(!autoPlay \|\| hasSrc \|\| !onLoadRequest\) return;/,
  );
  ok("Q4-row");
}
