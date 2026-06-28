import { useEffect, useState } from "react"
import { Code, Globe } from "lucide-react"
import { useTranslation } from "react-i18next"
import { Footer } from "@/components/footer"
import { LanguageSwitcher } from "@/components/language-switcher"
import { MaintenanceDialog } from "@/components/maintenance-dialog"
import { InstallDialog } from "@/components/install-dialog"
import { Results } from "@/components/results"
import { SearchBar } from "@/components/search-bar"
import { Button } from "@/components/ui/button"
import { withPrefix } from "@/lib/config"
import { useDebounce } from "@/hooks/use-debounce"
import { useFileServer } from "@/hooks/use-file-server"
import { useHealth } from "@/hooks/use-health"
import { useSearch } from "@/hooks/use-search"

export default function App() {
  const { t, i18n: i18nInstance } = useTranslation()
  const [query, setQuery] = useState("")

  useEffect(() => {
    document.title = t("app.title")
  }, [t, i18nInstance.resolvedLanguage])
  const debounced = useDebounce(query, 300)
  const { state, refetch } = useSearch(debounced)
  const health = useHealth()
  const fileServer = useFileServer()

  return (
    <div className="mx-auto flex min-h-svh max-w-3xl flex-col px-4 py-8">
      <header className="mb-6 flex items-center justify-between">
        <h1 className="text-lg font-semibold tracking-tight">{t("app.brand")}</h1>
        <div className="flex items-center gap-2">
          {fileServer.url && (
            <Button asChild variant="outline" size="sm" className="gap-2">
              <a href={fileServer.url} target="_blank" rel="noreferrer">
                <Globe className="size-4" />
                {t("nav.home")}
              </a>
            </Button>
          )}
          <Button asChild variant="outline" size="sm" className="gap-2">
            <a href={withPrefix("/swagger-ui")} target="_blank" rel="noreferrer">
              <Code className="size-4" />
              {t("nav.apiDocs")}
            </a>
          </Button>
          <LanguageSwitcher />
          <MaintenanceDialog />
          <InstallDialog
            instanceName={health.data?.instance_name ?? "plocate"}
            basePath={health.data?.base_path ?? null}
          />
        </div>
      </header>

      <main className="flex flex-1 flex-col gap-4">
        <SearchBar value={query} onChange={setQuery} />
        <div className="flex-1">
          <Results
            state={state}
            onRetry={refetch}
            fileServerUrl={fileServer.url}
          />
        </div>
      </main>

      <Footer health={health} />
    </div>
  )
}
