import { ArrowUpRight, File, Folder } from "lucide-react"
import { buildBrowseUrl, type FileItem } from "@/api"

type ResultItemProps = {
  item: FileItem
  fileServerUrl: string | null
}

export function ResultItem({ item, fileServerUrl }: ResultItemProps) {
  const isDir = item.type === "directory"
  const href =
    fileServerUrl !== null
      ? buildBrowseUrl(fileServerUrl, item.relative_path, isDir)
      : null

  const inner = (
    <>
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
      {href !== null && (
        <ArrowUpRight className="size-4 shrink-0 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100" />
      )}
    </>
  )

  const className =
    "group flex items-center gap-3 px-3 py-2 transition-colors hover:bg-muted/50"

  if (href === null) {
    return <div className={className}>{inner}</div>
  }

  return (
    <a
      href={href}
      target="_blank"
      rel="noreferrer"
      className={className}
      draggable={false}
    >
      {inner}
    </a>
  )
}
