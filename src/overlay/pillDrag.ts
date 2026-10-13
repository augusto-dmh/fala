/** Movement past which a press on the pill is a drag, not a click (Granola's nub: 3 px). */
export const DRAG_THRESHOLD_PX = 3;

export function passedDragThreshold(dx: number, dy: number): boolean {
  return Math.hypot(dx, dy) > DRAG_THRESHOLD_PX;
}

interface PointerLike {
  button: number;
  buttons: number;
  screenX: number;
  screenY: number;
}

interface ClickLike {
  stopPropagation(): void;
  preventDefault(): void;
}

/**
 * Pointer handlers that turn a press moved past the threshold into a drag of the pill. The
 * backend moves the window (`start_pill_drag`) until the button comes up; the click that ends a
 * drag does not open anything.
 */
export function createPillDrag(onStart: () => void) {
  let origin: { x: number; y: number } | null = null;
  let dragged = false;
  return {
    onPointerDown(event: PointerLike) {
      dragged = false;
      origin =
        event.button === 0 ? { x: event.screenX, y: event.screenY } : null;
    },
    onPointerMove(event: PointerLike) {
      if (!origin) return;
      if ((event.buttons & 1) === 0) {
        origin = null;
        return;
      }
      const dx = event.screenX - origin.x;
      const dy = event.screenY - origin.y;
      if (passedDragThreshold(dx, dy)) {
        origin = null;
        dragged = true;
        onStart();
      }
    },
    onPointerUp() {
      origin = null;
    },
    onClickCapture(event: ClickLike) {
      if (!dragged) return;
      dragged = false;
      event.stopPropagation();
      event.preventDefault();
    },
  };
}
