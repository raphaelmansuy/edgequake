/**
 * @module utils
 * @description Utility functions for class name merging.
 * Combines clsx and tailwind-merge for conditional class handling.
 *
 * @implements FEAT0733 - Tailwind class merging
 */

import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
