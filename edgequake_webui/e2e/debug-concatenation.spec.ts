import { test } from "@playwright/test";

test.describe("Debug Concatenation Test", () => {
  test("should debug exactly what concatenation issue is found", async ({
    page,
  }) => {
    await page.goto("/query");
    await page.waitForLoadState("networkidle");

    const textarea = page.getByPlaceholder(/ask|question|query/i).first();
    await textarea.fill("Explain what EdgeQuake is in one paragraph");

    const submitButton = page
      .getByRole("button", { name: /send|submit/i })
      .first();
    await submitButton.click();
    await page.waitForTimeout(8000);

    const pageText = await page.textContent("body");
    const cleanText =
      pageText
        ?.replace(/TFF8NDmVmyfhMoYlcY4VR/g, "")
        .replace(/__next_/g, "")
        .replace(/_app_/g, "") || "";

    console.log(
      "Full page text sample (first 1000 chars):",
      cleanText.substring(0, 1000)
    );

    const concatenationPatterns = [
      { name: "Onceuponatime", pattern: "Onceuponatime" },
      { name: "EdgeQuakeisa", pattern: "EdgeQuakeisa" },
      { name: "RAGframework", pattern: "RAGframework" },
      { name: "systemdesigned", pattern: "systemdesigned" },
      { name: "artificialintelligence", pattern: "artificialintelligence" },
      {
        name: "CamelCase regex",
        pattern: /\b(edge|rag|retrieval|knowledge)[a-z]+[A-Z][a-z]+/i,
      },
    ];

    for (const { name, pattern } of concatenationPatterns) {
      const hasIssue =
        typeof pattern === "string"
          ? cleanText.includes(pattern)
          : pattern.test(cleanText);
      console.log(`❓ ${name}:`, hasIssue);

      if (hasIssue && typeof pattern !== "string") {
        const matches = cleanText.match(pattern);
        console.log(`   Matches found:`, matches);
      }
    }

    // Also check for the actual response content separately
    console.log("\n🔍 Looking for actual chat response content...");

    // Try to find the actual response text by looking for common response patterns
    const responsePatterns = [
      /EdgeQuake[\s\S]*?(?=\n\n|\.$|$)/i,
      /RAG[\s\S]*?(?=\n\n|\.$|$)/i,
      /retrieval[\s\S]*?(?=\n\n|\.$|$)/i,
    ];

    for (const pattern of responsePatterns) {
      const match = cleanText.match(pattern);
      if (match) {
        console.log(
          "📝 Found response text:",
          match[0].substring(0, 200) + "..."
        );
        break;
      }
    }
  });
});
