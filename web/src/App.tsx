import { useState } from "react"
import { McpDialog } from "@/components/mcp-dialog"
import { Results } from "@/components/results"
import { SearchBar } from "@/components/search-bar"
import { useDebounce } from "@/hooks/use-debounce"
import { useSearch } from "@/hooks/use-search"

export default function App() {
  const [query, setQuery] = useState("")
  const debounced = useDebounce(query, 300)
  const { state, refetch } = useSearch(debounced)

  return (
    <div className="mx-auto flex min-h-svh max-w-3xl flex-col px-4 py-8">
      <header className="mb-6 flex items-center justify-between">
        <h1 className="text-lg font-semibold tracking-tight">plocate-web</h1>
        <McpDialog />
      </header>

      <main className="flex flex-1 flex-col gap-4">
        <SearchBar value={query} onChange={setQuery} />
        <div className="flex-1">
          <Results state={state} onRetry={refetch} />
        </div>
      </main>

      <footer className="mt-8 text-center text-muted-foreground text-xs">
        基于 plocate · 路径搜索，毫秒级响应
      </footer>
    </div>
  )
}
