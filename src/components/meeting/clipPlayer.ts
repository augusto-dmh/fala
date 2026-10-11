import { invoke } from "@tauri-apps/api/core";

/**
 * The WAV of a transcript line with its margins (`meeting_clip`). The command answers with raw
 * bytes, which tauri-specta cannot type, so it is not in `bindings.ts`; on error the promise
 * rejects with the `MeetingError` object.
 */
export function fetchClip(
  id: string,
  t0Ms: number,
  t1Ms: number,
): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>("meeting_clip", { id, t0Ms, t1Ms });
}

/** The browser pieces the player uses, injectable for the tests. */
export interface ClipDeps {
  fetch: typeof fetchClip;
  objectUrl: (blob: Blob) => string;
  revoke: (url: string) => void;
  audio: (url: string) => ClipAudio;
}

export interface ClipAudio {
  play: () => Promise<void>;
  pause: () => void;
  onEnded: (listener: () => void) => void;
}

const browser: ClipDeps = {
  fetch: fetchClip,
  objectUrl: (blob) => URL.createObjectURL(blob),
  revoke: (url) => URL.revokeObjectURL(url),
  audio: (url) => {
    const element = new Audio(url);
    return {
      play: () => element.play(),
      pause: () => element.pause(),
      onEnded: (listener) => element.addEventListener("ended", listener),
    };
  },
};

/**
 * Plays one clip at a time from a `Blob` URL: a new `play` stops the current one, and a clip
 * that arrives after a newer request is dropped. No file is written and the asset protocol is
 * not used.
 */
export class ClipPlayer {
  private token = 0;
  private current: { url: string; audio: ClipAudio } | null = null;

  constructor(
    private onChange: (playing: number | null) => void,
    private deps: ClipDeps = browser,
  ) {}

  /** Plays `[t0, t1]` of session `id`; `key` names what is playing (the segment `seq`). */
  async play(id: string, key: number, t0Ms: number, t1Ms: number) {
    this.stop();
    const token = this.token;
    this.onChange(key);
    try {
      const bytes = await this.deps.fetch(id, t0Ms, t1Ms);
      if (token !== this.token) return;
      const url = this.deps.objectUrl(new Blob([bytes], { type: "audio/wav" }));
      const audio = this.deps.audio(url);
      this.current = { url, audio };
      audio.onEnded(() => {
        if (token === this.token) this.stop();
      });
      await audio.play();
    } catch (error) {
      // A request overtaken by a newer one fails silently.
      if (token !== this.token) return;
      this.stop();
      throw error;
    }
  }

  stop() {
    this.token += 1;
    if (this.current) {
      this.current.audio.pause();
      this.deps.revoke(this.current.url);
      this.current = null;
    }
    this.onChange(null);
  }
}
