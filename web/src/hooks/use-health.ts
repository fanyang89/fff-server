import { useEffect, useRef, useState } from "react"
import { fetchHealth, type HealthResponse } from "@/api"

export type HealthState = {
  online: boolean
  data: HealthResponse | null
  loading: boolean
}

const POLL_MS = 10_000

export function useHealth(): HealthState {
  const [state, setState] = useState<HealthState>({
    online: false,
    data: null,
    loading: true,
  })
  const abortRef = useRef<AbortController | null>(null)

  useEffect(() => {
    const tick = async () => {
      const ctrl = new AbortController()
      abortRef.current?.abort()
      abortRef.current = ctrl
      try {
        const data = await fetchHealth(ctrl.signal)
        setState({ online: true, data, loading: false })
      } catch (e) {
        if ((e as Error).name === "AbortError") return
        setState((s) => ({ ...s, online: false, loading: false }))
      }
    }

    tick()
    const id = setInterval(tick, POLL_MS)
    return () => {
      clearInterval(id)
      abortRef.current?.abort()
    }
  }, [])

  return state
}
