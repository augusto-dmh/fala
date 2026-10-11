/** Timer functions, injectable so a test can drive the clock. */
export interface Timers {
  set: (fn: () => void, ms: number) => unknown;
  clear: (handle: unknown) => void;
}

const realTimers: Timers = {
  set: (fn, ms) => setTimeout(fn, ms),
  clear: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>),
};

/** How long after the last keystroke the typed notes are written to disk. */
export const SAVE_AFTER_MS = 2000;

/**
 * Saves the typed notes at most two seconds after the last change, again on `flush` (blur,
 * stop), and never twice with the same text: what was typed survives a crash a few seconds
 * later, like the audio does.
 */
export class AnnotationSaver {
  private pending: string | null = null;
  private handle: unknown = null;

  constructor(
    private readonly save: (text: string) => void,
    private saved: string,
    private readonly timers: Timers = realTimers,
  ) {}

  change(text: string): void {
    this.pending = text;
    if (this.handle !== null) this.timers.clear(this.handle);
    this.handle = this.timers.set(() => this.flush(), SAVE_AFTER_MS);
  }

  flush(): void {
    if (this.handle !== null) {
      this.timers.clear(this.handle);
      this.handle = null;
    }
    if (this.pending === null || this.pending === this.saved) return;
    this.saved = this.pending;
    this.pending = null;
    this.save(this.saved);
  }
}
