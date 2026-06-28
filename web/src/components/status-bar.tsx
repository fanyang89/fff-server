import { useTranslation } from "react-i18next"
import { Badge } from "@/components/ui/badge"
import { formatLatency } from "@/lib/format"

type StatusBarProps = {
  total: number
  truncated: boolean
  elapsedMs: number
}

export function StatusBar({ total, truncated, elapsedMs }: StatusBarProps) {
  const { t } = useTranslation()
  return (
    <div className="flex items-center gap-2 px-3 py-2 text-muted-foreground text-xs">
      <span>{t("status.count", { count: total })}</span>
      <span aria-hidden>·</span>
      <span>{t("status.elapsed", { dur: formatLatency(elapsedMs) })}</span>
      {truncated && (
        <Badge variant="secondary" className="font-normal">
          {t("status.truncated")}
        </Badge>
      )}
    </div>
  )
}
