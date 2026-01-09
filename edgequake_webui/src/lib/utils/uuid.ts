/**
 * @module uuid
 * @description UUID Generation Utilities
 *
 * Provides cross-browser compatible UUID v4 generation.
 * crypto.randomUUID() is not available in all contexts (non-secure, older browsers).
 *
 * @implements FEAT0720 - Cross-browser UUID generation
 * @implements FEAT0721 - Secure random fallback
 *
 * @enforces BR0717 - Use crypto.randomUUID when available
 * @enforces BR0718 - Fallback to Math.random for legacy
 */

/**
 * Generate a UUID v4 string.
 * Uses crypto.randomUUID() if available, otherwise falls back to Math.random().
 */
export function generateUUID(): string {
  // Try native crypto.randomUUID first (secure contexts only)
  if (
    typeof crypto !== "undefined" &&
    typeof crypto.randomUUID === "function"
  ) {
    return crypto.randomUUID();
  }

  // Fallback: use crypto.getRandomValues if available for better randomness
  if (
    typeof crypto !== "undefined" &&
    typeof crypto.getRandomValues === "function"
  ) {
    const bytes = new Uint8Array(16);
    crypto.getRandomValues(bytes);

    // Set version (4) and variant (10xx) bits
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join(
      ""
    );
    return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(
      12,
      16
    )}-${hex.slice(16, 20)}-${hex.slice(20)}`;
  }

  // Last resort fallback: Math.random (less secure but always available)
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === "x" ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

/**
 * Generate a short ID (8 characters) for temporary use.
 */
export function generateShortId(): string {
  return generateUUID().slice(0, 8);
}
