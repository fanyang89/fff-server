import { useTranslation } from "react-i18next"
import { Badge } from "@/components/ui/badge"

type StatusBarProps = {
  total: number
  truncated: boolean
}

export function StatusBar({ total, truncated }: StatusBarProps) {
  const { t } = useTranslation()
  return (
    <div className="flex items-center gap-2 px-3 py-2 text-muted-foreground text-xs">
      <span>{t("status.count", { count: total })}</span>
      {truncated && (
        <Badge variant="secondary" className="font-normal">
          {t("status.truncated")}
        </Badge>
      )}
    </div>
  )
}
