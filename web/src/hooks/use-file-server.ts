import { useEffect, useState } from "react"
import { fetchFileServer } from "@/api"

export type UseFileServer = {
  url: string | null
  loaded: boolean
}

export function useFileServer(): UseFileServer {
  const [state, setState] = useState<UseFileServer>({ url: null, loaded: false })

  useEffect(() => {
    const ctrl = new AbortController()
    fetchFileServer(ctrl.signal)
      .then((cfg) => setState({ url: cfg.url, loaded: true }))
      .catch((e: unknown) => {
        if ((e as Error).name === "AbortError") return
        // Silent fallback: no browse links. Search keeps working.
        setState({ url: null, loaded: true })
      })
    return () => ctrl.abort()
  }, [])

  return state
}
