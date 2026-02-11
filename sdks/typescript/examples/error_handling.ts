/**
 * Example: Error Handling Patterns
 *
 * Demonstrates how to handle different error types from the EdgeQuake SDK,
 * including retries, rate limiting, and graceful degradation.
 */

import {
  EdgeQuake,
  EdgeQuakeError,
  NetworkError,
  NotFoundError,
  RateLimitedError,
  TimeoutError,
  UnauthorizedError,
  ValidationError,
} from "@edgequake/sdk";

async function main() {
  const client = new EdgeQuake({
    baseUrl: "http://localhost:8080",
    apiKey: "your-api-key",
  });

  // --- Pattern 1: Specific error handling ---
  console.log("=== Pattern 1: Specific error types ===");
  try {
    await client.documents.get("non-existent-id");
  } catch (error) {
    if (error instanceof NotFoundError) {
      console.log("Document not found — could prompt user to upload");
    } else if (error instanceof UnauthorizedError) {
      console.log("Invalid API key — redirect to login");
    } else if (error instanceof RateLimitedError) {
      console.log("Rate limited — back off and retry");
    } else {
      console.log("Unexpected error:", error);
    }
  }

  // --- Pattern 2: Retry with exponential backoff ---
  console.log("\n=== Pattern 2: Retry with backoff ===");
  async function queryWithRetry(query: string, maxRetries = 3) {
    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      try {
        return await client.query.execute({ query });
      } catch (error) {
        if (error instanceof RateLimitedError && attempt < maxRetries) {
          const delay = Math.pow(2, attempt) * 1000; // 2s, 4s, 8s
          console.log(
            `Rate limited, retrying in ${delay}ms (attempt ${attempt}/${maxRetries})`,
          );
          await new Promise((r) => setTimeout(r, delay));
          continue;
        }
        if (error instanceof NetworkError && attempt < maxRetries) {
          console.log(
            `Network error, retrying (attempt ${attempt}/${maxRetries})`,
          );
          await new Promise((r) => setTimeout(r, 1000));
          continue;
        }
        throw error; // Non-retryable or max retries exceeded
      }
    }
  }

  try {
    const result = await queryWithRetry("What is EdgeQuake?");
    console.log("Query result:", result?.answer);
  } catch (error) {
    console.log("All retries exhausted:", error);
  }

  // --- Pattern 3: Graceful degradation ---
  console.log("\n=== Pattern 3: Graceful degradation ===");
  try {
    const health = await client.health();
    console.log("Backend healthy:", health.status);
  } catch (error) {
    if (error instanceof NetworkError || error instanceof TimeoutError) {
      console.log("Backend unreachable — showing cached data");
    } else {
      console.log("Unexpected health check failure:", error);
    }
  }

  // --- Pattern 4: Validation error details ---
  console.log("\n=== Pattern 4: Validation errors ===");
  try {
    await client.documents.upload({ content: "" }); // Empty content
  } catch (error) {
    if (error instanceof ValidationError) {
      console.log("Validation failed:", error.message);
      console.log("Status:", error.status); // 400 or 422
      console.log("Code:", error.code);
    }
  }

  // --- Pattern 5: Generic EdgeQuakeError catch-all ---
  console.log("\n=== Pattern 5: Catch-all ===");
  try {
    await client.documents.get("some-id");
  } catch (error) {
    if (error instanceof EdgeQuakeError) {
      // All SDK errors extend EdgeQuakeError
      console.log(
        `API error [${error.status}] ${error.code}: ${error.message}`,
      );
    } else {
      // Non-SDK error (e.g., TypeError from bad config)
      console.log("Unexpected error:", error);
    }
  }
}

main().catch(console.error);
