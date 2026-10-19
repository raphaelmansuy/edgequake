/**
 * SPEC-168 — two-key "leader" chords (`g` then `d`), Linear/GitHub style.
 *
 * Pure state machine so it is unit-testable without a DOM. Chords avoid the
 * browser-reserved ⌘/Ctrl combinations (⌘D bookmark, ⌘G find-next, ⌘⇧Q quit).
 */

export const CHORD_LEADER = "g";
export const CHORD_TIMEOUT_MS = 1200;

export type ChordResult =
  | { kind: "none" }
  | { kind: "armed" }
  | { kind: "complete"; key: string };

export interface ChordTracker {
  /** Feed a plain key press. */
  press(key: string, now: number): ChordResult;
  /** Drop any armed leader (focus change, Escape, modal open). */
  reset(): void;
  readonly armed: boolean;
}

export function createChordTracker(
  leader: string = CHORD_LEADER,
  timeoutMs: number = CHORD_TIMEOUT_MS,
): ChordTracker {
  let armedAt: number | null = null;
  return {
    press(key, now) {
      const k = key.toLowerCase();
      if (armedAt !== null) {
        const fresh = now - armedAt <= timeoutMs;
        armedAt = null;
        // A fresh second key always completes (including `g g`).
        if (fresh) return { kind: "complete", key: k };
      }
      if (k === leader) {
        armedAt = now;
        return { kind: "armed" };
      }
      return { kind: "none" };
    },
    reset() {
      armedAt = null;
    },
    get armed() {
      return armedAt !== null;
    },
  };
}
