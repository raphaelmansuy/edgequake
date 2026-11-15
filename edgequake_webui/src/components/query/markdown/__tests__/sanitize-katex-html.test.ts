import { JSDOM } from "jsdom";
import { describe, expect, it } from "vitest";
import { sanitizeKatexHtml } from "../utils/sanitize-katex-html";

function expectInert(html: string): void {
  const doc = new JSDOM(`<!DOCTYPE html><body>${html}</body>`).window.document;
  expect(doc.querySelector("a, img, script, iframe, object, embed, foreignObject")).toBeNull();
  for (const el of doc.body.querySelectorAll("*")) {
    for (const name of el.getAttributeNames()) {
      expect(name.toLowerCase().startsWith("on")).toBe(false);
    }
    expect(el.getAttribute("href")).toBeNull();
    expect(el.getAttribute("src")).toBeNull();
  }
}

describe("sanitizeKatexHtml", () => {
  it.each([
    '<a href="javascript:alert(1)">click</a>',
    '<a href=" javascript:alert(1)">click</a>',
    '<a href="JAVASCRIPT:alert(1)">click</a>',
    '<img src=x onerror=alert(1)>',
    '<span onclick="alert(1)">x</span>',
    '<span style="background:url(javascript:alert(1))">x</span>',
    '<span style="background:url(&#106;avascript:alert(1))">x</span>',
    '<span style="width:expression(alert(1))">x</span>',
    "<svg><script>alert(1)</script><path d=\"M0 0\"></path></svg>",
    '<svg><foreignObject><iframe src="javascript:alert(1)"></iframe></foreignObject></svg>',
    '<svg><a href="javascript:alert(1)"><text>x</text></a></svg>',
    '<math><mtext><img src=x onerror=alert(1)></mtext></math>',
    "<!--<script>alert(1)</script>--><span class=\"katex\">ok</span>",
    '<span class="katex"><span title=">" onclick="alert(1)">x</span></span>',
  ])("neutralises %s", (payload) => {
    const clean = sanitizeKatexHtml(payload);
    expectInert(clean);
    expect(clean.toLowerCase()).not.toContain("<script");
    expect(clean.toLowerCase()).not.toContain("javascript:");
  });

  it("keeps a layout span and is stable when run twice", () => {
    const raw =
      '<span class="katex"><span class="katex-strut" style="height:1em;vertical-align:-0.25em;"></span></span>';
    const once = sanitizeKatexHtml(raw);
    expect(once).toContain('class="katex"');
    expect(once).toContain("height:1em");
    expect(sanitizeKatexHtml(once)).toBe(once);
  });
});