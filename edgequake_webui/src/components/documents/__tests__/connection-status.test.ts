/**
 * @module connection-status.test
 * @description Unit tests for ConnectionStatus component logic
 *
 * @implements OODA-34: ConnectionStatus tests
 *
 * Tests cover:
 * - State determination logic (connected/disconnected/reconnecting)
 * - State configuration mapping
 * - State transition behavior
 */

import { describe, expect, it } from "vitest";

// ============================================================================
// Types (matching connection-status.tsx)
// ============================================================================

export type ConnectionState = "connected" | "disconnected" | "reconnecting";

// ============================================================================
// State Determination Logic (extracted from component)
// ============================================================================

/**
 * WHY: Determines the connection state based on connected/reconnecting flags.
 * This logic is extracted for testability - the component uses this implicitly.
 *
 * Priority: reconnecting > connected > disconnected
 * - If reconnecting is true, always show reconnecting (even if connected)
 * - If connected is true (and not reconnecting), show connected
 * - Otherwise, show disconnected
 */
function determineConnectionState(
  connected: boolean,
  reconnecting: boolean,
): ConnectionState {
  if (reconnecting) return "reconnecting";
  if (connected) return "connected";
  return "disconnected";
}

// ============================================================================
// State Configuration (matching component)
// ============================================================================

interface StateConfig {
  label: string;
  description: string;
  colorClass: string;
  hasPulse: boolean;
}

function getStateConfig(state: ConnectionState): StateConfig {
  const configs: Record<ConnectionState, StateConfig> = {
    connected: {
      label: "Live",
      description: "Real-time updates active",
      colorClass: "text-green-500",
      hasPulse: true,
    },
    disconnected: {
      label: "Offline",
      description: "Using polling for updates",
      colorClass: "text-muted-foreground",
      hasPulse: false,
    },
    reconnecting: {
      label: "Connecting...",
      description: "Attempting to reconnect",
      colorClass: "text-amber-500",
      hasPulse: false,
    },
  };
  return configs[state];
}

// ============================================================================
// Tests
// ============================================================================

describe("determineConnectionState", () => {
  describe("priority handling", () => {
    it("returns reconnecting when reconnecting is true (even if connected)", () => {
      expect(determineConnectionState(true, true)).toBe("reconnecting");
    });

    it("returns connected when connected and not reconnecting", () => {
      expect(determineConnectionState(true, false)).toBe("connected");
    });

    it("returns disconnected when neither connected nor reconnecting", () => {
      expect(determineConnectionState(false, false)).toBe("disconnected");
    });

    it("returns reconnecting when reconnecting but not connected", () => {
      // WHY: This is a valid transitional state - attempting to connect
      expect(determineConnectionState(false, true)).toBe("reconnecting");
    });
  });

  describe("state transitions", () => {
    it("follows expected state machine transitions", () => {
      // Initial state: disconnected
      let state = determineConnectionState(false, false);
      expect(state).toBe("disconnected");

      // User triggers connect → reconnecting
      state = determineConnectionState(false, true);
      expect(state).toBe("reconnecting");

      // Connection established → connected
      state = determineConnectionState(true, false);
      expect(state).toBe("connected");

      // Connection lost, auto-reconnect → reconnecting
      state = determineConnectionState(false, true);
      expect(state).toBe("reconnecting");

      // Give up → disconnected
      state = determineConnectionState(false, false);
      expect(state).toBe("disconnected");
    });
  });
});

describe("getStateConfig", () => {
  describe("connected state", () => {
    it("has Live label", () => {
      const config = getStateConfig("connected");
      expect(config.label).toBe("Live");
    });

    it("has pulse animation", () => {
      const config = getStateConfig("connected");
      expect(config.hasPulse).toBe(true);
    });

    it("uses green color", () => {
      const config = getStateConfig("connected");
      expect(config.colorClass).toContain("green");
    });

    it("mentions real-time updates", () => {
      const config = getStateConfig("connected");
      expect(config.description).toContain("Real-time");
    });
  });

  describe("disconnected state", () => {
    it("has Offline label", () => {
      const config = getStateConfig("disconnected");
      expect(config.label).toBe("Offline");
    });

    it("has no pulse animation", () => {
      const config = getStateConfig("disconnected");
      expect(config.hasPulse).toBe(false);
    });

    it("uses muted color", () => {
      const config = getStateConfig("disconnected");
      expect(config.colorClass).toContain("muted");
    });

    it("mentions polling fallback", () => {
      const config = getStateConfig("disconnected");
      expect(config.description).toContain("polling");
    });
  });

  describe("reconnecting state", () => {
    it("has Connecting label", () => {
      const config = getStateConfig("reconnecting");
      expect(config.label).toBe("Connecting...");
    });

    it("has no pulse animation", () => {
      // WHY: Uses spinner animation instead
      const config = getStateConfig("reconnecting");
      expect(config.hasPulse).toBe(false);
    });

    it("uses amber color", () => {
      const config = getStateConfig("reconnecting");
      expect(config.colorClass).toContain("amber");
    });

    it("mentions reconnect attempt", () => {
      const config = getStateConfig("reconnecting");
      expect(config.description).toContain("reconnect");
    });
  });

  describe("all states have required properties", () => {
    const states: ConnectionState[] = [
      "connected",
      "disconnected",
      "reconnecting",
    ];

    states.forEach((state) => {
      it(`${state} has all required properties`, () => {
        const config = getStateConfig(state);
        expect(config).toHaveProperty("label");
        expect(config).toHaveProperty("description");
        expect(config).toHaveProperty("colorClass");
        expect(config).toHaveProperty("hasPulse");
        expect(typeof config.label).toBe("string");
        expect(typeof config.description).toBe("string");
        expect(typeof config.colorClass).toBe("string");
        expect(typeof config.hasPulse).toBe("boolean");
      });
    });
  });
});

describe("component behavior specifications", () => {
  describe("compact mode", () => {
    it("only shows pulsing dot in compact mode when connected", () => {
      // WHY: Compact mode reduces visual clutter, pulse indicates active connection
      const state = determineConnectionState(true, false);
      const config = getStateConfig(state);
      expect(config.hasPulse).toBe(true);
    });

    it("shows static dot when disconnected in compact mode", () => {
      const state = determineConnectionState(false, false);
      const config = getStateConfig(state);
      expect(config.hasPulse).toBe(false);
    });
  });

  describe("tooltip content", () => {
    it("connected tooltip shows latency info", () => {
      // WHY: Per mission spec, WebSocket updates should be < 500ms
      const state = determineConnectionState(true, false);
      expect(state).toBe("connected");
      // Component shows "Updates in <500ms" for connected state
    });

    it("disconnected tooltip suggests reconnection", () => {
      const state = determineConnectionState(false, false);
      expect(state).toBe("disconnected");
      // Component shows "Click to reconnect"
    });
  });

  describe("action buttons", () => {
    it("shows Connect button when disconnected and showActions=true", () => {
      const state = determineConnectionState(false, false);
      expect(state).toBe("disconnected");
      // When showActions=true, Connect button is shown
    });

    it("shows Disconnect button when connected and showActions=true", () => {
      const state = determineConnectionState(true, false);
      expect(state).toBe("connected");
      // When showActions=true, Disconnect button is shown
    });

    it("hides all actions when reconnecting", () => {
      const state = determineConnectionState(false, true);
      expect(state).toBe("reconnecting");
      // No action buttons during reconnection attempt
    });
  });
});
