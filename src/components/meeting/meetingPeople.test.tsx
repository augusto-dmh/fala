// Prova da UI de quem estava na reunião e das sugestões de nome (ADR-0016: nada se aplica sem
// clique). Rode com `bun src/components/meeting/meetingPeople.test.tsx`: imprime `<check> ok` e
// sai com erro na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingLine } from "@/bindings";
import { SuggestionList } from "./MeetingPeoplePanel";
import { lineText, parseParticipants, type Translate } from "./meetingModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const en = JSON.parse(read("../../i18n/locales/en/translation.json"));
const panel = read("./MeetingPeoplePanel.tsx");
const view = read("./MeetingView.tsx");

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

// "Quem estava": nomes separados por vírgula, ponto e vírgula ou linha, sem vazios.
{
  assert.deepEqual(
    parseParticipants("Ana Souza, Bruno\nCarla; ;  Dani   Lima "),
    ["Ana Souza", "Bruno", "Carla", "Dani Lima"],
  );
  assert.deepEqual(parseParticipants("  ,\n"), []);
  ok("participants");
}

// Uma sugestão por pessoa, com "aplicar" e a frase; sem sugestão, nada aparece.
{
  const html = renderToStaticMarkup(
    <SuggestionList
      suggestions={[
        { person: 1, name: "Ana Souza", reason: "Diz que cuida do contrato." },
      ]}
      onApply={() => {}}
    />,
  );
  assert.ok(html.includes("meeting.people.suggestion"), html);
  assert.ok(html.includes("meeting.people.apply"), html);
  assert.ok(html.includes("Diz que cuida do contrato."), html);
  assert.equal(
    renderToStaticMarkup(
      <SuggestionList suggestions={[]} onApply={() => {}} />,
    ),
    "",
  );
  assert.equal(
    tr("meeting.people.suggestion", {
      person: tr("meeting.speaker.person", { n: 1 }),
      name: "Ana Souza",
    }),
    "Sugestão: Pessoa 1 é Ana Souza",
  );
  ok("suggestions");
}

// O único caminho até `applySpeakerSuggestion` é o clique em "aplicar".
{
  assert.equal(panel.match(/commands\.applySpeakerSuggestion/g)?.length, 1);
  assert.ok(/onClick=\{\(\) => onApply\(suggestion\.person\)\}/.test(panel));
  assert.ok(/onApply=\{\(person\) => void apply\(person\)\}/.test(panel));
  assert.ok(view.includes("<MeetingPeoplePanel"));
  ok("apply-only-on-click");
}

// O nome aplicado substitui "Pessoa N" na transcrição; "Eu" nunca muda.
{
  const line: MeetingLine = {
    seq: 1,
    channel: "system",
    person: 1,
    t0_ms: 65_000,
    t1_ms: 70_000,
    text: "Proponho 15 de novembro.",
  };
  assert.equal(
    lineText(line, tr),
    "[01:05] Pessoa 1: Proponho 15 de novembro.",
  );
  assert.equal(
    lineText({ ...line, name: "Ana Souza" }, tr),
    "[01:05] Ana Souza: Proponho 15 de novembro.",
  );
  assert.equal(
    lineText({ ...line, person: null, channel: "mic", name: null }, tr),
    "[01:05] Eu: Proponho 15 de novembro.",
  );
  ok("named-line");
}

// Toda chave nova existe em pt e em en.
{
  const keys = [
    "people.participants",
    "people.placeholder",
    "people.hint",
    "people.suggestions",
    "people.suggestion",
    "people.apply",
    "context.title",
    "context.description",
    "context.profile.title",
    "context.profile.description",
    "context.profile.placeholder",
    "context.language.title",
    "context.language.description",
    "context.language.ptBR",
    "context.language.en",
    "context.language.app",
    "context.keyterms.title",
    "context.keyterms.description",
  ];
  for (const key of keys) {
    for (const [name, locale] of [
      ["pt", pt],
      ["en", en],
    ]) {
      const value = key
        .split(".")
        .reduce((node, part) => node?.[part], locale.meeting);
      assert.equal(typeof value, "string", `${name}: meeting.${key}`);
    }
  }
  ok("locales");
}
