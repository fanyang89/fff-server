import { useEffect, useRef, useState } from "react"
import {
  AlertTriangle,
  CheckCircle2,
  LoaderCircle,
  RefreshCw,
  Wrench,
} from "lucide-react"
import { useTranslation } from "react-i18next"
import { triggerReindex } from "@/api"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import { useStats } from "@/hooks/use-stats"
import { formatBytes, formatDuration, formatRelativeTime } from "@/lib/format"

export function MaintenanceDialog() {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const { data, error } = useStats(open)
  const [triggering, setTriggering] = useState(false)
  const [notice, setNotice] = useState<{
    kind: "ok" | "warn" | "err"
    text: string
  } | null>(null)
  const abortRef = useRef<AbortController | null>(null)

  // Clear transient notice when the dialog closes.
  useEffect(() => {
    if (!open) {
      setNotice(null)
      setTriggering(false)
      abortRef.current?.abort()
    }
  }, [open])

  const reindexing = data?.index.reindexing ?? false

  const onTrigger = async () => {
    setNotice(null)
    setTriggering(true)
    const ctrl = new AbortController()
    abortRef.current?.abort()
    abortRef.current = ctrl
    try {
      const resp = await triggerReindex(ctrl.signal)
      setNotice(
        resp.status === "started"
          ? { kind: "ok", text: t("maintenance.noticeStarted") }
          : { kind: "warn", text: t("maintenance.noticeAlreadyRunning") },
      )
    } catch (e) {
      if ((e as Error).name === "AbortError") return
      setNotice({ kind: "err", text: t("maintenance.noticeFailed") })
    } finally {
      setTriggering(false)
    }
  }

  const last = data?.last_reindex ?? null

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="outline" size="sm" className="gap-2">
          <Wrench className="size-4" />
          {t("maintenance.trigger")}
        </Button>
      </DialogTrigger>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>{t("maintenance.title")}</DialogTitle>
          <DialogDescription>{t("maintenance.description")}</DialogDescription>
        </DialogHeader>

        <div className="space-y-5">
          <section className="space-y-2">
            <span className="text-sm font-medium">{t("maintenance.currentState")}</span>
            {error ? (
              <p className="flex items-center gap-1.5 text-destructive text-xs">
                <AlertTriangle className="size-3.5" />
                {t("maintenance.stateUnavailable")}
              </p>
            ) : reindexing ? (
              <p className="flex items-center gap-1.5 text-xs">
                <LoaderCircle className="size-3.5 animate-spin text-muted-foreground" />
                {t("maintenance.stateReindexing")}
              </p>
            ) : (
              <div className="space-y-1 text-muted-foreground text-xs">
                <p className="flex items-center gap-1.5">
                  <span className="size-2 rounded-full bg-emerald-500" />
                  {t("maintenance.stateIdle")}
                </p>
                {data?.index && (
                  <p>
                    {t("maintenance.dbInfo", {
                      time: formatRelativeTime(data.index.db_mtime_unix),
                      size: formatBytes(data.index.db_size_bytes),
                    })}
                  </p>
                )}
              </div>
            )}
          </section>

          <section className="space-y-2">
            <span className="text-sm font-medium">{t("maintenance.lastRebuild")}</span>
            {!last ? (
              <p className="text-muted-foreground text-xs">{t("maintenance.noRecord")}</p>
            ) : (
              <div className="space-y-1.5">
                <div className="flex items-center gap-2 text-xs">
                  {last.success ? (
                    <Badge
                      variant="secondary"
                      className="gap-1 font-normal text-emerald-600"
                    >
                      <CheckCircle2 className="size-3" />
                      {t("maintenance.rebuildSuccess")}
                    </Badge>
                  ) : (
                    <Badge variant="secondary" className="gap-1 font-normal text-destructive">
                      <AlertTriangle className="size-3" />
                      {t("maintenance.rebuildFailed")}
                    </Badge>
                  )}
                  <span className="text-muted-foreground">
                    {t("maintenance.lastRebuildInfo", {
                      time: formatRelativeTime(last.started_at_unix),
                      dur: formatDuration(last.duration_secs),
                    })}
                  </span>
                </div>
                {!last.success && last.error && (
                  <p className="rounded-md border border-destructive/40 bg-destructive/5 p-2 font-mono text-destructive text-xs">
                    {last.error}
                  </p>
                )}
              </div>
            )}
          </section>

          <section className="space-y-2">
            <Button
              className="w-full gap-2"
              onClick={onTrigger}
              disabled={triggering || reindexing}
            >
              {triggering ? (
                <LoaderCircle className="size-4 animate-spin" />
              ) : (
                <RefreshCw className="size-4" />
              )}
              {reindexing ? t("maintenance.reindexingBtn") : t("maintenance.reindexNow")}
            </Button>
            {notice && (
              <p
                className={
                  "text-xs " +
                  (notice.kind === "err"
                    ? "text-destructive"
                    : notice.kind === "warn"
                      ? "text-muted-foreground"
                      : "text-emerald-600")
                }
              >
                {notice.text}
              </p>
            )}
          </section>
        </div>
      </DialogContent>
    </Dialog>
  )
}
