// Prova dos checks do aviso na janela da pill (`.specs/features/mic-toasts/checks.md`, C6, C15 e
// C16). Rode com `bun src/overlay/notice.test.tsx`: imprime `<check> ok` e sai com erro na
// primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { Notice } from "./Notice";
import {
  NOTICE_KINDS,
  noticeKey,
  noticeTone,
  type NoticeKind,
} from "./noticeModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const css = read("./Notice.css");
const overlaySource = read("./RecordingOverlay.tsx");
const appSource = read("../App.tsx");
const locales = ["pt", "en"].map((lang) => ({
  lang,
  json: JSON.parse(read(`../i18n/locales/${lang}/translation.json`)),
}));

/** The value at a dotted i18n key, or undefined. */
function lookup(json: unknown, key: string): unknown {
  return key
    .split(".")
    .reduce<unknown>(
      (node, part) =>
        node && typeof node === "object"
          ? (node as Record<string, unknown>)[part]
          : undefined,
      json,
    );
}

// C6: the main window translates the two missing-model codes.
assert.match(
  appSource,
  /error_code === "model_not_downloaded"\s*\?\s*t\("errors\.modelNotDownloaded"\)/,
);
assert.match(
  appSource,
  /error_code === "model_not_found"\s*\?\s*t\("errors\.modelNotFound"\)/,
);
for (const { lang, json } of locales) {
  for (const key of ["errors.modelNotDownloaded", "errors.modelNotFound"]) {
    assert.equal(typeof lookup(json, key), "string", `${lang}: ${key}`);
  }
}
console.log("C6 ok");

// C15: text by code, tone, keys in both languages, literal colours.
const expected: Record<NoticeKind, { key: string; tone: "warn" | "error" }> = {
  mic_denied: { key: "overlay.notice.micDenied", tone: "error" },
  no_mic: { key: "overlay.notice.noMic", tone: "error" },
  mic_failed: { key: "overlay.notice.micFailed", tone: "error" },
  model_missing: { key: "overlay.notice.modelMissing", tone: "error" },
};
assert.deepEqual([...NOTICE_KINDS].sort(), Object.keys(expected).sort());
for (const kind of NOTICE_KINDS) {
  const want = expected[kind];
  assert.equal(noticeKey(kind), want.key, kind);
  assert.equal(noticeTone(kind), want.tone, kind);

  const html = renderToStaticMarkup(
    <Notice kind={kind} text={`texto-${kind}`} />,
  );
  assert.ok(html.includes(`texto-${kind}`), html);
  const dots = [...html.matchAll(/class="fnotice-dot (\w+)"/g)].map(
    (m) => m[1],
  );
  assert.deepEqual(dots, [want.tone], `${kind}: ${html}`);
}
assert.match(overlaySource, /text=\{t\(noticeKey\(notice\.kind\)\)\}/);
for (const { lang, json } of locales) {
  for (const { key } of Object.values(expected)) {
    const value = lookup(json, key);
    assert.equal(typeof value, "string", `${lang}: ${key}`);
    assert.ok((value as string).length > 0, `${lang}: ${key}`);
  }
}
assert.doesNotMatch(css, /var\(/, "Notice.css uses literal colours");
assert.match(css, /\.fnotice-dot\.error \{\s*background: #e5322d;/);
console.log("C15 ok");

// C16: the overlay listens, clears the notice on hide and on a new session, and draws an alone
// notice without the pill.
assert.match(overlaySource, /listen<OverlayNotice>\(\s*"overlay-notice"/);
const hide = overlaySource.match(
  /listen\("hide-overlay", \(\) => \{([\s\S]*?)\}\);/,
);
assert.ok(hide, "hide-overlay listener");
assert.match(hide[1], /setNotice\(null\);/);
const session = overlaySource.match(
  /if \(overlayState === "recording" \|\| overlayState === "streaming"\) \{([\s\S]*?)\n {8}\}/,
);
assert.ok(session, "recording/streaming reset block");
assert.match(session[1], /setNotice\(null\);/);
const alone = overlaySource.match(
  /if \(notice\?\.alone\) \{\s*return \(([\s\S]*?)\);\s*\}/,
);
assert.ok(alone, "alone branch");
assert.match(alone[1], /<Notice /);
assert.doesNotMatch(alone[1], /Pill|pill/);
console.log("C16 ok");
