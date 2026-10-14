import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { commands, type BindingSlot } from "@/bindings";
import {
  getKeyName,
  normalizeKey,
  type OSType,
} from "../../lib/utils/keyboard";
import { escapeCancelsRecording } from "./shortcutsModel";

interface FalaKeysEvent {
  modifiers: string[];
  key: string | null;
  is_key_down: boolean;
  hotkey_string: string;
}

export interface RecordingTarget {
  actionId: string;
  slot: BindingSlot;
}

const MODIFIERS = [
  "ctrl",
  "control",
  "shift",
  "alt",
  "option",
  "meta",
  "command",
  "cmd",
  "super",
  "win",
  "windows",
];

/**
 * Records one key combination for a slot of the shortcuts dialog, with either keyboard backend.
 * Every registered shortcut is suspended while it records, so none fires or swallows the keys.
 * A combination commits when its keys are released; Esc cancels (except on the cancel row).
 */
export const useComboRecorder = (
  implementation: string | undefined,
  osType: OSType,
  onCommit: (target: RecordingTarget, combo: string) => Promise<void>,
) => {
  const [target, setTarget] = useState<RecordingTarget | null>(null);
  const [keys, setKeys] = useState("");
  const keyedRef = useRef("");
  const modifierOnlyRef = useRef("");
  const pressedRef = useRef<string[]>([]);
  const recordedRef = useRef<string[]>([]);
  const falaKeys = implementation === "fala_keys";

  const end = useCallback(async () => {
    setTarget(null);
    setKeys("");
    keyedRef.current = "";
    modifierOnlyRef.current = "";
    pressedRef.current = [];
    recordedRef.current = [];
    if (falaKeys) {
      await commands.stopFalaKeysRecording().catch(console.error);
    } else {
      await commands.resumeAllBindings().catch(console.error);
    }
  }, [falaKeys]);

  const start = useCallback(
    async (next: RecordingTarget): Promise<string | null> => {
      if (target) await end();
      if (falaKeys) {
        const result = await commands.startFalaKeysRecording(next.actionId);
        if (result.status === "error") return String(result.error);
      } else {
        await commands.suspendAllBindings().catch(console.error);
      }
      setTarget(next);
      return null;
    },
    [end, falaKeys, target],
  );

  const targetRef = useRef(target);
  targetRef.current = target;

  useEffect(() => {
    if (!target) return;
    let active = true;
    const commit = async (combo: string) => {
      if (!active || !combo) return;
      active = false;
      await onCommit(target, combo);
      // A recording started meanwhile on another row is not this one's to end.
      if (targetRef.current === target) await end();
    };
    const cancel = () => {
      active = false;
      void end();
    };
    const cancels = escapeCancelsRecording(target.actionId);
    const cleanups: (() => void)[] = [];
    const onKey = (
      type: "keydown" | "keyup",
      handle: (e: KeyboardEvent) => void,
    ) => {
      // Captured before the page: Space must not press the focused button, nor Esc close the dialog.
      const listener = (e: KeyboardEvent) => {
        e.preventDefault();
        e.stopPropagation();
        if (active) handle(e);
      };
      window.addEventListener(type, listener, true);
      cleanups.push(() => window.removeEventListener(type, listener, true));
    };

    if (falaKeys) {
      // The backend records the keys; the page only cancels on Esc.
      onKey("keydown", (e) => {
        if (e.key === "Escape" && cancels) cancel();
      });
      onKey("keyup", () => {});
      void listen<FalaKeysEvent>("fala-keys-event", (event) => {
        if (!active) return;
        const { hotkey_string, is_key_down, key, modifiers } = event.payload;
        if (key === "escape" && cancels) return;
        if (is_key_down && hotkey_string) {
          if (key) keyedRef.current = hotkey_string;
          else modifierOnlyRef.current = hotkey_string;
          setKeys(hotkey_string);
        } else if (!is_key_down && key) {
          // Only a key pressed during this recording: never the Enter that opened it.
          void commit(keyedRef.current);
        } else if (
          !is_key_down &&
          modifiers.length === 0 &&
          !keyedRef.current
        ) {
          void commit(modifierOnlyRef.current);
        }
      }).then((stop) => {
        if (active) cleanups.push(stop);
        else stop();
      });
    } else {
      onKey("keydown", (e) => {
        if (e.repeat) return;
        if (e.key === "Escape" && cancels && recordedRef.current.length === 0) {
          cancel();
          return;
        }
        const key = normalizeKey(getKeyName(e, osType));
        if (!pressedRef.current.includes(key)) pressedRef.current.push(key);
        if (!recordedRef.current.includes(key)) recordedRef.current.push(key);
        setKeys(recordedRef.current.join("+"));
      });
      onKey("keyup", (e) => {
        const key = normalizeKey(getKeyName(e, osType));
        pressedRef.current = pressedRef.current.filter((k) => k !== key);
        if (pressedRef.current.length > 0 || recordedRef.current.length === 0) {
          return;
        }
        const isModifier = (k: string) => MODIFIERS.includes(k.toLowerCase());
        const sorted = [...recordedRef.current].sort(
          (a, b) => Number(isModifier(b)) - Number(isModifier(a)),
        );
        void commit(sorted.join("+"));
      });
    }
    return () => {
      active = false;
      cleanups.forEach((cleanup) => cleanup());
    };
  }, [end, falaKeys, onCommit, osType, target]);

  // Never leave the shortcuts suspended when the dialog goes away mid-recording.
  useEffect(
    () => () => {
      if (targetRef.current) void end();
    },
    [end],
  );

  return { target, keys, start, cancel: end };
};
