// Prova dos checks de `.specs/features/meeting-transcribe-retry/checks.md` (R5): o que a reunião
// mostra enquanto a transcrição automática espera uma nova tentativa ou falhou de vez.
// Rode com `bun src/components/meeting/retry.test.tsx`: imprime `<check> ok` e sai com erro na
// primeira falha. Sem instância do i18next, `t` devolve a chave.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { Translate } from "./meetingModel";
import { transcriptWait } from "./meetingModel";
import { SessionBar, type SessionBarProps } from "./SessionBar";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const keys: Translate = (key) => key;
const noop = () => {};
const ok = (id: string) => console.log(`${id} ok`);

const bar = (over: Partial<SessionBarProps> = {}) =>
  renderToStaticMarkup(
    <SessionBar
      draft={false}
      live={null}
      transcriptOpen={false}
      onToggleTranscript={noop}
      onRecordDraft={noop}
      transcribing={false}
      transcribeLabel="meeting.view.transcribe"
      transcribeDisabled={false}
      onTranscribe={noop}
      onCancelTranscription={noop}
      generateLabel="meeting.view.generate"
      generateDisabled={false}
      onGenerate={noop}
      templateSelect={null}
      templateMenu={() => null}
      t={keys}
      {...over}
    />,
  );

// W1: esperando, a barra diz por quê e o botão tenta agora; falhou de vez, diz que o áudio e as
// notas estão salvos e o botão tenta de novo; transcrevendo, o estado de espera some.
{
  const offline = transcriptWait(
    { state: "waiting", reason: "offline" },
    false,
  );
  assert.deepEqual(offline, {
    status: "meeting.view.wait.offline",
    action: "meeting.view.retryNow",
  });
  assert.equal(
    transcriptWait({ state: "waiting", reason: "provider" }, false).status,
    "meeting.view.wait.provider",
  );
  assert.equal(
    transcriptWait({ state: "waiting", reason: "queued" }, false).status,
    "meeting.view.wait.queued",
  );
  const failed = transcriptWait(
    { state: "failed", error: { kind: "transcription", detail: "HTTP 401" } },
    false,
  );
  assert.deepEqual(failed, {
    status: "meeting.view.wait.failed",
    action: "meeting.view.retry",
  });
  assert.deepEqual(
    transcriptWait({ state: "waiting", reason: "offline" }, true),
    { status: null, action: "meeting.view.transcribing" },
  );
  assert.deepEqual(transcriptWait(undefined, false), {
    status: null,
    action: "meeting.view.transcribe",
  });
  ok("W1");
}

// W2: a barra mostra o estado num `role="status"`, à frente da dica de chave, e o rótulo do botão.
{
  const html = bar({
    transcriptStatus: "meeting.view.wait.offline",
    transcribeHint: "falta a chave",
    transcribeLabel: "meeting.view.retryNow",
  });
  // Só o estado no texto da barra; a dica fica no `title` do botão.
  assert.match(html, /role="status"[^>]*>meeting\.view\.wait\.offline<\/div>/);
  assert.ok(html.includes(">meeting.view.retryNow</button>"));
  assert.ok(bar({ transcribeHint: "falta a chave" }).includes("falta a chave"));
  ok("W2");
}

// W3: a página trata `waiting` como fim da tentativa (não como falha na lista), a reunião lê
// `auto_transcript`, e o erro de rede tem texto próprio.
{
  const page = read("./MeetingsPage.tsx");
  assert.match(
    page,
    /stage\.stage === "notes" \|\|\s*stage\.stage === "waiting"/,
  );
  assert.match(
    page,
    /stage\.stage === "failed" \|\|\s*stage\.stage === "waiting"/,
  );
  const view = read("./MeetingView.tsx");
  assert.match(
    view,
    /transcriptWait\(\s*detail\.auto_transcript,\s*props\.transcribing,/,
  );
  const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
  for (const key of ["offline", "provider", "queued", "failed", "local"])
    assert.ok(pt.meeting.view.wait[key], key);
  ok("W3");
}

// W4: a transcrição local diz que é local e, com a chave, oferece a nuvem; esperando o modelo
// local, a barra diz por quê.
{
  assert.deepEqual(transcriptWait(null, false, true, true), {
    status: "meeting.view.localTranscript",
    action: "meeting.view.transcribeCloud",
  });
  assert.equal(
    transcriptWait(null, false, true, false).action,
    "meeting.view.transcribe",
  );
  assert.equal(
    transcriptWait({ state: "waiting", reason: "local" }, false, true, true)
      .status,
    "meeting.view.wait.local",
  );
  assert.match(
    read("./MeetingView.tsx"),
    /detail\.transcript_local,\s*props\.keys\?\.elevenlabs/,
  );
  // A recusa da chave diz o que fazer, em vez da falha genérica.
  assert.equal(
    transcriptWait({ state: "failed", error: { kind: "invalid_key" } }, false)
      .status,
    "meeting.view.wait.keyRefused",
  );
  ok("W4");
}

console.log("transcript retry: all assertions passed");
