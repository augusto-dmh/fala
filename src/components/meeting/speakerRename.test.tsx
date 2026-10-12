// Prova do renomear "Pessoa N" (`meeting-notes-polish`, PR 1). Rode com
// `bun src/components/meeting/speakerRename.test.tsx`: imprime `<check> ok` e sai com erro na
// primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingLine } from "@/bindings";
import type { Translate } from "./meetingModel";
import { SpeakerName, TranscriptView } from "./TranscriptView";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const en = JSON.parse(read("../../i18n/locales/en/translation.json"));
const viewSource = read("./MeetingView.tsx");

function ok(id: string) {
  console.log(`${id} ok`);
}

const tr: Translate = (key, options = {}) => {
  const value = key.split(".").reduce((node, part) => node?.[part], pt);
  assert.equal(typeof value, "string", `chave sem texto em pt: ${key}`);
  return (value as string).replace(/\{\{(\w+)\}\}/g, (_, name) =>
    String(options[name]),
  );
};

const noop = () => {};

const lines: MeetingLine[] = [
  {
    seq: 12,
    channel: "system",
    person: 1,
    t0_ms: 65_000,
    t1_ms: 70_000,
    text: "Proponho 15 de novembro.",
  },
  {
    seq: 13,
    channel: "system",
    person: 2,
    name: "Ana",
    t0_ms: 70_000,
    t1_ms: 72_000,
    text: "Eu fico com o contrato.",
  },
  {
    seq: 14,
    channel: "mic",
    person: null,
    t0_ms: 72_000,
    t1_ms: 74_000,
    text: "Fechado.",
  },
];

// "Pessoa N" e o nome aplicado viram botão de renomear; "Eu" continua texto; sem `onRename`,
// nada muda.
{
  const html = renderToStaticMarkup(
    <TranscriptView
      lines={lines}
      audioAvailable
      playing={null}
      highlighted={null}
      onPlay={noop}
      onRename={noop}
      t={tr}
    />,
  );
  assert.equal(html.match(/<button[^>]*title="Clique para dar/g)?.length, 2);
  assert.ok(html.includes(">Pessoa 1</button>"), html);
  assert.ok(html.includes(">Ana</button>"), html);
  assert.ok(!html.includes(">Eu</button>"), html);
  assert.equal(
    renderToStaticMarkup(<SpeakerName line={lines[0]} t={tr} />),
    "Pessoa 1",
  );
  ok("RN-label");
}

// O nome vai ao comando pela página da sessão; numa sessão com notas, depois do nome trocado, a
// página oferece gerar de novo (nada é gerado sem clique).
{
  assert.match(viewSource, /commands\.setMeetingSpeakerName\(/);
  // O nome digitado e o "aplicar" do painel passam pelo mesmo ponto, que oferece gerar de novo.
  assert.match(
    viewSource,
    /const namesChanged = \(\) => \{\s*if \(detail\.notes_md !== null \|\| props\.generating\) setOfferRegenerate\(true\);/,
  );
  assert.match(viewSource, /if \(result\.status === "ok"\) namesChanged\(\);/);
  assert.match(viewSource, /onNamesChanged=\{namesChanged\}/);
  // Durante uma transcrição nova, os rótulos não são renomeáveis (ela apaga os nomes).
  assert.match(viewSource, /props\.transcribing\s*\?\s*undefined/);
  assert.match(viewSource, /: \(person, name\) => void rename\(person, name\)/);
  ok("RN-wiring");
}

// Toda string nova existe em pt e en.
{
  for (const key of ["hint", "label", "regenerate", "generate", "dismiss"]) {
    assert.equal(typeof pt.meeting.rename[key], "string", key);
    assert.equal(typeof en.meeting.rename[key], "string", key);
  }
  ok("RN-i18n");
}
