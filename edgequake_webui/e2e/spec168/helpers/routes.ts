/** SPEC-168 route table shared by capture, CLS and keyboard specs. */
export const ROUTES = [
  ["dashboard", "/"],
  ["documents", "/documents"],
  ["query", "/query"],
  ["graph", "/graph?stream=0"],
  ["pipeline", "/pipeline"],
  ["costs", "/costs"],
  ["workspace", "/workspace"],
  ["settings", "/settings"],
  ["knowledge", "/knowledge"],
  ["api-explorer", "/api-explorer"],
  ["login", "/login"],
] as const;
