import { expect, test } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";

/**
 * Progressive Load Testing for Document Upload Performance
 *
 * @implements FEAT0401: Document Upload (Text)
 * @implements FEAT0402: Document Upload (File)
 *
 * This test suite performs progressive load testing on the document upload functionality.
 * It starts with a single document and gradually increases the load to identify:
 * - Performance baseline
 * - Response time degradation
 * - System breaking points
 * - Throughput characteristics
 *
 * Test Phases:
 * 1. Warmup: Single upload to establish baseline
 * 2. Light Load: 5 concurrent uploads
 * 3. Medium Load: 10 concurrent uploads
 * 4. Heavy Load: 25 concurrent uploads
 * 5. Stress Load: 50 concurrent uploads
 */

// ============================================================================
// Test Configuration
// ============================================================================

interface PerformanceMetrics {
  phase: string;
  concurrency: number;
  totalUploads: number;
  successCount: number;
  failureCount: number;
  minDurationMs: number;
  maxDurationMs: number;
  avgDurationMs: number;
  p50DurationMs: number;
  p90DurationMs: number;
  p95DurationMs: number;
  p99DurationMs: number;
  throughputPerSecond: number;
  errorRate: number;
  startTime: number;
  endTime: number;
  totalDurationMs: number;
}

interface UploadResult {
  success: boolean;
  durationMs: number;
  documentId?: string;
  error?: string;
  statusCode?: number;
}

// ============================================================================
// Test Data Generators
// ============================================================================

/**
 * Generate test document content of varying sizes
 */
function generateTestContent(size: "small" | "medium" | "large"): string {
  const baseContent = `
# Test Document: Performance Evaluation

## Introduction
This document is part of a performance testing suite designed to evaluate the 
EdgeQuake knowledge graph system under various load conditions.

## Technical Architecture
EdgeQuake is a Retrieval-Augmented Generation (RAG) framework that combines 
graph-based knowledge representation with advanced LLM capabilities.

### Key Components
- Entity Extraction: Identifies and normalizes entities from documents
- Relationship Discovery: Maps connections between entities
- Vector Storage: Enables semantic search capabilities
- Graph Storage: Maintains entity-relationship graph structure

## Performance Characteristics
The system is designed to handle multiple concurrent document uploads while 
maintaining low latency and high throughput.

### Scalability Factors
1. Concurrent Processing: Multiple documents can be processed simultaneously
2. Async Operations: Long-running tasks are handled asynchronously
3. Resource Management: Efficient memory and CPU utilization
4. Storage Optimization: Deduplication and efficient indexing

## Test Methodology
Progressive load testing helps identify system behavior under increasing stress.
Starting with baseline measurements and gradually increasing load reveals:
- Linear scaling characteristics
- Resource saturation points
- Error rate thresholds
- Recovery capabilities
`;

  switch (size) {
    case "small":
      // ~500 bytes
      return baseContent.substring(0, 500);
    case "medium":
      // ~2KB
      return baseContent.repeat(3);
    case "large":
      // ~10KB
      return baseContent.repeat(15);
    default:
      return baseContent;
  }
}

/**
 * Calculate percentile from sorted array
 */
function percentile(sortedArray: number[], p: number): number {
  if (sortedArray.length === 0) return 0;
  const index = Math.ceil((sortedArray.length * p) / 100) - 1;
  return sortedArray[Math.max(0, Math.min(index, sortedArray.length - 1))];
}

/**
 * Analyze upload results and compute metrics
 */
function analyzeResults(
  phase: string,
  concurrency: number,
  results: UploadResult[],
  startTime: number,
  endTime: number
): PerformanceMetrics {
  const successResults = results.filter((r) => r.success);
  const failureResults = results.filter((r) => !r.success);

  const durations = successResults.map((r) => r.durationMs).sort((a, b) => a - b);
  
  const totalDurationMs = endTime - startTime;
  const throughputPerSecond = (successResults.length / totalDurationMs) * 1000;

  return {
    phase,
    concurrency,
    totalUploads: results.length,
    successCount: successResults.length,
    failureCount: failureResults.length,
    minDurationMs: durations.length > 0 ? Math.min(...durations) : 0,
    maxDurationMs: durations.length > 0 ? Math.max(...durations) : 0,
    avgDurationMs: durations.length > 0 
      ? durations.reduce((a, b) => a + b, 0) / durations.length 
      : 0,
    p50DurationMs: percentile(durations, 50),
    p90DurationMs: percentile(durations, 90),
    p95DurationMs: percentile(durations, 95),
    p99DurationMs: percentile(durations, 99),
    throughputPerSecond,
    errorRate: (failureResults.length / results.length) * 100,
    startTime,
    endTime,
    totalDurationMs,
  };
}

