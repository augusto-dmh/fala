// Prova dos checks do onboarding em três passos (`.specs/features/ui-onboarding/checks.md`, S1).
// Rode com `bun src/components/onboarding/onboarding.test.tsx`: imprime `<check> ok` e sai com
// erro na primeira falha.
import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync } from "node:fs";

// `@tauri-apps/plugin-os` reads its platform info from `window` during render.
(globalThis as Record<string, unknown>).window = {
  __TAURI_OS_PLUGIN_INTERNALS__: { os_type: "windows", platform: "windows" },
};

const { renderToStaticMarkup } = await import("react-dom/server");
const model = await import("./onboardingModel");
const { default: Onboarding } = await import("./Onboarding");
const { MicrophoneStep } = await import("./MicrophoneStep");
const { MicLevelMeter } = await import("./MicLevelMeter");
const { createMicLevelSession } = await import("./micLevelSession");
const { ShortcutStep } = await import("./ShortcutStep");
const { AiStep } = await import("./AiStep");
const { ModelFooter } = await import("./ModelFooter");
const { OnboardingPreview } = await import(
  "../settings/debug/OnboardingPreview"
);
const { useSettingsStore } = await import("../../stores/settingsStore");

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const exists = (path: string) => existsSync(new URL(path, import.meta.url));
const ok = (id: string) => console.log(`${id} ok`);
const noop = () => {};
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const en = JSON.parse(read("../../i18n/locales/en/translation.json"));
const appTsx = read("../../App.tsx");
const at = (obj: Record<string, unknown>, path: string): unknown =>
  path
    .split(".")
    .reduce<unknown>(
      (node, key) =>
        node && typeof node === "object"
          ? (node as Record<string, unknown>)[key]
          : undefined,
      obj,
    );

// i18next is not initialised here, so `t` returns the key: labels show as keys.
const {
  ONBOARDING_STEPS,
  DICTATION_MODEL_ID,
  nextStep,
  previousStep,
  micAccess,
  levelFromTimeDomain,
  pickInputDeviceId,
  modelAction,
  modelFooterState,
  footerError,
} = model;

// C1: three steps in order, and the microphone-only path ends after its step.
{
  assert.deepEqual([...ONBOARDING_STEPS], ["microphone", "shortcut", "ai"]);
  assert.equal(nextStep(ONBOARDING_STEPS, "microphone"), "shortcut");
  assert.equal(nextStep(ONBOARDING_STEPS, "shortcut"), "ai");
  assert.equal(nextStep(ONBOARDING_STEPS, "ai"), null);
  assert.equal(previousStep(ONBOARDING_STEPS, "microphone"), null);
  assert.equal(previousStep(ONBOARDING_STEPS, "shortcut"), "microphone");
  assert.equal(previousStep(ONBOARDING_STEPS, "ai"), "shortcut");
  assert.equal(nextStep(["microphone"], "microphone"), null);
  ok("C1");
}

// C2: only an explicit Windows denial blocks; the step shows how to lift it.
{
  const status = (
    supported: boolean,
    overall_access: "allowed" | "denied" | "unknown",
  ) => ({
    supported,
    overall_access,
    device_access: overall_access,
    app_access: overall_access,
    desktop_app_access: overall_access,
  });
  assert.equal(micAccess(status(true, "denied")), "denied");
  assert.equal(micAccess(status(true, "allowed")), "ok");
  assert.equal(micAccess(status(true, "unknown")), "ok");
  assert.equal(micAccess(status(false, "denied")), "ok");

  const denied = renderToStaticMarkup(
    <MicrophoneStep
      access="denied"
      waiting={false}
      onOpenPrivacySettings={noop}
    />,
  );
  for (const key of [
    "onboarding.microphone.denied.title",
    "onboarding.microphone.denied.step1",
    "onboarding.microphone.denied.step2",
    "onboarding.microphone.denied.step3",
    "onboarding.microphone.denied.openSettings",
  ]) {
    assert.ok(denied.includes(key), `denied panel shows ${key}`);
  }
  assert.ok(!denied.includes("settings.sound.microphone.title"));
  assert.ok(!denied.includes('role="meter"'));

  const allowed = renderToStaticMarkup(
    <MicrophoneStep access="ok" waiting={false} onOpenPrivacySettings={noop} />,
  );
  assert.ok(allowed.includes("settings.sound.microphone.title"), "selector");
  assert.ok(allowed.includes('role="meter"'), "meter");
  assert.ok(!allowed.includes("onboarding.microphone.denied"));
  ok("C2");
}

