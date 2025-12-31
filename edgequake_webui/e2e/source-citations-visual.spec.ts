import { test, expect } from "@playwright/test";

test.describe("Source Citations Visual Verification", () => {
  test("verify improved source citations UX", async ({ page }) => {
    // Navigate to query page
    await page.goto("/query");
    await page.waitForLoadState("networkidle");
    
    // Find the main query textarea specifically
    const textarea = page.getByRole("textbox", { name: "Ask a question..." });
    await textarea.fill("What are the key concepts in this knowledge base?");
    
    // Submit the query
    const submitBtn = page.getByRole("button", { name: /send|submit/i });
    await submitBtn.click();
    
    // Wait for response
    await page.waitForTimeout(8000);
    
    // Take screenshot of the result showing source citations
    await page.screenshot({ 
      path: "test-results/source-citations-ux-improved.png",
      fullPage: true 
    });
    
    // Verify we have response content
    const pageText = await page.textContent("body");
    expect(pageText?.length).toBeGreaterThan(500);
    
    console.log("✅ Screenshot saved: test-results/source-citations-ux-improved.png");
  });
});
