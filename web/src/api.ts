export type FileItem = {
  type: string
  name: string
  relative_path: string
  absolute_path: string
}

export type SearchResponse = {
  total_matched: number
  truncated: boolean
  items: FileItem[]
}

export type HealthResponse = {
  ok: boolean
  base_path: string
  db_present: boolean
  db_mtime_unix: number | null
  db_size_bytes: number | null
  reindexing: boolean
  plocate_available: boolean
  updatedb_available: boolean
}

export class SearchError extends Error {
  readonly retryable: boolean

  constructor(message: string, retryable: boolean) {
    super(message)
    this.name = "SearchError"
    this.retryable = retryable
  }
}

export async function fetchSearch(
  q: string,
  signal: AbortSignal,
): Promise<SearchResponse> {
  const params = new URLSearchParams({
    q,
    limit: "100",
    scope: "path",
    case: "true",
  })

  let res: Response
  try {
    res = await fetch(`/api/search?${params.toString()}`, {
      signal,
      headers: { accept: "application/json" },
    })
  } catch (e) {
    if ((e as Error).name === "AbortError") throw e
    throw new SearchError("无法连接到搜索服务", true)
  }

  if (!res.ok) {
    const retryable = res.status >= 500 || res.status === 429
    let detail = `请求失败 (${res.status})`
    try {
      const body = await res.json()
      if (body?.error) detail = body.error
    } catch {
      // ignore JSON parse errors
    }
    throw new SearchError(detail, retryable)
  }

  try {
    return (await res.json()) as SearchResponse
  } catch {
    throw new SearchError("响应解析失败", false)
  }
}

export async function fetchHealth(
  signal: AbortSignal,
): Promise<HealthResponse> {
  const res = await fetch("/api/health", {
    signal,
    headers: { accept: "application/json" },
  })
  if (!res.ok) throw new Error(`health failed (${res.status})`)
  return (await res.json()) as HealthResponse
}