/**
 * Format metrics for console output
 */
function formatMetrics(metrics: PerformanceMetrics): string {
  return `
╔════════════════════════════════════════════════════════════════════════╗
║ ${metrics.phase.padEnd(70)} ║
╠════════════════════════════════════════════════════════════════════════╣
║ Concurrency:          ${metrics.concurrency.toString().padEnd(10)} │ Total Uploads:      ${metrics.totalUploads.toString().padEnd(10)} ║
║ Success:              ${metrics.successCount.toString().padEnd(10)} │ Failures:           ${metrics.failureCount.toString().padEnd(10)} ║
║ Error Rate:           ${metrics.errorRate.toFixed(2).padEnd(10)}% │ Throughput:         ${metrics.throughputPerSecond.toFixed(2).padEnd(10)}/s ║
╠════════════════════════════════════════════════════════════════════════╣
║ Response Times (ms)                                                    ║
║ Min:                  ${metrics.minDurationMs.toFixed(0).padEnd(10)} │ Max:                ${metrics.maxDurationMs.toFixed(0).padEnd(10)} ║
║ Average:              ${metrics.avgDurationMs.toFixed(0).padEnd(10)} │ P50 (Median):       ${metrics.p50DurationMs.toFixed(0).padEnd(10)} ║
║ P90:                  ${metrics.p90DurationMs.toFixed(0).padEnd(10)} │ P95:                ${metrics.p95DurationMs.toFixed(0).padEnd(10)} ║
║ P99:                  ${metrics.p99DurationMs.toFixed(0).padEnd(10)} │ Total Duration:     ${metrics.totalDurationMs.toFixed(0).padEnd(10)} ║
╚════════════════════════════════════════════════════════════════════════╝
`;
}

// ============================================================================
// API Upload Functions
// ============================================================================

/**
 * Upload document via API with timing
 */
