// Prova do QA Q19 e Q20: os links abrem Configurações na linha certa, e "Reuniões" no rail volta à
// lista. Rode com `bun src/components/settingsAnchor.test.ts`.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { revealSettingsAnchor } from "./settingsAnchor";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);

// The anchored row scrolls to the middle and takes the focus itself (its control can still be
// loading or disabled).
{
  const calls: string[] = [];
  const row = {
    scrollIntoView: (options: unknown) =>
      calls.push(`scroll ${JSON.stringify(options)}`),
    focus: (options: unknown) => calls.push(`focus ${JSON.stringify(options)}`),
  };
  const root = {
    querySelector: (selector: string) =>
      selector === '[data-settings-anchor="meetings-key"]' ? row : null,
  } as unknown as ParentNode;

  assert.equal(revealSettingsAnchor(root, "meetings-key"), true);
  assert.deepEqual(calls, [
    'scroll {"block":"center"}',
    'focus {"preventScroll":true}',
  ]);
  assert.equal(revealSettingsAnchor(root, "microphone"), false);
  ok("Q19-reveal");
}

// The links set the anchor, the rows carry it, and the reveal runs after the scroll reset.
{
  const app = read("../App.tsx");
  assert.match(
    app,
    /"open-microphone-settings", \(\) => \{[\s\S]*?setSettingsAnchor\("microphone"\);/,
  );
  assert.match(
    app,
    /onOpenSettings=\{\(\) => \{[\s\S]*?setSettingsAnchor\("meetings-key"\);/,
  );
  assert.match(
    app,
    /if \(!main\) return;\s*revealSettingsAnchor\(main, settingsAnchor\);\s*setSettingsAnchor\(null\);/,
  );
  assert.match(
    read("./ui/SettingContainer.tsx"),
    /data-settings-anchor=\{anchor\}\s*tabIndex=\{anchor \? -1 : undefined\}/,
  );
  assert.match(
    read("./settings/MicrophoneSelector.tsx"),
    /anchor="microphone"/,
  );
  ok("Q19-links");
}

// "Meetings" in the rail, with a meeting open, goes back to the list.
{
  const app = read("../App.tsx");
  assert.match(
    app,
    /if \(next === "meetings" && destination === "meetings"\) \{\s*setMeetingsHome\(\(n\) => n \+ 1\);/,
  );
  assert.match(app, /<MeetingsPage\s+home=\{meetingsHome\}/);
  assert.match(
    read("./meeting/MeetingsPage.tsx"),
    /useEffect\(\(\) => \{\s*if \(home > 0\) setSelected\(null\);\s*\}, \[home\]\);/,
  );
  ok("Q20");
}
