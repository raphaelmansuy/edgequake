/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it } from "vitest";
import { createReturnFocus } from "../return-focus";

afterEach(() => {
  document.body.innerHTML = "";
});

function evt() {
  return new Event("focus", { cancelable: true });
}

describe("createReturnFocus", () => {
  it("restores the remembered opener and prevents Radix default", () => {
    document.body.innerHTML = '<button id="a">a</button>';
    const a = document.getElementById("a") as HTMLButtonElement;
    a.focus();
    const rf = createReturnFocus();
    rf.remember();
    a.blur();
    const e = evt();
    rf.restore(e);
    expect(document.activeElement).toBe(a);
    expect(e.defaultPrevented).toBe(true);
  });

  it("keeps the original opener when remembered from inside a dialog", () => {
    document.body.innerHTML =
      '<button id="a">a</button><div role="dialog"><input id="i" /></div>';
    const a = document.getElementById("a") as HTMLButtonElement;
    a.focus();
    const rf = createReturnFocus();
    rf.remember();
    (document.getElementById("i") as HTMLInputElement).focus();
    rf.remember();
    rf.restore(evt());
    expect(document.activeElement).toBe(a);
  });

  it("falls through when nothing (or a detached node) was remembered", () => {
    const rf = createReturnFocus();
    const e = evt();
    rf.restore(e);
    expect(e.defaultPrevented).toBe(false);
  });
});
