// Prova dos checks de `.specs/features/qa-transcribe-needs-audio/checks.md` (R5 e o resto do Q15,
// da segunda passada de QA, 2026-10-13).
// Rode com `bun src/components/meeting/transcribeAudio.test.tsx`: imprime `<check> ok` e sai com
// erro na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import type { MeetingSummary } from "@/bindings";
import { sessionHints } from "./meetingModel";
import { metaLine } from "./SessionHeader";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);
const t = (key: string) => key;

// A1: sem áudio no disco, "Transcrever" fica desligado; sem transcrição, diz por quê, com ou sem a
// chave; com transcrição, fica quieto. Um WAV ainda não convertido conta como áudio.
{
  const keys = { elevenlabs: true, anthropic: true };
  const fresh = { transcript: false, typed: true };
  assert.equal(
    sessionHints(keys, { ...fresh, audio: "deleted" }).transcribe,
    "meeting.source.noAudio",
  );
  assert.equal(
    sessionHints(null, { ...fresh, audio: "deleted" }).transcribe,
    "meeting.source.noAudio",
  );
  assert.equal(
    sessionHints(keys, { ...fresh, audio: "imported" }).transcribe,
    "meeting.granola.noAudio",
  );
  assert.equal(
    sessionHints(keys, { transcript: true, typed: false, audio: "deleted" })
      .transcribe,
    null,
  );
  assert.equal(
    sessionHints(keys, { ...fresh, audio: "kept" }).transcribe,
    null,
  );
  const view = read("./MeetingView.tsx");
  assert.match(view, /detail\.audio_available \|\| detail\.audio_pending/);
  assert.match(view, /audio !== "kept"/);
  ok("A1");
}

// A2: o cabeçalho de uma importação do Granola não mostra a duração "00:00"; uma gravada mostra.
{
  const summary = (over: Partial<MeetingSummary>): MeetingSummary =>
    ({
      id: "s",
      title: "",
      mode: "import",
      created_at: "2026-10-12T10:00:00-03:00",
      started_at: "2026-10-12T10:00:00-03:00",
      ended_at: null,
      recorded_ms: 0,
      audio_retained: false,
      transcribed: true,
      has_notes: true,
      ...over,
    }) as MeetingSummary;
  const now = new Date(2026, 9, 13, 12, 0);
  const granola = metaLine(
    summary({ imported_from: "granola" }),
    now,
    "en-US",
    t,
  );
  assert.ok(!granola.includes("00:00"), granola);
  const recorded = metaLine(summary({ recorded_ms: 139_000 }), now, "en-US", t);
  assert.ok(recorded.includes("02:19"), recorded);
  ok("A2");
}

console.log("transcribe audio: all assertions passed");
