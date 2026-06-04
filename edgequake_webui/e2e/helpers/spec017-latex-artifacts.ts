import fs from "node:fs";
import path from "node:path";

/** SPEC-017 webui LaTeX proof artifacts (separate from 001-audit/e2e). */
export const SPEC017_LATEX_E2E_DIR = path.resolve(
  __dirname,
  "../../../specs/017-dry-and-solid-audit/013-edgequake-webui/e2e",
);

export const SPEC017_LATEX_SCREENSHOTS = path.join(
  SPEC017_LATEX_E2E_DIR,
  "screenshots",
);

export function spec017LatexScreenshot(fileName: string): string {
  fs.mkdirSync(SPEC017_LATEX_SCREENSHOTS, { recursive: true });
  return path.join(SPEC017_LATEX_SCREENSHOTS, fileName);
}

export { LATEX_MARKDOWN_SAMPLE as SPEC017_LATEX_MARKDOWN } from "../../src/lib/fixtures/latex-markdown-sample";
