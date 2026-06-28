import { Flame } from "lucide-react"
import { useTranslation } from "react-i18next"
import { useTrending } from "@/hooks/use-trending"

type TrendingProps = {
  onPick: (query: string) => void
}

export function Trending({ onPick }: TrendingProps) {
  const { t } = useTranslation()
  const { items, loading } = useTrending()

  if (loading || items.length === 0) return null

  return (
    <section
      aria-label={t("trending.title")}
      className="flex flex-wrap items-center gap-1.5 text-sm"
    >
      <span className="inline-flex items-center gap-1 text-muted-foreground">
        <Flame className="size-3.5" aria-hidden />
        {t("trending.title")}
      </span>
      {items.map((it) => (
        <button
          key={it.query}
          type="button"
          onClick={() => onPick(it.query)}
          className="inline-flex max-w-[16rem] items-center gap-1 truncate rounded-full border border-border bg-muted/50 px-2.5 py-0.5 text-xs text-foreground transition-colors hover:border-primary/40 hover:bg-primary/10"
          title={it.query}
        >
          <span className="truncate">{it.query}</span>
        </button>
      ))}
    </section>
  )
}
