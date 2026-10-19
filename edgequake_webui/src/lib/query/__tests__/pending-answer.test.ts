import { describe, expect, it } from "vitest";
import {
  activeStepIndex,
  elapsedSeconds,
  stepStates,
  waitHint,
} from "../pending-answer";

describe("pending answer model", () => {
  it("starts on the first step when the stage is not known yet", () => {
    expect(activeStepIndex(null)).toBe(0);
    expect(stepStates(null)).toEqual(["active", "todo", "todo", "todo"]);
  });

  it("marks earlier steps done and later steps todo", () => {
    expect(stepStates("thinking")).toEqual(["done", "done", "active", "todo"]);
    expect(stepStates("generating")).toEqual(["done", "done", "done", "active"]);
  });

  it("does not claim progress for stopped or failed streams", () => {
    expect(stepStates("stopped")).toEqual(["todo", "todo", "todo", "todo"]);
    expect(stepStates("error")).toEqual(["todo", "todo", "todo", "todo"]);
  });

  it("escalates the hint as the wait grows", () => {
    expect(waitHint(0)).toBe("none");
    expect(waitHint(7)).toBe("none");
    expect(waitHint(8)).toBe("working");
    expect(waitHint(19)).toBe("working");
    expect(waitHint(20)).toBe("slow");
  });

  it("computes whole elapsed seconds and never goes negative", () => {
    expect(elapsedSeconds(undefined, 5000)).toBe(0);
    expect(elapsedSeconds(1000, 4999)).toBe(3);
    expect(elapsedSeconds(9000, 1000)).toBe(0);
  });
});
