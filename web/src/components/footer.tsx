import { LoaderCircle } from "lucide-react"
import { FeedbackDialog } from "@/components/feedback-dialog"
import { formatRelativeTime } from "@/lib/format"
import type { HealthState } from "@/hooks/use-health"

export function Footer({ health }: { health: HealthState }) {
  const { online, data, loading } = health

  let statusText: string
  let dotClass: string
  if (loading && !data) {
    statusText = "连接中"
    dotClass = "bg-muted-foreground"
  } else if (online) {
    statusText = "在线"
    dotClass = "bg-emerald-500"
  } else {
    statusText = "离线"
    dotClass = "bg-destructive"
  }

  const indexText = data
    ? data.reindexing
      ? "索引更新中"
      : `索引 ${formatRelativeTime(data.db_mtime_unix)}`
    : null

  return (
    <footer className="mt-8 flex items-center justify-center gap-3 text-muted-foreground text-xs">
      <span className="flex items-center gap-1.5">
        {loading && !data ? (
          <LoaderCircle className="size-3 animate-spin" />
        ) : (
          <span className={`size-2 rounded-full ${dotClass}`} />
        )}
        {statusText}
      </span>
      {indexText && (
        <>
          <span className="text-muted-foreground/40">·</span>
          <span>{indexText}</span>
        </>
      )}
      <FeedbackDialog />
    </footer>
  )
}