// C3: the meter's level, its device pick, its ARIA and its cleanup.
{
  assert.equal(levelFromTimeDomain(new Uint8Array(512).fill(128)), 0);
  const square = new Uint8Array(512).map((_, i) => (i % 2 ? 255 : 0));
  const loud = levelFromTimeDomain(square);
  assert.ok(loud >= 0.9 && loud <= 1, `square wave level ${loud}`);
  assert.ok(levelFromTimeDomain(new Uint8Array(0)) === 0);

  const devices = [
    { deviceId: "out", kind: "audiooutput", label: "Mic A" },
    { deviceId: "a", kind: "audioinput", label: "Mic A" },
    { deviceId: "b", kind: "audioinput", label: "Mic B" },
  ] as const;
  assert.equal(pickInputDeviceId(devices, "Mic B"), "b");
  assert.equal(pickInputDeviceId(devices, "Mic A"), "a");
  assert.equal(pickInputDeviceId(devices, "Default"), undefined);
  assert.equal(pickInputDeviceId(devices, null), undefined);
  assert.equal(pickInputDeviceId(devices, "Mic C"), undefined);

  const html = renderToStaticMarkup(<MicLevelMeter microphone="Mic A" />);
  assert.match(html, /role="meter"/);
  assert.match(html, /aria-valuemin="0"/);
  assert.match(html, /aria-valuemax="100"/);
  assert.match(html, /aria-valuenow="0"/);

  // The session behind the meter, driven with fakes: every opened track is
  // stopped and the audio context closed on stop, also when the stream only
  // arrives after the stop (unmount mid-start) or when a second start
  // overtakes the first (double click).
  type Fake = { stopped: number; track: { stop: () => void } };
  const fakeStream = (): Fake & { stream: MediaStream } => {
    const fake: Fake = { stopped: 0, track: { stop: () => fake.stopped++ } };
    return {
      ...fake,
      get stopped() {
        return fake.stopped;
      },
      stream: { getTracks: () => [fake.track] } as unknown as MediaStream,
    };
  };
  const harness = () => {
    const pending: ((s: MediaStream) => void)[] = [];
    let closed = 0;
    let frames = 0;
    let cancelled = 0;
    const session = createMicLevelSession(
      {
        mediaDevices: {
          enumerateDevices: async () => [],
          getUserMedia: () =>
            new Promise<MediaStream>((resolve) => pending.push(resolve)),
        },
        createContext: () =>
          ({
            close: async () => {
              closed++;
            },
            createAnalyser: () => ({
              fftSize: 0,
              getByteTimeDomainData: (b: Uint8Array) => b.fill(128),
            }),
            createMediaStreamSource: () => ({ connect: () => {} }),
          }) as never,
        requestFrame: () => ++frames,
        cancelFrame: () => {
          cancelled++;
        },
      },
      () => {},
    );
    const flush = () => new Promise((r) => setTimeout(r, 0));
    return {
      session,
      pending,
      flush,
      counts: () => ({ closed, frames, cancelled }),
    };
  };
  {
    // Normal test, then stop: the track stops, the context closes, the loop ends.
    const h = harness();
    const a = fakeStream();
    const started = h.session.start("Mic A");
    await h.flush();
    h.pending[0](a.stream);
    assert.equal(await started, "listening");
    h.session.stop();
    assert.equal(a.stopped, 1);
    assert.equal(h.counts().closed, 1);
    assert.equal(h.counts().cancelled, 1);
  }
  {
    // Unmount while getUserMedia is pending: the late stream is stopped at once.
    const h = harness();
    const late = fakeStream();
    const started = h.session.start("Mic A");
    await h.flush();
    h.session.stop();
    h.pending[0](late.stream);
    assert.equal(await started, "cancelled");
    assert.equal(late.stopped, 1);
    assert.equal(h.counts().frames, 0, "no frame loop after a cancelled start");
  }
  {
    // Double click: the first stream is released, only the second listens.
    const h = harness();
    const first = fakeStream();
    const second = fakeStream();
    const one = h.session.start("Mic A");
    await h.flush();
    const two = h.session.start("Mic A");
    await h.flush();
    h.pending[0](first.stream);
    h.pending[1](second.stream);
    assert.equal(await one, "cancelled");
    assert.equal(await two, "listening");
    assert.equal(first.stopped, 1);
    assert.equal(second.stopped, 0);
    h.session.stop();
    assert.equal(second.stopped, 1);
  }
  {
    // Microphone changed while listening: the running stream is released
    // before the new one opens, and only the new one stays open.
    const h = harness();
    const first = fakeStream();
    const second = fakeStream();
    const one = h.session.start("Mic A");
    await h.flush();
    h.pending[0](first.stream);
    assert.equal(await one, "listening");
    const two = h.session.start("Mic B");
    assert.equal(first.stopped, 1, "the first track stops on restart");
    assert.equal(h.counts().closed, 1, "the first context closes on restart");
    assert.equal(h.counts().cancelled, 1, "the first loop ends on restart");
    await h.flush();
    h.pending[1](second.stream);
    assert.equal(await two, "listening");
    assert.equal(second.stopped, 0);
  }
  {
    // A superseded start that fails does not report over the newer one.
    let reject: (e: Error) => void = () => {};
    const session = createMicLevelSession(
      {
        mediaDevices: {
          enumerateDevices: async () => [],
          getUserMedia: () =>
            new Promise<MediaStream>((_, r) => {
              reject = r;
            }),
        },
        createContext: () => ({}) as never,
        requestFrame: () => 0,
        cancelFrame: () => {},
      },
      () => {},
    );
    const started = session.start("Mic A");
    await new Promise((r) => setTimeout(r, 0));
    session.stop();
    reject(new Error("NotAllowedError"));
    assert.equal(await started, "cancelled");
  }
  const meter = read("./MicLevelMeter.tsx");
  assert.match(
    meter,
    /useEffect\(\(\) => \(\) => sessionRef\.current\?\.stop\(\), \[\]\)/,
  );
  assert.match(meter, /disabled=\{state === "starting"\}/);
  ok("C3");
}