async function uploadDocumentViaAPI(
  baseURL: string,
  content: string,
  title: string,
  workspaceId: string = "default-workspace"
): Promise<UploadResult> {
  const startTime = Date.now();

  try {
    const response = await fetch(`${baseURL}/api/v1/documents`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Workspace-Id": workspaceId,
      },
      body: JSON.stringify({
        content,
        title,
        async_processing: true, // Use async for performance testing
      }),
    });

    const durationMs = Date.now() - startTime;
    const data = await response.json();

    return {
      success: response.ok,
      durationMs,
      documentId: data.document_id,
      statusCode: response.status,
      error: !response.ok ? JSON.stringify(data) : undefined,
    };
  } catch (error) {
    const durationMs = Date.now() - startTime;
    return {
      success: false,
      durationMs,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

/**
 * Execute concurrent uploads with controlled concurrency
 */
async function executeConcurrentUploads(
  baseURL: string,
  count: number,
  concurrency: number,
  contentSize: "small" | "medium" | "large"
): Promise<UploadResult[]> {
  const results: UploadResult[] = [];
  const promises: Promise<void>[] = [];
  let completed = 0;

  for (let i = 0; i < count; i++) {
    const uploadPromise = (async () => {
      const content = generateTestContent(contentSize);
      const title = `perf-test-${contentSize}-${Date.now()}-${i}`;
      const result = await uploadDocumentViaAPI(baseURL, content, title);
      results.push(result);
      completed++;

      // Log progress
      if (completed % 10 === 0 || completed === count) {
        console.log(`  Progress: ${completed}/${count} uploads completed`);
      }
    })();

    promises.push(uploadPromise);

    // Control concurrency by waiting when we hit the limit
    if (promises.length >= concurrency) {
      await Promise.race(promises);
      // Remove completed promises
      const stillPending = promises.filter((p) => {
        let isPending = true;
        p.then(() => {
          isPending = false;
        }).catch(() => {
          isPending = false;
        });
        return isPending;
      });
      promises.length = 0;
      promises.push(...stillPending);
    }
  }

  // Wait for remaining uploads
  await Promise.all(promises);
  return results;
}

// ============================================================================
// Test Suite
// ============================================================================

test.describe("Upload Performance - Progressive Load Testing", () => {
  const baseURL = process.env.PLAYWRIGHT_BASE_URL || "http://localhost:3001";
  const allMetrics: PerformanceMetrics[] = [];
  const reportPath = path.join(__dirname, "../test-results/upload-performance-report.txt");

  test.beforeAll(() => {
    console.log("\n" + "=".repeat(80));
    console.log("UPLOAD PERFORMANCE - PROGRESSIVE LOAD TESTING");
    console.log("=".repeat(80));
    console.log(`Base URL: ${baseURL}`);
    console.log(`Start Time: ${new Date().toISOString()}`);
    console.log("=".repeat(80) + "\n");
  });

  test.afterAll(() => {
    // Generate comprehensive report
    console.log("\n" + "=".repeat(80));
    console.log("PERFORMANCE TEST SUMMARY");
    console.log("=".repeat(80));

    allMetrics.forEach((metrics) => {
      console.log(formatMetrics(metrics));
    });

    // Compare phases
    console.log("\n" + "=".repeat(80));
    console.log("COMPARATIVE ANALYSIS");
    console.log("=".repeat(80));
    console.log("Phase                    | Concurrency | Avg (ms) | P95 (ms) | Throughput/s | Error %");
    console.log("-".repeat(80));
    allMetrics.forEach((m) => {
      console.log(
        `${m.phase.padEnd(24)} | ${m.concurrency.toString().padEnd(11)} | ${m.avgDurationMs.toFixed(0).padEnd(8)} | ${m.p95DurationMs.toFixed(0).padEnd(8)} | ${m.throughputPerSecond.toFixed(2).padEnd(12)} | ${m.errorRate.toFixed(2)}`
      );
    });
    console.log("=".repeat(80) + "\n");

    // Save report to file
    const reportContent = allMetrics.map((m) => formatMetrics(m)).join("\n");
    fs.mkdirSync(path.dirname(reportPath), { recursive: true });
    fs.writeFileSync(reportPath, reportContent);
    console.log(`📊 Full report saved to: ${reportPath}\n`);
  });

  test("Phase 0: Warmup - Single Upload Baseline", async () => {
    console.log("\n🔥 Phase 0: Warmup - Establishing Baseline");
    console.log("-".repeat(80));

    const startTime = Date.now();
    const results = await executeConcurrentUploads(baseURL, 1, 1, "small");
    const endTime = Date.now();

    const metrics = analyzeResults("Phase 0: Warmup", 1, results, startTime, endTime);
    allMetrics.push(metrics);

    console.log(formatMetrics(metrics));

    // Assertions
    expect(metrics.successCount).toBe(1);
    expect(metrics.errorRate).toBe(0);
    expect(metrics.avgDurationMs).toBeLessThan(5000); // Baseline should be under 5s
  });

  test("Phase 1: Light Load - 5 Concurrent Uploads", async () => {
    console.log("\n⚡ Phase 1: Light Load");
    console.log("-".repeat(80));

    const startTime = Date.now();
    const results = await executeConcurrentUploads(baseURL, 10, 5, "small");
    const endTime = Date.now();

    const metrics = analyzeResults("Phase 1: Light Load", 5, results, startTime, endTime);
    allMetrics.push(metrics);

    console.log(formatMetrics(metrics));

    // Assertions
    expect(metrics.successCount).toBeGreaterThan(0);
    expect(metrics.errorRate).toBeLessThan(10); // Allow up to 10% error rate
    expect(metrics.p95DurationMs).toBeLessThan(10000); // P95 under 10s
  });

  test("Phase 2: Medium Load - 10 Concurrent Uploads", async () => {
    console.log("\n🚀 Phase 2: Medium Load");
    console.log("-".repeat(80));

    const startTime = Date.now();
    const results = await executeConcurrentUploads(baseURL, 20, 10, "medium");
    const endTime = Date.now();

    const metrics = analyzeResults("Phase 2: Medium Load", 10, results, startTime, endTime);
    allMetrics.push(metrics);

    console.log(formatMetrics(metrics));

    // Assertions
    expect(metrics.successCount).toBeGreaterThan(0);
    expect(metrics.errorRate).toBeLessThan(20); // Allow up to 20% error rate
    expect(metrics.avgDurationMs).toBeLessThan(15000); // Avg under 15s
  });

  test("Phase 3: Heavy Load - 25 Concurrent Uploads", async () => {
    console.log("\n💪 Phase 3: Heavy Load");
    console.log("-".repeat(80));

    const startTime = Date.now();
    const results = await executeConcurrentUploads(baseURL, 50, 25, "medium");
    const endTime = Date.now();

    const metrics = analyzeResults("Phase 3: Heavy Load", 25, results, startTime, endTime);
    allMetrics.push(metrics);

    console.log(formatMetrics(metrics));

    // Assertions
    expect(metrics.successCount).toBeGreaterThan(0);
    expect(metrics.errorRate).toBeLessThan(30); // System may degrade under heavy load
    expect(metrics.throughputPerSecond).toBeGreaterThan(0.5); // At least 0.5 uploads/sec
  });

  test("Phase 4: Stress Load - 50 Concurrent Uploads", async () => {
    console.log("\n🔴 Phase 4: Stress Load");
    console.log("-".repeat(80));

    const startTime = Date.now();
    const results = await executeConcurrentUploads(baseURL, 100, 50, "large");
    const endTime = Date.now();

    const metrics = analyzeResults("Phase 4: Stress Load", 50, results, startTime, endTime);
    allMetrics.push(metrics);

    console.log(formatMetrics(metrics));

    // Assertions
    expect(metrics.successCount).toBeGreaterThan(0);
    // Under stress, we expect higher error rates but system should not completely fail
    expect(metrics.errorRate).toBeLessThan(50); // At least 50% success rate
    expect(metrics.throughputPerSecond).toBeGreaterThan(0.3); // Minimum throughput maintained
  });

  test("Phase 5: Recovery Test - Return to Light Load", async () => {
    console.log("\n🔄 Phase 5: Recovery Test");
    console.log("-".repeat(80));
    console.log("Testing system recovery after stress...");

    // Wait a bit for system to recover
    await new Promise((resolve) => setTimeout(resolve, 5000));

    const startTime = Date.now();
    const results = await executeConcurrentUploads(baseURL, 10, 5, "small");
    const endTime = Date.now();

    const metrics = analyzeResults("Phase 5: Recovery", 5, results, startTime, endTime);
    allMetrics.push(metrics);

    console.log(formatMetrics(metrics));

    // Compare with Phase 1 to verify recovery
    const phase1Metrics = allMetrics.find((m) => m.phase === "Phase 1: Light Load");
    if (phase1Metrics) {
      const performanceDegradation =
        ((metrics.avgDurationMs - phase1Metrics.avgDurationMs) / phase1Metrics.avgDurationMs) *
        100;

      console.log(`\n📊 Recovery Analysis:`);
      console.log(`  Phase 1 Avg: ${phase1Metrics.avgDurationMs.toFixed(0)}ms`);
      console.log(`  Phase 5 Avg: ${metrics.avgDurationMs.toFixed(0)}ms`);
      console.log(`  Performance Change: ${performanceDegradation.toFixed(1)}%`);

      // System should recover to within 50% of original performance
      expect(Math.abs(performanceDegradation)).toBeLessThan(50);
    }

    // Assertions
    expect(metrics.successCount).toBeGreaterThan(0);
    expect(metrics.errorRate).toBeLessThan(15); // Should be close to Phase 1 error rate
  });

  test("Phase 6: Mixed Size Load - Varying Document Sizes", async () => {
    console.log("\n📦 Phase 6: Mixed Size Load");
    console.log("-".repeat(80));
    console.log("Testing with mixed document sizes...");

    const startTime = Date.now();
    const results: UploadResult[] = [];

    // Upload mix of small, medium, and large documents
    const uploads = [
      ...Array(10).fill("small"),
      ...Array(10).fill("medium"),
      ...Array(5).fill("large"),
    ];

    for (let i = 0; i < uploads.length; i++) {
      const size = uploads[i] as "small" | "medium" | "large";
      const content = generateTestContent(size);
      const title = `perf-test-mixed-${size}-${Date.now()}-${i}`;
      const result = await uploadDocumentViaAPI(baseURL, content, title);
      results.push(result);

      if ((i + 1) % 5 === 0) {
        console.log(`  Progress: ${i + 1}/${uploads.length} uploads completed`);
      }
    }

    const endTime = Date.now();
    const metrics = analyzeResults("Phase 6: Mixed Size", 1, results, startTime, endTime);
    allMetrics.push(metrics);

    console.log(formatMetrics(metrics));

    // Assertions
    expect(metrics.successCount).toBeGreaterThan(0);
    expect(metrics.errorRate).toBeLessThan(15);
  });
});

// ============================================================================
// Additional Performance Tests
// ============================================================================

test.describe("Upload Performance - Specific Scenarios", () => {
  const baseURL = process.env.PLAYWRIGHT_BASE_URL || "http://localhost:3001";

  test("Sustained Load - Constant Rate for 2 Minutes", async () => {
    console.log("\n⏱️  Sustained Load Test - 2 Minutes");
    console.log("-".repeat(80));

    const startTime = Date.now();
    const durationMs = 2 * 60 * 1000; // 2 minutes
    const targetRate = 5; // 5 uploads per minute
    const intervalMs = (60 * 1000) / targetRate; // Upload every N seconds

    const results: UploadResult[] = [];
    let uploadCount = 0;

    while (Date.now() - startTime < durationMs) {
      const content = generateTestContent("small");
      const title = `sustained-load-${Date.now()}-${uploadCount}`;
      const result = await uploadDocumentViaAPI(baseURL, content, title);
      results.push(result);
      uploadCount++;

      if (uploadCount % 5 === 0) {
        const elapsed = ((Date.now() - startTime) / 1000).toFixed(0);
        console.log(`  ${elapsed}s elapsed - ${uploadCount} uploads completed`);
      }

      // Wait for next interval
      const nextUploadTime = startTime + uploadCount * intervalMs;
      const waitTime = Math.max(0, nextUploadTime - Date.now());
      if (waitTime > 0) {
        await new Promise((resolve) => setTimeout(resolve, waitTime));
      }
    }

    const endTime = Date.now();
    const metrics = analyzeResults("Sustained Load", 1, results, startTime, endTime);

    console.log(formatMetrics(metrics));

    // Assertions
    expect(metrics.successCount).toBeGreaterThan(0);
    expect(metrics.errorRate).toBeLessThan(10);
    expect(uploadCount).toBeGreaterThanOrEqual(10); // At least 10 uploads in 2 minutes
  });

  test("Burst Load - Rapid Uploads with Pauses", async () => {
    console.log("\n⚡ Burst Load Test");
    console.log("-".repeat(80));

    const allResults: UploadResult[] = [];
    const bursts = [
      { count: 10, concurrency: 10, pause: 5000 },
      { count: 15, concurrency: 15, pause: 10000 },
      { count: 20, concurrency: 20, pause: 5000 },
    ];

    for (let i = 0; i < bursts.length; i++) {
      const burst = bursts[i];
      console.log(`\n  Burst ${i + 1}: ${burst.count} uploads at concurrency ${burst.concurrency}`);

      const results = await executeConcurrentUploads(
        baseURL,
        burst.count,
        burst.concurrency,
        "medium"
      );
      allResults.push(...results);

      const successRate =
        (results.filter((r) => r.success).length / results.length) * 100;
      console.log(`  Success Rate: ${successRate.toFixed(1)}%`);

      if (i < bursts.length - 1) {
        console.log(`  Pausing for ${burst.pause / 1000}s...`);
        await new Promise((resolve) => setTimeout(resolve, burst.pause));
      }
    }

    const metrics = analyzeResults(
      "Burst Load",
      20,
      allResults,
      Date.now() - 60000,
      Date.now()
    );
    console.log(formatMetrics(metrics));

    // Assertions
    expect(metrics.successCount).toBeGreaterThan(0);
    expect(metrics.errorRate).toBeLessThan(30);
  });
});
