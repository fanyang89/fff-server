import { useCallback, useEffect, useRef, useState } from "react"
import { fetchSearch, type SearchResponse } from "@/api"

export type SearchStatus =
  | "idle"
  | "loading"
  | "error"
  | "empty"
  | "success"

export type SearchState = {
  status: SearchStatus
  data: SearchResponse | null
  error: string
  query: string
}

const INITIAL: SearchState = {
  status: "idle",
  data: null,
  error: "",
  query: "",
}

export function useSearch(debouncedQuery: string) {
  const [state, setState] = useState<SearchState>(INITIAL)
  const [nonce, setNonce] = useState(0)
  const abortRef = useRef<AbortController | null>(null)

  const refetch = useCallback(() => setNonce((n) => n + 1), [])

  useEffect(() => {
    const q = debouncedQuery.trim()
    if (!q) {
      abortRef.current?.abort()
      setState(INITIAL)
      return
    }

    const ctrl = new AbortController()
    abortRef.current?.abort()
    abortRef.current = ctrl

    setState((s) => ({
      status: "loading",
      data: s.data,
      error: "",
      query: q,
    }))

    let cancelled = false
    fetchSearch(q, ctrl.signal)
      .then((data) => {
        if (cancelled) return
        setState({
          status: data.items.length === 0 ? "empty" : "success",
          data,
          error: "",
          query: q,
        })
      })
      .catch((e: unknown) => {
        if (cancelled) return
        if ((e as Error).name === "AbortError") return
        const msg =
          e instanceof Error ? e.message : "搜索时发生未知错误"
        setState((s) => ({
          status: "error",
          data: s.data,
          error: msg,
          query: q,
        }))
      })

    return () => {
      cancelled = true
      ctrl.abort()
    }
  }, [debouncedQuery, nonce])

  return { state, refetch }
}
