/**
 * Read width and height from image file headers.
 * Used to reserve the real aspect box before the bitmap paints.
 */

export interface ImageSize {
  width: number;
  height: number;
}

function u16be(bytes: Uint8Array, offset: number): number {
  return (bytes[offset]! << 8) | bytes[offset + 1]!;
}

function u32be(bytes: Uint8Array, offset: number): number {
  return (
    (bytes[offset]! << 24) |
    (bytes[offset + 1]! << 16) |
    (bytes[offset + 2]! << 8) |
    bytes[offset + 3]!
  ) >>> 0;
}

function u16le(bytes: Uint8Array, offset: number): number {
  return bytes[offset]! | (bytes[offset + 1]! << 8);
}

function pngSize(bytes: Uint8Array): ImageSize | null {
  if (bytes.length < 24) return null;
  const sig = [137, 80, 78, 71, 13, 10, 26, 10];
  for (let i = 0; i < sig.length; i++) {
    if (bytes[i] !== sig[i]) return null;
  }
  const width = u32be(bytes, 16);
  const height = u32be(bytes, 20);
  if (width === 0 || height === 0) return null;
  return { width, height };
}

function jpegSize(bytes: Uint8Array): ImageSize | null {
  if (bytes.length < 4 || bytes[0] !== 0xff || bytes[1] !== 0xd8) return null;
  let i = 2;
  while (i + 8 < bytes.length) {
    if (bytes[i] !== 0xff) {
      i += 1;
      continue;
    }
    const marker = bytes[i + 1]!;
    if (marker === 0xc0 || marker === 0xc1 || marker === 0xc2) {
      const height = u16be(bytes, i + 5);
      const width = u16be(bytes, i + 7);
      if (width === 0 || height === 0) return null;
      return { width, height };
    }
    if (marker === 0xd8 || marker === 0xd9 || (marker >= 0xd0 && marker <= 0xd7)) {
      i += 2;
      continue;
    }
    const len = u16be(bytes, i + 2);
    if (len < 2) return null;
    i += 2 + len;
  }
  return null;
}

function gifSize(bytes: Uint8Array): ImageSize | null {
  if (bytes.length < 10) return null;
  const head = String.fromCharCode(bytes[0]!, bytes[1]!, bytes[2]!);
  if (head !== "GIF") return null;
  const width = u16le(bytes, 6);
  const height = u16le(bytes, 8);
  if (width === 0 || height === 0) return null;
  return { width, height };
}

/** Aspect ratio (width / height), or null when the header is not recognized. */
export function imageAspectRatio(bytes: Uint8Array): number | null {
  const size = pngSize(bytes) ?? jpegSize(bytes) ?? gifSize(bytes);
  if (!size) return null;
  return size.width / size.height;
}
