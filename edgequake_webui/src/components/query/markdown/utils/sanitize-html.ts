/**
 * HTML Sanitization Utility
 *
 * Provides safe HTML sanitization using DOMPurify for markdown content.
 * Configured with strict settings appropriate for LLM-generated content.
 */
import DOMPurify from "dompurify";

// Flag to track if DOMPurify is available
let isInitialized = false;
let purify: typeof DOMPurify | null = null;

// DOMPurify config type
type PurifyConfig = {
  ALLOWED_TAGS?: string[];
  ALLOWED_ATTR?: string[];
  ALLOWED_URI_REGEXP?: RegExp;
  KEEP_CONTENT?: boolean;
  RETURN_DOM?: boolean;
  RETURN_DOM_FRAGMENT?: boolean;
  RETURN_TRUSTED_TYPE?: boolean;
  FORCE_BODY?: boolean;
  ADD_DATA_URI_TAGS?: string[];
  CUSTOM_ELEMENT_HANDLING?: {
    tagNameCheck: null | RegExp | ((tagName: string) => boolean);
    attributeNameCheck: null | RegExp | ((attrName: string) => boolean);
    allowCustomizedBuiltInElements?: boolean;
  };
};

/**
 * Initialize DOMPurify (only works in browser)
 */
function initializePurify(): typeof DOMPurify | null {
  if (typeof window === "undefined") {
    return null;
  }

  if (!isInitialized) {
    purify = DOMPurify;

    // Add hooks for additional security
    purify.addHook("afterSanitizeAttributes", (node) => {
      // Add rel="noopener noreferrer" to all links
      if (node.tagName === "A") {
        node.setAttribute("rel", "noopener noreferrer");
        // Open external links in new tab
        if (node.getAttribute("href")?.startsWith("http")) {
          node.setAttribute("target", "_blank");
        }
      }
    });

    isInitialized = true;
  }

  return purify;
}

/**
 * Strict configuration for DOMPurify
 * Only allows safe HTML elements commonly used in markdown
 */
const STRICT_CONFIG: PurifyConfig = {
  // Allowed HTML tags
  ALLOWED_TAGS: [
    // Text formatting
    "p",
    "br",
    "hr",
    "wbr",
    "strong",
    "b",
    "em",
    "i",
    "u",
    "s",
    "del",
    "ins",
    "mark",
    "small",
    "sub",
    "sup",

    // Headings
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",

    // Lists
    "ul",
    "ol",
    "li",
    "dl",
    "dt",
    "dd",

    // Tables
    "table",
    "thead",
    "tbody",
    "tfoot",
    "tr",
    "th",
    "td",
    "colgroup",
    "col",
    "caption",

    // Blocks
    "div",
    "span",
    "blockquote",
    "pre",
    "code",
    "details",
    "summary",

    // Links and media (carefully controlled)
    "a",
    "img",

    // Semantic
    "article",
    "section",
    "aside",
    "header",
    "footer",
    "figure",
    "figcaption",
    "main",
    "nav",

    // Definition
    "abbr",
    "cite",
    "dfn",
    "kbd",
    "samp",
    "var",
    "time",
    "address",

    // Ruby annotation
    "ruby",
    "rt",
    "rp",
  ],

  // Allowed attributes
  ALLOWED_ATTR: [
    // Global attributes
    "id",
    "class",
    "title",
    "lang",
    "dir",
    "aria-label",
    "aria-labelledby",
    "aria-describedby",
    "aria-hidden",
    "role",

    // Links
    "href",
    "target",
    "rel",
    "download",

    // Images
    "src",
    "alt",
    "width",
    "height",
    "loading",

    // Tables
    "colspan",
    "rowspan",
    "scope",
    "headers",

    // Details
    "open",

    // Data attributes (limited)
    "data-language",
    "data-line",
    "data-source",

    // Datetime
    "datetime",

    // Abbreviation
    "title",
  ],

  // Allowed URI schemes
  ALLOWED_URI_REGEXP:
    /^(?:(?:(?:f|ht)tps?|mailto|tel|callto|sms|cid|xmpp|xxx):|[^a-z]|[a-z+.\-]+(?:[^a-z+.\-:]|$))/i,

  // Security settings
  KEEP_CONTENT: true,
  RETURN_DOM: false,
  RETURN_DOM_FRAGMENT: false,
  RETURN_TRUSTED_TYPE: false,

  // Force output type
  FORCE_BODY: false,

  // Don't allow data URIs for images (could embed malicious content)
  ADD_DATA_URI_TAGS: [],

  // Disallow custom elements
  CUSTOM_ELEMENT_HANDLING: {
    tagNameCheck: null,
    attributeNameCheck: null,
    allowCustomizedBuiltInElements: false,
  },
};

