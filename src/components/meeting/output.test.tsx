// Prova do seletor "Som da reunião" e do aviso de saída (feature loopback-device-select). Rode com
// `bun src/components/meeting/output.test.tsx`: imprime `<check> ok` e sai com erro na primeira
// falha. Sem instância do i18next, `t` devolve a chave.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingOutputs, MeetingStatus } from "@/bindings";
import { CallOutputCard, MeetingPill } from "../../overlay/MeetingPill";
import { outputOptions, RecorderPanel } from "./RecorderPanel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const en = JSON.parse(read("../../i18n/locales/en/translation.json"));
const page = read("./MeetingsPage.tsx");
const overlay = read("../../overlay/RecordingOverlay.tsx");
const noticeCss = read("../../overlay/Notice.css");

function ok(id: string) {
  console.log(`${id} ok`);
}

const noop = () => {};

const status = (over: Partial<MeetingStatus> = {}): MeetingStatus => ({
  state: "idle",
  id: null,
  mode: null,
  recorded_ms: 0,
  mic_level: 0,
  system_level: 0,
  muted_mic: false,
  muted_system: false,
  mic_unheard: false,
  cap_remaining_ms: null,
  low_disk_bytes: null,
  mic_input: null,
  system_output: null,
  call_output: null,
  zero_mic: false,
  zero_system: false,
  dictating: false,
  ...over,
});

const outputs: MeetingOutputs = {
  devices: [
    { name: "Alto-falantes (Realtek(R) Audio)", is_default: true },
    { name: "Fone de ouvido (USB Audio Device)", is_default: false },
  ],
  selected: null,
};

const panel = (s: MeetingStatus, o: MeetingOutputs | null = outputs) =>
  renderToStaticMarkup(
    <RecorderPanel
      status={s}
      mode="meeting"
      title=""
      outputs={o}
      onModeChange={noop}
      onTitleChange={noop}
      onOutputChange={noop}
      onSwitchOutput={noop}
      onStart={noop}
      onNewDraft={noop}
      onPause={noop}
      onResume={noop}
      onStop={noop}
      onExtend={noop}
    />,
  );

// OUT1: parado, o painel mostra "Som da reunião" com a saída padrão e a lista.
{
  // QA Q26: the app's Dropdown with its name above, the default output named in the list.
  const html = panel(status());
  // The name above labels the Dropdown's button.
  assert.match(
    html,
    /<span id="([^"]+)"[^>]*>meeting\.recorder\.output<\/span><div class="relative "><button[^>]*aria-labelledby="\1 /,
    html,
  );
  assert.ok(!html.includes("<select"), html);
  // Closed, the button shows the default output.
  assert.ok(
    html.includes('class="truncate">meeting.recorder.outputDefault'),
    html,
  );
  const options = outputOptions(outputs, (name) => `default (${name})`);
  assert.equal(options[0].value, "");
  assert.ok(options[0].label.startsWith("default ("));
  for (const d of outputs.devices)
    assert.ok(options.some((o) => o.value === d.name && o.label === d.name));
  // A escolha lembrada aparece selecionada; um fone ausente continua na lista.
  const chosen = panel(status(), {
    ...outputs,
    selected: "Fone Bluetooth",
  });
  assert.ok(chosen.includes('class="truncate">Fone Bluetooth<'), chosen);
  assert.ok(
    outputOptions({ ...outputs, selected: "Fone Bluetooth" }, String).some(
      (o) => o.value === "Fone Bluetooth",
    ),
  );
  // Sem a lista ainda, nada de seletor vazio.
  assert.ok(!panel(status(), null).includes("meeting.recorder.output"));
  // A escolha vai para o Settings, e a troca a quente para o comando dela.
  assert.ok(page.includes("void commands.setMeetingOutput(name);"));
  assert.ok(page.includes("commands.switchMeetingOutput(name)"));
  assert.equal(pt.meeting.recorder.output, "Som da reunião");
  assert.equal(en.meeting.recorder.output, "Meeting sound");
  ok("OUT1");
}

// OUT2: gravando, o painel diz de onde vem o som e avisa quando a call toca noutro lugar.
{
  const recording = status({
    state: "recording",
    system_output: "Alto-falantes (Realtek(R) Audio)",
  });
  const html = panel(recording);
  assert.ok(html.includes("meeting.recorder.outputFrom"), html);
  assert.ok(!html.includes("meeting.recorder.callElsewhere"), html);
  assert.ok(!html.includes('meeting.recorder.output"'), "sem seletor ao vivo");
  const elsewhere = panel({
    ...recording,
    call_output: "Fone de ouvido (USB Audio Device)",
  });
  assert.ok(elsewhere.includes("meeting.recorder.callElsewhere"), elsewhere);
  assert.ok(elsewhere.includes("meeting.recorder.recordFrom"), elsewhere);
  assert.ok(elsewhere.includes('role="alert"'), elsewhere);
  assert.equal(
    pt.meeting.recorder.callElsewhere,
    "O som da chamada está saindo por {{device}}.",
  );
  assert.equal(pt.meeting.recorder.recordFrom, "Gravar deste dispositivo");
  ok("OUT2");
}

// OUT3: a pill mostra o mesmo aviso, com o botão, por cima do indicador da reunião (ADR-0005).
{
  const card = renderToStaticMarkup(
    <CallOutputCard text="aviso" switchLabel="trocar" onSwitch={noop} />,
  );
  assert.ok(card.includes("aviso") && card.includes(">trocar</button>"), card);
  assert.ok(card.includes("fnotice-dot warn"), card);
  assert.ok(noticeCss.includes(".fnotice"), "o cartão usa o estilo dos avisos");
  assert.ok(
    overlay.includes("commands.switchMeetingOutput(callOutput)"),
    "o botão da pill troca o dispositivo",
  );
  assert.ok(
    overlay.includes("setCallOutput(event.payload.call_output)"),
    "o aviso vem do MeetingStatus",
  );
  const stack = overlay.slice(overlay.indexOf("<CallOutputCard"));
  assert.ok(stack.indexOf("{meetingPill}") > 0, "a pill continua embaixo");
  assert.equal(pt.overlay.recordFrom, "Gravar deste dispositivo");
  ok("OUT3");
}

// OUT4: ~3 s de zero digital num canal deixam o ponto da pill âmbar; o aviso de 2 min continua.
{
  assert.ok(
    overlay.includes(
      "const zero = event.payload.zero_mic || event.payload.zero_system;",
    ),
  );
  assert.ok(overlay.includes("setMeetingMuted(muted || zero);"));
  const amber = renderToStaticMarkup(
    <MeetingPill paused={false} recordedMs={0} muted label="r" />,
  );
  assert.ok(amber.includes("muted"), amber);
  assert.ok(
    panel(status({ state: "recording", muted_system: true })).includes(
      "meeting.recorder.mutedSystem",
    ),
  );
  ok("OUT4");
}

console.log("output: all assertions passed");
