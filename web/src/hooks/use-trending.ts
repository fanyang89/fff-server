import { useEffect, useRef, useState } from "react"
import { fetchTrending, type TrendingItem } from "@/api"

export type TrendingState = {
  items: TrendingItem[]
  loading: boolean
}

const POLL_MS = 60_000

export function useTrending(): TrendingState {
  const [state, setState] = useState<TrendingState>({
    items: [],
    loading: true,
  })
  const abortRef = useRef<AbortController | null>(null)

  useEffect(() => {
    let cancelled = false

    const tick = async () => {
      const ctrl = new AbortController()
      abortRef.current?.abort()
      abortRef.current = ctrl
      try {
        const data = await fetchTrending(ctrl.signal)
        if (!cancelled) {
          setState({ items: data.items, loading: false })
        }
      } catch (e) {
        if ((e as Error).name === "AbortError") return
        // Silent: trending is a non-essential panel.
        if (!cancelled) setState((s) => ({ ...s, loading: false }))
      }
    }

    tick()
    const id = setInterval(tick, POLL_MS)
    return () => {
      cancelled = true
      clearInterval(id)
      abortRef.current?.abort()
    }
  }, [])

  return state
}
