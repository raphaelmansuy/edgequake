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
