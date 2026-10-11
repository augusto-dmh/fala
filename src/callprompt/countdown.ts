// The prompt's countdown: 20 s that stop while the pointer is on the card (Granola's rule).
// Pure, so the rule is tested without a window.

export interface Countdown {
  totalMs: number;
  remainingMs: number;
}

export const startCountdown = (totalMs: number): Countdown => ({
  totalMs,
  remainingMs: totalMs,
});

/** `elapsedMs` later; nothing passes while `hovered`. */
export const advance = (
  countdown: Countdown,
  elapsedMs: number,
  hovered: boolean,
): Countdown =>
  hovered
    ? countdown
    : {
        ...countdown,
        remainingMs: Math.max(0, countdown.remainingMs - elapsedMs),
      };

export const expired = (countdown: Countdown): boolean =>
  countdown.remainingMs <= 0;

/** From 1 (just shown) to 0 (closes). */
export const fraction = (countdown: Countdown): number =>
  countdown.totalMs > 0 ? countdown.remainingMs / countdown.totalMs : 0;
