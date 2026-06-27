import { useState } from "react"
import { Check, Copy, File, Folder } from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import type { FileItem } from "@/api"

export function ResultItem({ item }: { item: FileItem }) {
  const [copied, setCopied] = useState(false)
  const isDir = item.type === "directory"

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(item.absolute_path)
      setCopied(true)
      toast.success("已复制路径")
      setTimeout(() => setCopied(false), 1200)
    } catch {
      toast.error("复制失败")
    }
  }

  return (
    <div className="group flex items-center gap-3 px-3 py-2 transition-colors hover:bg-muted/50">
      {isDir ? (
        <Folder className="size-4 shrink-0 text-muted-foreground" />
      ) : (
        <File className="size-4 shrink-0 text-muted-foreground" />
      )}
      <div className="min-w-0 flex-1">
        <div className="truncate font-medium">{item.name}</div>
        <div className="truncate text-muted-foreground text-xs">
          {item.relative_path}
        </div>
      </div>
      <Button
        variant="ghost"
        size="icon"
        className="size-7 opacity-0 transition-opacity group-hover:opacity-100"
        onClick={copy}
        aria-label="复制路径"
      >
        {copied ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
      </Button>
    </div>
  )
}
