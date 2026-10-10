/**
 * Reasoning panel UI state machine (SPEC-155).
 * Pure transitions — no React effects syncing booleans.
 *
 * States:
 * - collapsed: default / post-answer (auto)
 * - live: streaming an open <think> block (auto-expanded)
 * - expanded: user forced open
 * - collapsed_locked: user forced closed (ignore auto-expand)
 */
export type ReasoningPanelPhase =
  | "collapsed"
  | "live"
  | "expanded"
  | "collapsed_locked";

export type ReasoningPanelEvent =
  | { type: "stream_live" }
  | { type: "stream_answer" }
  | { type: "stream_idle" }
  | { type: "user_toggle" };

export interface ReasoningPanelState {
  phase: ReasoningPanelPhase;
}

export function createReasoningPanelState(): ReasoningPanelState {
  return { phase: "collapsed" };
}

export function reduceReasoningPanel(
  state: ReasoningPanelState,
  event: ReasoningPanelEvent,
): ReasoningPanelState {
  switch (event.type) {
    case "stream_live":
      if (state.phase === "live") return state;
      // User lock wins until they toggle again
      if (state.phase === "collapsed_locked" || state.phase === "expanded") {
        return state;
      }
      return { phase: "live" };

    case "stream_answer":
      // Keep the live panel until the stream ends so the answer is not shoved up.
      if (state.phase === "live") return state;
      if (state.phase === "expanded" || state.phase === "collapsed_locked") {
        return state;
      }
      if (state.phase === "collapsed") return state;
      return { phase: "collapsed" };

    case "stream_idle":
      if (state.phase === "live") return { phase: "collapsed" };
      return state;

    case "user_toggle": {
      switch (state.phase) {
        case "collapsed":
        case "collapsed_locked":
        case "live":
          return { phase: "expanded" };
        case "expanded":
          return { phase: "collapsed_locked" };
        default:
          return state;
      }
    }

    default:
      return state;
  }
}

export function isReasoningPanelExpanded(phase: ReasoningPanelPhase): boolean {
  return phase === "live" || phase === "expanded";
}

export function isReasoningPanelLive(phase: ReasoningPanelPhase): boolean {
  return phase === "live";
}

/**
 * Derive the next panel event from stream/CoT facts (pure).
 * Callers reduce with this event when inputs change — no useEffect boolean sync.
 */
export function reasoningEventFromStream(opts: {
  isStreaming: boolean;
  cotOpen: boolean;
  hasResponseText: boolean;
}): ReasoningPanelEvent {
  if (opts.isStreaming && opts.cotOpen && !opts.hasResponseText) {
    return { type: "stream_live" };
  }
  if (opts.hasResponseText && opts.isStreaming) {
    return { type: "stream_answer" };
  }
  if (opts.hasResponseText && !opts.isStreaming) {
    return { type: "stream_idle" };
  }
  if (!opts.isStreaming) {
    return { type: "stream_idle" };
  }
  return { type: "stream_idle" };
}
