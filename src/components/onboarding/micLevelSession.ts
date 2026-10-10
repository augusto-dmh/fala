import { levelFromTimeDomain, pickInputDeviceId } from "./onboardingModel";

/** What the session needs from the browser, injectable so it can be tested. */
export interface MicLevelDeps {
  mediaDevices:
    | Pick<MediaDevices, "enumerateDevices" | "getUserMedia">
    | undefined;
  createContext: () => Pick<
    AudioContext,
    "close" | "createAnalyser" | "createMediaStreamSource"
  >;
  requestFrame: (tick: () => void) => number;
  cancelFrame: (handle: number) => void;
}

export type MicLevelStart = "listening" | "cancelled";

/**
 * One microphone test at a time. `stop()` releases the stream, the audio
 * context and the frame loop, and also cancels a start still waiting for
 * `getUserMedia`: a stream that arrives after it is stopped at once, so the
 * microphone never stays open behind an unmounted step or a double click.
 */
export const createMicLevelSession = (
  deps: MicLevelDeps,
  onLevel: (level: number) => void,
) => {
  let generation = 0;
  let stream: MediaStream | null = null;
  let context: ReturnType<MicLevelDeps["createContext"]> | null = null;
  let frame: number | null = null;

  const release = () => {
    if (frame !== null) deps.cancelFrame(frame);
    frame = null;
    stream?.getTracks().forEach((track) => track.stop());
    stream = null;
    void context?.close();
    context = null;
  };

  const stop = () => {
    generation += 1;
    release();
    onLevel(0);
  };

  /** Rejects with the browser's error when the microphone cannot be opened. */
  const start = async (
    microphone: string | null | undefined,
  ): Promise<MicLevelStart> => {
    stop();
    const mine = generation;
    if (!deps.mediaDevices?.getUserMedia) throw new Error("mediaDevices");
    const devices = await deps.mediaDevices.enumerateDevices().catch(() => []);
    if (mine !== generation) return "cancelled";
    const deviceId = pickInputDeviceId(devices, microphone);
    let opened: MediaStream;
    try {
      opened = await deps.mediaDevices.getUserMedia({
        audio: deviceId ? { deviceId: { exact: deviceId } } : true,
      });
    } catch (error) {
      // A superseded start must not report its failure over the newer one.
      if (mine !== generation) return "cancelled";
      throw error;
    }
    if (mine !== generation) {
      opened.getTracks().forEach((track) => track.stop());
      return "cancelled";
    }
    stream = opened;
    context = deps.createContext();
    const analyser = context.createAnalyser();
    analyser.fftSize = 1024;
    context.createMediaStreamSource(opened).connect(analyser);
    const buffer = new Uint8Array(analyser.fftSize);
    const tick = () => {
      analyser.getByteTimeDomainData(buffer);
      onLevel(levelFromTimeDomain(buffer));
      frame = deps.requestFrame(tick);
    };
    tick();
    return "listening";
  };

  return { start, stop };
};