// C4: the wizard downloads, then selects, Parakeet; never twice, never in a preview.
{
  assert.equal(DICTATION_MODEL_ID, "parakeet-tdt-0.6b-v3");
  const base = {
    models: [{ id: DICTATION_MODEL_ID, is_downloaded: false }],
    currentModel: "",
    downloading: {},
    verifying: {},
    extracting: {},
  };
  const id = DICTATION_MODEL_ID;
  const downloaded = [{ id, is_downloaded: true }];
  assert.equal(modelAction(base), "download");
  assert.equal(modelAction({ ...base, models: downloaded }), "select");
  assert.equal(
    modelAction({ ...base, models: downloaded, currentModel: id }),
    "none",
  );
  assert.equal(modelAction({ ...base, downloading: { [id]: true } }), "none");
  assert.equal(
    modelAction({ ...base, models: downloaded, verifying: { [id]: true } }),
    "none",
  );
  assert.equal(
    modelAction({ ...base, models: downloaded, extracting: { [id]: true } }),
    "none",
  );
  assert.equal(modelAction({ ...base, models: [] }), "none");

  const wizard = read("./Onboarding.tsx");
  assert.match(wizard, /downloadModel\(DICTATION_MODEL_ID\)/);
  assert.match(wizard, /selectModel\(DICTATION_MODEL_ID\)/);
  const call = wizard.indexOf("downloadModel(DICTATION_MODEL_ID)");
  const effect = wizard.lastIndexOf("useEffect(() => {", call);
  assert.match(
    wizard.slice(effect, call),
    /if \(\s*preview \|\|/,
    "the download effect returns early in a preview",
  );
  ok("C4");
}

// C5: the footer line for every model state.
{
  const id = DICTATION_MODEL_ID;
  const base = {
    models: [{ id, is_downloaded: false }],
    currentModel: "",
    downloading: {},
    verifying: {},
    extracting: {},
    progress: {},
    error: null,
  };
  const downloaded = [{ id, is_downloaded: true }];
  assert.deepEqual(
    modelFooterState({
      ...base,
      downloading: { [id]: true },
      progress: { [id]: { percentage: 42 } },
    }),
    { kind: "downloading", percent: 42 },
  );
  assert.deepEqual(modelFooterState({ ...base, verifying: { [id]: true } }), {
    kind: "verifying",
  });
  assert.deepEqual(modelFooterState({ ...base, extracting: { [id]: true } }), {
    kind: "extracting",
  });
  assert.deepEqual(
    modelFooterState({ ...base, models: downloaded, currentModel: id }),
    { kind: "ready" },
  );
  assert.deepEqual(modelFooterState({ ...base, error: "network down" }), {
    kind: "failed",
  });
  assert.deepEqual(modelFooterState(base), { kind: "pending" });
  // C21: the wizard's own failed download or selection shows as failed.
  assert.equal(footerError(null, false), null);
  assert.equal(footerError(null, true), "failed");
  assert.equal(footerError("network down", false), "network down");
  assert.equal(footerError("network down", true), "network down");
  assert.deepEqual(
    modelFooterState({ ...base, error: footerError(null, true) }),
    { kind: "failed" },
  );

  const downloading = renderToStaticMarkup(
    <ModelFooter state={{ kind: "downloading", percent: 42 }} onRetry={noop} />,
  );
  assert.match(downloading, /role="progressbar"/);
  assert.match(downloading, /aria-valuenow="42"/);
  assert.ok(downloading.includes("onboarding.footer.downloading"));
  const ready = renderToStaticMarkup(
    <ModelFooter state={{ kind: "ready" }} onRetry={noop} />,
  );
  assert.ok(ready.includes("onboarding.footer.ready"));
  assert.match(ready, /class="[^"]*\bbg-ok\b/);
  const failed = renderToStaticMarkup(
    <ModelFooter state={{ kind: "failed" }} onRetry={noop} />,
  );
  assert.ok(failed.includes("onboarding.footer.retry"));
  ok("C5");
}

