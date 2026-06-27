import { File, Folder } from "lucide-react"
import type { FileItem } from "@/api"

export function ResultItem({ item }: { item: FileItem }) {
  const isDir = item.type === "directory"

  return (
    <div className="flex items-center gap-3 px-3 py-2 transition-colors hover:bg-muted/50">
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
    </div>
  )
}
