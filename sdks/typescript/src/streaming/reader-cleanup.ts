/** Cancel an SSE response when iteration ends or its signal is aborted. */
export function readerCleanup(
  reader: ReadableStreamDefaultReader<Uint8Array>,
  signal?: AbortSignal,
): () => Promise<void> {
  // Cancellation wakes a pending read; the caller checks the signal again afterward.
  const onAbort = () => { void reader.cancel().catch(() => {}); };
  signal?.addEventListener("abort", onAbort, { once: true });
  return async () => {
    signal?.removeEventListener("abort", onAbort);
    try {
      await reader.cancel();
    } catch {
      // A failed response/cancel callback must not hide the original stream error.
    } finally {
      reader.releaseLock();
    }
  };
}
