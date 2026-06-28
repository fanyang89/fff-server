import i18n from "@/i18n"

export function formatRelativeTime(unixSeconds: number | null): string {
  if (!unixSeconds) return i18n.t("format.unknown")

  const now = Date.now()
  const then = unixSeconds * 1000
  const diff = Math.max(0, now - then)
  const sec = Math.floor(diff / 1000)

  if (sec < 60) return i18n.t("format.justNow")
  const min = Math.floor(sec / 60)
  if (min < 60) return i18n.t("format.minutesAgo", { count: min })
  const hr = Math.floor(min / 60)
  if (hr < 24) return i18n.t("format.hoursAgo", { count: hr })
  const day = Math.floor(hr / 24)
  if (day < 7) return i18n.t("format.daysAgo", { count: day })

  const d = new Date(then)
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, "0")
  const day2 = String(d.getDate()).padStart(2, "0")
  return `${y}-${m}-${day2}`
}

export function formatDuration(secs: number): string {
  if (secs < 60) {
    return i18n.t("format.durationSeconds", {
      sec: secs.toFixed(secs < 10 ? 1 : 0),
      count: Number(secs.toFixed(0)),
    })
  }
  const min = Math.floor(secs / 60)
  const rem = Math.round(secs % 60)
  return rem
    ? i18n.t("format.durationMinutesSeconds", { min, sec: rem })
    : i18n.t("format.durationMinutes", { count: min })
}

export function formatBytes(bytes: number | null | undefined): string {
  if (!bytes) return "—"
  const units = ["B", "KB", "MB", "GB", "TB"]
  let val = bytes
  let i = 0
  while (val >= 1024 && i < units.length - 1) {
    val /= 1024
    i++
  }
  return `${val.toFixed(val < 10 && i > 0 ? 1 : 0)} ${units[i]}`
}

/// Format a server-side query latency in milliseconds, auto-scaling to
/// microseconds (sub-ms) or seconds (≥1s) for display.
export function formatLatency(ms: number): string {
  if (ms < 1) return `${Math.round(ms * 1000)} µs`
  if (ms < 1000) return `${ms.toFixed(ms < 10 ? 1 : 0)} ms`
  return `${(ms / 1000).toFixed(2)} s`
}
