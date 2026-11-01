import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { eligibleModelsPath } from "../model-catalog";

const here = dirname(fileURLToPath(import.meta.url));

describe("eligibleModelsPath (EC-178-15)", () => {
  it("asks the tenant scope before a workspace exists", () => {
    expect(eligibleModelsPath({ scope: "tenant", id: "tenant-1", usage: "query" })).toBe(
      "/models/eligible?usage=query&tenant_id=tenant-1",
    );
  });

  it("asks the deployment scope with no id", () => {
    expect(eligibleModelsPath({ scope: "deployment", usage: "embedding" })).toBe(
      "/models/eligible?usage=embedding&scope=deployment",
    );
  });

  it("create and onboarding pickers pass tenant scope", () => {
    const root = join(here, "../../../../..");
    const rel = "components/onboarding/steps/model-defaults-step.tsx";
    const src = readFileSync(join(root, "src", rel), "utf8");
    // The scope is derived once from the tenant presence, then forwarded to every picker in the
    // step. Assert the derivation and the forwarding, not one exact JSX spelling: the prop was
    // renamed (`choiceScope={pickerScope}` -> `offerScope`/`pickerScope={pickerScope}`) and a
    // literal match turned a correct refactor into a red suite.
    expect(src, rel).toContain("tenantId ? 'tenant' : 'deployment'");
    expect(src, rel).toMatch(/(offerScope|choiceScope)=\{pickerScope\}/);
    expect(src, rel).toContain("pickerScope={pickerScope}");
  });
});

describe("choice surfaces do not import the inventory hooks (EC-178-40)", () => {
  const files = [
    "components/query/composer/model-chip.tsx",
    "components/query/query-settings-sheet.tsx",
    "hooks/use-query-stream-session.ts",
    "hooks/use-query-conversation-lifecycle.ts",
    "components/documents/document-dropzone.tsx",
    "components/workspace/workspace-model-config-grid.tsx",
    "components/workspace/workspace-extended-model-config.tsx",
    "components/onboarding/steps/model-defaults-step.tsx",
    "components/settings/server-llm-config-card.tsx",
    "components/settings/vision-llm-settings-card.tsx",
    "components/workspace/llm-model-selector.tsx",
    "components/workspace/embedding-model-selector.tsx",
  ];

  it("keeps useLlmModels and useEmbeddingModels off the choice files", () => {
    const root = join(here, "../../../../..");
    for (const rel of files) {
      const src = readFileSync(join(root, "src", rel), "utf8");
      expect(src, rel).not.toMatch(/useLlmModels|useEmbeddingModels/);
    }
  });
});

describe("offer writes invalidate the eligible prefix (EC-178-17)", () => {
  it("names eligibleRoot from both policy saves", () => {
    const root = join(here, "../../../../..");
    const src = readFileSync(join(root, "src/hooks/use-model-policy.ts"), "utf8");
    const tenant = src.slice(src.indexOf("function useSaveTenantModelPolicy"));
    const workspace = tenant.slice(tenant.indexOf("function useSaveWorkspaceModelPolicy"));
    expect(tenant.slice(0, tenant.indexOf("function useSaveWorkspaceModelPolicy"))).toContain(
      "eligibleRoot",
    );
    expect(workspace).toContain("eligibleRoot");
  });
});
