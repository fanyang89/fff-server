import { useState } from "react"
import { HelpCircle } from "lucide-react"
import { Trans, useTranslation } from "react-i18next"
import { Badge } from "@/components/ui/badge"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip"

const RULE_KEYS = ["multiKeyword", "glob"] as const

const EXAMPLES = [
  { input: "Cargo.toml", key: "syntax.examples.substring" },
  { input: "*.rs", key: "syntax.examples.rustFiles" },
  { input: "rust*json", key: "syntax.examples.rustJson" },
  { input: "**/2024/*.log", key: "syntax.examples.logs" },
  { input: "/etc/*.conf", key: "syntax.examples.etcConf" },
  { input: "config json", key: "syntax.examples.configJson" },
]

export function SyntaxHelpTrigger() {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <button
            type="button"
            aria-label={t("syntax.aria")}
            onClick={() => setOpen(true)}
            className="flex size-5 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          >
            <HelpCircle className="size-3.5" />
          </button>
        </TooltipTrigger>
        <TooltipContent>{t("syntax.tooltip")}</TooltipContent>
      </Tooltip>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>{t("syntax.title")}</DialogTitle>
            <DialogDescription>{t("syntax.description")}</DialogDescription>
          </DialogHeader>

          <div className="space-y-5">
            <section className="space-y-2">
              <span className="text-sm font-medium">{t("syntax.rulesHeading")}</span>
              <ul className="space-y-2">
                {RULE_KEYS.map((r) => (
                  <li
                    key={r}
                    className="flex flex-col gap-0.5 rounded-md border p-2.5"
                  >
                    <span className="text-xs font-medium">{t(`syntax.rules.${r}.title`)}</span>
                    <span className="text-muted-foreground text-xs">
                      {t(`syntax.rules.${r}.desc`)}
                    </span>
                  </li>
                ))}
              </ul>
            </section>

            <section className="space-y-2">
              <span className="text-sm font-medium">{t("syntax.examplesHeading")}</span>
              <ul className="divide-y rounded-md border">
                {EXAMPLES.map((ex) => (
                  <li
                    key={ex.input}
                    className="flex items-center gap-3 px-3 py-2"
                  >
                    <code className="shrink-0 font-mono text-xs font-medium">
                      {ex.input}
                    </code>
                    <span className="text-muted-foreground text-xs">{t(ex.key)}</span>
                  </li>
                ))}
              </ul>
            </section>

            <section className="space-y-2">
              <span className="text-sm font-medium">{t("syntax.currentBehavior")}</span>
              <div className="flex flex-wrap items-center gap-1.5">
                <Badge variant="secondary" className="font-normal">
                  {t("syntax.caseInsensitive")}
                </Badge>
                <Badge variant="secondary" className="font-normal">
                  {t("syntax.fullPath")}
                </Badge>
                <Badge variant="secondary" className="font-normal">
                  {t("syntax.limit100")}
                </Badge>
              </div>
              <p className="text-muted-foreground text-xs">
                <Trans
                  t={t}
                  i18nKey="syntax.apiParamsHint"
                  components={[<code className="font-mono" key="0" />]}
                  values={{ code: "/api/fuzzy" }}
                />
              </p>
            </section>
          </div>
        </DialogContent>
      </Dialog>
    </TooltipProvider>
  )
}
