import { useEffect, useRef, useState } from "react"
import { fetchStats, type StatsResponse } from "@/api"

export type StatsState = {
  data: StatsResponse | null
  loading: boolean
  error: boolean
}

const POLL_MS = 3_000

export function useStats(enabled: boolean): StatsState {
  const [state, setState] = useState<StatsState>({
    data: null,
    loading: true,
    error: false,
  })
  const abortRef = useRef<AbortController | null>(null)

  useEffect(() => {
    if (!enabled) {
      abortRef.current?.abort()
      return
    }

    let cancelled = false

    const tick = async () => {
      const ctrl = new AbortController()
      abortRef.current?.abort()
      abortRef.current = ctrl
      try {
        const data = await fetchStats(ctrl.signal)
        if (!cancelled) setState({ data, loading: false, error: false })
      } catch (e) {
        if ((e as Error).name === "AbortError") return
        if (!cancelled) setState((s) => ({ ...s, loading: false, error: true }))
      }
    }

    tick()
    const id = setInterval(tick, POLL_MS)
    return () => {
      cancelled = true
      clearInterval(id)
      abortRef.current?.abort()
    }
  }, [enabled])

  return state
}
