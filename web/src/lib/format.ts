export function formatRelativeTime(unixSeconds: number | null): string {
  if (!unixSeconds) return "未知"

  const now = Date.now()
  const then = unixSeconds * 1000
  const diff = Math.max(0, now - then)
  const sec = Math.floor(diff / 1000)

  if (sec < 60) return "刚刚"
  const min = Math.floor(sec / 60)
  if (min < 60) return `${min} 分钟前`
  const hr = Math.floor(min / 60)
  if (hr < 24) return `${hr} 小时前`
  const day = Math.floor(hr / 24)
  if (day < 7) return `${day} 天前`

  const d = new Date(then)
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, "0")
  const day2 = String(d.getDate()).padStart(2, "0")
  return `${y}-${m}-${day2}`
}

export function formatDuration(secs: number): string {
  if (secs < 60) return `${secs.toFixed(secs < 10 ? 1 : 0)} 秒`
  const min = Math.floor(secs / 60)
  const rem = Math.round(secs % 60)
  return rem ? `${min} 分 ${rem} 秒` : `${min} 分`
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
