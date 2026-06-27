import { Badge } from "@/components/ui/badge"

type StatusBarProps = {
  total: number
  truncated: boolean
}

export function StatusBar({ total, truncated }: StatusBarProps) {
  return (
    <div className="flex items-center gap-2 px-3 py-2 text-muted-foreground text-xs">
      <span>共 {total} 项</span>
      {truncated && (
        <Badge variant="secondary" className="font-normal">
          结果已截断，请细化查询
        </Badge>
      )}
    </div>
  )
}
