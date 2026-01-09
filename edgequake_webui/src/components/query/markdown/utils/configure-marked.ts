/**
 * Configure marked.js with custom extensions for EdgeQuake
 *
 * This module sets up marked.js once on import with:
 * - GFM (GitHub Flavored Markdown) support
 * - KaTeX math extension
 * - Custom citation extension for source references
 * - GitHub-style alerts (NOTE, TIP, WARNING, CAUTION, IMPORTANT)
 * - Collapsible details blocks
 */
import { marked } from "marked";

let isConfigured = false;

/**
 * Alert types supported by GitHub-style alerts
 */
export type AlertType = "note" | "tip" | "warning" | "caution" | "important";

/**
 * Configure marked.js with default options and extensions.
 * Safe to call multiple times - only configures once.
 */
export function configureMarked(): void {
  if (isConfigured) return;

  marked.setOptions({
    gfm: true,
    breaks: true,
    async: false,
  });

  // Add custom math extension for KaTeX (inline and block)
  marked.use({
    extensions: [
      // Block math: $$...$$
      {
        name: "math_block",
        level: "block",
        start(src: string) {
          return src.match(/\$\$/)?.index;
        },
        tokenizer(src: string) {
          const rule = /^\$\$([\s\S]+?)\$\$/;
          const match = rule.exec(src);
          if (match) {
            return {
              type: "math_block",
              raw: match[0],
              text: match[1].trim(),
            };
          }
          return undefined;
        },
        renderer(token) {
          // Placeholder - actual rendering done in React component
          return `<math-block>${token.text}</math-block>`;
        },
      },
      // Inline math: $...$
      {
        name: "math_inline",
        level: "inline",
        start(src: string) {
          return src.match(/\$/)?.index;
        },
        tokenizer(src: string) {
          // Match $...$ but not $$
          const rule = /^\$([^\$\n]+?)\$/;
          const match = rule.exec(src);
          if (match) {
            return {
              type: "math_inline",
              raw: match[0],
              text: match[1].trim(),
            };
          }
          return undefined;
        },
        renderer(token) {
          return `<math-inline>${token.text}</math-inline>`;
        },
      },
      // GitHub-style alerts: > [!NOTE], > [!TIP], > [!WARNING], > [!CAUTION], > [!IMPORTANT]
      {
        name: "github_alert",
        level: "block",
        start(src: string) {
          return src.match(/^>\s*\[!(?:NOTE|TIP|WARNING|CAUTION|IMPORTANT)\]/im)
            ?.index;
        },
        tokenizer(src: string) {
          // Match blockquote starting with [!TYPE]
          const rule =
            /^(?:>\s*\[!(NOTE|TIP|WARNING|CAUTION|IMPORTANT)\]\n?)((?:>.*(?:\n|$))*)/i;
          const match = rule.exec(src);
          if (match) {
            const alertType = match[1].toLowerCase() as AlertType;
            // Extract content from blockquote lines (remove leading >)
            const content = match[2]
              .split("\n")
              .map((line) => line.replace(/^>\s?/, ""))
              .join("\n")
              .trim();

            return {
              type: "github_alert",
              raw: match[0],
              alertType,
              text: content,
              tokens: [],
            };
          }
          return undefined;
        },
        renderer(token) {
          const typedToken = token as unknown as {
            alertType: AlertType;
            text: string;
          };
          return `<github-alert type="${typedToken.alertType}">${typedToken.text}</github-alert>`;
        },
      },
      // Collapsible details blocks
      {
        name: "details",
        level: "block",
        start(src: string) {
          return src.match(/<details/i)?.index;
        },
        tokenizer(src: string) {
          const rule =
            /^<details(?:\s+open)?>\s*\n?<summary>([\s\S]*?)<\/summary>\s*\n?([\s\S]*?)<\/details>/i;
          const match = rule.exec(src);
          if (match) {
            const isOpen = src.includes("<details open");
            return {
              type: "details",
              raw: match[0],
              summary: match[1].trim(),
              content: match[2].trim(),
              open: isOpen,
              tokens: [],
            };
          }
          return undefined;
        },
        renderer(token) {
          const typedToken = token as unknown as {
            summary: string;
            content: string;
            open: boolean;
          };
          const openAttr = typedToken.open ? " open" : "";
          return `<details${openAttr}><summary>${typedToken.summary}</summary>${typedToken.content}</details>`;
        },
      },
      // Citation extension for [source:id] syntax
      {
        name: "citation",
        level: "inline",
        start(src: string) {
          return src.match(/\[source:/)?.index;
        },
        tokenizer(src: string) {
          const rule = /^\[source:([^\]]+)\]/;
          const match = rule.exec(src);
          if (match) {
            return {
              type: "citation",
              raw: match[0],
              sourceId: match[1].trim(),
            };
          }
          return undefined;
        },
        renderer(token) {
          return `<citation source="${token.sourceId}"></citation>`;
        },
      },
    ],
  });

  isConfigured = true;
}

// Type augmentation for custom tokens
declare module "marked" {
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace Tokens {
    interface MathBlock {
      type: "math_block";
      raw: string;
      text: string;
    }
    interface MathInline {
      type: "math_inline";
      raw: string;
      text: string;
    }
    interface Citation {
      type: "citation";
      raw: string;
      sourceId: string;
    }
    interface GitHubAlert {
      type: "github_alert";
      raw: string;
      alertType: "note" | "tip" | "warning" | "caution" | "important";
      text: string;
      tokens: Token[];
    }
    interface Details {
      type: "details";
      raw: string;
      summary: string;
      content: string;
      open: boolean;
      tokens: Token[];
    }
  }
}
