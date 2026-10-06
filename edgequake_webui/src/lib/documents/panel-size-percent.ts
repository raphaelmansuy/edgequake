/** v4 `getSize()` is a number in some builds and `{ asPercentage }` in others. */
export function panelSizePercent(size: unknown): number | null {
  if (typeof size === "number" && Number.isFinite(size)) return size;
  if (
    size &&
    typeof size === "object" &&
    "asPercentage" in size &&
    typeof (size as { asPercentage: unknown }).asPercentage === "number"
  ) {
    const pct = (size as { asPercentage: number }).asPercentage;
    return Number.isFinite(pct) ? pct : null;
  }
  return null;
}
