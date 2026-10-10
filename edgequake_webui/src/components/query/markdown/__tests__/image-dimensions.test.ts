import { describe, expect, it } from "vitest";
import { imageAspectRatio } from "../image-dimensions";

function png(width: number, height: number): Uint8Array {
  const bytes = new Uint8Array(24);
  bytes.set([137, 80, 78, 71, 13, 10, 26, 10], 0);
  bytes[16] = (width >>> 24) & 0xff;
  bytes[17] = (width >>> 16) & 0xff;
  bytes[18] = (width >>> 8) & 0xff;
  bytes[19] = width & 0xff;
  bytes[20] = (height >>> 24) & 0xff;
  bytes[21] = (height >>> 16) & 0xff;
  bytes[22] = (height >>> 8) & 0xff;
  bytes[23] = height & 0xff;
  return bytes;
}

describe("imageAspectRatio", () => {
  it("reads a portrait PNG", () => {
    expect(imageAspectRatio(png(800, 1200))).toBeCloseTo(800 / 1200);
  });

  it("reads a JPEG SOF marker", () => {
    const bytes = new Uint8Array([
      0xff, 0xd8, 0xff, 0xc0, 0x00, 0x11, 0x08, 0x04, 0x00, 0x03, 0x00,
    ]);
    expect(imageAspectRatio(bytes)).toBeCloseTo(0x0300 / 0x0400);
  });

  it("returns null for an unknown header", () => {
    expect(imageAspectRatio(new Uint8Array([1, 2, 3, 4]))).toBeNull();
  });
});