/**
 * Relaxed configuration for trusted content
 * Includes more formatting options
 */
const RELAXED_CONFIG: PurifyConfig = {
  ...STRICT_CONFIG,
  ALLOWED_TAGS: [
    ...(STRICT_CONFIG.ALLOWED_TAGS || []),
    // Additional formatting
    "iframe",
    "video",
    "audio",
    "source",
    "track",
    "canvas",
    "svg",
    "path",
    "g",
    "rect",
    "circle",
    "line",
    "polygon",
    "polyline",
    "text",
    "tspan",
  ],
  ALLOWED_ATTR: [
    ...(STRICT_CONFIG.ALLOWED_ATTR || []),
    // SVG attributes
    "d",
    "fill",
    "stroke",
    "stroke-width",
    "viewBox",
    "xmlns",
    "x",
    "y",
    "cx",
    "cy",
    "r",
    "rx",
    "ry",
    "points",
    "transform",
    "opacity",
    // Media
    "controls",
    "autoplay",
    "loop",
    "muted",
    "poster",
    "preload",
    "playsinline",
    // iFrame (sandboxed)
    "sandbox",
    "allow",
    "allowfullscreen",
  ],
};

/**
 * Sanitize HTML content with strict settings
 *
 * @param html - The HTML string to sanitize
 * @returns Sanitized HTML string safe for rendering
 */
export function sanitizeHtml(html: string): string {
  const domPurify = initializePurify();

  if (!domPurify) {
    // SSR fallback - strip all HTML tags
    return html.replace(/<[^>]*>/g, "");
  }

  return domPurify.sanitize(html, STRICT_CONFIG) as string;
}

/**
 * Sanitize HTML with relaxed settings for trusted content
 * Use sparingly and only for content you control
 *
 * @param html - The HTML string to sanitize
 * @returns Sanitized HTML string
 */
export function sanitizeHtmlRelaxed(html: string): string {
  const domPurify = initializePurify();

  if (!domPurify) {
    return html.replace(/<[^>]*>/g, "");
  }

  return domPurify.sanitize(html, RELAXED_CONFIG) as string;
}

/**
 * Check if HTML content is safe (would not be modified by sanitization)
 *
 * @param html - The HTML string to check
 * @returns true if content is safe, false if it would be modified
 */
export function isHtmlSafe(html: string): boolean {
  const sanitized = sanitizeHtml(html);
  return sanitized === html;
}

/**
 * Remove all HTML tags from content
 * Useful for extracting plain text
 *
 * @param html - The HTML string
 * @returns Plain text with all HTML removed
 */
export function stripHtml(html: string): string {
  const domPurify = initializePurify();

  if (!domPurify) {
    return html.replace(/<[^>]*>/g, "");
  }

  // Use DOMPurify with empty allowed tags
  return domPurify.sanitize(html, { ALLOWED_TAGS: [], KEEP_CONTENT: true }) as string;
}

/**
 * Sanitize HTML specifically for markdown-generated content
 * Optimized for the output of marked.js
 *
 * @param html - The HTML string from markdown parsing
 * @returns Sanitized HTML string
 */
export function sanitizeMarkdownHtml(html: string): string {
  return sanitizeHtml(html);
}
