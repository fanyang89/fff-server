import i18n from "i18next"
import LanguageDetector from "i18next-browser-languagedetector"
import { initReactI18next } from "react-i18next"

import zh from "@/locales/zh/translation.json"
import en from "@/locales/en/translation.json"

export const SUPPORTED_LANGUAGES = ["zh", "en"] as const
export type Language = (typeof SUPPORTED_LANGUAGES)[number]
export const DEFAULT_LANGUAGE: Language = "zh"
export const LANGUAGE_STORAGE_KEY = "lang"

// Migrate stale region-tagged codes (e.g. "zh-CN") left by earlier builds;
// the detector only recognizes the base codes in SUPPORTED_LANGUAGES.
if (typeof localStorage !== "undefined") {
  const stored = localStorage.getItem(LANGUAGE_STORAGE_KEY)
  if (stored && stored.startsWith("zh") && stored !== "zh") {
    localStorage.setItem(LANGUAGE_STORAGE_KEY, "zh")
  }
}

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      zh: { translation: zh },
      en: { translation: en },
    },
    fallbackLng: DEFAULT_LANGUAGE,
    supportedLngs: [...SUPPORTED_LANGUAGES],
    nonExplicitSupportedLngs: true,
    interpolation: {
      escapeValue: false,
    },
    detection: {
      order: ["localStorage", "navigator"],
      lookupLocalStorage: LANGUAGE_STORAGE_KEY,
      caches: ["localStorage"],
    },
  })

i18n.on("languageChanged", (lng) => {
  if (typeof document !== "undefined") {
    document.documentElement.lang = lng
  }
})

export default i18n
