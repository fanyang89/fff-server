import { useEffect, useRef } from "react"
import { Search, X } from "lucide-react"
import { Input } from "@/components/ui/input"

type SearchBarProps = {
  value: string
  onChange: (v: string) => void
}

export function SearchBar({ value, onChange }: SearchBarProps) {
  const ref = useRef<HTMLInputElement>(null)
  const hasValue = value.length > 0

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault()
        ref.current?.focus()
        ref.current?.select()
      }
    }
    window.addEventListener("keydown", onKey)
    return () => window.removeEventListener("keydown", onKey)
  }, [])

  const clear = () => {
    onChange("")
    ref.current?.focus()
  }

  return (
    <div className="relative">
      <Search className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
      <Input
        ref={ref}
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="输入文件名或路径片段…"
        autoFocus
        autoComplete="off"
        spellCheck={false}
        className="h-12 pr-12 pl-9 text-base"
        aria-label="搜索文件"
      />
      {hasValue ? (
        <button
          type="button"
          onClick={clear}
          aria-label="清除"
          className="absolute top-1/2 right-3 flex size-5 -translate-y-1/2 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
        >
          <X className="size-3.5" />
        </button>
      ) : (
        <kbd className="pointer-events-none absolute top-1/2 right-3 -translate-y-1/2 select-none rounded border bg-muted px-1.5 py-0.5 font-mono text-muted-foreground text-xs">
          ⌘K
        </kbd>
      )}
    </div>
  )
}