// C6: the shortcut is drawn from the current binding, with a test field, the
// shortcut and activation controls, and the shortcuts registered on entry.
{
  useSettingsStore.setState({
    settings: {
      bindings: {
        transcribe: {
          id: "transcribe",
          name: "Transcribe",
          description: "",
          default_binding: "ctrl+shift+space",
          current_binding: "ctrl+shift+space",
        },
      },
    } as never,
  });
  const html = renderToStaticMarkup(<ShortcutStep modelReady preview />);
  const keys = [...html.matchAll(/<kbd[^>]*>([^<]*)<\/kbd>/g)].map((m) => m[1]);
  assert.deepEqual(keys, ["Ctrl", "Shift", "Space"]);
  assert.match(
    html,
    /<textarea[^>]*aria-label="onboarding.shortcut.testLabel"/,
  );
  assert.ok(html.includes("onboarding.shortcut.instruction"));
  assert.ok(!html.includes("onboarding.shortcut.waitModel"));
  // A changed shortcut is drawn as it is, not as the default.
  useSettingsStore.setState({
    settings: {
      bindings: {
        transcribe: {
          id: "transcribe",
          name: "Transcribe",
          description: "",
          default_binding: "ctrl+shift+space",
          current_binding: "alt+f9",
        },
      },
    } as never,
  });
  const changed = renderToStaticMarkup(<ShortcutStep modelReady preview />);
  assert.deepEqual(
    [...changed.matchAll(/<kbd[^>]*>([^<]*)<\/kbd>/g)].map((m) => m[1]),
    ["Alt", "F9"],
  );
  const waiting = renderToStaticMarkup(
    <ShortcutStep modelReady={false} preview />,
  );
  assert.ok(waiting.includes("onboarding.shortcut.waitModel"));
  const step = read("./ShortcutStep.tsx");
  assert.match(step, /<ShortcutInput shortcutId="transcribe"/);
  assert.match(step, /<ShortcutActivationSetting/);
  assert.match(step, /commands\.initializeEnigo\(\)/);
  assert.match(step, /commands\.initializeShortcuts\(\)/);
  ok("C6");
}

