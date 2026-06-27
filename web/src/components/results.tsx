import { CircleX, LoaderCircle, RotateCcw, SearchX } from "lucide-react"
import { Button } from "@/components/ui/button"
import type { SearchState } from "@/hooks/use-search"
import { ResultList } from "@/components/result-list"
import { StatusBar } from "@/components/status-bar"

type ResultsProps = {
  state: SearchState
  onRetry: () => void
}

export function Results({ state, onRetry }: ResultsProps) {
  const { status, data, error } = state

  if (status === "idle") {
    return (
      <div className="flex flex-col items-center gap-2 py-16 text-center text-muted-foreground">
        <SearchX className="size-8 opacity-40" />
        <p className="text-sm">输入关键字开始搜索</p>
      </div>
    )
  }

  if (status === "error") {
    return (
      <div className="flex flex-col items-center gap-3 py-16 text-center">
        <CircleX className="size-8 text-destructive" />
        <p className="text-sm text-muted-foreground">{error}</p>
        <Button variant="outline" size="sm" className="gap-2" onClick={onRetry}>
          <RotateCcw className="size-3.5" />
          重试
        </Button>
      </div>
    )
  }

  if (status === "empty") {
    return (
      <div className="flex flex-col items-center gap-2 py-16 text-center text-muted-foreground">
        <SearchX className="size-8 opacity-40" />
        <p className="text-sm">无匹配结果</p>
      </div>
    )
  }

  const loading = status === "loading"

  return (
    <div className="py-2">
      {data && !loading && (
        <StatusBar total={data.total_matched} truncated={data.truncated} />
      )}
      {loading && (
        <div className="flex items-center gap-2 px-3 py-2 text-muted-foreground text-xs">
          <LoaderCircle className="size-3.5 animate-spin" />
          搜索中…
        </div>
      )}
      <ResultList
        items={data?.items ?? []}
        loading={loading && !data}
      />
    </div>
  )
}
