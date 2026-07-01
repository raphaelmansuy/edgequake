/**
 * Client-side PDF page count extraction (SPEC-038).
 * Mirrors backend `extract_page_count` in pdf_upload/helpers.rs — DRY contract.
 */

/** Extract page count from raw PDF bytes via `/Count N` catalog tokens. */
export function extractPdfPageCount(pdfData: Uint8Array | ArrayBuffer): number | null {
  const bytes = pdfData instanceof Uint8Array ? pdfData : new Uint8Array(pdfData);
  const needle = new TextEncoder().encode("/Count ");
  let maxCount: number | null = null;
  let pos = 0;

  while (pos + needle.length < bytes.length) {
    let found = -1;
    for (let i = pos; i <= bytes.length - needle.length; i++) {
      let match = true;
      for (let j = 0; j < needle.length; j++) {
        if (bytes[i + j] !== needle[j]) {
          match = false;
          break;
        }
      }
      if (match) {
        found = i;
        break;
      }
    }
    if (found < 0) break;

    const start = found + needle.length;
    let digitEnd = start;
    while (digitEnd < bytes.length && bytes[digitEnd] >= 0x30 && bytes[digitEnd] <= 0x39) {
      digitEnd++;
    }
    if (digitEnd > start) {
      const num = Number.parseInt(
        new TextDecoder().decode(bytes.subarray(start, digitEnd)),
        10,
      );
      if (!Number.isNaN(num)) {
        maxCount = maxCount === null ? num : Math.max(maxCount, num);
      }
    }
    pos = digitEnd;
  }

  return maxCount;
}

/** Read a File as ArrayBuffer for page probing. */
export async function readFileAsArrayBuffer(file: File): Promise<ArrayBuffer> {
  return file.arrayBuffer();
}
