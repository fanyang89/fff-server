import { useState } from "react"
import { Code } from "lucide-react"
import { Footer } from "@/components/footer"
import { McpDialog } from "@/components/mcp-dialog"
import { Results } from "@/components/results"
import { SearchBar } from "@/components/search-bar"
import { Button } from "@/components/ui/button"
import { useDebounce } from "@/hooks/use-debounce"
import { useHealth } from "@/hooks/use-health"
import { useSearch } from "@/hooks/use-search"

export default function App() {
  const [query, setQuery] = useState("")
  const debounced = useDebounce(query, 300)
  const { state, refetch } = useSearch(debounced)
  const health = useHealth()

  return (
    <div className="mx-auto flex min-h-svh max-w-3xl flex-col px-4 py-8">
      <header className="mb-6 flex items-center justify-between">
        <h1 className="text-lg font-semibold tracking-tight">plocate-web</h1>
        <div className="flex items-center gap-2">
          <Button asChild variant="outline" size="sm" className="gap-2">
            <a href="/swagger-ui" target="_blank" rel="noreferrer">
              <Code className="size-4" />
              API 文档
            </a>
          </Button>
          <McpDialog />
        </div>
      </header>

      <main className="flex flex-1 flex-col gap-4">
        <SearchBar value={query} onChange={setQuery} />
        <div className="flex-1">
          <Results state={state} onRetry={refetch} />
        </div>
      </main>

      <Footer health={health} />
    </div>
  )
}
