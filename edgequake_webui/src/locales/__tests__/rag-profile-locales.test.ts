import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import en from "../en.json";
import fr from "../fr.json";
import zh from "../zh.json";

type Tree = { [key: string]: string | Tree };
type Locale = { ragProfile: Tree; query: { kinds: Tree; searchByImage: Tree } };

function leaves(tree: Tree, prefix: string): string[] {
  return Object.entries(tree).flatMap(([key, value]) =>
    typeof value === "string" ? [`${prefix}${key}`] : leaves(value, `${prefix}${key}.`),
  );
}

function get(tree: Tree, path: string): string | undefined {
  let node: string | Tree | undefined = tree;
  for (const part of path.split(".")) {
    if (typeof node !== "object" || node === undefined) return undefined;
    node = node[part];
  }
  return typeof node === "string" ? node : undefined;
}

const spec = (l: unknown) => l as Locale;
const slices = (l: Locale): Tree => ({
  ragProfile: l.ragProfile,
  query: { kinds: l.query.kinds, searchByImage: l.query.searchByImage },
});
const keysOf = (l: unknown) => leaves(slices(spec(l)), "").sort();
const placeholders = (text: string) => [...text.matchAll(/\{\{(\w+)\}\}/g)].map((m) => m[1]).sort();

function sourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (name === "__tests__" || name === "node_modules") return [];
    if (statSync(path).isDirectory()) return sourceFiles(path);
    return /\.(ts|tsx)$/.test(name) ? [path] : [];
  });
}

describe("SPEC-180 locales", () => {
  it("has the same keys in en, fr and zh", () => {
    expect(keysOf(fr)).toEqual(keysOf(en));
    expect(keysOf(zh)).toEqual(keysOf(en));
  });

  it("keeps the same {{placeholders}} in every language", () => {
    const base = slices(spec(en));
    for (const [name, locale] of [["fr", fr], ["zh", zh]] as const) {
      for (const key of keysOf(en)) {
        const mine = get(slices(spec(locale)), key) ?? "";
        expect(placeholders(mine), `${name}:${key}`).toEqual(placeholders(get(base, key) ?? ""));
      }
    }
  });

  it("has no empty strings", () => {
    for (const locale of [en, fr, zh]) {
      for (const key of keysOf(locale)) {
        expect(get(slices(spec(locale)), key)?.trim(), key).not.toBe("");
      }
    }
  });

  it("defines every literal ragProfile.* key the code asks for", () => {
    const root = join(__dirname, "..", "..");
    const defined = new Set(keysOf(en).map((k) => k));
    const used = new Set<string>();
    for (const file of sourceFiles(root)) {
      const text = readFileSync(file, "utf8");
      for (const match of text.matchAll(/['"`](ragProfile\.[A-Za-z.]+)['"`]/g)) used.add(match[1]!);
    }
    const missing = [...used].filter((key) => !defined.has(key));
    expect(missing).toEqual([]);
  });
});
