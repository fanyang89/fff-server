import { Check, Languages } from "lucide-react"
import { DropdownMenu } from "radix-ui"
import { useTranslation } from "react-i18next"
import i18n, { SUPPORTED_LANGUAGES, type Language } from "@/i18n"
import { cn } from "@/lib/utils"

const LANG_LABELS: Record<Language, string> = {
  zh: "中文",
  en: "English",
}

export function LanguageSwitcher() {
  const { i18n: i18nInstance } = useTranslation()
  const current = i18nInstance.resolvedLanguage as Language | undefined

  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>
        <button
          type="button"
          aria-label={i18n.t("language.switch")}
          className="inline-flex h-7 items-center gap-1 rounded-[min(var(--radius-md),12px)] border border-border bg-background px-2.5 text-[0.8rem] font-medium transition-colors hover:bg-muted hover:text-foreground aria-expanded:bg-muted aria-expanded:text-foreground dark:border-input dark:bg-input/30 dark:hover:bg-input/50"
        >
          <Languages className="size-3.5" />
          {current ? LANG_LABELS[current] : "中文"}
        </button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={4}
          className="z-50 min-w-32 overflow-hidden rounded-md border bg-popover p-1 text-sm text-popover-foreground shadow-md outline-none data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0"
        >
          {SUPPORTED_LANGUAGES.map((lng) => (
            <DropdownMenu.Item
              key={lng}
              onSelect={() => i18n.changeLanguage(lng)}
              className={cn(
                "flex cursor-pointer items-center justify-between gap-2 rounded-sm px-2 py-1.5 outline-none data-[highlighted]:bg-muted data-[highlighted]:text-foreground",
              )}
            >
              {LANG_LABELS[lng]}
              {current === lng && <Check className="size-3.5 text-emerald-600" />}
            </DropdownMenu.Item>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}
