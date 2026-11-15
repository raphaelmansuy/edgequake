/**
 * Sanitize KaTeX HTML before it is injected.
 *
 * Equation and math tokens skip the markdown `sanitizeHtml` gate on purpose:
 * that allowlist would delete `<equation>` and the formula, and it would also
 * drop the `<svg>` radicals and inline `style` metrics KaTeX needs. The HTML
 * KaTeX emits is still untrusted (a document chunk, a model answer, or a
 * pasted query), and with `trust: true` `\href{javascript:…}` becomes a live
 * anchor. This allowlist is the gate on that HTML.
 *
 * parse5 is the same WHATWG parser on the server and in the browser, so the
 * string SSR renders matches the string the client hydrates.
 */
import {
  parseFragment,
  serialize,
  type DefaultTreeAdapterTypes,
} from "parse5";

type ChildNode = DefaultTreeAdapterTypes.ChildNode;
type ParentNode = DefaultTreeAdapterTypes.ParentNode;
type Element = DefaultTreeAdapterTypes.Element;

/** Tags KaTeX's `output: "html"` mode actually emits. */
const ALLOWED_TAGS = new Set(["span", "svg", "path", "line"]);

/**
 * Attributes those tags need for layout. `href` and `src` are absent on
 * purpose: a math expression must not become a link or an image request.
 */
const ALLOWED_ATTRS = new Set([
  "aria-hidden",
  "class",
  "d",
  "fill",
  "height",
  "preserveaspectratio",
  "stroke",
  "stroke-linecap",
  "stroke-linejoin",
  "stroke-width",
  "style",
  "viewbox",
  "width",
  "x1",
  "x2",
  "xmlns",
  "y1",
  "y2",
]);

const SVG_NS = "http://www.w3.org/2000/svg";

/**
 * Drop the element and everything inside it. Unwrapping these would pull
 * nested HTML (`<foreignObject>`, `<math>`, `<style>`) back into the tree.
 */
const DROP_WITH_DESCENDANTS = new Set([
  "annotation",
  "embed",
  "foreignobject",
  "iframe",
  "math",
  "noembed",
  "noframes",
  "noscript",
  "object",
  "script",
  "style",
  "template",
  "textarea",
  "title",
  "xmp",
]);

/** CSS KaTeX never needs. `url()` is also a data-exfil sink; math has no images. */
const UNSAFE_STYLE =
  /url\s*\(|expression\s*\(|javascript\s*:|vbscript\s*:|data\s*:|@import|behavior\s*:|-moz-binding|mocha\s*:/i;

const UNSAFE_URL = /javascript\s*:|vbscript\s*:|data\s*:/i;

function isElement(node: ChildNode): node is Element {
  return (
    node.nodeName !== "#text" &&
    node.nodeName !== "#comment" &&
    node.nodeName !== "#documentType"
  );
}

function keepAttribute(tag: string, name: string, value: string): boolean {
  const attr = name.toLowerCase();
  if (attr.startsWith("on")) return false;
  if (!ALLOWED_ATTRS.has(attr)) return false;
  if (attr === "xmlns") return tag === "svg" && value === SVG_NS;
  if (attr === "style" && UNSAFE_STYLE.test(value)) return false;
  if (UNSAFE_URL.test(value)) return false;
  return true;
}

function sanitizeChildren(parent: ParentNode): void {
  const next: ChildNode[] = [];
  for (const child of parent.childNodes) {
    if (!isElement(child)) {
      if (child.nodeName === "#text") next.push(child);
      continue;
    }
    const tag = child.tagName.toLowerCase();
    if (DROP_WITH_DESCENDANTS.has(tag)) continue;
    if (!ALLOWED_TAGS.has(tag)) {
      sanitizeChildren(child);
      for (const grand of child.childNodes) {
        grand.parentNode = parent;
        next.push(grand);
      }
      continue;
    }
    child.attrs = child.attrs.filter((attr) =>
      keepAttribute(tag, attr.name, attr.value),
    );
    sanitizeChildren(child);
    child.parentNode = parent;
    next.push(child);
  }
  parent.childNodes = next;
}

/** Return KaTeX markup that cannot carry a `javascript:` URL or an event handler. */
export function sanitizeKatexHtml(html: string): string {
  const fragment = parseFragment(html);
  sanitizeChildren(fragment);
  return serialize(fragment);
}
