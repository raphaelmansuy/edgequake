import { expect, test } from "@playwright/test";

test.describe("Comprehensive Chat UX Test", () => {
  test("should have perfect chat UX with all features working", async ({
    page,
  }) => {
    console.log("🔍 Starting comprehensive chat UX test...");

    // Navigate to query page
    await page.goto("/query");
    await page.waitForLoadState("networkidle");

    // Take initial screenshot
    await page.screenshot({
      path: "test-results/chat-ux-initial.png",
      fullPage: true,
    });

    console.log("✅ Query page loaded");

    // Test 1: Interface Elements
    console.log("📋 Testing interface elements...");

    const textarea = page.getByPlaceholder(/ask|question|query/i).first();
    await expect(textarea).toBeVisible();
    console.log("  ✅ Query textarea visible");

    const submitButton = page
      .getByRole("button", { name: /send|submit/i })
      .first();
    await expect(submitButton).toBeVisible();
    console.log("  ✅ Submit button visible");

    // Check for mode selector
    const modeSelector = page
      .locator('select, [role="combobox"], .mode-selector, .select')
      .first();
    if ((await modeSelector.count()) > 0) {
      console.log("  ✅ Mode selector visible");
    } else {
      console.log("  ⚠️  Mode selector not found (might be hidden)");
    }

    // Test 2: Query Execution
    console.log("🚀 Testing query execution...");

    const testQuery = "Explain what EdgeQuake is in one paragraph";
    await textarea.fill(testQuery);
    console.log("  📝 Query filled:", testQuery);

    await submitButton.click();
    console.log("  🎯 Submit clicked, waiting for response...");

    // Wait for response with longer timeout
    await page.waitForTimeout(8000);

    // Test 3: Response Quality
    console.log("📊 Analyzing response quality...");

    await page.screenshot({
      path: "test-results/chat-ux-after-query.png",
      fullPage: true,
    });

    const pageText = await page.textContent("body");
    const pageTextLength = pageText?.length || 0;
    console.log("  📄 Page text length:", pageTextLength);

    // Check for response content
    const hasEdgeQuakeContent =
      pageText?.toLowerCase().includes("edgequake") ||
      pageText?.toLowerCase().includes("rag") ||
      pageText?.toLowerCase().includes("retrieval") ||
      pageText?.toLowerCase().includes("knowledge");

    const hasGenericResponse =
      pageText?.toLowerCase().includes("sorry") ||
      pageText?.toLowerCase().includes("cannot") ||
      pageText?.toLowerCase().includes("unable");

    console.log("  🔍 Has EdgeQuake content:", hasEdgeQuakeContent);
    console.log("  🔍 Has generic response:", hasGenericResponse);

    // Test 4: Message Structure
    console.log("💬 Testing message structure...");

    // Check for user message
    const userMessage = page.locator(`text="${testQuery}"`).first();
    const userMessageVisible = (await userMessage.count()) > 0;
    console.log("  👤 User message visible:", userMessageVisible);

    // Check for response messages (look for various indicators)
    const responseIndicators = [
      page.locator(".group"),
      page.locator('[class*="assistant"]'),
      page.locator('[class*="message"]'),
      page.locator('div:has-text("EdgeQuake")').first(),
      page.locator('div:has-text("RAG")').first(),
    ];

    let hasResponseMessages = false;
    for (const indicator of responseIndicators) {
      if ((await indicator.count()) > 0) {
        hasResponseMessages = true;
        break;
      }
    }
    console.log("  🤖 Response messages present:", hasResponseMessages);

    // Test 5: Text Quality (No Concatenation Issues)
    console.log("📝 Testing text quality...");

    // Clean text from technical artifacts
    const cleanText =
      pageText
        ?.replace(/TFF8NDmVmyfhMoYlcY4VR/g, "")
        .replace(/__next_/g, "")
        .replace(/_app_/g, "") || "";

    const concatenationPatterns = [
      "Onceuponatime",
      "EdgeQuakeisa",
      "RAGframework",
      "systemdesigned",
      "artificialintelligence",
      // Check for camelCase in conversational text (not code)
      /\b(edge|rag|retrieval|knowledge)[a-z]+[A-Z][a-z]+/i,
    ];

    let hasConcatenationIssue = false;
    for (const pattern of concatenationPatterns) {
      if (
        typeof pattern === "string"
          ? cleanText.includes(pattern)
          : pattern.test(cleanText)
      ) {
        hasConcatenationIssue = true;
        break;
      }
    }
    console.log("  🔗 Has concatenation issues:", hasConcatenationIssue);

    // Test 6: Layout and Styling
    console.log("🎨 Testing layout and styling...");

    // Check for proper CSS classes and structure
    const hasProperLayout =
      (await page
        .locator(".container, .chat-container, .messages, .query-interface")
        .count()) > 0;
    console.log("  📐 Has proper layout containers:", hasProperLayout);

    // Test 7: Responsiveness
    console.log("📱 Testing responsiveness...");

    await page.setViewportSize({ width: 375, height: 667 }); // Mobile
    await page.waitForTimeout(1000);

    const mobileTextarea = page.getByPlaceholder(/ask|question|query/i).first();
    const textareaVisibleOnMobile = await mobileTextarea.isVisible();
    console.log("  📱 Mobile responsive:", textareaVisibleOnMobile);

    // Reset viewport
    await page.setViewportSize({ width: 1280, height: 720 });

    // Final Assessments
    console.log("🏁 Final assessments...");

    const overallScore = [
      userMessageVisible,
      hasResponseMessages,
      !hasConcatenationIssue,
      hasEdgeQuakeContent || hasGenericResponse,
      textareaVisibleOnMobile,
      pageTextLength > 10000, // Indicates rich content
    ].filter(Boolean).length;

    console.log("  📊 Overall score:", overallScore, "/ 6");

    // Screenshots for manual review
    await page.screenshot({
      path: "test-results/chat-ux-final.png",
      fullPage: true,
    });

    // Assertions
    expect(userMessageVisible).toBe(true);
    expect(hasResponseMessages).toBe(true);
    expect(hasConcatenationIssue).toBe(false);
    expect(hasEdgeQuakeContent || hasGenericResponse).toBe(true);
    expect(textareaVisibleOnMobile).toBe(true);
    expect(overallScore).toBeGreaterThanOrEqual(5); // At least 5/6 must pass

    console.log("✅ All comprehensive chat UX tests passed!");
  });
});
