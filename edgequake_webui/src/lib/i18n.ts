/**
 * @module i18n
 * @description Internationalization configuration.
 * Supports English, Chinese, and French with browser language detection.
 *
 * @implements FEAT0729 - Multi-language support (en, zh, fr)
 * @implements FEAT0730 - Browser language detection
 *
 * @enforces BR0726 - Fallback to English for missing keys
 * @enforces BR0727 - Persist language preference
 */

import i18n from "i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import { initReactI18next } from "react-i18next";

import en from "@/locales/en.json";
import fr from "@/locales/fr.json";
import zh from "@/locales/zh.json";

export const languages = [
  { code: "en", name: "English", nativeName: "English" },
  { code: "zh", name: "Chinese", nativeName: "中文" },
  { code: "fr", name: "French", nativeName: "Français" },
] as const;

export type LanguageCode = (typeof languages)[number]["code"];

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      en: { translation: en },
      zh: { translation: zh },
      fr: { translation: fr },
    },
    fallbackLng: "en",
    interpolation: {
      escapeValue: false, // React already escapes values
    },
    detection: {
      order: ["localStorage", "navigator", "htmlTag"],
      caches: ["localStorage"],
      lookupLocalStorage: "edgequake-language",
    },
  });

export default i18n;
