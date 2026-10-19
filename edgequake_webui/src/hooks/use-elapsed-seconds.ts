"use client";

import { elapsedSeconds } from "@/lib/query/pending-answer";
import { useEffect, useState } from "react";

/** Whole seconds since `startedAt`, re-rendering once a second while `active`. */
export function useElapsedSeconds(startedAt: number | undefined, active: boolean): number {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (!active) return;
    const id = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, [active]);

  return active ? elapsedSeconds(startedAt, now) : 0;
}
