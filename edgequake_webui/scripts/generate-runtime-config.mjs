import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

const DEFAULT_API_URL = "http://localhost:8080";

const normalizeUrl = (value) => {
  const trimmed = typeof value === "string" ? value.trim() : "";
  if (!trimmed) {
    return "";
  }
  return trimmed.replace(/\/$/, "");
};

const deriveWebSocketUrl = (apiUrl) => {
  if (!apiUrl) {
    return "";
  }
  return apiUrl.replace(/^https:/, "wss:").replace(/^http:/, "ws:");
};

const normalizeWebSocketUrl = (value) => {
  const normalized = normalizeUrl(value);
  if (!normalized) {
    return "";
  }

  if (normalized.startsWith("http://") || normalized.startsWith("https://")) {
    return deriveWebSocketUrl(normalized);
  }

  return normalized;
};

const apiUrl =
  normalizeUrl(process.env.EDGEQUAKE_API_URL) ||
  normalizeUrl(process.env.NEXT_PUBLIC_API_URL) ||
  DEFAULT_API_URL;
const wsUrl =
  normalizeWebSocketUrl(process.env.EDGEQUAKE_WS_URL) ||
  normalizeWebSocketUrl(process.env.NEXT_PUBLIC_WS_URL) ||
  deriveWebSocketUrl(apiUrl);

const outputPath = resolve(process.cwd(), "public/runtime-config.js");
mkdirSync(dirname(outputPath), { recursive: true });

const payload = {
  apiUrl,
  wsUrl,
};

const content = `globalThis.__EDGEQUAKE_RUNTIME_CONFIG__ = ${JSON.stringify(payload)};\n`;
writeFileSync(outputPath, content, "utf8");

console.log(`[edgequake-webui] Wrote runtime config to ${outputPath}`);
