/**
 * Marked.js math extension definitions (tokenization only — React renders via MathTokenRenderer).
 * @implements UI-DRY-008 — single factory for all LaTeX delimiter styles
 */

type MathLevel = "block" | "inline";

interface MathExtensionConfig {
  name: string;
  level: MathLevel;
  startPattern: RegExp;
  bodyPattern: RegExp;
  htmlTag: string;
}

/** True when a bracket body is display LaTeX rather than a citation or link. */
export function isBareLatexBlock(body: string): boolean {
  const trimmed = body.trim();
  if (!trimmed || trimmed.includes("](")) return false;
  if (/^\d[\d,\s–-]*$/.test(trimmed)) return false;
  return /\\[A-Za-z]{2,}/.test(trimmed);
}

function defineMathExtension(config: MathExtensionConfig) {
  return {
    name: config.name,
    level: config.level,
    start(src: string) {
      return src.match(config.startPattern)?.index;
    },
    tokenizer(src: string) {
      const match = config.bodyPattern.exec(src);
      if (!match) {
        return undefined;
      }
      return {
        type: config.name,
        raw: match[0],
        text: match[1].trim(),
      };
    },
    renderer(token: { text: string }) {
      return `<${config.htmlTag}>${token.text}</${config.htmlTag}>`;
    },
  };
}

/** All custom math extensions for configure-marked (order: block before inline where relevant). */
export const MATH_MARKED_EXTENSIONS = [
  defineMathExtension({
    name: "math_block",
    level: "block",
    startPattern: /\$\$/,
    bodyPattern: /^\$\$([\s\S]+?)\$\$/,
    htmlTag: "math-block",
  }),
  defineMathExtension({
    name: "math_bracket_block",
    level: "block",
    startPattern: /\\\[/,
    bodyPattern: /^\\\[([\s\S]+?)\\\]/,
    htmlTag: "math-bracket-block",
  }),
  {
    name: "math_bare_bracket_block",
    level: "block" as const,
    start(src: string) {
      return src.search(/^[ \t]*\[\s*$/m);
    },
    tokenizer(src: string) {
      const match = /^[ \t]*\[\s*\n([\s\S]*?)\n[ \t]*\][ \t]*(?:\n|$)/.exec(src);
      if (!match || !isBareLatexBlock(match[1])) {
        return undefined;
      }
      return {
        type: "math_bare_bracket_block",
        raw: match[0],
        text: match[1].trim(),
      };
    },
    renderer(token: { text: string }) {
      return `<math-bare-bracket-block>${token.text}</math-bare-bracket-block>`;
    },
  },
  defineMathExtension({
    name: "math_inline",
    level: "inline",
    startPattern: /(?<!\$)\$(?!\$)/,
    bodyPattern: /^\$([^\$\n]+?)\$/,
    htmlTag: "math-inline",
  }),
  defineMathExtension({
    name: "math_paren_inline",
    level: "inline",
    startPattern: /\\\(/,
    bodyPattern: /^\\\(([\s\S]+?)\\\)/,
    htmlTag: "math-paren-inline",
  }),
];
