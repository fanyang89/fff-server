import { Skeleton } from "@/components/ui/skeleton"
import type { FileItem } from "@/api"
import { ResultItem } from "./result-item"

type ResultListProps = {
  items: FileItem[]
  loading: boolean
  fileServerUrl: string | null
}

export function ResultList({ items, loading, fileServerUrl }: ResultListProps) {
  if (loading) {
    return (
      <div className="space-y-1">
        {Array.from({ length: 5 }).map((_, i) => (
          <div key={i} className="flex items-center gap-3 px-3 py-2">
            <Skeleton className="size-4 rounded" />
            <div className="flex-1 space-y-1.5">
              <Skeleton className="h-3.5 w-1/3" />
              <Skeleton className="h-3 w-2/3" />
            </div>
          </div>
        ))}
      </div>
    )
  }

  return (
    <div className="space-y-0.5">
      {items.map((item, i) => (
        <ResultItem
          key={`${item.absolute_path}-${i}`}
          item={item}
          fileServerUrl={fileServerUrl}
        />
      ))}
    </div>
  )
}
