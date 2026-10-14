// Prova dos checks do diálogo de atalhos. Rode com `bun src/components/settings/shortcuts.test.tsx`:
// imprime `<check> ok` e sai com erro na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createInstance } from "i18next";
import { renderToStaticMarkup } from "react-dom/server";
import { I18nextProvider, initReactI18next } from "react-i18next";
import type { BindingResponse, ShortcutBinding } from "@/bindings";
import { ShortcutRow } from "./ShortcutsDialog";
import {
  dialogActions,
  escapeCancelsRecording,
  responseMessage,
  rowView,
} from "./shortcutsModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const en = JSON.parse(read("../../i18n/locales/en/translation.json"));
const ok = (id: string) => console.log(`${id} ok`);

const binding = (
  id: string,
  primary: string,
  alternate = "",
): ShortcutBinding => ({
  id,
  name: id,
  description: "",
  default_binding: "",
  current_binding: primary,
  alternate_binding: alternate,
});

const response = (partial: Partial<BindingResponse>): BindingResponse => ({
  success: true,
  binding: null,
  error: null,
  refusal: null,
  overlap: null,
  ...partial,
});

// Rows: the five actions in order, cancel hidden on Linux.
{
  assert.deepEqual(dialogActions(false), [
    "transcribe",
    "cancel",
    "paste_last",
    "copy_last",
    "meeting_toggle",
  ]);
  assert.ok(!dialogActions(true).includes("cancel"));
}
ok("rows");

// "+" only with one combination; 🗑 never on dictation's only one nor on cancel.
{
  const only = rowView(binding("transcribe", "ctrl+shift+space"));
  assert.equal(only.canAdd, true);
  assert.equal(only.slots[0].removable, false);
  const two = rowView(binding("transcribe", "ctrl+shift+space", "f9"));
  assert.equal(two.canAdd, false);
  assert.ok(two.slots.every((s) => s.removable));
  const cancel = rowView(binding("cancel", "escape"));
  assert.equal(cancel.canAdd, false);
  assert.equal(cancel.slots[0].removable, false);
  const empty = rowView(binding("meeting_toggle", ""));
  assert.equal(empty.slots.length, 0);
  assert.equal(empty.canAdd, false);
  const paste = rowView(binding("paste_last", "alt+shift+z"));
  assert.equal(paste.slots[0].removable, true);
  assert.equal(escapeCancelsRecording("paste_last"), true);
  assert.equal(escapeCancelsRecording("cancel"), false);
}
ok("buttons");

// Messages: in use with [Usar aqui], reserved, lone key, overlap as a warning.
{
  const i18n = createInstance();
  void i18n.use(initReactI18next).init({
    lng: "pt",
    resources: { pt: { translation: pt } },
    initAsync: false,
    interpolation: { escapeValue: false },
  });
  const render = (
    row: ShortcutBinding,
    combo: string,
    saved: BindingResponse,
  ) => {
    const message = responseMessage(saved);
    return renderToStaticMarkup(
      <I18nextProvider i18n={i18n}>
        <ShortcutRow
          binding={row}
          osType="windows"
          recording={null}
          recordedKeys=""
          note={message ? { message, combo, slot: "alternate" } : undefined}
          onRecord={() => {}}
          onRemove={() => {}}
          onTakeOver={() => {}}
        />
      </I18nextProvider>,
    );
  };
  const paste = binding("paste_last", "alt+shift+z");

  const inUse = render(
    paste,
    "ctrl+shift+space",
    response({
      success: false,
      refusal: {
        kind: "in_use",
        action: "copy_last",
        slot: "primary",
        take_over: true,
      },
    }),
  );
  assert.ok(inUse.includes("Já usado em “Copiar o último ditado”."), inUse);
  assert.ok(inUse.includes("Usar aqui"));

  const noTakeOver = render(
    paste,
    "ctrl+shift+space",
    response({
      success: false,
      refusal: {
        kind: "in_use",
        action: "transcribe",
        slot: "primary",
        take_over: false,
      },
    }),
  );
  assert.ok(noTakeOver.includes("Já usado em “Ditar”."));
  assert.ok(!noTakeOver.includes("Usar aqui"), "dictation keeps its only key");

  const reserved = render(
    paste,
    "ctrl+c",
    response({ success: false, refusal: { kind: "reserved" } }),
  );
  assert.ok(reserved.includes("O Windows ou outro app já usa Ctrl + C."));

  const lone = render(
    paste,
    "a",
    response({ success: false, refusal: { kind: "lone_key" } }),
  );
  assert.ok(lone.includes("uma tecla sozinha atrapalharia a digitação"));

  const overlap = render(
    paste,
    "ctrl+super",
    response({
      overlap: {
        action: "meeting_toggle",
        smaller: "ctrl+super",
        larger: "ctrl+super+alt",
      },
    }),
  );
  assert.ok(
    overlap.includes(
      "Ctrl + Super faz parte de Ctrl + Super + Alt (Gravar ou parar reunião)",
    ),
    overlap,
  );
  assert.ok(overlap.includes('role="status"'), "overlap is a warning");

  assert.equal(responseMessage(response({})), null, "a clean save");
}
ok("messages");

// Restore defaults: both refusals and the toast exist in pt and en.
{
  for (const locale of [pt, en]) {
    const s = locale.settings.shortcuts;
    for (const key of ["dictation_active", "meeting_recording"]) {
      assert.ok(s.resetRefused[key], key);
    }
    assert.ok(s.restored && s.reset && s.done && s.saved);
  }
  assert.equal(
    pt.settings.shortcuts.resetRefused.meeting_recording,
    "Pare a reunião antes de restaurar os atalhos.",
  );
}
ok("restore");