// C7: the AI step says what leaves the machine, takes the Gemini key, can be skipped.
{
  const html = renderToStaticMarkup(<AiStep onFinish={noop} />);
  for (const key of [
    "onboarding.ai.privacy",
    "onboarding.ai.skip",
    "onboarding.ai.finish",
  ]) {
    assert.ok(html.includes(key), key);
  }
  const step = read("./AiStep.tsx");
  assert.match(step, /<ApiKeyField/);
  assert.match(step, /updatePostProcessApiKey\(GEMINI_PROVIDER_ID/);
  assert.match(step, /GEMINI_PROVIDER_ID = "gemini"/);
  assert.ok(pt.onboarding.ai.privacy.includes("só o texto"));
  assert.ok(pt.onboarding.ai.privacy.includes("nunca o áudio"));
  assert.ok(en.onboarding.ai.privacy.includes("only the text"));
  assert.ok(en.onboarding.ai.privacy.includes("never the audio"));
  ok("C7");
}

// C8: the wizard frame: progress, step title, model footer and navigation.
{
  const render = (step: "microphone" | "shortcut" | "ai") =>
    renderToStaticMarkup(
      <Onboarding preview initialStep={step} onFinish={noop} />,
    );
  for (const step of ONBOARDING_STEPS) {
    const html = render(step);
    assert.ok(html.includes("onboarding.progress"), `${step} progress`);
    assert.ok(html.includes(`onboarding.${step}.title`), `${step} title`);
    assert.match(html, /onboarding\.footer\./, `${step} footer`);
    assert.equal(
      html.includes("onboarding.nav.back"),
      step !== "microphone",
      `${step} back`,
    );
  }
  assert.ok(render("microphone").includes("onboarding.nav.next"));
  assert.ok(render("shortcut").includes("onboarding.nav.next"));
  assert.ok(!render("ai").includes("onboarding.nav.next"));
  ok("C8");
}

// C9: App opens the new wizard; the macOS step and the model cards are gone.
{
  for (const gone of [
    "AccessibilityOnboarding",
    "checkAccessibilityPermission",
    "onModelSelected",
    '"model"',
  ]) {
    assert.ok(!appTsx.includes(gone), `App.tsx still cites ${gone}`);
  }
  assert.match(
    appTsx,
    /<Onboarding onFinish=\{\(\) => setOnboardingStep\("done"\)\} \/>/,
  );
  assert.match(appTsx, /<Onboarding\s+steps=\{\["microphone"\]\}/);
  assert.match(appTsx, /useState<RailDestination>\("home"\)/);
  assert.ok(!exists("./AccessibilityOnboarding.tsx"));
  assert.ok(!exists("./ModelCard.tsx"));
  assert.ok(exists("../settings/models/ModelCard.tsx"));
  for (const file of readdirSync(new URL(".", import.meta.url))) {
    if (file.endsWith(".test.tsx")) continue;
    assert.ok(
      !read(`./${file}`).includes("ModelCard"),
      `${file} imports ModelCard`,
    );
  }
  ok("C9");
}

// C10: the debug preview opens the new wizard on a chosen step.
{
  const preview = read("../settings/debug/OnboardingPreview.tsx");
  assert.match(
    preview,
    /export type OnboardingPreviewStep = OnboardingStepId;/,
  );
  const html = renderToStaticMarkup(<OnboardingPreview onPreview={noop} />);
  for (const step of ONBOARDING_STEPS) {
    assert.ok(
      html.includes(`settings.debug.onboardingPreview.${step}Button`),
      `${step} preview button`,
    );
  }
  assert.match(
    appTsx,
    /<Onboarding[^>]*\bpreview\s+initialStep=\{onboardingPreview\}/,
  );
  ok("C10");
}

// C11: the new strings exist in pt and en; the old wizard's are gone.
{
  const present = [
    "onboarding.progress",
    ...["back", "next"].map((k) => `onboarding.nav.${k}`),
    ...[
      "pending",
      "downloading",
      "verifying",
      "extracting",
      "ready",
      "failed",
    ].map((k) => `onboarding.footer.${k}`),
    ...["title", "description", "test", "stop", "unavailable"].map(
      (k) => `onboarding.microphone.${k}`,
    ),
    ...["title", "step1", "step2", "step3", "openSettings", "waiting"].map(
      (k) => `onboarding.microphone.denied.${k}`,
    ),
    ...["title", "instruction", "testLabel", "testPlaceholder"].map(
      (k) => `onboarding.shortcut.${k}`,
    ),
    ...["title", "privacy", "keyLabel", "skip", "finish"].map(
      (k) => `onboarding.ai.${k}`,
    ),
    ...["microphoneButton", "shortcutButton", "aiButton"].map(
      (k) => `settings.debug.onboardingPreview.${k}`,
    ),
  ];
  const absent = [
    "onboarding.permissions",
    "onboarding.subtitle",
    "onboarding.existingModelsTitle",
    "onboarding.downloadModelsTitle",
    "onboarding.showAllModels",
    "onboarding.showFewerModels",
    "settings.debug.onboardingPreview.permissionsButton",
    "settings.debug.onboardingPreview.modelsButton",
  ];
  for (const [lang, locale] of [
    ["pt", pt],
    ["en", en],
  ] as const) {
    for (const key of present) {
      const value = at(locale, key);
      assert.ok(
        typeof value === "string" && value.length > 0,
        `${lang} ${key}`,
      );
    }
    for (const key of absent) {
      assert.equal(at(locale, key), undefined, `${lang} still has ${key}`);
    }
  }
  ok("C11");
}

// C12: only semantic colour in the wizard.
{
  for (const file of readdirSync(new URL(".", import.meta.url))) {
    if (file.endsWith(".test.tsx")) continue;
    const source = read(`./${file}`);
    for (const banned of [
      "logo-primary",
      "background-ui",
      "mid-gray",
      "emerald",
      "bg-white/",
    ]) {
      assert.ok(!source.includes(banned), `${file} cites ${banned}`);
    }
    assert.ok(!/#[0-9a-fA-F]{3,8}\b/.test(source), `${file} has a hex colour`);
  }
  assert.match(read("./MicrophoneStep.tsx"), /\btext-warn\b/);
  assert.match(read("./MicrophoneStep.tsx"), /\bborder-warn\//);
  assert.match(read("./ModelFooter.tsx"), /failed: "bg-danger"/);
  assert.match(read("./ModelFooter.tsx"), /"text-danger"/);
  ok("C12");
}

// C20: a returning user who only needs the microphone back keeps their model:
// the wizard neither downloads nor selects one and shows no model footer.
{
  assert.match(
    appTsx,
    /<Onboarding\s+steps=\{\["microphone"\]\}\s+prepareModel=\{false\}/,
  );
  const wizard = read("./Onboarding.tsx");
  const call = wizard.indexOf("downloadModel(DICTATION_MODEL_ID)");
  const effect = wizard.lastIndexOf("useEffect(() => {", call);
  assert.match(wizard.slice(effect, call), /!prepareModel \|\|/);
  const html = renderToStaticMarkup(
    <Onboarding
      steps={["microphone"]}
      prepareModel={false}
      preview
      onFinish={noop}
    />,
  );
  assert.ok(!html.includes("onboarding.footer."), "no model footer");
  assert.ok(
    !html.includes("onboarding.progress"),
    "no step counter for one step",
  );
  assert.ok(html.includes("onboarding.nav.done"));
  assert.ok(!html.includes("onboarding.nav.back"));
  const full = renderToStaticMarkup(<Onboarding preview onFinish={noop} />);
  assert.match(
    full,
    /onboarding\.footer\./,
    "the full wizard keeps the footer",
  );
  ok("C20");
}

// C21: a download or selection that fails without a store error still ends
// in the failed footer, and "try again" clears it.
{
  const wizard = read("./Onboarding.tsx");
  assert.match(wizard, /if \(!ok\) setActionFailed\(true\)/);
  assert.match(wizard, /error: footerError\(error, actionFailed\)/);
  const retry = wizard.slice(wizard.indexOf("const retry = () => {"));
  assert.match(retry.slice(0, retry.indexOf("};")), /setActionFailed\(false\)/);
  ok("C21");
}
